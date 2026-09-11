//! Fixtures shared by the log's tests and by the two-implementation contract
//! suite.
//!
//! Extracted from `mod.rs`'s test module so a second test module can share them
//! rather than copy them. A second copy would drift from the first, and these
//! are exactly the fixtures whose drift is invisible: `two_posts_by_ascending_id`
//! determines an order rather than asserting one, so a divergent copy would
//! silently stop testing what its callers believe it tests.

use crate::arrival::{Arrival, MessageId};
use crate::identity::{Address, SecretKey};
use crate::op::{ModerationAction, Op, OpId, OpKind, SignedOp, VoteDirection};
use crate::stoa::{Genesis, Policy};

pub fn a_key(seed: u8) -> SecretKey {
    SecretKey::from_bytes(&[seed; 32]).unwrap()
}

pub fn a_stoa(title: &str) -> Address {
    Genesis {
        creator: a_key(1).public_key(),
        policy: Policy::Open,
        title: title.to_string(),
    }
    .address()
    // Infallible for this fixture: `address` fails only on a title over
    // MAX_TITLE_BYTES (1 KiB), and every caller here passes a short literal.
    // An `expect` rather than a silent fallback so that a future fixture
    // with a long title fails loudly here instead of producing an address
    // derived from something other than what it named.
    .expect("a short fixture title is always under the genesis title cap")
}

pub fn a_post(body: &str) -> Op {
    Op {
        stoa: a_stoa("Agora"),
        author: a_key(2).public_key(),
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: vec![],
        },
    }
}

pub fn signed(op: Op) -> SignedOp {
    op.sign(&a_key(2))
}

pub fn a_message_id(seed: u8) -> MessageId {
    MessageId::new(vec![seed; 32])
}

/// One op kind of each variant, with adversarial field values.
///
/// Shared by the panic test and the no-filtering test because both need
/// "every kind, awkwardly shaped" and a second copy would drift from the
/// first.
///
/// **Must list every `OpKind` variant.** `nothing_is_filtered_on_the_way_in`
/// counts what this returns, so a kind missing here is a kind neither test
/// covers — and nothing fails. `Entry::target`'s exhaustive match is what
/// makes a new variant visible; this list is the part that must then be
/// updated by hand.
pub fn every_op_kind() -> Vec<OpKind> {
    vec![
        OpKind::Post {
            thread: None,
            parent: None,
            body: String::new(),
            attachments: vec![],
        },
        OpKind::Post {
            thread: Some(OpId::from_hex(&"00".repeat(32)).unwrap()),
            parent: Some(OpId::from_hex(&"ff".repeat(32)).unwrap()),
            body: "x".repeat(64 * 1024),
            attachments: vec![String::new(); 64],
        },
        OpKind::Revise {
            target: OpId::from_hex(&"00".repeat(32)).unwrap(),
            body: "\u{0}\u{feff}🏛".to_string(),
            attachments: vec![],
        },
        OpKind::Moderate {
            target: OpId::from_hex(&"ff".repeat(32)).unwrap(),
            action: ModerationAction::Unhide,
        },
        OpKind::Vote {
            target: OpId::from_hex(&"ab".repeat(32)).unwrap(),
            direction: VoteDirection::Down,
        },
        OpKind::StoaMetadata {
            title: String::new(),
            description: "\u{0}🏛".to_string(),
        },
    ]
}

/// Two posts whose ids are known to differ, lower id first.
///
/// Determined rather than assumed, for the reason `arrival.rs`'s equivalent
/// fixture gives: the ids are hashes, and hardcoding the wrong guess would
/// make a degraded-order test pass for the wrong reason.
pub fn two_posts_by_ascending_id() -> (SignedOp, SignedOp) {
    let (one, two) = (signed(a_post("alpha")), signed(a_post("beta")));
    assert_ne!(
        one.op.id(),
        two.op.id(),
        "the fixture needs two distinct ops"
    );
    if one.op.id() < two.op.id() {
        (one, two)
    } else {
        (two, one)
    }
}

/// A population crossing every branch of `cmp_ops`, over distinct ops.
///
/// **This is the fixture the two-implementation agreement test turns on**, so
/// its coverage IS the coverage of the claim "the SQL `ORDER BY` is `cmp_ops`".
/// Every arrival shape the comparator distinguishes is present:
///
/// - no metadata at all — the degraded branch, and the only branch production
///   reaches today;
/// - a message id with no Lamport value, which `cmp_ops` treats as unordered
///   and which is the shape an SDS ephemeral message actually produces;
/// - a Lamport value with no message id, which reaches `cmp_tiebreak`'s
///   `(None, None)` arm — a shape neither named constructor can build;
/// - equal Lamport values with differing message ids (the §5.7 tiebreak);
/// - equal Lamport values with EQUAL message ids, forcing the op-id last
///   resort, which is the branch a sort key that stopped at the message id
///   would silently leave undefined;
/// - an EMPTY message id, which is a legal value and must not read as absence;
/// - Lamport `0`, `u64::MAX` and `u64::MAX - 1` — the boundaries, where a sort
///   key that overflows or clamps wrongly shows up and nowhere else.
///
/// One op per arrival, all distinct, because two entries sharing an op id tie
/// under `cmp_ops` by design. A fixture that built such a pair would be
/// exercising the precondition rather than the order, and any disagreement it
/// produced would be an artefact rather than a finding.
pub fn every_ordering_shape() -> Vec<(SignedOp, Arrival)> {
    let arrivals = vec![
        Arrival::unordered(),
        Arrival::from_parts(None, Some(a_message_id(1))),
        Arrival::from_parts(None, Some(a_message_id(9))),
        Arrival::from_parts(None, Some(MessageId::new(vec![]))),
        Arrival::from_parts(Some(5), None),
        Arrival::ordered(5, a_message_id(1)),
        Arrival::ordered(5, a_message_id(9)),
        Arrival::ordered(6, a_message_id(1)),
        Arrival::ordered(0, a_message_id(1)),
        Arrival::ordered(0, MessageId::new(vec![])),
        Arrival::ordered(u64::MAX, a_message_id(1)),
        Arrival::ordered(u64::MAX - 1, a_message_id(1)),
        // A PAIR sharing a Lamport value AND a message id, so only the op-id
        // last resort separates them. Two entries on purpose: one alone
        // exercises nothing.
        Arrival::ordered(7, a_message_id(3)),
        Arrival::ordered(7, a_message_id(3)),
    ];

    let out: Vec<(SignedOp, Arrival)> = arrivals
        .into_iter()
        .enumerate()
        .map(|(i, arrival)| (signed(a_post(&format!("shape {i}"))), arrival))
        .collect();

    // FIXTURE GUARD. The ops must be distinct, or `cmp_ops`'s precondition is
    // violated and the comparison is UNDEFINED rather than merely different —
    // which would make a disagreement between the two implementations an
    // artefact of this fixture rather than a finding about either.
    let mut ids: Vec<OpId> = out.iter().map(|(op, _)| op.op.id()).collect();
    let before = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(before, ids.len(), "the ordering fixture must be distinct ops");

    out
}
