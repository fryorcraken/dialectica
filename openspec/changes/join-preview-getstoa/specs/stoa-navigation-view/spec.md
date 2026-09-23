## ADDED Requirements

### Requirement: The preview asks the core what the Stoa is called, and renders the answer as what the reply says it is

When the preview is rendered for a reference, the view MUST call
`stoa-metadata`'s `getStoa` with that reference's address and genesis record,
without the user acting. The address and record sent MUST be the ones a join
from the same preview would send. This lookup MUST NOT be made while no
reference is being previewed, and MUST NOT be made for input that is not a
well-formed Stoa reference.

In this requirement a **lookup** is that call, and a **fallback reply** is a
successful `getStoa` reply whose `isGenesisFallback` is `true`.

**A fallback reply.** Its `title` MUST be rendered in the founding-title
position, labelled as the founding title, as "Joining shows what is being
joined, and joins nothing until the user acts" requires of an available founding
title. The screen MUST NOT render a current title or a description from it. It
MUST state that this machine holds no title set by a moderator for this Stoa,
and MUST NOT state or imply that the Stoa has not been renamed.

**A founding title answered as the empty string is available, not missing.** An
empty `title` in a fallback reply, and an empty founding title in a successful
join reply, MUST each be rendered as the founding title — the founding-title
position shown, labelled as founding, holding the empty value — and the
statement that no founding title is available here MUST NOT be rendered for
that reference. It MUST be treated as the founding title for the same-title
comparison of "A Stoa already held whose title matches is shown as a distinct
Stoa, not as a duplicate", exactly as a non-empty one is.

**A non-fallback reply.** Its `title` MUST be rendered in the current-title
position, labelled as the current title chosen by a moderator, and MUST NOT be
rendered in the founding-title position. Its `description`, where non-empty,
MUST be rendered with that current title and attributed to the same
moderator-set metadata; where empty, the screen MUST NOT render a description
caption. The screen MUST NOT state that what this Stoa is called is unknown
here.

**The same-title comparison MUST NOT be run against a current title.** A
`title` from a non-fallback reply is not a founding title, so it MUST NOT cause
an already-held Stoa to be rendered beside the preview. Until a founding title
is available for that reference, the screen states that the comparison has not
been made, as the lookalike requirement already requires where no founding
title is available.

A title and a description rendered from a lookup MUST be rendered as plain
text and MUST NOT pass through any rich-text or markup-interpreting path.

#### Scenario: Previewing a reference looks it up without joining it

- **WHEN** the preview is rendered for a well-formed reference and the user
  takes no action
- **THEN** a `getStoa` call has been made carrying that reference's address and
  genesis record
- **AND** no join call has been made

#### Scenario: The lookup carries what a join would carry

- **WHEN** a reference whose address carries the display prefix is previewed,
  and the user then acts on the join affordance
- **THEN** the address and record sent to `getStoa` are the same as the address
  and record sent to the join
- **AND** neither carries the display prefix

#### Scenario: No lookup is made with no reference, or for malformed input

- **WHEN** the preview has no reference to show, and again when the paste field
  is given text that is not a Stoa reference
- **THEN** no `getStoa` call has been made

#### Scenario: A fallback title is rendered as the founding title

- **WHEN** the lookup answers a fallback reply with a non-empty `title`
- **THEN** that title is rendered in the founding-title position, labelled as
  founding
- **AND** no current title is rendered
- **AND** the statement that no founding title is available here is not
  rendered

#### Scenario: A fallback is stated as no moderator-set title held here

- **WHEN** the lookup answers a fallback reply
- **THEN** the screen states that this machine holds no title set by a
  moderator for this Stoa
- **AND** nothing rendered states that the Stoa has not been renamed

#### Scenario: An empty founding title from a fallback is rendered as known

- **WHEN** the lookup answers a fallback reply whose `title` is the empty string
- **THEN** the founding-title position is rendered, labelled as founding, and
  holds the empty value
- **AND** the statement that no founding title is available here is not
  rendered
- **AND** no substitute title is rendered in place of the empty one

#### Scenario: An empty founding title from a join is rendered as known

- **WHEN** the lookup answers a non-fallback reply, and the user then joins and
  the join reply carries an empty founding title
- **THEN** the founding-title position is rendered, labelled as founding, and
  holds the empty value
- **AND** the statement that no founding title is available here is not
  rendered

#### Scenario: An empty founding title is compared like any other

- **WHEN** the lookup answers a fallback reply whose `title` is the empty
  string, and the peer holds a Stoa at a different address whose founding title
  is also the empty string
- **THEN** the already-held Stoa is rendered alongside the preview, with both
  addresses

#### Scenario: A non-fallback title is rendered as the current title

- **WHEN** the lookup answers a reply whose `isGenesisFallback` is `false`
- **THEN** its `title` is rendered in the current-title position, labelled as
  the current title chosen by a moderator
- **AND** the founding-title position is not filled from that reply
- **AND** the screen states that no founding title is available here, as
  "Joining shows what is being joined" requires
- **AND** nothing rendered states that what the Stoa is called is unknown

#### Scenario: A non-empty description is rendered with the current title

- **WHEN** the lookup answers a non-fallback reply whose `description` is
  non-empty
- **THEN** the description is rendered with the current title

#### Scenario: An empty description is not captioned

- **WHEN** the lookup answers a non-fallback reply whose `description` is the
  empty string
- **THEN** no description caption is rendered

#### Scenario: No description is rendered from a fallback reply

- **WHEN** the lookup answers a fallback reply whose `description` is non-empty
- **THEN** no description is rendered

#### Scenario: A current title is not compared against the Stoas already held

- **WHEN** the lookup answers a non-fallback reply whose `title` equals the
  founding title of a Stoa the peer already holds, at a different address, and
  no join has been made
- **THEN** no already-held Stoa is rendered beside the preview
- **AND** the screen states that the same-title comparison against the Stoas
  already held has not been made

#### Scenario: A looked-up title is not interpreted as markup

- **WHEN** the lookup answers a fallback reply, and again a non-fallback reply
  with a non-empty description, each carrying markup characters in its `title`
  and `description`
- **THEN** each is rendered as the literal characters it contains rather than
  as markup

### Requirement: A lookup that fails, or answers in no recognisable shape, renders no title and withdraws no join

Where the lookup reaches the screen as a failure — the error shape, or any
failure the view's one call path reports, such as an unreachable core — the
preview MUST render the reason given, unreworded, and MUST render no title and no description
from that lookup. It MUST NOT be rendered as a fallback: the founding-title
position MUST NOT be filled from it, and the statement that this machine holds
no moderator-set title MUST NOT be rendered.

A successful lookup reply whose `isGenesisFallback` is not a boolean, or whose
`title` or `description` is not a string, MUST be treated as a failed lookup,
with a reason of the view's own naming what was wrong with the reply in place
of the core's. It MUST NOT be rendered as a fallback reply or as a non-fallback
one.

A failed lookup, from any of these causes, MUST NOT be reported as a join having been attempted or refused, and MUST NOT withdraw
the join affordance. The join is a separate call and its outcome is reported
from its own reply.

#### Scenario: A refused lookup renders the core's reason and no title

- **WHEN** the lookup is answered with the error shape and no join has been made
- **THEN** the core's message is present in what is rendered
- **AND** no title and no description from that lookup is rendered
- **AND** the founding-title position is not rendered
- **AND** the statement that this machine holds no moderator-set title is not
  rendered

#### Scenario: A refused lookup is not a refused join

- **WHEN** the lookup is answered with the error shape and the user has taken
  no further action
- **THEN** the screen does not report the Stoa as not joined, or a join as
  refused
- **AND** the join affordance is offered

#### Scenario: The join after a refused lookup is reported from the join's reply

- **WHEN** the lookup is answered with the error shape, and the user then acts
  on the join affordance and the join succeeds
- **THEN** the screen reports the Stoa as joined

#### Scenario: A lookup reply of the wrong shape is a failure

- **WHEN** the lookup answers a successful reply with no `isGenesisFallback`,
  again with an `isGenesisFallback` that is a string, again with a `title` that
  is not a string, and again with a `description` that is not a string
- **THEN** in each case the screen renders a failure of its own naming what was
  wrong with the reply
- **AND** no title is rendered in the founding-title or the current-title
  position
- **AND** the join affordance is offered

### Requirement: A lookup's answer is rendered only for the reference it was made for

A lookup's answer — a title, a description, a fallback, or a failure — MUST be
rendered only while the reference it was made for is the one being previewed.
When the previewed reference changes, a lookup MUST be made for the new
reference, and nothing answered for the previous one MUST be rendered for it.

#### Scenario: A second preview renders nothing from the first one's lookup

- **WHEN** a reference is previewed and its lookup answers a fallback reply
  with a non-empty title, and a second reference at a different address is
  then previewed and its lookup is answered with the error shape
- **THEN** a `getStoa` call has been made carrying the second reference's
  address and record
- **AND** the first reference's title is not rendered
- **AND** the statement that this machine holds no moderator-set title is not
  rendered

#### Scenario: A second preview's own answer replaces the first

- **WHEN** a reference is previewed and its lookup answers a non-fallback reply,
  and a second reference is then previewed and its lookup answers a fallback
  reply
- **THEN** the second reference's title is rendered in the founding-title
  position
- **AND** the first reference's current title is not rendered
