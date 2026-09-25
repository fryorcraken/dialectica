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

**This is a guard, and what breaks without it is measured.** Each of these was
applied to the code, run, and reverted:

| Mutation | Turns red |
|---|---|
| `feed_page_json` adds `"authorKey": row.author` to every row | *filled in from the run* |
| `feed_page_json`'s envelope adds `"total"` | *filled in from the run* |
| `sanitised_json` adds a fourth key | *filled in from the run* |
| `thread_page_json` adds `"authorKey": item.author` | *filled in from the run* |

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
is clamped. That matches `thread-read`. The tests reach the numbers through the
constants rather than through literals, so the spec's silence on the values is
not quietly turned into a pin. Whether a caller must be able to rely on a number
is an owner question (proposal.md, *Open questions*).

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
