# Tasks

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

## Implementation

Each box below is ticked only where a test can show it. Where a requirement holds
because the code that would break it cannot be reached, it is labelled
**satisfied by construction** with what makes the absence real — a checkbox
claiming a test verifies something it structurally cannot is worse than an
unticked one.

- [x] **`Core.qml` binds `readThread`** through the view's single call path.
      The wire method is `read_thread`; the request carries
      `{stoa, genesis, thread, includeHidden}`.
- [x] **`DThreadScreen.qml`**, registered in `qmldir` with the `D` prefix and no
      gate exemption.
- [x] **Nesting from the parent chain**, with a visited set so the walk
      terminates over peer-supplied parents.
      `tst_thread_nesting.qml` — 14 tests.
- [x] **An unresolvable parent is not re-parented to the root.** The security
      requirement. Proved failing: replacing the `-1` branch with `depth + 1`
      turns `test_an_item_whose_parent_is_absent_is_not_re_parented_to_the_root`
      and `test_items_carrying_no_id_do_not_share_a_parent_slot` red, and nothing
      else in the suite notices.
- [x] **The indent is bounded** at 6 levels of 34px, clamping pixels only —
      `test_a_deep_chain_stops_indenting_but_keeps_every_item` asserts the
      computed depth keeps growing while the indent stops.
- [x] **The returned sequence is the rendered sequence**, and `position` is read
      by nothing. `test_the_sequence_rendered_is_the_sequence_returned` and
      `test_positions_that_are_not_numerals_are_not_parsed`.
- [x] **A withheld body and an empty body render differently**, distinguished by
      the field's presence rather than its contents.
      `tst_thread_states.qml` — `test_withheld_and_cleared_are_distinguishable_by_the_views_own_test`.
- [x] **A revised post is marked**, from `isRevised` and never from comparing
      `id` with `currentVersion`. `PostHeader`'s `edited` carries it.
- [x] **The earlier-versions affordance is inert** and reaches no call —
      `test_the_earlier_versions_control_reaches_no_core_call` asserts it carries
      no MouseArea, so there is no route to a call rather than a route that goes
      nowhere. Documented in `docs/PLAN.md` §9.2 case 2 entry 4.
- [x] **The reply composer is wired to `publish_reply`** —
      `tst_thread_reply.qml`, `test_submitting_a_reply_makes_a_publish_call`.
- [x] **The reply names the root's `id`, not its `currentVersion`**, and carries
      no thread identifier.
- [ ] **A reply to a non-root names that post rather than the root** —
      **satisfied by construction, not tested.** The screen offers ONE composer,
      at the thread's foot, as the design places it; there is no reply affordance
      on a non-root post, so a request misnaming a parent cannot be constructed.
      No test can exercise it because the interface offers no way to make such a
      reply. A per-post affordance is the piece that makes this testable, and it
      is not this one. Left unticked deliberately: the box would be a false
      statement either way.
- [x] **An unreadable thread and a thread with no replies are different
      screens**, and three refusals reach one state with no branch on their text.
- [x] **Every core call goes through `Core.call`** — an unreachable core, a
      non-JSON reply and a success shape carrying no items each reach the refused
      state rather than rendering as content.
- [x] **No score, tally or vote count is rendered.** The vote control renders
      arrows with `showScore` false and `interactive` false; documented in
      `docs/PLAN.md` §9.2 case 2 entry 5.
- [x] **Every string is rendered as core supplied it** — bodies go through
      `SanitisedText`, which is `Text.PlainText` with no property to change it,
      and the sanitiser's counts render as chips rather than being applied.
- [x] **The screen is reached from a feed row and can be left again**, and the
      way out survives a refused read. `tst_thread_navigation.qml` — 11 tests —
      plus `tst_navigation.qml`, which owns the transition after the
      `piece/ui-navigation` merge (design.md D9). Proved failing: the way-out
      assertion is `tst_navigation.qml`'s
      `test_a_thread_that_could_not_be_read_still_offers_the_way_back`.
- [x] **The screen carries no thread, Stoa or record of its own** holding a
      usable default.
- [x] **Rebased onto `cc37f2e`**, reconciling two independently-written thread
      screens across 27 conflict regions. The split, the two places navigation's
      version was kept, and the field-name defect BOTH pieces shipped are
      recorded in design.md D10 and *What the merge took from each side*.
- [x] Gates: `check_qml_reachable.py` (24 registered, 21 reached from `Main.qml`,
      the thread screen among them), `check_qml_names.py`, `check_qml_members.sh`,
      `check_probe_twins.sh`, and the full QML suite — 24 spec files, 453 tests,
      0 failures.
- [x] **The security property re-proved after the merge.** Mutating
      `resolveDepth`'s unresolvable-parent branch from `-1` to `depth + 1` turns
      `test_an_item_whose_parent_is_absent_is_not_re_parented_to_the_root` red on
      its "not a direct reply to the root" assertion, and
      `test_items_carrying_no_id_do_not_share_a_parent_slot` with it. Reverted and
      re-run green.
- [x] **PLAN.md reasoning migrated** — §9.1's moderation-shape divergence to
      `design.md` D7 and its bidi paragraph to D8, each left as a pointer rather
      than a second copy.
