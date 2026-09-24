## RENAMED Requirements

- FROM: `### Requirement: An identity is what participation needs, and this screen is where one is made`
- TO: `### Requirement: An identity is what participation needs, and this screen is withheld in this release`

## MODIFIED Requirements

### Requirement: An identity is what participation needs, and this screen is withheld in this release

An identity MUST be required before a user posts, replies or votes in a Stoa,
and MUST NOT be required to reach the list of Stoas, to look one up by address,
or to read one.

**Identity gates participation inside a Stoa — not launch, and not browsing.** A
peer holding no identity reaches their Stoas, pastes an address, and reads what
is there; what they cannot do is contribute to it. Gating launch instead would
demand a permanent, unchangeable choice from a user who has not yet seen
anything to decide it against, and would make a first run impossible to complete
for a peer whose keystore cannot be read at all.

**In this release the identity is this machine's key, the same in every Stoa, and
it is acquired on the Stoa list — not on this screen.** This screen acquires a
per-Stoa identity, which is out of scope for this release, so the view does not
instantiate it; `view-navigation` owns the route and the record that the screen
is deliberately uninstantiated. Every other requirement of this capability
describes the screen itself, and continues to hold of it as a component: it stays
built for the release that restores per-Stoa identity.

**The affordance leading a user to acquire an identity MUST NOT present that
identity as one chosen for a single Stoa.** In this release there is no such
choice, and the key a user creates signs in every Stoa.

**Where the posting gate is enforced, and the guidance offered from a closed
gate, are not this capability's.** `composer-view` owns both: it requires the
compose, reply and vote affordances to be rendered only when the posting probe
says posting is possible, requires a closed gate to show the probe's reason
verbatim, and requires an affordance leading to guidance on resolving the
blockage. Restating any of that here would put two live requirements on one
behaviour.

What this capability owns is the screen itself and the one signal by which a
keep becomes known outside it. When a keep reports that a candidate was kept,
the screen MUST announce that fact, and MUST announce it on no other outcome.
That signal is the whole of this screen's outward contract: whoever navigates
here decides what happens next, and the screen MUST NOT navigate anywhere
itself. A screen that chose its own successor would have to know which of
several callers routed to it, which it cannot.

The screen MUST take what it reports about stored state from the module's
replies alone, and MUST NOT report an identity from any value it stored itself
on a previous run. A remembered "this user has onboarded" outlives the thing it
remembers: a keystore that was deleted, moved, or is unreadable leaves the flag
set while the store it stands for is gone. The module is the only party that can
see the store, so it is the only party that can answer.

#### Scenario: The affordance to acquire an identity is not a per-Stoa choice

- **WHEN** a Stoa's feed reports that no identity exists
- **THEN** the affordance leading to acquiring one is offered
- **AND** no text on that affordance names a Stoa as what the identity is for

#### Scenario: A kept candidate is announced

- **WHEN** the keep reply says the candidate was kept
- **THEN** the screen announces that an identity was kept

#### Scenario: No other outcome announces one

- **WHEN** a keep is refused, and when the keep call comes back as the failure
  shape
- **THEN** neither announces that an identity was kept

#### Scenario: The screen navigates nowhere of its own accord

- **WHEN** a candidate is kept
- **THEN** the screen remains the screen that is shown
- **AND** it selects no successor screen itself

#### Scenario: No stored flag stands in for the module's answer

- **WHEN** the screen reports anything about what is stored
- **THEN** every such report is taken from a reply received in this run
- **AND** no value the view persisted across runs decides it
