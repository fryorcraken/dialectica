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
blockage, labelled with the bundle's `compose.fix` string, so that the reader is
shown the reason and a route to acting on it rather than the reason alone.

The view SHALL state that no compose box is shown and why, using the bundle's
`compose.apparatus` string, so the absence reads as a decision rather than as a
missing feature.

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

### Requirement: The composer never alters the text the user typed

The view SHALL publish the body exactly as the user entered it. It SHALL NOT
strip, replace, normalise, trim or reorder any character of the draft before
submitting it.

`content-authoring` requires text a caller supplies to reach the op unchanged,
because an op is signed over its bytes. A view that altered the draft would
publish, permanently and under the user's signature, something the user did not
write.

Where the draft contains characters the display-side sanitiser would remove or
mark when rendering it back — bidirectional overrides, zero-width characters, and
other invisibles — the view SHALL warn the author before they submit, naming how
many such characters were found. The warning SHALL NOT block submission.

The warning exists because the author is the only person who can still change the
text. A reader cannot consent to what they are shown, which is why peer text is
sanitised on display; an author can, which is why their own text is not.

#### Scenario: A draft is published byte-for-byte as typed

- **WHEN** a draft containing multi-byte characters, bidirectional controls and
  zero-width characters is submitted
- **THEN** the body sent to core is identical to the draft
- **AND** no character of it was removed or replaced

#### Scenario: An author is warned about invisible characters before submitting

- **WHEN** a draft contains characters the display sanitiser would remove or mark
- **THEN** the view displays a warning naming how many were found
- **AND** the submit affordance remains available

#### Scenario: A clean draft carries no warning

- **WHEN** a draft contains no such characters
- **THEN** no sanitiser warning is displayed

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

#### Scenario: A refusal leaves the draft intact

- **WHEN** a publish is refused for any reason
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
