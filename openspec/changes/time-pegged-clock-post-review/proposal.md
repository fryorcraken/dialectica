## Why

The change that pegged the op counter to the time (PR #165, archived as `openspec/changes/archive/2026-09-25-time-pegged-clock/`) wrote the reasoning for its reversed rules into six live specs. It read the owner's decision on issue #162, "The spec change must say so and why", as permission to do that. Asked about it, the owner answered "follow the readme". `.claude/agents/README.md`, "Reasoning migrates to `design.md`; behaviour migrates to a spec", says: "Reasoning never goes in a spec at all — a spec is a behaviour contract". This change takes that reasoning out of the specs and moves it to this change's `design.md`.

The same PR also merged four commits that landed after its review round and were never reviewed. This piece's review rows cover those commits (see "Review scope" below).

## What Changes

- **Remove the reasoning prose that #165 added to, or rewrote in, the live specs** of `op-ordering`, `op-format`, `op-transport`, `feed-view`, `post-revision` and `thread-read`. That covers history ("previously", "used to", "this replaces"), justification ("on purpose", "the cost is stated", the SDS provenance) and scenario notes about renaming.
- **No behaviour changes.** Every requirement keeps its normative text, and every scenario is carried over word for word. Where a paragraph mixed a behaviour statement with reasoning, the behaviour statement stays. Three of them are reworded so they read on their own. "Judgement calls" below lists them.
- **Reasoning that #165 did not add or rewrite is left alone**, even where it is still reasoning. There is plenty of it in these specs. Removing it is a separate piece.
- **Nothing is lost.** The archived `design.md` of #165 is history and is not edited. So every removed passage is recorded verbatim below, with where it came from, for the `dev-writer` to carry into this change's `design.md` Decisions.
- **This piece carries the review of #165's unreviewed tail.** See "Review scope".

## Review scope

Four commits landed on #165's branch after its review round and merged without review:

```
d6208fb Specify the two NO SPEC choices in time-pegged-clock (#162)
ae0c30d Reword the counter's reasoning in moderation.rs and revision.rs (#162)
c34ec46 Point the two specified choices at their requirements (#162)
c1f1a8f Pin the missing time-pegged-clock scenario: clock above time leaves the wall-clock alone (#162)
```

Their content is exactly `git diff 2eada33 c1f1a8f`. It touches `authoring.rs`, `moderation.rs`, `revision.rs`, `transport.rs` and the `op-ordering` and `op-transport` deltas. **Every review row in this change's `tasks.md` covers that range as well as this change's own diff.** A reviewer reads both.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

For each capability, the delta restates only the requirements #165 touched. The requirement text is edited to remove reasoning and the behaviour stays the same:

- `op-ordering`: "Ops are ordered by the counter they carry, not by anything the transport says"; "A peer's Lamport clock is a function of the ops it holds"; "The wall-clock decides nothing, and is handed out in a form that resists being sorted"; "An implausible wall-clock is clamped for display and reported as clamped"; "A published op takes the later of the current time and one above the author's clock"; "An op whose counter is more than one hour ahead of this peer's time is refused on arrival".
- `op-format`: "An op carries no ordering field and no per-peer state"; "An implausible wall-clock is accepted at the boundary, never refused".
- `op-transport`: "Every inbound payload is validated before it reaches storage"; "This capability decides nothing about an op beyond admitting it".
- `feed-view`: "The ordering label claims a position in the forum's order and never a position in time".
- `post-revision`: "The current version is the one the ordering rule places first".
- `thread-read`: "The items are a flat sequence in the system's order, with the root first"; "An item carries its ordering position and the author's asserted time, as two separate fields".

## Impact

- **Code, wire API, tests: none.** No behaviour changes, so no test should change. A test that has to change is evidence that this change altered behaviour, and that is a defect.
- **`openspec/specs/`**: six files, on archive.
- **`design.md`** (new, the `dev-writer`'s): gets the reasoning listed below.

## Reasoning removed from the specs, for `design.md`

Each entry gives the capability, the requirement and the passage, quoted verbatim from the live spec as #165 promoted it. The source of every passage is `openspec/changes/archive/2026-09-25-time-pegged-clock/specs/<capability>/spec.md`, where it appears under the same requirement. **"Added"** means #165 wrote the passage. **"Rewritten"** means #165 replaced text that was already there, and the entry says what that text was about.

Before writing new prose, check `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`. It may already argue some of these points. Where it does, a Decisions entry here can cite it instead of repeating it.

### `op-ordering` (18 passages)

**O1.** "Ops are ordered by the counter they carry…". Added at the end of the paragraph beginning "The property the prohibition was defending":

> **The current time this system now reads does not change that.** It decides the counter an op is signed with and whether an arriving op is admitted, and it never enters a comparison between two ops.

**O2.** Same requirement. Rewritten: it replaced the earlier reason, which said the transport's clock "is initialised from epoch-milliseconds, so it can never agree with a counter advanced on ops".

> The transport's clock and this system's counter both start from epoch milliseconds and take the same step on send, but the transport's clock also advances on traffic that no application sees. The two therefore advance on different sets of messages and cannot be relied on to agree op for op.

**O3.** "A peer's Lamport clock is a function of the ops it holds". Added:

> This clock previously excluded a counter more than a fixed bound above the rest, so that one absurd counter could not pull it to the ceiling. That exclusion is gone. The receive window, "An op whose counter is more than one hour ahead of this peer's time is refused on arrival", keeps such a counter out of the store altogether, so every counter the clock reads is one the peer admitted.

**O4.** Same requirement. Added:

> The time enters when an op is signed, through "A published op takes the later of the current time and one above the author's clock". It does not enter the clock, so the clock remains a function of the ops held and nothing else.

**O5.** Same requirement, in the paragraph that bounds the reception leak. Added:

> Pegging the counter to the time narrows the leak compared with a pure counter.

**O6.** Same requirement, the note under scenario "The clock reflects the highest counter it accepted". Added:

> This scenario keeps its name, and "accepted" now means admitted to the store. Its condition used to require every counter to be within the advance bound. It now holds however far apart the counters are, because the clock excludes no counter the peer holds.

**O7.** Same requirement, the note under scenario "A rebuild reaches the same answer as the original ingest". Added:

> This scenario keeps its name. It used to fix the clock at the highest counter within the advance bound, below an over-bound op's counter. There is no advance bound now, so the clock is the highest counter held.

**O8.** "The wall-clock decides nothing, and is handed out in a form that resists being sorted". Added:

> **This requirement used to stand for a wider principle — that the author's claimed time decides nothing — and that principle is withdrawn.** The Lamport counter is now pegged to its author's clock: an honest author signs the later of its current time and one above its clock. So the counter carries the author's reading of the time, and that reading decides both where the op orders and, through the receive window, whether a peer admits it at all. What bounds the author's choice is no longer that time decides nothing. It is that the window refuses a counter more than one hour ahead of the receiving peer's own time, so an author can lead honest ops by at most one hour.

**O9.** Same requirement. Added:

> What remains of this requirement is the part that still holds and still matters: the **separate** wall-clock field orders nothing, gates nothing, and is handed out in a form that resists being sorted. Nothing checks it. The window reads the counter and never this field, so the field is bounded by nothing, and a value bounded by nothing must not decide anything.

**O10.** "An implausible wall-clock is clamped for display and reported as clamped". Added. Its closing MUST NOT stays in the spec as "The clamp MUST NOT cause any op to be refused."

> **The receive window is not that promotion, and what separates them is the value each reads.** The window reads the counter, which orders, and refuses to admit an op whose counter is too far ahead. The clamp reads the wall-clock field, which orders nothing, and changes only how that field is shown. A peer with a badly skewed clock does pay a price under the window: it refuses honest ops, or has its own refused. That price is stated in the window's requirement rather than hidden here. The clamp adds nothing to it and MUST NOT be made to.

**O11.** "A published op takes the later of the current time and one above the author's clock". Added:

> This is the send rule of the SDS protocol (LIP-109), `max(timeNowInMs, current_lamport_timestamp + 1)`, applied to each Stoa separately. The clock it reads is the one the requirement "A peer's Lamport clock is a function of the ops it holds" defines, so the current time enters the counter here, when an op is signed, and nowhere in the clock itself.

**O12.** Same requirement. Added:

> **Pegging the counter to the time lets a peer that holds nothing order correctly.** A peer that has received nothing of a Stoa publishes at its current time. That places its op among the Stoa's recent ops, where a pure counter would have signed one and placed it below every op the peer had not yet received.

**O13.** Same requirement, the end of the paragraph beginning "One reading of the time signs both clock fields". Added:

> The counter is computed from the time and not read from the field, so this gives the wall-clock field no part in any decision. It is what makes exact the clock requirement's statement that a counter at its author's own time discloses nothing the wall-clock field of an honest op does not.

**O14.** Same requirement, the note under scenario "A reply carries a greater counter than the post it replies to, and the rule places it first". Added:

> This scenario replaces "A reply orders after the post it replies to". Its last clause placed the post first, which contradicts the ordering rule, because the rule places the higher counter first. `thread-read` shows a thread's replies in the reverse of this rule, which is where a reply appears after the post it answers.

**O15.** "An op whose counter is more than one hour ahead of this peer's time is refused on arrival", in the paragraph beginning "There is no lower bound". Added:

> History that reaches a peer late, through repair, sync or a snapshot of another peer's store, is in the past by definition, and a lower bound would refuse exactly that history.

**O16.** Same requirement, in the paragraph beginning "An op carrying no counter is not subject to the window". Added, as a clause:

> …and carries nothing to compare.

**O17.** Same requirement. Added:

> **This replaces the advance bound, and it is a refusal on purpose.** The advance bound stored every counter and declined only to advance the clock past an implausible one. So the clock stayed below ops that were already ordered above it. An honest peer answering an op signed far ahead therefore signed a *lower* counter than the op it answered, and in a thread the answered reply came after every answer to it. The window removes such an op instead. Nothing more than one hour ahead is stored, so the clock can follow every counter held, and an answer always carries the greater counter. The most an author can lead honest ops by is one hour. A counter at the maximum representable value is refused; under the advance bound, the same counter bought its author the head of the order permanently.

**O18.** Same requirement. Added. Its sentence "Neither refusal reaches the author." stays in the spec as "A refusal under this requirement does not reach the op's author."

> **The cost is stated, not hidden.** This refuses an authentic op for the value of a field, which this system previously ruled out. A peer whose clock runs more than an hour fast has every op it publishes refused by every peer whose clock is right, until those peers' time catches up with its counters. A peer whose clock runs more than an hour slow refuses honest ops. Neither refusal reaches the author. One hour of tolerance is what that trade buys, and it is far larger than the skew of any clock that is synchronised at all.

### `op-format` (4 passages)

**F1.** "An op carries no ordering field and no per-peer state". The lead paragraph predates #165. #165 rewrote both bullets: the first used to name the advance rule, and the second gained its last sentence. The bullets cannot stand without the lead, so the whole unit is removed. See "Judgement calls".

> **Why the two admitted fields are admitted, when this requirement previously forbade both.** The prior text refused a self-asserted Lamport value because it "would be forgeable by exactly the author it is meant to order", and refused a wall clock because "a wall clock is a field the adversary sets". Both observations are true and neither has been withdrawn. What changed is that the transport does not supply the alternative and is not going to: no Lamport value reaches this system, so the practical effect of the prohibition was not a transport-assigned order but **no order at all**, with every resolver falling to a hash. The two objections are answered rather than set aside, and each is answered somewhere a test can reach:
>
> - **The forgeable counter** is bounded by `op-ordering`'s receive window, which refuses an op whose counter is more than one hour ahead of the receiving peer's own time. An author can lead honest ops by at most that hour, and cannot place an op beyond it at all.
> - **The adversary-set wall clock** is not bounded into safety; it is removed from every decision. It orders nothing, breaks no tie, and gates nothing. The window reads the counter and never this field, so there is no decision for an adversary's value to reach.

**F2.** Same requirement. Rewritten: it replaced "The distinction the prior text collapsed", which argued that a counter is meaningful only relative to ops a peer has seen.

> **Why the two remain two fields, now that both carry a time.** The counter is pegged to its author's clock, so it is a claim about the time just as the wall-clock is. This requirement previously separated them by saying a counter is meaningful only relative to ops a peer has seen, while a wall-clock is an absolute claim about the world that a peer has nothing to check against. That distinction no longer holds and is withdrawn. What separates the two now is what each is held to. The counter is checked from above against the receiving peer's own time, and is raised past every counter its author held, so it may order. The wall-clock is checked by nothing, so it may not.

**F3.** "An implausible wall-clock is accepted at the boundary, never refused". Rewritten: it replaced "Refusing an op for a bad clock is a censorship vector, and that is the reason rather than a caveat."

> **This rule is now narrower than the principle it was written from, and the difference is deliberate.** It was written as one instance of a wider rule, that no op is refused for a field value. Its reason was that refusing an op for a bad clock is a censorship vector: a peer whose system clock is wrong (skewed, unset after a battery failure, or misconfigured) would have every op it publishes dropped by every conforming peer, silently and everywhere at once, with no error path by which the author learns of it. **That wider rule is withdrawn**, and the reason now describes a cost this system accepts. `op-ordering` refuses an op whose *counter* is more than one hour ahead of the receiving peer's time, and an honest peer signs its current time into the counter. So a peer whose clock runs more than an hour fast has its ops refused exactly as described. `op-ordering` states that cost and why it is accepted.

**F4.** Same requirement. Added. The framing and the closing clause are removed, and the behaviour sentences stay:

> What this requirement still guarantees is that the **wall-clock field** is never the reason. […] so the field remains a display value on which nothing is decided.

### `op-transport` (5 passages)

**T1.** "Every inbound payload is validated before it reaches storage". Rewritten: #165 changed "five failures" to "six" and added the sixth cause.

> **A boundary reporting only "invalid" sends the reader looking in the wrong place.** These six failures have six different causes and six different responses: a build that is behind, a corrupt or hostile payload, a forgery, a misdirected or replayed op, a peer sending more than the network permits, and an op signed by a clock more than an hour ahead of this one or by an author choosing a counter to jump the order. A peer that cannot tell them apart cannot report which of them is happening to its user or to a log.

**T2.** Same requirement, in the paragraph beginning "The window is judged last". Added. Also removed: the words "and its reasoning" from "The window's rule and its reasoning belong to `op-ordering`".

> A forgery reported as "too far ahead" would describe a forged op by a field its forger chose.

**T3.** Same requirement, in the paragraph beginning "An op this peer already holds". Added, together with the "therefore" that followed from it:

> Validation precedes every lookup by a property of the op, as the opening of this requirement says, and finding whether an op is already held is a lookup by its op id.

**T4.** Same paragraph. Added:

> This can happen only when this peer's current time has moved back after it admitted the op.

**T5.** "This capability decides nothing about an op beyond admitting it". Added:

> Its reasoning, and the reversal of this system's earlier rule that no op is refused for a field value, belong to `op-ordering`.

### `feed-view` (1 passage)

**V1.** "The ordering label claims a position in the forum's order and never a position in time". Rewritten: it replaced "A Lamport counter is a *causal* order: it says an author had seen what precedes their post, never when either was written. "Most recent" is a claim about instants, and the ordering carries no instants."

> The order is by Lamport counter, and a counter is its author's own claim about the time, raised above every counter that author held. Nothing verifies that claim. An author may sign it up to an hour ahead of a receiving peer's time, and an author whose clock is slow signs it behind. "Most recent" is a claim about when posts were written, and the ordering carries only what each author's clock said.

### `post-revision` (1 passage)

**R1.** "The current version is the one the ordering rule places first". Rewritten: #165 changed "is a causal order: … It says nothing about wall-clock time". The two fragments removed are:

> …pegged to its author's clock.
>
> Beyond that, it carries only the author's own claim about the time, which nothing verifies, and…

### `thread-read` (2 passages)

**H1.** "The items are a flat sequence in the system's order, with the root first". This is the body of the paragraph beginning "A caller SHALL NOT be told the sequence is chronological", and the SHALL NOT stays. Rewritten: it replaced an argument from "A counter says its author had seen something at the counter below it".

> A counter is its author's claim about the time, raised above every counter that author held. It does not say which op the author had seen, and it does not say when either reply was written, because an author's clock can run ahead by up to the hour `op-ordering`'s receive window tolerates, or behind by any amount. Two replies whose authors had not seen each other's are ordered by those two claims. The distinction is not pedantic here. The asserted time an item carries is a separate claim that nothing checks, and it can disagree with the sequence whenever an author lies in it, so a caller presenting the sequence as chronological would be making a claim the items themselves can visibly contradict.

**H2.** "An item carries its ordering position and the author's asserted time, as two separate fields". Rewritten: #165 added the "two reasons" and the reverse-order argument.

> The alternatives all invite arithmetic that means nothing. The op's own counter would be the obvious value to hand out, and it is the wrong one, for two reasons. First, it is not the returned sequence: the replies come in the reverse of the counters' order, and the root comes first whatever its counter. Second, a counter is its author's claim about the time, raised above whatever that author held, so two counters five apart are not five places in anything, and a caller subtracting them would compute a number with no referent in the thread. What a view needs is to render the returned sequence and to know an item's place in the whole thread across pages. Contracting only that keeps a later change free to alter the token's form without breaking a caller that stayed inside the contract.

## Judgement calls

**Removed together with #165's rewrite, although the text predates it.** F1's lead paragraph, T1's opening and most of its list of causes, and H2's opening and final sentence were all written before #165. #165 rewrote part of each unit, and what it left would not read alone. A reviewer who wants any of them back should say so. Restoring one is a spec edit that changes no behaviour.

**Kept as behaviour, not reasoning:**

- `op-ordering`, clock requirement: "The clock is the highest counter held, with no exception for a counter far above the rest." and "The clock does not read the current time."
- `op-ordering`, the two paragraphs on the reception leak, except O5. They state a privacy property: what a published counter discloses, and to whom.
- `op-ordering`, publish requirement: "The counter is a causality mechanism." Only the word "still" is dropped. The paragraph states the guarantee that the scenarios on answering pin. "What a counter states and what it does not" and "Saturation, not overflow" are also kept in full. Their explanatory sentences are carried over word for word from the text before #165, so they are outside this change's scope.
- `op-ordering`, window requirement: "Such an op was encoded under the version that predates the clock fields." (a definition), and "Neither refusal reaches the author.", reworded to stand alone as "A refusal under this requirement does not reach the op's author." Refusal is silent to the author, and an implementation that notified the author would change that.
- `op-ordering`, clamp requirement: the MUST NOT at the end of O10, reworded to stand alone as "The clamp MUST NOT cause any op to be refused."
- `op-transport`: "The window's rule belongs to `op-ordering` and is not restated here.", which draws the boundary between the two capabilities; "What this capability adds is that the window is checked at this boundary…"; and "The window is a judgement on a field's value, and it is not a judgement of authority. It asks how an op's counter stands against this peer's time at the moment of arrival."
- `op-format`: "The **wall-clock field** is never the reason an op is refused. The receive window reads the counter alone. An op whose counter is admissible is stored whatever its wall-clock says."
- `post-revision`: "The exception is an author publishing from a second device that has not yet received a version signed ahead of the time. …caps that lead at one hour." This sets the limit of the promise the requirement makes. Without it, "the last revision is current" reads as unconditional.
- `thread-read`: the paragraph beginning "Where both carry counters", in full. Its middle sentences name the cases that its MUST and MUST NOT apply to.

**Touched by #165 but left in place:** in the `op-ordering` clock requirement, the paragraph "What a reader can check, and what only an implementation can." #165 changed one clause of it so that it matches the scenario list. It is otherwise reasoning from before #165, so it is outside this change's scope.
