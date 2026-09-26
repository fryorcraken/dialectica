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

### 4. `create.yaml` goes red when the key block outlives the key

- [ ] 4.1 Break (design.md D6): `keyBlockLoader` also active in the key-held
      state, and `heldKey()` carrying `refusal: ""`, pushed alone. Predicted:
      `sitometres create spec` red on "the key is held, so a Stoa is offered
      and a key is not", both `not_text:` selectors naming what they still
      found; the other four spec jobs green. `QML lint` red on exactly three
      `tst_stoa_screens.qml` tests, measured locally (563 passed, 3 failed):
      `test_a_successful_mint_moves_the_screen_to_the_key_held_state`,
      `test_the_key_block_is_not_instantiated_when_a_key_is_held`,
      `test_the_key_state_is_asked_again_on_each_showing`
- [ ] 4.2 Revert pushed; both workflows green on it

### 5. `feed.yaml` goes red when an empty read renders as a failure

- [ ] 5.1 Break: `FeedScreen.reload()` puts a read with no items in the failed
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
- [ ] 5.2 Revert pushed; both workflows green on it

### 6. `thread.yaml` goes red when the thread is opened without its record

- [ ] 6.1 Break: `Main.openThread` sends `genesis: ""`, pushed alone.
      Predicted: `sitometres thread spec` red on "the thread was read, and
      holds the root with no replies", the core refusing an empty record; the
      other four green. `QML lint` red on three tests, measured locally (563
      passed, 3 failed): `tst_navigation.qml`'s
      `test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op`, and
      `tst_thread_navigation.qml`'s
      `test_the_feed_it_was_opened_from_is_recoverable_while_reading` and
      `test_the_thread_is_given_the_feeds_stoa_and_record`
- [ ] 6.2 Revert pushed; both workflows green on it

### 7. `moderation.yaml` goes red when an inert control withdraws the way out

- [ ] 7.1 Break: "Mark as moderated" hides `moderationBackButton`, pushed
      alone. Predicted: `sitometres moderation spec` red on "acting on an
      inert control leaves the way out offered", on its `text:` half after
      the step's 30s; the other four green. This is also the measurement
      design.md D5 rests on: a green here would mean the step reads before the
      click lands. `QML lint` red on one test, measured locally (565 passed, 1
      failed): `tst_navigation.qml`'s
      `test_the_moderation_screen_can_be_left_after_pressing_its_controls`
- [ ] 7.2 Revert pushed; both workflows green on it

### 8. Hand-back

- [ ] 8.1 No break in the branch's net diff: `git diff --stat` from the piece's
      base over `DStoaListScreen.qml`, `FeedScreen.qml`, `Main.qml` and
      `DModerationScreen.qml` shows only sections 1's changes
