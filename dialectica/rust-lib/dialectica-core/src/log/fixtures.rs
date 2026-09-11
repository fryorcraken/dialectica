//! Fixtures for the log's tests.
//!
//! Extracted from `mod.rs`'s test module so a second test module can share them
//! rather than copy them. A second copy would drift from the first, and these
//! are exactly the fixtures whose drift is invisible: `two_posts_by_ascending_id`
//! determines an order rather than asserting one, so a divergent copy would
//! silently stop testing what its callers believe it tests.

use crate::arrival::MessageId;
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
