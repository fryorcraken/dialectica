//! The keystore: a root secret on disk, and what may open it (PLAN.md §5.6).
//!
//! `identity.rs` mints and derives keys and has nowhere to put one — its own
//! doc says so. This is that place, and it is the only code in the project that
//! writes secret material to disk.
//!
//! # The bar this is held to
//!
//! §5.6 names the reference and the counter-example in the same breath: copy
//! radicle's model, and *"LEZ's own keystore is plaintext JSON at 0644
//! containing every secret, with `// TODO: Use password for storage
//! encryption`. Match the crypto, not the key handling."*
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
//! **No agent.** §5.6 names three unlock paths and this ships two — unencrypted
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
use zeroize::{Zeroize, Zeroizing};

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
/// **This cap is a security requirement, found by a test rather than
/// predicted.** The parameters are read FROM THE FILE, so they are values
/// anyone with write access to the keystore chooses — and `argon2::Params`
/// accepts an `m_cost` up to `u32::MAX`, which is a request to allocate four
/// terabytes. The blanket hostile-input sweep below flipped one byte of a
/// recorded cost and got the test binary SIGKILLed by the OOM killer; in a
/// module process that is the same abort PHASE0-FINDINGS §3 describes, reached
/// without a panic anywhere.
///
/// So a recorded cost above these is refused rather than attempted. The
/// ceilings are generous — 16x the memory this build writes, and ten times its
/// iterations — so a file from a future build with harder parameters still
/// opens, which is the portability the recording exists for. What they exclude
/// is only the range that is an allocation attack rather than a KDF.
const MAX_ACCEPTED_M_COST_KIB: u32 = 1_048_576; // 1 GiB
const MAX_ACCEPTED_T_COST: u32 = 32;
const MAX_ACCEPTED_P_COST: u32 = 16;

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
/// Named after radicle's `RAD_PASSPHRASE`, which is the model §5.6 points at.
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
/// The agent path (§5.6's third) is what closes this, and is deferred; see the
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
    if !Keystore::is_encrypted(path)? {
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
pub fn open_from_env(path: &Path) -> Result<Keystore, KeystoreError> {
    let unlock = unlock_from_env(path)?;
    Keystore::open(path, &unlock)
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
    /// **Every message names the fix**, not merely the fault. §5.6 requires it
    /// of the probe's `reason`, and the probe's reasons are these strings — so
    /// putting the guidance anywhere else would mean maintaining it twice.
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
    pub fn generate() -> Self {
        // Through `SecretKey` rather than `getrandom` directly, so there is one
        // place in this crate that decides where key entropy comes from.
        let sk = SecretKey::generate();
        let mut bytes = sk.to_bytes();
        let root = Zeroizing::new(bytes);
        // `to_bytes` handed out a plain array this type does not own — exactly
        // the copy `identity.rs` flags as deferred to here. Wipe it.
        bytes.zeroize();
        Keystore { root }
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
    /// The permission check happens **before** any content is read, which is
    /// the only ordering that means anything: checking afterwards would have
    /// already loaded a file this function is about to declare unsafe to use.
    pub fn open(path: &Path, unlock: &Unlock) -> Result<Self, KeystoreError> {
        check_permissions(path)?;
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(KeystoreError::NotFound)
            }
            Err(e) => return Err(KeystoreError::Io(e.to_string())),
        };
        Self::from_file_bytes(&bytes, unlock)
    }

    /// Whether a keystore at this path is encrypted, without unlocking it.
    ///
    /// Separate from `open` because the probe needs it: "a keystore exists and
    /// wants a passphrase you have not supplied" is a different reason from
    /// "the passphrase you supplied was rejected", and a caller cannot tell
    /// them apart by trying to open with nothing.
    pub fn is_encrypted(path: &Path) -> Result<bool, KeystoreError> {
        check_permissions(path)?;
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(KeystoreError::NotFound)
            }
            Err(e) => return Err(KeystoreError::Io(e.to_string())),
        };
        Ok(parse_header(&bytes)?.0 != Protection::None)
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
    if m_cost > MAX_ACCEPTED_M_COST_KIB
        || t_cost > MAX_ACCEPTED_T_COST
        || p_cost > MAX_ACCEPTED_P_COST
    {
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

/// Refuse a keystore any other local user can read or write.
///
/// **Refuse, not warn.** A warning on this path is a message nobody sees: the
/// module has no terminal, and its only caller is a view that would have to
/// choose to render it — which is the same reasoning that makes the probe a
/// gate rather than a hint. And the failure being refused is not hypothetical
/// damage: a secret that has been readable by every process on the machine is
/// a secret to replace, which is what the error says to do.
///
/// The check is on the permission bits only, not on ownership. Checking the
/// owner would need the process's own uid and is a different question — a file
/// owned by someone else is unreadable anyway, and the OS reports that.
#[cfg(unix)]
fn check_permissions(path: &Path) -> Result<(), KeystoreError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(KeystoreError::NotFound),
        Err(e) => return Err(KeystoreError::Io(e.to_string())),
    };
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(KeystoreError::PermissionsTooOpen { mode });
    }
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
fn check_permissions(_path: &Path) -> Result<(), KeystoreError> {
    Ok(())
}

/// Where a keystore's bytes are written before they are moved into place.
///
/// A named function rather than an inline `with_extension`, so that "the bytes
/// never go straight to the destination" is a property a test can assert on
/// instead of a line in the body of a function whose behaviour on a crash
/// cannot be observed. A mutation that wrote directly to the destination left
/// the whole suite green until this existed.
///
/// A **sibling** of the destination, not a path in the system temp directory:
/// a rename across filesystems is not a rename, it is a copy, and loses the
/// atomicity this whole arrangement is for.
fn staging_path(path: &Path) -> PathBuf {
    path.with_extension("tmp")
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
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), KeystoreError> {
    use std::io::Write;

    let tmp = staging_path(path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
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
            (1_048_577, 3, 4), // one KiB over the memory ceiling
            (65_536, 33, 4),   // one iteration over
            (65_536, 3, 17),   // one lane over
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
    fn a_cost_at_the_ceiling_is_still_attempted() {
        // The other side of the cap: a file from a future build with harder
        // parameters must still open, which is the whole reason the parameters
        // are recorded. Checked at the boundary rather than well inside it,
        // because an off-by-one in the comparison is the plausible mistake —
        // and only `t` is varied to the ceiling, since a 1 GiB Argon2 run is
        // not something a test should perform and `p` at 16 lanes would need
        // `m_cost >= 8 * p` to be a valid Argon2 parameter set at all, which
        // would drag the memory up with it.
        let mut bytes = cheaply_encrypted(7, "pw");
        bytes[7..11].copy_from_slice(&32u32.to_be_bytes());
        // Not `CostTooHigh`: the cost is accepted, so this gets as far as the
        // tag check and fails there because the key changed.
        assert_eq!(
            err_of(Keystore::from_file_bytes(&bytes, &a_pass("pw"))),
            KeystoreError::WrongPassphrase
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
        // §5.6's requirement on the probe's `reason`, enforced where the
        // strings actually live. "Unlocked: false" states a fault; a reason has
        // to say what to do about it.
        //
        // NO SPEC: "names a fix" is checked as "contains an imperative verb
        // from this list". The spec requires actionable guidance and cannot
        // define it mechanically; this is the closest checkable proxy, and it
        // fails loudly if someone adds a variant whose message is a bare
        // statement of fault.
        let verbs = [
            "create", "restrict", "check", "upgrade", "restore", "supply", "remove", "needs",
        ];
        for e in [
            KeystoreError::NotFound,
            KeystoreError::PermissionsTooOpen { mode: 0o644 },
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
            assert!(
                verbs.iter().any(|v| msg.contains(v)),
                "{e:?} reports a fault without naming a fix: {msg}"
            );
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
        // Hardcoded, not derived from `staging_path`.
        let dest = Path::new("/keys/identity.key");
        let staging = staging_path(dest);
        assert_ne!(staging, dest, "staging must not be the destination");
        assert_eq!(staging, PathBuf::from("/keys/identity.tmp"));
        assert_eq!(
            staging.parent(),
            dest.parent(),
            "staging must be a sibling, or the rename crosses a filesystem and \
             stops being atomic"
        );
    }

    #[test]
    fn a_failed_write_leaves_an_existing_keystore_intact() {
        // The consequence that matters. The root secret exists nowhere else,
        // so a write that fails must not have taken the previous key with it.
        //
        // The failure is arranged by occupying the staging path with a
        // DIRECTORY, which cannot be opened as a file and cannot be renamed
        // over — a real errno rather than a mocked one.
        let dir = TempDir::new("failed-write");
        a_keystore(7)
            .create(dir.path().as_path(), &Unlock::Unencrypted)
            .unwrap();
        std::fs::create_dir(staging_path(dir.path().as_path())).unwrap();

        let err = a_keystore(9).write_to(dir.path().as_path(), &Unlock::Unencrypted);
        assert!(err.is_err(), "the write should have failed");

        // And the original key is still there and still opens.
        let back = Keystore::open(dir.path().as_path(), &Unlock::Unencrypted).unwrap();
        assert_eq!(*back.root, [7u8; 32], "a failed write destroyed the key");

        std::fs::remove_dir(staging_path(dir.path().as_path())).unwrap();
    }

    #[test]
    fn a_write_leaves_no_temporary_file_behind() {
        // The temporary holds whatever was written of a secret. A leftover is
        // a second copy at a path nobody checks the permissions of.
        let dir = TempDir::new("no-temp");
        a_keystore(7).create(&dir.path(), &a_pass("pw")).unwrap();
        assert!(
            !dir.path().with_extension("tmp").exists(),
            "the temporary file survived a successful write"
        );
    }

    #[test]
    fn a_rewrite_replaces_the_file_rather_than_appending_to_it() {
        // The rename path, checked for the mistake it would most plausibly
        // hide: writing into the existing file instead of over it, which
        // leaves the old content's tail attached.
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
        // actionable — §5.6's requirement, checked where a view would read it.
        assert!(KeystoreError::Locked
            .to_string()
            .contains("DIALECTICA_PASSPHRASE"));
    }

    // ─── What this type refuses to expose ─────────────────────────────────

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
