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
