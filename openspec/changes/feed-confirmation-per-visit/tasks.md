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
      removing the visit hook, which restores the behaviour from before the
      fix. Every absence test then fails at its absence assertion, after its
      presence assertion has passed. design.md Decision 1 names each test.

## 2. Fix

- [x] 2.1 Add `DComposer.clearOutcome()`, which resets `outcome` and
      `outcomeDetail` together and leaves the draft alone (Decisions 2 and 3).
      The draft half is pinned by the two `NO SPEC:` draft tests. Resetting
      `outcome` is pinned by every absence test. Resetting `outcomeDetail` as
      well is satisfied by construction, and no test can show it. Every writer
      of `outcome` also writes the detail, and the detail renders only while the
      outcome is "refused", so the suite stays green without it (Decision 3).
- [x] 2.2 Add `beginVisit()` to `FeedScreen` and `DThreadScreen`, each clearing
      its own composer's outcome.
- [x] 2.3 Call them from `Main.qml`'s `onScreenShownChanged` (Decision 1).
      Verified by `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_publish_outcome_visits.qml` going green. Each
      branch is load-bearing on its own. Removing the `feed` branch reddens the
      feed's absence tests, and removing the `thread` branch reddens the reply
      composer's (Decision 1 names both sets).
- [x] 2.4 Check that the outcome survives the re-read within its visit
      (Decision 5). Verified by making `FeedScreen.reload()`, and separately
      `DThreadScreen.reload()`, clear the outcome. Each turns red every test
      for its composer that publishes and then asserts the outcome displayed,
      `test_the_outcome_stays_across_the_re_read_that_follows_the_publish` among
      the feed's.

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
