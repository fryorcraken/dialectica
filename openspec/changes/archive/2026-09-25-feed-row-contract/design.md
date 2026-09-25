## Context

The feed read is already built: `feed.rs::list_threads` selects and resolves the
rows, and `wire.rs` (`list_threads_from_request` → `list_threads_inner` →
`feed_page_json`) parses the request and writes the JSON. The `feed-read` delta
contracts what that code does today (see proposal.md, *Why*). So this change is
tests and prose. No production behaviour moves, and every requirement was
checked against the code before a test was written for it.

## Goals / Non-Goals

**Goals:**

- Every closed set the spec names has a test that asserts the whole set, with
  the expected keys written out by hand: the row without `latestReply`, the row
  with it, the row in every other state at once, the page envelope, and the
  `{text, removed, marked}` object for `body` and for each attachment.
- The two behaviours the spec states and no test pinned: a parentless post
  carrying a `thread` field is a row, and the page index one past the largest
  accepted is refused with a message naming `page`.
- `thread-read`'s new requirement is pinned: no item key but `author` holds the
  signer's hex.
- The code prose that says "no capability owns this shape" is corrected, and
  the `NO SPEC:` markers this change resolves are removed.

**Non-Goals:**

- Changing any output. A test written to this spec that fails is a finding
  against the code or the spec, and goes back to the spec-writer. It is not
  fixed here by choosing a side.
- The moderation flag's shape (#101), a feed time (`feed-view`), and accepting
  an integral float such as `1e2` as a page index. Each is a later change to a
  requirement this change writes down.
- The `thread-read` markers on `position`, `assertedTime` and the moderation
  spellings. They are not resolved by anything in these deltas, so they stay.

## Decisions

### 1. Write down the row as it is, because its meaning already changed once with nothing to cite

**Why this is being done now.** Issue #80 (PR #88) re-pointed the feed row's
`author` from an author address's hex to the author public key's hex. The edit
was correct and necessary. Leaving the row naming an author by a derivation that
no longer existed would have been worse. But it **changed the meaning of a wire
field that no requirement governed**, so a later change could re-point it again
with the same authority. #80 recorded this rather than doing it quietly: two
`// NO SPEC:` markers, and a deferral to #91. The deferral was right for that
piece. Writing a feed contract means specifying every field, the pagination
shape and the moderation flags, and bundling that into a deletion would have
left neither half reviewable.

**The concrete risk.** A caller that stored a row's `author` before #80 and
compares it with one read after gets **no error**, only a silent mismatch,
because both values are 64 hex characters. Until this change, nothing in
`openspec/specs/` said which value was correct.

**Alternative considered: redesign the row while specifying it.** For example,
make `isHidden` three-valued like the thread item, or add a time. Rejected.
Each is a behaviour change that needs its own review, and #101 already owns the
moderation shape. A contract that first describes what exists gives those
changes a requirement to modify, which is the thing that was missing.

### 2. `author` is the signing key's hex, and nothing derived from it travels beside it

The row's `author` is `entry.op.op.author.to_hex()`: the public key taken from
the verified op. It is not an address, and there is no name or mark beside it.

The argument is the one `generated-names` makes for names, applied to every
derived value. A value derived from the key, sent beside the key, is two values
that must agree and could disagree. A recipient has no way to tell which is
wrong, and a relay could strip or forge the derived one. Whoever renders the row
holds the key, so it derives the name and the mark itself. #80's `design.md` §5
(archived at `openspec/changes/archive/2026-09-16-key-sweep/design.md`) records
the edit that made this true, and the side effect it had: the row gained the
derivation's input, so a view can render both channels.

**Alternative considered: carry the address as well, for callers that stored
one.** Rejected, and #80 already rejected it. An address is a hash from which no
key can be recovered, so it is an input to nothing a reader is shown. Carrying
it would bring back the two-values-that-must-agree problem to help a caller
whose stored value is already wrong.

### 3. The closed set is asserted as an exact key set, and the absences are stated positively

The issue's own reasoning for the requirement's shape was: *what a row does not
carry, stated positively, since that is the half a closed-field-set test can
actually fail on.* A test that checks each expected key is present passes when a
key is added. Only a test that compares the whole set fails on an addition, and
an addition is how a derived name, a score or a time would arrive. The spec-test
reviewer measured this gap once on the slate reply: a `displayName` was added to
every candidate and the suite stayed green.

So every closed-set test here compares the object's sorted keys with a list
**written out at the call site**, never one built from the implementation's
output or its types. A list derived from `FeedRow` would change along with the
code and fail on nothing.

The row-state test builds a root that is revised, hidden and carries an
attachment, with a visible reply that has been voted on, and reads it with
`includeHidden:true`. The test first asserts that each of those states was
reached (`isHidden`, `isRevised`, `currentVersion` equal to the revision,
`replyCount` 1, the revision's attachment). Without those checks, a fixture
that failed to reach a state would test an ordinary row and pass for the wrong
reason.

**This is a guard, and what breaks without it is measured.** Each mutation below
was applied on its own, run against all of `dialectica-core`'s unit tests, and
reverted. The right-hand column is the complete red set.

| Mutation | Turns red |
|---|---|
| `feed_page_json` adds `"authorKey"` to every row | `the_feed_reply_is_the_ecosystems_pagination_shape`, `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`, `a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key` |
| `feed_page_json` adds a key only to a **hidden** row | `a_feed_row_revised_hidden_…` only |
| the envelope adds `"total"` | `the_feed_envelope_carries_exactly_items_page_and_has_more_on_every_page`, `a_feed_row_revised_hidden_…` |
| `sanitised_json` adds a fourth key | `a_feed_row_revised_hidden_…` only |
| an attachment entry alone adds a fourth key | `a_feed_row_revised_hidden_…` only |
| `thread_page_json` adds a top-level `"signer"` | `a_thread_reply_carries_exactly_its_contracted_keys_and_no_others`, `no_thread_item_holds_its_signers_key_under_any_key_but_author` |
| `thread_page_json` sets `position` to the author's hex | `no_thread_item_holds_its_signers_key_under_any_key_but_author` only |
| the row filter also demands `thread: None` | `a_parentless_post_carrying_a_thread_field_is_a_row_of_its_own` only |
| `parse_index`'s wrong-type message drops the field name | `malformed_pagination_fields_are_refused_by_name` only |

Before this change, nothing turned red for four of these: a key on a hidden
row only, a `total`, a fourth key on the sanitised object, and a row filter
that also required `thread: None`. So the new tests are the only guard on those
four properties. The thread-item test is mostly covered by the existing exact
key-set test. What it adds is a check on *values*: the signer's key copied into
a key the contract already allows is caught only by walking every value.

That last row also shows something outside this change. No wire test pins the
value of a thread item's `position`, so replacing it with any string leaves
every other test green.

`feed.rs`'s `a_row_carries_the_public_key_and_no_derived_display_name` keeps its
destructuring of `FeedRow`. It is the struct-level half: a new field fails to
compile there before it can reach the wire.

### 4. The thread item's key is `author`, for the reason #80 chose it

The issue cites `wire.rs:1819` as "the test that pins the shape" of the feed
row. At #88 that line was inside `thread_page_json`. It was the marker on the
**thread item's** spelling of its author key, and it pointed at #88's
`design.md` §4, not §5. The proposal covers the correction. The choice it marks
is resolved here by the `thread-read` requirement *An item's author key travels
under the JSON key `author`*.

The reasoning moves over from key-sweep §4. The thread item once carried
`author` (an address) and `authorKey` (the key), because the name read the key
while the mark read the address. Once both read the key, one field had to go.
`author` kept its name and took the key's hex, and `authorKey` was deleted,
because `authorKey` had been added *for* the split, and a single-field contract
should be spelled the way every other reply spells it. The other direction was
defensible: a caller still reading `authorKey` gets a missing field rather than
a wrong value, which is the loud failure. The spec now takes the decision, and
the row and the item share one documented spelling.

### 5. The pagination constants stay in code

`DEFAULT_PER_PAGE` and `MAX_PER_PAGE` in `feed.rs` are the values the spec calls
"a default" and "the cap". The spec contracts only their relation: the default
is no larger than the cap, zero is served as the default, and a larger request
is clamped. That matches `thread-read`. `feed.rs`'s `clamp_per_page` tests
reach the numbers through the constants rather than literals, and no test added
here names either value, so the spec's silence on them is not quietly turned
into a pin. Whether a caller must be able to rely on a number is an owner
question (proposal.md, *Open questions*).

The largest accepted page index is written as `usize::MAX`, not as a 64-bit
literal, for the same reason. It is this build's limit, not a constant of the
format. The index one past it is written as a `u128`, so the test can express
it on any target.

## Risks / Trade-offs

- **[The refusal for a page index past the largest reads oddly]** A `page` of
  2^64 is parsed as a float by `serde_json`, so it is refused with "page must be
  a non-negative integer written without a decimal point or exponent". The
  message names `page`, which is what the spec requires, but the reason it gives
  is for a different mistake. → Left as it is. Rewording it is a message change
  outside this change's "no behaviour change" line. It is recorded here so it is
  not rediscovered as a defect.
- **[The contract fixes today's shape, and #101 will change it]** → Intended.
  #101 modifies the `isHidden` requirement rather than changing an unspecified
  field, which is what closing the set was for.
