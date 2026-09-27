//! What orders two ops, and what a peer merely records about a delivery.
//!
//! # Why this file exists
//!
//! An ordering over the ops on one target that every peer computes identically.
//! Without it an edit has no defined "earlier version" and a hide has no defined
//! reversal, because every resolver falls back to ascending op id — a hash,
//! carrying no recency whatever.
//!
//! # The order is the op's own counter, and nothing the transport says
//!
//! [`cmp_ops`] leads with the **Lamport counter carried inside the op's signed
//! bytes**, descending, and breaks ties on **ascending op id**. Both inputs
//! travel with the op, so two peers holding the same two ops compute the same
//! order from the ops alone — consulting no local state, no arrival record, and
//! nothing either peer received separately.
//!
//! **This module previously forbade exactly that**, and the reversal is
//! deliberate. Its rule was the transport's: *"A peer SHALL NOT compute a
//! Lamport timestamp of its own, and SHALL NOT maintain a second logical clock
//! alongside the transport's."* That prohibition is withdrawn.
//!
//! The reasoning behind it is **preserved and re-aimed**, because it is still
//! the thing to defend against: *two orders over the same messages can disagree,
//! and the disagreement produces no error — each peer stays internally
//! consistent while rendering a thread differently from its neighbour.* That
//! failure is exactly as bad as it was. What changed is which single order is
//! authoritative.
//!
//! The prohibition assumed the transport's order was available to defer to. It
//! is not: the Reliable Channel API's `MessageReceivedEvent` carries exactly one
//! field, the reassembled payload, so `delivery_module`'s
//! `channelMessageReceived` cannot forward a Lamport value it was never given.
//! So the practical effect was never "use the transport's order instead of
//! ours" — it was **no order at all**. A rule that forbids the only available
//! order in favour of one that never arrives protects nothing.
//!
//! **Ordering does not consult the transport's Lamport timestamp or message id,
//! and cannot**: [`OpEntry`] does not carry an [`Arrival`], so there is nothing
//! for [`cmp_ops`] to read. A second order would be precisely the disagreement
//! the original reasoning names, arrived at from the other side. SDS's clock and
//! this counter both start from epoch-milliseconds and take the same step on
//! send, but SDS's also advances on traffic no application sees, so the two
//! advance on different sets of messages and cannot be relied on to agree op
//! for op.
//!
//! # The current time decides a counter and an admission, never a comparison
//!
//! [`next_counter`] reads the author's current time and [`exceeds_receive_window`]
//! reads the receiver's. Neither is an input to [`cmp_ops`], so two peers
//! holding the same ops still compute the same order from the ops alone.
//!
//! # [`Arrival`] survives, and records rather than orders
//!
//! A peer still writes down what the transport said about a delivery, because
//! that is an honest record and discarding it to make an ordering change would
//! be throwing away a fact. Nothing reads it to order anything.
//!
//! The consequence is that **"seeing one op twice" stops being a problem at
//! all**. Both clock fields are inside the preimage, so two arrivals of one op
//! carry identical values for them — a peer cannot receive one op with two
//! different counters, because that would be two ops with two ids. The old
//! hazard was that the richer arrival was a per-peer accident; the order no
//! longer depends on an arrival, richer or not. First-wins remains the store's
//! rule for the recorded metadata, which is now a question about a record rather
//! than about an order.

use crate::op::{Op, OpId};
use std::cmp::Ordering;

/// An SDS message id, as the transport assigns it.
///
/// **Not an [`OpId`].** An op id is the hash of an op's own canonical bytes and
/// is computed identically by every peer holding the op; this is assigned by the
/// transport, per LIP-109 as `keccak-256(senderId + timestamp + content)`.
/// `op.rs` is careful about the same distinction from the other side: "§5.7's
/// tiebreak — 'ties broken by ascending message id' — refers to that one, which
/// arrives with the message and is the transport's to assign. Two identifiers,
/// two jobs."
///
/// Held as an opaque byte string rather than a fixed-width array because the
/// width is the transport's to choose and is not settled at a contract we can
/// read. Nothing here parses it; it is compared, and comparison is all §5.7 asks
/// of it.
///
/// **What settling the width would unlock:** a `[u8; N]` here makes both this
/// and [`Arrival`] `Copy`, since `Arrival`'s other field is already an
/// `Option<u64>`. Today every op-log append clones an `Arrival`, which is
/// nobody's bottleneck and not a reason to guess at a width — but whoever
/// settles it should know that is what it buys.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MessageId(Vec<u8>);

impl MessageId {
    pub fn new(bytes: Vec<u8>) -> Self {
        MessageId(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The transport metadata a receiving peer records alongside an op.
///
/// # Both fields are optional, and the shape is the answer rather than a caveat
///
/// The alternative was `{lamport: u64, message_id: MessageId}` with a sentinel
/// — `0` meaning "not supplied". That makes "did the transport order this?" a
/// question every call site must remember to ask and answer the same way, which
/// is CLAUDE.md's fourth-slightly-different-guard signal.
///
/// `Option` makes it unaskable-incorrectly: a caller cannot reach the Lamport
/// value without confronting its absence, because the compiler will not let
/// them. The degraded path is not a branch that might be forgotten at the fourth
/// call site; it is the only route to the value.
///
/// **This is what a peer records, not what it computes.** Today the contract
/// supplies neither field, so the boundary constructs [`Arrival::unordered`] and
/// the degraded order applies. When the fields arrive, they are populated at the
/// boundary and nothing downstream changes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Arrival {
    /// SDS's logical clock for the channel, if the transport supplied it.
    ///
    /// Never a local clock reading. `delivery_module`'s `channelMessageReceived`
    /// does carry a `timestamp`, and it is tempting precisely because it is
    /// already in the signature — but it is a `CLOCK_REALTIME` read taken when
    /// the receiving peer's callback fired, so it differs per peer for one
    /// message and moves with NTP. Recording it here would be recording arrival
    /// sequence while believing we recorded a shared order.
    lamport: Option<u64>,
    /// SDS's message id, if the transport supplied it. The §5.7 tiebreak.
    message_id: Option<MessageId>,
}

impl Arrival {
    /// An op the transport ordered.
    pub fn ordered(lamport: u64, message_id: MessageId) -> Self {
        Arrival {
            lamport: Some(lamport),
            message_id: Some(message_id),
        }
    }

    /// An op that arrived with no ordering metadata at all.
    ///
    /// **What the boundary constructs today**, because
    /// `channelMessageReceived(channelId, senderId, payload, timestamp)` supplies
    /// neither field. Named rather than reached by passing two `None`s, so that
    /// grepping for it finds every place the contract's gap is being absorbed.
    pub fn unordered() -> Self {
        Arrival {
            lamport: None,
            message_id: None,
        }
    }

    /// Record exactly what the transport supplied, including partially.
    ///
    /// Exists because the two fields are independent at the contract level, so a
    /// future event could carry one and not the other. A recorder must be able
    /// to write down what it actually received; the alternative is inventing the
    /// other half, which is what this whole module refuses to do.
    pub fn from_parts(lamport: Option<u64>, message_id: Option<MessageId>) -> Self {
        Arrival {
            lamport,
            message_id,
        }
    }

    pub fn lamport(&self) -> Option<u64> {
        self.lamport
    }

    pub fn message_id(&self) -> Option<&MessageId> {
        self.message_id.as_ref()
    }
}

/// How far ahead of a receiving peer's own current time an op's counter may be
/// and still be admitted: one hour, in milliseconds.
///
/// # What it replaced, and why it refuses where its predecessor did not
///
/// This replaced `ADVANCE_BOUND`, which stored every counter and declined only
/// to advance the clock past one more than a million above it. That kept the
/// clock below ops already ordered above it, so an honest peer answering an op
/// signed far ahead signed a **lower** counter than the op it answered — and
/// since the thread read became the reverse of [`cmp_ops`] (#159), the answered
/// reply came after every answer to it. The window removes such an op instead:
/// nothing more than an hour ahead is stored, so the clock can follow every
/// counter held, and an answer always carries the greater counter.
///
/// It is a refusal of an authentic op for a field's value, which this system
/// previously ruled out. The cost — a peer whose clock runs more than an hour
/// fast has its ops refused, and one running more than an hour slow refuses
/// honest ops — is worked through for both in the archived `time-pegged-clock`
/// change's `design.md`, Decision 11.
///
/// # The same on every peer
///
/// A constant rather than a setting, because two peers with different windows
/// disagree about which ops exist, silently.
///
/// **Pinned by a hardcoded assertion**, following `MAX_FIELD_LEN` and
/// `LAYOUT_VERSION`: `cargo mutants` does not mutate a `const`, so a value that
/// drifted here would be invisible to it.
pub const RECEIVE_WINDOW_MS: u64 = 60 * 60 * 1000;

/// Whether an arriving op's counter is further ahead of this peer's current time
/// than [`RECEIVE_WINDOW_MS`] allows.
///
/// # It takes the counter and nothing else
///
/// Not an [`crate::op::OpClock`] and not an [`Op`]: `op-ordering` requires that
/// the op's wall-clock field not affect this decision, and a function handed a
/// `u64` cannot read a field it was never given. An op carrying no counter is
/// not subject to the window at all, which is the caller's `Option` to unwrap.
///
/// # Called from the receive boundary and from nowhere else
///
/// [`crate::transport::receive`] is its one caller. Rebuilding a store,
/// replaying ops and restoring a snapshot all go through
/// [`crate::log::OpLog::append`], which takes no time — so a path that holds no
/// `now_ms` cannot reach this without someone widening the log's API to carry
/// one. That is what keeps a store rebuilt on a peer whose clock has since been
/// set back from refusing ops it already admitted. `design.md` Decision 4.
///
/// # No arithmetic here can wrap
///
/// `saturating_sub` answers every `counter <= now_ms` as zero, which is "not
/// ahead" — the spec's "no lower bound" — however far in the past the counter
/// is. The obvious `counter > now_ms + RECEIVE_WINDOW_MS` overflows when
/// `now_ms` is within an hour of `u64::MAX`, panics in a debug build, and a
/// panic aborts the module process (PHASE0-FINDINGS §3) on a value the host's
/// clock chose.
pub fn exceeds_receive_window(counter: u64, now_ms: u64) -> bool {
    counter.saturating_sub(now_ms) > RECEIVE_WINDOW_MS
}

/// This peer's Lamport clock for one Stoa: the highest counter of the ops it
/// holds, or zero where it holds none.
///
/// # Derived from the ops, never stored
///
/// The ops are the only input, so a peer that reopens its store — or rebuilds it
/// entirely by replaying ops it re-fetched — computes the value it had, with no
/// migration, recovery step or high-water-mark record. A stored counter would be
/// a second source of truth, and the two disagree in exactly the cases that
/// matter: a store restored from a backup, a replay reaching further back than
/// the counter, a crash between appending an op and updating the counter. In
/// each, the stored value is the wrong one and the one a naive implementation
/// would believe.
///
/// # The maximum, with no exception for a counter far above the rest
///
/// This was an ascending fold that stopped at the first step larger than
/// `ADVANCE_BOUND`, and the sorting was its whole correctness argument. Both are
/// gone: the receive window keeps a counter more than an hour ahead out of the
/// store, so every counter this reads is one the peer admitted, and a maximum is
/// a function of the set whatever sequence the ops arrived in.
///
/// **It reads no time.** The current time enters when an op is signed, through
/// [`next_counter`], and never here, so the clock stays a function of the ops
/// held and nothing else.
///
/// # Scoped per Stoa by the caller
///
/// This function is handed the counters of one Stoa's ops. Ops of one Stoa never
/// order against ops of another, and a shared clock would leak one Stoa's
/// activity into another's counters — letting a reader in a quiet Stoa infer
/// that the peer is busy elsewhere.
pub fn clock_from_counters(counters: impl IntoIterator<Item = u64>) -> u64 {
    counters.into_iter().max().unwrap_or(0)
}

/// The counter a peer signs into its next op for a Stoa: the later of its
/// current time and one above its clock.
///
/// SDS's send rule (LIP-109, `max(timeNowInMs, current_lamport_timestamp + 1)`),
/// applied per Stoa. The two halves do different jobs:
///
/// - **One above the clock** keeps the counter a causality mechanism: a peer
///   holding an op at N publishes above N whatever its own time says, so an
///   answer always carries a greater counter than the op it answers.
/// - **The current time** lets a peer that holds nothing of a Stoa order
///   correctly: it publishes among the Stoa's recent ops rather than at one,
///   below everything it has not yet received.
///
/// One `max` rather than a branch on which is larger, because the two cases are
/// the same expression.
///
/// **Saturating, never wrapping.** A wrapped counter would place the highest op
/// below the lowest, inverting the order for every op in the Stoa at once.
/// Saturation costs one op its position; wrapping costs the Stoa its order.
pub fn next_counter(clock: u64, now_ms: u64) -> u64 {
    now_ms.max(clock.saturating_add(1))
}

/// One op as the order sees it: the counter it carries, and its id.
///
/// # It does NOT carry an [`Arrival`], and that is the point
///
/// The spec requires that ordering not consult the transport's Lamport timestamp
/// or message id. A comparator still handed an `Arrival` would satisfy that by
/// not writing a line — a guard, in CLAUDE.md's sense, that has to stay right
/// through every future edit. A comparator that cannot **name** an `Arrival`
/// satisfies it because there is nothing to consult.
///
/// So "every input to the comparison is carried inside the ops being compared"
/// is a statement about this type rather than about [`cmp_ops`]'s body, and a
/// change that reintroduced transport metadata into the order would have to
/// widen a public struct to do it.
///
/// `counter` is `None` exactly when the op was encoded under the version
/// predating the clock fields.
#[derive(Clone, Copy, Debug)]
pub struct OpEntry<'a> {
    pub counter: Option<u64>,
    pub id: &'a OpId,
}

impl<'a> OpEntry<'a> {
    /// From the op itself, which is the only correct source.
    ///
    /// Takes the whole [`Op`] rather than a counter a caller extracted, so that
    /// no call site can pair one op's counter with another op's id — which is
    /// the shape a two-argument constructor invites and which no test would
    /// catch, because both values are plausible.
    pub fn of(op: &'a Op, id: &'a OpId) -> Self {
        OpEntry {
            counter: op.clock.map(|c| c.counter),
            id,
        }
    }

    /// From a counter directly, for a caller that has one and no op.
    ///
    /// The storage layer's agreement test needs this — it compares the SQL order
    /// against this comparator over synthesised counters — and so does any
    /// caller reasoning about the order without materialising ops.
    pub fn new(counter: Option<u64>, id: &'a OpId) -> Self {
        OpEntry { counter, id }
    }
}

/// Order two ops: descending counter, then ascending op id.
///
/// [`Ordering::Less`] means "orders first", so sorting a slice with this puts
/// the leading op at the front.
///
/// **"Newest first", read as latest in the forum's own order** — and never "most
/// recent first", which is a claim about time this comparison does not verify.
/// The counter is pegged to its author's clock ([`next_counter`]), so it carries
/// that author's claim about the time, raised above every counter the author
/// held. It guarantees that a reply written after its author held another op
/// orders ahead of it in this rule. Between two ops neither author had seen, it
/// orders by two unverified clock readings — an author may sign up to
/// [`RECEIVE_WINDOW_MS`] ahead of a receiver's time, and a slow clock signs
/// behind. A reader who expects the sequence to track wall-clock time will find
/// it does not, and the interface must not suggest otherwise.
///
/// # The tiebreak is the op id, and not the transport's message id
///
/// An op id is a function of the op's own bytes, so every peer holding the op
/// computes the same one without consulting anything it received. The
/// transport's message id is assigned by the transport, does not reach this
/// system on any op, and would therefore be a tiebreak absent on every op —
/// which is not a tiebreak.
///
/// # PRECONDITION: the op ids compared must be distinct
///
/// **The caller must deduplicate by [`OpId`] before sorting.** This is a
/// contract on callers, not an implementation detail, because violating it
/// reintroduces precisely the failure this module exists to prevent.
///
/// The order is total over *distinct ops*. Two entries sharing one op id compare
/// [`Ordering::Equal`] — the op id is the last resort in every branch, so once it
/// ties there is nothing left to separate them. A slice containing both would
/// then sort into an order decided by the sort's stability and the input
/// sequence, and the input sequence is arrival order, which differs per peer.
///
/// **Both clock fields are inside the preimage, so one op cannot arrive with two
/// different counters** — that would be two ops with two ids. This is a stronger
/// position than the prior design, where the recorded arrival could genuinely
/// differ between two receipts of one op and a store had to choose between them.
///
/// §3.1 makes ops "idempotent by `opId`", so a store holding one entry per op id
/// satisfies the precondition by construction. The op log is structurally
/// immune: its entries live in a map keyed by [`OpId`], so a read iterates values
/// distinct by construction with no intermediate list that could be sorted
/// pre-dedup.
///
/// # Every input travels inside the ops, which is the whole safety property
///
/// It reads no clock, no arrival record and no ambient state — and it **cannot**,
/// because [`OpEntry`] carries neither. Two peers holding the same two ops
/// compute the same order from the ops alone. The prior design could not say
/// that: it depended on recorded arrival metadata, which is per-peer by
/// construction.
///
/// # Why the result is transitive
///
/// Counter-presence partitions any population into two blocks: every op carrying
/// a counter precedes every op that does not, and each block is independently
/// totally ordered. A partition into two totally-ordered blocks with a uniform
/// rule between them is transitive by construction — there is no boundary for a
/// mixed comparator to break on, because the boundary rule consults nothing but
/// presence.
///
/// # The degraded case, which is this change's migration answer
///
/// An op carrying **no** counter sorts after every op that carries one, whatever
/// the values involved. An op carries no counter exactly when it was encoded
/// under the version predating the clock fields, so the population this governs
/// is the ops that already exist.
///
/// Two such ops fall back to **ascending** [`OpId`] — lowest first — exactly as
/// they did before this change, so **already-stored content is neither reordered
/// among itself nor dropped**. Nothing needs rewriting, no stored op changes, and
/// no op is discarded for lacking a field its author's build could not have
/// written.
///
/// **Below rather than above**, because an op carrying a counter is better
/// evidence than one about which nothing is known. The alternative would head
/// every Stoa's feed permanently with its oldest content, and a post's current
/// version could never advance past a revision predating the change.
///
/// This remains a defined degraded order and not an approximation of a temporal
/// one, which it cannot be: an op id is a hash and carries no recency whatever.
/// The only property it buys is convergence — two peers agreeing on an arbitrary
/// order render consistently, and two peers disagreeing cannot be reasoned about
/// at all.
pub fn cmp_ops(a: OpEntry<'_>, b: OpEntry<'_>) -> Ordering {
    match (a.counter, b.counter) {
        // Both carry one: descending counter, so `b` leads the comparison.
        (Some(a_counter), Some(b_counter)) => b_counter
            .cmp(&a_counter)
            // A last resort on op id, so the order stays TOTAL even when two
            // ops share a counter. Without it, two distinct ops could compare
            // Equal and a sort would leave their relative order to chance —
            // which differs per peer, reintroducing exactly the divergence this
            // module prevents.
            .then_with(|| a.id.cmp(b.id)),

        // One carries a counter and one does not: the one that does leads,
        // whatever its value — including a counter of zero.
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,

        // Neither: the defined degraded order, unchanged from before this
        // change so that existing ops keep the relative order they already had.
        (None, None) => a.id.cmp(b.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SecretKey;
    use crate::op::{OpClock, OpKind};

    /// An op with no clock — the shape that predates the clock fields.
    fn an_op(body: &str) -> Op {
        Op {
            stoa: crate::identity::stoa_address(b"a genesis record"),
            author: SecretKey::from_bytes(&[2u8; 32]).unwrap().public_key(),
            clock: None,
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
    }

    /// An op carrying a counter, with a fixed wall-clock.
    ///
    /// The wall-clock is the SAME on every op this fixture builds, deliberately:
    /// a test about ordering must not be able to pass because two ops differed
    /// in a field the order is required not to read. Where a test needs two
    /// wall-clocks it says so and builds them itself.
    fn an_op_at(body: &str, counter: u64) -> Op {
        Op {
            clock: Some(OpClock {
                counter,
                asserted_ms: 1_789_729_304_000,
            }),
            ..an_op(body)
        }
    }

    fn entry<'a>(op: &'a Op, id: &'a OpId) -> OpEntry<'a> {
        OpEntry::of(op, id)
    }

    // ─── The fixture guard that makes the ordering tests mean something ───
    //
    // THE recorded defect family in this project is "a fixture where two
    // explanations give the same answer". For an ordering test the two
    // explanations are "ordered by counter" and "ordered by op id", and a
    // fixture in which the higher counter also has the lower id passes under
    // both — so it proves nothing about which rule the code implements.
    //
    // Every ordering assertion below draws its ops from a fixture that asserts
    // the two DISAGREE.

    /// Two ops whose counters and op ids rank them OPPOSITELY.
    ///
    /// Returns `(higher_counter, lower_counter)` where the higher-counter op
    /// also has the HIGHER op id — so ordering by counter puts the first
    /// element first, and ordering by op id would put it second. A test using
    /// this pair fails if the comparator falls back to the id.
    ///
    /// Searched rather than hardcoded, because which of two bodies hashes lower
    /// is not something a reader should take on trust, and a hardcoded guess
    /// that went stale would silently restore the coincidence this exists to
    /// prevent.
    fn a_pair_whose_counter_and_id_disagree() -> (Op, Op) {
        for n in 0..1000u32 {
            let high = an_op_at(&format!("high {n}"), 9);
            let low = an_op_at(&format!("low {n}"), 4);
            if high.id() > low.id() {
                return (high, low);
            }
        }
        panic!("no disagreeing pair in 1000 candidates, which is astronomically unlikely");
    }

    /// Two ops with distinct ids, lower first. Both carry no counter.
    fn two_ops_by_ascending_id() -> (Op, Op) {
        let (one, two) = (an_op("alpha"), an_op("beta"));
        assert_ne!(one.id(), two.id(), "the fixture needs two distinct ops");
        if one.id() < two.id() {
            (one, two)
        } else {
            (two, one)
        }
    }

    #[test]
    fn the_disagreeing_fixture_really_disagrees() {
        // The guard on the guard. If this stops holding, every ordering test
        // below becomes one that two explanations satisfy, and none of them
        // would fail to say so.
        let (high, low) = a_pair_whose_counter_and_id_disagree();
        assert!(
            high.clock.unwrap().counter > low.clock.unwrap().counter,
            "the first op must carry the higher counter"
        );
        assert!(
            high.id() > low.id(),
            "and the HIGHER op id, so that ordering by id would reverse the pair"
        );
    }

    // ─── Ordering by the op's own counter ────────────────────────────────

    #[test]
    fn a_higher_counter_orders_first() {
        let (high, low) = a_pair_whose_counter_and_id_disagree();
        assert_eq!(
            cmp_ops(entry(&high, &high.id()), entry(&low, &low.id())),
            Ordering::Less,
            "the higher counter leads, and the op ids say the opposite"
        );
        assert_eq!(
            cmp_ops(entry(&low, &low.id()), entry(&high, &high.id())),
            Ordering::Greater
        );
    }

    #[test]
    fn equal_counters_are_broken_by_ascending_op_id() {
        let (low_id, high_id) = two_ops_by_ascending_id();
        let a = Op {
            clock: Some(OpClock {
                counter: 7,
                asserted_ms: 1,
            }),
            ..low_id
        };
        let b = Op {
            clock: Some(OpClock {
                counter: 7,
                asserted_ms: 1,
            }),
            ..high_id
        };
        assert!(a.id() < b.id(), "the fixture's ids must rank this way");
        assert_eq!(
            cmp_ops(entry(&a, &a.id()), entry(&b, &b.id())),
            Ordering::Less,
            "within one counter, the lower op id leads"
        );
    }

    #[test]
    fn the_wall_clock_confers_no_position() {
        // Two ops sharing a counter and differing ONLY in wall-clock, one
        // centuries in the future. The order must be whatever their op ids give
        // — identical to the order they would have with the same wall-clock.
        let (a_base, b_base) = two_ops_by_ascending_id();

        let with_same = |op: &Op| Op {
            clock: Some(OpClock {
                counter: 5,
                asserted_ms: 1_000,
            }),
            ..op.clone()
        };
        let a_future = Op {
            clock: Some(OpClock {
                counter: 5,
                // Centuries ahead.
                asserted_ms: u64::MAX / 2,
            }),
            ..a_base.clone()
        };
        let b_same = with_same(&b_base);

        // The bodies differ, so the ids differ — but the ids are NOT what this
        // test varies. It varies the wall-clock and asserts the answer does not
        // move.
        let a_same = with_same(&a_base);
        let baseline = cmp_ops(entry(&a_same, &a_same.id()), entry(&b_same, &b_same.id()));
        let with_future = cmp_ops(
            entry(&a_future, &a_future.id()),
            entry(&b_same, &b_same.id()),
        );
        assert_eq!(
            baseline, with_future,
            "a wall-clock centuries in the future must confer no position"
        );
    }

    #[test]
    fn a_far_future_wall_clock_does_not_reach_the_head_of_the_order() {
        // The attack stated directly: a low counter and an absurd asserted time
        // must order BELOW a higher counter with an ordinary one.
        let future = Op {
            clock: Some(OpClock {
                counter: 1,
                asserted_ms: u64::MAX,
            }),
            ..an_op("the forger")
        };
        let honest = an_op_at("the honest post", 100);
        assert_eq!(
            cmp_ops(entry(&honest, &honest.id()), entry(&future, &future.id())),
            Ordering::Less,
            "the counter decides; the asserted time buys nothing"
        );
    }

    // ─── The degraded order, which is the migration answer ───────────────

    #[test]
    fn an_op_carrying_a_counter_leads_one_carrying_none() {
        // AT COUNTER ZERO, deliberately. Zero is where a sentinel-based design
        // breaks: any scheme that encoded "no counter" as some in-band value
        // collides with a real counter, and zero is the value most likely to be
        // chosen for it.
        let ordered = an_op_at("ordered", 0);
        let unordered = an_op("unordered");
        assert_eq!(
            cmp_ops(
                entry(&ordered, &ordered.id()),
                entry(&unordered, &unordered.id())
            ),
            Ordering::Less,
            "a counter of zero still beats no counter at all"
        );
        assert_eq!(
            cmp_ops(
                entry(&unordered, &unordered.id()),
                entry(&ordered, &ordered.id())
            ),
            Ordering::Greater
        );
    }

    #[test]
    fn two_ops_carrying_no_counter_are_ordered_by_ascending_op_id() {
        // The order that already existed, unchanged — which is what makes the
        // migration "nothing is reordered among itself".
        let (low, high) = two_ops_by_ascending_id();
        assert_eq!(
            cmp_ops(entry(&low, &low.id()), entry(&high, &high.id())),
            Ordering::Less
        );
    }

    // ─── Totality, antisymmetry, transitivity ────────────────────────────

    #[test]
    fn the_order_is_total_over_distinct_ops() {
        // Every combination of the two axes the comparator reads, over DISTINCT
        // ops. Any pair comparing `Equal` would leave a sort's output to its
        // stability, which is arrival order, which differs per peer.
        let population = a_population();
        for (i, (a_op, a_id)) in population.iter().enumerate() {
            for (j, (b_op, b_id)) in population.iter().enumerate() {
                if i == j {
                    continue;
                }
                assert_ne!(
                    cmp_ops(entry(a_op, a_id), entry(b_op, b_id)),
                    Ordering::Equal,
                    "two distinct ops must never tie"
                );
            }
        }
    }

    #[test]
    fn the_order_is_antisymmetric() {
        let population = a_population();
        for (a_op, a_id) in &population {
            for (b_op, b_id) in &population {
                let forward = cmp_ops(entry(a_op, a_id), entry(b_op, b_id));
                let backward = cmp_ops(entry(b_op, b_id), entry(a_op, a_id));
                assert_eq!(forward, backward.reverse(), "cmp must be antisymmetric");
            }
        }
    }

    #[test]
    fn the_order_is_transitive() {
        let population = a_population();
        for (a_op, a_id) in &population {
            for (b_op, b_id) in &population {
                if cmp_ops(entry(a_op, a_id), entry(b_op, b_id)) != Ordering::Less {
                    continue;
                }
                for (c_op, c_id) in &population {
                    if cmp_ops(entry(b_op, b_id), entry(c_op, c_id)) != Ordering::Less {
                        continue;
                    }
                    assert_eq!(
                        cmp_ops(entry(a_op, a_id), entry(c_op, c_id)),
                        Ordering::Less,
                        "a < b and b < c must give a < c"
                    );
                }
            }
        }
    }

    #[test]
    fn one_op_compared_with_itself_is_equal() {
        // Reflexivity. An arm returning anything else here breaks `Ord`'s
        // contract outright and makes a sort's behaviour undefined.
        let op = an_op_at("x", 3);
        assert_eq!(
            cmp_ops(entry(&op, &op.id()), entry(&op, &op.id())),
            Ordering::Equal
        );
    }

    /// A population crossing every branch of [`cmp_ops`], over distinct ops.
    ///
    /// Counters at both boundaries and in the middle, plus ops carrying none —
    /// so the ordered/unordered partition, the counter comparison and the op-id
    /// tiebreak are all exercised.
    fn a_population() -> Vec<(Op, OpId)> {
        let mut out = Vec::new();
        for (label, counter) in [
            ("zero", Some(0u64)),
            ("one", Some(1)),
            ("middle", Some(1_000_000)),
            ("max less one", Some(u64::MAX - 1)),
            ("max", Some(u64::MAX)),
            ("none", None),
            ("none too", None),
        ] {
            let op = match counter {
                Some(c) => an_op_at(label, c),
                None => an_op(label),
            };
            let id = op.id();
            out.push((op, id));
        }
        // Two ops sharing one counter, so the op-id tiebreak is reached.
        for label in ["tied one", "tied two"] {
            let op = an_op_at(label, 42);
            let id = op.id();
            out.push((op, id));
        }

        let mut ids: Vec<OpId> = out.iter().map(|(_, id)| *id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "the population must hold distinct ops");
        out
    }

    // ─── The clock, derived from the ops held ────────────────────────────

    #[test]
    fn a_peer_holding_no_ops_has_a_zero_clock() {
        assert_eq!(clock_from_counters(std::iter::empty()), 0);
    }

    #[test]
    fn the_clock_is_the_highest_counter_it_accepted() {
        assert_eq!(clock_from_counters([1, 5, 3, 2]), 5);
    }

    #[test]
    fn the_clock_is_the_highest_counter_however_far_apart_the_counters_are() {
        // The case the advance bound used to decide, and now does not: a gap of
        // a million and more, and the maximum representable value. A fold that
        // kept any bound on the step returns something below the maximum here.
        assert_eq!(clock_from_counters([1, 2_000_001, 3]), 2_000_001);
        assert_eq!(
            clock_from_counters([1, 2, 3, u64::MAX]),
            u64::MAX,
            "the clock excludes no counter the peer holds"
        );
    }

    #[test]
    fn the_clock_does_not_depend_on_the_order_the_counters_are_given_in() {
        let counters = [1u64, 2, 3, 1_789_729_304_000, 4, 5];
        let forward = clock_from_counters(counters);
        let mut reversed = counters;
        reversed.reverse();
        assert_eq!(forward, clock_from_counters(reversed));
        assert_eq!(
            forward,
            clock_from_counters([1_789_729_304_000, 1, 2, 3, 4, 5]),
            "the clock must be a function of the SET, not of the sequence"
        );
        assert_eq!(forward, 1_789_729_304_000);
    }

    #[test]
    fn a_lower_counter_never_moves_the_clock_backwards() {
        assert_eq!(clock_from_counters([9, 1, 2]), 9);
    }

    #[test]
    fn duplicate_counters_do_not_advance_the_clock_twice() {
        // Two ops at one counter are two ops, not two advances. A fold that
        // added rather than assigned would pass every test above and fail here.
        assert_eq!(clock_from_counters([5, 5, 5]), 5);
    }

    // ─── Publishing ─────────────────────────────────────────────────────

    /// A peer's current time for the publish tests: 2026-09-18T11:01:44Z.
    const NOW: u64 = 1_789_729_304_000;

    #[test]
    fn a_first_op_in_a_stoa_carries_the_current_time() {
        assert_eq!(
            next_counter(clock_from_counters(std::iter::empty()), NOW),
            NOW
        );
    }

    #[test]
    fn a_clock_behind_the_current_time_yields_the_current_time() {
        // Behind by more than one, so "one above the clock" and "the time" are
        // different answers and only the time is right.
        let clock = NOW - 500;
        assert_eq!(next_counter(clock, NOW), NOW);
        assert_ne!(next_counter(clock, NOW), clock + 1);
    }

    #[test]
    fn a_clock_at_or_ahead_of_the_current_time_yields_one_above_it() {
        // AT the time: one above wins by exactly one, which is the edge a `>`
        // written for `>=` would get wrong.
        assert_eq!(next_counter(NOW, NOW), NOW + 1);
        // Ahead of it, within the hour the window allows a held op to lead by.
        assert_eq!(next_counter(NOW + 1_000, NOW), NOW + 1_001);
    }

    #[test]
    fn a_clock_one_below_the_time_yields_the_time_by_either_rule() {
        // The one input where both halves of the `max` agree. Pinned so that
        // the tests above are known to sit on either side of it rather than on
        // it.
        assert_eq!(next_counter(NOW - 1, NOW), NOW);
    }

    #[test]
    fn publishing_after_receiving_advances_past_what_was_received() {
        // A received op ahead of this peer's time: the clock term decides, and
        // the answer is above what was received rather than at the time.
        let received = NOW + 60_000;
        assert_eq!(
            next_counter(clock_from_counters([received]), NOW),
            received + 1
        );
    }

    #[test]
    fn two_publishes_at_one_instant_take_successive_counters() {
        // The same current time at both publishes, with the first publish's
        // counter held between them. The second must still be greater, which
        // only the clock term can give.
        let first = next_counter(clock_from_counters(std::iter::empty()), NOW);
        let second = next_counter(clock_from_counters([first]), NOW);
        assert!(second > first, "{second} must exceed {first}");
    }

    #[test]
    fn the_next_counter_saturates_rather_than_wrapping() {
        // Wrapping would place the highest op below the lowest, inverting the
        // order for every op in the Stoa at once.
        assert_eq!(next_counter(u64::MAX, NOW), u64::MAX);
        assert_eq!(next_counter(u64::MAX, 0), u64::MAX);
        assert_eq!(next_counter(u64::MAX, u64::MAX), u64::MAX);
    }

    // ─── The receive window ─────────────────────────────────────────────

    #[test]
    fn a_counter_exactly_one_hour_ahead_is_within_the_window() {
        // Hardcoded 3,600,000 rather than the constant, so this sits on the
        // spec's edge even if the constant drifts.
        assert!(!exceeds_receive_window(NOW + 3_600_000, NOW));
    }

    #[test]
    fn a_counter_one_millisecond_beyond_the_window_exceeds_it() {
        assert!(exceeds_receive_window(NOW + 3_600_001, NOW));
    }

    #[test]
    fn a_counter_at_or_below_the_time_never_exceeds_the_window() {
        // No lower bound, however far in the past.
        for counter in [0, 1, NOW - 86_400_000, NOW - 1, NOW] {
            assert!(
                !exceeds_receive_window(counter, NOW),
                "counter {counter} is not ahead of {NOW}"
            );
        }
    }

    #[test]
    fn extreme_values_do_not_abort_the_window_check() {
        // Tests run in a debug build, where an overflowing `+` panics — so a
        // `counter > now_ms + RECEIVE_WINDOW_MS` implementation fails the
        // second case here by aborting rather than by an assertion.
        assert!(
            exceeds_receive_window(u64::MAX, 0),
            "the maximum is refused"
        );
        assert!(
            !exceeds_receive_window(0, u64::MAX),
            "zero at the maximal time is admitted"
        );
        assert!(!exceeds_receive_window(u64::MAX, u64::MAX));
        assert!(!exceeds_receive_window(u64::MAX, u64::MAX - 3_600_000));
        assert!(exceeds_receive_window(u64::MAX, u64::MAX - 3_600_001));
    }

    #[test]
    fn the_receive_window_is_pinned_to_one_hour() {
        // `cargo mutants` does not mutate a `const`, so a drifted value here
        // would be invisible to it. Written independently of the constant's own
        // `60 * 60 * 1000`.
        assert_eq!(RECEIVE_WINDOW_MS, 3_600_000);
    }
}
