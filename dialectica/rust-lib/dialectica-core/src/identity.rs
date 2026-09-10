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
//! **No keystore.** §5.6 specifies one (encrypted key at a fixed path, three
//! unlock paths, never prompt). It is filesystem work, and a pure crate that
//! cannot touch the disk is the wrong place for it; this module defines the key
//! types that a keystore will hand back.

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
    pub fn from_hex(s: &str) -> Result<Self, AddressError> {
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
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        let bytes: &[u8; 32] = bytes.try_into().map_err(|_| KeyError::NotAValidPublicKey)?;
        ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map(PublicKey)
            .map_err(|_| KeyError::NotAValidPublicKey)
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
/// Those copies are where the keystore (§5.6) takes over, and zeroizing them is
/// its job — a crate that cannot touch the disk cannot own a secret's lifetime
/// anyway. The denials above are about a key reaching a *log or a wire*, which
/// is the reachable threat here; memory hygiene is deferred, not solved.
pub struct SecretKey(ed25519_dalek::SigningKey);

impl SecretKey {
    /// Generate a fresh key from the OS random source.
    ///
    /// Fills a seed and uses the infallible constructor rather than
    /// `SigningKey::generate`, which wants a `CryptoRng` that `rand_core` 0.10
    /// no longer supplies an `OsRng` for. Identical result, one dependency
    /// instead of the `rand` stack.
    ///
    /// Panics if the OS random source fails. That is the right response and not
    /// a shortcut: there is no safe fallback for "I could not get entropy", and
    /// continuing with a predictable key would forge every signature this
    /// identity ever makes. It is also not reachable from a dispatch handler —
    /// key generation happens at keystore setup (§5.6), not while serving an
    /// inbound op.
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("the OS random source must be available to mint a key");
        SecretKey(ed25519_dalek::SigningKey::from_bytes(&seed))
    }

    /// Rebuild a key from stored bytes — what a keystore (§5.6) will call.
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

#[derive(Debug, PartialEq, Eq)]
pub enum KeyError {
    NotAValidPublicKey,
    NotAValidSecretKey,
    NotAValidSignature,
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::NotAValidPublicKey => write!(f, "not a valid public key"),
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
/// unambiguously typed, which is the serialiser's job and the serialiser does
/// not exist yet. When it lands, the op kind belongs in this digest, so the
/// property is structural rather than a convention the encoder must maintain.
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
/// two encodings of the same logical record yield two different addresses. The
/// serialiser that fixes a canonical form does not exist yet (`serde_json` does
/// not produce canonical JSON), and it arrives with the op model.
pub fn stoa_address(genesis_bytes: &[u8]) -> Address {
    let mut hasher = Sha256::new();
    hasher.update(STOA_ADDRESS_PREFIX);
    hasher.update(genesis_bytes);
    Address(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_verifies_against_its_own_key() {
        let sk = SecretKey::generate();
        let sig = sign_op_bytes(&sk, b"an op");
        assert!(verify_op_bytes(&sk.public_key(), b"an op", &sig));
    }

    #[test]
    fn a_signature_does_not_verify_against_a_different_key() {
        // The property moderation rests on (§6): a forged op is one signed by
        // somebody who is not who they claim to be.
        let author = SecretKey::generate();
        let impostor = SecretKey::generate();
        let sig = sign_op_bytes(&impostor, b"an op");
        assert!(!verify_op_bytes(&author.public_key(), b"an op", &sig));
    }

    #[test]
    fn a_signature_does_not_verify_over_different_bytes() {
        // Tamper with the op and the signature must stop matching, or "signed"
        // means nothing.
        let sk = SecretKey::generate();
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
            hex::encode(
                derive_stoa_key(&[7u8; 32], &stoa_address(b"a genesis record")).to_bytes()
            ),
            "b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf",
            "per-Stoa key derivation changed"
        );
    }

    #[test]
    fn different_keys_get_different_addresses() {
        let a = SecretKey::generate();
        let b = SecretKey::generate();
        assert_ne!(a.public_key().address(), b.public_key().address());
    }

    #[test]
    fn an_author_address_is_not_a_bare_hash_of_the_key() {
        // §5.1 is explicit that a RECORD is hashed, not the raw key, so that a
        // key log can be added later without changing anybody's address. This
        // is the test that stops someone "simplifying" that away — it is the
        // only thing distinguishing the two, since both produce 32 plausible
        // bytes.
        let sk = SecretKey::generate();
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
        let sk = SecretKey::generate();
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
        let sk = SecretKey::generate();
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
        let sk = SecretKey::generate();
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
        let sk = SecretKey::generate();
        let pk = sk.public_key();
        assert_eq!(PublicKey::from_bytes(&pk.to_bytes()).unwrap(), pk);
    }

    #[test]
    fn a_secret_key_survives_a_byte_round_trip() {
        // What a keystore (§5.6) will do on unlock: bytes in, same identity out.
        let sk = SecretKey::generate();
        let restored = SecretKey::from_bytes(&sk.to_bytes()).unwrap();
        assert_eq!(restored.public_key(), sk.public_key());
    }

    #[test]
    fn a_signature_survives_a_byte_round_trip() {
        // Signatures cross the wire as 64 bytes, so this is the path every
        // inbound op takes.
        let sk = SecretKey::generate();
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
    fn a_low_order_public_key_cannot_verify_anything() {
        // 32 zero bytes DECODE fine — they are a valid low-order Edwards point,
        // not garbage — so parsing accepts them, and a caller might reasonably
        // expect an all-zero key to have been rejected earlier. It is not: the
        // rejection happens at verification, which is why `verify_op_bytes` uses
        // `verify_strict`.
        //
        // **What this test does NOT do is pin `verify_strict` itself.** Both
        // assertions below would also hold under plain `verify`, because neither
        // signature was made under the zero key — so swapping the call would not
        // turn this red. Pinning the strict check properly needs a crafted
        // small-order forgery, which is fiddly enough that it is not here; the
        // guard against that swap is the doc comment on `verify_op_bytes` and
        // the deliberate absence of a `Verifier` import, not this test.
        //
        // What it does pin is the surprising decode behaviour above, so that
        // anyone who later "fixes" `PublicKey::from_bytes` to reject all-zero
        // finds out that something already depended on it parsing.
        let attacker_key = PublicKey::from_bytes(&[0u8; 32])
            .expect("all-zero decodes as a low-order point, which is the premise here");
        let honest = SecretKey::generate();
        let sig = sign_op_bytes(&honest, b"an op");
        assert!(
            !verify_op_bytes(&attacker_key, b"an op", &sig),
            "a low-order key must never verify"
        );

        // And it cannot verify a signature made under its own seed either.
        let zero_seed = SecretKey::from_bytes(&[0u8; 32]).unwrap();
        let self_sig = sign_op_bytes(&zero_seed, b"an op");
        assert!(
            !verify_op_bytes(&attacker_key, b"an op", &self_sig),
            "verify_strict must reject the low-order key regardless of who signed"
        );
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
        // Derivation and storage must agree: what §5.6 persists is the seed,
        // and reloading it must give back the same posting identity.
        let root = [7u8; 32];
        let stoa = stoa_address(b"a genesis record");
        let sk = derive_stoa_key(&root, &stoa);
        let restored = SecretKey::from_bytes(&sk.to_bytes()).unwrap();
        assert_eq!(restored.public_key(), sk.public_key());
    }

    #[test]
    fn an_authored_op_verifies_when_the_key_matches_the_claimed_author() {
        let sk = SecretKey::generate();
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
        let victim = SecretKey::generate();
        let attacker = SecretKey::generate();
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
        assert!(verify_op_bytes(
            &attacker.public_key(),
            b"a post",
            &sig
        ));
    }

    #[test]
    fn an_authored_op_is_rejected_when_the_bytes_were_tampered_with() {
        let sk = SecretKey::generate();
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
        let sk = SecretKey::generate();
        let pk = sk.public_key();
        let sig = sign_op_bytes(&sk, b"a post");
        let addr = pk.address();

        for bad_key in [vec![], vec![0u8; 31], vec![0u8; 33], vec![9u8; 64]] {
            assert!(!verify_authored_op(&addr, &bad_key, b"a post", &sig.to_bytes()));
        }
        for bad_sig in [vec![], vec![0u8; 63], vec![0u8; 65], vec![9u8; 32]] {
            assert!(!verify_authored_op(&addr, &pk.to_bytes(), b"a post", &bad_sig));
        }
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
