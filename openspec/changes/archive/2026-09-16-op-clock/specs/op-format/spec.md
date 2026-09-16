## ADDED Requirements

### Requirement: An op carries an application Lamport counter inside its signed bytes

An op SHALL carry a Lamport counter, and that counter SHALL be inside the bytes the signature covers and inside the bytes the op id is computed from.

The counter SHALL be a fixed-width unsigned integer of at least 64 bits, encoded at a fixed offset so that its presence is accounted for by the encoding's length. It SHALL NOT be optional, and it SHALL NOT be encoded as a variable-length or presence-tagged field: an op of this version either carries a counter or is not an op of this version.

**Being inside the signed preimage is the whole reason for placing it here rather than in an envelope.** A relay forwards the bytes it was given; it cannot raise an op's counter to promote it, cannot lower one to bury it, and cannot strip the field to force an op onto the degraded path. Any of those alters the preimage, so the signature fails and the op is refused. A value outside the signature would have none of that, and there is no transport envelope in this system a peer may trust — `op-transport` requires that the payload be exactly one op's wire form with no framing of this capability's own.

**Stripping is refused because the version travels with the bytes, and this is worth stating because the migration path looks like a hole in it.** An op declaring this version and carrying no counter is malformed and is refused as such; an op declaring the *earlier* version legitimately carries none and is accepted. A relay cannot turn the first into the second, because the version discriminant is itself inside the preimage — rewriting it breaks the signature exactly as rewriting the counter does. So "some ops carry no counter" is not a downgrade an attacker can reach, only a fact about ops signed before the field existed.

**What the signature does not defend against is the signing author**, and this capability does not claim it does. An author chooses the counter they sign. The bound on that choice is `op-ordering`'s, which is where the value is read; this capability contracts only that the value is carried, committed to, and canonical.

#### Scenario: The counter is inside the signed preimage

- **WHEN** a signed op has its counter changed and the original signature reattached
- **THEN** verification fails
- **AND** the original op still verifies, so the failure is not because both are broken

#### Scenario: The counter participates in the op id

- **WHEN** two ops are identical in every field except the counter
- **THEN** their encodings differ
- **AND** their ids differ

#### Scenario: The counter round-trips at every boundary value

- **WHEN** an op carrying a counter of zero, one, the maximum representable value, and one below the maximum is encoded and decoded
- **THEN** each decodes to the value it was given
- **AND** none is confused with any other

#### Scenario: The counter is not an optional field

- **WHEN** an op of this version is encoded
- **THEN** the counter occupies a fixed width at a fixed offset
- **AND** there is no presence tag by which it could be absent

#### Scenario: An op of this version carrying no counter is refused

- **WHEN** an op declaring this version has its counter bytes removed
- **THEN** decoding fails
- **AND** it is not read as an op of the earlier version

#### Scenario: A downgrade to the earlier version does not verify

- **WHEN** a signed op of this version has its version discriminant rewritten to the earlier one and its original signature reattached
- **THEN** verification fails
- **AND** the original op still verifies, so the failure is the rewrite rather than a broken fixture

### Requirement: An op carries an author-asserted wall-clock that is never an ordering value

An op SHALL carry a wall-clock instant the author asserts, inside the same signed bytes. It SHALL be a fixed-width value denoting milliseconds since the Unix epoch.

**This field is untrusted, and the contract states that rather than implying it.** The author chooses what to write; a hostile author writes whatever ranks best, and a broken author writes whatever its system clock says. It SHALL NOT be read as when the op was written, and no requirement in any capability SHALL be phrased as though it were.

Specifically, this value SHALL NOT decide an ordering, SHALL NOT break a tie in an ordering, SHALL NOT be compared against another op's wall-clock to establish which came first, SHALL NOT decide an expiry, a credential window, a rate limit, or a moderation outcome, and SHALL NOT be substituted for a Lamport counter that is absent.

It is carried because a reader is shown a time and expects one. That is its entire purpose, and every property a reader might want from it beyond display — that it is accurate, that it is comparable, that it is monotone across authors — is one this capability declines to provide.

**It is inside the signature for the same reason the counter is**: a relay cannot rewrite what a reader will be shown. That makes it as trustworthy as its author and no more, which is exactly the claim.

#### Scenario: The wall-clock is inside the signed preimage

- **WHEN** a signed op has its wall-clock changed and the original signature reattached
- **THEN** verification fails

#### Scenario: The wall-clock participates in the op id

- **WHEN** two ops are identical in every field except the wall-clock
- **THEN** their ids differ

#### Scenario: Any representable instant decodes, including implausible ones

- **WHEN** an op carries a wall-clock far in the future, far in the past, at zero, and at the maximum representable value
- **THEN** each decodes successfully
- **AND** decoding reports no error on account of the value

#### Scenario: Decoding does not compare the wall-clock against anything

- **WHEN** an op is decoded
- **THEN** no clock is read during decoding
- **AND** the result does not depend on when the decoding happened

### Requirement: An implausible wall-clock is accepted at the boundary, never refused

A peer SHALL NOT refuse, drop, or decline to store an op on account of its wall-clock value, however implausible. Every representable value SHALL be accepted.

**Refusing an op for a bad clock is a censorship vector, and that is the reason rather than a caveat.** A peer whose system clock is wrong — skewed, unset after a battery failure, or in a timezone misconfiguration — would have every op it publishes dropped by every conforming peer, silently and everywhere at once. From inside, that is indistinguishable from a moderation nobody performed, and there is no error path by which the author learns of it. A forum that is censorship-resistant by design cannot have a rule that drops an author for a clock.

The cost of accepting is real and is stated rather than hidden: **a reader may be shown a time that is not when the op was written.** That cost is bounded by display clamping, which `op-ordering` places on the read path, and is bounded absolutely by the fact that nothing decides anything on this value.

**This is where the alternative was rejected and why it must not be re-derived.** Clamping at the boundary — writing a corrected value into the stored op — is not available at any price: the op is signed and the wall-clock is inside the preimage, so a peer that rewrote it would hold an op that no longer verifies and whose id no longer matches. Clamping is therefore a read-time presentation rule and can be nothing else.

#### Scenario: A far-future wall-clock does not prevent storage

- **WHEN** an otherwise valid op carrying a wall-clock centuries in the future arrives
- **THEN** it is accepted and stored
- **AND** the refusal reasons the decoder reports do not include its wall-clock

#### Scenario: A far-past and a zero wall-clock are equally accepted

- **WHEN** otherwise valid ops carrying a zero wall-clock and one far in the past arrive
- **THEN** each is accepted and stored

#### Scenario: A stored op's wall-clock is the one its author signed

- **WHEN** an op carrying an implausible wall-clock is stored and read back
- **THEN** the value read back is the value the author signed
- **AND** the op still verifies, so nothing rewrote it

## MODIFIED Requirements

### Requirement: An op carries no ordering field and no per-peer state

An op SHALL carry exactly two values that its author chooses freely and that bear on ordering — the Lamport counter and the wall-clock contracted above — and SHALL carry no others.

An op SHALL NOT carry a transport message id, a sequence number, a session counter, an arrival counter, a device or installation identifier, or any other value that varies with an individual peer's history.

An op SHALL NOT carry the transport's sender identifier. That identifier binds at channel creation as a transport self-filter and is not an author identity; the author identity in an op is the key it carries and the address derived from it.

This omission SHALL be enforced by the encoding rather than merely documented: the encoding's length is fully accounted for by the fields that are present, so a further field cannot be added without the encoding's shape visibly changing.

**Why the two admitted fields are admitted, when this requirement previously forbade both.** The prior text refused a self-asserted Lamport value because it "would be forgeable by exactly the author it is meant to order", and refused a wall clock because "a wall clock is a field the adversary sets". Both observations are true and neither has been withdrawn. What changed is that the transport does not supply the alternative and is not going to: no Lamport value reaches this system, so the practical effect of the prohibition was not a transport-assigned order but **no order at all**, with every resolver falling to a hash. The two objections are answered rather than set aside, and each is answered somewhere a test can reach:

- **The forgeable counter** is bounded by `op-ordering`'s advance rule, which refuses to let a received counter raise this peer's own clock beyond a fixed distance. An inflated counter buys its author one op at the head of one Stoa's order and moves nothing else.
- **The adversary-set wall clock** is not bounded into safety; it is removed from every decision. It orders nothing, breaks no tie, and gates nothing, so there is no decision for an adversary's value to reach.

**The distinction the prior text collapsed, and which is the reason the two are not one field:** a Lamport counter is meaningful only relative to ops a peer has seen, so a bound on it is expressible in terms of the peer's own knowledge. A wall-clock is an absolute claim about the world, which a peer has nothing to check against. That is why one may order and the other may not.

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

### Requirement: The encoding declares its version

The encoding SHALL begin with a version discriminant, and decoding SHALL reject a version it does not recognise.

The version is the op format's own and SHALL NOT be shared with any other format's version. Two formats that version together mean a change to one invalidates the other for no reason.

**The addition of the Lamport counter and the wall-clock SHALL be a version increment**, because it changes the preimage every signature covers and every op id is computed from. A build SHALL decode both the version that carries the two fields and the version that does not, and SHALL report which of the two an op is, so that `op-ordering` can place an op carrying no counter.

A version increment is the correct cost here and a kind discriminant is not, which is the mistake available: the kinds are unchanged, and the change is to what every kind carries.

#### Scenario: The version is the first byte

- **WHEN** an op of any kind is encoded
- **THEN** the version discriminant is the first byte
- **AND** the kind discriminant is the second

#### Scenario: An unknown version is distinguishable from corruption

- **WHEN** an op declares an unrecognised version
- **THEN** the failure names the version rather than reporting a malformed op

#### Scenario: Both versions decode and report which they are

- **WHEN** an op carrying the two clock fields and an op predating them are each decoded
- **THEN** both decode successfully
- **AND** each reports which version it is
- **AND** the one predating them reports carrying no counter, rather than reporting a counter of zero

#### Scenario: An op of the earlier version keeps the id it always had

- **WHEN** an op encoded under the version predating the clock fields has its id computed
- **THEN** the id is the one that version has always produced
- **AND** adding the fields to the format did not change it
