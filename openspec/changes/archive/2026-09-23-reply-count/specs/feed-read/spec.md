## Purpose

Defines reading a Stoa's feed of thread heads, the list `listThreads` answers with. The requirements below cover the two fields each row reports about its thread's replies: how many there are, and which one is latest. The row's other fields, the pagination, and which threads appear as rows in what order are not contracted here, beyond the requirement that a thread's replies leave the rows and their order unchanged.

Three boundaries are named rather than restated:

- **`thread-read` owns which posts belong to a thread**, through its requirement *Membership is derived from the parent chain, and a post's own thread field is never trusted*, and owns which of them a default read returns. This capability counts and ranks what that rule places, and defines no membership rule of its own.
- **`op-ordering` owns what orders two ops.** This capability uses that order to pick the latest reply and does not compare any value itself.
- **`moderation-resolution` owns whether a reply is hidden.** This capability says what a row's reply fields do with that answer.

Publishing, any ordering of the feed other than the one it already has, and anything about a reply beyond its op id are out of scope.

## ADDED Requirements

### Requirement: A feed row reports how many of its thread's replies are visible

Every feed row MUST report its thread's reply count, as a non-negative integer JSON number under the key `replyCount`.

The count MUST equal the number of items, other than the root, that a thread read of that thread returns with hidden content excluded, across all of its pages. It therefore counts exactly the posts that:

- verify, and belong to the Stoa the feed is read in;
- are placed under this thread by `thread-read`'s parent-chain rule, at any depth; and
- are not hidden by a binding moderation.

A post's own `thread` field MUST NOT decide whether it is counted, or under which thread.

A reply whose parent is hidden, but which is not hidden itself, MUST be counted. The thread's root MUST NOT be counted. Revisions, votes and moderation ops MUST NOT be counted, whatever post they name.

The count MUST be present on every row, and MUST be zero, not omitted, for a thread with no countable replies.

The count MUST be over the ops this peer holds. It is not the number of replies that exist, and MUST NOT be documented or described as the thread's total.

#### Scenario: A thread with no replies reports zero

- **WHEN** the feed is read over a log holding a thread's root and no reply to it
- **THEN** that thread's row reports a reply count of zero
- **AND** the count is present, not omitted

#### Scenario: Replies at every depth are counted

- **WHEN** a thread holds a reply to its root and a reply to that reply, and the feed is read
- **THEN** that thread's row reports a reply count of two

#### Scenario: A hidden reply is not counted

- **WHEN** a thread holds two replies, a moderator of the Stoa hides one of them, and the feed is read
- **THEN** that thread's row reports a reply count of one
- **AND** the same log without the hide reports a count of two, so the fixture can tell a count that ignores moderation from one that applies it

#### Scenario: A reply beneath a hidden reply is still counted

- **WHEN** a moderator hides a reply that has a reply of its own, and the feed is read
- **THEN** the deeper reply is counted
- **AND** the hidden reply is not

#### Scenario: A hide by someone who is not a moderator removes nothing from the count

- **WHEN** an authentically signed hide of a reply is published by a key that is not a moderator of the Stoa, and the feed is read
- **THEN** that reply is counted

#### Scenario: A restored reply is counted again

- **WHEN** a moderator hides a reply and then unhides it, and the feed is read
- **THEN** that reply is counted

#### Scenario: A forged reply is not counted

- **WHEN** a thread holds a reply whose author field names one key and whose signature was made with another, and the feed is read
- **THEN** that reply is not counted
- **AND** the forged op is still in the log, so the count is not an artefact of it never having been stored

#### Scenario: A post claiming a thread it has no parent in is not counted there

- **WHEN** the log holds two threads, and a validly signed reply whose parent is in the first thread carries a `thread` field naming the second
- **THEN** the first thread's row counts that reply
- **AND** the second thread's row does not

#### Scenario: A post whose parent is not held is counted under no thread

- **WHEN** the log holds a reply naming a parent op id no op in the log carries, and whose `thread` field names a thread the log does hold
- **THEN** that thread's row does not count the reply
- **AND** once the missing parent is appended and it belongs to that thread, a second feed read counts the reply

#### Scenario: A post in a parent cycle is counted under no thread

- **WHEN** the log holds a post naming itself as its parent, and posts whose parent references form a cycle, each carrying a `thread` field naming a genuine thread
- **THEN** the feed read returns rows
- **AND** no row counts any of those posts

#### Scenario: Another Stoa's post is not counted

- **WHEN** a validly signed post belonging to a second Stoa names a thread root of the first Stoa as its parent, and the first Stoa's feed is read
- **THEN** that thread's row does not count it

#### Scenario: Ops that are not posts are not counted

- **WHEN** a thread's only reply is revised by its author, voted on, and unhidden by a moderator without ever having been hidden, and the feed is read
- **THEN** that thread's row reports a reply count of one

#### Scenario: The count agrees with the thread read

- **WHEN** a thread holding visible replies, a hidden reply, a reply beneath the hidden one, a forged reply and a post claiming the thread with no parent in it is read both through the feed and through a thread read with hidden content excluded
- **THEN** the row's reply count equals the number of items other than the root that the thread read returns across all of its pages

#### Scenario: Two peers holding different copies report different counts without error

- **WHEN** one peer holds all three replies of a thread and another holds only two of them, and each reads the feed
- **THEN** the first reports a count of three and the second a count of two
- **AND** neither feed read's answer is an error

### Requirement: A feed row names its thread's latest visible reply by the ordering rule

Where a thread has at least one countable reply, its row MUST report the latest one under the key `latestReply`, as that reply's op id. The op id MUST be in the same encoding the row uses for its thread identifier.

The latest reply MUST be the one `op-ordering`'s rule places first among exactly the replies the reply count counts. It MUST therefore be:

- one of the replies the count counts;
- never the root;
- never a hidden reply;
- never a post placed under the thread by its `thread` field alone; and
- never a post that fails verification.

A reply's place in that order MUST be taken from the reply post's own op. It MUST NOT be taken from a revision of the reply, a vote on it or a moderation of it. The reported op id MUST be the reply post's own, not the id of its current version.

The author-asserted time MUST NOT affect which reply is reported.

Where the thread has no countable reply, `latestReply` MUST be omitted from the row. It MUST NOT be sent as `null`, and it MUST NOT be sent as an empty string.

The field's name and its documentation MUST describe a position in the forum's order. They MUST NOT describe it as the reply written most recently in time.

#### Scenario: The reply the ordering rule places first is reported

- **WHEN** a thread holds two replies carrying different Lamport counters, appended with the higher-counter reply first, and the feed is read
- **THEN** the row's latest reply is the op id of the reply carrying the higher counter

#### Scenario: Two peers holding the same ops report the same latest reply

- **WHEN** two peers hold the same thread and replies, appended in opposite sequences, and each reads the feed
- **THEN** both rows report the same latest reply

#### Scenario: A deeper reply can be the latest

- **WHEN** a thread's reply to a reply carries a higher counter than every other reply in the thread, and the feed is read
- **THEN** the row's latest reply is that deeper reply

#### Scenario: A hidden reply is never the latest

- **WHEN** the reply that orders first in a thread is hidden by a moderator, the thread holds another visible reply, and the feed is read
- **THEN** the row's latest reply is the visible reply that orders first among the rest
- **AND** it is not the hidden reply
- **AND** the same log without the hide reports the hidden reply as latest, so the fixture can tell a field that ignores moderation from one that applies it

#### Scenario: A thread whose only reply is hidden reports no latest reply

- **WHEN** a thread's only reply is hidden by a moderator, and the feed is read
- **THEN** the row reports a reply count of zero
- **AND** `latestReply` is absent from the row

#### Scenario: A thread with no replies reports no latest reply

- **WHEN** a thread holds no reply, and the feed is read
- **THEN** `latestReply` is absent from the row
- **AND** it is not present holding `null` or an empty string

#### Scenario: Revising an earlier reply does not make it the latest

- **WHEN** a thread holds an earlier reply and a later reply by the ordering rule, the earlier reply's author then revises it, and the feed is read
- **THEN** the row's latest reply is still the later reply

#### Scenario: The reported id is the reply's own and not its revision's

- **WHEN** the latest reply in a thread has been revised by its author, and the feed is read
- **THEN** the row's latest reply is the reply post's op id
- **AND** it is not the revision's op id

#### Scenario: An asserted time does not decide the latest reply

- **WHEN** a thread's reply with the lower counter asserts a time far in the future, and the reply with the higher counter asserts an earlier time, and the feed is read
- **THEN** the row's latest reply is the reply with the higher counter

#### Scenario: A post injected by its thread field cannot become the latest reply

- **WHEN** a validly signed post whose parent chain does not reach a thread's root carries a `thread` field naming that thread and a counter higher than every reply in it, and the feed is read
- **THEN** that thread's row does not report the post as its latest reply
- **AND** it reports the thread's genuine reply that orders first

#### Scenario: A forged reply cannot become the latest reply

- **WHEN** a thread holds a reply whose signature does not verify and whose counter is higher than every genuine reply's, and the feed is read
- **THEN** the row's latest reply is the genuine reply that orders first

#### Scenario: The latest reply is among the thread read's items

- **WHEN** a thread with at least one visible reply is read through the feed and through a thread read with hidden content excluded
- **THEN** the row's latest reply is the op id of an item the thread read returns
- **AND** it is not the root's op id

### Requirement: A row's reply fields do not vary with the include-hidden flag

A row's reply count and latest reply MUST be the same whether or not the feed is read with hidden content included. The flag decides which rows the feed returns. It MUST NOT add hidden replies to a row's count, and it MUST NOT make a hidden reply eligible to be the latest.

A row for a thread whose root is hidden, returned when hidden content is included, MUST report the count and latest reply of that thread's replies by the same rules as any other row. Hiding a root does not hide its replies.

#### Scenario: The flag does not change a row's reply fields

- **WHEN** a thread whose root is not hidden holds a visible reply and a hidden reply, and the feed is read once with hidden content excluded and once with it included
- **THEN** the thread's row reports the same reply count in both reads
- **AND** the same latest reply in both reads
- **AND** in neither read does the count include the hidden reply

#### Scenario: A hidden thread's row counts its visible replies

- **WHEN** a moderator hides a thread's root, the thread holds two replies that are not hidden, and the feed is read with hidden content included
- **THEN** the thread's row is returned marked as hidden
- **AND** it reports a reply count of two
- **AND** it reports the one of those two replies that the ordering rule places first as its latest reply

### Requirement: A thread's replies do not change which rows the feed returns, or where

A thread's replies MUST NOT affect which threads appear as feed rows, the order the rows come in, or how they are cut into pages. A reply MUST NOT appear as a row. A thread's replies, their count and their counters MUST NOT move that thread's row.

#### Scenario: A new reply does not move its thread's row

- **WHEN** the feed holds two threads, a reply carrying a counter higher than either root is added to the thread whose row comes second, and the feed is read again
- **THEN** the rows come back in the same order as before the reply was added
- **AND** the reply does not appear as a row

### Requirement: Computing the reply fields fails as an error and never aborts

A store failure met while computing the reply fields MUST make the feed read fail with the wire contract's error shape, carrying the reason the store gave. It MUST NOT be reported as a row with a count of zero, a row without a latest reply, or a row missing from the page.

The read MUST fail in that way whichever reply in the Stoa the failure is met on. That includes a reply whose thread's row is on the requested page, a reply whose thread's row is on another page, and a reply whose thread's row the read does not return because its root is hidden and hidden content is excluded. A failure MUST NOT be skipped on the ground that the reply it was met on belongs to no row of the page returned.

Computing the reply fields MUST terminate, and MUST NOT abort the process, for any contents the log may hold. That includes forged ops, posts naming parents that are not held, a post naming itself as its parent, parent references forming a cycle of any length, and ops of every kind naming a reply.

#### Scenario: A store failure while counting is an error and not a zero

- **WHEN** the feed is read over a Stoa holding a thread with replies, against a store that answers every read except those keyed by one of that thread's replies' op ids, which fail
- **THEN** the feed read's answer is an error carrying the reason the store gave
- **AND** it carries no items
- **AND** the same store answering those reads too returns the thread's row, so the failure is the one met while computing the reply fields

#### Scenario: A store failure on a reply of a thread on another page fails the read

- **WHEN** the feed holds two threads and is read one row per page, and the store fails every read keyed by the op id of a reply to the thread whose row comes second, and the first page is read
- **THEN** the feed read's answer is an error carrying the reason the store gave
- **AND** it carries no items
- **AND** the same store answering those reads too returns the first page holding the first thread's row, so the failure is the one met on the other page's reply

#### Scenario: A store failure on a reply of a thread whose row is not returned fails the read

- **WHEN** a moderator hides one thread's root, a second thread is not hidden, the store fails every read keyed by the op id of a reply to the hidden thread, and the feed is read with hidden content excluded
- **THEN** the feed read's answer is an error carrying the reason the store gave
- **AND** it carries no items
- **AND** the same store answering those reads too returns the second thread's row and no row for the hidden thread, so the failure is the one met on the reply of a row the read does not return

#### Scenario: An adversarial log is read without aborting

- **WHEN** the feed is read against a log holding forged posts, posts naming parents that are not held, a post naming itself as its parent, a parent cycle, and ops of every other kind naming a reply
- **THEN** the read returns an answer rather than aborting or failing to terminate
- **AND** every row reports a reply count
