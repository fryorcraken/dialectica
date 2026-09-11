## Purpose

Defines what orders two ops in a Stoa, what a receiving peer records alongside an op in order to answer that, and what order it produces when the transport supplies no ordering metadata.

The complementary half of this contract lives in the `op-format` capability, whose requirement "An op carries no ordering field and no per-peer state" states that an op SHALL NOT carry a Lamport timestamp or a transport message id, and that the omission is enforced by the encoding's length. That requirement is not restated here: this capability governs what a peer records *alongside* an op, and `op-format` governs what an op may contain. Two specs asserting one rule is how two copies drift and the wrong one gets read.

## ADDED Requirements

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

- **WHEN** any two ops with distinct op ids are compared
- **THEN** exactly one of the two orders first
- **AND** the comparison is consistent however the two are presented

#### Scenario: The order is transitive

- **WHEN** one op orders before a second, and that second orders before a third
- **THEN** the first orders before the third

#### Scenario: Every peer computes the same order

- **WHEN** two peers hold the same ops with the same recorded transport metadata
- **THEN** both produce the same order
- **AND** neither consults its own clock, its arrival sequence, or any local state

### Requirement: Ops are deduplicated by op id before they are ordered

A caller SHALL deduplicate ops by op id before ordering them. The order is total over distinct op ids; two records of the same op id MAY compare as equal regardless of the transport metadata recorded against each.

This is a contract on callers rather than an implementation detail, which is why it is stated here. Ops are idempotent by op id, so a store holding one record per op id satisfies it by construction — but a caller ordering a list assembled before deduplication would not, and the consequence is the failure this capability exists to prevent: a tie leaves the relative order of two records to the sort's stability and the order they were assembled in, which is arrival order, which differs from peer to peer.

#### Scenario: Two records of the same op may tie

- **WHEN** two records carrying the same op id but different transport metadata are compared
- **THEN** the comparison may report neither as ordering first

#### Scenario: Distinct op ids never tie

- **WHEN** two ops with distinct op ids are compared, with any combination of recorded metadata
- **THEN** exactly one of the two orders first

### Requirement: Absent transport metadata is represented, never fabricated

A peer SHALL be able to record that the transport supplied no Lamport timestamp or no message id for an op. It SHALL NOT substitute a local clock reading, an arrival counter, or a default value for a metadata value it did not receive.

A substituted value is indistinguishable from a received one once recorded, so a peer that substitutes cannot later tell which of its ops are genuinely ordered. Two peers substituting different local values also order the same pair of ops differently, with nothing to detect it.

Recorded metadata SHALL NOT affect the op it is recorded against: the same op received twice with differing metadata SHALL yield the same op id both times. This is what makes metadata safe to accept from an untrusted transport, and it is the boundary between this capability and `op-format`, which owns the op's own contents.

#### Scenario: Two arrivals of the same op are the same op

- **WHEN** the same op is received twice with different ordering metadata
- **THEN** both yield the same op id

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

### Requirement: The Lamport timestamp alone decides whether an op is ordered

Where the transport supplies only part of its ordering metadata, the Lamport timestamp SHALL decide whether the op is ordered, and the message id SHALL NOT.

An op carrying a Lamport timestamp but no message id SHALL be treated as ordered by the transport and SHALL take its place by that timestamp. Where such an op ties with another on the timestamp, an op carrying a message id SHALL order before an op that carries none, and two ops carrying neither SHALL be separated by their op ids.

The two fields are independent at the contract level, so all four combinations are representable and each must have a defined answer. This particular combination is not reachable through any transport contract known today: the synchronisation layer requires a message id on every message, so a Lamport timestamp arriving without one describes no message it sends. It is specified because the type permits it, and an unspecified corner of a type is where the next reader's assumption goes.

Discarding a Lamport timestamp because the message id is missing would throw away the only value that orders, which is the one thing a recording peer must never do. Ordering an op the transport placed *after* ops it did not place would do the same by another route.

#### Scenario: An op with a Lamport timestamp and no message id is ordered

- **WHEN** an op carries a Lamport timestamp but no message id
- **THEN** it is reported as ordered by the transport
- **AND** it orders before every op carrying no Lamport timestamp

#### Scenario: Such an op is ordered by its timestamp against other ordered ops

- **WHEN** an op carrying a Lamport timestamp but no message id is compared with an op carrying a lower Lamport timestamp
- **THEN** the op with the higher timestamp orders first

#### Scenario: Within one Lamport value, a message id present leads one absent

- **WHEN** two ops share a Lamport timestamp and only one carries a message id
- **THEN** the op carrying a message id orders first

#### Scenario: Within one Lamport value, two ops with no message id are separated

- **WHEN** two ops share a Lamport timestamp and neither carries a message id
- **THEN** the op with the lower op id orders first
- **AND** the two do not compare as equal

### Requirement: A message id present without a Lamport timestamp does not order

An op whose metadata carries a message id but no Lamport timestamp SHALL be treated as unordered by the transport.

The message id is a tiebreak within one Lamport value, not an order in itself: message ids are assigned by hashing and carry no temporal meaning. Ordering by message id alone would produce a stable, total, and entirely arbitrary order that looks like a real one.

**This is the partial shape the transport actually produces**, and the reason this requirement is not a hypothetical corner. The synchronisation layer requires a message id on every message but sends ephemeral messages with the Lamport timestamp unset, so an ephemeral message carries exactly this combination. A peer that read the message id as an order would place such messages among ordered ops by hash value.

#### Scenario: A message id alone does not confer an order

- **WHEN** an op carries a message id but no Lamport timestamp
- **THEN** it orders below every op carrying a Lamport timestamp
- **AND** it is reported as unordered by the transport
