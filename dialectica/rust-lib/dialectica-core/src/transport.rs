//! How a Stoa's signed ops move between peers, and what must survive to be stored.
//!
//! # Why this file exists
//!
//! Every capability before this one assumed ops arrive from somewhere. `op.rs`
//! says what an op is, `log.rs` stores it, `arrival.rs` orders it — and nothing
//! said how one reaches another peer. This is that edge, and it is also the only
//! place attacker-controlled bytes cross into the peer.
//!
//! # The two halves, and why one of them cannot be here
//!
//! `modules().delivery_module.channel_send(..)` cannot appear in this crate: it
//! calls `lp_*` symbols undefined in a test binary (PLAN.md §2.3). So this module
//! holds everything on either side of that call — deriving a channel's identity,
//! deciding what an inbound payload is, deciding what a publish did — and the
//! adapter holds only the call. `wire.rs`'s delivery bridge is the Phase 0
//! precedent for the same split.
//!
//! The consequence worth stating rather than discovering: **a property that holds
//! only in the adapter is untested by definition**, because `cargo test` does not
//! compile that file.
//!
//! # The channel identity must be identical on every peer, and its failure is silent
//!
//! [`ChannelIdentity::of`] is a pure function of the Stoa address, and the whole
//! module is shaped around keeping it one. PLAN.md §4.3 states the failure mode:
//! two peers computing different channel ids "does not produce an error: it
//! produces two Stoas that cannot see each other, silently and permanently".
//!
//! Every other refusal in this module reports itself. This one cannot, so it is
//! held by construction — there is no local input to the derivation, and no way
//! to build a [`ChannelIdentity`] except by deriving one.
//!
//! # What arrives alongside an op, and what must never be read from it
//!
//! `channelMessageReceived(channelId, senderId, payload, timestamp)` supplies two
//! values that look like facts about the message and are not:
//!
//! - **`senderId` is not an author identity.** §4.1: "`senderId` is not an author
//!   identity, and the plan should not treat it as one." It is an
//!   application-chosen string the transport neither issues nor verifies, and
//!   nothing prevents two participants claiming one. Authorship comes from the
//!   op's signature and the author key inside it. So it is accepted, used for
//!   nothing, and stored nowhere.
//! - **`timestamp` is the receiving peer's own clock.** Measured, not inferred:
//!   `delivery_module_plugin.cpp` assigns it `currentTimestampNs()`, a
//!   `CLOCK_REALTIME` read taken when the callback fired. Two peers record two
//!   values for one message. `op-ordering`'s `design.md` ranks it as a *worse*
//!   ordering candidate than raw arrival sequence, because it is per-peer while
//!   appearing shared.
//!
//! Both are in [`InboundMessage`] because the event carries them and a boundary
//! that dropped a field from its signature would be hiding it rather than
//! refusing it. Neither reaches anything recorded.
//!
//! # Why every arrival is `Arrival::unordered()`
//!
//! No Lamport timestamp, no message id and no causal history reach this layer.
//! **That was established exhaustively and should not be re-litigated here**: the
//! `op-ordering` change's `design.md` records the four routes it ruled out, and
//! the loss point is one layer below the delivery module — the Reliable Channel
//! API's `MessageReceivedEvent` carries exactly one field, the reassembled
//! payload, so `channelMessageReceived` cannot forward what it was never given.
//!
//! `Arrival::unordered()`'s own doc comment names this boundary as its caller:
//! "**What the boundary constructs today** [...] Named rather than reached by
//! passing two `None`s, so that grepping for it finds every place the contract's
//! gap is being absorbed."

use crate::arrival::Arrival;
use crate::identity::Address;
use crate::log::{Appended, OpLog, OpLogError};
use crate::op::{OpError, OpId, SignedOp};
use std::collections::HashMap;

/// The largest payload a single message may carry, in bytes.
///
/// §4.4's **150 KiB** cap: "a network-wide gossipsub validation limit, not
/// unilaterally raisable". A payload over it is refused *before* it is decoded,
/// because a decoder handed 4 MiB does work proportional to the input before
/// refusing, and the input size is an attacker's free parameter here.
///
/// # Deliberately a SECOND constant, not a re-export of `op.rs`'s field cap
///
/// [`crate::op`]'s `MAX_FIELD_LEN` shares this value today and is a **different
/// bound**: it caps one variable-length field inside an op, and its own docs are
/// explicit that it "does NOT bound the total size of a decoded op, and must not
/// be read as doing so" — several capped fields sum well past 150 KiB. That
/// comment names the transport boundary as the right home for the total bound,
/// and this is it.
///
/// Aliasing the two would make a later change to either silently change the
/// other, which is the drift a shared constant looks like it prevents and does
/// not.
pub const MAX_MESSAGE_BYTES: usize = 150 * 1024;

/// Domain prefix for a content topic. §4.1's format, verbatim.
///
/// The `/dialectica/1/` head is load-bearing and not a naming preference: §4.2
/// establishes that autosharding hashes only `application` + `version`, so this
/// prefix is what places every dialectica topic on one shard. A different prefix
/// would be a routing change disguised as a rename.
const TOPIC_PREFIX: &str = "/dialectica/1/s/";
/// Suffix of a content topic, completing §4.1's `/dialectica/1/s/<hex>/proto`.
const TOPIC_SUFFIX: &str = "/proto";
/// Domain prefix for a channel id.
///
/// A different discriminant in the same position as the topic's `s`, because the
/// two are not one namespace: a content topic is disclosed to filtering, storage
/// and forwarding peers, and a channel id is an application-chosen rendezvous
/// string. One value for both would make a change to either a change to both.
///
/// Structured rather than the bare hex, so that §4.5's deferred `(stoa, thread)`
/// split has somewhere to put a thread segment — and so that two applications
/// sharing one node cannot collide on "some 32-byte hex string".
const CHANNEL_PREFIX: &str = "/dialectica/1/c/";

/// The two strings that name a Stoa's channel: the rendezvous, and what the
/// network filters on.
///
/// # One type rather than two functions, and the fields are private
///
/// The alternative was `channel_id(&Address)` and `content_topic(&Address)` as
/// free functions. Two functions can each be called with a *different* address;
/// one constructor taking one address cannot. That is the spec requirement
/// "Channel identity is a pure function of the Stoa address" made structural.
///
/// The fields are private because the only route to a `ChannelIdentity` must be
/// [`ChannelIdentity::of`]. A public `String` field would let a caller assemble
/// one by hand — with an epoch in it, or with another peer's value — and hand it
/// to a send. There would then be no error: just a channel nobody else is in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChannelIdentity {
    channel_id: String,
    content_topic: String,
    /// The Stoa this identity was derived for.
    ///
    /// Kept so [`OpenChannels`] can record the pair without being handed it
    /// separately — a caller supplying both could supply a mismatched pair, and
    /// every later Stoa comparison would then be against a lie.
    stoa: Address,
}

impl ChannelIdentity {
    /// Derive a Stoa's channel identity. A pure function of the address.
    ///
    /// **Nothing else participates**: no session counter, no epoch, no local
    /// sequence number, no device identifier, no clock reading, no count of prior
    /// opens, and not this peer's own identity. There is no parameter for any of
    /// them, which is what makes the property structural rather than checked.
    ///
    /// A deterministic epoch is not a variant of this and must not be
    /// reintroduced: PLAN.md §4.3 costs it out as "a rendezvous problem at each
    /// boundary — a value that changes must change for everyone at once", carried
    /// forever to route around a defect that belongs upstream.
    pub fn of(stoa: &Address) -> Self {
        let hex = stoa.to_hex();
        ChannelIdentity {
            channel_id: format!("{CHANNEL_PREFIX}{hex}"),
            content_topic: format!("{TOPIC_PREFIX}{hex}{TOPIC_SUFFIX}"),
            stoa: *stoa,
        }
    }

    /// The conversation every peer in the Stoa must join.
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    /// What the underlying network filters on.
    pub fn content_topic(&self) -> &str {
        &self.content_topic
    }

    /// The Stoa this identity names.
    pub fn stoa(&self) -> &Address {
        &self.stoa
    }
}

/// The channels this peer currently has open.
///
/// # A map from channel id to Stoa address, and the shape IS the Stoa check
///
/// The receive boundary asks two questions of this: *is this channel open?* and
/// *which Stoa is it for?* A `HashSet<String>` answers the first and not the
/// second, which would leave the Stoa-mismatch refusal with nothing to compare
/// against — and that refusal is the whole answer to an op copied unchanged onto
/// another Stoa's channel, where it is authentic, verifies, and is a post its
/// author never addressed there.
///
/// Keyed by channel id and not by address, because the inbound event supplies a
/// channel id and nothing else. Keying the other way would mean re-deriving every
/// open channel's identity per message to find the match — a scan where a lookup
/// will do, and a scan is where "compares only a prefix" lives.
///
/// # In memory, and nothing is lost by that
///
/// Every channel is closed on shutdown, so there is nothing to persist. Which
/// Stoas a peer *holds* is the Stoa-lifecycle capability's, and re-opening on
/// start is that capability's job rather than this one's.
#[derive(Debug, Default)]
pub struct OpenChannels {
    by_id: HashMap<String, Address>,
}

impl OpenChannels {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a channel as open.
    ///
    /// Takes a [`ChannelIdentity`] rather than `(channel_id, stoa)` so the map's
    /// invariant holds by construction: a channel id maps to the Stoa it was
    /// *derived for*, and there is no way to insert a pair the derivation did not
    /// produce.
    ///
    /// Idempotent. Re-opening a channel already open is not an error, and the
    /// recorded Stoa cannot change because the identity determines both.
    pub fn open(&mut self, identity: &ChannelIdentity) {
        self.by_id
            .insert(identity.channel_id.clone(), identity.stoa);
    }

    /// Forget a channel.
    ///
    /// Returns whether one was open. **Closing is best-effort at the transport
    /// and this records only the local half** — the content topic behind a
    /// channel is released by reference count across every channel on it, and
    /// those failures are not surfaced to a caller. So a peer cannot observe that
    /// a release reached the network and must not claim it did.
    pub fn close(&mut self, identity: &ChannelIdentity) -> bool {
        self.by_id.remove(&identity.channel_id).is_some()
    }

    /// Forget every channel, returning each identity's channel id.
    ///
    /// Shutdown closes every open channel, and a caller needs the ids in order to
    /// call the transport for each. Returned sorted, so that a caller's sequence
    /// of `channelClose` calls does not depend on hash iteration order — which is
    /// not a correctness property of the close, but makes a test of it assertable
    /// against a fixed expectation rather than a sorted copy.
    pub fn close_all(&mut self) -> Vec<String> {
        let mut ids: Vec<String> = self.by_id.keys().cloned().collect();
        ids.sort();
        self.by_id.clear();
        ids
    }

    /// The Stoa a channel was opened for, or `None` if it is not open.
    pub fn stoa_of(&self, channel_id: &str) -> Option<&Address> {
        self.by_id.get(channel_id)
    }

    /// Whether this peer has a channel open under this id.
    pub fn is_open(&self, channel_id: &str) -> bool {
        self.by_id.contains_key(channel_id)
    }

    /// How many channels are open.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// One inbound message, exactly as `channelMessageReceived` delivers it.
///
/// **Every field the event carries, including the two that must not be believed.**
/// A struct that dropped `sender_id` and `timestamp` would be hiding them rather
/// than refusing them, and a later reader would not know they had been considered.
/// Neither reaches anything recorded; see this module's documentation for the
/// measurements behind that.
///
/// Borrows rather than owns, because nothing here needs to keep the payload past
/// the decision — and because an owned `Vec<u8>` per message would be an
/// allocation on the one path whose input size an attacker chooses.
#[derive(Clone, Copy, Debug)]
pub struct InboundMessage<'a> {
    /// Which channel it arrived on. Attacker-controlled: any peer may compute a
    /// Stoa's channel identity and send on it.
    pub channel_id: &'a str,
    /// The transport's sender identifier. **Never an identity.** Accepted,
    /// unused, unstored.
    pub sender_id: &'a str,
    /// One signed op's wire form, and nothing else.
    pub payload: &'a [u8],
    /// The receiving peer's own clock at the moment its handler ran. Orders
    /// nothing.
    pub timestamp: i64,
}

/// Why an inbound payload was refused.
///
/// # Named `InboundRefusal` and not `Refusal`, because the crate already has one
///
/// `authoring::Refusal` exists, with a disjoint variant set and no relation to
/// this. Two public `Refusal`s in one crate is a collision that gets resolved by
/// whoever needs both first, under time pressure, at the call site rather than in
/// the types — and the call site that needs both is the one this change's seam is
/// for: a handler reporting a publish that then hands off to delivery. `stoa.rs`
/// and `op.rs` each name their error for what it is (`GenesisError`, `OpError`);
/// this follows them. Every variant here is about a payload that **arrived**, so
/// the qualifier is the type's actual subject rather than a disambiguating suffix.
///
/// Renamed on review (`findings/architecture.md` entry 2) while it was still free
/// to do: nothing outside this module names the type yet, so the diff is contained
/// to one file. It stops being free the moment the wiring lands.
///
/// # Each variant is a different cause with a different response
///
/// The spec requires the five refusals be "reported distinguishably from the
/// others", and the reason is what a reader does next: a build that is behind, a
/// corrupt or hostile payload, a forgery, a misdirected or replayed op, and a peer
/// sending more than the network permits are five problems, and a boundary that
/// said only "invalid" would send someone looking in the wrong place.
///
/// [`InboundRefusal::Undecodable`] **carries** the decoder's own error rather than
/// flattening it to a string: `op-format` already distinguishes eleven ways a byte
/// string is not an op, and discarding that one layer after the code that produced
/// it is the same mistake at a smaller scale.
#[derive(Debug, PartialEq, Eq)]
pub enum InboundRefusal {
    /// A payload on a channel identifier this peer has no open channel for.
    ///
    /// Not an error about the op: this peer was not listening, so nothing about
    /// the bytes has been judged.
    UnknownChannel,
    /// Larger than a single message may carry. Reported before any decode,
    /// because "no peer could legitimately have sent this" is a different fact
    /// from "this op is corrupt".
    TooLong { bytes: usize, limit: usize },
    /// The op decoder did not accept the payload.
    Undecodable(OpError),
    /// The signature does not verify, or the presented key does not bind to the
    /// author the op claims.
    ///
    /// One variant for both, because [`SignedOp::verify`] answers them together —
    /// it re-derives the author address from the presented key and compares, so a
    /// caller cannot be told which half failed without splitting a function whose
    /// whole purpose is that the pair is checked as one. The spec lists them in
    /// one bullet for the same reason.
    FailsVerification,
    /// The op names a Stoa other than the one whose channel it arrived on.
    ///
    /// An op's Stoa is inside its signature, so it cannot be *rewritten* to name
    /// another Stoa — but it can be **copied unchanged** onto another's channel,
    /// where it is authentic and verifies. Only this comparison refuses it.
    StoaMismatch {
        named: Address,
        channel_is_for: Address,
    },
    /// The op log could not be written.
    ///
    /// Not a judgement about the payload: the bytes were a well-formed, authentic
    /// op addressed to this channel's Stoa, and the store failed. Distinct from
    /// every other variant because it is the only one where retrying could work.
    Storage(OpLogError),
}

impl std::fmt::Display for InboundRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InboundRefusal::UnknownChannel => {
                write!(f, "no channel is open under that channel identifier")
            }
            InboundRefusal::TooLong { bytes, limit } => write!(
                f,
                "the payload is {bytes} bytes, over the {limit} a single message may carry"
            ),
            InboundRefusal::Undecodable(e) => write!(f, "the payload is not an op: {e}"),
            InboundRefusal::FailsVerification => write!(
                f,
                "the op's signature does not verify under the author it claims"
            ),
            InboundRefusal::StoaMismatch {
                named,
                channel_is_for,
            } => write!(
                f,
                "the op names Stoa {} but arrived on the channel for {}",
                named.to_hex(),
                channel_is_for.to_hex()
            ),
            InboundRefusal::Storage(e) => write!(f, "the op could not be stored: {e}"),
        }
    }
}

/// What admitting an op did.
///
/// Carries the op id because a caller acts on it — rebuilding a view, or naming
/// what arrived — and deriving it again from the op would be asking the same
/// question twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Admitted {
    pub id: OpId,
    /// Whether the log already held it. An op the peer already holds arriving
    /// again leaves it holding one op, and the arrival recorded first is the one
    /// kept — that is `op-log`'s rule, and this reports which happened rather
    /// than restating it.
    pub appended: Appended,
}

/// Decide what an inbound payload is, and store it if it is an op for this
/// channel's Stoa.
///
/// # Every check runs before anything is written
///
/// The order is the spec's and is not an implementation detail:
///
/// 1. **Is a channel open under this id?** If not, this peer was not listening
///    and nothing about the bytes is judged.
/// 2. **Is the payload within the message limit?** Before the decode, so a 4 MiB
///    payload costs a length comparison rather than a parse.
/// 3. **Does it decode?** The **whole** payload, through `op-format`'s decoder —
///    which refuses a wire form followed by an extra byte, so there is no region
///    of a payload the decoder never sees and no framing that would make trailing
///    bytes legitimate.
/// 4. **Does it verify?** Signature and author binding together.
/// 5. **Does it name this channel's Stoa?**
///
/// Only then is anything appended. A refusal therefore cannot partially apply:
/// there is no intermediate mutation to roll back, because the single write is
/// the last statement. That is CLAUDE.md's "keep handler bodies free of partial
/// mutation" rather than a rollback path.
///
/// # This decides nothing about the op beyond admitting it
///
/// Storing an op is not a statement that it is permitted, that its author may
/// moderate, that a revision it declares takes effect, or that the Stoa's policy
/// admits its author. Authority is decided on read, against state a peer may not
/// have held when the op arrived — `moderation.rs` is where that happens, and
/// `log.rs`'s module docs carry the argument for why a boundary that decided both
/// would decide authority once, from whatever the peer knew at one instant.
///
/// The converse holds too and is the half that reads as a contradiction: this
/// boundary does **not** admit an op that fails authenticity on the grounds that
/// authority is checked later. The two answer different questions against
/// different inputs, and neither substitutes for the other.
///
/// # No panic is reachable from any input
///
/// A panic here aborts the module process (`PHASE0-FINDINGS` §3): the caller waits
/// out a 20-second timeout and every later call reports `MODULE_NOT_LOADED`. The
/// bytes are chosen by whoever sent them, so a reachable panic is a remotely
/// triggerable denial of service. There is no indexing, no slicing and no
/// arithmetic on this path — the size check is a `>` comparison, and the decode is
/// `op-format`'s, whose own suite covers every prefix of a valid op.
pub fn receive<L: OpLog>(
    message: InboundMessage<'_>,
    channels: &OpenChannels,
    log: &mut L,
) -> Result<Admitted, InboundRefusal> {
    let channel_stoa = *channels
        .stoa_of(message.channel_id)
        .ok_or(InboundRefusal::UnknownChannel)?;

    // BEFORE the decode. The spec requires it, and the reason is that this is the
    // one bound whose input size an attacker chooses freely.
    if message.payload.len() > MAX_MESSAGE_BYTES {
        return Err(InboundRefusal::TooLong {
            bytes: message.payload.len(),
            limit: MAX_MESSAGE_BYTES,
        });
    }

    // The WHOLE payload, so no prefix is decoded in isolation. `Op::decode`'s
    // trailing-bytes check is what makes that true of a valid op followed by junk.
    let signed = SignedOp::from_bytes(message.payload).map_err(InboundRefusal::Undecodable)?;

    if !signed.verify() {
        return Err(InboundRefusal::FailsVerification);
    }

    // The comparison that refuses an authentic op copied onto another Stoa's
    // channel. Whole-address equality, never a prefix: `Address` is `PartialEq`
    // over `[u8; 32]`, so there is no shorter comparison available to write.
    if signed.op.stoa != channel_stoa {
        return Err(InboundRefusal::StoaMismatch {
            named: signed.op.stoa,
            channel_is_for: channel_stoa,
        });
    }

    let id = signed.op.id();
    // `Arrival::unordered()` and not `from_parts(None, None)`: the named
    // constructor is a statement that the transport supplied nothing, and
    // grepping for it finds every place that gap is absorbed. `message.timestamp`
    // and `message.sender_id` reach nothing here, by design.
    let appended = log
        .append(signed, Arrival::unordered())
        .map_err(InboundRefusal::Storage)?;
    Ok(Admitted { id, appended })
}

/// Why a publish did not fully succeed.
#[derive(Debug, PartialEq, Eq)]
pub enum PublishError {
    /// The op could not be stored, so it was not published either.
    ///
    /// The one failure that loses the op, and the reason storing comes first:
    /// there is nothing on the network claiming to exist.
    NotStored(OpLogError),
    /// This peer has no open channel for the op's Stoa.
    ///
    /// **The op is still stored** — the author authored it — and no channel is
    /// opened as a side effect. When a channel is opened, and for which Stoas, is
    /// the Stoa-lifecycle capability's; a publish is not how a peer comes to be in
    /// a Stoa.
    NoChannel { stoa: Address, id: OpId },
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::NotStored(e) => {
                write!(
                    f,
                    "the op was not published because it could not be stored: {e}"
                )
            }
            PublishError::NoChannel { stoa, .. } => write!(
                f,
                "the op is stored but was not published: no channel is open for Stoa {}",
                stoa.to_hex()
            ),
        }
    }
}

/// An op stored locally and ready to hand to the transport.
///
/// # Two fields, because a caller needs both and can derive neither
///
/// The `payload` is what goes to `channelSend` and the `channel_id` is which
/// channel it goes on. A caller that was handed only the op would have to
/// re-derive the identity — a second derivation site, which is the thing
/// [`ChannelIdentity`] exists to prevent.
///
/// **The payload is the op's wire form as stored, with nothing around it.** No
/// envelope, header or framing of this capability's own: an envelope would be
/// attacker-controlled bytes *outside* the signature, which is the one shape this
/// system has no defence for, since anything a peer reads from outside the signed
/// preimage is something any relay may rewrite. Everything a receiving peer needs
/// — the Stoa, the author key, the kind, the target — is already inside the signed
/// bytes.
///
/// # `#[must_use]`, because dropping one is indistinguishable from sending one
///
/// The tracker this seam is for keys a delivery outcome back to an op by this
/// struct's `id` and `channel_id`, which means it must observe **every**
/// `Publishable` that was sent. A `Publishable` dropped on a path that forgot to
/// send is, to that tracker, identical to one sent and never propagated — and
/// telling those two apart is precisely one of the three things this change
/// records as owed. So the attribute is not a lint preference: it is the only
/// compiler-visible signal separating the two states.
///
/// Dropping one is still a legitimate operation — `a_send_failure_does_not_lose_the_op`
/// drops one deliberately, to witness that the log survives — so the attribute is
/// paired with an explicit `let _ =` at that one site rather than omitted. An
/// explicit discard states the intent; a silent one cannot be told from a mistake.
/// This is the first `#[must_use]` in the crate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Publishable {
    pub id: OpId,
    pub channel_id: String,
    pub payload: Vec<u8>,
}

/// Store a locally-authored op, then say what to send and where.
///
/// # The ordering is the requirement, not an implementation detail
///
/// Append first; hand the bytes out second. Publishing first loses the op in
/// every case where the store fails after the send succeeded: the op is on the
/// network, other peers hold it, and its author does not — so the author's own
/// view is missing a post it published, and the peer has no record with which to
/// notice.
///
/// The consequence a caller must honour: **a transport failure on the returned
/// [`Publishable`] must not remove the op from the log.** It is signed and
/// stored; whether it reached the network is a separate fact, and discarding a
/// stored op on a send failure would make an op's existence depend on network
/// conditions at one instant. This function returns rather than sending, so there
/// is no path here that could undo the append — which is that rule made
/// structural.
///
/// # A peer's own op is recorded exactly as a received one
///
/// `Arrival::unordered()`, the same value the receive path records. A peer that
/// gave its own ops a position other peers cannot reproduce would render its own
/// threads differently from everyone else's, with no error anywhere.
///
/// # It does not open a channel
///
/// Publishing on a Stoa with no open channel is [`PublishError::NoChannel`], and
/// the op is stored anyway. There is no branch here that opens one: the parameter
/// is an `&OpenChannels`, so opening is not reachable from this function at all.
pub fn publish<L: OpLog>(
    op: SignedOp,
    channels: &OpenChannels,
    log: &mut L,
) -> Result<Publishable, PublishError> {
    let id = op.op.id();
    let stoa = op.op.stoa;
    // The wire form is taken from the op BEFORE it is moved into the log, and it
    // is the same bytes either way — `to_bytes` is a pure function of the op. The
    // spec's "the payload handed to the transport is the op's wire form as
    // stored" is therefore not a property that could drift.
    let payload = op.to_bytes();

    // FIRST. A store failure means nothing goes out.
    log.append(op, Arrival::unordered())
        .map_err(PublishError::NotStored)?;

    // The channel is looked up AFTER the append, so that a Stoa with no channel
    // still stores the op. Checking first and returning early would lose it.
    let identity = ChannelIdentity::of(&stoa);
    if !channels.is_open(identity.channel_id()) {
        return Err(PublishError::NoChannel { stoa, id });
    }

    Ok(Publishable {
        id,
        channel_id: identity.channel_id,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{sign_op_bytes, SecretKey};
    use crate::log::MemoryOpLog;
    use crate::op::{ModerationAction, Op, OpKind};
    use crate::stoa::{Genesis, Policy};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    /// A Stoa address from a genesis record, so the fixture's addresses are the
    /// ones a real peer would hold rather than synthetic bytes.
    fn a_stoa(title: &str) -> Address {
        Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: title.to_string(),
        }
        .address()
        .expect("a short title is well under the title cap")
    }

    fn a_post_in(stoa: Address, body: &str) -> Op {
        Op {
            stoa,
            author: a_key(2).public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
    }

    fn signed_post_in(stoa: Address, body: &str) -> SignedOp {
        a_post_in(stoa, body).sign(&a_key(2))
    }

    /// A peer with one channel open, for this Stoa.
    fn peer_in(stoa: Address) -> (OpenChannels, MemoryOpLog, ChannelIdentity) {
        let identity = ChannelIdentity::of(&stoa);
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        (channels, MemoryOpLog::new(), identity)
    }

    /// A [`MemoryOpLog`] whose `append` refuses.
    ///
    /// **The only route by which a test reaches [`InboundRefusal::Storage`] and
    /// [`PublishError::NotStored`].** Both variants are constructed by the
    /// implementation and neither was reachable from a test before this existed,
    /// because `MemoryOpLog::append` cannot fail — so the assertions naming them
    /// were checking a `Display` impl and a variant nothing produced, not the
    /// spec's requirement that a store failure be distinguishable from the other
    /// causes.
    ///
    /// Every read delegates to the real log rather than failing too. A fake that
    /// failed on read as well could not witness "the op is NOT in the log", which
    /// is the half of the store-failure contract that distinguishes it from
    /// [`PublishError::NoChannel`] — where the op IS stored.
    struct AppendFailsLog {
        inner: MemoryOpLog,
        why: &'static str,
    }

    impl AppendFailsLog {
        fn new(why: &'static str) -> Self {
            Self {
                inner: MemoryOpLog::new(),
                why,
            }
        }
    }

    impl OpLog for AppendFailsLog {
        fn append(&mut self, _op: SignedOp, _arrival: Arrival) -> Result<Appended, OpLogError> {
            Err(OpLogError::Storage(self.why.to_string()))
        }
        fn get(&self, id: &OpId) -> Result<Option<crate::log::Entry>, OpLogError> {
            self.inner.get(id)
        }
        fn iter(&self) -> Result<Vec<crate::log::Entry>, OpLogError> {
            self.inner.iter()
        }
        fn iter_stoa(&self, stoa: &Address) -> Result<Vec<crate::log::Entry>, OpLogError> {
            self.inner.iter_stoa(stoa)
        }
        fn iter_target(&self, target: &OpId) -> Result<Vec<crate::log::Entry>, OpLogError> {
            self.inner.iter_target(target)
        }
        fn len(&self) -> Result<usize, OpLogError> {
            self.inner.len()
        }
    }

    /// An inbound message carrying `payload` on `channel_id`, with values in the
    /// two fields that must not be believed.
    ///
    /// The sender id and timestamp are deliberately non-trivial rather than empty:
    /// a fixture passing `("", 0)` could not tell "the field is ignored" from "the
    /// field happened to be the zero value".
    fn inbound<'a>(channel_id: &'a str, payload: &'a [u8]) -> InboundMessage<'a> {
        InboundMessage {
            channel_id,
            sender_id: "a-participant",
            payload,
            timestamp: 1_700_000_000_000_000_000,
        }
    }

    // ─── Channel identity ─────────────────────────────────────────────────

    #[test]
    fn one_stoa_yields_one_channel_id_and_one_content_topic() {
        let identity = ChannelIdentity::of(&a_stoa("Agora"));
        assert!(!identity.channel_id().is_empty());
        assert!(!identity.content_topic().is_empty());
        // The two are different strings. A derivation that returned one value
        // twice would satisfy "both are present" and would mean the channel id
        // and the content topic were one namespace — which the module
        // documentation argues at length they are not.
        assert_ne!(identity.channel_id(), identity.content_topic());
    }

    #[test]
    fn two_stoas_do_not_share_a_channel() {
        let one = ChannelIdentity::of(&a_stoa("Agora"));
        let two = ChannelIdentity::of(&a_stoa("Lyceum"));
        assert_ne!(one.channel_id(), two.channel_id());
        assert_ne!(one.content_topic(), two.content_topic());
    }

    #[test]
    fn a_stoas_title_does_not_appear_in_its_content_topic() {
        // §4.1: "Never put a human-readable Stoa name in a topic." Filter, Store
        // and LightPush disclose content topics to peers, so a readable name in
        // one links a network address to an interest.
        //
        // The title is a string that could not appear by coincidence in hex, so
        // its absence is a real check rather than a probability.
        let title = "The Zzyzx Assembly";
        let identity = ChannelIdentity::of(&a_stoa(title));
        assert!(
            !identity.content_topic().contains(title),
            "the title reached the topic: {}",
            identity.content_topic()
        );
        assert!(
            !identity.channel_id().contains(title),
            "the title reached the channel id: {}",
            identity.channel_id()
        );
        // And the topic is derived from the ADDRESS alone: it contains the
        // address's hex and nothing else that varies.
        assert!(identity.content_topic().contains(&a_stoa(title).to_hex()));
    }

    #[test]
    fn no_per_op_value_reaches_the_channel_identity() {
        // One Stoa, ops of several threads, several authors and several kinds.
        // Every one travels on the same channel, because the identity is derived
        // from the Stoa and the op is not consulted at all.
        //
        // The assertion is against the identity derived from the Stoa ALONE, so a
        // derivation that mixed in anything from an op would differ from it.
        let stoa = a_stoa("Agora");
        let expected = ChannelIdentity::of(&stoa);
        let kinds = [
            OpKind::Post {
                thread: None,
                parent: None,
                body: "first".to_string(),
                attachments: vec![],
            },
            OpKind::Post {
                thread: Some(signed_post_in(stoa, "root").op.id()),
                parent: Some(signed_post_in(stoa, "root").op.id()),
                body: "a reply in another thread".to_string(),
                attachments: vec![],
            },
            OpKind::Moderate {
                target: signed_post_in(stoa, "victim").op.id(),
                action: ModerationAction::Hide,
            },
        ];
        for (seed, kind) in [3u8, 4, 5].into_iter().zip(kinds) {
            let author = a_key(seed);
            let op = Op {
                stoa,
                author: author.public_key(),
                kind,
            }
            .sign(&author);
            // The publish path is what actually routes an op, so assert through
            // it rather than only through the derivation.
            let (channels, mut log, _) = peer_in(stoa);
            let publishable = publish(op, &channels, &mut log).unwrap();
            assert_eq!(
                publishable.channel_id,
                expected.channel_id(),
                "an op's own properties reached the channel identity"
            );
        }
    }

    #[test]
    fn deriving_twice_yields_the_same_identity() {
        let stoa = a_stoa("Agora");
        assert_eq!(ChannelIdentity::of(&stoa), ChannelIdentity::of(&stoa));
    }

    #[test]
    fn identity_does_not_vary_with_the_peers_history() {
        // Named for the scenario it answers — "Identity does not vary with the
        // peer's history" — and NOT for the stronger claim that local state does
        // not participate at all. This test cannot witness that, and was once
        // named as though it could: a derivation appending
        // `std::process::id()` — local state, stable within a peer and different
        // between peers, which is exactly the silent permanent partition the
        // requirement exists to prevent — passes every assertion below, because
        // a process has one pid and agrees with itself every time it is asked.
        //
        // The stronger half is held in two places, neither of them here:
        // `the_derivation_is_a_pure_function_of_the_address` reassembles the
        // expected strings from the address alone, and
        // `the_derivation_is_pinned_to_a_known_answer` pins them against a value
        // this crate did not produce. Both fail under that mutation.
        //
        // What this test DOES witness is the half a single peer can vary: the
        // derivation is stable across its own history changing around it —
        // channels opened and closed, ops stored, time passing between calls. A
        // derivation consulting a count of prior opens, a session counter or a
        // clock would differ between the first and last assertion here.
        let stoa = a_stoa("Agora");
        let first = ChannelIdentity::of(&stoa);

        let mut channels = OpenChannels::new();
        let mut log = MemoryOpLog::new();
        for i in 0..5 {
            channels.open(&ChannelIdentity::of(&stoa));
            let op = signed_post_in(stoa, &format!("op {i}"));
            let _ = publish(op, &channels, &mut log);
            channels.close(&ChannelIdentity::of(&stoa));
        }
        // A binding constructing "another peer's key" used to sit here, never
        // read. It reached nothing — `of` takes no key — so it was a line a
        // reader could mistake for a check. That a caller cannot supply a
        // per-peer value is held by `of`'s signature and its private fields,
        // which is where the argument belongs rather than in a dead `let`.

        assert_eq!(
            ChannelIdentity::of(&stoa),
            first,
            "the derivation consulted something local"
        );
    }

    #[test]
    fn the_derivation_is_a_pure_function_of_the_address() {
        // The scenario "The derivation takes the Stoa address and nothing else",
        // witnessed for SEVERAL addresses rather than for the one the
        // known-answer pin fixes.
        //
        // Both names are reassembled here from the address's hex and the two
        // literal affixes, so the expectation is a function of the ADDRESS and of
        // nothing the derivation computed. A per-peer or per-session value
        // entering — a pid, a device id, an install counter, an epoch — makes the
        // derivation's output differ from a string derived this way, whether or
        // not that value is stable within the process asking. That is the half
        // `identity_does_not_vary_with_the_peers_history` cannot reach.
        //
        // The affixes are written as literals and not as the constants, for the
        // reason `the_content_topic_keeps_the_prefix_autosharding_reads` gives:
        // the constant is the thing that could change, so asserting against it
        // would agree with a change to it. This is deliberately weaker than the
        // known-answer pin — it does not fix the hex — and stronger in the one
        // way that matters here: it holds for every address rather than for one.
        for title in ["Agora", "Lyceum", "Academy", "The Zzyzx Assembly"] {
            let stoa = a_stoa(title);
            let hex = stoa.to_hex();
            let identity = ChannelIdentity::of(&stoa);
            assert_eq!(
                identity.channel_id(),
                format!("/dialectica/1/c/{hex}"),
                "the channel id for {title} is not the address and the prefix alone"
            );
            assert_eq!(
                identity.content_topic(),
                format!("/dialectica/1/s/{hex}/proto"),
                "the content topic for {title} is not the address and the affixes alone"
            );
        }
    }

    #[test]
    fn reopening_a_channel_does_not_change_its_identity() {
        // A user may leave a Stoa and rejoin it, and the channel id must be the
        // one every other peer is still using. There is no epoch, counter or
        // generation to distinguish the second derivation from the first.
        let stoa = a_stoa("Agora");
        let before = ChannelIdentity::of(&stoa);
        let mut channels = OpenChannels::new();
        channels.open(&before);
        assert!(channels.close(&before), "the channel must have been open");

        let after = ChannelIdentity::of(&stoa);
        assert_eq!(after.channel_id(), before.channel_id());
        assert_eq!(after.content_topic(), before.content_topic());

        // And reopening works without anything being reset.
        channels.open(&after);
        assert!(channels.is_open(before.channel_id()));
    }

    #[test]
    fn the_derivation_is_pinned_to_a_known_answer() {
        // CONSENSUS-CRITICAL, and pinned the way `identity.rs` and `stoa.rs` pin
        // theirs: hardcoded strings, derived independently of this
        // implementation.
        //
        // Every other test in this module is SELF-CONSISTENT — it compares one
        // derivation against another, so it passes unchanged if the format moves.
        // This one does not, which is its whole job. Two peers on builds that
        // disagree here do not error: each opens a channel nobody else is in, and
        // neither can observe it from inside.
        //
        // The address below was derived INDEPENDENTLY of this crate, which is
        // what makes it worth having: a value read back from the code it pins
        // agrees with any bug that code happens to have, and only ever detects
        // future change. `identity.rs` and `stoa.rs` make the same point about
        // theirs.
        //
        // Re-derive it without compiling anything here. `stoa_address` is
        // SHA-256 over the 32-byte domain prefix `STOA_ADDRESS_PREFIX`
        // (`b"/dialectica/1/Address/Stoa"` plus six NULs) followed by the record
        // bytes, so with `b"a genesis record"` as the record:
        //
        //     printf '/dialectica/1/Address/Stoa\0\0\0\0\0\0a genesis record' \
        //       > preimage.bin     # 48 bytes: 32 prefix + 16 record
        //     sha256sum preimage.bin
        //     # 6b1f1c28061e99c72e3340fb4fd07e8192b394e1327012e140f240a990d89cd8
        //
        // That is where the value came from, via `sha256sum` rather than this
        // crate's hasher. The two strings below are that hex interpolated into
        // §4.1's `/dialectica/1/s/<hex>/proto` and into the channel form.
        //
        // If this fails, do NOT update the expected values to match. Work out
        // what changed and whether the network can survive it.
        let stoa = crate::identity::stoa_address(b"a genesis record");
        assert_eq!(
            stoa.to_hex(),
            "6b1f1c28061e99c72e3340fb4fd07e8192b394e1327012e140f240a990d89cd8",
            "the fixture's Stoa address changed, so the two below prove nothing"
        );
        let identity = ChannelIdentity::of(&stoa);
        assert_eq!(
            identity.content_topic(),
            "/dialectica/1/s/6b1f1c28061e99c72e3340fb4fd07e8192b394e1327012e140f240a990d89cd8/proto",
            "the content topic derivation changed"
        );
        assert_eq!(
            identity.channel_id(),
            "/dialectica/1/c/6b1f1c28061e99c72e3340fb4fd07e8192b394e1327012e140f240a990d89cd8",
            "the channel id derivation changed"
        );
    }

    #[test]
    fn the_content_topic_keeps_the_prefix_autosharding_reads() {
        // §4.2: autosharding hashes only `application` + `version`, so
        // `/dialectica/1/` is what places every dialectica topic on one shard.
        // Asserted as a literal rather than against the constant, because the
        // constant is what could change and this is the interop claim.
        let identity = ChannelIdentity::of(&a_stoa("Agora"));
        assert!(
            identity.content_topic().starts_with("/dialectica/1/"),
            "got {}",
            identity.content_topic()
        );
        assert!(
            identity.channel_id().starts_with("/dialectica/1/"),
            "got {}",
            identity.channel_id()
        );
    }

    #[test]
    fn the_message_limit_is_pinned_to_the_transports_stated_value() {
        // §4.4's 150 KiB is "a network-wide gossipsub validation limit, not
        // unilaterally raisable", so this is interop and not a local preference.
        // A limit that had silently drifted upward would still refuse an absurd
        // payload and still satisfy any scenario probing only absurd sizes.
        assert_eq!(MAX_MESSAGE_BYTES, 153_600, "the message limit changed");
        assert_eq!(
            MAX_MESSAGE_BYTES,
            150 * 1024,
            "150 KiB is the value the network validates against"
        );
    }

    // ─── The sender identifier is never an identity ────────────────────────

    #[test]
    fn the_sender_identifier_does_not_establish_authorship() {
        // An op signed by one key, arriving under a sender identifier belonging
        // to someone else entirely. The stored op's author must be the one its
        // signature establishes.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let real_author = a_key(2);
        let op = a_post_in(stoa, "mine").sign(&real_author);
        let payload = op.to_bytes();

        let impostor = a_key(99).public_key();
        let admitted = receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                // The sender id claims to be a completely different key.
                sender_id: &impostor.to_hex(),
                payload: &payload,
                timestamp: 5,
            },
            &channels,
            &mut log,
        )
        .unwrap();

        let stored = log.get(&admitted.id).unwrap().unwrap();
        assert_eq!(
            stored.op.op.author,
            real_author.public_key(),
            "the sender identifier decided authorship"
        );
        assert_ne!(stored.op.op.author, impostor);
    }

    #[test]
    fn a_forged_sender_identifier_grants_nothing() {
        // The sharp case: the sender id matches a Stoa MODERATOR's, and the op's
        // signature does not verify. It must be refused as failing verification,
        // and the matching sender id must not admit it.
        //
        // The genesis creator is the Stoa's sole moderator (§6), so their key is
        // the most privileged value a sender id could claim.
        let creator = a_key(1);
        let genesis = Genesis {
            creator: creator.public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        };
        let stoa = genesis.address().unwrap();
        let (channels, mut log, identity) = peer_in(stoa);

        // A forgery: the op claims one author, the signature is another key's.
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = a_post_in(stoa, "forged");
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op: Op {
                author: victim.public_key(),
                ..op
            },
        };
        assert!(!forged.verify(), "the fixture must be an actual forgery");

        let refusal = receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                sender_id: &creator.public_key().to_hex(),
                payload: &forged.to_bytes(),
                timestamp: 5,
            },
            &channels,
            &mut log,
        )
        .unwrap_err();

        assert_eq!(refusal, InboundRefusal::FailsVerification);
        assert_eq!(log.len().unwrap(), 0, "a forgery was stored");
    }

    #[test]
    fn one_op_under_two_sender_identifiers_is_one_op() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let payload = signed_post_in(stoa, "arrives twice").to_bytes();

        let first = receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                sender_id: "participant-one",
                payload: &payload,
                timestamp: 1,
            },
            &channels,
            &mut log,
        )
        .unwrap();
        let second = receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                sender_id: "participant-two",
                payload: &payload,
                timestamp: 2,
            },
            &channels,
            &mut log,
        )
        .unwrap();

        assert_eq!(first.id, second.id, "the sender id reached the op id");
        assert_eq!(first.appended, Appended::Stored);
        assert_eq!(second.appended, Appended::AlreadyPresent);
        assert_eq!(log.len().unwrap(), 1, "the peer holds two ops");
    }

    #[test]
    fn the_sender_identifier_is_not_part_of_what_is_stored() {
        // Nothing recovered from a stored op carries the sender id it arrived
        // under. Asserted over the op's whole WIRE FORM rather than field by
        // field: a field-by-field check can only look at fields someone thought
        // to check, and the wire form is every byte the store kept.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let sender_id = "zzyzx-sender-identifier";
        let payload = signed_post_in(stoa, "stored").to_bytes();

        let admitted = receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                sender_id,
                payload: &payload,
                timestamp: 7,
            },
            &channels,
            &mut log,
        )
        .unwrap();

        let stored = log.get(&admitted.id).unwrap().unwrap();
        let bytes = stored.op.to_bytes();
        assert!(
            !bytes
                .windows(sender_id.len())
                .any(|w| w == sender_id.as_bytes()),
            "the sender identifier reached the stored bytes"
        );
        assert_eq!(bytes, payload, "the stored op is the op that arrived");
    }

    // ─── The arrival timestamp orders nothing ─────────────────────────────

    #[test]
    fn the_arrival_timestamp_is_not_recorded_as_ordering_metadata() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let payload = signed_post_in(stoa, "timed").to_bytes();

        let admitted = receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                sender_id: "s",
                payload: &payload,
                timestamp: 1_700_000_000_000_000_000,
            },
            &channels,
            &mut log,
        )
        .unwrap();

        let arrival = log.get(&admitted.id).unwrap().unwrap().arrival;
        // Hardcoded Nones, not "whatever the boundary produced". A boundary that
        // quietly recorded the timestamp as a Lamport value would be
        // indistinguishable from a real one forever after.
        assert_eq!(arrival.lamport(), None);
        assert_eq!(arrival.message_id(), None);
        assert!(!arrival.is_ordered_by_transport());
        assert_eq!(arrival, Arrival::unordered());
    }

    #[test]
    fn the_timestamp_handed_in_does_not_change_what_is_recorded() {
        // Far past, far future, zero, and the extremes of the type. All must
        // record the same thing — which is the assertion a boundary that
        // *scaled* or *offset* the timestamp would also pass, so the value
        // compared against is the hardcoded `Arrival::unordered()`.
        let stoa = a_stoa("Agora");
        for timestamp in [
            0i64,
            1,
            -1,
            i64::MIN,
            i64::MAX,
            -6_213_559_680_000_000_000,
            9_000_000_000_000_000_000,
        ] {
            let (channels, mut log, identity) = peer_in(stoa);
            let payload = signed_post_in(stoa, "same op").to_bytes();
            let admitted = receive(
                InboundMessage {
                    channel_id: identity.channel_id(),
                    sender_id: "s",
                    payload: &payload,
                    timestamp,
                },
                &channels,
                &mut log,
            )
            .unwrap();
            assert_eq!(
                log.get(&admitted.id).unwrap().unwrap().arrival,
                Arrival::unordered(),
                "timestamp {timestamp} reached the recorded arrival"
            );
        }
    }

    #[test]
    fn what_is_recorded_does_not_vary_with_receive_sequence() {
        // Two ops, received in one sequence and then in the reverse. Each op's
        // recorded arrival must be the same in both cases, and nothing recorded
        // may distinguish which was received first.
        //
        // A boundary that recorded a local counter would give the first-received
        // op a different value in the two runs, and the assertion compares the
        // two runs' maps rather than only checking both are "unordered".
        let stoa = a_stoa("Agora");
        let one = signed_post_in(stoa, "alpha");
        let two = signed_post_in(stoa, "beta");
        assert_ne!(one.op.id(), two.op.id(), "the fixture needs two ops");

        let recorded = |order: [&SignedOp; 2]| {
            let (channels, mut log, identity) = peer_in(stoa);
            let mut out = Vec::new();
            for (i, op) in order.into_iter().enumerate() {
                let payload = op.to_bytes();
                let admitted = receive(
                    InboundMessage {
                        channel_id: identity.channel_id(),
                        sender_id: "s",
                        payload: &payload,
                        timestamp: i as i64,
                    },
                    &channels,
                    &mut log,
                )
                .unwrap();
                out.push((admitted.id, log.get(&admitted.id).unwrap().unwrap().arrival));
            }
            out.sort_by_key(|(id, _)| *id);
            out
        };

        assert_eq!(
            recorded([&one, &two]),
            recorded([&two, &one]),
            "the receive sequence reached what was recorded"
        );
    }

    #[test]
    fn every_arrival_over_this_transport_is_recorded_as_unordered() {
        // The general form, across op kinds: whatever arrives, the metadata
        // reports it as not ordered by the transport, because the transport
        // supplied nothing to order it by.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let author = a_key(2);
        let target = signed_post_in(stoa, "subject").op.id();
        let kinds = [
            OpKind::Post {
                thread: None,
                parent: None,
                body: "p".to_string(),
                attachments: vec![],
            },
            OpKind::Revise {
                target,
                body: "r".to_string(),
                attachments: vec![],
            },
            OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
            OpKind::Vote {
                target,
                direction: crate::op::VoteDirection::Up,
            },
            OpKind::StoaMetadata {
                title: "t".to_string(),
                description: "d".to_string(),
            },
        ];
        for kind in kinds {
            let op = Op {
                stoa,
                author: author.public_key(),
                kind,
            }
            .sign(&author);
            let payload = op.to_bytes();
            let admitted = receive(
                inbound(identity.channel_id(), &payload),
                &channels,
                &mut log,
            )
            .unwrap();
            assert!(!log
                .get(&admitted.id)
                .unwrap()
                .unwrap()
                .arrival
                .is_ordered_by_transport());
        }
        assert_eq!(log.len().unwrap(), 5, "every kind must have been admitted");
    }

    // ─── The validation boundary ──────────────────────────────────────────

    #[test]
    fn a_payload_on_an_unknown_channel_is_refused() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, _) = peer_in(stoa);
        let payload = signed_post_in(stoa, "valid").to_bytes();

        // A well-formed, verifying op for a Stoa this peer HAS a channel for —
        // arriving on a channel id it does not hold. So the only thing that can
        // refuse it is the channel lookup.
        let refusal = receive(
            inbound("/dialectica/1/c/not-a-channel", &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(refusal, InboundRefusal::UnknownChannel);
        assert_eq!(log.len().unwrap(), 0, "something was stored");
    }

    #[test]
    fn a_payload_that_does_not_decode_is_refused_distinguishably() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);

        // A valid op with its last byte flipped: it decodes structurally in some
        // cases and not others, so the fixture is a payload that genuinely cannot
        // decode — truncated below the signature width.
        let refusal =
            receive(inbound(identity.channel_id(), b"junk"), &channels, &mut log).unwrap_err();
        assert!(
            matches!(refusal, InboundRefusal::Undecodable(_)),
            "got {refusal:?}"
        );
        assert_ne!(refusal, InboundRefusal::UnknownChannel);
        assert_ne!(refusal, InboundRefusal::FailsVerification);
        assert_eq!(log.len().unwrap(), 0, "a partial op was stored");
    }

    #[test]
    fn an_op_whose_signature_does_not_verify_is_refused_distinguishably() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);

        // Tamper with the BODY after signing, so the op decodes perfectly and
        // only the signature check can refuse it. A fixture that corrupted the
        // signature bytes would be refused by verification too, but this way the
        // decode is provably reached.
        let key = a_key(2);
        let signed = a_post_in(stoa, "original").sign(&key);
        let tampered = SignedOp {
            op: a_post_in(stoa, "tampered"),
            signature: signed.signature.clone(),
        };
        let payload = tampered.to_bytes();
        // The fixture must decode, or this test is the decode test again.
        assert!(SignedOp::from_bytes(&payload).is_ok());

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(refusal, InboundRefusal::FailsVerification);
        assert!(!matches!(refusal, InboundRefusal::Undecodable(_)));
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn an_op_whose_key_does_not_bind_to_its_claimed_author_is_refused() {
        // THE forgery: a VALID signature over untampered bytes, from a key that
        // is not the author the op names. Only re-deriving the address from the
        // presented key catches it, which is what `verify_authored_op` does.
        //
        // Distinguishable from a malformed payload: the payload decodes.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            author: victim.public_key(),
            ..a_post_in(stoa, "not mine")
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        let payload = forged.to_bytes();
        assert!(
            SignedOp::from_bytes(&payload).is_ok(),
            "the fixture must decode, or this is the decode test"
        );

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(refusal, InboundRefusal::FailsVerification);
        assert!(!matches!(refusal, InboundRefusal::Undecodable(_)));
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn the_whole_payload_is_what_is_decoded() {
        // A valid op's wire form followed by one extra byte. Refused, and the op
        // it begins with is NOT stored — so no prefix of the payload was decoded
        // in isolation.
        //
        // This is the property that makes "no envelope" checkable: a boundary
        // that framed the payload, or that decoded a prefix and ignored the rest,
        // would store the op and pass every other test here.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let op = signed_post_in(stoa, "valid");
        let mut payload = op.to_bytes();
        payload.push(0);

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert!(
            matches!(refusal, InboundRefusal::Undecodable(_)),
            "got {refusal:?}"
        );
        assert_eq!(
            log.get(&op.op.id()).unwrap(),
            None,
            "a prefix of the payload was decoded and stored"
        );
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn a_valid_op_is_stored_and_recorded_as_unordered() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let op = signed_post_in(stoa, "admitted");
        let payload = op.to_bytes();

        let admitted = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();
        assert_eq!(admitted.id, op.op.id());
        assert_eq!(admitted.appended, Appended::Stored);

        let stored = log.get(&op.op.id()).unwrap().unwrap();
        assert_eq!(stored.op, op);
        assert!(!stored.arrival.is_ordered_by_transport());
    }

    #[test]
    fn refusal_leaves_nothing_behind_and_the_channel_keeps_working() {
        // Every refusal in turn, then a valid op on the same channel. The log
        // must hold exactly the one valid op, and the channel must still be open.
        let stoa = a_stoa("Agora");
        let elsewhere = a_stoa("Lyceum");
        let (channels, mut log, identity) = peer_in(stoa);

        let mut oversized = signed_post_in(stoa, "big").to_bytes();
        oversized.resize(MAX_MESSAGE_BYTES + 1, 0);
        let refusals: Vec<Vec<u8>> = vec![
            b"junk".to_vec(),
            oversized,
            signed_post_in(elsewhere, "wrong stoa").to_bytes(),
        ];
        for payload in &refusals {
            assert!(
                receive(inbound(identity.channel_id(), payload), &channels, &mut log).is_err(),
                "the fixture must be refused"
            );
        }
        assert_eq!(log.len().unwrap(), 0, "a refusal stored something");
        assert!(
            channels.is_open(identity.channel_id()),
            "a refusal closed the channel"
        );

        let good = signed_post_in(stoa, "after the refusals");
        let payload = good.to_bytes();
        let admitted = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();
        assert_eq!(admitted.id, good.op.id());
        assert_eq!(log.len().unwrap(), 1);
    }

    #[test]
    fn every_refusal_is_reported_distinguishably() {
        // Five causes, five responses — a build that is behind, a corrupt or
        // hostile payload, a forgery, a misdirected op, and a peer sending more
        // than the network permits. A boundary that collapsed any two would send
        // a reader looking in the wrong place.
        //
        // Pairwise-distinct RENDERING, because the variants being distinct is
        // already guaranteed by the enum: what could collapse is the message a
        // caller or a log actually sees.
        let variants = every_refusal_variant();
        let mut seen = std::collections::HashSet::new();
        for refusal in &variants {
            assert!(
                seen.insert(refusal.to_string()),
                "{refusal:?} renders identically to another variant: {refusal}"
            );
        }
        for refusal in &variants {
            let rendered = refusal.to_string();
            assert!(!rendered.is_empty(), "{refusal:?} rendered empty");
            // The same discipline `stoa.rs` applies: this error reaches the
            // `{"error":"..."}` wire contract, so `Debug` output would put a Rust
            // type name in a user-facing field.
            assert!(
                !rendered.contains("::"),
                "{refusal:?} rendered as Rust syntax: {rendered}"
            );
        }
    }

    /// Every `InboundRefusal` variant, with a non-exhaustive match that fails to compile
    /// when one is added.
    ///
    /// The `match` is the reason this list cannot silently fall behind the enum.
    /// A bare array would compile forever while covering fewer and fewer
    /// variants — `stoa.rs` records that exact drift happening when
    /// `TitleTooLong` was added.
    fn every_refusal_variant() -> Vec<InboundRefusal> {
        let all = vec![
            InboundRefusal::UnknownChannel,
            InboundRefusal::TooLong {
                bytes: MAX_MESSAGE_BYTES + 1,
                limit: MAX_MESSAGE_BYTES,
            },
            InboundRefusal::Undecodable(OpError::Truncated),
            // A second decoder error, so the wrapped Display is exercised on
            // more than one path.
            InboundRefusal::Undecodable(OpError::TrailingBytes),
            InboundRefusal::FailsVerification,
            InboundRefusal::StoaMismatch {
                named: a_stoa("Agora"),
                channel_is_for: a_stoa("Lyceum"),
            },
            InboundRefusal::Storage(OpLogError::Storage("disk on fire".to_string())),
        ];
        if let Some(r) = all.first() {
            // Never executed; it exists only to make the compiler check the list.
            match r {
                InboundRefusal::UnknownChannel
                | InboundRefusal::TooLong { .. }
                | InboundRefusal::Undecodable(_)
                | InboundRefusal::FailsVerification
                | InboundRefusal::StoaMismatch { .. }
                | InboundRefusal::Storage(_) => {}
            }
        }
        all
    }

    #[test]
    fn a_store_failure_on_receive_is_refused_as_a_store_failure_and_not_as_a_forgery() {
        // The spec requires the five causes be "each reported distinguishably
        // from the others" because they call for five different responses. A
        // disk error reported as `FailsVerification` tells the reader a peer
        // forged an op, which sends them looking at the network for a fault on
        // their own disk — and `every_refusal_variant()` cannot catch it,
        // because it constructs `InboundRefusal::Storage` by hand and so proves only
        // that the variant renders distinctly, never that `receive` returns it.
        //
        // The payload here is a VALID, verifying op naming the right Stoa, so
        // every earlier guard passes and the append is the only thing that can
        // refuse. That is what makes the assertion specific: it fails if the
        // append's error is mapped to any other variant.
        let stoa = a_stoa("Agora");
        let identity = ChannelIdentity::of(&stoa);
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        let mut log = AppendFailsLog::new("database is locked");

        let op = signed_post_in(stoa, "arrives while the disk is unwritable");
        assert!(op.verify(), "the fixture op must pass every earlier guard");
        let bytes = op.to_bytes();
        let refusal =
            receive(inbound(identity.channel_id(), &bytes), &channels, &mut log).unwrap_err();

        assert!(
            matches!(refusal, InboundRefusal::Storage(OpLogError::Storage(_))),
            "a store failure was reported as {refusal:?}, which sends the reader \
             looking in the wrong place"
        );
        assert!(
            refusal.to_string().contains("database is locked"),
            "the store's own reason must survive so it can be named: {refusal}"
        );
        assert_eq!(log.len().unwrap(), 0, "a refused op must not be in the log");
    }

    // ─── The Stoa comparison ──────────────────────────────────────────────

    #[test]
    fn an_op_naming_another_stoa_is_refused() {
        // An AUTHENTIC op — it verifies under its author — copied unchanged onto
        // a different Stoa's channel. It cannot be rewritten to name this Stoa
        // without breaking the signature, but it can be copied, and only the
        // comparison refuses it.
        let here = a_stoa("Agora");
        let there = a_stoa("Lyceum");
        assert_ne!(here, there);
        let (channels, mut log, identity) = peer_in(here);

        let elsewhere = signed_post_in(there, "posted in the lyceum");
        assert!(
            elsewhere.verify(),
            "the fixture must be authentic, or this is the verification test"
        );
        let payload = elsewhere.to_bytes();

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            InboundRefusal::StoaMismatch {
                named: there,
                channel_is_for: here,
            }
        );
        // Distinguishable from a decode failure AND from a signature failure.
        assert!(!matches!(refusal, InboundRefusal::Undecodable(_)));
        assert_ne!(refusal, InboundRefusal::FailsVerification);
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn two_stoas_sharing_an_address_prefix_are_not_confused() {
        // WHOLE-address comparison, not "some comparison happens".
        //
        // `an_op_naming_another_stoa_is_refused` uses two hash-derived addresses
        // that almost certainly differ in byte 0, so a boundary comparing only a
        // PREFIX would pass it. This project has already shipped exactly that
        // defect twice — `op-log`'s design records a 2-byte prefix match in
        // `iter_stoa` and an 8-byte one in `iter_target`, each surviving the
        // whole suite — and the fix was to CONSTRUCT the addresses rather than
        // hunt a hash coincidence.
        //
        // Constructed the same way here: `Address::from_bytes` takes a `[u8; 32]`
        // and nothing about a channel's Stoa requires it to be hash-derived.
        let mut bytes = [0x5Au8; 32];
        let here = Address::from_bytes(bytes);
        // Differs in the LAST byte only: 31 bytes of agreement.
        bytes[31] = 0x5B;
        let there = Address::from_bytes(bytes);
        assert_ne!(here, there);
        assert_eq!(
            here.as_bytes()[..31],
            there.as_bytes()[..31],
            "the fixture must share a long prefix, or it tests the wrong thing"
        );

        let (channels, mut log, identity) = peer_in(here);
        let elsewhere = signed_post_in(there, "next door");
        let payload = elsewhere.to_bytes();

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            InboundRefusal::StoaMismatch {
                named: there,
                channel_is_for: here,
            },
            "a prefix comparison admitted another Stoa's op"
        );
        assert_eq!(log.len().unwrap(), 0);

        // And the op for THIS Stoa is admitted, so the comparison is not simply
        // refusing everything.
        let mine = signed_post_in(here, "at home");
        let mine_payload = mine.to_bytes();
        assert!(receive(
            inbound(identity.channel_id(), &mine_payload),
            &channels,
            &mut log
        )
        .is_ok());
    }

    #[test]
    fn two_channels_sharing_an_id_prefix_are_not_confused() {
        // The same property for the CHANNEL lookup. A `HashMap` compares whole
        // keys, so this holds by construction today — it is pinned because the
        // shape is what guarantees it, and a future scan-based lookup would be
        // the natural place for a prefix comparison to appear.
        let here = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(here);
        let payload = signed_post_in(here, "valid").to_bytes();

        // The open channel's id with one character appended, and with one
        // removed. Neither is the open channel.
        let longer = format!("{}0", identity.channel_id());
        let shorter = &identity.channel_id()[..identity.channel_id().len() - 1];
        for wrong in [longer.as_str(), shorter] {
            assert_eq!(
                receive(inbound(wrong, &payload), &channels, &mut log).unwrap_err(),
                InboundRefusal::UnknownChannel,
                "a prefix match admitted a payload on {wrong}"
            );
        }
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn a_cross_stoa_copy_does_not_join_the_peer_to_anything() {
        // Receiving an op must never cause a peer to hold a Stoa it did not
        // already hold: no channel opened, no identity derived into the open set,
        // and the op not stored.
        let here = a_stoa("Agora");
        let unjoined = a_stoa("A Stoa this peer has never heard of");
        let (channels, mut log, identity) = peer_in(here);
        let before = channels.len();

        let payload = signed_post_in(unjoined, "from elsewhere").to_bytes();
        assert!(receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log
        )
        .is_err());

        assert_eq!(log.len().unwrap(), 0, "the op was stored");
        assert_eq!(channels.len(), before, "a channel was opened");
        assert!(
            !channels.is_open(ChannelIdentity::of(&unjoined).channel_id()),
            "the peer opened a channel for the Stoa the op named"
        );
        assert_eq!(
            channels.stoa_of(ChannelIdentity::of(&unjoined).channel_id()),
            None
        );
    }

    // ─── The size bound ──────────────────────────────────────────────────

    #[test]
    fn a_payload_over_the_limit_is_refused_distinguishably() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let payload = vec![0u8; MAX_MESSAGE_BYTES + 1];

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            InboundRefusal::TooLong {
                bytes: MAX_MESSAGE_BYTES + 1,
                limit: MAX_MESSAGE_BYTES,
            }
        );
        // Distinguishable from a decode failure, which is the whole point: "no
        // peer could legitimately have sent this" is a different fact from "this
        // op is corrupt", and the payload above is also undecodable — so a
        // boundary checking size AFTER the decode would report the wrong one.
        assert!(!matches!(refusal, InboundRefusal::Undecodable(_)));
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn the_size_check_runs_before_the_decode() {
        // The ordering, pinned as an observable fact rather than left to the
        // reading order of the function.
        //
        // The fixture is a payload that is BOTH over-long and undecodable, and
        // the expectation is `TooLong`. A boundary that decoded first would
        // report `Undecodable` and pass every other size test here, because the
        // test above uses a payload that is also undecodable.
        //
        // This is the only test that can tell the two orderings apart, and it is
        // why the payload is junk rather than a valid op padded out.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let payload = vec![0xFFu8; MAX_MESSAGE_BYTES + 64];
        assert!(
            SignedOp::from_bytes(&payload).is_err(),
            "the fixture must be undecodable, or the orderings agree"
        );

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert!(
            matches!(refusal, InboundRefusal::TooLong { .. }),
            "the decode ran before the size check: got {refusal:?}"
        );
    }

    #[test]
    fn a_payload_at_the_limit_is_not_refused_for_its_size() {
        // The boundary is inclusive. Without this, a fencepost error in either
        // direction is invisible — both still "refuse something large" and every
        // other test passes.
        //
        // The payload is undecodable, so it is refused — but as `Undecodable`,
        // NOT as `TooLong`. That is exactly the scenario's claim: "it is not
        // refused on account of its size".
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let payload = vec![0u8; MAX_MESSAGE_BYTES];
        assert_eq!(payload.len(), MAX_MESSAGE_BYTES);

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert!(
            !matches!(refusal, InboundRefusal::TooLong { .. }),
            "a payload of exactly the limit was refused for its size"
        );
        assert!(
            matches!(refusal, InboundRefusal::Undecodable(_)),
            "got {refusal:?}"
        );
    }

    #[test]
    fn an_op_at_the_limit_is_admitted() {
        // The other half of the inclusive boundary: a payload at exactly the
        // limit that IS a valid op must be stored, not merely "not refused for
        // its size".
        //
        // Built by padding a post's body until the wire form lands exactly on the
        // limit, which is reachable because `MAX_FIELD_LEN` and
        // `MAX_MESSAGE_BYTES` are equal today — a body just under the field cap
        // yields a message just over it, so the arithmetic is checked rather than
        // assumed.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);

        let overhead = signed_post_in(stoa, "").to_bytes().len();
        let body_len = MAX_MESSAGE_BYTES - overhead;
        let op = signed_post_in(stoa, &"x".repeat(body_len));
        let payload = op.to_bytes();
        assert_eq!(
            payload.len(),
            MAX_MESSAGE_BYTES,
            "the fixture must sit exactly on the limit"
        );

        let admitted = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();
        assert_eq!(admitted.id, op.op.id());
        assert_eq!(log.len().unwrap(), 1);
    }

    #[test]
    fn a_body_at_the_authoring_cap_encodes_past_the_message_limit() {
        // The gap between two caps that are each correct on their own, measured
        // rather than argued. Found by review (findings/correctness.md entry 1,
        // findings/security.md entry 1), which reported 153,740 bytes and a
        // 140-byte overshoot; this reproduces the measurement in the suite so it
        // cannot drift out of the record.
        //
        // `authoring::MAX_BODY_LEN` is `op::MAX_FIELD_LEN`, which equals
        // `MAX_MESSAGE_BYTES` today. A body AT the authoring cap therefore encodes
        // to the limit PLUS the fixed wire overhead of a post, and `receive`
        // refuses it. The merged `content-authoring` spec requires that body to
        // publish ("A body at the cap is published"), so the two contracts
        // disagree and neither is wrong on its own — see design.md, "The publish
        // cap and the message limit leave a band of unreceivable ops".
        //
        // Asserted as arithmetic on measured values, not against a hardcoded
        // 153,740: a literal would pass a drifted cap by sitting off the boundary,
        // which is the defect `the_publish_body_cap_is_the_format_field_cap`
        // exists to prevent in `authoring.rs`.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);

        let overhead = signed_post_in(stoa, "").to_bytes().len();
        let at_cap = signed_post_in(stoa, &"x".repeat(crate::authoring::MAX_BODY_LEN));
        let payload = at_cap.to_bytes();

        assert_eq!(
            payload.len(),
            crate::authoring::MAX_BODY_LEN + overhead,
            "a post's wire form is its body plus a fixed overhead"
        );
        assert!(
            payload.len() > MAX_MESSAGE_BYTES,
            "the fixture must exceed the message limit, or it tests nothing: \
             {} bytes against a {MAX_MESSAGE_BYTES} limit",
            payload.len()
        );

        let refusal = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            InboundRefusal::TooLong {
                bytes: payload.len(),
                limit: MAX_MESSAGE_BYTES,
            },
            "an op the live publish path signs must be refused by every peer"
        );
        assert_eq!(log.len().unwrap(), 0, "a refused payload appends nothing");
    }

    // ─── No panic is reachable ────────────────────────────────────────────

    #[test]
    fn arbitrary_bytes_are_refused_without_a_panic() {
        // A panic here ABORTS the module process (PHASE0-FINDINGS §3), so a
        // panic reachable from a payload any peer may send is a remotely
        // triggerable denial of service.
        //
        // Every prefix of a valid op is included, not a handful of hand-picked
        // lengths: a boundary that read one field without a bounds check fails
        // only at the boundary that field happens to straddle.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let valid = signed_post_in(stoa, "the basis").to_bytes();

        let mut payloads: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![0xFF],
            vec![0xFF; 64],
            vec![0; MAX_MESSAGE_BYTES + 1],
        ];
        // Every prefix.
        for n in 0..valid.len() {
            payloads.push(valid[..n].to_vec());
        }
        // Every single-byte mutation of the first 96 bytes — the header, the
        // Stoa, the author key — plus the signature's first byte.
        for i in 0..96.min(valid.len()) {
            let mut m = valid.clone();
            m[i] ^= 0xFF;
            payloads.push(m);
        }
        // Lengths unrelated to anything the format uses.
        for n in [1usize, 63, 65, 97, 4096] {
            payloads.push(vec![0xABu8; n]);
        }

        for payload in payloads {
            // The assertion is that this returns at all. A panic fails the test
            // by aborting, which is louder than an assertion and is the only
            // signal available.
            let _ = receive(
                inbound(identity.channel_id(), &payload),
                &channels,
                &mut log,
            );
        }
    }

    #[test]
    fn a_hostile_channel_or_sender_identifier_does_not_panic() {
        // Empty, maximal and non-textual. `&str` is UTF-8 by construction, so
        // "non-textual" here means NUL and other control bytes, the replacement
        // character, and multi-byte characters at a boundary — the shapes a
        // slicing bug would hit.
        //
        // Two things this deliberately does NOT cover, because neither is a
        // reachable input: a `String` of raw invalid UTF-8 is unrepresentable in
        // Rust, and so is a lone surrogate — `\u{FFFD}` below is the replacement
        // character, which is what a decoder *substitutes* for one, not a
        // surrogate itself. Said plainly because the scope of this fixture list
        // is the clause a reader has to parse to know what is untested.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let payload = signed_post_in(stoa, "valid").to_bytes();

        let hostile = [
            String::new(),
            "\0".to_string(),
            "\0\0\0".to_string(),
            "\u{FFFD}".to_string(),
            "🏛".repeat(1000),
            "x".repeat(100_000),
            "/dialectica/1/c/".to_string(),
            format!("{}\0", identity.channel_id()),
            identity.channel_id().to_string(),
        ];
        for channel_id in &hostile {
            for sender_id in &hostile {
                let _ = receive(
                    InboundMessage {
                        channel_id,
                        sender_id,
                        payload: &payload,
                        timestamp: i64::MIN,
                    },
                    &channels,
                    &mut log,
                );
            }
        }
    }

    #[test]
    fn the_peer_keeps_receiving_after_a_refusal() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);

        assert!(receive(inbound(identity.channel_id(), b""), &channels, &mut log).is_err());
        let good = signed_post_in(stoa, "after");
        let payload = good.to_bytes();
        let admitted = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();
        assert_eq!(admitted.id, good.op.id());
    }

    // ─── Publishing ───────────────────────────────────────────────────────

    #[test]
    fn a_published_op_is_in_the_local_log() {
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let op = signed_post_in(stoa, "mine");
        let id = op.op.id();

        let publishable = publish(op.clone(), &channels, &mut log).unwrap();
        assert_eq!(publishable.id, id);
        assert_eq!(publishable.channel_id, identity.channel_id());
        // Readable WITHOUT any send having been confirmed — nothing here has
        // touched the transport, and the op is already stored. That is the
        // scenario's "whether or not the send has been confirmed".
        assert_eq!(log.get(&id).unwrap().unwrap().op, op);
    }

    #[test]
    fn the_bytes_published_are_the_bytes_stored() {
        // Nothing is prepended, appended or re-encoded around the op's wire
        // form. An envelope would be attacker-controlled bytes OUTSIDE the
        // signature, which is the one shape this system has no defence for.
        let stoa = a_stoa("Agora");
        let (channels, mut log, _) = peer_in(stoa);
        let op = signed_post_in(stoa, "exact bytes");
        let expected = op.to_bytes();

        let publishable = publish(op, &channels, &mut log).unwrap();
        assert_eq!(
            publishable.payload, expected,
            "the payload is not the op's wire form"
        );
        // And the same bytes as what the store holds, so a receiving peer's
        // decode of the payload yields the op its author stored.
        let stored = log.get(&publishable.id).unwrap().unwrap();
        assert_eq!(publishable.payload, stored.op.to_bytes());
    }

    #[test]
    fn a_peers_own_op_carries_no_ordering_metadata_of_its_own() {
        // A peer must not give its own ops a position other peers cannot
        // reproduce. So the recorded arrival is indistinguishable from that of an
        // op received from a peer — asserted by comparing the two directly rather
        // than by checking each is "unordered", which a boundary recording
        // different values in the two paths could still satisfy if both happened
        // to lack a Lamport value.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);

        let mine = signed_post_in(stoa, "authored here");
        let publishable = publish(mine.clone(), &channels, &mut log).unwrap();

        let theirs = signed_post_in(stoa, "authored elsewhere");
        let payload = theirs.to_bytes();
        let received = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();

        let mine_arrival = log.get(&publishable.id).unwrap().unwrap().arrival;
        let theirs_arrival = log.get(&received.id).unwrap().unwrap().arrival;
        assert_eq!(mine_arrival.lamport(), None);
        assert_eq!(
            mine_arrival, theirs_arrival,
            "a published op's arrival differs from a received one's"
        );
        assert_eq!(mine_arrival, Arrival::unordered());
    }

    #[test]
    fn publishing_without_an_open_channel_fails_and_opens_nothing() {
        // Distinguishable from a transport failure on an open channel, no channel
        // opened as a side effect, and the op still stored — the author authored
        // it.
        let stoa = a_stoa("Agora");
        let channels = OpenChannels::new();
        let mut log = MemoryOpLog::new();
        let op = signed_post_in(stoa, "no channel for this");
        let id = op.op.id();

        let err = publish(op.clone(), &channels, &mut log).unwrap_err();
        assert_eq!(err, PublishError::NoChannel { stoa, id });
        // Not the store failure, which is the other way a publish fails.
        assert!(!matches!(err, PublishError::NotStored(_)));
        assert!(channels.is_empty(), "publishing opened a channel");
        assert!(
            !channels.is_open(ChannelIdentity::of(&stoa).channel_id()),
            "publishing opened this Stoa's channel"
        );
        assert_eq!(
            log.get(&id).unwrap().unwrap().op,
            op,
            "the op was not stored"
        );
    }

    #[test]
    fn a_publish_that_could_not_store_is_not_reported_as_a_missing_channel() {
        // The other side of `publishing_without_an_open_channel_fails_and_opens_
        // nothing`'s `!matches!(err, NotStored(_))`, which could not fail while
        // nothing in a test's reach produced `NotStored`.
        //
        // The two failures are opposites and the spec requires they be
        // distinguishable: `NoChannel` means the op EXISTS and did not go out —
        // do not retry, do not discard — while `NotStored` means it does not
        // exist at all. A caller told the wrong one either loses a stored post or
        // keeps a post that was never written. The channel is OPEN here, so
        // `NoChannel` is not a possible answer and the append is the only
        // failure left.
        let stoa = a_stoa("Agora");
        let identity = ChannelIdentity::of(&stoa);
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        let mut log = AppendFailsLog::new("no space left on device");

        let op = signed_post_in(stoa, "the store is full");
        let id = op.op.id();
        let err = publish(op, &channels, &mut log).unwrap_err();

        assert!(
            matches!(err, PublishError::NotStored(OpLogError::Storage(_))),
            "a store failure was reported as {err:?}"
        );
        assert!(
            !matches!(err, PublishError::NoChannel { .. }),
            "a store failure reported as a missing channel claims the op exists"
        );
        assert!(
            err.to_string().contains("no space left on device"),
            "the store's reason must survive: {err}"
        );
        assert!(
            log.get(&id).unwrap().is_none(),
            "nothing may be in the log after the append that would have put it \
             there failed"
        );
    }

    #[test]
    fn the_two_ways_a_publish_fails_disagree_about_whether_the_op_exists() {
        // Pins the distinction itself rather than each variant separately, since
        // what a caller acts on is which of the two it got. One op, one Stoa, two
        // peers differing only in whether the channel is open and whether the
        // store works — and the two answers must differ in their variant AND in
        // whether the op is afterwards readable.
        let stoa = a_stoa("Agora");
        let identity = ChannelIdentity::of(&stoa);
        let op = signed_post_in(stoa, "one body, two failures");
        let id = op.op.id();

        // No channel, working store: stored, not sent.
        let mut stored_log = MemoryOpLog::new();
        let no_channel = publish(op.clone(), &OpenChannels::new(), &mut stored_log).unwrap_err();

        // Channel open, broken store: not stored.
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        let mut broken_log = AppendFailsLog::new("database is locked");
        let not_stored = publish(op, &channels, &mut broken_log).unwrap_err();

        assert_ne!(
            no_channel, not_stored,
            "the two failures a caller must respond to differently compare equal"
        );
        assert_ne!(
            no_channel.to_string(),
            not_stored.to_string(),
            "the two failures render identically, so a view cannot tell them apart"
        );
        assert!(
            stored_log.get(&id).unwrap().is_some(),
            "a missing channel must not lose the op"
        );
        assert!(
            broken_log.get(&id).unwrap().is_none(),
            "a store failure must not leave the op readable"
        );
    }

    #[test]
    fn publishing_on_another_stoas_open_channel_still_fails() {
        // A peer with SOME channel open is not a peer with THIS Stoa's channel
        // open. A boundary that checked "is any channel open" rather than "is
        // this Stoa's channel open" would pass the test above, because that one
        // uses an empty set.
        let here = a_stoa("Agora");
        let there = a_stoa("Lyceum");
        let (channels, mut log, _) = peer_in(here);

        let op = signed_post_in(there, "for the lyceum");
        let id = op.op.id();
        let err = publish(op, &channels, &mut log).unwrap_err();
        assert_eq!(err, PublishError::NoChannel { stoa: there, id });
        assert_eq!(log.len().unwrap(), 1, "the op must still be stored");
    }

    #[test]
    fn a_send_failure_does_not_lose_the_op() {
        // `publish` RETURNS the bytes rather than sending them, so there is no
        // code path here that could undo the append — which is the requirement
        // made structural rather than checked.
        //
        // What a test can witness is the consequence: after a publish, the op is
        // in the log and byte-identical, and a caller dropping the `Publishable`
        // without sending it — which is exactly what a transport failure looks
        // like from here — changes nothing.
        let stoa = a_stoa("Agora");
        let (channels, mut log, _) = peer_in(stoa);
        let op = signed_post_in(stoa, "the send will fail");
        let expected_bytes = op.to_bytes();
        let id = op.op.id();

        let publishable = publish(op.clone(), &channels, &mut log).unwrap();
        // The transport failed: the bytes were never handed over.
        drop(publishable);

        let stored = log.get(&id).unwrap().unwrap();
        assert_eq!(stored.op, op);
        assert_eq!(
            stored.op.to_bytes(),
            expected_bytes,
            "the stored op is not byte-identical to what was published"
        );
        assert!(stored.op.verify(), "the signature did not survive");
    }

    #[test]
    fn a_send_that_the_transport_accepted_is_not_a_delivery() {
        // The requirement "A successful publish is a statement about the local
        // log and nothing more", which contracts what this test pins: a publish
        // reports the handoff and no field of what it reports may carry, imply or
        // be documented as carrying a delivery outcome.
        //
        // This was a `NO SPEC:` marker until that requirement existed. The
        // behaviour is now specified rather than chosen, so what remains worth
        // knowing at the call site is where the delivery outcome went: the
        // requirement names three things owed and not supplied here — the bound,
        // what a peer records for an op in flight, and what it records for one
        // that never propagated — because `channelMessageSent` and
        // `messagePropagated` arrive on separate events keyed by the `requestId`
        // that `channelSend` returns, after this call is over.
        let stoa = a_stoa("Agora");
        let (channels, mut log, _) = peer_in(stoa);
        let op = signed_post_in(stoa, "may never arrive");
        let id = op.op.id();

        let publishable = publish(op, &channels, &mut log).unwrap();
        // Everything the caller is told: an op id, a channel, and bytes. Nothing
        // about whether any peer received it, and no field that could be read as
        // saying so.
        assert_eq!(publishable.id, id);
        assert!(!publishable.channel_id.is_empty());
        assert!(!publishable.payload.is_empty());
        assert!(
            log.get(&id).unwrap().is_some(),
            "the only thing a publish establishes is that the op is local"
        );
    }

    #[test]
    fn a_published_op_does_not_depend_on_being_received_back() {
        // The receive event does not fire for a participant's own messages, so a
        // peer relying on the receive path for its own ops would never store
        // them. Here no inbound arrival is ever delivered and the peer holds the
        // op regardless.
        let stoa = a_stoa("Agora");
        let (channels, mut log, _) = peer_in(stoa);
        let op = signed_post_in(stoa, "never comes back");
        let id = op.op.id();

        // Discarded on purpose: this test is about the log, and never sending is
        // the scenario. `Publishable` is `#[must_use]`, so the discard is written
        // out rather than implied.
        let _ = publish(op, &channels, &mut log).unwrap();
        assert!(log.get(&id).unwrap().is_some());
        assert_eq!(log.len().unwrap(), 1);
    }

    #[test]
    fn an_op_the_peer_already_holds_arriving_again_is_one_op() {
        // A peer's own published op, then the same op arriving on the channel.
        // One entry, and the arrival recorded FIRST is the one kept.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let op = signed_post_in(stoa, "mine, echoed");
        let id = op.op.id();

        // Discarded on purpose: the echo below is built from `op`, not from the
        // payload this returns. Written out because `Publishable` is `#[must_use]`.
        let _ = publish(op.clone(), &channels, &mut log).unwrap();
        let first_arrival = log.get(&id).unwrap().unwrap().arrival;

        let payload = op.to_bytes();
        let admitted = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();
        assert_eq!(admitted.id, id);
        assert_eq!(
            admitted.appended,
            Appended::AlreadyPresent,
            "a second arrival reported itself as new"
        );
        assert_eq!(log.len().unwrap(), 1);
        assert_eq!(
            log.get(&id).unwrap().unwrap().arrival,
            first_arrival,
            "the second arrival replaced the first's metadata"
        );
    }

    // ─── This boundary decides nothing beyond admitting ───────────────────

    #[test]
    fn an_authentic_moderation_op_from_a_non_moderator_is_admitted() {
        // Admitting is not a statement that the moderation binds. Authority is
        // decided on read, against state a peer may not have held when the op
        // arrived — so a boundary that also decided authority would decide it
        // once, from whatever this peer knew at one instant.
        let creator = a_key(1);
        let genesis = Genesis {
            creator: creator.public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        };
        let stoa = genesis.address().unwrap();
        let (channels, mut log, identity) = peer_in(stoa);

        let random_peer = a_key(9);
        assert_ne!(
            random_peer.public_key(),
            creator.public_key(),
            "the author must not be the Stoa's moderator"
        );
        let hide = Op {
            stoa,
            author: random_peer.public_key(),
            kind: OpKind::Moderate {
                target: signed_post_in(stoa, "victim").op.id(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&random_peer);
        assert!(hide.verify(), "authentic, and carrying no authority");
        let payload = hide.to_bytes();

        let admitted = receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();
        assert_eq!(admitted.appended, Appended::Stored);
        // And storing it says nothing about whether it binds: the moderator set
        // that would decide that is not consulted here, and `Admitted` carries no
        // field that could be read as an authority answer.
        let stored = log.get(&admitted.id).unwrap().unwrap();
        assert!(!crate::moderation::Moderators::of(&genesis)
            .unwrap()
            .contains(&stored.op.op.author));
    }

    #[test]
    fn an_unauthentic_op_is_not_admitted_on_the_promise_of_a_later_check() {
        // The converse, and the half that reads as a contradiction of the test
        // above. Authority being checked later is not a reason to admit a
        // forgery: the two checks answer different questions against different
        // inputs, and neither substitutes for the other.
        let stoa = a_stoa("Agora");
        let (channels, mut log, identity) = peer_in(stoa);
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            author: victim.public_key(),
            ..a_post_in(stoa, "forged")
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        let payload = forged.to_bytes();

        assert_eq!(
            receive(
                inbound(identity.channel_id(), &payload),
                &channels,
                &mut log
            )
            .unwrap_err(),
            InboundRefusal::FailsVerification
        );
        assert_eq!(
            log.len().unwrap(),
            0,
            "a forgery was stored for a reader to judge later"
        );
    }

    // ─── Channel lifecycle ────────────────────────────────────────────────

    #[test]
    fn closing_one_stoas_channel_closes_that_one_and_no_other() {
        // Named for the OPERATION, not for the event. There is no leave-Stoa
        // handler in this application, so a test named
        // `leaving_a_stoa_closes_its_channel_and_no_other` described something no
        // site does — and would have kept reading as satisfied once a handler
        // landed that closed the wrong channel or none. The obligation that some
        // handler must eventually call this is the spec's, named as owed; what is
        // checkable here is that the operation closes the one channel it names.
        let one = a_stoa("Agora");
        let two = a_stoa("Lyceum");
        let mut channels = OpenChannels::new();
        let (id_one, id_two) = (ChannelIdentity::of(&one), ChannelIdentity::of(&two));
        channels.open(&id_one);
        channels.open(&id_two);

        assert!(channels.close(&id_one));
        assert!(!channels.is_open(id_one.channel_id()));
        assert!(
            channels.is_open(id_two.channel_id()),
            "another Stoa's channel was closed"
        );
        assert_eq!(channels.len(), 1);
    }

    #[test]
    fn emptiness_tracks_what_is_open_in_both_directions() {
        // `is_empty` had two call sites and both asserted it returns TRUE, so
        // replacing its body with `true` passed the whole suite — the one
        // surviving mutant of 32 on this file. A predicate no test ever observes
        // returning false is a predicate with one observed value, which is a
        // constant.
        //
        // So this asserts the FALSE direction, and asserts it against `len`,
        // which is derived from the same map but is a different function: the two
        // disagreeing is the shape that catches either one going constant. A
        // hardcoded count is what each is compared against, rather than the other
        // being taken as the authority.
        let stoas = [a_stoa("Agora"), a_stoa("Lyceum")];
        let mut channels = OpenChannels::new();
        assert!(channels.is_empty(), "a fresh peer has no channel open");
        assert_eq!(channels.len(), 0);

        channels.open(&ChannelIdentity::of(&stoas[0]));
        assert!(
            !channels.is_empty(),
            "one channel is open, so this peer is not empty"
        );
        assert_eq!(channels.len(), 1);

        channels.open(&ChannelIdentity::of(&stoas[1]));
        assert!(!channels.is_empty());
        assert_eq!(channels.len(), 2);

        // Closing one leaves the other, so emptiness is not "anything was ever
        // closed" — the reading a single-channel fixture could not tell apart.
        assert!(channels.close(&ChannelIdentity::of(&stoas[0])));
        assert!(
            !channels.is_empty(),
            "one channel remains open, so this peer is still not empty"
        );
        assert_eq!(channels.len(), 1);

        assert!(channels.close(&ChannelIdentity::of(&stoas[1])));
        assert!(channels.is_empty(), "every channel was closed");
        assert_eq!(channels.len(), 0);
    }

    #[test]
    fn closing_every_open_channel_yields_each_channels_identifier() {
        // Named for the operation rather than for shutdown, which has no handler
        // in this application: `shutdown_closes_every_open_channel` claimed a
        // shutdown path this test never exercises. What it does check is the half
        // the absent handler will need — that `close_all` empties the record AND
        // reports each closed channel's identifier, so a caller can act on each at
        // the transport.
        let stoas = [a_stoa("Agora"), a_stoa("Lyceum"), a_stoa("Academy")];
        let mut channels = OpenChannels::new();
        let mut expected: Vec<String> = stoas
            .iter()
            .map(|s| ChannelIdentity::of(s).channel_id().to_string())
            .collect();
        for stoa in &stoas {
            channels.open(&ChannelIdentity::of(stoa));
        }
        assert_eq!(channels.len(), 3);

        let closed = channels.close_all();
        expected.sort();
        assert_eq!(closed, expected, "not every channel was closed");
        assert!(channels.is_empty());
        for stoa in &stoas {
            assert!(!channels.is_open(ChannelIdentity::of(stoa).channel_id()));
        }
    }

    #[test]
    fn a_channel_closed_can_be_reopened_under_the_same_identifier() {
        // In the same session, under the SAME channel identifier. Nothing is
        // reset and no epoch distinguishes the second open from the first.
        //
        // Named for the close-then-open operations rather than for "rejoining a
        // Stoa", which is an event with no site here — joining and leaving belong
        // to the Stoa-lifecycle capability. The derivation is done afresh below, as
        // a rejoin would, which is the part of a rejoin this file can witness.
        let stoa = a_stoa("Agora");
        let mut channels = OpenChannels::new();
        let identity = ChannelIdentity::of(&stoa);
        channels.open(&identity);
        let id_when_first_open = identity.channel_id().to_string();

        assert!(channels.close(&identity));
        // Rejoin: derive again from scratch, as a fresh join would.
        let rejoined = ChannelIdentity::of(&stoa);
        channels.open(&rejoined);

        assert_eq!(rejoined.channel_id(), id_when_first_open);
        assert!(channels.is_open(&id_when_first_open));
        assert_eq!(channels.stoa_of(&id_when_first_open), Some(&stoa));
    }

    #[test]
    fn closing_a_channel_keeps_the_ops_received_on_it() {
        // The ops are the peer's. Leaving a Stoa is a statement about what it
        // listens to, not about what it has seen.
        let stoa = a_stoa("Agora");
        let (mut channels, mut log, identity) = peer_in(stoa);
        let op = signed_post_in(stoa, "received before leaving");
        let payload = op.to_bytes();
        receive(
            inbound(identity.channel_id(), &payload),
            &channels,
            &mut log,
        )
        .unwrap();

        assert!(channels.close(&identity));
        assert_eq!(log.len().unwrap(), 1, "closing discarded the ops");
        assert_eq!(log.get(&op.op.id()).unwrap().unwrap().op, op);
        assert_eq!(log.iter_stoa(&stoa).unwrap().len(), 1);
    }

    #[test]
    fn closing_a_channel_that_is_not_open_is_not_an_error() {
        // A close is best-effort, and "it was not open" is an answer rather than
        // a failure — a shutdown closing every channel must not care whether a
        // leave already closed one.
        let stoa = a_stoa("Agora");
        let mut channels = OpenChannels::new();
        let identity = ChannelIdentity::of(&stoa);
        assert!(!channels.close(&identity), "nothing was open to close");
        channels.open(&identity);
        assert!(channels.close(&identity));
        assert!(!channels.close(&identity), "the second close found nothing");
    }

    #[test]
    fn an_open_channel_records_the_stoa_its_identity_was_derived_for() {
        // The map's invariant, pinned: a channel id maps to the Stoa the identity
        // was derived from, and there is no way to register a mismatched pair
        // because `open` takes the identity rather than the two values.
        //
        // This is what the Stoa-mismatch refusal compares against, so an
        // invariant that failed here would make that refusal compare against a
        // lie.
        let stoa = a_stoa("Agora");
        let identity = ChannelIdentity::of(&stoa);
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        assert_eq!(channels.stoa_of(identity.channel_id()), Some(&stoa));
        assert_eq!(identity.stoa(), &stoa);
    }

    #[test]
    fn opening_a_channel_twice_is_one_channel() {
        let stoa = a_stoa("Agora");
        let identity = ChannelIdentity::of(&stoa);
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        channels.open(&identity);
        channels.open(&ChannelIdentity::of(&stoa));
        assert_eq!(channels.len(), 1);
        assert_eq!(channels.stoa_of(identity.channel_id()), Some(&stoa));
    }

    #[test]
    fn a_publishable_carries_the_channel_the_op_belongs_on() {
        // Two Stoas, both open. Each op must go out on its own Stoa's channel —
        // a publish that returned "the" channel, or the first one, would put one
        // Stoa's ops on another's.
        let one = a_stoa("Agora");
        let two = a_stoa("Lyceum");
        let mut channels = OpenChannels::new();
        channels.open(&ChannelIdentity::of(&one));
        channels.open(&ChannelIdentity::of(&two));
        let mut log = MemoryOpLog::new();

        let in_one = publish(signed_post_in(one, "here"), &channels, &mut log).unwrap();
        let in_two = publish(signed_post_in(two, "there"), &channels, &mut log).unwrap();

        assert_eq!(in_one.channel_id, ChannelIdentity::of(&one).channel_id());
        assert_eq!(in_two.channel_id, ChannelIdentity::of(&two).channel_id());
        assert_ne!(in_one.channel_id, in_two.channel_id);
    }
}
