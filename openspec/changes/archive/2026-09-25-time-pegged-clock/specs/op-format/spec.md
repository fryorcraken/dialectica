## MODIFIED Requirements

### Requirement: An op carries no ordering field and no per-peer state

An op SHALL carry exactly two values that its author chooses freely and that bear on ordering — the Lamport counter and the wall-clock contracted above — and SHALL carry no others.

An op SHALL NOT carry a transport message id, a sequence number, a session counter, an arrival counter, a device or installation identifier, or any other value that varies with an individual peer's history.

An op SHALL NOT carry the transport's sender identifier. That identifier binds at channel creation as a transport self-filter and is not an author identity; the author identity in an op is the key it carries and the address derived from it.

This omission SHALL be enforced by the encoding rather than merely documented: the encoding's length is fully accounted for by the fields that are present, so a further field cannot be added without the encoding's shape visibly changing.

**Why the two admitted fields are admitted, when this requirement previously forbade both.** The prior text refused a self-asserted Lamport value because it "would be forgeable by exactly the author it is meant to order", and refused a wall clock because "a wall clock is a field the adversary sets". Both observations are true and neither has been withdrawn. What changed is that the transport does not supply the alternative and is not going to: no Lamport value reaches this system, so the practical effect of the prohibition was not a transport-assigned order but **no order at all**, with every resolver falling to a hash. The two objections are answered rather than set aside, and each is answered somewhere a test can reach:

- **The forgeable counter** is bounded by `op-ordering`'s receive window, which refuses an op whose counter is more than one hour ahead of the receiving peer's own time. An author can lead honest ops by at most that hour, and cannot place an op beyond it at all.
- **The adversary-set wall clock** is not bounded into safety; it is removed from every decision. It orders nothing, breaks no tie, and gates nothing. The window reads the counter and never this field, so there is no decision for an adversary's value to reach.

**Why the two remain two fields, now that both carry a time.** The counter is pegged to its author's clock, so it is a claim about the time just as the wall-clock is. This requirement previously separated them by saying a counter is meaningful only relative to ops a peer has seen, while a wall-clock is an absolute claim about the world that a peer has nothing to check against. That distinction no longer holds and is withdrawn. What separates the two now is what each is held to. The counter is checked from above against the receiving peer's own time, and is raised past every counter its author held, so it may order. The wall-clock is checked by nothing, so it may not.

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

**This rule is now narrower than the principle it was written from, and the difference is deliberate.** It was written as one instance of a wider rule, that no op is refused for a field value. Its reason was that refusing an op for a bad clock is a censorship vector: a peer whose system clock is wrong (skewed, unset after a battery failure, or misconfigured) would have every op it publishes dropped by every conforming peer, silently and everywhere at once, with no error path by which the author learns of it. **That wider rule is withdrawn**, and the reason now describes a cost this system accepts. `op-ordering` refuses an op whose *counter* is more than one hour ahead of the receiving peer's time, and an honest peer signs its current time into the counter. So a peer whose clock runs more than an hour fast has its ops refused exactly as described. `op-ordering` states that cost and why it is accepted.

What this requirement still guarantees is that the **wall-clock field** is never the reason. The receive window reads the counter alone. An op whose counter is admissible is stored whatever its wall-clock says, so the field remains a display value on which nothing is decided.

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
