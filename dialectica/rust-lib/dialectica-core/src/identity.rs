//! Identity: keys, addresses, and the signatures that bind an op to an author.
//!
//! # The scheme, and why it is this one
//!
//! **BIP-340 Schnorr over secp256k1, with SHA-256 addresses** — the scheme LEZ
//! uses, matched on purpose rather than chosen fresh (PLAN.md §5.1 asks to
//! match LEZ's cryptographic seriousness). Concretely LEZ signs via `k256`'s
//! `schnorr` feature and derives a public account address as SHA-256 over a
//! domain-separated 32-byte prefix; this module does the same with its own
//! prefixes.
//!
//! Matching the *primitive* is the part worth having. Matching LEZ's *crates*
//! is not: `lee` and `lee_core` are unpublished in-tree crates, and `lee_core`
//! depends on `risc0-zkvm` solely to reach `risc0_zkvm::sha::Impl`, whose
//! output on the host is byte-identical to `sha2`. The Cargo.toml records that
//! trade in full.
//!
//! # What is deliberately not here
//!
//! **No key rotation** (§5.3). Not an omission — rotation without spam
//! resistance is a ban-evasion feature, so it arrives with RLN, not before.
//! What this module owes that future is only that it must not *foreclose* it,
//! which is what hashing a record rather than a bare key achieves.
//!
//! **No keystore.** §5.6 specifies one (encrypted key at a fixed path, three
//! unlock paths, never prompt). It is filesystem work, and a pure crate that
//! cannot touch the disk is the wrong place for it; this module defines the key
//! types that a keystore will hand back.

use k256::schnorr::signature::{Signer, Verifier};
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
pub struct PublicKey(k256::schnorr::VerifyingKey);

impl PublicKey {
    /// Parse an x-only public key from wire bytes.
    ///
    /// **The length check is load-bearing and must not be "simplified" into the
    /// delegation below.** `k256::schnorr::VerifyingKey::from_bytes` returns a
    /// `Result`, which makes it look total — but it opens with
    /// `FieldBytes::from_slice(bytes)`, and that is `generic-array`'s
    /// `from_slice`, which **panics** on a length mismatch rather than
    /// returning an error:
    ///
    /// ```text
    /// panicked at generic-array/src/lib.rs:576: assertion `left == right`
    /// failed: left: 0, right: 32
    /// ```
    ///
    /// This function parses attacker-supplied bytes off the network — a
    /// public key arrives inside every inbound op — so reaching that panic is
    /// remotely triggerable. PHASE0-FINDINGS §3 measured what a panic in a
    /// dispatch handler actually does: the module process **aborts**, the
    /// caller waits out a 20s timeout, and every later call reports
    /// MODULE_NOT_LOADED. A one-line length guard is what stands between a
    /// malformed field and that.
    ///
    /// (`SecretKey::from_bytes` needs no such guard — `NonZeroScalar::try_from`
    /// rejects a wrong length as an error. The asymmetry is real, which is
    /// exactly why it is written down here rather than assumed either way.)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        if bytes.len() != 32 {
            return Err(KeyError::NotAValidPublicKey);
        }
        k256::schnorr::VerifyingKey::from_bytes(bytes)
            .map(PublicKey)
            .map_err(|_| KeyError::NotAValidPublicKey)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes().into()
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
    // Derived Debug on the inner k256 type would print the curve point's
    // internals. A public key is not secret, but it is noise in a test failure;
    // the hex is what a reader can act on.
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
pub struct SecretKey(k256::schnorr::SigningKey);

impl SecretKey {
    /// Generate a fresh key from the OS random source.
    pub fn generate() -> Self {
        SecretKey(k256::schnorr::SigningKey::random(
            &mut k256::elliptic_curve::rand_core::OsRng,
        ))
    }

    /// Rebuild a key from stored bytes — what a keystore (§5.6) will call.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        k256::schnorr::SigningKey::from_bytes(bytes)
            .map(SecretKey)
            .map_err(|_| KeyError::NotAValidSecretKey)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes().into()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(*self.0.verifying_key())
    }

    /// Sign a message that has already been domain-separated by [`signing_digest`].
    ///
    /// Private on purpose: everything dialectica signs goes through
    /// [`sign_op_bytes`], so there is no way to reach this with an
    /// undifferentiated message.
    fn sign_digest(&self, digest: &[u8; 32]) -> Signature {
        Signature(self.0.sign(digest))
    }
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
pub struct Signature(k256::schnorr::Signature);

impl Signature {
    /// Parse a 64-byte signature from wire bytes.
    ///
    /// **The length check is load-bearing**, for the same reason as
    /// [`PublicKey::from_bytes`] and via a different upstream panic. `k256`'s
    /// `TryFrom<&[u8]> for Signature` opens with
    /// `bytes.split_at(Self::BYTE_SIZE / 2)`, and `split_at` panics when the
    /// slice is shorter than the midpoint:
    ///
    /// ```text
    /// panicked at k256-0.13.4/src/schnorr.rs:146: mid > len
    /// ```
    ///
    /// A `TryFrom` returning `Result` reads as total, and is not. Both this and
    /// the public-key case were found by a test asserting malformed wire bytes
    /// are *rejected* rather than fatal — which is the only way to find them,
    /// since neither is visible in the signature of what it calls.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        if bytes.len() != 64 {
            return Err(KeyError::NotAValidSignature);
        }
        k256::schnorr::Signature::try_from(bytes)
            .map(Signature)
            .map_err(|_| KeyError::NotAValidSignature)
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
pub fn verify_op_bytes(key: &PublicKey, bytes: &[u8], signature: &Signature) -> bool {
    key.0.verify(&signing_digest(bytes), &signature.0).is_ok()
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
        // These parse attacker-supplied bytes off the wire. An unwrap here
        // would be a remote abort of the module process (PHASE0-FINDINGS §3),
        // so returning an error is the whole contract.
        assert!(PublicKey::from_bytes(&[]).is_err());
        assert!(PublicKey::from_bytes(&[0u8; 31]).is_err());
        assert!(PublicKey::from_bytes(&[0u8; 33]).is_err());
        // All-zero is not a valid curve point.
        assert!(PublicKey::from_bytes(&[0u8; 32]).is_err());
        assert!(SecretKey::from_bytes(&[]).is_err());
        assert!(SecretKey::from_bytes(&[0u8; 32]).is_err());
        assert!(Signature::from_bytes(&[]).is_err());
        assert!(Signature::from_bytes(&[0u8; 63]).is_err());
    }
}
