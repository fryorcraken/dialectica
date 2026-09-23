## ADDED Requirements

### Requirement: A blank title is not a valid title

A title is **blank** when every character in it is a **blank character**, as listed below. The empty title is blank, since it has no character that is not a blank character.

The blank characters are exactly these thirty code points. This list is the definition: it does not follow a later revision of any Unicode property, and a character not in it is not a blank character.

- **Whitespace** — the twenty-five code points carrying Unicode's `White_Space` property: U+0009, U+000A, U+000B, U+000C, U+000D, U+0020, U+0085, U+00A0, U+1680, U+2000, U+2001, U+2002, U+2003, U+2004, U+2005, U+2006, U+2007, U+2008, U+2009, U+200A, U+2028, U+2029, U+202F, U+205F and U+3000.
- **Zero-width** — U+200B ZERO WIDTH SPACE, U+200C ZERO WIDTH NON-JOINER, U+200D ZERO WIDTH JOINER, U+2060 WORD JOINER and U+FEFF ZERO WIDTH NO-BREAK SPACE.

A character outside the list is not a blank character even where it renders with no width or no visible mark — a bidirectional control such as U+200E LEFT-TO-RIGHT MARK, or U+180E MONGOLIAN VOWEL SEPARATOR — and a title made only of such characters is not blank.

**A record whose title is blank is not a valid record.** Encoding MUST refuse one, so that no record with a blank title has an encoding and no address names one, and decoding MUST refuse an input whose title is blank. Every blank title, the empty one included, MUST be refused with one and the same failure, and that failure MUST be distinguishable from every other refusal "A tampered or truncated record is rejected" lists.

**Only a title made entirely of blank characters is refused.** A title carrying at least one character that is not a blank character MUST NOT be refused on account of the blank characters it also carries, wherever in the title they sit, and MUST NOT be trimmed, normalised or otherwise altered: its encoding carries every character as given, and decoding returns every character unchanged.

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

## MODIFIED Requirements

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
