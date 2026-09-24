# view-navigation Specification

## Purpose
Defines which screens the view can reach, what selects between them, what travels across each transition, and that every registered component is instantiated somewhere or is recorded as deliberately not — so that a screen which works is also a screen a user can get to, and a user who enters a state can leave it. It exists because a component test instantiates the component it tests and therefore cannot distinguish a working screen from a working-but-unreachable one, which is how the view's only route to acquiring an identity came to be dead code under a passing suite.

**Boundary with the screen capabilities.** This capability owns transitions and nothing a screen renders. `view-identity-onboarding` owns what the onboarding screen shows and what it must never claim; `stoa-navigation-view` owns the list, preview, creation and sharing screens; `feed-view` owns the feed's ordering and time claims; `composer-view` owns where the posting gate is enforced and what a closed gate says; `thread-read` and the thread screen's own capability own what a thread renders. Nothing below re-specifies any of them, and a requirement here that named what a screen displays would put a second live requirement on a behaviour another capability already holds.

## Requirements

### Requirement: Every registered screen type is reachable, or is recorded as deliberately unreachable

Every type registered in the view's module registration MUST either be
instantiated somewhere the view's root reaches, or be recorded as deliberately
uninstantiated together with the reason it is.

**The rule covers every registration rather than the subset judged to be
screens**, and that breadth is deliberate. "Is this a screen?" is a judgement no
check can make from a registration, so a rule scoped to screens would need a
hand-maintained list of which types count — the shape this repo has already
recorded as going stale silently, since a type added to the registration would
enter unprotected with every check still green. A rule over every registration
needs no such list: a leaf component is reachable through whatever screen
instantiates it, so it discharges the rule without being named anywhere, and only
a type nothing instantiates ever has to be accounted for.

**A registration and an instantiation are different facts, and only the second
one a user can act on.** A registered type nothing instantiates is a component
that compiles, renders correctly when a test builds it, passes every assertion
made about it, and cannot be opened. This is the state the view's only route to
acquiring an identity was in.

**No component-level test can discharge this requirement, and that is why it is
here.** A component suite instantiates the component under test itself, so it
supplies exactly the reachability whose absence is the defect — the component is
reachable *from the test* no matter what the application does. Whatever checks
this MUST therefore read the registrations and the view's sources together,
rather than asserting against a component it constructed.

**A deliberately uninstantiated type is not a defect, and the record is what
makes the difference visible.** A type held back by a scope decision and a type
nobody noticed present identically; the reason is the only thing that
distinguishes them, and it MUST state why rather than merely asserting that the
omission was intended.

#### Scenario: A registered type nothing instantiates is reported

- **WHEN** a type is registered and no source the view's root reaches
  instantiates it
- **AND** no record states that it is deliberately uninstantiated
- **THEN** the check reports it

#### Scenario: A deliberately uninstantiated type is accepted with its reason

- **WHEN** a type is registered, is instantiated nowhere, and is recorded as
  deliberately uninstantiated with a stated reason
- **THEN** the check does not report it

#### Scenario: A newly registered type is covered without being listed

- **WHEN** a type is added to the registration, is instantiated nowhere, and is
  named by no list inside the check
- **THEN** the check reports it

### Requirement: Identity routing reads both probes, and neither answer is cached

The view MUST decide where a user without a usable identity is sent from **both**
the identity report and the posting capability probe, and MUST NOT decide it from
the posting probe alone. Neither answer may be retained across a rendering of the
state it decides: each MUST be re-read whenever that state is rendered.

**The two probes answer different questions and can honestly disagree.** A
stored identity whose keystore cannot currently be used is a real identity with
a fixable problem. Routing on the posting probe alone collapses that user into
"there is nobody here" and sends them to identity *creation* — which is the one
irreversible wrong answer available, since what is kept cannot afterwards be
changed. The user who most needs to be told what is wrong is instead offered a
new key.

**The prohibition on caching is the same rule the posting gate already carries,
one level up.** A retained "this user has an identity" outlives the keystore it
stands for: a key that became unusable between one render and the next leaves
the retained answer describing a state that no longer exists, and a navigation
layer holding it is wrong for every screen at once rather than for one control.
The identity report is subject to the same rule for the same reason — it reads a
store the view cannot see, so a previous run's answer is not evidence about this
one.

**What the two answers route to is three outcomes and not two.** A user with no
identity, a user with an identity that cannot be used, and a user who can act
are three states, and a routing that renders only two of them must be silently
merging a pair.

#### Scenario: A user with no identity reaches identity creation

- **WHEN** the identity report says no identity exists
- **THEN** the route to acquiring one is offered

#### Scenario: An unusable identity is not routed to creation

- **WHEN** the identity report says an identity exists
- **AND** the posting probe says posting is not possible
- **THEN** the user is not sent to identity creation as though they had none

#### Scenario: Both probes are re-read on each render of the routing state

- **WHEN** the state whose selection depends on them is rendered a second time
- **THEN** both the identity report and the posting probe are asked again
- **AND** neither is answered from a value retained from the earlier render

#### Scenario: No persisted value stands in for either probe

- **WHEN** the view is started and the routing state is first rendered
- **THEN** the route taken is decided from answers received in this run
- **AND** no value the view retained from a previous run decides it

### Requirement: A thread is opened from a feed row and can be left

A row of a Stoa's feed MUST be able to open the thread it heads, and a user who
has opened one MUST be able to return to the feed they came from without
restarting the view.

**What travels is the Stoa the feed is for, that Stoa's founding record where
the view holds one, and the root post's identifier.** The record travels for the
same reason it travels to the feed: moderation cannot be resolved for a Stoa
whose record this peer does not hold. **The view MUST NOT invent a record it was
not given** and MUST NOT substitute a placeholder or an empty one for a real one;
where it holds none, the thread is opened without one and whatever the core then
refuses is rendered as the refusal it is. A fabricated record fails verification
in the core and surfaces as a failure the user cannot act on.

**The identifier that travels is the root post's**, which does not move when the
post is edited — an identifier that changed under a revision would leave the
return route pointing at a thread that no longer answers to it.

**What the thread screen renders is outside this requirement**, and is owned by
the capability that specifies that screen. This requirement constrains only the
transition and what is handed across it.

#### Scenario: Acting on a feed row opens its thread

- **WHEN** a user acts on a feed row
- **THEN** the thread that row heads is rendered
- **AND** it is given that row's Stoa, that root post's identifier, and the
  founding record wherever the view holds one for that Stoa

#### Scenario: No record is invented for a Stoa the view has none for

- **WHEN** a thread is opened for a Stoa the view holds no founding record for
- **THEN** the thread is not given a fabricated or placeholder record

#### Scenario: The feed is reached again from the thread

- **WHEN** a thread is open
- **THEN** an affordance returning to the feed is offered
- **AND** acting on it renders the feed for the Stoa the thread was opened from,
  without the view being restarted

### Requirement: The view holds no Stoa of its own, and the feed is reached from the list

The view MUST obtain the Stoa it renders a feed for from the membership listing,
and MUST NOT carry a Stoa address or genesis record supplied as a property with a
default.

Today the top-level view takes an address, a title and a genesis record as
properties a developer fills in, empty by default, because nothing in the core
recorded which Stoas this peer was in. That is no longer true, and leaving the
properties in place leaves a second source for the one value this screen exists to
supply — a build shipping a hardcoded Stoa, and a Stoa on screen that the
membership does not record.

The address MUST travel from the list item to the feed, and the genesis record
MUST travel with it wherever the view holds one for that Stoa. The feed needs the
record because moderation cannot be resolved for a Stoa whose record this peer
does not hold, and passing it from the view is safe rather than a hole: the
address is the hash of the record, so the core re-derives it and refuses a
mismatch.

**The view MUST NOT invent a record it was not given**, and MUST NOT send a
placeholder or an empty one in place of a real one. Where no record is available
for the chosen Stoa the feed is opened with the address alone and whatever the
core then refuses is rendered as the failure it is — which is honest, and is
distinguishable from a Stoa holding nothing. A fabricated record would fail
verification in the core and surface as a refusal the user cannot act on.

#### Scenario: The feed is rendered for a Stoa chosen from the list

- **WHEN** a user acts on a row of the Stoa list for which the view holds a
  genesis record
- **THEN** the feed is rendered for that row's Stoa
- **AND** it is given both that Stoa's address and that genesis record

#### Scenario: No record is invented for a Stoa the view has none for

- **WHEN** a user acts on a row for which the view holds no genesis record
- **THEN** the feed is not given a fabricated or placeholder record

#### Scenario: No Stoa is rendered before one has been chosen

- **WHEN** the view is started and no Stoa has been chosen
- **THEN** no feed is rendered for any Stoa
- **AND** the view does not supply a Stoa address of its own

### Requirement: Every state a user can enter has a specified way out

A user who reaches the join preview MUST be able to return to the list without
restarting, both before acting and after a join has succeeded; a Stoa created
through the create affordance MUST reach the list without a restart; and a user
who has opened a Stoa's feed MUST be able to return to the list without a
restart.

**The general rule is that arriving somewhere is half a transition**, and a spec
that pins only the arrival leaves the return implemented by habit rather than by
contract. That is not hypothetical here: a return route that no scenario requires
can be deleted with every test still passing, so it is unprotected precisely
because it works. Stating it once, as a property of the view's states rather than
as a list of buttons, is what stops the next view piece rediscovering it.

**A return is specified as an outcome, not as a mechanism.** Whether it is a
cancel affordance, a back affordance, or an automatic return when a join
completes is a design decision; what is required is that the user reaches the list
again without restarting, and that the affordance which does it is not withdrawn
by the very state it exists to leave. A control hidden once the user has succeeded
is a control absent exactly when the return is needed.

The join preview's return MUST remain available after a successful join. Reporting
success and offering nothing further is a terminal state, and a user who has just
joined a Stoa is the user most likely to want to open it.

**The feed-to-list return is required here, and was previously carved out of this
requirement as unspecifiable.** That carve-out recorded that no route existed and
that none could be specified without the feed gaining a way to say it is finished.
The feed now has one, so the transition is contracted rather than named as a gap —
and until it was, it was a working route no scenario protected.

#### Scenario: The preview can be left without joining

- **WHEN** the preview is rendered and the user declines it
- **THEN** the list is rendered again
- **AND** no join call has been made

#### Scenario: The return is still available after a join succeeds

- **WHEN** a join has succeeded and the joined outcome is rendered
- **THEN** an affordance returning to the list is still offered
- **AND** acting on it renders the list

#### Scenario: A created Stoa reaches the list without a restart

- **WHEN** a Stoa is created successfully
- **THEN** the listing is read again
- **AND** the created Stoa is among the rows rendered, without the view being
  restarted

#### Scenario: The list is reached again from a Stoa's feed

- **WHEN** a Stoa's feed is rendered
- **THEN** an affordance returning to the list is offered
- **AND** acting on it renders the list, without the view being restarted

### Requirement: Exactly one screen is shown, and the selection has one source

The view MUST render exactly one main-area screen at a time, and the screen
shown MUST be determined from a single source of state rather than from
independent values that can each be set separately.

**Two independent values selecting one screen is a state machine with
unrepresentable-but-constructible states.** Where each screen's visibility is
decided by its own flag, the combination in which two are set has no rendering,
and whichever test happens to be written first silently fixes the resolution.
Deriving the selection from one source makes the ambiguous combination
impossible to construct rather than resolved by accident.

This requirement constrains the selection's **structure**, not the mechanism
that implements it: whether the view keeps a stack, an enumerated state, or a
value derived from other state is a design decision, and any of them discharges
this so long as the shown screen has one source.

#### Scenario: One screen is shown at a time

- **WHEN** the view is rendered in any state it can reach
- **THEN** exactly one main-area screen is rendered

#### Scenario: A transition does not leave the previous screen shown

- **WHEN** the view moves from one screen to another
- **THEN** the screen departed from is no longer rendered

### Requirement: Shared chrome states which screens it accompanies

Chrome the view renders alongside its screens — an element that is not itself a
screen and does not replace one — MUST be rendered in a stated set of states,
and that set MUST be specified rather than left to follow from wherever it
happens to be placed.

**Unplaced chrome and chrome placed everywhere are both defensible, and neither
is the default.** A status indicator that is absent from a screen is an
indicator whose absence a user reads as "nothing to report", which is a claim.
Chrome reporting the machine's condition is most needed exactly where something
has gone wrong, so where it is rendered at all, it MUST be rendered on **every**
main-area screen rather than on a subset. A per-screen subset is what makes its
absence ambiguous: a user cannot tell a screen that omits the indicator from a
machine with nothing to report.

**Chrome MUST be rendered before the values it reports have arrived, and MUST
fall to its least-claiming appearance while they have not.** Withholding it
until the view knows what to say is the tempting alternative and it is wrong:
every screen passes through the unbound state between appearing and the core
answering, so withholding hides the indicator exactly during the window in which
the machine's condition is least known. An unbound indicator rendering health
asserts that the machine works on the strength of no data, which is the same
overclaim in its strongest form; rendering it at its least-claiming appearance
says only that the answer has not arrived.

#### Scenario: Chrome accompanies every main-area screen

- **WHEN** each main-area screen the view can reach is rendered in turn
- **THEN** the chrome is rendered alongside each of them

#### Scenario: Unbound chrome does not report health

- **WHEN** the chrome is rendered and the values it reports have not been supplied
- **THEN** it does not render the appearance that asserts the machine is working
- **AND** it renders its least-claiming appearance instead

#### Scenario: Chrome is not withheld until its values arrive

- **WHEN** a screen is rendered and no value the chrome reports has yet been
  supplied
- **THEN** the chrome is rendered

### Requirement: Acquiring an identity is reached from the navigator

The view MUST offer a route from the state that reports a missing identity to the
place where one is acquired.

**In this release that place is the Stoa list, where this machine's key is
created**, because the identity in use in every Stoa is the machine key
(`identity`: *In this release one machine key is the identity in every Stoa*).
Acting on the route MUST render the Stoa list. What the list shows is
`stoa-navigation-view`'s, and nothing here specifies it.

**Following the route MUST NOT itself make any call to the module.** Creating the
key is an action the user takes on the list. A call made in consequence of
following the route, whether by the navigator or by the list on being shown,
MUST NOT create a key, request a slate, or keep a candidate on the user's behalf.

**Rendering the list is a showing of it, and the calls the list makes when shown
are the list's, not the route's.** What the list asks the module on being shown
is `stoa-navigation-view`'s to require, and beyond the prohibition above this
requirement neither forbids nor requires any of it. In this release that is the
read-only master-key query, which `stoa-navigation-view` requires on every
showing, so following the route leads to exactly the calls a showing of the list
makes and to none besides.

**The per-Stoa onboarding screen MUST NOT be reachable in this release.** The view
MUST NOT instantiate it, and its registration MUST carry the record that it is
deliberately uninstantiated, with a reason, which *Every registered screen type
is reachable, or is recorded as deliberately unreachable* requires of a type
nothing instantiates. Per-Stoa identity, which that screen acquires, is out of
scope for this release.

Once a key exists, the identity a Stoa's feed shows is taken from a fresh answer
to the identity report, as *Identity routing reads both probes, and neither answer
is cached* already requires; nothing about the route carries an identity back.

#### Scenario: A missing identity offers the route to acquiring one

- **WHEN** the routing state reports that no identity exists
- **THEN** an affordance reaching the place where one is acquired is offered
- **AND** acting on it renders the Stoa list

#### Scenario: Following the route makes no call of its own

- **WHEN** the affordance leading to acquiring an identity is acted on
- **THEN** every call that reaches the bridge in consequence is one the Stoa list
  makes on being shown

#### Scenario: Following the route creates no key, requests no slate and keeps nothing

- **WHEN** the affordance leading to acquiring an identity is acted on
- **THEN** no key-creation call, no slate call and no keep call reaches the bridge

#### Scenario: The per-Stoa onboarding screen is instantiated nowhere

- **WHEN** the view's registrations and the sources its root reaches are read
  together
- **THEN** the per-Stoa onboarding screen is instantiated nowhere the view's root
  reaches
- **AND** its registration carries the record that it is deliberately
  uninstantiated, with a reason
