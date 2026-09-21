# spec-test review — `ui-remaining-screens`

Reviewed blind to `DStoaListScreen.qml` and `DTheme.qml`, per role. Scope: the
one ADDED requirement in
`openspec/changes/ui-remaining-screens/specs/stoa-navigation-view/spec.md`
("Where one listed Stoa ends and the next begins is rendered, not left to
spacing") against `dialectica-ui/tests/tst_stoa_screens.qml`'s row-treatment
block (lines ~520-668).

No blocking findings. Below is what was checked and measured, for the record.

## 1. Scenario coverage

All three scenarios have direct tests at the right layer (a QML component test
observing rendered elements is the correct layer for a claim entirely about
what the view renders):

- "Each rendered row is separated from the next" ->
  `test_every_rendered_row_is_separated_from_the_next` (3 rows, asserts
  boundary count == row count, verbatim to the scenario's "number of
  boundaries equals the number of rows rendered").
- "The final row is terminated too" -> the same test, since a delegate that
  drops the last row's boundary produces 2 where 3 are asserted, and is
  additionally isolated by `test_one_row_draws_exactly_one_boundary` (1 vs 1,
  the case a dropped-last-row implementation fails outright: 0 found).
- "A list with no rows renders no boundaries" ->
  `test_an_empty_list_draws_no_row_boundary`.

No scenario found untestable as written; no scenario found without a test.

## 2. Can the tests fail? Two mutations run, both reproduced exactly

**Mutation 1 — drop the separator on the last row.** Added
`visible: index < screen.visibleRows.length - 1` to the `rowSeparator`
Rectangle in `DStoaListScreen.qml` (plus a `required property int index` the
delegate needed to expose it, since the delegate already declares
`required property var modelData` and QML's required-properties mode does not
inject `index` for free once any property on the delegate is `required`).

Result, via
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`:
**79 passed, 3 failed** —
`test_every_rendered_row_is_separated_from_the_next` (found 2, expected 3),
`test_one_row_draws_exactly_one_boundary` (found 0, expected 1),
`test_the_row_count_and_the_separator_count_move_together` (found 3, expected
4). `test_an_empty_list_draws_no_row_boundary` stayed green. This matches the
author's claimed numbers and failing set exactly (tasks.md's Tests section).

**Mutation 2 — hoist the separator out of the delegate to the list
container.** Removed the per-row `Rectangle` from inside the `ColumnLayout`
delegate and added one unconditional `Rectangle { objectName: "rowSeparator"
... }` as a sibling after the `Repeater`, inside the same outer
`ColumnLayout` the Repeater already sits in.

Result: **79 passed, 3 failed** — a *different* three:
`test_an_empty_list_draws_no_row_boundary` (found 1, expected 0),
`test_every_rendered_row_is_separated_from_the_next` (found 1, expected 3),
`test_the_row_count_and_the_separator_count_move_together` (found 1, expected
4). This also matches the author's claim exactly, including the specific
detail flagged in the brief as the claim worth testing hardest:
**`test_one_row_draws_exactly_one_boundary` PASSED** under this mutation — a
hoisted rule renders exactly one boundary regardless of row count, and a
one-row fixture cannot tell that apart from a correct per-row delegate. This
is the author's stated argument for why the relation test
(`test_the_row_count_and_the_separator_count_move_together`, 1-vs-4) earns its
place alongside the 3-row test rather than duplicating it, and it held up
under measurement: the two mutations turn different sets of three tests red,
confirming non-redundant coverage.

Note: my first attempt at mutation 2 added a `visible: screen.visibleRows.length
> 0` guard on the hoisted Rectangle "to be safe," which produced 80/2 (the
empty-list test passed because the guard suppressed it) — not the claimed
result. Removing that guard, i.e. a *genuinely* unconditional hoist as
"hoisting the separator out of the delegate" would actually be written,
reproduced the author's claimed 79/3 with the empty-list test among the
failures. Recorded so a future reader doesn't mistake the guarded variant for
the mutation under test.

Both mutations were reverted; `git status` and `git diff --stat` confirm a
clean tree after each revert and at handback.

Third mutation (`DTheme.rowTitle` -> `DTheme.body`, claimed 81/1 with `Actual
15, Expected 19`) was not re-run — budget is one or two mutations, and the two
above were judged higher-value: they guard the requirement actually added by
this delta, where the type-scale test guards a `NO SPEC:`-marked, deliberately
unrequired relation. The claimed failure mode (a hardcoded-vs-relation
distinction) is also easy to verify by reading alone: the test asserts
`DTheme.rowTitle.pixelSize > DTheme.body.pixelSize` and `< DTheme.heading
.pixelSize`, which is false whenever `rowTitle` and `body` resolve to the same
token, so it can fail. No mutation needed to see that this one can move.

## 3. The `NO SPEC:` marker

`test_a_row_title_is_set_apart_from_body_prose_without_reaching_a_heading`
carries `// NO SPEC: the type a row title is set at is NOT contracted
anywhere.` Judged honest rather than an evasion: `stoa-navigation-view`'s
Purpose (line 8) names "colours, type, metrics, the mark" as outside the
capability by name, and the separator requirement's own "What this does NOT
require" clause applies the identical carve-out to thickness, colour and the
drawing element. Marking the row-title's pixel size out of scope for the same
reason is consistent rather than convenient, and the marker correctly hands
the open question to a `spec-writer` — "which capability owns the view's type
scale" — rather than silently deciding it. No unmarked instance of this shape
found in the row-treatment block.

## 4. Requirement vs. styling — the framing judgement the brief asked for

Read against Purpose line 8 directly. The requirement text is a
presence/count contract ("a rendered boundary" exists "on every row including
the last," with the explicit exclusion of "thickness, colour, or the element
that draws it") rather than an appearance contract, and it does not name a
drawing component the way the address-abbreviation requirement names
`AddressLabel`. That keeps it on the correct side of the Purpose's carve-out.
The justification given — resolving *which row* a rendered half belongs to,
tied to the same-founding-title scenario the capability already contracts —
is stated in the requirement's own body, not asserted only in tasks.md, so
the argument is checkable against the requirement text itself and holds.

## 5. Test keying: `objectName: "rowSeparator"` rather than geometry

Judged correct from the spec's side. The requirement explicitly says "A
future treatment that divides rows some other way satisfies it, and is meant
to" — an intent-keyed lookup is what stays compatible with a future
non-Rectangle boundary, where a geometry-keyed lookup (thickness, height,
colour) would both over-fit a treatment the requirement declines to fix and
under-fit a correct future replacement. This repo's defect family (a fixture
where two explanations give the same answer) does not apply here in the
form the brief was watching for: the count-vs-existence mutations above show
the name-keyed walker does distinguish "a boundary renders" from "the right
number of boundaries render," which is the property actually contracted.

## 6. REMOVED/ADDED pair check

Not applicable. `openspec/changes/ui-remaining-screens/specs/` contains only
one file, one `## ADDED Requirements` section, no `## REMOVED` counterpart
anywhere in the repo (checked by grep for the requirement's own heading text
across `openspec/`). This is a new requirement, not an extraction.

## 7. PLAN.md (origin/main) staleness and shedding

Checked `docs/PLAN.md` at `origin/main` for "separator", "type scale", "19px",
"row title", "hairline", "boundary between rows", and "visual system" — no
hits beyond unrelated uses of "boundary" and "separator" (crypto domain
separators, module boundaries). PLAN.md never tracked the row-boundary or
row-title-size question at the level of detail this delta specifies, so there
is nothing here for the spec to have gone stale against, and nothing PLAN.md
needs to strike or shed as a result of this change landing.
