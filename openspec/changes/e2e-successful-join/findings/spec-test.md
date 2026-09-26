# spec-test review — e2e-successful-join

Scope: spec-to-test mapping only, per this review's dispatch. Did not read
`design.md` or any file under `dialectica-ui/src/` or
`dialectica/rust-lib/dialectica-core/src/`. Did not re-run the address-mutation
check on `seeded_reference.rs` (tester/security/architecture already ran it,
all 3 tests red) or poll CI (tester/correctness already confirmed).

## 1. Word-for-word check on both MODIFIED deltas

Extracted the live requirement text from `openspec/specs/stoa-navigation-view/spec.md`
(lines 536-568) and `openspec/specs/view-navigation/spec.md` (lines 203-258) to
`./tmp/spec-diff/` and ran `git diff --no-index` against the corresponding
requirement in each delta file.

- **`stoa-navigation-view`** ("A join is reported from the core's reply, never
  assumed"): clean. The only differences are the stated addition — one new
  paragraph ("**A successful reply to a join MUST be reported as a join**...")
  inserted between the first two paragraphs — and one new scenario ("Joining a
  Stoa not held is reported as joined"). Every other word is unchanged.
- **`view-navigation`** ("Every state a user can enter has a specified way
  out"): clean. The only differences are the stated addition to the opening
  sentence ("; a Stoa joined through the join preview MUST reach the list
  without a restart;") and one new scenario ("A joined Stoa reaches the list
  without a restart"). Every other word, including all four pre-existing
  scenarios, is unchanged.

No defect here in either delta.

## 2. Scenario-to-test mapping

All 8 scenarios across the two deltas are pinned:

**`stoa-navigation-view`**
- *Success is rendered only after a successful reply* — `tst_stoa_screens.qml`
  `test_a_failed_join_does_not_report_the_stoa_as_joined` (asserts `joinState
  === "failed"`, the `joined` signal did not fire, the core's failure text is
  rendered, and `joinedPanel` is absent). Can fail for the named reason.
- *Joining a Stoa already held is success* — `tst_stoa_screens.qml`
  `test_joining_a_stoa_already_held_is_success_with_no_warning` (asserts
  `joinState === "joined"` for a Stoa already in `heldStoas`, and scans body
  text for no collision/warning language). Can fail for the named reason.
- *Joining a Stoa not held is reported as joined* (new) — `seeded-join.yaml`'s
  "it is reported as joined, and no failure of the join is rendered" step
  (`joinState === 'joined'`, `joinFailure === ''`, `joinedPanel` present,
  `joinFailurePanel` absent, on a profile the precondition step establishes
  holds no Stoa), and `tst_navigation.qml`
  `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` (`joinState ===
  "joined"` after joining a Stoa absent from `stoaCount` beforehand). Together
  these can fail for the named reason; the yaml test is the one that pins "no
  failure of the join" specifically.

**`view-navigation`**
- *The preview can be left without joining* — `tst_navigation.qml`
  `test_declining_the_join_preview_leaves_the_list_with_no_join_call` (asserts
  `screenShown === "list"` after clicking `joinCancelButton` from
  `previewing`, and `join_stoa` was called 0 times). Can fail for the named
  reason.
- *The return is still available after a join succeeds* —
  `tst_navigation.qml` `test_a_joined_stoa_is_listed_once_the_join_screen_is_left`
  (asserts exactly one visible `joinCancelButton` after the join succeeds, and
  that clicking it renders the list), and `seeded-join.yaml`'s use of
  `joinCancelButton` after the joined state (a click on a hidden/absent
  control fails by name, so the click itself is the assertion that it was
  offered). Can fail for the named reason.
- *A created Stoa reaches the list without a restart* — `tst_e2e_handles.qml`
  `test_the_creation_handles_follow_the_creation_reply` (asserts
  `main.listedStoas` equals both the pre-existing and the newly created
  address after `list.create()`, on the same `main` instance). Can fail for
  the named reason.
- *A joined Stoa reaches the list without a restart* (new) —
  `tst_navigation.qml` `test_a_joined_stoa_is_listed_once_the_join_screen_is_left`
  (captures `readsBefore = callsTo(bridge, "list_stoas")`, joins, asserts the
  call count increased, then asserts `listedStoas` holds exactly the joined
  address and a `shareButton` is offered), and `seeded-join.yaml`'s "the
  joined Stoa is listed without a restart" step (`listReadState === 'ok'`,
  `listedStoas.length === 1`, the address matches, on a profile the
  precondition established held nothing). Both are non-vacuous: the call-count
  check in the QML test specifically rules out a screen that renders a stale
  listing, and the yaml test's count moving from 0 to 1 cannot happen without
  a fresh read. Can fail for the named reason.
- *The list is reached again from a Stoa's feed* — `tst_stoa_screens.qml`
  `test_a_user_who_opened_a_stoa_can_return_to_the_list` (asserts
  `screenShown === "list"` after `feedBackButton` is clicked, `chosen ===
  null`, and the list's rows are back on screen). Can fail for the named
  reason.

## 3. Unpinned scenarios

None found. Every scenario in both deltas has at least one test that can fail
for the reason the scenario names, at a layer that can observe it (QML
component tests for view-state/rendering claims, the `seeded-join.yaml` e2e
run for the cross-process claim that a real `joinStoa`/`listStoas` round trip
reports success and lists the Stoa).

- [x] **none** — both MODIFIED deltas preserve their live requirement text
      word for word apart from the stated addition, and every scenario in
      both deltas is pinned by a test that can fail for the reason the
      scenario names.
