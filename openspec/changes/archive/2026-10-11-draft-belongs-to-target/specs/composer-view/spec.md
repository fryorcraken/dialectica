# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
