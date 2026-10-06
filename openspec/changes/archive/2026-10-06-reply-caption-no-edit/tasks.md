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
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
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
      says nothing about editing a reply or reading its earlier versions"
      (open composer, after a publish, shut gate). Each asserts that its walk
      found text before it asserts there is no claim. Verify the
      open-composer and after-publish tests fail on the old caption, on the
      claim assertion
- [x] 1.4 Add tests that the text rendered with the open composer states that a
      reply is signed, before a publish and after one

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

## 4. Review findings (`dev-writer`'s)

- [x] 4.1 Rewrite `design.md`'s passages that said the spec permits a denial,
      and record why a denial is forbidden (Decision 4)
- [x] 4.2 Point Decisions 1 and 3 at *The open reply composer states that a
      reply is signed* in place of the `NO SPEC:` marker
- [x] 4.3 Record in Decision 3 that the walk ignores `visible`, and that it
      collects core-supplied text and the draft the requirement does not bind
- [x] 4.4 Rewrite the caption's comment in `DThreadScreen.qml` as the standing
      rule, citing `thread-view`, with no history and no pointer into this
      change's folder; verify
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_thread_reply.qml`
      and the static QML gates stay green
