# Give the application its own clock, and keep the wall clock out of every order

## Why

**An edit has no defined "earlier version", and a hide has no defined reversal.**
Both are the same missing thing: an ordering over ops on one target that every
peer computes identically. `post-revision` and `moderation-resolution` are both
built, both correct, and both currently resolve on **ascending op id** — a hash,
carrying no recency whatever. So a post's "current" version is a convergent
arbitrary choice, and `moderation-resolution` has to special-case `hide` to win
outright because last-write-wins over a set it cannot order is not a rule. A
recorded project memory states the consequence plainly: *no Lamport value reaches
us, so an Unhide cannot reverse a Hide.*

The transport is not going to supply that value, and waiting for it was already
rejected. `CLAUDE.md`: *"Ordering, recency and causality at forum scope are
dialectica's to build, carried inside the signed op preimage where a relay can
neither forge nor strip them."* PLAN §13 works it through and withdraws, on
2026-09-12, the argument that a dialectica-level clock could never agree with
SDS's — because it never had to. **A dialectica clock needs to agree with other
peers' dialectica clocks, and every peer sees the same ops.**

§13 also records that the withdrawal never reached the spec, and calls that *"an
open contradiction rather than a loose end"*. It is: `op-ordering` still forbids
a peer to compute a Lamport timestamp of its own. This change is the one §13 says
resolving it needs.

## What changes

Two fields enter the signed op preimage, and **their authority is deliberately
unequal**. Getting that asymmetry wrong in either direction is the way this piece
fails, so it is the spine of every requirement below.

- **A Lamport counter, authoritative for all ordering.** Revision currency,
  Hide/Unhide resolution, feed position — every one of them resolves on the
  Lamport value and a deterministic tiebreak, and on nothing else.
- **An author wall-clock, display-only and explicitly untrusted.** The author
  sets it and an author can lie. It may be rendered. It SHALL NOT decide an
  ordering, a comparison, a resolution, an expiry or any other decision.

The ordering rule moves from the transport's values to the op's own, and the
degraded path stops being the only path in production. Two merged prohibitions
are withdrawn explicitly rather than left standing.

**BREAKING — op identity changes, and this is larger than ordering.** An op id
hashes the preimage; the preimage gains two fields; so the same body published
twice now yields two ops rather than one. That is `content-authoring`'s
"Publishing the same content twice publishes one op", which PLAN §13 names as
symptom 2 of this same gap. It is modified here, which §13 says is the correct
place for the pressure to land.

### What each of the owner's six questions is answered with

1. **The tiebreak is the op id, ascending.** It is already `cmp_ops`'s last
   resort, and it is the only candidate that is a pure function of the op's own
   bytes — every peer holding the op computes the same one without consulting
   anything it received. The transport message id was the alternative and is
   rejected for the reason it is rejected today: it does not reach us, and a
   tiebreak that is absent on every op is not a tiebreak.
2. **A peer advances on publish, to one above the highest it has seen.** On
   receipt it advances to the received value where that value is higher, which is
   what makes the counter a causality mechanism rather than a per-peer sequence.
   **There is a ceiling**, and it is relative rather than absolute: an op whose
   counter exceeds what the peer has seen by more than a fixed bound is accepted,
   stored and ordered, but does **not** advance the peer's own clock. `u64::MAX`
   from a hostile peer therefore costs that peer one op at the top of one Stoa's
   order, and costs every honest peer nothing.
3. **Monotonicity is derived, not stored.** The peer's clock is the highest
   counter among the ops it holds, so a restart or a rebuild-by-replay recomputes
   the same value from the same log. A stored counter would be a second source of
   truth that a replay could contradict.
4. **An implausible wall-clock is accepted and clamped for display, never
   rejected.** Rejecting an op for a bad clock is a censorship vector — a peer
   with a skewed system clock would be silently dropped by everyone, and that is
   indistinguishable from moderation nobody performed. The cost is stated rather
   than hidden: a reader may be shown a clamped time that is not when the post was
   written, and the contract requires that the clamping be reported so a view can
   say so.
5. **The view is handed a rank, and the wall-clock is not a number.** The
   ordering position is an opaque token a view can only compare for sequence; the
   wall-clock arrives as a pre-formatted display string alongside an explicit
   untrusted marker. A view that wanted to sort on the wall-clock would have to
   parse a string back into a time first — which is the point.
6. **Migration: a version-2 op carries the fields, a version-1 op does not, and
   a version-1 op sorts below every version-2 op.** This reuses the degraded order
   already specified and already tested, so existing content is neither reordered
   among itself nor dropped. `op-format` already requires an unrecognised version
   to be refused rather than misparsed, so an older client meets a version-2 op as
   "a newer client wrote this".

### The two prohibitions this withdraws

Both are merged requirements that forbid exactly what the owner has decided to
build. Neither is left standing, because `openspec validate --strict` passes a
suite that contradicts itself and a reader arriving at the old text would have no
way to tell which requirement is live.

- `op-ordering` — *"A peer SHALL NOT compute a Lamport timestamp of its own, and
  SHALL NOT maintain a second logical clock alongside the transport's."*
  Withdrawn. Its stated reason was that two orders over the same messages can
  disagree; that reason is preserved and **re-aimed**, because it is still the
  thing to defend against. What changes is which single order is authoritative.
- `op-format` — *"An op carries no ordering field and no per-peer state"*, whose
  reasoning names both fields: *"A self-asserted Lamport value would be forgeable
  by exactly the author it is meant to order [...] a wall clock is a field the
  adversary sets."* Withdrawn as to these two fields, and **the objection is
  answered rather than dismissed**: the Lamport value gets the advance ceiling of
  question 2, and the wall clock is removed from every decision rather than
  clamped into safety. The requirement's *other* prohibitions — a transport
  message id, a session counter, the transport's sender identifier — stand
  unchanged.

## What this does not do

It does not build a feed ordering, an edit UI, or moderation reversal. Those
consume this contract and are separate pieces. It specifies the clock and what
core hands out.

It does not claim gap detection. PLAN §13 is precise about this and it is worth
not overclaiming: a scalar counter says *"I had seen something at N"*, never
*which*. **The parent pointer is the causal edge this forum actually cares
about** — a reply names its parent op id inside the signed preimage, so "I hold a
reply to X and no X" is already detectable, with no counter involved.

It does not touch `op-transport`, which was written deliberately neutral on this:
*"It is not a prohibition on a Stoa's ops carrying an ordering value of their own
[...] this capability neither provides nor forbids one."* That seam holds.

## Is "most recent first" honest under this contract?

**The owner asked and this proposal does not decide the label. The answer is: not
as stated, and there is one reading under which a near neighbour is honest.**

A Lamport order is a **causal** order, not a temporal one. It guarantees that if
Alice wrote B after seeing A, then B orders after A. It guarantees nothing
between two ops neither author had seen when writing — two people posting
simultaneously in different timezones are ordered by op-id hash, and a peer that
was offline for a week publishes at one above what *it* has seen, which may be
well below what everyone else has moved on to. So a feed labelled "most recent
first" would be making a claim about wall-clock time that the ordering does not
carry.

The honest near neighbour is **"latest first"** or **"newest first"** read as
*latest in the forum's own order* — which is what §7.2 already calls `new`
("Lamport order descending"). That is a real improvement on today, where the same
label would be ordering by hash. What it is not is a clock.

The wall-clock field does not rescue the stronger label, and that is the trap
worth naming: it is exactly the field that *looks* like it makes "most recent"
honest, and it is forgeable by every author. A feed sorted on it is a feed any
peer can pin itself to the top of permanently — **PLAN Appendix A measures that
exact failure in the nearest kin project**, where decay read an unclamped author
timestamp and the validator's future-timestamp warning was never called on the
ingest path.

So: the label is the owner's to choose between "newest first, by the forum's
order" and a clock claim this contract deliberately does not support.

## Capabilities

### New Capabilities

None. Every behaviour this change adds belongs to a capability that already
exists and already owns the question — which is the answer to "should the clock
be its own capability?": a clock nothing reads is not a capability, and
everything that reads it is `op-ordering`.

### Modified Capabilities

- `op-format` — the preimage gains the two fields; the encoding version
  increments; the "no ordering field and no per-peer state" prohibition is
  withdrawn as to exactly these two and kept for everything else.
- `op-ordering` — the ordering rule moves to the op's own counter; the own-clock
  prohibition is withdrawn; the clock's derivation, the advance bound and the
  wall-clock's exclusion are added; the degraded order is retained and re-aimed at
  ops carrying no counter; two requirements about transport metadata combinations
  are removed, since nothing reads those values any more.
- `content-authoring` — authoring the same content twice now publishes two ops.
- `moderation-resolution` — reversal works; the `hide`-wins preference narrows to
  ops carrying no counter; the "exactly two ops can ever exist" premise it rested
  on is false and is withdrawn.
- `post-revision` — currency among versions carrying counters becomes genuinely
  latest-wins rather than a convergent arbitrary choice.
- `thread-read` — items gain an ordering position and an author-asserted time as
  two separate fields; the blanket prohibition on describing anything in terms of
  recency narrows to wall-clock claims.
- `composer-view` — the submit control must be disabled while a publish is
  outstanding, because core no longer absorbs a double tap.

## Impact

- **Seven spec deltas**, as above. Three of them — `moderation-resolution`,
  `post-revision`, `thread-read` — were not in this piece's original scope and are
  here because the change makes text they already carry **false** rather than
  merely incomplete. Each asserts, in a justification or a scenario name, that no
  Lamport value reaches this system.
- **`composer-view` gains an obligation that core is giving up.** Deduplication
  previously absorbed a double-tapped submit; it no longer can, and core cannot
  tell a double tap from a deliberate repeat. This is a responsibility moving
  layers, not a responsibility disappearing, and the proposal names it because a
  moved obligation that nobody picks up is how a regression ships.
- **`docs/PLAN.md`** — §13's Lamport entry, §13's `createdAt` entry and §9.1 §8's
  feed-label bullet are resolved and struck through with pointers to the spec;
  §5.7's and §6's "no Lamport value reaches us" claims and the suspended-
  reversibility notes are corrected. Pruned in this change, per the repo rule.
- **`docs/UI-BRIEF.md`** — fixed in this change, as the rule requires, because it
  is designed against by someone who cannot read the code. Four places: the
  ordering-honesty section, the irreversibility warning (obligation 2,
  **withdrawn**), the duplicate-post obligation (**reversed**, and now the
  composer's), and the moderation sequencing note.
- **Storage** — the SQLite projection derives its sort-key columns from the
  arrival at write time, so they must come from the op instead. That is a
  `LAYOUT_VERSION` bump, and the existing check refuses a layout a build does not
  recognise rather than misreading it.
- **No change to `op-transport`, `op-log`, the identity layer, or the module wire
  envelope.** `op-transport` was written neutral on this question and stays
  correct unchanged.
