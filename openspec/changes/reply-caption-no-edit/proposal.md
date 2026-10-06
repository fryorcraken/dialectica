## Why

The thread screen's reply composer tells the user, under the box, that a reply
"can be edited later". Nothing in the app can edit a reply: no item carries an
edit control, and no method on the module surface publishes a revision. The
0.0.1 build is not to overclaim what it does, and the owner has decided to drop
the promise rather than build editing, which is not in 0.0.1.

## What Changes

- The text rendered beside the thread screen's reply composer no longer states
  that a reply can be edited. What remains states what a reply is, that it is
  signed, as the issue's expected behaviour asks, and claims nothing the screen
  does not do.
- Nor may that text state that a reply's earlier versions can be read. The
  bundle's caption carried that promise beside the edit one, and nothing on the
  module surface reads a prior version either.
- The text says nothing on either topic, in either direction: a denial ("a
  reply cannot be edited") is forbidden as well as a promise. Editing is
  planned work, so a denial would go false when it lands, with nothing to prompt
  its removal.
- Text core supplies and the view renders verbatim, and the user's own draft,
  are outside the rule: the view may not reword the first, and the second is not
  the screen's to state.
- No editing is built. Publishing a revision of a reply or a post from the
  interface stays out of scope, and nothing is added to the module surface.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `thread-view`: adds a requirement that no text the thread screen authors
  around the reply affordance says anything about editing a reply or reading
  its earlier versions, with editing a reply recorded as out of scope; and a
  requirement that the open reply composer states that a reply is signed.

## Impact

- `dialectica-ui/src/qml/DThreadScreen.qml`: the caption under the reply
  composer, and the comment above it, which describes the remaining sentence as
  true.
- QML component tests for the thread screen.
- No core change, no change to the module's API, no new dependency.
