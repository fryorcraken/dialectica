## MODIFIED Requirements

### Requirement: A Stoa metadata op carries display fields and no policy

An op MAY declare a Stoa's current display metadata. Such an op SHALL carry a display title and a description, both length-prefixed, and SHALL carry no posting policy field.

The Stoa it applies to and the key that signed it are the Stoa and author every op already carries; they SHALL NOT be repeated in the kind's own fields.

Why no policy is a decision of the `stoa-metadata` capability, which owns the concept; this capability records only that the wire format has no such field and that the encoding's length accounts for exactly the fields named above, so one cannot be added without the encoding visibly changing.

**A metadata op whose title is the empty string is not a valid op.** Encoding MUST refuse one, and decoding MUST refuse an input carrying one, as one of the refusals "A malformed op is rejected at the decoding boundary" requires: before the input reaches any state machine, with no partially-populated op produced, and reported distinguishably from every other refusal that requirement lists. An empty description is valid, and MUST NOT be refused.

#### Scenario: A metadata op's encoding accounts for exactly its declared fields

- **WHEN** a metadata op is encoded
- **THEN** the encoding's length equals the version, kind, Stoa address, author key, length-prefixed title and length-prefixed description
- **AND** a policy field or an ordering field could not be added without this changing

#### Scenario: A metadata op's title and description cannot be confused

- **WHEN** one metadata op's title and description are the same concatenated text as another's, split at a different point
- **THEN** their encodings differ
- **AND** their ids differ

#### Scenario: A metadata op with an empty title is refused on both sides

- **WHEN** a metadata op is given a title that is the empty string
- **THEN** encoding it fails
- **AND** decoding an input that is otherwise a well-formed metadata op, with a title length of zero, fails
- **AND** the decoding failure is distinguishable from truncation, from a length prefix disagreeing with the input, and from an over-long field

#### Scenario: A metadata op with an empty description is accepted

- **WHEN** a metadata op carrying a non-empty title and an empty description is encoded and decoded
- **THEN** both succeed
- **AND** the decoded description is empty

#### Scenario: A signature over a metadata op does not verify as another kind

- **WHEN** the signature from a validly signed metadata op is attached to an op of another kind
- **THEN** verification of that op fails
- **AND** the original metadata op still verifies

#### Scenario: A metadata op signed by a non-moderator is authentic

- **WHEN** a peer who is not a moderator of the Stoa signs a metadata op for it
- **THEN** verification succeeds
- **AND** this is not a statement that the rename takes effect
