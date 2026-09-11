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

pub mod sqlite;

pub use sqlite::SqliteOpLog;

use crate::arrival::{cmp_ops, Arrival, OpEntry};
use crate::identity::Address;
use crate::op::{OpId, OpKind, SignedOp};
use std::collections::HashMap;
use std::fmt;

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

/// What went wrong reaching the store — never what the store held.
///
/// What went wrong reaching the store — never what the store held.
///
/// # Why the trait is fallible when the only implementation cannot fail
///
/// [`MemoryOpLog`] returns `Ok` unconditionally, so a reader could reasonably
/// ask what this buys. Two answers, and the second is the load-bearing one:
///
/// 1. §3.3 specifies a store on disk, and a disk fails for reasons that have
///    nothing to do with the ops on it — it fills, a permission is revoked, a
///    file was written by a version that is not this one.
/// 2. **A panic aborts the module process.** PHASE0-FINDINGS §3 measured it: the
///    caller is told `timeout` after 20 seconds, the next call reports
///    `MODULE_NOT_LOADED`, and the word "panic" appears only in a daemon log. An
///    `unwrap` on a disk error is therefore a denial of service against the
///    peer, reachable by filling a disk — so the error has to be in the
///    signature, where a caller cannot not see it.
///
/// The asymmetry is the cheap half of a deliberate trade. The alternative is a
/// fallible trait for one implementation and an infallible one for the other,
/// which is two traits, which is no trait.
///
/// # Absence is not a failure, and the nesting is what says so
///
/// [`OpLog::get`] returns `Result<Option<Entry>, OpLogError>` and not
/// `Option<Result<..>>`. The outer layer answers "could the store be
/// consulted", the inner "did it hold this". §3.3 makes a peer holding a
/// partial set the normal case, so an op that is simply absent must never
/// arrive as a failure — and inverted, a caller would have to unwrap a failure
/// to ask the question before learning whether the question was answerable.
///
/// # What a caller should do with one
///
/// There is no caller yet, so this is recorded rather than demonstrated.
///
/// At the wire boundary a storage failure becomes `{"error":"..."}` — §2.5's
/// shape, never a partial success. Specifically **not** an empty feed: an empty
/// feed is indistinguishable from a Stoa nobody has posted in, so swallowing
/// this would render a forum whose store is broken as a forum that is merely
/// quiet. That is the same confusion the `Option`/`Result` nesting above exists
/// to prevent, reintroduced one layer up.
///
/// [`crate::guarded`] is not the mechanism. It converts a *panic* into the
/// error shape; this type exists so that there is no panic to convert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpLogError {
    /// The store could not be reached, opened, read or written.
    ///
    /// Carries the underlying description as a `String` rather than a database
    /// crate's own error type. `OpLogError` is part of the trait's contract, and
    /// a variant naming one implementation's dependency would put that
    /// dependency in a signature every implementation has to satisfy —
    /// including the one with no database behind it.
    Storage(String),
    /// The store declares a storage layout this build does not understand.
    ///
    /// **Refused rather than read on a best-effort basis.** Ops are the
    /// authority for every piece of forum state (§3.3), so a layout misread
    /// yields a forum state that is wrong with no error anywhere. A store that
    /// cannot be opened is a visible problem; a store opened wrongly is not.
    UnknownLayoutVersion { found: i32, expected: i32 },
    /// The store stamps a layout this build DOES understand, and is not it.
    ///
    /// Distinct from [`OpLogError::UnknownLayoutVersion`], which is a store this
    /// build has no claim on. This is the opposite: the store claims to be ours,
    /// and the structure the claim promises is absent or altered.
    ///
    /// **Refused at open rather than discovered at the first read.** Without
    /// this check such a file opens `Ok` and every subsequent call fails as
    /// `Storage("no such table: ops")` — an error that blames the disk, when the
    /// fact is that the file is not the layout it declares. The two call for
    /// different responses: a disk error is retried, a mislabelled store is not.
    ///
    /// Not [`OpLogError::CorruptEntry`] either: nothing was decoded. The tables
    /// the layout promises are not there to decode from.
    LayoutDoesNotMatchItsVersion { version: i32, why: String },
    /// A stored op could not be decoded back into an op.
    ///
    /// Distinct from [`OpLogError::Storage`] because the store worked perfectly
    /// and what it handed back did not — which points at a corrupted or
    /// hand-edited file rather than at the disk, and calls for a different
    /// response.
    CorruptEntry(String),
}

impl fmt::Display for OpLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpLogError::Storage(why) => {
                write!(f, "the op log's storage could not be used: {why}")
            }
            OpLogError::UnknownLayoutVersion { found, expected } => write!(
                f,
                "this op log was written with storage layout version {found}, and this \
                 build understands version {expected}; open it with a build that knows \
                 that layout rather than upgrading it in place"
            ),
            OpLogError::LayoutDoesNotMatchItsVersion { version, why } => write!(
                f,
                "this op log declares storage layout version {version}, which this build \
                 understands, but does not have that layout: {why}; it was not written by \
                 this build and must not be read as though it were"
            ),
            OpLogError::CorruptEntry(why) => write!(
                f,
                "an op stored in this log could not be read back: {why}; the storage is \
                 readable but its contents are not what this build wrote"
            ),
        }
    }
}

impl std::error::Error for OpLogError {}

/// What a peer stores, and what a reader may ask of it.
///
/// # Why this is a trait, and what is about to implement it
///
/// PLAN.md §9 Phase 1 names it: "pure Rust behind `Transport` and `Store`
/// traits, tested against fakes with no node running". §3.3 names SQLite as the
/// destination. The trait is the seam between the two, and it is not a
/// speculative abstraction in CLAUDE.md's sense — the second implementation is
/// specified in the plan, and the revision and moderation resolvers are both
/// written against this contract.
///
/// # Reads return OWNED entries, and the reason is structural
///
/// These returned `Vec<&Entry>` and `Option<&Entry>` until the SQLite
/// implementation was written, and the change is not a preference.
///
/// **A database cannot lend a reference to a row it has not materialised.**
/// [`MemoryOpLog`] owns its entries and so can lend them; a store holding rows
/// in a file would have to point the borrow into a cache built for the call —
/// which means the cache outlives the call, which means it lives in the struct,
/// which means a read needs `&mut self`, and the next read invalidates the last
/// one's references. Every route ends somewhere worse than owning.
///
/// `Cow<'_, [Entry]>` and `impl Iterator<Item = Entry>` were both evaluated and
/// both lose — a `Cow` would be `Owned` in every implementation, since
/// [`MemoryOpLog`] sorts on read and so constructs a `Vec` either way, and an
/// iterator is not object-safe and holds a borrow across the caller's fold. The
/// `sqlite-projection` change's `design.md` has the table.
///
/// The cost is a clone per entry per read in [`MemoryOpLog`]. §3.3 puts the
/// query traffic on the materialised view — the log is read to *rebuild* that
/// view, not to render a frame — and the read already allocated a `Vec` and
/// sorted it, so this is a constant factor on an operation that was O(n)
/// allocating already.
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
    fn append(&mut self, op: SignedOp, arrival: Arrival) -> Result<Appended, OpLogError>;

    /// One op by its id, or absence.
    ///
    /// Absence is a defined answer, not an error: the op may simply not have
    /// reached this peer yet. A failure means the store could not be consulted,
    /// which is a different fact — see [`OpLogError`] for why the two nest this
    /// way round rather than the other.
    fn get(&self, id: &OpId) -> Result<Option<Entry>, OpLogError>;

    /// Every op, in [`cmp_ops`] order.
    ///
    /// **This is replay.** §3.3's "a cache that can be rebuilt by replay" is a
    /// fold over exactly this sequence, so there is no separate `replay()` verb
    /// — it would be a second name for one job, and CLAUDE.md asks for one
    /// function, one job.
    fn iter(&self) -> Result<Vec<Entry>, OpLogError>;

    /// Every op in one Stoa, in [`cmp_ops`] order.
    ///
    /// The Stoa address, never a channel id (§4.5): "never let channel identity
    /// leak into payloads or storage keys", so that per-thread channels later
    /// become a routing change rather than a migration.
    fn iter_stoa(&self, stoa: &Address) -> Result<Vec<Entry>, OpLogError>;

    /// Every op naming `target` — **an op** — in [`cmp_ops`] order.
    ///
    /// The shape the revision and moderation resolvers fold over. A revision
    /// resolver reads this and keeps the first entry that is a `Revise` by the
    /// target's author; a moderation resolver reads it and narrows to the
    /// `Moderate` ops a then-moderator signed. Both want "the ops about this
    /// op, in the order that decides which is current", which is one question
    /// and so is one method.
    ///
    /// **The subject is an [`OpId`], and that is a real limit rather than a
    /// parameter choice.** [`Entry::target`] maps an op kind to the op it acts
    /// upon, so a subject addressed any other way is not expressible here. Stoa
    /// metadata (§5.7) is the case known to be coming: a metadata op names the
    /// **Stoa**, an [`Address`], so it needs its own read rather than a widened
    /// `target` — structurally it is the moderation resolver with a different
    /// subject, and it will reuse that resolver's authority check unchanged.
    /// Relevance scoring (§7.2) does *not* need one; it folds over [`iter`] or
    /// [`iter_stoa`] and consumes a resolver's output rather than being a peer
    /// of these two.
    ///
    /// [`iter`]: OpLog::iter
    /// [`iter_stoa`]: OpLog::iter_stoa
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
    fn iter_target(&self, target: &OpId) -> Result<Vec<Entry>, OpLogError>;

    /// How many distinct ops the log holds.
    fn len(&self) -> Result<usize, OpLogError>;

    /// Whether the log holds no ops.
    ///
    /// Present because clippy's `len_without_is_empty` requires it alongside
    /// [`OpLog::len`]. No caller needs it today.
    ///
    /// Note it answers only "has this peer seen anything at all" — "is this Stoa
    /// quiet" is `iter_stoa(..).is_empty()`, which is a different question this
    /// cannot stand in for.
    fn is_empty(&self) -> Result<bool, OpLogError> {
        Ok(self.len()? == 0)
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
    fn sorted(&self, keep: impl Fn(&Entry) -> bool) -> Vec<Entry> {
        let mut out: Vec<Entry> = self.entries.values().filter(|e| keep(e)).cloned().collect();
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

/// Every method returns `Ok`, and nothing here can fail.
///
/// A `HashMap` has no failure mode to report. See [`OpLogError`] for why the
/// trait is fallible anyway and why that asymmetry is the cheap half of the
/// trade rather than a wart.
impl OpLog for MemoryOpLog {
    fn append(&mut self, op: SignedOp, arrival: Arrival) -> Result<Appended, OpLogError> {
        let id = op.op.id();
        // `entry().or_insert()` rather than `contains_key` then `insert`: the
        // first-wins rule is then structural rather than a branch that a later
        // edit could invert. There is no code path here that overwrites an
        // existing entry, so no code path can replace recorded arrival metadata.
        Ok(match self.entries.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => Appended::AlreadyPresent,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Entry { op, arrival });
                Appended::Stored
            }
        })
    }

    fn get(&self, id: &OpId) -> Result<Option<Entry>, OpLogError> {
        Ok(self.entries.get(id).cloned())
    }

    fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
        Ok(self.sorted(|_| true))
    }

    fn iter_stoa(&self, stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
        Ok(self.sorted(|e| &e.op.op.stoa == stoa))
    }

    fn iter_target(&self, target: &OpId) -> Result<Vec<Entry>, OpLogError> {
        Ok(self.sorted(|e| e.target().as_ref() == Some(target)))
    }

    fn len(&self) -> Result<usize, OpLogError> {
        Ok(self.entries.len())
    }
}

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod contract;

#[cfg(test)]
mod tests {
    //! What is left here after the contract suite took the rest.
    //!
    //! Every behaviour asserted of the `OpLog` TRAIT lives in `contract.rs`,
    //! where it runs against both implementations. This module holds only the
    //! two that are not trait behaviours and so have nowhere else to go:
    //!
    //! - `two_logs_with_the_same_ops_read_the_same_order` builds TWO logs and
    //!   compares them, which the contract suite's one-log-per-behaviour shape
    //!   cannot express;
    //! - `an_entry_reports_the_target_its_kind_names` is about `Entry::target`,
    //!   a plain function on a struct, and needs no log at all.
    //!
    //! **Nothing else belongs here.** The copies that used to live alongside
    //! these were the same behaviour names asserted of `MemoryOpLog` alone,
    //! against fixtures that HUNTED a shared byte 0 out of two short titles
    //! rather than constructing one. A 2-byte prefix leak in `iter_stoa`
    //! passed them and failed `contract.rs` — this file is the one a reader
    //! opens first, so leaving the weaker copies here made them the template
    //! the next test would be written against. Add a trait behaviour to
    //! `contract.rs`, where `SHARED_PREFIX_BYTES` states the prefix length as
    //! a requirement instead of pinning a hash coincidence.

    use super::fixtures::*;
    use super::*;
    use crate::op::{ModerationAction, Op, VoteDirection};

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
            forwards.append(op.clone(), arrival.clone()).unwrap();
        }
        let mut backwards = MemoryOpLog::new();
        for (op, arrival) in ops.iter().rev() {
            backwards.append(op.clone(), arrival.clone()).unwrap();
        }

        let a: Vec<OpId> = forwards.iter().unwrap().iter().map(|e| e.id()).collect();
        let b: Vec<OpId> = backwards.iter().unwrap().iter().map(|e| e.id()).collect();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4, "the fixture must exercise all four");
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
