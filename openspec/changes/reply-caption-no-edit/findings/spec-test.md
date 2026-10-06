# spec-test review: reply-caption-no-edit

Read: `specs/thread-view/spec.md`, `proposal.md`, `dialectica-ui/tests/tst_thread_reply.qml`,
issue #177 with its comment. The implementation was read only at the lines
mutated (`DThreadScreen.qml` caption, `DPublishOutcome.qml` stored headline).

- [ ] **`tester`** — `tst_thread_reply.qml`, `claimsEditing` (line 462) and the three
      scenario tests that depend on it. The word list lets a paraphrased promise through.
      **Scenario:** the caption becomes "A reply is a signed record. You can fix typos
      later and replace it." Every scenario test passes, though the requirement forbids
      any statement that a reply can be edited or that a later version can be published.
      `fix`, `replac`, `undo`, `retract` and `newer`/`older one` are not in the pattern.
      The test comment admits the list is hand-maintained, but the matcher test's table
      has no row for any paraphrase outside the list, so adding one costs nothing and
      nothing prompts it.
      **Measured:** mutation 2, caption set to that text, `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_thread_reply.qml` printed `Totals: 27 passed, 0 failed`.
      Severity: low to medium. Widen the list and add those strings to the matcher
      test, or say in the spec that the check is the word list.

- [x] **`spec-writer`** — the requirement body ("MUST NOT state that a reply can be
      edited") against what the tests pin. The spec forbids a claim, which is a semantic
      property; the tests pin the presence of listed words. The two differ in one case
      the spec allows and the tests refuse: a sentence that denies the claim ("A reply
      cannot be edited"). The test header says this is deliberate and tells a future
      editor to narrow the pattern, but no scenario or `NO SPEC:` marker records the
      choice, so it is an unmarked decision the tests pin.
      **Scenario:** an owner who wants honest copy ("Replies cannot be edited in this
      version") finds the spec permits it and three tests fail on the word `edit` and
      the word `version`.
      Severity: low. Either forbid mentioning the topic in the requirement, or add a
      scenario saying a denial is permitted and let the tester narrow the pattern.
      **Outcome (spec-writer): fixed in the spec, first option.** The requirement now
      forbids the topic in both directions: the text "MUST NOT state whether a reply
      can be edited ..., whether a later version ... can be published, or whether an
      earlier version ... can be read: neither that it can nor that it cannot". It is
      renamed to *The text around the reply composer says nothing about editing a reply
      or reading its earlier versions*, and its scenarios say "states whether" in place
      of "states that". Why a denial is forbidden rather than permitted: editing is
      planned work (#99), so a denial goes false when it lands with nothing to prompt
      its removal, which is the same stale-copy failure this issue fixed in the other
      direction; and the owner's decision was to drop the text, not to replace it with
      its negation. The word matcher now approximates the spec rather than exceeding
      it.
      **Follow-ups.** `tester`: the header in `tst_thread_reply.qml` (lines 430 and
      446-451, "stricter than the spec on purpose ... which the spec permits") is now
      false; rewrite it against the renamed requirement, and add a denial row ("A reply
      cannot be edited.") to a matcher test's flagged table so the both-directions rule
      is pinned. `dev-writer`: design.md Decision 3 (paragraph "The matcher is stricter
      than the spec, on purpose") and the third Risks bullet say the spec permits a
      denial; rewrite both, and record the reason above as a Decision. The renamed
      title also appears in `tasks.md` line 26.

- [x] **`spec-writer`** — `NO SPEC:` at `test_the_caption_beside_the_composer_is_kept`
      (line 728). The spec forbids a claim but says nothing on whether a caption
      remains, so the dev kept "A reply is a signed record." and the test fails if it is
      deleted or emptied. Route: say in the spec that the thread screen states what a
      reply is, or drop the test if removing the caption is acceptable.
      Severity: low. The test is the only thing that stops the caption being deleted
      with all the claim tests still green.
      **Outcome (spec-writer): fixed in the spec; the dev's choice was right.** Issue
      #177's expected behaviour is "the caption describes what a reply is", and the
      owner's decision chose the issue's option that keeps "A reply is a signed record."
      New requirement *The open reply composer states that a reply is signed*: where the
      gate is open, text rendered with the composer MUST state that a reply is signed,
      and MUST remain rendered after a publish. It is on the statement, not the string.
      Two scenarios: open gate, and after a publish.
      **Follow-ups.** `tester`: replace the `NO SPEC:` comment with the requirement's
      name; strengthen `test_the_caption_beside_the_composer_is_kept` from "non-empty"
      to "states a reply is signed" (for example, the composer group's text matches
      `/\bsigned\b/i`), so a caption reduced to unrelated copy fails; and add a test for
      *The statement survives a publish*, which nothing pins today. `dev-writer`:
      design.md Decision 1's last alternative and Decision 3's "removing the caption is
      a choice the `NO SPEC:` test reports" now refer to a marker that should be gone;
      point them at the requirement.

- [x] **`spec-writer`** — the shut-gate scenario ("text rendered in place of the reply
      composer"). That group renders core's `reason` verbatim, and `composer-view`
      ("A closed gate shows the reason verbatim and offers a fix") forbids the view to
      reword it. The new requirement does not say whether it binds text core supplies.
      The test uses one hand-written reason, "no keystore". A real reason containing
      `update`, `change` or `version` would trip the matcher on text the view may not
      alter, and the spec gives no answer on which wins.
      Severity: low. Name the scope (view-authored text only, or probe text too).
      **Outcome (spec-writer): fixed in the spec, view-authored text only.** The
      requirement binds "text the thread screen authors", and names two kinds of text
      it does not bear on: text core supplies and the view renders as supplied (the
      probe's reason, and core's message on a refused publish, which `composer-view`'s
      *A closed gate shows the reason verbatim and offers a fix* and *A refused publish
      keeps the draft and names the refusal* forbid the view to reword), and the draft
      the user typed. Every scenario now says "text the screen authors and renders".
      Binding probe text would put the two capabilities in conflict, and one rule
      belongs to one capability. The same gap existed in the open group, which this
      finding did not name: a refused publish renders core's message there, and the
      draft field is in the walk.
      **Follow-up.** `tester`: the walks in `editClaimsUnder` collect core-supplied text
      and the composer's draft. With hand-written fixtures this cannot fail spuriously
      today, but the scope is now contracted: either exclude the fixture's reason, any
      refusal message and the draft from the flagged set, or add a comment that the
      fixture's core strings are chosen to avoid the matcher and why that is safe.

## Coverage, per scenario

| Scenario | Test | Layer sees it |
|---|---|---|
| The open composer's text makes no edit or version claim | `test_the_open_composers_text_makes_no_edit_or_version_claim` | yes, component layer renders the text |
| No claim after a reply is published | `test_no_edit_or_version_claim_appears_after_a_reply_is_published` | yes |
| The shut gate's text makes no claim | `test_the_shut_gates_text_makes_no_edit_or_version_claim` | yes |

Each walk asserts it reached text before it asserts there is no claim, and the
matcher is pinned both ways by hand-written strings, so a `return false` matcher
fails three tests. The scope test pins why the walk excludes the revised marker and
the earlier-versions label, which the requirement says it does not bear on.

## Mutations run

1. `DPublishOutcome.qml` line 97, "stored" headline extended with " It can be edited
   later." Result: only `test_no_edit_or_version_claim_appears_after_a_reply_is_published`
   failed (`Totals: 26 passed, 1 failed`), and the open-composer test stayed green. The
   after-publish test therefore does something the open one cannot. Command:
   `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_thread_reply.qml`.
   Reverted.
2. `DThreadScreen.qml` caption extended with a paraphrased promise. **Survived**
   (finding 1). Reverted.

No mutation is left in the tree (`git status --short` was empty after the revert).

## Clean areas

- Spec cross-references resolve: "The earlier-versions affordance is inert, and
  inertness is the existing convention" and "A revised post is marked as revised, and
  the mark claims nothing about what changed" both exist in `openspec/specs/thread-view/spec.md`.
- No requirement moved between capabilities, so part 4 does not apply.
- Against issue #177 and the owner's comment, the editing clause is current. The
  version-reading and shut-gate clauses go beyond what the owner decided, which the
  proposal justifies from the bundle's caption; nothing the issue states has been dropped.
- The out-of-scope paragraph ("no method on the module surface publishes a revision")
  has no scenario and no test at this layer. It is a scope note, not a behaviour, and
  I found no spec naming such a method.
