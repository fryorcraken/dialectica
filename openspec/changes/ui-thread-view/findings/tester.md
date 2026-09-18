# Tester findings — `ui-thread-view`

Found while proving the security property (D1) can fail for the reason it is
named, per the brief's instruction to look for a *second* route to the same
bad state beyond the two the code-reviewer already mutation-tested (`-1` →
`depth + 1`, and the parent-cycle branches).

- [ ] **`tester`** — `dialectica-ui/src/qml/DThreadScreen.qml:129-135`
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
