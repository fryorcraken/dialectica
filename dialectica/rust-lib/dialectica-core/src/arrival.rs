//! What a peer records alongside an op, and what orders two ops.
//!
//! # Why this file exists
//!
//! §5.7 orders an author's revisions by "the highest Lamport timestamp [...]
//! ties broken by ascending message id — the same rule SDS already applies
//! (§4.4), so nothing new is invented". Moderation ops on one target are
//! ordered the same way. [`op::Op`] deliberately carries neither value, because
//! a self-asserted Lamport timestamp is forgeable by the very author it exists
//! to order. Something has to hold them; this is it.
//!
//! # There is no ordering algorithm here, and that is the point
//!
//! SDS already supplies the order. Its `Message` carries a `lamport_timestamp`
//! and a `message_id` (LIP-109 §Message), it inserts into its local log "based
//! on Lamport timestamp", and it breaks ties "in ascending order of message ID".
//! That IS §5.7's rule. Nothing in this module invents an order; it records the
//! transport's and compares by it.
//!
//! **The gap is that the values do not reach us**, and it is a layer below the
//! contract §13 named. The Reliable Channel API's `MessageReceivedEvent` — the
//! spec `delivery_module` consumes — carries exactly one field, the reassembled
//! payload. `delivery_module.lidl`'s `channelMessageReceived` cannot forward
//! what it was never given. The change's `design.md` records the finding, the
//! citations, and what each layer would have to add.
//!
//! # Why a second Lamport clock is the one thing not to build
//!
//! It is not merely duplication: **it could not be made to agree.** SDS's clock
//! advances on traffic no application sees as ops — acknowledgements, sync
//! messages, ephemeral messages — and is initialised from epoch-milliseconds so
//! that new joiners order correctly without syncing history. A clock advanced
//! only on op arrivals runs behind it and diverges per peer according to what
//! that peer received and when.
//!
//! Two orders that disagree produce no error. Each peer stays internally
//! consistent and renders the thread differently from its neighbour, which is a
//! bug with no failing assertion anywhere. So there is no constructor here that
//! derives a Lamport value, no counter, and no peer state: the only way a value
//! enters is for the transport to have supplied it.

use crate::op::OpId;
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

    /// Whether the transport placed this op in its order.
    ///
    /// The Lamport timestamp alone decides this. **A message id without a
    /// Lamport timestamp does not order**: the id is a hash and carries no
    /// temporal meaning, so ordering by it alone would produce a stable, total
    /// and entirely arbitrary order that looks exactly like a real one. The
    /// spec is explicit that it is a tiebreak *within* one Lamport value.
    ///
    /// A caller can state this question rather than inferring it from where an
    /// op landed, which is what lets a UI say "ordered by id, not by the
    /// network" instead of presenting a degraded order as authoritative.
    pub fn is_ordered_by_transport(&self) -> bool {
        self.lamport.is_some()
    }
}

/// Order two ops: most recent first.
///
/// §5.7's rule, over the transport's values: descending Lamport timestamp, ties
/// broken by ascending message id. [`Ordering::Less`] means "orders first", so
/// sorting a slice with this puts the current version at the front.
///
/// # A pure function of its arguments, which is the whole safety property
///
/// It reads no clock, no arrival counter and no ambient state, so two peers
/// holding the same inputs cannot produce different outputs. That is what makes
/// the divergence this module exists to prevent unreachable rather than merely
/// unlikely — there is no local input that could differ.
///
/// # The degraded case
///
/// An op the transport did not order sorts **after** every op it did, whatever
/// the values involved: an op the transport placed is better evidence than one
/// it did not, and a post's current version should not be displaced by an op
/// about which nothing is known.
///
/// Two unordered ops fall back to ascending [`OpId`] — chosen because an op id
/// is a function of the op's own bytes, so every peer computes the same one from
/// the op alone. Arrival sequence and the delivery module's `timestamp` were
/// both rejected: they are per-peer, and the second is per-peer while looking
/// shared. The fallback is arbitrary with respect to time, and that is accepted,
/// because the property bought is *convergence* rather than accuracy. Two peers
/// agreeing on an arbitrary order render consistently; two peers disagreeing
/// cannot be reasoned about at all.
pub fn cmp_ops(a: (&Arrival, &OpId), b: (&Arrival, &OpId)) -> Ordering {
    let (a_arrival, a_id) = a;
    let (b_arrival, b_id) = b;

    match (a_arrival.lamport, b_arrival.lamport) {
        // Both ordered: §5.7's rule verbatim.
        (Some(a_lamport), Some(b_lamport)) => b_lamport
            .cmp(&a_lamport)
            // Descending Lamport, so `b` leads the comparison.
            .then_with(|| cmp_tiebreak(a_arrival, b_arrival))
            // A last resort on op id, so the order stays TOTAL even when two
            // ops share a Lamport value and neither carries a message id.
            // Without it, two distinct ops could compare Equal, and a sort
            // would leave their relative order to chance — which differs per
            // peer, reintroducing exactly the divergence this module prevents.
            .then_with(|| a_id.cmp(b_id)),

        // One ordered, one not: the ordered one leads, whatever its value.
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,

        // Neither ordered: the defined degraded order.
        (None, None) => a_id.cmp(b_id),
    }
}

/// The §5.7 tiebreak within one Lamport value: ascending message id.
///
/// Separate because it is a distinct job — "break a tie" is not "compare two
/// ops" — and because it is where the partial case lives. An op carrying a
/// message id is more completely described than one that does not, so it leads;
/// with neither, this says nothing and `cmp_ops` falls through to the op id.
fn cmp_tiebreak(a: &Arrival, b: &Arrival) -> Ordering {
    match (&a.message_id, &b.message_id) {
        (Some(a_id), Some(b_id)) => a_id.cmp(b_id),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SecretKey;
    use crate::op::{Op, OpKind};

    fn an_op(body: &str) -> Op {
        Op {
            stoa: crate::identity::stoa_address(b"a genesis record"),
            author: SecretKey::from_bytes(&[2u8; 32]).unwrap().public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
    }

    fn a_message_id(seed: u8) -> MessageId {
        MessageId::new(vec![seed; 32])
    }

    /// Two ops whose ids are known to differ, with the lower one first.
    ///
    /// Returned in a determined order rather than assumed, because the ids are
    /// hashes: which of two bodies hashes lower is not something a reader of
    /// this test should have to take on trust, and hardcoding the wrong guess
    /// would make a test pass for the wrong reason.
    fn two_ops_by_ascending_id() -> (Op, Op) {
        let (one, two) = (an_op("alpha"), an_op("beta"));
        assert_ne!(one.id(), two.id(), "the fixture needs two distinct ops");
        if one.id() < two.id() {
            (one, two)
        } else {
            (two, one)
        }
    }

    // ─── The transport's order ────────────────────────────────────────────

    #[test]
    fn a_higher_lamport_timestamp_orders_first() {
        // §5.7: "the highest Lamport timestamp is current". Descending, so the
        // current version sorts to the front.
        let op = an_op("x");
        let older = Arrival::ordered(1, a_message_id(1));
        let newer = Arrival::ordered(2, a_message_id(1));
        assert_eq!(cmp_ops((&newer, &op.id()), (&older, &op.id())), Ordering::Less);
        assert_eq!(
            cmp_ops((&older, &op.id()), (&newer, &op.id())),
            Ordering::Greater
        );
    }

    #[test]
    fn the_message_id_does_not_override_the_lamport_timestamp() {
        // The tiebreak must apply only WITHIN a Lamport value. Here the newer op
        // has the HIGHER message id, so a comparison that consulted the id first
        // — or that sorted Lamport ascending — would order them the other way.
        let op = an_op("x");
        let older = Arrival::ordered(1, a_message_id(9));
        let newer = Arrival::ordered(2, a_message_id(1));
        assert_eq!(cmp_ops((&newer, &op.id()), (&older, &op.id())), Ordering::Less);
    }

    #[test]
    fn equal_lamport_timestamps_are_broken_by_ascending_message_id() {
        // §5.7: "ties broken by ascending message id — the same rule SDS
        // already applies".
        let op = an_op("x");
        let low = Arrival::ordered(7, a_message_id(1));
        let high = Arrival::ordered(7, a_message_id(2));
        assert_eq!(cmp_ops((&low, &op.id()), (&high, &op.id())), Ordering::Less);
        assert_eq!(cmp_ops((&high, &op.id()), (&low, &op.id())), Ordering::Greater);
    }

    #[test]
    fn a_message_id_is_compared_by_bytes_not_by_length() {
        // A shorter id is not automatically lower. Pinned because the width is
        // the transport's to choose and a Vec comparison that got this wrong
        // would be invisible while every fixture used equal-length ids.
        let op = an_op("x");
        let short_high = Arrival::ordered(1, MessageId::new(vec![0x02]));
        let long_low = Arrival::ordered(1, MessageId::new(vec![0x01, 0xFF, 0xFF]));
        assert_eq!(
            cmp_ops((&long_low, &op.id()), (&short_high, &op.id())),
            Ordering::Less,
            "0x01FFFF must precede 0x02"
        );
    }

    #[test]
    fn the_order_is_the_same_whichever_way_the_pair_is_presented() {
        // Antisymmetry. A comparison that returned Less both ways would make a
        // sort's result depend on the input order, which differs per peer.
        let op = an_op("x");
        let cases = [
            (Arrival::ordered(1, a_message_id(1)), Arrival::ordered(2, a_message_id(1))),
            (Arrival::ordered(1, a_message_id(1)), Arrival::ordered(1, a_message_id(2))),
            (Arrival::ordered(1, a_message_id(1)), Arrival::unordered()),
            (Arrival::unordered(), Arrival::unordered()),
        ];
        for (a, b) in cases {
            let forward = cmp_ops((&a, &op.id()), (&b, &op.id()));
            let backward = cmp_ops((&b, &op.id()), (&a, &op.id()));
            assert_eq!(forward, backward.reverse(), "comparison is not antisymmetric");
        }
    }

    #[test]
    fn the_order_is_total_over_distinct_ops() {
        // No two DISTINCT ops may compare Equal, or a sort leaves their relative
        // order to chance — and chance differs per peer, which is the silent
        // divergence this module exists to prevent. The hard case is two ops
        // with identical metadata: only the op-id last resort separates them.
        let (low, high) = two_ops_by_ascending_id();
        let same = Arrival::ordered(1, a_message_id(1));
        assert_eq!(
            cmp_ops((&same, &low.id()), (&same, &high.id())),
            Ordering::Less
        );
        let both_unordered = Arrival::unordered();
        assert_eq!(
            cmp_ops((&both_unordered, &low.id()), (&both_unordered, &high.id())),
            Ordering::Less
        );
    }

    #[test]
    fn one_op_compared_against_itself_is_equal() {
        // Reflexivity, the other half of a well-formed ordering.
        let op = an_op("x");
        let arrival = Arrival::ordered(3, a_message_id(4));
        assert_eq!(
            cmp_ops((&arrival, &op.id()), (&arrival, &op.id())),
            Ordering::Equal
        );
    }

    #[test]
    fn sorting_puts_the_current_version_first() {
        // The rule at the level §5.7 states it: given an author's versions of a
        // post, the highest Lamport timestamp is current.
        let op = an_op("x");
        let id = op.id();
        let mut versions = [
            (Arrival::ordered(1, a_message_id(1)), id),
            (Arrival::ordered(3, a_message_id(1)), id),
            (Arrival::ordered(2, a_message_id(1)), id),
        ];
        versions.sort_by(|a, b| cmp_ops((&a.0, &a.1), (&b.0, &b.1)));
        assert_eq!(
            versions.iter().map(|v| v.0.lamport()).collect::<Vec<_>>(),
            vec![Some(3), Some(2), Some(1)]
        );
    }

    // ─── The degraded order ───────────────────────────────────────────────

    #[test]
    fn an_op_the_transport_ordered_beats_one_it_did_not() {
        // Whatever the value: an op the transport placed is better evidence
        // than an op about which nothing is known. Lamport 0 is the sharp case,
        // since a sentinel-based design would have made 0 mean "absent".
        let op = an_op("x");
        let unordered = Arrival::unordered();
        for lamport in [0u64, 1, u64::MAX] {
            let ordered = Arrival::ordered(lamport, a_message_id(1));
            assert_eq!(
                cmp_ops((&ordered, &op.id()), (&unordered, &op.id())),
                Ordering::Less,
                "lamport {lamport} must still beat an unordered op"
            );
        }
    }

    #[test]
    fn two_unordered_ops_are_ordered_by_ascending_op_id() {
        // The defined degraded order. Op id because it is a function of the
        // op's own bytes: every peer computes the same one WITHOUT consulting
        // anything it received, which arrival sequence and the delivery
        // module's local-clock `timestamp` both fail.
        let (low, high) = two_ops_by_ascending_id();
        let unordered = Arrival::unordered();
        assert_eq!(
            cmp_ops((&unordered, &low.id()), (&unordered, &high.id())),
            Ordering::Less
        );
    }

    #[test]
    fn the_degraded_order_does_not_depend_on_the_arrival_metadata() {
        // Two peers that saw the same ops must agree even with nothing from the
        // transport. Pinned by comparing the degraded result against the op ids
        // alone: if anything peer-local leaked into the comparison, these would
        // diverge.
        let (low, high) = two_ops_by_ascending_id();
        let unordered = Arrival::unordered();
        assert_eq!(
            cmp_ops((&unordered, &low.id()), (&unordered, &high.id())),
            low.id().cmp(&high.id())
        );
    }

    // ─── Absence is represented, never fabricated ─────────────────────────

    #[test]
    fn an_unordered_arrival_records_no_values_at_all() {
        // Asserted against hardcoded Nones rather than against whatever the
        // constructor produced. A constructor that quietly defaulted the
        // Lamport value to 0 would be indistinguishable from a real 0 forever
        // after, and every later peer disagreement would trace back here.
        let arrival = Arrival::unordered();
        assert_eq!(arrival.lamport(), None);
        assert_eq!(arrival.message_id(), None);
        assert!(!arrival.is_ordered_by_transport());
    }

    #[test]
    fn absence_is_not_equal_to_a_zero_lamport_timestamp() {
        // The sentinel trap, pinned. `lamport: 0` is a legitimate value SDS
        // could assign; "not supplied" is a different fact. A design that
        // collapsed them could not report which of its ops were genuinely
        // ordered.
        let absent = Arrival::unordered();
        let zero = Arrival::ordered(0, a_message_id(0));
        assert_ne!(absent, zero);
        assert!(!absent.is_ordered_by_transport());
        assert!(zero.is_ordered_by_transport());
    }

    #[test]
    fn an_ordered_arrival_reports_exactly_what_it_was_given() {
        let arrival = Arrival::ordered(42, a_message_id(7));
        assert_eq!(arrival.lamport(), Some(42));
        assert_eq!(arrival.message_id(), Some(&a_message_id(7)));
        assert!(arrival.is_ordered_by_transport());
    }

    #[test]
    fn a_message_id_without_a_lamport_timestamp_does_not_order() {
        // Ordering by message id alone is total, stable, and meaningless: the
        // id is a hash, and the spec makes it a tiebreak WITHIN one Lamport
        // value. An arbitrary order that announces itself is better than one
        // that looks real.
        let op = an_op("x");
        let id_only = Arrival::from_parts(None, Some(a_message_id(1)));
        assert!(!id_only.is_ordered_by_transport());

        let ordered = Arrival::ordered(0, a_message_id(9));
        assert_eq!(
            cmp_ops((&ordered, &op.id()), (&id_only, &op.id())),
            Ordering::Less,
            "a message id alone must not confer an order"
        );
    }

    #[test]
    fn two_ops_with_message_ids_but_no_lamport_fall_back_to_op_id() {
        // NO SPEC: the spec says such ops are "unordered by the transport" and
        // that unordered ops order by op id; it does not say whether their
        // message ids may break that tie. Chosen: they may not — the fallback
        // is op id alone, so the degraded order is derived entirely from the
        // ops themselves and never partly from transport metadata that was
        // explicitly declared insufficient to order.
        let (low, high) = two_ops_by_ascending_id();
        // The LOW op carries the HIGH message id, so a comparison that used the
        // message id would order them the other way round.
        let low_op_high_msg = Arrival::from_parts(None, Some(a_message_id(9)));
        let high_op_low_msg = Arrival::from_parts(None, Some(a_message_id(1)));
        assert_eq!(
            cmp_ops((&low_op_high_msg, &low.id()), (&high_op_low_msg, &high.id())),
            Ordering::Less
        );
    }

    #[test]
    fn a_lamport_timestamp_without_a_message_id_still_orders() {
        // NO SPEC: the spec requires the Lamport timestamp to decide
        // orderedness and does not say what a missing tiebreak does. Chosen:
        // the op is ordered, and the op id breaks the tie. Dropping to
        // unordered instead would discard a real Lamport value the transport
        // supplied, which is the one thing this module must never do.
        let op = an_op("x");
        let lamport_only = Arrival::from_parts(Some(5), None);
        assert!(lamport_only.is_ordered_by_transport());

        let lower = Arrival::from_parts(Some(4), None);
        assert_eq!(
            cmp_ops((&lamport_only, &op.id()), (&lower, &op.id())),
            Ordering::Less
        );

        // And within one Lamport value, an op WITH a message id is more
        // completely described than one without, so it leads.
        // NO SPEC: the spec is silent on this pairing.
        let with_id = Arrival::ordered(5, a_message_id(1));
        assert_eq!(
            cmp_ops((&with_id, &op.id()), (&lamport_only, &op.id())),
            Ordering::Less
        );
    }

    // ─── What this module structurally cannot do ──────────────────────────

    #[test]
    fn nothing_here_can_produce_a_lamport_timestamp() {
        // THE property that keeps a second clock from existing. Every route to
        // a Lamport value requires a caller to supply one, so a value can only
        // originate at the transport boundary. If a future change adds a
        // constructor that derives one — a counter, a clock read, a "next()" —
        // the two-orders-that-disagree failure becomes reachable, and it fails
        // silently by rendering threads differently on different peers.
        //
        // This is asserted by the only means available to a test: the
        // no-argument constructor yields no order at all.
        assert_eq!(Arrival::unordered().lamport(), None);
        assert!(!Arrival::unordered().is_ordered_by_transport());
    }
}
