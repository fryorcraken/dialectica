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
shared carries the founding record, not the address alone" forbids for the same
reason it forbids reconstructing one.

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

**Where no founding title is available, the preview MUST NOT render a title
caption over an empty value, and MUST state that no founding title is available
here and that joining is what would supply one.** The absence is the honest
rendering and is not an error state. Leaving the position captioned and blank is
the one option that misinforms: an empty founding title is a **legal** value —
"Creating a Stoa asks for a title and nothing else" requires an empty title be
accepted, and the listing renders such a row — so a caption over blank space
asserts that this Stoa's founding title *is* blank, on the screen where the
reader is deciding whether to trust an address, and the reader has no way to
tell that from "not known here". The address, which this screen does hold in
full, is what the decision rests on meanwhile.

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

### Requirement: No current title is rendered until one has been resolved

The preview MUST NOT render a current title, a present name, or any title
attributed to a moderator, while nothing supplies one.

A current title, in this requirement, is one carried by a moderator-signed
metadata op, and only `stoa-metadata`'s resolution supplies one. The requirement
there is "Current metadata resolves by last-write-wins, falling back to genesis".
Before a join, `getStoa` supplies a current title only in a reply whose
`isGenesisFallback` is `false`. A reply whose `isGenesisFallback` is `true`
supplies none: that resolution fell back to the founding values, so its `title`
is the founding title, and it establishes only that this peer holds no binding
metadata op for the Stoa, not that none exists. A founding title, which is what
`stoa-membership` reports and what a fallback reply carries, is never a current
title. Rendering the founding title under a "current title" caption would
therefore assert that a moderator has not renamed this Stoa. A screen that has
not resolved current metadata has not checked that, one holding a fallback reply
has checked only the ops its own peer holds, and the assertion is false for
every Stoa that has been renamed.

This is a prohibition on claiming, not on the layout. Reserving the position a
current title will occupy is left open, and is the reason the distinction is
worth carrying now; what a screen MUST NOT do is fill that position with the
founding value or with any placeholder that reads as a resolved name.

#### Scenario: The founding title is not also rendered as a current title

- **WHEN** the preview renders a Stoa whose only available title is the founding
  one
- **THEN** the founding title is rendered once, labelled as founding
- **AND** nothing rendered attributes a title to a moderator or presents one as
  the Stoa's present name

#### Scenario: A resolved current title is what fills that position, when one exists

- **WHEN** the preview is given a resolved current title alongside the founding
  one, and the two differ
- **THEN** both are rendered
- **AND** each is labelled as the value it is, so the reader can tell which was
  fixed at founding

### Requirement: A Stoa already held whose title matches is shown as a distinct Stoa, not as a duplicate

Where a founding title is available for the previewed Stoa and it equals that of
a Stoa this peer already holds, at a different address, the screen MUST render
the already-held Stoa alongside the one being previewed, each with its own
address.

**Where no founding title is available, the comparison cannot run, and the screen
MUST say so rather than let its silence be read as the comparison's result.**
The comparison is over founding titles; the requirement "Joining shows what is
being joined, and joins nothing until the user acts" records when the core API
supplies one before a join, which is for a Stoa `getStoa` reports as falling back
and for no other. So the comparison can run at preview time for exactly those
Stoas. This is the half of the deferral that is not cosmetic. An absent warning
reads as "checked, nothing found", so a reader handed an impersonating Stoa would
be misled by a check that never ran — the impersonation arriving *through* the
defence rather than around it. The screen MUST therefore state that the
same-title comparison against the Stoas already held has not been made, and MUST
NOT present the unrun comparison as a clean result. It MUST NOT substitute any
other title for the missing one, from a previous preview or otherwise: comparing
a title the user already trusts against an untrusted address is the impersonation
this requirement exists to expose, performed by the interface.

The comparison is **late, not absent**: once a founding title is available for
the previewed Stoa, it runs and the already-held Stoa is rendered as above. What
this costs while the gap lasts is the warning's timing — it becomes a record of
what happened rather than a warning about what is about to — and that cost is
stated here rather than left to be discovered.

This is the concrete case the whole title-is-not-an-identifier rule exists for,
and it is the one an impersonating Stoa produces on purpose. A reader who holds
*Nym Research* and is handed a second *Nym Research* is exactly the reader who
needs to be shown that these are two addresses; a screen that merely showed the
title again would have confirmed the impersonation.

The two MUST NOT be presented as the same Stoa, as a conflict to resolve, or as a
duplicate of one another. They are two Stoas, and joining the second does nothing
to the first.

**Consulting the held Stoas for this comparison is not the inference the
idempotence requirement forbids**, and the two are worth telling apart because
they read alike. Both comparisons this screen makes are permitted and one of them
is required: comparing *titles* to surface a lookalike, and comparing *addresses*
to tell a lookalike apart from the very Stoa being previewed — the second
scenario below cannot be satisfied any other way. Neither is forbidden by when it
runs; the title comparison running only once a founding title is available is the
API constraint above, not a restriction this paragraph imposes.

What is forbidden is narrower, and it is a comparison made **after** a join, for
one particular purpose: deciding whether a completed join was *new*. That is a
claim about what the core did, which the reply deliberately does not answer and
which a listing fetched earlier cannot supply. The prohibition is on that
inference, under "A join is reported from the core's reply, never assumed", and
not on address comparison as such.

#### Scenario: A same-title Stoa already held is shown beside the preview

- **WHEN** the preview is for a Stoa whose founding title equals that of a Stoa
  the peer already holds, at a different address
- **THEN** the already-held Stoa is rendered alongside it
- **AND** both addresses are rendered
- **AND** neither is presented as a duplicate or a conflict

#### Scenario: A same-title Stoa that is the same address is not shown as a second Stoa

- **WHEN** the preview is for a Stoa the peer already holds, at the same address
- **THEN** no second Stoa is rendered beside it as though it were a different one

#### Scenario: With no title available the comparison does not run and the screen says so

- **WHEN** the preview is rendered for a reference no founding title is available
  for, while the peer holds a Stoa whose founding title equals the previewed
  Stoa's
- **THEN** no already-held Stoa is rendered beside the preview
- **AND** the screen states that the same-title comparison against the Stoas
  already held has not been made
- **AND** no title from any other Stoa is rendered as this reference's title

#### Scenario: The comparison runs once a title is available

- **WHEN** a founding title becomes available for the previewed Stoa and equals
  that of a Stoa the peer already holds, at a different address
- **THEN** the already-held Stoa is rendered alongside it, with both addresses
