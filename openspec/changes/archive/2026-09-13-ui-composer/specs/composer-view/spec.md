## Purpose

Defines what the view does around publishing a post, a reply or a vote: which
affordances are gated on the posting probe, what a draft is owed when a publish
is refused, what may and may not be claimed once one succeeds, and what a vote
control may display given that nothing in the current contract reads a vote.

`content-authoring` owns the publish itself and `posting-capability` owns the
probe; neither is restated here. This capability contracts only the obligations
that fall on the interface because core's honest answer is incomplete without
something the view does.

## ADDED Requirements

### Requirement: A compose affordance is rendered only when the probe says posting is possible

The view SHALL ask the posting probe for the Stoa it is composing in, and SHALL
render a text input for a post or a reply only when that probe reports posting is
possible.

When the probe reports posting is not possible, the view SHALL render no text
input for that Stoa — not a disabled one, not a read-only one, and not one that
accepts text and refuses submission. A box the user can type into and not submit
loses what they wrote.

The view SHALL NOT decide this from a build flag, a configuration value, or a
probe answer obtained before the current render.

A probe that cannot be reached, or whose reply the view cannot interpret, SHALL
close the gate rather than open it.

**This capability governs how a compose affordance behaves, not which screens
offer one.** It does not require a reply affordance to be reachable, and a screen
listing thread heads rather than a thread's posts offers no place to put one: a
reply names a parent op, and a reply box under a thread head would be a
thread-view affordance on a screen that is not one. Every requirement below about
replying therefore constrains what the composer does **when** a reply is
submitted or refused, and is satisfied whether or not a screen currently offers
that path.

#### Scenario: A closed gate renders no text input

- **WHEN** the probe reports that posting is not possible
- **THEN** the view renders no text input for composing in that Stoa
- **AND** it renders no submit affordance for one

#### Scenario: An open gate renders a text input

- **WHEN** the probe reports that posting is possible
- **THEN** the view renders a text input and a submit affordance

#### Scenario: An unreachable probe closes the gate

- **WHEN** the probe call fails, or answers with something that is not a probe
  reply
- **THEN** the view renders no text input
- **AND** the state it reaches is the same one a probe reporting "not possible"
  produces

#### Scenario: A probe reporting neither capability nor a reason closes the gate

- **WHEN** the probe answers with an object carrying neither an affirmative
  capability nor a reason
- **THEN** the view renders no text input

### Requirement: A closed gate shows the reason verbatim and offers a fix

When the probe reports that posting is not possible, the view SHALL display the
reason the probe supplied, as supplied, without rewording, truncating or
substituting text of its own.

`posting-capability` requires that reason to name a fix, and requires that its
wording not be part of the contract. It follows that the view SHALL NOT select
what it displays, or which affordance it offers, by matching on the reason's
text: a view branching on prose turns every improvement to that prose into a
silent breaking change.

The view SHALL NOT substitute a reason of its own for the one the probe supplied,
including a reason drawn from a design reference. Two reasons for one blockage
are two things to maintain, and the copy the view would otherwise carry is not
checked against what the probe can actually establish.

The view SHALL also present an affordance leading to guidance on resolving the
blockage, so that the reader is shown the reason and a route to acting on it
rather than the reason alone.

The view SHALL state, **in the closed gate's own body**, that no compose box is
shown and why — that a box the reader could type into and not submit would lose
what they wrote — so the absence reads as a decision rather than as a missing
feature. The requirement is on the **statement being present where the gate is
rendered**, not on it occupying any particular region of the screen.

**These requirements are on what the interface says, not on which stored string
it says it with, and that is deliberate.** The design bundle supplies wording for
both (`compose.fix` and `compose.apparatus`) and remains the recommended source
for it, but **the bundle is not part of this repository** — `copy.json` has never
been committed to it — so a requirement to reproduce one of its strings verbatim
cannot be checked by anything inside the repository. What such a requirement
actually produces is a hand transcription pinned by a literal, which fails when
someone rewords the interface and never when the interface diverges from the
bundle: the opposite of the check it appears to be. A requirement no gate can
enforce is worse than a looser one that can, because it reads as enforced.

So conformance is judged on the statement being made and being accurate. Copying
the bundle's wording is the easiest way to satisfy that and SHALL NOT be read as
required by it.

That distinction is the requirement rather than a note about it. An earlier
version of this capability tied the sentence to a marginal annotation column;
that column is annotation explaining the design to a reader of the design, it
reached the shipped interface by mistake, and it is being removed. An obligation
expressed as "this text appears in that column" disappears with the column,
silently and while still being required. So the obligation is stated as text the
reader facing the gate can see, and SHALL NOT be discharged by placing it
anywhere a reader of the gate would not encounter it.

**The heading for a closed gate SHALL name the affordance actually withheld and
SHALL NOT promise that submitting will send anything.** Where the gate withholds
both posting and replying, a heading naming only replying is wrong about what is
blocked. And a heading or reason stating that the box returns when a submission
"would actually send" claims a delivery outcome: the probe answers whether a
publish would be accepted and stored locally, and no part of this system checks
whether anything sends. Requirement "A successful publish claims local storage
and never delivery" forbids that claim after a publish, and it is equally
forbidden before one.

#### Scenario: The closed-gate heading names what is withheld

- **WHEN** the gate is closed where both posting and replying are unavailable
- **THEN** the heading displayed does not describe only replying as unavailable

#### Scenario: No part of the closed gate promises a submission will send

- **WHEN** the gate is closed
- **THEN** no text the view supplies states that the compose box returns when a
  submission would send, or otherwise claims a delivery outcome

#### Scenario: The reason reaches the screen unchanged

- **WHEN** the probe reports posting is not possible with a given reason
- **THEN** that reason's text is displayed
- **AND** the text displayed is character-for-character the text the probe
  supplied

#### Scenario: Two different reasons both reach the screen unchanged

- **WHEN** the probe reports posting is not possible with one reason, and then
  with a different reason
- **THEN** each is displayed as supplied
- **AND** the view's behaviour is the same for both, no branch having been taken
  on the text

#### Scenario: The fix affordance is offered alongside the reason

- **WHEN** the probe reports posting is not possible
- **THEN** the view displays the reason
- **AND** displays an affordance leading to guidance on resolving it

#### Scenario: The closed gate says why there is no box

- **WHEN** the probe reports posting is not possible
- **THEN** the view states that no compose box is shown because one the reader
  could type into and not submit would lose what they wrote

#### Scenario: That statement survives without the annotation column

- **WHEN** the gate is closed and every region of the screen given over to
  annotating the design is disregarded
- **THEN** the statement explaining the missing box is still displayed

### Requirement: The composer never alters the text the user typed

The view SHALL publish the body exactly as the user entered it. It SHALL NOT
strip, replace, normalise, trim or reorder any character of the draft before
submitting it.

`content-authoring` requires text a caller supplies to reach the op unchanged,
because an op is signed over its bytes. A view that altered the draft would
publish, permanently and under the user's signature, something the user did not
write.

Where the draft contains characters the display-side sanitiser would **remove**
when rendering it back — bidirectional controls, zero-width characters, and other
glyphless characters that alter how their neighbours render — the view SHALL warn
the author before they submit, naming how many such characters were found. The
warning SHALL NOT block submission.

The warning exists because the author is the only person who can still change the
text. A reader cannot consent to what they are shown, which is why peer text is
sanitised on display; an author can, which is why their own text is not.

**The warning is scoped to removals and SHALL NOT be required to cover the
sanitiser's homoglyph marking**, and the boundary is drawn here rather than left
to an implementation to discover.

Whether a character is removed is a membership test against a fixed set. Whether
a character is *marked* is a judgement over the whole string: which of three
scripts dominates it, what happens on a tie, and which characters are excluded
from the judgement entirely — decided after removal has already changed the
string being judged. A second implementation of that judgement in the view would
produce a different number from the sanitiser's on the same text, and **nothing
in the system would observe the disagreement**, because the two counts are never
computed over the same string at the same time.

The view has no way to obtain the marked count for a draft: the sanitiser runs
only when stored content is rendered outward, and no method on the module surface
sanitises a caller-supplied string. Obtaining it would mean widening the wire
contract, which is out of scope for a change confined to the view.

The cost is stated rather than absorbed: **a draft mixing confusable scripts is
under-reported**, and the author is not warned about it. Closing that gap means
the view obtaining the count from the same code that produces it; reimplementing
the judgement in the view SHALL NOT be treated as a way to close it.

#### Scenario: A draft is published byte-for-byte as typed

- **WHEN** a draft containing multi-byte characters, bidirectional controls and
  zero-width characters is submitted
- **THEN** the body sent to core is identical to the draft
- **AND** no character of it was removed or replaced

**The warning is information and not a gate.** It SHALL NOT itself withhold
submission, and a draft SHALL NOT become unsubmittable by virtue of carrying such
characters. Whether the draft is submittable for other reasons — its length above
all — is decided elsewhere and is not affected either way by the warning.

#### Scenario: An author is warned about invisible characters before submitting

- **WHEN** a draft contains characters the display sanitiser would remove, and is
  otherwise submittable
- **THEN** the view displays a warning naming how many were found
- **AND** the submit affordance remains available

#### Scenario: The warning neither grants nor withholds submission

- **WHEN** two drafts identical in length, one containing such characters and one
  not, are each entered
- **THEN** both are submittable, or neither is
- **AND** the difference between them changes only whether the warning is
  displayed

#### Scenario: A clean draft carries no warning

- **WHEN** a draft contains no characters the display sanitiser would remove
- **THEN** no sanitiser warning is displayed

#### Scenario: A draft mixing confusable scripts is not warned about

- **WHEN** a draft contains a character from a minority script among confusable
  scripts, and no character the sanitiser would remove
- **THEN** no sanitiser warning is displayed, the view not judging script mixing
- **AND** the draft reaches core unchanged when it is submitted

### Requirement: The body limit is expressed in bytes and shown before submission

The view SHALL treat the limit on a body as a count of **bytes in its UTF-8
encoding**, never a count of characters. A draft of a given character count can
be several times that many bytes, so a view counting characters permits a
submission core refuses.

The view SHALL tell the user that a draft exceeds the limit **before** they
submit it, and SHALL keep the submit affordance unavailable while it does.

The view SHALL NOT truncate a draft to fit. Silently discarding the end of what
someone wrote is a worse outcome than refusing to send it.

Where core refuses a body for exceeding the cap despite this check, the view
SHALL report it as a refusal like any other and SHALL retain the draft.

#### Scenario: An over-length draft is refused before it is sent

- **WHEN** a draft whose UTF-8 encoding exceeds the limit is entered
- **THEN** the view reports that it is too long
- **AND** the submit affordance is unavailable
- **AND** no publish call is made

#### Scenario: The limit counts bytes rather than characters

- **WHEN** a draft of multi-byte characters whose character count is within the
  limit but whose UTF-8 byte count exceeds it is entered
- **THEN** the view reports that it is too long

#### Scenario: An over-length draft is not truncated

- **WHEN** a draft exceeds the limit
- **THEN** the draft the view holds is still the full text the user entered

#### Scenario: A draft within the limit is submittable

- **WHEN** a draft whose UTF-8 encoding is at or under the limit is entered
- **THEN** the submit affordance is available

### Requirement: A successful publish claims local storage and never delivery

When a publish succeeds, the view SHALL state that the content was stored on this
machine. It SHALL NOT state, or imply, that it was sent, delivered, received by
any peer, propagated, published to the Stoa at large, or that anyone else can see
it.

`content-authoring` requires the reply to carry no delivery outcome at all, and
the outcome a user cares about arrives after the call has returned. An interface
claiming delivery would therefore be claiming something no part of the system has
checked.

The view SHALL NOT display a count of peers reached, a delivery state, or a
progress indicator that resolves into a delivery claim.

**Saying nothing about delivery is not sufficient, and the view SHALL positively
deny delivery knowledge.** Where a publish has succeeded, the view SHALL state,
in the screen's own body, that whether any other peer has received the content is
not something this software can report. That statement SHALL accompany the
success itself rather than being available only elsewhere in the interface.

Every other delivery rule in this capability is a prohibition, and a prohibition
is discharged by silence. Silence is the wrong answer here, because the reader's
default assumption on seeing a forum post submit successfully is that it went
somewhere — so an interface that merely declines to mention delivery lets that
assumption stand unchallenged while being fully compliant.

**The stakes are specific to this system rather than general good manners.** A
publish reply carries no delivery outcome by design; delivery is not wired at
all; and a post whose body is legal but near the cap encodes to more than the
transport will carry, so it is accepted locally and silently refused by every
receiving peer. An author therefore cannot distinguish a post nobody has received
from one everybody has, and the interface is the only place that fact can be
told to them.

A prohibition-only contract also cannot be tested for. A test can sweep for a
forbidden claim and pass when the honest sentence is deleted, which is how such a
statement is lost: not by someone deciding to remove it, but by it leaving
attached to something else.

#### Scenario: A success denies delivery knowledge rather than omitting it

- **WHEN** a publish succeeds
- **THEN** the view states that whether any other peer has received the content
  is not something it can report
- **AND** that statement is displayed with the success rather than only in
  another part of the interface

#### Scenario: The denial survives without the annotation column

- **WHEN** a publish succeeds and every region of the screen given over to
  annotating the design is disregarded
- **THEN** the denial is still displayed

#### Scenario: A success names local storage

- **WHEN** a publish succeeds
- **THEN** the message displayed states the content was saved on this machine

#### Scenario: A success claims nothing about delivery

- **WHEN** a publish succeeds
- **THEN** the message displayed contains no claim that the content was sent,
  delivered, received, propagated, or is visible to anyone else

#### Scenario: No delivery indicator is rendered

- **WHEN** a publish succeeds
- **THEN** the view displays no peer count, delivery state or delivery progress
  for that content

### Requirement: The draft is cleared when the op was newly stored, and kept otherwise

Where a publish succeeds reporting the op was **newly stored**, the view SHALL
clear the draft.

Where a publish succeeds reporting the op was **not** newly stored, and on every
refusal, the view SHALL retain the draft. The three cases are therefore not
uniform, and the asymmetry is the decision rather than an inconsistency.

Clearing on a newly stored op removes an affordance that is ready to produce a
confusing outcome: the same text submitted again is the same op id, so the second
submission is a deduplicated no-op reported as "already published" — a state the
user reached by using a control that looked ready to publish something. Nothing
is lost by clearing, because the text is published and readable.

Keeping it in the other two cases follows from the same reasoning applied to
different facts. On a refusal nothing was published, so the draft is the only
copy. On a deduplicated publish nothing new was written, and a user whose
intention was to publish something different needs the text in front of them to
edit — clearing would take away exactly what they need.

The cost of clearing is named: a user writing a near-identical follow-up loses
their starting point and retypes it. That is a convenience, weighed against an
interface offering a control whose use produces a confusing no-op.

#### Scenario: A newly stored publish clears the draft

- **WHEN** a publish succeeds reporting the op was newly stored
- **THEN** the composer holds no draft

#### Scenario: The draft's fate differs across the three outcomes

- **WHEN** the same submission is made against a core reporting a newly stored
  op, against one reporting an op that was not newly stored, and against one
  returning a refusal
- **THEN** the draft is cleared in the first case
- **AND** retained in the other two

### Requirement: An already-published op is reported as its own outcome

Where a publish succeeds reporting that the op was not newly stored, the view
SHALL display a message stating that this content was already published.

That message SHALL be distinguishable from the message shown for a newly stored
op, and SHALL be distinguishable from the message shown for a refusal. Nothing
failed, so a refusal is wrong; nothing new was stored, so reporting a fresh
success leaves the user looking for a post that will never appear.

The view SHALL NOT display a progress indicator that resolves without a message,
and SHALL NOT report this outcome as an error.

#### Scenario: A deduplicated publish says the content was already published

- **WHEN** a publish succeeds reporting the op was not newly stored
- **THEN** the view displays a message stating the content was already published

#### Scenario: A deduplicated publish keeps the draft

- **WHEN** a publish succeeds reporting the op was not newly stored
- **THEN** the draft the composer holds is the text the user entered

#### Scenario: The three outcomes are mutually distinguishable

- **WHEN** the same submission is made against a core reporting a newly stored
  op, against one reporting an op that was not newly stored, and against one
  returning a refusal
- **THEN** the view reaches three states
- **AND** no two of the three display the same message

#### Scenario: An already-published op is not an error state

- **WHEN** a publish succeeds reporting the op was not newly stored
- **THEN** the view is not in the state it reaches for a refused publish

### Requirement: A refused publish keeps the draft and names the refusal

Where a publish is refused, the view SHALL retain the draft in the composer,
unchanged, and SHALL display the message core supplied.

The view SHALL NOT clear, truncate or alter the draft on any refusal. The refusal
most likely to occur in ordinary use — a reply whose parent has not yet reached
this peer — is expected to succeed on a retry, so discarding what the user wrote
is the worst available response to it.

The view SHALL NOT reword core's message, and SHALL NOT select what it displays
by matching on that message's text.

A refusal SHALL NOT itself withhold submission, so that a refusal expected to
succeed later can be retried. It does not follow that every refused draft is
submittable: where the draft is one the length gate withholds, that gate still
withholds it, and the retry becomes available when the draft comes under the
limit. A refusal removes no reason to submit and adds none.

#### Scenario: A refusal leaves the draft intact

- **WHEN** a publish is refused for any reason, for a draft the length gate does
  not withhold
- **THEN** the draft the composer holds is the text the user entered
- **AND** the submit affordance remains available so the submission can be
  retried

#### Scenario: The refusal's message reaches the screen

- **WHEN** a publish is refused with a given message
- **THEN** that message is displayed as supplied

#### Scenario: A retry after a refusal sends the same body

- **WHEN** a publish is refused and the submission is retried without the user
  editing the draft
- **THEN** the body sent on the retry is identical to the body sent on the first
  attempt

### Requirement: Every reply refusal is presented as possibly temporary, because the view cannot tell which is

`content-authoring` refuses a reply naming a parent this peer does not hold
distinguishably from one naming a held op that is not a post — and that
distinction reaches the view **only as differing prose inside one error message**.
The wire contract carries a single error shape with no machine-readable
discriminant, and `posting-capability` establishes for its own reasons that a
caller must not branch on a message's wording. So the view **cannot** determine
which refusal it received.

This capability therefore SHALL NOT require the view to distinguish them, and the
view SHALL NOT attempt to by matching on message text. What it SHALL do instead
is present every reply refusal in the way that is safe under either reading:
display core's message, retain the draft, and leave a retry available.

**That asymmetry is deliberate and is the reason the rule is stated this way.**
Offering a retry for a refusal that cannot succeed costs the user one press.
Withholding it from the parent-not-yet-arrived case — which is ordinary in a
peer-to-peer forum, is nobody's fault, and is expected to succeed once the parent
propagates — would tell the user a temporary condition is permanent and discard
the thing most likely to work.

The view SHALL NOT describe any reply refusal as an error the user caused or as
a permanent failure, since for the common case neither is true.

**Making these two mechanically distinguishable requires a discriminant core does
not expose**, and adding one widens the wire contract. That is out of scope here
and is named so it is not mistaken for an oversight.

#### Scenario: A refused reply keeps its draft and a retry

- **WHEN** a reply is refused because the parent is not held
- **THEN** core's message is displayed
- **AND** the draft is retained
- **AND** a retry is available

#### Scenario: A refusal for a target that is not a post is presented the same way

- **WHEN** a reply is refused because the target held is not a post
- **THEN** the draft is retained
- **AND** a retry is available, the view having no way to establish that retrying
  cannot help

#### Scenario: No reply refusal blames the user or claims permanence

- **WHEN** a reply is refused
- **THEN** the message the view supplies alongside core's does not state that the
  user caused the failure or that it is permanent

### Requirement: A published post is not shown until core has been read again

The view SHALL NOT insert a post, a reply or any row into a feed or thread on the
strength of a publish having succeeded. Content SHALL appear only as core
reported it in a read.

A row the view composed would carry fields the view invented — what the
sanitiser found, whether the post has been revised — and a view that invents them
renders something core never said.

After a successful publish the view SHALL re-read from core. Where the published
content does not appear in that read, the view SHALL NOT treat this as a failure
and SHALL NOT withdraw the success message, since a reply is not a thread head
and has no row of its own in a feed of thread heads.

The success message SHALL NOT direct the reader to the published content's
position on screen, because the view cannot establish that it has one.

#### Scenario: No row is added by a publish

- **WHEN** a publish succeeds and the subsequent read returns the rows it
  returned before
- **THEN** the rows the view holds are exactly the rows that read returned
- **AND** no row composed by the view appears among them

#### Scenario: Published content appears once core reports it

- **WHEN** a publish succeeds and the subsequent read returns a row for it
- **THEN** that row is displayed
- **AND** its fields are the ones the read supplied

#### Scenario: Content absent from the re-read does not retract the success

- **WHEN** a publish succeeds and the subsequent read does not return a row for
  it
- **THEN** the success message is still displayed
- **AND** the view is not in a failure state

### Requirement: The vote control displays no score

The view SHALL NOT display a number, bar, ratio or other magnitude purporting to
represent how a post was voted on.

No call in the current contract returns one: a publish reply carries an op id and
a newly-stored flag, and a feed row carries no score, no tally and no vote count.
A control rendering its default of zero would be worse than one rendering
nothing, because zero is a number and reads as a tally — the claim that this post
is known to have received no votes, which core has never said and which is false
as soon as any peer has voted.

The absence SHALL be a rendered decision rather than a value that happens to be
empty, so that a later change exposing a count has to decide to display it.

**That is a requirement on how the control is built, not on the interface
explaining itself.** It is discharged by the number being suppressed by default,
so that displaying one is something a future change must opt into; it does not
oblige the view to carry prose about why no number is there. Unlike the delivery
denial above, a missing number invites no false inference a reader would
otherwise draw — nothing appears, so nothing is claimed — whereas a successful
submission does invite one.

The view SHALL NOT present a vote as having moved a post, changed an ordering,
changed what anyone else sees, or fed into any ranking. No ordering offered by
the view SHALL be labelled as vote-based while no ordering consults votes.

#### Scenario: No score is displayed beside a post

- **WHEN** a post is rendered with a vote control
- **THEN** no number representing a score, tally or vote count is displayed

#### Scenario: The score is absent rather than zero

- **WHEN** a post is rendered with a vote control, and a post is rendered with a
  vote control after the viewer has published a vote on it
- **THEN** neither displays any numeral as that control's score
- **AND** the two are identical in what they display in the score's place, no
  number having appeared or changed

#### Scenario: The control claims no effect on ordering

- **WHEN** a vote is published successfully
- **THEN** the view displays no claim that the post moved, ranked differently, or
  became more or less visible to anyone

#### Scenario: No ordering is offered as vote-based

- **WHEN** the orderings the view offers are examined
- **THEN** none is labelled as ordering by votes

### Requirement: A row whose shape core did not guarantee degrades to what it can support, and is neither hidden nor fatal

Content rows reaching the view are peer-derived, and the view SHALL NOT assume
any field of a row is present or of the expected type merely because the module
it came through currently always sends it.

Where a row lacks a field an affordance needs, the view SHALL render the row and
SHALL render that affordance **inert** — present but offering no action — rather
than doing any of the following:

- omitting the row,
- failing the read that returned it,
- or rendering an affordance that acts on a substituted, defaulted or absent
  value.

**No value derived from a missing field SHALL reach a core call**, and the guard
SHALL be applied where the value is produced rather than at each place it is
used, so that a later consumer inherits it rather than having to restate it.

**The three outcomes differ in who bears the cost of one malformed row, which is
what decides between them rather than taste.**

Rendering the row inert costs the reader one control on one row. Omitting the row
hides peer content, which on a forum whose purpose is resisting censorship is the
outcome the system exists to prevent — and it hides it *silently*, so no reader
can tell a suppressed row from a row nobody wrote. Failing the read lets a single
malformed row blank an entire feed, which is a denial of service any peer can
mount for free, and it collides with this view's governing rule that an empty
store and an unreadable one must never look alike.

Only the first confines the damage to the part that is actually broken. A row the
view cannot offer every affordance for is still a row worth reading, and reading
is what the forum is for.

**This rule is general and SHALL NOT be read as being about any one field.** The
argument that a key is safe "by construction" because it *is* the post holds only
while every row carries that key, which is a property of peer-supplied data and
not of the code — and the same sentence would be equally wrong about any other
row field. A view that refuses to trust the shape of a reply's envelope while
trusting the shape of the elements inside it holds two positions about one reply.

#### Scenario: A row missing a field an affordance needs still renders

- **WHEN** a read returns a row lacking the field an affordance requires
- **THEN** the row's content is displayed
- **AND** the read is not in a failure state
- **AND** the other rows in that read are displayed

#### Scenario: The affordance that cannot work is inert rather than absent-acting

- **WHEN** a row lacks the field its vote control needs
- **THEN** that control offers no action
- **AND** acting on it reaches no core call

#### Scenario: Two rows missing the same field do not share one slot

- **WHEN** a read returns two rows that both lack the field identifying them, and
  a vote is recorded against the first
- **THEN** the second shows no vote

#### Scenario: No call carries a value derived from a missing field

- **WHEN** a vote is attempted on a row lacking its identifying field
- **THEN** no publish call is made
- **AND** no request is sent omitting the field that would have named the target

#### Scenario: A well-formed row is unaffected

- **WHEN** a read returns a row carrying every field
- **THEN** its affordances are offered and act normally

### Requirement: The vote control shows the viewer their own vote back, and only for this session

When a vote is published successfully, the view SHALL show that vote back on the
control for the post it targeted, in the direction published.

This is the one thing about a vote that is true and immediate: the user pressed
a button and the interface remembers. `posting-capability`'s probe governs
whether the control is offered at all, as for any other posting affordance.

No call returns the viewer's earlier votes, so the view SHALL show a vote back
only for a vote it published while it is open, and SHALL NOT present an unknown
vote state as an absence of a vote in any way that claims the user has not voted.
Where the view has no record of a vote on a post, the control SHALL render in the
same neutral state it renders before any vote is published.

A publish that is refused SHALL NOT change the control's state, because no vote
was recorded.

#### Scenario: A published vote is reflected on the control

- **WHEN** a vote is published successfully in a direction
- **THEN** the control for that post shows that direction as the viewer's own

#### Scenario: A refused vote leaves the control unchanged

- **WHEN** a vote publish is refused
- **THEN** the control shows the state it showed before the attempt

#### Scenario: A vote on one post does not mark another

- **WHEN** a vote is published on one post
- **THEN** the control for a different post shows no vote

#### Scenario: That separation does not rest on the rows being well-formed

- **WHEN** a vote is published on one post among rows that do **not** carry
  distinct identifying fields
- **THEN** no other row's control shows a vote
- **AND** the separation therefore holds for rows as peers may send them, not
  only for rows every field of which core happened to supply

#### Scenario: A post with no recorded vote renders neutrally

- **WHEN** a post is rendered for which the view has published no vote
- **THEN** its control shows neither direction as the viewer's own

### Requirement: Every publish goes through the view's single core call path

Every publish the view performs SHALL go through the same call path as every
other core call, so that a failure to reach core, a reply that is not JSON, and
a reply carrying core's error shape are each handled in one place rather than
once per call site.

A publish SHALL NOT be reported as successful on a reply the view could not
interpret. A reply that is not core's success shape for that call SHALL be
treated as a refusal, and the resulting state SHALL be the refused state — the
draft retained and a message displayed — rather than a success with an absent op
id.

#### Scenario: An unreachable core is a refusal, not a success

- **WHEN** the core module cannot be reached and a publish is attempted
- **THEN** the view is in its refused state
- **AND** the draft is retained

#### Scenario: A reply that is not JSON is a refusal

- **WHEN** a publish call returns text that is not JSON
- **THEN** the view is in its refused state
- **AND** it does not display a success message

#### Scenario: A success reply without an op id is a refusal

- **WHEN** a publish call returns an object carrying no op id and no error
- **THEN** the view is in its refused state
- **AND** it does not display a success message

#### Scenario: Core's error shape is a refusal carrying its message

- **WHEN** a publish call returns core's error shape
- **THEN** the view is in its refused state
- **AND** the message displayed is the one the error carried
