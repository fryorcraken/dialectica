## MODIFIED Requirements

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
