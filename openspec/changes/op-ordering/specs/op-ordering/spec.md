## Purpose

Defines what orders two ops in a Stoa, what a receiving peer records alongside an op in order to answer that, and what order it produces when the transport supplies no ordering metadata.

## ADDED Requirements

### Requirement: Ordering metadata is recorded alongside an op, never inside it

An op SHALL NOT carry any ordering field. The values that order ops SHALL be recorded by the receiving peer as metadata attached to the op on arrival.

A value inside the signed bytes is chosen by the op's author, and the author is the party the ordering exists to constrain. An author who could assert their own Lamport timestamp could place a revision after any other version of their post, which is the whole of what the revision rule prevents.

#### Scenario: The ordering metadata is not part of the op

- **WHEN** an op's canonical encoding is produced
- **THEN** it contains no Lamport timestamp, no message id, and no sequence number
- **AND** the encoding's length is fully accounted for by the op's own fields

#### Scenario: Two arrivals of the same op are the same op

- **WHEN** the same op is received twice with different ordering metadata
- **THEN** both yield the same op id
- **AND** the op's content and signature are unaffected by the metadata

#### Scenario: Ordering metadata is not signed

- **WHEN** an op's signature is verified
- **THEN** verification does not consult the ordering metadata
- **AND** changing the ordering metadata does not invalidate the signature

### Requirement: Ops are ordered by the transport's order, not an order of our own

Ops SHALL be ordered by descending Lamport timestamp, with ties broken by ascending message id, using the values the transport assigned.

This is the order the synchronisation layer already maintains for the same messages. A peer SHALL NOT compute a Lamport timestamp of its own, and SHALL NOT maintain a second logical clock alongside the transport's.

Two orders over the same messages can disagree, and the disagreement produces no error: each peer stays internally consistent while rendering a thread differently from its neighbour. Deriving the order from the transport's own values is what makes that failure unreachable rather than merely unlikely.

#### Scenario: A higher Lamport timestamp is more recent

- **WHEN** two ops carry different Lamport timestamps
- **THEN** the op with the higher timestamp orders first
- **AND** the message ids do not affect the result

#### Scenario: Equal Lamport timestamps are broken by message id

- **WHEN** two ops carry equal Lamport timestamps and different message ids
- **THEN** the op with the lower message id orders first

#### Scenario: The order is total

- **WHEN** any two distinct ops with recorded metadata are compared
- **THEN** exactly one of the two orders first
- **AND** the comparison is consistent however the two are presented

#### Scenario: Every peer computes the same order

- **WHEN** two peers hold the same ops with the same recorded transport metadata
- **THEN** both produce the same order
- **AND** neither consults its own clock, its arrival sequence, or any local state

### Requirement: Absent transport metadata is represented, never fabricated

A peer SHALL be able to record that the transport supplied no Lamport timestamp or no message id for an op. It SHALL NOT substitute a local clock reading, an arrival counter, or a default value for a metadata value it did not receive.

A substituted value is indistinguishable from a received one once recorded, so a peer that substitutes cannot later tell which of its ops are genuinely ordered. Two peers substituting different local values also order the same pair of ops differently, with nothing to detect it.

#### Scenario: A missing Lamport timestamp is recorded as missing

- **WHEN** an op arrives with no Lamport timestamp
- **THEN** the recorded metadata reports the timestamp as absent
- **AND** no local clock reading is recorded in its place

#### Scenario: A missing message id is recorded as missing

- **WHEN** an op arrives with no message id
- **THEN** the recorded metadata reports the message id as absent

#### Scenario: Absence is distinguishable from a real value

- **WHEN** metadata with an absent Lamport timestamp is compared against metadata carrying one
- **THEN** the two are not equal
- **AND** the absent one is reported as unordered by the transport

### Requirement: An op the transport did not order sorts below every op it did

An op whose recorded metadata carries no Lamport timestamp SHALL order after every op whose metadata carries one, whatever the values involved. Two such ops SHALL be ordered relative to each other by ascending op id.

This is a defined degraded order, not the ordering rule. Its purpose is that the comparison is total and identical on every peer even when the transport tells a peer nothing, so that two peers holding the same ops never disagree. Its purpose is not to approximate the transport's order, which it cannot do.

Ordering by op id is chosen because the op id is a function of the op's own bytes: every peer holding the op computes the same one without consulting anything it received. An arrival order or a local timestamp would differ per peer, which is the failure this requirement exists to prevent.

#### Scenario: An unordered op sorts below an ordered one

- **WHEN** an op with no Lamport timestamp is compared against an op that has one
- **THEN** the op with the timestamp orders first
- **AND** this holds regardless of how low that timestamp is

#### Scenario: Two unordered ops are ordered by op id

- **WHEN** two ops both lacking a Lamport timestamp are compared
- **THEN** the op with the lower op id orders first

#### Scenario: The degraded order is still total and peer-independent

- **WHEN** two peers hold the same ops, none of which carry a Lamport timestamp
- **THEN** both produce the same order

#### Scenario: A caller can tell a degraded order from an ordered one

- **WHEN** recorded metadata is inspected
- **THEN** whether the transport ordered the op is reported
- **AND** a caller can act on that without inferring it from the ordering result

### Requirement: A message id present without a Lamport timestamp does not order

An op whose metadata carries a message id but no Lamport timestamp SHALL be treated as unordered by the transport.

The message id is a tiebreak within one Lamport value, not an order in itself: message ids are assigned by hashing and carry no temporal meaning. Ordering by message id alone would produce a stable, total, and entirely arbitrary order that looks like a real one.

#### Scenario: A message id alone does not confer an order

- **WHEN** an op carries a message id but no Lamport timestamp
- **THEN** it orders below every op carrying a Lamport timestamp
- **AND** it is reported as unordered by the transport
