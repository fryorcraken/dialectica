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

use crate::arrival::{next_counter, Arrival};
use crate::identity::{Address, SecretKey};
use crate::log::{Appended, OpLog, OpLogError};
use crate::op::{Op, OpClock, OpId, OpKind, VoteDirection};

/// What a publish did, and what to tell the caller.
///
/// Carries [`Appended`] through **unchanged** rather than recomputing it.
/// `op-log` is the layer that knows whether an append stored something new;
/// asking the store with a `get` before the `append` would be two round trips
/// computing a value the store is about to compute anyway, so the two could
/// disagree.
///
/// # What this field no longer reports, and what replaced it
///
/// This doc used to say that an op's bytes "carry no timestamp and no nonce, so
/// one identity publishing the same content twice produces one op". **Both
/// halves are now false.** The preimage carries a counter and a wall-clock, and
/// [`publish`] stamps both — so authoring the same body twice produces **two
/// ops**, with two ids, and both are stored.
///
/// Content-dedup therefore no longer protects an author from an accidental
/// double publish, and there is no delete in this system: a revision replaces
/// content rather than withdrawing an op. Refusing the duplicate here is not the
/// answer either — two identical posts minutes apart are a legitimate thing to
/// write, and this module may not decide otherwise.
///
/// **`appended` still reports what it always did**, and it is now the narrower
/// fact: whether *this exact op* — these bytes, this counter, this asserted time
/// — was already held. A byte-identical replay still reports
/// [`Appended::AlreadyPresent`], which is what makes a re-delivered op idempotent;
/// what changed is that two separate authorings are no longer byte-identical.
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
    /// The body is longer than `op-format` will decode.
    ///
    /// **Refused before signing, because the alternative is a corrupt row.** The
    /// encode path has no cap — `put_bytes` writes any length — while the decode
    /// path refuses over [`crate::op::MAX_FIELD_LEN`]. Without this guard an
    /// over-cap body signs, appends and reports success, and then every list read
    /// on that store fails to decode it: `ordered_read` propagates the first
    /// decode error, so one row kills the whole read, permanently and across a
    /// restart, with no API able to remove it. The op is also undecodable by every
    /// conforming peer, so the caller is told it published something no reader can
    /// ever render.
    ///
    /// The bound belongs here rather than in the encoder because this is where a
    /// caller can be *told*. It carries both numbers so the message can say by how
    /// much.
    BodyTooLong { len: usize, cap: usize },
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
            Refusal::BodyTooLong { len, cap } => write!(
                f,
                "the body is {len} bytes and the format decodes at most {cap}; \
                 nothing was published"
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

/// What a publish needs from outside this crate: the signing key and the clock.
///
/// # Why the wall-clock is a parameter and not a `SystemTime::now()`
///
/// **`dialectica-core` reads no clock anywhere, and must not start.** A pure
/// crate that sampled the system clock would put a nondeterministic input into a
/// publish, so an op's bytes — and therefore its id — would depend on when a
/// test happened to run. The host's environment enters at `wire.rs`, which is
/// where every other ambient value already enters.
///
/// # Why they travel together
///
/// A key and a clock are both things this module is *given* rather than things
/// it finds, and pairing them in one argument keeps the three public builders'
/// signatures from growing a second loose `u64` that a caller could transpose
/// with a page number.
pub struct Authorship<'a> {
    /// The author's per-Stoa key ([`crate::identity::derive_stoa_key`]).
    ///
    /// Taking it rather than deriving one is what makes the identity
    /// un-parameterisable at the wire: there is no field a caller could set that
    /// reaches it.
    pub key: &'a SecretKey,
    /// This peer's wall-clock, in milliseconds since the Unix epoch, to be
    /// signed into the op as the author's assertion — and the current time the
    /// op's counter is pegged to (see this module's private `publish`).
    ///
    /// As the wall-clock field it decides nothing, here or on any reader. It is
    /// carried because a reader is shown a time and expects one. The counter
    /// takes the same instant because a peer has one reading of the time.
    ///
    /// **Named `asserted_ms` and not `now_ms`, deliberately.** Every other
    /// millisecond value in this crate is the *reading* peer's clock, used only
    /// to clamp for display ([`crate::asserted_time`], `thread.rs`, `wire.rs`).
    /// This one is the *author's*, and it enters the signed preimage — so it is
    /// attacker-chosen data on every peer but the one that wrote it, which is
    /// the opposite end of the mechanism. A shared name would let someone
    /// threading a clock through a new publish path assume it is the reader's,
    /// and this change's whole thesis is that the two must never be confusable.
    /// It is assigned verbatim to [`crate::op::OpClock::asserted_ms`], which is
    /// the name it should therefore carry.
    pub asserted_ms: u64,
}

/// Sign, append, and report what the append did.
///
/// **The order is the contract**, and it is one function so it is asserted once
/// rather than three times against three near-copies. The append completes
/// before this returns, so a caller's handoff to delivery cannot precede it; and
/// nothing here knows what delivery is, so no code path in this function can
/// wait on a delivery outcome.
///
/// # The counter is stamped HERE, from the log, and not by the caller
///
/// The op reaches this function with `clock: None` and leaves it signed, because
/// the counter is a function of what this peer holds and the three builders
/// above have no business computing it. That is also what makes "every published
/// op carries a counter" true by construction rather than by three call sites
/// each remembering to do it.
///
/// The counter is the later of this peer's current time and one above its clock
/// for the op's Stoa ([`next_counter`]), so it is above every counter this peer
/// holds of that Stoa and never behind its time.
///
/// # One instant signs both clock fields
///
/// `op-ordering` requires a publish to take its current time once and sign it
/// as both the counter's basis and the wall-clock field. That reading is
/// `who.asserted_ms`, which the adapter samples once (`now_ms()`). A second
/// field for the counter's time would let a caller pass two readings that
/// disagree, which is the state the requirement forbids; with one field it
/// cannot be written down. `design.md` Decision 3.
///
/// That does not make the wall-clock field decide anything: the counter is
/// computed from the host's time, and nothing reads the field to compute it.
fn publish<L: OpLog>(log: &mut L, who: &Authorship<'_>, op: Op) -> Result<Published, Refusal> {
    // Read BEFORE the op is signed: the counter is inside the preimage, so it
    // has to be known before there are bytes to sign.
    let clock = log.clock(&op.stoa)?;

    let op = Op {
        clock: Some(OpClock {
            counter: next_counter(clock, who.asserted_ms),
            asserted_ms: who.asserted_ms,
        }),
        ..op
    };

    let signed = op.sign(who.key);
    let id = signed.op.id();
    // `Arrival::unordered()`: this op did not arrive, so the transport said
    // nothing about it. The arrival is a record of a delivery and orders
    // nothing, so what it holds for a locally-published op is simply "nothing
    // was received".
    let appended = log.append(signed, Arrival::unordered())?;
    Ok(Published { id, appended })
}

/// The longest body a publish will sign, which is the longest one the format will
/// decode.
///
/// **Not a second number.** It is `op-format`'s own field cap, and
/// `the_publish_body_cap_is_the_format_field_cap` pins them as one value rather
/// than two that happen to agree — a drifted cap would otherwise leave this path
/// signing bodies the decoder refuses, which is the defect this constant exists to
/// prevent.
pub const MAX_BODY_LEN: usize = crate::op::MAX_FIELD_LEN;

/// Refuse a body the format cannot decode, before anything is signed.
///
/// Its own function rather than a line in each handler, because a guard is a job:
/// "is it called everywhere a body is accepted?" stays a question with an answer.
/// Two callers today, `post` and `reply`; a third operation carrying a
/// caller-supplied variable-length field needs it too.
fn body_within_cap(body: &str) -> Result<(), Refusal> {
    if body.len() > MAX_BODY_LEN {
        return Err(Refusal::BodyTooLong {
            len: body.len(),
            cap: MAX_BODY_LEN,
        });
    }
    Ok(())
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
///
/// An **over-cap body is refused**, and the asymmetry with the empty case is not
/// arbitrary: an empty body decodes, and a body over
/// [`crate::op::MAX_FIELD_LEN`] does not. See [`Refusal::BodyTooLong`] for what
/// publishing one would cost.
pub fn post<L: OpLog>(
    log: &mut L,
    who: &Authorship<'_>,
    stoa: Address,
    body: String,
) -> Result<Published, Refusal> {
    body_within_cap(&body)?;
    publish(
        log,
        who,
        Op {
            stoa,
            author: who.key.public_key(),
            // Stamped by `publish` from the log's clock. `None` here is not a
            // published op's clock — it is the absence of one on an op that has
            // not been through the one function that assigns them.
            clock: None,
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
/// **That audit now exists: [`crate::thread::thread_of`] performs it.** This
/// paragraph deferred it and the deferral has been discharged, so the sentence
/// above is a division of labour rather than a gap. Note the two functions share
/// a name and hold **opposite** rules about this field — the one here reads it
/// and trusts it, which is right for an op this peer is about to sign; the one
/// there never reads it at all, and follows `parent` to a root instead, which is
/// the only safe rule for an op that arrived from somebody else. A reader who
/// has met one should not assume the other is the same rule at another layer.
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
    who: &Authorship<'_>,
    stoa: Address,
    parent: OpId,
    body: String,
) -> Result<Published, Refusal> {
    // Before the store read: a body the format cannot decode is refusable without
    // knowing anything about the parent, so there is no reason to go to disk for
    // it.
    body_within_cap(&body)?;

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
        who,
        Op {
            stoa,
            author: who.key.public_key(),
            clock: None,
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
    who: &Authorship<'_>,
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
        who,
        Op {
            stoa,
            author: who.key.public_key(),
            clock: None,
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

    /// **A SECOND peer's root secret.** The causality requirements are about two
    /// identities — `op-ordering`: *"Publishing after receiving advances past what
    /// was received"*, *"Every peer that receives both computes the same
    /// relation"* — and a fixture with one author cannot distinguish a clock that
    /// folds over every op it holds from one scoped to the peer's own authorship.
    /// Two tests here said "an op from somebody else" in a comment while signing
    /// with the local key; this is the somebody else.
    const ANOTHER_ROOT: [u8; 32] = [11u8; 32];

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

    /// A fixed asserted time for tests that are not about the clock.
    ///
    /// **A constant, not `SystemTime::now()`.** It is inside the preimage, so a
    /// sampled value would make every op id in this suite depend on when the
    /// test ran — and two ops published in one test would differ by whether the
    /// millisecond ticked between them, which is a flake nobody could reproduce.
    /// 2026-09-18T11:01:44Z, by `date -u -d @1789729304`.
    ///
    /// **This is an ASSERTED time, not a reader's clock**, and `thread.rs` spells
    /// a constant of the same value `A_TIME` for the opposite role — the reading
    /// peer's own clock, passed as `now_ms`. The field was renamed
    /// `now_ms` → `asserted_ms` in this change and these two fixtures were not;
    /// the name is kept here only because renaming a fixture across two modules
    /// is churn without a reader, and recorded so the next person comparing the
    /// two files knows they name different things.
    const A_TIME: u64 = 1_789_729_304_000;

    /// Authorship with a fixed clock, for the tests that only need to publish.
    fn by(key: &SecretKey) -> Authorship<'_> {
        Authorship {
            key,
            asserted_ms: A_TIME,
        }
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

        let published = post(&mut log, &by(&key), stoa, "First".to_string()).unwrap();
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

        let published = post(&mut log, &by(&key), stoa, "the head".to_string()).unwrap();
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

        let published = post(&mut log, &by(&key), stoa, String::new()).unwrap();
        assert_eq!(body_of(&stored(&log, &published.id)), "");
    }

    #[test]
    fn a_published_op_verifies_against_the_identity_derived_for_its_stoa() {
        // The author is derived from the Stoa and is never a parameter. The
        // expected key is derived INDEPENDENTLY here rather than read back
        // from the op, so this cannot pass by the test agreeing with itself.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &by(&key), stoa, "mine".to_string()).unwrap();
        let op = stored(&log, &published.id);

        let expected = derive_stoa_key(&A_ROOT, &stoa).public_key();
        assert_eq!(op.op.author.to_hex(), expected.to_hex());
        assert!(op.verify(), "the op must verify against its own author");

        // And a DIFFERENT Stoa's identity is not it, or the assertion above
        // would hold for any key at all.
        let elsewhere = a_stoa("Lyceum");
        assert_ne!(
            op.op.author.to_hex(),
            derive_stoa_key(&A_ROOT, &elsewhere).public_key().to_hex()
        );
    }

    #[test]
    fn the_signing_identity_is_the_one_the_capability_probe_reports() {
        // The probe answers before a compose box renders and this path signs
        // after submit. If they disagreed, a user would be told one pseudonym
        // and post under another — and nothing would error.
        //
        // What this does and does not reach: the closure below re-implements the
        // composition `Keystore::stoa_public_key` performs (`stoa_key` then
        // `.public_key()`), rather than calling it. So this pins that the PUBLISH
        // path agrees with that composition from the same root — it would catch a
        // publish signing with a different key — but it would not catch
        // `Keystore::stoa_public_key` itself being changed to compose
        // differently. Closing that would mean reaching a real `Keystore`, which
        // this layer deliberately does not take.
        let stoa = a_stoa("Agora");
        let probe_reported = crate::wire::capability_for(&stoa, |s| {
            Ok(derive_stoa_key(&A_ROOT, s).public_key().to_hex())
        });
        let reported = match probe_reported {
            crate::wire::Capability::CanPost { identity } => identity,
            other => panic!("the probe must be able to post here, got {other:?}"),
        };

        let mut log = a_log();
        let published = post(
            &mut log,
            &by(&a_key(A_ROOT, &stoa)),
            stoa,
            "under which name?".to_string(),
        )
        .unwrap();
        assert_eq!(stored(&log, &published.id).op.author.to_hex(), reported);
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
        // expected author key comes from a HARDCODED seed, so nothing in the
        // publish path contributes to the expectation.
        //
        // The seed is `identity.rs`'s pinned value for
        // `derive_stoa_key([7; 32], stoa_address(b"a genesis record"))`, and
        // `keystore.rs::the_identity_survives_a_restart` reaches the same identity
        // by a third route. Every signature this peer produces stops matching
        // everyone else's if it changes, with no error anywhere, because each peer
        // stays internally consistent.
        //
        // **If this fails, do NOT update the expected value to match.** Work out
        // what changed in the derivation and whether the network survives it.
        let stoa = crate::identity::stoa_address(b"a genesis record");
        let key = derive_stoa_key(&A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &by(&key), stoa, "a pinned body".to_string()).unwrap();

        let pinned_seed =
            hex::decode("b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf")
                .unwrap();
        let expected = SecretKey::from_bytes(&pinned_seed).unwrap().public_key();
        assert_eq!(
            stored(&log, &published.id).op.author.to_hex(),
            expected.to_hex(),
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

    // ─── Authoring twice is two ops; re-publishing one is one ─────────────
    //
    // **THIS IS A REVERSAL, and the reversal is the point.** The previous
    // behaviour published one op for the same body twice, and the prior contract
    // recorded that it was "correct for one case and wrong for another": a
    // double-submitted form was deduplicated, which was wanted, and a person
    // deliberately posting the same short reply twice published once, which was
    // not.
    //
    // The rule underneath is UNCHANGED and is the single rule it always was:
    // identical bytes are one op. What changed is its input — the bytes now
    // carry a counter that advances between two publishes, so "the same content"
    // is no longer enough to make two publishes byte-identical.
    //
    // The case the old behaviour served is now unserved, and that is a real cost
    // rather than a neutral trade: a double-submitted form publishes twice.
    // Suppressing it belongs to whatever handles the submission, where the two
    // cases are distinguishable — `DComposer.qml` disables its control while a
    // publish is outstanding, and `composer-view` contracts it.

    #[test]
    fn the_same_body_authored_twice_yields_two_ops() {
        // The counter advances between the two publishes, so the bytes differ,
        // so the ids differ. The log holds both and a reader renders both.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let first = post(&mut log, &by(&key), stoa, "twice".to_string()).unwrap();
        let second = post(&mut log, &by(&key), stoa, "twice".to_string()).unwrap();

        assert_ne!(
            first.id, second.id,
            "authoring one body twice must produce two ops"
        );
        assert_eq!(log.len().unwrap(), 2, "two ops, not one");
        // BOTH are reported as newly stored. Neither is "already present" —
        // which is what a caller branches on, and reporting the second as a
        // duplicate would tell a user their post was not saved when it was.
        assert_eq!(first.appended, Appended::Stored);
        assert_eq!(second.appended, Appended::Stored);
        assert!(first.was_new() && second.was_new());
    }

    #[test]
    fn the_second_authoring_carries_the_higher_counter() {
        // And therefore leads in the order. Asserted on the stored ops rather
        // than inferred, so this says WHY the ids differ — a test that only
        // checked `assert_ne!` on the ids would pass for a nonce.
        //
        // The body is "two times" rather than "twice" because the pegged counter
        // (`time-pegged-clock`) re-rolled both digests and "twice" stopped
        // disagreeing — the drift the guard below exists to catch, caught.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let first = post(&mut log, &by(&key), stoa, "two times".to_string()).unwrap();
        let second = post(&mut log, &by(&key), stoa, "two times".to_string()).unwrap();

        let counter_of = |id: &OpId| {
            stored(&log, id)
                .op
                .clock
                .expect("a published op carries a clock")
                .counter
        };
        assert!(
            counter_of(&second.id) > counter_of(&first.id),
            "two successive publishes take successive counters"
        );

        // And the ordering rule places the second ahead of the first. Read
        // through the log rather than by re-deriving the comparison, so this
        // asserts the ORDER a reader sees rather than a property of two numbers.
        //
        // **The coincidence is made a checked precondition.** These two ops differ
        // only in their counter, so which op id sorts lower is a property of the
        // digest — which means "ordered by counter" and "ordered by ascending op
        // id" are both live explanations of the assertion below. The digests
        // happen to fall the right way today, so the assertion is sound; what was
        // missing is anything keeping it sound. A reword of the body, a different
        // key, another Stoa title, or a change to the asserted time would re-roll
        // both hashes and could silently make the two rules agree — and the
        // `now_ms` → `asserted_ms` rename in this very change re-rolled them
        // already, which is how much movement it takes.
        //
        // The disagreement required: `second.id > first.id`, so ascending op id
        // would place the FIRST op ahead and the descending-counter answer below
        // places the second. They are checked against each other rather than
        // stated, because which of two SHA-256 outputs is larger is not a fact to
        // take on trust — this repo has recorded the cost of doing so.
        //
        // Not searched, unlike `arrival.rs`'s disagreement fixture, because there
        // is nothing here to search over: this test is about two publishes of one
        // fixed body through the real `post` path, and varying the body would be
        // testing a different thing. So the relation is asserted instead, and the
        // failure message says what to do about it rather than what went wrong.
        assert!(
            second.id > first.id,
            "the fixture has drifted: the second op's id no longer sorts ABOVE \
             the first's, so ascending-op-id order and descending-counter order \
             now agree and the assertion below distinguishes neither. Re-roll the \
             fixture (a different body) until they disagree again — do not delete \
             this guard."
        );

        let ids: Vec<OpId> = log.iter().unwrap().iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![second.id, first.id], "newest first, by counter");
    }

    #[test]
    fn re_publishing_an_op_the_peer_already_holds_yields_one_op() {
        // The case that still deduplicates, and the reason `Appended` is
        // retained rather than dropped: an op received from the network before
        // this peer publishes an identical one is still one op, because the two
        // are the same op only if EVERY field matches, counter included.
        //
        // Reached by publishing once and appending the SAME SIGNED OP again,
        // which is what a re-publish is. Calling `post` twice would author
        // twice, which is the different act the test above covers.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let first = post(&mut log, &by(&key), stoa, "once".to_string()).unwrap();
        let held = stored(&log, &first.id);

        let again = log.append(held.clone(), Arrival::unordered()).unwrap();
        assert_eq!(
            again,
            Appended::AlreadyPresent,
            "every field matching, counter included, is one op"
        );
        assert_eq!(log.len().unwrap(), 1, "one op, not two");
        // Byte-identical: the op is signed, so a second append that rewrote the
        // entry would be a signature that no longer verifies on the peer that
        // received the first.
        assert_eq!(
            stored(&log, &first.id).to_bytes().unwrap(),
            held.to_bytes().unwrap()
        );
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
        let a = post(&mut log, &by(&key), agora, "hello".to_string()).unwrap();
        let b = post(&mut log, &by(&key), agora, "hellp".to_string()).unwrap();
        assert_ne!(a.id, b.id, "bodies differing by one character");
        assert_eq!(log.len().unwrap(), 2);

        // The Stoa. Same body, and each signed with the key that Stoa derives —
        // which is how a real publish would do it, so this varies the Stoa AND
        // the identity exactly as production does.
        let mut log = a_log();
        let here = post(&mut log, &by(&key), agora, "same words".to_string()).unwrap();
        let there = post(
            &mut log,
            &by(&a_key(A_ROOT, &lyceum)),
            lyceum,
            "same words".to_string(),
        )
        .unwrap();
        assert_ne!(here.id, there.id, "the same body in two Stoas");
        assert_eq!(log.len().unwrap(), 2);

        // The identity, holding the Stoa fixed. Two roots, one Stoa.
        let mut log = a_log();
        let mine = post(&mut log, &by(&key), agora, "shared thought".to_string()).unwrap();
        let yours = post(
            &mut log,
            &by(&a_key([9u8; 32], &agora)),
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
        let here = post(&mut log, &by(&one_key), agora, "same words".to_string()).unwrap();
        let there = post(&mut log, &by(&one_key), lyceum, "same words".to_string()).unwrap();

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

        let root = post(&mut log, &by(&key), stoa, "the head".to_string()).unwrap();
        let sibling_one = reply(&mut log, &by(&key), stoa, root.id, "first".to_string()).unwrap();
        let sibling_two = reply(&mut log, &by(&key), stoa, root.id, "second".to_string()).unwrap();
        assert_ne!(sibling_one.id, sibling_two.id, "the fixture needs two");

        let to_one = reply(
            &mut log,
            &by(&key),
            stoa,
            sibling_one.id,
            "agreed".to_string(),
        )
        .unwrap();
        let to_two = reply(
            &mut log,
            &by(&key),
            stoa,
            sibling_two.id,
            "agreed".to_string(),
        )
        .unwrap();

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

        let one = post(&mut log, &by(&key), stoa, "parent one".to_string()).unwrap();
        let two = post(&mut log, &by(&key), stoa, "parent two".to_string()).unwrap();
        assert_ne!(one.id, two.id);

        let to_one = reply(&mut log, &by(&key), stoa, one.id, "agreed".to_string()).unwrap();
        let to_two = reply(&mut log, &by(&key), stoa, two.id, "agreed".to_string()).unwrap();
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

        let root = post(&mut log, &by(&key), stoa, "the head".to_string()).unwrap();
        let child = reply(&mut log, &by(&key), stoa, root.id, "a reply".to_string()).unwrap();

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

        let root = post(&mut log, &by(&key), stoa, "the head".to_string()).unwrap();
        let middle = reply(&mut log, &by(&key), stoa, root.id, "a reply".to_string()).unwrap();
        let leaf = reply(
            &mut log,
            &by(&key),
            stoa,
            middle.id,
            "a reply to it".to_string(),
        )
        .unwrap();

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

        let root = post(&mut log, &by(&author), stoa, "the head".to_string()).unwrap();
        let one = reply(&mut log, &by(&author), stoa, root.id, "first".to_string()).unwrap();
        let two = reply(&mut log, &by(&other), stoa, root.id, "second".to_string()).unwrap();
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
            clock: None,
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "never received".to_string(),
                attachments: vec![],
            },
        }
        .id();

        let refused = reply(
            &mut log,
            &by(&key),
            stoa,
            absent,
            "into the void".to_string(),
        )
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

        let target = post(&mut log, &by(&key), stoa, "the subject".to_string()).unwrap();
        let a_vote = vote(&mut log, &by(&key), stoa, target.id, VoteDirection::Up).unwrap();

        let refused = reply(
            &mut log,
            &by(&key),
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
        let parent = post(&mut origin, &by(&key), stoa, "arrives late".to_string()).unwrap();
        let parent_op = stored(&origin, &parent.id);

        let mut log = a_log();
        assert!(reply(&mut log, &by(&key), stoa, parent.id, "eager".to_string()).is_err());

        log.append(parent_op, Arrival::unordered()).unwrap();
        let published = reply(&mut log, &by(&key), stoa, parent.id, "eager".to_string())
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

        let parent = post(&mut log, &by(&here), agora, "in the agora".to_string()).unwrap();

        let refused = reply(
            &mut log,
            &by(&there),
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

        reply(
            &mut log,
            &by(&here),
            agora,
            parent.id,
            "from here".to_string(),
        )
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

        let target = post(&mut log, &by(&key), stoa, "the subject".to_string()).unwrap();
        let up = vote(&mut log, &by(&key), stoa, target.id, VoteDirection::Up).unwrap();
        let down = vote(&mut log, &by(&key), stoa, target.id, VoteDirection::Down).unwrap();

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

        let target = post(&mut log, &by(&key), stoa, "the subject".to_string()).unwrap();
        let up = vote(&mut log, &by(&key), stoa, target.id, VoteDirection::Up)
            .expect("the first vote publishes");
        let down = vote(&mut log, &by(&key), stoa, target.id, VoteDirection::Down)
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

        let mine = post(&mut log, &by(&key), stoa, "my own post".to_string()).unwrap();
        vote(&mut log, &by(&key), stoa, mine.id, VoteDirection::Up)
            .expect("an identity may vote on its own post");

        // A moderation op, appended directly — this path does not publish one.
        let moderation = Op {
            stoa,
            author: key.public_key(),
            clock: None,
            kind: OpKind::Moderate {
                target: mine.id,
                action: ModerationAction::Hide,
            },
        }
        .sign(&key);
        let moderation_id = moderation.op.id();
        log.append(moderation, Arrival::unordered()).unwrap();

        vote(
            &mut log,
            &by(&key),
            stoa,
            moderation_id,
            VoteDirection::Down,
        )
        .expect("a vote is not refused on the grounds of the target's kind");
    }

    #[test]
    fn a_vote_on_an_absent_target_is_refused_and_appends_nothing() {
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let absent = OpId::from_hex(&"5c".repeat(32)).unwrap();
        let refused = vote(&mut log, &by(&key), stoa, absent, VoteDirection::Up)
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
            &by(&a_key(A_ROOT, &agora)),
            agora,
            "in the agora".to_string(),
        )
        .unwrap();
        let refused = vote(
            &mut log,
            &by(&a_key(A_ROOT, &lyceum)),
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

        let target = post(&mut log, &by(&key), stoa, "the subject".to_string()).unwrap();
        let published = vote(&mut log, &by(&key), stoa, target.id, VoteDirection::Up).unwrap();

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
        //
        // **The two halves are not equally strong, and the weaker one is kept
        // deliberately.** The byte-identical assertion discriminates: an
        // implementation that rewrote the target on a vote would fail it. The feed
        // row assertion currently cannot fail, because `feed::list_threads`
        // resolves `Post` ops and a `Vote` cannot appear in a row under any
        // implementation of this capability — so "the feed never read votes" is
        // both the thing asserted and the reason it passes (found by review,
        // findings/spec-test.md entry 5).
        //
        // Kept rather than deleted because it becomes a real test the moment
        // `relevance-ordering` lands and a scorer starts reading votes, which is
        // exactly when someone would otherwise add a score to a row without
        // noticing this requirement. The `items.len() == 1` guard below is what
        // stops it degrading into comparing two empty feeds.
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

        let published = post(&mut log, &by(&key), stoa, "unaffected".to_string()).unwrap();
        let before_bytes = stored(&log, &published.id).to_bytes().unwrap();
        let moderators = crate::moderation::Moderators::of(&genesis).unwrap();
        let before_row = crate::feed::list_threads(&log, &moderators, &stoa, 0, 20, false).unwrap();

        vote(&mut log, &by(&key), stoa, published.id, VoteDirection::Up).unwrap();
        vote(&mut log, &by(&key), stoa, published.id, VoteDirection::Down).unwrap();

        assert_eq!(
            stored(&log, &published.id).to_bytes().unwrap(),
            before_bytes
        );
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
        // NO SPEC: the spec's scenario "A refused publish creates no key material"
        // is a STRUCTURAL claim — that no keystore or key exists which did not
        // exist before — and it does not ask the refusal to say so. Stating it in
        // the message is chosen here, because the caller's next move ("is my
        // keystore now in some half-made state?") is a question the refusal can
        // answer for free.
        //
        // Asserting that a refusal SAYS no key was created is not asserting that
        // none was; the structural claim is discharged by core having no way to
        // create key material at all (tasks.md §11). This pins the sentence only.
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
            clock: None,
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
        let parent = post(&mut log, &by(&key), agora, "the parent".to_string()).unwrap();
        let cross = Op {
            stoa: lyceum,
            author: a_key(A_ROOT, &lyceum).public_key(),
            clock: None,
            kind: OpKind::Post {
                thread: Some(parent.id),
                parent: Some(parent.id),
                body: "across a Stoa boundary".to_string(),
                attachments: vec![],
            },
        }
        .sign(&a_key(A_ROOT, &lyceum));

        // Each really is refused by this module, or the test proves nothing.
        assert!(reply(&mut log, &by(&key), agora, absent_parent, "x".to_string()).is_err());
        assert!(reply(
            &mut log,
            &by(&a_key(A_ROOT, &lyceum)),
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
        // does not depend on this peer having been the publisher.
        //
        // The scenario's second clause is a COMPARISON — "the result does not
        // depend on this peer having been the publisher" — so the test is a
        // comparison. This used to tamper with a published op and assert it stopped
        // verifying, which is a property of Ed25519 rather than of this capability:
        // any `verify()` checking any signature over any encoding passes that,
        // including one that ignored the publish path entirely (found by review,
        // findings/spec-test.md entry 4).
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        // One op this peer published, through the publish path.
        let published = post(&mut log, &by(&key), stoa, "mine".to_string()).unwrap();
        let ours = stored(&log, &published.id);

        // The same op built and signed by hand, as an arriving one would be —
        // never through `post`, and appended the way an inbound op is.
        //
        // **The clock is spelled out rather than copied from `ours`.** Copying
        // would make the byte comparison below tautological: two ops built from
        // one op's fields are equal whatever the publish path did. Both fields
        // are `A_TIME`: that is what `by()` hands the publish path, and a first
        // op into an empty Stoa is counted at the current time — so this asserts
        // that the publish path stamps the values the contract says it does.
        let theirs = Op {
            stoa,
            author: key.public_key(),
            clock: Some(OpClock {
                counter: A_TIME,
                asserted_ms: A_TIME,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "mine".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);

        assert_eq!(
            ours.verify(),
            theirs.verify(),
            "verification must not depend on which side of the publish path an op came from"
        );
        assert!(ours.verify(), "both must actually verify, not both fail");
        assert_eq!(
            ours.to_bytes().unwrap(),
            theirs.to_bytes().unwrap(),
            "the published op and the hand-built one must be the same op, or the \
             comparison above compares two different things"
        );
    }

    // ─── The counter a publish stamps ─────────────────────────────────────

    #[test]
    fn a_first_op_in_a_stoa_carries_the_current_time() {
        // A peer holding no ops of the Stoa has a clock of zero, so one above it
        // is 1 and the current time is the later. Asserted on the STORED op, so
        // this is about what was signed rather than about what a helper
        // returned.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let published = post(&mut log, &by(&key), stoa, "first".to_string()).unwrap();
        assert_eq!(
            stored(&log, &published.id).op.clock.unwrap().counter,
            A_TIME,
            "a first op into an empty Stoa carries the current time"
        );
    }

    // `op-ordering`, "One reading of the time signs the counter and the
    // wall-clock alike": a publish takes its current time once and signs it as
    // both the counter's basis and the wall-clock field, so an op whose clock
    // was behind the time carries a counter equal to its wall-clock. This was a
    // `NO SPEC:` marker until the publish requirement said so.
    #[test]
    fn one_reading_of_the_time_signs_both_clock_fields() {
        // A time other than `A_TIME`, so neither field can be right by
        // agreeing with a fixture constant.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();
        let then = A_TIME + 123_456;

        let published = post(
            &mut log,
            &Authorship {
                key: &key,
                asserted_ms: then,
            },
            stoa,
            "one instant".to_string(),
        )
        .unwrap();
        let clock = stored(&log, &published.id).op.clock.unwrap();
        assert_eq!(clock.counter, then);
        assert_eq!(clock.asserted_ms, then);
    }

    #[test]
    fn a_clock_behind_the_current_time_publishes_at_the_time() {
        // Behind by far more than one, so "one above the clock" and "the time"
        // are different answers and only the time is right.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let peer = a_key(ANOTHER_ROOT, &stoa);
        let mut log = a_log();
        let received = Op {
            stoa,
            author: peer.public_key(),
            clock: Some(OpClock {
                counter: A_TIME - 86_400_000,
                asserted_ms: A_TIME - 86_400_000,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "yesterday".to_string(),
                attachments: vec![],
            },
        }
        .sign(&peer);
        log.append(received, Arrival::unordered()).unwrap();

        let published = post(&mut log, &by(&key), stoa, "today".to_string()).unwrap();
        let counter = stored(&log, &published.id).op.clock.unwrap().counter;
        assert_eq!(counter, A_TIME);
        assert_ne!(counter, A_TIME - 86_400_000 + 1);
    }

    #[test]
    fn publishing_after_receiving_advances_past_what_was_received() {
        // The causality property: a peer that holds an op at N publishes above
        // N, whatever its own time says. The received op is AHEAD of this peer's
        // time (within the hour the window allows), so the time alone would
        // sign below it and only the clock term gives the right answer.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        // **A genuinely different author**, which is the half this test used to
        // only claim. Its comment said "an op from somebody else" while signing
        // with the local key, so a clock that folded over only the peer's OWN
        // prior ops passed it — and the requirement is about causality BETWEEN
        // peers.
        let peer = a_key(ANOTHER_ROOT, &stoa);
        assert_ne!(
            key.public_key().to_bytes(),
            peer.public_key().to_bytes(),
            "the fixture needs two identities, or 'received' means 'authored'"
        );
        let mut log = a_log();

        // An op from somebody else, carrying a counter half a minute ahead of
        // this peer's time.
        let received = Op {
            stoa,
            author: peer.public_key(),
            clock: Some(OpClock {
                counter: A_TIME + 30_000,
                asserted_ms: A_TIME + 30_000,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "from a peer".to_string(),
                attachments: vec![],
            },
        }
        .sign(&peer);
        log.append(received, Arrival::unordered()).unwrap();

        let published = post(&mut log, &by(&key), stoa, "mine".to_string()).unwrap();
        assert_eq!(
            stored(&log, &published.id).op.clock.unwrap().counter,
            A_TIME + 30_001,
            "a publish takes one above the highest counter held"
        );
    }

    // `op-ordering`, "A counter taken from the clock leaves the wall-clock at
    // the current time": when this peer's clock for a Stoa is ABOVE its
    // current time, the counter takes the clock term (one above the held
    // maximum), and the wall-clock field still takes the current time — the
    // two fields diverge. `one_reading_of_the_time_signs_both_clock_fields`
    // covers the other branch, where the clock is BEHIND the time and the two
    // fields end up equal by construction; that fixture cannot tell "the
    // wall-clock is the current time" apart from "the wall-clock is whatever
    // the counter came out to", because in that scenario they are the same
    // number. This fixture separates them: a mutation that wrote the counter
    // into the wall-clock field (matching the letter of "one reading signs
    // both fields" while dropping which VALUE each field gets) would still
    // pass every other test in this file, and only fails here.
    #[test]
    fn a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time() {
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let peer = a_key(ANOTHER_ROOT, &stoa);
        let mut log = a_log();

        // An op from somebody else, carrying a counter far ahead of this
        // peer's current time — so this peer's clock for the Stoa ends up
        // above A_TIME, and the clock term (not the time) decides the
        // counter of the next publish.
        let received = Op {
            stoa,
            author: peer.public_key(),
            clock: Some(OpClock {
                counter: A_TIME + 3_000_000,
                asserted_ms: A_TIME + 3_000_000,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "from a fast clock".to_string(),
                attachments: vec![],
            },
        }
        .sign(&peer);
        log.append(received, Arrival::unordered()).unwrap();

        let published = post(&mut log, &by(&key), stoa, "mine".to_string()).unwrap();
        let clock = stored(&log, &published.id).op.clock.unwrap();

        // Both expectations are computed independently of the code under
        // test, from the fixture's own numbers, rather than read back from
        // what `publish` produced.
        assert_eq!(
            clock.counter,
            A_TIME + 3_000_001,
            "the counter is one above the clock this peer holds"
        );
        assert_eq!(
            clock.asserted_ms, A_TIME,
            "the wall-clock stays at this peer's current time, not at the \
             value the clock term produced for the counter"
        );
        assert_ne!(
            clock.asserted_ms, clock.counter,
            "the fixture must actually separate the two fields, or the \
             assertions above could agree by the two fields happening to \
             carry the same number"
        );
    }

    #[test]
    fn a_reply_to_an_op_ahead_of_the_time_still_carries_the_greater_counter() {
        // The case the advance bound got wrong: a parent signed ahead of the
        // replier's time. Under the bound, an over-bound parent never moved the
        // clock and its answer signed LOWER. Here the parent is ahead but within
        // the window, and the reply must carry the greater counter — which is
        // what places it after its parent in a thread.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let peer = a_key(ANOTHER_ROOT, &stoa);
        let mut log = a_log();
        let parent = Op {
            stoa,
            author: peer.public_key(),
            clock: Some(OpClock {
                counter: A_TIME + 3_000_000,
                asserted_ms: A_TIME,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "from a fast clock".to_string(),
                attachments: vec![],
            },
        }
        .sign(&peer);
        let parent_id = parent.op.id();
        log.append(parent, Arrival::unordered()).unwrap();

        let answer = reply(
            &mut log,
            &by(&key),
            stoa,
            parent_id,
            "an answer".to_string(),
        )
        .unwrap();
        assert!(
            stored(&log, &answer.id).op.clock.unwrap().counter > A_TIME + 3_000_000,
            "the answer must carry a greater counter than the op it answers"
        );
    }

    /// Hand an op held by one log to another through the real receive boundary,
    /// at the receiving peer's current time `now_ms`.
    fn deliver(from: &MemoryOpLog, id: &OpId, to: &mut MemoryOpLog, now_ms: u64) {
        use crate::transport::{receive, ChannelIdentity, InboundMessage, OpenChannels};
        let op = stored(from, id);
        let identity = ChannelIdentity::of(&op.op.stoa);
        let mut channels = OpenChannels::new();
        channels.open(&identity);
        let payload = op.to_bytes().unwrap();
        receive(
            InboundMessage {
                channel_id: identity.channel_id(),
                sender_id: "a peer",
                payload: &payload,
                timestamp: 0,
            },
            &channels,
            to,
            now_ms,
        )
        .expect("the fixture's ops are all within the window");
    }

    #[test]
    fn a_reply_carries_a_greater_counter_than_the_post_and_the_rule_places_it_first() {
        // Issue #162's first item. `op-ordering`'s scenario said "every peer
        // holding both places the post first", which contradicted the rule it
        // sat under: the rule places the HIGHER counter first, and the reply
        // carries it. Pinned here on BOTH peers, over the real receive path, at
        // one instant — so the reply's counter can only be greater through the
        // clock term.
        let stoa = a_stoa("Agora");
        let author = a_key(A_ROOT, &stoa);
        let replier = a_key(ANOTHER_ROOT, &stoa);

        // Searched, so that ascending op id would place the POST first: the
        // assertion below then fails under an id-ordered log rather than
        // agreeing with it by a coincidence of digests.
        for n in 0..64u32 {
            let mut first_peer = a_log();
            let mut second_peer = a_log();
            let post_id = post(&mut first_peer, &by(&author), stoa, format!("the post {n}"))
                .unwrap()
                .id;
            deliver(&first_peer, &post_id, &mut second_peer, A_TIME);
            let reply_id = reply(
                &mut second_peer,
                &by(&replier),
                stoa,
                post_id,
                "the reply".to_string(),
            )
            .unwrap()
            .id;
            if reply_id < post_id {
                continue;
            }
            deliver(&second_peer, &reply_id, &mut first_peer, A_TIME);

            let counter = |log: &MemoryOpLog, id: &OpId| stored(log, id).op.clock.unwrap().counter;
            assert!(counter(&second_peer, &reply_id) > counter(&second_peer, &post_id));
            for (which, log) in [("first", &first_peer), ("second", &second_peer)] {
                let ids: Vec<OpId> = log.iter().unwrap().iter().map(|e| e.id()).collect();
                assert_eq!(
                    ids,
                    vec![reply_id, post_id],
                    "the {which} peer must place the reply before the post"
                );
            }
            return;
        }
        panic!("no body in 64 gave a reply id above the post id");
    }

    #[test]
    fn an_op_signed_ahead_of_the_time_leads_only_until_the_time_passes_it() {
        // One peer holds an op signed half an hour ahead. A second peer that
        // does not hold it publishes once its own time has passed that counter:
        // its op carries the greater counter and the rule places it first.
        let stoa = a_stoa("Agora");
        let ahead = A_TIME + 1_800_000;
        let fast = a_key(ANOTHER_ROOT, &stoa);
        let mut first_peer = a_log();
        let early = post(
            &mut first_peer,
            &Authorship {
                key: &fast,
                asserted_ms: ahead,
            },
            stoa,
            "from a fast clock".to_string(),
        )
        .unwrap();

        // Searched, so that ascending op id would place the EARLY op first, and
        // an id-ordered log fails the assertion below rather than passing by a
        // coincidence of digests. A fixed body did coincide when first written.
        let later_key = a_key(A_ROOT, &stoa);
        let (mut second_peer, later) = (0..64u32)
            .find_map(|n| {
                let mut log = a_log();
                let published = post(
                    &mut log,
                    &Authorship {
                        key: &later_key,
                        asserted_ms: ahead + 1,
                    },
                    stoa,
                    format!("once the time has passed it {n}"),
                )
                .unwrap();
                (published.id > early.id).then_some((log, published))
            })
            .expect("no body in 64 gave an id above the early op's");
        assert!(
            stored(&second_peer, &later.id).op.clock.unwrap().counter
                > stored(&first_peer, &early.id).op.clock.unwrap().counter
        );

        // Both peers, holding both, place the later op first.
        deliver(&first_peer, &early.id, &mut second_peer, ahead + 1);
        deliver(&second_peer, &later.id, &mut first_peer, ahead + 1);
        for log in [&first_peer, &second_peer] {
            let ids: Vec<OpId> = log.iter().unwrap().iter().map(|e| e.id()).collect();
            assert_eq!(ids, vec![later.id, early.id]);
        }
    }

    #[test]
    fn a_publish_at_the_maximum_representable_clock_saturates() {
        // The receive window refuses a `u64::MAX` counter on arrival
        // (`transport.rs` pins that), but `append` admits one — a store written
        // before this change, or restored from a snapshot, can hold it. A peer
        // whose clock is the maximum must still publish, and its counter must
        // saturate rather than wrap to zero, which would place its op below
        // every other op in the Stoa.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        // Signed by the OTHER peer, for the reason spelled out on
        // `ANOTHER_ROOT`: a clock scoped to this peer's own authorship would
        // pass a single-author version of this test.
        let peer = a_key(ANOTHER_ROOT, &stoa);
        let mut log = a_log();
        let maximal = Op {
            stoa,
            author: peer.public_key(),
            clock: Some(OpClock {
                counter: u64::MAX,
                asserted_ms: A_TIME,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "u64::MAX".to_string(),
                attachments: vec![],
            },
        }
        .sign(&peer);
        log.append(maximal, Arrival::unordered()).unwrap();
        assert_eq!(log.clock(&stoa).unwrap(), u64::MAX);

        let published = post(&mut log, &by(&key), stoa, "still posting".to_string())
            .expect("publishing at the maximum clock succeeds");
        assert_eq!(
            stored(&log, &published.id).op.clock.unwrap().counter,
            u64::MAX,
            "saturated at the maximum, not wrapped"
        );
    }

    #[test]
    fn one_stoas_ops_do_not_advance_another_stoas_clock() {
        // The clock is per Stoa. A shared one would leak activity across Stoas —
        // a reader in a quiet Stoa could infer the peer is busy elsewhere — and
        // would order ops that never contend.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(agora, lyceum, "the fixture needs two Stoas");
        let mut log = a_log();

        // Several ops in one Stoa, so its clock is well above zero.
        let agora_key = a_key(A_ROOT, &agora);
        for _ in 0..5 {
            post(&mut log, &by(&agora_key), agora, "busy".to_string()).unwrap();
        }

        // A first op in the OTHER Stoa carries the current time. The busy
        // Stoa's clock is `A_TIME + 4` after five publishes at one instant, so
        // a shared clock would sign `A_TIME + 5` here.
        let lyceum_key = a_key(A_ROOT, &lyceum);
        let published = post(&mut log, &by(&lyceum_key), lyceum, "quiet".to_string()).unwrap();
        assert_eq!(log.clock(&agora).unwrap(), A_TIME + 4);
        assert_eq!(
            stored(&log, &published.id).op.clock.unwrap().counter,
            A_TIME,
            "the busy Stoa's counters must not reach the quiet one"
        );
    }

    #[test]
    fn the_asserted_time_is_the_one_the_caller_supplied() {
        // The publish path signs the caller's clock verbatim and neither samples
        // one nor adjusts what it is given. A core that read the system clock
        // here would make an op's bytes depend on when a test ran.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        // A deliberately implausible value, to show nothing clamps on the way
        // in — clamping is a read-time presentation rule, and the wall-clock is
        // inside the signed preimage so the boundary could not clamp it anyway.
        let absurd = u64::MAX;
        let who = Authorship {
            key: &key,
            asserted_ms: absurd,
        };
        let published = post(&mut log, &who, stoa, "from 584 million AD".to_string()).unwrap();

        let op = stored(&log, &published.id);
        assert_eq!(op.op.clock.unwrap().asserted_ms, absurd);
        assert!(op.verify(), "and it is signed over that value");
    }

    #[test]
    fn publish_stamps_a_counter_onto_an_op_of_any_kind() {
        // **The property that is TOTAL, rather than the three builders that
        // happen to exist today.** An earlier version of this test named
        // `post`, `reply` and `vote` — which is the recorded hand-maintained-
        // sweep shape. `OpKind` already has `Moderate`, `Revise` and
        // `StoaMetadata` with no builder yet, so three more were coming, each
        // absent from the list with the suite green.
        //
        // `publish` is the one function between a locally-built `Op` and
        // `append`, and it overwrites `clock` with `..op` regardless of what
        // the builder set — so "carries a counter" is a property of `publish`
        // and of the kind, not of the builder. Derived from `every_op_kind`
        // the way `op.rs`'s `one_of_each_kind` derives its version-2 half: a
        // seventh variant enters this test for free, because `every_op_kind`
        // is what the other sweeps already count.
        //
        // The input carries `clock: None` deliberately. The compiler demands
        // the field and accepts `None`, which is exactly the mistake a new
        // builder would make; this asserts `publish` corrects it rather than
        // trusting it.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let author = key.public_key();

        let kinds = crate::log::fixtures::every_op_kind();
        assert!(kinds.len() >= 6, "the fixture must still cover every kind");

        for kind in kinds {
            let mut log = a_log();
            let unstamped = Op {
                stoa,
                author: author.clone(),
                clock: None,
                kind,
            };
            let published = publish(&mut log, &by(&key), unstamped).unwrap();
            let clock = stored(&log, &published.id)
                .op
                .clock
                .expect("publish must stamp a clock onto an op of every kind");
            assert_eq!(
                clock.counter, A_TIME,
                "the first op into an empty log is counted at the current time"
            );
            assert_eq!(clock.asserted_ms, A_TIME, "and carries the author's time");
        }
    }

    #[test]
    fn the_counters_ascend_across_successive_publishes() {
        // The other half of the old sweep, kept because it is a different
        // claim: each publish reads the clock the one before it advanced, so
        // three ops into one Stoa come back strictly ascending. The test above
        // covers "a counter is stamped"; this covers "the value moves".
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let root = post(&mut log, &by(&key), stoa, "root".to_string()).unwrap();
        let child = reply(&mut log, &by(&key), stoa, root.id, "reply".to_string()).unwrap();
        let ballot = vote(
            &mut log,
            &by(&key),
            stoa,
            root.id,
            crate::op::VoteDirection::Up,
        )
        .unwrap();

        let counter_of = |id: &OpId| stored(&log, id).op.clock.unwrap().counter;
        assert!(counter_of(&root.id) < counter_of(&child.id));
        assert!(counter_of(&child.id) < counter_of(&ballot.id));
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
            let published = post(&mut log, &by(&key), stoa, body.to_string()).unwrap();
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

        let a = post(&mut log, &by(&key), stoa, composed.to_string()).unwrap();
        let b = post(&mut log, &by(&key), stoa, decomposed.to_string()).unwrap();
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
        //
        // The cap is referenced rather than written as a literal. It used to be
        // `150 * 1024` here, which meant a drifted cap would silently move this
        // test off the boundary it exists to sit on.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let body = "x".repeat(MAX_BODY_LEN);
        let published = post(&mut log, &by(&key), stoa, body.clone()).unwrap();
        assert_eq!(body_of(&stored(&log, &published.id)).len(), body.len());
    }

    #[test]
    fn the_publish_body_cap_is_the_format_field_cap() {
        // One number, not two that agree today. If these were independent
        // constants, raising the format's cap would leave the publish path
        // refusing bodies the format accepts, and LOWERING it would leave the
        // publish path signing bodies the format refuses — which is the defect
        // this pair exists to prevent, in the direction that corrupts a store.
        assert_eq!(MAX_BODY_LEN, crate::op::MAX_FIELD_LEN);
    }

    #[test]
    fn a_body_one_byte_over_the_cap_is_refused_rather_than_signed() {
        // The boundary tested on the side that was missing. A body AT the cap
        // round-trips (above); one byte over does not decode, so signing it
        // produces an op this module's own decoder refuses.
        //
        // What publishing it would cost, and why this is not a cosmetic refusal:
        // the op is appended and reported as published, and from then on every
        // list read on that store fails — `ordered_read` propagates the first
        // decode error, so one row kills the whole read, across a restart, with no
        // API able to remove it. The op is also undecodable by every conforming
        // peer. Found by review (findings/security.md S1, findings/correctness.md
        // C1, findings/spec-test.md entry 3), measured against SqliteOpLog.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let over = "x".repeat(MAX_BODY_LEN + 1);
        assert_eq!(
            post(&mut log, &by(&key), stoa, over.clone()),
            Err(Refusal::BodyTooLong {
                len: MAX_BODY_LEN + 1,
                cap: MAX_BODY_LEN,
            }),
            "a body the format cannot decode must be refused before it is signed"
        );
        assert_eq!(log.len().unwrap(), 0, "a refused publish appends nothing");

        // A reply too, and before the parent is even looked up: the body is
        // refusable without a store read.
        let seed = post(&mut log, &by(&key), stoa, "the subject".to_string()).unwrap();
        assert_eq!(
            reply(&mut log, &by(&key), stoa, seed.id, over),
            Err(Refusal::BodyTooLong {
                len: MAX_BODY_LEN + 1,
                cap: MAX_BODY_LEN,
            }),
        );
        assert_eq!(log.len().unwrap(), 1, "only the seed is stored");
    }

    #[test]
    fn every_op_a_publish_produces_decodes_again() {
        // The property the cap exists to protect, asserted directly rather than
        // through the cap: whatever this path signs, `op-format` can read back.
        //
        // Be honest about what this does and does not catch. It does NOT reproduce
        // the over-cap defect: its longest body is AT the cap, so it passed even
        // with the guard disabled. What it pins is the invariant going forward —
        // any future field this path writes gets round-tripped, so an encode/decode
        // asymmetry in a NEW field fails here rather than in a user's store.
        //
        // The reason no such test existed is recorded in tasks.md §10: every
        // publish test uses `MemoryOpLog`, which stores the live `SignedOp` and
        // never round-trips it through bytes, so no fixture here could observe an
        // asymmetry at all. This one goes through `to_bytes`/`from_bytes`
        // explicitly, which is what closes that blind spot.
        let stoa = a_stoa("Agora");
        let key = a_key(A_ROOT, &stoa);
        let mut log = a_log();

        let bodies = ["", "short", &"x".repeat(MAX_BODY_LEN)];
        for body in bodies {
            let published = post(&mut log, &by(&key), stoa, body.to_string()).unwrap();
            let signed = stored(&log, &published.id);
            let bytes = signed.to_bytes().unwrap();
            let decoded = crate::op::SignedOp::from_bytes(&bytes)
                .expect("an op this path published must decode again");
            assert_eq!(
                decoded.op.id(),
                published.id,
                "a published op must decode back to the same op id"
            );
        }
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
            post(&mut BrokenLog, &by(&key), stoa, "x".to_string()).unwrap_err(),
            reply(&mut BrokenLog, &by(&key), stoa, id, "x".to_string()).unwrap_err(),
            vote(&mut BrokenLog, &by(&key), stoa, id, VoteDirection::Up).unwrap_err(),
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
