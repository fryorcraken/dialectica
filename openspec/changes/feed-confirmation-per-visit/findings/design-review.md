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

- [ ] **`dev-writer`** — `design.md:70-75` the mutation evidence is stale against
      the suite. It says "the six absence tests in `tst_publish_outcome_visits.qml`
      fail" with the hook removed, and "turns seven tests red" for the
      `reload()` mutation. The suite now has 21 tests, with absence tests added
      after the design was written (moderation return, thread failed read, reply
      in another thread, every reply outcome, and others). Re-measure both
      numbers against the final tree and rewrite them, or name the tests instead
      of counting them. **Not verified by me:** the mutation edit was denied.

- [ ] **`dev-writer`** — `design.md:89-97` the cross-Stoa draft hazard is
      recorded as "a behaviour question for the spec" but is deferred nowhere.
      Submitting publishes Stoa A's text into Stoa B, which is a harm beyond the
      stale-message defect the issue reports. A deferral needs a destination:
      name the GitHub issue it is filed under, or file one and cite it. As it
      stands the only trace is a `NO SPEC:` test, which says what happens, not
      that anyone meant to leave it. Also say whether the reply composer's
      identical shape (`parentOp` follows `threadId`, "not tested") is in that
      same issue.

- [ ] **`dev-writer`** — `design.md` Decision 2 and Decision 3 are guard
      decisions with no mutation evidence. Decision 2 should say which test
      turns red if `clearOutcome()` also clears the draft
      (`test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`,
      presumably) and Decision 3 whether anything fails if `clearOutcome()`
      clears only one of the two properties. Suggestion, not a contradiction.

- [ ] **`dev-writer`** — `design.md` Decision 1, last paragraph, claims a
      transition's intermediate `screenShown` values are harmless, and only the
      `openThread` case is argued. The return paths (thread to feed, moderation
      to feed) are not: the order in which `reading`/`moderating` and `chosen`
      are set decides whether `screenShown` passes through "list" or goes
      straight to "feed", and a test exists for the moderation return. State the
      return-path orderings, or say the claim rests on the tests rather than on
      reading the setters. Suggestion.
