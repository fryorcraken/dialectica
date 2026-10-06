# spec-test review: feed-confirmation-per-visit

Reviewed the delta `specs/composer-view/spec.md`, the existing
`openspec/specs/composer-view/spec.md`, issue #179 (read fresh with
`gh issue view 179`) and `dialectica-ui/tests/tst_publish_outcome_visits.qml`.
The implementation was read only at the lines mutated.

**Coverage.** All eight scenarios have a test at a layer that can see them (the
QML suite driving `Main.qml`, which is the only layer that holds the long-lived
screens whose reuse is the defect). The tests also cover the requirement's prose
beyond its scenarios: other kinds of reply outcome, a different thread, a failed
re-read of each composer, the moderation round trip, paging and the show-hidden
toggle. Every absence assertion is preceded by the presence it withdraws, and
visibility is checked on every ancestor, so none passes on a composer that never
showed anything. No expectation is taken from the implementation. The delta is
`ADDED` only, so there are no moved requirements to verify. The issue states
nothing the spec no longer asks for; both return paths and the read-failure
panel it names are covered.

**The failed-read scenario for the thread screen (the tester's question).** The
reading is sound, with one limit. A failed thread read removes the reply
composer, so "no outcome" during the failure is vacuously true (line 541). The
test does not rely on it: it recovers the read on the same, non-publishing
visit and asserts absence once the composer is back (lines 543-550). That is the
only observable form of the requirement for that composer, and it fails if the
outcome were left uncleared. It would not catch an outcome that is wrongly
displayed during the failure, but nothing can see that, because the composer is
not on screen. See the spec finding below.

**Mutations run** (suite: `sh dialectica-ui/tests/run-qml-tests.sh
dialectica-ui/tests/tst_publish_outcome_visits.qml`, output read from that
command). Both were over-clearing mutations, the shape the dev's own mutation
(`FeedScreen.reload()`) did not cover for the thread screen and for paging.
Neither survived. Both are reverted; `git status --short` was empty afterwards.

1. `DThreadScreen.reload()` clears the reply composer's outcome. Result: 6 of 21
   failed, including `test_a_replys_outcome_stays_across_a_failed_re_read_of_the_thread`
   and `test_changing_what_the_thread_lists_within_the_visit_keeps_the_outcome`.
   Killed.
2. The feed's "Previous" button clears the post composer's outcome (the "Next"
   half of the paging test already passes on this mutation by design). Result:
   `test_paging_the_feed_within_the_visit_keeps_the_outcome` failed at "and
   paging back" (line 620). Killed.

I did not re-run an under-clearing mutation (an empty `beginVisit`). The dev
reports six absence tests failing before the fix; by reading, each of those
tests would fail on it, since each asserts presence first and absence after the
visit boundary.

- [ ] **`spec-writer`** — delta line 22-24 ("Within the visit in which a publish
      was submitted, its outcome MUST remain displayed across the re-read ...
      whatever that re-read returns")
      **Scenario:** a reply is published, the re-read fails. The thread screen
      renders its reply composer only on a successful read, so the composer
      and its outcome are not on screen at all during the failure; the sentence
      read literally cannot be satisfied for that composer. The test
      (`test_a_replys_outcome_stays_across_a_failed_re_read_of_the_thread`) had
      to reinterpret it as "displayed again when the read recovers on the same
      visit". Say so in the requirement, for example "remains the composer's
      outcome, and is displayed whenever the composer is", and add a scenario
      for it. Same limit applies to the failed-read statement for the reply
      composer ("the composer displays no publish outcome" is vacuous while the
      composer is absent). **Severity:** low, a wording defect, not a coverage
      gap.

- [ ] **`spec-writer`** — the requirement's prose has no scenario for six
      behaviours the tests pin: re-read, paging and changing what a screen lists
      do not begin a visit (tests `test_paging_...`, `test_changing_what_the_feed_...`,
      `test_changing_what_the_thread_...`); returning from the moderation screen
      begins a visit (`test_a_confirmation_is_gone_after_returning_from_moderation`);
      a reply's outcome does not follow the user into another thread
      (`test_a_replys_outcome_does_not_follow_...`); every kind of reply outcome
      is gone on the next visit (`test_every_kind_of_reply_outcome_...`). All are
      stated in the requirement text, so none is a `NO SPEC:` choice, but a
      behaviour with a test and no scenario is invisible to the next change that
      rewrites the scenarios. **Scenario:** a later edit trims the scenario list
      to "what is scenario-ed" and the paging rule, the one that guards against
      the over-clearing fix, loses its stated contract (mutation 2 above shows
      the test is what stops it). **Severity:** low.

- [ ] **`dev-writer`** — `test_a_draft_typed_in_one_stoa_is_still_held_and_published_in_another`
      (lines 668-691, `NO SPEC:`) pins what its own comment calls "very probably
      a defect". **Scenario:** the suite asserts that a draft typed for Stoa A
      is submitted to Stoa B and sent as `stoa: B`. That is a permanent,
      signed publish into the wrong Stoa, and the test makes the broken
      behaviour the green state: the fix for it turns CI red ("a fix flips both
      assertions"). Do not keep it as a passing pin. Remove it from the suite and
      record the finding as an issue (this is the owner's to file or approve; the
      spec is the `spec-writer`'s to decide, see the next entry), or if the
      owner wants it visible, mark it skipped with the issue number. **Severity:**
      medium, because the test defends a data-integrity bug.

- [ ] **`spec-writer`** — the draft's fate across visits is explicitly left
      undecided by the delta (lines 29-31), and two `NO SPEC:` tests pin it:
      `test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`
      and the cross-Stoa test above. Decide it or leave it undecided on purpose.
      **Scenario:** if the spec later rules that a draft is cleared with the
      visit, test 1 is wrong the moment it lands; if it rules a draft belongs to
      its Stoa, the cross-Stoa test is wrong. Routing these to the spec-writer is
      the correct path (they are spec gaps, not code defects). Note that test 1,
      unlike the cross-Stoa test, pins a defensible behaviour (the draft the
      view already held) and can stay until the spec decides; the cross-Stoa
      one should not (previous entry). **Severity:** medium for the cross-Stoa
      half, low for the rest.

No unmarked behaviour beyond the above: `test_the_outcome_stays_whatever_the_re_read_returns`
asserts `feedReadState` as a precondition for its fourth case (an answer with no
`items` is `failed`), which depends on how the feed classifies a malformed
answer rather than on this requirement, but it is a precondition check and
fails for a reason that names itself, so no box is opened.
