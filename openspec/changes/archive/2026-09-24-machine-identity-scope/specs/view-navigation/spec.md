## ADDED Requirements

### Requirement: Acquiring an identity is reached from the navigator

The view MUST offer a route from the state that reports a missing identity to the
place where one is acquired.

**In this release that place is the Stoa list, where this machine's key is
created**, because the identity in use in every Stoa is the machine key
(`identity`: *In this release one machine key is the identity in every Stoa*).
Acting on the route MUST render the Stoa list. What the list shows is
`stoa-navigation-view`'s, and nothing here specifies it.

**Following the route MUST NOT itself reach the module.** Creating the key is an
action the user takes on the list; the route MUST NOT create a key, request a
slate, or keep a candidate on the user's behalf.

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

#### Scenario: Following the route asks the module for nothing

- **WHEN** the affordance leading to acquiring an identity is acted on
- **THEN** no key-creation call, no slate call and no keep call reaches the bridge

#### Scenario: The per-Stoa onboarding screen is instantiated nowhere

- **WHEN** the view's registrations and the sources its root reaches are read
  together
- **THEN** the per-Stoa onboarding screen is instantiated nowhere the view's root
  reaches
- **AND** its registration carries the record that it is deliberately
  uninstantiated, with a reason

## REMOVED Requirements

### Requirement: Onboarding is entered from the navigator and returns to it

**Reason**: It routes a user with no identity to the per-Stoa onboarding screen,
which acquires a per-Stoa identity — out of scope for this release, in which the
identity in use in every Stoa is the machine key. Its scenarios describe entering
and leaving a screen the view no longer instantiates.

**Migration**: The route from a missing identity now leads to the Stoa list,
where this machine's key is created — see *Acquiring an identity is reached from
the navigator*. The requirement's return rules (no successor chosen by the
screen, a return not conditional on a keep succeeding) apply again when per-Stoa
identity is restored and the screen is reachable.
