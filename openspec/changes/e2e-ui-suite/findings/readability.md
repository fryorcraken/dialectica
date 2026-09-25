# Readability review — `e2e-ui-suite`

Scope: readability only (comments, docstrings, naming, prose, one-job-per-function,
data-vs-logic shape). Correctness, security and architecture are separate reviewers'
rows.

## Findings

- [ ] **`dev-writer`** — `openspec/changes/e2e-ui-suite/tasks.md:31` — task 1.1's
      "measured" count is stale by a factor that changed under it
      **Scenario:** tasks.md 1.1 reads "disabling the step-count branch turns
      exactly three of its checks red." That was true only before the tester's
      pass added the "more steps than the spec" case. Today, in both
      `dialectica-ui/tests/adjudicate-ui-run.py`'s own docstring and design.md D1,
      the number is **five**, and design.md D1 explains why: "The count moved from
      three to five when the tester added the 'more steps' case." I traced the
      current `tst_adjudicate_ui_run.py` by hand against the count-check removed
      (the direct mutation was blocked by this session's permission classifier as
      a "Security Test Removal," so I verified by inspection rather than by
      running it): removing `len(steps) != expected` turns red both checks of
      "a run that stopped early fails" (test 2), both checks of "a report with
      MORE steps than the spec fails too" (test 3), and "reports the count" in
      "every failing condition is reported" (test 6) — 2+2+1 = 5, matching
      design.md D1 and the script's own docstring, not tasks.md 1.1's "three."
      A reader who trusts tasks.md 1.1 as the current ground truth (rather than
      design.md, which correctly narrates the history) is told a number the rest
      of the change has already superseded — exactly the "prune as you go" failure
      CLAUDE.md names: the surrounding claim (the exact count) stopped holding when
      the tester's box above it changed, and 1.1 was never revisited.
      **Fix is a one-line edit**: change "exactly three" to "exactly five" in
      tasks.md 1.1, or drop the number and point at design.md D1 the way 1.1
      already does parenthetically.
      **Severity:** low — it is a stale number in a task checklist, not in the
      shipped code or the design record itself (which is already correct), and it
      does not affect what merges. It is worth an unticked box because it is
      exactly the "measured claim in a comment" pattern this review is asked to
      run down, and it is checkable and wrong today.

## What I checked and found clean

- **`Main.qml`'s five new `readonly property` handles** (`listReadState`,
  `stoaCount`, `pasteFailure`, `joinState`, `joinFailure`) and their comment block:
  every name resolves to a real property on the screen it claims to project
  (`DStoaListScreen.readState`/`.visibleRows`/`.pasteFailure`,
  `DJoinScreen.joinState`/`.failure` — checked by `grep` against both files), the
  comment's claim that renaming one breaks a spec is true by construction (the
  spec reads exactly these names off `root.`), and the comment matches the file's
  existing conventions (section header style, bold-emphasis convention) rather
  than introducing a new one.
- **`tst_e2e_handles.qml`**: mutated `listReadState` to a fixed `"ok"` and ran it
  with `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_e2e_handles.qml`
  — `test_a_failed_listing_is_not_read_as_ok` went red as the docstring claims a
  constant handle would, mutation reverted, working tree clean afterward. The
  fixture rationale in the file's header (why each case is input-dependent) reads
  true against its own bodies.
- **`adjudicate-ui-run.py` / `tst_adjudicate_ui_run.py`**: docstrings are precise
  about what is and is not proved (exit code vs. report, why the count direction
  cuts both ways), and the "five checks" claim traces correctly by hand (see the
  finding above for why I didn't run the actual mutation). No dead prose, no
  restating-the-code comments — each comment earns its place by saying why, not
  what.
- **`tst_scaffold_values_unchanged.py`**: docstring is explicit and accurate about
  the boundary of what it proves ("this file does not attempt [proving `lgs`
  itself never rewrites a value]"), which matches tasks.md 2.3 and design.md's
  Risks section — no overclaiming.
- **`join.yaml`**: 14 `- name:` steps, matching the "14 steps" figure repeated in
  design.md D5, tasks.md 3.1/4.1 and `validate-ui-specs.mjs`'s own report — counted
  directly rather than trusted. Comments explain the *why* of each omission
  (no `calls:`, no wording pins, no row-count assertions) rather than restating
  the step.
- **`ci.yml`'s updated header/build-job/"Not yet a job" comments**: the "lint job
  asserts the entry's role" claim in the "NOTHING INSTALLS" replacement text
  checks out — `ci.yml:479-482` does assert
  `[modules.delivery_module].role == "dependency"`. No stale claims reintroduced.
- **`check_qml_reachable.py`**'s one-line docstring update (pointing at
  `ui-tests.yml`/`tests/ui/` instead of the stalled `piece/e2e-sitometres`) is
  accurate and minimal.
- **`ui-tests.yml` and `validate-ui-specs.mjs`**: comments are dense but each one
  is a "why, not what" note in the house style (e.g. the apt-sources aside, the
  "count is sound in both directions" matrix-vs-directory argument); none read as
  filler or as restating the adjacent line.
- **`design.md` and `proposal.md`**: internally consistent citations (the D6
  anchor text in `Main.qml`'s comment matches design.md's own heading; the sha
  and run-number citations in D1/D7 resolve to a real, matching Actions run,
  spot-checked with `gh run view 36090846720`).

No typos or naming inconsistencies found in a targeted grep across the new/changed
prose files.
