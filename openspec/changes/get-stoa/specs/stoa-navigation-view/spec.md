## MODIFIED Requirements

### Requirement: No current title is rendered until one has been resolved

The preview MUST NOT render a current title, a present name, or any title
attributed to a moderator, while nothing supplies one.

A current title is carried by a moderator-signed metadata op, and only
`stoa-metadata`'s resolution supplies one. The requirement there is "Current
metadata resolves by last-write-wins, falling back to genesis", and its reply
says whether it fell back. A founding title, which is what `stoa-membership`
reports, is never a current title. Rendering the founding title under a
"current title" caption would therefore assert that a moderator has not renamed
this Stoa. A screen that has not resolved current metadata has not checked
that, and it is false for every Stoa that has been renamed.

This is a prohibition on claiming, not on the layout. Reserving the position a
current title will occupy is left open, and is the reason the distinction is
worth carrying now; what a screen MUST NOT do is fill that position with the
founding value or with any placeholder that reads as a resolved name.

#### Scenario: The founding title is not also rendered as a current title

- **WHEN** the preview renders a Stoa whose only available title is the founding
  one
- **THEN** the founding title is rendered once, labelled as founding
- **AND** nothing rendered attributes a title to a moderator or presents one as
  the Stoa's present name

#### Scenario: A resolved current title is what fills that position, when one exists

- **WHEN** the preview is given a resolved current title alongside the founding
  one, and the two differ
- **THEN** both are rendered
- **AND** each is labelled as the value it is, so the reader can tell which was
  fixed at founding
