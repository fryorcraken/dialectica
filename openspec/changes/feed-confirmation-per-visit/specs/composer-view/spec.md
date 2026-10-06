## ADDED Requirements

### Requirement: A publish outcome is displayed only on the visit in which the publish was made

A **visit** to a screen is the span from the view rendering that screen in place
of a different main-area screen until it renders another main-area screen in
its place. Opening a Stoa from the list begins a visit to the feed, and so does
returning to the feed from a thread; opening a thread begins a visit to the
thread screen. Re-reading, paging or changing what a screen lists while it stays
rendered does not begin a new visit.

A **publish outcome** is any of the three messages a composer displays once a
publish reports: the message for a newly stored op, the message for an op that
was already published, and the message for a refusal.

The view MUST NOT display a publish outcome on any visit other than the one in
which that publish was submitted. On a visit in which no publish has been
submitted from a composer, that composer MUST display no publish outcome. This
holds whether the later visit is for the same Stoa or thread or a different one,
and whatever state that visit's read reaches, a failed read included.

Within the visit in which a publish was submitted, its outcome MUST remain
displayed across the re-read that follows a successful publish, whatever that
re-read returns.

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
