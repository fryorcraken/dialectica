# spec-test re-review: draft-belongs-to-target

Range: `fb554d3d..HEAD` (four commits: `15939866` spec, `c43bc367` composer,
`c3df2067` tests, `f7dfbaf7` spec). Read: the delta
`specs/composer-view/spec.md` against the live `openspec/specs/composer-view/spec.md`,
`dialectica-ui/tests/tst_draft_targets.qml`, and the diffs of `tst_composer.qml`
and `tst_publish_outcome_visits.qml`, plus issue #203 and its comment, fresh.
The implementation was read only at the lines mutated and at the lines a test
quotes.

- [ ] **`tester`** — `dialectica-ui/tests/tst_draft_targets.qml:625` and
      `:1018-1019`. Both comments say the invisible characters are "**Spelled as
      escapes, not typed**" / "spelled as escapes because neither can be seen".
      They are typed. `git grep -n -P "\x{202E}|\x{200B}|\x{FEFF}"` finds the raw
      characters on lines 630, 632-634 and 1020, and `git grep -n -F "\u202"`
      finds no escape anywhere in the file.
      **Scenario:** the comment's own stated reason ("a formatter or a paste that
      stripped them would leave a test that passes over a plainer string") is the
      property it claims to provide and does not. The length pin on line 631
      (29) still catches a strip from `typed`, and the "contains 2 invisible
      character(s)" assertion on line 1021 catches one from the warning test, so
      the tests are not weakened. But the `indexOf` checks on lines 632-634 are
      vacuous against a strip that hit them too (`indexOf("")` is 0), and the next
      reader is told a protection exists that is not what is there.
      **Measured:** by reading and the two greps above; not mutated. Severity:
      low. Either write the characters as `‮`, `‬`, `​`, `‍`,
      `﻿` (and the two in line 1020) so the comment is true and the
      `indexOf` checks mean something, or reword both comments to say the length
      pin is the protection.

- [ ] **`tester`** — `dialectica-ui/tests/tst_draft_targets.qml:1089`
      `test_two_targets_whose_parts_run_together_alike_do_not_share_a_draft`. The
      new scenario says "any other single character, or none, in place of `:`".
      The sweep covers every UTF-16 code unit and none, which is a single
      character only for the BMP: a character outside it is two code units, and
      no pair here is built from one.
      **Scenario:** a key joining its parts with an astral character (for example
      U+1F3DB) maps `("a"+S+"b","c")` and `("a","b"+S+"c")` onto one string only
      when the separator in the pair is `S`. Every pair here uses a one-code-unit
      separator, so the strings differ and the test passes over it.
      **Measured:** by reading; not mutated (the two-mutation budget was spent
      below). Severity: low; unreachable through `Main.qml`, since a Stoa address
      is 64 hex characters, which is the test's own stated reason for driving a
      composer. Add a few astral separators to the sweep, or have the spec say
      "any single UTF-16 code unit" so the scenario is the one the test pins.

- [ ] **`tester`** — `dialectica-ui/tests/tst_draft_targets.qml:913-916`
      `test_a_draft_is_back_when_a_failed_read_recovers`, the new failed-state
      assertion. It compares the **number** of displayed nodes carrying the draft
      with the number of draft fields rendered. The requirement's clause is that
      the text is displayed nowhere "but in the field of a composer rendered for
      its target", which is about which node, not how many.
      **Scenario:** a rendered field that does not hold the draft (empty, or
      another target's text) beside a banner that does gives one matching node and
      one field, and the counts agree. The assertion is green over a display of the
      draft in a banner.
      **Measured:** by reading; the round-one outcome's own mutants (a banner
      appended to the thread screen's failure text, and to the feed's) do fail it,
      because there the field also still holds the draft, so each adds a second
      node. Severity: low; the implausible shape is a field that lost its text and a
      banner that gained it. Assert that every node `textsShownContaining` matches
      is named `<kind>DraftField`, which says the clause as written.

Mutations run, both with `sh dialectica-ui/tests/run-qml-tests.sh
dialectica-ui/tests/tst_draft_targets.qml`, baseline 41 passed, 0 failed:

- **Dropped `root.kind` from `targetKey`** (`DComposer.qml:76`). 1 failed, 40
  passed: `test_a_post_and_a_reply_to_the_empty_parent_do_not_share_a_draft`
  ("the same address and parent, as a reply": actual `meant as a post`, expected
  empty), and only that one. **Killed**, by the new test the range added for the
  clause the other tests cannot see; round one found the two-instance scenarios
  pass without `kind` in the key.
- **`clearDraftOf` cleared the field whatever the key**
  (`DComposer.qml:134`, `if (key === root.targetKey)` removed). 1 failed, 40
  passed: `test_a_publish_answered_after_the_composer_was_re_pointed_clears_only_what_it_named`
  at line 806 ("the draft of the Stoa now shown was not the one published, and is
  kept": actual empty, expected `meant for B`). **Killed**, by the test the new
  scenario pins.

**Both mutations are reverted** (`git checkout -- dialectica-ui/src/qml/DComposer.qml`);
`git status --short` was empty afterwards. None is in the tree or in a commit.

Clean, by reading unless stated:

- **Coverage of the range.** Every requirement and scenario added or reworded
  after `fb554d3d` has a test that pins its wording: *Two targets that differ never
  share a draft* and its scenario (the sweep, the post-against-reply-to-empty-parent
  test, the post-with-a-parent test); *A post draft is back after the moderation
  screen was visited*; *Drafts for six hundred targets are all held at once*; *A
  publish that reports after its composer was pointed elsewhere clears the target
  it named* (all three THEN clauses are asserted: the call's address and body,
  the second Stoa's field, the first Stoa's empty field); the reworded *A draft
  whose composer is not rendered* requirement; and both *failed read recovers*
  scenarios (post and reply), each with the failed-state clause and the two
  recovery clauses. The reply case also asserts `threadReadState` is `failed`
  and the post case `feedReadState`, so neither passes on a read that did not fail.
- **Layer.** The scenarios needing a composer pointed mid-publish or a
  separator that no address contains are driven on `DComposer` directly, which
  the delta words as a composer being pointed at a target; the QML component
  layer sees it. Nothing here needs a real Basecamp.
- **Round one's outcomes do what they say.** The sweep is a code-unit sweep and
  passes (the suite's 41 tests ran in 14 s); the four prose-only tests each
  carry a `Scenario:` line and the stale section heading was replaced; the
  failed-state assertion exists and the spec was reworded to say what it
  asserts. The round-one note that a feed composer survives a failed read is now
  in the spec as "not decided here", and the test does not assert either way.
- **Every changed test pins something the spec states.** The
  `applyReply`-without-a-key test in `tst_composer.qml` calls the function in a
  form no screen uses, but what it asserts (a stored reply leaves no draft shown
  and none held for the target) is the live *The draft is cleared when the op was
  newly stored* scenario and the delta's *A published draft is not brought back*;
  it cites `design.md` for the call shape, not for the behaviour. The comment-only
  edits in `tst_composer.qml` and `tst_publish_outcome_visits.qml` name functions
  and facts that exist (`clearDraftOf`). `git grep NO SPEC` over the three test
  files returns nothing.
- **Self-consistency and staleness.** The new requirement's definition of "the
  same target" agrees with *A draft belongs to the target* (post: address; reply:
  address and parent) and with the issue's decision 2. *Whether a failed read stops a
  screen rendering its composer is not decided here* agrees with the modified
  outcome requirement's "where a read fails and the screen stops rendering the
  composer". The six-hundred scenario is issue decision 7 (no cap). No requirement
  covers a scope the issue no longer states. No requirement moved between
  capabilities.
