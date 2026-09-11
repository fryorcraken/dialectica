## MODIFIED Requirements

### Requirement: The log records what arrived, and decides nothing about it

The log SHALL store every op appended to it, together with the transport metadata recorded on its arrival. It SHALL NOT verify a signature, check an authorisation, or reject an op on the basis of its content when appending.

Verification is the reader's job.

An implementation that persists SHALL apply this identically. Persistence SHALL NOT become an occasion to filter: a store that refused to write an op it could not verify would make a forgery indistinguishable from an op that never arrived, which is the same defect whether the store is in memory or on disk.

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

## ADDED Requirements

### Requirement: The log's contract holds for every implementation

The behaviour every other requirement in this capability describes SHALL hold for each implementation of the log, whether it keeps ops in memory or persists them. No implementation SHALL be exempt from a requirement on the grounds of how it stores.

Where two implementations are given the same ops with the same recorded arrival metadata, every read defined here SHALL return the same sequence from each.

#### Scenario: Two implementations given the same ops read alike

- **WHEN** the same ops with the same recorded arrival metadata are appended to an in-memory log and to a persistent log
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

The recorded decay point SHALL be taken from the transport's ordering metadata, and SHALL NOT be read from a local clock. A locally-read time differs between two peers that received the same op, which would make two peers holding identical ops rank them differently.

No ordering defined by this capability SHALL consult these values. Until a relevance change lands they carry no meaning, and the defined read order is unaffected by them.

#### Scenario: An op's author is recorded in a form a later ranking can group by

- **WHEN** an op is appended to a persistent log
- **THEN** the stored entry records which identity signed it
- **AND** that record is independent of whether the signer is currently a moderator

#### Scenario: No stored value combines two ops

- **WHEN** any number of ops naming one target are appended to a persistent log
- **THEN** no stored value counts, sums or weights them
- **AND** every stored value is a property of a single op

#### Scenario: The decay point does not come from a local clock

- **WHEN** the same op with the same recorded arrival metadata is appended to two persistent logs at different moments
- **THEN** both record the same decay point
- **AND** an op whose arrival carried no ordering metadata is distinguishable from one whose arrival placed it at the ordering's origin

#### Scenario: Reserving for a ranking does not change what a read returns

- **WHEN** an in-memory log and a persistent log are given the same ops
- **THEN** every read returns the same sequence from each
- **AND** the persistent log's reserved values do not reorder it
