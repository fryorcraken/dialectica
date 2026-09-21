## ADDED Requirements

### Requirement: The screen states that nothing it offers takes effect

Every control on this screen is inert in this build, and the screen MUST say so
in what it renders. It MUST NOT be possible to reach a state in which a control
appears actionable and nothing on the screen accounts for the fact that acting on
it does nothing.

**An inert destructive control is a new hazard rather than a neutral
placeholder**, which is why this is the first requirement rather than a note at
the end. A user who presses "Mark as moderated" and is told nothing has one of two
beliefs afterwards, and the software chose which: that the post is now moderated
for the Stoa's readers, or that the app is broken. The first is the dangerous one
— a moderator who believes a post is hidden stops dealing with it. The screen
therefore carries the absence in its own rendering rather than relying on nothing
visibly happening, because nothing visibly happening is exactly what a successful
moderation would also look like on this screen.

The account MUST name the reason: no moderation-publishing method exists on the
core contract, so there is nothing for these controls to call. **It MUST NOT
describe the absence as a failure, an error, or a temporary fault**, none of which
it is.

#### Scenario: The inertness is rendered, not merely true

- **WHEN** the screen is rendered
- **THEN** what is rendered states that its controls publish nothing in this
  build
- **AND** it names the absence of a core method as the reason

#### Scenario: The account is not an error state

- **WHEN** the screen is rendered
- **THEN** nothing rendered presents the inertness as a failure or as a fault to
  be retried

### Requirement: No control on this screen makes a call, and no reply is rendered as one

The screen MUST make no call to the core, and MUST NOT render any value obtained
from one.

This is the half that keeps the previous requirement from decaying into a
sentence somebody edits. A screen that made a call and discarded the reply, or
that made one "to see whether it works", would be a screen whose inertness is a
property of its current bindings rather than of what it is — and the first person
to wire one control would have no contract telling them the others are not merely
unfinished.

The lists this screen renders MUST be fixtures held in the view. They MUST NOT be
read from any core reply, and MUST NOT be presented as this peer's moderation
state.

#### Scenario: Acting on a control publishes nothing

- **WHEN** any control on this screen is acted on
- **THEN** no call is made to the core
- **AND** nothing on the screen changes to report a moderation as having happened

#### Scenario: The lists are fixtures and say so

- **WHEN** the screen renders its moderated authors and its moderated posts
- **THEN** what is rendered is not obtained from any core reply
- **AND** nothing rendered states that these are the posts or authors this peer
  has moderated

### Requirement: The screen states the ceiling on what moderation can do

Where the screen explains moderating, it MUST state that moderating asks the
Stoa's readers to hide a post, and MUST NOT state or imply that it deletes a post
or removes a person from the Stoa.

**This is a permanent property of the design and not a limitation of this
build.** A Stoa is a set of peers exchanging signed ops; there is no server
holding the only copy and no membership to revoke, so a moderation op is a
published judgement that honest readers honour, and nothing more. A screen
promising deletion promises something the architecture forbids — and it would
promise it to the one user whose decisions depend on knowing the difference. The
wording survives the arrival of a real publishing path unchanged, which is the
test of whether a claim belongs in a contract or in a comment.

#### Scenario: What moderating does is stated at its true reach

- **WHEN** the screen renders its explanation of moderating
- **THEN** what it states is that readers are asked to hide the post
- **AND** nothing rendered states that a post is deleted or that a person is
  removed from the Stoa or the network

### Requirement: The two lists are two decisions, and unmaking one does not unmake the other

The screen MUST render moderated authors and moderated posts as two separate
lists, each row carrying its own control, and MUST NOT offer a single control
that acts on both.

Moderating an author and moderating one of their posts are different judgements
with different scopes, and they are independently reversible by design. A shared
control would make the smaller reversal unavailable: a moderator who wanted one
post back would have to un-moderate the author. The separation is contracted
rather than left to the layout because it is a property of what the lists mean,
and a later screen that merged them would look like a simplification.

#### Scenario: Each list carries its own per-row control

- **WHEN** the screen renders a moderated author and a moderated post
- **THEN** each row carries a control that names only that row
- **AND** no control acts on both lists

### Requirement: An address on this screen is rendered by the view's address component

Every address this screen renders MUST go through the view's existing address
component, which owns the head-8 middle-8 tail-6 abbreviation. A second
abbreviation MUST NOT be written here.

An elision keeping only a head and a tail is the shape vanity-address generators
are built to defeat, and a second implementation is how one screen quietly
acquires the weaker form. This screen is a place a reader decides who somebody is,
which is the case that makes the middle group load-bearing.

An address MUST be rendered as plain text, and a name accompanying it MUST be
rendered as plain text, through no markup-interpreting path.

#### Scenario: Addresses carry the middle group

- **WHEN** the screen renders an address
- **THEN** what is rendered is the view's abbreviation, carrying a middle group
  as well as a head and a tail

#### Scenario: A name is not interpreted as markup

- **WHEN** the screen renders a name containing markup characters
- **THEN** the name is rendered as the literal characters it contains

### Requirement: The screen is reachable, and leaving it returns where the user was

The screen MUST be reachable from the view's root through an affordance a user
can act on, and MUST offer a way back to the screen it was reached from without
the view being restarted.

**Reachability is contracted because a registered screen nobody can open passes
every test written about it.** That is the defect `view-navigation` was added for,
and a screen whose entire purpose is that the owner can look at it is the worst
possible one to leave unmounted. A static gate proves the screen is instantiated
somewhere the root reaches; this requirement is what makes a route to it part of
the contract rather than an implementation detail a later change may drop.

The return MUST remain available whatever the user did on the screen. Acting on an
inert control MUST NOT withdraw it.

#### Scenario: The screen is reached from where its subject lives

- **WHEN** a user acts on the affordance that opens moderation
- **THEN** the moderation screen is rendered

#### Scenario: The way back is offered and works

- **WHEN** the moderation screen is rendered and the user acts on its way out
- **THEN** the screen it was reached from is rendered again
- **AND** the view has not been restarted

#### Scenario: Acting on an inert control does not strand the user

- **WHEN** a control on the screen is acted on
- **THEN** the way out is still offered
