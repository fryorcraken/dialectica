## Why

The join preview is the screen where a user decides whether to trust an address
handed to them, and it currently tells them nothing about what the Stoa is
called: it calls no core method before a join, so it renders "nothing here
knows what this Stoa is called" for every reference. Issue #98 (PR #144) added
`getStoa`, which answers a Stoa's title for an `(address, record)` pair this
peer has not joined, and restated `stoa-navigation-view` against it — so the
spec now obliges the preview to render a founding title that the view does not
fetch. Issue #143 (milestone 0.0.1) closes that gap.

## What Changes

- **The preview asks `getStoa` about the reference it is showing**, with the
  address and genesis record it already holds, as soon as it is shown and
  without the user acting. Nothing is joined by asking.
- **What the reply's `title` is rendered as is decided by `isGenesisFallback`,
  and by nothing else.** A fallback reply's `title` is the founding title and
  fills the founding-title position; a non-fallback reply's `title` is a
  current title and fills the current-title position, never the founding one.
  This is what `stoa-navigation-view` already requires of any `getStoa` reply;
  this change adds the call that produces one.
- **`isGenesisFallback` is surfaced, not only used to pick a panel.** A fallback
  reply establishes only that this peer holds no moderator-set title for the
  Stoa, not that the Stoa was never renamed; the preview says so, rather than
  letting the absence of a current title read as "not renamed".
- **The description is rendered with the current title, and only there.** A
  moderator sets it in the same op as a current title, a fallback reply always
  carries an empty one, and the bundle's preview has no description position.
  So it is shown only from a non-fallback reply, and only when non-empty.
- **A founding title that is known to be empty is rendered as known.** Until
  now "no title" on the preview always meant "not known here". With `getStoa`,
  a fallback reply can answer an empty founding title — a legal value — and
  the preview renders it as the founding title rather than as the unknown case.
- **A lookup that fails is a failure the screen renders, and not a fallback.**
  The core's reason is shown; no title is rendered from it; it is not reported
  as a refused join; and the join affordance is still offered, since the join
  is a separate call the core answers for itself.
- **A reply shape that is neither success nor the error shape is a failure.**
  A reply missing `isGenesisFallback` as a boolean, or `title` or
  `description` as a string, is not rendered as a fallback or as a rename.
- **A lookup belongs to the reference it was made for.** A title answered for
  one reference is never rendered for the next one previewed.
- **Out of scope, and settled by the owner on the issue:** the same-title
  (lookalike) comparison is not extended to current titles. A Stoa renamed to
  match one the user holds reports `isGenesisFallback: false`, so no founding
  title is available and the comparison does not run; that is accepted at this
  stage. This change pins that the comparison does not run against a current
  title rather than leaving it to be inferred.
- **Also out of scope:** display-point mitigation of bidirectional and
  zero-width characters in titles (`stoa-metadata`'s "A displayed title is
  never an identifier" places it on the renderer; the preview's founding title
  does not do it today either), and publishing a metadata op (issue #125). Until
  #125 lands this build publishes none, so in practice every lookup falls back
  unless an op arrives from elsewhere.

## Capabilities

### New Capabilities

None. The join preview's contract is `stoa-navigation-view`'s.

### Modified Capabilities

- `stoa-navigation-view`: ADDED "The preview asks the core what the Stoa is
  called, and renders the answer as what the reply says it is" (the lookup is
  made for the reference on screen, before any join and for no malformed input;
  a fallback `title` fills the founding position, an empty one included; a
  non-fallback `title` and its non-empty `description` fill the current
  position; a fallback is stated as "no moderator-set title held here"; a
  current title is never compared against held Stoas); ADDED "A lookup that
  fails, or answers in no recognisable shape, renders no title and withdraws no
  join" (the core's reason is rendered, the outcome is neither a fallback nor a
  refused join, and the join affordance remains); ADDED "A lookup's answer is
  rendered only for the reference it was made for". No existing requirement's
  text changes: "Joining shows what is being joined", "No current title is
  rendered until one has been resolved" and "A Stoa already held whose title
  matches…" were restated against `getStoa` by #98 and already say which reply
  field decides what.

## Impact

- `dialectica-ui/src/qml/Core.qml`: one new wrapper naming `get_stoa`, the only
  place the method string appears.
- `dialectica-ui/src/qml/DJoinScreen.qml`: the lookup, its outcome tied to the
  reference as the join outcome already is, the two title panels driven from
  it, the fallback statement, the description, and the no-title note's copy —
  which currently says nothing knows what the Stoa is called and that only
  joining asks, both false once a lookup answers.
- `dialectica-ui/src/qml/Main.qml` if the preview's entry point is where the
  lookup is triggered.
- `dialectica-ui/tests/tst_stoa_screens.qml`: QML specs driving the preview
  against fake `getStoa` replies — fallback, non-fallback, empty founding
  title, failure, malformed shape, and a second reference after the first.
- No core change and no wire change: `getStoa` exists as #98 shipped it.
