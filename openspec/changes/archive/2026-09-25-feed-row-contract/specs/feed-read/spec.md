## ADDED Requirements

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
