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
/// - **message ids of DIFFERING LENGTHS, in prefix relationships** — see below;
/// - Lamport `0`, `u64::MAX`, `u64::MAX - 1`, and `i64::MAX as u64` — the
///   boundaries, where a sort key that overflows, clamps or mis-signs shows up
///   and nowhere else.
///
/// # Why the message ids are not all the same length
///
/// They were — every one was `vec![seed; 32]` or empty — and **a review found
/// that this test could not catch a missing `sort_msg` in the `ORDER BY`.**
/// With equal-length ids, comparing by length and comparing lexicographically
/// give the same answer on every pair, so the branch the column exists for was
/// never exercised.
///
/// `[0x01]`, `[0x01, 0x00]`, `[0x01, 0xFF]`, `[0x02]` are prefix-related and of
/// differing lengths, which is where Rust's derived `Ord` on `Vec<u8>` and
/// SQLite's BLOB `memcmp` could in principle disagree — a shorter id that is a
/// prefix of a longer one sorts first under both, and nothing in this suite
/// established that until these were added. `arrival.rs`'s
/// `a_message_id_is_compared_by_bytes_not_by_length` pins the Rust half; this
/// is what pins that SQLite agrees.
///
/// It is also the first thing a message-id width change would break, and
/// `arrival.rs` records that the width is the transport's to choose and is not
/// settled.
///
/// One op per arrival, all distinct, because two entries sharing an op id tie
/// under `cmp_ops` by design. A fixture that built such a pair would be
/// exercising the precondition rather than the order, and any disagreement it
/// produced would be an artefact rather than a finding.
pub fn every_ordering_shape() -> Vec<(SignedOp, Arrival)> {
    // Prefix-related and of differing lengths. Grouped at one Lamport value so
    // the tiebreak is what separates them and nothing else can.
    let short = MessageId::new(vec![0x01]);
    let longer_zero = MessageId::new(vec![0x01, 0x00]);
    let longer_high = MessageId::new(vec![0x01, 0xFF]);
    let next = MessageId::new(vec![0x02]);

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
        // The sign boundary of the `i64` the sort key casts into, and the one
        // Lamport value whose key collides with the unordered filler.
        Arrival::ordered(i64::MAX as u64, a_message_id(1)),
        // THE LENGTH-VERSUS-LEXICOGRAPHIC GROUP. All at Lamport 4, so the
        // message id is the only thing that can order them:
        //   [0x01] < [0x01, 0x00] < [0x01, 0xFF] < [0x02]
        // A comparison by length would put [0x01] and [0x02] together ahead of
        // the two-byte pair, which is a different sequence.
        Arrival::ordered(4, short),
        Arrival::ordered(4, longer_zero),
        Arrival::ordered(4, longer_high),
        Arrival::ordered(4, next),
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
