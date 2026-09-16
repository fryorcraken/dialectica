# Design: the op clock

## Context

Two fields enter the signed op preimage with deliberately unequal authority: a
Lamport counter that is authoritative for every ordering, and an author-asserted
wall-clock that decides nothing. The spec settles *what* must hold. This records
*how*, and — where the spec left a value or a shape open — *why this one*.

The shape of the existing code decides most of it. Three facts dominate:

- `cmp_ops(a: OpEntry, b: OpEntry)` takes `OpEntry { arrival: &Arrival, id: &OpId }`.
  Ordering therefore reads transport metadata **because the comparator is handed
  it**. That is the thing to change structurally, not by discipline.
- `dialectica-core` reads no clock anywhere. `grep` for `SystemTime` finds one
  hit, in a keystore *benchmark*. The crate is pure, and the wall-clock must be
  injected rather than sampled.
- `Op { stoa, author, kind }` is constructed inline at four publish sites and in
  a hand-maintained test fixture list. Adding two fields to a public struct with
  public fields would compile at none of them — which is the property to keep.

## Decisions

### 1. The counter is `Option<u64>` on `Op`, not a second op type

**Chosen:** `Op` gains `clock: Option<OpClock>`, where
`OpClock { counter: u64, asserted_ms: u64 }`. `None` is exactly "encoded under
version 1"; `Some` is exactly version 2.

**Considered:** a `Op` / `LegacyOp` enum, or a `version: u8` field with two
loose `Option`s beside it.

**Ruled out because:** the two fields are present or absent *together* — the
spec requires that an op of version 2 carrying no counter is malformed, and that
a version-1 op "reports carrying no counter, rather than reporting a counter of
zero". One `Option` around a struct of both makes the version discriminant and
the field presence **the same fact**, so there is no state where a build holds a
version-2 op with a missing wall-clock. Two independent `Option`s would admit
three impossible combinations and need a guard at every read. A separate op type
would fork every consumer — `cmp_ops`, the resolvers, the log, the wire — for a
distinction only the encoder cares about.

`canonical_bytes` writes `VERSION_2` and the eight-plus-eight bytes when
`clock` is `Some`, and `VERSION_1` and nothing when it is `None`. So
**a version-1 op's bytes and id are unchanged by this change**, which is the
migration requirement, and it holds by construction rather than by a test that
compares against a recorded constant.

### 2. `cmp_ops` stops being handed an `Arrival` at all

**Chosen:** `OpEntry { counter: Option<u64>, id: &OpId }`, constructed by
`OpEntry::of(entry)` from the op's own `clock`. `cmp_ops` no longer imports
`Arrival`, and `cmp_tiebreak` is deleted with the transport message-id tiebreak
it implemented.

**Considered:** keeping the signature and reading `a.op.clock` inside.

**Ruled out because:** the spec's requirement is *"Ordering SHALL NOT consult
the transport's Lamport timestamp or message id"*. A comparator that still
receives an `Arrival` satisfies that by not writing a line, which is the fourth
slightly-different guard. A comparator that cannot name an `Arrival` satisfies
it because there is nothing to consult — and the spec scenario "the inputs the
comparison reads are examined; every one of them is carried inside the ops being
compared" becomes a statement about the type rather than about the body.

`Arrival` itself survives unchanged, and is still recorded and still stored. The
spec says exactly that: *"Recorded arrival metadata MAY still be retained as a
record of what a peer received; it SHALL NOT order."* Deleting it would discard
a peer's honest record of a delivery to make an ordering change.

### 3. The clock is derived by a fold over held ops, and the bound is measured
against that derivation

**Chosen:** `OpLog::clock(&self, stoa) -> Result<u64, OpLogError>`, computed as
a fold over the Stoa's ops:

```
clock = 0
for each counter c held, in ASCENDING counter order:
    if c <= clock            -> no change
    else if c - clock <= ADVANCE_BOUND -> clock = c
    else                     -> no change      (over-bound: stored, not advancing)
```

**Ascending order is what makes this a function of the op set** rather than of
arrival order, which `op-ordering`'s spec requires in its own words: *"The excess
SHALL be measured against a value computed from the ops the peer holds, and SHALL
NOT be measured against whatever the peer's clock happened to be at the instant
the op arrived"* (`specs/op-ordering/spec.md:83`), with the reason at `:85` — the
arrival-order version *"is the one that falls out of writing the check on the
receive path"*.

Folding in arbitrary order, a run of ordinary ops followed by `u64::MAX` accepts
the jump on one peer and refuses it on another. Folding ascending, every peer
walks the same ladder from 0 and stops at the same rung, whatever sequence the
ops arrived in. A peer that receives the `u64::MAX` op first and a peer that
receives it last compute the same clock.

**Considered:** checking the bound on the receive path against the clock as it
then stood — which is the version that falls out of writing the check where the
op arrives, and is precisely what the spec forbids.

**Note this makes the bound transitive over a chain.** A hostile author cannot
jump the clock, but a *sequence* of ops each within `ADVANCE_BOUND` of the last
walks it up legitimately, one bound at a time. That is correct and is the point:
an author paying for N ops to advance N × bound has paid N ops' worth of
publishing, which is the same cost an honest busy Stoa pays.

### 4. `ADVANCE_BOUND = 1_000_000`

The spec requires a fixed named constant identical on every peer and does not
fix the value. Reasoning for this one:

- It must be **far above any honest gap**, or a peer returning from a long
  offline period is refused the advance and publishes below the Stoa, sorting
  its own post to the bottom of the feed. A Stoa producing one op a second
  continuously needs 11.5 days to cover a million.
- It must be **far below `u64::MAX`**, or the attack it exists to stop — pinning
  a peer's clock at the ceiling so it can never publish again — is reachable in
  a handful of ops. At a million per step, walking from 0 to `u64::MAX` takes
  ~1.8 × 10^13 ops, each of which must be published, delivered and stored.
- A round decimal number rather than a power of two, because nothing here is a
  bit mask and a reader should not look for a reason it is 2^20.

Pinned by a hardcoded assertion, following `LAYOUT_VERSION` and `MAX_FIELD_LEN`:
`cargo mutants` does not mutate a `const`, and this project has shipped a
`VERSION_1` defect that left the suite green.

### 5. The wall-clock reaches the view as text, and the instant is never public

**Chosen:** `Op::clock` is `pub(crate)`-readable for the counter, and the
asserted instant leaves core **only** through `format_asserted_time`, which
returns `AssertedTime { text: String, clamped: bool, }`. No `ThreadItem` field
and no JSON field carries the instant as a number.

This is the spec's structural device and the owner's instruction: sorting on the
displayed time would need parsing a string back into an instant, which is
visibly wrong in a diff rather than a plausible field access.

The ordering position is carried separately as `ThreadItem::position` — an
opaque decimal string of the item's index within the returned sequence, not the
counter. **Not the counter**, because the counter is a number that means an
order and would invite arithmetic; an index says "this is where it goes" and
nothing else. The spec asks only that the field "a caller uses to place it in
order is distinct from the field carrying the time", and that placing items in
order requires no reading of the time.

### 6. Clamping is read-time and needs a clock the caller supplies

`dialectica-core` reads no clock, and must not start: a pure crate that samples
`SystemTime` is untestable at the boundaries that matter and would put a
nondeterministic input into a read. So the reading peer's own clock arrives as
an argument — `read_thread(..., now_ms: u64)` — and `wire.rs`, which is where
the host's environment already enters, samples it.

`FUTURE_ALLOWANCE_MS` = 24 hours. A day is comfortably above any honest clock
skew (NTP-corrected clocks sit within seconds; an unsynchronised one drifts
minutes a month) and far below the range in which "posted in 2387" becomes
unremarkable.

`FLOOR_MS` = `1_262_304_000_000` — 2010-01-01T00:00:00Z. Before any op of this
system could have been written, and after the epoch, so a zero wall-clock
clamps. A floor of 0 would clamp nothing at the bottom, and the spec requires a
floor that a zero value falls below.

### 7. The publish path takes the clock as an argument, and the compiler
enumerates the sites

`Op`'s `clock` field is public like its siblings, so every inline `Op { .. }`
construction fails to compile until it is supplied. That is deliberate and is
the answer to the stale-sweep-list trap: `authoring::post`, `reply`, `vote` and
the fixture list are found by `cargo build`, not by a grep someone has to
remember to run.

The publish helper reads the log's clock for the Stoa and signs at one above it,
saturating. A peer publishes within any bound by construction, so no publish
needs a bound check.

### 8. `score_epoch` takes the counter, and the wall-clock is not a candidate

The SQLite projection reserves a `score_epoch` column for §7.2 rule 5's ranking
decay. Nothing reads it yet. It previously held the recorded arrival's Lamport
value; it now holds the op's own counter.

**The wall-clock is the newly available wrong answer here**, and it is worth
naming because a reserved column nothing reads is the easiest place to leave a
defect — no test fails, and the rows are written to every peer's store long
before the read that would expose it. A decay reading an author-asserted time
would let a post claiming a future instant pin itself above every honest one
permanently. PLAN Appendix A measures that exact failure in the nearest kin
project, where the validator's future-timestamp warning was never called on the
ingest path.

The counter is the only value in the row that is both **shared across peers**
and **not chooseable for rank** — the advance bound is what makes the second
true. The recorded arrival was shared with nobody; the wall-clock is chosen by
the author.

### 9. `LAYOUT_VERSION` is bumped, and the check that enforces it had to move too

The sort columns derive from a different thing, so a store written by a
version-1 build holds rows whose `sort_*` values answer a question this build no
longer asks. Reading them would produce a silently wrong order rather than an
error, which is why the layout version exists.

**`check_layout`'s own column list is part of the layout**, and missing it was a
real defect in this change caught by the persistence tests: it still named the
four deleted columns, so every reopen of a store this build had just written
failed with `LayoutDoesNotMatchItsVersion`. In production that would have made
every store permanently unopenable, with no migration path by design. Recorded
because the failure mode is invisible in the schema diff — the `CREATE TABLE`
and the check are forty lines apart and nothing ties them together.

### 10. Equal counters tiebreak on the op id, ascending

Two ops can carry one counter — two authors who had each seen the same thing
publish at the same N — so the order needs a second key or it is not total, and
a `LIMIT` page over a non-total order silently repeats and skips rows.

**Chosen: the op id, ascending.** It is already `cmp_ops`'s last resort, and it
is the only candidate that is a **pure function of the op's own bytes**: every
peer holding the op computes the same one without consulting anything it
received, so the tiebreak converges for the same structural reason the counter
does.

**Considered: the transport message id**, which is what `cmp_tiebreak`
implemented before this change deleted it. Rejected because it does not reach
us — a tiebreak absent on every op is not a tiebreak — and because it is a
per-peer value, so two peers would break the same tie two ways.

**The cost, stated:** a hash carries no recency whatever, so which of two
equal-counter ops leads is arbitrary. That is the honest answer rather than a
defect: the ops are genuinely concurrent, and any rule that made one "later"
would be inventing an order the ops do not carry. It is recorded here because
tiebreaking on a hash looks arbitrary at a call site, and a reader who does not
find the reasoning will reach for something that looks more temporal.

### 11. An implausible wall-clock is accepted and clamped, never refused

**Chosen: accept every clock value at the boundary, clamp at read time for
display, and report that the clamp happened.** `Op::decode` validates no clock
value; `format_asserted` bounds what is rendered and returns `clamped: bool`
beside the text.

**Considered — and it is the intuitive one: refuse at the boundary.** CLAUDE.md's
security posture says to validate untrusted input before it reaches a state
machine, and a far-future timestamp is exactly the shape that rule is about. It
is rejected here because **refusing an op for its clock is a censorship vector**:
a peer whose system clock is skewed would be silently dropped by every honest
peer, and from that peer's side the result is indistinguishable from moderation
nobody performed. In a censorship-resistant forum that is the failure mode the
whole design exists to avoid, and it would be reachable by an accident rather
than by an attack.

This is also why **the whole clamp lives at read time** rather than at the
boundary, which decision 6 takes as given rather than as chosen. A boundary
clamp would rewrite a signed field — the op would no longer verify — so the only
place a clamp can live is where the value is rendered.

**The cost, stated:** a reader may be shown a time that is not when the post was
written. That is why the clamp is *reported* rather than silently applied: a
view can say the time is not to be trusted, which a silently substituted value
could not.

Note the refusal this does *not* buy back: the clock decides nothing, so an
absurd value costs its author nothing and gains them nothing. Accepting it is
cheap precisely because the field is display-only.

### 12. Content-dedup ends, and there is no delete behind it

**This is the change's one irreversible user-visible consequence**, and it is a
decision rather than a side effect.

An op id is a function of the preimage; the preimage gained a counter and a wall
clock; so the same body authored twice now produces **two ops with two ids**,
where `content-authoring` previously *guaranteed* one. A byte-identical replay
still dedups — that is what keeps a re-delivered op idempotent — but two separate
authorings are no longer byte-identical.

**Considered: a nonce**, recorded in `PLAN.md` as struck-through history. It was
rejected long before this change for separating two identical posts while doing
nothing for ordering; the counter does both, which is why it is the field that
landed. Worth restating because a reader may reach for a nonce to *restore* the
old dedup and find only the old rejection.

**Considered: refusing the duplicate in `authoring`.** Rejected — two identical
posts minutes apart are a legitimate thing to write, and core cannot tell that
from a double tap. At the core layer a double tap and a deliberate repeat are
the same act.

**The cost this forecloses, stated plainly because it is stated nowhere else in
core:** there is no delete in this system, and a revision replaces content rather
than withdrawing an op. So an accidental duplicate is **permanent, for every peer
that received it**, and the author's only remedy is editing one of the two into
an apology. That sentence previously existed only in `DComposer.qml`, as
motivation for a QML guard — which is not where a core-contract consequence is
discoverable. The guard is the right response and it is the *view's* response:
the affordance belongs where the two acts are still distinguishable.

## The four capabilities beyond the original scope

- **`composer-view`** — the urgent one, and it lands here. `DComposer.qml` now
  disables its submit control from submission until an outcome, on every
  outcome including a refusal. Core no longer absorbs a double tap and cannot:
  at that layer a double tap and a deliberate repeat are identical acts. There
  is no delete in this system, so an accidental duplicate is permanent.

  **What the guard buys on today's transport, stated precisely, because the
  obvious test would be measuring the wrong thing.** `Core.call` reaches the host
  through `bridge.callModule`, which is **synchronous**, and QML's JavaScript is
  single-threaded — so `submit()` runs start to finish within one event-handler
  turn and no second activation can be delivered while a publish is outstanding.
  **The duplicate this guards against is already unreachable**, by the runtime's
  shape rather than by anything in the component.

  That is a reason to write the guard, not to skip it. The property being
  defended is "one gesture, one op", and it currently holds for a reason the
  component neither chose nor controls; the day `callModule` gains an
  asynchronous form, the window opens and nothing would report that it had.

  It is also the reason `tst_composer.qml` does **not** contain a
  "tap twice, assert one publish" test. That test would pass with the guard
  deleted. What is asserted instead is what the component does.

  **The guard is two independent pieces, and each is pinned by exactly one
  test.** An earlier version of this entry said "each of those fails when the
  guard is removed", which review found overstated; both mutations were re-run
  and counted:

  | Mutation | Tests failing |
  |---|---|
  | remove `submit()`'s `if (publishing) return` | **1** — `test_a_submit_while_one_is_outstanding_sends_nothing` |
  | remove the button's `&& !root.publishing` | **1** — `test_the_control_is_absent_while_a_publish_is_outstanding` |

  One test per piece is the right number rather than a shortfall — they defend
  different things, and the second is the one that survives `callModule`
  becoming asynchronous. What the count does correct is the implication that
  the three `test_the_control_returns_*` tests cover the guard: they fail under
  **neither** mutation, because they assert the flag is *lowered* on each
  outcome, which is a separate property (a composer that locked itself out
  after a refusal). They are worth keeping and they are not guard coverage.
- **`moderation-resolution`** — the `Hide`-wins preference is keyed on the
  leading candidate and stays correct. Its *justification* rested on "exactly
  two ops can ever exist", which free preimage bytes falsify; the doc comment is
  rewritten. The code was checked for a bounded-candidate-set assumption and
  carries none — it uses `iter().position()` over an unbounded `Vec`.
- **`post-revision`** and **`thread-read`** — both asserted that no Lamport
  value reaches us. Corrected in place.

## What this deliberately does not do

It does not delete `Arrival`, does not touch `op-transport`, and does not add
gap detection. A scalar counter says "I had seen something at N", never *which*;
the parent pointer is the causal edge this forum acts on and it already exists.
