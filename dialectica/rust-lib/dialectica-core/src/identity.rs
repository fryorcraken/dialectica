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
/// BIP-340 keys are **x-only** — 32 bytes, no parity byte. That is the whole
/// serialised form.
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

/// A 64-byte BIP-340 signature.
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
/// `SHA256(OP_SIGNING_PREFIX || bytes)`. The prefix is the whole point: it
/// makes a signature meaningful only as "this author signed this op", so no
/// signature produced anywhere else in the system — or by any other system
/// sharing these keys — can be replayed as one.
pub fn signing_digest(bytes: &[u8]) -> [u8; 32] {
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
/// Returns a plain `bool`. There is exactly one thing a caller may do with a
/// bad signature — drop the op (§3.3: "the store may hold junk; the reader
/// never trusts it") — so distinguishing *why* it failed would offer a choice
/// that does not exist, and inviting a caller to branch on it is how a
/// "recoverable" verification failure becomes an accepted op.
///
/// **`verify_strict`, never `verify`, and this is a correctness requirement
/// rather than belt-and-braces.** RFC 8032 permits both cofactored and
/// uncofactored verification, and `verify` accepts low-order public keys and
/// non-canonical encodings that `verify_strict` rejects. Every peer verifies
/// independently (§3.3, §6), so two peers using different rules would disagree
/// about whether the same op is validly signed — a partition in a system whose
/// whole moderation story rests on peers reaching the same verdict from the
/// same bytes.
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

/// A Stoa's address, derived from its genesis record's canonical bytes.
///
/// This is what makes a pasted Stoa address **self-authenticating** (§4.8):
/// re-deriving it from the record you received either reproduces the address
/// you were given or it does not, so a tampered record cannot masquerade as the
/// Stoa you meant to join. No registry is consulted, which is what keeps
/// permissionless creation (§1) from needing one.
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
    fn an_address_is_stable_for_a_key() {
        let sk = SecretKey::generate();
        assert_eq!(sk.public_key().address(), sk.public_key().address());
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
        // Distinct prefixes, so no byte string is valid as both. Without this,
        // a Stoa address could be presented as an author's and vice versa.
        let sk = SecretKey::generate();
        let record = {
            let mut h = Sha256::new();
            h.update([1u8]);
            h.update(sk.public_key().to_bytes());
            h.finalize().to_vec()
        };
        // Same underlying bytes, two derivations: they must differ.
        let mut author_preimage = vec![1u8];
        author_preimage.extend_from_slice(&sk.public_key().to_bytes());
        assert_ne!(
            sk.public_key().address(),
            stoa_address(&author_preimage),
            "author and Stoa derivations must be domain-separated"
        );
        assert_ne!(stoa_address(&record), sk.public_key().address());
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
        // not garbage — so parsing accepts them. That is not the bug it looks
        // like; it is why `verify_op_bytes` uses `verify_strict`, which rejects
        // low-order public keys at verification time.
        //
        // This is the attack the strict check exists for: with lenient
        // `verify()`, a small-order key can be made to accept signatures it
        // never authorised, so an attacker publishing ops under such a key
        // could have them treated as validly signed. Pinning it here means
        // anyone who "simplifies" `verify_strict` to `verify` gets a red test
        // rather than a silent authenticity hole.
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
