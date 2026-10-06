# Readability findings: reply-caption-no-edit

Dimension: readability only. Diff read with `git diff origin/main...HEAD`.

- [x] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:562` — `stringsUnder` is a second copy of `collectTexts` (line 388)
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
      **Outcome (tester): fixed**, the same fix as architecture finding 1:
      `stringsUnder` is deleted and every walk is `collectTexts`, with an optional
      `renderedOnly` argument. See `findings/architecture.md`.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DThreadScreen.qml:725-733` — the caption's comment narrates change history and points at a document that moves
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
      **Fixed** in the commit "Record why the reply caption may not deny editing
      either, and drop the caption comment's history (#177)": the history sentence
      ("this caption carried the first until ...") and the pointer to the in-flight
      change are gone. The comment keeps the standing rule, now in both directions as
      the moved spec requires ("promise or denial"), says why the caption is kept
      (`thread-view` requires the text to state a reply is signed), and cites
      `thread-view`. Comment-only; `tst_thread_reply.qml` stays 27 passed, 0 failed,
      and the three static QML gates pass.

- [x] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:462` — `claimsEditing` (and `editClaimsUnder`, line 569) are named for a narrower job than they do
      **Scenario:** the regex also matches `\bversions?\b`, and the tests rely on that: "earlier
      versions stay readable" and "Every version of a reply is kept" are asserted flagged in
      `test_the_matcher_flags_a_claim_that_an_earlier_version_can_be_read`. A reader of
      `claimsEditing("earlier versions stay readable")` expecting an edit claim cannot tell why
      it is true without reading the regex; the doc comment says it, the name does not. Same
      for `editClaimsUnder`, which also returns every collected `texts` and so is a walk plus a
      filter. A name covering both claim families (`promisesEditOrVersion`) would let the
      call sites read as the requirement does.
      **Severity:** low, stylistic.
      **Outcome (tester): fixed, with a different name than suggested.**
      `claimsEditing` is `mentionsEditOrVersion`, not `promises...`: the
      requirement forbids a denial too (design.md Decision 4), so the matcher flags
      a mention, and "promise" would name half of what it checks. `editClaimsUnder`
      is gone with `stringsUnder`; its two jobs are `collectTexts` (the walk) and
      `mentionsAmong` (the filter), composed by `mentionsUnder`, which returns
      both `texts` and `flagged` for the callers that anchor on the former.
      design.md carries the new names (`git grep -F claimsEditing` over the change
      folder and the test returns only the findings' own text).

- [x] **`tester`** — `dialectica-ui/tests/tst_thread_reply.qml:443` — the comment cites a test by a truncated name, `test_the_walk_does_not_reach_the_thread_rows_...`
      **Scenario:** the real function is
      `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision` (line 624). The
      ellipsis form matches nothing under `git grep -F` as written, so a reader following the
      pointer has to guess the suffix; spell it in full.
      **Severity:** low, stylistic.
      **Outcome (tester): fixed.** The comment spells
      `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision` in
      full. The new comment cites `test_text_core_supplies_and_the_draft_are_outside_the_requirement`
      in full too; checked with `git grep -F` for each name in the test file.

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
