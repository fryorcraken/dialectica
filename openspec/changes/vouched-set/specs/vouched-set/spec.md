## Purpose

Defines what owns a reader's private weight state — the identities whose assessments this reader weighs more heavily — how that state relates to the op log and to replay, what it resolves to for an identity the reader holds no ops for, how it fails when it cannot be read, and what a view may ask of it.

Weight state has two provenances and they behave differently. A **declared** vouch is authored by the reader and never published, so nothing can reproduce it. **Earned** weight accrues from the reader's own assessment ops, which are published and therefore replayable. A **dismissal** of earned weight is authored by the reader and never published, like a declared vouch, because the assessments that earned the weight remain valid ops that replay would otherwise honour again.

## ADDED Requirements

### Requirement: Weight state is never published

No vouch, dismissal or weight value SHALL be encoded into any op, placed on any channel, or made observable to any peer. The system SHALL provide no operation by which one reader learns any part of another reader's weight state.

A published vouch graph re-links pseudonyms that are unlinkable across Stoas by construction, and does so using the reader's own social graph — a worse disclosure than the one that unlinkability prevents. It would also be a sybil amplifier: minted identities vouching for each other would manufacture standing. Kept local, an attacker can affect only their own ranking, which is not an attack.

#### Scenario: No op kind expresses a vouch

- **WHEN** the set of op kinds the system can encode is enumerated
- **THEN** none of them expresses a vouch, a dismissal, or a weight

#### Scenario: Weight state does not reach the network

- **WHEN** a reader vouches for an identity, and then dismisses an earned weight
- **THEN** no message is published as a result of either

### Requirement: Weight state is scoped to one Stoa

Every vouch and every dismissal SHALL name an identity in exactly one Stoa and SHALL apply only within that Stoa. No stored collection SHALL span more than one Stoa, and no operation SHALL accept or return weight state for more than one Stoa at a time.

A collection spanning Stoas is a cross-Stoa list of the reader's own judgements, which is the artefact the never-published requirement exists to prevent, reached by another route. Single-Stoa scoping is what bounds any disclosure to the one Stoa it concerns.

#### Scenario: A vouch applies only in the Stoa it names

- **WHEN** a reader vouches for a key in one Stoa, and the same key is present in another Stoa
- **THEN** the identity is weighted in the first Stoa
- **AND** the identity is not weighted in the second

#### Scenario: Every operation names its Stoa

- **WHEN** any operation on weight state is invoked
- **THEN** it names exactly one Stoa
- **AND** it returns nothing about any other Stoa

### Requirement: Declared vouches and dismissals survive a rebuild of the derived view

A rebuild of the materialised view from the op log SHALL leave every declared vouch and every dismissal unchanged. Nothing that can be reproduced from the op log SHALL be required to reproduce them, and no rebuild SHALL be able to remove them as a step in its ordinary operation.

Ops are the authority and the view is a cache, but a declared vouch has no op and a dismissal contradicts one. Neither is reproducible from anything, and losing them is silent: a lost vouch presents as an identity being plain, which is the state of every uncurated identity, and a lost dismissal presents as a weight the reader already removed coming back. A reader cannot distinguish either from the ordinary movement of a feed as ops arrive.

#### Scenario: A rebuild preserves declared vouches

- **WHEN** a reader holds declared vouches and the view is rebuilt from the op log
- **THEN** every declared vouch is still held
- **AND** each still names the same identity and the same Stoa

#### Scenario: A rebuild preserves dismissals

- **WHEN** a reader has dismissed an earned weight and the view is rebuilt from the op log
- **THEN** that identity does not carry earned weight after the rebuild
- **AND** this holds even though the assessments that earned it are still in the log

#### Scenario: A rebuild recomputes earned weight

- **WHEN** a reader has earned weight for an identity and the view is rebuilt from the op log
- **THEN** earned weight for that identity is present again
- **AND** it was recomputed from the reader's own assessment ops rather than carried across

### Requirement: Earned weight is computed from the reader's own assessments only

Earned weight SHALL be computed only from assessment ops authored by the reader's own identity in the Stoa concerned. Assessments by any other identity SHALL contribute nothing to it, and the response axis SHALL contribute nothing to it.

Weight earned from agreement builds a machine that finds a reader more of what they already think. Weight earned from assessed quality can raise the standing of someone the reader consistently disagrees with and consistently finds worth reading, which is the thing the separation of the two axes exists to make reachable. Weight earned from other readers' assessments would be a vouch graph assembled locally out of published ops, which the never-published requirement forbids in substance.

#### Scenario: Another reader's assessments earn nothing

- **WHEN** an identity is assessed as constructive many times by readers other than this one
- **THEN** that identity carries no earned weight for this reader

#### Scenario: The response axis earns nothing

- **WHEN** the reader records agreement with an identity's posts and records no assessment of them
- **THEN** that identity carries no earned weight

#### Scenario: Disagreement does not prevent earning

- **WHEN** the reader assesses an identity's posts as constructive and records disagreement with each of them
- **THEN** that identity carries earned weight

### Requirement: A vouch for an identity the reader holds no ops for is recorded and effective

A vouch naming a well-formed identity SHALL be recorded whether or not the reader holds any op authored by that identity, and the identity SHALL resolve as vouched from that moment. Resolution SHALL NOT consult the op log to decide whether a declared vouch applies.

Two peers routinely hold different sets of ops, so an identity the reader has seen nothing from is the ordinary case rather than an error. Refusing such a vouch would make the same action succeed or fail according to whether a post had propagated yet, which is a per-peer accident, and would foreclose vouching for someone named to the reader out of band.

#### Scenario: A vouch for an unseen identity is recorded

- **WHEN** a reader vouches for a well-formed identity no op in the log names
- **THEN** the vouch succeeds
- **AND** the identity is reported as declared in that Stoa's list

#### Scenario: Losing an identity's ops does not revoke a vouch

- **WHEN** a reader holds a declared vouch and the log no longer holds any op by that identity
- **THEN** the identity is still reported as declared

#### Scenario: A malformed identity is refused

- **WHEN** a vouch names something that is not a well-formed identity
- **THEN** the reply carries an error
- **AND** no vouch is recorded

### Requirement: Unreadable weight state yields no weights, is reported distinctly, and is not written over

When weight state cannot be read, every identity SHALL resolve as plain and ranking SHALL proceed. The condition SHALL be reported through a state distinguishable from empty weight state. While the condition persists, any operation that would write weight state SHALL be refused with an error that names a fix, and SHALL NOT replace what could not be read.

Failing closed would refuse a service unrelated to the failure: the reader could not read the Stoa at all, because ranking could not complete, on account of a preference whose only function is to reorder a list that reads perfectly well unordered. Resolving everything as plain grants nothing — it is the state of every new reader — so the failure is safe as well as correct.

What makes it correct depends on the other two clauses. Folding the failure into "you have no vouches" leaves a reader who vouched for twelve identities unable to tell which happened. Treating the unreadable state as empty **on the write path** would replace it with whatever is written next, turning a recoverable read failure into permanent loss.

#### Scenario: Ranking proceeds with everyone plain

- **WHEN** weight state cannot be read and a Stoa is ranked
- **THEN** the ranking completes
- **AND** every identity is weighted as plain

#### Scenario: Unreadable is distinguishable from empty

- **WHEN** weight state cannot be read
- **THEN** the reported state differs from the state reported for a reader who holds no weight state at all

#### Scenario: A write against unreadable state is refused

- **WHEN** weight state cannot be read and a vouch, unvouch or dismissal is attempted
- **THEN** the reply carries an error
- **AND** the unreadable state is left as it was

#### Scenario: The error names a fix

- **WHEN** an operation is refused because weight state cannot be read
- **THEN** the message names an action that addresses the condition

### Requirement: No operation reports weight state by anyone other than the caller

No operation SHALL take an identity and report anything about weight held by a reader other than the caller, SHALL report a count or any aggregate of vouches across readers, or SHALL take a parameter naming whose weight state to read.

Vouching confers no status on the person vouched for. A count, or an answer to "who weighs this identity", is a published vouch graph reconstructed by the caller, with every problem the never-published requirement exists to avoid. The absence of a per-identity probe is the mechanism: given one, a caller reconstructs a per-author display by iterating a participant list, from a surface that never refused anything.

#### Scenario: There is no per-identity probe

- **WHEN** the operations on weight state are enumerated
- **THEN** none of them takes an identity and answers whether it is weighted

#### Scenario: There is no count

- **WHEN** the operations on weight state are enumerated
- **THEN** none of them reports how many readers weight a given identity

#### Scenario: No operation names a reader

- **WHEN** any operation on weight state is invoked
- **THEN** it takes no parameter identifying which reader's state to act on

### Requirement: A reader may read its own weight state, with each entry's provenance

The reader SHALL be able to list the identities carrying weight in one Stoa, paginated, and each entry SHALL report whether its weight is declared, earned, or both. The listing SHALL report no numeric weight.

Declared and earned must stay distinguishable: one is something the reader chose and can revoke, the other is something they should be told has happened and be able to undo, and a view given a flat list of keys has no basis to say which. Both at once is a real state rather than an edge case — a reader may declare a vouch for an identity that is also accruing — and collapsing it would hide that un-declaring leaves the accrual in place.

A numeric weight is a number a view can render beside an author, which is the display this specification forbids in all but name. The provenance flag carries what the reader must be told and no more.

Without a listing the state is write-only: a reader cannot audit it, cannot be told what has accrued, and cannot unvouch an identity whose key they no longer hold, because unvouching requires the key.

#### Scenario: The listing is paginated

- **WHEN** a reader lists its weight state for a Stoa
- **THEN** the reply carries a page of items, the page number, and whether more remain

#### Scenario: A declared vouch is reported as declared

- **WHEN** a reader vouches for an identity that has earned nothing, and lists its weight state
- **THEN** that entry is reported as declared and not as earned

#### Scenario: An accrual is reported as earned

- **WHEN** an identity carries earned weight and no declared vouch, and the reader lists its weight state
- **THEN** that entry is reported as earned and not as declared

#### Scenario: An identity that is both is reported as both

- **WHEN** a reader declares a vouch for an identity that also carries earned weight
- **THEN** the entry reports both provenances

#### Scenario: Un-declaring leaves an accrual in place

- **WHEN** a reader unvouches an identity that also carries earned weight
- **THEN** the entry is still listed
- **AND** it is reported as earned and not as declared

#### Scenario: No numeric weight is reported

- **WHEN** a reader lists its weight state
- **THEN** no entry carries a numeric weight

### Requirement: Vouching, unvouching and dismissing are idempotent

Vouching for an identity already vouched for, unvouching one not vouched for, and dismissing an accrual already dismissed SHALL each succeed and SHALL leave the state as though the operation had been applied once.

A view that has to know whether it already sent one of these — across a restart, a reconnect, or a user pressing a button twice — is a view carrying state the module owns. A user pressing a button twice is ordinary use, not an error to report.

#### Scenario: Vouching twice is one vouch

- **WHEN** a reader vouches for one identity twice
- **THEN** both calls succeed
- **AND** the listing reports that identity once

#### Scenario: Unvouching an identity that is not vouched for succeeds

- **WHEN** a reader unvouches an identity it has not vouched for
- **THEN** the call succeeds
- **AND** the listing is unchanged

#### Scenario: Dismissing twice is one dismissal

- **WHEN** a reader dismisses an accrual twice
- **THEN** both calls succeed
- **AND** the identity carries no earned weight

### Requirement: Weight state moves between a user's own devices only by explicit export, one Stoa at a time

Weight state SHALL NOT be transferred automatically, over any transport, or as a side effect of any other operation. An export SHALL be produced only by an explicit request, SHALL cover exactly one Stoa, SHALL name that Stoa, SHALL carry the reader's declared vouches and dismissals, and SHALL carry no secret key or material derived from one. An import SHALL be refused when the artefact names a Stoa other than the one being imported into.

An artefact spanning Stoas is the cross-Stoa list of a reader's own judgements that the never-published requirement exists to prevent, produced by this system, at rest, for a user to mishandle. Naming the Stoa is what makes a wrong-Stoa import refusable rather than a silent application of keys from elsewhere; refusal rather than a partial merge, because a wrong-Stoa import is either a mistake or an attack and neither has a sensible partial outcome.

Dismissals must travel with the vouches. A second device holds the same assessment ops, so without them it re-earns weight the reader dismissed, which is the same silent restoration the dismissal exists to prevent arriving across devices instead of across a rebuild.

Earned weight itself need not travel: it is recomputed from assessment ops the second device already receives.

#### Scenario: Export happens only on request

- **WHEN** a reader vouches, unvouches, dismisses, or reads weight state
- **THEN** no export is produced

#### Scenario: An export covers one Stoa and names it

- **WHEN** a reader exports weight state
- **THEN** the artefact covers exactly one Stoa
- **AND** it names that Stoa

#### Scenario: An export carries dismissals

- **WHEN** a reader who has dismissed an accrual exports weight state
- **THEN** the artefact carries that dismissal

#### Scenario: An export carries no secret material

- **WHEN** a reader exports weight state
- **THEN** the artefact carries no secret key and nothing derived from the root secret

#### Scenario: An import naming another Stoa is refused

- **WHEN** an artefact naming one Stoa is imported into another
- **THEN** the reply carries an error
- **AND** no vouch or dismissal from the artefact is applied
