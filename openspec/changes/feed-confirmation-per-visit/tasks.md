## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Reproduce

- [x] 1.1 Give `DComposer`'s outcome element a per-kind `objectName`
      (`postOutcomeMessage` / `replyOutcomeMessage`), a handle with no behaviour
      change, so a test can ask whether the outcome is displayed. Verified by
      `check_qml_names.py` accepting the tree (design.md Decision 4).
- [x] 1.2 Write `dialectica-ui/tests/tst_publish_outcome_visits.qml`, driving
      `Main.qml` through each of the requirement's scenarios. Verified by
      watching the six absence tests fail before the fix, each after its
      presence assertion passed.

## 2. Fix

- [x] 2.1 Add `DComposer.clearOutcome()`, which resets `outcome` and
      `outcomeDetail` together and leaves the draft alone (Decisions 2 and 3).
- [x] 2.2 Add `beginVisit()` to `FeedScreen` and `DThreadScreen`, each clearing
      its own composer's outcome.
- [x] 2.3 Call them from `Main.qml`'s `onScreenShownChanged` (Decision 1).
      Verified by `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_publish_outcome_visits.qml` going green. Removing
      the `thread` branch turns exactly the reply test red.
- [x] 2.4 Check that the outcome survives the re-read within its visit
      (Decision 5). Verified by mutating `FeedScreen.reload()` to clear the
      outcome, which turns seven tests red, including
      `test_the_outcome_stays_across_the_re_read_that_follows_the_publish`.

## 3. Leave the undecided visible

- [x] 3.1 Pin today's draft behaviour under `NO SPEC:`: a draft survives a
      reopen of the same Stoa, and a draft typed in one Stoa is published to
      another when submitted there (design.md Decision 2, routed to the
      `spec-writer`).

## 4. Gates

- [x] 4.1 `sh dialectica-ui/tests/run-qml-tests.sh` (whole suite) exits 0.
- [x] 4.2 The static QML gates pass, each test first:
      `tst_check_qml_names.py` / `check_qml_names.py`,
      `tst_check_qml_reachable.py` / `check_qml_reachable.py`,
      `tst_check_qml_members.sh` / `check_qml_members.sh`.
- [x] 4.3 No Rust is touched, so the Rust layer and `nix build ./dialectica#lgx`
      cannot see this change and were not run.
