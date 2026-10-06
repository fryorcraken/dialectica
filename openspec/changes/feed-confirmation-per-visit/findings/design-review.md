# Design review: feed-confirmation-per-visit

The code takes every recorded decision. `Main.qml` `onScreenShownChanged` calls
`feed.beginVisit()` / `thread.beginVisit()` (Decision 1); each calls the
composer's `clearOutcome()`, which writes `outcome` and `outcomeDetail` and not
the draft (Decisions 2 and 3); `DComposer` names the element
`root.kind + "OutcomeMessage"` (Decision 4); nothing in `reload()` touches the
composer (Decision 5). The alternatives in Decision 1 are real ones, each with
what ruled it out. The issue's "unverified" cause is confirmed in Context. The
suite passes (21 tests) on this tree. I did not re-run the mutations: the
harness denied an edit to `Main.qml`, so the measured claims below are unchecked
by me.

- [x] **`dev-writer`** — `design.md:70-75` the mutation evidence is stale against
      the suite. It says "the six absence tests in `tst_publish_outcome_visits.qml`
      fail" with the hook removed, and "turns seven tests red" for the
      `reload()` mutation. The suite now has 21 tests, with absence tests added
      after the design was written (moderation return, thread failed read, reply
      in another thread, every reply outcome, and others). Re-measure both
      numbers against the final tree and rewrite them, or name the tests instead
      of counting them. **Not verified by me:** the mutation edit was denied.
      **Fixed** in the commit that ticks this box ("Design: re-measure every
      mutation claim against the final suite"). Re-measured fresh on the final
      tree, one edit at a time, each followed by a run of
      `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_publish_outcome_visits.qml`
      and then `git checkout --`. Decision 1 now names the tests instead of
      counting them. Removing the `feed` branch reddens six feed absence tests,
      removing the `thread` branch reddens four reply absence tests, and
      removing the whole hook reddens exactly those ten. Each fails at its
      absence assertion. `test_a_publish_on_the_later_visit_displays_its_own_outcome`
      stays green without the hook, and the decision says why. In Decision 5
      the claim "passes every absence test" was false and is gone. Clearing in
      `FeedScreen.reload()` reddens every feed test that publishes and then
      asserts presence, and the absence tests among them fail at their presence
      assertion. The `DThreadScreen.reload()` equivalent is now measured too.
      Decision 5 describes the red sets by class and names the tests that are
      not absence tests, so the text does not go stale when a test is added.

- [ ] **`dev-writer`** — `design.md:89-97` the cross-Stoa draft hazard is
      recorded as "a behaviour question for the spec" but is deferred nowhere.
      Submitting publishes Stoa A's text into Stoa B, which is a harm beyond the
      stale-message defect the issue reports. A deferral needs a destination:
      name the GitHub issue it is filed under, or file one and cite it. As it
      stands the only trace is a `NO SPEC:` test, which says what happens, not
      that anyone meant to leave it. Also say whether the reply composer's
      identical shape (`parentOp` follows `threadId`, "not tested") is in that
      same issue.

- [x] **`dev-writer`** — `design.md` Decision 2 and Decision 3 are guard
      decisions with no mutation evidence. Decision 2 should say which test
      turns red if `clearOutcome()` also clears the draft
      (`test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`,
      presumably) and Decision 3 whether anything fails if `clearOutcome()`
      clears only one of the two properties. Suggestion, not a contradiction.
      **Fixed** in the commit that ticks this box, and measured. Decision 2: if
      `clearOutcome()` also empties the draft, exactly the two `NO SPEC:` draft
      tests go red,
      `test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`
      and `test_a_draft_typed_in_one_stoa_is_still_held_and_published_in_another`.
      Decision 3: clearing only `outcomeDetail` reddens the same ten absence
      tests as removing the hook. Clearing only `outcome` leaves all 21 results
      green. That half is satisfied by construction, not by a test. Every writer
      of `outcome` (`applyReply`'s two success branches and `refuse`) also
      writes `outcomeDetail`, and `DPublishOutcome` renders `detail` only while
      the outcome is "refused". Decision 3 and `tasks.md` 2.1 now say this
      rather than implying a test pins it.

- [x] **`dev-writer`** — `design.md` Decision 1, last paragraph, claims a
      transition's intermediate `screenShown` values are harmless, and only the
      `openThread` case is argued. The return paths (thread to feed, moderation
      to feed) are not: the order in which `reading`/`moderating` and `chosen`
      are set decides whether `screenShown` passes through "list" or goes
      straight to "feed", and a test exists for the moderation return. State the
      return-path orderings, or say the claim rests on the tests rather than on
      reading the setters. Suggestion.
      **Fixed** in the commit that ticks this box, both by tracing and by a
      probe. Traced: `enterOnly` writes `previewing`, `chosen`, `reading`,
      `moderating` in that order, and `screenShown` tests `moderating` and
      `reading` before `chosen`. On a return, writing `chosen` while `reading` or
      `moderating` is still set leaves the value unchanged, and the next write
      moves it straight to "feed". Probed: a scratch spec in `./tmp/`, not
      committed, drove each transition on `Main.qml` and recorded every
      `screenShownChanged` value. `closeThread` and `closeModeration` each
      produced only "feed". `openThread` produced "list" then "thread", and
      `moderateIn` produced "list" then "moderation". `open` and `closeFeed`
      each produced one value. A deliberately wrong expectation for
      `closeThread` failed, so the comparison is not vacuous. Decision 1 now
      carries the table, the reason for the asymmetry, and what the tests can
      and cannot see of it. They see where a transition lands, not what it
      passes through.

## Re-review

Reviewed `git diff c933da11...HEAD` of `design.md` and `tasks.md` against the
code and the final suite (21 tests). Every edit was made, run with
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_publish_outcome_visits.qml`
and reverted with `git checkout --`; the tree is clean. None was denied.

Verified true on this tree:

- Removing the `feed` branch of `Main.qml`'s hook reddens exactly the six tests
  Decision 1 names, each at its absence assertion.
- Removing the `thread` branch reddens exactly the four named. The thread
  failed-read test fails at line 548 in that run, after the retry.
- `test_a_publish_on_the_later_visit_displays_its_own_outcome` stays green with
  the `feed` branch removed, as stated.
- Decision 2: adding `root.draft = ""` to `clearOutcome()` reddens exactly the
  two `NO SPEC:` draft tests.
- Decision 3: clearing only `outcomeDetail` reddens the ten absence tests;
  clearing only `outcome` (dropping the `outcomeDetail` line) leaves all 21
  green. Every writer of `outcome` in `dialectica-ui/src/qml/` is in
  `DComposer.qml` (lines 298, 314, 334) and each writes `outcomeDetail` on the
  next line, so "satisfied by construction" holds. `DJoinScreen.outcome` is a
  different property on a different component.
- Decision 5: `composer.clearOutcome()` first in `FeedScreen.reload()` turns 11
  tests red, including all four named and the feed absence tests at their
  presence assertions. `replyComposer.clearOutcome()` first in
  `DThreadScreen.reload()` turns 6 red, including the two named.
- The return-path table matches the setters: `enterOnly` writes in
  `stateNames` order and `screenShown` tests `moderating`, `reading`, then
  `chosen`, so `closeThread` and `closeModeration` go straight to "feed" and
  `openThread` and `moderateIn` pass through "list". I traced it; I did not
  rerun the author's scratch spec, which was not committed.
- `tasks.md` rows 1.2, 2.1, 2.3 and 2.4 say nothing the tree does not do.

The two boxes above that carry a "Fixed" note (Decisions 2 and 3, and the
return-path table) are confirmed by this pass and can be ticked. The first-round
cross-Stoa box is the owner's and is left as it was.

- [x] **none** — re-reviewed the design delta; no new findings
