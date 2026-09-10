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
//! **No epoch, session counter, or any other per-peer value.** The record is
//! hashed by every peer to obtain the Stoa's address, so a field that varies
//! with one peer's history gives that peer a different address for the same
//! Stoa — which is not an error anyone sees, it is two Stoas that cannot see
//! each other. §4.3 states the same rule for the channel id, which is derived
//! from this address: it "can carry no per-peer state".

use crate::identity::{stoa_address, Address, KeyError, PublicKey};

/// The encoding generation.
///
/// First byte of every record, and part of the address. A genesis record is
/// immutable and address-determining, so a new field cannot be added in
/// place — it changes the address of every Stoa already created. This
/// discriminant is what makes an old client REFUSE a newer record legibly
/// instead of misparsing it, and what lets two generations coexist.
const VERSION_1: u8 = 1;

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
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let title = self.title.as_bytes();
        let mut out = Vec::with_capacity(1 + 32 + 1 + 4 + title.len());
        out.push(VERSION_1);
        out.extend_from_slice(&self.creator.to_bytes());
        out.push(self.policy.to_byte());
        // `as u32` cannot truncate meaningfully here: a title long enough to
        // overflow u32 is far past any size this record is ever encoded at, and
        // the length is checked against the input on the way back in.
        out.extend_from_slice(&(title.len() as u32).to_be_bytes());
        out.extend_from_slice(title);
        out
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

        let mut len = [0u8; 4];
        len.copy_from_slice(cursor.take(4)?);
        let len = u32::from_be_bytes(len) as usize;
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
    pub fn address(&self) -> Address {
        stoa_address(&self.canonical_bytes())
    }

    /// Whether this record is the one `address` names.
    ///
    /// The check that makes a pasted address self-authenticating (§4.8), and it
    /// consults nothing: no registry, no peer, no third party. That is the
    /// whole point — a wrong or tampered record fails to match, and the failure
    /// needs no one's cooperation to detect.
    pub fn matches(&self, address: &Address) -> bool {
        &self.address() == address
    }
}

/// A bounds-checked read head.
///
/// Exists so that "did the input end?" is asked in ONE place. Hand-rolled
/// slicing at each field is how a decoder acquires a panicking index — and a
/// panic here is reached from inbound peer data, where PHASE0-FINDINGS §3
/// measured what an unguarded panic costs: the module process aborts.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Cursor { bytes, at: 0 }
    }

    /// The next `n` bytes, or `Truncated`. Never panics, never wraps:
    /// `checked_add` because `at + n` on a hostile length could overflow and
    /// wrap to a value that passes a naive bounds check.
    fn take(&mut self, n: usize) -> Result<&'a [u8], GenesisError> {
        let end = self.at.checked_add(n).ok_or(GenesisError::Truncated)?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or(GenesisError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn finish(self) -> Result<(), GenesisError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(GenesisError::TrailingBytes)
        }
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
        assert_eq!(g.canonical_bytes(), g.canonical_bytes());
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
                base.canonical_bytes(),
                other.canonical_bytes(),
                "a field is missing from the encoding"
            );
            assert_ne!(
                base.address(),
                other.address(),
                "a field is missing from the address"
            );
        }
    }

    #[test]
    fn the_title_length_is_encoded_and_not_merely_implied() {
        // The concatenation trap, and the reason the title is length-prefixed.
        //
        // An earlier version of this test compared "ab" with "abc" and PASSED
        // even with the prefix deleted — different-length titles produce
        // different bytes either way, so it proved nothing about the prefix.
        // Watched failing against a stubbed-out prefix, which is the only way
        // that was visible.
        //
        // What actually needs asserting is that the length is CARRIED, not
        // inferred from where the input happens to end. Encode a record, then
        // hand the decoder the same bytes with one extra byte appended: with a
        // prefix the title stays put and the extra byte is trailing garbage;
        // without one the title would silently absorb it.
        let g = Genesis {
            title: "ab".to_string(),
            ..a_record()
        };
        let mut extended = g.canonical_bytes();
        extended.push(b'c');

        assert_eq!(
            Genesis::decode(&extended),
            Err(GenesisError::TrailingBytes),
            "the title must not absorb bytes beyond its declared length"
        );

        // And the length prefix is really present in the encoding: the four
        // bytes before the title spell its length.
        let len_at = TITLE_LEN_AT;
        assert_eq!(
            u32::from_be_bytes(g.canonical_bytes()[len_at..len_at + 4].try_into().unwrap()),
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
            assert_eq!(Genesis::decode(&g.canonical_bytes()).unwrap(), g);
        }
    }

    #[test]
    fn truncation_at_any_point_is_refused() {
        // Every prefix of a valid record, not just a couple of hand-picked
        // lengths: a decoder that reads one field without a bounds check fails
        // only at the boundary that field happens to straddle.
        let bytes = a_record().canonical_bytes();
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
        let mut bytes = a_record().canonical_bytes();
        bytes.push(0);
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::TrailingBytes));
    }

    #[test]
    fn a_lying_length_prefix_is_refused() {
        let mut bytes = a_record().canonical_bytes();
        let len_at = TITLE_LEN_AT;
        // Claim the title is far longer than the input holds.
        bytes[len_at..len_at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::LengthMismatch));
    }

    #[test]
    fn an_unknown_policy_is_refused_rather_than_defaulted() {
        // The security-relevant one. Defaulting an unrecognised policy to Open
        // is how a token-gated Stoa becomes world-postable on an old client.
        let mut bytes = a_record().canonical_bytes();
        bytes[POLICY_AT] = 99;
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::UnknownPolicy(99)));
    }

    #[test]
    fn an_unknown_version_is_refused_and_says_so() {
        // Distinguishable from `Truncated` on purpose: this one means "a newer
        // client wrote this", which is a different thing to tell a user than
        // "this data is corrupt".
        let mut bytes = a_record().canonical_bytes();
        bytes[0] = 99;
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::UnknownVersion(99)));
    }

    #[test]
    fn an_invalid_title_encoding_is_refused() {
        let g = a_record();
        let mut bytes = g.canonical_bytes();
        // Replace the title's first byte with a lone continuation byte.
        let title_at = TITLE_LEN_AT + 4;
        bytes[title_at] = 0x80;
        assert_eq!(Genesis::decode(&bytes), Err(GenesisError::InvalidTitle));
    }

    #[test]
    fn a_record_verifies_against_its_own_address() {
        let g = a_record();
        assert!(g.matches(&g.address()));
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
        assert!(!impostor.matches(&real.address()));
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
        assert_ne!(one.address(), two.address());
    }

    #[test]
    fn the_policy_is_in_the_encoding_at_a_fixed_offset() {
        // Policy has one variant today, so this cannot be tested by building
        // two records and comparing.
        //
        // An earlier version mutated a byte of an already-produced encoding and
        // asserted the hash moved. That tests SHA-256, not this encoding: it
        // passes even with the policy deleted from `canonical_bytes()`
        // entirely, because some other field then occupies that offset. Caught
        // in review by exactly that mutation.
        //
        // Pinning the layout is what actually fails when the field is dropped.
        let g = a_record();
        let bytes = g.canonical_bytes();
        assert_eq!(
            bytes[POLICY_AT],
            Policy::OPEN,
            "the policy byte must be encoded at its documented offset"
        );
        // And the encoding is exactly as long as the layout says, so a field
        // cannot be dropped while another silently slides into its place.
        assert_eq!(bytes.len(), TITLE_LEN_AT + 4 + g.title.len());
    }

    #[test]
    fn the_version_is_the_first_byte_of_the_encoding() {
        // Same defect, same fix as the policy test above.
        let bytes = a_record().canonical_bytes();
        assert_eq!(
            bytes[0],
            VERSION_1,
            "the version must be the first byte of the encoding"
        );
    }

}
