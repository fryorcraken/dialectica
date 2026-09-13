//! The Stoa genesis record: what a Stoa *is*, and how it becomes an address.
//!
//! # Why this file exists
//!
//! A Stoa is a genesis record its creator publishes (PLAN.md §1). That is what
//! makes creation permissionless — there is no registry to register with, so
//! the record itself has to carry everything needed to identify the Stoa.
//!
//! [`identity::stoa_address`] already hashes a record, but it takes opaque
//! `&[u8]`: nothing said what those bytes *were*. Without a canonical encoding
//! two peers holding the same Stoa could compute different addresses for it,
//! and §4.8's promise that pasting an address is enough to verify what you
//! joined would be unenforceable — there would be nothing to check against.
//!
//! # The encoding, and the two traps it is shaped around
//!
//! **Every variable-length field is length-prefixed.** Concatenating two
//! variable-length fields lets distinct records collide: `("ab", "c")` and
//! `("a", "bc")` produce identical bytes and therefore an identical address for
//! two different Stoas. `identity.rs` avoided this by putting its fixed-width
//! field first, which works for two fields and stops working at three — a
//! prefix is the fix that keeps working.
//!
//! **Decoding is strict.** A genesis record arrives from a peer, so it is
//! attacker-controlled (CLAUDE.md's security posture: validate at the boundary,
//! before anything reaches a state machine). Truncation, trailing bytes, a
//! lying length prefix, an unknown policy and an unknown version are each
//! refused rather than absorbed.
//!
//! **Two different bounds govern the title length prefix**, and confusing them
//! is how one of them silently stops being checked.
//!
//! 1. **The cap.** A claim over [`MAX_TITLE_BYTES`] is refused as
//!    `TitleTooLong`, checked *before* the read so it costs nothing, and
//!    enforced on the encode side too — so the decoder refuses exactly what the
//!    encoder declines to produce.
//! 2. **The available input.** A claim under the cap but past what the buffer
//!    holds is refused as `LengthMismatch`.
//!
//! The cap is checked first, which is worth knowing before writing a test
//! against either: a claim above the cap never reaches the input comparison, so
//! a test that reaches for `u32::MAX` to exercise the lying-prefix path
//! silently stops exercising it. Each bound has its own boundary pair, and
//! `the_two_length_bounds_are_reported_distinguishably` pins that they do not
//! collapse into one error.
//!
//! **Neither bound makes a record fit an SDS message, and nothing here does.**
//! The cap bounds one field; a record is that field plus a fixed header. Today
//! that cannot exceed the 150 KiB message limit, but this module does not check
//! it and should not be read as promising it — the same "a cap is not a message
//! bound" gap `op.rs` has, where several capped fields can sum past the limit.
//!
//! **The right home for that check is the transport boundary**, where the SDS
//! frame is actually visible. This decoder is handed a `&[u8]` and has no way
//! to know whether it arrived in one message, was read from local storage, or
//! was assembled by a caller, so a message-size limit here would be guessing at
//! a constraint it cannot observe.
//!
//! # What is deliberately not here
//!
//! **No policy enforcement.** The record *declares* a policy; nothing checks a
//! poster against it yet. `Open` needs no check, which is why it is the variant
//! that ships first.
//!
//! **No mutable metadata and no moderator set.** Editable title/description is
//! moderator-scoped state and belongs with mutable moderation. The creator is
//! the sole moderator (§6), which follows from `creator` without storing a set.
//!
//! **No per-peer value of any kind** — no session counter, no local sequence
//! number. The record is hashed by every peer to obtain the Stoa's address, so
//! a field that varies with one peer's history gives that peer a different
//! address for the same Stoa — which is not an error anyone sees, it is two
//! Stoas that cannot see each other. §4.3 states the same rule for the channel
//! id, which is derived from this address: it "can carry no per-peer state".

use crate::cursor::{Cursor, OutOfBounds};
use crate::identity::{stoa_address, Address, KeyError, PublicKey};

/// The encoding generation.
///
/// First byte of every record, and part of the address. A genesis record is
/// immutable and address-determining, so a new field cannot be added in
/// place — it changes the address of every Stoa already created. This
/// discriminant is what makes an old client REFUSE a newer record legibly
/// instead of misparsing it, and what lets two generations coexist.
const VERSION_1: u8 = 1;

/// The longest a title may be, in bytes.
///
/// **Address-determining, so it cannot be added later** — raising or lowering it
/// changes which records are valid, and any Stoa created above a later cap
/// becomes undecodable. Same argument that puts `policy` in the record now
/// (PLAN.md §13).
///
/// 1 KiB is far above any plausible forum title and far below anything that
/// makes hashing or an allocation interesting. Bytes rather than characters
/// because the encoding is bytes; a title of multi-byte characters gets fewer
/// of them, which is the right trade for a bound that has to be exact.
///
/// It also makes the `as u32` cast below unrepresentable rather than merely
/// unlikely: without a bound, a 2^32-byte title encodes a length prefix of `0`
/// and the record stops round-tripping.
const MAX_TITLE_BYTES: usize = 1024;

/// How a Stoa decides who may post.
///
/// One variant today. The field exists now because PLAN.md §13 costs it out:
/// adding it in Phase 1 is an enum with one variant, adding it later means
/// migrating every Stoa already created — and there is no in-place migration
/// for an address derived from an immutable record.
///
/// Open, invite, first-post-approval and token-threshold (§7.1) are variants of
/// ONE mechanism rather than separate features, so reserving the space costs
/// nothing and keeps the later ones from disturbing the record's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Anyone may post. Needs no verification, which is why it is first.
    Open,
}

impl Policy {
    /// Every variant, so a test can iterate them.
    ///
    /// **Add every new variant here.** `policy_all_holds_every_variant_and_each_maps_to_its_pinned_byte`
    /// fails if you do not, and that test — not this comment — is what enforces it.
    ///
    /// This doc used to claim the `cargo mutants` finding
    /// *"replace `Policy::to_byte` with `0`"* was closed by a test iterating
    /// `ALL`. It is not, and **the mutant is still reported MISSED**: with one
    /// variant whose discriminant is `0`, `to_byte` and the constant `0` are the
    /// same function on the whole domain, so no test can distinguish them. It is
    /// an equivalent mutant, and the only thing that kills it is a second variant
    /// existing. Measured, not assumed — including that asserting
    /// `Policy::Open.to_byte() == 0` against a hardcoded literal survives it too.
    ///
    /// What the iterating test *did* leave open is bookkeeping: a second variant
    /// added to the enum but not to `ALL` left every iterating test green over the
    /// one entry `ALL` still had. That is what the named test above closes, with an
    /// exhaustive `match` so the enforcement is a compile error rather than a
    /// request to remember.
    pub const ALL: [Policy; 1] = [Policy::Open];

    /// Explicit discriminants: these bytes are on the wire and in the address,
    /// so they are part of the format and must not follow declaration order.
    const OPEN: u8 = 0;

    fn to_byte(self) -> u8 {
        match self {
            Policy::Open => Self::OPEN,
        }
    }

    fn from_byte(b: u8) -> Result<Self, GenesisError> {
        match b {
            Self::OPEN => Ok(Policy::Open),
            // NOT defaulted to Open. Treating an unrecognised policy as open is
            // how a token-gated Stoa silently becomes world-postable on an
            // older client. Refusing means an old client cannot display a Stoa
            // it does not understand, which is the recoverable direction.
            other => Err(GenesisError::UnknownPolicy(other)),
        }
    }
}

/// A Stoa's genesis record. Immutable, and the preimage of its address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Genesis {
    /// The creator's public key. This is what makes them the Stoa's initial
    /// sole moderator (§6) — a record without a valid one does not describe a
    /// Stoa at all.
    pub creator: PublicKey,
    /// Declared at creation and immutable thereafter.
    pub policy: Policy,
    /// Human-readable, and explicitly NOT identity: names are never unique, and
    /// §4.8 warns that announcements may impersonate a Stoa by name. The
    /// address is the identity.
    pub title: String,
}

/// Why a byte string is not a genesis record.
///
/// Each variant names a DIFFERENT mistake. A decoder that only says "invalid"
/// sends the reader looking in the wrong place — the same reasoning that makes
/// `parse_channel_id` distinguish a missing field from a wrong-typed one.
#[derive(Debug, PartialEq, Eq)]
pub enum GenesisError {
    /// A version this build does not know. Distinguishable from malformed
    /// input on purpose: it means "newer client", not "corrupt".
    UnknownVersion(u8),
    /// A policy discriminant this build does not know. Never defaulted.
    UnknownPolicy(u8),
    /// Input ended before a field did.
    Truncated,
    /// A complete record, followed by bytes that are not part of it. Refused
    /// because accepting them would let two byte strings decode to the same
    /// record while hashing to different addresses.
    TrailingBytes,
    /// A length prefix claiming more bytes than the input holds.
    LengthMismatch,
    /// The title is not valid UTF-8.
    InvalidTitle,
    /// The creator key is not a valid public key.
    InvalidCreator(KeyError),
    /// The title exceeds [`MAX_TITLE_BYTES`]. Carries the length found, since
    /// "too long" without a number leaves the caller guessing by how much.
    TitleTooLong(usize),
}

impl std::fmt::Display for GenesisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenesisError::UnknownVersion(v) => {
                write!(f, "unknown genesis record version {v}")
            }
            GenesisError::UnknownPolicy(p) => {
                write!(f, "unknown posting policy {p}")
            }
            GenesisError::Truncated => write!(f, "genesis record ended mid-field"),
            GenesisError::TrailingBytes => {
                write!(f, "trailing bytes after a complete genesis record")
            }
            GenesisError::LengthMismatch => {
                write!(f, "a length prefix disagrees with the bytes present")
            }
            GenesisError::InvalidTitle => write!(f, "title is not valid UTF-8"),
            GenesisError::InvalidCreator(e) => write!(f, "creator key: {e}"),
            GenesisError::TitleTooLong(n) => {
                write!(f, "title is {n} bytes, the maximum is {MAX_TITLE_BYTES}")
            }
        }
    }
}

impl From<OutOfBounds> for GenesisError {
    /// The shared read head reports only *that* it ran out; this says what
    /// running out means for a genesis record.
    ///
    /// Kept as a `From` rather than spelled at each call site so that every
    /// bounds failure in `decode` maps the same way. The one site that means
    /// something else — a length prefix claiming more than the input holds — is
    /// mapped explicitly there, and reads as the deliberate exception it is.
    fn from(e: OutOfBounds) -> Self {
        match e {
            OutOfBounds::Truncated => GenesisError::Truncated,
            OutOfBounds::Trailing => GenesisError::TrailingBytes,
        }
    }
}

impl Genesis {
    /// The canonical encoding. Exactly one valid byte string per record.
    ///
    /// Layout, in order:
    ///
    /// ```text
    /// version   1 byte
    /// creator   32 bytes  (fixed width)
    /// policy    1 byte
    /// title     4-byte BE length, then that many bytes of UTF-8
    /// ```
    ///
    /// Fixed-width fields come first and need no prefix. The title is
    /// length-prefixed rather than trailing-to-end-of-input, so that a second
    /// variable-length field (an invite list, a token identifier) can be added
    /// in a later version without the boundary between them becoming ambiguous.
    ///
    /// Fails for a title over [`MAX_TITLE_BYTES`]. Encoding is fallible so the
    /// bound holds on both sides: a record the decoder would reject must not be
    /// one the encoder will produce, or the two disagree about what is valid.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, GenesisError> {
        let title = self.title.as_bytes();
        if title.len() > MAX_TITLE_BYTES {
            return Err(GenesisError::TitleTooLong(title.len()));
        }
        let mut out = Vec::with_capacity(1 + 32 + 1 + 4 + title.len());
        out.push(VERSION_1);
        out.extend_from_slice(&self.creator.to_bytes());
        out.push(self.policy.to_byte());
        // The bound above is what makes this cast total: a title that could
        // truncate is refused before reaching it.
        out.extend_from_slice(&(title.len() as u32).to_be_bytes());
        out.extend_from_slice(title);
        Ok(out)
    }

    /// Decode a canonical encoding, refusing anything else.
    ///
    /// Strict by design: this input is attacker-controlled, and every lenient
    /// reading is a way for two peers to disagree about what a Stoa is.
    pub fn decode(bytes: &[u8]) -> Result<Self, GenesisError> {
        let mut cursor = Cursor::new(bytes);

        match cursor.take(1)?[0] {
            VERSION_1 => {}
            other => return Err(GenesisError::UnknownVersion(other)),
        }

        let creator_bytes = cursor.take(32)?;
        let creator = PublicKey::from_bytes(creator_bytes).map_err(GenesisError::InvalidCreator)?;

        let policy = Policy::from_byte(cursor.take(1)?[0])?;

        let len = cursor.take_length()?;
        // Checked BEFORE the read, so an over-long title costs nothing to
        // refuse — and so the decoder rejects exactly what the encoder refuses
        // to produce.
        if len > MAX_TITLE_BYTES {
            return Err(GenesisError::TitleTooLong(len));
        }
        // A prefix claiming more than the input holds is a LengthMismatch
        // rather than a Truncated: the input is not short, the claim is wrong,
        // and saying so points at the right half of the problem.
        let title_bytes = cursor
            .take(len)
            .map_err(|_| GenesisError::LengthMismatch)?
            .to_vec();
        let title = String::from_utf8(title_bytes).map_err(|_| GenesisError::InvalidTitle)?;

        // Nothing may follow a complete record.
        cursor.finish()?;

        Ok(Genesis {
            creator,
            policy,
            title,
        })
    }

    /// This Stoa's address: the hash of its canonical encoding.
    ///
    /// Prefer this over calling [`stoa_address`] with hand-assembled bytes —
    /// going through the record is what makes it impossible to hash a
    /// non-canonical encoding by accident.
    pub fn address(&self) -> Result<Address, GenesisError> {
        Ok(stoa_address(&self.canonical_bytes()?))
    }

    /// Whether this record is the one `address` names.
    ///
    /// The check that makes a pasted address self-authenticating (§4.8), and it
    /// consults nothing: no registry, no peer, no third party. That is the
    /// whole point — a wrong or tampered record fails to match, and the failure
    /// needs no one's cooperation to detect.
    /// A record too long to encode matches nothing: it has no address, so it
    /// cannot be the one any address names.
    pub fn matches(&self, address: &Address) -> bool {
        self.address().is_ok_and(|a| &a == address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SecretKey;

    fn a_key(seed: u8) -> PublicKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap().public_key()
    }

    fn a_record() -> Genesis {
        Genesis {
            creator: a_key(1),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
    }

    /// Offset of the policy byte in a canonical encoding.
    const POLICY_AT: usize = 1 + 32;
    /// Offset of the title's 4-byte length prefix.
    const TITLE_LEN_AT: usize = POLICY_AT + 1;

    #[test]
    fn encodes_identically_every_time() {
        let g = a_record();
        assert_eq!(g.canonical_bytes().unwrap(), g.canonical_bytes().unwrap());
    }

    #[test]
    fn two_records_differing_in_any_field_encode_differently() {
        // Every field must participate, or two distinct Stoas share an address.
        // Varying each in turn is what catches a field left out of the
        // encoding — which would be invisible in a round-trip test, since the
        // value would still come back from the struct it never left.
        let base = a_record();

        let different_creator = Genesis {
            creator: a_key(2),
            ..base.clone()
        };
        let different_title = Genesis {
            title: "Stoa".to_string(),
            ..base.clone()
        };

        for other in [different_creator, different_title] {
            assert_ne!(
                base.canonical_bytes().unwrap(),
                other.canonical_bytes().unwrap(),
                "a field is missing from the encoding"
            );
            assert_ne!(
                base.address().unwrap(),
                other.address().unwrap(),
                "a field is missing from the address"
            );
        }
    }

    #[test]
    fn the_title_length_is_encoded_and_not_merely_implied() {
        // The concatenation trap, and the reason the title is length-prefixed.
        //
        // The property is that the length is CARRIED, not inferred from where
        // the input happens to end. Appending a byte to a valid encoding is
        // what shows it: with a prefix the title stays put and the extra byte
        // is trailing garbage; without one the title absorbs it.
        //
        // Comparing two different-length titles looks equivalent and is not —
        // they encode differently with or without a prefix, so such a test
        // passes either way.
        let g = Genesis {
            title: "ab".to_string(),
            ..a_record()
        };
        let mut extended = g.canonical_bytes().unwrap();
        extended.push(b'c');

        assert_eq!(
            Genesis::decode(&extended),
            Err(GenesisError::TrailingBytes),
            "the title must not absorb bytes beyond its declared length"
        );

        // And the length prefix is really present in the encoding: the four
        // bytes before the title spell its length.
        let bytes = g.canonical_bytes().unwrap();
        assert_eq!(
            u32::from_be_bytes(
                bytes[TITLE_LEN_AT..TITLE_LEN_AT + 4]
                    .try_into()
                    .unwrap()
            ),
            2,
            "the title's length must be encoded ahead of it"
        );
    }

    #[test]
    fn decode_of_encode_is_the_identity() {
        for title in ["", "Agora", "Ἀγορά — the marketplace", "🏛"] {
            let g = Genesis {
                title: title.to_string(),
                ..a_record()
            };
            assert_eq!(Genesis::decode(&g.canonical_bytes().unwrap()).unwrap(), g);
        }
    }

    #[test]
    fn truncation_at_any_point_is_refused() {
        // Every prefix of a valid record, not just a couple of hand-picked
        // lengths: a decoder that reads one field without a bounds check fails
        // only at the boundary that field happens to straddle.
        let bytes = a_record().canonical_bytes().unwrap();
        for n in 0..bytes.len() {
            let err = Genesis::decode(&bytes[..n]).unwrap_err();
            assert!(
                matches!(
                    err,
                    GenesisError::Truncated | GenesisError::LengthMismatch
                ),
                "truncating to {n} bytes gave {err:?}"
            );
        }
    }

    #[test]
    fn trailing_bytes_are_refused() {
        // Accepting them would let two byte strings decode to the same record
        // while hashing to different addresses — the exact ambiguity canonical
        // encoding exists to remove.
        let mut bytes = a_record().canonical_bytes().unwrap();
        bytes.push(0);
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::TrailingBytes));
    }

    #[test]
    fn every_error_renders_without_leaking_rust_syntax() {
        // This is the decoder for peer bytes, so it is the first error type to
        // reach the `{"error":"..."}` wire contract. `format!("{:?}")` would put
        // `UnknownPolicy(99)` — a Rust type name — in a user-facing field.
        //
        // Asserting the absence of `(` and `::` is what makes this fail if
        // someone derives Display or falls back to Debug, rather than only
        // checking that some string came out.
        //
        // The `match` below is the reason this list cannot silently fall behind
        // the enum. A bare array would compile forever while covering fewer and
        // fewer variants — which is exactly what happened when `TitleTooLong`
        // was added. Adding a variant now fails to compile until it is listed.
        fn every_variant() -> Vec<GenesisError> {
            let all = vec![
                GenesisError::UnknownVersion(9),
                GenesisError::UnknownPolicy(99),
                GenesisError::Truncated,
                GenesisError::TrailingBytes,
                GenesisError::LengthMismatch,
                GenesisError::InvalidTitle,
                GenesisError::InvalidCreator(KeyError::NotAValidPublicKey),
                // The weak-key case renders through KeyError's own Display, so
                // it is a second path worth covering rather than a repeat.
                GenesisError::InvalidCreator(KeyError::WeakPublicKey),
                GenesisError::TitleTooLong(2000),
            ];
            // Non-exhaustive match => compile error when a variant is added.
            // Never executed; it exists only to make the compiler check the
            // list above.
            if let Some(e) = all.first() {
                match e {
                    GenesisError::UnknownVersion(_)
                    | GenesisError::UnknownPolicy(_)
                    | GenesisError::Truncated
                    | GenesisError::TrailingBytes
                    | GenesisError::LengthMismatch
                    | GenesisError::InvalidTitle
                    | GenesisError::InvalidCreator(_)
                    | GenesisError::TitleTooLong(_) => {}
                }
            }
            all
        }
        let errors = every_variant();
        // Distinguishability is a spec requirement — "a decoder that says only
        // 'invalid' sends the reader looking in the wrong place" — and the
        // format assertions below do not check it: collapsing three arms to the
        // same string passes them. Pairwise distinctness is what catches that.
        let mut seen = std::collections::HashSet::new();
        for e in &errors {
            assert!(
                seen.insert(e.to_string()),
                "{e:?} renders identically to another variant: {}",
                e
            );
        }

        for e in errors {
            let rendered = e.to_string();
            assert!(!rendered.is_empty(), "{e:?} rendered empty");
            assert!(
                !rendered.contains('(') && !rendered.contains("::"),
                "{e:?} rendered as Rust syntax: {rendered}"
            );
        }
    }

    #[test]
    fn a_lying_length_prefix_is_refused() {
        let mut bytes = a_record().canonical_bytes().unwrap();
        // Claim more than the input holds, but stay UNDER MAX_TITLE_BYTES — the
        // bound is checked first, so a `u32::MAX` claim would be refused as
        // TitleTooLong and this test would stop exercising the lying-prefix
        // path it is named for.
        let claim = (MAX_TITLE_BYTES - 1) as u32;
        bytes[TITLE_LEN_AT..TITLE_LEN_AT + 4].copy_from_slice(&claim.to_be_bytes());
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::LengthMismatch));
    }

    /// Overwrite the title's length prefix with `claim`.
    fn with_title_length_claim(bytes: &mut [u8], claim: u32) {
        bytes[TITLE_LEN_AT..TITLE_LEN_AT + 4].copy_from_slice(&claim.to_be_bytes());
    }

    #[test]
    fn the_available_input_boundary_accepts_the_largest_fit_and_refuses_one_more() {
        // A BOUNDARY PAIR for the SECOND bound, which is easy to miss because
        // the first one now has its own pair.
        //
        // Two different limits govern a title length prefix, and they fail
        // with different errors:
        //
        //   1. the CAP    — `len > MAX_TITLE_BYTES` => TitleTooLong.
        //                   Pinned by `a_title_at_the_maximum_is_accepted` and
        //                   `a_title_over_the_maximum_is_refused_on_both_sides`.
        //   2. the INPUT  — a claim under the cap but past what the buffer
        //                   holds => LengthMismatch. That is this test.
        //
        // Both are boundaries and both can drift independently. The cap pair
        // says nothing about (2): it varies lengths around 1024 while the
        // record stays self-consistent, so a decoder that stopped comparing the
        // claim against the remaining input would keep passing it.
        //
        // Stays well under MAX_TITLE_BYTES on purpose, for the reason
        // `a_lying_length_prefix_is_refused` now documents: the cap is checked
        // first, so a claim above it is refused as TitleTooLong and never
        // reaches the input comparison this test exists to pin.
        let g = Genesis {
            title: "abcdef".to_string(),
            ..a_record()
        };
        let fits = g.title.len() as u32;
        assert!(
            (fits as usize) < MAX_TITLE_BYTES,
            "the fixture must sit below the cap, or this tests the wrong bound"
        );

        // Exactly what the input holds: accepted, and yields the real record.
        // Asserting the VALUE rather than `is_ok()` is what rules out a decoder
        // that accepted the length and then returned something else.
        let mut ok = g.canonical_bytes().unwrap();
        with_title_length_claim(&mut ok, fits);
        assert_eq!(
            Genesis::decode(&ok).unwrap(),
            g,
            "the largest claim the input satisfies must be accepted"
        );

        // One byte more than the input holds: refused, with the specific
        // variant. Asserting the variant rather than `is_err()` keeps this from
        // passing on some unrelated rejection — and specifically distinguishes
        // it from TitleTooLong, which is the other way a length prefix dies.
        let mut over = g.canonical_bytes().unwrap();
        with_title_length_claim(&mut over, fits + 1);
        assert_eq!(
            Genesis::decode(&over),
            Err(GenesisError::LengthMismatch),
            "one byte past what the input holds must be a LengthMismatch"
        );
    }

    #[test]
    fn the_two_length_bounds_are_reported_distinguishably() {
        // The cap and the input bound must not collapse into one error.
        //
        // A caller matching on TitleTooLong to say "that title is too long for
        // a Stoa" and on LengthMismatch to say "this record is corrupt" gets
        // both wrong if the decoder reports either for both. The two tests
        // above each pin one side; this pins that they stay different, which
        // neither does on its own.
        let mut over_cap = a_record().canonical_bytes().unwrap();
        with_title_length_claim(&mut over_cap, (MAX_TITLE_BYTES + 1) as u32);

        let mut past_input = a_record().canonical_bytes().unwrap();
        with_title_length_claim(&mut past_input, (MAX_TITLE_BYTES - 1) as u32);

        assert_eq!(
            Genesis::decode(&over_cap),
            Err(GenesisError::TitleTooLong(MAX_TITLE_BYTES + 1))
        );
        assert_eq!(
            Genesis::decode(&past_input),
            Err(GenesisError::LengthMismatch)
        );
    }

    #[test]
    fn a_title_over_the_maximum_is_refused_on_both_sides() {
        // The bound must hold symmetrically: a record the decoder refuses must
        // not be one the encoder will produce, or the two disagree about what
        // is valid and a peer can hold a record it cannot re-derive an address
        // for.
        let too_long = Genesis {
            title: "x".repeat(MAX_TITLE_BYTES + 1),
            ..a_record()
        };
        assert_eq!(
            too_long.canonical_bytes(),
            Err(GenesisError::TitleTooLong(MAX_TITLE_BYTES + 1)),
            "the encoder must refuse an over-long title"
        );
        assert!(
            !too_long.matches(&a_record().address().unwrap()),
            "a record with no encoding matches no address"
        );

        // And the decode side, reached with a hand-built prefix claiming a
        // length over the bound. Refused BEFORE the read, so it costs nothing.
        let mut bytes = a_record().canonical_bytes().unwrap();
        let claim = (MAX_TITLE_BYTES + 1) as u32;
        bytes[TITLE_LEN_AT..TITLE_LEN_AT + 4].copy_from_slice(&claim.to_be_bytes());
        assert_eq!(
            Genesis::decode(&bytes),
            Err(GenesisError::TitleTooLong(MAX_TITLE_BYTES + 1)),
            "the decoder must refuse an over-long title"
        );
    }

    #[test]
    fn a_title_at_the_maximum_is_accepted() {
        // The boundary itself is inclusive. Without this, a fencepost error in
        // either check is invisible — both directions still "refuse something
        // long" and every other test passes.
        let at_limit = Genesis {
            title: "x".repeat(MAX_TITLE_BYTES),
            ..a_record()
        };
        let bytes = at_limit
            .canonical_bytes()
            .expect("a title of exactly MAX_TITLE_BYTES must encode");
        assert_eq!(Genesis::decode(&bytes).unwrap(), at_limit);
    }

    #[test]
    fn a_length_prefix_shorter_than_the_title_is_refused() {
        // The other direction of a lying prefix. Under-claiming is refused as
        // TrailingBytes rather than LengthMismatch — the decoder reads the
        // title it was promised, then finds bytes after it. Different error,
        // same refusal, and the distinction is worth pinning: a caller that
        // matched only on LengthMismatch would mishandle this.
        let g = Genesis {
            title: "abcdef".to_string(),
            ..a_record()
        };
        let mut bytes = g.canonical_bytes().unwrap();
        bytes[TITLE_LEN_AT..TITLE_LEN_AT + 4].copy_from_slice(&2u32.to_be_bytes());
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::TrailingBytes));
    }

    #[test]
    fn an_unknown_policy_is_refused_rather_than_defaulted() {
        // The security-relevant one. Defaulting an unrecognised policy to Open
        // is how a token-gated Stoa becomes world-postable on an old client.
        let mut bytes = a_record().canonical_bytes().unwrap();
        bytes[POLICY_AT] = 99;
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::UnknownPolicy(99)));
    }

    #[test]
    fn an_unknown_version_is_refused_and_says_so() {
        // Distinguishable from `Truncated` on purpose: this one means "a newer
        // client wrote this", which is a different thing to tell a user than
        // "this data is corrupt".
        let mut bytes = a_record().canonical_bytes().unwrap();
        bytes[0] = 99;
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::UnknownVersion(99)));
    }

    #[test]
    fn an_invalid_creator_key_is_refused() {
        // Roughly half of all 32-byte strings are not valid Edwards points, so
        // this branch is reachable from any peer that sends a malformed record
        // — not a theoretical arm. It is also the branch enforcing this
        // module's own claim that "a record without a valid [creator] does not
        // describe a Stoa at all", which was asserted by nothing until review
        // pointed out every other error variant had a test and this one did not.
        let mut bytes = a_record().canonical_bytes().unwrap();
        // `[0x02; 32]` is not a valid compressed Edwards point. Picked by
        // probing rather than assumed: all-ones IS valid, so the obvious
        // "obviously bogus" constant would have made this test pass for the
        // wrong reason.
        for b in bytes.iter_mut().skip(1).take(32) {
            *b = 0x02;
        }
        assert_eq!(
            Genesis::decode(&bytes),
            Err(GenesisError::InvalidCreator(KeyError::NotAValidPublicKey))
        );
    }

    #[test]
    fn a_creator_key_that_can_never_verify_is_refused_distinguishably() {
        // The spec's scenario "A creator key that can never verify a signature
        // is refused" was asserted by NOTHING at this boundary until now, and
        // the gap was found by writing the identity spec rather than by any
        // gate. Measured: with the `is_weak()` check disabled in
        // `PublicKey::from_bytes`, every test in this file stayed green —
        // including `an_invalid_creator_key_is_refused`, which uses `[0x02; 32]`
        // and therefore only ever exercises the OTHER refusal.
        //
        // That is the fixture trap this project keeps paying for: the existing
        // test exercises the creator-key path where one of the two rules is
        // silent, so it cannot tell the two apart. This one makes them disagree
        // — the key here is well-formed (it decompresses; that is precisely
        // what makes it dangerous) and is refused for the other reason.
        //
        // Why it matters one layer up, from `identity.rs`'s own argument: a
        // record naming a low-order creator decodes, hashes to a stable address
        // and self-authenticates, producing a forum whose sole moderator (§6)
        // can never authorise anything — indistinguishable from a legitimate
        // one by any later check.
        let mut bytes = a_record().canonical_bytes().unwrap();
        // All-zeros: the low-order point an attacker would reach for, and the
        // case the genesis record makes dangerous.
        for b in bytes.iter_mut().skip(1).take(32) {
            *b = 0x00;
        }
        assert_eq!(
            Genesis::decode(&bytes),
            Err(GenesisError::InvalidCreator(KeyError::WeakPublicKey)),
            "a creator key that can never verify must be refused, and \
             distinguishably from a malformed one"
        );
    }

    #[test]
    fn an_invalid_title_encoding_is_refused() {
        let g = a_record();
        let mut bytes = g.canonical_bytes().unwrap();
        // Replace the title's first byte with a lone continuation byte.
        let title_at = TITLE_LEN_AT + 4;
        bytes[title_at] = 0x80;
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::InvalidTitle));
    }

    #[test]
    fn a_record_verifies_against_its_own_address() {
        let g = a_record();
        assert!(g.matches(&g.address().unwrap()));
    }

    #[test]
    fn a_substituted_record_fails_verification() {
        // §4.8: a Stoa address in a post is attacker-supplied content, so the
        // check that matters is that a DIFFERENT record cannot pass for the one
        // an address names.
        let real = a_record();
        let impostor = Genesis {
            creator: a_key(2),
            ..real.clone()
        };
        assert!(!impostor.matches(&real.address().unwrap()));
    }

    #[test]
    fn two_stoas_with_the_same_title_have_different_addresses() {
        // The title is not identity. §4.8 warns that announcements may
        // impersonate a Stoa by name; this is the property that makes that
        // impersonation detectable.
        let one = a_record();
        let two = Genesis {
            creator: a_key(2),
            ..a_record()
        };
        assert_eq!(one.title, two.title);
        assert_ne!(one.address().unwrap(), two.address().unwrap());
    }

    #[test]
    fn the_title_reaches_the_address() {
        // The other half of "the title is not identity": the test above varies
        // only the creator, so it stays green even if `address()` ignored the
        // title entirely. Two Stoas by the SAME creator differing only in title
        // must still be distinct, or renaming a Stoa would silently collide it
        // with another of the creator's.
        let one = a_record();
        let two = Genesis {
            title: "A different name".to_string(),
            ..a_record()
        };
        assert_eq!(one.creator, two.creator);
        assert_ne!(one.address().unwrap(), two.address().unwrap());
    }

    #[test]
    fn the_wire_format_is_pinned_to_a_known_answer() {
        // EVERY constant here is consensus-critical. The version byte and the
        // policy discriminant are both inside the address preimage, so changing
        // either re-mints the address of every Stoa in existence — with no error
        // anywhere, because each peer stays internally consistent. Two peers on
        // different builds simply stop seeing the same Stoa.
        //
        // Every other test in this module is SELF-CONSISTENT: it compares the
        // encoder's output against the constants the encoder just wrote, so it
        // passes unchanged if someone edits one. This one does not, which is
        // its whole job — hence hardcoded hex rather than values recomputed
        // from the constants. `identity.rs` does the same in
        // `the_wire_constants_are_pinned_to_known_answers`.
        //
        // If this fails, do NOT update the expected values to match. Work out
        // what changed and whether the network can survive it.
        // The constants themselves, asserted DIRECTLY and not only through the
        // encoding below. `cargo mutants` structurally cannot see a wrong
        // `const` — it mutates functions, not constants — so a constant is only
        // ever pinned by an assertion someone wrote on purpose. This repo has
        // already shipped a VERSION_1 defect that left the whole suite green
        // for exactly that reason.
        //
        // The hex blob below does cover these bytes, but it covers them
        // incidentally: a reader auditing "is the policy discriminant pinned?"
        // has to decode the blob by hand to find out. These two lines answer it.
        assert_eq!(VERSION_1, 1, "the genesis encoding version changed");
        assert_eq!(Policy::OPEN, 0, "the open policy discriminant changed");

        let g = a_record();

        assert_eq!(
            hex::encode(g.canonical_bytes().unwrap()),
            "018a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c\
             000000000541676f7261",
            "the genesis wire format changed"
        );
        assert_eq!(
            g.address().unwrap().to_hex(),
            "80329cf05603a0c9ce7a749a53e271253307ba89d4924856e4017459d03a025f",
            "Stoa address derivation changed"
        );

        // The title bound is interop, not a local preference: a peer at 1024
        // encodes a 900-byte title that a peer at 777 refuses, and the two
        // silently disagree about what is a valid Stoa. Every other test
        // recomputes from the constant and so survives a change to it; this
        // absolute value is what makes changing it a decision rather than an
        // edit.
        assert_eq!(MAX_TITLE_BYTES, 1024, "the title bound is network-visible");
    }

    #[test]
    fn every_policy_round_trips_through_its_discriminant() {
        // Found by `cargo mutants`: replacing `to_byte` with a hardcoded `0`
        // survived the whole suite, because `Policy::OPEN` IS 0 while there is
        // one variant. The moment a second lands, a `to_byte` that ignored its
        // input would encode every policy as Open — silently, since the
        // discriminant is inside the address.
        //
        // Iterating every variant is what makes this fail then. It does NOT make
        // the mutant die today — that mutant is still reported MISSED, and is
        // unkillable while `Policy` has one inhabitant whose byte is `0`. Nor does
        // iterating `Policy::ALL` catch a variant that was never added to `ALL`:
        // this test would keep passing over the one entry it has.
        // `policy_all_holds_every_variant_and_each_maps_to_its_pinned_byte` is
        // what closes that, and it carries the measurements.
        for policy in Policy::ALL {
            assert_eq!(
                Policy::from_byte(policy.to_byte()),
                Ok(policy),
                "{policy:?} did not round-trip through its discriminant"
            );
        }
        // Distinct variants must not share a byte, or two policies collide.
        let mut seen = std::collections::HashSet::new();
        for policy in Policy::ALL {
            assert!(
                seen.insert(policy.to_byte()),
                "{policy:?} reuses a discriminant"
            );
        }
    }

    #[test]
    fn policy_all_holds_every_variant_and_each_maps_to_its_pinned_byte() {
        // WHY THIS EXISTS, and what the `Policy::ALL` doc comment used to claim
        // and could not deliver.
        //
        // `cargo mutants` reports `replace Policy::to_byte -> u8 with 0` as a
        // MISSED mutant, and it still does at the time of writing. That is not a
        // coverage gap that a better test can close: `Policy` has exactly one
        // inhabitant and its discriminant IS 0, so `to_byte` and the constant `0`
        // are the SAME FUNCTION on the whole domain. No fixture can tell them
        // apart — measured, not argued: asserting
        // `Policy::Open.to_byte() == 0` against a hardcoded literal (the fix
        // `findings/security.md` entry 5 proposed) passes under the mutation too.
        // It is an EQUIVALENT MUTANT, and the only thing that kills it is a
        // second variant existing.
        //
        // So what is actually at risk is not `to_byte`'s body. It is the
        // bookkeeping the round-trip test above depends on: that second variant
        // being listed in `Policy::ALL`. `ALL`'s doc says "a test iterating this
        // fails when a new variant is added without a discriminant" — but a
        // variant added to the enum and NOT added to `ALL` leaves every iterating
        // test still green over the one entry it does have, so the promise did
        // not hold by itself.
        //
        // The `match` below is what makes it hold, and it holds at COMPILE time
        // rather than by anyone remembering: adding a variant to `Policy` without
        // adding it here is a non-exhaustive-match error, and the error points at
        // the table that must grow.
        //
        // The bytes are HARDCODED LITERALS, not `Self::OPEN`. Reading the
        // discriminant back out of the implementation would be the
        // "ask the code what it wrote and agree" shape — and `Policy::OPEN` is a
        // `const`, which `cargo mutants` cannot mutate at all, so an assertion
        // written against it is the one thing no gate can check.
        //
        // If this fails, do NOT update the expected byte to match. The
        // discriminant is inside the address preimage, so changing it re-mints
        // every Stoa address in existence with no error anywhere.

        // The pinned table: one row per variant, each byte a hardcoded literal.
        let pinned: &[(Policy, u8)] = &[(Policy::Open, 0)];

        // What forces a new variant into that table. This `match` binds nothing
        // and computes nothing — its only job is to be exhaustive, so adding a
        // variant to `Policy` without adding a row above is a COMPILE error whose
        // message names this line. A comment asking the next author to remember is
        // what this replaces, because the old one did exactly that and the
        // remembering is the part that does not hold.
        for (policy, _) in pinned {
            match policy {
                Policy::Open => {}
            }
        }
        assert_eq!(
            pinned.len(),
            Policy::ALL.len(),
            "the pinned table and Policy::ALL disagree about how many variants exist"
        );

        for (policy, byte) in pinned {
            assert_eq!(
                policy.to_byte(),
                *byte,
                "{policy:?} no longer encodes as the byte the format pins"
            );
            // `ALL` is what the round-trip test above iterates, so a variant
            // pinned here and missing there would leave that test green over a
            // domain it no longer covers. This is the assertion that catches it.
            assert!(
                Policy::ALL.contains(policy),
                "{policy:?} is pinned here but missing from Policy::ALL, \
                 which every iterating test in this module walks"
            );
        }
    }

    #[test]
    fn a_genesis_record_yields_a_32_byte_address() {
        // The type makes this unbreakable today (`Address` wraps `[u8; 32]`),
        // but the spec states it and a reimplementation would work from the
        // spec. It becomes a real check the moment `Address` gains a second
        // constructor.
        assert_eq!(a_record().address().unwrap().as_bytes().len(), 32);
    }

    #[test]
    fn the_encoding_is_exactly_as_long_as_the_layout_says() {
        // A field cannot be dropped while another silently slides into its
        // place. Complements the known-answer test above: that one catches a
        // changed VALUE, this one catches a changed SHAPE for any title.
        for title in ["", "Agora", "🏛"] {
            let g = Genesis {
                title: title.to_string(),
                ..a_record()
            };
            assert_eq!(
                g.canonical_bytes().unwrap().len(),
                TITLE_LEN_AT + 4 + title.len(),
                "unexpected encoding length for title {title:?}"
            );
        }
    }
}

