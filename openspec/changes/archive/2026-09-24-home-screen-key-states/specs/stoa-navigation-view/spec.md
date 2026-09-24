## REMOVED Requirements

### Requirement: Creating a Stoa asks for a title and nothing else, and is always offered

**Reason**: This requirement has two halves, and only one is reversed. The
reversed half says the affordance must be offered "whatever the keystore's
state" and must not be hidden "on a guess about whether a key exists", with the
scenario "The affordance is offered when no key exists". The home-screen design
now requires "Create a Stoa" to be absent while this machine holds no key.
`identity-onboarding`'s "Whether this peer holds a master key is reportable
without creating one" turns the guess into an answer the core gives. The other
half is unchanged: a title and nothing else, the core's reason on a refusal, and
an empty title passed through.

**Migration**: Replaced by "Creating a Stoa asks for a title and nothing else,
and is offered only once the core reports a key", added below. It carries the
title-only rule, the core's-reason rule and the empty-title rule over unchanged,
and replaces "always offered" with "offered in the key-held state and not
instantiated in any other". Tests that assert the create affordance is present
with no key held assert the removed behaviour and must be inverted.

## ADDED Requirements

### Requirement: Creating a Stoa asks for a title and nothing else, and is offered only once the core reports a key

The create affordance MUST take a title and MUST NOT take, or offer to take, a
creator key or any identity selection. The creator key is fixed by the core as
this peer's master key, and `stoa-membership`'s "Creating a Stoa produces a
genesis record the creator can moderate" states that it is not a parameter.

The affordance MUST be offered when, and only when, the home screen is in its
key-held state, as defined by "The home screen's key state is one value, taken
from the core's answer in this run". In every other key state it MUST NOT be
instantiated. Hiding it, disabling it or greying it out does not meet this
requirement: its title field and its action MUST both be absent from the
screen's element tree. It MUST NOT be gated on a build flag.

Where creation fails for want of a usable key, the screen MUST render the reason
the core gave, unreworded. A key can be reported as held and still be unusable
when creation is attempted, so this path stays reachable in the key-held state.

An **empty title MUST be accepted** by the field and passed through rather than
refused.

#### Scenario: The create affordance offers a title and no key

- **WHEN** the create affordance is rendered in the key-held state
- **THEN** it accepts a title
- **AND** it offers no field for a creator key or an identity to create under

#### Scenario: The affordance is offered and usable when a key is held

- **WHEN** the home screen is in its key-held state
- **THEN** the create affordance is present and can be acted on

#### Scenario: The affordance is not instantiated when no key is held

- **WHEN** the home screen is in its no-key state
- **THEN** the screen's element tree, visible and invisible elements alike,
  contains no create action and no title field for a new Stoa

#### Scenario: The affordance is not instantiated when the key state could not be read

- **WHEN** the home screen is in its could-not-be-read key state
- **THEN** the screen's element tree, visible and invisible elements alike,
  contains no create action and no title field for a new Stoa

#### Scenario: A creation refused for want of a key renders the core's reason

- **WHEN** the home screen is in its key-held state, creation is attempted, and
  the core refuses it because no usable signing key exists
- **THEN** the screen renders the reason the core gave
- **AND** it does not report a Stoa as created

#### Scenario: An empty title reaches the core rather than being refused by the view

- **WHEN** the user creates a Stoa with an empty title
- **THEN** the create call is made
- **AND** the reply decides the outcome rather than a check in the view

### Requirement: The home screen's key state is one value, taken from the core's answer in this run

The home screen MUST be in exactly one of three key states at any time, and
which one MUST be determined by a single value, never by a combination of
independent flags:

- **No key held**: entered only on a successful reply to `identity-onboarding`'s
  master-key query that states no master key is held.
- **Key held**: entered on a successful reply to that query that states a
  master key is held and names it by a non-empty public key. It is also entered
  on a successful reply to the operation that creates a master key, if that
  reply names a non-empty public key. This holds whether or not the reply says
  the call created the key.
- **Could not be read**: entered on the failure shape from the query. It is
  also entered on a successful reply that is neither of the two shapes above,
  which includes a reply stating a key is held without naming one and a reply
  stating neither outcome.

A reply MUST NOT be treated as stating that a key is held unless it states so
with the boolean true. It MUST NOT be treated as stating that none is held
unless it states so with the boolean false.

The screen MUST ask the query when it is first shown. It MUST ask again each
time it is shown after being hidden, and an answer received for an earlier
showing MUST NOT decide the key state of a later one. It MUST also ask when the
user acts on the action that reads the key again, offered in the
could-not-be-read state under "A key state that could not be read is told apart
from both others"; the answer to that ask decides the key state by the same
rules as an answer to a showing's ask. The key state MUST NOT be
decided by any value the view persisted across runs.

The screen MUST NOT call the operation that creates a master key in order to
learn the key state. It MUST call that operation only when the user acts on the
action that creates this machine's key.

The key state MUST NOT depend on the outcome of the Stoa listing. Each of the
three key states is rendered the same way whether the listing succeeded with
Stoas, succeeded with none, or failed.

#### Scenario: A reply stating no key is held puts the screen in the no-key state

- **WHEN** the home screen is shown and the master-key query replies that no key
  is held
- **THEN** the screen is in its no-key state

#### Scenario: A reply naming a held key puts the screen in the key-held state

- **WHEN** the home screen is shown and the master-key query replies that a key
  is held, naming its public key
- **THEN** the screen is in its key-held state

#### Scenario: A failed query puts the screen in the could-not-be-read state

- **WHEN** the home screen is shown and the master-key query answers with the
  failure shape
- **THEN** the screen is in its could-not-be-read state

#### Scenario: A reply claiming a key without naming one is not the key-held state

- **WHEN** the master-key query replies that a key is held but carries no public
  key, or an empty one
- **THEN** the screen is in its could-not-be-read state
- **AND** it is not in its key-held state

#### Scenario: A reply stating neither outcome is not the no-key state

- **WHEN** the master-key query replies successfully with neither the boolean
  true nor the boolean false as its statement of whether a key is held
- **THEN** the screen is in its could-not-be-read state
- **AND** it is not in its no-key state

#### Scenario: Arriving asks the query and mints nothing

- **WHEN** the home screen is first shown
- **THEN** the master-key query has been called once
- **AND** the operation that creates a master key has not been called

#### Scenario: The key state is asked again on each showing

- **WHEN** the home screen is shown while the query replies that no key is held,
  is hidden, and is shown again while the query replies that a key is held
- **THEN** the query has been called once for each showing
- **AND** the screen is in its key-held state, the earlier answer
  notwithstanding

#### Scenario: The key state does not follow the listing

- **WHEN** the Stoa listing answers with the failure shape and the master-key
  query replies that no key is held
- **THEN** the screen is in its no-key state
- **AND** the action that creates this machine's key is rendered

#### Scenario: No persisted value stands in for the core's answer

- **WHEN** the home screen renders a key state
- **THEN** that state is taken from a reply received in this run
- **AND** no value the view persisted across runs decides it

### Requirement: With no key held, making the key is the only task on the screen

In the no-key state the screen MUST render the key block. The key block is the
label, the explanation, and an action that creates this machine's key. It MUST
be positioned above the paste section. The paste section MUST be rendered and
usable, because previewing a Stoa needs no key.

The create affordance is not instantiated in this state, under "Creating a Stoa
asks for a title and nothing else, and is offered only once the core reports a
key".

Acting on the action that creates this machine's key MUST call the operation
that creates a master key once, and the request MUST name no Stoa. The outcome
MUST follow the reply:

- a successful reply naming a non-empty public key puts the screen in its
  key-held state;
- the failure shape leaves the screen in its no-key state and renders a
  statement that no key was created, together with the core's reason unreworded.
  No key is rendered as held;
- a successful reply naming no public key, or an empty one, is treated as the
  failure shape. The screen does not reach the key-held state and renders a
  failure naming what was wrong with the reply.

A failure rendered after a press belongs to the showing in which the press was
made. It MUST NOT be rendered on any later showing of the screen, whatever key
state that showing's answer puts the screen in.

#### Scenario: The no-key state renders the key block and the paste section

- **WHEN** the home screen is in its no-key state
- **THEN** the key block's label, explanation and create-key action are rendered
- **AND** the paste field and its action are rendered
- **AND** the key block is positioned above the paste section

#### Scenario: A reference can be previewed with no key held

- **WHEN** the home screen is in its no-key state and a well-formed Stoa
  reference is pasted and acted on
- **THEN** a preview of that reference is requested
- **AND** no join call has been made

#### Scenario: Creating the key calls the mint once and names no Stoa

- **WHEN** the user acts on the create-key action
- **THEN** the operation that creates a master key has been called exactly once
- **AND** its request names no Stoa

#### Scenario: A successful mint moves the screen to the key-held state

- **WHEN** the user acts on the create-key action and the reply names a public
  key
- **THEN** the screen is in its key-held state
- **AND** the key block is no longer in the screen's element tree
- **AND** the create affordance is present

#### Scenario: A refused mint renders the core's reason and stays in the no-key state

- **WHEN** the user acts on the create-key action and the reply is the failure
  shape
- **THEN** the screen is in its no-key state
- **AND** it renders that no key was created, with the core's reason unreworded
- **AND** no key is rendered as held

#### Scenario: A mint success naming no key is not a key

- **WHEN** the user acts on the create-key action and the reply is a success
  carrying no public key
- **THEN** the screen is not in its key-held state
- **AND** no key is rendered as held

#### Scenario: A refused mint is not rendered on a later showing

- **WHEN** the user acts on the create-key action and the reply is the failure
  shape, and the screen is then hidden and shown again while the master-key
  query still replies that no key is held
- **THEN** the screen is in its no-key state
- **AND** neither the statement that no key was created nor the core's reason
  from the earlier press is rendered

### Requirement: With a key held, the key is a line at the foot of the card and making one is not offered

In the key-held state:

- the key block, meaning its explanation and its create-key action, MUST NOT be
  instantiated. It MUST be absent from the screen's element tree, not merely
  hidden.
- the create affordance MUST be rendered, positioned above the paste section.
- a key line MUST be rendered below both the create affordance and the paste
  section. It carries the label and the held key. The key MUST be rendered in
  its abbreviated form (head 8, middle 8, tail 6) through the view's one address
  component. It MUST NOT be abbreviated by any other means, and MUST NOT be
  rendered in full.
- where the reply that established the state reports the key as not protected
  at rest, the key line MUST carry the unencrypted-storage warning, rendered in
  the accent colour. Where the reply reports the key as protected, the screen
  MUST NOT render the unencrypted-storage warning. Where the reply omits the
  protection field, the screen MUST make no claim about protection either way.

The screen MUST NOT render a statement that this machine already had a key, or
that nothing was replaced. The key-held state MUST render the same text however
it was reached: from the master-key query, from a mint reply that says the call
created the key, or from one that says it did not.

#### Scenario: The key block is not instantiated when a key is held

- **WHEN** the home screen is in its key-held state
- **THEN** the screen's element tree, visible and invisible elements alike,
  contains no action that creates this machine's key
- **AND** it contains no element carrying the key block's explanation

#### Scenario: Creating a Stoa sits above pasting a reference

- **WHEN** the home screen is in its key-held state
- **THEN** the create affordance is positioned above the paste section

#### Scenario: The key line sits at the foot and abbreviates through the one component

- **WHEN** the home screen is in its key-held state for a reported public key
- **THEN** a key line carrying the label and that key is rendered
- **AND** it is positioned below both the create affordance and the paste
  section
- **AND** the key is rendered by the view's address component in its
  abbreviated form, carrying that public key

#### Scenario: An unprotected key carries the warning in the accent colour

- **WHEN** the key-held state was established by a reply reporting the key as
  not protected at rest
- **THEN** the unencrypted-storage warning is rendered
- **AND** it is rendered in the accent colour

#### Scenario: A protected key carries no unencrypted warning

- **WHEN** the key-held state was established by a reply reporting the key as
  protected at rest
- **THEN** no unencrypted-storage warning is rendered

#### Scenario: An omitted protection field produces no claim

- **WHEN** the key-held state was established by a reply carrying no protection
  field
- **THEN** no unencrypted-storage warning is rendered
- **AND** nothing rendered states that the key is protected

#### Scenario: A mint reporting an existing key says nothing about it already existing

- **WHEN** the user acts on the create-key action and the reply names a key and
  says the call did not create it
- **THEN** the screen is in its key-held state
- **AND** nothing rendered states that this machine already had a key or that
  nothing was replaced

#### Scenario: The key-held state renders the same however it was reached

- **WHEN** the key-held state is reached once from the master-key query, once
  from a mint reply saying the key was created, and once from a mint reply
  saying it was not, each naming the same key and the same protection
- **THEN** the three screens render the same text

### Requirement: A key state that could not be read is told apart from both others

In the could-not-be-read state the screen MUST render the statement
`Whether this machine holds a key could not be read.`, exactly as given, and
below it the reason. The reason is the core's message unreworded when the query
answered with the failure shape, or otherwise a failure naming what was wrong
with the reply.

In this state the screen MUST render an action labelled
`Try reading the key again`, exactly as given. Acting on it MUST call the
master-key query exactly once and MUST NOT call the operation that creates a
master key. The reply decides the key state under "The home screen's key state
is one value, taken from the core's answer in this run", so a reply naming a
held key puts the screen in its key-held state, a reply stating no key is held
puts it in its no-key state, and a failure leaves it in the could-not-be-read
state rendering the new reason. The action MUST NOT be rendered in the no-key
state or the key-held state.

In this state the screen MUST NOT instantiate the action that creates this
machine's key, and MUST NOT instantiate the create affordance. It MUST NOT
render the key block's explanation, anything stating that no key is held, or a
key line. The paste section MUST be rendered and usable.

#### Scenario: The could-not-be-read state states what failed above the reason

- **WHEN** the master-key query answers with the failure shape, and again when
  it replies successfully with neither the boolean true nor the boolean false as
  its statement of whether a key is held
- **THEN** in both cases `Whether this machine holds a key could not be read.`
  is rendered
- **AND** it is positioned above the reason

#### Scenario: Reading the key again asks the query and mints nothing

- **WHEN** the home screen is in its could-not-be-read state and the user acts
  on `Try reading the key again`
- **THEN** the master-key query has been called once more than before the action
- **AND** the operation that creates a master key has not been called

#### Scenario: Reading the key again reaches the key-held state once the key can be read

- **WHEN** the home screen is in its could-not-be-read state, the query now
  replies that a key is held naming its public key, and the user acts on
  `Try reading the key again`
- **THEN** the screen is in its key-held state
- **AND** neither the statement that the key state could not be read nor the
  earlier reason is rendered

#### Scenario: Reading the key again that fails again renders the new reason

- **WHEN** the home screen is in its could-not-be-read state, the query now
  answers with the failure shape carrying a different message, and the user
  acts on `Try reading the key again`
- **THEN** the screen is in its could-not-be-read state
- **AND** the new message is rendered unreworded
- **AND** the earlier message is not rendered

#### Scenario: The read-again action belongs to the could-not-be-read state alone

- **WHEN** the home screen is in its no-key state, and separately in its
  key-held state
- **THEN** in neither is an action labelled `Try reading the key again` rendered

#### Scenario: A failed query renders the core's reason and offers neither creation

- **WHEN** the master-key query answers with the failure shape
- **THEN** the core's message is rendered unreworded
- **AND** the screen's element tree contains neither the action that creates
  this machine's key nor the create affordance
- **AND** no key line is rendered

#### Scenario: A failed query does not claim that no key is held

- **WHEN** the master-key query answers with the failure shape
- **THEN** the key block's explanation is not rendered
- **AND** nothing rendered states that this machine holds no key

#### Scenario: Pasting stays available when the key state could not be read

- **WHEN** the home screen is in its could-not-be-read state
- **THEN** the paste field and its action are rendered and can be acted on

#### Scenario: The could-not-be-read state and the no-key state are not alike

- **WHEN** one screen is driven with a query reply stating no key is held, and
  another with the failure shape
- **THEN** the two screens are in different key states
- **AND** the text they render differs

### Requirement: The home screen's copy is the design's, verbatim

The home screen MUST render these strings exactly as given, wherever the state
it is in renders the element they belong to:

| Element | Text |
|---|---|
| heading | `Stoas you joined` |
| note beside the heading | `NO DIRECTORY EXISTS · JOIN BY ADDRESS` |
| key label, in the key block and in the key line | `THIS MACHINE'S KEY` |
| key block explanation | `A Stoa records its creator's key, so this machine needs one before it can create or post. Making it writes a key here and tells nobody. The same key signs in every Stoa you hold.` |
| create-key action | `Create this machine's key` |
| unencrypted-storage warning | `Stored unencrypted on this machine. Anyone who can read the file can post as you.` |
| create affordance label | `CREATE A STOA` |
| create title field, while empty | `Title of the new Stoa` |
| create action | `Create it` |
| paste label | `PASTE A STOA REFERENCE — THE ADDRESS AND ITS FOUNDING RECORD` |
| paste action | `Look at it first` |
| row share action | `Copy a shareable reference` |
| row open action | `Open` |

The create title field's placeholder MUST be rendered only while the field is
empty, and MUST NOT be submitted as a title. A Stoa created from an empty field
is created with the empty title.

#### Scenario: The no-key state renders its copy verbatim

- **WHEN** the home screen is in its no-key state
- **THEN** it renders the heading, the note beside it, the key label, the key
  block explanation, the create-key action, the paste label and the paste
  action, each exactly as given

#### Scenario: The key-held state renders its copy verbatim

- **WHEN** the home screen is in its key-held state for a key reported as not
  protected at rest
- **THEN** it renders the heading, the note beside it, the create affordance
  label, the create action, the paste label, the paste action, the key label and
  the unencrypted-storage warning, each exactly as given

#### Scenario: A listed row renders its actions verbatim

- **WHEN** the home screen renders a Stoa row whose record it holds
- **THEN** the row's share action reads `Copy a shareable reference`
- **AND** its open action reads `Open`

#### Scenario: The title placeholder shows only while the field is empty

- **WHEN** the create title field is empty
- **THEN** `Title of the new Stoa` is rendered in it
- **AND** once text is entered, the placeholder is no longer rendered

#### Scenario: The placeholder is never submitted as a title

- **WHEN** the user acts on the create action with the title field empty
- **THEN** the title sent to the core is the empty string
