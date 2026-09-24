## Why

PR #155 (issue #150) merged to `main` as `0cbe1d4`. It showed key creation or
Stoa creation on the home screen depending on whether this machine holds a key.
The owner asked for one more six-reviewer round over the feature as merged, with
any fixes going into a new PR. That review is done, and its results are in this
change's `findings/`. Four reviewers (correctness, security, readability,
design) found nothing. Two found one defect each, and both defects are in the
tests, not in what the app does. This piece fixes those two, and it has no
GitHub issue of its own.

## What Changes

- **A creation test is driven from the one key state where creation can
  happen** (spec-test finding). Two tests in
  `dialectica-ui/tests/tst_stoa_screens.qml`,
  `test_a_created_stoa_is_openable_from_the_creation_reply_alone` and
  `test_a_creation_success_carrying_no_address_is_a_failure`, call
  `screen.create()` on a fixture that gives `get_master_key` no reply. That
  puts them in the could-not-be-read key state. `stoa-navigation-view` says the
  screen MUST NOT instantiate the create affordance in that state. So the
  tests exercise creation from a state where no user could reach it. Their
  fixtures move to the key-held state (`spec.heldKeyReply(...)`), the pattern
  the file's other creation tests already use. What each test asserts stays
  the same.
- **The identity-route test's control can no longer drift silently from the
  route** (architecture finding).
  `test_following_the_route_makes_no_call_of_its_own` in
  `dialectica-ui/tests/tst_navigation.qml` uses `closeFeed()` as its control
  for `acquireIdentity()`. The only thing tying the two together is two
  comments. If `closeFeed()` changes by itself, the test fails and its message
  blames the route, which did not change. The fix is a structural link between
  the two, so a change to one of them either shows up in the other or fails
  with a message that names the right cause. The finding lists two options, and
  `design.md` records which one is chosen.

Neither fix changes observable behaviour. The first one only changes fixtures.
The second one either changes a test, or refactors `Main.qml` so that
`acquireIdentity()` does exactly what it does today. The route still makes no
call of its own, as `view-navigation` requires.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true` alongside `schema:`. Neither
finding changes a requirement:

- The spec-test finding is about tests that are **out of line with** the spec.
  `stoa-navigation-view` already says the create affordance exists only when
  the core reports a key. The fix brings the fixtures into line with that
  text.
- The architecture finding is about how a test's control is tied to the code
  it stands in for. `view-navigation`'s "Acquiring an identity is reached from
  the navigator" already says what the route may and may not call, and neither
  option in the finding changes that.

## Impact

- `dialectica-ui/tests/tst_stoa_screens.qml`: the two creation tests' fixtures.
- `dialectica-ui/tests/tst_navigation.qml`, and possibly
  `dialectica-ui/src/qml/Main.qml` (a refactor that changes no behaviour):
  the identity-route test's control.
- No core, wire-contract, spec or dependency change.
