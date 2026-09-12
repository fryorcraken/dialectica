//! The publish path: what a caller supplies, and what must hold of the op.
//!
//! # Why this file exists
//!
//! `wire.rs` exposed a version string, a ping, a panic probe, a capability probe
//! and a delivery bridge — nothing that reached an op, a log or a resolver.
//! `posting-capability` contracts the probe a view asks *before* showing a
//! compose box; nothing contracted what happens when the user presses submit.
//! This is that.
//!
//! # This module decides, and does not parse
//!
//! Everything here takes typed values — an [`Address`], an [`OpId`], a
//! [`SecretKey`] — and never a JSON string. `wire.rs` owns the parse and the one
//! failure shape; the split is `feed.rs`'s and it is not cosmetic. It is what
//! lets "a reply to a parent in another Stoa is refused" be asserted without
//! also asserting how a Stoa address is spelled in JSON.
//!
//! Nor does it open a store or find a key. `dialectica-core` structurally
//! cannot reach the host's persistence path, and a publish that went looking for
//! a key would be doing discovery at a moment its caller does not control —
//! which is also what makes "a publish creates no key material as a side
//! effect" a property nobody has to remember.
//!
//! # The checks here are a THIRD thing, and reading them as anything else gives
//! no protection at all
//!
//! `op.rs` verifies authenticity. `moderation.rs` and `revision.rs` decide
//! authority on read. What this module does is neither: it constrains the ops
//! **this peer creates**.
//!
//! Every op arriving from a peer arrived without passing a single check in this
//! file. So a reader that treated "the publish path refuses a reply to an absent
//! parent" as having established anything about a stored reply would have been
//! given nothing — the log stores exactly such a reply, deliberately
//! (`log::OpLog::append` decides nothing), and `a_reply_whose_parent_is_absent_is_stored`
//! pins that it does. This is the failure `op-format` records as measured in a
//! comparable project, where authority is checked on the send path and never on
//! read.
//!
//! Nothing in this module may be relied upon by any reader, and no function here
//! asserts a property of an op this peer received rather than created.

use crate::arrival::Arrival;
use crate::identity::{Address, SecretKey};
use crate::log::{Appended, OpLog, OpLogError};
use crate::op::{Op, OpId, OpKind, VoteDirection};

/// What a publish did, and what to tell the caller.
///
/// Carries [`Appended`] through **unchanged** rather than recomputing it. An op
/// id is a function of the op's bytes and those bytes carry no timestamp and no
/// nonce, so one identity publishing the same content twice produces one op —
/// and both calls reach a caller as a success naming one op id. The only thing
/// that tells them apart is this field, and `op-log` is the layer that knows the
/// answer. Asking the store with a `get` before the `append` would be two round
/// trips computing a value the store is about to compute anyway, so the two
/// could disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub id: OpId,
    /// Whether the append stored an op the log did not already hold.
    pub appended: Appended,
}

impl Published {
    /// Whether this publish stored something new, for a caller that only wants
    /// the boolean.
    pub fn was_new(&self) -> bool {
        self.appended == Appended::Stored
    }
}

/// Why a publish was refused.
///
/// **A typed enum and not a `String`**, because two requirements turn on
/// specific refusals being told apart: "no such op is held" from "the op held is
/// not a post", and a wrong-typed field from a missing one. A message can carry
/// that distinction and cannot *hold* it — a test asserting on substrings pins
/// the wording, so the next reword breaks the test without breaking the
/// behaviour. Tests here assert on the variant; [`Refusal::Display`] is pinned
/// separately, once, as a shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No identity is available to sign with.
    ///
    /// Carries the reason from whatever was asked, so the message names what is
    /// missing rather than only that something is. The publish path does not
    /// create an identity, a keystore or any key material to resolve this.
    NoIdentity(String),
    /// The parent or target op id names no op this peer holds.
    ///
    /// Distinct from [`Refusal::TargetIsNotAPost`] so a caller can tell a
    /// propagation gap from a category mistake: the first is fixed by waiting,
    /// the second is not.
    NotHeld { what: &'static str, id: OpId },
    /// The op this peer holds under that id is not a post.
    TargetIsNotAPost(OpId),
    /// The Stoa named in the request is not the Stoa the parent or target op
    /// belongs to.
    ///
    /// A reply is signed over the Stoa it names, so a reply naming one Stoa and
    /// a parent in another is an op whose thread is in a Stoa its signature does
    /// not cover.
    WrongStoa {
        what: &'static str,
        requested: Address,
        actual: Address,
    },
    /// The store could not be reached. Never "the store held nothing".
    Storage(OpLogError),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NoIdentity(why) => write!(
                f,
                "no identity is available to sign with: {why}; nothing was published \
                 and no key was created"
            ),
            Refusal::NotHeld { what, id } => write!(
                f,
                "this peer does not hold the {what} {}; it may not have propagated \
                 yet — nothing was published",
                id.to_hex()
            ),
            Refusal::TargetIsNotAPost(id) => write!(
                f,
                "the op {} is held but is not a post, so it cannot be replied to",
                id.to_hex()
            ),
            Refusal::WrongStoa {
                what,
                requested,
                actual,
            } => write!(
                f,
                "the {what} belongs to Stoa {} and the request names Stoa {}; an op \
                 is signed over the Stoa it names, so the two must agree",
                actual.to_hex(),
                requested.to_hex()
            ),
            Refusal::Storage(e) => write!(f, "{e}"),
        }
    }
}

impl From<OpLogError> for Refusal {
    fn from(e: OpLogError) -> Self {
        Refusal::Storage(e)
    }
}

/// Sign, append, and report what the append did.
///
/// **The order is the contract**, and it is one function so it is asserted once
/// rather than three times against three near-copies. The append completes
/// before this returns, so a caller's handoff to delivery cannot precede it; and
/// nothing here knows what delivery is, so no code path in this function can
/// wait on a delivery outcome.
///
/// The key must be the author's per-Stoa key. Taking it rather than deriving one
/// is what makes the identity un-parameterisable at the wire: there is no field
/// a caller could set that reaches this argument.
fn publish<L: OpLog>(log: &mut L, key: &SecretKey, op: Op) -> Result<Published, Refusal> {
    let signed = op.sign(key);
    let id = signed.op.id();
    // `Arrival::unordered()`: this op did not arrive, so no transport ordered
    // it. Claiming a Lamport value here would be this peer inventing ordering
    // metadata for its own content, which is precisely the forgeable
    // self-assertion `op.rs` refuses to put in the signed bytes.
    let appended = log.append(signed, Arrival::unordered())?;
    Ok(Published { id, appended })
}

/// Publish a top-level post: a `Post` naming no parent and no thread.
///
/// `thread: None` because a thread-opening post cannot know its own thread id —
/// that id is the hash of the bytes being signed. `op-format` contracts the
/// `None`, and the read side derives the thread as the root's own op id.
///
/// An **empty body is accepted**: `op-format` contracts an empty
/// variable-length field as a value rather than an absence, and refusing it here
/// would make the publish path disagree with the format about what an op may
/// contain.
pub fn post<L: OpLog>(
    log: &mut L,
    key: &SecretKey,
    stoa: Address,
    body: String,
) -> Result<Published, Refusal> {
    publish(
        log,
        key,
        Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body,
                // Attachments are out of this change's scope, so the list is
                // empty — which `op-format` contracts as a value.
                attachments: vec![],
            },
        },
    )
}

/// The thread an op belongs to, given the op and its id.
///
/// # The derivation, and what it trusts
///
/// A thread-opening post carries `thread: None`, because its own id is the hash
/// of the bytes being signed and so is not knowable at signing time. A reply
/// carries `thread: Some(t)`. So the thread an op belongs to is its own `thread`
/// field when it has one, and its own op id when it does not.
///
/// One expression, and it satisfies both directions: a reply to a root lands in
/// the root's thread, and a reply to a reply lands in the thread its parent
/// names.
///
/// **The parent's `thread` field is inside the parent's signed bytes**, so it is
/// its author's claim rather than a relay's. It may still be a *wrong* claim —
/// an inbound op can name a thread its own parent does not belong to, and the
/// log stores it, because the log decides nothing. This propagates the claim
/// rather than auditing it, and that is the correct division: auditing a thread
/// graph is a read-side question about which ops render where, and answering it
/// here would be a publish-path check a reader might come to rely on.
///
/// # `op.rs`'s doc comment on `thread` was wrong, and this is why it matters
///
/// It said "the store fills the thread in as its own id on ingest". The store
/// does not and cannot: the op is signed, so rewriting a field would invalidate
/// the signature. A reader who believed it would expect `thread` to be non-`None`
/// on every stored root and would derive the thread wrongly. The comment is
/// corrected in the same change as this function.
fn thread_of(op: &Op, id: OpId) -> OpId {
    match &op.kind {
        OpKind::Post { thread, .. } => thread.unwrap_or(id),
        // Not a post. Unreachable from `reply`, which refuses a non-post parent
        // before it gets here — but an answer rather than a panic, because a
        // panic aborts the module process. Answering with `id` treats the op as
        // its own thread root, which is the same answer a root post gets and so
        // introduces no shape a caller has not already seen.
        _ => id,
    }
}

/// Publish a reply: a `Post` naming its parent, with the thread **derived**.
///
/// # The thread is not a parameter, and that is the point
///
/// A thread supplied alongside a parent can disagree with it, and a reply filed
/// under a thread its parent does not belong to is a reply no reader will find
/// under either. There is no argument here to get wrong, so that state is
/// unrepresentable rather than checked at each call site.
///
/// # The cost, contracted rather than hidden
///
/// Deriving the thread means reading the parent, so a reply to a parent this
/// peer does not hold **cannot be published** — there is nothing to derive from,
/// and publishing into a guessed thread is the state deriving was chosen to
/// prevent. The two refusals are told apart: absent, versus held but not a post.
///
/// This is an outbound check and establishes nothing about an inbound reply. See
/// the module documentation.
pub fn reply<L: OpLog>(
    log: &mut L,
    key: &SecretKey,
    stoa: Address,
    parent: OpId,
    body: String,
) -> Result<Published, Refusal> {
    let entry = log.get(&parent)?.ok_or(Refusal::NotHeld {
        what: "parent",
        id: parent,
    })?;
    let parent_op = &entry.op.op;

    // A reply to a vote (or a moderation, or a metadata op) is a category
    // mistake, and it is refused distinguishably from an absent parent so a
    // caller can tell it from a propagation gap.
    if !matches!(parent_op.kind, OpKind::Post { .. }) {
        return Err(Refusal::TargetIsNotAPost(parent));
    }
    if parent_op.stoa != stoa {
        return Err(Refusal::WrongStoa {
            what: "parent",
            requested: stoa,
            actual: parent_op.stoa,
        });
    }

    let thread = thread_of(parent_op, parent);
    publish(
        log,
        key,
        Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: Some(thread),
                parent: Some(parent),
                body,
                attachments: vec![],
            },
        },
    )
}

/// Publish a vote on any op this peer holds.
///
/// # What is refused, and the one thing deliberately not refused
///
/// The target must be **present** — a vote naming an op nobody holds is a vote
/// on nothing — and it must be in the Stoa the request names. Beyond that
/// nothing is refused: not the target's kind, not its author, not that this
/// identity has voted on that target before.
///
/// Whether a vote *counts* is decided by whoever counts votes. A publish path
/// refusing a vote on a kind a future scorer would ignore would be that decision
/// made twice, in two places that can disagree — and the publish path is the
/// copy a scorer cannot see.
///
/// # Nothing here reports an effect
///
/// The reply is a [`Published`] like the other two: an op id and whether it was
/// new. No score, count, tally, rank or position, because **no ordering in the
/// current contract reads a `Vote` op** — `feed.rs` says so in as many words.
/// Reporting one would be a caller inferring an effect that does not exist.
pub fn vote<L: OpLog>(
    log: &mut L,
    key: &SecretKey,
    stoa: Address,
    target: OpId,
    direction: VoteDirection,
) -> Result<Published, Refusal> {
    let entry = log.get(&target)?.ok_or(Refusal::NotHeld {
        what: "target",
        id: target,
    })?;
    if entry.op.op.stoa != stoa {
        return Err(Refusal::WrongStoa {
            what: "target",
            requested: stoa,
            actual: entry.op.op.stoa,
        });
    }

    publish(
        log,
        key,
        Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Vote { target, direction },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::derive_stoa_key;
    use crate::log::MemoryOpLog;
    use crate::op::{ModerationAction, SignedOp};
    use crate::stoa::{Genesis, Policy};

    /// The root secret a keystore would hold. Fixed, so every derived address in
    /// these tests is reproducible.
    const A_ROOT: [u8; 32] = [7u8; 32];

    fn a_stoa(title: &str) -> Address {
        Genesis {
            creator: crate::identity::SecretKey::from_bytes(&[1u8; 32])
                .unwrap()
                .public_key(),
            policy: Policy::Open,
            title: title.to_string(),
        }
        .address()
        .expect("a short title is well under MAX_TITLE_BYTES")
    }

    /// The per-Stoa signing key, derived exactly as the keystore derives it.
    fn a_key(root: [u8; 32], stoa: &Address) -> SecretKey {
        derive_stoa_key(&root, stoa)
    }

    fn a_log() -> MemoryOpLog {
        MemoryOpLog::new()
    }

    /// The stored op behind an id, or a panic naming what was missing.
    fn stored(log: &MemoryOpLog, id: &OpId) -> SignedOp {
        log.get(id)
            .expect("the memory log never fails")
            .unwrap_or_else(|| panic!("the log holds no op {}", id.to_hex()))
            .op
    }

    fn body_of(op: &SignedOp) -> &str {
        match &op.op.kind {
            OpKind::Post { body, .. } => body,
            other => panic!("expected a post, got {other:?}"),
        }
    }

    fn parent_of(op: &SignedOp) -> Option<OpId> {
        match &op.op.kind {
            OpKind::Post { parent, .. } => *parent,
            other => panic!("expected a post, got {other:?}"),
        }
    }

    fn thread_field_of(op: &SignedOp) -> Option<OpId> {
        match &op.op.kind {
            OpKind::Post { thread, .. } => *thread,
            other => panic!("expected a post, got {other:?}"),
        }
    }

    // ─── Sign, append, hand off — in that order ───────────────────────────

    #[test]
    fn a_published_post_is_in_the_log_when_the_call_returns() {
        // The ordering requirement's observable half: the append has completed
        // by the time a caller holds the op id, so a caller can read what it
        // just created without waiting for anything.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, "First".to_string()).unwrap();
        assert_eq!(
            stored(&log, &published.id).op.id(),
            published.id,
            "the op id in the reply must name the op now in the log"
        );
        assert_eq!(body_of(&stored(&log, &published.id)), "First");
    }

    #[test]
    fn a_published_post_is_a_thread_root_naming_no_parent() {
        // `parent: None` is what makes this a thread head rather than a reply —
        // `feed.rs` filters on exactly that field. A post published with a
        // parent set would render as a reply and never appear in a feed.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, "the head".to_string()).unwrap();
        let op = stored(&log, &published.id);
        assert_eq!(parent_of(&op), None, "a post names no parent");
        assert_eq!(
            thread_field_of(&op),
            None,
            "a thread-opening post cannot know its own thread id"
        );
        assert!(matches!(op.op.kind, OpKind::Post { .. }));
    }

    #[test]
    fn an_empty_body_is_published_rather_than_refused() {
        // `op-format` contracts an empty variable-length field as a VALUE, so
        // refusing it here would make the publish path disagree with the format
        // about what an op may contain.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, String::new()).unwrap();
        assert_eq!(body_of(&stored(&log, &published.id)), "");
    }

    #[test]
    fn a_published_op_verifies_against_the_identity_derived_for_its_stoa() {
        // The author is derived from the Stoa and is never a parameter. The
        // expected address is derived INDEPENDENTLY here rather than read back
        // from the op, so this cannot pass by the test agreeing with itself.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, "mine".to_string()).unwrap();
        let op = stored(&log, &published.id);

        let expected = derive_stoa_key(&A_ROOT, &stoa).public_key().address();
        assert_eq!(op.op.author.address(), expected);
        assert!(op.verify(), "the op must verify against its own author");

        // And a DIFFERENT Stoa's identity is not it, or the assertion above
        // would hold for any key at all.
        let elsewhere = a_stoa("Lyceum");
        assert_ne!(
            op.op.author.address(),
            derive_stoa_key(&A_ROOT, &elsewhere).public_key().address()
        );
    }

    #[test]
    fn the_signing_identity_is_the_one_the_capability_probe_reports() {
        // The probe answers before a compose box renders and this path signs
        // after submit. If they disagreed, a user would be told one pseudonym
        // and post under another — and nothing would error.
        //
        // What this does and does not reach: the closure below re-implements the
        // composition `Keystore::stoa_address` performs (`stoa_public_key` then
        // `.address()`), rather than calling it. So this pins that the PUBLISH
        // path agrees with that composition from the same root — it would catch a
        // publish signing with a different key — but it would not catch
        // `Keystore::stoa_address` itself being changed to compose differently.
        // Closing that would mean reaching a real `Keystore`, which this layer
        // deliberately does not take.
        let stoa = a_stoa("Agora");
        let probe_reported = crate::wire::capability_for(&stoa, |s| {
            Ok(derive_stoa_key(&A_ROOT, s).public_key().address().to_hex())
        });
        let reported = match probe_reported {
            crate::wire::Capability::CanPost { identity } => identity,
            other => panic!("the probe must be able to post here, got {other:?}"),
        };

        let mut log = a_log();
        let published = post(
            &mut log,
            &a_key(A_ROOT, &stoa),
            stoa,
            "under which name?".to_string(),
        )
        .unwrap();
        assert_eq!(
            stored(&log, &published.id).op.author.address().to_hex(),
            reported
        );
    }

    #[test]
    fn a_published_ops_author_is_pinned_to_a_known_answer_from_another_file() {
        // `a_published_op_verifies_against_the_identity_derived_for_its_stoa`
        // derives its expectation by calling `derive_stoa_key` — the same
        // function the fixture's key came from — so it holds however that
        // derivation is defined. That is the right test for "the author is the
        // Stoa's identity and not some other key", and it is structurally unable
        // to notice the derivation itself changing.
        //
        // This is the other half, on the same pattern as
        // `identity.rs::the_wire_constants_are_pinned_to_known_answers`: the
        // expected author address comes from a HARDCODED seed, so nothing in the
        // publish path contributes to the expectation.
        //
        // The seed is `identity.rs`'s pinned value for
        // `derive_stoa_key([7; 32], stoa_address(b"a genesis record"))`, and
        // `keystore.rs::the_identity_survives_a_restart` reaches the same address
        // by a third route. Every address and signature this peer produces stops
        // matching everyone else's if it changes, with no error anywhere, because
        // each peer stays internally consistent.
        //
        // **If this fails, do NOT update the expected value to match.** Work out
        // what changed in the derivation and whether the network survives it.
        let stoa = crate::identity::stoa_address(b"a genesis record");
        let key = derive_stoa_key(&A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, "a pinned body".to_string()).unwrap();

        let pinned_seed =
            hex::decode("b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf")
                .unwrap();
        let expected = SecretKey::from_bytes(&pinned_seed)
            .unwrap()
            .public_key()
            .address();
        assert_eq!(
            stored(&log, &published.id).op.author.address(),
            expected,
            "a published op's author no longer matches the pinned per-Stoa derivation"
        );
        // The Stoa address is pinned too, because an author derived correctly
        // from the WRONG Stoa address would satisfy the assertion above only by
        // coincidence — and the fixture's Stoa is what makes the seed the right
        // expectation at all.
        assert_eq!(
            stoa.to_hex(),
            "6b1f1c28061e99c72e3340fb4fd07e8192b394e1327012e140f240a990d89cd8",
            "Stoa address derivation changed"
        );
        assert_eq!(stored(&log, &published.id).op.stoa, stoa);
    }

    // ─── The same content twice is one op ─────────────────────────────────

    #[test]
    fn the_same_content_published_twice_is_one_op_and_the_caller_is_told() {
        // The contracted consequence of an op id being a function of content
        // alone. Both publishes succeed, both name one id, and `appended` is
        // what tells a wire caller a deduplicated publish from a first one —
        // there is no other signal.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let first = post(&mut log, &key, stoa, "twice".to_string()).unwrap();
        let second = post(&mut log, &key, stoa, "twice".to_string()).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.appended, Appended::Stored);
        assert_eq!(
            second.appended,
            Appended::AlreadyPresent,
            "the newly-stored-or-already-present answer must be passed on, not discarded"
        );
        assert!(first.was_new() && !second.was_new());
        assert_eq!(log.len().unwrap(), 1, "one op, not two");
    }

    #[test]
    fn a_repeated_publish_does_not_disturb_the_stored_op() {
        // Byte-identical, not merely equal by field: the op is signed, so a
        // second publish that rewrote the entry would be a signature that no
        // longer verifies on the peer that received the first.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let first = post(&mut log, &key, stoa, "stable".to_string()).unwrap();
        let before = stored(&log, &first.id).to_bytes();
        post(&mut log, &key, stoa, "stable".to_string()).unwrap();
        assert_eq!(stored(&log, &first.id).to_bytes(), before);
    }

    #[test]
    fn content_differing_in_any_way_publishes_a_second_op() {
        // The other side of dedup, and the one that would break if the op id
        // were a function of anything coarser than the whole op. Each pair
        // differs in EXACTLY one respect.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(agora, lyceum, "the fixture needs two Stoas");
        let key = a_key(A_ROOT, &agora);

        // One character of body.
        let mut log = a_log();
        let a = post(&mut log, &key, agora, "hello".to_string()).unwrap();
        let b = post(&mut log, &key, agora, "hellp".to_string()).unwrap();
        assert_ne!(a.id, b.id, "bodies differing by one character");
        assert_eq!(log.len().unwrap(), 2);

        // The Stoa. Same body, and each signed with the key that Stoa derives —
        // which is how a real publish would do it, so this varies the Stoa AND
        // the identity exactly as production does.
        let mut log = a_log();
        let here = post(&mut log, &key, agora, "same words".to_string()).unwrap();
        let there = post(
            &mut log,
            &a_key(A_ROOT, &lyceum),
            lyceum,
            "same words".to_string(),
        )
        .unwrap();
        assert_ne!(here.id, there.id, "the same body in two Stoas");
        assert_eq!(log.len().unwrap(), 2);

        // The identity, holding the Stoa fixed. Two roots, one Stoa.
        let mut log = a_log();
        let mine = post(&mut log, &key, agora, "shared thought".to_string()).unwrap();
        let yours = post(
            &mut log,
            &a_key([9u8; 32], &agora),
            agora,
            "shared thought".to_string(),
        )
        .unwrap();
        assert_ne!(mine.id, yours.id, "the same body from two identities");
        assert_eq!(log.len().unwrap(), 2);
    }

    #[test]
    fn the_same_body_in_two_stoas_is_two_ops_with_the_identity_held_fixed() {
        // The spec lists "the same body in two Stoas" and "the same body from two
        // identities" as two SEPARATE scenarios, and
        // `content_differing_in_any_way_publishes_a_second_op`'s Stoa leg varies
        // BOTH at once — because it signs each with the key that Stoa derives,
        // which is what production does. Two explanations then give the same
        // answer: the ids differ because the Stoas differ, or because the keys
        // do. A `post` that ignored its `stoa` argument entirely passes that leg.
        //
        // So this holds the key FIXED and varies only the Stoa. There is now one
        // explanation left for the ids differing, and it is the one the scenario
        // names. (Signing a Lyceum op with an Agora key is not what production
        // does; it is what isolating one variable requires, and `post` takes the
        // two independently so it is expressible.)
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(agora, lyceum, "the fixture needs two Stoas");
        let one_key = a_key(A_ROOT, &agora);

        let mut log = a_log();
        let here = post(&mut log, &one_key, agora, "same words".to_string()).unwrap();
        let there = post(&mut log, &one_key, lyceum, "same words".to_string()).unwrap();

        assert_ne!(
            here.id, there.id,
            "the Stoa is inside the signed bytes, so one body in two Stoas is two ops"
        );
        assert_eq!(log.len().unwrap(), 2);
        // And each really landed in the Stoa it named, which is the property the
        // differing ids are evidence FOR rather than a substitute for.
        assert_eq!(stored(&log, &here.id).op.stoa, agora);
        assert_eq!(stored(&log, &there.id).op.stoa, lyceum);
    }

    #[test]
    fn two_replies_to_two_siblings_in_one_thread_are_two_ops() {
        // The spec's "two replies with the same body to different parents are two
        // ops", with the thread held FIXED.
        //
        // `two_replies_with_one_body_to_different_parents_are_two_ops` uses two
        // ROOTS as the parents, so the parent and the derived thread covary and
        // the ids differ whichever field the implementation actually wrote. A
        // `reply` that set `parent: Some(thread)` — dropping the parent entirely
        // and duplicating the thread — passes it. Verified: that mutation leaves
        // it green.
        //
        // Here both parents are replies in ONE thread, so the two ops agree in
        // Stoa, author, body and thread and differ only in `parent`. Nothing but
        // the parent is left to explain two ids.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let root = post(&mut log, &key, stoa, "the head".to_string()).unwrap();
        let sibling_one = reply(&mut log, &key, stoa, root.id, "first".to_string()).unwrap();
        let sibling_two = reply(&mut log, &key, stoa, root.id, "second".to_string()).unwrap();
        assert_ne!(sibling_one.id, sibling_two.id, "the fixture needs two");

        let to_one = reply(&mut log, &key, stoa, sibling_one.id, "agreed".to_string()).unwrap();
        let to_two = reply(&mut log, &key, stoa, sibling_two.id, "agreed".to_string()).unwrap();

        // The thread is the same for both, which is what removes it as an
        // explanation for the ids differing.
        assert_eq!(thread_field_of(&stored(&log, &to_one.id)), Some(root.id));
        assert_eq!(thread_field_of(&stored(&log, &to_two.id)), Some(root.id));
        assert_eq!(parent_of(&stored(&log, &to_one.id)), Some(sibling_one.id));
        assert_eq!(parent_of(&stored(&log, &to_two.id)), Some(sibling_two.id));
        assert_ne!(
            to_one.id, to_two.id,
            "the parent is inside the signed bytes, so two parents are two ops"
        );
        assert_eq!(log.len().unwrap(), 5);
    }

    #[test]
    fn two_replies_with_one_body_to_different_parents_are_two_ops() {
        // The parent is inside the signed bytes, so it participates in the op
        // id. A reply whose id ignored its parent would deduplicate two
        // genuinely different replies into one.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let one = post(&mut log, &key, stoa, "parent one".to_string()).unwrap();
        let two = post(&mut log, &key, stoa, "parent two".to_string()).unwrap();
        assert_ne!(one.id, two.id);

        let to_one = reply(&mut log, &key, stoa, one.id, "agreed".to_string()).unwrap();
        let to_two = reply(&mut log, &key, stoa, two.id, "agreed".to_string()).unwrap();
        assert_ne!(to_one.id, to_two.id);
        assert_eq!(log.len().unwrap(), 4);
    }

    // ─── A reply's thread is derived ──────────────────────────────────────

    #[test]
    fn a_reply_names_its_parent_and_is_derived_into_the_parents_thread() {
        // The derivation for a reply to a ROOT: the parent carries `thread:
        // None`, so the thread is the parent's own op id. Expected value is the
        // root's id, hardcoded from the first publish rather than read back
        // from the reply's own field.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let root = post(&mut log, &key, stoa, "the head".to_string()).unwrap();
        let child = reply(&mut log, &key, stoa, root.id, "a reply".to_string()).unwrap();

        let op = stored(&log, &child.id);
        assert!(
            matches!(op.op.kind, OpKind::Post { .. }),
            "a reply is a post"
        );
        assert_eq!(parent_of(&op), Some(root.id));
        assert_eq!(
            thread_field_of(&op),
            Some(root.id),
            "a reply to a root belongs to the root's thread"
        );
    }

    #[test]
    fn a_reply_to_a_reply_is_derived_into_the_thread_its_parent_belongs_to() {
        // THE test that separates deriving from copying. A copy-the-parent's-id
        // implementation gives `thread == middle.id` here, which is a thread
        // that does not exist: the grandchild belongs to the ROOT's thread.
        //
        // Three levels are needed, because at two levels "the parent's id" and
        // "the parent's thread" are the same value and the two implementations
        // agree.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let root = post(&mut log, &key, stoa, "the head".to_string()).unwrap();
        let middle = reply(&mut log, &key, stoa, root.id, "a reply".to_string()).unwrap();
        let leaf = reply(&mut log, &key, stoa, middle.id, "a reply to it".to_string()).unwrap();

        assert_ne!(root.id, middle.id, "the fixture needs three distinct ops");
        let op = stored(&log, &leaf.id);
        assert_eq!(parent_of(&op), Some(middle.id), "the parent is the reply");
        assert_eq!(
            thread_field_of(&op),
            Some(root.id),
            "the thread is the ROOT's, not the immediate parent's id"
        );
    }

    #[test]
    fn two_replies_to_one_parent_land_in_one_thread_however_else_they_differ() {
        // Deriving means the thread cannot vary with anything a caller chooses.
        // The two requests differ in every accepted field except the parent —
        // different bodies, different identities — and land in one thread.
        let stoa = a_stoa("Agora");
        let mut log = a_log();
        let author = a_key(A_ROOT, &stoa);
        let other = a_key([11u8; 32], &stoa);

        let root = post(&mut log, &author, stoa, "the head".to_string()).unwrap();
        let one = reply(&mut log, &author, stoa, root.id, "first".to_string()).unwrap();
        let two = reply(&mut log, &other, stoa, root.id, "second".to_string()).unwrap();
        assert_ne!(one.id, two.id, "the fixture needs two distinct replies");

        assert_eq!(thread_field_of(&stored(&log, &one.id)), Some(root.id));
        assert_eq!(thread_field_of(&stored(&log, &two.id)), Some(root.id));
    }

    #[test]
    fn a_reply_to_a_parent_the_peer_does_not_hold_is_refused_and_appends_nothing() {
        // The contracted cost of deriving rather than accepting a thread: with
        // no parent to read there is no thread to derive.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        // An op id for something never appended.
        let absent = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "never received".to_string(),
                attachments: vec![],
            },
        }
        .id();

        let refused = reply(&mut log, &key, stoa, absent, "into the void".to_string())
            .expect_err("a reply to an absent parent must be refused");
        assert_eq!(
            refused,
            Refusal::NotHeld {
                what: "parent",
                id: absent
            }
        );
        assert_eq!(log.len().unwrap(), 0, "a refused publish appends nothing");
    }

    #[test]
    fn a_reply_naming_a_non_post_is_refused_distinguishably_from_an_absent_parent() {
        // Two different mistakes: one is fixed by waiting for propagation, the
        // other is not. Collapsing them would send a caller to wait for
        // something that is already here.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let target = post(&mut log, &key, stoa, "the subject".to_string()).unwrap();
        let a_vote = vote(&mut log, &key, stoa, target.id, VoteDirection::Up).unwrap();

        let refused = reply(
            &mut log,
            &key,
            stoa,
            a_vote.id,
            "replying to a vote".to_string(),
        )
        .expect_err("a reply to a vote must be refused");
        assert_eq!(refused, Refusal::TargetIsNotAPost(a_vote.id));
        assert_ne!(
            refused,
            Refusal::NotHeld {
                what: "parent",
                id: a_vote.id
            },
            "held-but-wrong-kind must not read as absent"
        );
        assert_eq!(log.len().unwrap(), 2, "nothing was appended by the refusal");
    }

    #[test]
    fn a_reply_refused_for_an_absent_parent_succeeds_once_the_parent_arrives() {
        // The refusal is about what this peer holds NOW, not about the reply.
        // A peer that cached the refusal could never recover from a propagation
        // gap.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);

        // Build the parent in one log so its id is known, then publish the
        // reply against a log that does not hold it.
        let mut origin = a_log();
        let parent = post(&mut origin, &key, stoa, "arrives late".to_string()).unwrap();
        let parent_op = stored(&origin, &parent.id);

        let mut log = a_log();
        assert!(reply(&mut log, &key, stoa, parent.id, "eager".to_string()).is_err());

        log.append(parent_op, Arrival::unordered()).unwrap();
        let published = reply(&mut log, &key, stoa, parent.id, "eager".to_string())
            .expect("the same reply must publish once its parent is held");
        assert_eq!(
            thread_field_of(&stored(&log, &published.id)),
            Some(parent.id)
        );
    }

    #[test]
    fn a_cross_stoa_reply_is_refused_and_a_same_stoa_one_publishes() {
        // Both directions, because an implementation that always refused and
        // one that never did each pass half of this.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let mut log = a_log();
        let here = a_key(A_ROOT, &agora);
        let there = a_key(A_ROOT, &lyceum);

        let parent = post(&mut log, &here, agora, "in the agora".to_string()).unwrap();

        let refused = reply(
            &mut log,
            &there,
            lyceum,
            parent.id,
            "from elsewhere".to_string(),
        )
        .expect_err("a reply naming another Stoa's parent must be refused");
        assert_eq!(
            refused,
            Refusal::WrongStoa {
                what: "parent",
                requested: lyceum,
                actual: agora
            }
        );
        assert_eq!(log.len().unwrap(), 1, "nothing was appended");

        reply(&mut log, &here, agora, parent.id, "from here".to_string())
            .expect("a reply within one Stoa must publish");
        assert_eq!(log.len().unwrap(), 2);
    }

    // ─── Votes ────────────────────────────────────────────────────────────

    #[test]
    fn both_vote_directions_publish_and_differ_in_op_id() {
        // The direction is inside the signed bytes, so two directions on one
        // target are two ops. An encoding that dropped it would make a lowering
        // vote indistinguishable from a raising one.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let target = post(&mut log, &key, stoa, "the subject".to_string()).unwrap();
        let up = vote(&mut log, &key, stoa, target.id, VoteDirection::Up).unwrap();
        let down = vote(&mut log, &key, stoa, target.id, VoteDirection::Down).unwrap();

        assert_ne!(up.id, down.id, "the two directions are two ops");
        for (published, expected) in [(&up, VoteDirection::Up), (&down, VoteDirection::Down)] {
            match stored(&log, &published.id).op.kind {
                OpKind::Vote {
                    target: t,
                    direction,
                } => {
                    assert_eq!(t, target.id);
                    assert_eq!(direction, expected);
                }
                other => panic!("expected a vote, got {other:?}"),
            }
        }
        assert_eq!(log.len().unwrap(), 3);
    }

    #[test]
    fn voting_twice_on_one_target_publishes_two_ops_and_neither_refuses_the_other() {
        // Whether a vote COUNTS is the scorer's decision. A publish path
        // refusing a second vote would be that decision made twice, in two
        // places that can disagree — and this is the copy a scorer cannot see.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let target = post(&mut log, &key, stoa, "the subject".to_string()).unwrap();
        let up = vote(&mut log, &key, stoa, target.id, VoteDirection::Up)
            .expect("the first vote publishes");
        let down = vote(&mut log, &key, stoa, target.id, VoteDirection::Down)
            .expect("the second vote is not refused on the strength of the first");
        assert!(log.get(&up.id).unwrap().is_some());
        assert!(log.get(&down.id).unwrap().is_some());
    }

    #[test]
    fn an_identity_may_vote_on_its_own_post_and_on_any_kind_the_peer_holds() {
        // Not refused on the target's author, and not on its kind. A vote on a
        // moderation op is odd and is not this path's to forbid.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let mine = post(&mut log, &key, stoa, "my own post".to_string()).unwrap();
        vote(&mut log, &key, stoa, mine.id, VoteDirection::Up)
            .expect("an identity may vote on its own post");

        // A moderation op, appended directly — this path does not publish one.
        let moderation = Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Moderate {
                target: mine.id,
                action: ModerationAction::Hide,
            },
        }
        .sign(&key);
        let moderation_id = moderation.op.id();
        log.append(moderation, Arrival::unordered()).unwrap();

        vote(&mut log, &key, stoa, moderation_id, VoteDirection::Down)
            .expect("a vote is not refused on the grounds of the target's kind");
    }

    #[test]
    fn a_vote_on_an_absent_target_is_refused_and_appends_nothing() {
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let absent = OpId::from_hex(&"5c".repeat(32)).unwrap();
        let refused = vote(&mut log, &key, stoa, absent, VoteDirection::Up)
            .expect_err("a vote on nothing must be refused");
        assert_eq!(
            refused,
            Refusal::NotHeld {
                what: "target",
                id: absent
            }
        );
        assert_eq!(log.len().unwrap(), 0);
    }

    #[test]
    fn a_cross_stoa_vote_is_refused() {
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let mut log = a_log();

        let target = post(
            &mut log,
            &a_key(A_ROOT, &agora),
            agora,
            "in the agora".to_string(),
        )
        .unwrap();
        let refused = vote(
            &mut log,
            &a_key(A_ROOT, &lyceum),
            lyceum,
            target.id,
            VoteDirection::Up,
        )
        .expect_err("a vote naming another Stoa's target must be refused");
        assert_eq!(
            refused,
            Refusal::WrongStoa {
                what: "target",
                requested: lyceum,
                actual: agora
            }
        );
        assert_eq!(log.len().unwrap(), 1);
    }

    #[test]
    fn a_published_vote_is_readable_by_its_own_id_and_by_a_target_restricted_read() {
        // Both reads the requirement names. The target read is the one a later
        // scorer would fold over, so a vote invisible to it would be a vote
        // that accumulates no history.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let target = post(&mut log, &key, stoa, "the subject".to_string()).unwrap();
        let published = vote(&mut log, &key, stoa, target.id, VoteDirection::Up).unwrap();

        assert_eq!(stored(&log, &published.id).op.id(), published.id);
        let by_target: Vec<OpId> = log
            .iter_target(&target.id)
            .unwrap()
            .iter()
            .map(|e| e.id())
            .collect();
        assert_eq!(by_target, vec![published.id]);
    }

    #[test]
    fn a_voted_post_reads_back_byte_identical_and_its_feed_row_is_unchanged() {
        // Nothing reads a `Vote` op, so voting must change nothing a reader
        // sees. Byte-identical on the op, and identical on the resolved feed
        // row — which is where a score would show up if one existed.
        let genesis = Genesis {
            creator: crate::identity::SecretKey::from_bytes(&[1u8; 32])
                .unwrap()
                .public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        };
        let stoa = genesis.address().unwrap();
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, "unaffected".to_string()).unwrap();
        let before_bytes = stored(&log, &published.id).to_bytes();
        let moderators = crate::moderation::Moderators::of(&genesis).unwrap();
        let before_row = crate::feed::list_threads(&log, &moderators, &stoa, 0, 20, false).unwrap();

        vote(&mut log, &key, stoa, published.id, VoteDirection::Up).unwrap();
        vote(&mut log, &key, stoa, published.id, VoteDirection::Down).unwrap();

        assert_eq!(stored(&log, &published.id).to_bytes(), before_bytes);
        let after_row = crate::feed::list_threads(&log, &moderators, &stoa, 0, 20, false).unwrap();
        assert_eq!(
            before_row, after_row,
            "voting must change nothing this capability reports about the post"
        );
        assert_eq!(before_row.items.len(), 1, "the fixture must have a row");
    }

    // ─── The refusal shape ────────────────────────────────────────────────

    #[test]
    fn a_refusal_names_the_id_or_stoa_it_is_about() {
        // Pinned as a shape once, so every other test can assert on the variant
        // and none has to pin the wording. A message that named no identifier
        // would leave a caller unable to tell which of several parents was the
        // problem.
        let id = OpId::from_hex(&"ab".repeat(32)).unwrap();
        let held = Refusal::NotHeld { what: "parent", id }.to_string();
        assert!(held.contains("parent"), "got {held}");
        assert!(held.contains(&id.to_hex()), "got {held}");

        let kind = Refusal::TargetIsNotAPost(id).to_string();
        assert!(kind.contains("not a post"), "got {kind}");

        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let stoa = Refusal::WrongStoa {
            what: "target",
            requested: lyceum,
            actual: agora,
        }
        .to_string();
        assert!(stoa.contains(&agora.to_hex()), "got {stoa}");
        assert!(stoa.contains(&lyceum.to_hex()), "got {stoa}");

        let none = Refusal::NoIdentity("the keystore is locked".to_string()).to_string();
        assert!(none.contains("the keystore is locked"), "got {none}");
        assert!(
            none.contains("no key was created"),
            "the refusal must say no key material was created, got {none}"
        );
    }

    // ─── A publish check is a third thing ─────────────────────────────────

    #[test]
    fn an_op_a_publish_would_refuse_is_stored_anyway_when_it_arrives() {
        // The direction of the mistake that matters. An outbound check
        // constrains only the ops THIS peer creates; every op from a peer
        // arrived without passing one. A reader treating a publish refusal as
        // having established anything about a stored op would have been given
        // nothing.
        //
        // Two of the three shapes this module refuses are built by hand,
        // appended, and must be readable — indistinguishable from an op this peer
        // published. The two are the orphan (an absent parent) and the cross-Stoa
        // reply; the third, a non-post parent, is not built here. The point is the
        // direction rather than an enumeration, so two suffice to make it.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        let key = a_key(A_ROOT, &agora);
        let mut log = a_log();

        let absent_parent = OpId::from_hex(&"3c".repeat(32)).unwrap();
        let orphan = Op {
            stoa: agora,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: Some(absent_parent),
                parent: Some(absent_parent),
                body: "a reply to nothing we hold".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        // A reply whose Stoa differs from its parent's: publish the parent in
        // one Stoa and hand-build a reply naming the other.
        let parent = post(&mut log, &key, agora, "the parent".to_string()).unwrap();
        let cross = Op {
            stoa: lyceum,
            author: a_key(A_ROOT, &lyceum).public_key(),
            kind: OpKind::Post {
                thread: Some(parent.id),
                parent: Some(parent.id),
                body: "across a Stoa boundary".to_string(),
                attachments: vec![],
            },
        }
        .sign(&a_key(A_ROOT, &lyceum));

        // Each really is refused by this module, or the test proves nothing.
        assert!(reply(&mut log, &key, agora, absent_parent, "x".to_string()).is_err());
        assert!(reply(
            &mut log,
            &a_key(A_ROOT, &lyceum),
            lyceum,
            parent.id,
            "x".to_string()
        )
        .is_err());

        for op in [orphan, cross] {
            let id = op.op.id();
            assert_eq!(
                log.append(op, Arrival::unordered()).unwrap(),
                Appended::Stored,
                "the log must store an op the publish path refuses"
            );
            let entry = stored(&log, &id);
            assert!(
                entry.verify(),
                "and it verifies like any other — the log does not mark it as ours"
            );
        }
    }

    #[test]
    fn a_published_op_is_verified_on_read_like_any_other() {
        // Verification is over the op's own bytes and signature, and the result
        // does not depend on this peer having been the publisher. Shown by
        // tampering with a published op: it must stop verifying.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &key, stoa, "mine".to_string()).unwrap();
        let ours = stored(&log, &published.id);
        assert!(ours.verify());

        let tampered = SignedOp {
            op: Op {
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: "not what was signed".to_string(),
                    attachments: vec![],
                },
                ..ours.op.clone()
            },
            signature: ours.signature.clone(),
        };
        assert!(
            !tampered.verify(),
            "having been published here buys an op nothing on read"
        );
    }

    // ─── Hostile text reaches the op unchanged ────────────────────────────

    #[test]
    fn a_body_is_published_exactly_as_supplied() {
        // `op-format` contracts an accepted encoding as re-encoding to itself,
        // so a transformation here would mean the op published is not the
        // content the caller supplied. Sanitisation is a RENDERING concern and
        // `feed.rs` does it on the way out.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);

        for body in [
            "Ἀγορά — the marketplace",
            "\u{202E}reversed\u{202C}",
            "zero\u{200B}width",
            "  leading and trailing  ",
            "MiXeD CaSe",
            "a\0b",
            "🏛",
        ] {
            let mut log = a_log();
            let published = post(&mut log, &key, stoa, body.to_string()).unwrap();
            assert_eq!(
                body_of(&stored(&log, &published.id)).as_bytes(),
                body.as_bytes(),
                "the body must reach the op byte for byte"
            );
        }
    }

    #[test]
    fn two_bodies_differing_only_by_unicode_normalisation_are_two_ops() {
        // The sharp case for "no normalisation". These two strings render
        // identically and are different bytes: NFC "é" against NFD "e" + U+0301.
        // A publish path that normalised would collapse them into one op.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let composed = "caf\u{00E9}";
        let decomposed = "cafe\u{0301}";
        assert_ne!(
            composed.as_bytes(),
            decomposed.as_bytes(),
            "the fixture needs two byte strings"
        );

        let a = post(&mut log, &key, stoa, composed.to_string()).unwrap();
        let b = post(&mut log, &key, stoa, decomposed.to_string()).unwrap();
        assert_ne!(a.id, b.id, "normalisation forms must publish as two ops");
        assert_eq!(log.len().unwrap(), 2);
        assert_eq!(body_of(&stored(&log, &a.id)), composed);
        assert_eq!(body_of(&stored(&log, &b.id)), decomposed);
    }

    #[test]
    fn a_maximal_body_publishes_rather_than_panicking() {
        // `op-format` caps a variable-length field at MAX_FIELD_LEN. A body AT
        // the cap must encode, hash and store without a panic — and a panic
        // aborts the module process, so this is a denial of service and not a
        // cosmetic failure.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let body = "x".repeat(150 * 1024);
        let published = post(&mut log, &key, stoa, body.clone()).unwrap();
        assert_eq!(body_of(&stored(&log, &published.id)).len(), body.len());
    }

    // ─── A storage failure is never a silent success ──────────────────────

    #[test]
    fn a_store_that_cannot_be_read_is_a_refusal_and_not_an_absent_parent() {
        // §11.1 obligation 5 at the write path. "The store is broken" and "the
        // parent has not propagated" call for opposite responses — wait, versus
        // fix the disk — and a publish that reported the second for the first
        // would send someone waiting forever.
        struct BrokenLog;
        impl OpLog for BrokenLog {
            fn append(
                &mut self,
                _op: crate::op::SignedOp,
                _arrival: Arrival,
            ) -> Result<Appended, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn get(&self, _id: &OpId) -> Result<Option<crate::log::Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn iter(&self) -> Result<Vec<crate::log::Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn iter_stoa(&self, _stoa: &Address) -> Result<Vec<crate::log::Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn iter_target(&self, _target: &OpId) -> Result<Vec<crate::log::Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn len(&self) -> Result<usize, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
        }

        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let id = OpId::from_hex(&"11".repeat(32)).unwrap();

        for refusal in [
            post(&mut BrokenLog, &key, stoa, "x".to_string()).unwrap_err(),
            reply(&mut BrokenLog, &key, stoa, id, "x".to_string()).unwrap_err(),
            vote(&mut BrokenLog, &key, stoa, id, VoteDirection::Up).unwrap_err(),
        ] {
            assert!(
                matches!(refusal, Refusal::Storage(_)),
                "a store failure must not be reported as an absent parent, got {refusal:?}"
            );
            assert!(
                refusal.to_string().contains("the disk is on fire"),
                "the underlying reason must survive, got {refusal}"
            );
        }
    }
}
