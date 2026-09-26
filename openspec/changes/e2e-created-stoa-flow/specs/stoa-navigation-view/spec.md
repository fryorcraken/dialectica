## REMOVED Requirements

### Requirement: What is shared carries the founding record, not the address alone

**Reason**: Two of its clauses describe a view that no longer exists, and the
tool refuses a MODIFIED block that drops a scenario, so the requirement is
replaced rather than amended. It said a Stoa just created "cannot be shared at
all" because neither the creation reply nor the listing carried a genesis record,
and it had a scenario to match ("A Stoa just created offers no share").
`stoa-membership`'s "A reply naming a Stoa carries the record that Stoa's address
is the hash of" has put the record on both replies since `genesis-in-replies`,
and the view keeps it. It also required the screen to account for a missing
share in words the user can read ("A held Stoa whose record is not held offers no
share, and the screen says why"). That account was the list's marginal note on
sharing, which `drop-apparatus` removed with the annotation column on the
owner's decision, and nothing has rendered it since.

**Migration**: Replaced by "What is shared carries the founding record, and a
Stoa is shareable wherever a reply carried it", added below. Every other
obligation carries across unchanged: both halves, the address in full, no
reconstructed record, no share without a record, and the absence not rendered as
an error. Its scenarios carry across except the two named above. The first is
replaced by one requiring the opposite, and the second by one requiring a row
with no share and no error, without the account.

## ADDED Requirements

### Requirement: What is shared carries the founding record, and a Stoa is shareable wherever a reply carried it

A share affordance MUST produce something carrying **both** the Stoa's address
and its genesis record, and MUST NOT produce the address alone.

This is a property of the address rather than a shortcoming of the share. The
address is a one-way hash of the record: enough to verify a record somebody hands
over, and not enough to reconstruct one. `stoa-membership`'s requirement "Joining
takes an address and the record it names, and verifies rather than trusts" states
that a bare address is not joinable. So a share producing only an address
produces something whose recipient can do nothing with it — and, worse, something
that looks like it should work.

What is shared MUST carry the address **in full**. An abbreviated address is a
recognition aid for a reader looking at a screen; as a thing to be pasted it is
lossy, and the abbreviation exists precisely because a head and a tail can be
ground to match.

**The view MUST NOT reconstruct a genesis record it was not given**, and MUST NOT
offer a share for a Stoa whose record it does not hold. Deriving a record from an
address is exactly what a one-way hash forbids, and a share affordance that
produced a plausible-looking string without one would produce something that
fails to verify at the recipient — a failure that surfaces on somebody else's
machine, as a refusal they cannot explain. Where the record is not available for
a Stoa, no share is offered for it; the affordance's absence is the honest
rendering, and it is not an error state.

**The view holds a record for every Stoa a core reply named with one.**
`stoa-membership`'s requirement "A reply naming a Stoa carries the record that
Stoa's address is the hash of" puts the record on the creation reply, on the join
reply and on every listing item. The view MUST keep the record each of those
replies carries for the Stoa it names, and MUST offer a share for that Stoa
wherever its row is rendered. A Stoa created in this session MUST be shareable
from the record its creation reply carried, whether or not a later listing item
for it carries one. A Stoa the listing reports MUST be shareable from the record
its listing item carried, whether or not the view created or joined it in this
run.

A listing item that carries no record, or an empty one, supplies no record, and
MUST NOT take away a record an earlier reply supplied for the same Stoa. Where no
reply has supplied a record for a listed Stoa, its row MUST still be rendered,
carrying its address. The view MUST NOT offer a share for it, MUST NOT render
anything as an error for it, and MUST leave the list in its read-succeeded state.

#### Scenario: A Stoa joined in this session can be shared

- **WHEN** a Stoa is joined from a pasted reference and its row is rendered
- **THEN** a share is offered for it, the record having arrived with the paste

#### Scenario: A Stoa just created can be shared from its creation reply

- **WHEN** a Stoa is created successfully with a creation reply carrying its
  record, and the listing read afterwards reports that Stoa with an item carrying
  no record
- **THEN** a share is offered for that Stoa's row
- **AND** what the share produces carries the record the creation reply carried
- **AND** the address the creation reply returned is rendered, so what was made
  can be named

#### Scenario: A Stoa the listing reports can be shared from its listing item

- **WHEN** the listing reports a Stoa whose item carries its record, and the view
  has neither created nor joined that Stoa in this run
- **THEN** a share is offered for that Stoa's row
- **AND** what the share produces carries the record the listing item carried

#### Scenario: A listing item with no record gets a row, no share, and no error

- **WHEN** the listing reports one Stoa whose item carries no record field, and
  another whose item carries an empty record, and the view has neither created
  nor joined either in this run
- **THEN** a row is rendered for each, carrying its address
- **AND** no share is offered for either row
- **AND** nothing is rendered as an error for either row
- **AND** the list is in its read-succeeded state

#### Scenario: A share carries both halves

- **WHEN** a user shares a Stoa whose genesis record the view holds
- **THEN** what is produced contains that Stoa's full address
- **AND** it contains the Stoa's genesis record
- **AND** it contains the address unabbreviated

#### Scenario: What is shared is what a join accepts

- **WHEN** what a share produced for a Stoa is supplied back to the paste field
- **THEN** the preview it produces names that same Stoa

#### Scenario: No share is offered for a Stoa whose record the view does not hold

- **WHEN** the list renders a Stoa for which no genesis record was supplied
- **THEN** no share affordance is offered for that row
- **AND** nothing is produced that carries the address without a record

## MODIFIED Requirements

### Requirement: Joining shows what is being joined, and joins nothing until the user acts

Acting on a pasted or in-post address MUST reach a preview of what would be
joined, and MUST NOT join it. Joining MUST require a separate, explicit action by
the user after the preview has been rendered.

An address inside a post is attacker-supplied content — the sender chose it — and
an interface that joined on paste, or on opening a link, would enrol a user in a
Stoa they never chose. A user who does not know they joined a Stoa is the harm
the preview exists to prevent.

The preview MUST render the address **in full**, not abbreviated. This is the
screen where a decision is being made about which Stoa this is, and the
abbreviation is a recognition aid rather than a basis for a decision. The
component that renders addresses already distinguishes the two forms; the full
one is required here.

**Where a founding title is available for the previewed Stoa**, the preview MUST
render it, and MUST label it as the **founding** title rather than as the Stoa's
name or current title. `stoa-membership`'s requirement "A listed title is a
founding title, and is identified as such" is what puts the distinction on the
wire; rendering it unlabelled would discard it at the last step.

**Whether one is available is not the view's choice: it is whether a core call
answers one for a `(address, record)` pair this peer has not joined.** A preview
happens before a join. The title is inside the record the reader was handed, so a
missing one is a gap in the module surface rather than in the data, and closing
it inside the view would mean decoding the genesis record there, which "What is
shared carries the founding record, and a Stoa is shareable wherever a reply
carried it" forbids for the same reason it forbids reconstructing one.

**Before a join, `stoa-metadata`'s `getStoa` call answers a founding title for a
Stoa it reports as falling back, and for no other.** A `getStoa` reply whose
`isGenesisFallback` is `true` carries the founding title as its `title`, so for
that Stoa a founding title is available. A reply whose `isGenesisFallback` is
`false` carries a current title as its `title` and no founding title beside it,
and no other call answers the founding title of a pair this peer has not joined —
joining is what answers one. So for that Stoa no founding title is available at
preview time. Which of the two a `title` is MUST be taken from
`isGenesisFallback`, and a `title` from a reply whose `isGenesisFallback` is
`false` MUST NOT be labelled as the founding title.

**Where both a successful join reply and a fallback reply carry a founding title
for the reference on screen, the join reply's is the one rendered.** A fallback
reply's `title` MUST fill the founding-title position only while no successful
join reply for that reference carries a founding title that is not blank, and
the two MUST NOT both be rendered.

**A founding title that is blank is not an available founding title**,
whichever reply carried it, a join reply included — blank being the empty
string, or a string made only of the blank characters `stoa-genesis`'s
requirement "A blank title is not a valid title" lists. It MUST NOT be rendered
in the founding-title position or labelled as the founding title, and the screen
MUST render that Stoa as one no founding title is available for. A blank title
is not a valid title: `stoa-genesis` refuses a record carrying one, and no core
reply carries one.

**Where no founding title is available, the preview MUST NOT render a title
caption over an empty value, and MUST state that no founding title is available
here and, before a join has succeeded, that joining is what would supply one.**
The absence is the honest rendering and is not an error state. A caption over
blank space asserts that this Stoa's founding title *is* blank, which no valid
Stoa's is, on the screen where the reader is deciding whether to trust an
address. The address, which this screen does hold in full, is what the decision
rests on meanwhile.

#### Scenario: Opening an address previews rather than joins

- **WHEN** a user acts on an address, from the paste field or from an affordance
  inside a post
- **THEN** the preview is rendered
- **AND** no join call has been made

#### Scenario: The join call is made only on the user's explicit action

- **WHEN** the preview has been rendered and the user has taken no further action
- **THEN** no join call has been made
- **AND** the join call is made when, and only when, the user acts on the join
  affordance

#### Scenario: The preview shows the address in full

- **WHEN** the preview is rendered for a Stoa
- **THEN** the whole of that Stoa's address is rendered, rather than an
  abbreviation of it

#### Scenario: The founding title is labelled as founding

- **WHEN** the preview renders a Stoa's founding title
- **THEN** what is rendered identifies that title as the founding value

#### Scenario: A preview with no founding title available captions nothing and says why

- **WHEN** the preview is rendered for a pasted reference and no founding title is
  available for it
- **THEN** no title caption is rendered over an empty value
- **AND** the screen states that no founding title is available here and that
  joining is what would supply one
- **AND** what it states does not claim the Stoa has no founding title, the title
  being unknown here rather than known to be absent
- **AND** nothing is rendered as an error for the missing title
- **AND** the address is still rendered in full

#### Scenario: A blank founding title from a join is not rendered as a founding title

- **WHEN** the lookup answers a non-fallback reply, and the user then joins and
  the join succeeds with a reply whose founding title is the empty string — and
  again with a join reply whose founding title is U+0020 U+200B
- **THEN** in each case the screen reports the Stoa as joined
- **AND** the founding-title position is not rendered
- **AND** no title caption is rendered over an empty value
- **AND** the screen states that no founding title is available here

#### Scenario: A join reply's founding title takes the place of a fallback reply's

- **WHEN** the lookup answers a fallback reply whose `title` is one title that
  is not blank, and the user then joins and the join succeeds with a reply whose
  founding title is a different title that is not blank
- **THEN** the join reply's founding title is rendered in the founding-title
  position
- **AND** the fallback reply's `title` is not rendered

#### Scenario: A fallback reply's title stays when the join reply's is blank

- **WHEN** the lookup answers a fallback reply whose `title` is not blank, and
  the user then joins and the join succeeds with a reply whose founding title is
  the empty string — and again with a join reply whose founding title is U+0020
  U+200B
- **THEN** in each case the fallback reply's `title` is rendered in the
  founding-title position, labelled as founding
- **AND** the statement that no founding title is available here is not
  rendered
