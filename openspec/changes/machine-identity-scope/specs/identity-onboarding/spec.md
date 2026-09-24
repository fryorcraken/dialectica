## MODIFIED Requirements

### Requirement: Keeping a candidate persists it, and keeping is one step

Keeping a candidate MUST store what is needed to reproduce that identity, such
that the identity is unchanged after a restart.

Keeping MUST either complete or change nothing. There MUST be no state in which
part of a choice was recorded.

An identity becomes real when it signs, and nothing signs during onboarding — so
before a candidate is kept there is nothing to lose, and after it is kept there
must be nothing missing.

**The one exception is a failure storage will not let the keep undo.** A keep
that stored a master key where none was stored, and then failed, MUST remove that
master key and MUST NOT remove one that was stored before it began. Where storage
refuses that removal, the keep has changed something it cannot take back, and its
reply MUST say so: it MUST carry a reason and no identity, and the reason MUST
state that a master key was left stored, so that it differs from the reason the
same failure gives when the master key was removed. A reply that read as "nothing
changed" would hide from the user a master key that the identity report then
names in every Stoa.

**The identity kept MUST be the candidate the offering reply displayed at the
position selected** — the same public key, not merely a candidate derived at the
same position. A selection is made on what the user was shown, and an offering and
a keep are two separate calls, so anything the offering depended on and the keep
re-established can differ between them while every guard on the selection still
passes. When it does, the user is given a working identity they never saw, which is
the outcome the requirement on malformed input calls unrecoverable.

The reply MUST state whether the candidate was kept, and where it was, MUST carry
the identity kept — its public key — and where it was not, MUST carry a reason and
no identity. This is the posting probe's shape rather than a second convention for
the same job, and it is what makes a caller able to show the user the identity they
kept without asking a second question.

**In this release what a keep stores is a choice for that Stoa and nothing more.**
It does not change the identity in use there, which is the machine key in every
Stoa (`identity`: *In this release one machine key is the identity in every
Stoa*). So "the identity kept" below means the key the stored choice reproduces,
read back from the master key and the recorded path — not the identity the
identity report names.

#### Scenario: The identity kept is the candidate that was displayed

- **WHEN** a set of candidates is offered, and a candidate is then kept at a selected
  position
- **THEN** the identity the keep reports has the public key the offering reply
  carried at that position
- **AND** this holds for every position in the set

#### Scenario: A kept identity survives a restart

- **WHEN** a candidate is kept, and the stored state is then loaded afresh
- **THEN** the key derived from the stored master key and the path recorded for
  that Stoa is the identity that was kept

#### Scenario: A kept identity survives more than one restart

- **WHEN** a candidate is kept and the stored state is loaded afresh twice
- **THEN** both loads reproduce the same identity from the master key and the
  recorded path

#### Scenario: A kept identity can sign as the identity it reported

- **WHEN** a candidate is kept and an op is then signed by the identity it yields
- **THEN** the op's author is the identity that keeping it reported

#### Scenario: A successful keep reports the identity it kept

- **WHEN** a candidate is kept
- **THEN** the reply states that it was kept
- **AND** carries that identity's public key
- **AND** carries no author address
- **AND** carries no reason

#### Scenario: A failed keep records nothing

- **WHEN** keeping a candidate fails, and storage permits removing any master key
  the keep stored
- **THEN** no identity is reported as kept
- **AND** a subsequent load finds no recorded choice and no master key that were
  not there before
- **AND** the reply carries a reason and no identity

#### Scenario: A failed keep that cannot remove the master key it stored says so

- **WHEN** keeping a candidate stores a master key where none was stored, then
  fails, and storage refuses to remove that master key
- **THEN** no identity is reported as kept
- **AND** the reply carries a reason and no identity
- **AND** the reason differs from the reason the same failure gives when the
  master key is removed

### Requirement: A chosen derivation path is recorded, because it cannot be recomputed

The derivation path a user chose for a Stoa MUST be recorded in storage, and that
record MUST survive a restart.

This is the cost of letting the user choose, and it is a new obligation rather
than an inherited one. Where derivation takes only a master key and a Stoa
address, an identity is recomputable from those two and nothing needs preserving
beyond the master key and the list of Stoas joined. A user-chosen path is a third
input that no value on the network carries, so an unrecorded path is an identity
that cannot be reproduced from any surviving material.

The record MUST be readable in full by the module, so that it can later be
exported and preserved somewhere other than the machine that made it. **Export
itself is not in this change** — the owner's sequence is local storage first,
export and remote backup later — so what is required here is that nothing about
the record's storage prevents it: no value in it may be derivable only on the
machine that wrote it, and none may be unreadable once written.

The record MUST NOT be required to be secret. It says which path was chosen and
reveals nothing that a published identity does not already reveal. This is why it
does not belong in the keystore file, whose every field is accounted for.

**In this release the record is not consulted by anything that signs or that
reports the identity in use.** It holds the choices kept, and it is preserved,
because a per-Stoa key that has signed an op can be reproduced from nothing else.

#### Scenario: A chosen path is readable after a restart

- **WHEN** a candidate is kept for a Stoa and the stored state is loaded afresh
- **THEN** the path that was chosen for that Stoa is readable

#### Scenario: The identity follows from the master key and the recorded path

- **WHEN** the recorded path for a Stoa is read, and derivation is performed from
  the master key and that path
- **THEN** the result is the identity that was kept for that Stoa

#### Scenario: Distinct choices for distinct Stoas are recorded separately

- **WHEN** different paths are chosen for two different Stoas
- **THEN** each Stoa's recorded path is the one chosen for it

#### Scenario: The record is fully readable once written

- **WHEN** paths have been recorded for several Stoas through this module
- **THEN** every recorded pairing of Stoa and path can be read back
- **AND** reading them requires nothing beyond the stored record itself

### Requirement: The interface can state that identity recovery needs the record

The module MUST report, to a caller asking for the identity in use, whether
recovering that identity needs more than the master key.

A user who believes their exported master key is a complete backup has been misled
by omission, and the caller cannot discover this for itself: it has no filesystem
access. So the statement has to come from the module.

**In this release the identity in use is the machine key, and the master key
alone recovers it.** The report MUST therefore state that recovery needs nothing
beyond the master key, for every Stoa, whatever the record of per-Stoa choices
holds. A choice recorded by a keep is recoverable only together with the record,
but it is not the identity in use in this release, so it is not what this report
describes.

**Remote backup and export are not part of this change.** This requirement is
confined to what is checkable now. A requirement covering the backed-up state
belongs to the change that implements backup, because until then there is no way
to reach that state and so no way to test a claim about it.

#### Scenario: The unbacked state is reportable

- **WHEN** the identity in use is reported for a Stoa with a recorded per-Stoa
  choice, and for a Stoa with none, and no export or remote backup exists
- **THEN** each reply states that recovering the identity needs nothing beyond
  the master key

  The scenario keeps its name because the name is how this delta addresses it.
  What it asserts inverts: the identity in use is the machine key, so the state
  reported is that the master key alone suffices — including for a Stoa whose
  recorded choice would itself need the record.

### Requirement: Keeping an identity does not replace an existing one

Where a choice is already recorded for a Stoa, keeping another candidate for that
Stoa MUST be refused rather than replacing it, and the refusal MUST be
distinguishable from other failures.

A master key exists in exactly one place. Replacing it silently discards every
identity derived from it, while the ops those identities signed remain published
and unreachable — and no error anywhere says it happened.

Replacing an identity deliberately is a separate operation and is not defined by
this capability.

#### Scenario: A second keep is refused

- **WHEN** a choice is recorded for a Stoa and a candidate is kept again for that
  Stoa
- **THEN** the attempt is refused
- **AND** the recorded choice and the stored master key are unchanged

#### Scenario: The refusal names the situation

- **WHEN** keeping is refused because a choice is already recorded for that Stoa
- **THEN** the reason is distinguishable from a malformed request and from a
  storage failure

### Requirement: The current identity is reportable, separately from whether posting is possible

The module MUST be able to report the identity in use in a Stoa, or that there is
none, together with a reason when there is none.

**In this release the identity in use is the machine key in every Stoa**
(`identity`: *In this release one machine key is the identity in every Stoa*).
Where a machine key is stored and readable, the report MUST name it, for every
Stoa, whether or not a per-Stoa choice is recorded for that Stoa and whether or
not the record of choices can be read. Where no machine key is stored, the report
MUST state that there is none, with a reason.

This is a different question from whether posting is possible, and the two can
honestly disagree: a stored identity whose keystore permissions are too open is a
real identity that cannot currently be used. A caller with only the posting probe
would have to render "you are nobody" to a user who has an identity and a fixable
problem.

The reply MUST carry an identity or a reason, never both and never neither —
matching the posting probe's shape rather than introducing a second convention for
the same job.

The reply MUST name the identity by its public key and MUST NOT carry an author
address. The public key is the sole author identifier, and it is what the generated
display name and the visual mark are both derived from.

#### Scenario: An existing identity is reported

- **WHEN** a machine key is stored and readable
- **THEN** the reply states that there is an identity
- **AND** carries the machine key's public key
- **AND** carries no author address
- **AND** carries no reason

#### Scenario: No per-Stoa choice is needed for an identity to be reported

- **WHEN** a machine key is stored and readable, and no choice is recorded for the
  Stoa asked about
- **THEN** the reply states that there is an identity
- **AND** carries the machine key's public key

#### Scenario: No identity is reported with a reason

- **WHEN** no machine key is stored
- **THEN** the reply states that there is none
- **AND** carries a reason
- **AND** carries no identity

#### Scenario: An unusable identity is distinguishable from an absent one

- **WHEN** an identity is stored but cannot be loaded
- **AND** separately, no identity is stored at all
- **THEN** the two replies carry different reasons

### Requirement: The derivation path is carried in the replies that name an identity

A reply describing a candidate and a reply reporting a kept identity MUST each
carry the derivation path that identity derives from.

The path is the one input to a per-Stoa identity that no published value carries
and that the module alone holds, so a caller that cannot see it cannot show the
user what must be preserved. The path is not secret: the record of it "reveals
nothing that a published identity does not already reveal".

**The reply reporting the identity in use MUST carry no path in this release.**
The identity in use is the machine key, which derives from no path, so any path
in that reply would describe a key that is not the one in use.

#### Scenario: A candidate carries its path

- **WHEN** a slate is generated
- **THEN** each candidate carries the derivation path it derives from

#### Scenario: A kept identity is reported with its path

- **WHEN** a candidate is kept
- **THEN** the reply carries the path of the candidate that was kept

#### Scenario: The identity in use is reported with its path

- **WHEN** the identity in use is asked for and there is one, for a Stoa with a
  recorded per-Stoa choice and for a Stoa with none
- **THEN** neither reply carries a derivation path

  The scenario keeps its name because the name is how this delta addresses it.
  What it asserts inverts: the identity in use derives from no path in this
  release, so the reply carries none — and a Stoa with a recorded choice is named
  because that is where a path would be available to report wrongly.

### Requirement: One device holds the master key, and the record has one writer

The master key MUST be held on a single device, and the record of chosen
derivation paths MUST have exactly one writer.

This is a deliberate scope limit rather than a property of the design, and taking
it buys the absence of a whole class of problem: with one writer there is no
reconciliation between divergent records, no question of which device's record is
authoritative, and no per-device path allocation to keep disjoint. A restore is a
restore onto that device, not a sync protocol between peers.

**Additional devices are not this capability's concern**, and the intended shape
is recorded so that this requirement is not mistaken for a claim that additional
devices are impossible: a second instance — a phone, or a daemon performing
storage backups — would generate its own key locally and have it approved by the
device holding the master key, so that the master key never leaves the one device.
Specifying that approval is a separate change with its own capability.

**In this release a restore brings back the choices kept, not the identity in
use.** The identity in use is the machine key in every Stoa (`identity`: *In this
release one machine key is the identity in every Stoa*), and a recorded choice
does not change it. A record restored beside a master key MUST reproduce each
kept choice from that master key and the recorded path, and MUST NOT change the
identity in use in any Stoa.

#### Scenario: The record is written by one writer

- **WHEN** chosen paths are recorded
- **THEN** the module is the only writer of that record
- **AND** no reconciliation between two versions of it is required

#### Scenario: A restore targets the device holding the master key

- **WHEN** a record is restored alongside a master key
- **THEN** the key derived from the restored master key and the path the restored
  record names for a Stoa is the identity that was kept for that Stoa
- **AND** the identity in use in that Stoa is the restored master key's machine
  key, not the key the restored record names
- **AND** no other device's record participates

  The scenario keeps its name because the name is how this delta addresses it.
  It previously asserted that the identities in use are those the record names;
  in this release the record names kept choices, and the identity in use is the
  machine key whatever the record holds.

### Requirement: Every entry point refuses malformed input rather than guessing

Each method of this capability MUST reject input it cannot interpret, and MUST
distinguish an absent field from one of the wrong type.

A caller selecting a candidate supplies a value naming one. Coercing an
out-of-range or wrongly-typed selection to a default would store an identity the
user did not choose — which is unrecoverable, because the choice cannot be
recomputed.

A selection that does not name a candidate in the current set MUST be refused,
and MUST NOT be satisfied by any other candidate.

The module MUST NOT abort on any input. A failure on this path returns an error;
it does not crash the process serving every other call.

#### Scenario: A selection outside the current set is refused

- **WHEN** a candidate is selected that the current set does not contain
- **THEN** the attempt is refused
- **AND** no choice is recorded and no master key is stored that were not there
  before

#### Scenario: A selection made against a superseded set is refused

- **WHEN** a slate is generated, then regenerated, and a candidate from the first
  set is selected
- **THEN** the attempt is refused rather than storing a candidate from the second

#### Scenario: A malformed request is refused by kind

- **WHEN** a request is not valid JSON, or omits a required field, or supplies one
  of the wrong type
- **THEN** the reply is the failure shape
- **AND** carries no result alongside it

#### Scenario: No input aborts the module

- **WHEN** any method of this capability is called with arbitrary input
- **THEN** it returns a reply rather than terminating the process

### Requirement: Nothing is stored before a candidate is kept

Generating a slate MUST NOT write to storage.

A slate that persisted would record choices the user has not made, and on the
derivation-path model there is nothing to persist in any case: the paths are
reproducible from the master key. If the module stops between generating a slate
and keeping a candidate, nothing is lost, because nothing was published and no
identity existed to lose.

#### Scenario: Generating a slate writes nothing

- **WHEN** a slate is generated and no candidate is kept
- **THEN** no master key and no recorded choice are stored that were not there
  before
- **AND** on a peer that held no master key, a caller asking who the user is
  still finds none

#### Scenario: A discarded slate leaves no trace

- **WHEN** a slate is generated and then superseded by another
- **THEN** no record of the first remains in storage
