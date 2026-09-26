# Tasks

## Stages

- [x] spec — `spec-writer`
- [ ] design + code — `dev-writer`
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

- [ ] 4.1 Break (design.md D6): `Main.qml`'s `onJoined` without
      `list.reload()`, pushed alone. Predicted: `sitometres seeded-join spec`
      red on step 11, "the joined Stoa is listed without a restart, and can be
      shared", after its 30s, steps 12–14 `inconclusive`, `verdict: fail`;
      steps 1–10 passing, the join among them; the other five UI jobs green.
      `QML lint` red on `tst_navigation.qml`'s
      `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` alone,
      measured locally over the four files that drive a join (the other
      three, `tst_stoa_screens.qml`, `tst_e2e_handles.qml` and
      `tst_render_probe.qml`, green with the break)
- [ ] 4.2 Revert pushed; both workflows green on it

### 5. Hand-back

- [ ] 5.1 No break in the branch's net diff: `Main.qml` unchanged against the
      fork point, and the tip is the revert or a documentation commit on it
