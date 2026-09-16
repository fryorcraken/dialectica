## MODIFIED Requirements

### Requirement: The view reaches onboarding through the one core bridge, by name

The view SHALL call the slate, keep and identity-report methods through the same
single bridge every other core call goes through, and each SHALL reach that
bridge under its own distinct method name.

The view has no second route to anything: the engine is sandboxed with a
deny-all network access manager and no filesystem access, so a method the bridge
does not expose is behaviour no screen can reach. One bridge is what gives the
view exactly one error branch, and a distinct method name per call is what makes
a call that went to the wrong method observable rather than silent.

**Where the method string is written is not contracted here.** Collecting the
three behind named wrappers is worth doing — it makes a renamed core method one
edit instead of several, and a typo a load error instead of a screen that
renders its empty state forever — but it is a property of how the source is
arranged rather than of what the view does. An inline call and a wrapper reach
the bridge identically, so no scenario can tell them apart: an earlier version
of this requirement demanded wrappers, and substituting an inline call at a call
site left the whole suite green. A requirement nothing can discharge is worse
than none, so this is left to review and lint.

Each request SHALL be a JSON object carrying the fields that method reads and
SHALL NOT carry a field standing in for the identity being acted on. The
identity follows from the Stoa and the selection; a request naming one would ask
the module to act as somebody it is not.

**The prohibition stays broad, and deliberately outlives the author address.** It
forbids a request naming the identity to act as by *any* value — a public key, a
derivation path, or any other — rather than enumerating the shapes such a value
could take. An enumeration would have to lose its address entry now that no
author address exists, and a list that shrinks as values are deleted is one that
stops covering the next value somebody adds. What makes it checkable is that the
fields each request may carry are themselves specified above: a request carrying
only those fields names no identity, whatever the identity might have been
spelled as.

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
- **THEN** each request carries only the fields that method reads
- **AND** no request carries a further field, so none names a public key, a
  derivation path or any other value as the identity to act as

### Requirement: A candidate row shows the full public key and the mark, and presents nothing as a name

Each candidate row SHALL show that candidate's public key **in full**, unelided,
and SHALL show the mark derived from that public key. It SHALL NOT present any
value as that candidate's name.

The key is shown in full here and abbreviated elsewhere because this is where a
decision is being made: the user is choosing between keys, and the keys are the
only unforgeable way to tell the candidates apart. The abbreviation exists for
places where reading happens, and it is defined in one component precisely so
that a second, hand-rolled elision does not appear alongside it.

**The row shows the key because the key is what a candidate is.** A candidate
previously carried an author address alongside its key, and the row showed the
address; with the author address deleted, the public key is the sole identifier
an identity has, and it is the value the mark and the name are both computed
from. `generated-names`' requirement *The three channels read pairwise disjoint
bytes of the public key* is the authority on which bytes each channel reads.

**No generated name is shown on a row.** Which words a key produces is a separate
contract, and this row's job is to show what a candidate *is* rather than what it
renders as. So a row SHALL NOT show a derivation path, an index, a position
number or a truncation of the key in a name's place: each would be read as the
thing the user is choosing, and none of them is. The row SHALL leave the name
unshown rather than substituted.

The mark SHALL NOT be presented as a verification, a badge, or anything that
reads as checked. It is derived from the same public key any impersonator can
grind against, so it is a second recognition channel and not a second guarantee.

#### Scenario: The address is shown in full and not abbreviated

- **WHEN** a slate is shown
- **THEN** each row's key text is that candidate's public key in its entirety
- **AND** it is not an abbreviated form of it

  The scenario keeps its name because it is the case this delta alters. What is
  shown in full is now the public key: it is the value a candidate is identified
  by, and the reason for showing it unelided is unchanged — this is where a
  choice is being made, so the row must carry the whole of what distinguishes one
  candidate from another.

#### Scenario: Every row carries a mark drawn from its own address

- **WHEN** a slate is shown
- **THEN** each row shows a mark
- **AND** each mark is derived from that row's public key rather than from a
  shared or fixed value

  The mark's input moved from the address to the key when the three display
  channels were allocated across the key's bytes; this scenario records the input
  it actually has rather than the one it had when it was written.

#### Scenario: No row presents a name

- **WHEN** a slate is shown
- **THEN** no row shows a generated name
- **AND** no row shows a derivation path, an index or a shortened public key in a
  position that reads as a name

#### Scenario: Two candidates are distinguishable by what is on screen

- **WHEN** a slate carrying two candidates with different public keys is shown
- **THEN** the two rows show different key text

#### Scenario: No row shows a second identifier beside the key

- **WHEN** a slate is shown and the identifying text each row renders is
  enumerated
- **THEN** the candidate's public key is the only identifier shown
- **AND** no row shows a second 64-character hex value beside it

  Stated as "one identifier, and no second hex value" rather than "no author
  address", because a row cannot be asked what a value it renders *was derived
  from* — an address and a key are both 32 bytes and both render as 64 hex
  characters, so the two are indistinguishable by inspection. What a test can
  check is how many such values a row shows, and that the one it shows is the
  key the slate supplied.

### Requirement: The screen states that a name is not unique and not an identifier

Wherever the screen explains what the user is choosing, it SHALL state that
generated names are not unique and are not identifiers, and that the public key
is what distinguishes two participants.

Uniqueness is not merely unbuilt, it is unavailable: there is no authority to
hold a namespace, so two peers can each believe a name is free, and a lookalike
name is obtainable by pressing refresh. The interface therefore has to be
correct when two identities present the same name, and the correctness is that
the public key is always present.

**This is required even though no row shows a name**, and the two requirements
are not in tension. What the user is choosing is a key whose name follows from
it, and the name is what they will be known by; a screen that explained the
choice without saying the resulting name settles nothing would have taught the
user the opposite of what is true, and would need correcting later by a change
that has no reason to look here. The note explains the choice; the row shows the
key the choice is over.

The screen SHALL NOT disambiguate two identical names by numbering them.
Numbering requires agreeing which arrived first, and arrival order differs per
peer — two readers would number the same pair oppositely, each certain the other
was looking at the impostor.

#### Scenario: The uniqueness note is shown with the candidates

- **WHEN** a set of candidates is shown
- **THEN** the screen shows a note stating that names are not unique, are not
  identifiers, that someone else in this Stoa may hold the same one, and that
  the public key is what tells participants apart
- **AND** the note asserts no number of words, so that it fails on a count being
  reintroduced rather than on the sentence being reworded

#### Scenario: Nothing is numbered to tell two names apart

- **WHEN** a set of candidates is shown
- **THEN** no row carries an ordinal, a suffix or a count distinguishing it from
  another row

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
- **AND** shows the public key the reply carried

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

- **WHEN** a keep succeeds and the reply's public key differs from the public key
  of the row that was selected
- **THEN** the public key shown is the reply's

### Requirement: Every piece of module-supplied text is rendered as plain text

Every value the screen renders that came from the module SHALL be rendered as
plain text, and SHALL NOT be rendered through any format that interprets markup.

This is narrower than it looks and is required anyway. The values this screen
renders — public keys, module reasons, protection state — are module-generated
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

- **WHEN** a keep reply says the candidate was kept but carries no public key
- **THEN** the screen is in its failed state rather than showing a kept identity
  with an empty key

#### Scenario: A failure state and a kept state are never both shown

- **WHEN** the screen is in any of its states
- **THEN** at most one of kept, refused and failed is shown
- **AND** which one is determined by a single value rather than by a combination
  of independent flags

## RENAMED Requirements

- FROM: `### Requirement: A candidate row shows the full address and the mark, and presents nothing as a name`
- TO: `### Requirement: A candidate row shows the full public key and the mark, and presents nothing as a name`

The old name states the address in its first clause, so leaving it would name the
requirement after the value this change deletes.
