## Why

A feed row's fields are governed by the code and by nothing else. When #80 re-pointed
the row's `author` from an author address's hex to the signing key's hex, it changed
the meaning of a wire field that no requirement governed. A caller comparing a stored
`author` across that change gets a silent mismatch, since both values are 64 hex
characters, and nothing in `openspec/specs/` says which value is correct. Two `// NO
SPEC:` markers record the gap, and issue #91 is where it was deferred to.

The issue says no `feed` capability exists. **That is stale:** `feed-read` has
existed since the reply-count change (#100), but it covers only `replyCount` and
`latestReply`. Its Purpose says outright that the row's other fields, the pagination,
and which threads appear as rows in what order are not contracted. This change
contracts them, in `feed-read` rather than in a second feed capability.

It writes down current behaviour. It does not redesign the row.

## What Changes

- **`feed-read` contracts the whole feed read**, beyond the two reply fields it has
  now:
  - **The request:** the Stoa and its genesis record are required, and a record that
    does not describe the Stoa is refused. `page`, `perPage` and `includeHidden` are
    optional, each with a restrictive default, and each refused by name when it has
    the wrong type. No ordering parameter is taken, and a field naming one changes
    nothing.
  - **Which rows are returned, and in what order:** the authentic posts in the Stoa
    that name no parent, placed in `op-ordering`'s sequence of their root posts.
    Hidden roots are excluded unless asked for, and they are excluded before the page
    is cut.
  - **A closed field set for a row:** exactly `thread`, `currentVersion`, `author`,
    `body`, `attachments`, `isRevised`, `isHidden` and `replyCount`, plus
    `latestReply` where the thread has a countable reply, and nothing else. The
    fields a row does *not* carry are stated positively: no display name, mark,
    address, score, time, position or total.
  - **What each field means:** `author` is the signing public key's hex, and nothing
    derived from that key travels beside it. `currentVersion` and `isRevised` come
    from `post-revision`. `body` and `attachments` are the current version's, run
    through the sanitiser, each as a closed `{text, removed, marked}` object.
    `isHidden` is the root's moderation, which can be true only in a read that asked
    for hidden content.
  - **The pagination shape:** a closed `{items, page, hasMore}` envelope with no
    total. It also covers the defaults, the cap, a page size of zero, a page past the
    end, and pages tiling the feed.
  - **Failure:** a refused request or a store failure is the error shape with no
    items, and never an empty feed.
- **`feed-read`'s Purpose is corrected in the live spec directly.** A delta does not
  carry a Purpose. The live text says the fields above are "not contracted here", so
  it would read false once this change archived. The new wording lists the
  boundaries and makes no claim about coverage, so it reads true both before and after
  archive, and archive has nothing to hand-edit. This follows the precedent that
  `thread-reply-order` set.
- **`thread-read` gains one requirement naming the JSON key for an item's author key:
  `author`.** It exists because of the second marker the issue names, which is
  explained below.

### The two markers the issue names, and where each is resolved

The issue cites `feed.rs:657` and `wire.rs:1819`. Both line numbers are from the
tree at #88. This section names each site by the function that holds it rather than
by a line number, because line numbers drift as the change's own edits land.

- **`feed.rs:657`** is the marker in the `feed.rs` test
  `a_row_carries_the_public_key_and_no_derived_display_name`. The requirement *A row
  names its author by the signing public key, and by nothing derived from it*
  resolves it.
- **`wire.rs:1819` was not a feed marker.** At #88 that line was inside
  `thread_page_json`: it was the marker on the **thread item's** `author` spelling,
  and it pointed at #88's `design.md` §4 rather than §5. The issue calls it "the test
  that pins the shape" of the feed row, and that description does not hold. The choice
  it marks is present twice in `wire.rs`: in `thread_page_json`, on the item's
  `author` key, and in the test `the_wire_reports_the_author_as_one_key_and_no_name`. The new
  `thread-read` requirement *An item's author key travels under the JSON key
  `author`* resolves both.

The feed's own JSON-shape tests in `wire.rs` (`the_feed_reply_is_the_ecosystems_pagination_shape`,
`a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`) carry no marker.

## Capabilities

### New Capabilities

None. `feed-read` is the feed's capability already. A second one would split a single
read across two specs.

### Modified Capabilities

- `feed-read`: ADDED requirements for the request, the rows and their order, the
  closed row field set, the meaning of `thread` / `currentVersion` / `isRevised`,
  `author`, `body` / `attachments`, `isHidden`, the pagination shape, and failure.
  The existing requirements, including the `replyCount` and `latestReply` ones, are
  unchanged. The Purpose line is corrected in the live spec.
- `thread-read`: ADDED *An item's author key travels under the JSON key `author`*.
  The existing *An author is reported as a public key, and never as a name* is left
  unchanged: it already requires the value, and this adds only the key name.

## Impact

- **No behaviour change is intended.** Every requirement describes what
  `feed.rs::list_threads`, `wire.rs::list_threads_inner` /
  `list_threads_from_request` / `feed_page_json` and `thread_page_json` do on this
  tree. If a test written to this spec fails, either the code or the spec is wrong,
  and the finding goes back to the `spec-writer`.
- **Tests:** the closed-set scenarios need the row's key set, the envelope's key set
  and the sanitised object's key set asserted exactly, in each row state: no reply,
  with a reply, and revised, hidden and with attachments at once.
  `wire.rs`'s two existing key-set tests cover the first two states. Request
  refusals on `genesis` and `stoa` need `list_threads_from_request`, because the
  in-crate `list_threads` takes a decoded record.
- **Stale prose in code the `dev-writer` should correct in the same change:**
  - `FeedRow::author`'s doc comment (`feed.rs`, "No capability owns this reply's
    shape…").
  - The `NO SPEC:` marker in `a_row_carries_the_public_key_and_no_derived_display_name`.
  - That test's reference to a test named
    `the_feed_json_is_pinned_to_the_exact_shape_a_view_is_written_against`, which
    does not exist in `wire.rs`.
  - The two thread-read markers, in `thread_page_json` and in
    `the_wire_reports_the_author_as_one_key_and_no_name`.
  - The `thread-read` markers on `position`, `assertedTime` and the moderation
    spellings are **not** resolved here and stay.
- **Coordination:**
  - #101 (0.0.2) changes the moderation flags' shape later. This change specifies
    `isHidden` as it is today, so #101 will modify these requirements.
  - `feed-view`'s spec expects a feed time to arrive. This change's closed set
    forbids one, so adding it later is a change to the row requirement, which is the
    purpose of closing the set.

## Open questions for the owner

### A post naming no parent is a row whatever its `thread` field says

A root post published by this peer carries no `thread` field. A peer can still sign a
post that names no parent but does carry a `thread` field naming some other thread. The
feed serves that post as a row of its own, because a row is decided by the absent
parent alone. The spec writes this down as current behaviour, consistent with
`thread-read`'s rule that a post's own `thread` field decides nothing. Should such a
post be refused as malformed instead?

### `isHidden` is two-valued where the thread read is three-valued

A thread item reports `unmoderated` / `hidden` / `unhidden` and names the deciding op.
A feed row reports a boolean, so a root a moderator restored reads the same as one no
moderator touched. The spec records the boolean as it is. Whether the row should carry
the thread read's shape is #101's question, and this change does not pre-empt it.

### The pagination constants are not pinned in the spec

The default page size and the cap are contracted as existing, with the default no
larger than the cap and zero served as the default. Their values (`DEFAULT_PER_PAGE`,
`MAX_PER_PAGE` in `feed.rs`) are not written into the spec, which matches
`thread-read`. If a caller must be able to rely on a number, it has to be named in the
spec.

### Resolving the thread-read marker here, rather than in its own piece

The issue's second marker concerns `thread-read`, not the feed. This change resolves
it because the issue names it, and because the row and the item then share one
documented spelling. If you would rather keep this piece strictly to `feed-read`, the
`thread-read` delta can be dropped without affecting anything else in the change. The
two `wire.rs` markers would then stay, and a new issue would carry them.
