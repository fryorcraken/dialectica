# Correctness review — position-and-index (#166 / PR #180)

Reviewed dimension: **correctness only** (dispatched for this dimension alone).

Read issue #166 (`gh issue view 166 --json body,comments`) including the
owner's decision comment of 2026-09-25
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024):
malformed `index` is refused with an error naming `index`; leaving the
message unspecified is ruled out. Confirmed the added `identity-onboarding`
requirement and its scenarios match that decision exactly (name required,
reason text explicitly left unspecified, both recorded and justified in
`proposal.md`'s "Deliberately left unspecified").

## What I checked

- Full `git diff origin/main...HEAD` (three dots): `.openspec.yaml`,
  `proposal.md`, `design.md`, `tasks.md`, the two spec deltas
  (`identity-onboarding`, `thread-read`), and the test additions in
  `dialectica/rust-lib/dialectica-core/src/wire.rs`. Confirmed via
  `git diff --stat` that no other source file changed — the piece's own claim
  of "no production code changes" holds.
- Read the production code the new tests exercise: `parse_index` (`wire.rs`),
  `read_thread` / `Placed::at` / `resolve_item` (`thread.rs`), and `cmp_ops`
  (`arrival.rs`), to check the design's factual claims about how positions and
  reply order are computed.
- Built the SDK symlink and ran the full `dialectica` + `dialectica-core` test
  suite: 1185 unit tests + 30 e2e tests, all green.
- Reproduced three of the mutations `design.md` and `tasks.md` claim were
  measured, to verify the claims rather than trust the prose:
  - Dropped `{field}` from `parse_index`'s wrong-type message →
    `each_malformed_kind_of_index_is_refused_by_name` failed on the first
    case (`negative (-1)`), exactly as claimed.
  - Set `Placed::at`'s position to the item's own op id →
    `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
    and `a_position_is_the_same_whatever_page_size_the_read_used` **stayed
    green**, and `the_item_at_a_place_carries_that_places_position_in_every_read`
    (plus the pre-existing `thread::tests::every_item_carries_its_index_in_the_whole_thread_as_its_position`)
    **failed** — exactly the split D2/D5 describe.
  - Set the position to a constant `"0"` → the uniqueness test and the core
    index test failed, the page-size test stayed green — exactly as D2
    describes.
  - All three mutations were reverted by `Edit` afterward; `git diff --stat`
    on `thread.rs` and `wire.rs` is empty, confirming the tree is back to
    `origin/main...HEAD`'s content.
- Traced the hidden-reply fixture
  (`a_thread_log_with_a_hidden_reply_and_a_reply_after_it`) against `cmp_ops`
  and `read_thread`'s reverse walk to confirm the claimed ordering (ascending
  explicit counters land in append order: root, `to_hide`, `after`, `filler`)
  and against `resolve_item`/`Placed::at` to confirm the include/exclude
  reads produce 4 vs 3 items and that place-by-place comparison plus the
  `after`-item comparison exercise exactly the "place, not item" property the
  new `thread-read` scenario states.
- Checked `MALFORMED_INDEXES`'s eight entries against the new
  `identity-onboarding` requirement's five malformed categories (negative,
  fractional, decimal-point-spelled-whole, exponent-spelled-whole, not-a-
  number, too-large) — not-a-number is exercised by three concrete JSON
  types (string, array, boolean); all categories are covered.
- Checked `keep_with_raw_index` splices `index` as raw JSON text (not a
  quoted string), so `1.5`, `1e2`, `[]`, `true`, `"two"` and the too-large
  literal all reach `parse_index` as the intended JSON shapes.
- Confirmed `tasks.md`'s task list matches what's actually in `wire.rs` (task
  2.3's `a_selection_outside_the_set_is_refused_and_stores_nothing` is indeed
  the test extended with the new `reason`/no-`error` assertions; task 3.1's
  fixture is the same one task 3.2/3.3 use; task 3.4 is
  `the_item_at_a_place_carries_that_places_position_in_every_read`).

## Findings

None. I did not find a correctness defect in this piece.

The spec deltas, design rationale, and test code are internally consistent
with each other and with the actual (unchanged) production behaviour. Every
falsifiable claim in `design.md` that I could cheaply re-run — the three
named mutations and the "no production code changes" claim — reproduced
under direct testing rather than only reading the prose. The new
`identity-onboarding` requirement matches the owner's decision comment
verbatim in effect (name required, message text explicitly out of scope).

This review covered correctness only. Security, readability, and
architecture are separate reviewer dimensions and are not addressed here.

## Re-review of the findings-round commits

Read issue #166 again (`gh issue view 166 --json body,comments`), including
the owner's decision comment of 2026-09-25
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024):
a malformed `index` is refused with an error naming `index`; leaving the
message unspecified is ruled out. `tasks.md`'s "Re-review of the
findings-round commits" section assigns me the fixture refactor in
`70bef050` only — `wire.rs` is otherwise untouched in `git diff
19667a04...HEAD` (confirmed by `--stat`: the only other files changed are
`design.md`, `proposal.md`, the two spec deltas, and findings/tasks docs,
none of which are this dimension's).

### What I checked

- `git diff 19667a04...HEAD -- dialectica/rust-lib/dialectica-core/src/wire.rs`
  in full. Two hunks: (1) `a_thread_post` now delegates to a new
  `a_thread_post_with_clock(..., clock: Option<OpClock>)`, which is the one
  place the fixture's `Op` literal is written; (2) the `Op` literal
  previously inlined in `a_thread_log_with_a_hidden_reply_and_a_reply_after_it`'s
  `under_root_at` closure is replaced by a call to the new helper, passing
  the same `stoa`, `author`, `clock`, `thread`, `parent`, `body`,
  `attachments: vec![]` it built by hand before. A third hunk is pure
  rustfmt (the `thread_positions_at(&log, MAX_PER_PAGE, false)` call
  reformatted to four lines) — no textual or behavioural change, part of
  the separate `4e2b8c0f` commit.
- Diffed field-by-field: both the old inline literal and the new helper set
  `stoa`, `author`, `clock`, `thread`, `parent`, `body`, `attachments`
  identically; nothing is dropped, defaulted, or reordered.
- Ran the full suite before touching anything:
  `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core` — 1185 unit tests + 30 e2e tests, all green (baseline).
- **Mutation, to check the refactor didn't quietly make a test unfalsifiable
  by decoupling the fixture's `clock` parameter from the `Op` it builds**:
  edited `a_thread_post_with_clock` to hardcode `clock: None` regardless of
  the argument passed in (the shape a careless refactor could produce, since
  the field is set once for both callers). Ran the two position tests that
  exercise `a_thread_log_with_a_hidden_reply_and_a_reply_after_it` (the only
  fixture that passes `Some(OpClock{...})` through this parameter):
  - `the_item_at_a_place_carries_that_places_position_in_every_read` —
    **failed**: `assertion `left != right` failed: the reply following the
    hidden one must carry different positions in the two reads: including
    "1", excluding "1"`. With the clock dropped, all three replies tie-break
    on op id (an unpredictable hash) instead of the intended append order,
    so the fixture's "adjacency" precondition (`to_hide` immediately before
    `after`) is no longer guaranteed and the test's core assertion breaks.
  - `a_position_is_the_same_whatever_page_size_the_read_used` — stayed
    green, as expected: that test only checks a property invariant across
    page sizes, not against a specific fixture ordering.
  - Reverted the mutation with `Edit`; `git status --short` and `git diff
    --stat` on `wire.rs` are both empty afterward, confirming the tree is
    back to `19667a04...HEAD`'s content.
- Conclusion: the refactor is behaviour-preserving, and the fixture that
  exercises the new `clock` parameter can still fail when that wiring is
  broken — the consolidation did not create an untestable fixture.

### Findings

None for correctness. The `70bef050` fixture refactor changes no test
behaviour and is still load-bearing under mutation.

This re-review covered correctness only, scoped to `70bef050` per
`tasks.md`'s assignment. I did not re-review `b55d164`, `9e550eb`, or
`4e2b8c0` — those are readability's, architecture's, and design's rows in
this same re-review round.
