## Context

See `proposal.md` — Why. PR #165 (archived as
`openspec/changes/archive/2026-09-25-time-pegged-clock/`) pegged the op counter
to the time and wrote the reasoning for it into six live specs. The owner ruled
"follow the readme", and `.claude/agents/README.md` says reasoning never goes in
a spec. This change's deltas take that reasoning out, and change no behaviour.

This file is where that reasoning now lives. `proposal.md`, "Reasoning removed
from the specs", quotes each removed passage verbatim under a label (O1–O18 for
`op-ordering`, F1–F4 `op-format`, T1–T5 `op-transport`, V1 `feed-view`, R1
`post-revision`, H1–H2 `thread-read`). Each Decision below names the labels it
carries. Throughout, **"archived design"** means
`openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`, and
**"archived proposal"** means the `proposal.md` beside it. Both are history and
are not edited.

There is no code in this change. The rules the reasoning explains are
implemented in `dialectica-core` (`arrival.rs`, `authoring.rs`, `transport.rs`,
`log/sqlite.rs`) as #165 left them.

## Goals / Non-Goals

**Goals:**

- Every passage the deltas remove is either argued below or covered by a
  citation of the archived design that argues it.
- The reasoning is grouped by the decision it explains rather than by the spec
  it was removed from, because several passages in different specs argue one
  decision.

**Non-Goals:**

- Removing reasoning that predates #165. `proposal.md` scopes it out.
- Re-arguing what the archived design already argues. Where it does, a
  Decision here cites it.
- Checking the reasoning for truth. It is carried as #165 wrote it. Where a
  claim rests on something outside this repo, it is marked as #165's.

## Decisions

### 1. The reasoning goes here, and the archived design is cited rather than extended

*Chosen:* this change's `design.md` holds every removed passage the archived
design does not already argue. Where the archived design argues a passage, the
Decision cites its Decision number instead of repeating it.

*Considered:* **editing the archived design** to add what it lacks. *Rejected*:
the archive is the record of what #165 decided and why, and editing it makes
that record say something #165 did not. **Leaving the prose in the specs.**
*Rejected* by the owner's ruling. **Citing the archived proposal** for passages
only it argues (its "The recorded rules this change reverses"). *Rejected* as
the only home: a proposal records why a change was wanted, and a reader looking
for why a mechanism is built as it is reads a `design.md`. The Decisions below
cite the archived proposal where it helps, and still state the reasoning.

Where each label went:

| Label | Destination |
|---|---|
| O1 | Decision 2, citing archived design Decision 2 |
| O2 | Decision 2 |
| O3, O4 | archived design Decisions 1, 2 and 7 (see Decision 3) |
| O5 | Decision 5 |
| O6, O7 | Decision 3 |
| O8, O9 | Decision 7 |
| O10 | Decision 8 |
| O11 | archived design Decisions 1 and 2 (see Decision 4) |
| O12 | Decision 4 |
| O13 | Decision 5, citing archived design Decision 3 for its first half |
| O14 | Decision 6 |
| O15, O16 | Decision 8 |
| O17 | archived design Decisions 1, 10 and 11 (see Decision 8) |
| O18 | archived design Decision 11 and its first Risk (see Decision 8) |
| F1, F2, F4 | Decision 7 |
| F3 | Decision 8 |
| T1 | Decision 9, citing archived design Decision 9 |
| T2 | archived design Decision 9 |
| T3, T4 | archived design, Risks, "A peer that holds an op, and then has its clock set back" |
| T5 | Decision 8 |
| V1, R1, H1, H2 | Decision 10 |

`proposal.md`, "Judgement calls", records which sentences were kept as
behaviour and which pre-#165 text went out with #165's rewrite of it. Those
are the spec-writer's calls and are not repeated here.

### 2. The time never enters a comparison, and the transport's clock still cannot be the order (O1, O2)

**O1.** The current time decides exactly two things: the counter an op is
signed with, and whether an arriving op is admitted. It never enters a
comparison between two ops. The archived design's Decision 2 shows how: `core`
reads no clock, and the time reaches it only as an argument to publish and to
receive. The ordering rule's property that every input to a comparison travels
with the op is therefore untouched by pegging.

**O2.** `op-ordering` forbids ordering by the transport's Lamport timestamp.
Before #165 the reason was that the transport's clock "is initialised from
epoch-milliseconds, so it can never agree with a counter advanced on ops".
Pegging made that reason false: the transport's clock and this system's counter
now both start from epoch milliseconds and take the same step on send. The
reason that still holds, as #165 recorded it, is that the transport's clock also
advances on traffic that no application sees. The two clocks therefore advance
on different sets of messages and cannot be relied on to agree op for op. The
prohibition stands on that, and on the older argument the spec keeps: two orders
over the same messages disagree without producing an error.

### 3. The clock is the highest counter held (O3, O4, O6, O7)

**O3 and O4** are argued in the archived design. Decision 1 says why the advance
bound's exclusion is gone: nothing over the window is stored, so the clock can
follow every counter held. Decision 7 says the clock became a plain maximum.
Decision 2 says the time enters `core` only at publish and receive, so the clock
reads none and stays a function of the ops held.

One nuance the archived design's Decision 7 already records: `OpLog::append`
still admits a counter the window would refuse, from a store written before
#165, a snapshot or a test fixture. "Every counter the clock reads is one the
peer admitted" is true of the receive path, and the clock is the maximum over
whatever the store holds either way.

**O6 and O7 are about scenario names kept while their meaning changed.** #165
kept "The clock reflects the highest counter it accepted" and "A rebuild reaches
the same answer as the original ingest" under their names. The first used to
require every counter to be within the advance bound, and "accepted" now means
admitted to the store. It holds however far apart the counters are, because the
clock excludes no counter the peer holds. The second used to fix the clock at the
highest counter within the bound, below an over-bound op's counter. With no
bound, the clock is the highest counter held. The names were kept because a
delta addresses a scenario by name.

### 4. A peer that holds nothing publishes at its own time (O11, O12)

**O11** is argued in the archived design's Decision 1: the publish rule is
SDS's send rule (LIP-109, `max(timeNowInMs, current_lamport_timestamp + 1)`).
It is applied per Stoa because the clock it reads is per Stoa. Decision 2 says
why the time enters there and not in the clock.

**O12.** Pegging lets a peer that holds nothing order correctly. A peer that has
received nothing of a Stoa publishes at its current time, which places its op
among the Stoa's recent ops. Under a pure counter it would have signed one,
below every op it had not yet received. This is also SDS's reason for starting
its timestamp at the current epoch time: a new joiner orders correctly against
recent messages without first syncing past ones. The archived proposal cites the
LIP-109 lines ("Participant state", lines 148–155).

### 5. What a pegged counter discloses (O5, O13)

`op-ordering` keeps, as a privacy property, that a published counter can
disclose the highest counter its author held.

**O5. Pegging narrows that leak compared with a pure counter.** Under a pure
counter every publish signed one above the clock, so every op disclosed the
highest counter its author held, and any op the observer planted was a usable
probe. Under pegging the counter discloses the clock only where the author holds
an op at or above its own current time. A probe therefore needs an op signed
ahead of the target's time, and the receive window caps how far ahead that can
be.

**O13.** The first half, that the counter is computed from the time and not read
from the wall-clock field, so the field decides nothing, is the archived
design's Decision 3. The second half: one reading of the time signing both
fields is what makes exact `op-ordering`'s statement that a counter at its
author's own time discloses nothing the wall-clock field of an honest op does
not. The two carry the same value in that case, not two readings a moment apart.

### 6. The reply scenario was replaced, not reworded (O14)

"A reply carries a greater counter than the post it replies to, and the rule
places it first" replaces "A reply orders after the post it replies to", which
was issue #162's first item. The old scenario's last clause placed the post
first, which contradicts the ordering rule: the reply carries the greater
counter, and the rule places the greater counter first. The place where a reply
does appear after the post it answers is `thread-read`, which returns a thread's
replies in the reverse of the rule. So the scenario was replaced with one that
states what the rule does, and the reader-facing order stays `thread-read`'s.

### 7. The author's claimed time now decides; the wall-clock field still does not (O8, O9, F1, F2, F4)

This is the second of the two rules the owner's decision reverses on purpose.
The archived proposal lists where it was recorded.

**O8. The wider principle, that the author's claimed time decides nothing, is
withdrawn.** `op-ordering`'s wall-clock requirement used to stand for it. The
counter is now pegged to its author's clock: an honest author signs the later of
its current time and one above its clock. So the counter carries the author's
reading of the time, and that reading decides both where the op orders and,
through the receive window, whether a peer admits it at all. What bounds the
author's choice is no longer that time decides nothing. It is that the window
refuses a counter more than one hour ahead of the receiving peer's own time, so
an author can lead honest ops by at most one hour.

**O9, F4. What survives is narrower and still holds.** The *separate* wall-clock
field orders nothing, gates nothing, and is handed out in a form that resists
being sorted. Nothing checks it. The window reads the counter and never this
field, which the archived design's Decision 5 makes structural: the predicate
takes a `u64` counter and cannot read the field. A value bounded by nothing must
not decide anything, so the field remains a display value on which nothing is
decided.

**F1. Why an op carries the two fields at all, when `op-format` once forbade
both.** The prior text refused a self-asserted Lamport value as "forgeable by
exactly the author it is meant to order", and a wall clock as "a field the
adversary sets". Both observations are true and neither is withdrawn. What
changed is that the transport does not supply the alternative and is not going
to: no Lamport value reaches this system, so the prohibition's practical effect
was not a transport-assigned order but no order at all, with every resolver
falling to a hash. The two objections are answered rather than set aside, each
somewhere a test can reach:

- **The forgeable counter** is bounded by the receive window. An author can lead
  honest ops by at most one hour and cannot place an op beyond it.
- **The adversary-set wall clock** is not bounded into safety. It is removed
  from every decision: it orders nothing, breaks no tie and gates nothing, so
  there is no decision for an adversary's value to reach.

**F2. Why they remain two fields, now that both carry a time.** Before #165,
`op-format` separated them by saying a counter is meaningful only relative to
ops a peer has seen, while a wall-clock is an absolute claim about the world
that a peer has nothing to check against. Pegging makes the counter a claim
about the time too, so that distinction no longer holds and is withdrawn. What
separates them now is what each is held to. The counter is checked from above
against the receiving peer's own time, and is raised past every counter its
author held, so it may order. The wall-clock is checked by nothing, so it may
not. Merging the two is out of scope by the owner's ruling on #162.

### 8. An op is now refused for a field value, and the cost is accepted (O10, O15, O16, O17, O18, F3, T5)

This is the first rule the owner's decision reverses: that no op is refused for
a field value. The archived proposal lists where it was recorded.

**O17** is argued in the archived design. Decision 1 gives the defect the
advance bound caused: it stored every counter but kept the clock below ops
already ordered above it, so an honest answer signed a lower counter than the
op it answered and came after it in a thread. It also gives the rejected
alternatives. Decision 10 records that a counter at `u64::MAX` bought its author
the head of the order permanently under the bound, and is refused now. Decision
11, "There is no ratchet", is why the most an author can lead honest ops by is
one hour.

**O18** is argued in the archived design's Decision 11, which works through a
receiver whose clock is slow or fast, and its first Risk: a peer more than an
hour wrong is cut off and is not told. The one sentence #165 added beyond those
is the sizing: one hour of tolerance is far larger than the skew of any clock
that is synchronised at all.

**F3. `op-format`'s wall-clock rule is now narrower than the principle it was
written from.** It was written as one instance of the wider rule, and its reason
was that refusing an op for a bad clock is a censorship vector. A peer whose
system clock is wrong (skewed, unset after a battery failure, or misconfigured)
would have every op it publishes dropped by every conforming peer, silently and
everywhere at once, with no error path by which the author learns of it. The
wider rule is withdrawn, and that reason now describes a cost this system
accepts: an honest peer signs its current time into the counter, and the window
refuses a counter more than an hour ahead, so a peer whose clock runs more than
an hour fast has its ops refused exactly as described. What `op-format` still
guarantees is that the wall-clock *field* is never the reason.

**T5.** `op-transport` pointed at `op-ordering` for this reasoning and for the
reversal. Both are here now, and the spec keeps only the boundary between the
two capabilities.

**O10. The window is not the promotion `op-ordering`'s clamp requirement
forbids.** That requirement says display clamping is acceptable only because
nothing depends on it, and so may never become an ordering rule. The window and
the clamp are separated by the value each reads. The window reads the counter,
which orders, and refuses to admit an op whose counter is too far ahead. The
clamp reads the wall-clock field, which orders nothing, and changes only how
that field is shown. A peer with a badly skewed clock does pay a price under the
window: it refuses honest ops, or has its own refused. That price belongs to the
window, and the clamp adds nothing to it. The spec keeps the behaviour half:
the clamp must not cause any op to be refused.

**O15. There is no lower bound because late history is old by definition.**
History that reaches a peer late, through repair, sync or a snapshot of another
peer's store, is in the past by definition, and a lower bound would refuse
exactly that history. The owner's decision on #162 states the same.

**O16. An op carrying no counter is not subject to the window** because it was
encoded under the version before the clock fields and carries nothing to
compare. The archived design's Decision 5 records how: the caller's `Option`
skips the check.

### 9. Why the transport boundary reports six refusals apart (T1, T2, T3, T4)

**T1.** A boundary reporting only "invalid" sends the reader looking in the
wrong place. The six refusals have six different causes and six different
responses: a build that is behind, a corrupt or hostile payload, a forgery, a
misdirected or replayed op, a peer sending more than the network permits, and an
op signed by a clock more than an hour ahead of this one or by an author
choosing a counter to jump the order. A peer that cannot tell them apart cannot
report which one is happening to its user or to a log. The archived design's
Decision 9 adds why the window's refusal carries both numbers: so that a log line
can tell a clock an hour fast from a counter chosen to jump the order.

**T2** is the archived design's Decision 9: the window is judged last so that a
forgery is reported as a forgery, not by a field its forger chose.

**T3 and T4** are the archived design's Risk "A peer that holds an op, and then
has its clock set back by more than an hour, refuses that op if it arrives
again". Validation precedes every lookup by a property of the op, and finding
whether an op is already held is a lookup by its op id, so a held op beyond the
window is refused as ahead of time rather than reported as held. That can happen
only when the peer's current time has moved back after it admitted the op.

### 10. What a reader may say a counter means (V1, R1, H1, H2)

Before #165, `feed-view`, `post-revision` and `thread-read` each justified a
prohibition by saying a counter is a *causal* order that carries no time. Pegging
made that false, and #165 rewrote each reason. The prohibitions themselves did
not change, and the specs keep them. The reasons are these.

**A counter is its author's own claim about the time, raised above every counter
that author held, and nothing verifies it.** An author may sign up to an hour
ahead of a receiving peer's time, which is what the window tolerates. An author
whose clock is slow signs behind, by any amount. It follows that:

- **V1, `feed-view`.** "Most recent" is a claim about when posts were written,
  and the ordering carries only what each author's clock said. So the feed may
  say "newest first" in the positional sense and must not say "most recent
  first".
- **R1, `post-revision`.** The current version is the one the rule places
  first. Beyond the causal guarantee that a revision written after seeing an
  earlier one orders after it, the counter carries only the author's unverified
  claim about the time. So a reader must not be told the current version is the
  most recent by any clock.
- **H1, `thread-read`, sequence.** A counter does not say which op its author
  had seen, and it does not say when either of two replies was written. Two
  replies whose authors had not seen each other's are ordered by those two
  claims. The distinction matters in a thread because each item also carries an
  asserted time, which nothing checks, and which can disagree with the sequence
  whenever an author lies in it. A caller presenting the sequence as
  chronological would be making a claim the items themselves can visibly
  contradict.
- **H2, `thread-read`, position.** The op's own counter would be the obvious
  position to hand out, and it is the wrong one, for two reasons. It is not the
  returned sequence: the replies come in the reverse of the counters' order, and
  the root comes first whatever its counter. And two counters five apart are not
  five places in anything, so a caller subtracting them computes a number with
  no referent in the thread. The alternatives all invite arithmetic that means
  nothing. What a view needs is to render the returned sequence and to know an
  item's place in the whole thread across pages. Contracting only that leaves a
  later change free to alter the token's form without breaking a caller that
  stayed inside the contract.

## Risks / Trade-offs

- **[Three code comments point at `op-ordering` for a statement this change
  removes]** → In `dialectica/rust-lib/dialectica-core/src/`: `arrival.rs`, on
  `RECEIVE_WINDOW_MS` ("`op-ordering` states the cost"), and `op.rs` twice, in
  the module doc's "forgeable counter" bullet ("`op-ordering` states the cost")
  and on the wall-clock field `asserted_ms` ("`op-ordering` states that cost").
  The cost was O18, which leaves the spec here, so once this is archived all
  three point at text that is not there. The fix is a comment edit repointing
  them to the archived design's Decision 11. It is not made in this change,
  which was scoped to change no code; it is open for the owner to route.
  `asserted_time.rs`'s "`op-ordering` says why the clamp may not be promoted"
  stays true, because the sentence it points at predates #165 and is kept.
- **[A spec read alone now states rules without #165's reasons]** → That is the
  README's rule working as intended. The reasons are in this file and the
  archived design, which the archive keeps together with the change.
- **[Reasoning that predates #165 is still in these specs]** → Out of scope, as
  `proposal.md` says. Removing it is a separate piece.

## Migration Plan

None. No behaviour, code or stored data changes. On archive, the deltas replace
the requirements #165 touched in six files under `openspec/specs/`.
