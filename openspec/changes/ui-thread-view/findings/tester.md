# Tester findings — `ui-thread-view`

Found while proving the security property (D1) can fail for the reason it is
named, per the brief's instruction to look for a *second* route to the same
bad state beyond the two the code-reviewer already mutation-tested (`-1` →
`depth + 1`, and the parent-cycle branches).

- [x] **`tester`** — `dialectica-ui/src/qml/DThreadScreen.qml:129-135`
      (`parentOf`) — **[security]** `parentOf` collapses **any** non-string
      `parent` field — not only an absent one — to `""`:
      ```qml
      function parentOf(item) {
          if (item === null || item === undefined)
              return ""
          return typeof item.parent === "string" && item.parent !== ""
              ? item.parent
              : ""
      }
      ```
      `resolveDepth` treats `parentOf(item) === ""` as "this item IS the
      root" and returns depth `0` **unconditionally**, before the visited-set
      walk and its `-1` guards are ever reached (`DThreadScreen.qml:172-173`).
      So an item whose `parent` field is present but is `null`, a number, an
      object, or an explicit `""` — none of which is "the root reporting no
      parent" — is rendered at the root's own depth, with no "IN REPLY TO A
      POST NOT SHOWN" notice. This is the exact outcome the security
      requirement forbids ("The view SHALL NOT... render it as though its
      parent were the root"), reached through a different code path than the
      one already mutation-tested: it never enters the `while` loop, so
      mutating the `parent not on this page` or `parent cycle` branches
      (`-1` → `depth + 1`) does not touch it, and neither of the two existing
      security tests exercises it.

      **Scenario:** a page containing an item whose `parent` field decodes to
      `null` (or any non-string) renders that item as if it directly answered
      the root, indistinguishable from a genuine reply to the root, with the
      "answers a post not shown" notice suppressed.

      **Measured:** added
      `test_an_item_reporting_a_non_string_parent_is_not_rendered_as_the_root`
      to `tst_thread_nesting.qml` (constructs a non-root item with
      `parent: null`). Currently **FAILS** against the live code:
      `resolveDepth` returns `0`, expected `-1`. Run:
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_thread_nesting.qml`
      — 14 passed, 1 failed (this one).

      **Is this reachable today?** `wire.rs:7041` and the `read_thread`
      response builder (`wire.rs:2027`) only ever omit `parent` (root) or
      serialise it as a hex op-id string (non-root) — the current core never
      sends a malformed `parent`. So this is boundary defence against a
      shape the view's own contract does not currently produce, in the same
      posture `DThreadScreen.qml` already takes toward an item missing `id`
      (`itemId`'s own guard, and the sibling test
      `test_items_carrying_no_id_do_not_share_a_parent_slot`) — peer-supplied
      items are not validated element-by-element anywhere upstream of this
      screen, and this repo's stated security posture
      (`CLAUDE.md`: "Never trust an inbound message... Validate at the
      boundary") treats "core is honest today" as not the same claim as
      "the view cannot be handed this shape." `itemId` already guards the
      analogous case for `id`; `parentOf` does not for `parent`.

      **Suggested fix:** distinguish "the field is absent" (root) from "the
      field is present but not a usable string" (unresolvable, must yield
      `-1`) in `parentOf`, the same way `itemId` already treats a present
      non-string `id` as yielding no usable key. One shape: have `parentOf`
      return a sentinel distinguishable from "no parent" for the
      present-but-malformed case, and have `resolveDepth` route that
      sentinel to `-1` rather than into the `parent === ""` (root) branch.

      The test is left in the committed suite, currently failing, rather than
      held back or weakened — per `tester.md`'s invariant, a test that cannot
      fail is worse than no test, and holding a provably-failing test out of
      the suite would report a security property as checked when it is not.
      This blocks the `tests — tester` row from being marked complete until
      `dev-writer` fixes `parentOf` (or the finding is rejected with an
      argument for why the current shape is intentional).

      **Fixed** in `0444726`, by reshaping rather than by adding a guard.
      `parentOf` now returns `{ kind, id }` over three kinds — `parentRoot`
      (no `parent` field), `parentNamed` (a usable op id), `parentUnusable`
      (present but not a usable string, or no item at all) — and
      `resolveDepth` takes its depth-0 branch only on `parentRoot`, routing
      `parentUnusable` to `-1`. The root case and the malformed case no
      longer share a value, so a caller cannot read one as the other by
      omission; it has to name the kind it means.

      Your suggested sentinel shape was considered and rejected, with the
      reason recorded in design.md D13: a sentinel still leaves a caller that
      compares against `""` compiling and silently taking the root branch,
      which is the same failure mode one value later. The finding itself —
      that a guard proved by mutation along one route was bypassed along
      another — is recorded as D13's opening, and D1's closing paragraph is
      corrected, since it described the absent/present distinction as though
      the code already made it.

      `test_an_item_reporting_a_non_string_parent_is_not_rendered_as_the_root`
      is unchanged and now passes: `tst_thread_nesting.qml` is **16 passed, 0
      failed**. Mutating the new `parentUnusable` branch's `-1` to `0` turns
      that test and only that test red (15 passed, 1 failed), so it pins the
      fix rather than decorating it.

      Your `-1` guard was re-proved intact afterwards, as the brief required:
      mutating the not-on-this-page branch `-1` → `depth + 1` still turns
      `test_an_item_whose_parent_is_absent_is_not_re_parented_to_the_root`,
      `test_items_carrying_no_id_do_not_share_a_parent_slot` and
      `test_the_unresolved_parent_notice_is_visible_only_where_it_must_be`
      red — while leaving the new test green. The two routes are
      independently pinned, which is the measurement that says the fix
      extends the guard rather than routing around it.

      On your `itemId` observation: checked, and it does **not** have the
      same defect, so it was deliberately left alone rather than swept along.
      Its `""` is single-valued — both call sites treat it as a *refusal*
      (`itemsById` skips the item, `resolveDepth` skips seeding `visited`),
      and neither reads `itemId(x) === ""` as an affirmative fact the way
      `resolveDepth` read `parentOf(x) === ""` as "this is the root". There
      is no second meaning for it to collide with. Recorded in D13, because
      "the sibling function has the same shape" is the obvious next question
      and the answer is not symmetric.

      One behaviour change beyond the finding, called out because it is not
      what you reported: a `null`/`undefined` **item** now classifies as
      `parentUnusable` rather than `parentRoot`. An absent item is not a
      root, and the previous `""` return made it one.

      Your reachability note is carried into D13 verbatim in substance —
      `wire.rs:7041` and `:2027` only omit `parent` or send a hex op id — with
      an explicit instruction not to delete the guard as unreachable, since
      "core is honest today" is a different claim from "the view cannot be
      handed this shape."
