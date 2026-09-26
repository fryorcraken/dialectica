# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer` — design.md D1–D6; `seeded-join.yaml`
      green in CI and seen red for its reason (Implementation 3–4); the
      committed reference checked against the core by `seeded_reference.rs`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

The specification is proven in CI and nowhere else: the owner's rules keep
sitometres and a directly launched Basecamp out of local hands (the
`e2e-ui-suite` change's design.md, Risks). Every box in 3–4 names the run that
ticked it and the head SHA it ran on; design.md D6 says why each run is waited
out before the next push.

### 1. The seeder and its check

- [x] 1.1 `dialectica-core/tests/seeded_reference.rs` (design.md D1, D2):
      `seeded_record()` from a fixed seed and title, and three tests reading
      the committed reference out of `seeded-join.yaml` and putting it through
      `Genesis::decode`, `get_stoa` and `join_stoa`. Written first and run with
      no spec present: all three red on "cannot read the seeded join spec",
      each printing the reference the core writes, which is the literal the
      spec commits. Green once the spec existed
- [x] 1.2 Seen red for what it guards, each mutation applied alone and
      reverted: the committed address's first digit changed turns all three
      red (the record test on "not the hash of the committed record", the
      other two on the core's refusal); `VERSION_1` set to 2 turns all three
      red on "unknown genesis record version 1", beside `stoa.rs`'s
      known-answer test. `cargo clippy -p dialectica-core --all-targets -D
      warnings` clean, and the file is `rustfmt`-clean

### 2. The specification, the view's one name, and the matrix

- [x] 2.1 `joinCancelButton` on the join screen's Cancel (design.md D4).
      `tst_navigation.qml`'s
      `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` presses it
      after a successful join against a fake whose listing answers from
      whether a join was made: green, and red with `Main.qml`'s `onJoined`
      reload removed ("the listing is read again once the join has
      succeeded"), the only test in that file to go red.
      `tst_stoa_screens.qml` green (136)
- [x] 2.2 `dialectica-ui/tests/ui/seeded-join.yaml`, 14 steps (design.md D3,
      D4). Parses with `yq`, and `yq` reads the paste step's text as the same
      string the Rust check reads; the sitometres schema check is `ui-specs`
      in CI (3.1)
- [x] 2.3 `ui-tests.yml`'s matrix gains `seeded-join`; its "every spec in the
      tree is in the matrix" count follows from `strategy.job-total`.
      `join.yaml`'s "does NOT cover" paragraph points at the new spec

### 3. Green in CI

- [x] 3.1 Predicted: every `ci.yml` job green, `UI spec validation` printing
      `seeded-join.yaml: ok (14 steps)` beside the other five, and `Rust core
      tests` running the three seeded tests; all six `ui-tests.yml` jobs
      green, `seeded-join`'s adjudicator printing `verdict: pass` and `ok: all
      14 steps passed`

      **Observed, on head `8c098d7`, as predicted.** UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36230185986 green
      in all six jobs. `sitometres seeded-join spec` printed `14 passed in
      9.7s`, and its adjudicator printed `verdict: pass`, fourteen `[pass]`
      and `ok: all 14 steps passed`. The preview step saw "A Stoa seeded
      outside this profile" and `fallbackNote`, with `joinState ===
      'previewing'` (1.0s). The join step logged `calls: dialectica.join_stoa,
      dialectica.list_stoas` and saw `joinedPanel` and not `joinFailurePanel`.
      "go back to the list" clicked `"Cancel"`, and the next step held
      `listedStoas.length === 1` and `listedStoas[0] === 'a4b3e43d…'`, with
      `shareButton` seen. The feed step held `feedReadState === 'ok'` and
      `feedRowCount === 0`. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36230185961 green
      in every job (`Release` skipped). `UI spec validation` printed
      `seeded-join.yaml: ok (14 steps)` among `ok: 6 spec(s) parsed`.
      `Rust core tests` ran `tests/seeded_reference.rs`, 3 passed, and its
      count gate printed `ok: all 1213 declared tests ran and passed`

### 4. `seeded-join.yaml` goes red when the joined Stoa does not reach the list

- [x] 4.1 Break (design.md D6): `Main.qml`'s `onJoined` without
      `list.reload()`, pushed alone. Predicted: `sitometres seeded-join spec`
      red on step 11, "the joined Stoa is listed without a restart, and can be
      shared", after its 30s, steps 12–14 `inconclusive`, `verdict: fail`;
      steps 1–10 passing, the join among them; the other five UI jobs green.
      `QML lint` red on `tst_navigation.qml`'s
      `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` alone,
      measured locally over the four files that drive a join (the other
      three, `tst_stoa_screens.qml`, `tst_e2e_handles.qml` and
      `tst_render_probe.qml`, green with the break)

      **Observed, on head `661ac18`, the break pushed alone, as predicted.**
      UI tests https://github.com/fryorcraken/dialectica/actions/runs/36238223919:
      `sitometres seeded-join spec` failed on step 11, "the joined Stoa is
      listed without a restart, and can be shared", after 30.0s, `waitFor
      never came true within 30000ms` on `sees {"objectName":"shareButton"}`,
      `root.listedStoas.length === 1` and `root.listedStoas[0] ===
      'a4b3e43d…'`, each evaluated to false. The step's other two conditions,
      `screenShown === 'list'` and `listReadState === 'ok'`, were not among
      them. sitometres printed `10 passed, 1 failed, 3 inconclusive` ("3
      later step(s) were not attempted"), and the adjudicator printed
      `verdict: fail`, ten `[pass]`, one `[fail]` and three
      `[inconclusive]`. Steps 1–10 passed, the join among them. **The call
      log shows the break itself:** step 9 logged `calls: dialectica.join_stoa`
      alone, where the green run (3.1) logged `dialectica.join_stoa,
      dialectica.list_stoas`. `join`, `create`, `feed`, `thread` and
      `moderation` green. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36238223969 red
      in `QML lint` only, on its "QML component tests" step:
      `tst_navigation.qml` 26 passed, 1 failed, the one predicted
      (`test_a_joined_stoa_is_listed_once_the_join_screen_is_left`, "the
      listing is read again once the join has succeeded", line 855);
      `tst_stoa_screens.qml` 136 passed and `tst_render_probe.qml` 11 passed
      on CI's Qt 6.8.3. `Lint`, `UI spec validation`, `Rust core tests` and
      `Build LGX` green; `Release` skipped
- [x] 4.2 Revert pushed; both workflows green on it. **Observed, on head
      `431e722`** (the revert `ba383bc` plus the 4.1 record), as predicted:
      UI tests https://github.com/fryorcraken/dialectica/actions/runs/36238723726
      green in all six jobs, `sitometres seeded-join spec` printing `14
      passed in 9.7s`, `verdict: pass` and `ok: all 14 steps passed`, with
      the join step again logging `calls: dialectica.join_stoa,
      dialectica.list_stoas`; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36238723730
      green in every job (`Release` skipped), `tst_navigation.qml`'s
      `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` passing in
      `QML lint`

### 5. Hand-back

- [x] 5.1 No break in the branch's net diff: `git diff 8c098d7 HEAD --
      dialectica-ui/src/qml/Main.qml` is empty, and `Main.qml` is not in the
      branch's diff against its fork point `b20e8f4`. The tip is a
      documentation commit on top of the 4.2 revert. Its own CI state is
      reported in the hand-back and on the PR, because this file cannot name
      a run of the commit that contains it
