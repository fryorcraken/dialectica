# op-log Specification

## Purpose

Defines what a peer stores when an op reaches it, what the store refuses to decide, and what a reader may ask of a set of ops that is legitimately incomplete.

## Requirements

### Requirement: The log records what arrived, and decides nothing about it

The log SHALL store every op appended to it, together with the transport metadata recorded on its arrival. It SHALL NOT verify a signature, check an authorisation, or reject an op on the basis of its content when appending, except for the one refusal below.

Verification is the reader's job.

**A log that keeps ops as their encoding MUST refuse to append an op the `op-format` capability's encoding refuses to encode** — in this build, a metadata op whose title is blank. A log keeps ops as their encoding when it stores each op's encoded bytes and decodes them on read, as a peer's persistent log does. Such a log MUST store nothing for the refused op, MUST report the refusal distinguishably from a storage failure, and MUST leave every op it already held readable exactly as before. This refusal is the op format's and not a judgement of the op's authenticity or authority: it applies to no op that encoding admits, whatever its signature or author.

A log that keeps ops as values, and so decodes nothing on read, is not under that refusal, and MUST store such an op as it stores any other.

An implementation that persists SHALL apply this identically, and the refusal above is the only one it makes. Persistence SHALL NOT become an occasion to filter by signature or authority: a store that refused to write an op it could not verify would make a forgery indistinguishable from an op that never arrived, which is the same defect whether the store is in memory or on disk.

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

#### Scenario: A persisted op is byte-identical after a restart

- **WHEN** an op is appended to a persistent log, the log is closed, and a new log is opened over the same storage
- **THEN** the op read back is byte-identical to the one appended
- **AND** its signature still verifies
- **AND** its recorded arrival metadata is unchanged

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

### Requirement: An op is stored once, identified by its op id

Appending an op already present in the log SHALL leave the log holding exactly one entry for that op. The op id SHALL be the sole identity used for this, and it SHALL be derived from the op's own content rather than taken from any transport-assigned identifier.

Deduplication SHALL be a property of how entries are held, not a check performed before each write. An implementation SHALL be unable to hold two entries for one op id, rather than relying on every insertion site remembering to look first.

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

#### Scenario: Deduplication survives a restart

- **WHEN** an op is appended to a persistent log, the log is reopened, and the same op is appended again
- **THEN** the result reports it as already present
- **AND** the log holds one entry for it

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

An implementation whose storage can fail SHALL report that failure as a distinct outcome from an empty result, and SHALL NOT panic. An empty answer means the peer holds no matching ops; a failure means the store could not be consulted, and the two SHALL NOT be reported the same way.

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

#### Scenario: A storage failure is reported, not confused with emptiness

- **WHEN** a persistent log's storage is unusable
- **THEN** the operation reports a failure
- **AND** the failure is distinguishable from a log that legitimately holds no matching ops

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

### Requirement: The log's contract holds for every implementation

The behaviour every other requirement in this capability describes SHALL hold for each implementation of the log, whether it keeps ops in memory or persists them. No implementation SHALL be exempt from a requirement on the grounds of how it stores, except where a requirement itself distinguishes a log that keeps ops as their encoding from one that keeps them as values.

Where two implementations are given the same ops with the same recorded arrival metadata, and neither refuses any of them, every read defined here SHALL return the same sequence from each.

#### Scenario: Two implementations given the same ops read alike

- **WHEN** the same ops with the same recorded arrival metadata, none of which either log refuses, are appended to an in-memory log and to a persistent log
- **THEN** an unrestricted read returns the same sequence of ops from each
- **AND** a read restricted to a Stoa returns the same sequence from each
- **AND** a read restricted to a target returns the same sequence from each
- **AND** each reports the same count

#### Scenario: Re-arrival is resolved the same way by every implementation

- **WHEN** an op is appended twice with differing arrival metadata to an in-memory log and to a persistent log
- **THEN** each holds one entry for it
- **AND** each reports the arrival metadata recorded first

### Requirement: A persistent log survives the process that wrote it

An implementation that persists SHALL make every op it has stored readable by a log subsequently opened over the same storage, with the recorded arrival metadata and the defined read order unchanged.

#### Scenario: Ops outlive the log object that stored them

- **WHEN** ops are appended to a persistent log and the log is dropped
- **THEN** a log opened over the same storage holds every one of them
- **AND** reads return them in the same order as before

#### Scenario: Whether an arrival was ordered survives a restart

- **WHEN** a log holding both transport-ordered and unordered ops is reopened
- **THEN** each op is still reported with the ordering state it was recorded with
- **AND** the read order is unchanged

### Requirement: A persistent log declares the layout it was written with

An implementation that persists SHALL record which version of its storage layout it wrote, and SHALL refuse to operate over storage written under a version it does not understand, reporting that refusal by name.

It SHALL NOT read such storage on a best-effort basis. Ops are the authority for every piece of forum state, so a layout misread yields a forum state that is wrong with no error anywhere.

#### Scenario: Storage from an unknown layout version is refused

- **WHEN** a log is opened over storage declaring a layout version it does not understand
- **THEN** the open fails
- **AND** the failure names the version found and the version expected
- **AND** no op is read from that storage

#### Scenario: Storage this version wrote is accepted

- **WHEN** a log is opened over storage it previously wrote
- **THEN** the open succeeds
- **AND** every op stored is readable

### Requirement: A persistent log verifies the layout its declared version promises

A declared layout version is a claim about the storage beside it, not a fact about it. An implementation that persists SHALL, when opening storage declaring a version it understands, verify that the storage actually has that layout, and SHALL refuse the open by name when it does not.

It SHALL NOT defer that discovery to the first read. A store that opens successfully and then fails every read reports the failure as though the storage could not be reached, when the fact is that the storage is not the layout it declares — and those two call for different responses. The refusal SHALL be distinguishable from the refusal of an unknown layout version, because a reader told to find a build that understands the layout cannot act on that advice when the build in hand already declares it.

#### Scenario: Storage declaring a known version without that layout is refused at open

- **WHEN** a log is opened over storage declaring a layout version it understands, whose structure is absent or altered
- **THEN** the open fails
- **AND** the failure names the declared version and what was missing
- **AND** no op is read from that storage

### Requirement: A persistent log stores the inputs a ranking is computed from, never a ranking

An implementation that persists SHALL record, for each op, the values a later relevance projection computes from: the identity of the op's author, and the point in the ordering from which a score would decay.

It SHALL NOT store a score, a weight, a vote total, or any other value derived by combining ops. Such a value depends on facts that are not known when an op is stored — which identities are moderators is resolved on read from the Stoa's genesis record, and any per-reader weighting differs between two readers of the same store — so no single stored value could be correct for every read.

The recorded decay point SHALL be the Lamport counter the op carries in its own signed bytes, and SHALL be recorded as absent for an op that carries none. It SHALL NOT be taken from the arrival metadata recorded against the op, SHALL NOT be read from a local clock, and SHALL NOT be taken from the op's wall-clock field. Recorded arrival metadata and a locally-read time each differ between two peers that received the same op, which would make two peers holding identical ops rank them differently; the wall-clock field is a value its author chooses.

No read order defined by this capability SHALL consult these values, and the defined read order is unaffected by them.

#### Scenario: An op's author is recorded in a form a later ranking can group by

- **WHEN** an op is appended to a persistent log
- **THEN** the stored entry records which identity signed it
- **AND** that record is independent of whether the signer is currently a moderator

#### Scenario: No stored value combines two ops

- **WHEN** any number of ops naming one target are appended to a persistent log
- **THEN** no stored value counts, sums or weights them
- **AND** every stored value is a property of a single op

#### Scenario: The decay point is the op's own counter

- **WHEN** the same op is appended to two persistent logs at different moments, with differing recorded arrival metadata
- **THEN** both record the same decay point
- **AND** that decay point is the counter the op carries

#### Scenario: An op carrying no counter is recorded as having none

- **WHEN** an op carrying no counter, an op whose counter is zero and an op whose counter is the maximum representable value are appended to a persistent log
- **THEN** the op carrying no counter is recorded as having no decay point
- **AND** each of the other two is recorded with its own counter, distinguishable from the first and from each other

#### Scenario: Reserving for a ranking does not change what a read returns

- **WHEN** an in-memory log and a persistent log are given the same ops, none of which either refuses
- **THEN** every read returns the same sequence from each
- **AND** the persistent log's reserved values do not reorder it
