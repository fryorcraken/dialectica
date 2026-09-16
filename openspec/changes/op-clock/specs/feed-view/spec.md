## Purpose

Defines what the feed screen may and may not claim about the order it presents posts in, and about the time it shows against each one. It exists because the feed is where a reader forms their belief about *when* things happened, and the two values behind that belief — a causal counter and an author's own assertion — support a much narrower claim than a forum feed is ordinarily read as making.

## ADDED Requirements

### Requirement: The ordering label claims a position in the forum's order and never a position in time

The feed SHALL label its ordering in terms of position within the forum's order, and SHALL NOT label or describe it as being by wall-clock time.

**The distinction is between two readings of a superlative, not between two vocabularies**, and stating it that way is what keeps this requirement from forbidding its own label. *Newest*, *latest* and *first* each have a reading that is about position in a sequence and a reading that is about instants. This requirement permits the positional reading and forbids the temporal one; it does not maintain a list of banned words, which would go stale the moment someone invented a twelfth way to claim recency.

**"Newest first" is permitted and is the label this interface uses.** It is honest read as *latest in the forum's own order*, which is what the ordering is: `op-ordering` orders by descending Lamport counter carried in each op's signed bytes, with ties broken by ascending op id. A post that orders first is the latest one the forum's order knows of. That is a real claim, it is the claim the reader wants, and every peer holding the same ops computes the same answer.

**"Most recent first" is forbidden, and the difference between the two phrases is the whole of this requirement.** A Lamport counter is a *causal* order: it says an author had seen what precedes their post, never when either was written. "Most recent" is a claim about instants, and the ordering carries no instants. The feed SHALL NOT render that phrase, nor any other phrase asserting that the sequence is chronological, in time order, by date, or by when posts were written.

The denial SHALL be rendered rather than left implicit. A reader meeting a forum feed assumes a chronological one unless told otherwise, so a label that merely declines to claim time leaves the interface relying on the reader not to make the ordinary assumption. What is rendered SHALL state that the feed is not ordered by the time it displays, and SHALL give as the reason that the displayed time is the author's own claim — not that no time is available. Every op carries a time; the feed declines to order by it. A denial phrased as a limitation awaiting a missing field would promise a chronological feed that this design has decided against.

**The denial SHALL deny the temporal reading, and SHALL NOT negate the label.** This is where the two halves of this requirement can be made to collide, and they did: an interface can carry an honest label and an honest denial and still contradict itself, if the denial reaches for the label's own words to name what is being denied. A denial opening *"Not newest first"* asserts that the feed is not the thing its own label says it is, which leaves a reader no way to tell which of the two strings is live. Because the permitted reading and the forbidden one are readings of the *same* superlative, the denial SHALL name the forbidden reading — ordering by the displayed time, by the clock, by when posts were written — rather than the superlative itself. Where the denial refers to the label, it SHALL distinguish the two readings rather than reject the label outright.

**This SHALL NOT be satisfied by one honest string beside a second string that undoes it.** No text the feed renders, in any of its states, SHALL assert a chronological ordering.

#### Scenario: The ordering label names the order rather than a time

- **WHEN** the feed renders its ordering label
- **THEN** what is rendered describes a position in the forum's order
- **AND** it does not describe the sequence as chronological, as by date, or as by when posts were written

#### Scenario: The phrase asserting a clock order does not appear

- **WHEN** every string the feed renders, in each of its states, is examined
- **THEN** no string among them states that the feed is ordered most recently first
- **AND** no string among them states that the order is by the time posts were written

#### Scenario: The denial is present and gives the author-assertion as its reason

- **WHEN** the feed is rendered
- **THEN** it states that the feed is not ordered by the time it displays
- **AND** the reason it gives is that the displayed time is the author's own claim, which anyone could set
- **AND** it does not give as the reason that no time is available to this machine

#### Scenario: The denial does not negate the ordering label

- **WHEN** the feed renders its denial
- **THEN** what it states the feed is not ordered by is the time it displays
- **AND** it does not assert that the feed is not ordered newest first
- **AND** where it uses the label's own words, it distinguishes the positional reading from the temporal one rather than rejecting the label

#### Scenario: No second string undoes the denial

- **WHEN** the feed is rendered in its populated, empty and failed states
- **THEN** no string rendered in any of them asserts a chronological ordering

### Requirement: A displayed time is marked as the author's assertion, never as a verified instant

Where the feed renders a post's time, it SHALL render alongside it an indication that the value is what the post's author asserted, and SHALL NOT present it as an instant this peer or any other has verified.

**The feed renders no time today, and this requirement is written to bind when it does rather than to describe something that exists.** The feed listing's rows carry no asserted time: `thread-read` contracts that field on a **thread's** items, and the feed lists threads through a separate reply shape that was not widened by this change. So the requirement's force today is a **prohibition** — the feed MUST NOT render a time it has not been given, which the absent-time clause below states — and its positive half binds for exactly as long as the field is absent, becoming unconditional the moment a feed row carries one. It is stated now because the field's arrival is the moment the marking is easiest to omit, and because a screen that acquired a bare time would be making the forgeable-as-verified claim with nothing in the spec to stop it.

Nothing verifies it and nothing can. The wall-clock sits inside the signed preimage, so a relay cannot forge it, but the **author** sets it freely and an author is not trusted — `op-ordering` requires that the value decide no ordering, comparison, resolution or gating for exactly that reason, and hands it to a reader as display text rather than as a number a comparison would accept. A time rendered bare reads as an observed fact, and a reader who takes it as one has been given a forgeable value with the authority of a verified one.

Where `thread-read` reports a time as **clamped**, the feed SHALL NOT present the clamped value as the time the author asserted. The clamp is this peer's substitution for an implausible claim, so rendering it unmarked would attribute to the author a time they did not assert.

Where `thread-read` reports a time as **absent** — an op encoded before the clock fields existed — the feed SHALL render no time for that post and SHALL NOT substitute one. A substituted value is indistinguishable from an asserted one once rendered.

#### Scenario: A rendered time carries its author-assertion marking

- **WHEN** the feed renders a post whose item carries an asserted time
- **THEN** what is rendered indicates the time is the author's claim
- **AND** nothing rendered presents it as verified, confirmed, or observed by this peer

#### Scenario: A clamped time is not attributed to the author

- **WHEN** the feed renders a post whose item reports its time as clamped
- **THEN** what is rendered does not present the clamped value as the time the author asserted

#### Scenario: An absent time renders as no time at all

- **WHEN** the feed renders a post whose item reports its time as absent
- **THEN** no time is rendered for that post
- **AND** no substitute value is rendered in its place

### Requirement: Rows out of chronological order are correct, and the interface does not present itself as sorting by the time it shows

The feed SHALL place rows in the order the read returned them, and SHALL NOT reorder them by any value it displays. Where the feed displays times, a row whose displayed time is earlier than the row below it SHALL be rendered in the position the order gives it, and the disagreement SHALL NOT be treated as an error, hidden, or corrected.

**The ordering half binds today and the display half binds when times arrive.** Core returns the feed's rows already in the ordering rule's sequence, so what this requires of the view now is that it render that sequence and compute no sequence of its own — which is checkable today, against rows whose returned order differs from any order the view might have preferred. The out-of-chronological-order consequence becomes observable once a row carries a time, and is stated here because it is the point at which someone will read the disagreement as a bug.

**This is the observable consequence of ordering causally while displaying an asserted time, and it is correct behaviour rather than a defect.** Two authors who had not seen one another's posts are separated by op id; an author whose clock is wrong, or who is lying, asserts a time that disagrees with their post's position. So a feed showing both values will, on real data, show them disagreeing. A build that "fixed" this by sorting on the displayed time would be sorting on a value any author can set, which is the ranking failure `docs/PLAN.md` Appendix A measures in the nearest kin project: a post claiming a future instant pins itself above every honest one, permanently.

The feed SHALL NOT render the displayed time as though it were the sort key — it SHALL NOT be presented as a column the rows are ordered by, and no affordance SHALL offer to sort or re-sort the feed by it. An interface that *looks* like it sorts by the time it shows makes the disagreement read as a bug in the feed rather than as the honest difference between two different values.

#### Scenario: The rendered sequence is the returned sequence

- **WHEN** the feed is given a page of items in a given sequence
- **THEN** the rows are rendered in that sequence
- **AND** the view computes no sequence of its own from the items' contents

#### Scenario: A row with an earlier displayed time can sit above one with a later time

- **WHEN** the feed is given items carrying displayed times that disagree with the sequence they were returned in
- **THEN** the rows are rendered in the returned sequence
- **AND** the row whose displayed time is earlier appears above the row whose displayed time is later, where the returned sequence places it there
- **AND** nothing is rendered marking that pair as an error or an inconsistency

#### Scenario: No affordance offers to sort by the displayed time

- **WHEN** the feed is rendered
- **THEN** no control is offered that sorts or re-sorts the rows by the time displayed against them
