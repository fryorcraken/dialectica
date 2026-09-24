## MODIFIED Requirements

### Requirement: The items are a flat sequence in the system's order, with the root first

The items SHALL be a flat sequence. Each SHALL name its parent, which is what lets a caller reconstruct the reply structure; the sequence itself SHALL NOT be nested, and the read SHALL NOT report a depth or an indentation level.

The root post MUST be the first item of the first page. The replies MUST follow in the **exact reverse** of the sequence the system's ordering rule places them in: the reply the rule places last MUST be the first reply, and the reply the rule places first MUST be the last. This capability MUST NOT define an order of its own, MUST NOT sort by arrival, and MUST NOT compare any value itself. A second implementation of the ordering rule could disagree with the first, and two orders that disagree produce no error anywhere — each peer stays internally consistent while rendering one thread differently from its neighbour.

**What that order guarantees is convergence and causal position, and still not wall-clock recency.** The rule leads with the Lamport counter an op carries in its own signed bytes and places the higher counter first, so its reverse places a reply written after its author saw another reply after that reply, and every peer holding both agrees. Two consequences of reversing the rule's sequence, rather than re-sorting the replies, are part of this contract: replies carrying equal counters MUST come in descending op id, the reverse of the rule's tiebreak; and a reply carrying no counter — one encoded before the clock fields existed — MUST come before every reply that carries one, those among themselves in descending op id. An op id is a hash and carries no temporal meaning whatever.

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
- **THEN** their relative order is the exact reverse of the one the ordering rule places them in
- **AND** it is not the sequence they were appended in, where the two differ

  The name is kept while the content changes, because the name is how this delta
  addresses the scenario it amends. "The ordering rule's" now means the rule's
  sequence reversed, and nothing else.

#### Scenario: A reply orders after the reply it answers

- **WHEN** one peer publishes a reply, a second receives it and publishes a reply to it
- **THEN** the second orders after the first in the returned sequence

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
