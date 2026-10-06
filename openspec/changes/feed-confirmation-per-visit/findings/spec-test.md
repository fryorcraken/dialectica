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

- [x] **`spec-writer`** — delta line 22-24 ("Within the visit in which a publish
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
      **Outcome (`spec-writer`): fixed.** The requirement now states that a
      composer the screen is not rendering displays nothing, and that every
      obligation applies whenever the composer is rendered. The within-visit
      obligation reads "MUST be displayed whenever that composer is rendered,
      until the visit ends or a later publish from that composer reports its
      own outcome", with an explicit clause that where a failed read takes the
      composer off screen the outcome MUST be displayed again once a read on
      the same visit succeeds. The later-visit obligation reads "MUST display
      no publish outcome whenever it is rendered", including "after a failed
      read is retried and succeeds on that visit". Two scenarios added: "A
      reply's outcome is displayed again when a failed re-read recovers"
      (pinned by `test_a_replys_outcome_stays_across_a_failed_re_read_of_the_thread`)
      and "A failed read on the later visit to the thread carries no outcome
      once it recovers" (pinned by
      `test_a_failed_read_on_the_later_visit_to_the_thread_carries_no_outcome`).

- [x] **`spec-writer`** — the requirement's prose has no scenario for six
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
      **Outcome (`spec-writer`): fixed.** Six scenarios added, one per
      behaviour: "A confirmation is gone after returning from the moderation
      screen", "Every kind of reply outcome is gone on the next visit to the
      thread", "A reply's outcome does not follow the user into another
      thread", "Paging the feed does not withdraw the outcome", "Changing what
      the feed lists does not withdraw the outcome" and "Changing what the
      thread lists does not withdraw the outcome". Each matches the
      observation its named test already makes. The visit definition now
      names the moderation screen among the ways back to the feed, says a
      retried read is a re-read, and says the hidden-content toggle is a
      change to what a screen lists, so each scenario has a sentence of
      prose to rest on rather than an inference.

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

## Re-review

Scope: `git diff c933da11...HEAD` over `specs/` and `dialectica-ui/tests`. Only
the spec delta and `tst_publish_outcome_visits.qml` were read; the
implementation only at the lines mutated. The draft-ownership entries above are
untouched.

**The rewritten sentences hold and can be tested.** "A composer the screen is
not rendering displays nothing" plus "the obligations apply whenever the
composer is rendered" resolves the vacuity the first review raised; the
recovery obligation ("displayed again once a read on the same visit succeeds")
is observable and `test_a_replys_outcome_stays_across_a_failed_re_read_of_the_thread`
observes it by text and by count. The widened visit definition (moderation
return, retried read is a re-read, hidden-content toggle is a change of
listing) gives each of the eight new scenarios a sentence to rest on. All
eight have a test at the only layer that can see them (the QML suite driving
`Main.qml`), matching name for name: moderation return, every kind of reply
outcome, another thread, later failed thread read, failed re-read recovers,
paging, feed lists, thread lists. The suite is 21 tests, 21 pass on the
unmutated tree (`sh dialectica-ui/tests/run-qml-tests.sh
dialectica-ui/tests/tst_publish_outcome_visits.qml`).

**The helper merge hid nothing.** `visibleNodes` reproduces the three walkers'
predicate exactly (`here && matches(node)`, `here` being own and every
ancestor's `visible`). `openTheThread(view, root)` keeps the link-target
assertion on the no-`root` path and keeps the old `openThreadNamed`'s
`reading.rootOp` assertion on both paths, which also subsumes the explicit
"the same thread" compare removed from
`test_a_replys_outcome_is_gone_on_the_next_visit_to_the_thread`. The new
`storedMessageShown` checks are not derived from the implementation: the copy
is pinned by an existing requirement ("the message displayed states the content
was saved on this machine").

**Mutations run** (output read from the suite command above). Both reverted
with `git checkout --`; `git status --short` is empty.

1. `FeedScreen.reload()` clears the post composer's outcome (first line). 11 of
   21 failed, including `test_the_outcome_stays_across_the_re_read_that_follows_the_publish`
   at its text check (line 412), `test_paging_the_feed_...`,
   `test_changing_what_the_feed_lists_...` and the absence tests, each at the
   presence assertion before it leaves. That confirms the new test comment's
   measured claim ("turns red every feed test in this file that publishes and
   then asserts the outcome displayed ... the absence tests go red too"). Killed.
2. `DThreadScreen.beginVisit()` emptied (under-clearing on the thread). 4 of 21
   failed: `test_a_replys_outcome_is_gone_on_the_next_visit_to_the_thread`,
   `test_a_replys_outcome_does_not_follow_the_user_into_another_thread`,
   `test_a_failed_read_on_the_later_visit_to_the_thread_carries_no_outcome`
   (at line 548, after the retry, which is the case the new scenario adds) and
   `test_every_kind_of_reply_outcome_is_gone_on_the_next_visit_to_the_thread`
   (at the first "already published" case). Killed.

I did not mutate for the new `storedMessageShown` assertions in the paging and
hidden-toggle tests: a mutation that swaps which message survives needs an
implementation edit contrived enough that reading is the better judge, and the
text is pinned by hardcoded copy, not by anything the test asked the
implementation for.

- [ ] **`tester`** — three clauses of the rewritten requirement have no test
      and no scenario. (a) "until the visit ends **or a later publish from that
      composer reports its own outcome**": nothing publishes twice on one visit
      (`test_a_publish_on_the_later_visit_displays_its_own_outcome` publishes on
      a later visit, a different obligation). (b) The visit definition says
      asking for hidden content "to be included **or excluded**" changes what a
      screen lists, but both toggle tests press "SHOW HIDDEN" once and never
      toggle back. (c) "and after a failed read is retried and succeeds on that
      visit" is pinned for the thread composer only; the feed's
      `test_a_failed_read_on_the_later_visit_carries_no_outcome` never retries.
      **Scenario:** a change that stops a second publish on the same visit from
      replacing the first outcome (the stale confirmation stays over a refusal),
      or that clears or resurrects the outcome on toggle-off or on a feed retry,
      passes all 21 tests. **Not measured**, found by reading. **Severity:** low;
      each is one short test, or, if the owner considers one not worth pinning,
      the `spec-writer` drops the clause.

- [ ] **`tester`** — `tst_publish_outcome_visits.qml` lines 436-439 are stale
      against the spec this round changed: "The requirement is wider than its
      eight scenarios ... Each is a test here" and the section title "what the
      scenarios above leave open". The spec now carries 16 scenarios (counted:
      `git grep -c "^#### Scenario"`), eight of them pinned by tests under that
      heading, and those tests carry no `// Scenario:` line naming the scenario
      they pin, unlike the first eight scenarios' tests. **Scenario:** the next editor trusts the
      comment, treats those tests as extras beyond the spec, and trims them as
      redundant, or cannot tell which scenario a test pins. **Severity:** low,
      a comment defect.
