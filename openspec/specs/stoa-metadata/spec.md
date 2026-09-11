# stoa-metadata Specification

## Requirements

### Requirement: A Stoa's current metadata is distinct from its founding values

A Stoa SHALL have two separately answerable descriptions of itself: the
**founding** values fixed in its genesis record, and the **current** display
metadata carried by a moderator-signed op.

The genesis title is inside the address preimage, so it can never change
without minting a different Stoa — that is what makes an address
self-authenticating. The metadata op carries what the Stoa is called *today*.
Both SHALL remain readable; the current value SHALL NOT overwrite or obscure
the founding one.

Current metadata SHALL comprise a display title and a description. The
description has no genesis counterpart, which is part of why this is its own
concept rather than a mechanism for overriding genesis values one-for-one: the
genesis record carries the minimum needed to *identify* a Stoa, and this
carries what a reader needs to *render* one.

The encoding of the op that carries this is the `op-format` capability's.

#### Scenario: Renaming a Stoa does not change its address

- **WHEN** a Stoa's metadata declares a title different from its genesis title
- **THEN** the Stoa's address, computed from the genesis record, is unchanged

#### Scenario: The founding title remains readable after a rename

- **WHEN** a metadata op declares a new title
- **THEN** the genesis record still decodes to its founding title
- **AND** the two titles are separately available

### Requirement: Current metadata resolves by last-write-wins, falling back to genesis

A reader SHALL prefer the most recent valid metadata op for a Stoa, and SHALL
fall back to the genesis record's founding values when it holds none.

"Most recent" SHALL be decided by the order the transport establishes, as the
`op-ordering` capability defines it. This capability SHALL NOT define its own
ordering, and a metadata op SHALL NOT assert its own position in one: a
self-asserted ordering value is forgeable by exactly the author it is meant to
order, which for a moderator-owned field is the difference between "the latest
rename wins" and "whoever claims the highest number wins".

Where the transport supplied no ordering metadata, resolution SHALL use the
degraded order `op-ordering` defines rather than inventing one. Two metadata ops
that cannot be ordered by the transport still resolve identically on every peer.

**Resolution is not implemented here.** Metadata ops accumulate and nothing
reads them yet. This requirement fixes the rule so that whoever implements
resolution does not have to re-derive it; it does not claim the behaviour
exists.

#### Scenario: A Stoa with no metadata op shows its founding values

- **WHEN** a reader holds a Stoa's genesis record and no metadata op for it
- **THEN** the Stoa's displayed title is the founding title

### Requirement: Current metadata does not include the posting policy

The posting policy SHALL NOT be part of a Stoa's mutable current metadata. It is
declared at creation, in the genesis record, and is immutable.

A rename is cosmetic; a policy change is authorisation. Three things rule the
field out rather than merely deferring it by taste:

- **The fallback rule inverts safely for a title and unsafely for a policy.**
  The rule above falls back to the genesis value when no op has been seen.
  Applied to a policy, a peer that missed a *tightening* would fall back to the
  *looser* founding policy — admitting posters the Stoa has since excluded.
  That is the same silent widening `stoa-genesis` refuses when it declines to
  default an unrecognised policy discriminant, reached by a different route: a
  design that refuses to default a policy on decode and then hands one back on
  resolve has closed only the front door.
- A policy change is retroactive in a way a rename is not: it alters who may
  post, for everyone, including over content already published.
- The policy has exactly one accepted value, so a metadata op carrying one could
  not express a change. Behaviour that cannot be varied SHALL NOT be specified
  as varying.

Excluding it SHALL NOT cost an encoding version to add later. A future
policy-changing act SHALL be expressible as a new op kind taking an unused
discriminant, which changes no existing op's id and no Stoa's address.

Adding it SHALL require all three of: a second accepted policy value, so a
change is expressible and testable; a settled ordering, so "the current policy"
is determinable; and a policy fallback rule specified separately from the
display one and **fail-closed** — a peer that cannot establish the current
policy treats the Stoa as more restrictive than its founding value, or declines
to post, rather than guessing.

#### Scenario: A policy change cannot be expressed as current metadata

- **WHEN** a Stoa's current metadata is set
- **THEN** no posting policy is among the values it can carry
- **AND** the genesis record remains the only source of the Stoa's policy

### Requirement: A displayed title is never an identifier

A Stoa's displayed title SHALL NOT be treated as identifying it. Two Stoas may
carry identical titles, and a title is chosen freely by whoever signed the op.

Display text reaches a reader exactly as its author wrote it — the encoding
validates UTF-8 and deliberately applies no normalisation, because normalising
would break the canonicality every peer's agreement on op ids depends on. A
title may therefore contain bidirectional controls, zero-width characters, or
homoglyphs of an established Stoa's name, and while no authority check exists
any peer may sign such an op for any Stoa.

Whoever renders a title SHALL therefore mitigate at the point of display:
strip or visibly mark bidi and zero-width controls, show the Stoa address
alongside any name, and never resolve or match a Stoa by title. The address is
the identity.

#### Scenario: Two Stoas may share a displayed title

- **WHEN** two distinct Stoas declare the same current title
- **THEN** both remain distinct Stoas with distinct addresses
- **AND** neither is treated as the other

#### Scenario: Display text is preserved rather than sanitised in the data layer

- **WHEN** a metadata op carries a title containing control or zero-width characters
- **THEN** decoding preserves them unchanged
- **AND** the obligation to render them safely rests with the renderer
