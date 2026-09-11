# Design: the op log, and why it is a trait before it is a database

## Decisions

### Decision: A trait with an in-memory implementation now, not SQLite

This was the change's open question, and the two sides are both real.

**For SQLite now.** §3.3 says it in as many words: "Each peer keeps a **local
SQLite store** holding every op it has seen". That is the destination, and
CLAUDE.md warns against building an abstraction over a single implementation —
"do not refactor speculatively [...] make room for the change in front of you,
not one you imagine". A trait with one implementor is the canonical shape of that
mistake. `rusqlite` is also permitted: the crate's hard constraint is **zero SDK
types**, which is about `lp_*` symbols that do not link into a test binary, and an
ordinary crate with a C library behind it is not an SDK type.

**For the trait now, which is what was chosen.** Three things decided it.

1. **§9 asks for the trait by name.** Phase 1 is "pure Rust behind `Transport`
   and `Store` traits, tested against fakes with no node running". The trait is
   not a guess about a second implementation — the second implementation is
   specified, in the document that sets the phase boundaries. CLAUDE.md's
   warning is against abstractions derived from one instance; this one is
   derived from a plan that names two.

   **Be precise about what is being deferred, because §9 does not defer it.**
   The same sentence lists "the op log and its SQLite projection" as Phase 1
   work, alongside the traits. Phase 2 is "wire the real modules. Swap the fakes
   for `delivery_module` channels and `storage_module`" — which is transport and
   storage modules, not the local store. So this change ships one half of a
   Phase 1 item and defers the other half; it is **not** the case that §9 put
   SQLite in a later phase. Reasons 2 and 3 below are why the deferral is worth
   taking anyway, and they are arguments against §9's sequencing rather than
   readings of it.

2. **The second consumer already exists.** Two changes — a revision resolver
   (`phase2/revision-resolver`) and a moderation resolver
   (`phase2/moderation-resolver`) — are written against this contract. Named as
   branches rather than as "being written right now", which stops being true the
   moment this is archived. An abstraction with one implementation and three
   consumers is not speculative; the consumers are what make the seam
   load-bearing today rather than later.

3. **SQLite would make the Phase 1 testing discipline harder to keep.**
   "Tested against fakes with no node running" is about more than nodes: a test
   that needs a database file needs a temp directory, a schema migration, and a
   teardown, and every resolver test inherits all three. §3.3 puts the *query*
   traffic on the materialised view — a later change — so the log's own reads are
   not where a database earns its keep. What a database buys here is durability,
   which nothing in Phase 1 exercises.

**What would reverse this, checkable by a command.** `MemoryOpLog` is the only
implementor: when `grep -rn 'impl OpLog' dialectica/rust-lib/` still returns one
line at the end of Phase 1, the trait did not earn itself and should be collapsed
into the concrete type. That is a fact a reader can establish today, rather than a
future event nobody is watching for.

Separately, if the SQLite implementation turns out to want a different shape
(streaming reads rather than `Vec<&Entry>`, because a peer's history outgrows
memory), the trait was drawn at the wrong altitude and should be redrawn with two
implementations in hand rather than kept as first drafted. `Vec<&Entry>` is the
return type most likely to be wrong for that reason, and it is named here so the
next author sees it as a question rather than a constraint.

**No new dependency.** `rusqlite` is not added by this change. The argument above
is why, and the decision is reversible in one `Cargo.toml` line.

### Decision: `MemoryOpLog` is the implementation and the fake, not a test double

It lives in the module rather than behind `#[cfg(test)]`, because a peer
configured without persistence is a real peer. The alternative — a production
SQLite store and a separate in-memory double for tests — has the standard failure
mode: the double drifts from the real one, and the tests keep passing against
behaviour nothing ships.

Writing the fake as the implementation means the resolvers' tests exercise the
code a peer runs.

### Decision: Nothing is verified, filtered or rejected on append

§3.3 puts verification on read: "The store may hold junk; the reader never trusts
it." Three distinct reasons, and the second is the one a reviewer is most likely
to want to argue with:

1. **A filtered op is indistinguishable from an op never received.** "Did someone
   try to forge this?" and "is this valid?" are different questions with different
   responses, and a store that dropped the forgery can answer only the second.

2. **Validity is not decidable at append time.** §6 makes a moderation op valid
   "only when signed by a current moderator" — which needs the Stoa's moderator
   set *as of that op's Lamport time*. An op that looks unauthorised under the set
   a peer holds today may be authorised under a set established by an op that has
   not arrived yet (§4.7's backfill delivers history out of order by design).
   Filtering on write forecloses that permanently, and silently.

3. **A write-side guard has to be right at every call site; a read-side guard has
   one place to be wrong.** CLAUDE.md: "A guard is a job. Keep it separate, so 'is
   it called everywhere?' stays a question with an answer."

This is also the §6.2 defect measured in the nearest kin project, which "checks
moderator authority only on the send path and never on the read path". Putting the
check on the write path here would be the same mistake with the sides swapped.

### Decision: The first arrival's metadata wins, and the caller is told which happened

The same op reaches a peer more than once by ordinary means — retransmission,
causal-history backfill, SDS-Repair. Each arrival carries its own metadata, and
both records are equally truthful, so something must choose.

**First wins**, for a reason about stability rather than truth. Keeping the last
would make a peer's recorded order depend on how many times each op happened to
reach it and in what sequence — which differs per peer, and is precisely the
divergence `arrival.rs` exists to prevent, reintroduced one layer up. It would
also let a thread a user is reading reorder underneath them because a duplicate
arrived, which is a visible defect with no error behind it.

The rejected alternative — **keep whichever arrival carries more metadata** — is
the tempting one, and `arrival.rs`'s own module documentation now warns the next
consumer off it directly ("Seeing one op twice: do NOT keep the 'richer'
arrival"). That warning landed because this change asked for it; the reasoning
lives there rather than being restated here, since two copies drift and the
wrong one gets read. The short form: whether a peer receives the richer copy is a
per-peer accident, so convergence beats completeness, the same trade the degraded
order makes.

The choice is observable, so it is in the spec rather than only here.

### Decision: Dedup is established before the sort, structurally

**The history matters here, and it is not what the shape suggests.** The
`HashMap` was written first, for §3.1's idempotence alone. The `cmp_ops`
precondition was raised mid-change by a review of the parent branch, and the map
was then *confirmed* to make the offending list unconstructible. The safety was
discovered, not designed for.

An author who believes the map was chosen for the precondition will assume
any dedup-ing schema inherits the property. It does not follow automatically: a
schema that dedups on write but materialises rows into an intermediate list before
ordering — a `SELECT ... ORDER BY` over a table with a non-unique index, say —
would re-open exactly this hole. The property to preserve is "no two entries with
one op id can reach the comparator", and it must be checked deliberately in any
new storage shape rather than assumed from the old one.

`cmp_ops` is total over **distinct** ops. Two entries sharing an `OpId` but
carrying different `Arrival` metadata compare `Equal` — correctly, since §5.7's
rule has nothing to say about one op against itself — and a sort over such a pair
would leave their relative order to the sorting algorithm. That is not a defined
order: it varies with the sequence the peer received in, so two honest peers would
render one thread differently with no error anywhere.

The log cannot present that pair, and not by remembering not to. Entries live in a
`HashMap<OpId, Entry>`, so one op id is one entry and the sort iterates values that
are distinct by construction. There is no intermediate list of arrivals that could
be sorted before being deduplicated, because there is no intermediate list.

This is CLAUDE.md's "put the complexity in the data structure, not the logic"
doing the work: the alternative — a `Vec` plus a contains-check before each push —
is a guard that has to be right at every insertion site, and the failure mode when
it is not is invisible.

### Decision: A map sorted on read, not a structure ordered on insert

The obvious alternative is a `BTreeMap` keyed on `(Arrival, OpId)`, giving ordered
reads for free. It is rejected for a specific reason rather than on principle.

The sort key includes `Arrival`, which is **the transport's to supply and this
peer's to record**. An ordered structure would place an op according to the
metadata available at insert time, and the whole point of the upstream fix that is
expected to land is that this metadata changes character. Sorting on read makes the
order a pure function of the log's current contents — the property that has to hold
whatever else moves.

The cost is a sort per read. It is acceptable because §3.3 puts the read traffic on
the materialised view, which is a later change and is where an index belongs; the
log is the authority, and an authority is read to rebuild a cache, not to render a
frame.

### Decision: `iter_target` is one method, not one per op kind

A revision resolver wants `Revise` ops on a target; a moderation resolver wants
`Moderate` ops on it. Two methods would have served both precisely.

One method is better because "what acts on this subject?" is one question, and
because the alternative puts the kind taxonomy in the store's API — so a fifth op
kind would widen the store's surface rather than being absorbed by it. A resolver
narrows the result it is given, which is a `find_map` it was going to write anyway.

**A `Post`'s `parent` is not a target.** A reply names its parent as a reply
relationship, not as a subject acted upon. Conflating them would make a moderation
resolver see every reply to a post alongside the moderations of it, which is a
defect that would surface as "hiding a post hid its replies' moderation state" and
be very hard to trace back here.

**The same argument settles `sorted`'s signature.** It takes a predicate rather
than an enum of read kinds, for the reason above run once more: an enum would put
the taxonomy of reads inside the one function all three reads share, so adding a
fourth read would mean editing the shared body. A predicate absorbs it.

### Decision: `Entry` keeps the op and the arrival side by side

They are different kinds of fact and the type says so. The op is signed,
byte-stable, and identical on every peer; the arrival is this peer's unsigned
record of one delivery, legitimately different on the peer next door. A flattened
type — a `SignedOp` with `lamport` and `message_id` fields hanging off it — would
invite a reader to treat a locally-recorded Lamport value with the confidence due
to a signed field, which is the confusion `op.rs` and `arrival.rs` are each at
pains to prevent from their own side.

### Decision: `append` returns what it did, rather than leaving it to be inferred

`Appended::{Stored, AlreadyPresent}` exists so a caller never has to count the log
before and after. That is a two-step check every call site would spell the same
way until one of them spelled it differently, and the information is genuinely
wanted: a newly stored op is one to gossip onward and to rebuild a view from, and
a duplicate is neither.

Returning `bool` was the alternative. The named variants win on the usual grounds
— `Appended::AlreadyPresent` reads at the call site where `false` needs the
signature to interpret it — and on one specific one: the polarity of a `bool` here
is genuinely ambiguous, since "true" could as easily mean "this was new" as "the
append succeeded", and both readings are plausible for a method that never fails.

### Decision: Replay is `iter`, with no separate verb

§3.3's "a cache that can be rebuilt by replay" is a fold over the log in order,
which is exactly `iter()`. A `replay()` that called `iter()` would be a second name
for one job — CLAUDE.md's "one function, one job" read in the other direction.

Insertion order is deliberately **not** reachable through any method. A reader that
could ask for it would be one refactor away from ordering a thread by the sequence
one peer's network happened to deliver in.

## What the two blocked resolvers do with this

The concrete part, since unblocking them is the point of landing the log separately.

**The revision resolver** (§5.7: "the highest Lamport timestamp is current",
"a version signed by anyone other than the post's original author is invalid and
dropped on read"):

```
current_version(log, post_id):
    original = log.get(post_id)?          # absent target is a defined absence
    for entry in log.iter_target(post_id):   # already in cmp_ops order
        if entry is Revise
           and entry.op.verify()             # authenticity, on READ
           and entry.author == original.author:   # §5.7's authorship rule
            return entry
    return original                        # no valid revision: the original stands
```

The first match wins because `iter_target` is already in `cmp_ops` order. The
author check is the resolver's because it needs the target op, which `op.rs`
records it cannot do alone.

**"First" is most-recent only on the ordered branch, and that branch is not the
one production reaches.** `cmp_ops` leads with the highest Lamport timestamp when
the transport supplied one; when it did not — every op today — the fallback is
*ascending op id*, which carries no recency at all. So a resolver written against
this takes the first entry because that is the position the ordering rule defines
as current, not because the log promises recency it cannot currently deliver. A
reader who reads "most-recent-first" into the degraded branch will look for
recency in a hash and not find it; `arrival.rs` now says which way that branch
runs, after the moderation resolver made exactly this mistake.

**The moderation resolver** (§5.7: "among *moderation* ops on the same target,
last-write-wins by Lamport order, valid only if the signer was a moderator at that
time"):

```
is_hidden(log, target_id, moderator_set):
    for entry in log.iter_target(target_id):
        if entry is Moderate
           and entry.op.verify()
           and moderator_set.contains(entry.author):   # authority, not authenticity
            return entry.action == Hide
    return false
```

Same shape, different predicate — which is why `iter_target` is one method. The
`log.rs` test `a_resolver_can_be_written_against_the_trait_alone` is this fold,
written generically over `OpLog`, standing in for both.

Both are defined over a partial set: a target the peer has never received yields
an empty `iter_target` and a defined answer, not an error.

## How an ordered `Arrival` flows through when upstream lands

No schema migration, and the reason is that there is nothing about the arrival in
the shape.

`append(op, arrival)` takes an `Arrival` and stores it whole. Today the boundary
constructs `Arrival::unordered()` because `channelMessageReceived` supplies neither
a Lamport timestamp nor a message id. When the two upstream changes the
`op-ordering` design specifies land — `MessageReceivedEvent` carrying the SDS
values, and `delivery_module` forwarding them — the boundary constructs
`Arrival::ordered(lamport, message_id)` instead, and **nothing else changes**:

- `Entry` already holds an `Arrival` with both fields optional, so no field is
  added and none changes type.
- `cmp_ops` already handles the ordered branch; it is the branch production has
  never exercised, not a branch that does not exist.
- Ops recorded before the fix keep their `unordered()` arrivals and sort below the
  newly ordered ones — which is the correct answer, not a compromise: an op the
  transport placed is better evidence than one about which nothing is known.
- A peer's existing log is therefore **not rewritten**. The mixture is the
  expected steady state during the transition, and
  `ordered_and_unordered_ops_coexist_with_the_ordered_ones_first` pins it.

The one thing that does not follow automatically: a peer's *old* ops never become
ordered retroactively, because the Lamport values for them were never transmitted
and cannot be reconstructed. That is a permanent consequence of the current gap
rather than a property of this design, and it is why the degraded order has to be
convergent rather than approximate.

## The ordering regime: pressure this contract does not answer, and the shape to avoid

**Not built, and recorded because the next author will otherwise re-derive a
design that has already been superseded.**

Three consumers have now asked one question this API cannot answer: *is this
sequence ordered by the transport, or is it the degraded op-id order?* Each was
answered with prose rather than a type — `iter_target`'s doc comment here,
`arrival.rs`'s `cmp_ops` summary after the moderation resolver read recency into
the degraded branch, and the same warning repeated in the moderation resolver
itself. Three copies of one warning, none enforced by a compiler, is the signal
that the warning should stop being prose.

**The obvious fix is the wrong one.** Carrying the regime on the log's read
result — `Ordered { regime: Regime, entries: Vec<&Entry> }`, with
`Regime::{ByTransport, DegradedByOpId, Mixed}` — was proposed and then
superseded by an architecture review. The reason is specific: that shape
describes the **pre-filter** sequence, while a resolver needs the regime of the
**post-filter leader** it actually selected. A moderation resolver folding over
`iter_target` and taking the first authorised `Moderate` wants to know how *that
entry* came to lead, which is not answered by how the sequence it was drawn from
was ordered. The two would coexist as two ways to ask one question, and the
weaker one reads as authoritative.

**Where it belongs instead: on the result of a resolver's fold**, not on the
log's read result. The log returns entries; whatever selects among them reports
the regime of what it selected. That keeps one answer to one question and leaves
this contract unchanged.

Two constraints on whoever builds it. `Mixed` must be a distinct case rather than
a boolean — it is the state the upstream transition actually produces, and
`ordered_and_unordered_ops_coexist_with_the_ordered_ones_first` already pins that
it exists. And `cmp_ops` must not change: it is a comparator, and it has to stay
`sort_by`-compatible.

## What this change deliberately does not build

- **The resolvers.** Sketched above, built by other changes against this trait.
- **The materialised view and its query indexes.** §3.3 distinguishes the log
  (authority) from the view (a cache rebuildable by replay). This is the log. The
  view is where "newest threads, paginated replies, a Stoa's index" live, and where
  an index earns its cost.
- **The SQLite implementation.** A later change against this trait, and still a
  Phase 1 item per §9 rather than a deferral to Phase 2. The `Vec<&Entry>`
  return type is the part most likely to need revisiting; see the first decision.
- **The boundary that constructs an `Arrival`.** Unchanged from `op-ordering`:
  there is nothing to construct one from beyond `Arrival::unordered()`.
- **Any op-format change.** `an_op_carries_no_ordering_fields` still passes
  untouched.
- **Eviction, retention or compaction.** §4.7 makes local retention the thing that
  "makes v1 usable", so a log that discarded ops would remove the tier the plan
  leans on. When snapshots land (§4.7's third tier) the question becomes real.
