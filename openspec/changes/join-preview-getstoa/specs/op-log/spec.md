## MODIFIED Requirements

### Requirement: The log records what arrived, and decides nothing about it

The log SHALL store every op appended to it, together with the transport metadata recorded on its arrival. It SHALL NOT verify a signature, check an authorisation, or reject an op on the basis of its content when appending, except for the one refusal below.

Verification is the reader's job.

**A log that keeps ops as their encoding MUST refuse to append an op the `op-format` capability's encoding refuses to encode** — in this build, a metadata op whose title is blank. A log keeps ops as their encoding when it stores each op's encoded bytes and decodes them on read, as a peer's persistent log does. Such a log MUST store nothing for the refused op, MUST report the refusal distinguishably from a storage failure, and MUST leave every op it already held readable exactly as before. This refusal is the op format's and not a judgement of the op's authenticity or authority: it applies to no op that encoding admits, whatever its signature or author.

A log that keeps ops as values, and so decodes nothing on read, is not under that refusal, and MUST store such an op as it stores any other.

#### Scenario: An op with an invalid signature is stored

- **WHEN** an op whose signature does not verify is appended
- **THEN** the append succeeds
- **AND** the op is subsequently readable from the log
- **AND** the log reports nothing about its validity

#### Scenario: An op from a peer with no authority is stored

- **WHEN** a moderation op signed by someone who is not a moderator is appended
- **THEN** the append succeeds and the op is readable
- **AND** the log does not distinguish it from a moderation op signed by a moderator

#### Scenario: The stored op is byte-identical to the one appended

- **WHEN** an op is appended and then read back
- **THEN** the op and its signature are unchanged
- **AND** its op id is unchanged

#### Scenario: An op with no encoding is refused by a log that keeps encodings, and nothing is written

- **WHEN** a log that keeps ops as their encoding, holding one op of a Stoa, is appended a metadata op for that Stoa, authentically signed, whose title is the empty string
- **THEN** the append reports a refusal
- **AND** the refusal is distinguishable from a storage failure
- **AND** the log holds exactly the one op it held before
- **AND** a read restricted to that Stoa succeeds and returns that one op

#### Scenario: A log that keeps ops as values stores an op with no encoding

- **WHEN** a log that keeps ops as values is appended a metadata op, authentically signed, whose title is the empty string
- **THEN** the append succeeds
- **AND** the op is readable from the log, with its title unchanged

## ADDED Requirements

### Requirement: A stored entry that does not decode fails every read that would return it

A read that would return a stored entry the log cannot decode back into an op MUST fail, and MUST report that a stored entry did not decode, distinguishably from a storage failure. It MUST NOT skip the entry, MUST NOT return the other entries as though that one were absent, and MUST NOT report the entry as absent. This holds for a read by op id, for a read restricted to a Stoa or a target whose result would include the entry, and for an unrestricted read.

A read MUST NOT repair, rewrite, migrate or remove such an entry. It stays as it was stored.

**This includes a metadata op whose title is blank that was stored before the `op-format` capability refused one.** Its stored bytes are an encoding that capability now refuses to decode, so a read that would return it fails as above, carrying the reason the decoding gave. It is not returned as an op, so no reader holds it as a metadata op that fails to bind.

#### Scenario: An entry that does not decode fails a read rather than being skipped

- **WHEN** a log that keeps ops as their encoding holds two ops of one Stoa, and the stored bytes of one of them are replaced by bytes that are not an op's encoding
- **THEN** a read of that entry by its op id fails
- **AND** a read restricted to that Stoa fails, rather than returning the other op alone
- **AND** an unrestricted read fails
- **AND** each failure is distinguishable from a storage failure

#### Scenario: A blank-titled metadata op stored before the refusal is reported and left as it was

- **WHEN** a log that keeps ops as their encoding holds, for a Stoa, an entry stored as it would have been before the `op-format` capability refused a blank title: a metadata op signed by that Stoa's creator whose title is the empty string, its bytes laid out as every other metadata op's are and followed by its signature — and again with such an entry whose title is U+0020 U+200B
- **THEN** a read restricted to that Stoa fails
- **AND** its reason names the title as blank
- **AND** the entry's stored bytes afterwards are the bytes that were stored before the read
