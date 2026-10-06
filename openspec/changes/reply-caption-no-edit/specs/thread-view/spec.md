## ADDED Requirements

### Requirement: The text around the reply composer does not promise that a reply can be edited, or that its earlier versions can be read

Text the thread screen renders with the reply composer MUST NOT state that a
reply can be edited after it is published, that a later version of it can be
published, or that an earlier version of it can be read. The same holds for the
text the screen renders in the composer's place where the posting gate is shut.

This holds before a reply is published and after one is, since the text beside
the composer stays on screen across a publish.

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
requirement forbids is a claim about what the user can do to or with a reply,
not a report of what an author did to a post.

#### Scenario: The open composer's text makes no edit or version claim

- **WHEN** a thread is rendered and the posting probe reports posting is
  possible for that Stoa
- **THEN** no text rendered with the reply composer states that a reply can be
  edited, that a later version of it can be published, or that an earlier
  version of it can be read

#### Scenario: No edit or version claim appears after a reply is published

- **WHEN** a reply is published from the reply composer and the thread is read
  again
- **THEN** no text rendered with the reply composer states that the reply can be
  edited, that a later version of it can be published, or that an earlier
  version of it can be read

#### Scenario: The shut gate's text makes no edit or version claim

- **WHEN** a thread is rendered and the posting probe reports posting is not
  possible for that Stoa
- **THEN** no text rendered in place of the reply composer states that a reply
  can be edited, that a later version of it can be published, or that an
  earlier version of it can be read
