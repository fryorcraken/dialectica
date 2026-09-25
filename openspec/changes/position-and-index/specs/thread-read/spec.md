## MODIFIED Requirements

### Requirement: An item carries its ordering position and the author's asserted time, as two separate fields

Each item SHALL carry its position in the sequence the read returns, as a field distinct from every other. That sequence is the whole thread's, with the root first and the replies in the reverse of the ordering rule's sequence, as the requirement "The items are a flat sequence in the system's order, with the root first" contracts. Each item SHALL also carry the author's asserted time as display text, alongside an explicit marker that the value is the author's claim.

The two SHALL NOT be one field and SHALL NOT be derivable from one another. A caller needing an item's place in the thread SHALL have a field that is correct to read for that; a caller rendering a time SHALL have a field that is correct to render and that no comparison would accept. **Neither is the asserted time**, which answers no question about order at all.

**The separation is the defence, and it is a shape rather than a rule anyone has to remember.** The ordering position and the asserted time answer different questions, and the second is forgeable by every author. A single field serving both would be a number that looks like a time and decides a sequence, which is the arrangement that has already failed in the nearest comparable project: an author-asserted timestamp read for ranking, with nothing clamping it, letting a post claiming a future instant pin itself above every honest one permanently. Their validator noticed future timestamps and produced a warning that was never called on the ingest path. The defect was not the missing check; it was that the value was sitting there as a number for whoever wanted to rank by it.

**The position SHALL identify where an item sits in the whole thread's sequence, and it SHALL NOT be a sort key.** The sequence a caller renders is **the sequence the read returned**; the position exists so that an item's place in the whole thread is knowable from the item, and so it does not change when the page size does. An item's position SHALL be the same value whatever page size the read used, and SHALL index the whole thread rather than restarting at each page. Two positions SHALL be unequal for two distinct items returned by one read of a thread, taken across all of that read's pages.

**The position MUST be determined by the place alone, and by nothing about the item that occupies it.** A value taken from the item itself, such as its op id or its author, does not meet this even where it is unique and the same at every page size. Where two reads of one thread return different items at the same place in their sequences, those two items MUST carry the same position. The same item can therefore carry different positions in two reads whose sequences differ, such as one read that includes hidden replies and one that does not. In each read its position is its place in the sequence that read returned.

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

#### Scenario: The item at a place carries that place's position in every read

- **WHEN** a moderator hides a reply that has at least one other reply after it in the thread's sequence, and the thread is read across every page once with hidden content included and once without
- **THEN** at each place both reads fill, the two items at that place carry the same position
- **AND** the reply that follows the hidden reply in the including read carries a different position in each read

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
