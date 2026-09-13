//! Identity: keys, addresses, and the signatures that bind an op to an author.
//!
//! # The scheme, and why it is this one
//!
//! **Ed25519 with SHA-256 addresses**, chosen on this forum's own criteria.
//!
//! The alternative considered and rejected was BIP-340 Schnorr over secp256k1,
//! to match LEZ so that LEZ proof-of-holding (PLAN.md §7.2) would interoperate.
//! PLAN.md §5.4 refutes that reasoning: a claim binds to a *presenter-chosen*
//! key rather than to a matching curve, and there is no single "LEZ scheme" to
//! match in any case. With that constraint gone, the deciding criterion is key
//! derivation — see [`derive_stoa_key`] — followed by parse safety and verify
//! cost. The Cargo.toml carries the full comparison, including the honest
//! counter-argument.
//!
//! # What is deliberately not here
//!
//! **No key rotation** (§5.3). A user's per-Stoa identity is permanent: keys are
//! *derived* per Stoa, never rolled over within one. That is not an omission —
//! §5.3's argument is that a key which can be discarded at will is a key nothing
//! can be attached to, so rotation waits until standing lives on a revocable
//! credential (§5.5) rather than on a keypair. Hashing a record rather than a
//! bare key (see [`PublicKey::address`]) is what keeps that door open.
//!
//! **No key storage.** That is [`crate::keystore`]'s job — this module defines
//! the key types it hands back, and deliberately knows nothing about files,
//! passphrases or where a secret lives. The split is what keeps the signing and
//! derivation primitives testable without a filesystem.

// `Signer` is the trait behind `.sign()`. There is deliberately no `Verifier`
// import: that trait's `verify()` is the LENIENT check, and `verify_strict` is
// an inherent method on `VerifyingKey`. Importing `Verifier` would put a
// same-named, weaker `verify` in scope right beside the one call that must not
// use it — see `verify_op_bytes`.
use ed25519_dalek::Signer;
use sha2::{Digest, Sha256};

/// Domain separation for an author address.
///
/// A fixed 32 bytes, in LEZ's own style (`b"/LEE/v0.3/AccountId/Public/..."`,
/// zero-padded to 32). The length is padded rather than natural so that the
/// prefix can never vary in size — a variable-length prefix concatenated with
/// variable-length data is the classic way to make two different inputs hash
/// the same, and padding removes the question rather than arguing about it.
///
/// The version is in the string on purpose. A future scheme change mints
/// different addresses from identical keys, which is what we want: an address
/// says which rules produced it.
const AUTHOR_ADDRESS_PREFIX: &[u8; 32] = b"/dialectica/1/Address/Author\0\0\0\0";

/// Domain separation for a Stoa address. Distinct from the author prefix, so
/// no byte string is ever both a valid author address and a valid Stoa address.
const STOA_ADDRESS_PREFIX: &[u8; 32] = b"/dialectica/1/Address/Stoa\0\0\0\0\0\0";

/// Domain separation for what a signature actually covers.
///
/// Signing a bare payload is how a signature made for one purpose gets replayed
/// as another. Every signature in dialectica commits to a prefix naming its
/// purpose, so an author's signature over a post can never be presented as
/// their signature over a moderation action.
const OP_SIGNING_PREFIX: &[u8; 32] = b"/dialectica/1/Signed/Op\0\0\0\0\0\0\0\0\0";

/// HKDF salt for per-Stoa key derivation ([`derive_stoa_key`]).
///
/// Versioned, so a future derivation scheme produces different keys from the
/// same root rather than silently colliding with this one.
const STOA_KEY_SALT: &[u8] = b"/dialectica/1/Identity/Stoa";

/// HKDF salt for per-Stoa derivation that also takes a **user-chosen path**
/// ([`derive_stoa_key_at_path`]).
///
/// **Version 2, and the bump is the requirement rather than housekeeping.** The
/// `identity` capability requires that where a scheme taking a path and one
/// taking only the root and the Stoa both exist, they be distinguishable, "so
/// that one scheme's identities cannot be silently reproduced by the other".
///
/// Without the bump, path 0 would append four zero bytes to the info and HKDF
/// would produce a *different* key anyway — so the separation would hold, but by
/// an accident of the info encoding rather than by a decision. Making path 0
/// equal the two-input scheme may well be wanted one day, since it would keep
/// existing identities valid; the point of the bump is that such a change has to
/// be somebody's decision and not a collision nobody noticed.
///
/// The cost of the bump is zero today because nothing has been derived under the
/// old scheme in the field: `identity-onboarding` is the change that mints the
/// first keystore. It would not be zero later, which is why this comment exists.
const STOA_KEY_SALT_WITH_PATH: &[u8] = b"/dialectica/2/Identity/Stoa";

/// A 32-byte address: an author's, or a Stoa's.
///
/// One type for both because they are the same construction over different
/// prefixes, and because a `String` here would invite a caller to compare an
/// address to a display form. Comparison is on the bytes.
///
/// `Ord` and `Hash` are here for the store (§3.3), which keys and indexes by
/// address: they make an `Address` usable as a map key and give a deterministic
/// sort, which matters because every peer must order a rebuilt projection the
/// same way. The order itself is lexicographic over a hash and therefore
/// meaningless — do not read it as ranking anything.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Address([u8; 32]);

impl Address {
    /// An address from its 32 raw bytes.
    ///
    /// **Infallible, and that is not a gap in validation.** An address is a
    /// hash output, so every 32-byte string is a syntactically valid one —
    /// there is nothing to check that would not be a lie about what this type
    /// guarantees. What an address means is settled by re-deriving it from the
    /// record or key it names ([`PublicKey::address`], [`stoa_address`]), and a
    /// `Result` here would suggest that a successful construction had said
    /// something about that.
    ///
    /// Takes a fixed-size array rather than a slice deliberately: the length is
    /// a type constraint, so a caller holding wire bytes does its own checked
    /// conversion and this cannot be handed the wrong thing. The same reasoning
    /// as [`PublicKey::from_bytes`], one step further along — there is not even
    /// an error case left to return.
    /// `const` so that a fixed address can be a `const Address` rather than a
    /// function call. Nothing else about the constructor changes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Address(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex — the form a user copies and pastes (§4.8: "a Stoa address
    /// is a copyable string").
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse the display form back.
    ///
    /// Strict about length and about hex validity, because this parses
    /// **attacker-supplied content**: §4.8 has Stoa addresses appearing inside
    /// posts, where anything at all may show up. A lenient parser that accepted
    /// a truncated address would let two different Stoas collide in the UI.
    /// **The length is checked BEFORE the decode, and the ordering is the
    /// point.** `hex::decode` allocates `s.len() / 2` bytes from a length this
    /// function does not control: a 64 MiB hex string arriving in a request's
    /// `stoa` field built a 32 MiB `Vec` and only *then* met the "must be 32
    /// bytes" refusal. Measured, with the refusal it produced:
    /// `{"error":"stoa: address must be 32 bytes (64 hex chars), got 33554432
    /// bytes"}`.
    ///
    /// `wire::MAX_REQUEST_BYTES` also closes this, and both are kept because they
    /// are different layers: the envelope bounds what any request may cost, and
    /// this bounds what this function may allocate regardless of who calls it —
    /// including a future caller that is not a wire handler at all. The cheaper
    /// check is also the more local one.
    ///
    /// **What it deliberately does not do is reorder the error taxonomy.** The
    /// obvious spelling — `if s.len() != 64 { return WrongLength(s.len() / 2) }` —
    /// closes the allocation but also changes what a *short* junk input reports:
    /// `from_hex("nothex!!")` would become `WrongLength(4)` where it has always
    /// been `NotHex`, and `address_parsing_rejects_attacker_supplied_junk` pins
    /// that. A security fix that quietly reclassifies a caller-visible error is
    /// two changes in one diff.
    ///
    /// So the guard is on the **over-long** case only, which is the case where
    /// the allocation is the problem. Everything at or under 64 characters costs
    /// at most 32 bytes to decode, and reaches exactly the arms it always did.
    /// A `WrongLength` above the cap is reported in hex characters halved, which
    /// is what the decoded length would have been.
    pub fn from_hex(s: &str) -> Result<Self, AddressError> {
        if s.len() > 64 {
            return Err(AddressError::WrongLength(s.len() / 2));
        }
        let bytes = hex::decode(s).map_err(|_| AddressError::NotHex)?;
        let bytes: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| AddressError::WrongLength(bytes.len()))?;
        Ok(Address(bytes))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AddressError {
    NotHex,
    WrongLength(usize),
}

impl std::fmt::Display for AddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddressError::NotHex => write!(f, "address is not valid hex"),
            AddressError::WrongLength(n) => {
                write!(f, "address must be 32 bytes (64 hex chars), got {n} bytes")
            }
        }
    }
}

/// A public key: the verifying half, and what an author is known by on the wire.
///
/// An Ed25519 public key is a **compressed Edwards point** — 32 bytes, and that
/// is the whole serialised form.
#[derive(Clone)]
pub struct PublicKey(ed25519_dalek::VerifyingKey);

impl PublicKey {
    /// Parse a public key from wire bytes.
    ///
    /// **The length is enforced by the type, not by a guard**, and that is the
    /// point of the scheme choice. This takes a slice because callers hold
    /// wire bytes, converts to `&[u8; 32]` with a checked `try_into`, and hands
    /// that to a constructor whose signature cannot accept anything else.
    ///
    /// The rejected alternative made this a hazard worth naming. `k256`'s
    /// equivalent returns a `Result`, which reads as total — and then opens
    /// with `FieldBytes::from_slice(bytes)`, `generic-array`'s, which **panics**
    /// on a length mismatch. A public key arrives inside every inbound op, so
    /// that panic was remotely triggerable, and PHASE0-FINDINGS §3 measured the
    /// consequence: the module process **aborts**, the caller waits out a 20s
    /// timeout, and every later call reports MODULE_NOT_LOADED. `guarded()`
    /// does not help, because the abort happens below it.
    ///
    /// A guard would have been enough. A type that cannot express the mistake
    /// is better, because it does not depend on the next person remembering.
    /// **Low-order points are refused here**, not left to fail at verification.
    /// There are eight of them (all-zeros among them); each decompresses to a
    /// valid Edwards point, so `VerifyingKey::from_bytes` accepts all eight, and
    /// `verify_strict` then refuses every signature under them.
    ///
    /// Leaving that to verification is safe for authenticity and unsafe for
    /// everything built on top. A Stoa genesis record naming a low-order creator
    /// decodes, hashes to a stable address, and self-authenticates — producing a
    /// forum whose sole moderator (§6) can never authorise anything, which no
    /// check distinguishes from a legitimate one. Refusing at the parse is the
    /// boundary validation CLAUDE.md asks for, and it costs nothing: a key that
    /// can never verify a signature is not a key worth holding.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        let bytes: &[u8; 32] = bytes.try_into().map_err(|_| KeyError::NotAValidPublicKey)?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|_| KeyError::NotAValidPublicKey)?;
        if key.is_weak() {
            return Err(KeyError::WeakPublicKey);
        }
        Ok(PublicKey(key))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    /// This key's author address.
    ///
    /// **A record is hashed, not the bare key** — §5.1 is explicit, and the
    /// reason is forward compatibility: if rotation ever lands (§5.3), the
    /// record can grow into a key *log* and the address survives instead of
    /// every author having to migrate. It costs nothing today, and hashing the
    /// raw key would foreclose it permanently.
    ///
    /// Today the record is exactly one key, so the preimage is
    /// `prefix || 0x01 || key`. The `0x01` is the record's key count, and it is
    /// present from the first commit precisely so that a two-key record is a
    /// *different* preimage rather than an ambiguous one.
    pub fn address(&self) -> Address {
        let mut hasher = Sha256::new();
        hasher.update(AUTHOR_ADDRESS_PREFIX);
        hasher.update([1u8]);
        hasher.update(self.to_bytes());
        Address(hasher.finalize().into())
    }
}

impl std::fmt::Debug for PublicKey {
    // Derived Debug on the inner type would print the curve point's internals.
    // A public key is not secret, but it is noise in a test failure; the hex is
    // what a reader can act on.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", self.to_hex())
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for PublicKey {}

/// A secret key: the signing half.
///
/// Deliberately NOT `Clone`, `Debug` or `Serialize`. Each of those is a way a
/// secret key ends up somewhere it should not be — a log line, a JSON reply, a
/// second copy nobody tracks — and none of them is needed to sign.
///
/// **What that does not cover, said plainly so the omission is not mistaken for
/// coverage: memory.** `ed25519-dalek` zeroizes its own `SigningKey` on drop
/// (the `zeroize` feature is on by default), but [`SecretKey::to_bytes`] hands
/// out a plain `[u8; 32]` this type no longer controls, and the seed locals in
/// [`SecretKey::generate`] and [`derive_stoa_key`] are ordinary stack arrays.
///
/// **Those copies are [`crate::keystore`]'s to own, and it does** — it moves
/// what [`SecretKey::to_bytes`] returns straight into a `Zeroizing` buffer
/// rather than binding it to a plain local first, so there is no second copy
/// for anyone to forget to wipe. (It once wiped one explicitly; review found
/// that line could be deleted with no test noticing, because a stack local
/// after its function returns is not observable.)
///
/// The denials above remain about a key reaching a *log or a wire*, which is
/// the reachable threat at this layer; the lifetime of a secret in memory is
/// owned one layer up, because that is the layer that knows when a secret stops
/// being needed.
pub struct SecretKey(ed25519_dalek::SigningKey);

impl SecretKey {
    /// Generate a fresh key from the OS random source.
    ///
    /// Fills a seed and uses the infallible constructor rather than
    /// `SigningKey::generate`, which wants a `CryptoRng` that `rand_core` 0.10
    /// no longer supplies an `OsRng` for. Identical result, one dependency
    /// instead of the `rand` stack.
    ///
    /// **Fallible, because this IS reachable from a dispatch handler.**
    ///
    /// There is no safe fallback for "I could not get entropy" — continuing with a
    /// predictable key would forge every signature this identity ever makes — so
    /// the failure must stop the operation. What it must not do is `panic`.
    ///
    /// This doc comment used to say the opposite: *"not reachable from a dispatch
    /// handler — key generation happens at keystore setup, not while serving an
    /// inbound op."* That was true when it was written and `identity-onboarding`
    /// made it false. On a fresh install — the only install onboarding exists for —
    /// every `generateIdentitySlate` and `keepIdentity` call mints a master key, so
    /// every one of them reached the `expect` this function used to carry. Review
    /// found it, and found it in the worst arrangement: the mint was in the adapter,
    /// *outside* `core::guarded`, so `catch_unwind` never saw it and
    /// PHASE0-FINDINGS §3's measured consequence applied in full — the module
    /// process aborts, the caller waits out its 20s timeout, and every later call
    /// reports `MODULE_NOT_LOADED`.
    ///
    /// `SlateNonce::generate` had already taken the fallible shape for exactly this
    /// reason, and its comment cited the contrast with *this* function. Rather than
    /// update that comment to keep the asymmetry, the asymmetry is removed: both
    /// randomness calls on the onboarding path return a `Result`.
    pub fn generate() -> Result<Self, RandomnessUnavailable> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|_| RandomnessUnavailable)?;
        Ok(SecretKey(ed25519_dalek::SigningKey::from_bytes(&seed)))
    }

    /// Rebuild a key from stored bytes — what [`crate::keystore`] calls.
    ///
    /// **Every 32-byte string is a valid Ed25519 seed**, so the only failure
    /// mode is the length, and the `try_into` is what checks it. This is a real
    /// difference from the rejected scheme rather than a stylistic one: a
    /// secp256k1 scalar must land in `[1, n)`, so the equivalent needs a
    /// rejection path for a value that is the right length and still unusable.
    /// See [`derive_stoa_key`], where that difference stops being cosmetic.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        let bytes: &[u8; 32] = bytes.try_into().map_err(|_| KeyError::NotAValidSecretKey)?;
        Ok(SecretKey(ed25519_dalek::SigningKey::from_bytes(bytes)))
    }

    /// The 32-byte seed. What a keystore stores.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }

    /// Sign a message that has already been domain-separated by [`signing_digest`].
    ///
    /// Private on purpose: everything dialectica signs goes through
    /// [`sign_op_bytes`], so there is no way to reach this with an
    /// undifferentiated message.
    ///
    /// Ed25519 hashes its input internally, so the 32-byte digest is signed as
    /// a *message* rather than as a pre-hash. Domain separation is unaffected —
    /// it is already inside those 32 bytes — but the distinction is real enough
    /// that a test pins it.
    fn sign_digest(&self, digest: &[u8; 32]) -> Signature {
        Signature(self.0.sign(digest))
    }
}

/// Derive a user's signing key for one Stoa from their root secret.
///
/// **This function is why the scheme is Ed25519** (PLAN.md §5.4). A user has one
/// identity per Stoa (§5.2), so every Stoa they join needs its own keypair from
/// one root secret — deriving many keypairs from one root is a first-class
/// requirement rather than a convenience.
///
/// Here that is one HKDF-SHA512 expansion into an **infallible** constructor,
/// because every 32-byte string is a valid seed. The BIP-340 equivalent needs a
/// rejection loop for scalars outside `[1, n)`, BIP-341's conditional parity
/// negation (`seckey = n - seckey` when the point has odd y), and raw `Scalar`
/// arithmetic that `k256::schnorr` does not expose at all. Each is a place for
/// a silent bug in the identity path — one that signs happily and verifies
/// against a different key.
///
/// **What this deliberately does not provide is public derivation.** There is no
/// way to compute a Stoa key's *public* half from the root *public* key, and
/// that is the property §5.2's cross-Stoa unlinkability depends on: if there
/// were, anyone holding the root public key could link a user's pseudonyms
/// across every Stoa they participate in. SLIP-0010 declines to define
/// non-hardened derivation for Ed25519 for cryptographic reasons; here that
/// refusal is exactly what is wanted.
///
/// **The result must be stable for the lifetime of the identity.** §5.2 makes
/// the per-Stoa pseudonym permanent, and §4.1 ties the SDS `senderId` to it —
/// which SDS assumes is immutable. So the same `(root, stoa)` must always yield
/// the same key: no session counter, no time input, nothing that varies between
/// runs. That is why this takes exactly two arguments.
///
/// The salt is a version string, so a future derivation scheme yields different
/// keys from the same root rather than colliding with this one.
pub fn derive_stoa_key(root: &[u8; 32], stoa: &Address) -> SecretKey {
    let hk = hkdf::Hkdf::<sha2::Sha512>::new(Some(STOA_KEY_SALT), root);
    let mut seed = [0u8; 32];
    hk.expand(stoa.as_bytes(), &mut seed)
        .expect("32 bytes is far below HKDF-SHA512's output limit");
    SecretKey(ed25519_dalek::SigningKey::from_bytes(&seed))
}

/// Derive a signing key for one Stoa from a root secret **and a chosen path**.
///
/// This is [`derive_stoa_key`] with the third input the `identity` capability
/// now admits, and the one `identity-onboarding` selects between: a user is
/// offered several candidates for a Stoa and they differ by this value alone.
///
/// # This is the same scheme with one more input, not a second scheme
///
/// One HKDF-SHA512 expansion, the same infallible `from_bytes`, the same refusal
/// of public derivation. `identity-onboarding` requires that derivation "remain
/// that of the `identity` capability" and forbids introducing a second scheme,
/// so everything [`derive_stoa_key`]'s doc comment argues applies here unchanged
/// — including why Ed25519 was chosen for exactly this operation.
///
/// **The salt differs**, and that is the one deliberate divergence. See
/// [`STOA_KEY_SALT_WITH_PATH`]: it is what keeps this scheme's path-0 identity
/// distinct from the two-input scheme's, which the spec requires.
///
/// # The recomputability guarantee is narrower, and this is where it narrows
///
/// [`derive_stoa_key`] makes an identity reproducible from the root and the Stoa
/// address alone, so nothing needs backing up beyond the root and the list of
/// Stoas joined. **A user-chosen path is a third input that no value on the
/// network carries**, so an unrecorded path is an identity that cannot be
/// reproduced from any surviving material. `identity-onboarding` moves the
/// guarantee onto the recorded path and requires the record be stored; see
/// [`crate::identity_store`].
///
/// That is a real cost, accepted in exchange for the user getting a choice. It is
/// stated here rather than only in the spec because this function is where a
/// reader meets the third argument and asks what it costs.
///
/// # The path's encoding
///
/// `info = stoa || path.to_be_bytes()`. Unambiguous without a length prefix
/// because an [`Address`] is a fixed 32 bytes — there is no variable-length
/// concatenation here, which is the hazard the fixed-width prefixes elsewhere in
/// this file exist to avoid. Big-endian so the bytes read in the order the
/// number is written, which matters only for a human comparing a test vector.
///
/// A `u32` rather than a BIP-32 path string: what onboarding offers is an index,
/// and a string would be a parser meeting caller input for no present gain.
/// Accepting a string later is additive.
pub fn derive_stoa_key_at_path(root: &[u8; 32], stoa: &Address, path: u32) -> SecretKey {
    let hk = hkdf::Hkdf::<sha2::Sha512>::new(Some(STOA_KEY_SALT_WITH_PATH), root);
    let mut info = [0u8; 36];
    info[..32].copy_from_slice(stoa.as_bytes());
    info[32..].copy_from_slice(&path.to_be_bytes());
    let mut seed = [0u8; 32];
    hk.expand(&info, &mut seed)
        .expect("32 bytes is far below HKDF-SHA512's output limit");
    SecretKey(ed25519_dalek::SigningKey::from_bytes(&seed))
}

/// The OS random source was unavailable.
///
/// Its own type rather than a [`KeyError`] arm, because it is not a fact about a
/// key: every other arm there says "these bytes are not the thing you claimed",
/// and this says "the machine could not give me entropy". Collapsing them would
/// make `NotAValidSecretKey` mean two things, one of which is not about the
/// secret key at all.
///
/// A unit struct rather than an enum with one arm: there is one way for this to
/// happen and nothing to carry. `getrandom`'s own error is deliberately not
/// wrapped — it names an OS errno that no caller of this can act on differently,
/// and the fix ("the OS random source is unavailable") is the same in every case.
#[derive(Debug, PartialEq, Eq)]
pub struct RandomnessUnavailable;

impl std::fmt::Display for RandomnessUnavailable {
    /// Names the fix, following `KeystoreError::Display`'s documented obligation —
    /// this string can reach a view through a slate or keep reply.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the operating system's random source is unavailable, so no key can be \
             minted; check that /dev/urandom is reachable and that no sandbox policy \
             is blocking getrandom"
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum KeyError {
    NotAValidPublicKey,
    /// A low-order point. Well-formed, and able to verify nothing — kept
    /// distinct from `NotAValidPublicKey` because "this is not a key" and "this
    /// is a key that can never work" send a reader to different places.
    WeakPublicKey,
    NotAValidSecretKey,
    NotAValidSignature,
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::NotAValidPublicKey => write!(f, "not a valid public key"),
            KeyError::WeakPublicKey => {
                write!(
                    f,
                    "low-order public key, which can never verify a signature"
                )
            }
            KeyError::NotAValidSecretKey => write!(f, "not a valid secret key"),
            KeyError::NotAValidSignature => write!(f, "not a valid signature"),
        }
    }
}

/// A 64-byte Ed25519 signature — `R` and `s`, 32 bytes each.
#[derive(Clone, PartialEq, Eq)]
pub struct Signature(ed25519_dalek::Signature);

impl Signature {
    /// Parse a 64-byte signature from wire bytes.
    ///
    /// Like [`PublicKey::from_bytes`], the length is a type constraint rather
    /// than a guard, and the rejected scheme is why that is worth stating:
    /// `k256`'s `TryFrom<&[u8]> for Signature` returns a `Result` and then
    /// opens with `bytes.split_at(BYTE_SIZE / 2)`, which panics `mid > len` on
    /// anything shorter than 32 bytes.
    ///
    /// Every signature on the wire reaches this function, so that was the
    /// second remotely-triggerable abort in as many types. Here the constructor
    /// takes `&[u8; 64]` and cannot be handed the wrong thing.
    ///
    /// **Any 64 bytes parse.** Ed25519 defers every validity question to
    /// verification, so a successful parse means "this is 64 bytes", not "this
    /// is a good signature". That is the correct split — [`verify_op_bytes`] is
    /// where a signature is judged — but it does mean this returning `Ok` says
    /// nothing about authenticity.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        let bytes: &[u8; 64] = bytes.try_into().map_err(|_| KeyError::NotAValidSignature)?;
        Ok(Signature(ed25519_dalek::Signature::from_bytes(bytes)))
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signature({})", self.to_hex())
    }
}

/// What a signature over an op actually commits to.
///
/// `SHA256(OP_SIGNING_PREFIX || bytes)`. The prefix separates a dialectica op
/// signature from a signature over anything else — another protocol reusing
/// these keys, or a future non-op thing this project signs. That is what it
/// buys, and it is worth having.
///
/// **What it does NOT buy, because there is one prefix for all ops:**
/// separation between op *kinds*. A post and a moderation action both go
/// through here, so a signature is not intrinsically bound to which sort of op
/// it authorises — that separation has to come from the canonical bytes being
/// unambiguously typed.
///
/// **That is now supplied.** [`crate::op::Op::canonical_bytes`] puts the op
/// kind in the second byte of every preimage, so two ops of different kinds
/// cannot encode alike and no signature over one is a signature over the
/// other. The property is structural rather than a convention the encoder must
/// maintain, which is what this comment previously said was owed.
/// `pub(crate)`, deliberately. This is the raw value a signature is made over,
/// and exporting it is an invitation to hand-roll a verification path — which
/// is exactly the split [`verify_authored_op`] exists to prevent, since the
/// step that goes missing is always the address binding. Callers outside this
/// crate get [`sign_op_bytes`] and [`verify_authored_op`]; §2.5 says widening
/// the surface is a deliberate act, and this does not need widening.
pub(crate) fn signing_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(OP_SIGNING_PREFIX);
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Sign an op's canonical bytes.
pub fn sign_op_bytes(key: &SecretKey, bytes: &[u8]) -> Signature {
    key.sign_digest(&signing_digest(bytes))
}

/// Verify a signature over an op's canonical bytes.
///
/// # This proves possession of a key, not identity
///
/// A `true` here means "whoever holds this key's secret signed these bytes" and
/// says **nothing about who they are**. An inbound op arrives with a claimed
/// author, and binding the key to that claim is a separate step —
/// [`verify_authored_op`] is the ingest path's entry point and does both. Reach
/// for this one only when the key is already known to be the right key.
///
/// Returns a plain `bool`. There is exactly one thing a caller may do with a
/// bad signature — drop the op (§3.3: "the store may hold junk; the reader
/// never trusts it") — so distinguishing *why* it failed would offer a choice
/// that does not exist, and inviting a caller to branch on it is how a
/// "recoverable" verification failure becomes an accepted op.
///
/// **`verify_strict`, never `verify`, and this is a correctness requirement
/// rather than belt-and-braces.** The difference is narrower than it is often
/// described, and worth stating exactly: in dalek 3 it is *only* the small-order
/// check on `R` and on `A` (`verifying.rs:380`). It is **not** the
/// cofactored/cofactorless axis — both verifiers are cofactorless — and it is
/// not canonicality: non-canonical `R` is rejected by both (upstream says so at
/// `verifying.rs:501`), and non-canonical `A` by neither.
///
/// Every peer verifies independently (§3.3, §6), so all peers must run the
/// *same* predicate: two on different rules would disagree about whether the
/// same op is validly signed, which is a partition in a system whose whole
/// moderation story rests on peers reaching the same verdict from the same
/// bytes. The implication runs one way — plain `verify` accepts a strict
/// superset, so a peer calling it admits ops the others reject.
///
/// The same reasoning is why `verify_batch` is not used and the `batch` feature
/// is off: it does not perform the strict check, so batching would reintroduce
/// exactly this divergence, and a batch failure does not say which signature
/// failed — which is a denial-of-service lever when the inputs are hostile.
pub fn verify_op_bytes(key: &PublicKey, bytes: &[u8], signature: &Signature) -> bool {
    key.0
        .verify_strict(&signing_digest(bytes), &signature.0)
        .is_ok()
}

/// Verify an op that arrived over the wire, as bytes, claiming an author.
///
/// **This is the function the ingest path should call, and the reason it exists
/// is the address check.** Verifying a signature under a key proves that
/// whoever holds that key's secret signed the bytes — and *nothing about who
/// they are*. An attacker can generate a key, sign anything with it, and attach
/// any author address they like; only re-deriving the address from the key
/// catches that. §5.1's record-hashed address is what makes the check possible,
/// and this is where it gets made.
///
/// Splitting the steps across a caller is how that check goes missing: parse
/// key, parse signature, verify, and the one line that binds the key to the
/// claimed identity is the easiest of the four to forget, because the other
/// three are visibly load-bearing and this one looks like bookkeeping. CLAUDE.md
/// puts it as "a guard is a job — keep it separate, so 'is it called
/// everywhere?' stays a question with an answer." One function is that answer.
///
/// Takes raw bytes rather than parsed types because raw bytes are what a caller
/// has: every one of these fields arrives inside an inbound op, all of them
/// attacker-controlled. Malformed input of any shape is a `false`, never a
/// panic.
///
/// Returns a plain `bool`, for the same reason [`verify_op_bytes`] does — the
/// only thing to do with a bad op is drop it (§3.3).
pub fn verify_authored_op(
    author: &Address,
    key_bytes: &[u8],
    op_bytes: &[u8],
    signature_bytes: &[u8],
) -> bool {
    let Ok(key) = PublicKey::from_bytes(key_bytes) else {
        return false;
    };
    // Before checking the signature at all: does this key even belong to the
    // author being claimed? A valid signature by the wrong key is exactly the
    // forgery this rejects, and doing it first means an attacker cannot spend
    // our verification time on a key that was never going to be accepted.
    if key.address() != *author {
        return false;
    }
    let Ok(signature) = Signature::from_bytes(signature_bytes) else {
        return false;
    };
    verify_op_bytes(&key, op_bytes, &signature)
}

/// A Stoa's address, derived from its genesis record's canonical bytes.
///
/// This is what makes a pasted Stoa address **self-authenticating** (§4.8):
/// re-deriving it from the record you received either reproduces the address
/// you were given or it does not, so a tampered record cannot masquerade as the
/// Stoa you meant to join. No registry is consulted, which is what keeps
/// permissionless creation (§1) from needing one.
///
/// **The caller owns canonicalisation.** "The record's canonical bytes" is an
/// obligation this function cannot discharge: it hashes whatever it is given, so
/// two encodings of the same logical record yield two different addresses.
///
/// [`crate::stoa::Genesis::address`] is the entry point that discharges it, and
/// it is the one to prefer — going through the record makes hashing a
/// non-canonical encoding impossible by accident. This byte-oriented primitive
/// stays for callers that already hold canonical bytes.
pub fn stoa_address(genesis_bytes: &[u8]) -> Address {
    let mut hasher = Sha256::new();
    hasher.update(STOA_ADDRESS_PREFIX);
    hasher.update(genesis_bytes);
    Address(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh key, or a test failure.
    ///
    /// `SecretKey::generate` returns a `Result` because it is reachable from a
    /// dispatch handler, where a panic aborts the module process. In a test a panic
    /// IS the reporting mechanism, and a machine with no working random source
    /// cannot run this suite meaningfully anyway — so the unwrap is concentrated
    /// here rather than repeated at twenty call sites, where it would be twenty
    /// chances to write something subtler than "this cannot fail in a test".
    fn a_fresh_key() -> SecretKey {
        SecretKey::generate().expect("a test host has a working random source")
    }

    #[test]
    fn minting_a_key_is_fallible_rather_than_a_panic() {
        // The regression test for the review finding that `SecretKey::generate`'s
        // `expect` became reachable from a dispatch handler while its doc comment
        // still said it was not. A panic on a handler path aborts the module process
        // (PHASE0-FINDINGS §3), which is a dead module rather than a failed call.
        //
        // What is checkable is the SHAPE, not the failure: `getrandom` cannot be made
        // to fail from a test without a seccomp sandbox, and a test that installed
        // one would be testing the sandbox. So this pins that the signature is
        // fallible and that its error names a fix — the two things that would have to
        // be undone to reintroduce the panic.
        //
        // A `Result` is what makes the fix structural: reverting to `-> Self` is a
        // compile error at `Keystore::generate`, not a silent change of failure mode.
        let minted: Result<SecretKey, RandomnessUnavailable> = SecretKey::generate();
        assert!(
            minted.is_ok(),
            "a test host must have a working random source"
        );

        // The message names the fix rather than only the fault, which is what lets it
        // reach a view through a slate or keep reply and still be actionable.
        let reason = RandomnessUnavailable.to_string();
        assert!(
            reason.contains("getrandom") && reason.contains("check"),
            "the randomness failure must name what to check: {reason}"
        );
        // And it converts into the keystore's own error, so the handler path has one
        // error type rather than two.
        assert_eq!(
            crate::keystore::KeystoreError::from(RandomnessUnavailable),
            crate::keystore::KeystoreError::NoRandomness
        );
    }

    #[test]
    fn a_signature_verifies_against_its_own_key() {
        let sk = a_fresh_key();
        let sig = sign_op_bytes(&sk, b"an op");
        assert!(verify_op_bytes(&sk.public_key(), b"an op", &sig));
    }

    #[test]
    fn a_signature_does_not_verify_against_a_different_key() {
        // The property moderation rests on (§6): a forged op is one signed by
        // somebody who is not who they claim to be.
        let author = a_fresh_key();
        let impostor = a_fresh_key();
        let sig = sign_op_bytes(&impostor, b"an op");
        assert!(!verify_op_bytes(&author.public_key(), b"an op", &sig));
    }

    #[test]
    fn a_signature_does_not_verify_over_different_bytes() {
        // Tamper with the op and the signature must stop matching, or "signed"
        // means nothing.
        let sk = a_fresh_key();
        let sig = sign_op_bytes(&sk, b"an op");
        assert!(!verify_op_bytes(&sk.public_key(), b"a different op", &sig));
    }

    #[test]
    fn signing_is_domain_separated_from_a_bare_digest() {
        // A signature must commit to what it is FOR. If the op prefix were
        // dropped, a signature over some other protocol's SHA-256 of the same
        // bytes would verify here — which is the replay this prefix prevents.
        let bare = {
            let mut h = Sha256::new();
            h.update(b"an op");
            let out: [u8; 32] = h.finalize().into();
            out
        };
        assert_ne!(
            signing_digest(b"an op"),
            bare,
            "the signing digest must not be a plain hash of the payload"
        );
    }

    #[test]
    fn the_wire_constants_are_pinned_to_known_answers() {
        // EVERY constant in this file is consensus-critical: change one byte of
        // a prefix, the record count, or the HKDF salt, and every address and
        // signature this peer produces stops matching everyone else's — with no
        // error anywhere, because each peer is internally consistent.
        //
        // Every other test here is self-consistent and would pass unchanged if
        // someone edited a prefix string. This one would not. That is its whole
        // job, and it is why the expected values are hardcoded hex rather than
        // recomputed from the constants.
        //
        // If this fails, do NOT update the expected values to match. Work out
        // what changed and whether the network can survive it.
        let sk = SecretKey::from_bytes(&[7u8; 32]).unwrap();
        assert_eq!(
            sk.public_key().address().to_hex(),
            "f875158a79d255a6dd83307d5918298cd819eafb8218ef14cd34ebf2bc385ef4",
            "author address derivation changed"
        );
        assert_eq!(
            stoa_address(b"a genesis record").to_hex(),
            "6b1f1c28061e99c72e3340fb4fd07e8192b394e1327012e140f240a990d89cd8",
            "Stoa address derivation changed"
        );
        assert_eq!(
            hex::encode(signing_digest(b"an op")),
            "c36ef2e92cbeb698b11f9acb8526252e9b5a6e1cadd47b9dcdd4126f55e5f83a",
            "op signing digest changed"
        );
        assert_eq!(
            hex::encode(derive_stoa_key(&[7u8; 32], &stoa_address(b"a genesis record")).to_bytes()),
            "b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf",
            "per-Stoa key derivation changed"
        );

        // The path-taking scheme, pinned the same way and for the same reason.
        //
        // Both values below were computed with OpenSSL's own HKDF rather than by
        // reading back what this code produced:
        //
        //   openssl kdf -keylen 32 -kdfopt digest:SHA512 \
        //     -kdfopt hexkey:<root> -kdfopt hexsalt:<salt> \
        //     -kdfopt hexinfo:<stoa||path> HKDF
        //
        // That invocation was FIRST validated by reproducing the version-1 value
        // above exactly, which is what makes these two trustworthy rather than
        // merely plausible. A value read back from this implementation would be
        // the implementation agreeing with itself — the defect this whole test
        // exists to avoid.
        //
        // Path 1 and path 0 are both pinned. Path 0 is the interesting one: it is
        // where the version-1 and version-2 schemes would collide if the salt
        // bump were ever reverted, and `the_path_taking_scheme_does_not_collide_
        // with_the_scheme_without_one` is the test that notices.
        let pinned_stoa = stoa_address(b"a genesis record");
        assert_eq!(
            hex::encode(derive_stoa_key_at_path(&[7u8; 32], &pinned_stoa, 1).to_bytes()),
            "b10513080c36903e20a08c3e4f114603dc9cd6fb57d247776312ada32322d5ef",
            "path-taking per-Stoa key derivation changed"
        );
        assert_eq!(
            hex::encode(derive_stoa_key_at_path(&[7u8; 32], &pinned_stoa, 0).to_bytes()),
            "45bf5b4ecdb032428e71e9079b09c3c793663813940d87555bad1ed3072f2fe8",
            "path-taking per-Stoa key derivation at path 0 changed"
        );
    }

    #[test]
    fn different_keys_get_different_addresses() {
        let a = a_fresh_key();
        let b = a_fresh_key();
        assert_ne!(a.public_key().address(), b.public_key().address());
    }

    #[test]
    fn an_author_address_is_not_a_bare_hash_of_the_key() {
        // §5.1 is explicit that a RECORD is hashed, not the raw key, so that a
        // key log can be added later without changing anybody's address. This
        // is the test that stops someone "simplifying" that away — it is the
        // only thing distinguishing the two, since both produce 32 plausible
        // bytes.
        let sk = a_fresh_key();
        let bare = {
            let mut h = Sha256::new();
            h.update(sk.public_key().to_bytes());
            let out: [u8; 32] = h.finalize().into();
            out
        };
        assert_ne!(sk.public_key().address().as_bytes(), &bare);
    }

    #[test]
    fn an_author_address_and_a_stoa_address_never_collide() {
        // The prefixes are what separate the two derivations, so the test has
        // to hold everything else equal: feed `stoa_address` the EXACT preimage
        // that `address()` hashes internally (`0x01 || key`), and the only
        // remaining difference is the prefix. Comparing two hashes of unrelated
        // inputs would pass whether or not the prefixes differed, which is the
        // trap here — it looks like a test and proves nothing.
        //
        // Without this, one byte string could be valid as both an author and a
        // Stoa address, and either could be presented as the other.
        let sk = a_fresh_key();
        let mut author_preimage = vec![1u8];
        author_preimage.extend_from_slice(&sk.public_key().to_bytes());
        assert_ne!(
            sk.public_key().address(),
            stoa_address(&author_preimage),
            "author and Stoa derivations must be domain-separated"
        );
    }

    #[test]
    fn a_stoa_address_changes_if_the_genesis_record_changes() {
        // The self-authenticating property (§4.8): a tampered record cannot
        // reproduce the address it claims to be.
        assert_ne!(stoa_address(b"record one"), stoa_address(b"record two"));
    }

    #[test]
    fn an_address_survives_a_hex_round_trip() {
        let sk = a_fresh_key();
        let addr = sk.public_key().address();
        assert_eq!(Address::from_hex(&addr.to_hex()).unwrap(), addr);
    }

    #[test]
    fn address_parsing_rejects_attacker_supplied_junk() {
        // §4.8 puts addresses inside posts, so this parser meets hostile input
        // by design. A truncated address that parsed would let two Stoas
        // collide in the UI.
        assert_eq!(Address::from_hex("nothex!!"), Err(AddressError::NotHex));
        assert_eq!(Address::from_hex(""), Err(AddressError::WrongLength(0)));
        assert_eq!(Address::from_hex("00ff"), Err(AddressError::WrongLength(2)));
        // 33 bytes — one too many, the case a length check written as `>=` or a
        // truncating copy would wave through.
        let too_long = "ab".repeat(33);
        assert_eq!(
            Address::from_hex(&too_long),
            Err(AddressError::WrongLength(33))
        );
    }

    #[test]
    fn an_over_long_hex_address_is_refused_before_it_is_decoded() {
        // `hex::decode` allocates `s.len() / 2` bytes from a length this parser
        // does not control. §4.8 puts addresses inside posts and a request's
        // `stoa` field carries one, so that length is attacker-supplied — a
        // 64 MiB hex string built a 32 MiB `Vec` and only then met the "must be
        // 32 bytes" refusal.
        //
        // What makes this a real assertion rather than a restatement of the
        // existing `WrongLength` test: the input is over-long AND not valid hex.
        // Only an implementation that checks the length BEFORE decoding can
        // answer `WrongLength`; one that decodes first answers `NotHex`, because
        // the decode fails before any length is compared. Swap the two and this
        // is the test that goes red.
        let over_long_and_not_hex = "z".repeat(1024);
        assert_eq!(
            Address::from_hex(&over_long_and_not_hex),
            Err(AddressError::WrongLength(512)),
            "the length must be checked before the decode allocates"
        );

        // And the boundary from both sides, so a `>` written as `>=` is caught:
        // 64 characters is the legitimate length and must still reach the decode.
        let exactly_64_not_hex = "z".repeat(64);
        assert_eq!(
            Address::from_hex(&exactly_64_not_hex),
            Err(AddressError::NotHex),
            "a 64-character input must still be decoded, so junk in it is NotHex"
        );
    }

    #[test]
    fn a_right_length_public_key_that_is_not_a_point_is_rejected() {
        // The rejection path an attacker actually controls. Getting the LENGTH
        // right is trivial in a real op — the interesting case is 32 bytes that
        // are not a decompressable Edwards point, which is the `map_err` arm in
        // `PublicKey::from_bytes` that the wrong-length tests never reach.
        //
        // y = 2 (little-endian, sign bit clear). Solving the curve equation for
        // that y gives an x² with no square root mod p, so decompression fails.
        // Checked with an independent computation rather than guessed — the
        // first candidate tried here, 0xFF..FF, turned out to decode.
        let not_a_point = {
            let mut b = [0u8; 32];
            b[0] = 2;
            b
        };
        assert!(PublicKey::from_bytes(&not_a_point).is_err());

        // And through the wire-level entry point, where it must be `false`
        // rather than a panic.
        let sk = a_fresh_key();
        let sig = sign_op_bytes(&sk, b"a post");
        assert!(!verify_authored_op(
            &sk.public_key().address(),
            &not_a_point,
            b"a post",
            &sig.to_bytes()
        ));
    }

    #[test]
    fn a_public_key_survives_a_byte_round_trip() {
        let sk = a_fresh_key();
        let pk = sk.public_key();
        assert_eq!(PublicKey::from_bytes(&pk.to_bytes()).unwrap(), pk);
    }

    #[test]
    fn a_secret_key_survives_a_byte_round_trip() {
        // What the keystore does on unlock: bytes in, same identity out.
        let sk = a_fresh_key();
        let restored = SecretKey::from_bytes(&sk.to_bytes()).unwrap();
        assert_eq!(restored.public_key(), sk.public_key());
    }

    #[test]
    fn a_signature_survives_a_byte_round_trip() {
        // Signatures cross the wire as 64 bytes, so this is the path every
        // inbound op takes.
        let sk = a_fresh_key();
        let sig = sign_op_bytes(&sk, b"an op");
        let restored = Signature::from_bytes(&sig.to_bytes()).unwrap();
        assert!(verify_op_bytes(&sk.public_key(), b"an op", &restored));
    }

    #[test]
    fn malformed_key_and_signature_bytes_are_rejected_rather_than_panicking() {
        // These parse attacker-supplied bytes off the wire, so the contract is
        // that a wrong length is an ERROR and never an abort. This test is what
        // caught two remotely-triggerable panics in the previously-chosen
        // scheme, where `Result`-returning parsers panicked on a short slice.
        //
        // Under Ed25519 the constructors take fixed-size arrays, so these now
        // pass by construction rather than by a guard someone remembered. The
        // test is KEPT anyway: it is the regression evidence, and it is what
        // would fail if anyone reintroduced a slice-taking parser.
        for n in [0usize, 31, 33, 64] {
            assert!(
                PublicKey::from_bytes(&vec![0u8; n]).is_err(),
                "a {n}-byte public key must be rejected"
            );
            assert!(
                SecretKey::from_bytes(&vec![0u8; n]).is_err(),
                "a {n}-byte secret key must be rejected"
            );
        }
        for n in [0usize, 63, 65] {
            assert!(
                Signature::from_bytes(&vec![0u8; n]).is_err(),
                "a {n}-byte signature must be rejected"
            );
        }
    }

    #[test]
    fn an_all_zero_secret_key_is_accepted_because_every_seed_is_valid() {
        // Deliberately pinned, because it is a real behaviour difference from
        // the rejected scheme and the sort of thing a reader would otherwise
        // assume was an oversight. A secp256k1 scalar must land in `[1, n)`, so
        // all-zero is invalid there; an Ed25519 secret key is a SEED that gets
        // hashed, so every 32-byte string is valid — including this one.
        //
        // It is not a weakness: the seed is hashed and clamped internally, and
        // an attacker who can choose your seed has already won. What matters is
        // that `from_bytes` reports length problems and nothing else, which is
        // what the test above asserts.
        assert!(SecretKey::from_bytes(&[0u8; 32]).is_ok());
    }

    #[test]
    fn verification_refuses_a_low_order_key_obtained_around_the_parse() {
        // Defence in depth, and NOT redundant with the parse guard below: that
        // one pins that a low-order key cannot be built through `from_bytes`,
        // this one that verification refuses it even when one is held. A future
        // third `PublicKey` construction site would reopen exactly this door,
        // and only this test would notice.
        //
        // **What this does NOT pin is `verify_strict` versus `verify`.** Both
        // refuse every input reachable here, so swapping the call leaves this
        // green — measured, not assumed. Separating them needs a crafted
        // small-order forgery, which is fiddly enough to be absent; the guard
        // against that swap is `verify_op_bytes`'s doc comment and the
        // deliberately-absent `Verifier` import, not a test. Said plainly so
        // nobody reads this as cover it does not provide.
        let low_order = PublicKey(
            ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32])
                .expect("the all-zero point decompresses; that is what makes it dangerous"),
        );

        let zero_seed = SecretKey::from_bytes(&[0u8; 32]).unwrap();
        let self_sig = sign_op_bytes(&zero_seed, b"an op");
        assert!(
            !verify_op_bytes(&low_order, b"an op", &self_sig),
            "a low-order key must verify nothing, including a signature under its own seed"
        );

        let honest = a_fresh_key();
        assert!(
            !verify_op_bytes(&low_order, b"an op", &sign_op_bytes(&honest, b"an op")),
            "a low-order key must verify nothing, including an honest signature"
        );
    }

    #[test]
    fn a_low_order_public_key_is_refused_at_the_parse() {
        // All eight low-order points decompress to valid Edwards points, so
        // `VerifyingKey::from_bytes` accepts every one of them and only
        // `verify_strict` refuses the signatures. Our parse refuses them first.
        //
        // Why the parse and not only verification: a key that can never verify
        // anything is not merely useless, it is dangerous one layer up. A Stoa
        // genesis record naming a low-order creator decodes, hashes to a stable
        // address and self-authenticates — a forum whose sole moderator (§6) can
        // never authorise anything, indistinguishable from a real one.
        //
        // All-zeros is the one an attacker would reach for, and it is the case
        // the genesis record makes dangerous.
        assert_eq!(
            PublicKey::from_bytes(&[0u8; 32]),
            Err(KeyError::WeakPublicKey),
            "the all-zero point must be refused"
        );

        // The identity element, y = 1, is the other trivially-writable one.
        let mut one = [0u8; 32];
        one[0] = 1;
        assert_eq!(
            PublicKey::from_bytes(&one),
            Err(KeyError::WeakPublicKey),
            "the identity element must be refused"
        );

        // An honest key is unaffected — the check must reject the low-order
        // points, not anything that merely looks unusual. Without this the
        // check could be `Err` unconditionally and the assertions above would
        // still pass.
        for _ in 0..16 {
            assert!(
                PublicKey::from_bytes(&a_fresh_key().public_key().to_bytes()).is_ok(),
                "a generated key must still parse"
            );
        }
    }

    #[test]
    fn a_derived_stoa_key_is_deterministic() {
        // The whole point: a user must be able to re-derive the key they posted
        // with, from the root, on any device and after any restart. §5.2 makes
        // the per-Stoa identity permanent and §4.1 ties the SDS `senderId` to
        // it, so instability here is not a private inconvenience — it changes
        // the identifier other peers know them by.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let a = derive_stoa_key(&root, &stoa);
        let b = derive_stoa_key(&root, &stoa);
        assert_eq!(a.public_key(), b.public_key());
    }

    #[test]
    fn different_stoas_get_unlinkable_keys() {
        // §5.2's one privacy property: a user's identity in Stoa A must not be
        // tied to their identity in Stoa B by the protocol. Same root, two
        // Stoas, and the keys must differ.
        let root = [7u8; 32];
        assert_ne!(
            derive_stoa_key(&root, &stoa_address(b"stoa one")).public_key(),
            derive_stoa_key(&root, &stoa_address(b"stoa two")).public_key()
        );
    }

    #[test]
    fn different_roots_get_different_keys_in_the_same_stoa() {
        // Two users in one Stoa must not collide — and SDS requires their
        // sender ids to differ (§4.1), which this is what supplies.
        let stoa = stoa_address(b"a genesis record");
        assert_ne!(
            derive_stoa_key(&[1u8; 32], &stoa).public_key(),
            derive_stoa_key(&[2u8; 32], &stoa).public_key()
        );
    }

    #[test]
    fn a_derived_key_signs_and_verifies_like_any_other() {
        // Derivation must produce a usable key, not merely a distinct one.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let sk = derive_stoa_key(&root, &stoa);
        let sig = sign_op_bytes(&sk, b"a post");
        assert!(verify_op_bytes(&sk.public_key(), b"a post", &sig));
    }

    #[test]
    fn a_derived_key_round_trips_through_a_keystore() {
        // Derivation and storage must agree: what the keystore persists is the seed,
        // and reloading it must give back the same posting identity.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let sk = derive_stoa_key(&root, &stoa);
        let restored = SecretKey::from_bytes(&sk.to_bytes()).unwrap();
        assert_eq!(restored.public_key(), sk.public_key());
    }

    #[test]
    fn an_authored_op_verifies_when_the_key_matches_the_claimed_author() {
        let sk = a_fresh_key();
        let pk = sk.public_key();
        let sig = sign_op_bytes(&sk, b"a post");
        assert!(verify_authored_op(
            &pk.address(),
            &pk.to_bytes(),
            b"a post",
            &sig.to_bytes()
        ));
    }

    #[test]
    fn a_validly_signed_op_under_the_wrong_key_is_still_rejected() {
        // THE forgery this function exists to stop, and the one a caller doing
        // the steps by hand would miss: the signature is perfectly valid, the
        // bytes are untampered, and the op is still not from the author it
        // claims. Only re-deriving the address from the key catches it.
        let victim = a_fresh_key();
        let attacker = a_fresh_key();
        let sig = sign_op_bytes(&attacker, b"a post");

        // The attacker signs with their own key but claims the victim's address.
        assert!(!verify_authored_op(
            &victim.public_key().address(),
            &attacker.public_key().to_bytes(),
            b"a post",
            &sig.to_bytes()
        ));

        // And the signature itself is genuinely valid — so a caller who checked
        // only the signature would have accepted this.
        assert!(verify_op_bytes(&attacker.public_key(), b"a post", &sig));
    }

    #[test]
    fn an_authored_op_is_rejected_when_the_bytes_were_tampered_with() {
        let sk = a_fresh_key();
        let pk = sk.public_key();
        let sig = sign_op_bytes(&sk, b"a post");
        assert!(!verify_authored_op(
            &pk.address(),
            &pk.to_bytes(),
            b"a different post",
            &sig.to_bytes()
        ));
    }

    #[test]
    fn an_authored_op_with_malformed_fields_is_false_rather_than_fatal() {
        // Every field here arrives inside an inbound op and is
        // attacker-controlled. A panic would abort the module process
        // (PHASE0-FINDINGS §3), so wrong lengths and junk must be a plain
        // `false` on every field independently.
        let sk = a_fresh_key();
        let pk = sk.public_key();
        let sig = sign_op_bytes(&sk, b"a post");
        let addr = pk.address();

        for bad_key in [vec![], vec![0u8; 31], vec![0u8; 33], vec![9u8; 64]] {
            assert!(!verify_authored_op(
                &addr,
                &bad_key,
                b"a post",
                &sig.to_bytes()
            ));
        }
        for bad_sig in [vec![], vec![0u8; 63], vec![0u8; 65], vec![9u8; 32]] {
            assert!(!verify_authored_op(
                &addr,
                &pk.to_bytes(),
                b"a post",
                &bad_sig
            ));
        }
    }

    #[test]
    fn a_path_derived_key_is_deterministic() {
        // The same property `a_derived_stoa_key_is_deterministic` pins for the
        // two-input scheme, and it matters MORE here: under the path-taking
        // scheme the path is the value that has to be recorded, so instability
        // would mean a recorded path naming an identity that no longer exists.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        assert_eq!(
            derive_stoa_key_at_path(&root, &stoa, 42).public_key(),
            derive_stoa_key_at_path(&root, &stoa, 42).public_key()
        );
    }

    #[test]
    fn different_paths_give_different_identities_in_one_stoa() {
        // The property the whole slate rests on: five candidates for ONE Stoa
        // differ by path alone, so if paths did not separate keys there would be
        // nothing to choose between.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let mut seen = Vec::new();
        for path in [0u32, 1, 2, 7, 1000, u32::MAX] {
            let pk = derive_stoa_key_at_path(&root, &stoa, path).public_key();
            assert!(
                !seen.contains(&pk),
                "path {path} produced a key another path already produced"
            );
            seen.push(pk);
        }
    }

    #[test]
    fn the_path_taking_scheme_does_not_collide_with_the_scheme_without_one() {
        // THE requirement the salt bump exists for. `identity` requires that
        // where both schemes exist they be distinguishable, "so that one
        // scheme's identities cannot be silently reproduced by the other".
        //
        // Path 0 is the only value where a reader would expect them to agree, so
        // it is the value that has to be checked. Revert
        // `STOA_KEY_SALT_WITH_PATH` to version 1 and this still fails — the four
        // appended zero bytes change the info — which is precisely why the bump
        // is argued as a decision rather than relied on as a mechanism.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        assert_ne!(
            derive_stoa_key(&root, &stoa).public_key(),
            derive_stoa_key_at_path(&root, &stoa, 0).public_key(),
            "the two derivation schemes must not produce one identity at path 0"
        );

        // And no path at all reproduces the two-input scheme's key. A handful
        // rather than exhaustively: the point is that path 0 is not special-cased
        // into equivalence, not a proof over 2^32.
        let without = derive_stoa_key(&root, &stoa).public_key();
        for path in [0u32, 1, 2, 3, 4, 5, u32::MAX] {
            assert_ne!(
                derive_stoa_key_at_path(&root, &stoa, path).public_key(),
                without,
                "path {path} reproduced the pathless scheme's identity"
            );
        }
    }

    #[test]
    fn a_path_derived_key_signs_and_verifies_like_any_other() {
        // Derivation must produce a USABLE identity, not merely a distinct one —
        // the same thing `a_derived_key_signs_and_verifies_like_any_other` pins,
        // and the reason it is repeated is that a new derivation is a new place
        // for a seed to be mangled into something that signs but verifies
        // against a different key.
        let key = derive_stoa_key_at_path(&[7u8; 32], &stoa_address(b"a genesis record"), 3);
        let sig = sign_op_bytes(&key, b"a post");
        assert!(verify_op_bytes(&key.public_key(), b"a post", &sig));
        // Through the wire-level entry point too, which is where the address
        // binding lives: a path-derived key must be attributable to its own
        // address like any other.
        assert!(verify_authored_op(
            &key.public_key().address(),
            &key.public_key().to_bytes(),
            b"a post",
            &sig.to_bytes()
        ));
    }

    #[test]
    fn the_path_taking_scheme_keeps_cross_stoa_unlinkability() {
        // The path is a new input and must not have become the ONLY input.
        // One root, one path, two Stoas: the keys must still differ, or a
        // user who chose path 3 everywhere would carry one key across Stoas.
        let root = [7u8; 32];
        assert_ne!(
            derive_stoa_key_at_path(&root, &stoa_address(b"stoa one"), 3).public_key(),
            derive_stoa_key_at_path(&root, &stoa_address(b"stoa two"), 3).public_key()
        );
        // And two roots at one path and one Stoa must differ, or two users who
        // both chose path 3 would collide.
        let stoa = stoa_address(b"a genesis record");
        assert_ne!(
            derive_stoa_key_at_path(&[1u8; 32], &stoa, 3).public_key(),
            derive_stoa_key_at_path(&[2u8; 32], &stoa, 3).public_key()
        );
    }

    #[test]
    fn a_path_derived_key_is_not_the_root_key() {
        // The root must never itself be the identity — it is the one value that,
        // if leaked, yields every identity the user has. Pinned for the new
        // scheme as well as the old, because a new derivation is a new place for
        // the root to be passed through unchanged.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let root_as_key = SecretKey::from_bytes(&root).unwrap();
        for path in [0u32, 1, 2] {
            assert_ne!(
                derive_stoa_key_at_path(&root, &stoa, path).public_key(),
                root_as_key.public_key(),
                "path {path} derived the root key itself"
            );
        }
    }

    #[test]
    fn the_whole_path_reaches_derivation_and_not_only_its_low_byte() {
        // The pinned constants use paths 0 and 1, which differ in the LAST byte
        // alone — so an encoding that fed only the low byte, or only the low two,
        // would reproduce both pinned values exactly. That is the shape this
        // project's defect family takes: two explanations, one answer.
        //
        // A third pinned value fixes it, at a path whose low bytes are zero so
        // that only the HIGH bytes distinguish it from path 0. Computed with
        // OpenSSL, not read back from this code:
        //
        //   openssl kdf -keylen 32 -kdfopt digest:SHA512 \
        //     -kdfopt hexkey:<07 x32> \
        //     -kdfopt hexsalt:2f6469616c6563746963612f322f4964656e746974792f53746f61 \
        //     -kdfopt hexinfo:<stoa>01000000 HKDF
        //
        // and that invocation was validated by reproducing the version-1 value
        // `b62b6b59…` exactly first. Path 0x01000000 = 16,777,216: every byte but
        // the third-from-top is zero, so a derivation reading only the low byte,
        // the low two bytes, or the low three would all produce path 0's key.
        let stoa = stoa_address(b"a genesis record");
        assert_eq!(
            hex::encode(derive_stoa_key_at_path(&[7u8; 32], &stoa, 0x0100_0000).to_bytes()),
            "f1e32c8f4601cb1651be57d58e39f28cc1a6e4ef8a69b9bdd2155ef953a7572b",
            "the high bytes of a path do not reach derivation"
        );

        // And the byte ORDER, which the pinned values also cannot see: 0x00000001
        // and 0x01000000 are each other's byte-reversal, so a little-endian
        // encoding would swap the two keys rather than producing a wrong one.
        // Asserted as an inequality against the path-1 pinned value, so a swap is
        // caught even if the hex above were ever regenerated.
        assert_ne!(
            derive_stoa_key_at_path(&[7u8; 32], &stoa, 0x0100_0000).public_key(),
            derive_stoa_key_at_path(&[7u8; 32], &stoa, 1).public_key(),
            "a path and its byte-reversal derive one key, so the encoding is \
             order-blind"
        );
    }

    #[test]
    fn a_derived_key_is_not_the_root_key() {
        // The root secret must never itself sign anything: it is the one value
        // that, if leaked, yields every Stoa identity a user has. Deriving is
        // what keeps a compromised per-Stoa key from being a compromised root.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let root_as_key = SecretKey::from_bytes(&root).unwrap();
        assert_ne!(
            derive_stoa_key(&root, &stoa).public_key(),
            root_as_key.public_key()
        );
    }
}
