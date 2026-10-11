# composer-view Specification

## Purpose
Defines what the view does around publishing a post, a reply or a vote: which
affordances are gated on the posting probe, what a draft is owed when a publish
is refused, what may and may not be claimed once one succeeds, and what a vote
control may display given that nothing in the current contract reads a vote.

`content-authoring` owns the publish itself and `posting-capability` owns the
probe; neither is restated here. This capability contracts only the obligations
that fall on the interface because core's honest answer is incomplete without
something the view does.

## Requirements

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
publish reply carries no delivery outcome by design; where core hands the op to
the network, it learns nothing of whether any peer received
it; and a post whose body is legal but near the cap encodes to more than the
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

Clearing on a newly stored op removes an affordance that is ready to produce an
outcome the user did not intend. **The reason has changed, and the behaviour has
not.** It previously rested on the same text submitted again being the same op
id, so that a second submission was a no-op the user reached through a control
that looked ready to publish. That is no longer so: an op's bytes carry a counter
that advances between publishes, so **the same text submitted again is a second
post.** Leaving a published draft in the composer now offers a control that is
ready to duplicate the post rather than one ready to do nothing, which is the
worse of the two outcomes and makes clearing more clearly right than before.
Nothing is lost by clearing, because the text is published and readable.

Keeping it in the other two cases follows from the same reasoning applied to
different facts. On a refusal nothing was published, so the draft is the only
copy. On a publish reporting the op was not newly stored, this peer already held
that exact op, and a user whose intention was to publish something different
needs the text in front of them to edit — clearing would take away exactly what
they need.

The cost of clearing is named: a user writing a near-identical follow-up loses
their starting point and retypes it. That is a convenience, weighed against an
interface offering a control whose use publishes a duplicate that cannot be
removed.

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

**This outcome has become uncommon and SHALL NOT be dropped on that account.**
It previously arrived whenever one person submitted the same body twice, which
was the ordinary case. It now arrives only when this peer already holds the exact
op — every field matching, counter included — which is a re-publish rather than a
re-authoring. The outcome still reaches the view, core still reports it, and a
view that stopped handling it would present a state it cannot render. **A rarer
branch is a branch that is harder to notice breaking**, which is the reason to
keep its scenarios rather than a reason to trim them.

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

### Requirement: The submit control is disabled while a publish is outstanding

The view SHALL disable its submit control when a publish is submitted, and SHALL NOT re-enable it until that publish has reported an outcome.

**This obligation arrives with the op clock and is a consequence of it.** An op's bytes now carry a counter that advances between two publishes, so two submissions of one draft are two distinct ops rather than one. The deduplication that previously absorbed a double-tapped submit no longer does, and **core cannot restore it**: at that layer a double tap and a person deliberately posting the same line twice are identical acts producing identical-looking ops, correctly signed and correctly ordered. The view is the only layer at which the two are distinguishable, because only the view knows that one gesture occurred.

The failure this prevents is not cosmetic. A person who taps twice publishes twice, both posts appear, and **neither can be removed** — this system has no delete, and a revision replaces a post's content rather than withdrawing it. So the duplicate is permanent, visible to every peer, and the author's only remedy is to edit one into an apology.

The control SHALL be re-enabled on every outcome, including a refusal, because a refused publish stored nothing and the user must be able to retry.

Disabling SHALL be a property of the control rather than a message asking the user to wait. A prompt not to double-tap relies on the person reading it in the moment they are least likely to.

#### Scenario: The submit control is disabled once a publish is submitted

- **WHEN** a publish is submitted
- **THEN** the submit control is disabled

#### Scenario: A second activation during an outstanding publish submits nothing

- **WHEN** the submit control is activated twice in succession with no outcome reported between the two
- **THEN** exactly one publish is submitted to core

#### Scenario: The control is re-enabled when the publish succeeds

- **WHEN** a publish reports success
- **THEN** the submit control is enabled

#### Scenario: The control is re-enabled when the publish is refused

- **WHEN** a publish reports a refusal
- **THEN** the submit control is enabled
- **AND** the draft is retained, so the user can retry

### Requirement: A publish outcome is displayed only on the visit in which the publish was made

A **visit** to a screen is the span from the view rendering that screen in place
of a different main-area screen until it renders another main-area screen in
its place. Opening a Stoa from the list begins a visit to the feed, and so does
returning to the feed from a thread or from the moderation screen; opening a
thread begins a visit to the thread screen. Re-reading, paging or changing what
a screen lists while it stays rendered does not begin a new visit; a read retried
after a failure is a re-read, and asking for hidden content to be included or
excluded changes what a screen lists.

A **publish outcome** is any of the three messages a composer displays once a
publish reports: the message for a newly stored op, the message for an op that
was already published, and the message for a refusal.

A composer the screen is not rendering displays nothing, an outcome included.
The obligations below apply at every moment of a visit at which the composer is
rendered.

The view MUST NOT display a publish outcome on any visit other than the one in
which that publish was submitted. On a visit in which no publish has been
submitted from a composer, that composer MUST display no publish outcome
whenever it is rendered. This holds whether the later visit is for the same
Stoa or thread or a different one, and whatever state that visit's reads reach,
a failed read included, and after a failed read is retried and succeeds on that
visit.

Within the visit in which a publish was submitted, its outcome MUST be displayed
whenever that composer is rendered, until the visit ends or a later publish from
that composer reports its own outcome. It MUST NOT be withdrawn by the re-read
that follows a successful publish, whatever that re-read returns, nor by any
other re-read, paging or change to what the screen lists on that visit. Where a
read on that visit fails and the screen stops rendering the composer, the
outcome MUST be displayed again once a read on the same visit succeeds and the
composer is rendered again.

This requirement applies to every composer the view mounts: the feed's post
composer and the thread screen's reply composer.

It constrains the outcome message only. What becomes of a draft held when a
screen is left is contracted by *A draft belongs to the target it was entered
for* and *An unsubmitted draft is kept for its target while the view stays
open*, and is not decided by this requirement.

#### Scenario: A confirmation is gone after reopening the Stoa from the list

- **WHEN** a post is published from the feed and the publish reports the op was
  newly stored, the user returns to the Stoa list, and the user opens the same
  Stoa again
- **THEN** the feed's composer displays no publish outcome
- **AND** no text stating the content was saved on this machine is displayed

#### Scenario: A confirmation is gone after returning from a thread

- **WHEN** a post is published from the feed and the publish reports the op was
  newly stored, the user opens a thread from the feed, and the user returns to
  the feed
- **THEN** the feed's composer displays no publish outcome

#### Scenario: Every kind of outcome is gone on the next visit

- **WHEN** a publish from the feed reports the op was not newly stored, and
  separately a publish from the feed is refused, and after each the user returns
  to the Stoa list and opens the same Stoa again
- **THEN** in each case the feed's composer displays no publish outcome

#### Scenario: An outcome does not follow the user into another Stoa

- **WHEN** a post is published in one Stoa's feed and the publish reports the op
  was newly stored, the user returns to the Stoa list, and the user opens a
  different Stoa
- **THEN** that Stoa's feed displays no publish outcome

#### Scenario: A failed read on the later visit carries no outcome

- **WHEN** a post is published from the feed and the publish reports the op was
  newly stored, the user returns to the Stoa list, and the user opens the same
  Stoa again while the posting probe reports posting is possible and the feed
  read answers with the error shape
- **THEN** the feed is in its failed state
- **AND** the feed's composer displays no publish outcome

#### Scenario: A reply's outcome is gone on the next visit to the thread

- **WHEN** a reply is published from the thread screen and the publish reports
  the op was newly stored, the user returns to the feed, and the user opens the
  same thread again
- **THEN** the thread screen's composer displays no publish outcome

#### Scenario: The outcome stays for the rest of the visit that produced it

- **WHEN** a post is published from the feed, the publish reports the op was
  newly stored, and the re-read that follows returns a row for it
- **THEN** the message for a newly stored op is displayed

#### Scenario: A publish on the later visit displays its own outcome

- **WHEN** a post is published from the feed and reports the op was newly
  stored, the user leaves the feed and opens the same Stoa again, and a second
  post is published from the feed on that visit and is refused
- **THEN** the feed's composer displays the refusal
- **AND** it does not display the message for a newly stored op

#### Scenario: A confirmation is gone after returning from the moderation screen

- **WHEN** a post is published from the feed and the publish reports the op was
  newly stored, the user opens the moderation screen from the feed, and the user
  leaves it back to the feed
- **THEN** the feed's composer displays no publish outcome

#### Scenario: Every kind of reply outcome is gone on the next visit to the thread

- **WHEN** a reply published from the thread screen reports the op was not newly
  stored, and separately a reply published from the thread screen is refused,
  and after each the user returns to the feed and opens the same thread again
- **THEN** in each case the thread screen's composer displays no publish outcome
- **AND** no text from the outcome displayed on the earlier visit is displayed

#### Scenario: A reply's outcome does not follow the user into another thread

- **WHEN** a reply is published in one thread and the publish reports the op was
  newly stored, the user returns to the feed, and the user opens a different
  thread of the same Stoa
- **THEN** that thread's reply composer is rendered
- **AND** it displays no publish outcome

#### Scenario: A failed read on the later visit to the thread carries no outcome once it recovers

- **WHEN** a reply is published from the thread screen and the publish reports
  the op was newly stored, the user returns to the feed, the user opens the same
  thread again and its read answers with the error shape, and the user retries
  the read on that visit and it succeeds
- **THEN** the thread screen's reply composer is rendered
- **AND** it displays no publish outcome

#### Scenario: A reply's outcome is displayed again when a failed re-read recovers

- **WHEN** a reply is published from the thread screen, the publish reports the
  op was newly stored, the re-read that follows answers with the error shape, and
  the user retries the read on the same visit and it succeeds
- **THEN** the thread screen's reply composer is rendered
- **AND** it displays the message for a newly stored op

#### Scenario: Paging the feed does not withdraw the outcome

- **WHEN** a post is published from the feed, the publish reports the op was
  newly stored, the feed's read reports a further page, and the user moves to the
  next page and then back to the first
- **THEN** after each move the feed's composer displays the message for a newly
  stored op

#### Scenario: Changing what the feed lists does not withdraw the outcome

- **WHEN** a post is published from the feed, the publish reports the op was
  newly stored, and the user asks the feed to include hidden content, so that the
  feed is read again with hidden content included
- **THEN** the feed's composer displays the message for a newly stored op

#### Scenario: Changing what the thread lists does not withdraw the outcome

- **WHEN** a reply is published from the thread screen, the publish reports the
  op was newly stored, and the user asks the thread screen to include hidden
  content, so that the thread is read again with hidden content included
- **THEN** the thread screen's composer displays the message for a newly stored
  op

### Requirement: A draft belongs to the target it was entered for

A draft's **target** is what a publish of it would name: the Stoa address for a
post, and the Stoa address together with the parent op for a reply. A reply's
target is identified by its parent, not by its thread. Whenever a composer is
rendered, its field MUST hold only text entered for the target that composer
would publish to. Where no draft is held for that target, the field MUST be
empty.

#### Scenario: A draft typed in one Stoa is not in another Stoa's composer

- **WHEN** text is entered in one Stoa's post composer and not submitted, the
  user returns to the Stoa list, and the user opens a different Stoa
- **THEN** that Stoa's post composer is rendered
- **AND** its field is empty

#### Scenario: A reply typed in one thread is not in another thread of the same Stoa

- **WHEN** text is entered in one thread's reply composer and not submitted, the
  user returns to the feed, and the user opens a different thread of the same
  Stoa
- **THEN** that thread's reply composer is rendered
- **AND** its field is empty

#### Scenario: A reply typed in one thread is not in a thread of a different Stoa

- **WHEN** text is entered in one thread's reply composer and not submitted, and
  the user opens a thread in a different Stoa
- **THEN** that thread's reply composer is rendered
- **AND** its field is empty

#### Scenario: Two parents carrying the same op id in different Stoas do not share a draft

- **WHEN** text is entered in the reply composer of a thread in one Stoa and not
  submitted, and the user opens a thread in a different Stoa whose root carries
  the same op id
- **THEN** that thread's reply composer is rendered
- **AND** its field is empty

#### Scenario: A post draft and a reply draft in one Stoa are separate

- **WHEN** text is entered in a Stoa's post composer and not submitted, and the
  user opens a thread of that Stoa
- **THEN** the thread's reply composer is rendered
- **AND** its field is empty

#### Scenario: A reply draft is not in the post composer of its Stoa

- **WHEN** text is entered in a thread's reply composer and not submitted, and
  the user returns to that Stoa's feed
- **THEN** the feed's post composer is rendered
- **AND** its field is empty

### Requirement: Two targets that differ never share a draft

Two targets are the same target only where both are for a post or both for a
reply, they name the same Stoa address and, for a reply, they name the same
parent op. The view MUST NOT hold one draft for two targets that differ in any
of these, whatever characters the Stoa address or the parent op contain.

#### Scenario: Targets whose address and parent read alike when joined do not share a draft

- **WHEN** text is entered in a reply composer pointed at Stoa address `a:b`
  and parent op `c`, the composer is pointed at Stoa address `a` and parent op
  `b:c`, and it is then pointed at the first address and parent again, and the
  same is done with any other single character, or none, in place of `:`
- **THEN** while pointed at the second address and parent, the field is empty
- **AND** once pointed at the first again, the field holds the text entered

### Requirement: Text is submitted only to the target it was entered for

The view MUST NOT make a publish call whose Stoa address or parent op differs
from those of the target the body it carries was entered for. This is a
requirement on the publish calls the view makes, and it holds whatever any
field displays.

#### Scenario: Text typed in one Stoa is not published to another

- **WHEN** text is entered in one Stoa's post composer and not submitted, the
  user opens a different Stoa, and every submit affordance that Stoa's feed
  offers is acted on
- **THEN** no publish call is made whose body is the text entered in the first
  Stoa

#### Scenario: A post written in the second Stoa is published there with its own text

- **WHEN** text is entered in one Stoa's post composer and not submitted, the
  user opens a different Stoa, enters different text there, and submits it
- **THEN** exactly one publish call is made
- **AND** it names the second Stoa's address
- **AND** its body is the text entered in the second Stoa

#### Scenario: A draft returned to is published to its own Stoa

- **WHEN** text is entered in one Stoa's post composer and not submitted, the
  user opens a different Stoa, then opens the first Stoa again and submits
- **THEN** the publish call made names the first Stoa's address
- **AND** its body is the text entered in the first Stoa

#### Scenario: A reply typed in one thread is not published to another thread of the same Stoa

- **WHEN** text is entered in one thread's reply composer and not submitted, the
  user opens a different thread of the same Stoa, and every submit affordance
  that thread screen offers is acted on
- **THEN** no publish call is made whose body is the text entered in the first
  thread

#### Scenario: A reply typed in one thread is not published to a thread of a different Stoa

- **WHEN** text is entered in one thread's reply composer and not submitted, the
  user opens a thread in a different Stoa, and every submit affordance that
  thread screen offers is acted on
- **THEN** no publish call is made whose body is the text entered in the first
  thread

#### Scenario: A reply written in the second thread names that thread's parent and Stoa

- **WHEN** text is entered in one thread's reply composer and not submitted, the
  user opens a thread in a different Stoa, enters different text there, and
  submits it
- **THEN** exactly one publish call is made
- **AND** it names the second thread's Stoa address and its root's op id as the
  parent
- **AND** its body is the text entered in the second thread

#### Scenario: A reply draft returned to is published to its own parent

- **WHEN** text is entered in one thread's reply composer and not submitted, the
  user opens a different thread, then opens the first thread again and submits
- **THEN** the publish call made names the first thread's Stoa address and its
  root's op id as the parent
- **AND** its body is the text entered in the first thread

### Requirement: An unsubmitted draft is kept for its target while the view stays open

A draft no publish has cleared MUST stay held for its target for as long as the
view stays open, whichever screens are rendered meanwhile. Each time a composer
for that target is rendered, its field MUST hold that draft, character for
character as it was last edited. The view MUST NOT discard a held draft on
account of how many other targets hold one.

#### Scenario: A post draft is back when the same Stoa is reopened

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  returns to the Stoa list, and the user opens the same Stoa again
- **THEN** the post composer's field holds the text entered

#### Scenario: A post draft is back after a thread was opened from the feed

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  opens a thread from the feed, and the user returns to the feed
- **THEN** the post composer's field holds the text entered

#### Scenario: A post draft is back after the moderation screen was visited

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  opens the moderation screen from the feed, and the user leaves it back to the
  feed
- **THEN** the post composer's field holds the text entered

#### Scenario: A reply draft is back when the same thread is reopened

- **WHEN** text is entered in a thread's reply composer and not submitted, the
  user returns to the feed, and the user opens the same thread again
- **THEN** the reply composer's field holds the text entered

#### Scenario: A draft is back after other targets were visited and written in

- **WHEN** text is entered in one Stoa's post composer and not submitted, the
  user opens a different Stoa and enters different text there without
  submitting, and the user then opens each Stoa again in turn
- **THEN** each Stoa's post composer holds the text entered in that Stoa

#### Scenario: The draft held is the text as last edited

- **WHEN** text is entered in a Stoa's post composer, the user leaves and opens
  the same Stoa again, changes the text, leaves, and opens the same Stoa again
- **THEN** the post composer's field holds the changed text

#### Scenario: A draft the user emptied is not brought back

- **WHEN** text is entered in a Stoa's post composer, the user deletes all of
  it, leaves, and opens the same Stoa again
- **THEN** the post composer's field is empty

#### Scenario: A draft comes back with every character it had

- **WHEN** a draft containing multi-byte characters, bidirectional controls,
  zero-width characters and leading and trailing whitespace is entered and not
  submitted, and the user leaves and opens the same target again
- **THEN** the field's text is identical to the text entered

#### Scenario: Drafts for several targets are all held at once

- **WHEN** a different text is entered and not submitted in the post composers
  of three Stoas and in the reply composers of two threads, and each of the
  five is then opened again
- **THEN** each composer holds the text entered for its own target

#### Scenario: Drafts for six hundred targets are all held at once

- **WHEN** a post composer is pointed at six hundred Stoa addresses in turn and
  a different text is entered for each without submitting, and the composer is
  then pointed at each of the six hundred again
- **THEN** for each address the field holds the text entered for it

#### Scenario: A draft kept by a refusal is back without the refusal

- **WHEN** a post is submitted and refused, the user returns to the Stoa list,
  and the user opens the same Stoa again
- **THEN** the post composer's field holds the text that was submitted
- **AND** the composer displays no publish outcome

#### Scenario: A draft kept by an already-published outcome is back without that outcome

- **WHEN** a post is submitted and the publish reports the op was not newly
  stored, the user returns to the Stoa list, and the user opens the same Stoa
  again
- **THEN** the post composer's field holds the text that was submitted
- **AND** the composer displays no publish outcome

### Requirement: A publish clears only the draft of the target it named

Where a publish reports the op was newly stored, the draft cleared MUST be the
one held for the target that publish named, and a draft held for any other
target MUST be left as it was. A draft cleared this way MUST NOT be in the
field on any later visit to that target.

#### Scenario: A published draft is not brought back

- **WHEN** a post is submitted and the publish reports the op was newly stored,
  the user returns to the Stoa list, and the user opens the same Stoa again
- **THEN** the post composer's field is empty

#### Scenario: A publish in one Stoa leaves another Stoa's draft held

- **WHEN** text is entered in one Stoa's post composer and not submitted, the
  user opens a different Stoa and publishes a post there that reports the op was
  newly stored, and the user opens the first Stoa again
- **THEN** the first Stoa's post composer holds the text entered there

#### Scenario: A published reply leaves the Stoa's post draft held

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  opens a thread of that Stoa and publishes a reply that reports the op was
  newly stored, and the user returns to the feed
- **THEN** the post composer's field holds the text entered there

#### Scenario: A published reply leaves another thread's reply draft held

- **WHEN** text is entered in one thread's reply composer and not submitted, the
  user opens a different thread and publishes a reply there that reports the op
  was newly stored, and the user opens the first thread again
- **THEN** the first thread's reply composer holds the text entered there

#### Scenario: A publish that reports after its composer was pointed elsewhere clears the target it named

- **WHEN** a post composer holds an unsubmitted draft for each of two Stoas, the
  draft for the first is submitted, the composer is pointed at the second Stoa
  before the publish reports, and the publish then reports the op was newly
  stored
- **THEN** the publish call made names the first Stoa's address and carries the
  text entered for the first Stoa
- **AND** the field, pointed at the second Stoa, holds the text entered for the
  second Stoa
- **AND** once the composer is pointed at the first Stoa again, its field is
  empty

### Requirement: A draft whose composer is not rendered stays held and is displayed nowhere

While no composer for a draft's target is rendered, as when the posting probe
reports posting is not possible for its Stoa, the view MUST keep the draft held
and MUST NOT display its text anywhere. Once one is rendered again while the
view stays open, its field MUST hold the draft.

Whether a failed read stops a screen rendering its composer is not decided
here. While one has failed, a draft's text MUST NOT be displayed anywhere but in
the field of a composer rendered for its target.

#### Scenario: A draft behind a shut gate is not displayed

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  returns to the Stoa list, and the user opens the same Stoa again while the
  posting probe reports posting is not possible
- **THEN** no text input for composing is rendered
- **AND** the text entered is displayed nowhere on the screen

#### Scenario: A draft is back when the gate opens again

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  leaves and opens the same Stoa while the posting probe reports posting is not
  possible, and the user leaves and opens the same Stoa again while the probe
  reports posting is possible
- **THEN** the post composer's field holds the text entered

#### Scenario: A reply draft behind a shut gate is held and comes back

- **WHEN** text is entered in a thread's reply composer and not submitted, the
  user leaves and opens the same thread while the posting probe reports posting
  is not possible, and the user leaves and opens the same thread again while the
  probe reports posting is possible
- **THEN** on the visit with the gate shut, no text input for a reply is
  rendered and the text entered is displayed nowhere on the screen
- **AND** on the visit with the gate open, the reply composer's field holds the
  text entered

#### Scenario: A draft is back when a failed read recovers

- **WHEN** text is entered in a Stoa's post composer and not submitted, the user
  leaves and opens the same Stoa while the feed read answers with the error
  shape, and the user retries the read on that visit and it succeeds
- **THEN** while the feed is in its failed state, the text entered is displayed
  nowhere other than in the field of a rendered post composer
- **AND** once the retried read has succeeded, the post composer is rendered
- **AND** its field holds the text entered

#### Scenario: A reply draft is back when a failed thread read recovers

- **WHEN** text is entered in a thread's reply composer and not submitted, the
  user leaves and opens the same thread while the thread read answers with the
  error shape, and the user retries the read on that visit and it succeeds
- **THEN** while the thread screen is in its failed state, the text entered is
  displayed nowhere other than in the field of a rendered reply composer
- **AND** once the retried read has succeeded, the reply composer is rendered
- **AND** its field holds the text entered

### Requirement: An unsubmitted draft is held by the view alone and ends with it

The view MUST NOT pass draft text to the core module other than as the body of a
publish the user submitted. A view opened anew MUST hold no draft for any
target. Keeping a draft across a restart of the host is out of scope.

#### Scenario: An unsubmitted draft reaches no core call

- **WHEN** text is entered in a post composer and in a reply composer, neither
  is submitted, and the user moves between the Stoa list, feeds and threads
- **THEN** no call made to the core module carries either text

#### Scenario: A view opened anew holds no draft

- **WHEN** text is entered in a Stoa's post composer and not submitted, the view
  is closed, a view is opened anew, and the same Stoa is opened in it
- **THEN** the post composer's field is empty

### Requirement: A restored draft is not announced

The view MUST NOT display a message, marker or indicator stating that a draft
was kept, restored or carried over from an earlier visit. A composer whose field
holds a draft restored on a later visit MUST display what it displays when the
same text has just been typed into it on a first visit.

#### Scenario: A restored draft looks like the same text typed afresh

- **WHEN** text is entered in a Stoa's post composer, the user leaves and opens
  the same Stoa again, and separately the same text is typed into that Stoa's
  post composer on a first visit
- **THEN** the texts the composer displays are the same in both cases

#### Scenario: A restored draft carries the same warning as when it was typed

- **WHEN** a draft containing characters the display sanitiser would remove is
  entered and not submitted, and the user leaves and opens the same target again
- **THEN** the composer displays the warning naming how many such characters
  were found
- **AND** the composer displays no text it did not display before the user left

#### Scenario: A restored over-length draft is still withheld

- **WHEN** a draft whose UTF-8 encoding exceeds the limit is entered, and the
  user leaves and opens the same target again
- **THEN** the field holds the full text entered
- **AND** the view reports that it is too long
- **AND** the submit affordance is unavailable
