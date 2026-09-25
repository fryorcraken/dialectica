# feed-read Specification

## Purpose
Defines reading a Stoa's feed of thread heads, the list `listThreads` answers with.

Five boundaries are named rather than restated:

- **`thread-read` owns which posts belong to a thread**, through its requirement *Membership is derived from the parent chain, and a post's own thread field is never trusted*, and owns which of them a default read returns. This capability counts and ranks what that rule places, and defines no membership rule of its own.
- **`op-ordering` owns what orders two ops.** This capability uses that order and does not compare any value itself.
- **`moderation-resolution` owns whether a post is hidden.** This capability says what the feed does with that answer.
- **`post-revision` owns which version of a post is current.** This capability reports that answer and does not restate how it is reached.
- **`generated-names` owns that a display name never travels.** This capability says what a row carries instead.

Publishing, any ordering of the feed other than the one it already has, and anything about a reply beyond its op id are out of scope.

## Requirements

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

### Requirement: A feed request names its Stoa and carries that Stoa's genesis record

A feed read MUST take a JSON object carrying the Stoa's address under the key `stoa` and that Stoa's genesis record, hex-encoded, under the key `genesis`. Both are required.

The read MUST be refused with the wire contract's error shape, carrying no items, when:

- `stoa` is absent, is not a string, or is not a valid Stoa address;
- `genesis` is absent, is not a string, is not hex, or does not decode as a genesis record;
- the genesis record's address is not the address named by `stoa`.

A refusal for an absent field MUST be distinguishable by its message from a refusal for the same field holding the wrong type. An explicit `null` in either field MUST be refused as the wrong type, not reported as missing.

The moderator set applied to the read MUST be the one the supplied genesis record names.

The feed read MUST take no parameter selecting an ordering. A request carrying a field such as `order` MUST be answered exactly as the same request without it.

#### Scenario: A request with no genesis record is refused

- **WHEN** the feed is read with a valid `stoa` and no `genesis`
- **THEN** the reply is the error shape
- **AND** it carries no items

#### Scenario: A genesis record describing another Stoa is refused

- **WHEN** the feed is read with a `stoa` naming one Stoa and a `genesis` record whose address is a different Stoa's
- **THEN** the reply is the error shape
- **AND** it carries no items

#### Scenario: A missing Stoa is refused distinguishably from a wrong-typed one

- **WHEN** the feed is read once with `stoa` omitted and once with `stoa` holding a number
- **THEN** both replies are the error shape
- **AND** their messages differ, the second saying the field has the wrong type

#### Scenario: A malformed genesis record is refused

- **WHEN** the feed is read with a `genesis` that is not hex, and again with one that is hex but does not decode as a genesis record
- **THEN** both replies are the error shape and carry no items

#### Scenario: An ordering argument changes nothing

- **WHEN** the feed is read once plainly and once with `"order":"top"` added to the same request
- **THEN** the two replies are identical

### Requirement: The rows are the Stoa's authentic thread heads, in the ordering rule's sequence

A feed row MUST be returned for each post that:

- belongs to the Stoa the feed is read in;
- verifies; and
- names no parent.

A post that names a parent is a reply, and it MUST NOT be a row. Whether a post is a row MUST be decided by whether it names a parent. A post that names no parent MUST be a row even when it carries a `thread` field. Revisions, votes, moderation ops and metadata ops MUST NOT be rows.

The rows MUST come in the sequence `op-ordering`'s rule places their root posts in, with the first placed first. A root's row MUST keep its place when the root is revised, voted on or moderated. This capability defines no order of its own.

A row whose root is hidden by a binding moderation MUST be omitted unless the request asks for hidden content, in which case it MUST be returned in its place in the sequence. Hidden rows MUST be omitted **before** the rows are cut into pages.

The request asks for hidden content with the boolean `includeHidden`. Where the field is absent or holds `null`, hidden content MUST NOT be included. Where it holds any value that is neither a boolean nor `null`, the read MUST be refused with the error shape.

#### Scenario: Rows follow the ordering rule

- **WHEN** a Stoa holds two thread roots carrying different Lamport counters, appended with the lower-counter root first, and the feed is read
- **THEN** the root carrying the higher counter is the first row
- **AND** two peers holding the same roots appended in opposite sequences return the rows in the same sequence

#### Scenario: Revising a root does not move its row

- **WHEN** a Stoa holds two threads, the root of the second row is revised by its author, and the feed is read again
- **THEN** the rows come back in the same sequence as before the revision

#### Scenario: A forged root is not a row

- **WHEN** the log holds a post naming no parent whose author field names one key and whose signature was made with another, and the feed is read
- **THEN** no row is returned for it
- **AND** the forged op is still in the log

#### Scenario: Another Stoa's root is not a row

- **WHEN** the log holds a validly signed post naming no parent that belongs to a second Stoa, and the first Stoa's feed is read
- **THEN** no row is returned for it

#### Scenario: A parentless post carrying a thread field is a row

- **WHEN** the log holds a validly signed post that names no parent and carries a `thread` field naming another thread's root, and the feed is read
- **THEN** that post is returned as a row of its own
- **AND** the row of the thread its `thread` field names does not count it as a reply

#### Scenario: A hidden root is omitted by default and returned when asked for

- **WHEN** a moderator of the Stoa hides one of two thread roots, and the feed is read once without `includeHidden` and once with `"includeHidden":true`
- **THEN** the first read returns only the other thread's row
- **AND** the second returns both rows, in the ordering rule's sequence

#### Scenario: A null include-hidden flag includes nothing hidden

- **WHEN** a feed holding a hidden root is read with `"includeHidden":null`
- **THEN** the reply equals the reply to the same request without the field

#### Scenario: A wrong-typed include-hidden flag is refused

- **WHEN** the feed is read with `"includeHidden":"yes"`
- **THEN** the reply is the error shape and carries no items

#### Scenario: Hidden rows are omitted before the page is cut

- **WHEN** the rows that the ordering rule places first are hidden, and the feed is read at a page size smaller than the number of visible rows
- **THEN** the first page is full of visible rows

### Requirement: A feed row carries exactly a closed set of fields

A row whose thread has no reply counted under *A feed row reports how many of its thread's replies are visible* MUST carry exactly these eight keys and no others:

- `thread`
- `currentVersion`
- `author`
- `body`
- `attachments`
- `isRevised`
- `isHidden`
- `replyCount`

A row whose thread has at least one such reply MUST carry exactly those eight keys and `latestReply`, and no others.

A row MUST NOT carry any other key in any state: revised or not, hidden or not, with attachments or without, in a read that includes hidden content or one that does not. In particular, a row MUST NOT carry:

- a display name, a mark or any other value derived from the author's key;
- an author address;
- a vote score, tally or count of votes;
- a time of any kind, asserted or otherwise;
- an ordering position;
- the deciding moderation op;
- a count of the Stoa's threads.

#### Scenario: A row with no reply carries exactly eight keys

- **WHEN** the feed is read over a Stoa holding one thread with no reply, and the row's keys are enumerated
- **THEN** they are exactly `attachments`, `author`, `body`, `currentVersion`, `isHidden`, `isRevised`, `replyCount` and `thread`

#### Scenario: A row with a reply carries exactly nine keys

- **WHEN** the feed is read over a Stoa holding one thread with one visible reply, and the row's keys are enumerated
- **THEN** they are exactly the eight keys above and `latestReply`

#### Scenario: A row in every other state carries no further key

- **WHEN** a thread's root carries an attachment and has been revised by its author and hidden by a moderator, the thread holds a visible reply that has been voted on, and the feed is read with `"includeHidden":true`
- **THEN** that row's keys are exactly the eight keys above and `latestReply`

### Requirement: A row identifies its thread and the version it renders

`thread` MUST be the root post's op id, as 64 lowercase hexadecimal characters. It MUST NOT change when the root is revised. It MUST be a value the thread read accepts as the thread to read.

`currentVersion` MUST be the op id, in the same encoding, of the root's current version as `post-revision` resolves it.

`isRevised` MUST be a JSON boolean. It MUST be `true` exactly when that current version is a revision rather than the root post itself. So `isRevised` MUST be `false` exactly when `currentVersion` equals `thread`.

#### Scenario: An unrevised root reports itself as its current version

- **WHEN** a thread whose root has never been revised is read through the feed
- **THEN** the row's `currentVersion` equals its `thread`
- **AND** `isRevised` is `false`

#### Scenario: A revised root reports the revision and keeps its thread id

- **WHEN** a thread's root is revised by its author and the feed is read
- **THEN** the row's `thread` is still the root post's op id
- **AND** `currentVersion` is the revision's op id
- **AND** `isRevised` is `true`

#### Scenario: A revision by someone else changes nothing

- **WHEN** a thread's root has a revision signed by a key other than the root's author, and the feed is read
- **THEN** the row's `currentVersion` equals its `thread`
- **AND** `isRevised` is `false`

#### Scenario: The thread id opens the thread

- **WHEN** a row's `thread` is passed as the thread to a thread read of the same Stoa
- **THEN** that read returns the thread, with the root post as its first item

### Requirement: A row names its author by the signing public key, and by nothing derived from it

`author` MUST be the public key that signed the root post, as 64 lowercase hexadecimal characters. It MUST be taken from the verified op and from nothing else. It MUST NOT change when the root is revised.

`author` MUST NOT be an author address or any other hash of the key. A value derived from the key MUST NOT travel on the row beside it, as the requirement on the row's closed field set states.

For the same post, a row's `author` MUST equal the `author` of that post's item in a thread read.

#### Scenario: The author is the signing key's hex

- **WHEN** a thread's root is signed by a key, and the feed is read
- **THEN** the row's `author` equals that public key's lowercase hex

#### Scenario: Two authors are two keys

- **WHEN** a Stoa holds two threads whose roots are signed by different keys, and the feed is read
- **THEN** the two rows' `author` values differ
- **AND** each equals the public key that signed that row's root

#### Scenario: Keys that derive one display name stay two rows

- **WHEN** two threads' roots are signed by two different keys that derive the same display name, and the feed is read
- **THEN** two rows are returned
- **AND** their `author` values are the two different keys

#### Scenario: The feed and the thread read name an author alike

- **WHEN** a thread is read through the feed and through a thread read
- **THEN** the row's `author` equals the root item's `author`

### Requirement: A row's body and attachments are its current version's, sanitised, with what sanitising found

`body` MUST be the current version's body. `attachments` MUST be a JSON array holding one entry for each of the current version's attachment references, in their order. `attachments` MUST be an empty array, not absent, for a version with none.

`body` and each attachment entry MUST be a JSON object carrying exactly three keys and no others:

- `text`: the sanitised string;
- `removed`: a non-negative integer counting the characters sanitising removed;
- `marked`: a non-negative integer counting the characters sanitising marked and left in place.

Each MUST be an object even when sanitising found nothing. The sanitising MUST be the one `thread-read`'s requirement *Every string a thread read returns is sanitised, and the stored op is not* applies to a thread item, so the same post reads the same in both. The stored op MUST keep its author's bytes.

#### Scenario: A revised root shows the revision's content

- **WHEN** a thread's root is revised by its author to a new body and a new attachment reference, and the feed is read
- **THEN** the row's `body.text` is the revision's body
- **AND** `attachments` holds the revision's reference and not the original's

#### Scenario: A hostile body is sanitised and reported

- **WHEN** a thread's root body carries a bidirectional override and a character that renders as another, and the feed is read
- **THEN** `body.text` no longer carries the override
- **AND** `body.removed` is 1 and `body.marked` is 1

#### Scenario: An ordinary body is still an object

- **WHEN** a thread's root body needs no sanitising, and the feed is read
- **THEN** `body` is an object whose keys are exactly `marked`, `removed` and `text`
- **AND** `removed` and `marked` are both 0

#### Scenario: A root with no attachments carries an empty array

- **WHEN** a thread's root carries no attachment reference, and the feed is read
- **THEN** `attachments` is present and is an empty array

#### Scenario: The feed and the thread read sanitise alike

- **WHEN** a thread whose root carries a hostile body and a hostile attachment reference is read through the feed and through a thread read
- **THEN** the row's `body` equals the root item's `body`
- **AND** the row's `attachments` equals the root item's `attachments`

### Requirement: A row reports whether its thread's root is hidden

`isHidden` MUST be a JSON boolean. It MUST be `true` exactly when `moderation-resolution` resolves the root post as hidden by a binding moderation, and `false` otherwise. That includes a root no moderation names, a root hidden and then restored, and a root named only by a moderation that does not bind.

`isHidden` MUST describe the root post only. The hiding of a reply MUST NOT set it.

A row with `isHidden` `true` is returned only when the request asks for hidden content, so every row of a read that does not ask for it MUST carry `isHidden` `false`. A hidden root's row MUST carry its `body` and `attachments` like any other row.

#### Scenario: A hidden root is marked when asked for

- **WHEN** a moderator of the Stoa hides a thread's root, and the feed is read with `"includeHidden":true`
- **THEN** that thread's row carries `isHidden` `true`
- **AND** it carries the root's `body`

#### Scenario: A restored root is not marked

- **WHEN** a moderator hides a thread's root and then unhides it, and the feed is read without `includeHidden`
- **THEN** the row is returned
- **AND** it carries `isHidden` `false`

#### Scenario: A hide by someone who is not a moderator marks nothing

- **WHEN** an authentically signed hide of a thread's root is published by a key that is not a moderator of the Stoa, and the feed is read
- **THEN** the row is returned carrying `isHidden` `false`

#### Scenario: A hidden reply does not mark its thread

- **WHEN** a moderator hides a reply in a thread whose root is not hidden, and the feed is read with `"includeHidden":true`
- **THEN** that thread's row carries `isHidden` `false`

### Requirement: The feed is paged by index and size, and answers in a closed pagination shape

A feed read MUST accept an optional page index under the key `page` and an optional page size under the key `perPage`. Each MUST be a non-negative integer.

- Where `page` is absent or `null`, the first page, index 0, MUST be served.
- Where `perPage` is absent, `null` or zero, a default page size MUST be served. The default MUST NOT exceed the cap.
- A `perPage` above the cap MUST be served at the cap, not refused.
- A `page` or `perPage` that is negative, fractional, written with a decimal point or an exponent, not a number, or an integer larger than the largest this peer accepts MUST be refused with the error shape. The message MUST name the field.

A successful reply MUST be a JSON object carrying exactly three keys and no others:

- `items`: the page's rows, in the feed's sequence;
- `page`: the index served;
- `hasMore`: a boolean that MUST be `true` exactly when at least one row follows this page's last row in this peer's copy.

The reply MUST NOT carry a count of the feed's rows, or of anything else in the Stoa.

Successive pages MUST partition the rows. Concatenating every page in index order MUST reproduce exactly the sequence a single read with a page size covering every row would return, with no row missing and none repeated.

A page index past the last page MUST be served as a page with no items, reporting the index asked for and `hasMore` `false`. It MUST NOT be refused. That includes the largest index this peer accepts.

#### Scenario: The reply carries exactly items, page and hasMore

- **WHEN** the feed is read successfully and the reply's keys are enumerated
- **THEN** they are exactly `hasMore`, `items` and `page`

#### Scenario: Absent pagination fields default

- **WHEN** the feed is read with neither `page` nor `perPage`
- **THEN** the reply reports `page` 0
- **AND** it carries no error

#### Scenario: Pages partition the feed exactly

- **WHEN** a feed of several rows is read one page at a time at a page size smaller than the feed
- **THEN** concatenating the pages in index order reproduces the single-read sequence
- **AND** no row appears twice and none is missing

#### Scenario: A further page is reported only while one exists

- **WHEN** a feed whose row count is an exact multiple of the page size is read page by page
- **THEN** every page but the last reports `hasMore` `true`
- **AND** the last full page reports `hasMore` `false`

#### Scenario: A page past the end is empty rather than an error

- **WHEN** the feed is read at a page index beyond its last page
- **THEN** the reply carries no error
- **AND** it carries no items, reports the index asked for and reports `hasMore` `false`

#### Scenario: The largest accepted page index is an empty page

- **WHEN** the feed is read at the largest page index this peer accepts
- **THEN** the reply is a page with no items, reporting that index, rather than an error
- **AND** a page index one larger is refused with a message naming `page`

#### Scenario: An oversized page size is served at the cap

- **WHEN** the feed of a Stoa holding more rows than the cap is read with a `perPage` far above the cap
- **THEN** the reply carries no error
- **AND** it carries exactly the cap's number of rows

#### Scenario: A page size of zero is served at the default

- **WHEN** the feed is read once with `"perPage":0` and once with no `perPage`
- **THEN** the two replies are identical

#### Scenario: Malformed pagination fields are refused by name

- **WHEN** the feed is read with a `page` of -1, of 1.5, of 1e2 and of `"many"`, and with a `perPage` of each of the same values
- **THEN** every reply is the error shape and carries no items
- **AND** each message names the field it refused

#### Scenario: A null page reads as the first page

- **WHEN** the feed is read with `"page":null`
- **THEN** the reply equals the reply to the same request without `page`

### Requirement: A feed read that fails is the error shape, and never an empty feed

Every refusal and every failure of a feed read MUST be the wire contract's error shape. The reply MUST NOT carry both an error and `items`.

A failure to open this peer's store, and a store failure met while reading the Stoa's ops, resolving a root's current version or resolving a root's moderation, MUST make the read fail with the error shape, carrying the reason the store gave. Such a failure MUST NOT be reported as an empty page, and MUST NOT be reported as a page missing the rows the failure was met on.

A feed read MUST terminate, and MUST NOT abort the process, for any request and for any contents the log may hold.

#### Scenario: A store that cannot be opened is an error and not an empty feed

- **WHEN** the feed is read and this peer's store cannot be opened
- **THEN** the reply is the error shape carrying the store's reason
- **AND** it carries no items

#### Scenario: A store failure while reading is an error and not an empty feed

- **WHEN** the feed is read against a store that fails when asked for the Stoa's ops
- **THEN** the reply is the error shape carrying the store's reason
- **AND** it carries no items

#### Scenario: A refusal carries no items

- **WHEN** a feed read is refused for any reason this capability names
- **THEN** the reply carries an error and no `items` key
