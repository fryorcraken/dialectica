# Readability findings: reply-caption-no-edit

Dimension: readability only. Diff read with `git diff origin/main...HEAD`.

- [ ] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:562` — `stringsUnder` is a second copy of `collectTexts` (line 388)
      **Scenario:** `collectTexts(item, out)` (pre-existing on `origin/main`) and the new
      `stringsUnder(item, out)` have identical bodies: push `item.text` when it is a string,
      recurse over `item.children`. Two functions with two names for one walk in one file means
      a fix to one (for example, a change to what counts as text, or to descend into popups,
      which the comment above `stringsUnder` says the walk cannot reach) is not made to the
      other, and a reader has to compare bodies to learn they are the same. Reuse
      `collectTexts` from `editClaimsUnder`, or delete it in favour of the new name.
      **Severity:** low-medium, a genuine defect (duplication), not taste.
      **Measured:** `git grep -n -E "function (collectTexts|stringsUnder)" HEAD -- dialectica-ui/tests`
      returns both definitions, 174 lines apart.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DThreadScreen.qml:725-733` — the caption's comment narrates change history and points at a document that moves
      **Scenario:** "this caption carried the first until `thread-view` forbade it. The
      reply-caption-no-edit change's design.md says why the first survived the cut." This
      records what the code used to be, which is the commit's job and `CLAUDE.md`'s "write down
      only what a command cannot tell you" cuts against it; and it cites a change by name and
      `design.md` by bare filename, where `openspec archive` moves that folder to
      `changes/archive/<date>-reply-caption-no-edit/`, so the pointer reads as a live path and
      resolves to nothing after the closer runs. The part worth keeping is the standing rule
      ("nothing on this screen edits a reply, so no edit or version claim belongs here") and the
      reason a reader asks "why": why the caption was kept at all. Keep the rule, drop the
      history sentence, and cite the capability (`thread-view`) or the archived path rather than
      the in-flight change.
      **Severity:** low. Genuine defect (comment that rots), partly taste.

- [ ] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:462` — `claimsEditing` (and `editClaimsUnder`, line 569) are named for a narrower job than they do
      **Scenario:** the regex also matches `\bversions?\b`, and the tests rely on that: "earlier
      versions stay readable" and "Every version of a reply is kept" are asserted flagged in
      `test_the_matcher_flags_a_claim_that_an_earlier_version_can_be_read`. A reader of
      `claimsEditing("earlier versions stay readable")` expecting an edit claim cannot tell why
      it is true without reading the regex; the doc comment says it, the name does not. Same
      for `editClaimsUnder`, which also returns every collected `texts` and so is a walk plus a
      filter. A name covering both claim families (`promisesEditOrVersion`) would let the
      call sites read as the requirement does.
      **Severity:** low, stylistic.

- [ ] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:443` — the comment cites a test by a truncated name, `test_the_walk_does_not_reach_the_thread_rows_...`
      **Scenario:** the real function is
      `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision` (line 624). The
      ellipsis form matches nothing under `git grep -F` as written, so a reader following the
      pointer has to guess the suffix; spell it in full.
      **Severity:** low, stylistic.

Clean, in prose:

- The production change in `DThreadScreen.qml` is two lines of behaviour (the caption text and
  an `objectName`), minimal and in the file's existing style. `objectName` follows the sibling
  `replyComposerOpen` / `replyGateShut` naming, and the `// **bold**` comment lead is idiomatic
  across `dialectica-ui/src/qml/`.
- The matcher tests are well organised (one per family of claim, one for what must be left
  alone, a scope test that explains why the walk is scoped), and each comment says why rather
  than what. Hand-written inputs are labelled as such.
- No stale copy of the removed caption remains outside the tests' deliberate negative examples:
  `git grep -n -i -E "can be edited|edited later|signed record"` hits only the new comment, the
  new caption, and the matcher's claim table.
