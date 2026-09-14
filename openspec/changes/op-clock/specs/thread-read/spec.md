## ADDED Requirements

### Requirement: An item carries its ordering position and the author's asserted time, as two separate fields

Each item SHALL carry the position the ordering rule gives it, as a field distinct from every other, and SHALL carry the author's asserted time as display text alongside an explicit marker that the value is the author's claim.

The two SHALL NOT be one field and SHALL NOT be derivable from one another. A caller placing items in order SHALL have a field that is correct to use for that; a caller rendering a time SHALL have a field that is correct to render and that no comparison would accept.

**The separation is the defence, and it is a shape rather than a rule anyone has to remember.** The ordering position and the asserted time answer different questions, and the second is forgeable by every author. A single field serving both would be a number that looks like a time and decides a sequence, which is the arrangement that has already failed in the nearest comparable project: an author-asserted timestamp read for ranking, with nothing clamping it, letting a post claiming a future instant pin itself above every honest one permanently. Their validator noticed future timestamps and produced a warning that was never called on the ingest path. The defect was not the missing check; it was that the value was sitting there as a number for whoever wanted to rank by it.

The asserted time SHALL be clamped for display and reported as clamped where it is implausible, per the ordering capability's requirement, and this capability SHALL NOT define a clamp of its own.

An item whose op carries no asserted time — one encoded before the fields existed — SHALL report the time as absent rather than substituting a value. A substituted value is indistinguishable from an asserted one once rendered, and there is nothing to substitute that would be true.

#### Scenario: Every item carries both fields

- **WHEN** a thread's items are read
- **THEN** each carries a field giving its position in the order
- **AND** each carries the author's asserted time as display text

#### Scenario: The time is not a number a comparison would accept

- **WHEN** an item carrying an asserted time is read
- **THEN** the time is display text
- **AND** no field of the item carries the instant as a bare number

#### Scenario: The asserted time is marked as the author's claim

- **WHEN** an item carrying an asserted time is read
- **THEN** it carries a marker identifying the value as author-asserted

#### Scenario: Ordering the items requires only the position field

- **WHEN** items are placed in the order the read returned them
- **THEN** the position field alone determines that order
- **AND** the asserted time is not consulted

#### Scenario: An item whose op predates the fields reports the time as absent

- **WHEN** an item is read whose op carries no asserted time
- **THEN** the time is reported as absent
- **AND** no substitute value is reported in its place

#### Scenario: An implausible asserted time is reported as clamped

- **WHEN** an item is read whose op asserts a time far beyond the allowance
- **THEN** the displayed time is clamped
- **AND** the item reports that it was clamped

## MODIFIED Requirements

### Requirement: The items are a flat sequence in the system's order, with the root first

The items SHALL be a flat sequence. Each SHALL name its parent, which is what lets a caller reconstruct the reply structure; the sequence itself SHALL NOT be nested, and the read SHALL NOT report a depth or an indentation level.

The root post SHALL be the first item of the first page. The replies SHALL follow in the order the system's ordering rule places them, and this capability SHALL NOT define an order of its own, SHALL NOT sort by arrival, and SHALL NOT compare any value itself. A second implementation of the ordering rule could disagree with the first, and two orders that disagree produce no error anywhere — each peer stays internally consistent while rendering one thread differently from its neighbour.

**What that order guarantees is convergence and causal position, and still not wall-clock recency.** The rule leads with the Lamport counter an op carries in its own signed bytes, so a reply written after its author saw another reply orders after it, and every peer holding both agrees. Where an op carries no counter — one encoded before the clock fields existed — the rule falls back to ascending op id, which is a hash and carries no temporal meaning whatever.

**A caller SHALL NOT be told the sequence is chronological.** A Lamport order says a reply was written knowing of what precedes it; it does not say when either was written, and two replies whose authors had not seen one another's are separated by op id. The distinction is not pedantic here: the asserted time an item carries is the author's claim and disagrees with the sequence whenever an author's clock is wrong or an author is lying, so a caller presenting the sequence as chronological would be making a claim the items themselves can visibly contradict.

A field MAY be named or described in terms of the order — *latest*, *first*, *position* — where what it names is the ordering rule's position. A field SHALL NOT be named or described in terms of wall-clock time unless it is the author-asserted time, which SHALL be marked as such.

A flat sequence is specified rather than a tree because whether a thread view should paginate by reply order or by reply tree is an open question that running against real traffic decides, and a flat page carrying each item's parent does not foreclose either answer. A tree would need a page boundary chosen before anyone knows where one should fall.

**A view rendering the replies nested is served by this shape and is not in tension with it.** Nesting is a depth, a depth is a function of how many parents separate a post from its root, and every item carries the parent that answers it. A reader therefore computes the indentation it wants from what it was given. Reporting a depth from here would be a second answer to a question the parent field already settles, and the two could disagree on a partial set of ops — which is why the read reports parents and not depths.

#### Scenario: The root is the first item

- **WHEN** the first page of a thread with several replies is read
- **THEN** the first item is the root post

#### Scenario: The root is not repeated on a later page

- **WHEN** a thread with more replies than fit one page is read page by page
- **THEN** the root appears on the first page only
- **AND** it occupies one of that page's item slots rather than being carried beside them

#### Scenario: Two peers holding the same ops return the same sequence

- **WHEN** two peers hold the same ops, appended in opposite sequences, and each reads the thread
- **THEN** both return the same sequence of op ids

#### Scenario: The sequence is the ordering rule's and is not re-sorted here

- **WHEN** a thread's replies are read
- **THEN** their relative order is the one the ordering rule places them in
- **AND** it is not the sequence they were appended in, where the two differ

#### Scenario: A reply orders after the reply it answers

- **WHEN** one peer publishes a reply, a second receives it and publishes a reply to it
- **THEN** the second orders after the first in the returned sequence

#### Scenario: The sequence does not follow the asserted times

- **WHEN** a thread's replies carry asserted times that disagree with the order their counters give
- **THEN** the returned sequence is the one the counters give
- **AND** the asserted times did not affect it

#### Scenario: A deep chain of replies comes back flat

- **WHEN** a thread holds a reply to a reply to a reply and is read
- **THEN** all of them appear as items of one sequence
- **AND** each names its own parent
- **AND** no item carries a nested list of its own replies
