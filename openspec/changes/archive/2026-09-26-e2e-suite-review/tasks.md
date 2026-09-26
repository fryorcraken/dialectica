# Tasks

## Stages

- [x] spec — `spec-writer` — `proposal.md`; one delta, `specs/view-navigation/spec.md` (two requirements added, from findings/spec-test.md); `skip_specs` dropped
- [x] design + code — `dev-writer` — design.md written; CLAUDE.md line; both proofs red then green in CI (Implementation 2 and 3); review findings fixed (Implementation 5, design.md D6–D10)
- [x] tests — `tester` — `tst_e2e_handles.qml`: table test for the opening
      screen (empty listing/no key held, failed listing, no bridge — the
      fourth case, failed key query, was already pinned in
      `tst_stoa_screens.qml`) checking both `screenShown` and each mounted
      screen's own `visible`; table test extending the paste-refusal case to
      the missing-`genesis` JSON shape. Both proved able to fail by mutation,
      restored. Settled the `NO SPEC:` marker-count mismatch between
      `tasks.md` 5.2/`correctness.md` and the file: corrected the wording,
      the code was already right (findings/correctness.md, tasks.md 5.2)
- [x] review: correctness — `code-reviewer` — findings/correctness.md: two findings for `dev-writer` (adjudicate-ui-run.sh's missing-report cause, and its uncaught no-steps crash), rest checked clean
- [x] review: security — `code-reviewer` — findings/security.md: one low-severity finding (unquoted `${{ matrix.spec }}` in ui-tests.yml run: bodies); rest clean
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer` — one finding in
      findings/spec-test.md
- [x] review: design — `design-reviewer` — decisions match the code; one
      finding in findings/design-review.md (adjudicator's missing-report
      diagnosis is wrong when an earlier step stopped the job, per design.md
      D1's own account)
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

The two proofs are CI runs and nothing local can stand in for them: the owner's
rules keep sitometres and a directly launched Basecamp, and `lgs basecamp
setup`/`install`, out of local hands. Every box in 2 and 3 names the run that
ticked it and the head SHA it ran on. design.md D3 says why each run is waited
out before the next push.

### 1. The owner's rule, and the archive's claims it rests on

- [x] 1.1 CLAUDE.md names `yq` for YAML and `jq` for JSON, never Python, in
      "When you forbid a tool, name the replacement" (design.md D4). Verified
      by reading the diff: one entry, phrased by the approval click it costs
- [x] 1.2 No Python in the suite parses YAML or JSON: `git grep -n -i -e python
      -e "import yaml" -e "json.load"` over the files proposal part 1 lists
      returns only `check_qml_reachable.py`'s shebang, and that file reads QML
      (design.md D5)
- [x] 1.3 Archived D12's reasons for keeping `validate-ui-specs.mjs` re-read at
      the pinned tag `v0.1.2` (`6dc23e2`) rather than the fork commit the
      archive cites; all hold, and the citation is corrected in design.md D5

### 2. The no-key step goes red with the create affordance present

- [x] 2.1 Break: `createBlockLoader`'s `active:` set to `true` in
      `DStoaListScreen.qml`, pushed alone. Predicted (design.md D2): the
      `sitometres join spec` job fails; the adjudicator prints `verdict: fail`,
      four `[pass]`, one `[fail]` on "a fresh profile is offered a key and not
      a Stoa", and three errors (verdict, `report has 5 steps, spec has 14`,
      the step by name). ci.yml's `QML lint` job also red, on exactly the three
      `tst_stoa_screens.qml` tests the break reddened locally (Qt 6.10.3,
      133 passed, 3 failed; `tst_navigation.qml` and `tst_render_probe.qml`,
      the other two specs that instantiate the screen, stayed green):
      `test_the_create_affordance_is_not_instantiated_when_no_key_is_held`,
      `test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words`
      and `test_a_reply_claiming_a_key_without_naming_one_is_not_the_key_held_state`

      **Observed, on head `53930e4`, the break pushed alone:**
      UI tests https://github.com/fryorcraken/dialectica/actions/runs/36140112191
      failed. "Run the spec" exited 1 on step 5, "a fresh profile is offered a
      key and not a Stoa", after its 30s: `waitFor never came true within
      30000ms`, with `does not see "{\"objectName\":\"createTitleField\"}" —
      still visible on QQuickTextInput` and the same for `createStoaButton` on
      `FlatButton_QMLTYPE_135`. Steps 1–4 passed. The adjudicator printed
      `verdict: fail`, four `[pass]`, one `[fail]` on that step, and nine
      `[inconclusive]`, then two errors: the verdict, and "steps that did not
      pass" naming step 5 and the nine after it. "lgs left scaffold.toml's
      values alone" was green, so the guard did not stand in front of it.
      **The prediction was wrong about the count.** sitometres 0.1.2 did not
      end the report at the failed step: it printed `9 later step(s) were not
      attempted` and wrote them into the report as `inconclusive`, so the
      report held 14 steps, the count condition held, and no count error was
      printed. The run is red on conditions 1 and 2 (design.md D2).
      CI https://github.com/fryorcraken/dialectica/actions/runs/36140112290
      failed in `QML lint` only, on "QML component tests": `tst_stoa_screens.qml`
      133 passed, 3 failed, the three named above, every other spec green, as
      predicted. `Lint`, `UI spec validation`, `Rust core tests` and
      `Build LGX` green
- [x] 2.2 Revert pushed; both workflows green on it. **Observed, on head
      `785df81`** (the pull_request merge `279d10a` onto `main` at `8368b2f`,
      the same base as 2.1): UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36141018728
      green, the adjudicator printing `verdict: pass` and `ok: all 14 steps
      passed`, the guard `ok: scaffold.toml's values are unchanged`; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36141018879
      green in every job

### 3. The scaffold guard goes red on a value `lgs` itself changed

- [x] 3.1 Break: an unknown key under `[modules.dialectica_ui]` in
      `scaffold.toml`, pushed alone. Predicted (design.md D1): "lgs left
      scaffold.toml's values alone" exits 1 with its `::error::`, its diff
      showing that key present before and absent after, and the job stops
      there, before sitometres. ci.yml green. Answers: `lgs` 0.3.1's own
      rewrite drops a key it does not know, and the real step in CI reports it
      as a changed value

      **Observed, on head `d3d24e2`, the break pushed alone:** UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36142050293
      failed. `lgs basecamp setup` and `lgs basecamp install` both ran green,
      then "lgs left scaffold.toml's values alone" printed a one-hunk diff
      whose only change is `-      "e2e_suite_review_probe": "a key lgs does
      not know; setup's rewrite should drop it",` under `dialectica_ui`, and
      `::error::an lgs basecamp verb changed a value in scaffold.toml — see the
      diff above`. "Locate", "The build really has the QML inspector" and "Run
      the spec" were skipped, so sitometres never started. As predicted.
      **The record answers the second question:** the value that differed was
      one `lgs`'s own rewrite changed, since no step of the job writes
      `scaffold.toml` between the snapshots. It therefore also answers the
      first, whether the real step fires in CI on a changed value. It does not
      say which of the two verbs dropped the key (design.md D1).
      **Not predicted:** "The run proved what the spec asks", which runs on
      `always()`, also failed, with `no JSON report — sitometres was killed
      before it could write one (job timeout?), so nothing was proved`. The
      exit is right and the stated cause is wrong: sitometres was never
      started. Reported as a finding (design.md D1), not changed here.
      CI https://github.com/fryorcraken/dialectica/actions/runs/36142050127
      green in every job, as predicted, including `Lint`'s
      "scaffold.toml kept the values a lgs verb can silently rewrite"
- [x] 3.2 Revert pushed; both workflows green on it. **Observed, on head
      `c4d8f04`** (the pull_request merge `ad0e9f5` onto `main` at `8368b2f`,
      the same base as 3.1): UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36142854131
      green, the guard printing `ok: scaffold.toml's values are unchanged` and
      the adjudicator `verdict: pass`, `ok: all 14 steps passed`; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36142854166
      green in every job

### 4. Hand-back

- [x] 4.1 Neither break is in the branch's net diff: `git diff --stat 8368b2f
      HEAD -- scaffold.toml dialectica-ui/src/qml/DStoaListScreen.qml` prints
      nothing. The tip is this record, a documentation-only commit on top of
      the 3.2 revert; its own CI state is reported in the hand-back and on the
      PR, not here, because this file cannot name a run of the commit that
      contains it

### 5. Review findings addressed to `dev-writer`

Each finding's box, outcome and predicted-versus-observed measurement is in
`findings/`; these rows are the ordering.

- [x] 5.1 The adjudicator names both causes of a missing report
      (`correctness.md` 1, `design-review.md`; design.md D6).
      `tst_adjudicate_ui_run.sh` "names a run that never started" was red
      before the change and is green after it
- [x] 5.2 A spec with no, a non-list or an unparseable `steps:` is a reported
      problem, not a jq abort (`correctness.md` 2; design.md D7). Three new
      cases in `tst_adjudicate_ui_run.sh`, under the one `NO SPEC:` marker
      that `proposal.md`'s markers section already describes as covering all
      three (not three separate marker lines — checked against the file by
      `tester`, which found two `NO SPEC:` lines total, not three), and two
      mutations measured
- [x] 5.3 No `${{ … }}` in any workflow's `run:` body (`security.md`;
      design.md D8). `tst_workflow_run_bodies.sh` wired into `ui-specs`, red
      against the workflows #164 left and green after
- [x] 5.4 One `install-yq.sh` for both workflows (`architecture.md`; design.md
      D9). Satisfied by construction: one copy cannot drift, and no local test
      can run it. CI on the pushed tip is the evidence
- [x] 5.5 Decision citations in the suite's files name their change
      (`readability.md`; design.md D10). Prose, checked by `git grep`, and no
      test can see it
- [x] 5.6 design.md D5 records the correction to the archived proposal's
      "every behaviour `join.yaml` asserts is already a requirement", which
      the `view-navigation` delta makes true, and to archived D1's premise
      for the missing-report message
