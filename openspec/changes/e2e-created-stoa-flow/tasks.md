# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer` — design.md D1–D8; four specs green in CI
      and each seen red for its reason (Implementation 3–7); #152's binding race
      found by `feed.yaml`, fixed test-first (3.2–3.3, design.md D8)
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

The specs are proven in CI and nowhere else: the owner's rules keep sitometres
and a directly launched Basecamp out of local hands (the `e2e-ui-suite`
change's design.md, Risks). Every box in 3–7 names the run that ticked it and
the head SHA it ran on; design.md D6 says why each run is waited out before
the next push.

### 1. Make the change easy, then the view's side of it

- [x] 1.1 `FeedScreen.visibleRows` holds the read-state guard once, and the
      Repeater and the empty state read it (design.md D3). No behaviour
      change: the full QML suite, qmllint and `check_qml_members.sh` are as
      before, in their own commit
- [x] 1.2 Seven read-only root handles on `Main.qml` (design.md D3), pinned in
      `tst_e2e_handles.qml` by three new tests and two extended ones, written
      first and seen red (five failures, each `Actual undefined`), then green.
      Five mutations measured, those design.md D3 lists, each turning exactly
      the test it names red; `feedReadState` and `threadReadState` are direct
      reads of one property each and were not mutated
- [x] 1.3 `objectName`s the specs drive: `openStoaButton`, the kind-derived
      composer names, and `readThreadArea` / `moderateArea` on the
      `MouseArea`s (design.md D4). `tst_composer.qml`'s
      `test_the_field_and_the_submit_are_named_by_kind` green, and red with the
      submit's name fixed at `"postSubmitButton"`
- [x] 1.4 Stale prose in `Main.qml`, `FeedScreen.qml` and `join.yaml` pruned
      (design.md D7)

### 2. The four specifications and the matrix

- [x] 2.1 `create.yaml`, `feed.yaml`, `thread.yaml`, `moderation.yaml` under
      `dialectica-ui/tests/ui/` (design.md D1, D2, D5). Each parses with `yq`;
      the sitometres schema check is `ui-specs` in CI (task 3.1)
- [x] 2.2 `ui-tests.yml`'s matrix is `[join, create, feed, thread,
      moderation]`; its "every spec in the tree is in the matrix" count follows
      from `strategy.job-total`. `tst_workflow_run_bodies.sh` and
      `tst_ui_tool_pins.sh` green locally

### 3. All five specs green in CI

- [ ] 3.1 Predicted: every `ci.yml` job green, `UI spec validation` printing
      `create.yaml: ok (12 steps)`, `feed.yaml: ok (17 steps)`,
      `thread.yaml: ok (21 steps)`, `moderation.yaml: ok (15 steps)`; every
      `ui-tests.yml` job green, each adjudicator printing `verdict: pass` and
      `ok: all N steps passed` with N the count above (14 for `join`). If
      `feed.yaml`'s read step is red with "genesis record ended mid-field",
      that is #152: stop and report, do not weaken the step

      **Observed, on head `34eaa20`: `feed.yaml` red on its central step, and
      stopped here per the brief.** UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36213442819:
      `join`, `create`, `thread` and `moderation` green, the adjudicator
      printing `verdict: pass` and `ok: all 12 steps passed` for `create`,
      `ok: all 21 steps passed` for `thread` and `ok: all 15 steps passed`
      for `moderation`, whose inert-control step took 1.0s, the settle floor
      design.md D5 rests on; `UI spec validation` printed the four step
      counts predicted above; `sitometres feed
      spec` failed on "the feed was read, and holds nothing",
      `root.feedReadState === 'ok'` evaluating to false after its 30s and
      `feedRowCount === 0` passing. Every later step passed: after the post
      was published the re-read was "ok" and held one row. The adjudicator
      printed `verdict: fail`, sixteen `[pass]`, one `[fail]`. (sitometres
      carried on past a failed `expect:`, where `e2e-suite-review` saw it
      stop after a failed `wait_for:`.) The Basecamp log records the two
      `list_threads` calls, at the open and after the publish, but not their
      replies, so the run itself does not name the reason.

      **The cause, found locally at the component layer and not fixed here:**
      `FeedScreen` reloads on `onStoaAddressChanged`, and when `Main.open()`
      sets `chosen`, the feed's `stoaAddress` binding updates before its
      `stoaGenesis` binding. So the first `list_threads` of every open carries
      the previous genesis, which is `""` coming from the list. A throwaway
      probe (not committed) opened a Stoa with record `"00ff"` through
      `Main.open()` and recorded exactly one `list_threads`, whose arguments
      were `{"stoa":"abab…","genesis":"","page":0,"includeHidden":false}`.
      The core decodes `""` as "genesis record ended mid-field". Any later
      read carries the right record, which fits #152's report of an error on
      entering a Stoa that "Try reading again" clears. It is a view defect,
      and `feed.yaml` is right to fail on it. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36213442743:
      `Lint`, `QML lint`, `UI spec validation` and `Rust core tests` green
- [x] 3.2 Runner's routing: fix it in this piece (#152's reproduction). Test
      first: `tst_navigation.qml`'s
      `test_the_feed_is_read_with_the_record_on_every_route_onto_it` and
      `test_the_thread_is_read_with_the_record_when_it_is_opened`, against a
      fake that answers a read only when it carries the chosen Stoa's record,
      checking every read. **Red against the unfixed code** on each of the
      feed's three routes, run one at a time (25 passed, 1 failed each): "opened from the
      list", "back from a thread" and "back from moderation", each with the
      fake's refusal of `''`. The thread test passed against the unfixed code
      only through binding order (design.md D8)
- [x] 3.3 Fix (design.md D8): `Main.qml` withholds the feed's address until
      its record has landed, and the thread's id until its address and record
      have. The first fix tried, re-reading on both halves, was replaced when
      the every-read check went red on it ("read 1 of 2 carried the
      record"). Green: 568 component tests with the bindings in the committed
      order and in reversed order; removing the feed's guard turns exactly the
      feed test red; the thread's guard, removed with `threadId` bound first,
      turns three tests red. qmllint and `check_qml_members.sh` clean. No spec
      change: `view-navigation` already forbids sending an empty record in
      place of a real one
- [x] 3.4 Predicted on the pushed fix: every `ci.yml` job green; all five
      `ui-tests.yml` jobs green, `feed` with `ok: all 17 steps passed`

      **Observed, on head `3044b1d`:** UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36214314703 green
      in all five jobs, `feed` printing "the feed was read, and holds nothing"
      `PASS` in 1.0s, `verdict: pass` and `ok: all 17 steps passed`. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36214314665
      green in every job. Before the fix, the record-only push `2caa004` had
      given a second red sample on the same step, UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36213846816
      (CI https://github.com/fryorcraken/dialectica/actions/runs/36213846682
      green), so the race was deterministic on this route, not intermittent

### 4. `create.yaml` goes red when the key block outlives the key

- [x] 4.1 Break (design.md D6): `keyBlockLoader` also active in the key-held
      state, and `heldKey()` carrying `refusal: ""`, pushed alone. Predicted:
      `sitometres create spec` red on "the key is held, so a Stoa is offered
      and a key is not", both `not_text:` selectors naming what they still
      found; the other four spec jobs green. `QML lint` red on exactly three
      `tst_stoa_screens.qml` tests, measured locally (563 passed, 3 failed):
      `test_a_successful_mint_moves_the_screen_to_the_key_held_state`,
      `test_the_key_block_is_not_instantiated_when_a_key_is_held`,
      `test_the_key_state_is_asked_again_on_each_showing`

      **Observed, on head `c20f577`, the break pushed alone, as predicted.**
      UI tests https://github.com/fryorcraken/dialectica/actions/runs/36214768812:
      `sitometres create spec` failed on "the key is held, so a Stoa is
      offered and a key is not" after its 30s, `waitFor never came true`, with
      `does not see "{\"objectName\":\"keyBlock\"}" — still visible on
      QQuickRectangle` and the same for `createKeyButton` on
      `FlatButton_QMLTYPE_132`. Six steps passed; the adjudicator printed
      `verdict: fail`, one `[fail]` and five `[inconclusive]` ("5 later
      step(s) were not attempted"). `join`, `feed`, `thread` and `moderation`
      green. Unlike 3.1's failed `expect:`, a failed `wait_for:` stops the
      run, as `e2e-suite-review` observed. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36214768792 red
      in `QML lint` only, on "QML component tests": `tst_stoa_screens.qml` 133
      passed, 3 failed, the three named above (CI's Qt is 6.8.3, and the
      local prediction was made on 6.10.3). `Lint`, `UI spec validation`,
      `Rust core tests` and `Build LGX` green
- [x] 4.2 Revert pushed; both workflows green on it. **Observed, on head
      `3d7d45c`** (the revert `d996fb1` plus the 4.1 record): UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36215199269 green
      in all five jobs; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36215199261 green
      in every job

### 5. `feed.yaml` goes red when an empty read renders as a failure

- [x] 5.1 Break: `FeedScreen.reload()` puts a read with no items in the failed
      state, pushed alone. Predicted: `sitometres feed spec` red on "the feed
      was read, and holds nothing"; `thread` and `moderation` green, their
      prefixes asserting nothing about the feed's read (design.md D2). `QML
      lint` red on five tests, measured locally (561 passed, 5 failed):
      `tst_e2e_handles.qml`'s `test_the_feed_handles_follow_the_feed_screen`,
      `tst_feed_copy.qml`'s
      `test_the_empty_state_says_whose_copy_the_emptiness_is_a_fact_about`,
      `tst_feed_extent_claim.qml`'s
      `test_a_screen_asserting_no_extent_owes_no_locality_line`, and
      `tst_feed_states.qml`'s `test_an_empty_store_is_the_ok_state_with_no_rows`
      and `test_the_two_states_are_distinguishable`

      **Observed, on head `aa0e694`, the break pushed alone.** UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36215615054:
      `sitometres feed spec` failed on "the feed was read, and holds nothing",
      `root.feedReadState === 'ok'` evaluating to false after its 30s, every
      other step passing; `verdict: fail`, sixteen `[pass]`, one `[fail]`.
      `create`, `thread`, `moderation` and `join` green, as predicted: the two
      prefixes that pass through the feed assert nothing about its read
      (design.md D2). CI
      https://github.com/fryorcraken/dialectica/actions/runs/36215615065 red
      in `QML lint` only, on the five tests predicted **and a sixth the
      prediction predates**: `tst_navigation.qml`'s
      `test_the_feed_is_read_with_the_record_on_every_route_onto_it` (3.2),
      whose fake answers the feed's read with no items, so the break reads it
      as a failure. `Lint`, `UI spec validation`, `Rust core tests` and
      `Build LGX` green
- [x] 5.2 Revert pushed; both workflows green on it. **Observed, on head
      `fa3eaa4`** (the revert `ad672a1` plus the 5.1 record): UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36215967034 green
      in all five jobs; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36215966995 green
      in every job

### 6. `thread.yaml` goes red when the thread is opened without its record

- [x] 6.1 Break: `Main.openThread` sends `genesis: ""`, pushed alone.
      Predicted: `sitometres thread spec` red on "the thread was read, and
      holds the root with no replies", the core refusing an empty record; the
      other four green. `QML lint` red on five tests, re-measured locally on
      the fixed tree (563 passed, 5 failed; the first measurement, before
      3.3, found the first and the last two): `tst_navigation.qml`'s
      `test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op`,
      `test_the_feed_is_read_with_the_record_on_every_route_onto_it` ("back
      from a thread") and `test_the_thread_is_read_with_the_record_when_it_is_opened`,
      and `tst_thread_navigation.qml`'s
      `test_the_feed_it_was_opened_from_is_recoverable_while_reading` and
      `test_the_thread_is_given_the_feeds_stoa_and_record`

      **Observed, on head `9d9323b`, the break pushed alone.** UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36216375991:
      `sitometres thread spec` failed on "the thread was read, and holds the
      root with no replies" after its 30s, on all three checks
      (`noRepliesNotice` not seen, `threadReadState === 'ok'` false,
      `threadItemCount === 1` false). The next step, "write a reply", then
      failed with `No object has objectName "replyDraftField"`, because a
      failed read renders no composer. That stopped the run, and four steps
      were `inconclusive`. Fifteen steps passed before the read. `create`,
      `feed`, `moderation` and `join` green. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36216375946 red
      in `QML lint` only, on exactly the five re-measured tests. `Lint`,
      `UI spec validation`, `Rust core tests` and `Build LGX` green
- [x] 6.2 Revert pushed; both workflows green on it. **Observed, on head
      `18f5bee`** (the revert `df19182` plus the 6.1 record): UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36216812878 green
      in all five jobs; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36216812897 green
      in every job

### 7. `moderation.yaml` goes red when an inert control withdraws the way out

- [x] 7.1 Break: "Mark as moderated" hides `moderationBackButton`, pushed
      alone. Predicted: `sitometres moderation spec` red on "acting on an
      inert control leaves the way out offered", on its `text:` half after
      the step's 30s; the other four green. This is also the measurement
      design.md D5 rests on: a green here would mean the step reads before the
      click lands. `QML lint` red on one test, measured locally and
      re-measured after 3.3 (567 passed, 1 failed): `tst_navigation.qml`'s
      `test_the_moderation_screen_can_be_left_after_pressing_its_controls`

      **Observed, on head `ed5c482`, the break pushed alone, as predicted.**
      UI tests https://github.com/fryorcraken/dialectica/actions/runs/36217172782:
      `sitometres moderation spec` failed on "acting on an inert control
      leaves the way out offered" after 30.1s. It clicked
      `FlatButton_QMLTYPE_130 "Mark as moderated"`, then
      `x sees "{\"objectName\":\"moderationBackButton\"}"`, while
      `+ state "root.screenShown === 'moderation'"` held. So the step read a
      snapshot taken after the click, which is the measurement design.md D5
      rests on. "take the way out" then failed with no such control, and one
      step was `inconclusive`. `create`, `feed`, `thread` and `join` green. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36217172763 red
      in `QML lint` only, on exactly the one predicted test. `Lint`, `UI spec
      validation`, `Rust core tests` and `Build LGX` green
- [x] 7.2 Revert pushed; both workflows green on it. **Observed, on head
      `456b95f`** (the revert `4eb514b` plus the 7.1 record): UI tests
      https://github.com/fryorcraken/dialectica/actions/runs/36217546045 green
      in all five jobs; CI
      https://github.com/fryorcraken/dialectica/actions/runs/36217546041 green
      in every job

### 8a. A click that beat the layout (found on the final tip)

- [x] 8a.1 **Observed, on head `80eef16`, a documentation-only commit:** UI
      tests https://github.com/fryorcraken/dialectica/actions/runs/36217960486
      red in `sitometres thread spec` only, on "it was saved on this machine,
      and the thread now holds the root and the reply" (`waitFor never came
      true`; `threadItemCount === 2` false). "publish it" had passed, clicking
      `FlatButton_QMLTYPE_136 "Publish the reply"`, but the Basecamp log in
      its evidence has no `publish_reply` call at all. `create`, `feed`,
      `moderation` and `join` green. CI
      https://github.com/fryorcraken/dialectica/actions/runs/36217960370
      green on the same head
- [x] 8a.2 Cause, measured with a throwaway component probe (not committed):
      the reply submit is at y=140 in the turn the draft is set and at y=163
      after a layout pass, 37px tall, so a click aimed before the pass misses
      it (design.md D9)
- [x] 8a.3 A `wait_for:` on the submit control before each click, in
      `feed.yaml` and `thread.yaml`: steps 18 and 23. Both parse with `yq`
- [ ] 8a.4 Predicted on the pushed fix: `UI spec validation` printing
      `feed.yaml: ok (18 steps)` and `thread.yaml: ok (23 steps)`; all five
      UI jobs green; every CI job green

### 8. Hand-back

- [x] 8.1 No break in the branch's net diff. `git diff --stat db07bdd HEAD`
      over `DStoaListScreen.qml`, `FeedScreen.qml`, `Main.qml` and
      `DModerationScreen.qml` lists no `DModerationScreen.qml` change, and
      `DStoaListScreen.qml`'s five lines are the `openStoaButton` name and its
      comment. The rest is sections 1 and 3.3. `git grep` for each break's
      text ("e2e proof break", `id: wayOut`, `refusal: "",`,
      `genesis: "", rootOp`) finds nothing. The tip is a documentation commit
      on top of the 7.2 revert. Its own CI state is reported in the hand-back
      and on the PR, because this file cannot name a run of the commit that
      contains it
