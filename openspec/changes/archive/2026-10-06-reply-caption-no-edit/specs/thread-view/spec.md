## ADDED Requirements

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
