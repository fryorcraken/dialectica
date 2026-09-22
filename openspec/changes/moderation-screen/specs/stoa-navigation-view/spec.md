## MODIFIED Requirements

### Requirement: Every number rendered is one this peer can actually answer

The list and the join preview MUST NOT render a count of members, of total posts,
of peers, or of anything else no peer can observe. A count of what **this machine
holds** MAY be rendered as a placeholder while no call answers it, on the
conditions this requirement states, and MUST NOT be rendered as a measurement.

**Two separate reasons, and conflating them is how one of them gets argued
away.** A global count — members, total posts, how many peers are in a Stoa — is
forbidden **permanently**, because there is no membership list and no peer sees
the whole of a Stoa, so any such number would be invented rather than merely
unavailable. That half of this requirement is unchanged and is not relaxed by
anything below. A count of what *this machine* holds is legitimate to render and
would be welcome; it is simply not computed today.

**The second half is amended by owner decision, and this paragraph is the record
of that.** It previously forbade a held-post count outright, on the ground that
nothing computes one. The owner has decided that this MVP renders the design's row
counts anyway, so that the screen can be seen as designed. The reasoning that
produced the stricter form is not withdrawn and is restated below as the condition
the placeholder must meet; what changed is a scope decision about what ships,
which the owner owns, and not a finding that the original reasoning was wrong.

**No per-row count of held posts is available**, and that has not changed. A
listed item carries an address and a founding title, no call answers how many
posts this peer holds for a given Stoa, and the paginated thread listing reports
whether a further page exists rather than a total.

So where a row renders a count, it MUST be a **placeholder**, and three things
MUST hold:

- **It MUST be marked in the source as a placeholder**, naming what would supply
  the real value. An unmarked placeholder is indistinguishable from a number
  somebody believed, which is the failure the original prohibition was protecting
  against and is the one thing this amendment does not relax.
- **It MUST NOT be derived from any other call's reply.** In particular the
  length of a page from any other call MUST NOT be rendered in that position: a
  page length looks like a total, is not one, and would be wrong by an amount
  that grows with the Stoa — and, worse, would vary with the data, so a reader
  comparing two rows would be reading a real signal that means something other
  than what the row says. A placeholder that does not move is honest about being
  a placeholder in a way that a derived wrong number is not.
- **It MUST NOT be presented as this peer's measurement of the Stoa it labels.**
  A count is a fixture in this build; nothing about the rendering may assert
  otherwise.

**The scenario "A row renders no count of held posts" is narrowed rather than
dropped**, and keeping its name is deliberate. It required that a row carry no
held-post count at all; it now requires that a row carry no count *this peer
computed*, which is the half with a permanent reason. A reader comparing this
capability against the version that forbade the count outright finds the same
heading and sees which clause moved, where a deleted scenario would leave them
unable to tell an amendment from an oversight.

**What restores the stricter form: a core call that answers the number.** When
the contract answers how many posts this peer holds for a Stoa, the placeholder
is replaced by that answer and this amendment's permission is spent — a
placeholder surviving beside a call that could answer it is a defect, not a
phase. Until then this change's own `design.md` carries the entry in its
documented list of what core cannot yet serve, which is what makes the
placeholder documented rather than merely tolerated.

**An unread count is permitted on the same terms and for a different reason**,
and the distinction matters because the two are worked off differently. Unread is
excluded from the MVP by owner ruling rather than by a missing call: it needs
peer-local state that is not a projection of ops, which nothing in this design
has yet needed. So no core call will remove this entry by arriving; a decision to
build peer-local state is what removes it. A rendered unread count MUST meet every
condition above.

#### Scenario: A row renders no count of held posts

- **WHEN** the list renders a Stoa
- **THEN** any count on the row is a placeholder rather than a value this peer
  computed or read from a reply
- **AND** the row contains no count of members, peers, or anything else global

#### Scenario: A rendered row count is a placeholder, not a reply

- **WHEN** the list renders a Stoa and a count appears on its row
- **THEN** that count is not read from any reply the core gave
- **AND** it does not change when a listing for that Stoa returns a different
  number of items

#### Scenario: No other call's page length is rendered as a row's count

- **WHEN** the list is rendered while a thread listing for some Stoa has also
  been answered
- **THEN** no number from that listing appears in any row

#### Scenario: Two rows carrying placeholder counts do not claim to differ by measurement

- **WHEN** the list renders two Stoas for which this peer holds different amounts
  of content
- **THEN** nothing rendered attributes either row's count to a read this peer
  performed

#### Scenario: Nothing global is rendered

- **WHEN** the list renders a Stoa, and the join preview renders one
- **THEN** neither renders a count of members, of peers, or of the total posts in
  that Stoa
