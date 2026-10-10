# spec-test review: draft-belongs-to-target

Scope: the delta `specs/composer-view/spec.md` read with the live
`openspec/specs/composer-view/spec.md`, `dialectica-ui/tests/tst_draft_targets.qml`
and the diff of `tst_publish_outcome_visits.qml`. The implementation was read
only at the lines mutated.

- [x] **`tester`** — `tst_draft_targets.qml`, requirement *A draft whose composer
      is not rendered stays held and is displayed nowhere*, the clause "whether
      because the posting probe reports posting is not possible for its Stoa or
      because a read failed". The shut-gate tests assert that no draft field is
      rendered and that the text is displayed nowhere. The failed-read case
      (`test_a_draft_is_back_when_a_failed_read_recovers`) asserts only the
      recovery: it never checks, while `feedReadState` / `threadReadState` is
      `failed`, that the draft text is displayed nowhere or that no draft field
      is rendered.
      **Scenario:** a change that leaves the draft visible during a failed read,
      for example as text in an error banner or a field left mounted, passes
      every test, because the only assertion on that state is made after the
      retry succeeds.
      **Measured:** not mutated. Read from the test body, lines 865-893. The
      requirement's "MUST NOT display its text anywhere" holds for both causes,
      and only one is pinned. Severity: low to medium. Add the shut-gate test's
      two assertions to the failed state, for the post and the reply case.

      **Fixed, with one part of the request not reachable as written**
      (`tester`). `test_a_draft_is_back_when_a_failed_read_recovers` now
      asserts, while `feedReadState` / `threadReadState` is `failed`, that the
      number of displayed nodes carrying the draft equals the number of draft
      fields rendered: the draft is displayed nowhere but in a composer's
      field. Measured before writing it: **the feed's post composer is still
      rendered through a failed list read** (`FeedScreen.qml` gates it on the
      posting probe alone, and the probe runs before the list read), so for
      the post case the shut-gate pair "no field, no text" is false of the
      code and of the spec, whose clause reads "While no composer for a
      draft's target is rendered". The thread screen does stop rendering its
      reply composer (0 fields, 0 nodes with the text). So the assertion is
      the one that holds in both: text only in rendered fields. It is
      deliberately silent on whether a screen keeps its composer through a
      failed read, which the spec does not decide.
      **Fails without it:** in `DThreadScreen.qml`, the failure message's
      `text: screen.failure` became `screen.failure + " " +
      replyComposer.draft`: predicted the reply case fails, observed
      `reply: ... 0 composer field(s) rendered and nowhere else`, actual 1,
      expected 0. The same edit to the feed's first failure-banner `Text`
      (`... can be shown." + composer.draft`): predicted the post case fails,
      observed actual 2, expected 1. Both reverted. A mutant that merely
      keeps the thread screen's composer mounted through the failure is **not**
      caught, and is not a spec violation: a rendered composer holding its own
      target's draft is allowed. **Spec observation for the owner, not acted
      on:** the clause "because a read failed" holds for the thread screen
      only; the feed's post composer survives a failed read.

- [x] **`tester`** — `tst_draft_targets.qml:1042`
      `test_two_targets_whose_parts_run_together_alike_do_not_share_a_draft`. The
      separator sweep is a hand-written list of eleven strings, and the
      injectivity claim it makes ("the key must not depend on" the separator)
      holds only for those.
      **Scenario:** a key built by joining the three parts with a character
      outside the list is non-injective for a peer-supplied parent containing
      that character, and this test does not notice.
      **Measured:** mutation in `DComposer.qml` `targetKeyOf`, replacing
      `JSON.stringify([kind, stoaAddress, parent])` with
      `[kind, stoaAddress, parent].join("\u0001")`. Run with
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_draft_targets.qml`:
      **39 passed, 0 failed. The mutant survived.** Severity: low. A joined key
      cannot collide through `Main.qml` today, because a Stoa address is 64 hex
      characters. The test exists to hold that independence, and it does not hold
      it for an unlisted separator. Sweep every code unit 0 to 127, or build the
      colliding pair from the characters of the two parts instead of from a list.

      **Fixed** (`tester`). The test now sweeps every UTF-16 code unit,
      0 to 0xFFFF, and the empty separator, building the colliding pair from
      the separator in hand (`"a"+s+"b"` with `"c"`, against `"a"` with
      `"b"+s+"c"`), on one composer. It costs about eight seconds, which is
      most of the file's runtime; it is the only way to reach whichever
      character a mutant picks. **Fails without it:** the reviewer's mutant,
      `[kind, stoaAddress, replyParent].join("\u0001")` in `targetKeyOf`'s
      place: predicted fail at U+1, observed `separator U+1: the second target
      holds 'for the first, separator U+1'`. A second mutant with a character
      the old list could never have held, `.join("�")`: predicted fail at
      the far end of the sweep, observed `separator U+fffd: the second target
      holds ...`. Reverted.

      **New scenario, checked.** *Targets whose address and parent read alike
      when joined do not share a draft* says "any other single character, or
      none" and asserts empty-then-restored; the test asserts the same two
      things per separator.

- [x] **`spec-writer`** — `specs/composer-view/spec.md`, requirements *A publish
      clears only the draft of the target it named* and *An unsubmitted draft is
      kept for its target while the view stays open*. Tests pin behaviour the
      scenarios do not describe, and none carries a `NO SPEC:` marker:
      (a) the draft survives a visit to the moderation screen
      (`test_a_post_draft_is_back_after_the_moderation_screen_was_visited`),
      which rests on the prose "whichever screens are rendered meanwhile" alone;
      (b) a publish answered after the composer was re-pointed clears the target
      it named, not the one now shown
      (`test_a_publish_answered_after_the_composer_was_re_pointed_clears_only_what_it_named`),
      which rests on the first sentence of the requirement and is reachable only
      by driving `DComposer` directly, since no screen can re-point a composer
      between a submit and its answer;
      (c) 600 drafts held at once and a separator sweep, both from prose.
      **Scenario:** a later edit to the spec drops or rewords the prose, and the
      tests that were its only pin look like unowned tests to the next reader.
      **Measured:** read only. Severity: low. Either add a scenario for (a),
      and for (b) a line saying that clause is pinned at the composer layer
      because the view layer cannot observe it, or say they are deliberately
      unscenario'd.

      **Outcome (`spec-writer`): fixed**, in the delta
      `specs/composer-view/spec.md`. Each of the four tests now has a scenario,
      so none needs a `NO SPEC:` marker.
      (a) Scenario *A post draft is back after the moderation screen was
      visited*, under *An unsubmitted draft is kept for its target while the
      view stays open*.
      (b) Scenario *A publish that reports after its composer was pointed
      elsewhere clears the target it named*, under *A publish clears only the
      draft of the target it named*. It is worded on a composer being pointed
      at a target, the only place the behaviour can be observed. The spec does
      not say why no screen reaches it (the core call is synchronous); that is
      reasoning, and belongs in `design.md` if it is not there already.
      (c) Scenario *Drafts for six hundred targets are all held at once*, under
      the same retention requirement. For the separator sweep, a new
      requirement, *Two targets that differ never share a draft*, states when
      two targets are the same target, with the scenario *Targets whose address
      and parent read alike when joined do not share a draft*. It is its own
      requirement because `openspec validate --strict` warns on a requirement
      text over 500 characters. That scenario says
      "any other single character, or none", so the hand-written list of eleven
      separators falls short of it; the `tester` finding above this one is the
      entry that closes that.
      For the `tester`: the four tests can cite their scenarios by name, and the
      section heading "Requirements stated in the spec's own words, past its
      scenarios" in `tst_draft_targets.qml` no longer describes them. No
      assertion needs to change on account of this entry.

Clean, checked by reading and, where stated, by mutation:

- **Coverage.** Every scenario of the seven ADDED requirements has a test, 36
  scenarios in all (52 in the delta, less the 16 under MODIFIED). One scenario,
  "A post draft is back when the same Stoa is reopened", is covered by the retained
  `test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened` in
  `tst_publish_outcome_visits.qml`, which the section comment of
  `tst_draft_targets.qml` says. The "post and reply draft in one Stoa" scenarios
  are satisfied by `Main.qml` mounting two composer instances, so they pin the
  behaviour and not the target key's `kind` element. That is not a coverage gap.
- **Layer.** The QML component layer drives `Main.qml` through a fake forum, and
  the overlay says that layer sees the navigation. Nothing here needs a real
  Basecamp.
- **The MODIFIED requirement.** Against the live text, only the closing paragraph
  differs, and it now names the two requirements that decide a draft's fate. The
  test diff changes comments and the cross-Stoa test, which flips from asserting
  the defect to asserting its absence. The other scenarios are unchanged.
- **Mutation 1.** Dropped `stoaAddress` from the key in `targetKeyOf`: 14 of 39
  tests failed, including the cross-Stoa, same-op-id, re-pointed-composer, undo,
  restore-warning and over-length tests. Killed, by the tests that name the
  property. `test_two_targets_whose_parts_run_together_alike_do_not_share_a_draft`
  passed under it, correctly, since that test varies the parent too.
- **Mutation 2.** The surviving `\u0001` join, above.
- **Both mutations are reverted**; `git status` was empty after the last
  `git checkout -- dialectica-ui/src/qml/DComposer.qml`.
- **`NO SPEC:` markers.** None in `tst_draft_targets.qml`. The two in
  `tst_publish_outcome_visits.qml` were removed by this piece, as the issue
  asked. The `NO SPEC` hits elsewhere in `dialectica-ui/tests/` predate this
  piece and belong to other capabilities.
- **Moved requirements.** None: no `REMOVED` section, so part 4 does not apply.
- **Spec soundness.** Self-consistent with the live *draft is cleared when the op
  was newly stored, and kept otherwise*, *A refused publish keeps the draft* and
  the length-gate requirements. Every scenario is checkable at the QML layer. The
  issue's two open questions (which target owns a draft, and whether a draft
  survives leaving) are both decided, and the issue asks for the reply composer's
  untested cross-thread case to be pinned, which three tests do. No requirement
  covers a scope the issue no longer states.
