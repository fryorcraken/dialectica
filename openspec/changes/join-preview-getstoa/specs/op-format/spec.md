## MODIFIED Requirements

### Requirement: Valid text is never normalised or otherwise transformed

Decoding SHALL NOT normalise, case-fold, reorder, strip or otherwise transform text that is valid UTF-8. Rejecting text that is *not* valid UTF-8 is already required by "A malformed op is rejected at the decoding boundary"; this requirement governs what happens to text that passes that check.

Declining to transform valid text is a canonicality requirement, not an oversight. Any normalisation applied at decode would mean an accepted byte string re-encoding to something other than itself, which contradicts "An accepted encoding re-encodes to itself" and would break op-id agreement between peers — the property the whole encoding exists to provide. Two peers on builds with different Unicode tables would compute different ids for one op, silently.

**The consequence is that display text is attacker-controlled and SHALL NOT be trusted for rendering or identification.** A title may contain bidirectional controls, zero-width characters, or homoglyphs of an established Stoa's name, and while no authority check exists any peer may sign such an op. Mitigation belongs to whoever renders: strip or visibly mark bidi and zero-width controls, show the Stoa address alongside any name, and never treat a title as an identifier. The Stoa address is the identity; a name never is.

**Only the final sentence changes**, from "The address is the identity" to "The Stoa address is the identity". The requirement is about titles and its subject was always the Stoa — the preceding clause says "show the Stoa address alongside any name" — so nothing here changes meaning. The bare sentence is scoped because it is the one a later reader would cite out of context as authority for an author address, which this change deletes.

#### Scenario: Valid text is returned exactly as it arrived

- **WHEN** a text field carries valid UTF-8, including multi-byte sequences, bidirectional controls and zero-width characters, and — where the field is a metadata op's title — at least one character that is not blank, as "A Stoa metadata op carries display fields and no policy" uses the term
- **THEN** decoding returns those characters unchanged
- **AND** re-encoding reproduces the original bytes

#### Scenario: Canonically-equivalent text stays distinct

- **WHEN** two ops carry titles that differ only by Unicode normalisation form, such as a combining sequence against its precomposed equivalent
- **THEN** their encodings differ
- **AND** their ids differ, rather than collapsing onto one op

### Requirement: A Stoa metadata op carries display fields and no policy

An op MAY declare a Stoa's current display metadata. Such an op SHALL carry a display title and a description, both length-prefixed, and SHALL carry no posting policy field.

The Stoa it applies to and the key that signed it are the Stoa and author every op already carries; they SHALL NOT be repeated in the kind's own fields.

Why no policy is a decision of the `stoa-metadata` capability, which owns the concept; this capability records only that the wire format has no such field and that the encoding's length accounts for exactly the fields named above, so one cannot be added without the encoding visibly changing.

**A metadata op whose title is blank is not a valid op.** Blank has the meaning, and the exact list of blank characters, that the `stoa-genesis` capability's requirement "A blank title is not a valid title" gives it; the empty title is blank. Encoding MUST refuse such an op, and decoding MUST refuse an input carrying one, as one of the refusals "A malformed op is rejected at the decoding boundary" requires: before the input reaches any state machine, with no partially-populated op produced, and reported distinguishably from every other refusal that requirement lists. Every blank title, the empty one included, MUST be refused with one and the same failure.

A title carrying at least one character that is not a blank character MUST NOT be refused on account of the blank characters it also carries, and MUST NOT be trimmed or otherwise altered. The description is not held to this rule: an empty description, and a description made only of blank characters, are valid and MUST NOT be refused.

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

#### Scenario: A metadata op whose title is made only of blank characters is refused on both sides

- **WHEN** a metadata op is given, in turn, a title consisting of exactly one of the thirty blank characters, for every one of them, and then the title U+0020 U+200B U+3000 U+FEFF U+0009
- **THEN** encoding each fails
- **AND** decoding an input that is otherwise a well-formed metadata op carrying that title fails
- **AND** each failure is the same failure an empty title produces

#### Scenario: A metadata op whose title has one visible letter among blank characters is accepted

- **WHEN** a metadata op whose title is U+0020 U+200B, the letter `a`, U+3000 U+FEFF is encoded and decoded
- **THEN** both succeed
- **AND** the decoded title is those five characters in that order, with none removed at either edge

#### Scenario: A metadata op with an empty description is accepted

- **WHEN** a metadata op carrying a title that is not blank and an empty description is encoded and decoded
- **THEN** both succeed
- **AND** the decoded description is empty

#### Scenario: A metadata op with a blank description is accepted

- **WHEN** a metadata op carrying a title that is not blank and the description U+0020 U+200B is encoded and decoded
- **THEN** both succeed
- **AND** the decoded description is those two characters unchanged

#### Scenario: A signature over a metadata op does not verify as another kind

- **WHEN** the signature from a validly signed metadata op is attached to an op of another kind
- **THEN** verification of that op fails
- **AND** the original metadata op still verifies

#### Scenario: A metadata op signed by a non-moderator is authentic

- **WHEN** a peer who is not a moderator of the Stoa signs a metadata op for it
- **THEN** verification succeeds
- **AND** this is not a statement that the rename takes effect
