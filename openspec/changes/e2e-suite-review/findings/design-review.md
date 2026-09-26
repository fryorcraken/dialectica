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

- [ ] **`dev-writer`** — `dialectica-ui/tests/adjudicate-ui-run.sh:47-49`'s
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
