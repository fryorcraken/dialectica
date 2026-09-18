## Why

**Five registered types in `dialectica-ui/src/qml/qmldir` are instantiated
nowhere in the view, and 92 passing test functions run over four of them.**
Measured against this tree: `DOnboardingScreen`, `DStatusBar`, `DVouchStamp`,
`DIdentityChip` and `DKeyNameWindow` each appear in `qmldir` and in no
production instantiation. `grep -c "function test_"` reports 50 in
`tst_onboarding_states.qml`, 13 in `tst_status_bar.qml`, 14 in
`tst_identity_chip.qml` and 15 in `tst_vouch_stamp.qml`.

The worst of these is the identity flow. `DOnboardingScreen` is **the view's
only route to acquiring an identity** — `view-identity-onboarding`'s "An
identity is what participation needs, and this screen is where one is made"
says so — and no user can reach it. A fresh install therefore cannot obtain a
key at all, while a green suite reports the screen works. It does work; it is
simply unreachable, and **a component suite is structurally unable to tell those
two apart**, because it instantiates the screen itself.

`view-identity-onboarding` already predicted this and scoped it out, in the
requirement text: "Neither the route into this screen nor the route back out is
built, and this capability does not yet contract either… Both belong to
whichever change wires the banner's fix affordance to this screen." **This is
that change.**

`stoa-navigation-view` carries the matching half, under "Every state a user can
enter has a specified way out": it names the feed-to-list return as the one
transition deliberately left unspecified, because "no route exists today". A
route exists today — `Main.qml`'s `closeFeed()` — so that carve-out is now stale
and the return is unprotected precisely because it works. Both halves are the
same gap seen from two capabilities, and neither can be closed from inside the
screen it concerns.

## What Changes

- **A reachability rule over `qmldir`.** Every registered screen-level type is
  either reachable from the view's root or is deliberately not, with the reason
  recorded. This is the contract that makes the current defect a test failure
  rather than a thing nobody looked for. It is the static half; the runtime half
  is `piece/e2e-sitometres`'.
- **The onboarding route, in both directions.** A user with no usable identity
  can reach identity creation, and a user who acquires one arrives somewhere
  they can act. The screen keeps its existing contract that it "SHALL NOT
  navigate anywhere itself" — the navigator owns the routing, and
  `DOnboardingScreen.qml:37`'s `identityKept()` signal, which deliberately
  carries no identity, is what it listens to.
- **Routing reads both identity probes, and treats their disagreement as
  informative.** `who_am_i` and `get_capabilities` answer different questions and
  the trait says so at `dialectica/rust-lib/src/lib.rs:258`: "the two can
  honestly disagree: a stored identity whose keystore permissions are too open is
  a real identity that cannot currently be used. A view with only the posting
  probe would have to render 'you are nobody' to a user who has an identity and a
  fixable problem." Routing on the posting probe alone would send that user to
  key *creation*, which for an existing identity is the one irreversible wrong
  answer available.
- **No navigation state caches either probe.** `get_capabilities`'s trait doc
  contracts the reprobe — "The probe re-determines its answer on every call by
  design, so a view asks it whenever it renders rather than caching it" — and
  `FeedScreen.qml:41-43` already honours it, citing PLAN.md: "caching it across a
  keystore change is how a button outlives the key that justified it." A cached
  "has identity" at the navigation layer would reintroduce exactly that, one
  level up and across every screen at once.
- **The thread route, in and out only.** A feed row can open a thread and the
  thread can be left. **The thread screen's own behaviour is
  `piece/ui-thread-view`'s and is not specified here** — see the boundary note
  below.
- **`DStatusBar`'s placement.** Confirmed unmounted. It is shared chrome rather
  than a screen, so the question is which states it is visible in, not which
  screen reaches it.
- **`stoa-navigation-view`'s feed-to-list carve-out is removed**, since the
  transition it says does not exist now does.

## Where this piece meets `piece/ui-thread-view`

The two pieces run concurrently and meet at one boundary: **this piece contracts
the transition, that piece contracts the destination.** A feed row's opening a
thread, what travels with it, and the way back to the feed are here; what the
thread screen renders, how it nests replies, and what it does with a hidden root
are there. Neither restates the other — two live requirements on one behaviour is
how they come to contradict, which `view-identity-onboarding` records as its own
reason for not restating `composer-view`.

## What this is not

**It is not a redesign of the navigator.** `Main.qml`'s existing shape — screen
selection derived from properties, with no StackView, so that the state has one
source rather than two that can disagree — is a recorded decision and this change
keeps it. What changes is which screens are in the set and what selects them.

**It does not specify what any newly-reachable screen renders.**
`view-identity-onboarding` owns 18 requirements over the onboarding screen and
`feed-view` owns the feed; this change adds no requirement about either screen's
contents. Reachability and contents are different questions, and conflating them
is what would put a second live requirement on a screen another capability
already owns.

**It does not wire everything it finds unmounted.** Three of the five are
deliberate and this change records them rather than instantiating them:

- **`DVouchStamp`** — voting is out of the MVP by owner ruling 1, so its
  non-instantiation is a scope decision.
- **`DKeyNameWindow`** — it MUST stay uninstantiated. It is the name-disjointness
  gate's measurement apparatus, and `tst_identicon.qml` asserts that it exposes
  no renderer at all, on the ground that a renderer would flip its
  malformed-input obligation. Its own test records the measurement — `grep -rl
  DKeyNameWindow dialectica-ui/` returns `qmldir` and that test file and nothing
  else — and names the "no consumer" half as held by a spec requirement. This
  change supplies that requirement. **A dev-writer that wires this one has broken
  the gate**, which is why it is called out here rather than left to be inferred
  from the reachability rule.
- **`DIdentityChip`** — it is a consumer of the identity report rather than a
  screen, so where it belongs follows from the routing this change contracts,
  not from a scope ruling.

Under the reachability rule each of these says so and states why, which is the
difference between a scope decision and an oversight — the two are
indistinguishable from the registration alone.

## Capabilities

### New Capabilities

- `view-navigation`: which screens the view can reach, what selects between
  them, and what travels across each transition. **The generality is
  demonstrated rather than assumed**, which is the condition for extracting a
  capability: three capabilities already hand off routing they cannot own, in
  their own text. `view-identity-onboarding` says the screen "SHALL NOT navigate
  anywhere itself" and that neither its route in nor its route out is contracted
  by it; `stoa-navigation-view` names the feed-to-list return as outside itself
  because closing it "is a change to a screen this capability does not own"; and
  the thread screen arrives needing the same handoff. Routing is what none of
  them can hold, and one capability asserting it is the alternative to three
  asserting fragments of it.

### Modified Capabilities

- `stoa-navigation-view`: two requirements move out to `view-navigation`
  verbatim — "The view holds no Stoa of its own, and the feed is reached from the
  list" and "Every state a user can enter has a specified way out" — because both
  are statements about transitions rather than about the Stoa screens' rendering,
  which is what this capability's Purpose scopes it to. **The text moves
  unedited**; the stale feed-to-list carve-out inside the second one is corrected
  in `view-navigation`'s copy, and that correction is the one behaviour change
  here, called out rather than smuggled into a move.

## Impact

- `dialectica-ui/src/qml/Main.qml` — the screen set, the routes into and out of
  onboarding and the thread, and `DStatusBar`'s placement.
- `dialectica-ui/src/qml/qmldir` — no new registrations expected; the reachability
  rule reads it.
- `dialectica-ui/tests/` — the reachability check needs a home that instantiates
  the root rather than a component, since no component-level suite can see this
  defect. `Main.qml` is instantiated by no spec today, which is the same blind
  spot that let `DTheme.noSuchDesk` through (CLAUDE.md records it).
- `docs/PLAN.md` — §9.2's case 2 list gains nothing; `DStatusBar`'s delivery lamp
  already has an entry's worth of reasoning in `DStatusBar.qml:55-61`, and
  whether that is a case 2 placeholder needing a PLAN entry is a question this
  change answers rather than inherits.
- No core change. No trait method is added, and no wire shape moves.
