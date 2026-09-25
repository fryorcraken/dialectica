# Design review — feed-row-contract

No findings. The Decisions section is in good shape and the code takes every
decision it records.

## What was checked

- **Decision 1** (write down current behaviour, no redesign): the change adds
  no new production behaviour. `feed.rs` and `wire.rs` are untouched except for
  the doc-comment and marker cleanups tasks.md lists; `cargo test
  --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core` passes (1153 + 30 tests, 0 failed) with the spec's tests
  added on top, confirming no test needed a behaviour change to pass.
- **Decision 2** (`author` is the signing key's hex, nothing derived travels
  beside it): matches `FeedRow::author` (`entry.op.op.author.to_hex()`) and the
  wire tests (`the_wire_reports_the_author_as_one_key_and_no_name`,
  `a_row_carries_the_public_key_and_no_derived_display_name`), which assert the
  absence of `name`, `displayName`, `generatedName`, `mark`, `address`,
  `authorAddress` and `authorKey`.
- **Decision 3** (closed sets asserted as exact key lists, written out at the
  call site): confirmed in `wire.rs` —
  `the_feed_reply_is_the_ecosystems_pagination_shape`,
  `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`,
  `a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`
  and `the_feed_envelope_carries_exactly_items_page_and_has_more_on_every_page`
  all assert a sorted key list as a literal, never derived from `FeedRow` or
  its JSON output. The hidden/revised/attachment/reply fixture asserts each
  state was actually reached before asserting the key set, as the entry
  describes.
- **Decision 4** (`thread-read`'s item author key is `author`, not
  `authorKey`): `thread_page_json` in `wire.rs` emits `"author": item.author`
  with a comment pointing at the `thread-read` requirement and this change's
  `design.md`, and no `NO SPEC:` marker remains on that field. The markers on
  `position`, `assertedTime` and the moderation-state spellings are still
  present, matching the proposal's statement that those stay unresolved.
- **Decision 5** (pagination constants stay in code; largest index is
  `usize::MAX`, not a literal): `clamp_per_page` tests reach the numbers
  through `DEFAULT_PER_PAGE`/`MAX_PER_PAGE`, no test in this change pins either
  value, and the largest-index test (`wire.rs`) is written from `usize::MAX`
  with the one-larger value computed as `u128::try_from(largest).unwrap() +
  1`, exactly as recorded.
- **Markers and stale prose** (tasks.md §4): `git grep -n "NO SPEC:"` shows
  no remaining marker in `feed.rs`, and `FeedRow::author`'s doc comment no
  longer claims "no capability owns this reply's shape" — the
  `a_row_carries_the_public_key_and_no_derived_display_name` test now cites
  the three `wire.rs` key-set tests instead of the nonexistent
  `the_feed_json_is_pinned_to_the_exact_shape_a_view_is_written_against`.
- **Issue reasoning migrated**: the issue's two markers (`feed.rs:657` at #88,
  `wire.rs:1819` at #88) are traced to their current locations and resolved
  requirements in both `proposal.md` and `design.md` Decision 4, including the
  correction that the issue mis-described the second marker as covering the
  feed row when it in fact covered the thread item's `author` spelling. The
  issue's own reasoning for a closed-set test ("what a row does not carry,
  stated positively") is quoted and extended in Decision 3.
- **Open questions**: the two behavioural questions the issue implicitly
  raises (parentless-post-with-`thread`-field, and the boolean vs
  three-valued `isHidden`) are carried into `proposal.md`'s "Open questions
  for the owner" rather than decided in code, consistent with the brief that
  this change writes down current behaviour and defers any redesign.

## Gates run in this tree

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core` — passed (1153 unit/integration tests in the lib crate + 30
  in `end_to_end.rs`, 0 failed).
- `nix build ./dialectica#lgx` — succeeded.
