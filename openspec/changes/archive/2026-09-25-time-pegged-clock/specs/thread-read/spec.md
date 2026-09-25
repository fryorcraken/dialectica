## MODIFIED Requirements

### Requirement: The items are a flat sequence in the system's order, with the root first

The items SHALL be a flat sequence. Each SHALL name its parent, which is what lets a caller reconstruct the reply structure; the sequence itself SHALL NOT be nested, and the read SHALL NOT report a depth or an indentation level.

The root post MUST be the first item of the first page. The replies MUST follow in the **exact reverse** of the sequence the system's ordering rule places them in: the reply the rule places last MUST be the first reply, and the reply the rule places first MUST be the last. This capability MUST NOT define an order of its own, MUST NOT sort by arrival, and MUST NOT compare any value itself. A second implementation of the ordering rule could disagree with the first, and two orders that disagree produce no error anywhere — each peer stays internally consistent while rendering one thread differently from its neighbour.

**What that order guarantees is convergence and causal position, and still not wall-clock recency.** The rule leads with the Lamport counter an op carries in its own signed bytes and places the higher counter first, so its reverse places a reply carrying the higher of two counters after the other, and every peer holding both agrees. Two consequences of reversing the rule's sequence, rather than re-sorting the replies, are part of this contract: replies carrying equal counters MUST come in descending op id, the reverse of the rule's tiebreak; and a reply carrying no counter — one encoded before the clock fields existed — MUST come before every reply that carries one, those among themselves in descending op id. An op id is a hash and carries no temporal meaning whatever.

**Where both carry counters, a reply comes after the reply it answers when its counter is the greater of the two, and the read MUST NOT restore that relation where the counters do not give it.** `op-ordering`'s publish rule gives the answering reply the greater counter whenever both replies carry counters and the answering peer held the reply it answered when it published. That includes the case where the answered reply's counter was ahead of the answering peer's time. The rule does not give it where either reply carries no counter, nor where the answering reply's author did not follow the publish rule and signed a lower counter. In those cases the replies MUST still come in the reverse of the rule's sequence, even where that places a reply before the reply it answers; the read MUST NOT move a reply after its parent.

**A caller SHALL NOT be told the sequence is chronological.** A counter is its author's claim about the time, raised above every counter that author held. It does not say which op the author had seen, and it does not say when either reply was written, because an author's clock can run ahead by up to the hour `op-ordering`'s receive window tolerates, or behind by any amount. Two replies whose authors had not seen each other's are ordered by those two claims. The distinction is not pedantic here. The asserted time an item carries is a separate claim that nothing checks, and it can disagree with the sequence whenever an author lies in it, so a caller presenting the sequence as chronological would be making a claim the items themselves can visibly contradict.

A field MAY be named or described in terms of the order — *latest*, *first*, *position* — where what it names is a place in the system's order: the ordering rule's, or the sequence this read derives from it. A field SHALL NOT be named or described in terms of wall-clock time unless it is the author-asserted time, which SHALL be marked as such.

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
- **THEN** their relative order is the exact reverse of the one the ordering rule places them in
- **AND** it is not the sequence they were appended in, where the two differ

  The name is kept while the content changes, because the name is how this delta
  addresses the scenario it amends. "The ordering rule's" now means the rule's
  sequence reversed, and nothing else.

#### Scenario: A reply orders after the reply it answers

- **WHEN** one peer publishes a reply carrying a counter, and a second receives it and publishes a reply to it carrying a counter
- **THEN** the second orders after the first in the returned sequence
- **AND** this holds when the first reply's counter was ahead of the second peer's current time when the second published

#### Scenario: A reply carrying a lower counter than the reply it answers comes before it

- **WHEN** a thread holds a reply, and an answer to it carrying a lower counter than the reply it answers, and is read
- **THEN** the answer comes before the reply it answers
- **AND** the replies are the exact reverse of the rule's sequence, with no reply moved after its parent

#### Scenario: The lowest counter leads the replies and the highest ends them

- **WHEN** a thread whose replies carry distinct counters is read
- **THEN** the first reply returned is the one carrying the lowest counter
- **AND** the last reply returned is the one carrying the highest
- **AND** this is not the sequence the ordering rule places them in, which leads with the highest

#### Scenario: Replies with equal counters are reversed rather than re-sorted

- **WHEN** a thread holds two replies carrying equal counters, and is read
- **THEN** the reply with the higher op id comes first
- **AND** this is the reverse of the order the ordering rule gives the two

#### Scenario: A reply carrying no counter precedes the replies that carry one

- **WHEN** a thread holds replies carrying no counter alongside replies carrying one, and is read
- **THEN** every reply carrying no counter comes before every reply carrying one
- **AND** the replies carrying no counter come in descending op id among themselves
- **AND** a reply carrying no counter that answers a reply carrying one comes before the reply it answers
- **AND** the root is still the first item, whether or not it carries a counter

#### Scenario: The sequence does not follow the asserted times

- **WHEN** a thread's replies carry asserted times that disagree with the order their counters give
- **THEN** the returned sequence is the one this requirement derives from the counters
- **AND** the asserted times did not affect it

#### Scenario: A deep chain of replies comes back flat

- **WHEN** a thread holds a reply to a reply to a reply and is read
- **THEN** all of them appear as items of one sequence
- **AND** each names its own parent
- **AND** no item carries a nested list of its own replies

### Requirement: An item carries its ordering position and the author's asserted time, as two separate fields

Each item SHALL carry its position in the sequence the read returns, as a field distinct from every other. That sequence is the whole thread's, with the root first and the replies in the reverse of the ordering rule's sequence, as the requirement "The items are a flat sequence in the system's order, with the root first" contracts. Each item SHALL also carry the author's asserted time as display text, alongside an explicit marker that the value is the author's claim.

The two SHALL NOT be one field and SHALL NOT be derivable from one another. A caller needing an item's place in the thread SHALL have a field that is correct to read for that; a caller rendering a time SHALL have a field that is correct to render and that no comparison would accept. **Neither is the asserted time**, which answers no question about order at all.

**The separation is the defence, and it is a shape rather than a rule anyone has to remember.** The ordering position and the asserted time answer different questions, and the second is forgeable by every author. A single field serving both would be a number that looks like a time and decides a sequence, which is the arrangement that has already failed in the nearest comparable project: an author-asserted timestamp read for ranking, with nothing clamping it, letting a post claiming a future instant pin itself above every honest one permanently. Their validator noticed future timestamps and produced a warning that was never called on the ingest path. The defect was not the missing check; it was that the value was sitting there as a number for whoever wanted to rank by it.

**The position SHALL identify where an item sits in the whole thread's sequence, and it SHALL NOT be a sort key.** The sequence a caller renders is **the sequence the read returned**; the position exists so that an item's place in the whole thread is knowable from the item, and so it does not change when the page size does. An item's position SHALL be the same value whatever page size the read used, and SHALL index the whole thread rather than restarting at each page. Two positions SHALL be unequal for two distinct items of one thread.

**A caller SHALL NOT be required to sort by it, and SHALL NOT be able to rely on doing so.** This is the part that has to be said explicitly, because the value looks sortable and is not: nothing here promises that comparing two positions in a caller's own natural ordering reproduces the sequence, and a caller that sorted on them would be relying on a property this contract does not give. Nor SHALL a caller rely on the token being a number, on arithmetic over two of them meaning anything, on two items' values being adjacent or any fixed distance apart, or on a position being comparable against one from a **different** thread's read. The read SHALL surface it in a form that does not present itself as a quantity.

The alternatives all invite arithmetic that means nothing. The op's own counter would be the obvious value to hand out, and it is the wrong one, for two reasons. First, it is not the returned sequence: the replies come in the reverse of the counters' order, and the root comes first whatever its counter. Second, a counter is its author's claim about the time, raised above whatever that author held, so two counters five apart are not five places in anything, and a caller subtracting them would compute a number with no referent in the thread. What a view needs is to render the returned sequence and to know an item's place in the whole thread across pages. Contracting only that keeps a later change free to alter the token's form without breaking a caller that stayed inside the contract.

The asserted time SHALL be clamped for display and reported as clamped where it is implausible, per the ordering capability's requirement, and this capability SHALL NOT define a clamp of its own.

**The asserted time SHALL reach a caller as a grouped value holding exactly three things: the display text, the author-assertion marker, and the clamped report.** This is a requirement on what a **read hands out**, not on how an implementation stores the value: the marker is the same for every time this system can carry — every one of them is its author's claim — so an implementation may hold two members and supply the third at the boundary, and that is what this one does. What the requirement fixes is that a caller finds three, because a caller is who the marker is for. The three travel together because each is unsafe without the others — text without the marker reads as an observed instant, and text without the clamped report attributes this peer's substitution to the author. A caller SHALL find all three where it finds the time, and SHALL NOT have to correlate them from separate places in the item. **No fourth member SHALL carry the instant in any numeric or otherwise comparable form.** That prohibition is the whole of the defence and it is stated as a property of the shape, not of the field names: a millisecond value added beside the text would restore exactly the sortable number the ordering capability's requirement exists to withhold, and it would do so without altering any sentence about ordering.

An item whose op carries no asserted time — one encoded before the fields existed — SHALL report the time as absent rather than substituting a value. A substituted value is indistinguishable from an asserted one once rendered, and there is nothing to substitute that would be true.

#### Scenario: Every item carries both fields

- **WHEN** a thread's items are read
- **THEN** each carries a field giving its position in the returned sequence
- **AND** each carries the author's asserted time as display text

#### Scenario: The time is not a number a comparison would accept

- **WHEN** an item carrying an asserted time is read
- **THEN** the time is display text
- **AND** no field of the item carries the instant as a bare number

#### Scenario: The asserted time is marked as the author's claim

- **WHEN** an item carrying an asserted time is read
- **THEN** it carries a marker identifying the value as author-asserted

#### Scenario: Rendering the thread in order consults no time

- **WHEN** the items of a thread are rendered in the order the read returned them
- **THEN** no asserted time is consulted to arrive at that order
- **AND** the order is unchanged where the asserted times disagree with it

#### Scenario: A position indexes the whole thread and does not restart per page

- **WHEN** a thread longer than one page is read page by page
- **THEN** the positions continue across the page boundary rather than restarting
- **AND** an item's position is the same value whatever page size the read used

#### Scenario: Two items of one thread never share a position

- **WHEN** a thread's items are read across every page
- **THEN** no two of them carry the same position

#### Scenario: The position is not surfaced as a quantity

- **WHEN** an item is read
- **THEN** the position is carried as text and not as a number

#### Scenario: The time's three members travel together and no fourth carries the instant

- **WHEN** an item carrying an asserted time is read
- **THEN** the display text, the author-assertion marker and the clamped report are found together as one grouped value
- **AND** that value carries nothing else

#### Scenario: An item whose op predates the fields reports the time as absent

- **WHEN** an item is read whose op carries no asserted time
- **THEN** the time is reported as absent
- **AND** no substitute value is reported in its place

#### Scenario: An implausible asserted time is reported as clamped

- **WHEN** an item is read whose op asserts a time far beyond the allowance
- **THEN** the displayed time is clamped
- **AND** the item reports that it was clamped
