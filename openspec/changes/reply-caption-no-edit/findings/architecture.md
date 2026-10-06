# Architecture review: reply-caption-no-edit

Dimension: architecture (one instance). Diff read: `git diff origin/main...HEAD`.

- [x] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:562` — `stringsUnder` is a second copy of `collectTexts` (line 388)
      **Scenario:** both are `if (typeof item.text === "string") out.push(item.text)` followed by a recursion over `item.children`, byte for byte apart from the name. `collectTexts` was already in the file on `origin/main`; this change added `stringsUnder` beside it and did not reuse it. A later fix to the walk (reading `contentItem`, skipping a hidden subtree, descending into a `Repeater`'s delegates) has to be made twice, and it is easy to make it in one and have the open-group walk and the revised-marker walk disagree about what "rendered text" means in the same file.
      **Severity:** low. **Kind:** genuine defect (duplication introduced by this change), not taste. **Fix shape:** delete `stringsUnder` and call `collectTexts` from `editClaimsUnder`; move the comment about placeholders and `children`-only reach onto `collectTexts`, where it applies to both users.
      **Outcome (tester): fixed.** `stringsUnder` is deleted. Every walk in
      `tst_thread_reply.qml` is `collectTexts`, which gained an optional
      `renderedOnly` argument (the signed-statement walks need "rendered", the
      claim walks need "everything", so one function with a flag and not two
      copies). The placeholder, `children`-only and `visible` comment now sits on
      the helpers that use the walk and `collectTexts` points to it. Checked with
      `git grep -F "function stringsUnder"`: no hit. Not a behaviour, so no test
      fails without it; the walks' reach is pinned by the tests that use them.

- [x] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:601`, `:654`, `:710`, `:590` — the no-claim guard's reach is two container names, so text beside the composer but outside both containers is unguarded
      **Scenario:** the three scenario tests walk `findChild(screen, "replyComposerOpen")` and `findChild(screen, "replyGateShut")` and nothing else. The only text outside those walks that the tests look at is the caption, and only through `findChild(screen, "replyCaption")` in `verifyWalkCoversTheOpenGroup`. A `Text { text: "A reply can be edited later."; visible: screen.capability.canPost === true }` added as a sibling of the two `ColumnLayout`s in `DThreadScreen.qml`, between them or after the shut gate, is rendered "with the reply composer" in the spec's sense and sits in neither subtree, so none of the scenario tests walks it. The same holds for a new unnamed Text inside the group's parent. Only a caption given the exact `objectName` `replyCaption` is protected against moving out of the group.
      **Measured:** not measured. I tried the mutation above in my worktree and the harness refused the edit, so this is read off the structure: `findChild` plus a `children` walk cannot visit a sibling. It is plain from the tests' own comments that the scope is chosen by container, not by what the screen renders.
      **Severity:** low to medium. **Kind:** a shape the design accepted knowingly for the thread rows (design.md Decision 3, "The walk is scoped to those two subtrees") but did not carry to its other edge: design.md names a caption moved out of the group, and does not name new text added beside the two groups. **Fix shape, either one:** walk the whole screen and subtract the thread rows (the rows are already identifiable by `threadItems`, and the scope test already shows exactly which two strings they add), which makes the scope an exclusion of known reports and not an inclusion of two containers; or state the residual risk in design.md's Risks section next to the word-list risk, so the next reader knows the walk is a guard on two subtrees and not on the screen.
      **Outcome (tester): fixed, the first option, and the second recorded for
      what remains.** `test_no_text_beside_the_thread_rows_makes_an_edit_or_version_claim`
      walks every text in the screen's content column except the thread's rows
      (`textsBesideTheRows`; the rows are named by `itemAt` on `threadItems`,
      because a Repeater parents its delegates beside itself, in the screen's
      single content ColumnLayout, and not under itself), in three
      states: open gate, shut gate, open gate after a publish. It asserts the rows
      exist, that the walk reached the composer's place, and that `edited` is not
      in it. **Measured**, the reviewer's mutation (a sibling `Text { text: "A
      reply can be edited later."; visible: screen.capability.canPost === true }`
      before the shut gate): that test and the core-supplied-text test went red;
      all three group tests stayed green, so the gap was real. Predicted the
      same two. Also measured: a walk beside the rows that does not leave the rows
      out turns red that test, the scope test and the core-supplied-text test; a
      caption moved out of the group carrying "Earlier versions stay readable."
      turns six tests red (open-composer and after-publish on the caption-in-group
      assertion, beside-the-rows, core-supplied-text, and both signed-statement
      tests). design.md Decision 3 carries the measurements and a new Risks bullet
      names what the walk still does not reach (a claim inside a row's delegate,
      a popup or tooltip, text outside the content column).

## Clean areas

- **The production change** (`DThreadScreen.qml`, one `objectName` and one string) is the minimum that satisfies the requirement. It adds no state, no branch and no new property, and the caption stays inside the `replyComposerOpen` group, so it is covered by the walk. Nothing about it reshapes the screen.
- **Layering and the core API.** No core method, no `Core.qml` change, no module-surface change. The spec's "no method on the module surface publishes a revision" holds in the tree: `git grep` for revise/edit over `DComposer.qml` and `DThreadScreen.qml` finds only comments and the `edited` marker.
- **Stale copies elsewhere.** `git grep` for the old sentence, "edited later", "can be edited" and "earlier versions stay" outside this change finds the code comment (now correct), the test's own hand-written claim tables, and the archived `ui-thread-view` design, which design.md Decision 2 deliberately leaves alone and supersedes in writing. No other screen or composer carries a version or edit promise.
- **The code comment's pointer** to "the reply-caption-no-edit change's design.md" follows the convention of the neighbouring comments in `src/qml` (`design.md` plus a change name), so it is not a finding. It will need the archive path to find after merge, which a grep for the change name gives.
- **Matcher shape.** A word-list matcher is a hand-maintained list, which this project's memory warns about, but design.md Decision 3 names the alternative (pin the exact strings), says why it was rejected, and records the residual paraphrase risk. I did not re-raise it. I ran the suite file: all 27 tests in `tst_thread_reply.qml` pass.
- **Dependencies, CI gates.** No new dependency. The spec file is an existing `tst_*.qml`, so the `every QML spec file actually ran` count is unaffected, and no file moved.

## What was not done

- No `cargo mutants` run: the diff touches no Rust.
- No mutation of the QML source: the harness denied the one edit I tried (a sibling `Text` outside both groups), so finding 2 is unmeasured.
