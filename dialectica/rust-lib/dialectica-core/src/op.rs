//! The op: the signed envelope that is the unit everything else is built from.
//!
//! # Why this file exists
//!
//! PLAN.md §3.3: "An *op* is a signed operation, and it is the unit everything
//! else is built from. [...] Nothing else crosses the wire, and the forum's
//! whole state is a function of the ops a peer has seen."
//!
//! [`identity::verify_authored_op`] was built to check one, and
//! [`identity::sign_op_bytes`] to make one, but both take opaque `&[u8]`:
//! nothing defined what those bytes *were*. This supplies the missing half —
//! the field set, and one canonical encoding of it.
//!
//! # The op kind is inside the signed bytes, and that is a correctness fix
//!
//! `identity.rs` says it plainly, and this module is the promised repair:
//!
//! > **What it does NOT buy, because there is one prefix for all ops:**
//! > separation between op *kinds*. [...] that separation has to come from the
//! > canonical bytes being unambiguously typed, which is the serialiser's job
//! > and the serialiser does not exist yet.
//!
//! There is one `OP_SIGNING_PREFIX` for every op, so a signature commits to
//! "some dialectica op" and not to which one. Put the kind first in the
//! encoding and the property becomes structural: a `Post` and a `Hide` differ
//! in their first byte, so no signature over one is a valid signature over the
//! other. This is why [`Op::canonical_bytes`] leads with the discriminant
//! rather than treating field order as cosmetic.
//!
//! # What an op does NOT carry, and why each omission is deliberate
//!
//! **No Lamport timestamp, and no message id.** §4.4 has SDS assigning both;
//! §5.7 orders revisions by "the highest Lamport timestamp [...] ties broken by
//! ascending message id — the same rule SDS already applies [...] so nothing
//! new is invented". A self-asserted Lamport value would be forgeable by the
//! author it is meant to order, which defeats the purpose. Both are transport
//! metadata, recorded alongside the op rather than inside it.
//!
//! **This is currently owed by the transport rather than supplied by it.**
//! `contracts/delivery_module.lidl` exposes
//! `channelMessageReceived(channelId, senderId, payload, timestamp)` — no
//! Lamport clock and no SDS message id — so §5.7's ordering rule has no input
//! at the contract we actually have.
//!
//! The gap turns out to sit a layer BELOW that contract, and it is not
//! dialectica's to close: the Reliable Channel API's `MessageReceivedEvent`,
//! which `delivery_module` consumes, carries exactly one field — the reassembled
//! payload — so `channelMessageReceived` cannot forward what it was never given.
//! `channelMessageReceived`'s `timestamp` is the receiving peer's own
//! `CLOCK_REALTIME` read and orders nothing. [`crate::arrival`] holds what a
//! peer records instead, and the `op-ordering` change's `design.md` carries the
//! citations and what each upstream layer would have to add.
//!
//! **No `senderId`.** §4.1: "`senderId` is not an author identity, and the plan
//! should not treat it as one." It binds at channel creation as a transport
//! self-filter. The author identity in an op is the key and the address.
//!
//! **No `channelId`.** §4.5: "never let channel identity leak into payloads or
//! storage keys", so that one-channel-per-thread later becomes a routing change
//! rather than a migration. The Stoa *address* is carried instead — it is a
//! pure function of the addressed object, which is exactly what §4.5 asks for.
//!
//! **No wall-clock timestamp.** Nothing in the plan calls for one, an author
//! chooses it freely, and a forum that ordered by it would be ordering by a
//! field its adversary sets.
//!
//! **No sequence number of any kind.** §4.3's rule for the channel id — "not a
//! session counter, not a local sequence number, not anything that varies with
//! one peer's history" — applies with equal force to a value inside a signed op
//! that every peer must agree about.
//!
//! **No posting policy, in the one kind that might have carried one.**
//! [`OpKind::StoaMetadata`] supersedes a Stoa's *display* metadata and not its
//! policy; the reasoning is on that variant, and at length in the
//! `stoa-metadata-op` change's `design.md`.

use crate::cursor::{Cursor, OutOfBounds};
use crate::identity::{
    sign_op_bytes, verify_authored_op, Address, KeyError, PublicKey, SecretKey, Signature,
};
use sha2::{Digest, Sha256};

/// Domain separation for an op id.
///
/// Distinct from every address prefix in `identity.rs`, so no byte string is
/// ever both a valid op id and a valid author or Stoa address. Padded to a
/// fixed 32 bytes for the same reason those are: a variable-length prefix
/// concatenated with variable-length data is how two different inputs come to
/// hash the same.
const OP_ID_PREFIX: &[u8; 32] = b"/dialectica/1/Id/Op\0\0\0\0\0\0\0\0\0\0\0\0\0";

/// The encoding generation.
///
/// Not the same discriminant as the genesis record's, and not shared with it:
/// the two formats version independently, because a change to one has no reason
/// to invalidate the other.
const VERSION_1: u8 = 1;

/// The maximum length of any single variable-length field, in bytes.
///
/// Derived from §4.4's **150 KiB** SDS message cap, "a network-wide gossipsub
/// validation limit, not unilaterally raisable" — so no single field larger
/// than this could have arrived inside one message.
///
/// # What this cap does, and what it deliberately does not
///
/// Its job is bounding **allocation before data**: a 4-byte length prefix can
/// claim 4 GiB, and a decoder that reserved that much on a hostile peer's
/// promise would be a remote memory-exhaustion lever. `Cursor::take` would
/// refuse the read afterwards — but only after the allocation. Checking the
/// claim first is the whole point, which is why `take_length_within_cap` compares
/// before it reads.
///
/// **It does NOT bound the total size of a decoded op, and must not be read as
/// doing so.** The bound is per field and does not compose: a metadata op with
/// both fields at the cap decodes at 307,274 bytes, twice the SDS cap, and the
/// `Post` attachment path reaches 768,076 bytes because `take_string_list`
/// bounds the element count and each element separately. Both were measured,
/// not estimated.
///
/// That is not a hole to close here, for two reasons. Amplification is roughly
/// 1:1 — reaching 307 KB of decoded op costs the sender 307 KB — so it is not
/// a lever in the sense the per-field cap closes. And **the total-size check
/// belongs at the transport boundary, where the SDS frame is known.** This
/// module is handed a byte slice and cannot see the frame it arrived in, so a
/// combined bound here would be a guess at a number the caller already has.
/// Put it where the frame is; do not add it here.
///
/// The value is pinned by `the_field_cap_is_pinned_to_a_known_answer` — a cap
/// that silently drifted upward would still refuse an absurd prefix and still
/// pass every test that only probes absurd values.
const MAX_FIELD_LEN: usize = 150 * 1024;

/// A 32-byte op id: the hash of an op's canonical bytes.
///
/// **Content-derived rather than transport-assigned**, and the distinction
/// carries weight. §3.1 describes ops as "idempotent by `opId`", and §4.7's
/// snapshot seam needs to stitch "snapshot at Lamport T" to "ops since T"
/// "without gaps or double-application" — both are dedup questions, and a peer
/// replaying its own store or applying a snapshot has no transport envelope to
/// consult. An id computed from the op itself is available in every one of
/// those settings.
///
/// This is deliberately **not** the SDS message id. §5.7's tiebreak — "ties
/// broken by ascending message id" — refers to that one, which arrives with the
/// message and is the transport's to assign. Two identifiers, two jobs; naming
/// this one `OpId` and leaving the other to the store is what keeps them from
/// being confused.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OpId([u8; 32]);

impl OpId {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex, for display and for a moderation op naming its target.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse the display form back.
    ///
    /// Strict about length and hex validity for the same reason
    /// [`Address::from_hex`] is: this parses content that may have arrived from
    /// a peer, and a lenient parser that accepted a truncated id would let a
    /// moderation op name something other than what it appears to name.
    pub fn from_hex(s: &str) -> Result<Self, OpIdError> {
        let bytes = hex::decode(s).map_err(|_| OpIdError::NotHex)?;
        let bytes: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| OpIdError::WrongLength(bytes.len()))?;
        Ok(OpId(bytes))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpIdError {
    NotHex,
    WrongLength(usize),
}

impl std::fmt::Display for OpIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpIdError::NotHex => write!(f, "op id is not valid hex"),
            OpIdError::WrongLength(n) => {
                write!(f, "an op id must be 32 bytes (64 hex chars), got {n} bytes")
            }
        }
    }
}

/// What a moderator is doing to a target.
///
/// **One op kind with an action field, rather than two kinds**, because §6.2
/// says where this is going: "a moderation certificate is an op carrying N
/// independent signatures over the same `(target, action, epoch)` tuple". A
/// threshold certificate signs one tuple, so `action` has to be a field in it
/// rather than a choice of envelope.
///
/// **`Unhide` exists because §5.7's ordering rule is otherwise vacuous.** §13
/// asks the question directly — "Is a hide reversible? §5.7 orders 'moderation
/// ops on the same target' by last-write-wins, which only means something if a
/// hide can be undone. §6's op sketch shows only `hide`. Name the inverse op or
/// say hides are terminal." Naming it is the cheaper answer: last-write-wins
/// over a set of one is not an ordering, and a moderation system with no
/// correction path makes every mistake permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationAction {
    /// Conforming peers stop rendering the target. §6.1 is careful about the
    /// ceiling: this changes what peers *render*, and cannot unpublish
    /// anything.
    Hide,
    /// Withdraw a previous `Hide`. The inverse §13 asked to be named.
    Unhide,
}

impl ModerationAction {
    // Explicit discriminants: these bytes are on the wire and inside a
    // signature, so they are part of the format and must not follow
    // declaration order.
    const HIDE: u8 = 0;
    const UNHIDE: u8 = 1;

    fn to_byte(self) -> u8 {
        match self {
            ModerationAction::Hide => Self::HIDE,
            ModerationAction::Unhide => Self::UNHIDE,
        }
    }

    fn from_byte(b: u8) -> Result<Self, OpError> {
        match b {
            Self::HIDE => Ok(ModerationAction::Hide),
            Self::UNHIDE => Ok(ModerationAction::Unhide),
            other => Err(OpError::UnknownModerationAction(other)),
        }
    }
}

/// Which way a vote points.
///
/// Both directions are carried even though Appendix A measured that OpChan
/// "collected [downvotes], attached to posts, and then filtered [them] out of
/// every scorer" — the signal there is upvote-only. Recording the direction
/// costs a byte and keeps the decision about what to *count* in the scorer,
/// where §7.2 can change it, rather than in the wire format, where changing it
/// is a version bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteDirection {
    Up,
    Down,
}

impl VoteDirection {
    const UP: u8 = 0;
    const DOWN: u8 = 1;

    fn to_byte(self) -> u8 {
        match self {
            VoteDirection::Up => Self::UP,
            VoteDirection::Down => Self::DOWN,
        }
    }

    fn from_byte(b: u8) -> Result<Self, OpError> {
        match b {
            Self::UP => Ok(VoteDirection::Up),
            Self::DOWN => Ok(VoteDirection::Down),
            other => Err(OpError::UnknownVoteDirection(other)),
        }
    }
}

/// What an op does. §3.3's list, minus the two that are not ops.
///
/// §3.3 names five: "Creating a Stoa, posting, publishing a revision of your
/// own post (§5.7), hiding something as a moderator (§6), voting". Creating a
/// Stoa is not here — a Stoa is a genesis record its creator publishes
/// (`stoa.rs`), and hashing that record is what creates it; there is nothing
/// for a *signed op* to add. The other four are here.
///
/// **There is no `Reply` kind.** §4.1 puts `threadId` and `parentPostId` "in
/// the **payload**", so a reply is a `Post` whose parent is set. A separate
/// kind would make "is this a reply?" two questions instead of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    /// A new post, or a reply when `parent` is set.
    Post {
        /// The thread this belongs to.
        ///
        /// **Carried from day one even though one channel serves a whole Stoa
        /// today.** §4.5 defers per-thread channels but says exactly why the
        /// field lands now: "Ops carry `threadId` from day one anyway (the
        /// topic cannot carry it), so the split becomes a routing change rather
        /// than a migration."
        ///
        /// A thread is named by the id of the op that started it. A top-level
        /// post is its own thread's root, which it cannot know at signing time
        /// — its id is the hash of the bytes being signed — so a thread-opening
        /// post carries `None` and the store fills the thread in as its own id
        /// on ingest.
        thread: Option<OpId>,
        /// The post being replied to, if any. `None` is a top-level post.
        parent: Option<OpId>,
        /// The post's text.
        body: String,
        /// Logos Storage CIDs for any attachments (§4.6).
        ///
        /// **Structurally optional, and an unresolvable one never invalidates
        /// the post.** §4.6: "the CID is a field in a signed op, and
        /// unresolvable is a *fetch* outcome, not a validation failure. Render
        /// the attachment as missing; never let it invalidate the post or the
        /// thread." So this module validates that the strings decode, and says
        /// nothing about whether they resolve.
        attachments: Vec<String>,
    },
    /// A new version of one of the author's own posts (§5.7).
    ///
    /// Not an edit in place: "A post is never edited in place. An edit is a new
    /// version of that post, published and signed by the same author." Whether
    /// the signer matches the target's author is a *validation* question the
    /// store answers on read, since it needs the target op to answer it.
    Revise {
        /// The post this supersedes.
        target: OpId,
        body: String,
        attachments: Vec<String>,
    },
    /// A moderator's judgement about a target (§6).
    ///
    /// Valid "only when signed by a current moderator" (§6), which this module
    /// cannot check: it needs the Stoa's moderator set as of this op's Lamport
    /// time. Encoding and signature are settled here; authority is settled on
    /// read.
    Moderate {
        /// The op being acted on. §6.1: a moderation op "names an `opId`".
        ///
        /// §6.1 also describes an author-scoped variant that "differs from
        /// `hide` only in what it names" — deferred with the mutable moderator
        /// set, so the target is an op id and nothing else today.
        target: OpId,
        action: ModerationAction,
    },
    /// A vote on a post.
    ///
    /// **Collected in v1, read by nothing.** §13 asked whether votes are an op
    /// at all: "v1 either collects vote ops nothing reads or has no votes.
    /// Collecting them early is cheap and makes the history available when
    /// scoring lands; deciding by accident is not." §7.2 rule 2 ships no score,
    /// so nothing reads these yet — but the kind is in the format, so scoring
    /// arrives without a wire-format version bump.
    Vote {
        target: OpId,
        direction: VoteDirection,
    },
    /// What the Stoa is called *today* (§5.7).
    ///
    /// The genesis record's title is a **founding** value: it is inside the
    /// address preimage, so changing it mints a different Stoa. This op carries
    /// the current one. §5.7 states the relationship — "genesis values are what
    /// the *address commits to* and can never change; the metadata op carries
    /// what the Stoa is called *today*. A reader prefers the latest valid op and
    /// falls back to the genesis values."
    ///
    /// The Stoa this applies to is [`Op::stoa`], like every other kind; the
    /// signer is [`Op::author`]. Neither is repeated here.
    ///
    /// **No `policy`, and that is the answer to §5.7's open question** rather
    /// than an omission. Three reasons, at length in the `stoa-metadata-op`
    /// change's `design.md`; the one that decides it: §5.7's own reader rule
    /// falls back to the genesis value when no op has been seen, and a peer that
    /// missed a *tightening* would fall back to the **looser** founding policy.
    /// That is the widening `stoa.rs` refuses on decode — "treating an
    /// unrecognised policy as open is how a token-gated Stoa silently becomes
    /// world-postable" — arriving instead by resolution. Refusing to default a
    /// policy and then handing one back by fallback closes the front door only.
    ///
    /// Leaving it out costs no version to add later, and that was checked rather
    /// than assumed. §13 records that `policy` landed in the *genesis* record
    /// early because "adding the field later would have changed the address of
    /// every Stoa already created" — that pressure does not transfer, because an
    /// op's id is the hash of that one op. A future policy-changing act takes
    /// the next free kind discriminant and an older client meets it as
    /// [`OpError::UnknownKind`], changing no existing op's id and no Stoa's
    /// address.
    StoaMetadata {
        /// The displayed title, superseding the genesis title for display.
        title: String,
        /// A field the genesis record deliberately does not have at all.
        ///
        /// A description is not identity, so it has no business in an address
        /// preimage where every re-wording would mint a new Stoa. Carrying it
        /// here is part of what makes this a metadata op rather than a
        /// title-override op: the genesis record is the minimum needed to
        /// *identify* a Stoa, this is what a reader needs to *render* one.
        description: String,
    },
}

impl OpKind {
    // On the wire and inside every signature. Explicit, and never reordered:
    // inserting a value into the used range would re-mean every op already
    // signed. A new kind takes the next free value and nothing else moves.
    const POST: u8 = 0;
    const REVISE: u8 = 1;
    const MODERATE: u8 = 2;
    const VOTE: u8 = 3;
    const STOA_METADATA: u8 = 4;

    fn to_byte(&self) -> u8 {
        match self {
            OpKind::Post { .. } => Self::POST,
            OpKind::Revise { .. } => Self::REVISE,
            OpKind::Moderate { .. } => Self::MODERATE,
            OpKind::Vote { .. } => Self::VOTE,
            OpKind::StoaMetadata { .. } => Self::STOA_METADATA,
        }
    }
}

/// A signed operation: the whole of what crosses the wire.
///
/// The author's **public key** travels in the op, not merely their address.
/// §3.3 puts verification on read with no directory to resolve an address
/// against, so a peer holding only an address could not check the signature.
/// The address is recoverable from the key ([`PublicKey::address`]), which is
/// what [`identity::verify_authored_op`] re-derives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    /// Which Stoa this belongs to.
    ///
    /// The Stoa *address*, never the channel id (§4.5). Inside the signed bytes
    /// so that an op lifted from one Stoa's channel and replayed on another's
    /// fails verification rather than arriving as a valid post in a Stoa its
    /// author never addressed.
    pub stoa: Address,
    /// Who wrote it. A per-Stoa key (§5.2), derived by
    /// [`identity::derive_stoa_key`].
    pub author: PublicKey,
    pub kind: OpKind,
}

/// Why a byte string is not an op.
///
/// Each variant names a different mistake, matching `GenesisError`'s reasoning:
/// a decoder that only says "invalid" sends the reader looking in the wrong
/// place.
#[derive(Debug, PartialEq, Eq)]
pub enum OpError {
    /// A version this build does not know. Means "newer client", not "corrupt".
    UnknownVersion(u8),
    /// An op kind discriminant this build does not know.
    UnknownKind(u8),
    /// A moderation action this build does not know. Never defaulted — see
    /// [`Op::decode`].
    UnknownModerationAction(u8),
    /// A vote direction this build does not know.
    UnknownVoteDirection(u8),
    /// The optional-field tag was neither absent (0) nor present (1).
    InvalidOptionTag(u8),
    /// Input ended before a field did.
    Truncated,
    /// A complete op followed by bytes that are not part of it.
    TrailingBytes,
    /// A length prefix claiming more bytes than the input holds.
    LengthMismatch,
    /// A length prefix over [`MAX_FIELD_LEN`], which no legitimate op can carry
    /// because SDS would not have delivered it (§4.4).
    FieldTooLong(usize),
    /// A text field is not valid UTF-8.
    InvalidText,
    /// The author key is not a valid public key.
    InvalidAuthor(KeyError),
}

impl From<OutOfBounds> for OpError {
    fn from(e: OutOfBounds) -> Self {
        match e {
            OutOfBounds::Truncated => OpError::Truncated,
            OutOfBounds::Trailing => OpError::TrailingBytes,
        }
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpError::UnknownVersion(v) => write!(f, "unknown op encoding version {v}"),
            OpError::UnknownKind(k) => write!(f, "unknown op kind {k}"),
            OpError::UnknownModerationAction(a) => write!(f, "unknown moderation action {a}"),
            OpError::UnknownVoteDirection(d) => write!(f, "unknown vote direction {d}"),
            OpError::InvalidOptionTag(t) => {
                write!(f, "an optional field's tag must be 0 or 1, got {t}")
            }
            OpError::Truncated => write!(f, "the op ended mid-field"),
            OpError::TrailingBytes => write!(f, "bytes follow the end of the op"),
            OpError::LengthMismatch => write!(f, "a length prefix exceeds the input"),
            OpError::FieldTooLong(n) => {
                write!(f, "a field claims {n} bytes, over the {MAX_FIELD_LEN} cap")
            }
            OpError::InvalidText => write!(f, "a text field is not valid UTF-8"),
            OpError::InvalidAuthor(e) => write!(f, "the author key is invalid: {e}"),
        }
    }
}

/// An op together with the signature over its canonical bytes.
///
/// Kept as a separate type rather than a `signature` field on [`Op`] because
/// the signature is made over the op's bytes: an `Op` that contained its own
/// signature would have to define whether the signature covers itself. This
/// shape makes the preimage exactly "the op", with no carve-out to remember.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedOp {
    pub op: Op,
    pub signature: Signature,
}

impl Op {
    /// The canonical encoding: exactly one valid byte string per op.
    ///
    /// Layout:
    ///
    /// ```text
    /// version    1 byte
    /// kind       1 byte      <- first, so a signature commits to the kind
    /// stoa      32 bytes
    /// author    32 bytes
    /// <kind-specific fields>
    /// ```
    ///
    /// A variable-length field is a 4-byte big-endian length then its bytes; an
    /// optional field is a 1-byte tag (0 absent, 1 present) then, if present,
    /// the value. Both rules exist for the same reason `stoa.rs` gives:
    /// concatenating variable-length fields lets distinct records collide,
    /// because `("ab","c")` and `("a","bc")` produce identical bytes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(VERSION_1);
        // The kind leads. This is the whole of the op-kind separation
        // `identity.rs` said the serialiser owed it: two ops of different kinds
        // cannot share a preimage, so no signature over one is a signature over
        // the other.
        out.push(self.kind.to_byte());
        out.extend_from_slice(self.stoa.as_bytes());
        out.extend_from_slice(&self.author.to_bytes());

        match &self.kind {
            OpKind::Post {
                thread,
                parent,
                body,
                attachments,
            } => {
                put_option_id(&mut out, thread);
                put_option_id(&mut out, parent);
                put_bytes(&mut out, body.as_bytes());
                put_string_list(&mut out, attachments);
            }
            OpKind::Revise {
                target,
                body,
                attachments,
            } => {
                out.extend_from_slice(target.as_bytes());
                put_bytes(&mut out, body.as_bytes());
                put_string_list(&mut out, attachments);
            }
            OpKind::Moderate { target, action } => {
                out.extend_from_slice(target.as_bytes());
                out.push(action.to_byte());
            }
            OpKind::Vote { target, direction } => {
                out.extend_from_slice(target.as_bytes());
                out.push(direction.to_byte());
            }
            OpKind::StoaMetadata { title, description } => {
                // Two adjacent variable-length fields, so both are prefixed
                // through the same helper the other kinds use. Without prefixes
                // title "ab" + description "c" and title "a" + description "bc"
                // encode identically — two different acts with one op id.
                put_bytes(&mut out, title.as_bytes());
                put_bytes(&mut out, description.as_bytes());
            }
        }
        out
    }

    /// Decode a canonical encoding, refusing anything else.
    ///
    /// Strict because this is the ingest path: every byte here arrived from a
    /// peer and is attacker-controlled. CLAUDE.md's posture — "Never trust an
    /// inbound message [...] Validate at the boundary, before it reaches any
    /// state machine" — is this function's whole job.
    ///
    /// **An unknown discriminant is refused, never defaulted.** The genesis
    /// record's reasoning transfers exactly: defaulting an unrecognised
    /// moderation action to `Hide` would let a newer client's `Unsomething`
    /// read as a hide on an older one, which is a moderation action nobody
    /// took.
    pub fn decode(bytes: &[u8]) -> Result<Self, OpError> {
        let mut cursor = Cursor::new(bytes);

        match cursor.take(1)?[0] {
            VERSION_1 => {}
            other => return Err(OpError::UnknownVersion(other)),
        }

        let kind_byte = cursor.take(1)?[0];
        let stoa = Address::from_bytes(cursor.take_array::<32>()?);
        let author =
            PublicKey::from_bytes(cursor.take(32)?).map_err(OpError::InvalidAuthor)?;

        let kind = match kind_byte {
            OpKind::POST => OpKind::Post {
                thread: take_option_id(&mut cursor)?,
                parent: take_option_id(&mut cursor)?,
                body: take_string(&mut cursor)?,
                attachments: take_string_list(&mut cursor)?,
            },
            OpKind::REVISE => OpKind::Revise {
                target: OpId(cursor.take_array::<32>()?),
                body: take_string(&mut cursor)?,
                attachments: take_string_list(&mut cursor)?,
            },
            OpKind::MODERATE => OpKind::Moderate {
                target: OpId(cursor.take_array::<32>()?),
                action: ModerationAction::from_byte(cursor.take(1)?[0])?,
            },
            OpKind::VOTE => OpKind::Vote {
                target: OpId(cursor.take_array::<32>()?),
                direction: VoteDirection::from_byte(cursor.take(1)?[0])?,
            },
            OpKind::STOA_METADATA => OpKind::StoaMetadata {
                // Through `take_string`, so the length cap and the bounds check
                // come from the shared path rather than a second copy of them.
                title: take_string(&mut cursor)?,
                description: take_string(&mut cursor)?,
            },
            other => return Err(OpError::UnknownKind(other)),
        };

        cursor.finish()?;

        Ok(Op {
            stoa,
            author,
            kind,
        })
    }

    /// This op's id: the hash of its canonical bytes, domain-separated.
    ///
    /// Prefixed like every other hash in this crate, so an op id can never
    /// collide with an author address or a Stoa address — which matters because
    /// all three are 32 bytes and a moderation op names one of them.
    pub fn id(&self) -> OpId {
        let mut hasher = Sha256::new();
        hasher.update(OP_ID_PREFIX);
        hasher.update(self.canonical_bytes());
        OpId(hasher.finalize().into())
    }

    /// Sign this op, producing the envelope that crosses the wire.
    ///
    /// The key must be the author's per-Stoa key ([`identity::derive_stoa_key`])
    /// — signing with any other key produces an op that
    /// [`SignedOp::verify`] rejects, since the address it re-derives will not
    /// match. Taking the key rather than reading one from ambient state is
    /// CLAUDE.md's "pass what it needs".
    pub fn sign(self, key: &SecretKey) -> SignedOp {
        let signature = sign_op_bytes(key, &self.canonical_bytes());
        SignedOp { op: self, signature }
    }
}

impl SignedOp {
    /// Whether this op is authentically from the author it claims.
    ///
    /// Delegates to [`identity::verify_authored_op`], which is the function
    /// that binds the key to the claimed address — the check that a caller
    /// doing the steps by hand forgets. Nothing here re-implements it.
    ///
    /// **This answers authenticity only.** It does not ask whether the signer
    /// is a moderator (§6), whether a revision's author owns the post it
    /// supersedes (§5.7), or whether the Stoa's policy admits the poster
    /// (§7.1). Each of those needs state this type does not have, and each is
    /// the store's to answer on read (§3.3). Reading a `true` here as "this op
    /// is valid" is precisely the conflation §6.2 measured in the nearest kin
    /// project, which "checks moderator authority only on the send path and
    /// never on the read path".
    pub fn verify(&self) -> bool {
        verify_authored_op(
            &self.op.author.address(),
            &self.op.author.to_bytes(),
            &self.op.canonical_bytes(),
            &self.signature.to_bytes(),
        )
    }

    /// The wire form: the op's canonical bytes, then the 64-byte signature.
    ///
    /// The signature trails rather than leads so that the signed preimage is a
    /// prefix of the message — a decoder never has to skip over the signature
    /// to find the bytes it covers.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.op.canonical_bytes();
        out.extend_from_slice(&self.signature.to_bytes());
        out
    }

    /// Decode the wire form.
    ///
    /// **Does not verify.** Decoding and verifying are separate jobs, and a
    /// caller must be able to decode an op in order to decide what to do with a
    /// bad signature. [`SignedOp::verify`] is the other half, and §3.3 puts it
    /// on read.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, OpError> {
        // The signature is fixed-width and at the end, so split it off first;
        // what remains must be exactly the op, which `Op::decode`'s own
        // trailing-bytes check then enforces.
        let split = bytes.len().checked_sub(64).ok_or(OpError::Truncated)?;
        let (op_bytes, sig_bytes) = bytes.split_at(split);
        let op = Op::decode(op_bytes)?;
        // Any 64 bytes parse as a signature — Ed25519 defers every validity
        // question to verification — so this cannot fail here, and its failure
        // arm exists only because the length could not be wrong.
        let signature = Signature::from_bytes(sig_bytes).map_err(|_| OpError::Truncated)?;
        Ok(SignedOp { op, signature })
    }
}

// ─── Encoding helpers ─────────────────────────────────────────────────────
//
// Separate functions rather than inline blocks so that the length-prefix rule
// is written once. A format where three call sites each spell their own prefix
// is a format where one of them eventually spells it differently.

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    // `as u32` cannot truncate meaningfully: `MAX_FIELD_LEN` is checked on the
    // way back in, and a field long enough to overflow u32 is orders of
    // magnitude past what SDS would carry.
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn put_string_list(out: &mut Vec<u8>, items: &[String]) {
    out.extend_from_slice(&(items.len() as u32).to_be_bytes());
    for item in items {
        put_bytes(out, item.as_bytes());
    }
}

/// An optional id as a presence tag then, if present, 32 bytes.
///
/// A tag rather than a sentinel value: the all-zero id is a legitimate hash
/// output, so reserving it to mean "absent" would make one valid op
/// unrepresentable and, worse, silently reinterpret it.
fn put_option_id(out: &mut Vec<u8>, id: &Option<OpId>) {
    match id {
        None => out.push(0),
        Some(id) => {
            out.push(1);
            out.extend_from_slice(id.as_bytes());
        }
    }
}

// ─── Decoding helpers ─────────────────────────────────────────────────────

/// A length prefix, refused if it claims more than [`MAX_FIELD_LEN`].
///
/// The cap is applied here rather than after reading, because the whole point
/// is to refuse before allocating on a hostile peer's promise.
///
/// **The returned count means different things to its two callers**, and the
/// cap applies to the number either way rather than to any total. In
/// [`take_string`] it is a count of BYTES; in [`take_string_list`] it is a
/// count of ELEMENTS, each of which is then separately capped as it is read.
/// So a list is bounded at `MAX_FIELD_LEN` elements of `MAX_FIELD_LEN` bytes,
/// not at `MAX_FIELD_LEN` bytes in total — see [`MAX_FIELD_LEN`] on why that
/// is the accepted shape and where a total-size bound belongs instead.
fn take_length_within_cap(cursor: &mut Cursor<'_>) -> Result<usize, OpError> {
    let len = cursor.take_length()?;
    if len > MAX_FIELD_LEN {
        return Err(OpError::FieldTooLong(len));
    }
    Ok(len)
}

fn take_string(cursor: &mut Cursor<'_>) -> Result<String, OpError> {
    let len = take_length_within_cap(cursor)?;
    // A prefix claiming more than the input holds is a LengthMismatch rather
    // than a Truncated: the input is not short, the claim is wrong, and saying
    // so points at the right half of the problem.
    let bytes = cursor.take(len).map_err(|_| OpError::LengthMismatch)?;
    String::from_utf8(bytes.to_vec()).map_err(|_| OpError::InvalidText)
}

fn take_string_list(cursor: &mut Cursor<'_>) -> Result<Vec<String>, OpError> {
    let count = take_length_within_cap(cursor)?;
    // Deliberately NOT `Vec::with_capacity(count)`: the count is a hostile
    // peer's claim, and reserving on it is the memory-exhaustion lever the cap
    // exists to close. The vector grows as elements actually arrive.
    let mut items = Vec::new();
    for _ in 0..count {
        items.push(take_string(cursor)?);
    }
    Ok(items)
}

fn take_option_id(cursor: &mut Cursor<'_>) -> Result<Option<OpId>, OpError> {
    match cursor.take(1)?[0] {
        0 => Ok(None),
        1 => Ok(Some(OpId(cursor.take_array::<32>()?))),
        // Not "anything non-zero is present". A third value means the sender
        // and this decoder disagree about the format, and guessing which they
        // meant is how two peers derive different ids for one op.
        other => Err(OpError::InvalidOptionTag(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::derive_stoa_key;
    use crate::stoa::{Genesis, Policy};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    fn a_stoa() -> Address {
        Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
        .address()
        .expect("a short title is well under MAX_TITLE_BYTES")
    }

    fn a_post() -> Op {
        Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "First".to_string(),
                attachments: vec![],
            },
        }
    }

    fn an_id(seed: u8) -> OpId {
        OpId([seed; 32])
    }

    /// Every op kind, for the properties that must hold across all of them.
    fn one_of_each_kind() -> Vec<Op> {
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        vec![
            a_post(),
            Op {
                stoa,
                author: author.clone(),
                kind: OpKind::Post {
                    thread: Some(an_id(7)),
                    parent: Some(an_id(8)),
                    body: "A reply".to_string(),
                    attachments: vec!["cid-one".to_string(), "cid-two".to_string()],
                },
            },
            Op {
                stoa,
                author: author.clone(),
                kind: OpKind::Revise {
                    target: an_id(9),
                    body: "Edited".to_string(),
                    attachments: vec!["cid".to_string()],
                },
            },
            Op {
                stoa,
                author: author.clone(),
                kind: OpKind::Moderate {
                    target: an_id(10),
                    action: ModerationAction::Hide,
                },
            },
            Op {
                stoa,
                author: author.clone(),
                kind: OpKind::Moderate {
                    target: an_id(10),
                    action: ModerationAction::Unhide,
                },
            },
            Op {
                stoa,
                author: author.clone(),
                kind: OpKind::Vote {
                    target: an_id(11),
                    direction: VoteDirection::Up,
                },
            },
            Op {
                stoa,
                author,
                kind: OpKind::StoaMetadata {
                    title: "The Agora, renamed".to_string(),
                    description: "A marketplace of arguments".to_string(),
                },
            },
        ]
    }

    // ─── Layout offsets ───────────────────────────────────────────────────
    //
    // Named once, so a test that pokes at a byte says WHICH byte rather than
    // re-deriving `1 + 1 + 32 + 32` and leaving the reader to check the
    // arithmetic. These mirror `canonical_bytes`'s documented layout:
    //
    //     version 1 | kind 1 | stoa 32 | author 32 | <kind-specific>

    /// Offset of the kind byte. Version is first, kind second.
    const KIND_AT: usize = 1;
    /// Offset of the 32-byte Stoa address.
    const STOA_AT: usize = KIND_AT + 1;
    /// Offset of the 32-byte author key.
    const AUTHOR_AT: usize = STOA_AT + 32;
    /// Offset of the first kind-specific byte — where every kind's own fields
    /// begin, and where the common header ends.
    const KIND_FIELDS_AT: usize = AUTHOR_AT + 32;

    /// Width of a length prefix. Every variable-length field carries one.
    const LEN_PREFIX: usize = 4;
    /// Width of an optional field's presence tag.
    const OPTION_TAG: usize = 1;

    /// Where a `Post`'s body length prefix sits: after the two option tags.
    const POST_BODY_LEN_AT: usize = KIND_FIELDS_AT + OPTION_TAG + OPTION_TAG;
    /// Where a `Post`'s body text begins.
    const POST_BODY_AT: usize = POST_BODY_LEN_AT + LEN_PREFIX;

    /// Byte offsets within a `StoaMetadata` op, derived from its title length.
    ///
    /// Returned as named fields rather than computed at each call site,
    /// because the description's position DEPENDS on the title's length — the
    /// one offset here that is not a constant. Spelling it inline forced each
    /// test to hardcode its own fixture's title length as a magic number and
    /// then explain the coupling in a comment; this makes the dependency an
    /// argument instead.
    struct MetadataOffsets {
        title_len_at: usize,
        title_at: usize,
        description_len_at: usize,
        description_at: usize,
    }

    fn metadata_offsets(title_len: usize) -> MetadataOffsets {
        let title_len_at = KIND_FIELDS_AT;
        let title_at = title_len_at + LEN_PREFIX;
        let description_len_at = title_at + title_len;
        MetadataOffsets {
            title_len_at,
            title_at,
            description_len_at,
            description_at: description_len_at + LEN_PREFIX,
        }
    }

    // ─── Encoding ─────────────────────────────────────────────────────────

    #[test]
    fn encodes_identically_every_time() {
        for op in one_of_each_kind() {
            assert_eq!(op.canonical_bytes(), op.canonical_bytes());
        }
    }

    #[test]
    fn decode_of_encode_is_the_identity() {
        for op in one_of_each_kind() {
            assert_eq!(Op::decode(&op.canonical_bytes()).unwrap(), op);
        }
    }

    #[test]
    fn the_version_is_the_first_byte_and_the_kind_is_the_second() {
        // Pinned as a layout, not inferred from a hash moving: mutating a byte
        // and asserting the id changes would test SHA-256, and would still
        // pass with a field deleted, because another slides into its place.
        for op in one_of_each_kind() {
            let bytes = op.canonical_bytes();
            assert_eq!(bytes[0], VERSION_1, "version must lead");
            assert_eq!(
                bytes[KIND_AT],
                op.kind.to_byte(),
                "the kind must be the second byte"
            );
        }
    }

    #[test]
    fn the_kind_byte_is_inside_the_signed_preimage() {
        // THE property this module owes `identity.rs`, which records that one
        // OP_SIGNING_PREFIX serves every op so "a signature is not
        // intrinsically bound to which sort of op it authorises — that
        // separation has to come from the canonical bytes being unambiguously
        // typed".
        //
        // Two ops identical in every other respect, differing only in kind,
        // must not share a preimage. Delete the `out.push(self.kind.to_byte())`
        // line and this fails.
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        let target = an_id(3);

        let moderate = Op {
            stoa,
            author: author.clone(),
            kind: OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
        };
        let vote = Op {
            stoa,
            author,
            kind: OpKind::Vote {
                target,
                direction: VoteDirection::Up,
            },
        };

        // Same stoa, same author, same 32-byte target, same trailing
        // discriminant byte (HIDE and UP are both 0) — so without the kind
        // byte these two encode identically.
        assert_ne!(
            moderate.canonical_bytes(),
            vote.canonical_bytes(),
            "a moderation and a vote must not share a signing preimage"
        );
    }

    #[test]
    fn a_signature_over_one_kind_does_not_verify_as_another() {
        // The consequence of the property above, at the level that matters: an
        // attacker replaying a vote signature as a moderation must fail.
        let key = a_key(2);
        let stoa = a_stoa();
        let target = an_id(3);

        let vote = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Vote {
                target,
                direction: VoteDirection::Up,
            },
        };
        let signed_vote = vote.sign(&key);

        // Take the vote's signature and attach it to a moderation op.
        let forged = SignedOp {
            op: Op {
                stoa,
                author: key.public_key(),
                kind: OpKind::Moderate {
                    target,
                    action: ModerationAction::Hide,
                },
            },
            signature: signed_vote.signature.clone(),
        };
        assert!(
            !forged.verify(),
            "a vote's signature must not authorise a moderation"
        );
        // And the original is genuinely valid, so the test is not passing
        // because both are broken.
        assert!(signed_vote.verify());
    }

    #[test]
    fn every_field_participates_in_the_encoding() {
        // Varying each field in turn catches one left out of the encoding —
        // invisible in a round-trip test, since the value comes back from the
        // struct it never left.
        let base = a_post();
        let others = [
            Op {
                stoa: Genesis {
                    creator: a_key(1).public_key(),
                    policy: Policy::Open,
                    title: "Another".to_string(),
                }
                .address()
                .expect("a short title is well under MAX_TITLE_BYTES"),
                ..base.clone()
            },
            Op {
                author: a_key(3).public_key(),
                ..base.clone()
            },
            Op {
                kind: OpKind::Post {
                    thread: Some(an_id(1)),
                    parent: None,
                    body: "First".to_string(),
                    attachments: vec![],
                },
                ..base.clone()
            },
            Op {
                kind: OpKind::Post {
                    thread: None,
                    parent: Some(an_id(1)),
                    body: "First".to_string(),
                    attachments: vec![],
                },
                ..base.clone()
            },
            Op {
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: "Second".to_string(),
                    attachments: vec![],
                },
                ..base.clone()
            },
            Op {
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: "First".to_string(),
                    attachments: vec!["cid".to_string()],
                },
                ..base.clone()
            },
        ];
        for other in others {
            assert_ne!(
                base.canonical_bytes(),
                other.canonical_bytes(),
                "a field is missing from the encoding"
            );
            assert_ne!(base.id(), other.id(), "a field is missing from the id");
        }
    }

    #[test]
    fn the_moderation_action_participates_in_the_encoding() {
        // Hide and Unhide differ in one byte, and that byte must be carried:
        // an encoding that dropped it would make a withdrawal indistinguishable
        // from the hide it withdraws.
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        let hide = Op {
            stoa,
            author: author.clone(),
            kind: OpKind::Moderate {
                target: an_id(1),
                action: ModerationAction::Hide,
            },
        };
        let unhide = Op {
            stoa,
            author,
            kind: OpKind::Moderate {
                target: an_id(1),
                action: ModerationAction::Unhide,
            },
        };
        assert_ne!(hide.canonical_bytes(), unhide.canonical_bytes());
        assert_ne!(hide.id(), unhide.id());
    }

    #[test]
    fn the_vote_direction_participates_in_the_encoding() {
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        let up = Op {
            stoa,
            author: author.clone(),
            kind: OpKind::Vote {
                target: an_id(1),
                direction: VoteDirection::Up,
            },
        };
        let down = Op {
            stoa,
            author,
            kind: OpKind::Vote {
                target: an_id(1),
                direction: VoteDirection::Down,
            },
        };
        assert_ne!(up.canonical_bytes(), down.canonical_bytes());
    }

    #[test]
    fn a_length_prefix_separates_adjacent_variable_length_fields() {
        // The concatenation trap. A post's body and its attachment list are
        // adjacent and both variable-length, so without prefixes the boundary
        // between them is ambiguous: body "ab" + attachment "c" would encode
        // the same as body "a" + attachment "bc".
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        let one = Op {
            stoa,
            author: author.clone(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "ab".to_string(),
                attachments: vec!["c".to_string()],
            },
        };
        let two = Op {
            stoa,
            author,
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "a".to_string(),
                attachments: vec!["bc".to_string()],
            },
        };
        assert_ne!(
            one.canonical_bytes(),
            two.canonical_bytes(),
            "two distinct posts must not share an encoding"
        );
        assert_ne!(one.id(), two.id());
    }

    #[test]
    fn an_empty_body_and_an_empty_attachment_list_round_trip() {
        // Empty is legitimate and must not be confused with absent — a zero
        // length prefix is a real encoding, not a missing field.
        let op = a_post();
        assert_eq!(Op::decode(&op.canonical_bytes()).unwrap(), op);
    }

    #[test]
    fn a_body_survives_multibyte_utf8() {
        for body in ["", "Ἀγορά — the marketplace", "🏛", "a\0b"] {
            let op = Op {
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: body.to_string(),
                    attachments: vec![],
                },
                ..a_post()
            };
            assert_eq!(Op::decode(&op.canonical_bytes()).unwrap(), op);
        }
    }

    // ─── Op id ────────────────────────────────────────────────────────────

    #[test]
    fn an_op_id_is_not_a_bare_hash_of_the_canonical_bytes() {
        // Domain separation, for the same reason `identity.rs` separates its
        // two address derivations: an op id, an author address and a Stoa
        // address are all 32 bytes, and a moderation op names one of them.
        let op = a_post();
        let bare = {
            let mut h = Sha256::new();
            h.update(op.canonical_bytes());
            let out: [u8; 32] = h.finalize().into();
            out
        };
        assert_ne!(op.id().as_bytes(), &bare);
    }

    #[test]
    fn an_op_id_never_collides_with_a_stoa_address() {
        // Hold everything equal except the prefix: hash the EXACT same
        // preimage both ways. Comparing hashes of unrelated inputs would pass
        // whether or not the prefixes differed, which is the trap here.
        let op = a_post();
        let bytes = op.canonical_bytes();
        assert_ne!(
            op.id().as_bytes(),
            crate::identity::stoa_address(&bytes).as_bytes(),
            "op id and Stoa address derivations must be domain-separated"
        );
    }

    #[test]
    fn an_op_id_survives_a_hex_round_trip() {
        let id = a_post().id();
        assert_eq!(OpId::from_hex(&id.to_hex()).unwrap(), id);
    }

    #[test]
    fn op_id_parsing_rejects_attacker_supplied_junk() {
        // A moderation op names its target by id, and that id may reach us as
        // text. A truncated id that parsed would let a hide name something
        // other than what it appears to name.
        assert_eq!(OpId::from_hex("nothex!!"), Err(OpIdError::NotHex));
        assert_eq!(OpId::from_hex(""), Err(OpIdError::WrongLength(0)));
        assert_eq!(OpId::from_hex("00ff"), Err(OpIdError::WrongLength(2)));
        assert_eq!(
            OpId::from_hex(&"ab".repeat(33)),
            Err(OpIdError::WrongLength(33))
        );
    }

    #[test]
    fn the_op_id_constant_is_pinned_to_a_known_answer() {
        // Consensus-critical, exactly like `identity.rs`'s pinned constants:
        // change the prefix or the layout and every peer computes different
        // ids for the same op, with no error anywhere because each peer stays
        // internally consistent.
        //
        // If this fails, do NOT update the expected value to match. Work out
        // what changed and whether the network can survive it.
        //
        // The value was derived INDEPENDENTLY before being pinned, and that
        // is what makes it worth having. A constant produced by the code it
        // pins agrees with any bug that code happens to have — it only ever
        // detects future change, never present error. This one was
        // reconstructed from the RFC 8032 point arithmetic and the layout as
        // written on `canonical_bytes`, so it also pins the implementation
        // against the documentation. To re-derive it, encode by hand from that
        // doc comment: version 1, kind 0, the Stoa address, the public key for
        // seed [7; 32], two zero option tags, then the length-prefixed body
        // and a zero attachment count — and hash it under OP_ID_PREFIX.
        let op = Op {
            stoa: crate::identity::stoa_address(b"a genesis record"),
            author: a_key(7).public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "pinned".to_string(),
                attachments: vec![],
            },
        };
        assert_eq!(
            op.id().to_hex(),
            "1a27fb18f40107bfb00393e935cdf20fe04ea78e0432de8676c1253cefb31fe8",
            "op id derivation changed"
        );
    }

    // ─── Signing ──────────────────────────────────────────────────────────

    #[test]
    fn a_signed_op_verifies_against_its_own_author() {
        for op in one_of_each_kind() {
            // The author must be the key that signs, or the address check in
            // `verify_authored_op` rejects it — which is the point of that
            // function.
            let key = a_key(2);
            let signed = op.sign(&key);
            assert!(signed.verify(), "a well-formed op must verify");
        }
    }

    #[test]
    fn an_op_signed_by_someone_else_is_rejected() {
        // THE forgery: a valid signature, untampered bytes, and still not from
        // the author it claims. Only re-deriving the address from the key
        // catches it, which is what `verify_authored_op` does.
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            author: victim.public_key(),
            ..a_post()
        };
        let signed = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        assert!(!signed.verify());
    }

    #[test]
    fn a_tampered_op_does_not_verify() {
        // Change the body after signing and the signature must stop matching,
        // or "signed" means nothing.
        let key = a_key(2);
        let signed = a_post().sign(&key);
        let tampered = SignedOp {
            op: Op {
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: "Tampered".to_string(),
                    attachments: vec![],
                },
                ..signed.op.clone()
            },
            signature: signed.signature.clone(),
        };
        assert!(!tampered.verify());
    }

    #[test]
    fn an_op_replayed_into_another_stoa_does_not_verify() {
        // The Stoa is inside the signed bytes, so an op lifted from one Stoa's
        // channel and put on another's fails rather than arriving as a valid
        // post its author never addressed there.
        let key = a_key(2);
        let signed = a_post().sign(&key);
        let elsewhere = SignedOp {
            op: Op {
                stoa: crate::identity::stoa_address(b"a different stoa"),
                ..signed.op.clone()
            },
            signature: signed.signature.clone(),
        };
        assert!(!elsewhere.verify());
    }

    #[test]
    fn a_derived_per_stoa_key_signs_an_op_for_that_stoa() {
        // The realistic path: §5.2's per-Stoa identity, derived from a root,
        // signing into the Stoa it was derived for.
        let stoa = a_stoa();
        let key = derive_stoa_key(&[7u8; 32], &stoa);
        let op = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "Hello".to_string(),
                attachments: vec![],
            },
        };
        assert!(op.sign(&key).verify());
    }

    // ─── The wire form ────────────────────────────────────────────────────

    #[test]
    fn a_signed_op_survives_a_wire_round_trip() {
        let key = a_key(2);
        for op in one_of_each_kind() {
            let signed = op.sign(&key);
            let restored = SignedOp::from_bytes(&signed.to_bytes()).unwrap();
            assert_eq!(restored, signed);
            assert!(restored.verify(), "verification must survive the round trip");
        }
    }

    #[test]
    fn the_signed_preimage_is_a_prefix_of_the_wire_form() {
        // So a decoder never skips over the signature to find what it covers.
        let signed = a_post().sign(&a_key(2));
        let wire = signed.to_bytes();
        assert!(wire.starts_with(&signed.op.canonical_bytes()));
        assert_eq!(wire.len(), signed.op.canonical_bytes().len() + 64);
    }

    #[test]
    fn a_wire_form_shorter_than_a_signature_is_refused() {
        // The `checked_sub` in `from_bytes`. Without it this underflows.
        for n in [0usize, 1, 63] {
            assert_eq!(SignedOp::from_bytes(&vec![0u8; n]), Err(OpError::Truncated));
        }
    }

    #[test]
    fn a_wire_form_with_trailing_bytes_is_refused() {
        // The signature is split off the end first, so a trailing byte shifts
        // what is read as the signature and leaves a byte over in the op —
        // which `Op::decode`'s own check catches.
        let signed = a_post().sign(&a_key(2));
        let mut wire = signed.to_bytes();
        wire.push(0);
        assert_eq!(SignedOp::from_bytes(&wire), Err(OpError::TrailingBytes));
    }

    // ─── Hostile input ────────────────────────────────────────────────────

    #[test]
    fn truncation_at_any_point_is_refused() {
        // Every prefix length, not a few hand-picked ones: a decoder missing a
        // bounds check fails only at the boundary that field happens to
        // straddle. A panic here would abort the module process.
        for op in one_of_each_kind() {
            let bytes = op.canonical_bytes();
            for n in 0..bytes.len() {
                let err = Op::decode(&bytes[..n]).unwrap_err();
                assert!(
                    matches!(err, OpError::Truncated | OpError::LengthMismatch),
                    "truncating to {n} bytes gave {err:?}"
                );
            }
        }
    }

    #[test]
    fn trailing_bytes_are_refused() {
        // Accepting them would let two byte strings decode to the same op
        // while hashing to different ids — the ambiguity canonical encoding
        // exists to remove.
        for op in one_of_each_kind() {
            let mut bytes = op.canonical_bytes();
            bytes.push(0);
            assert_eq!(Op::decode(&bytes), Err(OpError::TrailingBytes));
        }
    }

    #[test]
    fn an_unknown_version_is_refused_and_says_so() {
        // "A newer client wrote this" is a different thing to tell a user than
        // "this is corrupt".
        let mut bytes = a_post().canonical_bytes();
        bytes[0] = 99;
        assert_eq!(Op::decode(&bytes), Err(OpError::UnknownVersion(99)));
    }

    #[test]
    fn an_unknown_op_kind_is_refused() {
        let mut bytes = a_post().canonical_bytes();
        bytes[KIND_AT] = 99;
        assert_eq!(Op::decode(&bytes), Err(OpError::UnknownKind(99)));
    }

    #[test]
    fn an_unknown_moderation_action_is_refused_rather_than_defaulted() {
        // The security-relevant one, mirroring the genesis record's unknown
        // policy. Defaulting an unrecognised action to Hide would apply a
        // moderation nobody performed; defaulting it to Unhide would silently
        // drop one that was.
        let op = Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            kind: OpKind::Moderate {
                target: an_id(1),
                action: ModerationAction::Hide,
            },
        };
        let mut bytes = op.canonical_bytes();
        // The action is the final byte of a moderation op.
        *bytes.last_mut().unwrap() = 99;
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::UnknownModerationAction(99))
        );
    }

    #[test]
    fn an_unknown_vote_direction_is_refused() {
        let op = Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            kind: OpKind::Vote {
                target: an_id(1),
                direction: VoteDirection::Up,
            },
        };
        let mut bytes = op.canonical_bytes();
        *bytes.last_mut().unwrap() = 99;
        assert_eq!(Op::decode(&bytes), Err(OpError::UnknownVoteDirection(99)));
    }

    #[test]
    fn an_invalid_option_tag_is_refused_rather_than_read_as_present() {
        // "Anything non-zero means present" would let a sender and this
        // decoder disagree about the format and still both proceed — which is
        // how two peers derive different ids for one op.
        let op = a_post();
        let bytes = op.canonical_bytes();
        // The thread tag is a Post's first kind-specific byte.
        let tag_at = KIND_FIELDS_AT;
        assert_eq!(bytes[tag_at], 0, "the fixture's thread is absent");
        let mut bad = bytes.clone();
        bad[tag_at] = 2;
        assert_eq!(Op::decode(&bad), Err(OpError::InvalidOptionTag(2)));
    }

    #[test]
    fn an_invalid_author_key_is_refused() {
        // Roughly half of all 32-byte strings are not valid Edwards points, so
        // this is reachable from any peer sending a malformed op. `[0x02; 32]`
        // is genuinely invalid — probed, not assumed, since all-0xFF decodes.
        let mut bytes = a_post().canonical_bytes();
        for b in bytes.iter_mut().skip(AUTHOR_AT).take(32) {
            *b = 0x02;
        }
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::InvalidAuthor(KeyError::NotAValidPublicKey))
        );
    }

    #[test]
    fn a_lying_length_prefix_is_refused() {
        let op = a_post();
        let mut bytes = op.canonical_bytes();
        // Under MAX_FIELD_LEN on purpose: a claim above the cap dies as
        // FieldTooLong and never reaches the input comparison this pins.
        bytes[POST_BODY_LEN_AT..POST_BODY_LEN_AT + LEN_PREFIX]
            .copy_from_slice(&1000u32.to_be_bytes());
        assert_eq!(Op::decode(&bytes), Err(OpError::LengthMismatch));
    }

    #[test]
    fn a_field_over_the_sds_message_cap_is_refused_before_allocating() {
        // A 4-byte prefix can claim 4 GiB. SDS caps a message at 150 KiB
        // (§4.4), so a field larger than that could never have arrived
        // legitimately, and reserving for one is a remote memory-exhaustion
        // lever. The refusal must come from the cap, not from running out of
        // input — so this asserts the specific error.
        let op = a_post();
        let mut bytes = op.canonical_bytes();
        bytes[POST_BODY_LEN_AT..POST_BODY_LEN_AT + LEN_PREFIX]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::FieldTooLong(u32::MAX as usize))
        );
    }

    #[test]
    fn the_field_cap_is_pinned_to_a_known_answer() {
        // The cap's VALUE, which nothing else checks.
        //
        // Every other cap test probes `u32::MAX` — about 28,000x the cap — so
        // it proves that *a* cap exists and nothing at all about *where*. A cap
        // silently raised to 150 MB still refuses 4 GiB and still passes all of
        // them, while leaving open the very memory-exhaustion lever the cap
        // exists to close. That is this repo's own defect class — asserting
        // against a value so far outside the boundary that the boundary is
        // unconstrained — in a new dress.
        //
        // The boundary pair below cannot catch it either: both are expressed in
        // terms of `MAX_FIELD_LEN`, so they MOVE with a drifted cap. Only a
        // hardcoded pin is left, and `cargo mutants` structurally cannot cover
        // it — it mutates functions, not `const` values.
        //
        // 150 KiB is §4.4's SDS message cap. If this fails, do NOT update the
        // expected value: work out why the cap moved and whether the network
        // can survive it.
        assert_eq!(MAX_FIELD_LEN, 150 * 1024);
    }

    /// A post whose body is exactly `len` bytes.
    ///
    /// Returns real bytes rather than a doctored prefix, because the accept
    /// side has to actually contain the data it claims — a prefix claiming
    /// 150 KiB over a short input is a `LengthMismatch` and would prove nothing
    /// about the cap.
    fn a_post_with_body_of(len: usize) -> Vec<u8> {
        Op {
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "a".repeat(len),
                attachments: vec![],
            },
            ..a_post()
        }
        .canonical_bytes()
    }

    #[test]
    fn a_field_exactly_at_the_cap_is_accepted() {
        // The accept half of the boundary. Without it the cap is bounded from
        // one side only, and `len > MAX_FIELD_LEN` tightened to `len >= …`
        // would pass every other test in this module.
        let bytes = a_post_with_body_of(MAX_FIELD_LEN);
        let op = Op::decode(&bytes).expect("a field exactly at the cap must decode");
        match op.kind {
            OpKind::Post { body, .. } => assert_eq!(body.len(), MAX_FIELD_LEN),
            other => panic!("expected a post, got {other:?}"),
        }
    }

    #[test]
    fn a_field_one_byte_over_the_cap_is_refused() {
        // The refuse half, one byte the other side. This is what catches an
        // off-by-one — `len > MAX_FIELD_LEN + 1` — which every `u32::MAX` test
        // in this module survives, and it asserts the exact claimed length so
        // the refusal is demonstrably the cap's and not the input's.
        let bytes = a_post_with_body_of(MAX_FIELD_LEN + 1);
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::FieldTooLong(MAX_FIELD_LEN + 1))
        );
    }

    #[test]
    fn an_attachment_count_over_the_cap_is_refused_before_allocating() {
        // The same lever through the list count rather than a byte length: a
        // huge count with no elements behind it. `Vec::with_capacity(count)`
        // here would allocate on the claim alone.
        let op = Op {
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "x".to_string(),
                attachments: vec![],
            },
            ..a_post()
        };
        let mut bytes = op.canonical_bytes();
        // The attachment count is the final 4 bytes of this encoding.
        let n = bytes.len();
        bytes[n - 4..].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::FieldTooLong(u32::MAX as usize))
        );
    }

    #[test]
    fn an_invalid_utf8_body_is_refused() {
        let op = a_post();
        let mut bytes = op.canonical_bytes();
        bytes[POST_BODY_AT] = 0x80; // a lone continuation byte
        assert_eq!(Op::decode(&bytes), Err(OpError::InvalidText));
    }

    #[test]
    fn a_hostile_op_is_never_a_panic_for_any_input_shape() {
        // The blanket property, because the specific cases above cannot cover
        // every shape an attacker may send. PHASE0-FINDINGS §3: a panic aborts
        // the module process, so every one of these must be a Result.
        //
        // **Seeded from EVERY kind, and every byte of each.** This test was
        // previously seeded from `a_post()` alone and capped at the first 80
        // bytes, which made it a guard that looked blanket and covered one
        // case: a post's kind byte is 0, and the flip set below has never
        // contained every kind discriminant, so no other kind's decode arm was
        // ever the arm being fuzzed. The point of this test is to catch a
        // FUTURE kind added without the bounds-checked cursor discipline, and
        // narrowed that way it could not.
        //
        // `truncation_at_any_point_is_refused` and `trailing_bytes_are_refused`
        // both iterate `one_of_each_kind()` already; this is the third of the
        // blanket properties and it now does too.
        for op in one_of_each_kind() {
            let valid = op.canonical_bytes();
            for n in 0..valid.len() {
                for b in [0u8, 1, 2, 4, 99, 0x80, 0xFF] {
                    let mut bytes = valid.clone();
                    bytes[n] = b;
                    let _ = Op::decode(&bytes);
                    let _ = SignedOp::from_bytes(&bytes);
                }
            }
        }
        // And entirely arbitrary inputs, including the empty one.
        for len in [0usize, 1, 33, 65, 200] {
            for fill in [0u8, 1, 0xFF] {
                let _ = Op::decode(&vec![fill; len]);
                let _ = SignedOp::from_bytes(&vec![fill; len]);
            }
        }
    }

    // ─── What this module deliberately does not decide ────────────────────

    #[test]
    fn verification_answers_authenticity_and_not_authority() {
        // A moderation op signed by someone who is not a moderator is
        // AUTHENTIC — it really is from them — and must still verify here.
        // Authority needs the Stoa's moderator set at this op's Lamport time,
        // which this type does not have, and §3.3 puts on read.
        //
        // §6.2 measured what conflating the two costs: the nearest kin project
        // "checks moderator authority only on the send path and never on the
        // read path, so any peer can forge a moderation". This test pins the
        // split so nobody reads a `true` here as "this hide is binding".
        let random_peer = a_key(9);
        let op = Op {
            stoa: a_stoa(),
            author: random_peer.public_key(),
            kind: OpKind::Moderate {
                target: an_id(1),
                action: ModerationAction::Hide,
            },
        };
        assert!(
            op.sign(&random_peer).verify(),
            "authenticity holds regardless of authority"
        );
    }

    #[test]
    fn a_revision_by_a_different_author_is_authentic_and_still_not_valid() {
        // §5.7: "A version signed by anyone other than the post's original
        // author is invalid and dropped on read." Dropped on READ — this
        // module cannot tell, because it does not have the target op. Pinned
        // so the absence is a decision rather than an oversight.
        let stranger = a_key(9);
        let op = Op {
            stoa: a_stoa(),
            author: stranger.public_key(),
            kind: OpKind::Revise {
                target: an_id(1),
                body: "Not mine to edit".to_string(),
                attachments: vec![],
            },
        };
        assert!(op.sign(&stranger).verify());
    }

    #[test]
    fn an_op_carries_no_ordering_fields() {
        // §5.7 orders by Lamport time with a message-id tiebreak, and §4.4 has
        // SDS assigning both. A self-asserted Lamport value would be forgeable
        // by the very author it orders.
        //
        // The encoding's length is fully accounted for by the fields that ARE
        // here, so a timestamp or sequence number cannot be added without this
        // failing — which is what makes the omission enforced rather than
        // merely documented.
        let op = a_post();
        let body_len = match &op.kind {
            OpKind::Post { body, .. } => body.len(),
            _ => unreachable!(),
        };
        let expected = 1  // version
            + 1           // kind
            + 32          // stoa
            + 32          // author
            + 1           // thread tag (absent)
            + 1           // parent tag (absent)
            + 4 + body_len
            + 4; // attachment count
        assert_eq!(
            op.canonical_bytes().len(),
            expected,
            "the encoding has a field the layout does not account for"
        );
    }

    // ─── The Stoa metadata op ─────────────────────────────────────────────

    fn a_metadata_op() -> Op {
        Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            kind: OpKind::StoaMetadata {
                title: "Renamed".to_string(),
                description: "Now with a description".to_string(),
            },
        }
    }

    #[test]
    fn the_metadata_fields_participate_in_the_encoding() {
        // Varying each in turn catches one left out of the encode arm, which a
        // round-trip test cannot see: the value comes back from the struct it
        // never left.
        let base = a_metadata_op();
        let others = [
            Op {
                kind: OpKind::StoaMetadata {
                    title: "A different name".to_string(),
                    description: "Now with a description".to_string(),
                },
                ..base.clone()
            },
            Op {
                kind: OpKind::StoaMetadata {
                    title: "Renamed".to_string(),
                    description: "A different description".to_string(),
                },
                ..base.clone()
            },
        ];
        for other in others {
            assert_ne!(
                base.canonical_bytes(),
                other.canonical_bytes(),
                "a metadata field is missing from the encoding"
            );
            assert_ne!(
                base.id(),
                other.id(),
                "a metadata field is missing from the id"
            );
        }
    }

    #[test]
    fn a_metadata_ops_title_and_description_cannot_be_confused() {
        // The concatenation trap, and the reason both fields are prefixed.
        // Title "ab" + description "c" against title "a" + description "bc":
        // the same concatenated text, split differently. Without the length
        // prefixes these encode identically — two different acts, one op id.
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        let one = Op {
            stoa,
            author: author.clone(),
            kind: OpKind::StoaMetadata {
                title: "ab".to_string(),
                description: "c".to_string(),
            },
        };
        let two = Op {
            stoa,
            author,
            kind: OpKind::StoaMetadata {
                title: "a".to_string(),
                description: "bc".to_string(),
            },
        };
        assert_ne!(
            one.canonical_bytes(),
            two.canonical_bytes(),
            "two distinct metadata ops must not share an encoding"
        );
        assert_ne!(one.id(), two.id());
    }

    #[test]
    fn a_metadata_op_and_a_revision_do_not_share_a_preimage() {
        // THE property, at the level of the bytes, and the fixture is the whole
        // test: two ops whose kind-specific tails are byte-identical, so that
        // the ONLY thing separating their preimages is the kind byte. Delete
        // `out.push(self.kind.to_byte())` and these encode alike.
        //
        // A `Revise` tail is  target[32] | len(4) | body | count(4)
        // A metadata tail is  len(4) | title | len(4) | description
        //
        // Aligning them: let the target's first four bytes spell the title's
        // length, so 00 00 00 24 (36). The title is then the target's remaining
        // 28 bytes, followed by the body's own 4-byte prefix and the body —
        // 28 + 4 + 4 = 36, as claimed. What follows in the Revise is the
        // attachment count, four zero bytes, which the metadata reads as a
        // zero-length description.
        let stoa = a_stoa();
        let author = a_key(2).public_key();

        let mut target_bytes = [0u8; 32];
        target_bytes[3] = 36;
        // The tail 28 bytes must be valid UTF-8, since they become the title.
        for (i, b) in target_bytes[4..].iter_mut().enumerate() {
            *b = b'a' + (i as u8 % 26);
        }

        let mut title = String::from_utf8(target_bytes[4..].to_vec()).unwrap();
        title.push_str("\u{0}\u{0}\u{0}\u{4}"); // the body's length prefix
        title.push_str("wxyz"); // the body

        let revise = Op {
            stoa,
            author: author.clone(),
            kind: OpKind::Revise {
                target: OpId(target_bytes),
                body: "wxyz".to_string(),
                attachments: vec![],
            },
        };
        let metadata = Op {
            stoa,
            author,
            kind: OpKind::StoaMetadata {
                title,
                description: String::new(),
            },
        };

        // Assert the fixture really is aligned before asserting the property.
        // Without this the test would pass whenever the two merely differ,
        // which they do for a dozen reasons having nothing to do with the kind
        // byte — the trap this project has shipped three times.
        assert_eq!(
            revise.canonical_bytes()[KIND_FIELDS_AT..],
            metadata.canonical_bytes()[KIND_FIELDS_AT..],
            "the fixture must differ ONLY in the kind byte"
        );
        assert_ne!(
            revise.canonical_bytes(),
            metadata.canonical_bytes(),
            "a revision and a metadata op must not share a signing preimage"
        );
        assert_ne!(revise.id(), metadata.id());
    }

    #[test]
    fn a_signature_over_a_metadata_op_does_not_verify_as_a_moderation() {
        // The consequence at the level that matters, and the pairing that makes
        // it worth having: a metadata op and a moderation are BOTH
        // moderator-signed acts by the same key in the same Stoa. If a
        // signature did not commit to which, a moderator persuaded to rename a
        // Stoa would have signed a hide.
        let key = a_key(2);
        let stoa = a_stoa();

        let metadata = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::StoaMetadata {
                title: "Renamed".to_string(),
                description: String::new(),
            },
        };
        let signed_metadata = metadata.sign(&key);

        let forged = SignedOp {
            op: Op {
                stoa,
                author: key.public_key(),
                kind: OpKind::Moderate {
                    target: an_id(1),
                    action: ModerationAction::Hide,
                },
            },
            signature: signed_metadata.signature.clone(),
        };
        assert!(
            !forged.verify(),
            "a rename's signature must not authorise a hide"
        );
        // And the original is genuinely valid, so this is not passing because
        // both are broken.
        assert!(signed_metadata.verify());
    }

    #[test]
    fn a_metadata_op_carries_no_policy_and_no_ordering_field() {
        // The layout pinned against hardcoded sizes, as
        // `an_op_carries_no_ordering_fields` does for a post. A `policy` byte,
        // a Lamport value or a sequence number cannot be added to the encode
        // arm without this failing — which is what makes each omission enforced
        // rather than merely documented.
        //
        // The title and description lengths are the LITERALS below, not
        // `title.len()` read back off the fixture: asking the implementation
        // what it wrote and agreeing is the defect this project has shipped
        // three times.
        let op = Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            kind: OpKind::StoaMetadata {
                title: "abcde".to_string(),         // 5 bytes
                description: "fghijkl".to_string(), // 7 bytes
            },
        };
        let expected = 1  // version
            + 1           // kind
            + 32          // stoa
            + 32          // author
            + 4 + 5       // title, length-prefixed
            + 4 + 7; // description, length-prefixed
        assert_eq!(
            op.canonical_bytes().len(),
            expected,
            "the metadata encoding has a field the layout does not account for"
        );
    }

    #[test]
    fn an_over_long_metadata_title_is_refused_before_allocating() {
        // A 4-byte prefix can claim 4 GiB, and SDS caps a message at 150 KiB
        // (§4.4), so a field larger than that could never have arrived
        // legitimately. The refusal must come from the CAP rather than from
        // running out of input, which is why this asserts the specific error.
        let mut bytes = a_metadata_op_with_lengths(2, 2);
        let at = metadata_offsets(2).title_len_at;
        bytes[at..at + LEN_PREFIX].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::FieldTooLong(u32::MAX as usize))
        );
    }

    #[test]
    fn an_over_long_metadata_description_is_refused_before_allocating() {
        // The second field needs its own case: a cap applied to the first
        // string only would leave this one an open memory-exhaustion lever.
        let mut bytes = a_metadata_op_with_lengths(2, 2);
        let at = metadata_offsets(2).description_len_at;
        bytes[at..at + LEN_PREFIX].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::FieldTooLong(u32::MAX as usize))
        );
    }

    /// A metadata op with a title of `title_len` and a description of
    /// `description_len`, as real bytes.
    fn a_metadata_op_with_lengths(title_len: usize, description_len: usize) -> Vec<u8> {
        Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            kind: OpKind::StoaMetadata {
                title: "a".repeat(title_len),
                description: "b".repeat(description_len),
            },
        }
        .canonical_bytes()
    }

    #[test]
    fn a_metadata_field_exactly_at_the_cap_is_accepted() {
        // The accept half of the boundary, for both fields at once. The two
        // together decode to over 300 KB — twice the SDS message cap — which is
        // deliberate and is documented on `MAX_FIELD_LEN`: the cap bounds
        // allocation-before-data per field, not the total size of an op. The
        // total-size check belongs at the transport boundary, where the frame
        // is known.
        let bytes = a_metadata_op_with_lengths(MAX_FIELD_LEN, MAX_FIELD_LEN);
        let op = Op::decode(&bytes).expect("fields exactly at the cap must decode");
        match op.kind {
            OpKind::StoaMetadata { title, description } => {
                assert_eq!(title.len(), MAX_FIELD_LEN);
                assert_eq!(description.len(), MAX_FIELD_LEN);
            }
            other => panic!("expected metadata, got {other:?}"),
        }
    }

    #[test]
    fn a_metadata_field_one_byte_over_the_cap_is_refused() {
        // The refuse half, one byte over, for EACH field independently — a cap
        // applied to the title only would leave the description open, and a
        // `u32::MAX` probe cannot tell the difference.
        for (title_len, description_len) in [(MAX_FIELD_LEN + 1, 2), (2, MAX_FIELD_LEN + 1)] {
            let bytes = a_metadata_op_with_lengths(title_len, description_len);
            assert_eq!(
                Op::decode(&bytes),
                Err(OpError::FieldTooLong(MAX_FIELD_LEN + 1)),
                "title {title_len}, description {description_len}"
            );
        }
    }

    #[test]
    fn a_lying_metadata_length_prefix_is_refused_in_either_direction() {
        // The spec requires each malformation be "reported distinguishably",
        // and calls out a prefix disagreeing "in either direction". Both
        // directions were previously covered for a Post only.
        //
        // OVER-claiming is a LengthMismatch: the input is not short, the claim
        // is wrong. UNDER-claiming is TrailingBytes: the decoder reads the
        // field it was promised, then finds bytes after it. Different errors,
        // same refusal, and the distinction is worth pinning — a caller
        // matching only on LengthMismatch would mishandle the second.
        // Over-claiming, on the title. 1000 stays under MAX_FIELD_LEN, or this
        // would die as FieldTooLong and never reach the input comparison.
        let mut over = a_metadata_op_with_lengths(4, 4);
        let over_at = metadata_offsets(4).title_len_at;
        over[over_at..over_at + LEN_PREFIX].copy_from_slice(&1000u32.to_be_bytes());
        assert_eq!(Op::decode(&over), Err(OpError::LengthMismatch));

        // Under-claiming, on the DESCRIPTION rather than the title, because
        // the description is the last field. Shrinking the title's prefix
        // instead would make the decoder read the description's length from
        // the middle of the title's text, and the resulting error would be
        // whatever those four bytes happened to spell — a test passing for a
        // reason unrelated to under-claiming.
        let mut under = a_metadata_op_with_lengths(4, 6);
        let under_at = metadata_offsets(4).description_len_at;
        under[under_at..under_at + LEN_PREFIX].copy_from_slice(&2u32.to_be_bytes());
        assert_eq!(Op::decode(&under), Err(OpError::TrailingBytes));
    }

    #[test]
    fn the_metadata_caps_two_length_bounds_are_reported_distinguishably() {
        // There are TWO bounds on one length prefix, and they must not collapse
        // into one error:
        //
        //   1. the CAP   — len > MAX_FIELD_LEN            => FieldTooLong
        //   2. the INPUT — a claim under the cap, past the
        //                  bytes that remain               => LengthMismatch
        //
        // Each is pinned separately elsewhere; nothing pinned that they stay
        // DIFFERENT, and a decoder reporting either for both would pass every
        // one of those tests. A caller matching on FieldTooLong to say "no peer
        // could have sent that" and on LengthMismatch to say "this op is
        // corrupt" gets both wrong if they merge.
        //
        // The op-format analogue of the same property op-model pinned for the
        // genesis record — two agents reaching it from opposite ends.
        let at = metadata_offsets(4).title_len_at;

        let mut over_cap = a_metadata_op_with_lengths(4, 4);
        over_cap[at..at + LEN_PREFIX].copy_from_slice(&((MAX_FIELD_LEN + 1) as u32).to_be_bytes());

        // Under the cap on purpose. A claim of u32::MAX would die as
        // FieldTooLong and never reach the input comparison at all — which is
        // exactly how the genesis lying-prefix test stopped testing anything
        // once a cap landed in front of it.
        let mut past_input = a_metadata_op_with_lengths(4, 4);
        past_input[at..at + LEN_PREFIX]
            .copy_from_slice(&((MAX_FIELD_LEN - 1) as u32).to_be_bytes());

        assert_eq!(
            Op::decode(&over_cap),
            Err(OpError::FieldTooLong(MAX_FIELD_LEN + 1))
        );
        assert_eq!(Op::decode(&past_input), Err(OpError::LengthMismatch));
    }

    #[test]
    fn trailing_bytes_after_a_metadata_op_are_refused() {
        // Metadata-specific rather than relying on the generic loop: the
        // decode arm consumes two variable-length fields and then must leave
        // the cursor exhausted. An arm that stopped early would be caught only
        // by `trailing_bytes_are_refused`, and a future kind is as likely to
        // regress this as any other property.
        let mut bytes = a_metadata_op().canonical_bytes();
        bytes.push(0);
        assert_eq!(Op::decode(&bytes), Err(OpError::TrailingBytes));
    }

    #[test]
    fn invalid_utf8_in_metadata_is_refused() {
        // Both fields, and not lossily converted — `from_utf8_lossy` would map
        // distinct inputs onto one op, which is the ambiguity a canonical
        // encoding exists to remove.
        let offsets = metadata_offsets(2);
        for at in [offsets.title_at, offsets.description_at] {
            let mut bytes = a_metadata_op_with_lengths(2, 2);
            bytes[at] = 0x80; // a lone continuation byte
            assert_eq!(Op::decode(&bytes), Err(OpError::InvalidText));
        }
    }

    #[test]
    fn a_metadata_op_round_trips_through_multibyte_and_empty_text() {
        // Empty is legitimate and must not be confused with absent — a zero
        // length prefix is a real encoding.
        for (title, description) in [
            ("", ""),
            ("Ἀγορά", "ἡ ἀγορά — the marketplace"),
            ("🏛", ""),
            ("", "a\0b"),
        ] {
            let op = Op {
                kind: OpKind::StoaMetadata {
                    title: title.to_string(),
                    description: description.to_string(),
                },
                ..a_metadata_op()
            };
            assert_eq!(Op::decode(&op.canonical_bytes()).unwrap(), op);
        }
    }

    #[test]
    fn display_text_is_preserved_exactly_and_never_normalised() {
        // Strict UTF-8 with NO normalisation, and that is a canonicality
        // requirement rather than an oversight. Normalising, case-folding or
        // stripping at decode would mean an accepted byte string re-encodes to
        // something other than itself — two peers, two op ids, for what each
        // believes is one op. The encoding's whole job is that they agree.
        //
        // The consequence is that display text is attacker-controlled: these
        // are exactly the shapes used to impersonate a Stoa by name — a
        // right-to-left override, a zero-width joiner, a Cyrillic homoglyph of
        // "Agora". They MUST survive intact here, and the obligation to render
        // them safely sits with the renderer. §4.8 already says a name is never
        // an identifier; this is the data-layer half of that.
        for hostile in [
            "Agora\u{202E}txet desrever", // right-to-left override
            "Ag\u{200B}ora",              // zero-width space
            "\u{0410}gora",               // Cyrillic А, a homoglyph of A
            "agora",                      // differs from "Agora" only by case
            "e\u{0301}",                  // combining acute, NOT folded to é
        ] {
            let op = Op {
                kind: OpKind::StoaMetadata {
                    title: hostile.to_string(),
                    description: String::new(),
                },
                ..a_metadata_op()
            };
            let bytes = op.canonical_bytes();
            let decoded = Op::decode(&bytes).unwrap();
            match &decoded.kind {
                OpKind::StoaMetadata { title, .. } => {
                    // Compared against the ORIGINAL literal, not against
                    // anything the decoder produced.
                    assert_eq!(title, hostile, "display text was transformed");
                }
                other => panic!("expected metadata, got {other:?}"),
            }
            // And re-encoding reproduces the same bytes, which is the
            // canonicality property the non-transformation exists to protect.
            assert_eq!(decoded.canonical_bytes(), bytes);
        }

        // The pair that would collide under NFC normalisation must stay
        // distinct: "e" + combining acute against the precomposed "é".
        let combining = Op {
            kind: OpKind::StoaMetadata {
                title: "e\u{0301}".to_string(),
                description: String::new(),
            },
            ..a_metadata_op()
        };
        let precomposed = Op {
            kind: OpKind::StoaMetadata {
                title: "\u{00E9}".to_string(),
                description: String::new(),
            },
            ..a_metadata_op()
        };
        assert_ne!(
            combining.canonical_bytes(),
            precomposed.canonical_bytes(),
            "normalisation would collapse two distinct ops onto one encoding"
        );
        assert_ne!(combining.id(), precomposed.id());
    }

    #[test]
    fn renaming_a_stoa_does_not_change_its_address() {
        // The whole point of the split. The genesis record is immutable and
        // address-determining; the metadata op carries the current name. A
        // rename that moved the address would not be a rename, it would be a
        // different Stoa — which is exactly what editing the genesis title
        // does, and why this op exists.
        let genesis = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        };
        // Fallible since main capped the genesis title at `MAX_TITLE_BYTES`;
        // "Agora" is five bytes, so the error arm is unreachable here.
        let address = genesis
            .address()
            .expect("a five-byte title is well under MAX_TITLE_BYTES");

        let rename = Op {
            stoa: address,
            author: a_key(1).public_key(),
            kind: OpKind::StoaMetadata {
                title: "The Agora".to_string(),
                description: "Renamed".to_string(),
            },
        };
        assert!(rename.sign(&a_key(1)).verify());

        // The address is still the genesis record's, and the genesis record
        // still carries its FOUNDING title — both remain answerable.
        assert_eq!(
            genesis
                .address()
                .expect("a five-byte title is well under MAX_TITLE_BYTES"),
            address
        );
        assert_eq!(genesis.title, "Agora");
        // And the hardcoded address from `stoa.rs`'s pinned known-answer test,
        // so this is checked against a value no code in this test produced.
        assert_eq!(
            address.to_hex(),
            "80329cf05603a0c9ce7a749a53e271253307ba89d4924856e4017459d03a025f",
            "a rename must not move the Stoa address"
        );
    }

    #[test]
    fn a_metadata_op_by_a_non_moderator_is_authentic() {
        // Specified, by the "authenticity, not authority" requirement: a
        // metadata op signed by a peer who is not a moderator really IS from
        // that peer and must verify. Whether the rename BINDS needs the Stoa's
        // moderator set, which this type does not have and §3.3 puts on read.
        //
        // Pinned so that nobody reads `verify() == true` as "this Stoa is now
        // called that" — which is precisely the conflation §6.2 measured in the
        // nearest kin project.
        let random_peer = a_key(9);
        let op = Op {
            stoa: a_stoa(),
            author: random_peer.public_key(),
            kind: OpKind::StoaMetadata {
                title: "Hijacked".to_string(),
                description: String::new(),
            },
        };
        assert!(
            op.sign(&random_peer).verify(),
            "authenticity holds regardless of authority"
        );
    }

    #[test]
    fn a_metadata_op_replayed_into_another_stoa_does_not_verify() {
        // A rename lifted from one Stoa's channel and put on another's must
        // fail, rather than arriving as a rename of a Stoa its signer never
        // addressed. The Stoa is inside the signed bytes, which is what makes
        // this hold.
        let key = a_key(2);
        let signed = a_metadata_op().sign(&key);
        let elsewhere = SignedOp {
            op: Op {
                stoa: crate::identity::stoa_address(b"a different stoa"),
                ..signed.op.clone()
            },
            signature: signed.signature.clone(),
        };
        assert!(!elsewhere.verify());
    }

    #[test]
    fn an_unallocated_kind_discriminant_is_refused_as_unknown() {
        // What makes "a later kind costs an unused discriminant, not a version
        // bump" true: an older client meeting a discriminant it does not know
        // refuses with a NAMED error rather than misparsing.
        //
        // This pins the PROPERTY, not a particular number. Naming "5" in the
        // prose would go stale the moment another kind lands there while this
        // test still passed — so the value is found by scanning upward from
        // the highest allocated discriminant instead. Add a kind and this
        // keeps testing the first genuinely free value.
        let highest = one_of_each_kind()
            .iter()
            .map(|op| op.canonical_bytes()[KIND_AT])
            .max()
            .expect("one_of_each_kind is never empty");
        let unallocated = highest + 1;

        let mut bytes = a_metadata_op().canonical_bytes();
        bytes[KIND_AT] = unallocated;
        assert_eq!(
            Op::decode(&bytes),
            Err(OpError::UnknownKind(unallocated)),
            "discriminant {unallocated} must be refused, not misparsed"
        );
    }

    #[test]
    fn the_metadata_kind_discriminant_is_pinned_to_a_known_answer() {
        // Consensus-critical: the discriminant is inside every signature and
        // every op id, so changing it re-mints the id of every metadata op in
        // existence with no error anywhere, because each peer stays internally
        // consistent.
        //
        // Hardcoded, not read back from `OpKind::STOA_METADATA` — asking the
        // implementation what it wrote and agreeing is the defect this project
        // has shipped three times. If this fails, do NOT update the expected
        // value: work out what changed and whether the network survives it.
        assert_eq!(
            a_metadata_op().canonical_bytes()[KIND_AT],
            4,
            "the Stoa metadata kind discriminant changed"
        );
        // And it is distinct from every discriminant already allocated.
        for op in one_of_each_kind() {
            if matches!(op.kind, OpKind::StoaMetadata { .. }) {
                continue;
            }
            assert_ne!(
                op.canonical_bytes()[KIND_AT],
                4,
                "another kind reuses the Stoa metadata discriminant"
            );
        }
    }
}
