# Code review — `ui-remaining-screens` (all four dimensions)

Reviewed as one pass covering correctness, security, readability and
architecture, per the dispatch brief. Scope: the delta this piece actually
ships against `origin/main` — `dialectica-ui/src/qml/DStoaListScreen.qml`
(delegate block, lines ~374-507) and `dialectica-ui/src/qml/DTheme.qml`
(`rowTitle` token), plus the spec delta, `design.md`, `tasks.md` and
`tst_stoa_screens.qml`'s row-treatment tests.

## Verified claims (no findings — recorded so the closer does not re-derive them)

- **`git diff origin/main...HEAD` withdrawal claim holds.** `qmldir`,
  `docs/PLAN.md`, `dialectica-ui/tests/tst_composer_claims.qml`,
  `dialectica-ui/src/qml/FeedScreen.qml` and `dialectica-ui/src/qml/Main.qml`
  are byte-identical to `origin/main` (`git diff origin/main...HEAD --stat` for
  each returns nothing). Only `DStoaListScreen.qml`, `DTheme.qml`,
  `tst_stoa_screens.qml` and the openspec/docs files under
  `openspec/changes/ui-remaining-screens/` differ.
- **The spec-scoping quote is accurate.** `openspec/specs/stoa-navigation-view/spec.md:8`
  reads exactly as cited in the brief and in design.md D5b — verified by
  `sed -n '8p'` against the merged spec, not the delta.
- **The ADDED requirement's argument holds up.** The delta
  (`openspec/changes/ui-remaining-screens/specs/stoa-navigation-view/spec.md`)
  contracts the row boundary and explicitly declines to contract the type,
  citing the Purpose sentence that puts "colours, type, metrics" outside the
  capability by name. The argument that the separator resolves *which row an
  attacker-controlled title/address pair belongs to* — not mere decoration —
  is sound given the capability already contracts that two Stoas can carry
  byte-identical founding titles
  (`test_two_stoas_with_the_same_title_render_differently` exists and passes).
  The `NO SPEC:` marker on the type is the right call for the same reason:
  pinning 19px would contradict the capability's own scoping sentence, and the
  gap ("which capability owns the view's type scale") is correctly left for a
  `spec-writer`, not silently resolved by a dev.
- **Both claimed mutations reproduce exactly**, re-run against
  `tst_stoa_screens.qml` (81 baseline tests, `sh
  dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`):
  - Dropping the separator on the last row
    (`visible: index < visibleRows.length - 1`, with `required property int index`
    added since the `ColumnLayout` delegate needs it declared explicitly under
    Qt 6's `required property` rules) → **79 passed, 2 failed**:
    `test_every_rendered_row_is_separated_from_the_next` (3 expected, 2 found)
    and `test_the_row_count_and_the_separator_count_move_together` (1 expected,
    0 found) fail; `test_an_empty_list_draws_no_row_boundary` stays green.
    Matches the claim exactly.
  - Hoisting the separator out of the delegate to the list container (a single
    `Rectangle objectName: "rowSeparator"` after the `Repeater`, not inside it)
    → **78 passed, 3 failed** — all three separator tests fail, including the
    empty-list one (1 found where 0 expected). Matches the claim exactly.
  - `font: DTheme.rowTitle` → `DTheme.body` → **80 passed, 1 failed**:
    `Actual (): 15, Expected (): 19` on
    `test_a_row_title_is_set_apart_from_body_prose_without_reaching_a_heading`.
    Matches exactly.
  - Additionally verified the "relation, not literal" claim beyond what the
    brief asked: with the mutation reverted, temporarily raising
    `DTheme.body`'s `pixelSize` from 15 to 19 (so `rowTitle == body == 19`)
    still fails the same test, on the `rowTitle > body` assertion — proving a
    hardcoded `compare(pixelSize, 19)` would have passed here while the actual
    test correctly catches the collapsed distinction.
  - All mutations cleanly reverted; `git status` is clean and
    `git diff --stat` on both touched files is empty after each revert.
- **`tst_render_probe.qml:273`'s reading is accurate.**
  `test_a_populated_stoa_list_screen_paints_content` calls
  `assertContentPaints`, which (lines 206-220) asserts only that the sampled
  content area is not a single flat colour. It has no separator-specific
  assertion, and a screen with the separator dropped entirely still paints a
  title, address, identicon and two buttons — the probe would not move.
- **The stale `lib.rs:258` citation is out of scope, correctly.**
  `DIdentityChip.qml:49` and `DThreadScreen.qml:50` do cite the stale line
  number (the "can honestly disagree" text now lives at `lib.rs:284-285`,
  confirmed by `grep -n "honestly.*disagree"` — off by one line from the
  brief's `284-288` but the same sentence), and both files are byte-identical
  to `origin/main` — this piece did not introduce or touch them. Bundling an
  unrelated one-line citation fix into a piece scoped to the row treatment
  would be exactly the kind of unreviewable, unrelated diff CLAUDE.md's
  "make the change easy, then make the easy change" warns against. Leaving it
  for a separate piece is the right call.
- **Every `Text` element touched by this piece's actual diff carries explicit
  `textFormat: Text.PlainText`.** The row title at
  `DStoaListScreen.qml:420-428` (peer-supplied, unnormalised content) is
  explicit. No `Text` element in the file was found defaulting to
  `Text.AutoText`.
- **No new imports, no new dependencies.** `DStoaListScreen.qml` still imports
  only `QtQuick` and `QtQuick.Layouts`.
- **Gates all green**: `check_qml_names.py dialectica-ui`,
  `check_qml_members.sh`, `check_qml_reachable.py dialectica-ui`, and
  `openspec validate ui-remaining-screens --strict` all pass clean.
- **Architecture of the `ColumnLayout`-wrapping-`RowLayout` shape is sound.**
  Nothing outside the delegate references the delegate root by id or by
  `Repeater.itemAt` in a way the reindent could break (checked by grep across
  the QML and both test files); wrapping in a `ColumnLayout` to give the row
  and its separator a shared parent is the standard idiom for "row content
  plus a boundary" in QtQuick Layouts, not an over-engineered indirection.

## Findings

- [x] **`dev-writer`** — `openspec/changes/ui-remaining-screens/design.md:230` vs.
      the dispatch brief's "57 lines" — **readability/documentation, low severity.**
      **Scenario:** design.md itself says "the 62 lines this piece ships"; the
      actual `git diff origin/main...HEAD -- dialectica-ui/src/qml/DStoaListScreen.qml`
      is 142 insertions / 81 deletions (net +61 in that file, +5 in
      `DTheme.qml`), because the reindent commit (`d59b479`) touches nearly
      every line of the delegate via `git diff` even though `git diff -w`
      shows only comment reflow and leading whitespace. Neither "57" nor "62"
      is a wrong description of *behavioural* change, but a future reader
      diffing the file wholesale (without `-w`) and expecting "62 lines" will
      find a much larger diff and may wonder whether they are looking at the
      right commit. **Not a defect** — the reindent is honestly labelled as
      its own commit with `git diff -w` verified in its own commit message —
      but the "62 lines" figure in design.md could usefully say "62 lines of
      behavioural diff (`git diff -w`); the raw diff is larger because of the
      reindent in `d59b479`" so nobody re-derives the discrepancy from
      scratch the way this review did. Optional wording fix, not a blocker.

      **Fixed** — but not by the suggested wording. The figure is **dropped**
      rather than restated. A count qualified as "62 lines of behavioural diff
      (`git diff -w`)" would still be a number nobody re-measures: correct on
      the day it was written and silently wrong after the next commit to the
      delegate, which is the failure mode CLAUDE.md's "do not write down
      anything a command can answer" names. The sentence's actual claim is that
      the row treatment was untested, and that claim does not need a line count
      at all, so D5c now opens "Answering *nothing tests the row treatment this
      piece ships*". An italic note underneath records why the count is absent —
      that the raw diff is much larger than the behavioural change because of
      the reindent in `d59b479`, and that `git diff -w` is what separates them —
      so the discrepancy this review re-derived is written down once without a
      figure that can rot. Nothing in the line count was load-bearing for the
      argument.

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:556-587`
      (`test_the_row_count_and_the_separator_count_move_together`) —
      **correctness/test-quality, the author's own flagged low-confidence
      test.** **Scenario:** the function creates `one` (a `DStoaListScreen`),
      captures `afterOne` at line 567 (before `destroy()`, which is correct —
      the count is read while the object is live), calls `one.destroy()` at
      line 568, then immediately constructs `four` via `makeList()`, which
      reassigns `Core.bridge` and creates a second `DStoaListScreen` while the
      first's deletion is still only *queued* (QML `destroy()` schedules
      deletion on the event loop rather than freeing synchronously). The two
      screens' component trees can briefly coexist. In this repo's actual
      qmltestrunner execution the test passes reliably (verified in this
      review's mutation runs above, and the design.md D5c table's numbers
      reproduce exactly), so there is no live bug today. But the test's
      correctness leans on `Repeater` building delegate children synchronously
      on model assignment and on `visibleNamed`'s walk running before any
      queued deletion resolves — neither of which the test states or pins,
      and both of which are timing assumptions rather than assertions.
      **Suggested fix (not applied — findings only):** split into two
      independent test functions, each creating, measuring and destroying
      exactly one screen, so the "row count and separator count move
      together" relation is established by comparing two independent
      measurements rather than by two objects whose lifetimes overlap in the
      same function body. This removes the object-lifetime dependency the
      author already flagged as their own least-confident test, at no cost to
      what is covered.

      **Fixed.** Split, as suggested, but through a helper rather than by
      duplicating the create/measure/destroy sequence into each function:
      `separatorsForRowCount(replies, expectedRows)` creates one screen,
      asserts its row count, reads the separator count and destroys it before
      returning, so the overlap is removed by construction rather than by each
      test remembering the ordering. Only the number outlives the screen. The
      two functions are now `test_one_row_draws_exactly_one_boundary` and
      `test_the_row_count_and_the_separator_count_move_together`; baseline goes
      from 81 to **82 passed, 0 failed**.

      **Both mutations re-run against the split, and both halves still catch
      what the combined test caught** — this was the risk in the split and it
      was measured, not assumed:

      - drop the separator on the last row (`visible: rowBlock.index <
        screen.visibleRows.length - 1`, with `required property int index`
        added) → **79 passed, 3 failed**: `test_one_row_draws_exactly_one_boundary`
        (1 expected, 0 found), `test_every_rendered_row_is_separated_from_the_next`
        (3 expected, 2 found) and the relation test (4 expected, 3 found). The
        empty-list test correctly stays green. The combined form failed 2 here;
        the split fails 3, so nothing was lost.
      - hoist the separator to the list container → **79 passed, 3 failed**:
        the empty-list test (0 expected, 1 found), the three-row test (3
        expected, 1 found) and the relation test (4 expected, 1 found). **The
        second property is intact** — the empty-list case still catches the
        mutation nothing else does.

      One result worth recording because it justifies keeping both halves:
      `test_one_row_draws_exactly_one_boundary` **passes** under the hoist, since
      a hoisted rule renders exactly one and a one-row list cannot tell that from
      a correct delegate. That is why the relation test re-measures the one-row
      count from its own screen instead of asserting `afterFour - afterOne == 3`
      against a hardcoded `1`: reducing it to a literal would turn the relation
      back into a second assertion about the four-row fixture. Each of the two
      mutations now turns a *different* set of three tests red, which is what
      makes each test in the set non-redundant.

      `font: DTheme.rowTitle` → `DTheme.body` re-measured too rather than
      relayed: **81 passed, 1 failed**, `Actual 15, Expected 19`. All three
      mutations reverted; `git status` shows only the intended files.

      The reasoning is now durable in `design.md` — D5d records why each test
      owns one screen's whole lifetime and what breaks if the two-screens-in-one-
      function form comes back, and D5c's mutation table carries the re-measured
      numbers. `tasks.md`'s two coverage rows were updated to match (four
      separator tests, not three).

No other correctness, security, readability or architecture defects were
found in the piece's actual diff. The four review dimensions named in the
dispatch brief (correctness, security, readability, architecture) were all
covered in this single pass, as instructed for a small change.
