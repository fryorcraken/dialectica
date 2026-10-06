# Re-review: spec-test (after the fix round)

Scope: `specs/thread-view/spec.md` and `proposal.md` as they stand, against
`dialectica-ui/tests/tst_thread_reply.qml`. Issue #177 re-read with
`gh issue view 177 --comments`: the owner's decision is "drop the text", editing
not built, which the spec still states; no requirement exceeds the issue.

- [x] **`tester`** — `tst_thread_reply.qml`, `statesReplyIsSigned` (the signed
      matcher) and `test_the_signed_matcher_accepts_what_states_it_and_refuses_what_does_not`
      **Scenario:** the negation clause is `(\bnot|\bnever|n't)\s+(yet\s+)?(be\s+)?signed`.
      `\bnot` needs a word boundary, so `cannot be signed` carries no negation
      the clause can see, and `not cryptographically signed` / `not always
      signed` carry a word between the negation and `signed`. Each is read as a
      statement that a reply is signed. The `notStates` table has `is not
      signed`, `never signed` and `isn't signed` but none of these shapes, so
      the table pins the three forms the regex was written for.
      **Measured:** mutation 1 below. Caption changed to `A reply cannot be
      signed.`; `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_thread_reply.qml` printed `Totals: 33 passed, 0
      failed`. Both signed-statement scenario tests survived a caption that
      says the opposite of the requirement. Severity: medium (a requirement
      with two scenarios, both unable to fail on a denial of its own subject).

      **Outcome (tester): fixed.** The negation clause is now
      `(\bnot\b|\bnever\b|\bno\b|\bcannot\b|n't)(\s+\w+){0,2}\s+signed\b`, so
      `cannot` and a word or two between the negator and `signed` are seen.
      `notStates` gained `A reply cannot be signed.`, `Your reply is not
      cryptographically signed.`, `Replies are not always signed.` and `Replies
      can't be signed.`, and `states` gained a trailing negation (`A reply is a
      signed record, not a private message.`) so the clause is not widened into
      refusing a true caption. Watched failing first, on the old matcher: the
      matcher test went red on the first three (and on the two subject rows of
      the next entry), nothing else. Mutation, after the fix: caption `A reply
      cannot be signed.` turns `test_the_open_composers_text_states_that_a_reply_is_signed`
      and `test_the_statement_that_a_reply_is_signed_survives_a_publish` red and
      no other test (predicted the same; observed the same). Not caught, and
      said so in the matcher's comment and in design.md Decision 3: a negator
      three or more words from `signed` ("Not every reply you publish here is
      signed.", measured passing as a statement).

- [x] **`tester`** — same matcher, subject not checked
      **Scenario:** `/\bsigned\b/i` accepts any text containing the word, about
      a reply or not: `Signed in as alice.` or `Your draft is signed off by the
      app.` rendered in the open group satisfies both signed scenarios. The
      spec's obligation is a statement that a *reply* is signed. The
      "survives a publish" test is the weaker for it, since any `signed` text
      that is in the group after a publish passes. Add to `notStates` a
      `signed` text with another subject and require the matcher to refuse it.
      Not run as a mutation: the same survival as above, by reading the regex.
      Severity: low-medium.

      **Outcome (tester): fixed, with a residual.** `statesReplyIsSigned` now
      also requires `\brepl(y|ies)\b`. `notStates` gained `Signed in as
      alice.` and `Your draft is signed off by the app.`; the old matcher
      accepted both (watched failing), the new refuses both. Mutation: caption
      `Signed in as alice.` turns the two signed-statement tests red and no other
      (predicted the same; observed the same). Residual, measured: `Your reply
      draft is signed off by the app.` names a reply, says `signed` of something
      else, and passes as a statement. A matcher cannot tell a subject from a
      mention without reading the sentence; it is stated in the comment and in
      design.md Decision 3 instead of growing a clause. The shipped caption is
      the one literal that has to satisfy it.

- [x] **`tester`** — `mentionsEditOrVersion`, the word list, prefixed and
      unlisted forms
      **Scenario:** every stem is anchored with a leading `\b`, so a prefixed
      form is not a word start: `A reply stays unchanged once published.`,
      `Replies are uneditable.`, `An unrevised reply...` pass. Denials by a
      word the list lacks also pass: `Replies are immutable.`, `A reply is
      permanent.`, `Once published a reply is final.`, `Publishing is
      irreversible.`, `A reply cannot be altered.` (`alter`), `Replies cannot
      be deleted or taken back.`. `test_the_matcher_flags_a_denial_as_well_as_a_promise`
      has eight denials, all built on a listed stem. The tests' header says the
      list is hand-maintained and names the tables as where a miss is added;
      these are misses a reader can name now, in the direction the proposal
      treats as the harder one to catch (the denial). Not mutated; read from
      the regex. Severity: low-medium, and the spec scenarios do forbid these
      statements, so the three claim scenarios cannot fail on them.

      **Outcome (tester): fixed in part, rejected in part.** Fixed: the stems
      take an optional `un` or `non` prefix (`unchanged`, `uneditable`,
      `noneditable`, `unrevised`), and the list gained `alter` and `immutab`.
      Six rows added to the denial table (`A reply stays unchanged once
      published.`, `Replies are uneditable.`, `Replies are noneditable.`, `An
      unrevised reply stays as it was written.`, `Replies are immutable.`, `A
      reply cannot be altered.`); watched failing first, all six let through by
      the old matcher, none now. Rejected: `permanent`, `final`, `irreversible`,
      `deleted or taken back`. They are denials by implication; a stem for each
      is the endless widening the brief rules out, and the next paraphrase
      would pass the same way. Measured, by adding them to the table and
      watching it go red: `Replies are final.`, `Replies are permanent.`, `Once
      published a reply is final.`, `Publishing is irreversible.`, `Replies
      cannot be deleted or taken back.`, `reread what it said before` and `You
      can go back to what it said before.` all pass the matcher (and `You can
      redo it` does not). They are listed in the matcher's comment and in
      design.md Risks, which is where the spec's unguarded reach is recorded.

- [x] **`tester`** — `test_text_core_supplies_and_the_draft_are_outside_the_requirement`
      **Scenario:** one test, two jobs. Its beside-the-rows assertions
      (lines 823 and 856) fail on any claim the screen authors beside the rows,
      not only on core text wrongly flagged. The test's name says it pins the
      scope; a failure there will be reported against the scope rule.
      **Measured:** mutation 2 below. A stray `Replies can be edited later.`
      Text beside the groups failed this test as well as
      `test_no_text_beside_the_thread_rows_makes_an_edit_or_version_claim`.
      It is not wrong, only noisy: separate the claim from the scope, or
      accept the double report. Severity: low.

      **Outcome (tester): fixed.** The two beside-the-rows assertions no longer
      ask the whole column to be clean. For each of the reason, the draft and
      the refusal they assert the walk reaches it, the matcher flags it
      unexempted, and the exempted walk no longer lists it. A stray claim beside
      the groups is therefore the beside-the-rows test's alone. Mutation, a Text
      `Replies can be edited later.` between the groups: only
      `test_no_text_beside_the_thread_rows_makes_an_edit_or_version_claim` goes
      red (predicted the same; observed the same; before this fix it was two).
      The new assertions can fail for their own reason: skipping the shut gate
      in `textsBesideTheRows` turns the scope test red on `the walk beside the
      rows reaches core's reason` (and the beside-the-rows test on its anchor);
      an exemption that exempts nothing turns it red on the group assertion.

## Mutations run (all reverted, `git status --short` empty after each)

1. **Caption `A reply cannot be signed.`** in
   `dialectica-ui/src/qml/DThreadScreen.qml` (the `replyCaption` Text).
   **Survived**: `Totals: 33 passed, 0 failed`, from `sh
   dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_thread_reply.qml`.
   The first finding.
2. **A stray `Text { text: "Replies can be edited later." }` between the open
   and shut groups**, in the content column, in neither group. **Killed** by
   `test_no_text_beside_the_thread_rows_makes_an_edit_or_version_claim`
   (`an open gate: ... Actual (): [Replies can be edited later.]`) and by
   `test_text_core_supplies_and_the_draft_are_outside_the_requirement`. The
   group walks, which cannot see it, stayed green: the walk beside the rows is
   what catches it, as designed. Same command.

No mutation is left in the tree.

## Clean, by reading and by those runs

- **Scenario coverage.** Every scenario has a test at a layer that can see it:
  open, after publish and shut-gate claim scenarios (the three group tests);
  signed, and signed after publish (the two signed tests). The publish
  scenario's re-read is asserted by `publishedScreen` (`outcome == "stored"`,
  `items.length == 2`), so a caller cannot walk the pre-publish tree.
- **Exclusion of core-supplied text and the draft by exact string.** A claim
  cannot hide behind it. The exemption is `suppliedText.indexOf(text) === -1`
  on a string the test itself chose, passed only in the one scope test. A
  screen-authored sentence differing by a character, or concatenated with
  core's text, is not exempt, and would fail as a false positive, not pass as a
  false negative. The test shows the strings are rendered verbatim and that
  the matcher flags them, so the scope and not a weak matcher spares them, in
  both directions.
- **The walk beside the rows** reaches text in neither group (mutation 2), has
  its own non-vacuity (`rowCount > 0`, the per-state anchor, `edited` absent)
  and is pinned against the revised rows by
  `test_the_walk_does_not_reach_the_thread_rows_that_report_a_revision`.
  Residual, unreachable by this suite: a Text that is not a descendant of the
  content column, and text in a popup or tooltip (`collectTexts` reads
  `children`); the test's comment says so.
- **The claim matcher's tables** are hand-written, none read off the screen,
  and the truthful table is the screen's whole copy; the claim walks ignore
  `visible` and the signed walks use `renderedOnly`, each pinned
  (`test_the_claim_walk_reaches_a_text_that_is_hidden`).
- **No `NO SPEC:` markers** in the test file. One unmarked pin: the tests
  require the caption (when present) to lie inside `replyComposerOpen`, which
  the spec does not say (it says "rendered with the reply composer"). Harmless
  and stated in the test; the spec-writer may want a clause if the placement is
  meant to be a contract.
- **Spec sound.** The two requirements are consistent with each other, with
  the proposal and with the issue; no scenario is untestable as written. No
  requirement moved between capabilities, so part 4 does not apply.
