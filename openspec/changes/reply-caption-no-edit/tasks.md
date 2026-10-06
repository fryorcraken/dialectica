## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Tests first

- [x] 1.1 In `dialectica-ui/tests/tst_thread_reply.qml`, add an edit-claim
      matcher and a test of it in both directions; verify with
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_thread_reply.qml`,
      and verify a `return false` matcher turns that test red
- [x] 1.2 Give the caption in `dialectica-ui/src/qml/DThreadScreen.qml`
      `objectName: "replyCaption"`, leaving its text unchanged, so the tests
      below can find it
- [x] 1.3 Add one test per scenario of "The text around the reply composer
      does not promise that a reply can be edited" (open composer, after a
      publish, shut gate). Each asserts that its walk found text before it
      asserts there is no claim. Verify the open-composer and after-publish
      tests fail on the old caption, on the claim assertion
- [x] 1.4 Add the `NO SPEC:` test that the caption is kept and is not empty

## 2. The fix

- [x] 2.1 Cut the caption to "A reply is a signed record." and rewrite the
      comment above it; verify task 1's tests pass
- [x] 2.2 Prove the shut-gate test can fail: temporarily add an edit claim to
      the shut gate, watch that test fail, revert

## 3. Gates

- [x] 3.1 Whole QML suite green: `sh dialectica-ui/tests/run-qml-tests.sh`
- [x] 3.2 Static QML gates green, each after its own test:
      `check_qml_names.py`, `check_qml_reachable.py`, `check_qml_members.sh`
- [x] 3.3 `openspec validate reply-caption-no-edit --strict` passes
