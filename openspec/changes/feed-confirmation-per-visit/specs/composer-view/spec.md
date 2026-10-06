## ADDED Requirements

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

It constrains the outcome message only. Whether a draft held when a screen is
left is still held on a later visit to that screen is not decided by this
requirement.

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
