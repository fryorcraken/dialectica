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

A Stoa whose founding title is **empty** MUST render as a row, MUST NOT be
omitted, and MUST NOT be given a substitute title. An empty title is legal — the
genesis record has no minimum length and `stoa-membership` requires creation to
accept one — so a row that collapsed or that read "Untitled" would be,
respectively, a Stoa the user cannot reach and a title no peer agrees on.

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
