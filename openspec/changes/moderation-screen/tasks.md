# Tasks

## Stages

- [ ] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

**The spec row is unticked and no `spec-writer` ran.** This piece's
`proposal.md` and both spec deltas were written by the `dev-writer`, because the
owner's reversal arrived as a direct instruction rather than through PLAN.md.
That is a deviation from the flow and it is recorded rather than papered over:
the row belongs to an agent that did not run, and a reviewer should read the
deltas knowing the author of the contract also wrote the code against it.

## Implementation

Every box below is ticked against a gate or a named test **run in this
worktree**. Where a requirement cannot be discharged by anything in this tree it
is said so rather than ticked.

### The amendments

- [x] `docs/PLAN.md` ruling 3 records the reversal without deleting the original
      reasoning, names the owner as the decider, and says what it costs. §6's
      restatement is amended in step, and the trait paragraph it contains is
      kept with its premise re-pointed rather than struck.
- [x] Ruling 2 is amended to separate the *count* (still out) from the *rendered
      position* (in, as a placeholder), and says no core change works that entry
      off.
- [x] The "a scope note does not override a merged requirement" paragraph is
      amended to cite this change as the mechanism working, not an exception.
- [x] The counts went through a **spec delta**, not a PLAN edit:
      `specs/stoa-navigation-view/spec.md` amends the requirement, states what
      changed, that the owner decided it, and that a core call answering the
      number restores the stricter form.
- [x] The retired scenario "A row renders no count of held posts" is **narrowed
      under its own name** rather than deleted, so a reader comparing versions
      sees which clause moved. `openspec validate moderation-screen --strict`
      refuses a MODIFIED block that drops a scenario, which is what caught it.
- [x] `openspec validate moderation-screen --strict` — valid.

### The screen

- [x] The inertness is rendered, names the absent core method, and does not read
      as a fault: `tst_moderation_screen.qml`'s
      `::test_the_screen_states_that_it_publishes_nothing`,
      `::test_the_notice_names_the_missing_core_method_as_the_reason`,
      `::test_the_notice_does_not_present_the_absence_as_a_fault` (a hardcoded
      fault-word list, which caught my own copy saying "retry").
- [x] The lists are declared examples:
      `::test_the_notice_says_the_lists_are_examples`.
- [x] No control calls the core, proven with a **recording** fake and a non-zero
      baseline: `tst_navigation.qml::test_nothing_on_the_moderation_screen_calls_the_core`.
- [x] The ceiling on what moderating reaches is stated:
      `::test_the_screen_states_what_moderating_cannot_do` pins all three
      clauses — asks readers to hide, does not delete, does not block a user.
- [x] Two lists, one control per row, never one control for both:
      `::test_the_two_lists_are_separate_with_their_own_controls`. This is the
      test that caught the `ListView` instantiating one delegate of two.
- [x] Fixtures cannot be overwritten from outside:
      `::test_the_fixture_lists_cannot_be_written_from_outside`, which pins that
      QML **throws** rather than ignoring the write.
- [x] Every address carries the middle group:
      `::test_every_address_carries_the_middle_group`, with the 8/8/6 offsets
      hardcoded from the fixture rather than recomputed from `DTheme`.
- [x] Nothing peer-supplied renders as markup: a **sweep** over every `Text`
      (`::test_every_text_the_screen_renders_is_plain_text`, non-vacuous — it
      asserts it found more than eight), plus
      `::test_a_name_containing_markup_is_rendered_as_its_characters` driving a
      string the implementation did not choose.
- [x] No moderator authority is claimed:
      `::test_the_screen_claims_no_moderator_authority`.

### The route

- [x] Reachable through an affordance a user acts on, with the Stoa travelling:
      `tst_navigation.qml::test_the_feed_offers_a_route_into_moderation`.
- [x] Registered, and reached transitively from `Main.qml`:
      `check_qml_reachable.py` — 25 registered, 23 reached, 2 recorded.
- [x] The way out survives an inert press:
      `::test_the_moderation_screen_can_be_left_after_pressing_its_controls`,
      and `::test_cancel_leaves_the_moderation_screen` for the second exit.
- [x] Exactly one screen in every reachable state, moderation included:
      `::test_exactly_one_screen_is_shown_in_every_reachable_state`, with
      `::test_the_screen_walk_covers_every_state_the_navigator_has` pinning the
      hand-written screen list against `stateNames` so a seventh state cannot be
      added with the walk silently covering six.

### The navigator reshape

- [x] `enterOnly` is the one transition primitive; no transition clears a
      sibling by hand.
- [x] Proven to fix a live defect:
      `::test_entering_onboarding_from_moderation_clears_the_moderation_state`
      fails against the pre-reshape file.
- [x] `::test_no_transition_leaves_two_states_set_at_once` derives its count
      from `stateNames` rather than restating property names.

**Not ticked, and labelled rather than claimed:** `openThread`'s missing clear is
**satisfied by construction**, not by a test. It refuses outright when `chosen` is
null, so it is unreachable from moderation and the omission could never fire. The
test comment says so.

### The row count placeholder

- [x] Marked at the site that renders it, naming what would supply the real
      value: `DStoaListScreen.qml`.
- [x] Constant across fixtures differing in every quantity a screen could reach
      for, and carrying no numeral and no claim of emptiness:
      `tst_stoa_screens.qml::test_the_row_count_placeholder_claims_no_measurement`.
- [x] The two pre-existing count assertions still pass unchanged, which is what
      shows the narrowing was narrow.

### Gates run in this worktree

- [x] `check_qml_names.py dialectica-ui` — 46 files, 25 entries. It caught the
      test's `name: "ModerationScreen"` label, which is the rule working on a
      spelling rather than on a location.
- [x] `check_qml_reachable.py dialectica-ui` — 25 registered, 23 reached.
- [x] `tst_check_qml_reachable.py` — 16 cases, both directions.
- [x] `check_qml_members.sh` — 26 files.
- [x] `run-qml-tests.sh` — 20 spec files, 0 failed.
- [x] `openspec validate moderation-screen --strict` — valid.

**What these cannot see**, said rather than left to read as coverage:

- **No basecamp launch was performed.** The screen has not been seen rendering,
  and the visual match to the reference is by reading its CSS rather than by
  comparing pixels. The owner asked to *look at* this screen, so that gap is the
  one that matters most here.
- The reachability gate proves instantiation, never that a user can reach the
  screen at runtime. The route test is the closest a component suite gets.
- The QML suite runs with the host absent, so it is blind to a host type-name
  collision by construction.
- `check_qml_members.sh` globs `src/qml/*.qml` only, so the new test spec is
  outside it. `check_qml_names.py` does reach it.
- **No Rust changed**, so no Rust gate can see this change. `cargo test` was not
  run and reporting it would say nothing about this diff.
