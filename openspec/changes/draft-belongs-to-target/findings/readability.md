# Readability findings

Dimension: readability only. Reviewed `git diff origin/main...HEAD` for
`dialectica-ui/src/qml/DComposer.qml`, `dialectica-ui/tests/tst_draft_targets.qml`
and the changed tests in `dialectica-ui/tests/tst_publish_outcome_visits.qml`.
Every entry below is a defect of the reader's experience, not of behaviour; the
last two are low and stylistic, and say so.

- [x] **`dev-writer`** — `DComposer.qml:110-112` — the comment says "Without it
      every cross-target test in `tst_draft_targets.qml` is red", which is false
      and is the sentence a maintainer will trust when deciding what the handler
      is for.
      **Scenario:** delete `onTargetKeyChanged: root.showDraftOfTarget()` and run
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_draft_targets.qml`.
      18 of the 37 test functions fail and 19 stay green. Cross-target tests that
      stay green include `test_a_post_written_in_the_second_stoa_is_published_there_with_its_own_text`,
      `test_a_draft_returned_to_is_published_to_its_own_stoa`,
      `test_a_reply_written_in_the_second_thread_names_that_threads_parent_and_stoa`
      and `test_a_reply_draft_returned_to_is_published_to_its_own_parent`: each
      re-types or re-reads text that the stale field happens to hold already. The
      accurate sentence is about the tests that expect the other target's field
      to be empty.
      **Severity:** low-medium; a false claim about what pins the one load-bearing
      handler. **Measured:** 18 failed, 21 passed (including the init and cleanup
      cases), handler removed.

      **Fixed** (`dev-writer`), in the commit that ticks this box. Measured
      again on my tree with the handler's body emptied: `tst_draft_targets.qml`
      reports 21 passed, 18 failed, and the four tests named above are among
      the green ones. The comment now says which tests go red (those that move
      to another target and read or submit what its field holds) and that a
      test which enters text again, or returns to the only target it wrote in,
      does not. It carries no count, since the tester adds to that file next.

- [x] **`dev-writer`** — `design.md:123-124` — "sixteen tests in
      `tst_draft_targets.qml` go red, the cross-target ones of every requirement"
      is stale and overstates in the same breath.
      **Scenario:** same mutation as above gives 18, not 16 (the two tests added
      in the last commit, the key-injectivity test and the no-cap test, fail too),
      and four tests that cross targets stay green, so "of every requirement" does
      not hold either. The file's own rule is that a number comes from a command
      run against the tree in front of you.
      **Severity:** low. **Measured:** 18 failures on this tip.

      **Fixed** (`dev-writer`), in the commit that ticks this box. Measured on
      my tree, handler body emptied: 18 of the 37 test functions in
      `tst_draft_targets.qml` fail, and one more in
      `tst_publish_outcome_visits.qml`, which the old sentence did not mention.
      Decision 3 now describes the red tests by what they do, names the four
      cross-target tests that stay green and why, and tells the reader to run
      the two files for a count in place of printing one that the next added
      test makes stale. `tasks.md` 2.2 carried the same "cross-target tests"
      wording and now points at Decision 3.

- [x] **`dev-writer`** — `DComposer.qml:332, 356, 390` — the new local in
      `submit()` and the new parameter of `applyReply()` are both named
      `published`, which is also the name of this component's signal.
      **Scenario:** lines 390-391 read `root.clearDraftOf(published)` followed
      directly by `root.published()`: one name, two meanings, two lines apart, one
      of them a string key and the other a signal call. It works only because the
      signal is always reached through `root.`. A reader scanning for where the
      signal fires, or someone dropping the `root.` while editing, gets the
      string. qmllint does not warn (`qmllint-qt6 --unqualified disable -I
      dialectica-ui/src/qml dialectica-ui/src/qml/DComposer.qml` is silent), so
      nothing will catch it. A name that says what it holds, such as
      `publishedKey` or `namedKey`, removes it.
      **Severity:** low-medium; a naming defect, not a behaviour one.

      **Fixed** (`dev-writer`), in the commit that ticks this box: both are
      `publishedKey`. The architecture review raised the same name; one rename
      answers both.

- [x] **`dev-writer`** — `DComposer.qml:90-93` — `heldDraft()` returns
      `typeof held === "string" ? held : ""`, and nothing in the file says why a
      map this component fills only with strings needs the check.
      **Scenario:** a reader asks "why would it not be a string?" and cannot
      answer from the code. The reason is in `design.md` Decision 2 (a plain
      object inherits names such as `constructor`) and is undercut there by the
      same paragraph saying no key can be one, since every key begins with `[`.
      So it is either a guard worth one comment line or dead weight worth
      removing, and the file does not tell the reader which.
      **Severity:** low; stylistic, an absent "why" comment where a reader would
      ask.

      **Fixed** (`dev-writer`), in the commit that ticks this box: it is
      neither of the two. The check is what turns the `undefined` of a target
      with nothing held into the "" a string field can be assigned, which is
      the common case and not a defence against an inherited name. A two-line
      comment on `heldDraft` says so, and `design.md` Decision 2 no longer
      presents it as a guard.

- [x] **`tester`** — `tst_draft_targets.qml:622` and `:987` — the string literals
      carry raw invisible characters (U+202E, U+202C, U+200B, U+200D, U+FEFF),
      not escapes.
      **Scenario:** line 987 types a literal that reads `"safetextmore"` on
      screen (it holds a U+202E and a U+200B) and the next lines assert
      the warning says "contains 2 invisible character(s)". In an editor, a diff
      view or a review tool the literal reads `"safetextmore"`; the reader cannot
      see where the 2 comes from, a formatter or a paste can strip the characters
      and turn the test into a vacuous one, and a bidi override in source
      reorders whatever follows it on screen. Spelling them as
      backslash-u escapes with their hex digits would be checkable at a glance, and line 622's own comment already lists the
      characters by name. The pattern is copied from `tst_composer.qml:214, 252,
      317-319`, which is how this family spreads.
      **Severity:** medium for a test whose value rests on characters nobody can
      see; stylistic otherwise.

      **Fixed** (`tester`), in `tst_draft_targets.qml`. Both literals are
      backslash-u escapes now (`‮`, `‬`, `​`, `‍`,
      `﻿`); the positions were read off the old literals with
      `git grep -P` before the swap, not guessed. The every-character test
      also pins what a strip would remove: `typed.length` is 29 (counted by
      hand from the pieces, not read off the string), and each of the five
      invisible characters is asserted present, so a formatter that drops them
      now fails the test where it used to leave it passing over a plainer
      string. The warning test needs no new assertion: it already asserts
      "contains 2 invisible character(s)", which only the two escapes produce.
      `tst_composer.qml:214, 252, 317-319` were not touched; the finding names
      them as the source of the pattern, not as a defect to fix here.

- [x] **`tester`** — `tst_composer.qml:347` — the comment still says "A reviewer
      deleted `clearDraft()` from the stored arm", and `clearDraft` no longer
      exists: this change renamed it to `clearDraftOf`.
      **Scenario:** `git grep -n "clearDraft\b"` outside the archive returns only
      this line. Someone following the comment to the function finds nothing. The
      line is outside the diff but this change is what orphaned it.
      **Severity:** low; stale reference.

      **Fixed** (`tester`). The comment now says the reviewer deleted "the
      draft-clearing call (then `clearDraft()`, now `clearDraftOf()`)". It is
      still an account of what a reviewer once did, so it keeps the history
      and says the name is dead. `git grep -n -F "clearDraft()"` over
      `dialectica-ui` still returns that one line, now flagged as the old
      name, so a reader who follows it finds the explanation and not a gap.

- [x] **`tester`** — `tst_publish_outcome_visits.qml:812-820` — "This test used
      to assert the opposite of its first two halves, as a pin of what the view
      did while the spec was silent" narrates the previous state of the test.
      **Scenario:** the sentence is true on the day it is written and a changelog
      from then on; the specflow block asks for "do not append a changelog of
      what landed", and `git log` already carries it. The scenario names above it
      are what a reader needs.
      **Severity:** low; stylistic, and in line with this file's habit of
      narrating, so a preference rather than a defect.

      **Fixed** (`tester`). The sentence about what the test "used to assert"
      is gone; the comment now says what is true of the test today (the feed
      mounts one composer for every Stoa, so the test goes through the feed's
      controls, as the text could follow the user into B). The scenario names
      above it are unchanged.

## Clean areas

- **`DComposer.qml` structure.** The new section is one block, each function has
  one job (`targetKeyOf`, `heldDraft`, `holdDraft`, `showDraftOfTarget`,
  `clearDraftOf`), and the comment on the `draft` alias was rewritten to match.
  `clearOutcome()`'s comment no longer says the draft's fate is open.
- **Comments I ran.** "The two agree in every test above" (the `clearDraftOf`
  re-pointing test): replacing the body with `field.text = ""` turns exactly one
  test red, as stated. "Assigning `text` also resets the field's undo history":
  writing `text` in place of `insert()` makes `canUndo` false, as the undo test
  says. "Main.qml empties a screen's address before it sets the next one" matches
  `Main.qml:117` and the `enterOnly` primitive. `qmllint-qt6` is silent on the
  file.
- **Test file layout.** `tst_draft_targets.qml` has one section per requirement
  and the header's seven names match the spec's requirements apart from the
  outcome-visit one, which is held by the other file. Scenario names are quoted
  above the tests. The helpers (`visibleNodes`, `forumBridge`) are a second copy
  of `tst_publish_outcome_visits.qml`'s, which the file says; there is no shared
  helper module in `dialectica-ui/tests/` and `visibleNamed` already exists in
  four files, so I did not box it.
- **Tests renamed in `tst_publish_outcome_visits.qml`.** The new name says what
  the assertions now say, and the scenario list above it matches.

No mutation is left in the tree; each was reverted with `git checkout --`.
