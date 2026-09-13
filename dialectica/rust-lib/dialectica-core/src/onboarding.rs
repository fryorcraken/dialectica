//! The onboarding slate: several candidate identities, and keeping one.
//!
//! The contract is the `identity-onboarding` spec; the reasoning behind the
//! shapes is in that change's `design.md`. What is repeated here is only what a
//! reader of THIS file needs in order not to undo it.
//!
//! # A candidate is a derivation path, not a master key
//!
//! The five candidates in a slate come from **one** master key and differ by
//! their derivation path alone. The alternative — minting five independent root
//! secrets and keeping one — was a first implementation's shape and it is worse
//! for a reason that has nothing to do with the slate: it gives the user five
//! things to back up instead of one, and discards four of them. One master key
//! plus a path index is one secret to save.
//!
//! That is also why there is no clearing obligation on discarded candidates here
//! in the form one might expect. Under this model there are no unkept *secrets*
//! to begin with: the paths are public, the master key is one value the keystore
//! owns, and a candidate that is not chosen leaves nothing behind because nothing
//! about it was ever separately held.
//!
//! # A slate is a nonce, not held state
//!
//! [`Slate::from_nonce`] reproduces a slate exactly from 32 bytes. So what spans
//! the two calls — generate, then keep — is the nonce and nothing else, and two
//! of the spec's requirements become structural rather than guarded:
//!
//! - **"A selection made against a superseded set is refused."** A selection
//!   quotes the nonce it was made against. A nonce that is not the live one fails
//!   the comparison, so the superseded case and the never-existed case are one
//!   code path — there is no second path to get wrong.
//! - **"Generating a slate writes nothing" / "A discarded slate leaves no
//!   trace."** A slate that is a nonce plus a derivation is nothing to discard.
//!
//! # Nothing here panics
//!
//! A panic aborts the module process (PHASE0-FINDINGS §3). The one fallible thing
//! this file does is ask the OS for randomness, and that is a `Result`.

use crate::identity::{Address, PublicKey, SecretKey};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// How many candidates a slate holds.
///
/// **Fixed by this implementation and reported with the set, never requested by
/// the caller.** The spec's reason is worth restating because it is a security
/// argument rather than an API preference: a caller-supplied count is *"a number
/// that decides how much key derivation this module performs"*, which is an
/// unbounded work request from the other side of the module boundary.
///
/// Five, from PLAN.md §5.2.1: *"At onboarding they are shown a slate of five
/// generated identities and pick one."*
pub const SLATE_SIZE: usize = 5;

/// Domain separation for deriving a slate's paths from its nonce.
///
/// A fixed 32 bytes, padded rather than natural, following every other prefix in
/// `identity.rs` and for the same reason: a variable-length prefix concatenated
/// with variable-length data is how two different inputs come to share a
/// preimage. Padding removes the question rather than arguing about it.
///
/// Versioned, so a future way of laying out a slate produces different paths from
/// the same nonce rather than colliding with this one.
const SLATE_PATH_PREFIX: &[u8; 32] = b"/dialectica/1/Slate/Path\0\0\0\0\0\0\0\0";

/// The largest index [`Slate::from_nonce`] will walk to while looking for
/// [`SLATE_SIZE`] distinct paths.
///
/// **A bound, not an expectation.** Two paths derived from one nonce collide with
/// probability around 2⁻³¹ per pair, so the walk terminates at index 4 in every
/// run anybody will ever observe. The cap exists because "will not happen" is not
/// "cannot happen", and an unbounded loop reachable from a wire method is a
/// denial of service whatever its expected iteration count.
///
/// Generous relative to what is needed: reaching even index 10 would require
/// several simultaneous collisions.
const MAX_PATH_WALK: u32 = 64;

/// One past the largest derivation path this build can produce.
///
/// **The mask and the read-back guard are one rule, and this is where it lives.**
/// [`derive_path`] masks the top bit off, so every path this build writes is below
/// 2³¹ — and `identity_store`'s decode refuses anything at or above this value for
/// the same reason it refuses a negative one. Naming the bound once is what makes
/// "is it applied everywhere?" a question with an answer: review found the mask
/// applied at one call site and the guard bounding the whole of `u32`, which is a
/// wider range than any slate can offer, so a hand-edited row in [2³¹, 2³²) was
/// accepted and derived a working identity nobody chose.
///
/// Two rules that must agree cannot be two constants. A reader changing the mask
/// changes the guard, because there is only one thing to change.
pub const PATH_LIMIT: u32 = 0x8000_0000;

/// The 32 bytes a slate is reproduced from.
///
/// **Public randomness, not key material**, and the type says so by being
/// `Clone` and `Debug` — which [`SecretKey`] deliberately is not. A nonce names
/// which five paths were offered; a path is not secret (the spec: the record
/// *"SHALL NOT be required to be secret"*), and the candidates need the master
/// key rather than the nonce in order to exist at all.
///
/// It is carried to the caller and quoted back, which is the whole mechanism by
/// which a superseded selection is refused, so it has to be representable as text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlateNonce([u8; 32]);

impl SlateNonce {
    /// A fresh nonce from the OS random source.
    ///
    /// A `Result` rather than `SecretKey::generate`'s `expect`, and the difference
    /// is reachability: generating a slate is something a dispatch handler does on
    /// request, and a panic there aborts the module process. `SecretKey::generate`
    /// runs at keystore setup, which is why it may `expect`.
    pub fn generate() -> Result<Self, OnboardingError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| OnboardingError::NoRandomness)?;
        Ok(SlateNonce(bytes))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse the form a caller quotes back.
    ///
    /// Strict about length and hex validity, because this parses caller-supplied
    /// content. A lenient parser would be worse here than in most places: a nonce
    /// that was accepted after truncation would name a *different* slate, and
    /// keeping a candidate from it would store an identity the user did not
    /// choose — which the spec calls unrecoverable, because the choice cannot be
    /// recomputed.
    pub fn from_hex(s: &str) -> Result<Self, OnboardingError> {
        let bytes = hex::decode(s).map_err(|_| OnboardingError::NonceNotHex)?;
        let bytes: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| OnboardingError::NonceWrongLength(bytes.len()))?;
        Ok(SlateNonce(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// The derivation path for the candidate at `index` of the slate `nonce` names.
///
/// `SHA256(prefix || nonce || index)`, first four bytes big-endian, **top bit
/// masked off**.
///
/// # The mask is not cosmetic
///
/// It keeps every path below 2³¹, which buys two things that are cheap now and
/// awkward to retrofit:
///
/// - The value is a positive SQLite `INTEGER` on every path through
///   [`crate::identity_store`], so a stored path never needs reinterpreting and
///   the out-of-range refusal there is about a hand-edited file rather than about
///   values this code writes. That refusal is bounded by [`PATH_LIMIT`], the same
///   constant this mask is expressed in, so the two cannot drift apart.
/// - It is a valid **non-hardened** BIP-32 index. `proposal.md` records the LEZ
///   wallet direction, where a path would be handed to
///   `get_public_key_for_path`; a path above 2³¹ is the hardened range and would
///   mean something different there. Nothing in this change implements that
///   direction — the mask is what keeps it from needing a migration of recorded
///   paths if it is ever taken.
///
/// # Derived rather than random
///
/// Five random `u32`s would have to be stored or re-randomised, which is the
/// held-slate shape this file exists to avoid. One nonce reproduces all five, so
/// the spec's *"Requesting another set yields different candidates"* follows from
/// a fresh nonce rather than from a uniqueness check across slates.
pub fn derive_path(nonce: &SlateNonce, index: u32) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(SLATE_PATH_PREFIX);
    hasher.update(nonce.as_bytes());
    hasher.update(index.to_be_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    let mut head = [0u8; 4];
    head.copy_from_slice(&digest[..4]);
    // `PATH_LIMIT - 1` rather than a literal `0x7fff_ffff`, so the mask and
    // `identity_store`'s read-back guard are the same constant rather than two
    // that have to be kept in step. `PATH_LIMIT` is a power of two, so masking
    // with `PATH_LIMIT - 1` is exactly "keep every path below it".
    u32::from_be_bytes(head) & (PATH_LIMIT - 1)
}

/// One candidate identity: what the view is shown, and what keeping it needs.
///
/// **No secret**, and that is the spec's own requirement rather than caution:
/// *"a reply describing candidates SHALL NOT carry a secret key, a master key, a
/// seed, a mnemonic, or any value from which one could be reconstructed."* The
/// view cannot sign, because signing is the module's; widening a reply later is
/// additive, while a secret that has crossed the module boundary cannot be
/// recalled.
///
/// The public key is present as well as the address, because the spec requires
/// it: the generated display name is derived from the public key rather than from
/// the address. What that derivation IS belongs to a different capability — this
/// one *"SHALL NOT define how a display name or a visual mark is derived from a
/// key"* — so this type carries the input and names nothing about the output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Which candidate of the slate this is, `0..SLATE_SIZE`. What a caller
    /// quotes to keep it.
    pub index: usize,
    /// The derivation path. Public, and the value that gets recorded.
    pub path: u32,
    /// The address this identity would post under — the only unforgeable way to
    /// tell two candidates apart.
    pub address: Address,
    /// The public key, because a display name is derived from it.
    pub public_key: PublicKey,
}

/// A slate: the nonce it is reproduced from, and its candidates.
///
/// Not `Clone`: a slate is cheap to reproduce from its nonce, and a second copy
/// of one would be a second answer to "which slate is live".
#[derive(Debug)]
pub struct Slate {
    pub nonce: SlateNonce,
    pub candidates: Vec<Candidate>,
}

impl Slate {
    /// A fresh slate for a Stoa, from a master key.
    ///
    /// The master key is a `&[u8; 32]` — the keystore's root — rather than a
    /// `Keystore`, so that this function can be exercised without a file and so
    /// that it has no opinion about where a secret lives.
    pub fn generate(master_key: &[u8; 32], stoa: &Address) -> Result<Self, OnboardingError> {
        Self::from_nonce(master_key, stoa, SlateNonce::generate()?)
    }

    /// Reproduce the slate a nonce names.
    ///
    /// **This is the function that makes a slate reproducible rather than held**,
    /// and the property it must have is that the same nonce, master key and Stoa
    /// always give the same five candidates — because `keep` recomputes the slate
    /// a selection was made against rather than looking one up.
    ///
    /// # The walk, and why it is not a retry loop
    ///
    /// The spec requires no two candidates in a set share a public key or an
    /// address. Two paths from one nonce could in principle collide, so distinctness
    /// is established rather than assumed: the index walks forward and a path
    /// already held is skipped.
    ///
    /// A retry loop with a fresh nonce would have been the obvious alternative and
    /// is worse: it makes the slate non-reproducible from its nonce, which is the
    /// one property everything else here rests on. Walking the index keeps
    /// reproducibility, because `keep` performs the identical walk.
    ///
    /// Distinctness of *paths* is what is checked, and it gives distinctness of
    /// keys and addresses for free: derivation is deterministic and injective in
    /// practice, so two distinct paths give two distinct keys, and two distinct
    /// keys give two distinct addresses because an address is a hash of a record
    /// containing the key. Checking the paths is checking the thing that is
    /// actually under this function's control.
    pub fn from_nonce(
        master_key: &[u8; 32],
        stoa: &Address,
        nonce: SlateNonce,
    ) -> Result<Self, OnboardingError> {
        let mut candidates: Vec<Candidate> = Vec::with_capacity(SLATE_SIZE);
        let mut paths: Vec<u32> = Vec::with_capacity(SLATE_SIZE);

        for step in 0..MAX_PATH_WALK {
            if candidates.len() == SLATE_SIZE {
                break;
            }
            let path = derive_path(&nonce, step);
            if paths.contains(&path) {
                continue;
            }
            paths.push(path);
            // The secret is held only as long as it takes to take its public
            // half, and in a `Zeroizing` so that the buffer is overwritten on
            // drop rather than by a line somebody has to remember. The spec
            // extends `keystore`'s clearing obligation to slate material, and
            // this is where that material exists.
            //
            // `to_bytes()` IS called, on the next line, and what makes that safe is
            // that its `[u8; 32]` is moved straight into the wrapper with **no
            // intermediate binding** — the temporary is consumed, so there is no
            // second copy to forget. That is `Keystore::generate`'s shape, argued at
            // length there after review found that deleting an explicit wipe left
            // the whole suite green.
            //
            // An earlier version of this comment said there was no `to_bytes` call
            // at all. Security review caught it: the shape described was a different
            // correct shape, so a reader who trusted the sentence and bound a local
            // first would have been told by this comment that no local exists.
            //
            // What this does NOT cover is one layer down. `derive_stoa_key_at_path`
            // builds a `seed` on the stack and does not wipe it, and `identity.rs`
            // defers memory lifetime to `keystore` — which owned it when derivation
            // ran once at setup, and does not own this path, which runs five times
            // per slate. Residual memory, not a reachable leak: nothing reads those
            // bytes back. Recorded in `design.md` rather than fixed here.
            let key: Zeroizing<[u8; 32]> =
                Zeroizing::new(candidate_key(master_key, stoa, path).to_bytes());
            let public_key = SecretKey::from_bytes(&*key)
                // Every 32-byte string is a valid Ed25519 seed, so this cannot
                // fail — and it is still a `Result` rather than an `expect`,
                // because an `expect` on this path is a panic in a module process.
                .map_err(|_| OnboardingError::Derivation)?
                .public_key();
            candidates.push(Candidate {
                index: candidates.len(),
                path,
                address: public_key.address(),
                public_key,
            });
        }

        if candidates.len() != SLATE_SIZE {
            // Not reachable: it needs `MAX_PATH_WALK` derivations to yield fewer
            // than five distinct values. An error rather than an `expect` for the
            // reason every other arm here is.
            return Err(OnboardingError::Derivation);
        }
        Ok(Slate { nonce, candidates })
    }

    /// The candidate at `index`, or a refusal.
    ///
    /// **Refuses rather than coercing**, which the spec requires by name: coercing
    /// an out-of-range selection to a default *"would store an identity the user
    /// did not choose — which is unrecoverable, because the choice cannot be
    /// recomputed"*. So there is no clamp, no modulo and no nearest-match.
    pub fn candidate(&self, index: usize) -> Result<&Candidate, OnboardingError> {
        self.candidates
            .get(index)
            .ok_or(OnboardingError::NoSuchCandidate {
                index,
                of: self.candidates.len(),
            })
    }
}

/// The signing key for one candidate.
///
/// A named function with one caller, so that "a candidate's key comes from the
/// path-taking per-Stoa derivation and from nowhere else" is a property with a
/// single place to check it. `identity-onboarding` requires derivation *"remain
/// that of the `identity` capability"* and forbids a second scheme; this is the
/// one line that could violate that, and it is one line.
fn candidate_key(master_key: &[u8; 32], stoa: &Address, path: u32) -> SecretKey {
    crate::identity::derive_stoa_key_at_path(master_key, stoa, path)
}

/// What can go wrong on the onboarding path, each arm distinguishable.
///
/// Distinguishable for `KeystoreError`'s reason: the spec requires that a refusal
/// because an identity already exists be *"distinguishable from a malformed
/// request and from a storage failure"*, and that is only possible if the error
/// said which.
#[derive(Debug, PartialEq, Eq)]
pub enum OnboardingError {
    /// The OS random source was unavailable, so no nonce could be minted.
    NoRandomness,
    /// The quoted nonce was not valid hex.
    NonceNotHex,
    /// The quoted nonce was the wrong length.
    NonceWrongLength(usize),
    /// The quoted nonce is not the one the live slate was generated from.
    ///
    /// **This is the superseded case as well as the never-existed case**, and
    /// they are deliberately one arm. Telling them apart would require keeping
    /// every nonce ever issued, which is a growing list held to answer a question
    /// with one useful answer: generate a fresh slate and choose from it.
    NonceIsNotTheLiveSlate,
    /// No slate has been generated, so there is nothing to keep from.
    NoLiveSlate,
    /// The selection does not name a candidate in the set.
    NoSuchCandidate { index: usize, of: usize },
    /// Derivation did not produce a usable key. Not reachable with the constants
    /// here; present because the alternative is an `expect`, which on this path
    /// would be a panic in a module process.
    Derivation,
}

impl std::fmt::Display for OnboardingError {
    /// Every message names the fix, following `KeystoreError::Display`'s
    /// documented obligation — these strings reach a view, and a message a user
    /// cannot act on is a message not worth carrying.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardingError::NoRandomness => write!(
                f,
                "the OS random source is unavailable, so no identities could be \
                 generated; try again"
            ),
            OnboardingError::NonceNotHex => write!(
                f,
                "the slate identifier is not valid hex; pass back the one the slate \
                 was returned with"
            ),
            OnboardingError::NonceWrongLength(n) => write!(
                f,
                "a slate identifier is 32 bytes (64 hex chars), got {n} bytes; pass \
                 back the one the slate was returned with"
            ),
            OnboardingError::NonceIsNotTheLiveSlate => write!(
                f,
                "that slate is no longer the current one; generate a slate and choose \
                 from the candidates it returns"
            ),
            OnboardingError::NoLiveSlate => write!(
                f,
                "no slate has been generated; generate one before keeping an identity"
            ),
            OnboardingError::NoSuchCandidate { index, of } => write!(
                f,
                "there is no candidate {index} in a slate of {of}; choose one of \
                 0 to {}",
                of.saturating_sub(1)
            ),
            OnboardingError::Derivation => write!(
                f,
                "identity derivation failed; this is a bug — report it rather than \
                 retrying"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{sign_op_bytes, stoa_address, verify_authored_op};

    fn a_stoa() -> Address {
        stoa_address(b"a genesis record")
    }

    fn a_nonce() -> SlateNonce {
        SlateNonce([7u8; 32])
    }

    #[test]
    fn the_slate_constants_are_pinned_to_known_answers() {
        // Following `identity.rs`'s wire constants, and for the same reason: the
        // prefix string and the mask are consensus-critical in the silent
        // direction. Change either and a recorded path names a different
        // identity, with no error anywhere.
        //
        // Both values were computed with OpenSSL rather than read back from this
        // code:
        //
        //   printf '<prefix||nonce||index>' | openssl dgst -sha256
        //
        // and the mask applied by hand. A value read back from the implementation
        // would be the implementation agreeing with itself.
        //
        // Index 0's digest begins 0x831b85ca — top bit SET — so masking gives
        // 0x031b85ca = 52,135,370.
        assert_eq!(
            derive_path(&a_nonce(), 0),
            52_135_370,
            "slate path 0 changed"
        );
        // Index 1's begins 0xe27a008a, also with the top bit set, giving
        // 0x627a008a = 1,652,162,698. Two masked cases rather than one, because a
        // mask that was accidentally a no-op would need a digest whose top bit
        // happened to be clear to hide it.
        assert_eq!(
            derive_path(&a_nonce(), 1),
            1_652_162_698,
            "slate path 1 changed"
        );
        assert_eq!(SLATE_SIZE, 5, "the slate size is five (PLAN.md §5.2.1)");
    }

    #[test]
    fn every_derived_path_is_below_two_to_the_thirty_one() {
        // What the mask buys: a positive SQLite INTEGER, and a non-hardened BIP-32
        // index. Over many indices rather than the five a slate uses, because the
        // property is about the function and not about one slate.
        let nonce = a_nonce();
        for index in 0..500u32 {
            let path = derive_path(&nonce, index);
            assert!(
                path < 0x8000_0000,
                "path at index {index} is {path}, at or above 2^31"
            );
        }
    }

    #[test]
    fn a_slate_holds_the_fixed_number_of_candidates() {
        // The spec: the reply carries the fixed number, and the caller does not
        // choose it. There is no parameter to pass, which is the stronger form of
        // "not requested by the caller" — checked here as the count.
        let slate = Slate::generate(&[7u8; 32], &a_stoa()).unwrap();
        assert_eq!(slate.candidates.len(), SLATE_SIZE);
    }

    #[test]
    fn every_candidate_in_a_slate_is_distinct() {
        // The spec: no two candidates share a public key, and no two share an
        // address. Checked on all three of path, key and address — the path is
        // what the walk establishes, and the other two are what the spec names.
        let slate = Slate::generate(&[7u8; 32], &a_stoa()).unwrap();
        for (i, a) in slate.candidates.iter().enumerate() {
            for b in slate.candidates.iter().skip(i + 1) {
                assert_ne!(a.path, b.path, "two candidates share a path");
                assert_ne!(a.public_key, b.public_key, "two candidates share a key");
                assert_ne!(a.address, b.address, "two candidates share an address");
            }
        }
        // And the indices are 0..SLATE_SIZE in order, since a caller selects by
        // index and an index that did not match the position would select the
        // wrong candidate.
        for (position, candidate) in slate.candidates.iter().enumerate() {
            assert_eq!(candidate.index, position);
        }
    }

    #[test]
    fn a_nonce_reproduces_an_identical_slate() {
        // The property every other decision here rests on: `keep` recomputes the
        // slate a selection was made against rather than looking one up, so a
        // nonce that did not reproduce exactly would keep the wrong candidate.
        let master = [7u8; 32];
        let stoa = a_stoa();
        let first = Slate::from_nonce(&master, &stoa, a_nonce()).unwrap();
        let second = Slate::from_nonce(&master, &stoa, a_nonce()).unwrap();
        assert_eq!(first.candidates, second.candidates);
    }

    #[test]
    fn one_nonce_gives_different_slates_for_different_master_keys_and_stoas() {
        // The nonce is not the identity: it selects paths, and the paths are
        // derived against a master key and a Stoa. A slate that depended on the
        // nonce alone would hand two users the same candidates.
        let stoa = a_stoa();
        let mine = Slate::from_nonce(&[1u8; 32], &stoa, a_nonce()).unwrap();
        let theirs = Slate::from_nonce(&[2u8; 32], &stoa, a_nonce()).unwrap();
        assert_ne!(mine.candidates, theirs.candidates);

        let elsewhere =
            Slate::from_nonce(&[1u8; 32], &stoa_address(b"another stoa"), a_nonce()).unwrap();
        assert_ne!(mine.candidates, elsewhere.candidates);

        // The PATHS are the same in all three, because a path depends on the
        // nonce alone. Asserted rather than left implicit, because it is the fact
        // that makes the previous assertions about keys rather than about paths.
        let paths = |s: &Slate| s.candidates.iter().map(|c| c.path).collect::<Vec<_>>();
        assert_eq!(paths(&mine), paths(&theirs));
        assert_eq!(paths(&mine), paths(&elsewhere));
    }

    #[test]
    fn a_second_slate_shares_no_candidate_with_the_first() {
        // The spec: "no candidate in the second set has a public key from the
        // first". Over several regenerations rather than one pair, because a
        // single pair differing could be luck in a design that reused nonces.
        let master = [7u8; 32];
        let stoa = a_stoa();
        let mut seen: Vec<PublicKey> = Vec::new();
        for round in 0..8 {
            let slate = Slate::generate(&master, &stoa).unwrap();
            for candidate in &slate.candidates {
                assert!(
                    !seen.contains(&candidate.public_key),
                    "round {round} reoffered a key from an earlier slate"
                );
            }
            seen.extend(slate.candidates.iter().map(|c| c.public_key.clone()));
        }
    }

    #[test]
    fn regeneration_is_not_limited() {
        // The spec: "each request is answered, and none is refused on the ground
        // of how many preceded it". PLAN.md §5.2.1 is explicit that unlimited
        // regeneration is the design and that the grinding it enables is accepted
        // — so a rate limit here would be a silent departure from a decision that
        // was argued at length.
        let master = [7u8; 32];
        let stoa = a_stoa();
        for round in 0..200 {
            assert!(
                Slate::generate(&master, &stoa).is_ok(),
                "slate {round} was refused"
            );
        }
    }

    #[test]
    fn a_candidates_key_is_the_path_taking_per_stoa_derivation() {
        // `identity-onboarding` requires derivation "remain that of the `identity`
        // capability" and forbids a second scheme. This is the test that would
        // catch a slate that derived its own way — it compares against the
        // primitive directly rather than against another slate.
        let master = [7u8; 32];
        let stoa = a_stoa();
        let slate = Slate::from_nonce(&master, &stoa, a_nonce()).unwrap();
        for candidate in &slate.candidates {
            assert_eq!(
                candidate.public_key,
                crate::identity::derive_stoa_key_at_path(&master, &stoa, candidate.path)
                    .public_key(),
                "candidate {} does not come from the path-taking derivation",
                candidate.index
            );
            assert_eq!(candidate.address, candidate.public_key.address());
        }
    }

    #[test]
    fn a_candidate_is_an_identity_that_can_sign() {
        // A candidate must be a usable identity, not merely a displayable one.
        // Through the wire-level entry point, which is where the address binding
        // lives: an op signed by the candidate's key must be attributable to the
        // address the candidate showed.
        let master = [7u8; 32];
        let stoa = a_stoa();
        let slate = Slate::from_nonce(&master, &stoa, a_nonce()).unwrap();
        let candidate = slate.candidate(2).unwrap();
        let key = crate::identity::derive_stoa_key_at_path(&master, &stoa, candidate.path);
        let sig = sign_op_bytes(&key, b"a post");
        assert!(
            verify_authored_op(
                &candidate.address,
                &candidate.public_key.to_bytes(),
                b"a post",
                &sig.to_bytes()
            ),
            "an op signed as this candidate is not attributed to its address"
        );
        // The negative: another candidate's key must not verify against this
        // one's address, or the assertion above would hold for any key.
        let other = slate.candidate(3).unwrap();
        let other_key = crate::identity::derive_stoa_key_at_path(&master, &stoa, other.path);
        assert!(!verify_authored_op(
            &candidate.address,
            &other_key.public_key().to_bytes(),
            b"a post",
            &sign_op_bytes(&other_key, b"a post").to_bytes()
        ));
    }

    #[test]
    fn a_selection_outside_the_set_is_refused_rather_than_coerced() {
        // The spec: a selection that does not name a candidate "SHALL be refused,
        // and SHALL NOT be satisfied by any other candidate". So the assertion is
        // not merely that an error comes back but that no candidate does.
        let slate = Slate::generate(&[7u8; 32], &a_stoa()).unwrap();
        for index in [SLATE_SIZE, SLATE_SIZE + 1, 99, usize::MAX] {
            match slate.candidate(index) {
                Err(OnboardingError::NoSuchCandidate { index: got, of }) => {
                    assert_eq!(got, index);
                    assert_eq!(of, SLATE_SIZE);
                }
                other => panic!("selection {index} was not refused: {other:?}"),
            }
        }
        // And every in-range index IS satisfied, or the refusal above could be
        // unconditional and these assertions would still pass.
        for index in 0..SLATE_SIZE {
            assert_eq!(slate.candidate(index).unwrap().index, index);
        }
    }

    #[test]
    fn a_nonce_survives_a_hex_round_trip_and_parses_strictly() {
        let nonce = a_nonce();
        assert_eq!(SlateNonce::from_hex(&nonce.to_hex()).unwrap(), nonce);

        // The parse meets caller input, and a nonce accepted after truncation
        // would name a DIFFERENT slate — so keeping from it would store an
        // identity the user did not choose.
        assert_eq!(
            SlateNonce::from_hex("nothex!!"),
            Err(OnboardingError::NonceNotHex)
        );
        assert_eq!(
            SlateNonce::from_hex(""),
            Err(OnboardingError::NonceWrongLength(0))
        );
        assert_eq!(
            SlateNonce::from_hex("00ff"),
            Err(OnboardingError::NonceWrongLength(2))
        );
        // 33 bytes — one too many, the case a `>=` length check would wave
        // through.
        assert_eq!(
            SlateNonce::from_hex(&"ab".repeat(33)),
            Err(OnboardingError::NonceWrongLength(33))
        );
    }

    #[test]
    fn a_slate_carries_no_secret_anywhere_in_its_candidates() {
        // The spec: no field of the reply contains the master key's bytes, and
        // none contains any candidate's secret key bytes. Checked by searching
        // the concatenation of every byte a candidate exposes for each secret —
        // which is stronger than checking the fields by name, because it catches
        // a field somebody adds later.
        //
        // **The master key is NOT `[7u8; 32]` here, and that is the fixture's
        // whole point.** Every other test in this file uses `[7; 32]` for the
        // master key and `[7; 32]` for the nonce, which is harmless where the two
        // are never compared. Here it made the test FAIL on its first run — the
        // search found the nonce, which a slate legitimately exposes, and reported
        // it as the master key. Two explanations, one answer, which is exactly the
        // fixture defect this project has shipped before. Distinct values are what
        // make the assertion about the thing it names.
        let master = [0xa5u8; 32];
        let stoa = a_stoa();
        let slate = Slate::from_nonce(&master, &stoa, a_nonce()).unwrap();
        assert_ne!(
            &master,
            slate.nonce.as_bytes(),
            "the master key and the nonce must differ, or this test cannot tell \
             which one it found"
        );

        let mut exposed: Vec<u8> = Vec::new();
        exposed.extend_from_slice(slate.nonce.as_bytes());
        for c in &slate.candidates {
            exposed.extend_from_slice(&c.path.to_be_bytes());
            exposed.extend_from_slice(c.address.as_bytes());
            exposed.extend_from_slice(&c.public_key.to_bytes());
        }

        assert!(
            !contains_window(&exposed, &master),
            "the master key's bytes appear in what a slate exposes"
        );
        for c in &slate.candidates {
            let secret =
                crate::identity::derive_stoa_key_at_path(&master, &stoa, c.path).to_bytes();
            assert!(
                !contains_window(&exposed, &secret),
                "candidate {}'s secret key bytes appear in what a slate exposes",
                c.index
            );
        }

        // The detection itself must work, or the assertions above prove nothing.
        // A value that IS in there must be found.
        assert!(
            contains_window(&exposed, slate.candidates[0].address.as_bytes()),
            "the search is broken, so the assertions above prove nothing"
        );
    }

    #[test]
    fn no_value_a_slate_exposes_can_sign_as_any_candidate() {
        // The spec's sharpest form of the no-secret requirement: "when every value
        // in a slate reply is taken as key material, none of them yields a
        // signature that verifies under any candidate's public key".
        //
        // Stronger than the byte search above, because it would also catch a value
        // that is a secret in a DIFFERENT encoding — reversed, or transformed —
        // which a substring search cannot see.
        //
        // A master key distinct from the nonce, for the reason
        // `a_slate_carries_no_secret_anywhere_in_its_candidates` records: where the
        // two are equal, a finding about one cannot be told from a finding about
        // the other.
        let master = [0xa5u8; 32];
        let stoa = a_stoa();
        let slate = Slate::from_nonce(&master, &stoa, a_nonce()).unwrap();

        // Every 32-byte value a slate exposes: the nonce, and each candidate's
        // address and public key. The master key is deliberately NOT in this list
        // — it is not something a slate exposes, which is what the test above
        // asserts, and putting it here would be testing a different claim.
        let mut as_key_material: Vec<[u8; 32]> = vec![*slate.nonce.as_bytes()];
        for c in &slate.candidates {
            as_key_material.push(*c.address.as_bytes());
            as_key_material.push(c.public_key.to_bytes());
        }

        for material in &as_key_material {
            // Every 32-byte string is a valid Ed25519 seed, so each of these
            // yields a key that signs. The question is whether it signs as any
            // candidate.
            let key = SecretKey::from_bytes(material).unwrap();
            let sig = sign_op_bytes(&key, b"a post");
            for c in &slate.candidates {
                assert!(
                    !crate::identity::verify_op_bytes(&c.public_key, b"a post", &sig),
                    "a slate value signed as candidate {}",
                    c.index
                );
            }
        }

        // And the control: the real secret DOES sign as its candidate, so the
        // assertions above are about the values being wrong rather than about
        // verification never succeeding.
        let real =
            crate::identity::derive_stoa_key_at_path(&master, &stoa, slate.candidates[0].path);
        assert!(
            crate::identity::verify_op_bytes(
                &slate.candidates[0].public_key,
                b"a post",
                &sign_op_bytes(&real, b"a post")
            ),
            "verification never succeeds, so the assertions above prove nothing"
        );
    }

    #[test]
    fn every_error_message_names_a_fix() {
        // The obligation `KeystoreError::Display` carries. These strings reach a
        // view, and a message a user cannot act on is one not worth carrying.
        let errors = [
            OnboardingError::NoRandomness,
            OnboardingError::NonceNotHex,
            OnboardingError::NonceWrongLength(3),
            OnboardingError::NonceIsNotTheLiveSlate,
            OnboardingError::NoLiveSlate,
            OnboardingError::NoSuchCandidate { index: 9, of: 5 },
            OnboardingError::Derivation,
        ];
        for e in errors {
            let message = e.to_string();
            assert!(
                ["try again", "pass back", "generate", "choose", "report"]
                    .iter()
                    .any(|verb| message.contains(verb)),
                "this message names no fix: {message}"
            );
        }
    }

    #[test]
    fn the_no_such_candidate_message_names_a_range_a_caller_can_use() {
        // The range is computed from the slate's size, and `saturating_sub` is
        // there because a zero-length slate would otherwise underflow — which is
        // not reachable, and a panic in a Display impl would abort the module
        // process all the same.
        assert!(OnboardingError::NoSuchCandidate { index: 9, of: 5 }
            .to_string()
            .contains("0 to 4"));
        assert!(OnboardingError::NoSuchCandidate { index: 0, of: 0 }
            .to_string()
            .contains("0 to 0"));
    }

    /// Whether `haystack` contains `needle` as a contiguous run.
    ///
    /// Hand-rolled rather than reached for from a crate: one need, in tests.
    fn contains_window(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }
}
