## Context

See `proposal.md`, "Why", for the defect. The cause, confirmed against the code:
`Main.qml` mounts `FeedScreen` (`id: feed`) and `DThreadScreen` (`id: thread`)
once each and switches between screens with one `visible:` binding per screen
on `screenShown`. Nothing is destroyed on the way out. `DComposer` keeps its
outcome in `outcome` / `outcomeDetail`, and only `applyReply` and `refuse` write
them. So an outcome set on one visit is still in the property on the next one
and is rendered again: over an empty composer, over a failed read, and in
another Stoa. The issue marked this cause as unverified. It holds, and the
thread screen's reply composer has the same shape.

The same long-lived-instance shape is behind #152, where the feed re-reads only
when `stoaAddress` changes. That is why both screens already carry "re-pointed,
not recreated" comments, and why this change does not try to recreate them.

## Goals / Non-Goals

**Goals:**

- A composer's outcome is withdrawn when a new visit to its screen begins, and
  at no other time. This covers both composers the view mounts.

**Non-Goals:**

- The draft. `composer-view` leaves it undecided. See Decision 2.
- The feed's session vote marks (`ownVotes`) and the closed gate's `showFix`
  disclosure also outlive a visit. Neither is a publish outcome, so the
  requirement does not reach them.

## Decisions

### 1. The navigator begins a visit, on a change of `screenShown`

`Main.qml`'s `onScreenShownChanged` calls `feed.beginVisit()` when the feed
becomes the screen shown and `thread.beginVisit()` when the thread does. Each
`beginVisit()` calls its composer's `clearOutcome()`.

The spec defines a visit as the span from a screen being rendered in place of a
different main-area screen until another one replaces it. That is exactly a
change of `screenShown`, and the navigator is the only layer that holds it.
Each alternative tracks a proxy for that event, not the event itself:

- **`onVisibleChanged` in each screen.** `DStoaListScreen` uses this to re-ask
  the key state on each showing. But an item's `visible` also changes when an
  ancestor is hidden, which happens for reasons that are not a screen change.
  For the list that is harmless, since asking again costs a call. Here it would
  withdraw an outcome in the middle of the visit that produced it, which the
  spec forbids.
- **`onStoaAddressChanged` / `onThreadIdChanged`.** Today leaving the feed
  empties `chosen`, so the address goes to "" and back. That is how `Main.qml`
  happens to route, not a definition of a visit. A route that kept the address
  bound while the thread was open (to avoid a re-read, say) would silently
  bring the defect back.
- **A visit token** (a counter the navigator bumps, with each outcome stamped
  by the visit it came from and shown only while the stamps match). This holds
  the invariant in the data rather than in a call, but it needs a stamp on each
  outcome, a binding on each screen and a comparison in `DPublishOutcome`'s
  caller. For two composers and one call site that is more machinery than the
  guard it replaces.
- **Recreating the screens through a `Loader` on each visit.** This resets
  everything, so it would also decide the draft question (Decision 2) without
  saying so, and it reverses the mount-once design that #152's fix and both
  screens' read triggers are built on.

**A transition can pass through an intermediate value, and none of them begins
a visit.** Every transition is `enterOnly`, which writes the four states in
`stateNames` order (`previewing`, `chosen`, `reading`, `moderating`), and
`screenShown`'s ternary tests `moderating` and `reading` before `chosen`. So the
two routes out of the feed and the two routes back differ:

| Transition | `screenShown` takes, in order |
|---|---|
| list to feed (`open`) | "feed" |
| feed to thread (`openThread`) | "list", "thread" |
| thread to feed (`closeThread`) | "feed" |
| feed to moderation (`moderateIn`) | "list", "moderation" |
| moderation to feed (`closeModeration`) | "feed" |
| feed to list (`closeFeed`) | "list" |

On the way out, `chosen` is emptied before the destination is written, so the
value reads "list" for an instant. On the way back, `chosen` is written while
`reading` or `moderating` is still set, which leaves the value unchanged, and
the next write empties the old state and moves it straight to "feed". Each
return therefore calls `feed.beginVisit()` exactly once. The only intermediate
is "list", and the hook does nothing for "list", so an intermediate never
withdraws an outcome. An intermediate that did reach the hook would still be
harmless unless a transition started and ended on the same screen, and no
transition does. A transition that re-points a screen without changing
`screenShown` is a different case, covered under Risks.

The table was measured, not read off the setters: a scratch spec drove each
transition on `Main.qml` and recorded every `screenShownChanged` value. A
deliberately wrong expectation ("list", "feed" for `closeThread`) was also run
and failed, so the comparison is not vacuous. The scratch spec was not
committed. The tests below see where
each transition lands, which is what a user can see. They do not see the values
a transition passes through, so a reordering of `stateNames` that added a "list"
step to a return would leave them green. By the argument above, it would also
be harmless.

**What breaks without it.** Measured against `tst_publish_outcome_visits.qml`
as it stands. Each case makes the edit, runs
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_publish_outcome_visits.qml`,
and then reverts:

- Removing the `feed` branch reddens the feed's absence tests. Each fails at its
  absence assertion, after its presence assertion has passed:
  `test_a_confirmation_is_gone_after_reopening_the_stoa_from_the_list`,
  `test_a_confirmation_is_gone_after_returning_from_a_thread`,
  `test_a_confirmation_is_gone_after_returning_from_moderation`,
  `test_a_failed_read_on_the_later_visit_carries_no_outcome`,
  `test_a_retried_feed_read_on_the_later_visit_carries_no_outcome`,
  `test_an_already_published_or_refused_outcome_is_gone_on_the_next_visit` and
  `test_an_outcome_does_not_follow_the_user_into_another_stoa`.
  The retried-read test makes its one absence assertion after the retry. It is
  red here for the reason the failed-read test is: without the hook the outcome
  is on screen throughout the later visit. The failed-read test is red during
  the failure, and the retried-read test is red after the retry. What only the
  retried-read test catches is an outcome hidden during the failure that comes
  back on the retry.
- Removing the `thread` branch reddens the reply composer's absence tests, in
  the same way:
  `test_a_replys_outcome_is_gone_on_the_next_visit_to_the_thread`,
  `test_a_replys_outcome_does_not_follow_the_user_into_another_thread`,
  `test_a_failed_read_on_the_later_visit_to_the_thread_carries_no_outcome` and
  `test_every_kind_of_reply_outcome_is_gone_on_the_next_visit_to_the_thread`.
  The thread failed-read test fails only after the retry. While the read is
  failed, the composer is not rendered, so the outcome is absent with or
  without the hook.
- Removing the whole hook reddens the union of those two lists and nothing
  else. `test_a_publish_on_the_later_visit_displays_its_own_outcome` stays green
  without the hook, because the later publish replaces the outcome either way.
  That test pins the later outcome. It does not pin the withdrawal.

### 2. The draft is kept across visits: the smallest change, and marked NO SPEC

`clearOutcome()` touches `outcome` and `outcomeDetail` only. The draft lives in
the field (`draft` is an alias of `field.text`) and is left alone. That is the
behaviour the view already had, and the smallest change that meets the
requirement. `test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`
pins it under a `NO SPEC:` marker.

The alternative is to clear the draft in `beginVisit()` as well. That is one
more line, but it decides a question the spec deliberately leaves open, and it
throws away text the user typed. Both are choices for the owner to make.

**What breaks without it:** if `clearOutcome()` also empties the draft, exactly
the two `NO SPEC:` draft tests go red. They are
`test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened` and
`test_a_draft_typed_in_one_stoa_is_still_held_and_published_in_another`. Every
outcome test stays green, so nothing else in the suite depends on the draft
being kept. This was measured by mutating `clearOutcome()` and running the file.

**The cross-Stoa case is real.** The feed's composer is one instance for every
Stoa and takes its `stoaAddress` from the screen. So a draft typed in Stoa A is
still in the field when Stoa B is opened, and submitting it publishes to B.
`test_a_draft_typed_in_one_stoa_is_still_held_and_published_in_another`
reproduces it: `publish_post` goes out with B's address and A's text. The reply
composer has the same shape by reading (`parentOp` follows `threadId`). That one
is not tested. This change does not fix either. The test pins today's behaviour
under `NO SPEC:` so the gap is visible, and a fix flips its assertions. Which
Stoa (or thread) a draft belongs to is a behaviour question for the spec.

### 3. The composer owns the reset

The screen calls `composer.clearOutcome()`. It does not write
`composer.outcome = ""` itself. `outcome` and `outcomeDetail` are one value
split across two properties, and `DComposer`'s rule is that every write leaves
them consistent. Keeping the one reset inside the component means a screen
cannot clear one and leave the other.

**What breaks without it, measured by mutating `clearOutcome()`:**

- **Clearing only `outcomeDetail` brings the defect back.** It reddens the same
  absence tests as removing the hook (Decision 1), because `outcome` is what
  decides whether anything renders.
- **Clearing only `outcome` leaves the suite green.** That is not a gap the
  suite can close. Every path that writes `outcome` also writes `outcomeDetail`
  (`applyReply`'s two success branches and `refuse`), and `DPublishOutcome`
  renders `detail` only while the outcome is "refused". So a stale detail
  behind an empty outcome cannot be displayed, and the next outcome overwrites
  it. Writing both halves keeps the component's invariant, not a behaviour a
  test can see. The detail would become visible only if a writer set "refused"
  without setting the detail. That writer would be the defect, and this reset
  would not stop it.

### 4. The outcome element is named `<kind>OutcomeMessage`

`DComposer` names its `DPublishOutcome` from `kind`, as it already names the
draft field and the submit button. Each screen mounts one composer of its own
kind, so `postOutcomeMessage` and `replyOutcomeMessage` each name exactly one
element, and a test can ask whether that element is displayed. The obvious
name, `…PublishOutcome`, fails `check_qml_names.py`. That gate reads the string
as a bare reference to `DPublishOutcome`'s undecorated name, and it rejected
the first version of this change.

### 5. Within a visit, nothing withdraws the outcome

The re-read that follows a newly stored op does not touch the composer. This is
the guard against over-correcting.

**What breaks without it, measured** by making each edit and running the file:

- **`composer.clearOutcome()` as the first line of `FeedScreen.reload()`**
  turns red every feed test that publishes and then asserts the outcome
  displayed. In each of them the first publish is newly stored or already
  published, and that publish is followed by a re-read, so the outcome is gone
  before the first presence assertion. That set includes
  `test_the_outcome_stays_across_the_re_read_that_follows_the_publish`,
  `test_the_outcome_stays_whatever_the_re_read_returns`,
  `test_paging_the_feed_within_the_visit_keeps_the_outcome` and
  `test_changing_what_the_feed_lists_within_the_visit_keeps_the_outcome`. It
  also includes the feed's absence tests and
  `test_a_publish_on_the_later_visit_displays_its_own_outcome`, each red at the
  presence assertion it makes before leaving. So over-clearing does not pass
  the absence tests: they cannot reach the assertion it would satisfy. The
  last two in the set cover both composers in one table:
  `test_a_later_publish_on_the_same_visit_replaces_the_earlier_outcome` and
  `test_asking_for_hidden_content_to_be_excluded_again_keeps_the_outcome`. Each
  is red at its first post case. No other test goes red.
- **`replyComposer.clearOutcome()` as the first line of `DThreadScreen.reload()`**
  does the same to every reply test that publishes and then asserts the outcome
  displayed. That set includes
  `test_a_replys_outcome_stays_across_a_failed_re_read_of_the_thread` and
  `test_changing_what_the_thread_lists_within_the_visit_keeps_the_outcome`, as
  well as the reply composer's absence tests at their presence assertions. The
  same two tables go red here too, each at its first reply case, after its post
  cases have passed. No other test goes red.

## Risks / Trade-offs

- [A new screen that mounts a composer must be added to `onScreenShownChanged`
  by hand.] → The list has two entries, and each entry is the screen's own
  `beginVisit()`, so the omission is visible where the screen is added. Nothing
  enforces it. If the third composer screen arrives, that is the point to
  revisit the visit-token alternative in Decision 1.
- [A future route that re-points the feed from one Stoa straight to another,
  without passing through another screen, would by the spec's own definition
  stay within one visit, and the outcome would follow the user into the second
  Stoa.] → No such route exists: every way onto a feed passes through the list,
  a thread or moderation. If one is added, the spec's visit definition is the
  thing to revisit, not this hook.
- [The component tests cannot see Basecamp.] → This fix is pure navigator and
  component logic, with no host dependency. `tst_publish_outcome_visits.qml`
  drives `Main.qml`, the layer the defect lives in. No end-to-end step was
  added. `feed.yaml` and `thread.yaml` still assert the confirmation within
  the visit that produced it, which this change leaves alone.
