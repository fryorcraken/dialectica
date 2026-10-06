# thread-view Specification

## Purpose
Defines what the thread screen renders given one page of a thread read: the
nesting it computes from a flat sequence, what it renders for an item whose
parent it does not hold, how a revised post and a withheld body are presented,
and which affordances are inert because no call backs them.

`thread-read` owns what the read returns, `composer-view` owns what the reply
composer does once a reply is submitted or refused, and `view-navigation` owns
how the screen is entered and left. None of them is restated here.
This capability contracts only the obligations that fall on the interface because
the read's answer is flat, partial and peer-derived, and something has to decide
what a reader is shown of that.

## Requirements

### Requirement: The screen renders the returned sequence and computes no order of its own

The view SHALL render the thread's items in the order the read returned them, and
SHALL NOT sort, reorder or re-rank them by any value it holds.

`thread-read`'s *An item carries its ordering position and the author's asserted
time, as two separate fields* states that the sequence a caller renders is the
sequence the read returned, that the position is not a sort key, and that a
caller SHALL NOT be able to rely on sorting by it. The position is an opaque
token: the view SHALL NOT parse it as a number, perform arithmetic on it, or
derive a distance, a gap or a count from two of them.

The view SHALL NOT order items by the asserted time it displays, for the reason
`feed-view`'s *Rows out of chronological order are correct* gives for the feed:
the time is the author's own claim and any author can set it.

Nesting is a separate question from ordering, and computing an indent SHALL NOT
change the sequence. Where an item is rendered indented beneath its parent, it
still occupies the place the returned sequence gives it.

#### Scenario: The rendered sequence is the returned sequence

- **WHEN** the screen is given a page of items in a given sequence
- **THEN** the items are rendered in that sequence
- **AND** the view computes no sequence of its own from the items' contents

#### Scenario: A sequence the view might have preferred differently is left alone

- **WHEN** the screen is given a page whose returned sequence disagrees with the
  order the items' positions would produce under the view's own natural
  comparison
- **THEN** the items are rendered in the returned sequence

#### Scenario: No arithmetic is performed on a position

- **WHEN** the screen is given items whose positions are tokens that are not
  decimal numerals
- **THEN** the items are rendered
- **AND** the screen is not in a failure state
- **AND** nothing is rendered reporting a distance, a gap or a count derived from
  two positions

### Requirement: Nesting is computed from the parent chain the view holds, and a depth it cannot establish is not invented

The view SHALL compute an item's indentation from the parent references the read
supplied, and SHALL NOT expect the read to report a depth. `thread-read`'s *The
items are a flat sequence in the system's order, with the root first* states that
the read reports no depth or indentation level, and that a view rendering the
replies nested is served by that shape.

**The root is the item reporting no parent, and it is not a missing-parent
case.** `thread-read`'s *Each returned post renders its current version and says
whether it was revised* requires the root to report no parent and every other
item to report one, which is what lets the view identify the root within a flat
page without comparing op ids. The view SHALL render the item reporting no parent
as the thread's root, at the outermost depth, and SHALL NOT render it as
answering a post not shown. Everything below about an unresolvable parent
concerns an item that **reports** a parent the view does not hold.

**An item whose reported parent is not among the items the view holds SHALL still
be rendered.** `thread-read`'s *Each returned post renders its current version and
says whether it was revised* states that a reported parent is an op id and not a
promise that the parent is among the items: it may fall on an earlier page, or be
absent from every page because it is hidden and hidden content was not asked for.
The view SHALL render such an item at a depth it can justify rather than
discarding it, and SHALL NOT omit it, SHALL NOT fail the read that returned it,
and SHALL NOT render it as though its parent were the root.

Where the view cannot establish an item's depth, what it renders SHALL indicate
that the item answers something not shown, rather than presenting the item as a
direct reply to the thread's root. An item silently re-parented to the root is
rendered as part of a conversation it was not part of, which is the same
misplacement `thread-read`'s *Membership is derived from the parent chain* exists
to prevent on the core side — a view that undoes it on the render side gives the
attacker the outcome the read denied.

The indentation the view applies SHALL be bounded, so that a chain of replies
longer than the screen is wide remains readable. Reaching that bound SHALL change
only the indent applied and SHALL NOT change which item is reported as an item's
parent, SHALL NOT omit an item, and SHALL NOT alter the sequence.

#### Scenario: The item reporting no parent is rendered as the root

- **WHEN** the screen is given a page whose first item reports no parent
- **THEN** that item is rendered at the outermost depth
- **AND** nothing rendered for it states that it answers a post not shown

#### Scenario: A reply is rendered beneath the parent it names

- **WHEN** the screen is given a root and a reply naming that root as its parent
- **THEN** the reply is rendered as nested beneath the root

#### Scenario: A reply to a reply is rendered deeper than its own parent

- **WHEN** the screen is given a root, a reply to it, and a reply to that reply
- **THEN** the deepest item is rendered at a greater depth than the item it names
  as its parent

#### Scenario: An item whose parent is not on the page is still rendered

- **WHEN** the screen is given a page containing an item whose parent op id
  matches no item on that page
- **THEN** that item's content is rendered
- **AND** the screen is not in a failure state
- **AND** the other items on that page are rendered

#### Scenario: An item whose parent is absent is not presented as a reply to the root

- **WHEN** the screen is given a page containing an item whose parent op id
  matches no item on that page
- **THEN** what is rendered indicates that it answers a post not shown
- **AND** it is not rendered as though the root were the post it answers

#### Scenario: A deep chain does not indent off the screen

- **WHEN** the screen is given a chain of replies deeper than the bound on
  indentation
- **THEN** every item of the chain is rendered
- **AND** no item is indented beyond that bound
- **AND** the sequence rendered is the returned sequence

### Requirement: A withheld body and an empty body are rendered differently

Where an item reports **no body**, the view SHALL render it as content withheld
rather than as a post whose text is empty, and SHALL NOT substitute a placeholder
that reads as the author's own text.

Where an item reports a body that **is** empty, the view SHALL NOT present it as
withheld.

`thread-read`'s *A hidden root is returned and marked, never silently dropped*
makes these two different facts and requires the read to keep them apart — a
withheld body is reported as absent "so that a caller cannot mistake withheld
content for content an author cleared", and an author who revised their post to
an empty body reports a body that is empty. The distinction survives the whole
core path and is destroyed at the last step if the view renders both as blank.

A withheld body SHALL be rendered as a moderation outcome rather than as a
failure of the read, an error, or a missing post. `thread-read` requires the
hidden root to be returned **marked**, and the mark is what the view renders.

#### Scenario: An item reporting no body renders as withheld

- **WHEN** the screen is given an item carrying no body and marked hidden
- **THEN** what is rendered states the content is withheld
- **AND** the screen is not in a failure state

#### Scenario: An item whose body is empty does not render as withheld

- **WHEN** the screen is given an item whose body is present and empty
- **THEN** what is rendered does not state that content was withheld

#### Scenario: The two are distinguishable on screen

- **WHEN** the screen is given one item carrying no body and one item whose body
  is present and empty
- **THEN** what is rendered for the two differs

### Requirement: A revised post is marked as revised, and the mark claims nothing about what changed

Where an item reports that the post has been revised, the view SHALL render an
indication that it was revised.

What is rendered SHALL NOT state or imply what the earlier text said, when it was
changed, or how many times. `thread-read`'s *Each returned post renders its
current version and says whether it was revised* supplies one fact — that the
current version is not the first — and supplies nothing else, so an interface
claiming more would be claiming something no call answered.

The view SHALL determine this from the field the item reports and SHALL NOT infer
it by comparing the post's identifier with its version identifier, or by
comparing content. `thread-read` requires the answer to be decided by which op the
current version is, because an author may revise a post to text identical to the
original.

Where an item reports it has **not** been revised, the view SHALL render no
revised marker.

#### Scenario: A revised item is marked

- **WHEN** the screen is given an item reporting it was revised
- **THEN** a revised indication is rendered for that item

#### Scenario: An unrevised item carries no marker

- **WHEN** the screen is given an item reporting it was not revised
- **THEN** no revised indication is rendered for that item

#### Scenario: The marker claims nothing about the earlier version

- **WHEN** the screen is given an item reporting it was revised
- **THEN** nothing rendered for that item states what the post previously said,
  when it was changed, or how many times

#### Scenario: The marker follows the reported field and not the identifiers

- **WHEN** the screen is given an item reporting it was not revised whose version
  identifier differs from its own identifier
- **THEN** no revised indication is rendered for that item

### Requirement: The earlier-versions affordance is inert, and inertness is the existing convention

Where the screen offers an affordance for reading a post's earlier versions, that
affordance SHALL be rendered **inert** — present but offering no action — and
SHALL NOT reach any core call.

**Inert carries the meaning `composer-view` already fixed for it**, in *A row
whose shape core did not guarantee degrades to what it can support*: present but
offering no action, rather than absent, and never acting on a substituted or
defaulted value. That capability reaches inertness from a different premise — a
row missing a field an affordance needs — and this one reaches it from a call
that does not exist. **The premise differs and the rendered meaning is the same,
which is why this states the one and reuses the other** rather than defining a
second notion of an inert control for a reader to tell apart from the first.

No method on the module surface reads a prior version: superseded versions stay
in the op log, and no contract method exposes them. The affordance is therefore
inert because the call does not exist, not because it was not wired.

The view SHALL NOT render prior version text it obtained by any other route, and
SHALL NOT present the current version as though it were an earlier one.

**This SHALL NOT be read as licence for an inert reply composer.** The reply
composer is backed by a call core serves, so it is wired; the two are treated
differently because the contract differs, not because one was harder.

#### Scenario: The earlier-versions affordance reaches no call

- **WHEN** the earlier-versions affordance is acted on
- **THEN** no core call is made

#### Scenario: No earlier version text is rendered

- **WHEN** the earlier-versions affordance is acted on
- **THEN** no text is rendered as being a previous version of the post

### Requirement: The reply composer is wired to the publish call and names the parent it is under

Where a reply affordance is offered for a post, submitting it SHALL make a reply
publish call naming that post as the parent.

**Whether one is offered at all is `composer-view`'s and not this
capability's.** Its *A compose affordance is rendered only when the probe says
posting is possible* requires the probe to be asked for the Stoa being composed
in and requires **no** text input where the probe says posting is not possible —
not a disabled one and not one that accepts text and refuses submission. This
requirement is therefore conditional on the gate being open, and SHALL NOT be
read as requiring a reply box the probe withholds.

What this screen supplies is the instantiation those requirements were written
for. `composer-view` notes that it "does not require a reply affordance
to be reachable", a screen listing thread heads having nowhere to put one; a
thread screen does, so the reply path becomes reachable here for the first time.

`composer-view` owns what happens next — the gate on the posting probe, the
draft's fate across the three outcomes, the byte-counted limit, the disabled
submit while a publish is outstanding, and that a published reply is not shown
until core has been read again. None of that is restated here, and this
requirement SHALL NOT be read as narrowing any of it.

What falls to this screen is the parent. The parent SHALL be the op id of the
post whose reply affordance was acted on, and SHALL NOT be the thread's root
identifier for a reply made to something other than the root. A composer that
sent the root regardless would publish every reply as a top-level answer, which
verifies, stores and renders in the wrong place permanently.

**A reply affordance on a post other than the root is out of scope**: the screen
offers one, for the thread's root. Where a later change offers one on another
post, the rule above names that post as the parent.

**No thread identifier SHALL be sent with a reply.** `content-authoring` refuses a
reply request carrying one, the thread being derived from the parent.

**Where the view holds no op id for a post, no reply call SHALL be made for it**,
and the reply affordance for that post SHALL be inert under the convention above.
`composer-view`'s *A row whose shape core did not guarantee* requires that no
value derived from a missing field reach a core call, and requires the guard
where the value is produced rather than at each use.

#### Scenario: A closed gate offers no reply box on this screen either

- **WHEN** a thread is rendered and the posting probe reports posting is not
  possible for that Stoa
- **THEN** no text input for a reply is rendered for any item of the thread

#### Scenario: A reply to the root names the root

- **WHEN** a reply is submitted from the affordance on the root post
- **THEN** the publish call names the root's op id as the parent

#### Scenario: No thread identifier is sent

- **WHEN** a reply is submitted
- **THEN** the request sent carries no thread identifier

#### Scenario: A post carrying no identifier offers no working reply affordance

- **WHEN** the screen is given an item carrying no op id and its reply affordance
  is acted on
- **THEN** no publish call is made
- **AND** no request is sent omitting the field that would have named the parent

### Requirement: An unreadable thread and a thread with no replies are different screens

Where the read is refused, the view SHALL render the message core supplied and
SHALL NOT render an empty thread.

Where the read succeeds returning only the root, the view SHALL render the root
and SHALL indicate that it holds no replies. It SHALL NOT render this as a
failure.

`thread-read`'s *A thread the peer does not hold is refused, and an empty thread
is served* makes these two different replies precisely so they can be rendered
differently, and states the cost of collapsing them: a reader shown an empty
thread when the thread was never received concludes a post vanished.

The view SHALL NOT reword core's message and SHALL NOT select what it renders by
matching on that message's text. `thread-read` contracts three distinguishable
refusals — not held, held but not a post, and a reply rather than a root — and
that distinction reaches the view only as differing prose inside one error shape,
with no machine-readable discriminant. `composer-view`'s *Every reply refusal is
presented as possibly temporary* establishes for the composer that a view must
not branch on message wording, and the same reasoning applies to a read: a view
branching on prose turns every improvement to that prose into a silent breaking
change.

It follows that the view SHALL NOT describe a refused read as permanent or as
the user's fault, since for the not-held case the thread may still arrive.

#### Scenario: A refused read shows core's message and no items

- **WHEN** a thread read is refused with a given message
- **THEN** that message is rendered as supplied
- **AND** no thread items are rendered

#### Scenario: A thread with only a root renders as a thread with no replies

- **WHEN** a thread read succeeds returning only the root item
- **THEN** the root is rendered
- **AND** the screen indicates it holds no replies
- **AND** the screen is not in a failure state

#### Scenario: The two states are not the same screen

- **WHEN** a read is refused, and a read succeeds returning only the root
- **THEN** the two render differently

#### Scenario: Three refusals render alike, no branch having been taken on the text

- **WHEN** a read is refused with the not-held message, with the not-a-post
  message, and with the reply-rather-than-a-root message
- **THEN** each message is rendered as supplied
- **AND** the view's behaviour is the same for all three

#### Scenario: No refusal is described as permanent or as the user's fault

- **WHEN** a thread read is refused
- **THEN** the text the view supplies alongside core's does not state that the
  user caused the failure or that the thread can never arrive

### Requirement: No score, tally or vote count is rendered on the thread screen

The view SHALL NOT render a number, bar, ratio or other magnitude purporting to
represent how a post in a thread was voted on.

No field of a thread item carries one. `thread-read` places any tally over votes
out of its scope, and `composer-view`'s *The vote control displays no score*
forbids a rendered score wherever a vote control appears — including here.

Voting is outside the MVP by a recorded owner decision, so the thread screen is
not required to offer a vote control at all. Where one is offered,
`composer-view` governs it and this requirement adds only that the thread screen
is not an exception to it.

#### Scenario: No score is rendered beside a thread item

- **WHEN** a thread is rendered
- **THEN** no number representing a score, tally or vote count is rendered for
  any item

#### Scenario: No ordering is offered as vote-based

- **WHEN** the thread screen is rendered
- **THEN** no control is offered that orders or re-orders the items by votes

### Requirement: Every core call the thread screen makes goes through the view's single call path

Every core call this screen makes SHALL go through the same call path as every
other core call in the view, so that a failure to reach core, a reply that is not
JSON, and a reply carrying core's error shape are each handled in one place
rather than once per call site.

A read SHALL NOT be reported as successful on a reply the view could not
interpret. A reply that is not the read's success shape SHALL be treated as a
refusal and SHALL reach the refused state, rather than rendering an empty thread
— which is the collapse the requirement above exists to prevent, arriving by a
different route.

`stoa-navigation-view`'s *Every core call on these screens goes through the
view's one call path* and `composer-view`'s *Every publish goes through the
view's single core call path* state this for the screens they own. It is restated
for this screen because the obligation is on each screen's calls, and a screen
added without it is a screen where an uninterpretable reply renders as content.

The items the view renders SHALL be those the read returned. The view SHALL NOT
insert an item on the strength of a publish having succeeded, per
`composer-view`'s *A published post is not shown until core has been read again*.

#### Scenario: An unreachable core is a refusal, not an empty thread

- **WHEN** the core module cannot be reached and a thread read is attempted
- **THEN** the screen is in its refused state
- **AND** it does not render a thread holding no replies

#### Scenario: A reply that is not JSON is a refusal

- **WHEN** a thread read returns text that is not JSON
- **THEN** the screen is in its refused state
- **AND** no items are rendered

#### Scenario: A success shape carrying no items is not rendered as content

- **WHEN** a thread read returns an object carrying neither items nor an error
- **THEN** the screen is in its refused state

#### Scenario: No item is added by a publish

- **WHEN** a reply publish succeeds and the subsequent read returns the items it
  returned before
- **THEN** the items rendered are exactly the items that read returned
- **AND** no item composed by the view appears among them

### Requirement: Every string rendered from an item is rendered as the read supplied it

The view SHALL render a post's body as the read returned it, and SHALL NOT
re-sanitise, normalise, unescape or otherwise alter it.

`thread-read`'s *Every string a thread read returns is sanitised, and the stored
op is not* makes sanitising core's job and requires the reply to report what
sanitising found. A second sanitiser in the view would be a second implementation
of a judgement that has already been made, and the two would disagree with
nothing in the system observing it.

Where the read reports that characters were removed or marked for an item, the
view SHALL be able to render that report, so that a reader is told text was
altered rather than shown altered text presented as the author's own. Marking
SHALL NOT be rendered as a correction: `thread-read` requires a character that
renders as another to be reported rather than replaced, because replacing it
would make a deceptive string into a plausible one.

The view SHALL NOT render an author's public key as though it were a name, and
SHALL NOT derive a display name or a mark by any rule other than the one
`generated-names` fixes. `thread-read`'s *An author is reported as a public key,
and never as a name* supplies the key as the input and leaves deriving the name
to whatever renders it.

#### Scenario: A body is rendered as returned

- **WHEN** the screen is given an item whose body the read reports as sanitised
- **THEN** the text rendered is the text the read supplied
- **AND** the view applies no further transformation to it

#### Scenario: A sanitiser report is renderable

- **WHEN** the screen is given an item whose read reports characters removed or
  marked
- **THEN** the screen can render that a change was made to the text displayed

#### Scenario: A marked character is not corrected

- **WHEN** the screen is given an item whose read reports a character as marked
- **THEN** that character is still present in the text rendered

### Requirement: The text around the reply composer says nothing about editing a reply or reading its earlier versions

Text the thread screen authors and renders with the reply composer MUST NOT
state whether a reply can be edited after it is published, whether a later
version of it can be published, or whether an earlier version of it can be
read: neither that it can nor that it cannot. The same holds for the text the
screen authors and renders in the composer's place where the posting gate is
shut.

This holds before a reply is published and after one is, since the text beside
the composer stays on screen across a publish.

**Two kinds of text rendered in those places are not authored by the screen**,
and this requirement does not bear on them:

- **Text core supplies and the view renders as supplied**: the posting probe's
  reason behind a shut gate, and core's message on a refused publish.
  `composer-view`'s *A closed gate shows the reason verbatim and offers a fix*
  and *A refused publish keeps the draft and names the refusal* forbid the view
  to reword either, and nothing here asks it to.
- **The draft the user typed**, which is the user's text.

**Editing a reply from the interface is out of scope.** No affordance on the
thread screen and no method on the module surface publishes a revision.
`post-revision` contracts how core resolves a revision it holds, which is a
different fact from the interface being able to author one, and nothing here
narrows it.

**Reading a prior version is absent** for the reason *The earlier-versions
affordance is inert, and inertness is the existing convention* gives: no method
on the module surface reads one. That requirement, and not this one, governs the
earlier-versions affordance it describes, which is offered on a post rather than
in the text around the reply composer.

**This requirement does not bear on the revised marker.** The marker reports a
fact the read returned about a post, and *A revised post is marked as revised,
and the mark claims nothing about what changed* governs it. What this
requirement forbids is a statement about what the user can or cannot do to or
with a reply, not a report of what an author did to a post.

#### Scenario: The open composer's text makes no edit or version statement

- **WHEN** a thread is rendered and the posting probe reports posting is
  possible for that Stoa
- **THEN** no text the screen authors and renders with the reply composer
  states whether a reply can be edited, whether a later version of it can be
  published, or whether an earlier version of it can be read

#### Scenario: No edit or version statement appears after a reply is published

- **WHEN** a reply is published from the reply composer and the thread is read
  again
- **THEN** no text the screen authors and renders with the reply composer
  states whether the reply can be edited, whether a later version of it can be
  published, or whether an earlier version of it can be read

#### Scenario: The shut gate's text makes no edit or version statement

- **WHEN** a thread is rendered and the posting probe reports posting is not
  possible for that Stoa
- **THEN** no text the screen authors and renders in place of the reply
  composer states whether a reply can be edited, whether a later version of it
  can be published, or whether an earlier version of it can be read

### Requirement: The open reply composer states that a reply is signed

Where the posting gate is open, the thread screen MUST render, with the reply
composer, text stating that a reply is signed. That text MUST remain rendered
after a reply is published from the composer.

The obligation is on the statement, not on a particular string: any wording
that states a reply is signed satisfies it.

#### Scenario: The open composer's text states that a reply is signed

- **WHEN** a thread is rendered and the posting probe reports posting is
  possible for that Stoa
- **THEN** text rendered with the reply composer states that a reply is signed

#### Scenario: The statement survives a publish

- **WHEN** a reply is published from the reply composer and the thread is read
  again
- **THEN** text rendered with the reply composer still states that a reply is
  signed
