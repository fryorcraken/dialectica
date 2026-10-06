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

A transition passes through intermediate values. `openThread` empties `chosen`
before it sets `reading`, so `screenShown` reads "list" for an instant. That is
harmless, because beginning a visit only withdraws what the previous visit left.

**What breaks without it:** with the hook removed, the six absence tests in
`tst_publish_outcome_visits.qml` fail on their absence assertions, each after
its presence assertion has passed. Measured before the fix. Removing only the
`thread` branch turns exactly
`test_a_replys_outcome_is_gone_on_the_next_visit_to_the_thread` red (also
measured).

### 2. The draft is kept across visits: the smallest change, and marked NO SPEC

`clearOutcome()` touches `outcome` and `outcomeDetail` only. The draft lives in
the field (`draft` is an alias of `field.text`) and is left alone. That is the
behaviour the view already had, and the smallest change that meets the
requirement. `test_an_unsubmitted_draft_is_still_held_when_the_same_stoa_is_reopened`
pins it under a `NO SPEC:` marker.

The alternative is to clear the draft in `beginVisit()` as well. That is one
more line, but it decides a question the spec deliberately leaves open, and it
throws away text the user typed. Both are choices for the owner to make.

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
the guard against over-correcting: clearing the outcome in `FeedScreen.reload()`
passes every absence test, and it turns seven tests red, including
`test_the_outcome_stays_across_the_re_read_that_follows_the_publish` (measured).

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
