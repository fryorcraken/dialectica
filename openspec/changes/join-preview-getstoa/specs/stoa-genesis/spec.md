## MODIFIED Requirements

### Requirement: A tampered or truncated record is rejected

Decoding MUST reject any input that is not the canonical encoding of a valid record: truncated input, trailing bytes, a length prefix disagreeing with the data present in either direction, an unrecognised version, an unrecognised policy discriminant, a title that is not valid UTF-8, a title longer than the maximum, an empty title, and a creator key that is not a valid public key or that can never verify a signature.

Each MUST be reported distinguishably. A decoder that says only "invalid" sends the reader looking in the wrong place.

**A record whose title is the empty string is not a valid record.** Encoding MUST refuse one, so that no record with an empty title has an encoding and no address names one, and decoding MUST refuse an input whose title is empty.

A record arriving from a peer is attacker-controlled. Rejection MUST happen at the decoding boundary, before the record reaches any state machine, and MUST NOT be reported as a valid record carrying default values.

#### Scenario: Truncated input is refused

- **WHEN** a genesis record's encoding is truncated at any point
- **THEN** decoding fails
- **AND** no partially-populated record is produced

#### Scenario: Trailing bytes are refused

- **WHEN** input carries a valid encoding followed by extra bytes
- **THEN** decoding fails

#### Scenario: A length prefix claiming more than the input holds is refused

- **WHEN** a length prefix claims more bytes than the input provides
- **THEN** decoding fails

#### Scenario: A length prefix claiming less than the input holds is refused

- **WHEN** a length prefix claims fewer bytes than the input provides
- **THEN** decoding fails
- **AND** the remaining bytes are not silently ignored

#### Scenario: A title that is not valid UTF-8 is refused

- **WHEN** a record's title bytes are not valid UTF-8
- **THEN** decoding fails
- **AND** the bytes are not lossily converted, which would map distinct inputs onto one record

#### Scenario: A creator key that is not a valid public key is refused

- **WHEN** a record carries a creator key that is not a valid public key
- **THEN** decoding fails
- **AND** the failure is distinguishable from a malformed encoding

#### Scenario: A creator key that can never verify a signature is refused

- **WHEN** a record carries a well-formed creator key under which no signature can ever verify
- **THEN** decoding fails
- **AND** the failure is distinguishable from a malformed key

#### Scenario: A title longer than the maximum is refused on both sides

- **WHEN** a record's title exceeds the maximum length
- **THEN** encoding it fails
- **AND** decoding an input declaring that length fails
- **AND** the input is refused before its title is read

#### Scenario: An empty title is refused on both sides

- **WHEN** a record is given a title that is the empty string
- **THEN** encoding it fails
- **AND** decoding an input that is otherwise a well-formed record, with a title length of zero, fails
- **AND** both failures are distinguishable from a truncated input, from a length prefix disagreeing with the data, and from a title longer than the maximum

#### Scenario: A title of one byte is accepted

- **WHEN** a record whose title is a single ASCII letter is encoded and decoded
- **THEN** both succeed
- **AND** the decoded title is that letter

#### Scenario: An unknown policy is refused rather than defaulted

- **WHEN** a record carries a policy discriminant this version does not recognise
- **THEN** decoding fails
- **AND** the record is not treated as an open Stoa
