## MODIFIED Requirements

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

**The thread screen MUST NOT carry a thread identifier, a Stoa address or a
genesis record of its own supplied as a property with a usable default.** This
is "The view holds no Stoa of its own, and the feed is reached from the list"
applied to the value the thread screen exists to render: a second source for it
is a build shipping a hardcoded thread. No thread screen is rendered, and no
thread read is made, before a thread has been chosen.

**The return to the feed MUST NOT be withdrawn by any state the thread screen
can be in**, a refused read included — that is the state a user is most likely
to need to leave. The return is specified as an outcome and not as a mechanism,
as "Every state a user can enter has a specified way out" states for every
return.

**What the thread screen renders is outside this requirement**, and is owned by
the capability that specifies that screen. This requirement constrains only the
transition and what is handed across it.

#### Scenario: Acting on a feed row opens its thread

- **WHEN** a user acts on a feed row
- **THEN** the thread that row heads is rendered
- **AND** it is given that row's Stoa, that root post's identifier, and the
  founding record wherever the view holds one for that Stoa
- **AND** the thread read made names that row's thread identifier and that Stoa

#### Scenario: No record is invented for a Stoa the view has none for

- **WHEN** a thread is opened for a Stoa the view holds no founding record for
- **THEN** the thread is not given a fabricated or placeholder record

#### Scenario: The feed is reached again from the thread

- **WHEN** a thread is open
- **THEN** an affordance returning to the feed is offered
- **AND** acting on it renders the feed for the Stoa the thread was opened from,
  without the view being restarted

#### Scenario: The way out survives a refused read

- **WHEN** a thread is open and its read was refused
- **THEN** an affordance returning to the feed is still offered
- **AND** acting on it renders the feed

#### Scenario: No thread is rendered before one has been chosen

- **WHEN** the view is started and no thread has been chosen
- **THEN** no thread screen is rendered
- **AND** no thread read call is made

#### Scenario: The thread screen carries no thread of its own

- **WHEN** the thread screen's properties are enumerated
- **THEN** none supplies a thread identifier, a Stoa address or a genesis record
  holding a usable default

## ADDED Requirements

### Requirement: The moderation screen is reachable, and leaving it returns where the user was

The moderation screen MUST be reachable from the view's root through an
affordance a user can act on, and MUST offer a way back to the screen it was
reached from without the view being restarted.

**Reachability is contracted because a registered screen nobody can open passes
every test written about it.** That is the defect this capability was added for,
and a screen whose entire purpose is that the owner can look at it is the worst
possible one to leave unmounted. A static gate proves the screen is instantiated
somewhere the root reaches; this requirement is what makes a route to it part of
the contract rather than an implementation detail a later change may drop.

The return MUST remain available whatever the user did on the screen. Acting on an
inert control MUST NOT withdraw it.

What the moderation screen renders, and what its controls may claim, is
`moderation-view`'s. This requirement constrains only the route in and out.

#### Scenario: The screen is reached from where its subject lives

- **WHEN** a user acts on the affordance that opens moderation
- **THEN** the moderation screen is rendered

#### Scenario: The way back is offered and works

- **WHEN** the moderation screen is rendered and the user acts on its way out
- **THEN** the screen it was reached from is rendered again
- **AND** the view has not been restarted

#### Scenario: Acting on an inert control does not strand the user

- **WHEN** a control on the moderation screen is acted on
- **THEN** the way out is still offered
