## ADDED Requirements

### Requirement: Where one listed Stoa ends and the next begins is rendered, not left to spacing

Every row of the Stoa list MUST be visually separated from what follows it by a
rendered boundary, and that boundary MUST be present on every row including the
last.

This is the same concern as "A listed Stoa is rendered with its address, never
with its title alone", read one level out. That requirement makes each row carry
the half an attacker does not control; this one makes it unambiguous **which row
a given half belongs to**. Two Stoas may carry identical founding titles — the
capability already contracts that case and requires both rows to render — and a
row carries a title, an address, and up to two buttons stacked and side by side.
Where those groups divide is therefore load-bearing: a reader who attributes one
row's address to the next row's title has been misled about which Stoa they are
about to open, and nothing in the content itself corrects them. Whitespace alone
does not carry that division, because the gap between two rows and the gap
between a title and its own address are both just gap.

**The last row is included deliberately, and that is the clause worth stating.**
Dropping the boundary on the final row is the common convention and it is wrong
here: the list is followed on this screen by further affordances — pasting an
address, creating a Stoa — and an unterminated final row reads as continuing into
them. The list must read as a bounded block.

**What this does NOT require.** It does not fix the boundary's thickness, colour,
or the element that draws it. The capability's Purpose puts the visual system —
"colours, type, metrics, the mark" — outside itself by name, and this requirement
holds to that: it contracts that a boundary is rendered per row, not what it
looks like. A future treatment that divides rows some other way satisfies it, and
is meant to.

#### Scenario: Each rendered row is separated from the next

- **WHEN** the listing returns two or more Stoas this peer is in
- **THEN** a boundary is rendered for each row
- **AND** the number of boundaries equals the number of rows rendered

#### Scenario: The final row is terminated too

- **WHEN** the listing returns one or more Stoas and the last row is rendered
- **THEN** a boundary is rendered below the last row, as below every other row

#### Scenario: A list with no rows renders no boundaries

- **WHEN** the membership listing succeeds and returns no Stoas
- **THEN** no row is rendered
- **AND** no boundary is rendered, so the empty state carries no rule belonging
  to a row that is not there
