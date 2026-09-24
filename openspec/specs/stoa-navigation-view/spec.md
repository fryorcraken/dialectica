# stoa-navigation-view Specification

## Purpose
Defines what the view renders, and what it MUST refuse to claim, on the screens where a user reaches a Stoa: the list of Stoas this peer is in, the preview shown before joining one, the creation of a new one, and the sharing of one with somebody else. It exists because joining is the point where a user acts on a string from an untrusted channel, and every property that makes that safe is one the rendering has to carry rather than one the core can discharge on the view's behalf.

The boundary with the core capabilities is the load-bearing part of this contract, and it is stated here rather than restated as a requirement. `stoa-membership` owns what creating, joining and listing **do**; `module-wire-contract` owns the envelope and the single error shape; `stoa-metadata` owns the distinction between a founding and a current title; `posting-capability` owns the vocabulary for why a key is unusable. Nothing below re-specifies any of them. What this capability owns is the obligation each of those creates and cannot meet from inside the core: a core that returns a founding title honestly labelled has still not stopped a screen from captioning it as the current one, and a core that refuses a record not matching its address has still not stopped a screen from reporting a join it never made.

Two things are deliberately outside this capability. **The feed itself** — the ordering row, the posting gate, the empty-versus-unreadable pair over a Stoa's posts — is the existing feed screen's, and this capability constrains only what is handed to it. **The visual system** — colours, type, metrics, the mark, the 8-8-6 address abbreviation — is fixed by the components already in the view; requirements below say which component owns a rendering decision and never restate what it renders.

## Requirements

### Requirement: A listed Stoa is rendered with its address, never with its title alone

Every row of the Stoa list MUST render the Stoa's address alongside whatever else
it renders, and MUST NOT be able to render a title without one.

A founding title is chosen freely by whoever created the Stoa, is not unique, is
verified against nothing, and can be picked to resemble another Stoa's. The
address is the only distinguishing half, and `stoa-membership`'s requirement
"Each item carries the address, not only the title" puts it on every item
precisely so that a row always has it available. A row showing two Stoas with the
same title and no addresses has rendered exactly the part an attacker controls.

The address MUST be rendered through the view's existing address component, which
owns the head-8 middle-8 tail-6 abbreviation. A second abbreviation MUST NOT be
written: an elision that keeps only a head and a tail is the shape vanity-address
generators are built to defeat, and a second implementation is how one screen
quietly acquires the weaker form.

A Stoa whose founding title is **blank** — the empty string, or a string made
only of the blank characters `stoa-genesis`'s requirement "A blank title is not
a valid title" lists — MUST render as a row carrying its address, MUST NOT be
omitted, and MUST NOT be given a substitute title. A blank title is not a valid
founding title — `stoa-genesis` refuses a record carrying one, and
`stoa-membership` refuses to create or join one and reports a listing that
reaches a retained one as a failure — so the core does not list one; a listing
that carries one anyway still names a Stoa the core reports this peer as being
in.

A founding title MUST be rendered as plain text and MUST NOT be rendered through
any rich-text or markup-interpreting path. It carries whatever characters its
creator typed, unnormalised: the core preserves it exactly because normalising
would change the address and split one Stoa into two.

#### Scenario: A row carries the address as well as the title

- **WHEN** the list renders a Stoa the peer is in
- **THEN** the rendered row contains that Stoa's address
- **AND** the row contains the founding title from the same item

#### Scenario: Two Stoas presenting the same title are distinguishable in the list

- **WHEN** the listing returns two Stoas whose founding titles are identical and
  whose addresses differ
- **THEN** both rows are rendered
- **AND** the two rows differ in what they render, because each carries its own
  address

#### Scenario: A Stoa with an empty founding title still gets a row

- **WHEN** the listing returns a Stoa whose founding title is the empty string
- **THEN** a row is rendered for it
- **AND** the row carries that Stoa's address
- **AND** no substitute title is rendered in place of the empty one

#### Scenario: A Stoa whose founding title is made only of blank characters still gets a row

- **WHEN** the listing returns a Stoa whose founding title is U+0020 U+200B
- **THEN** a row is rendered for it
- **AND** the row carries that Stoa's address
- **AND** no substitute title is rendered in place of the blank one

#### Scenario: A title is not interpreted as markup

- **WHEN** the listing returns a Stoa whose founding title contains markup
  characters
- **THEN** the title is rendered as the literal characters it contains rather
  than as markup

### Requirement: Holding no Stoas and failing to read membership are different screens

The list MUST distinguish three outcomes of a listing call, and MUST NOT render
any two of them alike: the peer is in Stoas and they are shown; the peer is in
none; the membership could not be read.

These mean opposite things. A peer in no Stoas is a new install and the answer is
to create or join one. A peer whose membership state cannot be read is a peer
that may hold many Stoas and cannot see them, and telling that user they belong
to nothing invites them to re-join Stoas they are already in — which is why
`stoa-membership`'s requirement "Membership state that cannot be opened is a
failure, not an empty listing" makes the core report the error shape rather than
an empty page. That requirement puts the two answers on the wire as different
replies; this one is what stops the screen collapsing them again.

The failure state MUST render the reason the core gave, unreworded. Core failure
messages are written to name a fix, and a screen that replaced one with a
friendlier sentence would be maintaining the same guidance twice and losing the
actionable half.

A reply that is neither a listing nor the error shape — a success carrying no
array of items, or an `items` field that is not an array — MUST be treated as a
failure and MUST NOT be rendered as an empty list. The core keeps the two reply
shapes disjoint, and this screen does not rest on a guarantee made in another
module, for the same reason the feed screen does not: an absent array assigned
into a row model renders as "you are in no Stoas", which is the one confusion the
whole requirement exists to prevent.

#### Scenario: An empty listing is the success state, not a failure

- **WHEN** the listing call succeeds and carries no items
- **THEN** the screen is in its read-succeeded state
- **AND** it reports no failure

#### Scenario: A membership that cannot be read is a failure, not an empty list

- **WHEN** the listing call answers with the error shape
- **THEN** the screen is in its failed state
- **AND** it renders no Stoa rows
- **AND** the core's message is present in what is rendered

#### Scenario: The empty state and the failed state are not the same state

- **WHEN** one screen is driven with a successful listing carrying no items, and
  another with the error shape
- **THEN** the two screens are in different states

#### Scenario: A success with no items array is a failure rather than an empty list

- **WHEN** the listing call answers with an object carrying no `items` array, and
  again with an `items` field that is not an array
- **THEN** the screen is in its failed state in both cases
- **AND** it reports a failure of its own naming what was wrong with the reply

### Requirement: Every number rendered is one this peer can actually answer

The list and the join preview MUST NOT render a count of members, of total posts,
of peers, or of anything else no peer can observe; and MUST NOT render a count of
what this machine holds unless a call answers it.

**Two separate reasons, and conflating them is how one of them gets argued
away.** A global count — members, total posts, how many peers are in a Stoa — is
forbidden permanently, because there is no membership list and no peer sees the
whole of a Stoa, so any such number would be invented rather than merely
unavailable. A count of what *this machine* holds is legitimate to render and
would be welcome; it is simply not computed today.

The second reason is what bites on this screen. **No per-row count of held posts
is available.** A listed item carries an address and a founding title, no call
answers how many posts this peer holds for a given Stoa, and the paginated thread
listing reports whether a further page exists rather than a total. So a row MUST
render without a held-post count, and MUST NOT substitute one. In particular the
length of a page from any other call MUST NOT be rendered in that position: a page
length looks like a total, is not one, and would be wrong by an amount that grows
with the Stoa.

A phrase asserting that nothing has been received for a Stoa MUST NOT be rendered
in the list either, for the same reason and not for the global one: it is a claim
about a count nothing computed, so this build cannot distinguish a Stoa this peer
holds nothing for from one it has not counted.

#### Scenario: A row renders no count of held posts

- **WHEN** the list renders a Stoa
- **THEN** the row contains no count of posts held for that Stoa
- **AND** it contains no count of members, peers, or anything else global

#### Scenario: No other call's page length is rendered as a row's count

- **WHEN** the list is rendered while a thread listing for some Stoa has also
  been answered
- **THEN** no number from that listing appears in any row

### Requirement: What is shared carries the founding record, not the address alone

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

**The case a user meets first is the one worth naming rather than leaving as an
edge case: a Stoa this peer has just created cannot be shared at all.** Neither the
listing reply nor the creation reply carries a genesis record — creation answers
with the address, the founding title and the policy — so the only records the view
ever holds are the ones a user pasted when joining, and they are lost on restart.

The consequence is sharper than the restart case and points the other way from
what a reader would assume: **the primary creation flow ends with an address on
screen and nothing to hand anybody**, immediately, for a Stoa whose entire purpose
is to be shared and which no registry can be looked up in later. A joined Stoa can
be shared until the view restarts; a created one cannot be shared even once.

This is a degradation of the interface caused by a core reply, not a decision this
capability is making, and it is recorded so the core change that widens the
listing item has a requirement pointing at it. **What the view MUST do meanwhile is
account for the absence somewhere the user can read it**, so that a share missing
from a row reads as a reference this copy does not have rather than as a feature
that broke. The screen MUST NOT present the missing share as an error, and MUST
NOT offer a share that produces nothing.

#### Scenario: A Stoa joined in this session can be shared

- **WHEN** a Stoa is joined from a pasted reference and its row is rendered
- **THEN** a share is offered for it, the record having arrived with the paste

#### Scenario: A Stoa just created offers no share

- **WHEN** a Stoa is created successfully and its row is rendered
- **THEN** no share is offered for it, the creation reply carrying no record
- **AND** its address is still rendered, so what was made can be named

#### Scenario: A held Stoa whose record is not held offers no share, and the screen says why

- **WHEN** the list renders a Stoa from the listing for which no record is held
- **THEN** no share is offered for that row
- **AND** the screen carries an explanation that a row without a record offers no
  share
- **AND** nothing is rendered as an error for that row

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

### Requirement: The reference encoding is a compatibility surface and is fixed here

A shared reference MUST be a JSON object carrying the address under `stoa` and the
genesis record under `genesis`, each a string of the bare value with no display
prefix. A paste MUST accept exactly that, and the two MUST NOT be specified
separately.

**This is in the spec because it outlives the build that wrote it.** Every other
format decision on these screens is internal and revisable; this one is not. A
user who copies a reference holds a string that may be pasted days later, possibly
into a different build, so changing the encoding strands strings already in the
wild. Within one build a single implementation keeps the two ends in step —
between two builds nothing does, and a requirement is what a future change reads
before touching it.

**The encoding MUST be self-describing, and that is a behavioural choice rather
than a matter of taste.** A user who pastes half a reference must get a *malformed*
failure, not a verification failure. A positional format would accept a truncated
second field as a short record, forward it to the core, and come back as a record
that does not hash to the address — collapsing two of the three outcomes the
preceding requirement spends its whole text keeping apart, and manufacturing the
same accusation a surviving display prefix does.

**Whether the two halves must look like addresses before they are forwarded is
deliberately left open.** The view checks that a reference carries two halves of
the right type and nothing more; it does not check their character class, so a
value that is not hex, or that carries a homoglyph, is forwarded and refused by the
core. That is sound today — verification is the core's single check and a second
implementation of it is what this design refuses — and it is recorded as undecided
rather than as settled, because tightening it without a requirement risks refusing
an address encoding a later version uses. **What MUST NOT happen is the view
reporting such input as verified, or as malformed, on its own authority**: the
core's answer is what the screen renders.

#### Scenario: A share round-trips through a paste

- **WHEN** a reference produced by a share is pasted back
- **THEN** it parses as a reference
- **AND** the address and record recovered are the ones shared

#### Scenario: A truncated reference fails as malformed, not as unverified

- **WHEN** part of a reference is pasted
- **THEN** the screen reports it as not a Stoa reference
- **AND** no join call has been made, so nothing can be reported as a verification
  failure

#### Scenario: A half of the wrong type is refused rather than forwarded

- **WHEN** a reference is pasted whose `stoa` or `genesis` is a number, an object,
  an array, or absent
- **THEN** the screen reports it as not a Stoa reference
- **AND** no join call has been made

#### Scenario: A well-typed half the core rejects is the core's answer to give

- **WHEN** a reference is pasted whose halves are strings that are not valid
  addresses, and the user acts on it
- **THEN** what the screen reports is the core's reply
- **AND** the view does not report it as verified on its own authority

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

### Requirement: No current title is rendered until one has been resolved

The preview MUST NOT render a current title, a present name, or any title
attributed to a moderator, while nothing supplies one.

A current title, in this requirement, is one carried by a moderator-signed
metadata op, and only `stoa-metadata`'s resolution supplies one. The requirement
there is "Current metadata resolves by last-write-wins among binding ops,
falling back to genesis".
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

### Requirement: What the address proves is stated exactly, and nothing broader

Where the preview explains what the address gives the reader, it MUST describe a
match between the address and the record it was handed, and MUST NOT describe
that match as establishing anything about the Stoa itself.

The check is a hash comparison between two supplied inputs and consults nothing
else — no registry, no peer, no network — which `stoa-genesis`'s "An address
verifies the record it names" states and `stoa-membership` relies on. So it
establishes exactly one fact: **the record shown is the record this address
names.** It does not establish that this address is the one the user was meant to
receive, that its creator is who the user believes, that the Stoa is not an
imitation of another, or that anyone else can reach it.

That gap is the whole of the residual risk on this screen, and it MUST NOT be
narrowed by the copy. A reader who has pasted an address from a hostile channel
and been shown a verified record has verified the attacker's record against the
attacker's address, perfectly successfully. Wording that presents pasting as
*the* verification, without saying what remains unverified, tells that reader
they have finished checking when they have not started.

The preview MUST therefore state that what is shown other than the address is
unverified. The founding title above all: it is decoration, freely chosen, and
matched against nothing.

#### Scenario: The explanation does not claim more than a hash match

- **WHEN** the preview renders its explanation of the address
- **THEN** what it states is that the record shown is the one the address names
- **AND** nothing rendered claims the address itself has been checked against any
  registry, peer, or third party

#### Scenario: The unverified remainder is named

- **WHEN** the preview is rendered
- **THEN** it states that what is shown besides the address is not verified

### Requirement: A malformed address and an unjoinable one are different failures

The preview MUST distinguish input it could not make sense of from input it
understood and could not act on, and MUST NOT render the two alike.

A user pasting from an untrusted channel gets three outcomes and they call for
three different actions. Input that is not a well-formed Stoa reference at all —
truncated, wrong alphabet, missing its record half — means the paste went wrong
and the answer is to paste again. Input that is well-formed but whose record does
not verify against its address means somebody handed over a record that is not
the one that address names, and the answer is emphatically **not** to try again:
`stoa-membership` requires the core to refuse such a join, and a screen offering a
retry there is a screen inviting the user to keep pressing a button until they
mistake the refusal for a transient fault. Input that is well-formed and verifies
is the ordinary case.

The view MUST NOT decide for itself that a well-formed pair verifies. Verification
is `stoa-membership`'s "Verification consults nothing but the two inputs", it
happens in the core, and the view's own check is limited to whether the input
carries the two halves at all. A view that re-derived an address would be a second
implementation of the one check this design rests on.

The failure the core reported MUST be rendered. A refusal that names what was
wrong is the difference between a user who knows the record they were sent is
wrong and a user who thinks the app is broken.

**A display prefix is a reading aid and MUST NOT reach the core.** The interface
renders an address with a human-facing prefix (`stoa:`), and a user pasting from a
screen or a chat message brings it along — sometimes more than once, since a
double-click that selects a rendered address and a paste onto a field already
holding one both produce `stoa:stoa:<hex>`. Every such prefix MUST be removed
before a value is sent to the core or rendered as the address itself, however many
are present, and one MUST NOT be added on the way out.

**The reason this is a requirement and not a formatting detail is that leaving one
in place converts a paste artefact into an accusation.** A prefixed value is not
the hash of anything, so the core answers that the record does not hash to the
address — a *verification* failure, which this requirement's whole purpose is to
keep separate from a malformed paste. The user is then told, by the software, that
whoever sent them the reference sent a bad record, when the fault was in their own
clipboard and nothing on the screen offers a way to discover that. It also makes
the address rendered "in full" not the address.

Stripping MUST NOT be a fixed number of passes. A single pass is the defect that
shipped here; a fix that strips exactly twice is the same defect one paste further
out.

#### Scenario: A display prefix never reaches the core

- **WHEN** a reference whose address carries the display prefix is pasted and the
  user acts on it
- **THEN** what is sent to the core carries no display prefix
- **AND** the address rendered as the full address carries none either

#### Scenario: A repeated prefix is stripped rather than forwarded

- **WHEN** a reference whose address carries the display prefix repeated, with or
  without whitespace between the repetitions, is pasted
- **THEN** every prefix is removed before anything is sent
- **AND** the outcome is not a verification failure, which is what a surviving
  prefix would produce

#### Scenario: A prefix is not added to what is shared

- **WHEN** a share is produced for a Stoa
- **THEN** neither half of what is produced carries the display prefix

#### Scenario: Input that is not a Stoa reference is refused before any call

- **WHEN** the paste field is given text that does not carry both an address and
  a genesis record
- **THEN** the screen reports that what was pasted is not a Stoa reference
- **AND** no join call has been made

#### Scenario: A record that does not match its address is a distinct failure

- **WHEN** the user joins a well-formed pair and the core refuses it because the
  record does not verify against the address
- **THEN** the screen renders the core's refusal
- **AND** what it renders differs from what it renders for text that is not a
  Stoa reference

#### Scenario: The view does not verify the record itself

- **WHEN** a well-formed pair is supplied whose record does not name the address
- **THEN** the screen's own report of the outcome comes from the core's reply
  rather than from a check the view performed
- **AND** the screen does not report the pair as joinable before the core has
  answered

### Requirement: A join is reported from the core's reply, never assumed

The screen MUST report a join as having happened only on a successful reply from
the core, and MUST NOT render success on the strength of having made the call.

Failure is always the single error shape and never a partial success, which is
what lets one branch decide this. A screen that navigated onward as soon as it
dispatched the call would show the user a Stoa they are not recorded as being in,
and the discrepancy would survive the restart — membership is what the core
retained, not what the screen displayed.

**Joining a Stoa the peer is already in MUST be rendered as success.** The core
reports the same success either way, deliberately: a pasted address is exactly
the input a user supplies twice, and `stoa-membership`'s requirement "Joining a
Stoa the peer is already in changes nothing and is not a failure" makes the
second attempt succeed. The reply does not say whether the join was new, and the
screen MUST NOT infer newness from anything else — from the Stoa's presence in a
listing it fetched earlier, above all — nor present the repeat as an error, a
warning, or a collision to resolve.

#### Scenario: Success is rendered only after a successful reply

- **WHEN** the user acts on the join affordance and the core answers with a
  failure
- **THEN** the screen does not report the Stoa as joined
- **AND** it renders the core's failure

#### Scenario: Joining a Stoa already held is success

- **WHEN** the user joins a Stoa that is already among the ones the peer is in,
  and the core answers successfully
- **THEN** the screen reports success
- **AND** it renders no error, warning, or collision

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

**A blank founding title matches nothing** — blank being the empty string, or a
string made only of the blank characters `stoa-genesis`'s requirement "A blank
title is not a valid title" lists. A previewed Stoa whose founding title is
blank is one no founding title is available for, as "Joining shows what is being
joined, and joins nothing until the user acts" states, so the comparison does
not run for it. A held Stoa whose founding title is blank MUST NOT be rendered
beside any preview as a same-title Stoa. Two blank titles are not equal titles
for this comparison, even when they are the same characters.

Titles that are not blank are compared as they are carried, with no character
removed, replaced or added on either side: the comparison MUST NOT trim or
normalise them.

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

#### Scenario: A blank title is not a same-title match

- **WHEN** the peer holds a Stoa at a different address whose founding title is
  the empty string, and the previewed reference's lookup answers a fallback
  reply whose `title` is the empty string — and again when that lookup answers a
  non-fallback reply and the user then joins and the join reply carries an empty
  founding title — and again with the held Stoa's founding title, the lookup's
  fallback `title` and the join reply's founding title each U+0020 U+200B in
  place of the empty string
- **THEN** in each case no already-held Stoa is rendered beside the preview
- **AND** the screen states that the same-title comparison against the Stoas
  already held has not been made

#### Scenario: A title with one visible letter among blank characters is compared as it is

- **WHEN** the peer holds a Stoa at a different address whose founding title is
  U+0020 U+200B, the letter `a`, U+3000, and the previewed reference's lookup
  answers a fallback reply whose `title` is those same four characters
- **THEN** the already-held Stoa is rendered alongside the preview, with both
  addresses
- **AND** when the held Stoa's founding title is instead the letter `a` alone,
  no already-held Stoa is rendered beside the preview

### Requirement: Creating a Stoa asks for a title and nothing else, and is always offered

The create affordance MUST take a title and MUST NOT take, or offer to take, a
creator key or any identity selection.

The creator key is what makes the creator the Stoa's sole moderator and it is
fixed inside the address preimage forever — `stoa-membership`'s "Creating a Stoa
produces a genesis record the creator can moderate" states that the key is not a
parameter and cannot be. A field for one would be a field that mints a Stoa
nobody can moderate, at an address that cannot be un-minted.

The affordance MUST be offered whatever the keystore's state, and MUST NOT be
hidden or disabled on a guess about whether a key exists. The posting probe takes
a Stoa address and there is no Stoa yet at creation, so there is nothing to ask it
about; a screen that hid the button would be hiding it on something other than an
answer the core gave. It MUST NOT be gated on a build flag.

Where creation fails for want of a usable key, the screen MUST render the reason
the core gave. That reason is `posting-capability`'s vocabulary — it names a fix —
and rendering it unreworded is what makes a creation failure read the same as a
posting failure, which is deliberate.

A **blank title MUST be passed through to the core** by the field, exactly as
typed, rather than refused or altered by the view, and the core's reply MUST
decide the outcome. Blank is what `stoa-genesis`'s requirement "A blank title is
not a valid title" defines: the empty string, or a string made only of the blank
characters it lists. `stoa-membership` refuses a blank title, so the screen MUST
NOT report a Stoa as created for one, and MUST render the reason the core gave,
unreworded.

#### Scenario: The create affordance offers a title and no key

- **WHEN** the create affordance is rendered
- **THEN** it accepts a title
- **AND** it offers no field for a creator key or an identity to create under

#### Scenario: The affordance is offered when no key exists

- **WHEN** the create affordance is rendered on a peer with no usable signing key
- **THEN** the affordance is present and can be acted on

#### Scenario: A creation refused for want of a key renders the core's reason

- **WHEN** creation is attempted and the core refuses it because no usable signing
  key exists
- **THEN** the screen renders the reason the core gave
- **AND** it does not report a Stoa as created

#### Scenario: An empty title reaches the core rather than being refused by the view

- **WHEN** the user creates a Stoa with an empty title
- **THEN** the create call is made
- **AND** the reply decides the outcome rather than a check in the view

#### Scenario: A title made only of blank characters reaches the core as typed

- **WHEN** the user creates a Stoa with a title of three U+0020 spaces, and again
  with the title U+200B U+3000
- **THEN** in each case the create call is made
- **AND** the title it carries is exactly what was typed
- **AND** the reply decides the outcome rather than a check in the view

#### Scenario: A creation refused for a blank title renders the core's reason

- **WHEN** the user creates a Stoa with an empty title, and again with a title of
  three U+0020 spaces, and the core answers with the error shape
- **THEN** the screen renders the reason the core gave
- **AND** it does not report a Stoa as created
- **AND** no address is rendered as that of a Stoa just created

### Requirement: A created Stoa's address is shown, and creating the same title twice is not a collision

After a successful creation the screen MUST render the address the core returned.

There is no registry to look a Stoa up in later, so the address is the only way to
share what was just made. `stoa-membership` requires the creation call to return
it rather than a bare success for this reason, and a screen that discarded it
would leave the user holding a Stoa they cannot name.

Creating a Stoa with a title the same creator has used before MUST be rendered as
success naming the same Stoa, and MUST NOT be rendered as a name collision, a
duplicate, or an error. A genesis record carries no nonce and no timestamp, so the
same creator and the same title **is** the same record and the same address —
`stoa-membership`'s "Creating the same Stoa twice yields one Stoa, not two" pins
this because the intuitive expectation is the opposite one.

The screen MUST NOT alter the user's title to obtain a different Stoa — appending
a number or a suffix on the user's behalf mints a Stoa with a title they did not
choose, permanently and at an address that cannot be withdrawn. Where a user wants
a second Stoa, the title they supply is what distinguishes it.

#### Scenario: The new Stoa's address is rendered after creation

- **WHEN** creation succeeds
- **THEN** the address the core returned is rendered

#### Scenario: The same title twice is one Stoa reported twice, not an error

- **WHEN** a Stoa is created and then created again with the same title, and the
  core returns the same address both times
- **THEN** the second outcome is rendered as success
- **AND** the address rendered is the one the first creation returned
- **AND** nothing is rendered as a collision, a duplicate, or an error

#### Scenario: The title submitted is the title the user typed

- **WHEN** the user creates a Stoa with a title that a previous creation already
  used
- **THEN** the title sent to the core is the one the user typed, with nothing
  appended to it

### Requirement: Nothing on these screens claims a per-Stoa identity, a membership, or a moderator

These screens MUST NOT state or imply that joining creates an identity for that
Stoa alone, that joining enrols the user in a membership other peers can observe,
or that the user moderates a Stoa.

Each is a claim the software does not keep. **Per-Stoa identity** is built in the
core and not switched on: one key signs in every Stoa in this release, so telling
a user that joining generates an identity for that Stoa alone tells them they have
an unlinkability property they do not have — the one failure here that could
actually harm someone. **Membership** is a record of what this user chose, held on
this machine; there is no membership list, nobody is notified, and no peer can be
prevented from publishing. **Moderation** is a question these screens cannot
answer at all: `stoa-membership` states that a listed Stoa means the user chose
it, not that the user governs it, and that the retained creator key is never
re-checked against the peer's current signing key — so a peer whose key changed
holds Stoas it created and can no longer moderate, silently and
indistinguishably.

What joining does do is outside these prohibitions and is honest to say: it begins
collecting that Stoa's records on this machine.

#### Scenario: No per-Stoa identity is promised

- **WHEN** the preview and its explanatory notes are rendered
- **THEN** nothing rendered states that joining generates an identity used only
  for that Stoa

#### Scenario: No moderator status is asserted for a listed or created Stoa

- **WHEN** the list renders a Stoa, and the creation outcome renders a newly
  created one
- **THEN** nothing rendered states that the user moderates it

### Requirement: Every core call on these screens goes through the view's one call path

Every call these screens make to the core MUST go through the view's single call
wrapper, and each core method MUST be named in exactly one place in the view.

That wrapper normalises every reply into exactly one of two shapes — a value or an
error — so the error branch exists once rather than once per call site. It is also
the only thing that makes these screens testable: the host injects the bridge, so
a screen calling the host directly is a screen whose failure paths cannot be
exercised anywhere. A method name spelled at a call site is a typo away from a
silent empty screen, which is the state this capability spends its other
requirements distinguishing from a real one.

A reply that is not JSON, an absent bridge, and a thrown call MUST each reach the
screen as a failure rather than as a value.

#### Scenario: A screen renders a failure when the core is unreachable

- **WHEN** any of these screens is rendered with no bridge to the core
- **THEN** the screen is in its failed state
- **AND** it reports that the core is not reachable

#### Scenario: A reply that is not JSON is a failure rather than a value

- **WHEN** a call from one of these screens is answered with text that is not JSON
- **THEN** the screen is in its failed state
- **AND** it renders no Stoa obtained from that reply

### Requirement: The preview asks the core what the Stoa is called, and renders the answer as what the reply says it is

When the preview is rendered for a reference, the view MUST call
`stoa-metadata`'s `getStoa` with that reference's address and genesis record,
without the user acting. The address and record sent MUST be the ones a join
from the same preview would send. This lookup MUST NOT be made while no
reference is being previewed, and MUST NOT be made for input that is not a
well-formed Stoa reference.

In this requirement a **lookup** is that call, a **fallback reply** is a
successful `getStoa` reply whose `isGenesisFallback` is `true`, and a
**non-fallback reply** is one whose `isGenesisFallback` is `false`. A reply that
"A lookup that fails, or answers in no recognisable shape, renders no title and
withdraws no join" treats as a failed lookup is neither, and nothing in this
requirement is rendered from it.

**A fallback reply.** Its `title` MUST be rendered in the founding-title
position, labelled as the founding title, as "Joining shows what is being
joined, and joins nothing until the user acts" requires of an available founding
title — unless that requirement gives the position to a successful join reply's
founding title for the same reference. The screen MUST NOT render a current
title or a description from it. It MUST state that this machine holds no title
set by a moderator for this Stoa, and MUST NOT state or imply that the Stoa has
not been renamed.

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

- **WHEN** the lookup answers a fallback reply whose `title` is not blank
- **THEN** that title is rendered in the founding-title position, labelled as
  founding
- **AND** no current title is rendered
- **AND** the statement that no founding title is available here is not
  rendered

#### Scenario: A looked-up title with one visible letter among blank characters is a title

- **WHEN** the lookup answers a fallback reply whose `title` is U+0020 U+200B,
  the letter `a`, U+3000 U+FEFF, and again a non-fallback reply with that
  `title`
- **THEN** in each case the title is rendered in the position the reply's
  `isGenesisFallback` decides
- **AND** the letter `a` is in what is rendered there
- **AND** no failure is rendered for the lookup

#### Scenario: A fallback is stated as no moderator-set title held here

- **WHEN** the lookup answers a fallback reply
- **THEN** the screen states that this machine holds no title set by a
  moderator for this Stoa
- **AND** nothing rendered states that the Stoa has not been renamed

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
`title` or `description` is not a string, or whose `title` is blank, MUST be
treated as a failed lookup, with a reason of the view's own naming what was
wrong with the reply in place of the core's. It MUST NOT be rendered as a
fallback reply or as a non-fallback one. A blank `title` is not a founding title
and not a current title, whatever `isGenesisFallback` says: `stoa-metadata`
requires that no successful `getStoa` reply carries one.

**Blank, on this screen, is exactly what the `stoa-genesis` capability's
requirement "A blank title is not a valid title" defines**: a title every
character of which is one of the thirty blank characters that requirement lists,
the empty title included. The view MUST test a title against that list, no wider
and no narrower. A title carrying at least one character outside the list is not
blank, however many blank characters it also carries or wherever they sit, and
MUST NOT be treated as blank or as a malformed reply.

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

#### Scenario: A lookup reply carrying a blank title is a failure

- **WHEN** the lookup answers a successful reply whose `title` is the empty
  string, once with `isGenesisFallback` `true` and once with it `false`, and
  again whose `title` is U+0020 U+200B U+3000, once with `isGenesisFallback`
  `true` and once with it `false`, and no join has been made
- **THEN** in each case the screen renders a failure of its own naming the
  title as blank
- **AND** neither the founding-title position nor the current-title position
  is rendered
- **AND** no description from that reply is rendered
- **AND** the statement that this machine holds no moderator-set title is not
  rendered
- **AND** the screen states that no founding title is available here
- **AND** the join affordance is offered

#### Scenario: The view's blank test is the stoa-genesis list, no wider and no narrower

- **WHEN** the lookup answers a fallback reply whose `title` consists of exactly
  one of the thirty blank characters, in turn for every one of them, and again
  a fallback reply whose `title` is U+200E alone, and again one whose `title`
  is U+180E alone
- **THEN** each of the thirty is rendered as a failure of the view's own naming
  the title as blank
- **AND** neither the U+200E reply nor the U+180E reply is rendered as a
  failure, and each fills the founding-title position

### Requirement: A lookup's answer is rendered only for the reference it was made for

A lookup's answer — a title, a description, a fallback, or a failure — MUST be
rendered only while the reference it was made for is the one being previewed.
When the previewed reference changes, a lookup MUST be made for the new
reference, and nothing answered for the previous one MUST be rendered for it.

#### Scenario: A second preview renders nothing from the first one's lookup

- **WHEN** a reference is previewed and its lookup answers a fallback reply
  with a title that is not blank, and a second reference at a different address is
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
