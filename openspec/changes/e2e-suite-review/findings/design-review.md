# Design review — e2e-suite-review

Checked this change's `design.md` (D1–D5) against the code as merged at
`8368b2f` plus this piece's own commits (`ee8e145`), against the archived
`openspec/changes/archive/2026-09-25-e2e-ui-suite/design.md`, and against
`gh issue view 134` and `gh pr view 172` (read fresh, not from the brief).

**Overall: the decisions are in good shape.** Every one of D1–D5 has what,
why, alternatives and cost, and D1–D3 (the two CI proofs and their pacing)
carry the observed run as mutation evidence rather than a prediction taken on
faith — checked against `tasks.md`'s run URLs and the actual scripts:

- D1's mechanism (`[modules.dialectica_ui]`'s unknown key, dropped by `lgs`'s
  own rewrite) matches `scaffold.toml`'s guard steps in `ui-tests.yml:263-271`
  and the run it cites (36142050293/36142854131).
- D2's break (`createBlockLoader.active` set to `true`) is the same guard
  gating the create block in `DStoaListScreen.qml:919`
  (`active: screen.machineKey.state === "held"`), and `join.yaml`'s step 5
  (`join.yaml:123-128`) asserts exactly what D2 says it does.
- D4's CLAUDE.md line (`CLAUDE.md:165-168`) sits where D4 says, in "When you
  forbid a tool, name the replacement," phrased by approval-click cost as
  that section's other entries are.
- D5's citation correction is real: the archive cites
  `paradoxcomputer/sitometres ab6b3ea` at `design.md:131` and `:506`
  (confirmed by `grep`), a fork commit, not the pinned `v0.1.2` both
  workflows install. D5's dash/bash correction is also confirmed: every
  script in `dialectica-ui/tests/*.sh` this piece touches has
  `#!/usr/bin/env sh`, and `ci.yml`/`ui-tests.yml` invoke each by path rather
  than `bash <path>`, so the real interpreter is the runner's `sh` (dash),
  not the job's `shell: bash`.
- The yq/jq rule (D12, confirmed rather than re-decided by this piece's D5)
  holds in the code: `require-jq-yq.sh`, `tst_adjudicate_ui_run.sh`,
  `tst_ui_tool_pins.sh`, and `tst_scaffold_values_unchanged.sh` all read YAML
  with `yq` and JSON with `jq`; `check_qml_reachable.py`'s only change is a
  docstring, confirmed with `git diff 8368b2f~1 8368b2f --
  dialectica-ui/tests/check_qml_reachable.py`.
- No contradiction with issue #134's stated scope: the issue's "what this
  needs" list (salvage vs. restart, get the run passing, cost/cadence, widen
  coverage) is what #164 already answered; this piece's proposal explicitly
  defers wider coverage to #134's next piece rather than silently narrowing
  it, and the PR body says so.

One finding, on the one place `design.md` itself names as unresolved rather
than fixed:

- [x] **`dev-writer`** — `dialectica-ui/tests/adjudicate-ui-run.sh:47-49`'s
      missing-report diagnosis is wrong whenever an earlier step stops the
      job, and this piece's own `design.md` D1 says so but leaves it
      unfixed ("Reported as a finding... not changed here").
      **Scenario:** the scaffold guard fails (or any step before "Run the
      spec" does) → sitometres never starts → no `report.json` exists → the
      adjudicator prints `"sitometres was killed before it could write one
      (job timeout?), so nothing was proved"`. That is a wrong diagnosis: no
      job timeout occurred and sitometres was never launched at all; the
      guard's own `::error::` already named the real cause two steps
      earlier. **Verified:** the run recorded in `tasks.md` 3.1
      (36142050293) shows exactly this — the guard's diff error, then the
      adjudicator's "killed before it could write one (job timeout?)"
      message on the same run, which design.md D1 itself flags as "Not
      predicted" and "the stated cause is wrong." Since the marker for this
      script is `NO SPEC:` (`tst_adjudicate_ui_run.sh:110`) and this piece's
      proposal explicitly keeps that marker rather than resolving it into a
      spec, the fix is a wording change to the diagnosis (e.g. distinguish
      "no report and the run step itself never started" from "no report and
      the run step started"), not a behaviour change requiring a spec.
      **Fixed** in the commit that flips this box, by the same change that
      answers `correctness.md`'s first finding (outcome and measurement
      there). The script cannot tell the two cases apart, so the message
      names both and points at the step log that can, rather than
      distinguishing them. design.md D6 records why neither way of
      distinguishing them was taken: skipping the step when the run step was
      skipped makes the gate's running depend on another step's `if:`, and
      passing the run's outcome in adds an input only to pick a sentence. D1's
      "reported as a finding, not changed here" now points at D6.

## Round 2 (HEAD 532794e)

Checked D6–D10, the D5 corrections (to archived proposal.md and archived D1),
the `.openspec.yaml`/`proposal.md`/spec delta changes, and CLAUDE.md/the
owner's constraints, against the code as it now stands at `532794e`, against
`gh issue view 134` and `gh pr view 172` read fresh, and against the archived
`e2e-ui-suite/design.md`. Ran the adjudicator's own test suite
(`dialectica-ui/tests/tst_adjudicate_ui_run.sh`) both as committed and under a
reconstruction of D7's rejected alternative (type check replaced by a bare
`length`, combined with removing the `if ! expected=$(…)` guard): got 6 red
checks, matching D7's claimed decomposition exactly (4 from the first change,
2 from the second, measured separately in the change's own history). Also
independently pulled the scaffold-guard red run
(`gh run view 36142050293`) and the current PR tip
(`gh pr checks 172`, all green including `sitometres join spec`).

**No new findings — the decisions are in good shape.** Each of D6–D10 states
what was chosen, the constraint, alternatives with what ruled them out, cost,
and mutation evidence, and each matches the code:

- D6 (`adjudicate-ui-run.sh:51-54`) — the message now names both causes,
  matches `tst_adjudicate_ui_run.sh`'s two new assertions ("names a run that
  never started" / "names a run that was killed"), and the step stays on
  `always()` per the rejected alternatives' reasoning.
- D7 (`adjudicate-ui-run.sh:64-77`, `tst_adjudicate_ui_run.sh`) — the `null`-or-
  `problem()` shape is in the code exactly as described, and the "what breaks
  without it, measured" mutation evidence reproduces (see above).
- D8 (`tst_workflow_run_bodies.sh`, `ui-tests.yml:65-73,232`, `ci.yml:1189-
  1193,1593`) — `REPORT`/`JUNIT` are job-level `env:` in `ui-tests.yml`'s
  `spec` job, both `cargo install` steps read `"$LGS_VERSION"`, and the new
  check globs `.github/workflows/*.yml` rather than naming the two files, as
  D8 says.
- D9 (`install-yq.sh`, called from both workflows with different package
  lists) — one script, `require_jq_yq` run at the end of the install rather
  than deferred to the first caller, matching D9's stated reason.
- D10 (`git grep -n -F "design.md"` over the files D10 names) — every hit in
  those files now qualifies its citation with a change name; the two
  apparently-bare hits in `ui-tests.yml` are the wrapped second line of a
  qualified citation, confirmed by reading the surrounding lines. Bare
  citations outside that file list (`Main.qml:82,102,509`, other repo files)
  are untouched, as D10 says they should be.

D5's corrections (the sitometres fork-commit citation, the dash/bash
correction, and the archived proposal's "every behaviour already a
requirement" claim) all check out against the archive and against the new
`view-navigation` delta, which contracts exactly the two behaviours D5/the
spec-test finding named and no more — confirmed against
`specs/view-navigation/spec.md` and `tst_e2e_handles.qml`'s new table tests.

All six round-1 review findings (`architecture.md`, two in `correctness.md`,
`readability.md`, `security.md`, `spec-test.md`) are ticked, and each ticked
entry's "Fixed" text matches a design.md decision (D6, D7, D8, D9, D10, and
the spec delta) rather than describing a change design.md does not record.
`.openspec.yaml` and `proposal.md` correctly reflect that `skip_specs` was
dropped once the spec delta landed. The owner's constraints all still hold in
this round's diff: no Python touches YAML or JSON (the new
`tst_workflow_run_bodies.sh` and `install-yq.sh` both use `yq`/`jq` only),
nothing added touches the owner's own Basecamp config or does a local
`npm`/`pip install`, `lgs` remains the CI-side builder/installer with no
change to that division, and `openspec/changes/archive/` has no diff since
`ee8e145` (confirmed: `git diff ee8e145...HEAD --stat --
openspec/changes/archive/` is empty).
