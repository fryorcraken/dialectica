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
//! # The two clock fields, and why their authority is unequal
//!
//! An op of the current version carries a **Lamport counter** and an
//! **author-asserted wall-clock**, both inside the signed preimage. They are not
//! two of a kind, and reading them as one is the way this goes wrong:
//!
//! - **The counter is authoritative for every ordering.**
//!   [`crate::arrival::cmp_ops`] leads with it and reads nothing else but the op
//!   id. Its being inside the preimage is what a relay cannot defeat: raising it
//!   to promote an op, lowering it to bury one, or stripping it to force the
//!   degraded path each alters the bytes, so the signature fails and the op is
//!   refused.
//! - **The wall-clock decides nothing at all.** The author picks it and an
//!   author can lie. It is carried because a reader is shown a time and expects
//!   one, and it is defended not by a clamp but by having no decision to reach:
//!   it orders nothing, breaks no tie, and gates nothing.
//!
//! **This file previously argued against both fields**, and the argument is
//! answered rather than dropped. It read: *"A self-asserted Lamport value would
//! be forgeable by the author it is meant to order, which defeats the purpose"*,
//! and *"a forum that ordered by [a wall clock] would be ordering by a field its
//! adversary sets"*. Both observations are true and neither is withdrawn. What
//! changed is that the transport supplies no alternative and is not going to —
//! the Reliable Channel API's `MessageReceivedEvent` carries exactly one field,
//! the reassembled payload, so `delivery_module`'s `channelMessageReceived`
//! cannot forward a Lamport value it was never given. The practical effect of
//! the prohibition was therefore not a transport-assigned order but **no order
//! at all**, with every resolver falling back to a hash.
//!
//! Each objection is answered somewhere a test can reach:
//!
//! - The **forgeable counter** is bounded by the receive window
//!   ([`crate::arrival::RECEIVE_WINDOW_MS`]): a peer refuses an op whose counter
//!   is more than one hour ahead of its own current time, so an author can lead
//!   honest ops by at most that hour and cannot place an op beyond it at all.
//!   This replaced an advance bound that stored every counter, and it is a
//!   refusal on purpose; the cost is in the archived `time-pegged-clock`
//!   change's `design.md`, Decision 11.
//! - The **adversary-set wall clock** is not bounded into safety; it is removed
//!   from every decision, which is the stronger defence because there is no
//!   decision left for a forged value to reach. The window reads the counter and
//!   never this field.
//!
//! **Why they remain two fields, now that both carry a time.** The counter is
//! pegged to its author's clock ([`crate::arrival::next_counter`]), so it is a
//! claim about the time just as the wall-clock is. What separates them is what
//! each is held to: the counter is checked from above against the receiving
//! peer's own time and is raised past every counter its author held, so it may
//! order. The wall-clock is checked by nothing, so it may not.
//!
//! # What an op does NOT carry, and why each omission is deliberate
//!
//! **No transport message id, and no sequence number of any kind.** §4.3's rule
//! for the channel id — "not a session counter, not a local sequence number, not
//! anything that varies with one peer's history" — applies with equal force to a
//! value inside a signed op that every peer must agree about. [`crate::arrival`]
//! holds what a peer records about a delivery; it no longer orders anything.
//!
//! **No `senderId`.** §4.1: "`senderId` is not an author identity, and the plan
//! should not treat it as one." It binds at channel creation as a transport
//! self-filter. The author identity in an op is the key it carries, and nothing
//! else — issue #80 deleted the author address that used to be named here
//! alongside it.
//!
//! **No `channelId`.** §4.5: "never let channel identity leak into payloads or
//! storage keys", so that one-channel-per-thread later becomes a routing change
//! rather than a migration. The Stoa *address* is carried instead — it is a
//! pure function of the addressed object, which is exactly what §4.5 asks for.
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
/// Distinct from the Stoa-address prefix in `identity.rs`, so no byte string is
/// ever both a valid op id and a valid Stoa address. Padded to a
/// fixed 32 bytes for the same reason those are: a variable-length prefix
/// concatenated with variable-length data is how two different inputs come to
/// hash the same.
const OP_ID_PREFIX: &[u8; 32] = b"/dialectica/1/Id/Op\0\0\0\0\0\0\0\0\0\0\0\0\0";

/// The encoding generation predating the clock fields.
///
/// Not the same discriminant as the genesis record's, and not shared with it:
/// the two formats version independently, because a change to one has no reason
/// to invalidate the other.
///
/// **Still decoded, never written.** [`Op::canonical_bytes`] emits this only for
/// an op whose [`Op::clock`] is `None`, which nothing in this build constructs
/// except a decode of bytes that already carried it. An op of this version keeps
/// the id it always had: its preimage is byte-for-byte what it was, because the
/// clock fields are written only in the `Some` arm.
const VERSION_1: u8 = 1;

/// The encoding generation carrying the Lamport counter and the wall-clock.
///
/// **A version increment and not a new kind discriminant.** The kinds are
/// unchanged; what changed is what every kind carries, so the discriminant that
/// has to move is the one covering the whole preimage. A kind-level answer would
/// have needed five new kinds and would have left `Op::kind` meaning two things.
const VERSION_2: u8 = 2;

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
///
/// # Why this is `pub`, and what depends on it
///
/// **A writer needs the same number the decoder enforces.** The encode helpers do
/// not check it — `put_bytes` writes any length — on the reasoning that the cap is
/// "checked on the way back in". That reasoning holds for ops that *arrive* and not
/// for ops this peer *creates*: an over-cap field encodes, signs and stores
/// happily, and is then refused by this module's own decoder, which is a row no
/// read can get past. So any path that builds an `Op` from caller input must refuse
/// the field before signing it, and to do that it has to see this value.
///
/// `authoring::MAX_BODY_LEN` is that use, and
/// `the_publish_body_cap_is_the_format_field_cap` pins the two as one number rather
/// than two that agree today.
pub const MAX_FIELD_LEN: usize = 150 * 1024;

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
        /// post carries `None`.
        ///
        /// **Nothing fills this in later, and nothing can.** An earlier version
        /// of this comment said the store filled the thread in as the op's own
        /// id on ingest; it does not, and it could not — the op is signed, so a
        /// store rewriting a field would invalidate the signature it is stored
        /// with. The correction matters because a reader who believed it would
        /// expect `thread` to be non-`None` on every stored root and would
        /// derive a reply's thread wrongly.
        ///
        /// So `None` on a root is permanent, and the thread an op belongs to is
        /// **derived**: its own `thread` when it has one, its own op id when it
        /// does not. [`crate::authoring`] does that on the publish path and
        /// records what the derivation trusts.
        ///
        /// # THIS FIELD IS THE AUTHOR'S CLAIM, AND THE READ SIDE NEVER BELIEVES IT
        ///
        /// The derivation above is correct **only where the op is this peer's
        /// own**, which is the publish path and nowhere else. An op arriving from
        /// a peer carries whatever its author chose, and the log stores it
        /// unexamined because the log decides nothing. A reader that placed posts
        /// by this field would let any peer inject a post into any thread it
        /// names — authentically signed, verifying perfectly, and rendered inside
        /// a conversation it was never part of.
        ///
        /// So there are **two** derivations in this crate with opposite rules,
        /// and which one is correct depends on where the op came from:
        ///
        /// - [`crate::authoring`]'s reads this field and trusts it. Right, for
        ///   ops this peer creates.
        /// - [`crate::thread::thread_of`] never reads it at all, and derives
        ///   membership by following `parent` to a root. **That is the rule for
        ///   anything that arrived over the network**, which is every op a read
        ///   renders.
        ///
        /// Reach for the second unless you are on the publish path. The field
        /// stays in the op because it is what a future per-thread routing split
        /// needs, not because a reader may rely on it.
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

/// The two clock values an op of the current version carries.
///
/// # Why one struct rather than two fields on [`Op`]
///
/// **They are present or absent together, and nothing else is representable.**
/// An op of [`VERSION_2`] carries both; an op of [`VERSION_1`] carries neither.
/// A pair of independent `Option`s beside each other would admit two states the
/// format does not have — a counter with no wall-clock, and the reverse — and
/// each would need a guard at every site that reads either. One `Option` around
/// this struct makes the encoding version and the presence of the fields **the
/// same fact**, which is CLAUDE.md's "put the complexity in the data structure".
///
/// That is also what lets `Op` answer "does this carry a counter?" without
/// anyone inferring it from an ordering result or from a sentinel value: it is
/// [`Option::is_some`] on a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpClock {
    /// The author's Lamport counter for this op's Stoa.
    ///
    /// **Authoritative for every ordering**, and forgeable by its author like
    /// every other field they sign. An honest author signs the later of its
    /// current time and one above its clock ([`crate::arrival::next_counter`]).
    /// The defence is not that the value is trustworthy — it is that a peer
    /// refuses an op whose counter is more than
    /// [`crate::arrival::RECEIVE_WINDOW_MS`] ahead of its own time, so an author
    /// can lead honest ops by at most an hour.
    pub counter: u64,
    /// Milliseconds since the Unix epoch, **as the author asserts them**.
    ///
    /// # This value decides nothing, and that is its whole defence
    ///
    /// It SHALL NOT order, break a tie, establish which of two ops came first,
    /// decide an expiry or a rate limit, or stand in for a counter that is
    /// absent. A hostile author writes whatever ranks best and a broken one
    /// writes whatever its system clock says; neither matters, because there is
    /// no decision for either to reach.
    ///
    /// **It is never handed out as a number.** A read surfaces it only through
    /// [`crate::asserted_time::format_asserted`], which returns display text and
    /// a clamped flag. A view that wished to sort on it would have to parse a
    /// string back into an instant first, which is the intent: it makes the
    /// wrong thing visibly wrong in a diff rather than a plausible field access.
    /// The nearest comparable project read an author-asserted timestamp for
    /// ranking decay with nothing clamping it, and the defect was not a missing
    /// check — it was that the value sat there as a number for whoever wanted to
    /// rank by it.
    ///
    /// Every representable value is accepted and stored: an op whose counter is
    /// admissible is stored whatever this field says, because the receive window
    /// reads the counter alone. That is narrower than it was. A skewed clock
    /// does get an author's ops refused — through the counter, which an honest
    /// author pegs to the same clock — and the archived `time-pegged-clock`
    /// change's `design.md`, Decision 11, works that cost through. What holds is that this field is
    /// never the reason.
    pub asserted_ms: u64,
}

/// A signed operation: the whole of what crosses the wire.
///
/// The author's **public key** travels in the op, and it is the whole of how an
/// op names its author. §3.3 puts verification on read with no directory to
/// resolve any other identifier against, so a peer must hold the key itself in
/// order to check the signature.
///
/// **No second author identifier is carried or derivable.** The key is the
/// author, so there is nothing beside it for a recipient to reconcile it
/// against, and nothing a relay could strip or forge separately from the value
/// the signature is checked under. An author address used to ride here too;
/// issue #80 deleted it.
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
    /// The clock values, or `None` for an op encoded before they existed.
    ///
    /// **`None` is not "a counter of zero".** Zero is a representable counter
    /// and orders above nothing; absence orders **below every op that carries
    /// one**, which is this change's whole migration answer. Conflating the two
    /// would place every pre-existing op among the new ones by a value nobody
    /// asserted.
    ///
    /// **A public field, deliberately.** Every inline `Op { .. }` construction
    /// must supply it, so the compiler enumerates the publish sites rather than
    /// a grep somebody has to remember to run — which is the recorded
    /// stale-sweep-list trap, and the reason this is not a defaulted builder.
    pub clock: Option<OpClock>,
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
    /// A metadata op's title is blank, as [`crate::stoa::is_blank_title`]
    /// defines it: empty, or made only of the thirty blank characters.
    ///
    /// One variant for every blank title, the empty one included, for the
    /// reason [`crate::stoa::GenesisError::BlankTitle`] gives. The description
    /// is not held to this: an empty or blank description is valid.
    BlankTitle,
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
            OpError::BlankTitle => write!(
                f,
                "a metadata op's title is blank: it has no character other than \
                 whitespace or zero-width characters"
            ),
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
    /// counter    8 bytes     <- VERSION_2 only, big-endian u64
    /// asserted   8 bytes     <- VERSION_2 only, big-endian u64
    /// <kind-specific fields>
    /// ```
    ///
    /// A variable-length field is a 4-byte big-endian length then its bytes; an
    /// optional field is a 1-byte tag (0 absent, 1 present) then, if present,
    /// the value. Both rules exist for the same reason `stoa.rs` gives:
    /// concatenating variable-length fields lets distinct records collide,
    /// because `("ab","c")` and `("a","bc")` produce identical bytes.
    ///
    /// # The clock fields carry NO presence tag, and that is the format
    ///
    /// They are fixed-width at a fixed offset, and whether they are there is
    /// decided by the **version byte** rather than by a tag beside them. An op
    /// declaring [`VERSION_2`] either carries sixteen bytes of clock or is not
    /// an op of that version and fails to decode.
    ///
    /// A presence tag would have made "a version-2 op with the fields stripped"
    /// a representable, decodable op — which is exactly the downgrade a relay
    /// must not be able to reach. It cannot reach it through the version byte
    /// either, because that byte is inside the preimage: rewriting it breaks the
    /// signature just as rewriting a counter does. So "some ops carry no
    /// counter" is a fact about ops signed before the field existed, never a
    /// state an attacker can manufacture.
    ///
    /// # A version-1 op's bytes are unchanged by this change
    ///
    /// The clock bytes are written only in the `Some` arm, so an op whose
    /// [`Op::clock`] is `None` encodes to exactly what it always did and hashes
    /// to exactly the id it always had. The migration needs no rewrite, no
    /// re-signing and no recomputation, and it holds by construction rather than
    /// by a test comparing against a recorded constant.
    ///
    /// # This is the layout, and [`Op::encode`] is the encoding
    ///
    /// Total over every `Op` value: it lays out whatever the struct holds, and it
    /// is what an op id and a signature are computed over. Whether an op *has an
    /// encoding* — whether the format admits it at all — is [`Op::encode`]'s
    /// question, and [`SignedOp::to_bytes`] goes through that one. For an op the
    /// format admits, the two return the same bytes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // The version says whether the clock fields are present, so the two are
        // written from one `match` and cannot disagree. A `push(VERSION_2)`
        // separated from the field write by twenty lines is how a build emits a
        // version whose bytes it did not write.
        match &self.clock {
            Some(clock) => {
                out.push(VERSION_2);
                // The kind leads, as it did before: this is the op-kind
                // separation `identity.rs` said the serialiser owed it. Two ops
                // of different kinds cannot share a preimage, so no signature
                // over one is a signature over the other.
                out.push(self.kind.to_byte());
                out.extend_from_slice(self.stoa.as_bytes());
                out.extend_from_slice(&self.author.to_bytes());
                // Big-endian, like every other multi-byte integer in this
                // format, so a reader never has to remember which field is
                // which endianness.
                out.extend_from_slice(&clock.counter.to_be_bytes());
                out.extend_from_slice(&clock.asserted_ms.to_be_bytes());
            }
            None => {
                out.push(VERSION_1);
                out.push(self.kind.to_byte());
                out.extend_from_slice(self.stoa.as_bytes());
                out.extend_from_slice(&self.author.to_bytes());
            }
        }

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

    /// The op's encoding: its canonical bytes, for an op the format admits.
    ///
    /// Fallible so that the encoder refuses exactly what [`Op::decode`] refuses
    /// — the symmetry `stoa.rs` keeps for the genesis record, where a record the
    /// decoder would reject must not be one the encoder will produce. Everything
    /// that writes an op's bytes out of this process goes through here, by way
    /// of [`SignedOp::to_bytes`].
    pub fn encode(&self) -> Result<Vec<u8>, OpError> {
        self.check_admitted()?;
        Ok(self.canonical_bytes())
    }

    /// Whether the format admits this op at all, beyond its structure.
    ///
    /// **The one guard, with two callers**: [`Op::encode`] and [`Op::decode`].
    /// One function is what makes "the encoder refuses exactly what the decoder
    /// refuses" hold by construction rather than by two checks kept in step.
    ///
    /// One rule today: a [`OpKind::StoaMetadata`] title must not be blank. The
    /// description is exempt, because an empty description is a value and not an
    /// absence.
    ///
    /// **Not called by [`Op::canonical_bytes`], [`Op::id`], [`Op::sign`] or
    /// [`SignedOp::verify`], deliberately.** An op the format does not admit can
    /// still be built as a struct, signed and held in memory, and
    /// `stoa-metadata`'s resolver is required to refuse one "a reader
    /// nonetheless holds". If this guard sat under `verify`, such an op would
    /// fail the authenticity check first, and the resolver's own blank check
    /// would never be what refused it. Deleting that check would then leave
    /// every test green. The change's `design.md` decision 2 has the
    /// alternatives.
    fn check_admitted(&self) -> Result<(), OpError> {
        match &self.kind {
            OpKind::StoaMetadata { title, .. } if crate::stoa::is_blank_title(title) => {
                Err(OpError::BlankTitle)
            }
            _ => Ok(()),
        }
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

        // Both versions decode, and which one this is is reported through
        // `Op::clock` rather than through a separate field. `op-ordering` needs
        // to place an op carrying no counter, and it asks the op.
        let carries_clock = match cursor.take(1)?[0] {
            VERSION_1 => false,
            VERSION_2 => true,
            // Named rather than reported as malformed: an unrecognised version
            // means "a newer client wrote this", which is a different fact from
            // corruption and calls for a different response.
            other => return Err(OpError::UnknownVersion(other)),
        };

        let kind_byte = cursor.take(1)?[0];
        let stoa = Address::from_bytes(cursor.take_array::<32>()?);
        let author = PublicKey::from_bytes(cursor.take(32)?).map_err(OpError::InvalidAuthor)?;

        // Sixteen fixed bytes, or nothing. A version-2 op whose clock bytes were
        // removed runs out of input here — or, if the kind's own fields happen
        // to supply enough bytes to read, fails the `finish()` check below when
        // the tail does not line up. Either way it is refused, and it is NEVER
        // read as an op of the earlier version: the version byte said what it
        // is, and this decoder does not second-guess it.
        //
        // No clock value is validated. Every `u64` is a representable counter
        // and every `u64` is a representable instant; refusing one here would be
        // refusing an op for a field, which is the censorship vector this format
        // deliberately does not open. Nothing is compared against a local clock,
        // so decoding reads no clock and its result does not depend on when it
        // happened.
        let clock = if carries_clock {
            Some(OpClock {
                counter: u64::from_be_bytes(cursor.take_array::<8>()?),
                asserted_ms: u64::from_be_bytes(cursor.take_array::<8>()?),
            })
        } else {
            None
        };

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

        let op = Op {
            stoa,
            author,
            clock,
            kind,
        };
        // AFTER the structure, as `Genesis::decode` does: an input wrong in two
        // ways reports the structural fault. And through the guard `encode`
        // uses, so the two sides cannot disagree about what is admitted.
        op.check_admitted()?;
        Ok(op)
    }

    /// This op's id: the hash of its canonical bytes, domain-separated.
    ///
    /// Prefixed like every other hash in this crate, so an op id can never
    /// collide with a Stoa address — which matters because both are 32 bytes and
    /// a moderation op names one of them.
    pub fn id(&self) -> OpId {
        let mut hasher = Sha256::new();
        hasher.update(OP_ID_PREFIX);
        hasher.update(self.canonical_bytes());
        OpId(hasher.finalize().into())
    }

    /// Sign this op, producing the envelope that crosses the wire.
    ///
    /// The key must be the one named in [`Op::author`]
    /// ([`identity::derive_stoa_key`]) — signing with any other key produces an
    /// op that [`SignedOp::verify`] rejects, because the **signature** will not
    /// verify under the key the op carries. Taking the key rather than reading
    /// one from ambient state is CLAUDE.md's "pass what it needs".
    pub fn sign(self, key: &SecretKey) -> SignedOp {
        let signature = sign_op_bytes(key, &self.canonical_bytes());
        SignedOp {
            op: self,
            signature,
        }
    }
}

impl SignedOp {
    /// Whether this op is authentically from the author it claims.
    ///
    /// Delegates to [`identity::verify_authored_op`], which establishes that the
    /// key this op carries signed the op's bytes. **That is the whole of
    /// authorship**: an op names its author by carrying the author's public key,
    /// so an op claiming a different author carries a different key and the
    /// signature then fails under it.
    ///
    /// This used to pass a fourth argument, `self.op.author.address()` — the
    /// claimed author, computed by calling `.address()` on the op's **own** key,
    /// which the callee then compared against `key.address()`. A value against
    /// itself. Issue #80 deleted the address and the parameter with it; nothing
    /// here re-implements the check, because there was never a check to
    /// re-implement on this path.
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
    ///
    /// Fallible because [`Op::encode`] is: an op the format has no encoding for
    /// has no wire form either, so it can be neither published nor stored as
    /// bytes a later read would refuse.
    pub fn to_bytes(&self) -> Result<Vec<u8>, OpError> {
        let mut out = self.op.encode()?;
        out.extend_from_slice(&self.signature.to_bytes());
        Ok(out)
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
            clock: None,
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

    /// A fixed asserted time, so an op id in this suite never depends on when
    /// the test ran.
    const A_TIME: u64 = 1_789_729_304_000;

    /// Every op kind, **in both encoding versions**.
    ///
    /// # Why this returns twice as many ops as there are kinds
    ///
    /// The clock fields are carried by every kind, so a property asserted "for
    /// every kind" that only ever saw version-1 ops would be asserting it for
    /// half the format. `one_of_each_kind_carries_both_versions` pins the count
    /// and the split, because this is the recorded hand-maintained-sweep shape:
    /// ten tests read this list, and a version omitted from it loses coverage in
    /// all ten silently.
    ///
    /// The version-2 half is derived by MAPPING over the version-1 half rather
    /// than by writing the kinds out again. A second hand-written list is a
    /// second list to keep in step, and the one that goes stale is always the
    /// one nothing forces you to look at.
    fn one_of_each_kind() -> Vec<Op> {
        let mut out = one_of_each_kind_without_a_clock();
        let with_clocks: Vec<Op> = out
            .iter()
            .enumerate()
            .map(|(i, op)| Op {
                clock: Some(OpClock {
                    // A DIFFERENT counter per op, so a test that round-trips
                    // this list cannot pass by writing one value everywhere.
                    counter: i as u64 + 1,
                    asserted_ms: A_TIME + i as u64,
                }),
                ..op.clone()
            })
            .collect();
        out.extend(with_clocks);
        out
    }

    /// Every op kind, carrying no clock. The version-1 population.
    fn one_of_each_kind_without_a_clock() -> Vec<Op> {
        let stoa = a_stoa();
        let author = a_key(2).public_key();
        vec![
            a_post(),
            Op {
                stoa,
                author: author.clone(),
                clock: None,
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
                clock: None,
                kind: OpKind::Revise {
                    target: an_id(9),
                    body: "Edited".to_string(),
                    attachments: vec!["cid".to_string()],
                },
            },
            Op {
                stoa,
                author: author.clone(),
                clock: None,
                kind: OpKind::Moderate {
                    target: an_id(10),
                    action: ModerationAction::Hide,
                },
            },
            Op {
                stoa,
                author: author.clone(),
                clock: None,
                kind: OpKind::Moderate {
                    target: an_id(10),
                    action: ModerationAction::Unhide,
                },
            },
            Op {
                stoa,
                author: author.clone(),
                clock: None,
                kind: OpKind::Vote {
                    target: an_id(11),
                    direction: VoteDirection::Up,
                },
            },
            Op {
                stoa,
                author,
                clock: None,
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
    //
    // **THESE ARE THE VERSION-1 OFFSETS**, and they stay that way. A version-2
    // op inserts sixteen bytes of clock between the author and the kind-specific
    // fields, so `KIND_FIELDS_AT` and everything derived from it is wrong for
    // one. The clock tests below use `CLOCK_AT` and `V2_KIND_FIELDS_AT` instead,
    // and no test may use a constant from one version against an op of the
    // other — which is why they are named apart rather than parameterised.

    /// Offset of the kind byte. Version is first, kind second, in both versions.
    const KIND_AT: usize = 1;
    /// Offset of the 32-byte Stoa address.
    const STOA_AT: usize = KIND_AT + 1;
    /// Offset of the 32-byte author key.
    const AUTHOR_AT: usize = STOA_AT + 32;
    /// Offset of the first kind-specific byte — where every kind's own fields
    /// begin, and where the common header ends.
    const KIND_FIELDS_AT: usize = AUTHOR_AT + 32;

    /// Offset of the 8-byte counter in a VERSION-2 op. Immediately after the
    /// author, which is where the common header used to end.
    const CLOCK_AT: usize = AUTHOR_AT + 32;
    /// Offset of the 8-byte asserted wall-clock in a version-2 op.
    const ASSERTED_AT: usize = CLOCK_AT + 8;
    /// Where a version-2 op's kind-specific fields begin.
    const V2_KIND_FIELDS_AT: usize = ASSERTED_AT + 8;

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
        //
        // The expected version is derived from the op's own `clock` rather than
        // hardcoded, because this list now carries BOTH versions. Hardcoding
        // either one would make this test assert the layout of half the format
        // — and asserting "it is one of the two" would pass for an op that
        // declared the wrong one of them.
        for op in one_of_each_kind() {
            let bytes = op.canonical_bytes();
            let expected = if op.clock.is_some() {
                VERSION_2
            } else {
                VERSION_1
            };
            assert_eq!(bytes[0], expected, "version must lead, and must be right");
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
            clock: None,
            kind: OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
        };
        let vote = Op {
            stoa,
            author,
            clock: None,
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
            clock: None,
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
                clock: None,
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
            clock: None,
            kind: OpKind::Moderate {
                target: an_id(1),
                action: ModerationAction::Hide,
            },
        };
        let unhide = Op {
            stoa,
            author,
            clock: None,
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
            clock: None,
            kind: OpKind::Vote {
                target: an_id(1),
                direction: VoteDirection::Up,
            },
        };
        let down = Op {
            stoa,
            author,
            clock: None,
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
            clock: None,
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
            clock: None,
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
        // Domain separation, for the same reason `identity.rs` prefixes its Stoa
        // address: an op id and a Stoa address are both 32 bytes, and a
        // moderation op names one of them.
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
            clock: None,
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
            // The author must be the key that signs, or `verify_authored_op`
            // rejects it — because the **signature** is checked under the key
            // the op carries, and no other key's signature verifies under it.
            //
            // This comment credited "the address check in `verify_authored_op`"
            // until issue #80. That check never refused anything on this path:
            // `SignedOp::verify` computed the claimed author by calling
            // `.address()` on the op's own key, so the comparison was a value
            // against itself. Measured: stubbing `verify_op_bytes` to return
            // `true` makes `an_op_signed_by_someone_else_is_rejected` fail,
            // which is what identifies the signature check as the mechanism.
            let key = a_key(2);
            let signed = op.sign(&key);
            assert!(signed.verify(), "a well-formed op must verify");
        }
    }

    #[test]
    fn an_op_signed_by_someone_else_is_rejected() {
        // THE forgery: a valid signature, untampered bytes, and still not from
        // the author it claims. The **signature check** is what catches it —
        // substituting the author substitutes the carried key, and the
        // attacker's signature does not verify under the victim's key.
        //
        // Stated precisely because the comment here credited an address
        // re-derivation until issue #80, and that was never the mechanism: the
        // only caller derived the claimed author from the op's own key, so the
        // comparison could not fail for any input. Measured — stubbing
        // `verify_op_bytes` to return `true` fails this test.
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
        // **The control, and this is the crate's REFERENCE forgery test** — the
        // one other modules' fixtures are written against, so a gap here
        // propagates. `!verify()` alone is a refusal guard that 64 fabricated
        // bytes satisfy identically: with the signature replaced by junk, this
        // test and all 65 in `op::tests` still passed. That version demonstrates
        // "a bad signature is refused", never "a valid signature under the wrong
        // key is refused" — and the latter is the attack the test is named for.
        //
        // It matters more after issue #80. The deleted address guard used to give
        // a forged op a second, independent reason to be refused; the signature
        // check is now the only one, so a test that cannot tell a forgery from
        // junk cannot see the mechanism it depends on.
        assert!(
            crate::identity::verify_authored_op(
                &attacker.public_key().to_bytes(),
                &signed.op.canonical_bytes(),
                &signed.signature.to_bytes()
            ),
            "the attacker's signature must be valid under the attacker's own key, \
             or this fixture is a junk signature rather than a forgery"
        );
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
            clock: None,
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
            let restored = SignedOp::from_bytes(&signed.to_bytes().unwrap()).unwrap();
            assert_eq!(restored, signed);
            assert!(
                restored.verify(),
                "verification must survive the round trip"
            );
        }
    }

    #[test]
    fn the_signed_preimage_is_a_prefix_of_the_wire_form() {
        // So a decoder never skips over the signature to find what it covers.
        let signed = a_post().sign(&a_key(2));
        let wire = signed.to_bytes().unwrap();
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
        let mut wire = signed.to_bytes().unwrap();
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
            clock: None,
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
            clock: None,
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
            clock: None,
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
            clock: None,
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

    #[test]
    fn a_vote_carries_no_score_field() {
        // §7.2 rule 1: "Relevance is a local projection, never an op. No score
        // is ever published." A published score is a claim no peer could
        // verify, and it would let a peer assert a ranking instead of deriving
        // one.
        //
        // Same technique as `an_op_carries_no_ordering_fields` above: the vote
        // encoding is fixed-length, so accounting for every byte is what makes
        // the omission *enforced* rather than merely documented. A score,
        // weight or rank field cannot be added without this failing.
        let op = Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            clock: None,
            kind: OpKind::Vote {
                target: an_id(3),
                direction: VoteDirection::Up,
            },
        };
        let expected = 1  // version
            + 1           // kind
            + 32          // stoa
            + 32          // author
            + 32          // target
            + 1; // direction
        assert_eq!(
            op.canonical_bytes().len(),
            expected,
            "the vote encoding has a field the layout does not account for"
        );
    }

    // ─── The Stoa metadata op ─────────────────────────────────────────────

    fn a_metadata_op() -> Op {
        Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            clock: None,
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
            clock: None,
            kind: OpKind::StoaMetadata {
                title: "ab".to_string(),
                description: "c".to_string(),
            },
        };
        let two = Op {
            stoa,
            author,
            clock: None,
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
            clock: None,
            kind: OpKind::Revise {
                target: OpId(target_bytes),
                body: "wxyz".to_string(),
                attachments: vec![],
            },
        };
        let metadata = Op {
            stoa,
            author,
            clock: None,
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
            clock: None,
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
                clock: None,
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
            clock: None,
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
            clock: None,
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
        // An empty DESCRIPTION is legitimate and must not be confused with
        // absent — a zero length prefix is a real encoding. An empty TITLE is
        // not admitted at all; `a_metadata_op_with_a_blank_title_is_refused_on_encode`
        // has it.
        for (title, description) in [
            ("a", ""),
            ("Ἀγορά", "ἡ ἀγορά — the marketplace"),
            ("🏛", ""),
            ("a", "a\0b"),
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

    // ─── A blank metadata title ──────────────────────────────────────────

    /// Every blank title the spec's scenarios name: the empty one, each of the
    /// thirty blank characters alone, and the mixed one.
    fn every_blank_title() -> Vec<String> {
        let mut titles = vec![String::new()];
        titles.extend(crate::stoa::BLANK_CHARACTERS.iter().map(|c| c.to_string()));
        titles.push("\u{0020}\u{200B}\u{3000}\u{FEFF}\u{0009}".to_string());
        titles
    }

    fn a_metadata_op_titled(title: &str, description: &str) -> Op {
        Op {
            kind: OpKind::StoaMetadata {
                title: title.to_string(),
                description: description.to_string(),
            },
            ..a_metadata_op()
        }
    }

    #[test]
    fn a_metadata_op_with_a_blank_title_is_refused_on_encode() {
        for title in every_blank_title() {
            let op = a_metadata_op_titled(&title, "a description");
            assert_eq!(op.encode(), Err(OpError::BlankTitle), "title {title:?}");
            // And so it has no wire form: the one every write out of this
            // process goes through.
            assert_eq!(
                op.sign(&a_key(2)).to_bytes(),
                Err(OpError::BlankTitle),
                "title {title:?}"
            );
        }
    }

    #[test]
    fn a_metadata_op_with_a_blank_title_is_refused_on_decode() {
        // The bytes come from `canonical_bytes`, which is the total layout and
        // so lays out a blank title exactly as a hostile peer would send it.
        for title in every_blank_title() {
            let bytes = a_metadata_op_titled(&title, "a description").canonical_bytes();
            assert_eq!(
                Op::decode(&bytes),
                Err(OpError::BlankTitle),
                "title {title:?}"
            );
        }
    }

    #[test]
    fn a_blank_metadata_title_is_its_own_refusal() {
        // The spec requires it be distinguishable from truncation, a lying length
        // prefix and an over-long field: all three are also shapes a zero-length
        // title prefix could be mistaken for. Compared as rendered text, since
        // that is what reaches a caller.
        let rendered = OpError::BlankTitle.to_string();
        for other in [
            OpError::Truncated,
            OpError::LengthMismatch,
            OpError::FieldTooLong(MAX_FIELD_LEN + 1),
            OpError::TrailingBytes,
            OpError::InvalidText,
        ] {
            assert_ne!(rendered, other.to_string());
        }
        assert!(
            rendered.contains("blank"),
            "the reason must name the title as blank"
        );
    }

    #[test]
    fn one_visible_letter_among_blank_characters_is_a_title_kept_as_given() {
        let title = "\u{0020}\u{200B}a\u{3000}\u{FEFF}";
        let op = a_metadata_op_titled(title, "");
        let bytes = op.encode().expect("one letter makes the title not blank");
        match Op::decode(&bytes).unwrap().kind {
            OpKind::StoaMetadata { title: decoded, .. } => assert_eq!(decoded, title),
            other => panic!("expected metadata, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_or_blank_description_is_valid() {
        // The rule is the title's alone. A description is not identity and not
        // a title, and an empty one is the ordinary case.
        for description in ["", "\u{0020}\u{200B}"] {
            let op = a_metadata_op_titled("Renamed", description);
            let bytes = op
                .encode()
                .expect("the description is not held to the rule");
            assert_eq!(Op::decode(&bytes).unwrap(), op);
        }
    }

    #[test]
    fn a_blank_title_does_not_make_other_kinds_unencodable() {
        // The guard is scoped to the metadata kind. A post with an empty body is
        // a valid post, and `an_empty_body_and_an_empty_attachment_list_round_trip`
        // pins it separately. This pins that the new guard did not reach it.
        let post = Op {
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: String::new(),
                attachments: vec![],
            },
            ..a_post()
        };
        assert_eq!(post.encode(), Ok(post.canonical_bytes()));
    }

    // Specified: "A metadata op with a blank title followed by trailing bytes
    // is refused as trailing bytes" names both the empty title and the mixed
    // blank title U+0020 U+200B. The structural fault is reported first, as
    // the genesis decoder does.
    #[test]
    fn a_blank_title_followed_by_trailing_bytes_reports_the_trailing_bytes() {
        for title in ["", "\u{0020}\u{200B}"] {
            let mut bytes = a_metadata_op_titled(title, "").canonical_bytes();
            bytes.push(0);
            assert_eq!(
                Op::decode(&bytes),
                Err(OpError::TrailingBytes),
                "title {title:?}"
            );
        }
    }

    #[test]
    fn a_blank_titled_op_can_still_be_built_signed_and_verified_in_memory() {
        // The property `stoa-metadata`'s resolver tests depend on, pinned here
        // where it lives. The format does not admit this op, and it is still an
        // authentic op by its author. If `verify` refused it, a resolver's own
        // blank check would never be what refused it, and deleting that check
        // would leave every resolver test green (design.md decision 2).
        let signed = a_metadata_op_titled("", "").sign(&a_key(2));
        assert!(signed.verify(), "authenticity is not admission");
        assert_eq!(signed.to_bytes(), Err(OpError::BlankTitle));
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
            clock: None,
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
            clock: None,
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

    // ─── The clock fields ─────────────────────────────────────────────────

    /// A post carrying a clock, for the tests below.
    fn a_post_with_clock(counter: u64, asserted_ms: u64) -> Op {
        Op {
            clock: Some(OpClock {
                counter,
                asserted_ms,
            }),
            ..a_post()
        }
    }

    #[test]
    fn one_of_each_kind_carries_both_versions() {
        // THE STALE-SWEEP GUARD. Ten tests iterate `one_of_each_kind`, and a
        // version missing from it loses coverage in all ten with nothing
        // failing. This asserts the split rather than leaving it to a comment.
        let all = one_of_each_kind();
        let bare = one_of_each_kind_without_a_clock();
        assert_eq!(
            all.len(),
            bare.len() * 2,
            "every kind must appear in both encoding versions"
        );
        assert_eq!(
            all.iter().filter(|op| op.clock.is_none()).count(),
            bare.len()
        );
        assert_eq!(
            all.iter().filter(|op| op.clock.is_some()).count(),
            bare.len()
        );
        // And the version-2 half covers every kind, not the same kind repeated.
        let kinds: Vec<u8> = all
            .iter()
            .filter(|op| op.clock.is_some())
            .map(|op| op.kind.to_byte())
            .collect();
        for kind in [
            OpKind::POST,
            OpKind::REVISE,
            OpKind::MODERATE,
            OpKind::VOTE,
            OpKind::STOA_METADATA,
        ] {
            assert!(
                kinds.contains(&kind),
                "kind {kind} is missing from the version-2 half"
            );
        }
    }

    #[test]
    fn the_version_byte_says_whether_the_clock_fields_are_there() {
        assert_eq!(a_post().canonical_bytes()[0], VERSION_1);
        assert_eq!(a_post_with_clock(1, 2).canonical_bytes()[0], VERSION_2);
    }

    #[test]
    fn the_clock_occupies_a_fixed_width_at_a_fixed_offset() {
        // No presence tag, and the fields are exactly where the layout says.
        // Read back as big-endian, matching every other integer in this format.
        let op = a_post_with_clock(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let bytes = op.canonical_bytes();

        assert_eq!(
            &bytes[CLOCK_AT..CLOCK_AT + 8],
            &0x0102_0304_0506_0708u64.to_be_bytes()
        );
        assert_eq!(
            &bytes[ASSERTED_AT..ASSERTED_AT + 8],
            &0x1112_1314_1516_1718u64.to_be_bytes()
        );

        // And the kind's own fields begin immediately after, with nothing in
        // between that could be a tag.
        let bare = a_post().canonical_bytes();
        assert_eq!(
            bytes.len(),
            bare.len() + 16,
            "the clock costs exactly sixteen bytes and no tag"
        );
        assert_eq!(
            &bytes[V2_KIND_FIELDS_AT..],
            &bare[KIND_FIELDS_AT..],
            "the kind's own fields are unchanged and merely displaced"
        );
    }

    #[test]
    fn the_counter_round_trips_at_every_boundary_value() {
        // Boundaries, not convenient middle values — a cast that truncates or a
        // width that is wrong shows up at the ends and nowhere else.
        let values = [0u64, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX];
        let mut seen: Vec<Vec<u8>> = Vec::new();
        for counter in values {
            let op = a_post_with_clock(counter, 7);
            let bytes = op.canonical_bytes();
            let back = Op::decode(&bytes).unwrap();
            assert_eq!(
                back.clock.unwrap().counter,
                counter,
                "counter {counter} must decode to itself"
            );
            assert_eq!(back, op);
            seen.push(bytes);
        }
        // None is confused with any other: distinct counters, distinct bytes.
        let before = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(before, seen.len(), "two counters encoded identically");
    }

    #[test]
    fn any_representable_instant_decodes_including_implausible_ones() {
        // Far future, far past, zero, maximum. Every one is accepted, and none
        // is refused on account of its value — refusing an op for a clock is the
        // censorship vector this format deliberately does not open.
        for asserted in [0u64, 1, 1_262_303_999_999, u64::MAX / 2, u64::MAX] {
            let op = a_post_with_clock(3, asserted);
            let back = Op::decode(&op.canonical_bytes())
                .unwrap_or_else(|e| panic!("asserted {asserted} was refused: {e}"));
            assert_eq!(back.clock.unwrap().asserted_ms, asserted);
        }
    }

    #[test]
    fn both_clock_fields_participate_in_the_op_id() {
        // A relay cannot raise a counter to promote an op, lower one to bury it,
        // or rewrite a wall-clock to change what a reader is shown — each alters
        // the preimage, so the id changes and the signature fails.
        let base = a_post_with_clock(5, 100);
        let other_counter = a_post_with_clock(6, 100);
        let other_time = a_post_with_clock(5, 101);

        assert_ne!(base.canonical_bytes(), other_counter.canonical_bytes());
        assert_ne!(
            base.id(),
            other_counter.id(),
            "the counter must change the id"
        );
        assert_ne!(base.canonical_bytes(), other_time.canonical_bytes());
        assert_ne!(
            base.id(),
            other_time.id(),
            "the wall-clock must change the id"
        );
    }

    #[test]
    fn changing_the_counter_breaks_the_signature() {
        // `a_key(2)` because `a_post`'s author IS `a_key(2)`. Signing with any
        // other key produces an op that fails verification for a reason that has
        // nothing to do with the tamper, which would make the assertion below
        // pass for the wrong reason.
        let key = a_key(2);
        let signed = a_post_with_clock(5, 100).sign(&key);
        assert!(
            signed.verify(),
            "the fixture must verify before it is broken"
        );

        let tampered = SignedOp {
            op: Op {
                clock: Some(OpClock {
                    counter: 6,
                    asserted_ms: 100,
                }),
                ..signed.op.clone()
            },
            signature: signed.signature.clone(),
        };
        assert!(!tampered.verify(), "a raised counter must not verify");
        // The original still verifies, so the failure is the tamper rather than
        // a broken fixture.
        assert!(signed.verify());
    }

    #[test]
    fn changing_the_wall_clock_breaks_the_signature() {
        let key = a_key(2);
        let signed = a_post_with_clock(5, 100).sign(&key);
        assert!(
            signed.verify(),
            "the fixture must verify before it is broken"
        );

        let tampered = SignedOp {
            op: Op {
                clock: Some(OpClock {
                    counter: 5,
                    asserted_ms: u64::MAX,
                }),
                ..signed.op.clone()
            },
            signature: signed.signature.clone(),
        };
        assert!(!tampered.verify(), "a rewritten wall-clock must not verify");
        assert!(signed.verify());
    }

    #[test]
    fn a_downgrade_to_the_earlier_version_does_not_verify() {
        // The strip attack, stated as the relay would attempt it: rewrite the
        // version byte so the op reads as one carrying no counter, and forward
        // the original signature. The version is inside the preimage, so it
        // fails exactly as rewriting a counter does.
        let key = a_key(2);
        let signed = a_post_with_clock(5, 100).sign(&key);
        assert!(
            signed.verify(),
            "the fixture must verify before it is broken"
        );

        let downgraded = SignedOp {
            op: Op {
                clock: None,
                ..signed.op.clone()
            },
            signature: signed.signature.clone(),
        };
        assert_eq!(
            downgraded.op.canonical_bytes()[0],
            VERSION_1,
            "the fixture must actually be a downgrade"
        );
        assert!(!downgraded.verify(), "a stripped clock must not verify");
        assert!(signed.verify(), "the original still verifies");
    }

    #[test]
    fn an_op_of_this_version_carrying_no_counter_is_refused() {
        // The clock bytes removed while the version byte still says version 2.
        // It must fail to decode, and it must NOT be read as an op of the
        // earlier version — the version says what it is, and the decoder does
        // not second-guess it.
        let bytes = a_post_with_clock(5, 100).canonical_bytes();
        let mut stripped = bytes[..CLOCK_AT].to_vec();
        stripped.extend_from_slice(&bytes[V2_KIND_FIELDS_AT..]);

        let result = Op::decode(&stripped);
        assert!(
            result.is_err(),
            "a version-2 op with no clock bytes must not decode, got {result:?}"
        );
    }

    #[test]
    fn an_unrecognised_version_is_named_rather_than_reported_as_malformed() {
        let mut bytes = a_post_with_clock(5, 100).canonical_bytes();
        bytes[0] = 99;
        assert_eq!(Op::decode(&bytes), Err(OpError::UnknownVersion(99)));
    }

    #[test]
    fn both_versions_decode_and_report_which_they_are() {
        // And the version-1 op reports carrying NO counter, rather than
        // reporting a counter of zero — which is the distinction the whole
        // migration rests on.
        let bare = Op::decode(&a_post().canonical_bytes()).unwrap();
        assert_eq!(bare.clock, None, "absence, not a counter of zero");

        let with = Op::decode(&a_post_with_clock(0, 0).canonical_bytes()).unwrap();
        assert_eq!(
            with.clock,
            Some(OpClock {
                counter: 0,
                asserted_ms: 0
            }),
            "a counter of zero is a counter, not an absence"
        );
        assert_ne!(bare.clock, with.clock);
    }

    #[test]
    fn an_op_of_the_earlier_version_keeps_the_id_it_always_had() {
        // THE MIGRATION GUARANTEE. The literal is the one
        // `the_op_id_constant_is_pinned_to_a_known_answer` carries, and it was
        // derived INDEPENDENTLY of this code — reconstructed from RFC 8032 point
        // arithmetic and the documented layout — which is what makes it able to
        // detect a present error rather than only a future change. A test
        // comparing two values this build computed would agree with any bug both
        // of them shared.
        //
        // The fixture is that test's, byte for byte, because the literal is only
        // meaningful against the op it was derived for. The two tests assert
        // different things over one value: that one pins the id derivation, and
        // this one pins that ADDING THE CLOCK FIELDS TO THE FORMAT did not move
        // it for an op that carries none.
        let op = Op {
            stoa: crate::identity::stoa_address(b"a genesis record"),
            author: a_key(7).public_key(),
            clock: None,
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
            "adding the clock fields must not have changed a version-1 op's id"
        );

        // And the same op WITH a clock has a different id, so the test above is
        // not passing because the field is being ignored.
        let with_clock = Op {
            clock: Some(OpClock {
                counter: 1,
                asserted_ms: 1,
            }),
            ..op.clone()
        };
        assert_ne!(op.id(), with_clock.id());
    }

    #[test]
    fn decoding_reads_no_clock_and_does_not_depend_on_when_it_happened() {
        // `Op::decode` is a pure function of its bytes. Asserted by decoding
        // the same bytes repeatedly and comparing — which a decoder that
        // compared an asserted time against the system clock would still pass,
        // so the STRONGER statement is structural: `decode` takes only `&[u8]`,
        // and there is no clock argument for it to read.
        //
        // What this test adds on top of the signature is that no HIDDEN clock
        // read happens either — no `SystemTime::now()` inside.
        let bytes = a_post_with_clock(5, u64::MAX).canonical_bytes();
        let first = Op::decode(&bytes).unwrap();
        let second = Op::decode(&bytes).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.clock.unwrap().asserted_ms, u64::MAX);
    }

    #[test]
    fn a_version_2_op_carries_the_two_admitted_values_and_no_others() {
        // The encoding's length is fully accounted for: a message id, a sequence
        // number or a sender identifier could not be added without this
        // changing. Stated as arithmetic over the named widths rather than as a
        // magic number, so the assertion says WHICH fields it accounts for.
        let op = a_post_with_clock(5, 100);
        let bytes = op.canonical_bytes();

        let version = 1;
        let kind = 1;
        let stoa = 32;
        let author = 32;
        let counter = 8;
        let asserted = 8;
        // The `Post`'s own fields: two option tags, a body length prefix, the
        // body, and an empty attachment list's count.
        let post_fields = OPTION_TAG + OPTION_TAG + LEN_PREFIX + "First".len() + LEN_PREFIX;

        assert_eq!(
            bytes.len(),
            version + kind + stoa + author + counter + asserted + post_fields,
            "the encoding's length must be exactly its fields"
        );
    }

    #[test]
    fn two_peers_encoding_one_op_agree_on_its_bytes() {
        // Every value reaching the encoding travels in the op, so nothing either
        // peer knows about its own history can change an op's identity. Asserted
        // by encoding one op through two independently-constructed values.
        let a = a_post_with_clock(5, 100);
        let b = Op {
            stoa: a_stoa(),
            author: a_key(2).public_key(),
            clock: Some(OpClock {
                counter: 5,
                asserted_ms: 100,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "First".to_string(),
                attachments: vec![],
            },
        };
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.id(), b.id());
    }
}
