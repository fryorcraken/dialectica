# stoa-genesis Specification

## Purpose
Defines what a Stoa's genesis record contains and how it encodes to bytes, so that a Stoa's address is reproducible by any peer and pasting an address is enough to verify what was joined.

## Requirements

### Requirement: A Stoa is defined by its genesis record

A Stoa **is** its genesis record: the record is not metadata describing a Stoa that exists elsewhere, it is the whole of what a Stoa is. That is what makes creation permissionless — there is no registry to register with, so publishing the record is the entire act of creation.

A genesis record MUST carry its creator's public key, a posting policy, and a human-readable title.

The record MUST be immutable. Creating a Stoa requires no approval and no registration with any service: publishing the record is the whole act of creation.

The creator's key in this record is what makes the creator the Stoa's initial sole moderator, so a record whose creator key is absent or malformed does not describe a Stoa at all.

The record's values are **founding** values, not current ones. The record fixes what the Stoa was created as, and therefore what its address commits to; it does not fix what the Stoa is called today. Superseding a title or policy is a separate capability — a moderator-signed metadata op — and is out of scope here. This requirement establishes only that the genesis values are immutable and that changing the displayed title therefore cannot change the address.

#### Scenario: A genesis record yields a Stoa address

- **WHEN** a genesis record is encoded and hashed
- **THEN** the result is a 32-byte Stoa address

#### Scenario: The title is not identity

- **WHEN** two different Stoas are created with the same title
- **THEN** their addresses differ
- **AND** neither is treated as the other

### Requirement: The encoding is canonical

A genesis record that is valid MUST have exactly one byte encoding. Every variable-length field MUST be length-prefixed, so that no two distinct records encode to the same bytes.

Validity is the precondition, not an afterthought: a record whose title exceeds the maximum has no encoding at all, so the length bound is what makes this requirement true rather than approximately true.

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

Decoding MUST reject any input that is not the canonical encoding of a valid record: truncated input, trailing bytes, a length prefix disagreeing with the data present in either direction, an unrecognised version, an unrecognised policy discriminant, a title that is not valid UTF-8, a title longer than the maximum, a blank title as "A blank title is not a valid title" defines one, and a creator key that is not a valid public key or that can never verify a signature.

Each MUST be reported distinguishably. A decoder that says only "invalid" sends the reader looking in the wrong place.

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

#### Scenario: An unknown policy is refused rather than defaulted

- **WHEN** a record carries a policy discriminant this version does not recognise
- **THEN** decoding fails
- **AND** the record is not treated as an open Stoa

### Requirement: An address verifies the record it names

A peer given a Stoa address and a candidate genesis record MUST be able to determine, without consulting any registry or third party, whether the record is the one that address names.

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

A genesis record MUST carry a posting policy. `open` MUST be the only policy this version accepts, and the field MUST be present in the encoding rather than implied by its absence.

The field is present now because the record is immutable and address-determining: adding it later would change the address of every Stoa already created, and there is no in-place upgrade path. Open, invite, first-post-approval and token-threshold are variants of one mechanism, so the space is reserved even while one variant is implemented.

#### Scenario: An open Stoa round-trips

- **WHEN** a record declaring the open policy is encoded and decoded
- **THEN** the decoded record declares the open policy

#### Scenario: The policy is carried in the encoding

- **WHEN** a record is encoded
- **THEN** the encoding carries the declared policy at a fixed position
- **AND** the encoding's length accounts for it, so it cannot be dropped without the encoding changing

### Requirement: The encoding declares its version

The encoding MUST begin with a version discriminant, and decoding MUST reject a version it does not recognise.

A genesis record is immutable and address-determining, so a future field cannot be added in place — it changes the address of every Stoa already created. The version discriminant is what makes that a legible refusal on an old client rather than a misparse, and what lets two encoding generations coexist on the network.

#### Scenario: An unknown version is refused

- **WHEN** a record declares a version this build does not recognise
- **THEN** decoding fails
- **AND** the failure is distinguishable from a malformed record

#### Scenario: The version is carried in the encoding

- **WHEN** a record is encoded
- **THEN** the version discriminant is the first byte of the encoding

### Requirement: The record carries no per-peer state

A genesis record MUST contain only values every peer agrees on. It MUST NOT carry a session counter, a local sequence number, or any other value that varies with an individual peer's history.

Every peer hashes the record to obtain the Stoa's address, so a per-peer value gives each peer a different address for the same Stoa. That failure is silent: it produces two Stoas that cannot see each other rather than an error anyone observes. The channel id derived from this address inherits the same constraint.

#### Scenario: Every peer computes the same address for the same Stoa

- **WHEN** two peers independently encode the same genesis record
- **THEN** both produce identical bytes
- **AND** both compute the same Stoa address

### Requirement: A blank title is not a valid title

A title is **blank** when every character in it is a **blank character**, as listed below. The empty title is blank, since it has no character that is not a blank character.

The blank characters are exactly these thirty code points. This list is the definition: it does not follow a later revision of any Unicode property, and a character not in it is not a blank character.

- **Whitespace** — the twenty-five code points carrying Unicode's `White_Space` property: U+0009, U+000A, U+000B, U+000C, U+000D, U+0020, U+0085, U+00A0, U+1680, U+2000, U+2001, U+2002, U+2003, U+2004, U+2005, U+2006, U+2007, U+2008, U+2009, U+200A, U+2028, U+2029, U+202F, U+205F and U+3000.
- **Zero-width** — U+200B ZERO WIDTH SPACE, U+200C ZERO WIDTH NON-JOINER, U+200D ZERO WIDTH JOINER, U+2060 WORD JOINER and U+FEFF ZERO WIDTH NO-BREAK SPACE.

A character outside the list is not a blank character even where it renders with no width or no visible mark — a bidirectional control such as U+200E LEFT-TO-RIGHT MARK, or U+180E MONGOLIAN VOWEL SEPARATOR — and a title made only of such characters is not blank.

**A record whose title is blank is not a valid record.** Encoding MUST refuse one, so that no record with a blank title has an encoding and no address names one, and decoding MUST refuse an input whose title is blank. Every blank title, the empty one included, MUST be refused with one and the same failure, and that failure MUST be distinguishable from every other refusal "A tampered or truncated record is rejected" lists.

**Only a title made entirely of blank characters is refused.** A title carrying at least one character that is not a blank character MUST NOT be refused on account of the blank characters it also carries, wherever in the title they sit, and MUST NOT be trimmed, normalised or otherwise altered: its encoding carries every character as given, and decoding returns every character unchanged.

**A blank title is the last refusal decoding reports.** An input to which any other refusal "A tampered or truncated record is rejected" lists also applies MUST be reported as that other refusal, and MUST NOT be reported as a blank title. Only an input that is otherwise exactly one well-formed record is refused for its title being blank.

#### Scenario: An empty title is refused on both sides

- **WHEN** a record is given a title that is the empty string
- **THEN** encoding it fails
- **AND** decoding an input that is otherwise a well-formed record, with a title length of zero, fails
- **AND** both failures are distinguishable from a truncated input, from a length prefix disagreeing with the data, from a title that is not valid UTF-8, and from a title longer than the maximum

#### Scenario: Each blank character alone is a blank title

- **WHEN** a record is given, in turn, a title consisting of exactly one of the thirty blank characters, for every one of them
- **THEN** encoding each fails
- **AND** decoding an input that is otherwise a well-formed record carrying that one-character title fails
- **AND** each failure is the same failure an empty title produces

#### Scenario: A title mixing several blank characters is refused on both sides

- **WHEN** a record is given the title U+0020 U+200B U+3000 U+FEFF U+0009
- **THEN** encoding it fails
- **AND** decoding an input that is otherwise a well-formed record carrying that title fails
- **AND** each failure is the same failure an empty title produces

#### Scenario: A title of one byte is accepted

- **WHEN** a record whose title is a single ASCII letter is encoded and decoded
- **THEN** both succeed
- **AND** the decoded title is that letter

#### Scenario: One visible letter among blank characters is accepted and kept as given

- **WHEN** a record whose title is U+0020 U+200B, the letter `a`, U+3000 U+FEFF is encoded and decoded
- **THEN** both succeed
- **AND** the decoded title is those five characters in that order, with none removed at either edge

#### Scenario: A title made only of characters outside the list is not blank

- **WHEN** a record whose title is U+200E alone is encoded and decoded, and again one whose title is U+180E alone
- **THEN** in each case both succeed
- **AND** the decoded title is that character unchanged

#### Scenario: A blank title followed by trailing bytes is refused as trailing bytes

- **WHEN** an input that is otherwise a well-formed record whose title is the empty string is followed by one extra byte, and again one whose title is U+0020 U+200B followed by one extra byte
- **THEN** decoding each fails with the trailing-bytes refusal
- **AND** neither failure is the one a blank title produces
