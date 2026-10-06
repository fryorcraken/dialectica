## Why

The thread screen's reply composer tells the user, under the box, that a reply
"can be edited later". Nothing in the app can edit a reply: no item carries an
edit control, and no method on the module surface publishes a revision. The
0.0.1 build is not to overclaim what it does, and the owner has decided to drop
the promise rather than build editing, which is not in 0.0.1.

## What Changes

- The text rendered beside the thread screen's reply composer no longer states
  that a reply can be edited. What remains describes what a reply is and claims
  nothing the screen does not do.
- No editing is built. Publishing a revision of a reply or a post from the
  interface stays out of scope, and nothing is added to the module surface.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `thread-view`: adds a requirement that no text the thread screen supplies
  around the reply affordance states that a reply can be edited, with editing a
  reply recorded as out of scope.

## Impact

- `dialectica-ui/src/qml/DThreadScreen.qml`: the caption under the reply
  composer, and the comment above it, which describes the remaining sentence as
  true.
- QML component tests for the thread screen.
- No core change, no change to the module's API, no new dependency.
