# stoa-genesis Specification

## Purpose
Defines what a Stoa's genesis record contains and how it encodes to bytes, so that a Stoa's address is reproducible by any peer and pasting an address is enough to verify what was joined.

## Requirements

### Requirement: A Stoa is defined by its genesis record

A Stoa SHALL be defined by a genesis record carrying its creator's public key, a posting policy, and a human-readable title.

The record SHALL be immutable. Creating a Stoa requires no approval and no registration with any service: publishing the record is the whole act of creation.

The creator's key in this record is what makes the creator the Stoa's initial sole moderator, so a record whose creator key is absent or malformed does not describe a Stoa at all.

#### Scenario: A genesis record yields a Stoa address

- **WHEN** a genesis record is encoded and hashed
- **THEN** the result is a 32-byte Stoa address

#### Scenario: The title is not identity

- **WHEN** two different Stoas are created with the same title
- **THEN** their addresses differ
- **AND** neither is treated as the other

### Requirement: The encoding is canonical

A genesis record SHALL have exactly one valid byte encoding. Every variable-length field SHALL be length-prefixed, so that no two distinct records encode to the same bytes.

This is what makes an address reproducible: two peers holding the same record compute the same address, and a peer cannot be shown a record that hashes to an address it does not describe.

#### Scenario: The same record always encodes identically

- **WHEN** the same genesis record is encoded twice
- **THEN** the two byte strings are identical

#### Scenario: Distinct records never share an encoding

- **WHEN** two genesis records differ in any field
- **THEN** their encodings differ
- **AND** their addresses differ

#### Scenario: A variable-length field's length is carried, not inferred

- **WHEN** a record's encoding is followed by an extra byte
- **THEN** decoding fails rather than absorbing that byte into the adjacent variable-length field

### Requirement: A tampered or truncated record is rejected

Decoding SHALL reject any input that is not the canonical encoding of a valid record: truncated input, trailing bytes, a length prefix disagreeing with the data present in either direction, an unrecognised version, an unrecognised policy discriminant, a title that is not valid UTF-8, and a creator key that is not a valid public key.

Each SHALL be reported distinguishably. A decoder that says only "invalid" sends the reader looking in the wrong place.

A record arriving from a peer is attacker-controlled. Rejection SHALL happen at the decoding boundary, before the record reaches any state machine, and SHALL NOT be reported as a valid record carrying default values.

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

#### Scenario: An unknown policy is refused rather than defaulted

- **WHEN** a record carries a policy discriminant this version does not recognise
- **THEN** decoding fails
- **AND** the record is not treated as an open Stoa

### Requirement: An address verifies the record it names

A peer given a Stoa address and a candidate genesis record SHALL be able to determine, without consulting any registry or third party, whether the record is the one that address names.

This is what makes a pasted address self-authenticating, and it is a security boundary: §4.8 has Stoa addresses appearing inside posts, which is attacker-supplied content.

#### Scenario: A matching record verifies

- **WHEN** a record is checked against the address computed from it
- **THEN** verification succeeds

#### Scenario: A substituted record fails verification

- **WHEN** a record differing in any field is checked against the original address
- **THEN** verification fails

#### Scenario: Verification consults nothing external

- **WHEN** a record is verified against an address
- **THEN** the check uses only the address and the record

### Requirement: A posting policy is declared at creation

A genesis record SHALL carry a posting policy. `open` SHALL be the only policy this version accepts, and the field SHALL be present in the encoding rather than implied by its absence.

The field is present now because the record is immutable and address-determining: adding it later would change the address of every Stoa already created, and there is no in-place upgrade path. Open, invite, first-post-approval and token-threshold are variants of one mechanism, so the space is reserved even while one variant is implemented.

#### Scenario: An open Stoa round-trips

- **WHEN** a record declaring the open policy is encoded and decoded
- **THEN** the decoded record declares the open policy

#### Scenario: The policy is carried in the encoding

- **WHEN** a record is encoded
- **THEN** the encoding carries the declared policy at a fixed position
- **AND** the encoding's length accounts for it, so it cannot be dropped without the encoding changing

### Requirement: The encoding declares its version

The encoding SHALL begin with a version discriminant, and decoding SHALL reject a version it does not recognise.

A genesis record is immutable and address-determining, so a future field cannot be added in place — it changes the address of every Stoa already created. The version discriminant is what makes that a legible refusal on an old client rather than a misparse, and what lets two encoding generations coexist on the network.

#### Scenario: An unknown version is refused

- **WHEN** a record declares a version this build does not recognise
- **THEN** decoding fails
- **AND** the failure is distinguishable from a malformed record

#### Scenario: The version is carried in the encoding

- **WHEN** a record is encoded
- **THEN** the version discriminant is the first byte of the encoding

### Requirement: The record carries no per-peer state

A genesis record SHALL contain only values every peer agrees on. It SHALL NOT carry a session counter, a local sequence number, or any other value that varies with an individual peer's history.

Every peer hashes the record to obtain the Stoa's address, so a per-peer value gives each peer a different address for the same Stoa. That failure is silent: it produces two Stoas that cannot see each other rather than an error anyone observes. The channel id derived from this address inherits the same constraint.

#### Scenario: Every peer computes the same address for the same Stoa

- **WHEN** two peers independently encode the same genesis record
- **THEN** both produce identical bytes
- **AND** both compute the same Stoa address
