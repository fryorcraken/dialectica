# spec-test review — position-and-index (#166 / PR #180)

No findings. Every scenario in both deltas has a test that can fail for the
reason it names, coverage was measured (not read off `design.md`/`tasks.md`
claims), and the spec text is self-consistent and matches the owner's decision
comment. What was checked, and how:

## 1. Scenario coverage

**`specs/identity-onboarding/spec.md` (ADDED)** — one requirement, three
scenarios, all pinned in `dialectica/rust-lib/dialectica-core/src/wire.rs`:

- *Each malformed kind of `index` is refused by name* →
  `each_malformed_kind_of_index_is_refused_by_name`. `MALFORMED_INDEXES`
  reproduces the scenario's WHEN clause exactly: `-1`, `1.5`, `0.0`, `1e2`,
  `"two"`, `[]`, `true`, `18446744073709551616` (8 values covering the spec's 5
  bullets, with the decimal-point/exponent and not-a-number bullets each split
  into their constituent spellings). Fixture uses a valid Stoa and a live,
  current slate, so `index` is the only malformed thing about the request,
  matching the WHEN clause.
- *A malformed `index` stores nothing* → `a_malformed_index_stores_nothing`,
  against both a fresh peer (no keystore appears) and one already holding a
  master key (file bytes unchanged, no new path recorded) — and a positive
  control at the end (`index: 0` on the fresh fixture does store), so the
  negative assertions above aren't vacuously true of a fixture that can never
  write.
- *A well-formed `index` naming no candidate is a not-kept reply, not a
  malformed one* → `a_selection_outside_the_set_is_refused_and_stores_nothing`,
  which this piece extended with the `reason`-present / `error`-absent
  assertions the scenario needs (`SLATE_SIZE` is the scenario's "one greater
  than the last position").

**`specs/thread-read/spec.md` (MODIFIED)** — the position/asserted-time
requirement's 11 scenarios. The seven scenarios about the asserted-time grouping,
clamping, and rendering order are unchanged text and already tested elsewhere in
`wire.rs` (`a_thread_read_clamps_an_implausible_asserted_time_and_reports_the_clamp`
and neighbours around line 8800–9050) and in `thread.rs`
(`the_sequence_follows_the_counters_and_not_the_asserted_times`). The four
position scenarios this piece touches:

- *A position indexes the whole thread and does not restart per page* →
  `a_position_is_the_same_whatever_page_size_the_read_used` (compares by item
  ID across `perPage` 1/2/3 and a single full page), plus the pre-existing
  `thread::tests::every_item_carries_its_index_in_the_whole_thread_as_its_position`.
- *Two items of one thread never share a position* →
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`,
  using `a_thread_log_with_shared_authors` (two authors each write two items) —
  the fixture the issue said was missing, since the pre-existing
  `a_thread_log` gives every item a distinct author and can't discriminate a
  position from a per-author value.
- *The item at a place carries that place's position in every read* (new
  scenario, the place rule) → `the_item_at_a_place_carries_that_places_position_in_every_read`,
  reading a thread with a hidden reply that has a reply after it, once
  including hidden content and once excluding it, and checking both clauses:
  same position at a place both reads fill, different position for the item
  whose place moved.
- *The position is not surfaced as a quantity* — unchanged text, and
  `thread_positions_at`'s `text("position")` helper (used by all three tests
  above) already requires the field to parse as a JSON string, which is the
  same check other pre-existing tests make.

No scenario in either delta is untestable as written, and none asserts
something no test could check.

## 2. Mutation testing (budget: 2, both run and reverted)

**Mutation 1 — the place rule.** `dialectica-core/src/thread.rs`, `Placed::at`:
changed `position: index.to_string()` to `position: self.id.clone()` (position
= the item's own op id, the exact mutation named in the brief). Ran:

```
cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core \
  the_item_at_a_place_carries_that_places_position_in_every_read
```

**Result: failed**, as expected — `assert_eq!` on the shared-place clause
caught two different items (different op ids) claiming different positions at
the same place. Reverted; `git diff dialectica/rust-lib/dialectica-core/src/thread.rs`
is empty.

**Mutation 2 — the field name in `parse_index`'s wrong-type message.**
`dialectica-core/src/wire.rs`, the `Some(_) => Err(...)` arm of `parse_index`:
changed `format!("{field} must be a number")` to the literal `"must be a
number"` (drops the field name, the mutation the issue and the owner's decision
comment both name). Ran:

```
cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core \
  each_malformed_kind_of_index_is_refused_by_name
```

**Result: failed** — `the refusal of an index that is a string ("two") must
name index: {"error":"must be a number"}`. Reverted; `git diff
dialectica/rust-lib/dialectica-core/src/wire.rs` is empty.

Both mutations are gone from the tree; nothing was left in place. Read-only
reasoning (not mutated) also rules out the other two mutations named in the
issue: a constant position and a per-author value both duplicate positions
across the shared-author fixture's five items, which
`no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
checks pairwise over every item; a per-page-restarting index diverges from the
one-page baseline that `a_position_is_the_same_whatever_page_size_the_read_used`
compares against.

## 3. `NO SPEC:` markers

`git diff origin/main...HEAD -- dialectica/rust-lib/dialectica-core/src/wire.rs`
(three-dot diff) adds no `NO SPEC:` marker. Read the new tests for an unmarked
choice pinned without a scenario behind it, and found none: `MALFORMED_INDEXES`
maps 1:1 onto the spec's bullet list, the "well-formed index" boundary
(`SLATE_SIZE`) is the spec's own "one greater than the last position", and the
hidden-reply fixture's field checked (`kept`, `publicKey`, `path`, `encrypted`,
`reason` absent from the error shape) restates the spec's "MUST NOT carry any
field reporting a keep" rather than adding a choice of its own.

## 4. Requirements moved between capabilities

Not applicable — the change is one ADDED requirement (identity-onboarding) and
one MODIFIED requirement (thread-read); nothing is REMOVED from either
capability, so there is no REMOVED/ADDED pair to check for drift.

## 5. Spec soundness

- **Self-consistency:** read the whole of both requirement blocks. The
  MODIFIED requirement reworks "Two positions SHALL be unequal for two
  distinct items of one thread" (base) into "...for two distinct items
  returned by one read of a thread, taken across all of that read's pages"
  (delta) — this is necessary given the new place-rule paragraph a few lines
  below it, which explicitly allows two different items in two different reads
  to share a position at a shared place, and the same item to carry different
  positions across two reads. Scoping uniqueness to "one read, across pages"
  is what keeps those two paragraphs from contradicting each other, not a
  smuggled behaviour change — both live inside the same MODIFIED requirement's
  text, not split across a REMOVED/ADDED pair.
- **Testability:** no scenario in either delta asserts something untestable.
- **Staleness against issue #166:** read fresh with `gh issue view 166
  --repo fryorcraken/dialectica --json body,comments`, including the owner's
  2026-09-25 decision comment
  (https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024).
  The decision — malformed input is rejected and the refusal names the field,
  as a new requirement in the capability owning `keep_identity` (settling item
  2) — matches `specs/identity-onboarding/spec.md` verbatim in substance: the
  ADDED requirement's title and body state exactly that, scoped to
  `identity-onboarding` (which owns `keep_identity`), with the absent/null
  case explicitly deferred to `module-wire-contract` as the decision implies by
  saying nothing about it. Item 1 (constant/per-author/restarting-index
  mutations against the existing position requirement) is unchanged per the
  decision, and the new shared-author fixture and its three tests are exactly
  what item 1 asked for.

Areas read and found clean, not requiring a box: both requirement bodies in
full, all 3 identity-onboarding scenarios, all 11 thread-read scenarios, the
full three-dot diff of `wire.rs`'s `mod tests`, `parse_index` (read only for
the mutation), `Placed::at` (read only for the mutation), and
`feed-read`'s `malformed_pagination_fields_are_refused_by_name` for comparison
against `parse_index`'s shared field-naming behaviour.

## Re-review of the findings-round commits

Scope: `git diff 19667a04...HEAD` (three dots) only — the `thread-read` delta's
MUST→SHALL change, the new Purpose paragraph in the live
`openspec/specs/identity-onboarding/spec.md`, and the test-fixture refactor in
`dialectica/rust-lib/dialectica-core/src/wire.rs` (commit `70bef05`). Read the
owner's 2026-09-25 decision comment
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024)
before starting; it settles item 2 (malformed `index` refused, naming `index`)
and leaves item 1 unchanged, which is what this round's diff reflects — no new
tests, only a spec wording fix and a fixture refactor.

**1. `thread-read`'s MUST→SHALL change.** Read the whole requirement block
(`specs/thread-read/spec.md`). The two sentences changed
(*The position SHALL be determined by the place alone…* and *…those two items
SHALL carry the same position*) sit inside a paragraph where every surrounding
sentence already uses SHALL; the two MUSTs were the only holdouts. No
requirement text, scenario, or testable claim moved — this is a keyword
normalisation, not a behaviour change. The 11 scenarios are unchanged text and
unchanged coverage from the first round.

**2. The new Purpose paragraph in the live `identity-onboarding` spec.**
`proposal.md` explains why: a delta cannot edit a Purpose, so the
cross-capability note ("index is refused in parallel with `feed-read`, not by
it") is written directly into the live file so it survives archive (which only
merges the ADDED requirement). Checked for soundness rather than trusting that
explanation:
- The paragraph's claim that `identity-onboarding` states *A malformed `index`
  in a keep request is refused with a message naming `index`* is a verbatim
  requirement title in `openspec/changes/position-and-index/specs/identity-onboarding/spec.md`
  (unchanged this round, already reviewed).
- Its claim that this covers "the same kinds of malformed value" as
  `feed-read`'s `page`/`perPage` checked against `openspec/specs/feed-read/spec.md`
  line 555: "negative, fractional, written with a decimal point or an exponent,
  not a number, or an integer larger than the largest this peer accepts" —
  matches the index requirement's five bullets exactly.
- Checked it does not contradict the immediately preceding boundary paragraph
  ("restates none of the three" — `identity`, `keystore`,
  `posting-capability`): the new paragraph is about a fourth capability,
  `feed-read`, and says explicitly "Neither restates the other," so the two
  paragraphs are about different pairings and do not conflict.

No self-consistency defect found.

**3. The `wire.rs` fixture refactor (`70bef05`).** Read `git diff
19667a04...HEAD -- dialectica/rust-lib/dialectica-core/src/wire.rs`: a pure
extract-function refactor. `a_thread_post` now delegates to a new
`a_thread_post_with_clock(..., None)`, preserving the old hardcoded
`clock: None`; `a_thread_log_with_a_hidden_reply_and_a_reply_after_it`'s inline
`Op` literal is replaced by a call to the same helper with its clock supplied.
No assertion, fixture data, or ordering changed.

Measured rather than trusted, in this worktree (`worktree-agent-ac1b4040025c5b8c6`),
against `thread_page_json` in `wire.rs`:

- **Mutation A — `"position": item.author`.** Baseline (`cargo test
  --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core position`) green (6/6). With the mutation:
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
  and `the_item_at_a_place_carries_that_places_position_in_every_read` FAILED;
  `a_position_is_the_same_whatever_page_size_the_read_used` stayed green, as
  its own comment says it must (an author-derived value is stable across page
  sizes). This is the exact regression issue #166 reported against the old
  fixture (`a_thread_log`, one author per item) — confirms the new
  shared-author fixture (`a_thread_log_with_shared_authors`) still catches it
  after the refactor. Reverted; `git diff` on the file was empty before the
  next mutation.
- **Mutation B — a per-page restarting index** (`.iter().enumerate()`,
  `"position": mutation_index.to_string()`, so the value resets to 0 at the
  start of every page's JSON, modelling a handler that indexes only the page it
  is building). Result:
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
  and `a_position_is_the_same_whatever_page_size_the_read_used` FAILED;
  `the_item_at_a_place_carries_that_places_position_in_every_read` stayed
  green — expected, since a per-page index restarts identically in both the
  including- and excluding-hidden reads and that test only compares those two
  reads against each other, not against a one-page baseline. Reverted; `git
  diff --stat` on the file was empty afterward, and baseline reran green
  (6/6).

Both mutations were caught by the tests the design intends to catch them, none
survived, and `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
dialectica -p dialectica-core` (full suite, no filter) passed 1185+30 tests
with nothing failed, confirming the refactor changed no other test's fixture
behaviour. No mutation is left in the tree; `git diff` on `wire.rs` is empty.

No findings from this round.

## Re-review after merging main

Scope: `tasks.md`'s "Re-review after merging main" section — the merge
(`d0f56a14`), the `thread-read` delta's rebase onto the live text after #173
(`7d587fd`), and the `design.md`/`proposal.md` quote corrections (`696d35f`),
on this worktree (`worktree-agent-a5d9ecba45521f9b0`). Read the owner's
2026-09-25 decision comment on issue #166
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024)
before starting: it settles item 2 (a malformed `index` is refused, naming
`index`; leaving the message unspecified is ruled out) and leaves item 1
unchanged, matching what both deltas already state.

**1. Scenario coverage on the merged tree.** Read both delta files in full
against the live `openspec/specs/thread-read/spec.md` (now carrying #173's
removal) and confirmed nothing regressed:

- `specs/identity-onboarding/spec.md` is untouched since its one commit
  (`4960f249`), predates every main PR that landed (#172, #173, #175, #181),
  and none of those touched `openspec/specs/identity-onboarding/spec.md` or
  `openspec/specs/feed-read/spec.md` (`git log --oneline` on both, checked).
  Its three scenarios are still pinned exactly as the first round recorded:
  `each_malformed_kind_of_index_is_refused_by_name`,
  `a_malformed_index_stores_nothing`,
  `a_selection_outside_the_set_is_refused_and_stores_nothing`.
- `specs/thread-read/spec.md`'s rebase (`7d587fd`) removes exactly one
  paragraph — "The alternatives all invite arithmetic that means nothing...",
  the same paragraph #173 removed from the live requirement (`git show
  2b53e6eb -- openspec/specs/thread-read/spec.md`) — and changes nothing
  else. Compared the full requirement block word by word against the live
  file (`openspec/specs/thread-read/spec.md` lines 704-778): every paragraph
  and every scenario the delta carries is either identical live text or one
  of this piece's three own changes (the narrowed uniqueness sentence, the
  place-rule paragraph, the new "item at a place" scenario). The removed
  paragraph carried no SHALL/MUST and no scenario of its own — pure
  reasoning — so no scenario lost its test. All 11 scenarios still resolve to
  the same tests the first round found:
  `a_position_is_the_same_whatever_page_size_the_read_used`,
  `every_item_carries_its_index_in_the_whole_thread_as_its_position`,
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`,
  `the_item_at_a_place_carries_that_places_position_in_every_read`, plus the
  unchanged asserted-time tests already cited in the first round.
- Confirmed `dialectica/rust-lib/dialectica-core/src/wire.rs` and
  `dialectica/rust-lib/dialectica-core/src/thread.rs` carry no changes from
  main between the piece's base and the merge (`git log --oneline
  8368b2f..d0f56a14 -- <path>` on both lists only this piece's own #166
  commits), so the merge could not have silently altered the code these
  tests exercise.

**2. Mutation re-measurement on the merged tree (budget: the three the brief
named, all run and reverted).** Baseline: `cargo test --manifest-path
dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` green, 1185
passed (core) + 30 passed (end-to-end), 0 failed.

- **Mutation 1 — position equal to the item's own op id, in
  `thread_page_json`** (`wire.rs`): changed `"position": item.position,` to
  `"position": item.id,`. Ran `cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica-core position`. **Result:
  failed** —
  `the_item_at_a_place_carries_that_places_position_in_every_read` panicked
  ("sit at the same place but carry different positions"), because two
  different items (different op ids) now claim different positions at a
  shared place across the including/excluding-hidden reads.
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
  and `a_position_is_the_same_whatever_page_size_the_read_used` stayed green,
  as expected — op ids are already unique and already page-size-stable, so
  this mutation is invisible to those two. Reverted; `git diff` on the file
  empty afterward.
- **Mutation 2 — a per-page restarting index, in `thread_page_json`**:
  added `.enumerate()` to the item iterator and set `"position":
  mutation_index.to_string(),` instead of `item.position`. Ran the same
  filtered command. **Result: failed** — both
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
  (two items at index 0 across the two authors' pairs) and
  `a_position_is_the_same_whatever_page_size_the_read_used` (position "0" at
  every `perPage` vs. "0".."4" at a single page) failed.
  `the_item_at_a_place_carries_that_places_position_in_every_read` stayed
  green, as expected — a per-page index restarts identically in both the
  including- and excluding-hidden reads, and that test only compares those
  two reads against each other. Reverted; `git diff` on the file empty
  afterward.
- **Mutation 3 — `{field}` dropped from `parse_index`'s wrong-type message**:
  changed `Err(error_json(&format!("{field} must be a number")))` to
  `Err(error_json("must be a number"))`. Ran `cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica-core
  each_malformed_kind_of_index_is_refused_by_name`. **Result: failed** — "the
  refusal of an index that is a string (\"two\") must name `index`:
  {\"error\":\"must be a number\"}". Reverted; `git diff` on the file empty
  afterward.

Re-ran the full suite after all three reverts: `cargo test --manifest-path
dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` green again,
1185 + 30 passed, 0 failed. `git status --short` on the whole tree is empty —
no mutation, and nothing else, is left in place.

**3. Soundness of the rebase itself.** The MODIFIED block's uniqueness
sentence ("...for two distinct items returned by one read of a thread, taken
across all of that read's pages") and the place-rule paragraph it sits beside
are unchanged by the rebase — the rebase touched only the removed paragraph,
below both. No new self-consistency question is introduced by merging main.

No findings from this round.

- [x] **none** — no spec-test findings on the merged tree; all three named mutations (position = op id, per-page index, `{field}` dropped from `parse_index`) failed the tests that name them, and the full suite is green (1185 + 30).
