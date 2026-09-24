## Context

See `proposal.md` for why this piece exists. Both defects it fixes are in
tests, and neither changes what the app does. The only source change is a
comment in `Main.qml` that described the coupling Decision 1 removes.

Two facts about the code shape both decisions:

- **`Main.qml`'s navigator state is its four state properties, and nothing
  else.** `screenShown` is `"list"` exactly when all four are null, and
  `DStoaListScreen` asks `get_master_key` from its own `onVisibleChanged`, so
  the list's showing is triggered by the state change and not by any navigator
  function.
- **`DStoaListScreen.create()` does not check the key state.** The key-held
  gate is the `createBlockLoader`'s `active` binding, which decides whether the
  create button exists. A test calling `create()` as a bare method therefore
  runs the same code in every key state, so the key state in its fixture is
  never exercised.

## Goals / Non-Goals

**Goals:**

- The identity-route test fails only when the route, or the transition
  primitive it goes through, reaches the bridge for something of its own. It
  names the route only when the route is at fault.
- The two creation tests exercise creation from the one state a user can
  create from, and fail if their fixture leaves that state.

**Non-Goals:**

- No change to `acquireIdentity()`, `closeFeed()` or `enterOnly()` bodies.
  `view-navigation` already says what the route may call, and it calls exactly
  that today.
- The `NO SPEC:` marker on
  `test_a_creation_success_carrying_no_address_is_a_failure` stays. It predates
  #155, and deciding what a success with no address should do is the spec's
  question, not this piece's.

## Decisions

### 1. The identity-route test's control shows the list without running any navigator function

`test_following_the_route_makes_no_call_of_its_own` compares the bridge calls
made after the identity chip fires against the calls a control makes on
reaching the list. The control used to be the feed's close button, which runs
`closeFeed()`. It now writes `chosen = null` on a feed-showing `Main`. From the
feed, `chosen` is the only state set, so clearing it is the list by the
navigator's own definition, and no function in `Main.qml` runs.

Decision 15 of `openspec/changes/archive/2026-09-24-home-screen-key-states/design.md`
recorded the old coupling. The control was valid only because `closeFeed()` and
`acquireIdentity()` had the same body, and all that held them together was two
comments asking whoever changed one to revisit the test. The architecture
review for this piece (`findings/architecture.md`) asked for a structural fix
and offered two options. Neither is taken, for these reasons:

- **Make `acquireIdentity()` delegate to `closeFeed()`.** Decision 15 names
  this as the worse failure: with the old control, a call added to
  `closeFeed()` then passes the test while the route makes that call. Measured
  here: `acquireIdentity() { root.closeFeed() }` with
  `Core.whoAmI("")` added to `closeFeed()`, against the old control, gave 24
  passed, 0 failed. It would also change behaviour, since the route would
  inherit whatever the feed's close does. That is coupling in the app, not
  only in the test.
- **Add a test that both functions produce the same navigator transition.**
  The two functions produce the same state even when one of them makes an
  extra call, so in the finding's own scenario that test stays green. The
  misleading failure still fires, still blaming the route.

Taking `closeFeed()` out of the comparison removes the dependency the finding
is about. Nothing links the two functions, so neither can drift from the other.

**What each mutation does to the test as it now stands** (measured, each
restored):

| Mutation to `Main.qml` | Old control | New control |
|---|---|---|
| M1: `Core.whoAmI("")` in `acquireIdentity()` | red | red: `[who_am_i,get_master_key]` vs `[get_master_key]` |
| M2: `Core.whoAmI("")` in `closeFeed()` only | red, blaming the route (architecture finding) | **green**: the route did not change |
| M3: M2, plus `acquireIdentity()` delegating to `closeFeed()` | **green** while the route calls `who_am_i` | red, same message as M1 |
| M4: `Core.whoAmI("")` in `enterOnly()` | green (both sides make the call) | red, same message as M1 |

The M4 row goes further than the finding asked. A call in the transition
primitive is a call the route makes, and `view-navigation` forbids it. The old
control made that call as well, so it could not see it.

**What breaks without this decision:** revert the control to
`feed.closed()` and M2 turns this test red with a message naming a route that
did not change. M3 and M4 then pass while the route makes a call.

**What the control relies on, and how that failure shows.** It assumes that
`chosen` is the only state set on a feed-showing `Main`, and that `chosen` is
writable from outside. If `open()` began setting a second state, clearing
`chosen` would not reach the list, and the control's own
`compare(showView.screenShown, "list", …)` fails, naming the control. If
`chosen` became read-only, the write throws at the control's line. Either way
the failure points at the control, not at the route.

### 2. The two creation tests go through the create button, in the key-held state

`test_a_created_stoa_is_openable_from_the_creation_reply_alone` and
`test_a_creation_success_carrying_no_address_is_a_failure` used to call
`screen.create()` on a fixture with no `get_master_key` reply, so the fake
answered with the error shape and put the screen in the could-not-be-read
state. `stoa-navigation-view` requires that the create affordance not be
instantiated in that state (`findings/spec-test.md`). Each fixture now carries
`spec.heldKeyReply(aKeyHex(), false)`, the pattern the file's other creation
tests use. Each test finds `createStoaButton` with `visibleNamed`, compares the
count to 1, and fires its `clicked()`. The assertions after that are unchanged.

Alternatives considered:

- **Add the reply and keep calling `create()` directly.** Rejected because the
  fix would not be observable. `create()` never reads the key state, and the
  baseline run proves it: both tests passed in the could-not-be-read state.
  Without the reply they would still pass, so the fixture change would be
  decoration that nothing depends on.
- **Assert `machineKey.state === "held"` as a precondition.** This makes the
  fixture load-bearing, but it asserts the state and not what the finding is
  about, which is whether the affordance exists. Going through the button
  checks both, and it also shows the button is wired to `create()`.

**What breaks without it** (measured, each restored):

- S1, dropping the `get_master_key` reply from both fixtures: both tests fail
  at the lookup, `0` against `1`.
- S2, `onClicked: {}` on `createStoaButton`: both tests fail at `createState`
  (`""` against `"created"` and `"failed"`). The existing
  `test_the_placeholder_is_never_submitted_as_a_title` fails too, since it
  already drove the button.
- S3, removing `rememberGenesis(...)` from `create()` and disabling its
  no-address guard: each test fails at the assertion its name is about, the
  genesis for the first and `createState` for the second. That shows neither
  test now passes on the button lookup alone.

## Risks / Trade-offs

- **[The route test's control bypasses `enterOnly`]** → That is the point,
  because a control that ran the primitive would miss M4. It does mean the
  control is not a transition any user performs. What it stands for is a
  showing of the list, and nothing in `Main.qml` besides the navigator state
  triggers one.
- **[No layer can see a change here beyond the QML suite]** → Both changes are
  confined to `tst_navigation.qml` and `tst_stoa_screens.qml`, plus one
  `Main.qml` comment. The QML suite is the only layer that runs them. A green
  Rust suite or build says nothing about them, and none is claimed.
