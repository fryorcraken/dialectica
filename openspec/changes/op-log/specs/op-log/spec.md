## Purpose

Defines what a peer stores when an op reaches it, what the store refuses to decide, and what a reader may ask of a set of ops that is legitimately incomplete.

## ADDED Requirements

### Requirement: The log records what arrived, and decides nothing about it

The log SHALL store every op appended to it, together with the transport metadata recorded on its arrival. It SHALL NOT verify a signature, check an authorisation, or reject an op on the basis of its content when appending.

Verification is the reader's job.

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

### Requirement: An op is stored once, identified by its op id

Appending an op already present in the log SHALL leave the log holding exactly one entry for that op. The op id SHALL be the sole identity used for this, and it SHALL be derived from the op's own content rather than taken from any transport-assigned identifier.

#### Scenario: The same op appended twice is one entry

- **WHEN** an op is appended and then the identical op is appended again
- **THEN** the log holds one entry for it
- **AND** the number of ops in the log has increased by one in total

#### Scenario: The caller is told whether the op was new

- **WHEN** an op is appended
- **THEN** the result states whether it was newly stored or already present
- **AND** a caller can act on that without counting the log before and after

#### Scenario: Two different ops are two entries

- **WHEN** two ops differing in any field are appended
- **THEN** the log holds both
- **AND** each is readable by its own op id

#### Scenario: Re-appending does not disturb the stored op

- **WHEN** an op already in the log is appended again
- **THEN** the op read back is the one first stored
- **AND** its recorded arrival metadata is the one first recorded

### Requirement: A re-arrival does not overwrite the recorded arrival metadata

When an op already present is appended again with different transport metadata, the log SHALL retain the metadata recorded on the first arrival and SHALL NOT replace it with the later one.

#### Scenario: A later arrival does not reorder an op already stored

- **WHEN** an op recorded with one Lamport timestamp is appended again with a higher one
- **THEN** the log reports the first timestamp
- **AND** the op's position in the log's order is unchanged

#### Scenario: An unordered re-arrival does not erase a recorded order

- **WHEN** an op recorded with a Lamport timestamp is appended again with no transport metadata
- **THEN** the log still reports the original timestamp
- **AND** the op is still reported as ordered by the transport

#### Scenario: A richer re-arrival does not replace a poorer recorded one

- **WHEN** an op recorded with no transport metadata is appended again with a Lamport timestamp
- **THEN** the log still reports no Lamport timestamp
- **AND** the op is still reported as unordered by the transport

### Requirement: No read presents two entries for one op id

A read SHALL NOT return two entries carrying the same op id, including when the same op was appended more than once with differing transport metadata. Deduplication SHALL be established before ops are placed in order, and SHALL NOT be applied as a pass over an already-ordered result.

#### Scenario: One op arriving twice with different metadata is one entry

- **WHEN** an op is appended with no transport metadata and then appended again with a message id
- **THEN** the log holds one entry for it
- **AND** a read returns it once

#### Scenario: Repeated delivery under varying metadata yields no duplicates

- **WHEN** several ops are each appended many times under different transport metadata
- **THEN** every read returns each op exactly once

### Requirement: Reading the log yields ops in the defined order

A read of the log SHALL return its ops in the order the ordering rule defines over each op's recorded arrival metadata and op id. The log SHALL NOT define an order of its own, and SHALL NOT order by insertion sequence.

#### Scenario: Ops are read in the ordering rule's order, not insertion order

- **WHEN** ops are appended in an order that differs from the ordering rule's
- **THEN** reading the log returns them in the ordering rule's order

#### Scenario: Two peers with the same ops read the same order

- **WHEN** two logs hold the same ops with the same recorded metadata, appended in different sequences
- **THEN** both return the same sequence of ops

#### Scenario: Ops the transport did not order still read in a defined order

- **WHEN** a log holds only ops with no recorded Lamport timestamp
- **THEN** reading it returns them in a defined, repeatable order
- **AND** that order is the same in a log that received them in the reverse sequence

### Requirement: A reader can restrict a read to one Stoa and to one target

The log SHALL offer a read restricted to the ops belonging to a given Stoa, and a read restricted to the ops naming a given op as their target. Each restricted read SHALL return its ops in the same order an unrestricted read would place them in.

A restricted read SHALL match on the **complete** identifier. It SHALL NOT return an op whose Stoa address or target op id merely shares a prefix with, or is otherwise distinguishable from, the one requested.

A read restricted to a target SHALL return every op naming that target, whatever the op's kind.

#### Scenario: A Stoa-restricted read excludes other Stoas

- **WHEN** a log holds ops from two Stoas and a read is restricted to one
- **THEN** only that Stoa's ops are returned
- **AND** they are in the same relative order as in an unrestricted read

#### Scenario: A target-restricted read returns every op naming that target

- **WHEN** a log holds a revision, a moderation and a vote all naming one op
- **THEN** a read restricted to that target returns all three
- **AND** they are in the same relative order as in an unrestricted read

#### Scenario: An op that names no target is never returned by a target read

- **WHEN** a log holds a post, which names no target
- **THEN** no target-restricted read returns it

#### Scenario: A target read does not match an op by its own id

- **WHEN** an op is read by its own id as a target
- **THEN** it is not itself returned
- **AND** only ops naming it are returned

#### Scenario: Two Stoas whose addresses share a prefix are not confused

- **WHEN** a log holds ops from two Stoas whose addresses share a leading byte but differ overall
- **THEN** a read restricted to either Stoa returns only that Stoa's ops
- **AND** neither Stoa's ops appear in the other's result

#### Scenario: Two targets whose op ids share a prefix are not confused

- **WHEN** a log holds moderation ops naming two different targets whose op ids share a leading byte
- **THEN** a read restricted to either target returns only the ops naming that target
- **AND** a moderation op aimed at one target never appears in the other's result

#### Scenario: A restricted read over an absent subject is empty, not an error

- **WHEN** a read is restricted to a Stoa or a target the log holds no ops for
- **THEN** the result is empty
- **AND** no error is reported

#### Scenario: A read over a populated log for an absent Stoa is empty

- **WHEN** a log holding ops is read restricted to a Stoa it holds no ops for
- **THEN** the result is empty
- **AND** the log still holds its ops

### Requirement: Every read is defined over the ops the peer happens to hold

The log SHALL answer every read from the ops it holds, and SHALL NOT report an error, an incomplete result, or an unknown outcome on the grounds that other ops may exist elsewhere.

#### Scenario: An empty log answers every read

- **WHEN** a log holding no ops is read, by Stoa, by target, or in full
- **THEN** each read returns an empty result
- **AND** none reports an error

#### Scenario: A read over a partial set is not an error

- **WHEN** a log holds a revision whose target op it has never received
- **THEN** the revision is returned by an unrestricted read
- **AND** a read restricted to the absent target returns the revision

#### Scenario: An op is readable whether or not its thread is known

- **WHEN** a reply whose parent op is absent from the log is appended
- **THEN** it is stored and readable

### Requirement: Ops appended with ordering metadata and without it live in one log

The log SHALL accept ops whose recorded arrival carries a Lamport timestamp and ops whose recorded arrival carries none, in any mixture, without distinguishing them at append time. It SHALL preserve for each op whether its arrival was ordered by the transport.

#### Scenario: Ordered and unordered ops coexist

- **WHEN** a log holds ops with recorded Lamport timestamps and ops without
- **THEN** every op is readable
- **AND** reading returns them in the ordering rule's order, which places the unordered ones after the ordered ones

#### Scenario: Whether an arrival was ordered survives storage

- **WHEN** an op recorded as ordered by the transport is read back
- **THEN** it is still reported as ordered by the transport
- **AND** an op recorded as unordered is still reported as unordered

### Requirement: Storing an op never aborts the process

Appending, reading, or counting SHALL NOT panic for any op the op decoder accepts, whatever its content or recorded metadata.

#### Scenario: A hostile op is stored without a panic

- **WHEN** ops with empty, maximal, and adversarially chosen field values are appended and read
- **THEN** no operation panics

#### Scenario: Reading an absent op id is a defined absence

- **WHEN** an op id not present in the log is looked up
- **THEN** the result reports absence
- **AND** no panic occurs
