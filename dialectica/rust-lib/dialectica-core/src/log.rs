//! The op log: the append-only store every piece of forum state derives from.
//!
//! # Why this file exists
//!
//! PLAN.md §3.3: "the forum's whole state is a function of the ops a peer has
//! seen [...] Ops are the authority; the view is a cache that can be rebuilt by
//! replay." [`op::SignedOp`] says what is stored and [`arrival::Arrival`] says
//! what orders it; neither says where it goes. This is where it goes.
//!
//! # The store decides nothing, and that is the whole design
//!
//! §3.3 puts verification on read: "Verification therefore happens on **read**,
//! filtering unsigned or badly-signed ops out. The store may hold junk; the
//! reader never trusts it."
//!
//! So [`OpLog::append`] does not verify a signature, does not check whether the
//! signer was a moderator, and does not reject an op for any reason except that
//! it is already here. Three separate reasons, and the third is the one that is
//! easy to miss:
//!
//! 1. **A filtered op is indistinguishable from an op never received.** A peer
//!    that silently discarded a forgery could not later answer "did someone try
//!    to forge this?", which is a different question from "is this valid?" and
//!    calls for a different response.
//! 2. **Validity is not decidable at append time.** §6 makes a moderation op
//!    valid "only when signed by a current moderator", which needs the Stoa's
//!    moderator set as of that op's Lamport time. An op that is unauthorised
//!    under today's set may be authorised under the set that a not-yet-received
//!    op establishes. Dropping it now forecloses that.
//! 3. **Filtering on write is a guard with many call sites.** CLAUDE.md: "A
//!    guard is a job. Keep it separate, so 'is it called everywhere?' stays a
//!    question with an answer." There is one reader per question; there is one
//!    writer. Putting the guard on the reader is the shape where forgetting it
//!    is visible.
//!
//! # The order is not this module's to choose
//!
//! Every read returns ops in [`arrival::cmp_ops`] order, and this module
//! contains no comparison of its own. Insertion order is deliberately *not*
//! available through any method: it is per-peer by construction, and a reader
//! that could reach it would be one refactor away from ordering a thread by the
//! sequence one peer's network happened to deliver in.
//!
//! # Dedup happens before the sort, and that ordering is load-bearing
//!
//! [`cmp_ops`] has a precondition its signature cannot express: it is total over
//! **distinct ops**, and two entries sharing an [`OpId`] but carrying different
//! [`Arrival`] metadata compare `Equal`. That is correct — the pair really is
//! tied under §5.7's rule, which has nothing to say about one op against itself
//! — but a sort over such a pair leaves their relative order to the sort's
//! stability, which is not a defined order and differs with the sequence a peer
//! happened to receive in.
//!
//! This log cannot present that pair, and not by remembering not to: entries are
//! held in a map keyed by [`OpId`], so one op id is one entry and `sorted`
//! iterates values that are distinct by construction. There is no intermediate
//! list of arrivals that could be sorted before being deduplicated, because
//! there is no intermediate list at all. CLAUDE.md's "put the complexity in the
//! data structure, not the logic" is the whole of the defence here.
//!
//! # Two peers holding different ops is the normal case
//!
//! §3.3: "Two peers routinely hold **different sets of ops** — one was offline,
//! one joined late, a message has not propagated yet." So no method here has a
//! "not enough information" outcome. A read over a log missing the op a revision
//! targets returns the revision; a read restricted to an absent target returns
//! nothing. Both are answers, neither is an error, and completeness is not a
//! property any peer can establish about itself.

use crate::arrival::{cmp_ops, Arrival, OpEntry};
use crate::identity::Address;
use crate::op::{OpId, OpKind, SignedOp};
use std::collections::HashMap;

/// One op as the log holds it: what arrived, and what the transport said.
///
/// The two are kept side by side rather than merged because they are different
/// kinds of fact. The op is signed, identical on every peer, and byte-stable;
/// the arrival is this peer's record of one delivery, unsigned, and legitimately
/// different on the peer next door. A type that flattened them would invite a
/// reader to treat a locally-recorded Lamport value with the confidence due to a
/// signed field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub op: SignedOp,
    pub arrival: Arrival,
}

impl Entry {
    /// This entry's identity in the log.
    ///
    /// Recomputed from the op rather than stored, so it cannot drift from the op
    /// it names. It is a hash of bytes the entry already holds; caching it would
    /// buy an allocation and cost the invariant.
    pub fn id(&self) -> OpId {
        self.op.op.id()
    }

    /// The op this one names, if its kind names one.
    ///
    /// One function rather than a match at each resolver, because "what does this
    /// op act on?" is one question whatever the answer's kind. A `Post` names no
    /// target: its `parent` is a reply relationship, not a subject the op acts
    /// upon, and conflating the two would make a target-restricted read return
    /// every reply to a post alongside the moderations of it.
    /// **Deliberately exhaustive, with no wildcard arm.** A new op kind must
    /// force a decision here, because the wrong default is silent: a kind that
    /// should name a target but returns `None` is simply never returned by
    /// `iter_target`, and no test that does not know about the kind can notice.
    /// `StoaMetadata` arrived while this change was in flight and the compiler
    /// stopped the build, which is the intended behaviour.
    pub fn target(&self) -> Option<OpId> {
        match &self.op.op.kind {
            OpKind::Post { .. } => None,
            // Acts on the Stoa, which it names through `Op::stoa` rather than
            // as a target op id. A metadata op supersedes the genesis title for
            // display (§5.7); it does not act upon another op, so no
            // target-restricted read should return it. Whatever resolves Stoa
            // metadata reads by Stoa, not by target.
            OpKind::StoaMetadata { .. } => None,
            OpKind::Revise { target, .. } => Some(*target),
            OpKind::Moderate { target, .. } => Some(*target),
            OpKind::Vote { target, .. } => Some(*target),
        }
    }
}

/// What an append did.
///
/// Returned rather than left for the caller to work out by counting, because
/// counting before and after is a two-step check every call site would have to
/// spell the same way, and because a caller genuinely wants to know: a newly
/// stored op is one to gossip onward and to rebuild a view from, and a duplicate
/// is neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appended {
    /// The log did not hold this op. It does now.
    Stored,
    /// The log already held this op. Nothing changed, including the arrival
    /// metadata already recorded — see [`OpLog::append`].
    AlreadyPresent,
}

/// What a peer stores, and what a reader may ask of it.
///
/// # Why this is a trait with one implementation today
///
/// PLAN.md §9 Phase 1 names it: "pure Rust behind `Transport` and `Store`
/// traits, tested against fakes with no node running". §3.3 names SQLite as the
/// destination. The trait is the seam between the two, and it is not a
/// speculative abstraction in CLAUDE.md's sense — the second implementation is
/// specified in the plan, and the revision and moderation resolvers are both
/// written against this contract.
///
/// Note §9 lists "the op log and its SQLite projection" as Phase 1 work, so the
/// in-memory-only implementation is half of a Phase 1 item deliberately
/// deferred, not something the plan scheduled later.
///
/// The argument and what would reverse it are in the change's `design.md`.
///
/// # Every method is defined over a partial set
///
/// Nothing here reports "I do not know". §3.3's different-op-sets case is the
/// normal case, so a read that finds nothing found nothing, and that is the
/// answer rather than a failure to answer.
pub trait OpLog {
    /// Record an op and what the transport said about its arrival.
    ///
    /// **Stores whatever it is given.** No signature check, no authority check,
    /// no content check. See this module's documentation for why each of those
    /// would be a defect rather than a hardening.
    ///
    /// **Idempotent by [`OpId`]** (§3.1). The same op reaches a peer more than
    /// once by ordinary means — retransmission, causal-history backfill,
    /// SDS-Repair — so a second append is expected traffic and not an error.
    ///
    /// **The first arrival's metadata is kept.** A second arrival of one op
    /// carries metadata that is just as truthful as the first's, so something
    /// must choose, and choosing the first is what keeps the read order stable:
    /// a thread already rendered does not reorder because a duplicate arrived.
    /// Choosing the last would make a peer's order depend on how many times each
    /// op happened to reach it, which differs per peer — the divergence
    /// [`arrival`](crate::arrival) exists to prevent, reintroduced at the store.
    fn append(&mut self, op: SignedOp, arrival: Arrival) -> Appended;

    /// One op by its id, or absence.
    ///
    /// Absence is a defined answer, not an error: the op may simply not have
    /// reached this peer yet.
    fn get(&self, id: &OpId) -> Option<&Entry>;

    /// Every op, in [`cmp_ops`] order.
    ///
    /// **This is replay.** §3.3's "a cache that can be rebuilt by replay" is a
    /// fold over exactly this sequence, so there is no separate `replay()` verb
    /// — it would be a second name for one job, and CLAUDE.md asks for one
    /// function, one job.
    fn iter(&self) -> Vec<&Entry>;

    /// Every op in one Stoa, in [`cmp_ops`] order.
    ///
    /// The Stoa address, never a channel id (§4.5): "never let channel identity
    /// leak into payloads or storage keys", so that per-thread channels later
    /// become a routing change rather than a migration.
    fn iter_stoa(&self, stoa: &Address) -> Vec<&Entry>;

    /// Every op naming `target`, in [`cmp_ops`] order.
    ///
    /// The shape both resolvers fold over. A revision resolver reads this and
    /// keeps the first entry that is a `Revise` by the target's author; a
    /// moderation resolver reads it and keeps the first entry that is a
    /// `Moderate` by a then-moderator. Both want "the ops about this subject, in
    /// the order that decides which is current", which is one question and so is
    /// one method.
    ///
    /// **Taking the first entry is not the same as taking the most recent.**
    /// [`cmp_ops`] leads with the highest Lamport timestamp only when the
    /// transport supplied one; otherwise — which is every op today — it falls
    /// back to *ascending op id*, an order carrying no recency whatever. Take the
    /// first entry because that is the position the ordering rule defines as
    /// current, not because this promises recency it cannot currently deliver.
    /// See [`crate::arrival::cmp_ops`], which states which way each branch runs.
    ///
    /// An op is never its own target: this returns the ops acting *on* `target`,
    /// not `target` itself.
    fn iter_target(&self, target: &OpId) -> Vec<&Entry>;

    /// How many distinct ops the log holds.
    fn len(&self) -> usize;

    /// Whether the log holds no ops.
    ///
    /// Present because clippy's `len_without_is_empty` requires it alongside
    /// [`OpLog::len`]. No caller needs it today.
    ///
    /// Note it answers only "has this peer seen anything at all" — "is this Stoa
    /// quiet" is `iter_stoa(..).is_empty()`, which is a different question this
    /// cannot stand in for.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The op log in memory.
///
/// **The Phase 1 implementation and the Phase 1 fake, which are the same thing.**
/// §9 asks for "fakes with no node running"; a peer configured without
/// persistence is a real peer, and its log is this. Writing the fake as the
/// implementation means the resolvers' tests exercise the code a peer runs,
/// rather than a double that can drift from it.
///
/// # Why a map and not a sorted structure
///
/// Ops are held keyed by id and sorted on read. The alternative — an ordered
/// structure maintained on insert — is worse here for a specific reason rather
/// than on general principle: the sort key is `(Arrival, OpId)`, and `Arrival`
/// is what the transport supplies. An ordered structure keyed on it would place
/// an op by the metadata available at insert time and would need to be re-keyed
/// if that ever changed. Sorting on read makes the order a pure function of the
/// current contents, which is the property that has to hold whatever else moves.
///
/// A peer's log is bounded by what it has received and is read far less often
/// than a rendered forum implies (§3.3 puts the read traffic on the materialised
/// view, which is a later change and is where an index belongs).
#[derive(Debug, Default)]
pub struct MemoryOpLog {
    /// Keyed by op id, which is the dedup rule made structural: a second append
    /// of one op cannot produce a second entry, because there is one slot.
    ///
    /// CLAUDE.md's "put the complexity in the data structure, not the logic" —
    /// the alternative was a vector plus a contains-check before each push, which
    /// is a guard that has to be right at every insertion site.
    entries: HashMap<OpId, Entry>,
}

impl MemoryOpLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// The shared body of every ordered read.
    ///
    /// Written once so the three public reads cannot disagree about what "in
    /// order" means. A second call site spelling its own `sort_by` is how one of
    /// them eventually spells it differently.
    fn sorted(&self, keep: impl Fn(&Entry) -> bool) -> Vec<&Entry> {
        let mut out: Vec<&Entry> = self.entries.values().filter(|e| keep(e)).collect();
        // `sort_by` and not `sort_unstable_by`: both are correct here only
        // because `cmp_ops` is total over distinct ops, and `sort_by`'s
        // stability makes that a property this code does not depend on. Ops in
        // this map are distinct by construction, so the two agree — the choice
        // costs nothing and removes a way for a future non-total comparison to
        // produce a peer-dependent result.
        out.sort_by(|a, b| {
            cmp_ops(
                OpEntry::new(&a.arrival, &a.id()),
                OpEntry::new(&b.arrival, &b.id()),
            )
        });
        out
    }
}

impl OpLog for MemoryOpLog {
    fn append(&mut self, op: SignedOp, arrival: Arrival) -> Appended {
        let id = op.op.id();
        // `entry().or_insert()` rather than `contains_key` then `insert`: the
        // first-wins rule is then structural rather than a branch that a later
        // edit could invert. There is no code path here that overwrites an
        // existing entry, so no code path can replace recorded arrival metadata.
        match self.entries.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => Appended::AlreadyPresent,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Entry { op, arrival });
                Appended::Stored
            }
        }
    }

    fn get(&self, id: &OpId) -> Option<&Entry> {
        self.entries.get(id)
    }

    fn iter(&self) -> Vec<&Entry> {
        self.sorted(|_| true)
    }

    fn iter_stoa(&self, stoa: &Address) -> Vec<&Entry> {
        self.sorted(|e| &e.op.op.stoa == stoa)
    }

    fn iter_target(&self, target: &OpId) -> Vec<&Entry> {
        self.sorted(|e| e.target().as_ref() == Some(target))
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrival::MessageId;
    use crate::identity::{sign_op_bytes, SecretKey};
    use crate::op::{ModerationAction, Op, VoteDirection};
    use crate::stoa::{Genesis, Policy};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    fn a_stoa(title: &str) -> Address {
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

    fn a_post(body: &str) -> Op {
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

    fn signed(op: Op) -> SignedOp {
        op.sign(&a_key(2))
    }

    fn a_message_id(seed: u8) -> MessageId {
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
    fn every_op_kind() -> Vec<OpKind> {
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
    fn two_posts_by_ascending_id() -> (SignedOp, SignedOp) {
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

    // ─── The store decides nothing ────────────────────────────────────────

    #[test]
    fn an_op_with_an_invalid_signature_is_stored_anyway() {
        // §3.3: "The store may hold junk; the reader never trusts it." A log
        // that filtered here would make a forgery attempt indistinguishable
        // from an op that never arrived — and those call for different
        // responses.
        let victim = a_key(2);
        let attacker = a_key(3);
        let op = Op {
            author: victim.public_key(),
            ..a_post("forged")
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        // The fixture must genuinely be a forgery, or this test proves nothing.
        assert!(!forged.verify(), "the fixture must be an actual forgery");

        let mut log = MemoryOpLog::new();
        let id = forged.op.id();
        assert_eq!(
            log.append(forged.clone(), Arrival::unordered()),
            Appended::Stored
        );
        assert_eq!(log.get(&id).map(|e| &e.op), Some(&forged));
        assert_eq!(log.iter().len(), 1);
    }

    #[test]
    fn a_moderation_op_from_a_peer_with_no_authority_is_stored() {
        // Authenticity is not authority (§6, and `op.rs`'s two tests pinning the
        // distinction). The log holds a moderation op from a random peer exactly
        // as it holds one from a moderator, because deciding between them needs
        // the moderator set at that op's Lamport time — which the log does not
        // have and the resolver does.
        let random_peer = a_key(9);
        let op = Op {
            stoa: a_stoa("Agora"),
            author: random_peer.public_key(),
            kind: OpKind::Moderate {
                target: signed(a_post("victim")).op.id(),
                action: ModerationAction::Hide,
            },
        };
        let hide = op.sign(&random_peer);
        // Authentic — it really is from them — and carries no authority.
        assert!(hide.verify());

        let mut log = MemoryOpLog::new();
        let id = hide.op.id();
        log.append(hide, Arrival::unordered());
        assert!(
            log.get(&id).is_some(),
            "an unauthorised moderation is stored"
        );
    }

    #[test]
    fn a_stored_op_reads_back_byte_identical() {
        // The op is signed, so any mutation by the store would break the
        // signature — asserted directly rather than trusted.
        let mut log = MemoryOpLog::new();
        let op = signed(a_post("exact"));
        let id = op.op.id();
        log.append(op.clone(), Arrival::ordered(5, a_message_id(3)));

        let entry = log.get(&id).unwrap();
        assert_eq!(entry.op, op);
        assert_eq!(entry.op.to_bytes(), op.to_bytes());
        assert_eq!(entry.id(), id);
        assert!(entry.op.verify(), "storage must not disturb the signature");
    }

    // ─── Dedup by op id ───────────────────────────────────────────────────

    #[test]
    fn the_same_op_appended_twice_is_one_entry() {
        // §3.1: ops are "idempotent by `opId`". Retransmission, causal-history
        // backfill and SDS-Repair all deliver ops a peer may already hold, so a
        // second append is ordinary traffic.
        let mut log = MemoryOpLog::new();
        let op = signed(a_post("once"));
        assert_eq!(
            log.append(op.clone(), Arrival::unordered()),
            Appended::Stored
        );
        assert_eq!(
            log.append(op.clone(), Arrival::unordered()),
            Appended::AlreadyPresent
        );
        assert_eq!(log.len(), 1);
        assert_eq!(log.iter().len(), 1);
    }

    #[test]
    fn the_append_result_says_whether_the_op_was_new() {
        // Hardcoded expectations, not a count of the log before and after: a
        // caller acts on this (gossip onward, rebuild a view) and must not have
        // to derive it.
        let mut log = MemoryOpLog::new();
        let one = signed(a_post("one"));
        let two = signed(a_post("two"));
        assert_eq!(
            log.append(one.clone(), Arrival::unordered()),
            Appended::Stored
        );
        assert_eq!(log.append(two, Arrival::unordered()), Appended::Stored);
        assert_eq!(
            log.append(one, Arrival::unordered()),
            Appended::AlreadyPresent
        );
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn two_ops_differing_in_any_field_are_two_entries() {
        // Dedup must be by the WHOLE op, not by author, Stoa or kind. A log that
        // keyed on anything coarser would silently drop a second post.
        let mut log = MemoryOpLog::new();
        let one = signed(a_post("first"));
        let two = signed(a_post("second"));
        let (one_id, two_id) = (one.op.id(), two.op.id());
        assert_ne!(one_id, two_id);

        log.append(one.clone(), Arrival::unordered());
        log.append(two.clone(), Arrival::unordered());
        assert_eq!(log.len(), 2);
        assert_eq!(log.get(&one_id).map(|e| &e.op), Some(&one));
        assert_eq!(log.get(&two_id).map(|e| &e.op), Some(&two));
    }

    #[test]
    fn dedup_is_by_op_id_and_not_by_signature() {
        // Two DIFFERENT ops from one author share no id; one op signed twice
        // shares one id. Ed25519 signatures here are deterministic, so this
        // pins the key rather than the signature bytes: a log keyed on the
        // signature would behave identically today and diverge the moment a
        // randomised scheme or a re-signed op appeared.
        let mut log = MemoryOpLog::new();
        let op = a_post("same bytes");
        let first = op.clone().sign(&a_key(2));
        let second = op.sign(&a_key(2));
        assert_eq!(first.op.id(), second.op.id(), "the fixture is one op");

        log.append(first, Arrival::unordered());
        assert_eq!(
            log.append(second, Arrival::unordered()),
            Appended::AlreadyPresent
        );
        assert_eq!(log.len(), 1);
    }

    // ─── The first arrival's metadata wins ────────────────────────────────

    #[test]
    fn a_second_arrival_does_not_overwrite_the_recorded_metadata() {
        // Hardcoded: the log must report 5, the value recorded FIRST, and not 9.
        // Keeping the last would make a peer's order depend on how many times
        // each op happened to reach it, which differs per peer — exactly the
        // divergence `arrival.rs` exists to prevent, reintroduced at the store.
        let mut log = MemoryOpLog::new();
        let op = signed(a_post("re-delivered"));
        let id = op.op.id();

        log.append(op.clone(), Arrival::ordered(5, a_message_id(1)));
        log.append(op, Arrival::ordered(9, a_message_id(2)));

        let entry = log.get(&id).unwrap();
        assert_eq!(entry.arrival.lamport(), Some(5));
        assert_eq!(entry.arrival.message_id(), Some(&a_message_id(1)));
    }

    #[test]
    fn a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one() {
        // THE test that separates first-wins from richer-wins, and the only
        // direction that does: unordered FIRST, then ordered.
        //
        // Every other re-arrival test here delivers the ordered copy first, and
        // in that direction first-wins and richer-wins agree — so the rule both
        // this module's docs and `arrival.rs` name as "the tempting wrong
        // answer" passed the whole suite without a single failure until this
        // existed. A rule two modules warn about with no test that can catch it
        // is a rule the next refactor walks straight into.
        //
        // Richer-wins is wrong because whether a peer ever receives the richer
        // copy is a per-peer accident: two peers holding the same ops would
        // record different metadata and order them differently, with no error.
        // Convergence beats completeness, the same trade the degraded order
        // makes.
        let mut log = MemoryOpLog::new();
        let op = signed(a_post("poor first, rich second"));
        let id = op.op.id();

        log.append(op.clone(), Arrival::unordered());
        log.append(op, Arrival::ordered(7, a_message_id(1)));

        let entry = log.get(&id).unwrap();
        assert_eq!(
            entry.arrival.lamport(),
            None,
            "first-wins keeps the poorer arrival; this is richer-wins"
        );
        assert_eq!(entry.arrival.message_id(), None);
        assert!(!entry.arrival.is_ordered_by_transport());
    }

    #[test]
    fn an_unordered_re_arrival_does_not_erase_a_recorded_order() {
        // Ordered first, then nothing. This is the BLUNT direction — a naive
        // "last write wins" fails it, but richer-wins agrees with it, so it
        // cannot distinguish first-wins from richer-wins on its own. The
        // direction that can is
        // `a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one`.
        //
        // Kept because last-write-wins is a real mistake and this is what
        // catches it: it would demote an ordered op to the degraded order and
        // move it in every thread it appears in.
        let mut log = MemoryOpLog::new();
        let op = signed(a_post("ordered once"));
        let id = op.op.id();

        log.append(op.clone(), Arrival::ordered(7, a_message_id(1)));
        log.append(op, Arrival::unordered());

        let entry = log.get(&id).unwrap();
        assert_eq!(entry.arrival.lamport(), Some(7));
        assert!(entry.arrival.is_ordered_by_transport());
    }

    #[test]
    fn a_re_arrival_does_not_move_the_op_in_the_read_order() {
        // The consequence at the level that matters: a thread already rendered
        // must not reorder because a duplicate arrived. Hardcoded expected
        // sequence, both before and after.
        // Lamport values assigned against op-id order, so a log ignoring the
        // metadata would fail the `before` assertion rather than agree with it.
        let (low, high) = two_posts_by_ascending_id();

        let mut log = MemoryOpLog::new();
        log.append(low.clone(), Arrival::ordered(1, a_message_id(1)));
        log.append(high.clone(), Arrival::ordered(2, a_message_id(1)));

        let before: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(before, vec![high.op.id(), low.op.id()]);

        // Re-deliver the low op claiming a Lamport value that would put it first.
        log.append(low.clone(), Arrival::ordered(99, a_message_id(1)));

        let after: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(after, vec![high.op.id(), low.op.id()], "the order moved");
    }

    #[test]
    fn one_op_arriving_twice_with_different_metadata_is_still_one_entry() {
        // THE case that would otherwise reach `cmp_ops` as an undefined tie.
        //
        // `cmp_ops` is total over DISTINCT ops; two entries sharing an op id but
        // carrying different `Arrival` metadata compare Equal — correctly, since
        // §5.7's rule has nothing to say about one op against itself. A sort over
        // such a pair would leave their relative order to the sort's stability,
        // which differs with the sequence a peer received in. Two honest peers
        // would then render one thread differently, with no error anywhere.
        //
        // Dedup must therefore happen BEFORE any sort, never as a pass over
        // sorted output. Here it is structural: entries are keyed by op id, so
        // the pair cannot be built. This asserts that it is not.
        let mut log = MemoryOpLog::new();
        let op = signed(a_post("arrives twice"));
        let id = op.op.id();

        // Deliberately the exact pair the probe found: unordered, versus a
        // message id with no Lamport value. Both are unordered by the transport,
        // so both fall through to the op-id comparison and tie.
        let plain = Arrival::unordered();
        let with_id = Arrival::from_parts(None, Some(a_message_id(1)));
        assert_ne!(plain, with_id, "the fixture needs two different arrivals");

        log.append(op.clone(), plain.clone());
        log.append(op, with_id);

        assert_eq!(log.len(), 1, "one op id is one entry");
        assert_eq!(log.iter().len(), 1, "no duplicate reaches the sort");
        // And the first-wins rule decided which arrival survived.
        assert_eq!(log.get(&id).unwrap().arrival, plain);
    }

    #[test]
    fn no_two_entries_in_a_read_ever_share_an_op_id() {
        // The general form of the property above, over a log built entirely from
        // re-deliveries with varying metadata. If any read could emit two
        // entries with one op id, the sort that produced it consulted an
        // undefined order.
        let mut log = MemoryOpLog::new();
        let ops = [
            signed(a_post("a")),
            signed(a_post("b")),
            signed(a_post("c")),
        ];
        let arrivals = [
            Arrival::unordered(),
            Arrival::from_parts(None, Some(a_message_id(1))),
            Arrival::ordered(3, a_message_id(2)),
            Arrival::from_parts(Some(3), None),
        ];
        // Every op delivered under every arrival: 12 appends, 3 distinct ops.
        for arrival in arrivals.iter() {
            for op in ops.iter() {
                log.append(op.clone(), arrival.clone());
            }
        }
        assert_eq!(log.len(), 3);

        let mut ids: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(ids.len(), 3);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 3, "a read emitted two entries with one op id");
    }

    // ─── The read order is the ordering rule's ────────────────────────────

    #[test]
    fn reading_returns_ops_in_lamport_order_not_insertion_order() {
        // Hardcoded expected sequence. The ops are appended in exactly the
        // reverse of the order that must come out, so a log returning insertion
        // order fails rather than coincidentally agreeing.
        //
        // The Lamport values are assigned AGAINST op-id order — the lower op id
        // gets the lower timestamp — so a read that fell back to ordering by op
        // id would return the reverse of what is asserted here. Without that,
        // this test would pass for a log that consulted no metadata at all, on
        // whatever order the fixture's hashes happened to land in.
        let (low_id, high_id) = two_posts_by_ascending_id();

        let mut log = MemoryOpLog::new();
        log.append(low_id.clone(), Arrival::ordered(1, a_message_id(1)));
        log.append(high_id.clone(), Arrival::ordered(2, a_message_id(1)));

        let ids: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![high_id.op.id(), low_id.op.id()],
            "highest Lamport must read first, against op-id order"
        );
    }

    #[test]
    fn the_message_id_tiebreak_is_used_and_is_not_the_op_id() {
        // Within one Lamport value, §5.7 breaks ties by ASCENDING message id.
        // The message ids are assigned against op-id order for the same reason
        // as above: a log that ignored the arrival and sorted by op id would
        // return the reverse.
        let (low_id, high_id) = two_posts_by_ascending_id();

        let mut log = MemoryOpLog::new();
        // The op with the LOWER op id carries the HIGHER message id.
        log.append(low_id.clone(), Arrival::ordered(7, a_message_id(9)));
        log.append(high_id.clone(), Arrival::ordered(7, a_message_id(1)));

        let ids: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![high_id.op.id(), low_id.op.id()],
            "the lower message id reads first, against op-id order"
        );
    }

    #[test]
    fn two_logs_with_the_same_ops_read_the_same_order() {
        // §3.3's convergence property at the store. Two peers received the same
        // ops in opposite sequences; both must render identically. A log that
        // ordered by insertion would return reversed sequences here.
        let ops = [
            (signed(a_post("a")), Arrival::ordered(2, a_message_id(1))),
            (signed(a_post("b")), Arrival::ordered(1, a_message_id(1))),
            (signed(a_post("c")), Arrival::unordered()),
            (signed(a_post("d")), Arrival::ordered(2, a_message_id(0))),
        ];

        let mut forwards = MemoryOpLog::new();
        for (op, arrival) in ops.iter() {
            forwards.append(op.clone(), arrival.clone());
        }
        let mut backwards = MemoryOpLog::new();
        for (op, arrival) in ops.iter().rev() {
            backwards.append(op.clone(), arrival.clone());
        }

        let a: Vec<OpId> = forwards.iter().iter().map(|e| e.id()).collect();
        let b: Vec<OpId> = backwards.iter().iter().map(|e| e.id()).collect();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4, "the fixture must exercise all four");
    }

    #[test]
    fn ops_the_transport_did_not_order_read_in_ascending_op_id() {
        // The degraded order, which is the ONLY order in production today. The
        // expected sequence is derived from the op ids alone — if anything
        // peer-local leaked into the read, the two logs would disagree.
        let (low, high) = two_posts_by_ascending_id();

        let mut log = MemoryOpLog::new();
        log.append(high.clone(), Arrival::unordered());
        log.append(low.clone(), Arrival::unordered());

        let ids: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![low.op.id(), high.op.id()]);
    }

    #[test]
    fn ordered_and_unordered_ops_coexist_with_the_ordered_ones_first() {
        // The mixture the transport fix will produce: a peer's existing log is
        // all unordered, and new arrivals carry Lamport values. Both must live
        // in one log, and the ordering rule places the ordered ones first
        // whatever their value.
        // The ORDERED op is given the HIGHER op id, so a log that ignored the
        // metadata and fell back to op id would put the unordered one first and
        // fail. Lamport 0 is the sharp value: it must still beat "no metadata".
        let (unordered, ordered_zero) = two_posts_by_ascending_id();

        let mut log = MemoryOpLog::new();
        log.append(unordered.clone(), Arrival::unordered());
        log.append(ordered_zero.clone(), Arrival::ordered(0, a_message_id(1)));

        let ids: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![ordered_zero.op.id(), unordered.op.id()],
            "even Lamport 0 beats an op the transport did not order"
        );
    }

    #[test]
    fn whether_an_arrival_was_ordered_survives_storage() {
        // The seam for the upstream fix: a peer must be able to say "this thread
        // is ordered by the network" or "by op id". A store that dropped the
        // distinction could not.
        let mut log = MemoryOpLog::new();
        let ordered = signed(a_post("ordered"));
        let unordered = signed(a_post("unordered"));
        log.append(ordered.clone(), Arrival::ordered(1, a_message_id(1)));
        log.append(unordered.clone(), Arrival::unordered());

        assert!(log
            .get(&ordered.op.id())
            .unwrap()
            .arrival
            .is_ordered_by_transport());
        assert!(!log
            .get(&unordered.op.id())
            .unwrap()
            .arrival
            .is_ordered_by_transport());
    }

    // ─── Restricted reads ─────────────────────────────────────────────────

    #[test]
    fn a_stoa_restricted_read_excludes_other_stoas() {
        // §4.5 keys on the Stoa ADDRESS, never a channel id. Hardcoded: exactly
        // the one op from the requested Stoa.
        let agora = a_stoa("Agora");
        let lyceum = a_stoa("Lyceum");
        assert_ne!(agora, lyceum);

        let here = signed(a_post("in the agora"));
        let there = signed(Op {
            stoa: lyceum,
            ..a_post("in the lyceum")
        });

        let mut log = MemoryOpLog::new();
        log.append(here.clone(), Arrival::unordered());
        log.append(there.clone(), Arrival::unordered());

        let ids: Vec<OpId> = log.iter_stoa(&agora).iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![here.op.id()]);
        let ids: Vec<OpId> = log.iter_stoa(&lyceum).iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![there.op.id()]);
        assert_eq!(log.iter().len(), 2, "both are in the unrestricted read");
    }

    #[test]
    fn two_stoas_sharing_an_address_prefix_are_not_confused() {
        // Whole-key matching, not merely "some filtering happens".
        //
        // `a_stoa_restricted_read_excludes_other_stoas` uses two Stoas whose
        // addresses differ in the first byte, so a read comparing only a PREFIX
        // passes it. This one cannot be passed that way: the two addresses are
        // chosen to COLLIDE in byte 0 and differ only later.
        //
        // Why it matters here specifically: a prefix-matching `iter_stoa` is a
        // cross-Stoa leak in a censorship-resistant forum — another Stoa's ops
        // bleeding into this Stoa's read. With 32-byte addresses a collision is
        // rare enough never to surface in casual testing and certain enough to
        // surface eventually, which is the worst combination.
        let (one, two) = ("S0", "S178");
        let (addr_one, addr_two) = (a_stoa(one), a_stoa(two));

        // FIXTURE GUARD. These addresses are hash-derived, so the collision is a
        // property of the current genesis encoding rather than of the titles. If
        // an encoding change breaks it, this test must fail loudly here rather
        // than silently stop exercising prefix confusion — the "a new check can
        // retire an old test" hazard, applied to our own fixture.
        assert_eq!(
            addr_one.as_bytes()[0],
            addr_two.as_bytes()[0],
            "fixture must share a first byte, or it tests nothing"
        );
        assert_ne!(addr_one, addr_two, "fixture must be two distinct Stoas");

        let here = signed(Op {
            stoa: addr_one,
            ..a_post("in the first")
        });
        let there = signed(Op {
            stoa: addr_two,
            ..a_post("in the second")
        });

        let mut log = MemoryOpLog::new();
        log.append(here.clone(), Arrival::unordered());
        log.append(there.clone(), Arrival::unordered());

        let ids: Vec<OpId> = log.iter_stoa(&addr_one).iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![here.op.id()], "a prefix match leaked a Stoa");
        let ids: Vec<OpId> = log.iter_stoa(&addr_two).iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![there.op.id()]);
    }

    #[test]
    fn two_targets_sharing_an_op_id_prefix_are_not_confused() {
        // The same property for `iter_target`, and the consequence is worse: a
        // moderation resolver handed ops aimed at a DIFFERENT post would apply
        // one post's `Hide` to an unrelated post. Silent, and invisible until
        // two op ids happen to collide.
        let (one, two) = ("p0", "p37");
        let (post_one, post_two) = (signed(a_post(one)), signed(a_post(two)));
        let (target_one, target_two) = (post_one.op.id(), post_two.op.id());

        // FIXTURE GUARD, for the same reason as above: op ids are hashes.
        assert_eq!(
            target_one.as_bytes()[0],
            target_two.as_bytes()[0],
            "fixture must share a first byte, or it tests nothing"
        );
        assert_ne!(target_one, target_two, "fixture must be two distinct ops");

        let author = a_key(2);
        let moderate_one = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Moderate {
                target: target_one,
                action: ModerationAction::Hide,
            },
        }
        .sign(&author);
        let moderate_two = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Moderate {
                target: target_two,
                action: ModerationAction::Unhide,
            },
        }
        .sign(&author);

        let mut log = MemoryOpLog::new();
        for op in [
            post_one,
            post_two,
            moderate_one.clone(),
            moderate_two.clone(),
        ] {
            log.append(op, Arrival::unordered());
        }

        let ids: Vec<OpId> = log
            .iter_target(&target_one)
            .iter()
            .map(|e| e.id())
            .collect();
        assert_eq!(
            ids,
            vec![moderate_one.op.id()],
            "a prefix match leaked another post's moderation"
        );
        let ids: Vec<OpId> = log
            .iter_target(&target_two)
            .iter()
            .map(|e| e.id())
            .collect();
        assert_eq!(ids, vec![moderate_two.op.id()]);
    }

    #[test]
    fn a_non_empty_log_read_against_an_absent_stoa_is_empty() {
        // The empty-log case is covered elsewhere; this is the populated one,
        // where a read that ignored its argument would return everything rather
        // than nothing.
        let mut log = MemoryOpLog::new();
        log.append(signed(a_post("in the agora")), Arrival::unordered());

        let elsewhere = a_stoa("Never Used");
        assert_eq!(log.iter_stoa(&elsewhere).len(), 0);
        assert_eq!(log.len(), 1, "the log is genuinely non-empty");
    }

    #[test]
    fn nothing_is_filtered_on_the_way_in() {
        // Split out of `storing_adversarial_ops_never_panics`, where this count
        // sat at the bottom of a 70-line test whose name advertised only
        // panic-freedom. It is load-bearing — it is what catches an append that
        // quietly drops a kind, such as a "validate early" filter on `Moderate`
        // — and a comment cannot defend an assertion from a well-intentioned
        // edit the way a name can. Anyone shrinking that fixture would have
        // adjusted the count without realising what they deleted.
        let author = a_key(2);
        let stoa = a_stoa("Agora");
        let mut log = MemoryOpLog::new();
        for kind in every_op_kind() {
            let op = Op {
                stoa,
                author: author.public_key(),
                kind,
            }
            .sign(&author);
            log.append(op, Arrival::unordered());
        }
        // Hardcoded rather than `every_op_kind().len()`, which would agree with
        // the fixture however wrong the fixture became. Update both together
        // when an op kind lands.
        assert_eq!(log.len(), 6, "an op kind was filtered on the way in");
    }

    #[test]
    fn a_target_restricted_read_returns_every_kind_that_names_the_target() {
        // The shape both resolvers fold over. A revise, a moderate and a vote on
        // one post must all come back, because "the ops about this subject" is
        // one question — a resolver narrows what it is given.
        let post = signed(a_post("the subject"));
        let target = post.op.id();
        let author = a_key(2);

        let revise = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Revise {
                target,
                body: "edited".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author);
        let moderate = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
        }
        .sign(&author);
        let vote = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Vote {
                target,
                direction: VoteDirection::Up,
            },
        }
        .sign(&author);

        let mut log = MemoryOpLog::new();
        for op in [post.clone(), revise.clone(), moderate.clone(), vote.clone()] {
            log.append(op, Arrival::unordered());
        }

        let mut got: Vec<OpId> = log.iter_target(&target).iter().map(|e| e.id()).collect();
        let mut expected = vec![revise.op.id(), moderate.op.id(), vote.op.id()];
        got.sort();
        expected.sort();
        assert_eq!(got, expected, "all three kinds naming the target");
        assert_eq!(log.len(), 4, "the post itself is still stored");
    }

    #[test]
    fn a_post_names_no_target_and_is_never_returned_by_a_target_read() {
        // A reply's `parent` is a reply relationship, not a subject acted upon.
        // Conflating them would make a moderation resolver see every reply to a
        // post alongside the moderations of it.
        let parent = signed(a_post("the parent"));
        let parent_id = parent.op.id();
        let reply = signed(Op {
            kind: OpKind::Post {
                thread: Some(parent_id),
                parent: Some(parent_id),
                body: "a reply".to_string(),
                attachments: vec![],
            },
            ..a_post("unused")
        });

        let mut log = MemoryOpLog::new();
        log.append(parent, Arrival::unordered());
        log.append(reply.clone(), Arrival::unordered());

        assert_eq!(
            log.iter_target(&parent_id).len(),
            0,
            "a reply is not an op acting on its parent"
        );
        assert_eq!(log.len(), 2, "both are stored");
    }

    #[test]
    fn a_stoa_metadata_op_names_no_target() {
        // A metadata op acts on the Stoa, which it names through `Op::stoa`
        // rather than as a target op id, so no target-restricted read should
        // return it. Pinned because `Entry::target` returning `None` for a kind
        // that should name one fails silently: the op is simply never returned,
        // and no test unaware of the kind notices.
        //
        // It IS stored and IS returned by a Stoa read — the log holds it, it
        // just is not about another op.
        let author = a_key(2);
        let stoa = a_stoa("Agora");
        let post = signed(a_post("some post"));
        let metadata = Op {
            stoa,
            author: author.public_key(),
            kind: OpKind::StoaMetadata {
                title: "Renamed".to_string(),
                description: "now with a description".to_string(),
            },
        }
        .sign(&author);

        let mut log = MemoryOpLog::new();
        log.append(post.clone(), Arrival::unordered());
        log.append(metadata.clone(), Arrival::unordered());

        assert_eq!(
            log.iter_target(&post.op.id()).len(),
            0,
            "a metadata op does not act on another op"
        );
        let ids: Vec<OpId> = log.iter_stoa(&stoa).iter().map(|e| e.id()).collect();
        assert!(
            ids.contains(&metadata.op.id()),
            "but it is stored and reachable by Stoa"
        );
    }

    #[test]
    fn a_target_read_does_not_return_the_target_itself() {
        // An op is never its own target: `iter_target` returns the ops acting ON
        // it. A log that included the subject would make every resolver's fold
        // start on an entry of the wrong kind.
        let post = signed(a_post("the subject"));
        let target = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());
        assert_eq!(log.iter_target(&target).len(), 0);
    }

    #[test]
    fn a_restricted_read_preserves_the_unrestricted_relative_order() {
        // A resolver folds over a restricted read and takes the first match, so
        // the restriction must not reorder. Hardcoded expected sequence.
        //
        // "First" means current under `cmp_ops`, which is most-recent only on
        // the ORDERED branch — the one this test's fixture uses, and the one
        // production does not reach today. The degraded branch is ascending op
        // id and carries no recency; `arrival.rs` says so explicitly after the
        // moderation resolver read recency into it.
        let post = signed(a_post("the subject"));
        let target = post.op.id();
        let author = a_key(2);
        let older = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Revise {
                target,
                body: "v2".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author);
        let newer = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Revise {
                target,
                body: "v3".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(older.clone(), Arrival::ordered(2, a_message_id(1)));
        log.append(newer.clone(), Arrival::ordered(3, a_message_id(1)));

        let ids: Vec<OpId> = log.iter_target(&target).iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![newer.op.id(), older.op.id()],
            "the current version reads first"
        );
    }

    // ─── A partial set is the normal case ─────────────────────────────────

    #[test]
    fn every_read_over_an_empty_log_is_empty_and_not_an_error() {
        // §3.3: a peer cannot establish that its set is complete, so a read that
        // needed completeness could never answer at all.
        let log = MemoryOpLog::new();
        assert_eq!(log.len(), 0);
        assert!(log.is_empty());
        assert_eq!(log.iter().len(), 0);
        assert_eq!(log.iter_stoa(&a_stoa("Agora")).len(), 0);
        assert_eq!(log.iter_target(&signed(a_post("absent")).op.id()).len(), 0);
        assert_eq!(log.get(&signed(a_post("absent")).op.id()), None);
    }

    #[test]
    fn a_revision_whose_target_is_absent_is_stored_and_readable() {
        // The dangling-reference case, which is ordinary: the target may simply
        // not have propagated yet. A store that required the target to be
        // present would drop ops that arrive out of order and never recover
        // them.
        let missing_target = signed(a_post("never received")).op.id();
        let author = a_key(2);
        let orphan = Op {
            stoa: a_stoa("Agora"),
            author: author.public_key(),
            kind: OpKind::Revise {
                target: missing_target,
                body: "revises something we lack".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author);

        let mut log = MemoryOpLog::new();
        log.append(orphan.clone(), Arrival::unordered());

        assert_eq!(
            log.get(&missing_target),
            None,
            "the target really is absent"
        );
        assert_eq!(log.iter().len(), 1);
        let ids: Vec<OpId> = log
            .iter_target(&missing_target)
            .iter()
            .map(|e| e.id())
            .collect();
        assert_eq!(ids, vec![orphan.op.id()]);
    }

    #[test]
    fn a_reply_whose_parent_is_absent_is_stored() {
        let mut log = MemoryOpLog::new();
        let orphan_parent = signed(a_post("never received")).op.id();
        let reply = signed(Op {
            kind: OpKind::Post {
                thread: Some(orphan_parent),
                parent: Some(orphan_parent),
                body: "a reply to nothing we hold".to_string(),
                attachments: vec![],
            },
            ..a_post("unused")
        });
        let id = reply.op.id();
        assert_eq!(log.append(reply, Arrival::unordered()), Appended::Stored);
        assert!(log.get(&id).is_some());
    }

    // ─── Hostile input ────────────────────────────────────────────────────

    #[test]
    fn storing_adversarial_ops_never_panics() {
        // PHASE0-FINDINGS §3: a panic ABORTS the module process, so a malformed
        // op would become a denial of service against the peer that received it.
        // Everything stored arrived from a peer.
        let author = a_key(2);
        let stoa = a_stoa("Agora");
        let mut log = MemoryOpLog::new();

        let kinds = every_op_kind();
        let arrivals = [
            Arrival::unordered(),
            Arrival::ordered(0, MessageId::new(vec![])),
            Arrival::ordered(u64::MAX, MessageId::new(vec![0xFF; 1024])),
            Arrival::from_parts(None, Some(MessageId::new(vec![1]))),
            Arrival::from_parts(Some(1), None),
        ];

        for kind in kinds {
            for arrival in arrivals.iter() {
                let op = Op {
                    stoa,
                    author: author.public_key(),
                    kind: kind.clone(),
                }
                .sign(&author);
                let id = op.op.id();
                log.append(op, arrival.clone());
                let _ = log.get(&id);
                let _ = log.iter();
                let _ = log.iter_stoa(&stoa);
                let _ = log.iter_target(&id);
                let _ = log.len();
            }
        }
        // No count assertion here on purpose. That check is a different job and
        // now has its own name — `nothing_is_filtered_on_the_way_in` — because
        // it used to sit at the bottom of this test, where anyone shrinking the
        // fixture would reasonably have adjusted the number without realising
        // they were deleting a filter check. This test is about panics only.
    }

    #[test]
    fn looking_up_an_absent_op_is_a_defined_absence() {
        let mut log = MemoryOpLog::new();
        log.append(signed(a_post("present")), Arrival::unordered());
        assert_eq!(log.get(&signed(a_post("absent")).op.id()), None);
        assert_eq!(log.get(&OpId::from_hex(&"00".repeat(32)).unwrap()), None);
    }

    // ─── What the trait buys, exercised through the trait ─────────────────

    #[test]
    fn a_resolver_can_be_written_against_the_trait_alone() {
        // NO SPEC: the spec requires a read API the resolvers can fold over; it
        // does not require that they be written generically over the trait
        // rather than against the concrete type. Chosen: generic, and pinned
        // here, because the SQLite implementation §3.3 names is the reason the
        // trait exists — a resolver that named `MemoryOpLog` would have to be
        // rewritten rather than relinked.
        //
        // This stands in for both real resolvers: it is the fold each performs,
        // over the read each uses.
        fn latest_moderation<L: OpLog>(log: &L, target: &OpId) -> Option<ModerationAction> {
            log.iter_target(target)
                .into_iter()
                .find_map(|e| match &e.op.op.kind {
                    OpKind::Moderate { action, .. } => Some(*action),
                    _ => None,
                })
        }

        let post = signed(a_post("the subject"));
        let target = post.op.id();
        let moderator = a_key(1);
        let hide = Op {
            stoa: a_stoa("Agora"),
            author: moderator.public_key(),
            kind: OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
        }
        .sign(&moderator);
        let unhide = Op {
            stoa: a_stoa("Agora"),
            author: moderator.public_key(),
            kind: OpKind::Moderate {
                target,
                action: ModerationAction::Unhide,
            },
        }
        .sign(&moderator);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(hide, Arrival::ordered(2, a_message_id(1)));
        log.append(unhide, Arrival::ordered(3, a_message_id(1)));

        // Last write wins by Lamport order (§5.7): the unhide is current.
        assert_eq!(
            latest_moderation(&log, &target),
            Some(ModerationAction::Unhide)
        );
    }

    #[test]
    fn an_entry_reports_the_target_its_kind_names() {
        // `Entry::target` is what `iter_target` filters on, so its mapping from
        // kind to target is pinned directly rather than only through the filter.
        let target = OpId::from_hex(&"7a".repeat(32)).unwrap();
        let author = a_key(2);
        let stoa = a_stoa("Agora");
        let cases = [
            (
                OpKind::Post {
                    thread: Some(target),
                    parent: Some(target),
                    body: "x".to_string(),
                    attachments: vec![],
                },
                None,
            ),
            (
                OpKind::Revise {
                    target,
                    body: "x".to_string(),
                    attachments: vec![],
                },
                Some(target),
            ),
            (
                OpKind::Moderate {
                    target,
                    action: ModerationAction::Hide,
                },
                Some(target),
            ),
            (
                OpKind::Vote {
                    target,
                    direction: VoteDirection::Up,
                },
                Some(target),
            ),
            (
                OpKind::StoaMetadata {
                    title: "t".to_string(),
                    description: "d".to_string(),
                },
                None,
            ),
        ];
        for (kind, expected) in cases {
            let entry = Entry {
                op: Op {
                    stoa,
                    author: author.public_key(),
                    kind,
                }
                .sign(&author),
                arrival: Arrival::unordered(),
            };
            assert_eq!(entry.target(), expected);
        }
    }
}
