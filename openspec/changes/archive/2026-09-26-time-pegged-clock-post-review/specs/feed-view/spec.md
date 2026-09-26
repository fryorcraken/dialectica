## MODIFIED Requirements

### Requirement: The ordering label claims a position in the forum's order and never a position in time

The feed SHALL label its ordering in terms of position within the forum's order, and SHALL NOT label or describe it as being by wall-clock time.

**The distinction is between two readings of a superlative, not between two vocabularies**, and stating it that way is what keeps this requirement from forbidding its own label. *Newest*, *latest* and *first* each have a reading that is about position in a sequence and a reading that is about instants. This requirement permits the positional reading and forbids the temporal one; it does not maintain a list of banned words, which would go stale the moment someone invented a twelfth way to claim recency.

**"Newest first" is permitted and is the label this interface uses.** It is honest read as *latest in the forum's own order*, which is what the ordering is: `op-ordering` orders by descending Lamport counter carried in each op's signed bytes, with ties broken by ascending op id. A post that orders first is the latest one the forum's order knows of. That is a real claim, it is the claim the reader wants, and every peer holding the same ops computes the same answer.

**"Most recent first" is forbidden, and the difference between the two phrases is the whole of this requirement.** The feed SHALL NOT render that phrase, nor any other phrase asserting that the sequence is chronological, in time order, by date, or by when posts were written.

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
