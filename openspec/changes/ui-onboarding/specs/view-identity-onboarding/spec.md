## Purpose

Defines what the view shows while a user acquires an identity — the states between a fresh install and a kept key, what a candidate row may present, and what the screen must never claim about permanence, linkability, encryption or backup — so that a user chooses a key knowing what choosing it costs, and no screen reports an identity that was not stored.

**Boundary with other capabilities.** `identity-onboarding` owns what a slate is, what keeping guarantees and what the replies carry; `module-wire-contract` owns the request and reply envelope. This capability owns only what the view does with those replies, and deliberately restates none of them.

## ADDED Requirements

### Requirement: The view reaches onboarding through the one core bridge, by name

The view SHALL call the slate, keep and identity-report methods through the same
single bridge every other core call goes through, and SHALL address each by a
named wrapper rather than by a method string written at the call site.

The view has no second route to anything: the engine is sandboxed with a
deny-all network access manager and no filesystem access, so a method the bridge
does not expose is behaviour no screen can reach. Naming each wrapper once is
what makes a renamed core method a single edit, and what stops a typo becoming a
screen that renders its empty state forever.

Each request SHALL be a JSON object carrying the fields that method reads and
SHALL NOT carry a field standing in for the identity being acted on. The
identity follows from the Stoa and the selection; a request naming one would ask
the module to act as somebody it is not.

#### Scenario: Each call reaches the bridge under its own method name

- **WHEN** the slate call, the keep call and the identity-report call are each made
- **THEN** each reaches the bridge naming a different method
- **AND** each names the same module as every other core call the view makes

#### Scenario: A request carries the fields its method reads

- **WHEN** the slate call and the identity-report call are made for a Stoa
- **THEN** each request is a JSON object carrying that Stoa
- **AND** the keep call's request additionally carries the set identifier it was
  offered and the position selected

#### Scenario: No request names an identity

- **WHEN** any of the three requests is made
- **THEN** no request carries a field naming an address, a public key or a
  derivation path as the identity to act as

### Requirement: The launch branch is the module's answer, never a remembered flag

The view SHALL decide whether to show onboarding or the forum by asking the
module who the user is, and SHALL make that decision from the reply alone. It
SHALL NOT decide from any value it stored itself on a previous run.

A remembered "this user has onboarded" outlives the thing it remembers: a
keystore that was deleted, moved, or is unreadable leaves the flag set and the
user looking at a forum they cannot post in, with no path back to the screen
that would fix it. The module is the only party that can see the store, so it is
the only party that can answer.

The reply distinguishes an identity present from an identity absent, and carries
a reason when absent. **Both absent cases SHALL reach onboarding** — no identity
stored, and an identity that exists but could not be loaded — because in each
the user has no usable identity and the screen that offers one is where they
must arrive. Where the reply is a failure rather than an answer, the view SHALL
show that failure rather than either branch, because it does not know which
branch is right.

#### Scenario: An identity reported present shows the forum

- **WHEN** the identity report says there is an identity
- **THEN** the view shows the forum rather than onboarding

#### Scenario: An identity reported absent shows onboarding

- **WHEN** the identity report says there is none
- **THEN** the view shows onboarding rather than the forum

#### Scenario: An unloadable identity shows onboarding rather than the forum

- **WHEN** the identity report says there is none, with a reason that names a
  store it could not read rather than an absent one
- **THEN** the view shows onboarding
- **AND** the two absent replies are distinguished by the reason the view holds,
  so a later screen can tell them apart

#### Scenario: A failed report shows the failure rather than guessing a branch

- **WHEN** the identity report comes back as the failure shape
- **THEN** the view shows neither the forum nor a slate
- **AND** the module's own message is what is shown

#### Scenario: The branch is re-asked rather than remembered

- **WHEN** the identity report is asked for, and then asked for again after the
  stored state has changed from having an identity to having none
- **THEN** the second answer decides the branch
- **AND** the first answer does not

### Requirement: Onboarding opens with no identity and nothing requested

Before the user acts, the screen SHALL show that no identity exists and SHALL
NOT have requested a slate.

A slate generated on arrival is a set of candidates the user never asked to see,
and it makes the first thing on screen a choice rather than an explanation of
what is being chosen. The screen's opening job is to say what a key is and that
it is permanent; the candidates arrive when the user asks for them.

The opening state SHALL offer exactly one action that reaches the module — the
one that requests a set of candidates. The identity report that decided this
screen would be shown is not such an action: it precedes the screen and is the
previous requirement's.

#### Scenario: The opening state has no candidates and made no slate call

- **WHEN** the onboarding screen is first shown
- **THEN** it holds no candidates
- **AND** no slate call has reached the bridge
- **AND** no keep call has reached the bridge

#### Scenario: The opening state says an identity is being chosen

- **WHEN** the onboarding screen is first shown
- **THEN** it shows the heading `Choose the identity you will keep here.`
- **AND** it shows text stating that a key is being picked and that its name
  cannot be changed afterwards

#### Scenario: Requesting candidates is what calls the module

- **WHEN** the action that requests candidates is invoked from the opening state
- **THEN** exactly one slate call reaches the bridge

### Requirement: A slate is shown as the module reported it, with its own count

The screen SHALL show one row per candidate the reply carries, in the order the
reply carries them, and SHALL take how many there are from the reply rather than
from a number written into the view.

The count is reported with the set precisely so that a caller need not hardcode
it; a view that laid out a fixed number of rows would show four of five, or an
empty fifth, the moment the module's count changed — and the module's count is
the module's to change.

The screen SHALL NOT present a slate whose reply it could not read as a slate. A
reply carrying no list of candidates, or a list that is not a list, SHALL be
shown as a failure rather than as a slate with nothing in it, for the same
reason an unreadable store must never render as an empty one: the two mean
opposite things and look identical when collapsed.

#### Scenario: Every candidate in the reply gets a row, in order

- **WHEN** a slate reply carrying several candidates is received
- **THEN** the screen holds as many candidates as the reply carried
- **AND** they are in the reply's order

#### Scenario: The count comes from the reply

- **WHEN** a slate reply is received whose candidate list is of a different
  length from any number the view might have assumed
- **THEN** the screen shows exactly that many rows

#### Scenario: A reply with no candidate list is a failure, not an empty slate

- **WHEN** a slate reply carries no list of candidates, or carries one that is
  not a list
- **THEN** the screen is in its failed state rather than showing a slate
- **AND** the failure names itself rather than leaving the screen blank

#### Scenario: A failed slate call is shown as the module's own failure

- **WHEN** the slate call comes back as the failure shape
- **THEN** the screen is in its failed state
- **AND** the module's message is shown unchanged rather than reworded

### Requirement: Asking for more candidates is unlimited and replaces what was shown

The action that requests a further set SHALL be available whenever a set is on
screen, SHALL be refused by the view for no reason of its own, and SHALL replace
the candidates shown with those of the new reply.

Regeneration is unlimited by contract, and a view that rationed it — a count, a
cooldown, a disabled control after some number of presses — would impose a limit
the module does not have, on the one action that makes the result a choice
rather than a value the user was handed. A user who refreshed forty times meant
the one they kept.

A new set SHALL discard the previous selection. The selection names a position
within a particular set, and a position carried across a refresh names a
candidate the user never looked at.

#### Scenario: Repeated requests are each answered

- **WHEN** the request-more action is invoked many times in succession
- **THEN** the number of slate calls that reached the bridge equals the number of
  invocations
- **AND** the last invocation's candidates are the ones on screen, so the final
  request was served rather than swallowed

#### Scenario: The control is never disabled by how often it was used

- **WHEN** the request-more action has been invoked many times
- **THEN** the control offering it is still enabled
- **AND** a further invocation still reaches the bridge

#### Scenario: A new set replaces the old candidates

- **WHEN** a set is shown, and a further set is received whose candidates differ
- **THEN** the screen holds only the second set's candidates
- **AND** none of the first set's candidates is still shown

#### Scenario: A refresh clears the selection

- **WHEN** a candidate is selected and a further set is then received
- **THEN** nothing is selected

#### Scenario: A failed refresh does not leave a stale set on screen as current

- **WHEN** a set is shown and a further request comes back as the failure shape
- **THEN** the screen is in its failed state
- **AND** nothing is selected

### Requirement: A candidate row shows the full address and the mark, and presents nothing as a name

Each candidate row SHALL show that candidate's address **in full**, unelided, and
SHALL show the mark derived from that address. It SHALL NOT present any value as
that candidate's name.

The address is shown in full here and abbreviated elsewhere because this is
where a decision is being made: the user is choosing between keys, and the
addresses are the only unforgeable way to tell the candidates apart. The
abbreviation exists for places where reading happens, and it is defined in one
component precisely so that a second, hand-rolled elision does not appear
alongside it.

**No generated name is available to show.** The module deliberately carries none
— which words a key produces is a separate contract — and the view cannot
compute one, because the name derives from the public key under a scheme that is
not built. So a row SHALL NOT show a derivation path, an index, a position
number or a truncation of the address in a name's place: each would be read as
the thing the user is choosing, and none of them is. The row SHALL leave the
name unshown rather than substituted.

The mark SHALL NOT be presented as a verification, a badge, or anything that
reads as checked. It is derived from the same address any impersonator can
grind against, so it is a second recognition channel and not a second guarantee.

#### Scenario: The address is shown in full and not abbreviated

- **WHEN** a slate is shown
- **THEN** each row's address text is that candidate's address in its entirety
- **AND** it is not an abbreviated form of it

#### Scenario: Every row carries a mark drawn from its own address

- **WHEN** a slate is shown
- **THEN** each row shows a mark
- **AND** each mark is derived from that row's address rather than from a shared
  or fixed value

#### Scenario: No row presents a name

- **WHEN** a slate is shown
- **THEN** no row shows a generated name
- **AND** no row shows a derivation path, an index or a shortened address in a
  position that reads as a name

#### Scenario: Two candidates are distinguishable by what is on screen

- **WHEN** a slate carrying two candidates with different addresses is shown
- **THEN** the two rows show different address text

### Requirement: Selection is explicit, and keeping is refused by the view until something is selected

The screen SHALL show which candidate is selected, SHALL start with none
selected, and SHALL NOT reach the module with a keep request while nothing is
selected.

A pre-selected candidate is a choice made for the user that a single press then
makes permanent. What is being chosen cannot be changed afterwards, so the act
of choosing is worth one deliberate step.

Selection SHALL change nothing outside the screen. Nothing is written until a
candidate is kept, so selecting SHALL reach the module not at all.

#### Scenario: Nothing is selected when a set arrives

- **WHEN** a set of candidates is received
- **THEN** no candidate is selected

#### Scenario: Selecting one candidate deselects any other

- **WHEN** one candidate is selected and then another is
- **THEN** only the second is selected

#### Scenario: Selecting reaches the module not at all

- **WHEN** candidates are selected in turn
- **THEN** no call reaches the bridge

#### Scenario: Keeping is not offered while nothing is selected

- **WHEN** a set is shown and nothing is selected
- **THEN** invoking the keep action sends no keep request to the bridge

#### Scenario: Keeping names the set it was offered with

- **WHEN** a candidate is selected and the keep action is invoked
- **THEN** the request carries the identifier of the set that candidate was
  offered in
- **AND** the position it was selected at

### Requirement: The screen warns that keeping cannot be undone, before it is done

While a set is on screen, and before the keep action is taken, the screen SHALL
state that the choice is permanent and that no later screen can change it.

An identity does not rotate, so a kept key is a key for as long as this user is
this participant, and the name it produces is a pure function of it. A user who
believes they picked a name will look for the screen that renames it, and that
screen cannot exist. The warning belongs at the point of action because that is
the last moment at which it is still information rather than an explanation of
something that already happened.

The screen SHALL NOT use the word "username", and SHALL NOT offer any affordance
that reads as renaming, editing or replacing the identity.

The screen SHALL state that nothing has been published while a set is merely on
offer, so that the user knows refreshing costs them nothing.

#### Scenario: The permanence warning is on screen with the candidates

- **WHEN** a set of candidates is shown
- **THEN** the screen shows the note `There is no settings screen where this can
  be changed later, because the name is only the key written out. Choosing again
  means being someone else here.`

#### Scenario: Refreshing is stated to cost nothing

- **WHEN** a set of candidates is shown
- **THEN** the screen shows the text `Refresh as often as you like. Nothing is
  published until you keep one.`

#### Scenario: The word username never appears

- **WHEN** every piece of text the onboarding screen shows is examined
- **THEN** none of it contains the word "username"

#### Scenario: No affordance offers a rename

- **WHEN** the onboarding screen is shown in any of its states
- **THEN** it offers no control that edits, renames or replaces a name or an
  identity

### Requirement: The screen states that a name is not unique and not an identifier

Wherever the screen explains what the user is choosing, it SHALL state that
generated names are not unique and are not identifiers, and that the address is
what distinguishes two participants.

Uniqueness is not merely unbuilt, it is unavailable: there is no authority to
hold a namespace, so two peers can each believe a name is free, and a lookalike
name is obtainable by pressing refresh. The interface therefore has to be
correct when two identities present the same name, and the correctness is that
the address is always present.

**This is required even though no row shows a name**, and the two requirements
are not in tension. What the user is choosing is a key whose name follows from
it, and the name is what they will be known by once the derivation ships; a
screen that explained the choice without saying the resulting name settles
nothing would have taught the user the opposite of what is true, and would need
correcting later by a change that has no reason to look here. The note explains
the choice; the row shows what the module can currently supply.

The screen SHALL NOT disambiguate two identical names by numbering them.
Numbering requires agreeing which arrived first, and arrival order differs per
peer — two readers would number the same pair oppositely, each certain the other
was looking at the impostor.

#### Scenario: The uniqueness note is shown with the candidates

- **WHEN** a set of candidates is shown
- **THEN** the screen shows a note stating that names are not unique, are not
  identifiers, that someone else in this Stoa may hold the same words, and that
  the address is what tells participants apart

#### Scenario: Nothing is numbered to tell two names apart

- **WHEN** a set of candidates is shown
- **THEN** no row carries an ordinal, a suffix or a count distinguishing it from
  another row

### Requirement: The screen does not claim the identity is unlinkable across Stoas

The onboarding screen SHALL NOT state or imply that the Stoas a user joins
cannot be connected to one another.

One key signs in every Stoa in this release, so an observer watching two Stoas
can tell it is the same participant. The property is suspended rather than
abandoned — the design is a different identity per Stoa and the mechanism for it
exists — but an interface telling a user they have a privacy property they do
not have is the one failure here that could actually harm someone, because it is
acted on.

The screen SHALL state what is true — that the name is computed from the key and
cannot be changed — and SHALL NOT put a replacement privacy claim in the
withheld sentence's place. Saying less is available; saying something false is
not.

#### Scenario: No copy claims cross-Stoa unlinkability

- **WHEN** every piece of text the onboarding screen shows is examined
- **THEN** none of it states that this identity cannot be linked to the user
  elsewhere, or that it is confined to this Stoa alone

#### Scenario: The true half of the claim is still made

- **WHEN** the onboarding screen is shown
- **THEN** it states that the user is picking a key and that the name is computed
  from it, so the name cannot be changed afterwards

### Requirement: A refusal to keep is told apart from a failure and from a success

The view SHALL treat the keep reply's three outcomes as three states: kept,
refused with a reason, and the failure shape.

A refusal arrives as a **successful reply** carrying a negative answer, so a view
that branched only on the bridge's success would read it as a keep that worked
and would report an identity that was never stored. That is the partial-success
confusion the one-failure-shape convention exists to prevent, arriving through
the convention rather than around it.

A refused keep SHALL leave the screen able to try again: the set already on
screen SHALL remain, and the reason SHALL be shown as the module wrote it, since
a module's reasons are written to name a fix and rewording one here would
maintain the same guidance twice.

The view SHALL NOT report an identity on any outcome but a keep that reported
one, and SHALL take the identity it reports from the keep reply rather than from
the candidate row it sent.

#### Scenario: A kept reply is the kept state

- **WHEN** the keep reply says the candidate was kept
- **THEN** the screen is in its kept state
- **AND** shows the address the reply carried

#### Scenario: A refusal is neither the kept state nor the failed state

- **WHEN** the keep reply says the candidate was not kept, with a reason
- **THEN** the screen is in neither its kept state nor its failed state
- **AND** the reason is shown as the module wrote it

#### Scenario: A refusal leaves the candidates on screen

- **WHEN** a keep is refused
- **THEN** the candidates that were on screen are still on screen
- **AND** the keep action can be invoked again

#### Scenario: The failure shape is its own state

- **WHEN** the keep call comes back as the failure shape
- **THEN** the screen is in its failed state
- **AND** it is not in the kept state

#### Scenario: The identity shown is the reply's, not the row's

- **WHEN** a keep succeeds and the reply's address differs from the address of
  the row that was selected
- **THEN** the address shown is the reply's

### Requirement: A generated slate that was never kept is never shown as an identity

Until a keep reports that a candidate was kept, the screen SHALL NOT show any
candidate as the user's identity, and closing or leaving the screen with a set
generated and nothing kept SHALL leave the view holding no identity.

Generating a set writes nothing, so a set that was abandoned is nothing to
recover and nothing to clean up. What the view must not do is carry a candidate
forward as though it were a choice: a candidate that reached the forum screen as
"you" would attribute a user's reading, and eventually their posting attempts,
to a key that exists nowhere.

On returning to a launch decision, the view SHALL ask the module again rather
than treat a candidate it still holds as an identity.

#### Scenario: A generated set is not an identity

- **WHEN** a set of candidates is received and none is kept
- **THEN** the view holds no identity
- **AND** the forum is not shown

#### Scenario: A selected candidate is not an identity

- **WHEN** a candidate is selected and no keep is made
- **THEN** the view holds no identity

#### Scenario: Leaving mid-flow leaves nothing behind

- **WHEN** a set is generated, a candidate is selected, and the launch decision
  is taken again
- **THEN** the decision is taken from the module's answer
- **AND** a module reporting no identity shows onboarding rather than the forum

### Requirement: After a keep, the screen reports the protection and the backup gap honestly

Once a candidate is kept, the screen SHALL show whether the stored master key was
encrypted, and SHALL show whether recovering the identity needs more than that
key. Both SHALL be taken from the module's replies rather than assumed.

Neither fact is discoverable by the view: it has no filesystem access, so a
screen that did not ask would be guessing about a secret on the user's disk.
Whether the key is encrypted depends on whether a passphrase was available, and
on an ordinary install there is none, so the key is stored in the clear — a real
configuration rather than a bug, and not a fact a user should have to read the
source to learn.

Where the reply says the key was **not** encrypted, the screen SHALL say so
plainly and SHALL NOT show a lock, a shield, or the word "secure". Protection
that reads as strong while being absent is worse than visible plaintext, because
plaintext is something a person can act on.

Where the reply says recovery needs more than the master key, the screen SHALL
NOT describe any saved value as a complete backup. Which candidate was kept is
recorded only on this machine, so a user holding the master key alone can derive
their identity and cannot know which identity it was.

**A reply omitting either field SHALL produce no claim about it.** Both fields
are required of their replies by the module's contract, so this state is not
reachable through the module as built — it is reachable through the bridge,
which is where the view's tests supply replies, and it is what stops the view
rendering a missing field's absence as a negative answer. An absent `false` and
a reported `false` mean different things and the screen must not collapse them.

#### Scenario: An unencrypted key is said to be unencrypted

- **WHEN** a keep succeeds reporting that the key was not encrypted
- **THEN** the screen states that it is not encrypted
- **AND** shows no lock, shield or claim of security

#### Scenario: An encrypted key is reported as encrypted

- **WHEN** a keep succeeds reporting that the key was encrypted
- **THEN** the screen states that it was encrypted
- **AND** the two replies produce different text, so the state is readable rather
  than implied

#### Scenario: The backup gap is stated when the module reports it

- **WHEN** the identity report says recovery needs more than the master key
- **THEN** the screen states that the record of the choice lives on this device
- **AND** no text presents a saved key as sufficient to get back in

#### Scenario: Neither fact is assumed when the module did not report it

- **WHEN** a reply omits the protection field, or omits the recovery field
- **THEN** the screen makes no claim about the omitted one

### Requirement: Every piece of module-supplied text is rendered as plain text

Every value the screen renders that came from the module SHALL be rendered as
plain text, and SHALL NOT be rendered through any format that interprets markup.

This is narrower than it looks and is required anyway. The values this screen
renders — addresses, module reasons, protection state — are module-generated
rather than peer-supplied, so none is attacker-controlled today. The obligation
is structural: the default text format in this toolkit **sniffs its input** and
switches to rich text when a string looks like markup, so an element left on the
default is one binding change away from rendering markup, and nothing about that
change would look like it touched rendering. A module reason is already built
from material a caller influenced.

Where the screen renders a value that is or becomes peer-supplied, it SHALL
route it through the component that shows what the sanitiser removed, rather
than rendering the raw string.

#### Scenario: No text element on the screen interprets markup

- **WHEN** every text element the onboarding screen renders is examined
- **THEN** each is set to plain text explicitly
- **AND** none is left on a format that decides by inspecting its input

#### Scenario: A reason containing markup is shown as its characters

- **WHEN** a keep is refused with a reason containing markup characters
- **THEN** the reason is shown as those characters
- **AND** they are not interpreted as formatting

### Requirement: The screen never claims a success it did not read

The screen SHALL take every statement it makes about stored state from a reply
that said it, and SHALL NOT show a state it inferred from an action having been
invoked.

An interface that showed a kept identity because the user pressed keep is an
interface that is right whenever the module is and silent when it is not, which
is the failure mode a user cannot detect. The module is the only party that
knows whether anything was written.

Where a reply is a shape the screen cannot read, it SHALL be a failure that names
itself rather than a state rendered from missing fields.

**What is stated here is what an observer of the screen can check**: that the
state after a call follows the reply rather than the invocation. The moment
between invoking a call and holding its reply is not described, because the
bridge answers within the call and there is no such observable moment to
describe.

#### Scenario: Invoking keep against a refusal does not reach the kept state

- **WHEN** the keep action is invoked and the reply says the candidate was not
  kept
- **THEN** the screen is not in its kept state, the invocation notwithstanding

#### Scenario: A reply missing the fields the kept state needs is a failure

- **WHEN** a keep reply says the candidate was kept but carries no address
- **THEN** the screen is in its failed state rather than showing a kept identity
  with an empty address

#### Scenario: A failure state and a kept state are never both shown

- **WHEN** the screen is in any of its states
- **THEN** at most one of kept, refused and failed is shown
- **AND** which one is determined by a single value rather than by a combination
  of independent flags
