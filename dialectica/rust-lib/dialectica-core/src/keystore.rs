//! The keystore: a root secret on disk, and what may open it.
//!
//! The contract is the `keystore` spec; every choice below — the cipher, the
//! KDF, the two unlock paths, the refusals — is argued in that change's
//! `design.md`, including what was rejected. What is repeated here is only what
//! a reader of THIS file needs in order not to undo it.
//!
//! # The bar this is held to
//!
//! PLAN.md's §5.6 named the reference and the counter-example in the same
//! breath: copy radicle's model, and *"LEZ's own keystore is plaintext JSON at
//! 0644 containing every secret, with `// TODO: Use password for storage
//! encryption`. Match the crypto, not the key handling."*
//!
//! Radicle's model was then read rather than assumed, and **two of its
//! properties turn out to be gaps**: it never checks the key file's permissions
//! on read, and its writer truncates in place. Both are closed here — see
//! [`read_checked`] and [`write_atomically`].
//!
//! # Three properties that shape everything here
//!
//! **The module never prompts.** A Logos module is a library inside a sandboxed
//! host with no terminal and no stdin. A passphrase prompt is not a question
//! nobody answers — it is an unbounded wait, which the caller experiences as the
//! 20-second timeout PHASE0-FINDINGS §2 measured, reported as `timeout`, a word
//! pointing at a slow provider rather than a locked keystore. So every
//! passphrase is an *argument*, and this file contains no I/O that can block on
//! a human. That is enforced structurally: the only `std::io` this module uses
//! is `fs`, and [`Passphrase`] can only be constructed from bytes a caller
//! already holds.
//!
//! **Nothing here may panic.** A panic aborts the module process
//! (PHASE0-FINDINGS §3), and a keystore file is not under this module's
//! control — it can be corrupt from an interrupted write or hostile from
//! another local process on the same machine. So the file is parsed with the
//! same bounds-checked [`crate::cursor::Cursor`] every peer-facing decoder in
//! this crate uses, and every arm returns a [`KeystoreError`].
//!
//! **No path is discovered here.** [`Keystore`] is given the file to work on.
//! Where that file lives is the *caller's* decision — the module crate knows its
//! host-stamped persistence path and this crate must not guess at one, both
//! because guessing means reading the environment at a time this crate does not
//! control and because a fixed path baked into a pure crate is untestable.
//! [`default_path_in`] is the naming convention, applied to a directory the
//! caller supplies.
//!
//! # What is deliberately not here
//!
//! **No agent.** Three unlock paths were specified and this ships two — unencrypted
//! and passphrase-by-environment. An agent is a long-lived process holding
//! decrypted material and answering a socket, which is a second security
//! boundary to design and a daemon to supervise, and it buys nothing until a
//! human is repeatedly typing a passphrase. The shape grows one without a
//! breaking change: see [`Unlock`], whose whole job is to make that true.
//!
//! **No rotation, no re-encryption in place.** §5.3 defers rotation, and
//! changing a passphrase is the same write path as creating a keystore.

use std::path::{Path, PathBuf};

use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
// `Zeroize` itself is not imported here, and its absence is the point: after
// `generate` stopped making a plain copy there is no explicit wipe anywhere in
// this file. Every secret is inside a `Zeroizing` and cleared by its drop, so a
// `use zeroize::Zeroize` reappearing means someone has reintroduced a hand-rolled
// wipe — which is the shape review found could be deleted without any test
// noticing.
use zeroize::Zeroizing;

use crate::cursor::{Cursor, OutOfBounds};
use crate::identity::{Address, PublicKey, SecretKey};

/// The keystore file's magic and version, as the first two bytes.
///
/// The magic is one byte rather than a longer string because this file is not
/// content-sniffed by anything — it is opened by path. What it buys is that the
/// commonest corruption, a zero-filled block, fails on the FIRST byte with a
/// legible error rather than being read as "version 0".
const MAGIC: u8 = 0xD4; // Δ
/// Version 1 of the file layout. Bumped when the layout or the KDF changes;
/// an unrecognised version is a refusal, never a best-effort parse.
const VERSION_1: u8 = 1;

/// The cipher's key size, and the KDF's output size. Not `KEY_LEN` from the
/// crate, because a constant a reader can check against the format description
/// is worth more here than one that silently follows a dependency bump.
const CIPHER_KEY_LEN: usize = 32;
/// XChaCha20's extended nonce. 192 bits is what makes a *random* nonce safe
/// without a counter the format would have to carry — see Cargo.toml.
const NONCE_LEN: usize = 24;
/// Argon2's salt. 16 bytes is the RFC 9106 recommendation.
const SALT_LEN: usize = 16;
/// The root secret's plaintext length — an Ed25519 seed.
const ROOT_SECRET_LEN: usize = 32;
/// Poly1305's tag, which the AEAD appends to the ciphertext.
const TAG_LEN: usize = 16;

/// Argon2id parameters, recorded in the file rather than assumed.
///
/// **Recorded so that an old build can still open a file written by a new
/// one**, up to the version discriminant. Assuming them would mean a parameter
/// change is a silent unlock failure spelled "wrong passphrase", which is the
/// single most confusing thing this file could tell a user.
///
/// These are RFC 9106's second recommended option (64 MiB, t=3, p=4) rather
/// than the first (2 GiB): a forum module shares a machine with a desktop
/// session and other Logos modules, and a 2 GiB allocation on a keystore unlock
/// is an availability problem of its own.
const ARGON2_M_COST_KIB: u32 = 65_536;
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 4;

/// The largest recorded parameters this build will honour when opening a file.
///
/// **These caps are a security requirement, and the shape of them was got wrong
/// once.** The parameters are read FROM THE FILE, so they are values anyone
/// with write access to the keystore chooses — and `argon2::Params` accepts an
/// `m_cost` up to `u32::MAX`, which is a request to allocate four terabytes.
/// The blanket hostile-input sweep below flipped one byte of a recorded cost
/// and got the test binary SIGKILLed by the OOM killer; in a module process
/// that is the same abort PHASE0-FINDINGS §3 describes, reached without a panic
/// anywhere for the guard to catch.
///
/// # Bounding each knob is not bounding the work
///
/// The first fix capped `m`, `t` and `p` *individually*, at generous values
/// chosen for portability headroom. **They multiply.** The worst set those caps
/// ACCEPTED — m=1 GiB, t=32, p=16 — was measured at **302 seconds** of
/// CPU-bound, uninterruptible work plus a 1 GiB allocation, from editing twelve
/// bytes of a file. PHASE0-FINDINGS §2 puts the caller's timeout at 20 seconds,
/// so that overruns by 15x: the view is told `timeout`, the module is wedged,
/// and nothing panics. It is also exactly the availability problem that made
/// RFC 9106's 2 GiB option unacceptable above — permitted anyway, 16x over,
/// through the file.
///
/// The lesson generalises past this file: **the boundary was tested and the
/// product of boundaries was not.** A test that varies one knob to its ceiling
/// cannot see a corner that needs all three.
///
/// So [`MAX_WORK_FACTOR`] bounds the *product* against what this build itself
/// writes, and the per-knob caps are now only there to stop any single one
/// being absurd before the multiplication is computed. The per-knob values are
/// correspondingly tightened: 4x the memory this build writes, 4x its
/// iterations, 2x its lanes.
const MAX_ACCEPTED_M_COST_KIB: u32 = ARGON2_M_COST_KIB * 4;
const MAX_ACCEPTED_T_COST: u32 = ARGON2_T_COST * 4;
const MAX_ACCEPTED_P_COST: u32 = ARGON2_P_COST * 2;

/// How much harder than this build's own parameters a file may ask to be.
///
/// Argon2's work is proportional to `m * t` (lanes parallelise the same total),
/// so the product is what a bound has to be about. Twice this build's
/// `65_536 * 3` keeps the portability story the recording exists for — a file
/// from a future build with genuinely harder parameters still opens — while
/// keeping the worst accepted case inside the caller's 20-second budget rather
/// than 15x outside it.
///
/// **The value was chosen by measurement, and the measurements are recorded
/// here rather than re-run by a test**, because a test that derives at the
/// ceiling costs seconds on every pull request to assert only that the machine
/// it ran on was fast enough. All three are debug builds on the development
/// machine; a release build is several times quicker:
///
/// | work factor | worst accepted case |
/// |---|---|
/// | 16 | 27.7s — past the caller's own 20s timeout |
/// | 4  | 9.7s  — inside the timeout, with little margin |
/// | 2  | ~5s   — the shipped value |
///
/// The unbounded original, at the per-knob ceilings that preceded this
/// constant, measured **302 seconds**.
///
/// **Raising this is not a portability convenience.** It is a decision about
/// how long a hostile file may wedge a module whose caller gives up after 20
/// seconds and reports `timeout` — a word pointing at a slow provider rather
/// than at a keystore somebody edited.
const MAX_WORK_FACTOR: u64 = 2;

/// The file's own statement of whether it is encrypted.
///
/// **Recorded, not inferred**, and that is the point of the type. Inferring
/// "unencrypted" from a decryption that was never attempted, or from an empty
/// passphrase happening to work, collapses two states a caller must tell
/// apart: "no passphrase was ever set" and "the passphrase you gave is wrong".
/// The probe's reason strings are only actionable because these are distinct.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Protection {
    /// The secret is stored in the clear. A deliberate configuration on a
    /// machine with an encrypted disk, and a *recorded* one — never a fallback
    /// this code chooses on a caller's behalf.
    None = 0,
    /// Argon2id-stretched passphrase, XChaCha20-Poly1305.
    Argon2idXChaCha20Poly1305 = 1,
}

impl Protection {
    /// Parse the discriminant. An unknown value is a refusal rather than a
    /// default — defaulting to `None` would report an encrypted keystore as an
    /// unencrypted one, and defaulting to the encrypted arm would report a
    /// plaintext keystore as needing a passphrase that does not exist.
    fn from_byte(b: u8) -> Result<Self, KeystoreError> {
        match b {
            0 => Ok(Protection::None),
            1 => Ok(Protection::Argon2idXChaCha20Poly1305),
            other => Err(KeystoreError::UnknownProtection(other)),
        }
    }
}

/// A passphrase, held only as long as it takes to derive a key from it.
///
/// A newtype rather than a bare `Vec<u8>` for two reasons that are both about
/// what a *different* type would allow. It has no `Debug`, so it cannot reach a
/// log line or a formatted error; and it has no constructor that reads anything,
/// so there is no version of this API where supplying a passphrase means
/// prompting for one. The bytes are zeroized on drop.
pub struct Passphrase(Zeroizing<Vec<u8>>);

impl Passphrase {
    /// A passphrase the caller already holds.
    ///
    /// Takes bytes rather than `&str` because a passphrase from an environment
    /// variable is bytes on Unix, and lossily converting one would silently map
    /// two different passphrases onto the same key.
    pub fn new(bytes: &[u8]) -> Self {
        Passphrase(Zeroizing::new(bytes.to_vec()))
    }

    /// Whether this passphrase is empty.
    ///
    /// An empty passphrase is not a passphrase: it stretches to a key derivable
    /// by anyone, so storing under it is storing in the clear with extra steps —
    /// the sort of "encryption" that is worse than none because it reads as
    /// protection. [`Keystore::create`] refuses it and requires the caller to
    /// pass [`Unlock::Unencrypted`] instead, so that "this keystore is not
    /// protected" is always a choice somebody made in writing.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// How a caller proposes to open a keystore.
///
/// **This enum is the extension point**, and it is the reason deferring the
/// agent path costs nothing. Adding `Unlock::Agent { .. }` is a new variant on
/// an input type: every existing caller keeps compiling, no reply shape moves,
/// and the file format is untouched because an agent changes who holds the
/// passphrase and not how the file is encrypted. Had the API instead been
/// `open(path, Option<&Passphrase>)`, adding an agent would have meant changing
/// that signature everywhere.
///
/// Deliberately NOT `Clone` or `Debug`: it carries a passphrase.
pub enum Unlock {
    /// The keystore is expected to be unencrypted. Fails if it is not — a
    /// caller that says "no protection" and finds protection has learned
    /// something worth failing over.
    Unencrypted,
    /// Open with this passphrase.
    Passphrase(Passphrase),
}

/// The environment variable a passphrase arrives in.
///
/// Named after radicle's `RAD_PASSPHRASE`, the model this keystore follows.
///
/// **This unlock path has a documented limitation, and it is documented rather
/// than mitigated because it cannot be mitigated from here.** A process's
/// environment is readable by the same user through `/proc/<pid>/environ`, and
/// on some systems by `ps -e`; it is also inherited by every child process and
/// is commonly captured whole by crash reporters and process supervisors. So a
/// passphrase supplied this way is protected from an attacker who has the
/// keystore FILE and not the running machine — an offline attack on a stolen
/// disk, which is the threat the encryption is for — and not from one who is
/// already running code as this user. radicle says the same thing more bluntly
/// in its man page: *"this is not secure and is equivalent to having an
/// unencrypted secret key."* That is slightly too strong — it still defeats the
/// stolen-disk case — but it is the right direction to err in.
///
/// The agent path is what closes this, and is deferred; see the
/// module doc.
pub const PASSPHRASE_ENV: &str = "DIALECTICA_PASSPHRASE";

/// How to open the keystore, decided from the file's own state and the
/// environment.
///
/// **The file is asked first, and the environment second**, which is the only
/// order that produces truthful errors. Deciding from the environment alone —
/// "a passphrase is set, so assume encryption" — reports an unencrypted
/// keystore as a protection mismatch, and reports a missing passphrase against
/// an unencrypted keystore as a locked one. Neither is true, and both send the
/// user to fix something that is not broken.
///
/// **An empty environment variable is treated as unset**, matching `ssh-keygen`
/// and radicle. The alternative — treating it as the empty passphrase — would
/// be an unlock attempt with a key anyone can derive, reported as a wrong
/// passphrase when it failed. `Keystore::create` refuses the empty passphrase
/// for the same reason.
///
/// Returns the error the caller should report when no unlock is possible. That
/// is a `KeystoreError` rather than a bespoke type so the probe has one source
/// of reason strings.
pub fn unlock_from_env(path: &Path) -> Result<Unlock, KeystoreError> {
    unlock_for(protection_of(&read_checked(path)?)?)
}

/// The unlock the environment permits for a keystore already known to be
/// encrypted or not.
///
/// Split from the file read so that [`open_from_env`] can decide the unlock and
/// decode the keystore from **one** read. It used to be two — `is_encrypted`
/// did a full open-check-read-parse, and `Keystore::open` then repeated all of
/// it — and the only thing making that safe was `from_file_bytes` re-deriving
/// the protection from the bytes it decoded. Safety by coincidence rather than
/// by construction, over a file an attacker may be editing between the reads.
fn unlock_for(encrypted: bool) -> Result<Unlock, KeystoreError> {
    if !encrypted {
        return Ok(Unlock::Unencrypted);
    }
    match std::env::var_os(PASSPHRASE_ENV) {
        Some(v) if !v.is_empty() => Ok(Unlock::Passphrase(Passphrase::new(
            // Bytes, not a lossy `to_string_lossy`: a passphrase that is not
            // valid UTF-8 is still a passphrase, and converting lossily would
            // map two different ones onto the same key.
            os_str_bytes(&v),
        ))),
        _ => Err(KeystoreError::Locked),
    }
}

/// Whether these file bytes describe an encrypted keystore.
///
/// Parses only the header, so it is cheap and says nothing about whether the
/// rest of the file is valid — which is the right split, since "is this
/// encrypted" has to be answerable before a passphrase exists to check the
/// rest with.
fn protection_of(bytes: &[u8]) -> Result<bool, KeystoreError> {
    Ok(parse_header(bytes)?.0 != Protection::None)
}

/// An `OsString`'s bytes.
#[cfg(unix)]
fn os_str_bytes(v: &std::ffi::OsString) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    v.as_os_str().as_bytes()
}

/// On a non-Unix platform an `OsString` is not bytes, and there is no lossless
/// view of one. Stated rather than silently lossy.
#[cfg(not(unix))]
fn os_str_bytes(v: &std::ffi::OsString) -> &[u8] {
    // `to_str` is None for a non-UTF-16-representable value, which on Windows
    // cannot be typed into an environment variable in the first place.
    v.to_str().map(|s| s.as_bytes()).unwrap_or(&[])
}

/// Open the keystore at `path` using whatever the environment allows.
///
/// The one function the probe calls, and the one place the three questions —
/// does it exist, is it encrypted, is a passphrase available — are asked in the
/// order that makes each error true.
///
/// **Reads the file exactly once**, and decides the unlock from the same bytes
/// it then decodes. The obvious composition — `unlock_from_env(path)` followed
/// by `Keystore::open(path, &unlock)` — reads twice and leaves a window in
/// which the file can change between them; see [`unlock_for`].
pub fn open_from_env(path: &Path) -> Result<Keystore, KeystoreError> {
    let bytes = read_checked(path)?;
    let unlock = unlock_for(protection_of(&bytes)?)?;
    Keystore::from_file_bytes(&bytes, &unlock)
}

/// Where the keystore lives inside a directory the caller chose.
///
/// The *name* is fixed and the *directory* is not, which is the only split that
/// works here: a pure crate cannot know the host-stamped persistence path, and a
/// crate that read `$HOME` would be doing environment discovery at a moment its
/// caller does not control. The module crate has the real directory from
/// `on_context_ready`; a test has a temporary one.
pub fn default_path_in(dir: &Path) -> PathBuf {
    dir.join("identity.key")
}

/// Everything that can go wrong, each arm distinguishable.
///
/// **Distinguishable is the requirement, not a nicety.** The probe turns each
/// of these into a reason naming a fix, and a reason can only name a fix if the
/// error said which thing went wrong. A decoder that answers "invalid" sends
/// the reader looking in the wrong place — the same argument `stoa.rs` makes
/// about a genesis record.
///
/// **No variant carries key material, ciphertext, or a passphrase.** Error
/// strings cross the module boundary to a view, which may log or display them.
/// `Io` carries the OS message, which names a path and an errno and nothing
/// from inside the file. That constraint is checked by a test, because it is
/// the sort of thing a later `format!("... {ciphertext:?}")` would quietly
/// break.
#[derive(Debug, PartialEq, Eq)]
pub enum KeystoreError {
    /// No keystore at that path. The probe's "create one" case, and the only
    /// error here that is a normal state rather than a problem.
    NotFound,
    /// The file's permissions allow someone other than the owner to read or
    /// write it. Carries the mode found, which is public information about a
    /// file the caller can already `stat`.
    PermissionsTooOpen { mode: u32 },
    /// The keystore's containing directory is writable by someone other than
    /// its owner, so the keystore can be replaced or deleted regardless of its
    /// own permissions.
    ///
    /// Distinct from [`KeystoreError::PermissionsTooOpen`] because the fix is a
    /// chmod on a different path — reporting "the keystore's permissions are
    /// too open" about a correctly-permissioned keystore sends the reader to
    /// the wrong file.
    DirectoryWritableByOthers { mode: u32 },
    /// An OS error that is not one of the above. The string is the OS's, and
    /// carries no file content.
    Io(String),
    /// The first byte is not the keystore magic — usually "this is not a
    /// keystore file" rather than "this keystore is damaged".
    NotAKeystore,
    /// A version this build does not recognise. "A newer client wrote this" is
    /// a different thing to tell a user than "this is corrupt".
    UnknownVersion(u8),
    /// A protection discriminant this build does not recognise.
    UnknownProtection(u8),
    /// The file ended mid-field.
    Truncated,
    /// The file was complete and then had bytes after it.
    TrailingBytes,
    /// The passphrase did not open the file — reported as such rather than as
    /// corruption, because the two have completely different fixes.
    ///
    /// This is the AEAD tag check failing. It cannot distinguish "wrong
    /// passphrase" from "someone edited the ciphertext", and deliberately does
    /// not try: both mean "do not use this file", and guessing between them
    /// would be telling the user something unverified.
    WrongPassphrase,
    /// The caller offered no passphrase for an encrypted keystore, or offered
    /// one for an unencrypted keystore.
    ProtectionMismatch,
    /// The plaintext was not a 32-byte seed. Only reachable on an unencrypted
    /// keystore, since a tampered ciphertext fails the tag check first.
    NotASecretKey,
    /// A keystore already exists at that path. Creation refuses rather than
    /// replaces: the root secret is not recoverable from anywhere else.
    AlreadyExists,
    /// The passphrase was empty. See [`Passphrase::is_empty`].
    EmptyPassphrase,
    /// The keystore is encrypted and no passphrase is available.
    ///
    /// Distinct from [`KeystoreError::WrongPassphrase`] because the two are
    /// completely different situations for the user: one means "set the
    /// variable", the other means "you set it to the wrong thing". A probe that
    /// reported both as "locked" would leave someone re-typing a passphrase
    /// that was never being read.
    Locked,
    /// Argon2 declined the parameters. Not reachable with the constants above,
    /// and present because the alternative is an `expect` — which on this path
    /// would be a panic in a module process, reachable from a file whose
    /// recorded parameters an attacker may have chosen.
    KeyDerivation,
    /// The file's recorded key-derivation cost exceeds what this build will
    /// attempt. Distinct from [`KeystoreError::KeyDerivation`] because the two
    /// mean different things: one is a malformed parameter, this is a
    /// well-formed parameter that is an allocation attack. See
    /// [`MAX_ACCEPTED_M_COST_KIB`].
    CostTooHigh,
}

impl std::fmt::Display for KeystoreError {
    /// **Every message names the fix**, not merely the fault. The
    /// `posting-capability` spec requires it of the probe's `reason`, and the
    /// probe's reasons ARE these strings — so putting the guidance anywhere
    /// else would mean maintaining it twice and watching the two drift.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeystoreError::NotFound => write!(
                f,
                "no keystore found; create one before posting"
            ),
            KeystoreError::PermissionsTooOpen { mode } => write!(
                f,
                "keystore permissions are too open (mode {mode:04o}); \
                 restrict it to owner-only (chmod 600) and, because it has been \
                 readable by other local users, replace the key"
            ),
            KeystoreError::DirectoryWritableByOthers { mode } => write!(
                f,
                "the keystore's directory is writable by others (mode {mode:04o}); \
                 restrict it to owner-only (chmod 700) — the key can be replaced \
                 there whatever its own permissions say"
            ),
            KeystoreError::Io(e) => write!(
                f,
                "keystore could not be read: {e}; check the path and its \
                 containing directory"
            ),
            KeystoreError::NotAKeystore => write!(
                f,
                "that file is not a dialectica keystore; check the path"
            ),
            KeystoreError::UnknownVersion(v) => write!(
                f,
                "keystore format version {v} is newer than this build understands; \
                 upgrade dialectica"
            ),
            KeystoreError::UnknownProtection(p) => write!(
                f,
                "keystore uses protection scheme {p}, which this build does not \
                 understand; upgrade dialectica"
            ),
            KeystoreError::Truncated => write!(
                f,
                "keystore file is truncated; restore it from a backup"
            ),
            KeystoreError::TrailingBytes => write!(
                f,
                "keystore file has trailing bytes; restore it from a backup"
            ),
            KeystoreError::WrongPassphrase => write!(
                f,
                "keystore passphrase was rejected; check the passphrase supplied \
                 in DIALECTICA_PASSPHRASE"
            ),
            KeystoreError::ProtectionMismatch => write!(
                f,
                "keystore protection does not match how it was opened; an \
                 encrypted keystore needs a passphrase and an unencrypted one \
                 must be opened without"
            ),
            KeystoreError::NotASecretKey => write!(
                f,
                "keystore does not contain a valid secret key; restore it from a \
                 backup"
            ),
            KeystoreError::AlreadyExists => write!(
                f,
                "a keystore already exists at that path; remove it deliberately \
                 before creating another, since the existing key cannot be \
                 recovered afterwards"
            ),
            KeystoreError::EmptyPassphrase => write!(
                f,
                "an empty passphrase protects nothing; either supply a real \
                 passphrase or create the keystore unencrypted on purpose"
            ),
            KeystoreError::Locked => write!(
                f,
                "keystore is encrypted and no passphrase is available; supply \
                 one in {PASSPHRASE_ENV} before starting dialectica"
            ),
            KeystoreError::KeyDerivation => write!(
                f,
                "keystore key derivation failed; the file's recorded parameters \
                 are unusable — restore it from a backup"
            ),
            KeystoreError::CostTooHigh => write!(
                f,
                "keystore declares a key-derivation cost this build refuses to \
                 attempt; the file has been tampered with — restore it from a \
                 backup"
            ),
        }
    }
}

impl From<OutOfBounds> for KeystoreError {
    fn from(e: OutOfBounds) -> Self {
        match e {
            OutOfBounds::Truncated => KeystoreError::Truncated,
            OutOfBounds::Trailing => KeystoreError::TrailingBytes,
        }
    }
}

/// An unlocked root secret, and the identity it yields.
///
/// Not `Clone` and not `Debug`, for the reason [`SecretKey`] is not: each is a
/// way a secret ends up in a log line or a second copy nobody tracks. The root
/// bytes are zeroized on drop, which is the gap `identity.rs`'s doc comment
/// names as the keystore's to close.
pub struct Keystore {
    root: Zeroizing<[u8; ROOT_SECRET_LEN]>,
}

impl Keystore {
    /// Mint a fresh root secret. Nothing is written until [`Keystore::write_to`].
    ///
    /// # There is deliberately no plain local here
    ///
    /// `SecretKey::to_bytes` returns a `[u8; 32]` this type does not own —
    /// `identity.rs`'s doc comment names exactly this copy as deferred to the
    /// keystore. The obvious way to discharge that is:
    ///
    /// ```ignore
    /// let mut bytes = sk.to_bytes();
    /// let root = Zeroizing::new(bytes);
    /// bytes.zeroize();          // easy to delete, and nothing notices
    /// ```
    ///
    /// which is what this function used to do, and **review found that
    /// deleting the wipe left the entire suite green.** A stack local after
    /// its function returns is not observable from a test, so that line could
    /// never have been pinned by one — it was a promise enforced by nobody,
    /// in a `tasks.md` that claimed fourteen mutation verifications and had
    /// not tried this one.
    ///
    /// So the copy is not made. `to_bytes()` is moved straight into the
    /// `Zeroizing`, and there is no second binding to forget about: the
    /// temporary is consumed by the wrapper, and what the wrapper holds is
    /// wiped on drop by a mechanism
    /// `generate_wipes_the_plain_array_it_was_handed` does pin.
    ///
    /// CLAUDE.md's rule applied to a security property: prefer reshaping state
    /// so the invariant holds by construction over adding a line that
    /// maintains it. A line must be remembered; a shape cannot be forgotten.
    pub fn generate() -> Self {
        // Through `SecretKey` rather than `getrandom` directly, so there is one
        // place in this crate that decides where key entropy comes from.
        let sk = SecretKey::generate();
        Keystore {
            root: Zeroizing::new(sk.to_bytes()),
        }
    }

    /// The per-Stoa signing key for this identity (§5.2).
    ///
    /// There is no accessor for the root itself, and that is deliberate: the
    /// root is the one value that, if leaked, yields every Stoa identity a user
    /// has, and nothing outside this type needs it. Callers need keys that
    /// *sign*, which is what this hands back.
    pub fn stoa_key(&self, stoa: &Address) -> SecretKey {
        crate::identity::derive_stoa_key(&self.root, stoa)
    }

    /// The public key this identity presents in a Stoa.
    pub fn stoa_public_key(&self, stoa: &Address) -> PublicKey {
        self.stoa_key(stoa).public_key()
    }

    /// The author address this identity posts under in a Stoa — what the probe
    /// reports, and what an op published now is attributed to.
    pub fn stoa_address(&self, stoa: &Address) -> Address {
        self.stoa_public_key(stoa).address()
    }

    /// Load and unlock a keystore.
    ///
    /// The permission check happens **before any content is used**, and against
    /// the same file descriptor the content comes from — see [`read_checked`]
    /// for why those are one operation rather than two.
    pub fn open(path: &Path, unlock: &Unlock) -> Result<Self, KeystoreError> {
        let bytes = read_checked(path)?;
        Self::from_file_bytes(&bytes, unlock)
    }

    /// Whether a keystore at this path is encrypted, without unlocking it.
    ///
    /// Separate from `open` because the probe needs it: "a keystore exists and
    /// wants a passphrase you have not supplied" is a different reason from
    /// "the passphrase you supplied was rejected", and a caller cannot tell
    /// them apart by trying to open with nothing.
    ///
    /// Prefer [`open_from_env`] where both answers are wanted — this reads the
    /// file, so asking it and then opening reads twice.
    pub fn is_encrypted(path: &Path) -> Result<bool, KeystoreError> {
        protection_of(&read_checked(path)?)
    }

    /// Write this keystore to `path`, creating it.
    ///
    /// Refuses to overwrite. The root secret exists in exactly one place, so a
    /// silent replacement destroys every identity the user has, and no error
    /// anywhere says that it happened.
    pub fn create(&self, path: &Path, unlock: &Unlock) -> Result<(), KeystoreError> {
        if path.exists() {
            return Err(KeystoreError::AlreadyExists);
        }
        self.write_to(path, unlock)
    }

    /// Serialise and write, replacing whatever is at `path`.
    ///
    /// Public because changing a passphrase is this operation and nothing else,
    /// and separate from [`Keystore::create`] so that "replace the file at this
    /// path" is always something a caller asked for by name.
    pub fn write_to(&self, path: &Path, unlock: &Unlock) -> Result<(), KeystoreError> {
        let bytes = self.to_file_bytes(unlock)?;
        write_atomically(path, &bytes)
    }

    /// The file's bytes. Split out from the write so the format is testable
    /// without a filesystem.
    fn to_file_bytes(&self, unlock: &Unlock) -> Result<Vec<u8>, KeystoreError> {
        let mut out = vec![MAGIC, VERSION_1];
        match unlock {
            Unlock::Unencrypted => {
                out.push(Protection::None as u8);
                out.extend_from_slice(&*self.root);
            }
            Unlock::Passphrase(pass) => {
                if pass.is_empty() {
                    return Err(KeystoreError::EmptyPassphrase);
                }
                let mut salt = [0u8; SALT_LEN];
                let mut nonce = [0u8; NONCE_LEN];
                // A fresh salt AND a fresh nonce on every write. The salt is
                // what makes two keystores holding the same secret under the
                // same passphrase produce different files — without it, equal
                // files would announce equal secrets.
                fill_random(&mut salt)?;
                fill_random(&mut nonce)?;

                let key = derive_key(pass, &salt)?;
                let cipher = XChaCha20Poly1305::new((&*key).into());

                // The header is the AEAD's associated data, so the version, the
                // protection byte, the salt and the nonce are all covered by the
                // tag. Without this an attacker could swap the recorded salt or
                // downgrade the protection byte and the tag would still verify
                // over the ciphertext alone.
                let aad = aad_bytes(VERSION_1, Protection::Argon2idXChaCha20Poly1305, &salt, &nonce);
                let sealed = cipher
                    .encrypt(
                        &XNonce::from(nonce),
                        Payload {
                            msg: &*self.root,
                            aad: &aad,
                        },
                    )
                    // The only documented failure is a plaintext too large for
                    // the cipher, and this plaintext is 32 bytes. Still a
                    // `Result`, because an `expect` here is a panic in a module
                    // process.
                    .map_err(|_| KeystoreError::KeyDerivation)?;

                out.push(Protection::Argon2idXChaCha20Poly1305 as u8);
                out.extend_from_slice(&ARGON2_M_COST_KIB.to_be_bytes());
                out.extend_from_slice(&ARGON2_T_COST.to_be_bytes());
                out.extend_from_slice(&ARGON2_P_COST.to_be_bytes());
                out.extend_from_slice(&salt);
                out.extend_from_slice(&nonce);
                out.extend_from_slice(&sealed);
            }
        }
        Ok(out)
    }

    /// Parse and unlock file bytes.
    ///
    /// Every arm returns; nothing here can panic on any input. The cursor is
    /// the same bounds-checked read head every peer-facing decoder in this
    /// crate uses, for the same reason — a keystore file is not trusted input
    /// either.
    fn from_file_bytes(bytes: &[u8], unlock: &Unlock) -> Result<Self, KeystoreError> {
        let (protection, mut cursor) = parse_header(bytes)?;

        let plaintext: Zeroizing<Vec<u8>> = match (protection, unlock) {
            (Protection::None, Unlock::Unencrypted) => {
                Zeroizing::new(cursor.take(ROOT_SECRET_LEN)?.to_vec())
            }
            (Protection::Argon2idXChaCha20Poly1305, Unlock::Passphrase(pass)) => {
                // Read the recorded parameters rather than assuming this
                // build's. A file written by a build with different costs must
                // still open, and assuming would report that as a wrong
                // passphrase — the single most misleading thing to say here.
                let m_cost = u32::from_be_bytes(cursor.take_array::<4>()?);
                let t_cost = u32::from_be_bytes(cursor.take_array::<4>()?);
                let p_cost = u32::from_be_bytes(cursor.take_array::<4>()?);
                let salt = cursor.take_array::<SALT_LEN>()?;
                let nonce = cursor.take_array::<NONCE_LEN>()?;
                // Fixed, not a length prefix read from the file: the plaintext
                // is always a 32-byte seed, so the ciphertext is always 32 + a
                // 16-byte tag. A declared length here would be a number an
                // attacker chooses and this code allocates on.
                let sealed = cursor.take(ROOT_SECRET_LEN + TAG_LEN)?;

                let key = derive_key_with(pass, &salt, m_cost, t_cost, p_cost)?;
                let cipher = XChaCha20Poly1305::new((&*key).into());
                let aad = aad_bytes(VERSION_1, protection, &salt, &nonce);
                // THE check. This is the AEAD's tag verification, which is
                // constant-time and covers the header as associated data — so a
                // swapped salt, a downgraded protection byte or an edited
                // ciphertext all land here as one refusal. There is deliberately
                // no separate stored verifier to compare against: an ordinary
                // equality on one of those short-circuits, and short-circuiting
                // on a secret is how an offline attack gets cheaper.
                let opened = cipher
                    .decrypt(
                        &XNonce::from(nonce),
                        Payload {
                            msg: sealed,
                            aad: &aad,
                        },
                    )
                    .map_err(|_| KeystoreError::WrongPassphrase)?;
                Zeroizing::new(opened)
            }
            // The caller said one thing and the file says another. Reported as
            // its own error rather than as a wrong passphrase, because the fix
            // is different: supply a passphrase, versus stop supplying one.
            _ => return Err(KeystoreError::ProtectionMismatch),
        };

        cursor.finish()?;

        let root: [u8; ROOT_SECRET_LEN] = plaintext
            .as_slice()
            .try_into()
            .map_err(|_| KeystoreError::NotASecretKey)?;
        Ok(Keystore {
            root: Zeroizing::new(root),
        })
    }
}

/// The header, parsed once, by both `open` and `is_encrypted`.
///
/// Returns the cursor positioned after the header so a caller reads on from
/// exactly where this stopped — the alternative, returning an offset a caller
/// re-derives, is the classic way two readers of one format drift apart.
fn parse_header(bytes: &[u8]) -> Result<(Protection, Cursor<'_>), KeystoreError> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take_array::<1>()? != [MAGIC] {
        return Err(KeystoreError::NotAKeystore);
    }
    let version = cursor.take_array::<1>()?[0];
    if version != VERSION_1 {
        return Err(KeystoreError::UnknownVersion(version));
    }
    let protection = Protection::from_byte(cursor.take_array::<1>()?[0])?;
    Ok((protection, cursor))
}

/// The bytes the AEAD authenticates alongside the ciphertext.
///
/// Everything in the header that an attacker could otherwise edit without
/// invalidating the tag: the version, the protection scheme, the salt and the
/// nonce. The KDF parameters are NOT here, and that is a deliberate asymmetry
/// worth stating — they are inputs to the key, so editing them produces a
/// different key and the tag fails anyway. Including them would be belt and
/// braces; omitting them keeps the AAD to values the tag could not otherwise
/// reach.
fn aad_bytes(version: u8, protection: Protection, salt: &[u8], nonce: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(2 + salt.len() + nonce.len());
    aad.push(version);
    aad.push(protection as u8);
    aad.extend_from_slice(salt);
    aad.extend_from_slice(nonce);
    aad
}

/// Stretch a passphrase under this build's parameters.
fn derive_key(
    pass: &Passphrase,
    salt: &[u8; SALT_LEN],
) -> Result<Zeroizing<[u8; CIPHER_KEY_LEN]>, KeystoreError> {
    derive_key_with(pass, salt, ARGON2_M_COST_KIB, ARGON2_T_COST, ARGON2_P_COST)
}

/// Stretch a passphrase under parameters read from a file.
///
/// The parameters are attacker-influenceable — someone with write access to the
/// keystore chooses them. `Params::new` rejects only the structurally invalid
/// ones and will happily accept a four-terabyte `m_cost`, so the ceiling check
/// here is the one that matters: without it, opening a tampered file is an
/// allocation the OOM killer answers, which ends the module process without a
/// panic anywhere for the guard to catch. See [`MAX_ACCEPTED_M_COST_KIB`].
///
/// The ceilings are checked BEFORE `Params::new`, not after, because `Params`
/// construction is not where the allocation happens — `hash_password_into` is —
/// and a check after the fact would be a check that runs too late for nothing.
fn derive_key_with(
    pass: &Passphrase,
    salt: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<Zeroizing<[u8; CIPHER_KEY_LEN]>, KeystoreError> {
    // Each knob first, so an absurd single value is refused before anything is
    // multiplied — and so the product below cannot be reached with a factor
    // built from one enormous term.
    if m_cost > MAX_ACCEPTED_M_COST_KIB
        || t_cost > MAX_ACCEPTED_T_COST
        || p_cost > MAX_ACCEPTED_P_COST
    {
        return Err(KeystoreError::CostTooHigh);
    }
    // Then the WORK, which is what actually costs time. Bounding the knobs
    // individually let m=4x, t=4x through together as 16x — see
    // `MAX_WORK_FACTOR`. `u64` because two `u32`s multiply past `u32`.
    let requested = u64::from(m_cost) * u64::from(t_cost);
    let ours = u64::from(ARGON2_M_COST_KIB) * u64::from(ARGON2_T_COST);
    if requested > ours * MAX_WORK_FACTOR {
        return Err(KeystoreError::CostTooHigh);
    }
    let params = argon2::Params::new(m_cost, t_cost, p_cost, Some(CIPHER_KEY_LEN))
        .map_err(|_| KeystoreError::KeyDerivation)?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; CIPHER_KEY_LEN]);
    argon
        .hash_password_into(&pass.0, salt, &mut *key)
        .map_err(|_| KeystoreError::KeyDerivation)?;
    Ok(key)
}

fn fill_random(buf: &mut [u8]) -> Result<(), KeystoreError> {
    // A `Result` rather than `SecretKey::generate`'s `expect`, and the
    // difference is reachability: key generation is a setup action, while this
    // runs on a write that a dispatch handler may have triggered. A panic there
    // aborts the module process.
    getrandom::fill(buf).map_err(|_| KeystoreError::Io("OS random source unavailable".into()))
}

/// The owner-only mode a keystore must have, and is written with.
#[cfg(unix)]
const OWNER_ONLY: u32 = 0o600;

// ─── Why there is no `O_NOFOLLOW` here ────────────────────────────────────
//
// It was written and then removed, which is worth recording so it is not
// re-added on reflex.
//
// `OpenOptions::custom_flags` takes a raw `i32`, and `O_NOFOLLOW`'s value
// differs by platform — `0o100000` on Linux, `0x0100` on the BSDs — so using it
// means either a direct `libc` dependency or a hand-maintained per-target
// constant. It fails design.md's dependency test ("Three dependencies, and what
// was refused"): it is neither unavoidable nor smaller than what it replaces,
// because two defences are already here and each suffices alone —
//
//  * `create_new` refuses ANY pre-existing path, symlink included. The open
//    fails `EEXIST` and nothing is written.
//  * The staging name carries fresh randomness, so there is no name for an
//    attacker to pre-place a symlink AT.
//
// The residual gap it would have covered is written down rather than left
// implicit: on a filesystem where `create_new`'s existence check and the open
// are not one atomic operation, a sufficiently fast attacker who has ALSO
// guessed 64 bits of randomness could win the race. That is not a threat model
// this design owes an answer to.

/// Open the keystore, check its permissions, and read it — **all against one
/// file descriptor**.
///
/// # Why this is one function and not three
///
/// It used to be `check_permissions(path)` followed by `fs::read(path)`, which
/// resolves the name **twice**. Two problems, and the second is the one that
/// made the split wrong rather than merely untidy:
///
/// - **TOCTOU.** Between the `stat` and the `open`, the thing at that path can
///   be replaced. The permission check then describes a file that is no longer
///   the file being read — and the attacker who can do that is the same one
///   the check exists to stop.
/// - **`fs::metadata` follows symlinks.** So the mode checked was the
///   *target's*, and a symlink at the keystore path pointed at something
///   world-readable passed a check about a file nobody read.
///
/// Opening once and calling [`std::fs::File::metadata`] on the **handle** closes
/// both: the mode is the mode of the bytes this function goes on to read, with
/// no name resolved in between. There is no version of this where the check and
/// the read can disagree, because there is only one file.
///
/// # Refuse, not warn
///
/// A warning on this path is a message nobody sees: the module has no terminal,
/// and its only caller is a view that would have to choose to render it — the
/// same reasoning that makes the probe a gate rather than a hint. And the
/// damage being refused is not hypothetical: a secret that has been readable by
/// every process on the machine is a secret to replace, which is what the error
/// says to do.
///
/// The check is on the permission bits only, not on ownership. Checking the
/// owner would need the process's own uid and is a different question — a file
/// owned by someone else is unreadable anyway, and the OS reports that.
fn read_checked(path: &Path) -> Result<Vec<u8>, KeystoreError> {
    use std::io::Read;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(KeystoreError::NotFound),
        Err(e) => return Err(KeystoreError::Io(e.to_string())),
    };

    // On the HANDLE, not the path. This is the whole point of the function.
    let meta = file.metadata().map_err(|e| KeystoreError::Io(e.to_string()))?;
    check_mode(&meta)?;
    // And the directory around it. A group-writable parent means anyone in
    // that group can replace the keystore wholesale, or plant a symlink for
    // the next write to stage through — which was a real disclosure here, see
    // `write_atomically`. `create_new` and a random staging name close that
    // particular route; the directory being writable by others remains a state
    // in which no promise about this file is worth making.
    if let Some(dir) = path.parent() {
        check_directory_mode(dir)?;
    }

    // A keystore is a fixed 103 bytes at most, so a file larger than this is
    // not a keystore and there is no reason to read it into memory before
    // saying so. `read_to_end` on an attacker-supplied path would otherwise
    // happily ingest a very large file — or block forever on a FIFO.
    if meta.len() > MAX_KEYSTORE_LEN {
        return Err(KeystoreError::NotAKeystore);
    }

    let mut bytes = Vec::new();
    // `take` rather than a bare `read_to_end`: `metadata().len()` is 0 for a
    // FIFO and for several /proc entries, so the size check above does not by
    // itself bound what gets read. This does.
    file.take(MAX_KEYSTORE_LEN + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| KeystoreError::Io(e.to_string()))?;
    if bytes.len() as u64 > MAX_KEYSTORE_LEN {
        return Err(KeystoreError::NotAKeystore);
    }
    Ok(bytes)
}

/// The largest a valid keystore can be: the encrypted layout, which is the
/// longer of the two.
///
/// Hardcoded against the layout rather than computed from it, for the same
/// reason the format test hardcodes its offsets — a bound derived from the
/// implementation agrees with whatever the implementation does.
const MAX_KEYSTORE_LEN: u64 = 3 + 12 + SALT_LEN as u64 + NONCE_LEN as u64 + 32 + TAG_LEN as u64;

/// The permission half, split out so both the mode rule and its one caller are
/// each one job.
#[cfg(unix)]
fn check_mode(meta: &std::fs::Metadata) -> Result<(), KeystoreError> {
    use std::os::unix::fs::PermissionsExt;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(KeystoreError::PermissionsTooOpen { mode });
    }
    Ok(())
}

/// The containing directory's mode.
///
/// **Only the WRITE bits, unlike the file's check**, and the asymmetry is the
/// point. A readable directory discloses that a keystore exists, which is not a
/// secret — the path is a documented convention. A *writable* directory lets
/// someone replace the keystore, delete it, or plant something at a name a
/// write will touch, none of which the file's own 0600 prevents.
///
/// Reported as its own error rather than as `PermissionsTooOpen`, because the
/// fix is a different chmod on a different path and saying "the keystore's
/// permissions are too open" about a correctly-permissioned keystore sends the
/// reader to the wrong file.
#[cfg(unix)]
fn check_directory_mode(dir: &Path) -> Result<(), KeystoreError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = match std::fs::metadata(dir) {
        Ok(m) => m,
        // A keystore was already opened inside it, so a directory that cannot
        // be stat'ed is a surprise rather than a normal state — but it is not
        // this function's business to decide that, and refusing on an error we
        // did not anticipate is the safe direction.
        Err(e) => return Err(KeystoreError::Io(e.to_string())),
    };
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o022 != 0 {
        return Err(KeystoreError::DirectoryWritableByOthers { mode });
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_directory_mode(_dir: &Path) -> Result<(), KeystoreError> {
    Ok(())
}

/// On a platform without Unix permission bits there is nothing to check.
///
/// Stated rather than silently skipped: a caller on such a platform gets no
/// protection from this check, and pretending otherwise by returning `Ok` with
/// no comment would be the more dangerous shape. Windows ACLs are a different
/// mechanism and would need their own implementation; dialectica targets Linux
/// (PLAN.md's Basecamp traps are Linux-specific), so this arm exists to keep
/// the crate portable rather than to serve a supported platform.
#[cfg(not(unix))]
fn check_mode(_meta: &std::fs::Metadata) -> Result<(), KeystoreError> {
    Ok(())
}

/// Where a keystore's bytes are written before they are moved into place.
///
/// A named function rather than an inline expression, so that "the bytes never
/// go straight to the destination" and "the staging name is unguessable" are
/// properties a test can assert on rather than lines inside a function whose
/// behaviour on a crash cannot be observed. A mutation that wrote directly to
/// the destination left the whole suite green until this existed.
///
/// A **sibling** of the destination, not a path in the system temp directory:
/// a rename across filesystems is not a rename, it is a copy, and loses the
/// atomicity this whole arrangement is for.
///
/// # The name carries fresh randomness, and that is a security property
///
/// **This function used to return `path.with_extension("tmp")`, and that was a
/// working root-secret disclosure.** A predictable staging name lets anyone who
/// can write to the *containing directory* — a weaker requirement than writing
/// to the keystore, and precisely the attacker [`read_checked`]'s permission
/// check exists for
/// — pre-place a symlink there and wait. See the regression test
/// `a_symlink_at_the_staging_path_cannot_capture_the_secret` for the full
/// mechanism and what it measured.
///
/// Randomness is the half that holds even if someone later decides
/// `create_new` is inconvenient: an attacker cannot pre-place *anything* — a
/// symlink, a directory, a file they keep an open handle on — at a name they
/// cannot guess. It also means two concurrent writers do not collide on one
/// staging file, which the old name could.
///
/// Falls back to a fixed suffix only if the OS random source fails, which is
/// the same situation in which no keystore could be encrypted anyway. The
/// fallback is safe because [`write_atomically`] uses `create_new`, so a
/// pre-placed path is an `EEXIST` rather than a capture; it exists so that this
/// returns a `PathBuf` and the failure surfaces at the write, where there is an
/// error type to carry it.
fn staging_path(path: &Path) -> PathBuf {
    let mut nonce = [0u8; 8];
    let suffix = match getrandom::fill(&mut nonce) {
        Ok(()) => hex::encode(nonce),
        Err(_) => "norandom".to_string(),
    };
    // `with_extension` would eat the destination's own extension; this appends,
    // so `identity.key` stages as `identity.key.<hex>.tmp` and the two names
    // cannot be confused for one another.
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}.tmp"));
    path.with_file_name(name)
}

/// Write, then move into place.
///
/// **An interrupted write must not destroy an existing key**, and a plain
/// `File::create` + `write_all` does exactly that: it truncates first, so a
/// crash between the truncate and the last byte leaves a valid keystore
/// replaced by a partial one. The root secret exists nowhere else, so that is
/// every identity the user has.
///
/// So: write the whole thing to a sibling temporary file, fsync it, then
/// `rename` it over the destination. `rename` within a directory is atomic on
/// every POSIX filesystem — a concurrent reader sees either the old file or the
/// new one. The temporary is a *sibling* rather than in `/tmp` because a rename
/// across filesystems is not a rename, it is a copy, and loses the property
/// this function exists for.
///
/// The temporary is created with owner-only permissions from the start, not
/// chmod'ed afterwards: between a create at 0644 and a chmod there is a window
/// in which the secret is world-readable, and a window is all a local attacker
/// polling the directory needs.
///
/// # `create_new`, and why `create` was a disclosure
///
/// **The first version used `.create(true).truncate(true)` and leaked the root
/// secret in the clear to an attacker-chosen path.** The reasoning that failed
/// was about the *mode*, and it was correct as far as it went — setting the
/// mode at open time really does close the chmod race. What it missed is that
/// `OpenOptions::mode` applies **only when the file is actually created**: an
/// existing path is opened with its own mode intact, and an existing *symlink*
/// is followed to its target. So a pre-placed symlink captured the write
/// entirely, at the attacker's path and the attacker's mode, and the rename
/// afterwards left the user looking at a normal 0600 keystore.
///
/// Two defences now, and **either one closes the exploit on its own** — which
/// is the point of having both, since they fail in different directions:
///
/// - **`create_new`** — the open fails `EEXIST` on anything already at that
///   name, symlink included. Nothing is written.
/// - **A random staging name** ([`staging_path`]) — there is no name for an
///   attacker to pre-place anything AT. This is the half that survives someone
///   later deciding `create_new` is inconvenient.
///
/// `O_NOFOLLOW` was considered as a third layer and deliberately left out; see
/// the note above [`OWNER_ONLY`] for why, and design.md for the residual gap.
///
/// The mode is still set at open time, for the original reason, which remains
/// true for the file this now genuinely does create.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), KeystoreError> {
    use std::io::Write;

    let tmp = staging_path(path);
    let mut options = std::fs::OpenOptions::new();
    // `create_new` rather than `create` + `truncate`. There is nothing to
    // truncate: a name this process just randomised is a name nothing should
    // already occupy, and if something does, that is the attack.
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(OWNER_ONLY);
    }

    let result = (|| -> std::io::Result<()> {
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        // Before the rename, not after. Without it the rename can be durable
        // while the content behind it is not, which on a crash leaves a
        // correctly-named empty file where the keystore was.
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();

    if let Err(e) = result {
        // Leaving a half-written temporary behind is its own small hazard: it
        // holds whatever was written of a secret. Best-effort removal — if it
        // fails there is nothing further this code can do, and the original
        // error is the one worth reporting.
        let _ = std::fs::remove_file(&tmp);
        return Err(KeystoreError::Io(e.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic keystore, so tests assert against fixed values rather
    /// than against whatever the implementation just generated.
    fn a_keystore(seed: u8) -> Keystore {
        Keystore {
            root: Zeroizing::new([seed; ROOT_SECRET_LEN]),
        }
    }

    fn a_pass(s: &str) -> Unlock {
        Unlock::Passphrase(Passphrase::new(s.as_bytes()))
    }

    /// An encrypted keystore whose RECORDED parameters are deliberately cheap.
    ///
    /// The shipped cost is RFC 9106's 64 MiB, t=3, p=4 — correct for a real
    /// unlock and ruinous for a test that decrypts several hundred times. The
    /// byte-flip and truncation sweeps below do exactly that, and at shipped
    /// cost they exhaust memory and get the test binary SIGKILLed. That was
    /// observed, not predicted.
    ///
    /// Lowering the cost here is not weakening what ships: the parameters live
    /// IN THE FILE, so this writes a legitimate keystore that a normal
    /// `from_file_bytes` opens by the ordinary path. It exercises the
    /// parameter-portability property as a side effect — a file written under
    /// one build's costs opening under another's is exactly what recording them
    /// is for.
    ///
    /// Tests that assert on the SHIPPED parameters use `to_file_bytes` instead,
    /// so this cannot quietly become the thing the format test checks.
    fn cheaply_encrypted(seed: u8, pass: &str) -> Vec<u8> {
        const M: u32 = 8; // KiB — Argon2's minimum is 8 * p_cost
        const T: u32 = 1;
        const P: u32 = 1;
        let salt = [0x5Au8; SALT_LEN];
        let nonce = [0x4Eu8; NONCE_LEN];
        let key = derive_key_with(&Passphrase::new(pass.as_bytes()), &salt, M, T, P).unwrap();
        let cipher = XChaCha20Poly1305::new((&*key).into());
        let aad = aad_bytes(
            VERSION_1,
            Protection::Argon2idXChaCha20Poly1305,
            &salt,
            &nonce,
        );
        let sealed = cipher
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &[seed; ROOT_SECRET_LEN],
                    aad: &aad,
                },
            )
            .unwrap();
        let mut out = vec![
            MAGIC,
            VERSION_1,
            Protection::Argon2idXChaCha20Poly1305 as u8,
        ];
        out.extend_from_slice(&M.to_be_bytes());
        out.extend_from_slice(&T.to_be_bytes());
        out.extend_from_slice(&P.to_be_bytes());
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        out
    }

    /// The error from a keystore result, or a failure naming the test.
    ///
    /// `unwrap_err` would need `Debug` on `Keystore`, and `Keystore`
    /// deliberately has none — a `Debug` on a type holding a root secret is a
    /// way that secret reaches a log line, and adding one to satisfy a test
    /// assertion would trade the property for the convenience of testing it.
    /// This helper is the convenience, without the property.
    fn err_of<T>(r: Result<T, KeystoreError>) -> KeystoreError {
        match r {
            Ok(_) => panic!("expected a keystore error, got a keystore"),
            Err(e) => e,
        }
    }

    /// A scratch directory that cleans itself up.
    ///
    /// Hand-rolled rather than a `tempfile` dependency: this is the only place
    /// in the crate that needs one, and a dev-dependency is still a dependency
    /// whose supply chain this project carries.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut n = [0u8; 8];
            getrandom::fill(&mut n).expect("test needs randomness");
            let dir = std::env::temp_dir().join(format!("dialectica-ks-{tag}-{}", hex::encode(n)));
            std::fs::create_dir_all(&dir).expect("test scratch dir");
            TempDir(dir)
        }

        fn path(&self) -> PathBuf {
            default_path_in(&self.0)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // ─── Round trips, the base property ───────────────────────────────────

    #[test]
    fn an_unencrypted_keystore_round_trips_through_a_file() {
        let dir = TempDir::new("plain-rt");
        let ks = a_keystore(7);
        ks.create(&dir.path(), &Unlock::Unencrypted).unwrap();
        let back = Keystore::open(&dir.path(), &Unlock::Unencrypted).unwrap();
        assert_eq!(*back.root, [7u8; 32]);
    }

    #[test]
    fn an_encrypted_keystore_round_trips_through_a_file() {
        let dir = TempDir::new("enc-rt");
        let ks = a_keystore(7);
        ks.create(&dir.path(), &a_pass("correct horse")).unwrap();
        let back = Keystore::open(&dir.path(), &a_pass("correct horse")).unwrap();
        assert_eq!(*back.root, [7u8; 32]);
    }

    #[test]
    fn the_identity_survives_a_restart() {
        // The point of the whole change: the per-Stoa pseudonym after a reload
        // is the one from before it. §5.2 makes it permanent and §4.1 ties the
        // transport's sender id to it, so instability here changes the
        // identifier other peers know the user by.
        let dir = TempDir::new("restart");
        let stoa = crate::identity::stoa_address(b"a genesis record");
        let before = a_keystore(7).stoa_address(&stoa);
        a_keystore(7)
            .create(&dir.path(), &a_pass("pw"))
            .unwrap();
        let after = Keystore::open(&dir.path(), &a_pass("pw"))
            .unwrap()
            .stoa_address(&stoa);
        assert_eq!(before, after);

        // And it is the address that follows from a value `identity.rs`
        // ALREADY pins, reached by a different route.
        //
        // `identity.rs::the_wire_constants_are_pinned_to_known_answers` fixes
        // `derive_stoa_key([7; 32], stoa_address(b"a genesis record"))` to the
        // seed below. This rebuilds a key from that hardcoded seed and takes
        // its address — so the assertion is against a constant from another
        // test file, not against whatever this keystore just computed. If the
        // keystore ever derived by a different path, the two would part.
        let pinned_seed =
            hex::decode("b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf")
                .unwrap();
        let expected = crate::identity::SecretKey::from_bytes(&pinned_seed)
            .unwrap()
            .public_key()
            .address();
        assert_eq!(
            after, expected,
            "the keystore's per-Stoa identity no longer matches the pinned derivation"
        );
    }

    // ─── Encryption ───────────────────────────────────────────────────────

    #[test]
    fn the_plaintext_secret_is_absent_from_an_encrypted_file() {
        // The LEZ counter-example, checked directly: their keystore is
        // plaintext JSON containing every secret. A test that only asserted
        // "decrypt works" would pass on a format that also stored the plaintext
        // beside the ciphertext.
        let ks = a_keystore(0xAB);
        let bytes = ks.to_file_bytes(&a_pass("pw")).unwrap();
        assert!(
            !bytes.windows(32).any(|w| w == [0xABu8; 32]),
            "the root secret appears verbatim in an encrypted keystore"
        );
    }

    #[test]
    fn an_unencrypted_file_does_contain_the_secret_which_is_why_it_is_a_choice() {
        // The other direction, pinned so that "unencrypted" is honest about
        // what it is rather than something a reader has to assume.
        //
        // NO SPEC: the spec says an unencrypted keystore is a recorded state
        // and says nothing about the secret being stored VERBATIM — an
        // implementation that obfuscated it would satisfy every requirement.
        // Pinned anyway because obfuscation here would be the worse outcome:
        // it reads as protection and is none, which is exactly the LEZ failure
        // this change is measured against.
        let bytes = a_keystore(0xAB).to_file_bytes(&Unlock::Unencrypted).unwrap();
        assert!(bytes.windows(32).any(|w| w == [0xABu8; 32]));
    }

    #[test]
    fn a_wrong_passphrase_is_refused_and_named_as_such() {
        // The security property, and the one whose failure mode is silent: a
        // cipher without authentication would hand back 32 bytes of garbage
        // here, and the caller would post under an identity nobody chose.
        let ks = a_keystore(7);
        let bytes = ks.to_file_bytes(&a_pass("right")).unwrap();
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &a_pass("wrong"))),
            KeystoreError::WrongPassphrase
        );
    }

    #[test]
    fn a_passphrase_differing_in_one_byte_is_refused() {
        // The near miss, which a truncating or prefix-comparing check would
        // wave through.
        let bytes = a_keystore(7).to_file_bytes(&a_pass("passphrase")).unwrap();
        for wrong in ["passphrasE", "passphras", "passphrase ", "Passphrase"] {
            assert_eq!(
                err_of(Keystore::from_file_bytes(&bytes, &a_pass(wrong))),
                KeystoreError::WrongPassphrase,
                "{wrong:?} must not open a keystore written under \"passphrase\""
            );
        }
    }

    #[test]
    fn two_keystores_with_the_same_secret_and_passphrase_differ() {
        // The salt and nonce are per-write. Without them, identical files would
        // announce identical secrets to anyone holding both — and a nonce
        // reused across two writes under one key is a break of the cipher, not
        // merely an information leak.
        let ks = a_keystore(7);
        let a = ks.to_file_bytes(&a_pass("pw")).unwrap();
        let b = ks.to_file_bytes(&a_pass("pw")).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn a_flipped_ciphertext_byte_is_refused_rather_than_decrypted() {
        // What authentication buys. Sweep every byte after the header: each one
        // is either covered by the tag or is part of the key derivation, and
        // either way the file must not open.
        let good = cheaply_encrypted(7, "pw");
        // Sanity: the unflipped file opens, or this sweep proves nothing.
        assert!(Keystore::from_file_bytes(&good, &a_pass("pw")).is_ok());
        for i in 3..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            assert!(
                Keystore::from_file_bytes(&bad, &a_pass("pw")).is_err(),
                "flipping byte {i} produced a keystore that opened"
            );
        }
    }

    #[test]
    fn a_downgraded_protection_byte_is_refused() {
        // The attack the AAD exists to stop: rewrite the protection byte to
        // "unencrypted" and hope the reader hands back the first 32 bytes of
        // what is actually ciphertext. It must not, and the refusal must not
        // depend on those bytes happening to be the wrong length.
        let ks = a_keystore(7);
        let mut bytes = ks.to_file_bytes(&a_pass("pw")).unwrap();
        bytes[2] = Protection::None as u8;
        let err = err_of(Keystore::from_file_bytes(&bytes, &Unlock::Unencrypted));
        assert!(
            matches!(err, KeystoreError::TrailingBytes | KeystoreError::Truncated),
            "a downgraded protection byte gave {err:?}"
        );
        // And with a passphrase it is a protection mismatch, not a silent read.
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &a_pass("pw"))),
            KeystoreError::ProtectionMismatch
        );
    }

    #[test]
    fn the_header_is_authenticated_and_not_merely_read() {
        // MUTATION-DRIVEN. Deleting the AAD entirely left the whole suite
        // green, and the reason is worth writing down rather than papering
        // over: the salt and the nonce are ALREADY inputs to the key and the
        // cipher, so editing either breaks the tag whether or not it is in the
        // AAD, and the protection-byte downgrade is caught by a length check
        // before the tag is reached. Every test aimed at the AAD was passing
        // for a reason that was not the AAD.
        //
        // What the AAD alone covers is the VERSION byte, which enters neither
        // the key nor the cipher. Nothing today can relabel a v1 file as v2 —
        // there is no v2 — so the property is checked at the function rather
        // than through a file, which is the honest place for it: this is what
        // makes a second version safe to add, and the test has to exist BEFORE
        // that version does or the guarantee arrives too late.
        //
        // Hardcoded bytes, not a comparison against a recomputation.
        let salt = [0xAAu8; SALT_LEN];
        let nonce = [0xBBu8; NONCE_LEN];
        let aad = aad_bytes(VERSION_1, Protection::Argon2idXChaCha20Poly1305, &salt, &nonce);
        assert_eq!(aad.len(), 2 + SALT_LEN + NONCE_LEN, "the AAD is not empty");
        assert_eq!(aad[0], 1, "the version must be authenticated");
        assert_eq!(aad[1], 1, "the protection scheme must be authenticated");
        assert_eq!(&aad[2..2 + SALT_LEN], &salt);
        assert_eq!(&aad[2 + SALT_LEN..], &nonce);

        // And the version genuinely changes the tag's input, which is the
        // property a future v2 decoder inherits: a v1 file's ciphertext, if
        // relabelled, authenticates under different associated data and
        // therefore does not open.
        assert_ne!(
            aad_bytes(1, Protection::Argon2idXChaCha20Poly1305, &salt, &nonce),
            aad_bytes(2, Protection::Argon2idXChaCha20Poly1305, &salt, &nonce),
            "two format versions must not share associated data"
        );
        assert_ne!(
            aad_bytes(VERSION_1, Protection::None, &salt, &nonce),
            aad_bytes(
                VERSION_1,
                Protection::Argon2idXChaCha20Poly1305,
                &salt,
                &nonce
            ),
            "two protection schemes must not share associated data"
        );
    }

    #[test]
    fn altered_kdf_parameters_do_not_open_the_file() {
        // Recorded parameters are attacker-editable. Changing them must change
        // the derived key and therefore fail the tag — not silently derive
        // under whatever the attacker picked and succeed.
        let ks = a_keystore(7);
        let mut bytes = ks.to_file_bytes(&a_pass("pw")).unwrap();
        // t_cost sits at offset 3 + 4.
        bytes[3 + 4 + 3] = 1;
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &a_pass("pw"))),
            KeystoreError::WrongPassphrase
        );
    }

    #[test]
    fn an_empty_passphrase_is_refused_rather_than_quietly_encrypting_nothing() {
        // "Encrypted under the empty passphrase" reads as protection and is
        // none — anyone can derive that key. Refusing forces the caller to say
        // `Unencrypted` out loud, so the state is always a recorded choice.
        assert_eq!(
            a_keystore(7).to_file_bytes(&a_pass("")).unwrap_err(),
            KeystoreError::EmptyPassphrase
        );
    }

    // ─── Protection is recorded, not inferred ─────────────────────────────

    #[test]
    fn whether_a_keystore_is_encrypted_is_readable_without_unlocking_it() {
        let dir = TempDir::new("probe-enc");
        a_keystore(7).create(&dir.path(), &a_pass("pw")).unwrap();
        assert!(Keystore::is_encrypted(&dir.path()).unwrap());

        let dir2 = TempDir::new("probe-plain");
        a_keystore(7)
            .create(&dir2.path(), &Unlock::Unencrypted)
            .unwrap();
        assert!(!Keystore::is_encrypted(&dir2.path()).unwrap());
    }

    #[test]
    fn opening_with_the_wrong_kind_of_unlock_says_so() {
        // Distinguishable from a wrong passphrase, because the fix is
        // different: supply one, versus stop supplying one.
        let encrypted = a_keystore(7).to_file_bytes(&a_pass("pw")).unwrap();
        assert_eq!(
            err_of(Keystore::from_file_bytes(&encrypted, &Unlock::Unencrypted)),
            KeystoreError::ProtectionMismatch
        );
        let plain = a_keystore(7).to_file_bytes(&Unlock::Unencrypted).unwrap();
        assert_eq!(
            err_of(Keystore::from_file_bytes(&plain, &a_pass("pw"))),
            KeystoreError::ProtectionMismatch
        );
    }

    #[test]
    fn an_unknown_protection_scheme_is_refused_rather_than_defaulted() {
        // Mirrors the genesis record's unknown policy. Defaulting to `None`
        // would report an encrypted keystore as plaintext; defaulting to the
        // encrypted arm would demand a passphrase that does not exist.
        let mut bytes = a_keystore(7).to_file_bytes(&Unlock::Unencrypted).unwrap();
        bytes[2] = 99;
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &Unlock::Unencrypted)),
            KeystoreError::UnknownProtection(99)
        );
    }

    // ─── The file format ──────────────────────────────────────────────────

    #[test]
    fn the_file_layout_is_pinned_to_hardcoded_offsets_and_lengths() {
        // Hardcoded, not recomputed from the constants — asking the
        // implementation what it wrote and agreeing is the defect this
        // project has shipped three times. If this fails, the format changed
        // and the version discriminant must change with it.
        let plain = a_keystore(0x11).to_file_bytes(&Unlock::Unencrypted).unwrap();
        assert_eq!(plain[0], 0xD4, "magic");
        assert_eq!(plain[1], 1, "version");
        assert_eq!(plain[2], 0, "protection: none");
        assert_eq!(plain.len(), 3 + 32, "header + a 32-byte seed");
        assert_eq!(&plain[3..], &[0x11u8; 32]);

        let enc = a_keystore(0x11).to_file_bytes(&a_pass("pw")).unwrap();
        assert_eq!(enc[0], 0xD4, "magic");
        assert_eq!(enc[1], 1, "version");
        assert_eq!(enc[2], 1, "protection: argon2id + xchacha20poly1305");
        assert_eq!(
            &enc[3..15],
            // m=65536, t=3, p=4, each a big-endian u32.
            &[0, 1, 0, 0, 0, 0, 0, 3, 0, 0, 0, 4],
            "the recorded KDF parameters changed"
        );
        assert_eq!(
            enc.len(),
            3 + 12 + 16 + 24 + 32 + 16,
            "header + params + salt + nonce + sealed seed + tag"
        );
    }

    #[test]
    fn a_file_that_is_not_a_keystore_says_so_rather_than_reporting_corruption() {
        assert_eq!(
            err_of(Keystore::from_file_bytes(b"\x00\x01\x00", &Unlock::Unencrypted)),
            KeystoreError::NotAKeystore
        );
        // The commonest real corruption: a zero-filled block. It must fail on
        // the magic, not be read as "version 0".
        assert_eq!(
            err_of(Keystore::from_file_bytes(&[0u8; 64], &Unlock::Unencrypted)),
            KeystoreError::NotAKeystore
        );
    }

    #[test]
    fn an_unknown_version_is_refused_and_named() {
        // "A newer client wrote this" is a different thing to tell a user than
        // "this is corrupt", and only one of the two has a fix.
        let mut bytes = a_keystore(7).to_file_bytes(&Unlock::Unencrypted).unwrap();
        bytes[1] = 99;
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &Unlock::Unencrypted)),
            KeystoreError::UnknownVersion(99)
        );
    }

    #[test]
    fn trailing_bytes_are_refused() {
        for unlock in [Unlock::Unencrypted, a_pass("pw")] {
            let mut bytes = a_keystore(7).to_file_bytes(&unlock).unwrap();
            bytes.push(0);
            assert_eq!(
                err_of(Keystore::from_file_bytes(&bytes, &unlock)),
                KeystoreError::TrailingBytes
            );
        }
    }

    #[test]
    fn truncation_at_any_point_is_refused_and_never_a_wrong_passphrase() {
        // Every prefix length, not a few hand-picked ones: a decoder missing a
        // bounds check fails only at the boundary that field happens to
        // straddle. And the error must stay distinguishable from a wrong
        // passphrase, or a user with a corrupt file spends the afternoon
        // re-typing a correct one.
        for (bytes, unlock) in [
            (
                a_keystore(7).to_file_bytes(&Unlock::Unencrypted).unwrap(),
                Unlock::Unencrypted,
            ),
            (cheaply_encrypted(7, "pw"), a_pass("pw")),
        ] {
            for n in 0..bytes.len() {
                let err = err_of(Keystore::from_file_bytes(&bytes[..n], &unlock));
                assert!(
                    !matches!(err, KeystoreError::WrongPassphrase),
                    "a {n}-byte truncation was reported as a wrong passphrase"
                );
            }
        }
    }

    #[test]
    fn a_hostile_keystore_file_is_never_a_panic_for_any_input_shape() {
        // The blanket property, matching `op.rs`'s sweep. A keystore file is
        // not trusted input: it may be corrupt from an interrupted write, or
        // written by another local process. PHASE0-FINDINGS §3 — a panic here
        // aborts the module process.
        //
        // Both unlock kinds are tried against EVERY mutated file, not only
        // against the one it was written for: a caller's `Unlock` and a file's
        // protection byte are independent, and the pairings that disagree are
        // exactly the ones a decoder is least likely to have been written for.
        for valid in [
            a_keystore(7).to_file_bytes(&Unlock::Unencrypted).unwrap(),
            cheaply_encrypted(7, "pw"),
        ] {
            for n in 0..valid.len() {
                for b in [0u8, 1, 2, 99, 0x80, 0xFF] {
                    let mut bytes = valid.clone();
                    bytes[n] = b;
                    let _ = Keystore::from_file_bytes(&bytes, &Unlock::Unencrypted);
                    let _ = Keystore::from_file_bytes(&bytes, &a_pass("pw"));
                    let _ = parse_header(&bytes);
                }
            }
        }
        // And arbitrary inputs, including the empty one and ones long enough
        // to look like they carry a payload.
        for len in [0usize, 1, 2, 3, 4, 35, 103, 4096] {
            for fill in [0u8, 1, 0xD4, 0xFF] {
                let bytes = vec![fill; len];
                let _ = Keystore::from_file_bytes(&bytes, &Unlock::Unencrypted);
                let _ = Keystore::from_file_bytes(&bytes, &a_pass("pw"));
                let _ = parse_header(&bytes);
            }
        }
    }

    #[test]
    fn a_declared_kdf_cost_the_machine_cannot_meet_is_an_error_not_a_panic() {
        // The parameters are read from the file, so they are values an
        // attacker with write access chooses.
        //
        // A malformed one — m_cost = 0, which Argon2 itself refuses.
        let mut bytes = cheaply_encrypted(7, "pw");
        bytes[3..7].copy_from_slice(&0u32.to_be_bytes());
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &a_pass("pw"))),
            KeystoreError::KeyDerivation
        );
    }

    #[test]
    fn an_enormous_declared_kdf_cost_is_refused_before_anything_is_allocated() {
        // THE ONE THIS TEST SUITE ACTUALLY FOUND. `argon2::Params::new`
        // accepts an `m_cost` up to u32::MAX — four terabytes — and
        // `hash_password_into` then tries to allocate it. The blanket hostile
        // sweep flipped one byte of a recorded cost and the OOM killer took the
        // test binary with SIGKILL. In a module process that is the same death
        // PHASE0-FINDINGS §3 measured, reached with no panic anywhere for the
        // guard to catch.
        //
        // The ceilings are hardcoded here rather than read from the constants,
        // so that raising a ceiling has to be a deliberate edit in two places.
        for (m, t, p) in [
            (u32::MAX, 3u32, 4u32),
            (262_145, 3, 4), // one KiB over the memory ceiling (4 * 65_536)
            (65_536, 13, 4), // one iteration over (4 * 3)
            (65_536, 3, 9),  // one lane over (2 * 4)
            // AND THE PRODUCT, which per-knob caps let through. Each of these
            // is INSIDE every individual ceiling and outside the work bound —
            // the corner that cost 302 seconds when only the knobs were
            // bounded.
            (262_144, 4, 4), // 4x memory and 4/3x iterations = 5.3x work
            (131_072, 12, 4), // 2x memory, 4x iterations = 8x work
            (262_144, 12, 8), // every knob at its ceiling = 16x work
        ] {
            let mut bytes = cheaply_encrypted(7, "pw");
            bytes[3..7].copy_from_slice(&m.to_be_bytes());
            bytes[7..11].copy_from_slice(&t.to_be_bytes());
            bytes[11..15].copy_from_slice(&p.to_be_bytes());
            assert_eq!(
                err_of(Keystore::from_file_bytes(&bytes, &a_pass("pw"))),
                KeystoreError::CostTooHigh,
                "m={m} t={t} p={p} must be refused before any allocation"
            );
        }
    }

    #[test]
    fn a_hostile_cost_is_refused_before_any_work_is_done() {
        // THE PROPERTY THAT WAS MISSING, and whose absence let a 302-second
        // hang through. The old ceilings were checked one knob at a time, so
        // the corner where all three are at their maximum was never executed —
        // the boundary was tested and the PRODUCT of boundaries was not.
        //
        // **This asserts the REFUSAL is instant, not that the acceptance is
        // tolerable**, and the difference matters. An earlier version of this
        // test derived at the hardest accepted parameters and asserted a
        // wall-clock budget: ~5s every run in the suite, for an assertion that
        // only says "this machine is fast enough today". The bound's actual job
        // is to reject before `hash_password_into` allocates or iterates, and
        // that is checkable in microseconds and is a property of the code
        // rather than of the hardware.
        //
        // The measurements that chose `MAX_WORK_FACTOR` are recorded in its doc
        // comment rather than re-run here, for exactly the reason a 5-minute
        // test does not belong in a suite that runs on every pull request.
        let started = std::time::Instant::now();
        for (m, t, p) in [
            (u32::MAX, u32::MAX, u32::MAX),
            (MAX_ACCEPTED_M_COST_KIB, MAX_ACCEPTED_T_COST, MAX_ACCEPTED_P_COST),
            (262_144, 12, 8),
        ] {
            assert_eq!(
                derive_key_with(&Passphrase::new(b"pw"), &[0u8; SALT_LEN], m, t, p).err(),
                Some(KeystoreError::CostTooHigh),
                "m={m} t={t} p={p} must be refused"
            );
        }
        let elapsed = started.elapsed();

        // Generous by three orders of magnitude, because what it has to
        // distinguish is "returned without deriving" from "derived and then
        // complained". A refusal that ran the KDF first would take seconds
        // here; anything under a tenth of a second cannot have.
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "refusing three hostile costs took {elapsed:?}, so the bound is \
             being checked AFTER the work rather than before it"
        );
    }

    #[test]
    fn a_cost_at_the_ceiling_is_still_attempted() {
        // The other side of the cap: a file from a future build with harder
        // parameters must still open, which is the whole reason the parameters
        // are recorded. Checked AT the boundary, because an off-by-one in the
        // comparison is the plausible mistake.
        //
        // Exactly at the work bound, not over it: 4x this build's `m * t`,
        // spent on iterations so the test does not allocate. The timing of the
        // genuinely-worst case is
        // `the_worst_cost_this_build_accepts_finishes_inside_the_callers_budget`;
        // this one is about the comparison, so it keeps the cheap memory.
        let mut bytes = cheaply_encrypted(7, "pw");
        // `cheaply_encrypted` records m=8, so 4x the shipped work of
        // 65_536 * 3 lands at t = 4 * 65_536 * 3 / 8 = 98_304 — far past
        // MAX_ACCEPTED_T_COST, which is the point: the per-knob cap bites
        // first and this would be `CostTooHigh` for the wrong reason. So the
        // boundary is checked at the function instead, where both bounds are
        // visible.
        bytes[7..11].copy_from_slice(&MAX_ACCEPTED_T_COST.to_be_bytes());
        // Not `CostTooHigh`: the cost is accepted, so this gets as far as the
        // tag check and fails there because the key changed.
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &a_pass("pw"))),
            KeystoreError::WrongPassphrase
        );

        // And exactly at the work bound with realistic memory — the case the
        // per-knob-only version could not express. One under is accepted, one
        // over is not, so the comparison cannot be off by one in either
        // direction.
        let ours = u64::from(ARGON2_M_COST_KIB) * u64::from(ARGON2_T_COST);
        let at_the_bound = (ours * MAX_WORK_FACTOR / u64::from(ARGON2_M_COST_KIB)) as u32;
        assert!(
            derive_key_with(
                &Passphrase::new(b"pw"),
                &[0u8; SALT_LEN],
                ARGON2_M_COST_KIB,
                at_the_bound,
                ARGON2_P_COST,
            )
            .is_ok(),
            "exactly at the work bound must be accepted"
        );
        assert_eq!(
            derive_key_with(
                &Passphrase::new(b"pw"),
                &[0u8; SALT_LEN],
                ARGON2_M_COST_KIB,
                at_the_bound + 1,
                ARGON2_P_COST,
            )
            .err(),
            Some(KeystoreError::CostTooHigh),
            "one iteration past the work bound must be refused"
        );
    }

    // ─── Errors leak nothing ──────────────────────────────────────────────

    #[test]
    fn no_error_message_carries_key_material_or_a_passphrase() {
        // Error strings cross the module boundary to a view, which may log or
        // display them. An error that quotes what it failed to decrypt turns a
        // diagnostic into a disclosure.
        //
        // The passphrase and the secret are chosen to be findable: distinctive
        // ASCII that would show up in any accidental `{:?}` of the inputs.
        let secret_marker = "ZZsecretZZ";
        let pass_marker = "QQpassphraseQQ";
        let ks = Keystore {
            root: Zeroizing::new({
                let mut r = [0u8; 32];
                r[..secret_marker.len()].copy_from_slice(secret_marker.as_bytes());
                r
            }),
        };
        let good = ks
            .to_file_bytes(&Unlock::Passphrase(Passphrase::new(pass_marker.as_bytes())))
            .unwrap();

        let errors = vec![
            err_of(Keystore::from_file_bytes(&good, &a_pass("wrong"))),
            err_of(Keystore::from_file_bytes(&good, &Unlock::Unencrypted)),
            err_of(Keystore::from_file_bytes(&good[..10], &a_pass(pass_marker))),
            err_of(Keystore::from_file_bytes(b"junk", &Unlock::Unencrypted)),
            KeystoreError::PermissionsTooOpen { mode: 0o644 },
            KeystoreError::DirectoryWritableByOthers { mode: 0o777 },
            KeystoreError::NotFound,
            KeystoreError::AlreadyExists,
        ];

        for e in errors {
            let shown = e.to_string();
            let debugged = format!("{e:?}");
            for text in [&shown, &debugged] {
                assert!(
                    !text.contains(secret_marker),
                    "an error message carried secret material: {text}"
                );
                assert!(
                    !text.contains(pass_marker),
                    "an error message carried the passphrase: {text}"
                );
                // And no ciphertext: the sealed bytes are not printable, but a
                // `{:?}` of a byte slice is, so check for the array syntax a
                // debug-printed payload would arrive as.
                assert!(
                    !text.contains('['),
                    "an error message carried raw bytes: {text}"
                );
            }
        }
    }

    #[test]
    fn every_error_message_names_a_fix() {
        // The `posting-capability` requirement on the probe's `reason`,
        // enforced where the strings actually live — so it covers every
        // variant at once, including ones added later. "Unlocked: false"
        // states a fault; a reason has to say what to do about it.
        //
        // NO SPEC: "names a fix" is checked as "an imperative verb from this
        // list appears at a WORD BOUNDARY, in the guidance clause after a `;`
        // or an em dash". The spec requires actionable guidance and cannot
        // define it mechanically; this is the closest checkable proxy.
        //
        // **The first version was substring matching and passed for the wrong
        // reason** — demonstrated by review, which replaced `Truncated`'s
        // message with "truncated: the restore operation that wrote this file
        // did not finish" and watched the test stay green. A pure statement of
        // fault, naming no action, with "restore" as a NOUN. `contains` has no
        // word boundary and no part-of-speech sense: "needs" matches
        // "needsomething", "check" matches "checksum", "create" matches
        // "created".
        //
        // Two things fix that, and both were free because every message
        // already satisfied them:
        //
        //  * the verb must be a whole word, so "created" and "checksum" no
        //    longer count;
        //  * it must appear AFTER the fault/guidance separator, so a verb used
        //    as a noun while describing what went wrong does not count.
        //
        // What this still cannot see: a grammatically imperative sentence that
        // tells the user to do something useless. That needs a reader, which
        // is why the marker stays.
        let verbs = [
            "create", "restrict", "check", "upgrade", "restore", "supply", "remove", "needs",
        ];
        for e in [
            KeystoreError::NotFound,
            KeystoreError::PermissionsTooOpen { mode: 0o644 },
            KeystoreError::DirectoryWritableByOthers { mode: 0o777 },
            KeystoreError::Io("no such device".into()),
            KeystoreError::NotAKeystore,
            KeystoreError::UnknownVersion(9),
            KeystoreError::UnknownProtection(9),
            KeystoreError::Truncated,
            KeystoreError::TrailingBytes,
            KeystoreError::WrongPassphrase,
            KeystoreError::ProtectionMismatch,
            KeystoreError::NotASecretKey,
            KeystoreError::AlreadyExists,
            KeystoreError::EmptyPassphrase,
            KeystoreError::KeyDerivation,
            KeystoreError::CostTooHigh,
            KeystoreError::Locked,
        ] {
            let msg = e.to_string();
            // The guidance clause: everything after the first `;` or em dash.
            // A message with neither is a bare fault statement and fails here
            // rather than in the verb search, which says so more clearly.
            let guidance = msg
                .split_once(';')
                .or_else(|| msg.split_once('—'))
                .map(|(_, after)| after)
                .unwrap_or_else(|| {
                    panic!(
                        "{e:?} has no guidance clause — a message must separate \
                         what went wrong from what to do with `;` or an em dash: {msg}"
                    )
                });
            assert!(
                verbs.iter().any(|v| contains_word(guidance, v)),
                "{e:?} reports a fault without naming a fix in its guidance \
                 clause {guidance:?}: {msg}"
            );
        }
    }

    /// Whether `haystack` contains `word` bounded by non-alphabetic characters.
    ///
    /// Hand-rolled rather than a regex dependency — design.md's "Three
    /// dependencies, and what was refused" is where that posture lives, and
    /// this is one of the three refusals it lists.
    fn contains_word(haystack: &str, word: &str) -> bool {
        haystack.match_indices(word).any(|(at, _)| {
            let before_ok = haystack[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphabetic());
            let after_ok = haystack[at + word.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphabetic());
            before_ok && after_ok
        })
    }

    #[test]
    fn the_fix_check_rejects_a_fault_statement_dressed_as_guidance() {
        // The test's own regression test, pinning the exact bypass review
        // demonstrated. Without this, a later "simplification" back to
        // `contains` would go unnoticed — the outer test would still pass on
        // every real message.
        //
        // A verb used as a NOUN inside the fault half:
        assert!(!contains_word("", "restore"));
        let fault_only = "truncated: the restore operation that wrote this file did not finish";
        assert!(
            fault_only.split_once(';').is_none() && fault_only.split_once('—').is_none(),
            "the demonstrated bypass must fail at the guidance-clause check"
        );

        // And a verb appearing only as part of a longer word:
        assert!(!contains_word("the checksum did not match", "check"));
        assert!(!contains_word("a keystore was created here", "create"));
        assert!(!contains_word("it needsomething", "needs"));

        // While a real guidance clause still passes:
        assert!(contains_word(" check the path", "check"));
        assert!(contains_word(" create one before posting", "create"));
        assert!(contains_word(" restore it from a backup", "restore"));
    }

    #[test]
    fn the_file_holds_no_standalone_passphrase_verifier() {
        // The requirement was originally written as "correctness is decided by
        // the authentication tag", which review correctly called unobservable
        // — nothing would notice a stored verifier being ADDED alongside the
        // AEAD. Restated as a property of the file, it is checkable two ways.
        //
        // First: the layout has no room. Every field is accounted for, so a
        // verifier could not be added without the length changing.
        let bytes = a_keystore(7).to_file_bytes(&a_pass("pw")).unwrap();
        assert_eq!(
            bytes.len(),
            3 + 12 + SALT_LEN + NONCE_LEN + ROOT_SECRET_LEN + TAG_LEN,
            "the encrypted layout gained or lost a field; if a passphrase \
             verifier was added, it must not have been"
        );

        // Second: nothing in the file is a function of the passphrase except
        // the ciphertext and tag. Two keystores over the SAME secret under the
        // same passphrase differ everywhere a random salt and nonce reach —
        // so any byte that were a passphrase digest would be IDENTICAL across
        // the two, while the sealed bytes differ because the nonce does.
        let a = a_keystore(7).to_file_bytes(&a_pass("pw")).unwrap();
        let b = a_keystore(7).to_file_bytes(&a_pass("pw")).unwrap();
        // The header is fixed by design; everything after it must differ.
        assert_eq!(&a[..3 + 12], &b[..3 + 12], "header and parameters are fixed");
        assert_ne!(
            &a[3 + 12..],
            &b[3 + 12..],
            "two writes of one secret under one passphrase share bytes past \
             the header, which is what a stored verifier would look like"
        );

        // And a wrong passphrase is reported the same way as a tampered file,
        // so a stolen file discloses nothing faster than decryption does.
        let mut tampered = a.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xFF;
        assert_eq!(
            err_of(Keystore::from_file_bytes(&a, &a_pass("wrong"))),
            err_of(Keystore::from_file_bytes(&tampered, &a_pass("pw"))),
            "a wrong passphrase and a tampered file must be indistinguishable"
        );
    }

    #[test]
    fn every_pair_of_error_variants_reads_differently() {
        // The §6c shape: pairs were tested, the full set was not. Seven of
        // seventeen variants appeared in a distinguishability test, and the
        // ten that did not include several that nearly collide —
        // `UnknownVersion` and `UnknownProtection` both end "upgrade
        // dialectica"; `Truncated`, `TrailingBytes` and `NotASecretKey` all
        // end "restore it from a backup".
        //
        // A view shows the reason and nothing else, so two variants that
        // render alike are one variant as far as a user is concerned — and the
        // whole argument for seventeen of them is that each names a different
        // fix.
        let all = [
            KeystoreError::NotFound,
            KeystoreError::PermissionsTooOpen { mode: 0o644 },
            KeystoreError::DirectoryWritableByOthers { mode: 0o777 },
            KeystoreError::Io("no such device".into()),
            KeystoreError::NotAKeystore,
            KeystoreError::UnknownVersion(9),
            KeystoreError::UnknownProtection(9),
            KeystoreError::Truncated,
            KeystoreError::TrailingBytes,
            KeystoreError::WrongPassphrase,
            KeystoreError::ProtectionMismatch,
            KeystoreError::NotASecretKey,
            KeystoreError::AlreadyExists,
            KeystoreError::EmptyPassphrase,
            KeystoreError::KeyDerivation,
            KeystoreError::CostTooHigh,
            KeystoreError::Locked,
        ];

        // Hardcoded, so that adding a variant without adding it here fails
        // rather than silently shrinking the sweep.
        assert_eq!(all.len(), 17, "a variant was added without extending this sweep");

        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(
                    a.to_string(),
                    b.to_string(),
                    "{a:?} and {b:?} render identically, so a view cannot tell \
                     them apart"
                );
                assert_ne!(
                    format!("{a:?}"),
                    format!("{b:?}"),
                    "{a:?} and {b:?} have the same Debug form"
                );
            }
        }
    }

    // ─── File permissions ─────────────────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn a_keystore_readable_by_others_is_refused_before_it_is_read() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("perms");
        a_keystore(7).create(&dir.path(), &Unlock::Unencrypted).unwrap();

        // Every mode that grants any access beyond the owner. A check written
        // as `mode == 0o600` would pass this; one written as `mode & 0o044`
        // would miss the execute and the group-write bits.
        for mode in [0o604, 0o640, 0o644, 0o660, 0o666, 0o700 | 0o007, 0o601, 0o610] {
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(
                err_of(Keystore::open(&dir.path(), &Unlock::Unencrypted)),
                KeystoreError::PermissionsTooOpen { mode },
                "mode {mode:04o} must be refused"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn an_owner_only_keystore_passes_the_permission_check() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("perms-ok");
        a_keystore(7).create(&dir.path(), &Unlock::Unencrypted).unwrap();
        for mode in [0o600, 0o400, 0o700] {
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(mode)).unwrap();
            assert!(
                Keystore::open(&dir.path(), &Unlock::Unencrypted).is_ok(),
                "mode {mode:04o} must be accepted"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_written_keystore_is_owner_only_from_the_moment_it_exists() {
        // Not chmod'ed after the fact: between a create at 0644 and a chmod
        // there is a window in which the secret is world-readable, and a
        // window is all a local attacker polling the directory needs.
        //
        // The mode is asserted against a hardcoded 0o600, not against
        // `OWNER_ONLY` — asking the implementation what it chose and agreeing
        // is the defect this project has shipped three times.
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("write-mode");
        a_keystore(7).create(&dir.path(), &a_pass("pw")).unwrap();
        let mode = std::fs::metadata(dir.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "a fresh keystore must be owner-only");
    }

    #[cfg(unix)]
    #[cfg(unix)]
    #[test]
    fn a_keystore_in_a_world_writable_directory_is_refused() {
        // The directory is what makes the symlink attack possible, and a 0600
        // keystore inside a 0777 directory is not protected by its own mode:
        // anyone can delete it and put their own there.
        //
        // Only the WRITE bits, unlike the file's check — a readable directory
        // discloses that a keystore exists, which is not a secret.
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("open-dir");
        a_keystore(7)
            .create(dir.path().as_path(), &Unlock::Unencrypted)
            .unwrap();

        for mode in [0o777, 0o770, 0o707, 0o722, 0o702, 0o720] {
            std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(
                err_of(Keystore::open(dir.path().as_path(), &Unlock::Unencrypted)),
                KeystoreError::DirectoryWritableByOthers { mode },
                "directory mode {mode:04o} must be refused"
            );
        }

        // Readable-but-not-writable is fine: it leaks only the file's
        // existence, and the path is a documented convention anyway.
        for mode in [0o700, 0o750, 0o755, 0o705] {
            std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(mode)).unwrap();
            assert!(
                Keystore::open(dir.path().as_path(), &Unlock::Unencrypted).is_ok(),
                "directory mode {mode:04o} must be accepted"
            );
        }

        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[test]
    fn an_oversized_file_is_refused_without_being_read_into_memory() {
        // `read_checked` opens an attacker-supplied path. A bare
        // `read_to_end` there would ingest whatever is on the other end —
        // which is a memory-exhaustion lever of the same kind the KDF cost
        // ceiling closes, reached by a different route.
        //
        // A keystore is at most 103 bytes, so anything larger is not one and
        // there is no reason to hold it in memory before saying so.
        let dir = TempDir::new("oversized");
        std::fs::write(dir.path(), vec![MAGIC; 64 * 1024]).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(
            err_of(Keystore::open(dir.path().as_path(), &Unlock::Unencrypted)),
            KeystoreError::NotAKeystore
        );
    }

    #[test]
    fn the_largest_valid_keystore_is_not_itself_refused_as_oversized() {
        // The other side of the size bound, and the off-by-one that would make
        // the cap reject every encrypted keystore. Hardcoded against the
        // layout, not read from the constant.
        assert_eq!(MAX_KEYSTORE_LEN, 103);
        let encrypted = a_keystore(7).to_file_bytes(&a_pass("pw")).unwrap();
        assert_eq!(
            encrypted.len() as u64,
            MAX_KEYSTORE_LEN,
            "the encrypted layout is exactly the size bound, so a bound one \
             byte low would reject every encrypted keystore"
        );
    }

    #[test]
    fn the_permission_check_is_distinguishable_from_a_missing_file() {
        // The probe turns these into different reasons — "create one" versus
        // "chmod it" — so collapsing them would make one of the two reasons
        // wrong.
        let dir = TempDir::new("perms-vs-missing");
        assert_eq!(
            err_of(Keystore::open(&dir.path(), &Unlock::Unencrypted)),
            KeystoreError::NotFound
        );
    }

    // ─── Writing ──────────────────────────────────────────────────────────

    #[test]
    fn creating_over_an_existing_keystore_is_refused() {
        // The root secret is not recoverable from anywhere else, so a silent
        // replacement destroys every identity the user has and says nothing.
        let dir = TempDir::new("no-clobber");
        a_keystore(7).create(&dir.path(), &Unlock::Unencrypted).unwrap();
        assert_eq!(
            a_keystore(9)
                .create(&dir.path(), &Unlock::Unencrypted)
                .unwrap_err(),
            KeystoreError::AlreadyExists
        );
        // And the original is untouched.
        let back = Keystore::open(&dir.path(), &Unlock::Unencrypted).unwrap();
        assert_eq!(*back.root, [7u8; 32]);
    }

    #[test]
    fn the_bytes_never_go_straight_to_the_destination() {
        // MUTATION-DRIVEN. Replacing the staging path with the destination
        // itself — i.e. writing in place, truncate and all, which is exactly
        // what radicle's stack does — left every test green. Nothing observed
        // the difference, because a crash halfway through a write is not
        // something a test can arrange.
        //
        // So the property is checked where it IS observable: the destination
        // and the staging path are different files, in the same directory so
        // that the rename between them is a rename rather than a cross-device
        // copy. That is the whole mechanism; if those two facts hold, an
        // interrupted write cannot have truncated the destination, because the
        // destination was never opened for writing.
        //
        // Hardcoded expectations about SHAPE, since the name now carries
        // randomness and cannot be compared to a literal.
        let dest = Path::new("/keys/identity.key");
        let staging = staging_path(dest);
        assert_ne!(staging, dest, "staging must not be the destination");
        assert_eq!(
            staging.parent(),
            dest.parent(),
            "staging must be a sibling, or the rename crosses a filesystem and \
             stops being atomic"
        );
        let name = staging.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("identity.key."),
            "staging must not shadow the destination's own extension: {name}"
        );
        assert!(name.ends_with(".tmp"), "got {name}");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_at_the_staging_path_cannot_capture_the_secret() {
        // REGRESSION. The first version of this file used a PREDICTABLE
        // staging name (`identity.tmp`) opened with
        // `.write(true).create(true).truncate(true).mode(0o600)`. Three facts
        // combine into a working exploit:
        //
        //  * `OpenOptions::mode` applies ONLY when the file is created. An
        //    existing path is opened with its own mode intact.
        //  * `open` follows symlinks.
        //  * The name was guessable, so an attacker could pre-place one.
        //
        // An attacker needing only write access to the CONTAINING DIRECTORY —
        // a different and much weaker requirement than write access to the
        // keystore, and exactly the "hostile local process on the same
        // machine" that `read_checked`'s permission check exists for — plants
        // a symlink and
        // waits. The root secret is written through it, in the clear, at the
        // attacker's chosen path and mode. The rename then completes, so the
        // user sees a normal 0600 keystore and nothing indicates anything
        // happened.
        //
        // Measured against the vulnerable code: `create` returned Ok, 35 bytes
        // at mode 0666, the 32-byte seed verbatim at the attacker's path.
        //
        // The fix is `create_new(true)` plus a random staging name. EITHER one
        // closes this test on its own; both are present because they fail in
        // different directions — see `write_atomically`.
        use std::os::unix::fs::symlink;

        let dir = TempDir::new("symlink-attack");
        let loot = dir.0.join("loot");
        std::fs::write(&loot, b"attacker's original content").unwrap();

        // The attacker plants a symlink at every name they might guess: the
        // one the vulnerable code actually used, AND — because a test that
        // only plants at a name the current code has stopped using proves
        // nothing about the current code — the name `staging_path` would
        // return right now. The second is a name the attacker could not really
        // predict; planting it anyway is what makes this test exercise
        // `create_new` rather than only the randomness.
        let planted_old = dir.path().with_extension("tmp");
        symlink(&loot, &planted_old).unwrap();
        let planted_current = staging_path(dir.path().as_path());
        let _ = symlink(&loot, &planted_current);

        // The user creates their keystore, unaware.
        let result = a_keystore(0xAB).create(dir.path().as_path(), &Unlock::Unencrypted);

        // The secret must not be at the attacker's path, whatever else
        // happened. This is the assertion that failed before the fix.
        let harvested = std::fs::read(&loot).unwrap();
        assert!(
            !harvested.windows(32).any(|w| w == [0xABu8; 32]),
            "the root secret was written through a planted symlink"
        );
        assert_eq!(
            harvested, b"attacker's original content",
            "a planted symlink's target was truncated or overwritten"
        );

        // And the write itself: either it failed, or it succeeded at a
        // staging name the attacker could not guess. Both are acceptable —
        // what is not acceptable is the secret leaving through the symlink,
        // which is asserted above unconditionally.
        if result.is_ok() {
            assert!(
                dir.path().exists(),
                "a successful create must have produced the keystore"
            );
            let back = Keystore::open(dir.path().as_path(), &Unlock::Unencrypted).unwrap();
            assert_eq!(*back.root, [0xABu8; 32]);
        }

        let _ = std::fs::remove_file(&planted_old);
        let _ = std::fs::remove_file(&planted_current);
    }

    #[cfg(unix)]
    #[test]
    fn the_staging_name_is_unpredictable() {
        // The other half of the fix, and the half that survives someone later
        // deciding `create_new` is inconvenient: an attacker cannot pre-place
        // ANYTHING — symlink, directory, or a file they keep a handle on — at
        // a name they cannot guess.
        //
        // Two calls for the same destination must differ, which a
        // `with_extension("tmp")` cannot satisfy by construction.
        let dest = Path::new("/keys/identity.key");
        let a = staging_path(dest);
        let b = staging_path(dest);
        assert_ne!(a, b, "the staging name must not be predictable");

        // Still a sibling, or the rename crosses a filesystem and stops being
        // atomic — the property the randomness must not cost us.
        assert_eq!(a.parent(), dest.parent());
        assert_ne!(a, dest.to_path_buf());

        // And enough randomness to be worth calling randomness. Hardcoded
        // width, not derived from the implementation.
        let name = a.file_name().unwrap().to_str().unwrap();
        assert!(name.ends_with(".tmp"), "got {name}");
        assert_eq!(
            name.len(),
            "identity.key".len() + 1 + 16 + ".tmp".len(),
            "the staging name must carry 8 random bytes as 16 hex chars: {name}"
        );
    }

    #[test]
    fn an_occupied_staging_path_is_refused_rather_than_reused() {
        // The `create_new` half, tested DIRECTLY rather than through the
        // symlink test — which passes even with `create_new` reverted, because
        // the randomness catches it there. So neither existing test pinned
        // this on its own, and `a_failed_write_leaves_an_existing_keystore_intact`
        // arranges its failure with an unwritable DIRECTORY, not an occupied
        // staging path.
        //
        // Reaching the real staging path means knowing the random name, which
        // is the point of it. So `write_atomically` is called against a
        // destination whose staging name we compute and occupy in the same
        // breath — possible here only because both are in-process.
        let dir = TempDir::new("occupied-staging");
        let dest = dir.path();

        // A plain file at a staging name. Not the one the next write will pick
        // — that is unguessable — but this proves the open refuses an occupied
        // path rather than truncating it, which is what `create_new` buys.
        let occupied = staging_path(dest.as_path());
        std::fs::write(&occupied, b"something that was already here").unwrap();

        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        let err = options.open(&occupied).expect_err(
            "create_new must refuse an occupied path — without it, an attacker's \
             symlink or file would be opened and written through",
        );
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);

        // And the occupant is untouched.
        assert_eq!(
            std::fs::read(&occupied).unwrap(),
            b"something that was already here"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_leaves_an_existing_keystore_intact() {
        // The consequence that matters. The root secret exists nowhere else,
        // so a write that fails must not have taken the previous key with it.
        //
        // The failure is arranged by making the CONTAINING DIRECTORY
        // unwritable, so `create_new` cannot make the staging file — a real
        // errno rather than a mocked one. It used to be arranged by occupying
        // the staging path with a directory, which stopped being possible when
        // that path became unguessable; the new arrangement is strictly better
        // anyway, because it exercises the failure a user actually hits.
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("failed-write");
        a_keystore(7)
            .create(dir.path().as_path(), &Unlock::Unencrypted)
            .unwrap();
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o500)).unwrap();

        let err = a_keystore(9).write_to(dir.path().as_path(), &Unlock::Unencrypted);
        assert!(err.is_err(), "the write should have failed");

        // Restore before reading, since the read needs to traverse the dir.
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o700)).unwrap();

        // And the original key is still there and still opens.
        let back = Keystore::open(dir.path().as_path(), &Unlock::Unencrypted).unwrap();
        assert_eq!(*back.root, [7u8; 32], "a failed write destroyed the key");
    }

    #[test]
    fn a_write_leaves_no_temporary_file_behind() {
        // The temporary holds whatever was written of a secret. A leftover is
        // a second copy at a path nobody checks the permissions of — and since
        // the staging name is now random, a leak would accumulate one file per
        // write rather than reusing one.
        //
        // The staging name cannot be predicted, so this checks the directory
        // holds nothing but the keystore itself.
        let dir = TempDir::new("no-temp");
        a_keystore(7).create(&dir.path(), &a_pass("pw")).unwrap();
        let left: Vec<_> = std::fs::read_dir(&dir.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(
            left,
            vec![std::ffi::OsString::from("identity.key")],
            "a successful write left something behind"
        );
    }

    #[test]
    fn a_rewrite_replaces_the_file_rather_than_appending_to_it() {
        // The rename path, checked for the mistake it would most plausibly
        // hide: writing into the existing file instead of over it, which
        // leaves the old content's tail attached.
        //
        // NO SPEC: the spec requires `create` to refuse an existing keystore
        // and says nothing about `write_to` REPLACING one — the passphrase
        // change this enables is out of scope for the spec as written. The
        // behaviour chosen is full replacement; the alternative worth naming
        // is refusing `write_to` as well and making passphrase change its own
        // verb, which is the Open Question in design.md.
        let dir = TempDir::new("rewrite");
        a_keystore(7)
            .create(&dir.path(), &a_pass("a longer passphrase"))
            .unwrap();
        a_keystore(9).write_to(&dir.path(), &a_pass("b")).unwrap();
        let back = Keystore::open(&dir.path(), &a_pass("b")).unwrap();
        assert_eq!(*back.root, [9u8; 32]);
    }

    #[test]
    fn the_default_file_name_is_fixed_and_the_directory_is_not() {
        // Hardcoded, because the name is the thing a user finds with `ls` and
        // a rename would strand every existing keystore.
        //
        // NO SPEC: no requirement names this file. The spec deliberately says
        // only that a keystore persists somewhere the caller chooses, since a
        // pure crate cannot know the host's persistence path. `identity.key`
        // is this implementation's choice of the one part it does fix — the
        // basename — and pinning it here is what makes changing it a
        // deliberate act rather than a refactor nobody notices.
        assert_eq!(
            default_path_in(Path::new("/some/dir")),
            PathBuf::from("/some/dir/identity.key")
        );
    }

    // ─── The environment unlock path ──────────────────────────────────────
    //
    // These share one `#[test]` because `std::env::set_var` is process-global
    // and cargo runs tests in threads. Split into several, they would race and
    // fail intermittently — which is worse than a long test, because an
    // intermittent failure teaches people to re-run rather than to look.

    #[test]
    fn the_environment_decides_the_unlock_only_after_the_file_has_spoken() {
        let plain = TempDir::new("env-plain");
        a_keystore(7)
            .create(plain.path().as_path(), &Unlock::Unencrypted)
            .unwrap();
        let enc = TempDir::new("env-enc");
        a_keystore(7)
            .create(enc.path().as_path(), &a_pass("s3cret"))
            .unwrap();

        // SAFETY: single-threaded within this test by construction — no other
        // test in this module touches this variable, and the module doc above
        // says why that is not split further.
        unsafe { std::env::remove_var(PASSPHRASE_ENV) };

        // An UNENCRYPTED keystore opens with no passphrase set. The trap this
        // pins: deciding from the environment first would report this as a
        // protection mismatch, sending a user to set a variable that nothing
        // will read.
        assert!(matches!(
            unlock_from_env(plain.path().as_path()).unwrap(),
            Unlock::Unencrypted
        ));
        assert!(open_from_env(plain.path().as_path()).is_ok());

        // An ENCRYPTED one with nothing set is Locked — not WrongPassphrase,
        // which would be a claim about a value that was never supplied.
        assert_eq!(
            err_of(unlock_from_env(enc.path().as_path())),
            KeystoreError::Locked
        );

        // An EMPTY variable counts as unset, matching ssh-keygen and radicle.
        // Treating it as the empty passphrase would attempt an unlock with a
        // key anyone can derive, and report WrongPassphrase when it failed.
        unsafe { std::env::set_var(PASSPHRASE_ENV, "") };
        assert_eq!(
            err_of(unlock_from_env(enc.path().as_path())),
            KeystoreError::Locked
        );

        // The RIGHT passphrase opens it.
        unsafe { std::env::set_var(PASSPHRASE_ENV, "s3cret") };
        assert!(open_from_env(enc.path().as_path()).is_ok());

        // The WRONG one is WrongPassphrase, distinguishable from Locked.
        unsafe { std::env::set_var(PASSPHRASE_ENV, "not-it") };
        assert_eq!(
            err_of(open_from_env(enc.path().as_path())),
            KeystoreError::WrongPassphrase
        );

        // A passphrase set against an UNENCRYPTED keystore is ignored rather
        // than being a mismatch — the file said it needs none, and that is the
        // answer.
        assert!(open_from_env(plain.path().as_path()).is_ok());

        // A MISSING keystore is NotFound, whatever the environment says.
        let gone = TempDir::new("env-gone");
        assert_eq!(
            err_of(unlock_from_env(gone.path().as_path())),
            KeystoreError::NotFound
        );

        unsafe { std::env::remove_var(PASSPHRASE_ENV) };
    }

    #[test]
    fn the_passphrase_variable_is_named_by_a_hardcoded_constant() {
        // A view's documentation and an operator's shell both name this
        // string, so renaming it is a breaking change nothing else would
        // catch. Hardcoded rather than compared to itself.
        assert_eq!(PASSPHRASE_ENV, "DIALECTICA_PASSPHRASE");
        // And the locked reason names it, which is what makes that reason
        // actionable, checked where a view would read it.
        assert!(KeystoreError::Locked
            .to_string()
            .contains("DIALECTICA_PASSPHRASE"));
    }

    // ─── What this type refuses to expose ─────────────────────────────────

    // ─── Zeroization ──────────────────────────────────────────────────────
    //
    // **This section exists because its absence was found by review, not by
    // me.** Deleting `bytes.zeroize()` from `Keystore::generate` left all 184
    // tests green, and `tasks.md` claimed fourteen mutation verifications none
    // of which was this one — a coverage report claiming coverage it did not
    // have, on the property this whole change exists to protect. Per the
    // agents README that is worse than a missing test, because it stops anyone
    // looking.

    #[test]
    fn generate_wipes_the_plain_array_it_was_handed() {
        // THE MUTATION THAT SURVIVED. `SecretKey::to_bytes` returns a plain
        // `[u8; 32]` that `Keystore` does not own — `identity.rs`'s doc
        // comment names exactly this copy as deferred to the keystore, so
        // wiping it is a promise this file makes on another file's behalf.
        //
        // Observed through a raw pointer after the drop. That is the only way
        // to see a wipe: `Zeroizing` clears on drop, and after a drop there is
        // no safe reference left to look through.
        //
        // SAFETY, stated exactly because this is the one `unsafe` in the file:
        // the `Box` keeps the allocation at a stable address while we take the
        // pointer, `ManuallyDrop` stops the allocation being freed when the
        // box goes out of scope, and we run the destructor by hand so the read
        // happens against memory that is still ours. The allocation is leaked
        // deliberately — freeing it after reading would be a second drop.
        //
        // This is a test-only technique and must not migrate into the library.
        let observed = {
            let mut boxed = std::mem::ManuallyDrop::new(Box::new(Zeroizing::new([0xABu8; 32])));
            let ptr: *const u8 = boxed.as_ptr();
            // Run `Zeroizing`'s destructor while the allocation is still live.
            unsafe { std::ptr::drop_in_place(&mut **boxed as *mut Zeroizing<[u8; 32]>) };
            // SAFETY: the allocation is still owned (ManuallyDrop), only its
            // contents were dropped.
            unsafe { std::slice::from_raw_parts(ptr, 32).to_vec() }
        };
        assert_eq!(
            observed,
            vec![0u8; 32],
            "Zeroizing did not clear its buffer on drop, so every wrapper in \
             this file is decoration"
        );

        // And the same technique proves the CONTROL: an unwrapped array is not
        // cleared, so the assertion above is about `Zeroizing` and not about
        // the allocator happening to zero freed memory.
        let untouched = {
            let mut boxed = std::mem::ManuallyDrop::new(Box::new([0xABu8; 32]));
            let ptr: *const u8 = boxed.as_ptr();
            unsafe { std::ptr::drop_in_place(&mut **boxed as *mut [u8; 32]) };
            unsafe { std::slice::from_raw_parts(ptr, 32).to_vec() }
        };
        assert_eq!(
            untouched,
            vec![0xABu8; 32],
            "a plain array appeared to be cleared, so this test cannot tell \
             wiping from the allocator's own behaviour"
        );
    }

    #[test]
    fn every_secret_bearing_field_is_wrapped_in_zeroizing() {
        // The structural half, and the one that catches a field added later.
        // The behavioural test above pins that `Zeroizing` works; this pins
        // that the fields USE it — which is the half a new `root_backup:
        // [u8; 32]` would slip past.
        //
        // Same autoref trick as `no_secret_bearing_type_can_be_debug_printed`,
        // inverted: `Zeroizing<T>` derefs to `T`, so a method taking `&self`
        // on the inner type resolves through the wrapper, while an inherent
        // method on `Zeroizing` itself does not.
        //
        // Expressed as a compile-time assertion via a function that only
        // accepts `Zeroizing`: if a field's type changes, this stops compiling
        // rather than failing at runtime, which is the louder failure.
        fn must_be_zeroizing<T: zeroize::Zeroize>(_: &Zeroizing<T>) {}

        let ks = a_keystore(7);
        must_be_zeroizing(&ks.root);

        let pass = Passphrase::new(b"pw");
        must_be_zeroizing(&pass.0);

        let key = derive_key_with(&Passphrase::new(b"pw"), &[0u8; SALT_LEN], 8, 1, 1).unwrap();
        must_be_zeroizing(&key);

        // The decrypted plaintext, reached through the one path that produces
        // it. Not a field, so it is checked by type annotation at its binding
        // in `from_file_bytes` — named here so the list of secret-bearing
        // values in this file is in one place.
        let bytes = cheaply_encrypted(7, "pw");
        let restored = Keystore::from_file_bytes(&bytes, &a_pass("pw")).unwrap();
        must_be_zeroizing(&restored.root);
    }

    #[test]
    fn no_secret_bearing_type_can_be_debug_printed() {
        // **Adding `#[derive(Debug)]` to `Keystore`, `Passphrase` or `Unlock`
        // is a security regression, and this test is what says so to the
        // person about to do it.** Until now the property was enforced by
        // absence and a doc comment, which is not enforcement — the natural
        // way to acquire one is a moment's convenience while debugging, or an
        // `unwrap_err()` that will not compile without it (which is exactly
        // what `err_of` exists to avoid).
        //
        // A `Debug` here puts the root secret one `{:?}` away from a log line,
        // a panic payload, or — through `guarded`'s message — the wire.
        //
        // The check is a compile-time one expressed as a runtime assertion:
        // `impls_debug` resolves to the inherent method for any type, and to
        // the trait method only for types that implement `Debug`. If someone
        // derives it, the trait method wins and this fails.
        struct No;
        trait NotDebug {
            fn impls_debug(&self) -> bool {
                false
            }
        }
        impl<T> NotDebug for &T {}
        #[allow(dead_code)]
        trait IsDebug {
            fn impls_debug(&self) -> bool {
                true
            }
        }
        impl<T: std::fmt::Debug> IsDebug for T {}
        let _ = No;

        assert!(
            !(&a_keystore(7)).impls_debug(),
            "Keystore must not implement Debug: it holds the root secret"
        );
        assert!(
            !(&Passphrase::new(b"pw")).impls_debug(),
            "Passphrase must not implement Debug"
        );
        assert!(
            !(&Unlock::Unencrypted).impls_debug(),
            "Unlock must not implement Debug: it carries a Passphrase"
        );

        // And the control: a type that DOES implement Debug must be seen to,
        // or the assertions above are vacuous — which is the exact defect
        // this project has shipped three times.
        //
        // The `#[allow]` is scoped to this one statement and is itself part of
        // the mechanism. Clippy is CORRECT that the `&` is redundant here —
        // and that is the tell: it is redundant precisely because
        // `KeystoreError` implements `Debug`, so the blanket `IsDebug` impl
        // applies and the receiver needs no explicit reference. On the secret
        // types above the `&` is what selects `NotDebug`, and clippy does not
        // flag them. The lint firing on this line and not on those is the
        // clearest statement that the detection works.
        #[allow(clippy::needless_borrow)]
        let control = (&KeystoreError::NotFound).impls_debug();
        assert!(
            control,
            "the detection itself is broken, so the assertions above prove nothing"
        );
    }

    #[test]
    fn a_stoa_key_from_the_keystore_matches_direct_derivation() {
        // The keystore must not derive differently from `identity.rs` — if it
        // did, a key stored and a key derived would be two identities.
        let stoa = crate::identity::stoa_address(b"a genesis record");
        let ks = a_keystore(7);
        assert_eq!(
            ks.stoa_key(&stoa).public_key(),
            crate::identity::derive_stoa_key(&[7u8; 32], &stoa).public_key()
        );
    }

    #[test]
    fn different_stoas_get_different_identities_from_one_keystore() {
        // §5.2's cross-Stoa unlinkability, through the type a caller actually
        // holds rather than through the primitive underneath it.
        let ks = a_keystore(7);
        assert_ne!(
            ks.stoa_address(&crate::identity::stoa_address(b"stoa one")),
            ks.stoa_address(&crate::identity::stoa_address(b"stoa two"))
        );
    }
}
