## MODIFIED Requirements

### Requirement: An op carries no ordering field and no per-peer state

An op SHALL carry exactly two values that its author chooses freely and that bear on ordering — the Lamport counter and the wall-clock contracted above — and SHALL carry no others.

An op SHALL NOT carry a transport message id, a sequence number, a session counter, an arrival counter, a device or installation identifier, or any other value that varies with an individual peer's history.

An op SHALL NOT carry the transport's sender identifier. That identifier binds at channel creation as a transport self-filter and is not an author identity; the author identity in an op is the key it carries and the address derived from it.

This omission SHALL be enforced by the encoding rather than merely documented: the encoding's length is fully accounted for by the fields that are present, so a further field cannot be added without the encoding's shape visibly changing.

#### Scenario: The encoding's length accounts for every field present

- **WHEN** an op is encoded
- **THEN** its length equals exactly the sum of the version, the kind, the Stoa address, the author key, the Lamport counter, the wall-clock, and that kind's own fields
- **AND** a message id, a sequence number or a sender identifier could not be added without this changing

#### Scenario: An op carries no transport message id

- **WHEN** an op is encoded
- **THEN** no transport message id is among its fields
- **AND** the two admitted values are the counter and the wall-clock and nothing else

#### Scenario: Two peers encoding one op agree on its bytes

- **WHEN** the same op, with the same counter and wall-clock, is encoded by two peers with different local histories
- **THEN** both produce identical bytes
- **AND** both compute the same id

#### Scenario: No peer-local value reaches the encoding

- **WHEN** one op is encoded twice on one peer, with the peer's clock and op log changed between the two encodings
- **THEN** both encodings are identical

#### Scenario: No ordering field participates in the id

- **WHEN** the same op, with the same counter and wall-clock, is encoded by two peers whose own clocks and op logs differ
- **THEN** both produce identical bytes
- **AND** both compute the same id
- **AND** no value derived from either peer's own history took part

**This scenario is retained under its original name and its meaning has narrowed.** It previously asserted that no ordering field existed to participate in the id. Two now do, and they participate deliberately — the counter and the wall-clock are inside the preimage and change the id, which is what makes them unforgeable by a relay. What it now asserts is the property that actually mattered and that still holds: **the id is a function of the op alone.** Every value reaching the encoding travels in the op, so two peers encoding one op agree, and nothing either peer knows about its own history can change an op's identity.

### Requirement: An implausible wall-clock is accepted at the boundary, never refused

A peer SHALL NOT refuse, drop, or decline to store an op on account of its wall-clock value, however implausible. Every representable value SHALL be accepted.

The **wall-clock field** is never the reason an op is refused. The receive window reads the counter alone. An op whose counter is admissible is stored whatever its wall-clock says.

The cost of accepting is real and is stated rather than hidden: **a reader may be shown a time that is not when the op was written.** That cost is bounded by display clamping, which `op-ordering` places on the read path, and is bounded absolutely by the fact that nothing decides anything on this value.

**This is where the alternative was rejected and why it must not be re-derived.** Clamping at the boundary — writing a corrected value into the stored op — is not available at any price: the op is signed and the wall-clock is inside the preimage, so a peer that rewrote it would hold an op that no longer verifies and whose id no longer matches. Clamping is therefore a read-time presentation rule and can be nothing else.

#### Scenario: A far-future wall-clock does not prevent storage

- **WHEN** an otherwise valid op, whose counter is within `op-ordering`'s receive window, carrying a wall-clock centuries in the future arrives
- **THEN** it is accepted and stored
- **AND** the refusal reasons the decoder reports do not include its wall-clock

#### Scenario: A far-past and a zero wall-clock are equally accepted

- **WHEN** otherwise valid ops carrying a zero wall-clock and one far in the past arrive
- **THEN** each is accepted and stored

#### Scenario: A stored op's wall-clock is the one its author signed

- **WHEN** an op carrying an implausible wall-clock is stored and read back
- **THEN** the value read back is the value the author signed
- **AND** the op still verifies, so nothing rewrote it
