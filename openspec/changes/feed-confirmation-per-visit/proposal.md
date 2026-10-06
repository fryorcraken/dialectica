## Why

The feed's publish confirmation — "Your post was saved on this machine", with
its delivery denial — is still on screen after the user leaves the Stoa and
comes back, under an empty composer on a visit where nothing was posted, and
over a failed read. It is a claim about an action the user did not just take.
`composer-view` contracts what a publish outcome may say, but not how long it
may go on saying it, so nothing forbids an outcome from outliving the visit
that produced it.

## What Changes

- A publish outcome — newly stored, already published, or refused — is
  displayed only on the visit to the screen in which that publish was made. A
  later visit to that screen, whether by reopening the Stoa from the list or by
  returning from a thread, displays no outcome until its own composer reports
  one.
- The rule covers every composer the view mounts: the feed's post composer and
  the thread screen's reply composer.
- Within the visit that produced it, the outcome stays: the re-read that
  follows a successful publish does not withdraw it.
- What becomes of an unsubmitted draft when the screen is left is **not**
  decided by this change, and the requirement says so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `composer-view`: adds a requirement that a publish outcome belongs to the
  visit in which the publish was made, and is not displayed on any later visit.

## Impact

- `dialectica-ui/src/qml/FeedScreen.qml`, `dialectica-ui/src/qml/DThreadScreen.qml`,
  `dialectica-ui/src/qml/DComposer.qml` and possibly `dialectica-ui/src/qml/Main.qml`
  — wherever a visit's start is observable to the composer.
- QML component tests driving navigation in `Main.qml`
  (`dialectica-ui/tests/tst_navigation.qml`, `dialectica-ui/tests/tst_stoa_screens.qml`).
- No core change, no wire-contract change.
