# op-format Specification

## MODIFIED Requirements

### Requirement: An op is the unit that crosses the wire

An op SHALL be a signed operation, and it SHALL be the only thing that crosses the wire. A peer's view of the forum is a function of the ops it has seen, so anything not expressible as an op is not expressible at all.

An op SHALL carry the Stoa it belongs to, the public key of its author, and exactly one kind describing what it does. The kinds are: a post (which is a reply when it names a parent), a revision of one of the author's own posts, a moderator's judgement about a target, a vote on a target, and a declaration of the Stoa's current display metadata.

Creating a Stoa SHALL NOT be an op. A Stoa is a genesis record its creator publishes, and hashing that record is what creates it — see the `stoa-genesis` capability. There is nothing for a signed op to add.

Replying SHALL NOT be a separate kind. A reply is a post that names a parent, so that "is this a reply?" is one question about a field rather than a second question about which kind arrived.

The author's **public key** SHALL travel in the op, not merely their address. Verification happens on read with no directory to resolve an address against, so a peer holding only an address could not check the signature. The address SHALL be recoverable from the key rather than carried separately.

Kind discriminants SHALL be appended, never inserted among those already allocated. Inserting would re-mean every op already signed, since the discriminant is inside both the signature and the op id. A kind added later therefore costs one unused discriminant and no encoding version, and an older client meets it as an unrecognised kind rather than misparsing it.

#### Scenario: Every kind round-trips through the encoding

- **WHEN** an op of any kind is encoded and decoded
- **THEN** the decoded op equals the original in every field

#### Scenario: A reply is a post that names a parent

- **WHEN** a post op names a parent op
- **THEN** it is the same kind as a post that names none
- **AND** both round-trip through the encoding

#### Scenario: An unallocated kind discriminant is refused rather than misparsed

- **WHEN** an op declares the first discriminant above those allocated
- **THEN** decoding fails naming that discriminant
- **AND** it is not read as any allocated kind

### Requirement: A field longer than the transport allows is refused before allocating

Decoding SHALL refuse any length prefix exceeding the maximum a single message can carry, and SHALL do so before allocating on the strength of that prefix. The limit applies to a variable-length field's byte length and to a list's element count alike.

That maximum SHALL be **150 KiB**, the network-wide gossipsub validation limit the transport applies to one message and which cannot be raised unilaterally. The value is named here rather than left to the implementation because the requirement's whole justification depends on it: a cap that had silently drifted upward would still refuse an absurd prefix, still satisfy a scenario that probes only absurd values, and still leave the lever open.

A four-byte length prefix can claim four gibibytes. A field larger than the message cap could never have arrived legitimately — and a decoder that reserved memory on a hostile peer's promise before discovering the input was short would be a remote memory-exhaustion lever. Discovering the shortfall afterwards is too late; the allocation has already happened.

The refusal SHALL be reported as an over-long field, distinguishably from running out of input, so that the cap is demonstrably what rejected it.

**This cap bounds a single field, and SHALL NOT be read as bounding a decoded op's total size.** The bound does not compose: an op with several variable-length fields, or a list whose count and elements are each bounded separately, can decode to a multiple of the message cap. That is accepted here rather than closed, because the amplification is approximately 1:1 — a large decoded op costs the sender a comparably large message — and because a total-size check requires knowing the transport frame the bytes arrived in. This capability is handed a byte slice and cannot see the frame. **A total-size bound belongs at the transport boundary**, and adding one here would be a guess at a number the caller already holds.

#### Scenario: An over-long field length is refused by the cap

- **WHEN** a body's length prefix claims far more than the message cap allows
- **THEN** decoding fails reporting an over-long field
- **AND** the failure is not merely "the input ended"

#### Scenario: An over-long list count is refused by the cap

- **WHEN** an attachment list's count claims far more than the message cap allows, with no elements behind it
- **THEN** decoding fails reporting an over-long field
- **AND** no capacity is reserved on the strength of the count

#### Scenario: A field exactly at the cap is accepted

- **WHEN** a variable-length field carries exactly the maximum permitted bytes, and the input actually holds them
- **THEN** decoding succeeds and recovers the field in full

#### Scenario: A field one byte over the cap is refused

- **WHEN** a variable-length field carries exactly one byte more than the maximum
- **THEN** decoding fails reporting an over-long field
- **AND** the reported length is the claimed length, so the cap is demonstrably what refused it

#### Scenario: The cap's value is pinned against silent drift

- **WHEN** the configured maximum is compared against the transport's stated message limit
- **THEN** they are equal
- **AND** a change to either fails rather than passing quietly

### Requirement: Text fields are validated as UTF-8 and not otherwise transformed

Decoding SHALL reject text that is not valid UTF-8, and SHALL NOT normalise, case-fold, reorder, strip or otherwise transform text that is valid.

Refusing invalid UTF-8 is required because a lossy conversion maps distinct inputs onto one op, which would give two byte strings one op id.

Declining to transform *valid* text is equally deliberate and is a canonicality requirement, not an oversight. Any normalisation applied at decode would mean an accepted byte string re-encoding to something other than itself, breaking op-id agreement between peers — the property the whole encoding exists to provide.

**The consequence is that display text is attacker-controlled and SHALL NOT be trusted for rendering or identification.** A title may contain bidirectional controls, zero-width characters, or homoglyphs of an established Stoa's name, and while no authority check exists any peer may sign such an op. Mitigation belongs to whoever renders: strip or visibly mark bidi and zero-width controls, show the Stoa address alongside any name, and never treat a title as an identifier. The address is the identity; a name never is.

#### Scenario: Text that is not valid UTF-8 is refused

- **WHEN** a text field's bytes are not valid UTF-8
- **THEN** decoding fails
- **AND** the bytes are not lossily converted

#### Scenario: Valid text is returned exactly as it arrived

- **WHEN** a text field carries valid UTF-8, including multi-byte sequences and embedded control characters
- **THEN** decoding returns those characters unchanged
- **AND** re-encoding reproduces the original bytes

## ADDED Requirements

### Requirement: A Stoa metadata op carries display fields and no policy

An op MAY declare a Stoa's current display metadata. Such an op SHALL carry a display title and a description, both length-prefixed, and SHALL carry no posting policy field.

The Stoa it applies to and the key that signed it are the Stoa and author every op already carries; they SHALL NOT be repeated in the kind's own fields.

Why no policy is a decision of the `stoa-metadata` capability, which owns the concept; this capability records only that the wire format has no such field and that the encoding's length accounts for exactly the fields named above, so one cannot be added without the encoding visibly changing.

#### Scenario: A metadata op's encoding accounts for exactly its declared fields

- **WHEN** a metadata op is encoded
- **THEN** the encoding's length equals the version, kind, Stoa address, author key, length-prefixed title and length-prefixed description
- **AND** a policy field or an ordering field could not be added without this changing

#### Scenario: A metadata op's title and description cannot be confused

- **WHEN** one metadata op's title and description are the same concatenated text as another's, split at a different point
- **THEN** their encodings differ
- **AND** their ids differ

#### Scenario: A signature over a metadata op does not verify as another kind

- **WHEN** the signature from a validly signed metadata op is attached to an op of another kind
- **THEN** verification of that op fails
- **AND** the original metadata op still verifies

#### Scenario: A metadata op signed by a non-moderator is authentic

- **WHEN** a peer who is not a moderator of the Stoa signs a metadata op for it
- **THEN** verification succeeds
- **AND** this is not a statement that the rename takes effect
