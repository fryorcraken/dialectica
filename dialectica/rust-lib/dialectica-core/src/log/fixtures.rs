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

/// A post carrying NO counter — the shape that predates the clock fields.
///
/// The default for fixtures that are not about ordering, because it is the
/// population whose ids and degraded order this change must leave untouched.
pub fn a_post(body: &str) -> Op {
    Op {
        stoa: a_stoa("Agora"),
        author: a_key(2).public_key(),
        clock: None,
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: vec![],
        },
    }
}

/// A post carrying a counter, with a fixed wall-clock.
///
/// **The wall-clock is the same on every op this builds**, so an ordering test
/// drawing from it cannot pass because two ops happened to differ in a field the
/// order is required not to read. A test that needs two wall-clocks says so.
pub fn a_post_at(body: &str, counter: u64) -> Op {
    Op {
        clock: Some(crate::op::OpClock {
            counter,
            asserted_ms: 1_789_729_304_000,
        }),
        ..a_post(body)
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

/// How many leading bytes a prefix-confusion fixture must share.
///
/// **The number is the test's strength**, so it is a named constant rather
/// than a literal buried in an assertion, and it is asserted rather than
/// assumed.
///
/// A review found a **2-byte** prefix match on `iter_stoa` and an **8-byte**
/// match on `iter_target` passing the entire suite, because the fixtures they
/// faced pinned only byte 0 — a hash coincidence that two short titles happened
/// to produce. Any query comparing fewer than this many bytes now fails.
///
/// 16 rather than 31: it is comfortably past any plausible accidental
/// truncation (a `substr(.., 8)`, a `u64` read of the first 8 bytes, a
/// hex-prefix comparison) while leaving 16 bytes for the divergence to be
/// unmistakable. A fixture agreeing on 31 of 32 bytes would test the same
/// property and read as a puzzle.
pub const SHARED_PREFIX_BYTES: usize = 16;

/// Two distinct 32-byte keys agreeing on the first [`SHARED_PREFIX_BYTES`].
///
/// **Constructed, not hunted.** A hash collision this long is not findable by
/// trying titles, and the earlier fixtures' "pick two names and assert byte 0
/// matches" pinned the luck they had rather than the property the test needs.
///
/// Nothing in the log requires a Stoa address or an op id to be hash-derived:
/// §4.5 makes the address an opaque key to the store, and §3.3 makes a target
/// naming an op this peer never received entirely ordinary. So a synthetic key
/// is a valid fixture, and it is the only way to state the prefix length as a
/// requirement rather than as an observation.
fn two_keys_sharing_a_long_prefix() -> ([u8; 32], [u8; 32]) {
    let mut one = [0xA5u8; 32];
    let mut two = [0xA5u8; 32];
    // Diverge at exactly the first byte past the shared prefix, so the
    // fixture's guarantee is "agrees on N, differs at N" rather than "agrees
    // on at least N somewhere".
    one[SHARED_PREFIX_BYTES] = 0x01;
    two[SHARED_PREFIX_BYTES] = 0x02;

    // FIXTURE GUARDS, and unlike the ones they replace these pin a property
    // rather than a coincidence: the two keys agree on every byte of the
    // prefix, AND differ at the byte immediately after it.
    assert_eq!(
        one[..SHARED_PREFIX_BYTES],
        two[..SHARED_PREFIX_BYTES],
        "the fixture must share its whole declared prefix"
    );
    assert_ne!(
        one[SHARED_PREFIX_BYTES], two[SHARED_PREFIX_BYTES],
        "the fixture must diverge immediately after the prefix"
    );
    (one, two)
}

/// Two Stoa addresses agreeing on their first [`SHARED_PREFIX_BYTES`] bytes.
pub fn two_addresses_sharing_a_long_prefix() -> (Address, Address) {
    let (one, two) = two_keys_sharing_a_long_prefix();
    (Address::from_bytes(one), Address::from_bytes(two))
}

/// Two op ids agreeing on their first [`SHARED_PREFIX_BYTES`] bytes.
pub fn two_op_ids_sharing_a_long_prefix() -> (OpId, OpId) {
    let (one, two) = two_keys_sharing_a_long_prefix();
    let hex = |bytes: [u8; 32]| {
        OpId::from_hex(&hex::encode(bytes)).expect("32 bytes of hex is always a valid op id")
    };
    (hex(one), hex(two))
}

/// A population crossing every branch of `cmp_ops`, over distinct ops.
///
/// **This is the fixture the two-implementation agreement test turns on**, so
/// its coverage IS the coverage of the claim "the SQL `ORDER BY` is `cmp_ops`".
/// Every shape the comparator distinguishes is present:
///
/// - ops carrying **no counter** — the degraded branch, and the population that
///   already exists;
/// - counters at `0`, `1`, `u64::MAX`, `u64::MAX - 1`, and `i64::MAX as u64` —
///   the boundaries, where a sort key that overflows, clamps or mis-signs shows
///   up and nowhere else. `i64::MAX as u64` is specifically the counter whose
///   sort key collides with the filler written for an op carrying none;
/// - **several ops sharing one counter**, forcing the op-id last resort, which
///   is the branch a sort key that stopped at the counter would leave
///   undefined;
/// - ops whose **counter order and op-id order disagree**, which is what makes
///   the agreement test able to fail. See below.
///
/// # Why the arrivals vary independently of the counters
///
/// Each op is paired with an arrival, and **the arrivals are deliberately
/// uncorrelated with the counters** — including ops carrying a high counter
/// whose arrival records nothing, and ops carrying no counter whose arrival
/// records a Lamport value and a message id.
///
/// That is the whole of what makes this fixture test "ordering does not consult
/// the transport". A fixture whose arrival always agreed with the op's counter
/// would pass whether the `ORDER BY` read `sort_counter` or `arrival_lamport`.
/// These disagree, so a query that reached for the arrival produces a different
/// sequence from `cmp_ops` and the agreement test says so.
///
/// # The counter/op-id disagreement
///
/// THE recorded defect family here is "a fixture where two explanations give the
/// same answer". For ordering, the two explanations are "by counter" and "by op
/// id". Bodies are searched rather than hardcoded so that at least one pair
/// ranks oppositely under the two rules — asserted by the fixture guard at the
/// end, so a future edit that accidentally restored the coincidence fails loudly
/// here instead of quietly weakening every test downstream.
///
/// One op per shape, all distinct, because two entries sharing an op id tie
/// under `cmp_ops` by design. A fixture that built such a pair would be
/// exercising the precondition rather than the order, and any disagreement it
/// produced would be an artefact rather than a finding.
pub fn every_ordering_shape() -> Vec<(SignedOp, Arrival)> {
    // (counter, arrival) — the two varied INDEPENDENTLY, on purpose.
    let shapes: Vec<(Option<u64>, Arrival)> = vec![
        // Carrying no counter, with every arrival shape. The arrival's Lamport
        // value and message id must change nothing about where these land.
        (None, Arrival::unordered()),
        (None, Arrival::from_parts(None, Some(a_message_id(1)))),
        (
            None,
            Arrival::from_parts(None, Some(MessageId::new(vec![]))),
        ),
        // An op carrying NO counter whose arrival records a HIGH Lamport value.
        // Under the old rule this led the whole population; under the new one it
        // sorts below every op that carries a counter. Nothing distinguishes the
        // two implementations better than this row.
        (None, Arrival::ordered(u64::MAX, a_message_id(9))),
        (None, Arrival::ordered(5, a_message_id(2))),
        // Carrying counters, with arrivals that say something else entirely.
        (Some(0), Arrival::unordered()),
        (Some(0), Arrival::ordered(u64::MAX, a_message_id(1))),
        (Some(1), Arrival::from_parts(Some(9_999), None)),
        (Some(5), Arrival::unordered()),
        (Some(6), Arrival::from_parts(None, Some(a_message_id(4)))),
        // The sign boundary of the `i64` the sort key casts into, and the one
        // counter whose key collides with the filler written for "no counter".
        (Some(i64::MAX as u64), Arrival::unordered()),
        (Some(u64::MAX - 1), Arrival::ordered(0, a_message_id(1))),
        (Some(u64::MAX), Arrival::unordered()),
        // FOUR ops sharing one counter, so only the op-id last resort separates
        // them. Four rather than two: with two, a comparison that got the
        // direction wrong has a 50% chance of agreeing by luck on any given run.
        (Some(7), Arrival::unordered()),
        (Some(7), Arrival::ordered(1, a_message_id(1))),
        (Some(7), Arrival::ordered(2, MessageId::new(vec![]))),
        (Some(7), Arrival::from_parts(None, Some(a_message_id(8)))),
    ];

    let out: Vec<(SignedOp, Arrival)> = shapes
        .into_iter()
        .enumerate()
        .map(|(i, (counter, arrival))| {
            let body = format!("shape {i}");
            let op = match counter {
                Some(c) => a_post_at(&body, c),
                None => a_post(&body),
            };
            (signed(op), arrival)
        })
        .collect();

    // FIXTURE GUARD 1. The ops must be distinct, or `cmp_ops`'s precondition is
    // violated and the comparison is UNDEFINED rather than merely different —
    // which would make a disagreement between the two implementations an
    // artefact of this fixture rather than a finding about either.
    let mut ids: Vec<OpId> = out.iter().map(|(op, _)| op.op.id()).collect();
    let before = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(
        before,
        ids.len(),
        "the ordering fixture must be distinct ops"
    );

    // FIXTURE GUARD 2. At least one pair must rank OPPOSITELY under "by
    // counter" and "by op id". Without such a pair every ordering assertion
    // built on this fixture is satisfied by both explanations, and the suite
    // would report green over an implementation that ignored the counter
    // entirely.
    let disagreeing = out.iter().any(|(a, _)| {
        out.iter().any(|(b, _)| match (a.op.clock, b.op.clock) {
            (Some(a_clock), Some(b_clock)) => {
                a_clock.counter > b_clock.counter && a.op.id() > b.op.id()
            }
            _ => false,
        })
    });
    assert!(
        disagreeing,
        "the fixture must contain a pair whose counter and op id rank oppositely, \
         or an implementation ignoring the counter would pass"
    );

    // FIXTURE GUARD 3. At least one op carrying NO counter must have an arrival
    // recording a HIGHER Lamport value than some op that DOES carry a counter.
    // That is the pair a query reading `arrival_lamport` instead of
    // `sort_counter` gets wrong, and without it the two columns agree.
    let arrival_disagrees = out.iter().any(|(no_counter, arrival)| {
        no_counter.op.clock.is_none()
            && arrival.lamport().is_some_and(|recorded| {
                out.iter().any(|(with_counter, _)| {
                    with_counter.op.clock.is_some_and(|c| c.counter < recorded)
                })
            })
    });
    assert!(
        arrival_disagrees,
        "the fixture must contain an op with no counter whose RECORDED Lamport \
         value exceeds some op's counter, or reading the arrival would pass"
    );

    out
}
