## MODIFIED Requirements

### Requirement: The deciding moderation is named, not merely counted

Where a target is subject to a binding moderation, the reader SHALL be able to identify the op that decided it, including its author and its action.

A moderator acting on a post is acting on a judgement they can name, in the same way `post-revision`'s requirement "A post is never edited in place" keeps every superseded version so that a moderator acting on a post acts on a version they can name by its own op id. A bare boolean cannot support a moderator reversing a specific hide, a reader being shown who hid something, or an interface distinguishing "hidden by this Stoa's moderator" from any other reason content is absent.

#### Scenario: A hidden target names the op that hid it

- **WHEN** a target is reported as hidden
- **THEN** the moderation op that decided it is identified
- **AND** its author and action are available

#### Scenario: An unhidden target names the unhide that lifted it

- **WHEN** a hide is followed by a binding unhide
- **THEN** the target is reported as not hidden
- **AND** the unhide is identified as the deciding op, distinguishing it from a target nobody ever moderated
