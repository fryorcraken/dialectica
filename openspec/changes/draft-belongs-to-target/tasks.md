## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

Every QML command below is run from the repository root. "The spec" is
`dialectica-ui/tests/tst_draft_targets.qml`, run with
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_draft_targets.qml`.

## 1. Pin the contract before changing the composer

- [x] 1.1 Rewrite the two `NO SPEC` tests in
      `dialectica-ui/tests/tst_publish_outcome_visits.qml`: the same-Stoa one
      keeps its assertion and loses its marker; the cross-Stoa one flips to
      "B's field is empty, nothing B's feed offers publishes A's text, and
      reopening A shows the draft and publishes it to A". Verified by watching
      the flipped test fail against the unchanged composer.
- [x] 1.2 Add the spec, driving `Main.qml` through a fake forum whose gate
      answer depends on the Stoa asked about and whose thread read echoes the
      thread asked for. One section per requirement of the delta. Verified by
      running it against the unchanged composer and reading which tests fail.

## 2. A draft belongs to its target

- [x] 2.1 In `DComposer.qml`, add `targetKeyOf`, the `targetKey` binding and
      `heldDrafts` with `heldDraft` / `holdDraft` (design.md, Decisions 1, 2
      and 5). Verified with 2.2.
- [x] 2.2 Re-fill the field on a change of `targetKey`, and hold every edit
      under the current key from the field's `onTextChanged` (Decision 3).
      Verified by the spec and `tst_publish_outcome_visits.qml` passing, and by
      emptying the handler and watching the cross-target tests go red.
- [x] 2.3 Clear the draft under the key the publish named: `submit()` reads the
      key before the call and `applyReply` passes it to `clearDraftOf`
      (Decision 4). Verified by
      `test_a_publish_answered_after_the_composer_was_re_pointed_clears_only_what_it_named`,
      which goes red when `clearDraftOf` empties the field unconditionally.
- [x] 2.4 Bring `DComposer.qml`'s comments on `draft` and `clearOutcome` in
      line: the field is no longer the only place a draft lives, and the
      draft's fate is no longer open. Verified by reading the file.

## 3. Requirements that hold without new code

Each is pinned by a test in the spec that also passes against the unchanged
composer, so the test guards the behaviour and does not show this change made
it.

- [x] 3.1 *A draft whose composer is not rendered stays held and is displayed
      nowhere*, for the same target: the composer stays mounted and its screen's
      gate stops rendering it, as before. Pinned by the two
      `…_behind_a_shut_gate_is_held_unseen_and_comes_back` tests and
      `test_a_draft_is_back_when_a_failed_read_recovers`.
- [x] 3.2 *An unsubmitted draft is held by the view alone and ends with it*:
      satisfied by construction. `heldDrafts` is a property of a composer, which
      lives as long as its `Main.qml`, and the only expressions that pass draft
      text to `Core` are the two publish calls in `submit()`. Pinned by
      `test_an_unsubmitted_draft_reaches_no_core_call` and
      `test_a_view_opened_anew_holds_no_draft`.
- [x] 3.3 *A restored draft is not announced*: satisfied by construction,
      nothing was added that displays anything. Pinned by the three tests of
      that section. The warning and over-length tests fail against the
      unchanged composer, at their assertion about the other Stoa's composer;
      `test_a_restored_draft_looks_like_the_same_text_typed_afresh` passes
      against it.

## 4. Gates

- [x] 4.1 The whole QML suite: `sh dialectica-ui/tests/run-qml-tests.sh` exits 0.
- [x] 4.2 The static QML gates the overlay's "Test layers" names, test first:
      `tst_check_qml_names.py` then `check_qml_names.py dialectica-ui`;
      `tst_check_qml_reachable.py` then `check_qml_reachable.py dialectica-ui`;
      `tst_check_qml_members.sh` then `check_qml_members.sh`. All six exit 0.
- [x] 4.3 `qmllint --unqualified disable -I dialectica-ui/src/qml` on
      `DComposer.qml`, as `ci.yml`'s `qmllint` step runs it: no output, exit 0.

## Workflow follow-up

- The end-to-end UI specs run only in `.github/workflows/ui-tests.yml`, on the
  PR. None was added or changed.
