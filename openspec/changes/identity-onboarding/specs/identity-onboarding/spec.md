## Purpose

Defines how a user comes to have an identity: what a slate of candidates is, what
keeping one guarantees, what must be recorded so the choice survives, and what the
replies may and may not carry — so that a fresh install can reach a state where
posting is possible, without a secret crossing the module boundary and without a
choice the user made becoming unrecoverable.

**Boundary with other capabilities.** `identity` owns what an identity is, how a
key derives and what an address is; `keystore` owns how a root secret is stored at
rest, what unlocks it and what is refused; `posting-capability` owns the probe that
gates posting. This capability owns only the act of *acquiring* an identity, and
deliberately restates none of the three.

## ADDED Requirements

### Requirement: A user is offered several candidate identities and chooses one

The module SHALL be able to present a set of candidate identities for the user to
choose between, and SHALL be able to present a different set on request, without
limit.

A generated identity cannot be chosen the way a username can — there is no
registry to hold a typed name and a typed name carried between Stoas would defeat
cross-Stoa unlinkability. Offering a set and allowing regeneration is what makes
the result a choice the user meant rather than a value they were handed.

The number of candidates in a set SHALL be fixed by the implementation and
reported with the set, rather than requested by the caller. A caller-supplied
count is a number that decides how much key derivation this module performs.

#### Scenario: A set of candidates is offered

- **WHEN** a slate is requested
- **THEN** the reply carries the fixed number of candidates
- **AND** states that number alongside them

#### Scenario: Every candidate in a set is distinct

- **WHEN** a slate is requested
- **THEN** no two candidates in it share a public key
- **AND** no two share an address

#### Scenario: Requesting another set yields different candidates

- **WHEN** a slate is requested, and then a slate is requested again
- **THEN** no candidate in the second set has a public key from the first

#### Scenario: Regeneration is not limited

- **WHEN** a slate is requested many times in succession
- **THEN** each request is answered
- **AND** none is refused on the ground of how many preceded it

### Requirement: A candidate is a derivation path over one master key

The candidates in a slate SHALL be derived from a single master key, and SHALL
differ from one another by their derivation path alone.

What the user is choosing is which path becomes their identity. Deriving several
independent master keys instead would give the user several secrets to preserve
where one suffices, and would discard all but one of them.

The master key SHALL be generated locally, and SHALL be exportable by the user, so
that one saved value is sufficient to reproduce every identity derived from it —
subject to the recorded path, below.

Derivation SHALL remain that of the `identity` capability. This capability adds
the path as an input to it and SHALL NOT introduce a second derivation scheme.

#### Scenario: Candidates come from one master key

- **WHEN** a slate is generated
- **THEN** every candidate derives from the same master key
- **AND** no candidate requires a master key the others do not

#### Scenario: A candidate is reproducible from the master key and its path

- **WHEN** a candidate is derived, and derivation is performed again from the same
  master key and the same path
- **THEN** the two derivations yield the same public key

#### Scenario: Different paths give different identities

- **WHEN** two candidates with different paths are derived from one master key
- **THEN** their public keys differ

### Requirement: A slate reply carries public values only

A reply describing candidates SHALL carry, for each candidate, values sufficient
to identify and display it, and SHALL NOT carry a secret key, a master key, a seed,
a mnemonic, or any value from which one could be reconstructed.

The view has no use for a secret: it cannot sign, because signing is the module's.
Widening a reply later is additive, while a secret that has crossed the module
boundary cannot be recalled.

A candidate's address SHALL be present, because an address is the only unforgeable
way to tell two candidates apart. A candidate's public key SHALL be present,
because the generated display name is derived from the public key rather than from
the address.

#### Scenario: No secret appears in a slate reply

- **WHEN** a slate is generated
- **THEN** no field of the reply contains the master key's bytes
- **AND** no field contains any candidate's secret key bytes

#### Scenario: A candidate carries an address and a public key

- **WHEN** a slate is generated
- **THEN** each candidate carries an address
- **AND** each carries a public key

#### Scenario: A slate reply value cannot sign

- **WHEN** every value in a slate reply is taken as key material
- **THEN** none of them yields a signature that verifies under any candidate's
  public key

### Requirement: Keeping a candidate persists it, and keeping is one step

Keeping a candidate SHALL store what is needed to reproduce that identity, such
that the identity is unchanged after a restart.

Keeping SHALL either complete or change nothing. There SHALL be no state in which
part of a choice was recorded.

An identity becomes real when it signs, and nothing signs during onboarding — so
before a candidate is kept there is nothing to lose, and after it is kept there
must be nothing missing.

#### Scenario: A kept identity survives a restart

- **WHEN** a candidate is kept, and the stored state is then loaded afresh
- **THEN** the identity reported is the one that was kept

#### Scenario: A kept identity survives more than one restart

- **WHEN** a candidate is kept and the stored state is loaded afresh twice
- **THEN** both loads report the same identity

#### Scenario: A kept identity can sign as the identity it reported

- **WHEN** a candidate is kept and an op is then signed by the identity it yields
- **THEN** the op's author is the identity that keeping it reported

#### Scenario: A failed keep records nothing

- **WHEN** keeping a candidate fails
- **THEN** no identity is reported as kept
- **AND** a subsequent load finds no identity that was not there before

### Requirement: A chosen derivation path is recorded, because it cannot be recomputed

The derivation path a user chose for a Stoa SHALL be recorded in storage, and that
record SHALL survive a restart.

This is the cost of letting the user choose, and it is a new obligation rather
than an inherited one. Where derivation takes only a master key and a Stoa
address, an identity is recomputable from those two and nothing needs preserving
beyond the master key and the list of Stoas joined. A user-chosen path is a third
input that no value on the network carries, so an unrecorded path is an identity
that cannot be reproduced from any surviving material.

The record SHALL be readable in full by the module, so that it can later be
exported and preserved somewhere other than the machine that made it. **Export
itself is not in this change** — the owner's sequence is local storage first,
export and remote backup later — so what is required here is that nothing about
the record's storage prevents it: no value in it may be derivable only on the
machine that wrote it, and none may be unreadable once written.

The record SHALL NOT be required to be secret. It says which path was chosen and
reveals nothing that a published identity does not already reveal. This is why it
does not belong in the keystore file, whose every field is accounted for.

#### Scenario: A chosen path is readable after a restart

- **WHEN** a candidate is kept for a Stoa and the stored state is loaded afresh
- **THEN** the path that was chosen for that Stoa is readable

#### Scenario: The identity follows from the master key and the recorded path

- **WHEN** the recorded path for a Stoa is read, and derivation is performed from
  the master key and that path
- **THEN** the result is the identity in use for that Stoa

#### Scenario: Distinct choices for distinct Stoas are recorded separately

- **WHEN** different paths are chosen for two different Stoas
- **THEN** each Stoa's recorded path is the one chosen for it

#### Scenario: The record is fully readable once written

- **WHEN** paths have been recorded for several Stoas
- **THEN** every recorded pairing of Stoa and path can be read back
- **AND** reading them requires nothing beyond the stored record itself

### Requirement: One device holds the master key, and the record has one writer

The master key SHALL be held on a single device, and the record of chosen
derivation paths SHALL have exactly one writer.

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

#### Scenario: The record is written by one writer

- **WHEN** chosen paths are recorded
- **THEN** the module is the only writer of that record
- **AND** no reconciliation between two versions of it is required

#### Scenario: A restore targets the device holding the master key

- **WHEN** a record is restored alongside a master key
- **THEN** the identities in use are those the record names
- **AND** no other device's record participates

### Requirement: The interface can state that identity recovery needs the record

The module SHALL make available, to a caller, that a master key alone is not
sufficient to recover the user's identities while the recorded paths exist only in
local storage.

A user who believes their exported master key is a complete backup has been misled
by omission, and the caller cannot discover this for itself: it has no filesystem
access. So the honest statement has to come from the module.

**Remote backup and export are not part of this change** — the owner's sequence is
local storage first, export and remote backup later. This requirement is therefore
confined to what is checkable now: that the module reports the unbacked state. A
requirement covering the backed-up state belongs to the change that implements
backup, because until then there is no way to reach that state and so no way to
test a claim about it.

#### Scenario: The unbacked state is reportable

- **WHEN** paths have been recorded and no export or remote backup exists
- **THEN** a caller asking whether recovery needs more than the master key is told
  that it does

### Requirement: Keeping an identity does not replace an existing one

Where an identity is already stored, keeping a candidate SHALL be refused rather
than replacing it, and the refusal SHALL be distinguishable from other failures.

A master key exists in exactly one place. Replacing it silently discards every
identity derived from it, while the ops those identities signed remain published
and unreachable — and no error anywhere says it happened.

Replacing an identity deliberately is a separate operation and is not defined by
this capability.

#### Scenario: A second keep is refused

- **WHEN** an identity is stored and a candidate is kept again
- **THEN** the attempt is refused
- **AND** the stored identity is unchanged

#### Scenario: The refusal names the situation

- **WHEN** keeping is refused because an identity already exists
- **THEN** the reason is distinguishable from a malformed request and from a
  storage failure

### Requirement: The current identity is reportable, separately from whether posting is possible

The module SHALL be able to report the identity in use, or that there is none,
together with a reason when there is none.

This is a different question from whether posting is possible, and the two can
honestly disagree: a stored identity whose keystore permissions are too open is a
real identity that cannot currently be used. A caller with only the posting probe
would have to render "you are nobody" to a user who has an identity and a fixable
problem.

The reply SHALL carry an identity or a reason, never both and never neither —
matching the posting probe's shape rather than introducing a second convention for
the same job.

The reply SHALL carry the public key as well as the address, because the generated
display name is derived from the public key.

#### Scenario: An existing identity is reported

- **WHEN** an identity is stored and readable
- **THEN** the reply states that there is an identity
- **AND** carries its address and its public key
- **AND** carries no reason

#### Scenario: No identity is reported with a reason

- **WHEN** no identity is stored
- **THEN** the reply states that there is none
- **AND** carries a reason
- **AND** carries no identity

#### Scenario: An unusable identity is distinguishable from an absent one

- **WHEN** an identity is stored but cannot be loaded
- **AND** separately, no identity is stored at all
- **THEN** the two replies carry different reasons

### Requirement: The protection applied to a stored master key is reported

When a master key is stored, the module SHALL report to the caller whether it was
encrypted.

The keystore encrypts under a supplied passphrase and stores in the clear when
none is supplied, and it records which. A caller that is not told cannot say which
of the two happened, and "the secret on your disk is in the clear" is not a fact a
user should have to read the source to learn.

This requirement is about reporting, not about which protection to apply. **Whether
a passphrase is obtained, and how, is deliberately not settled** — the owner has
deferred it, and this capability requires only that whichever protection applies be
recorded in the file and reportable, so that an unencrypted keystore is a state the
interface can name rather than a silent default.

Two things are worth stating so that the question is not later thought answered
when it is not:

- **A single fixed passphrase compiled into the build is not an option**, however
  it is labelled. It would be identical across every install and readable in the
  source, so it defends a stolen keystore file against nobody — while the file
  records itself as encrypted and every inspection of it, including this project's
  own, reports it as protected. The failure mode is not weak protection but
  protection that reads as strong, which is worse than `Protection::None`: an
  audit can see the latter.
- **Approving a second device's key does not resolve this.** The two concern
  different things. A second device generating its own key and having it approved
  is about *authorisation* — how another instance signs without holding the master
  key — and it usefully reduces how many devices hold the master key at all. But
  the master key still rests encrypted on the one device that has it, and what it
  is encrypted under is untouched by anything a second device does. The questions
  are adjacent and independent.

#### Scenario: An encrypted store is reported as encrypted

- **WHEN** a master key is stored and a passphrase was available
- **THEN** the reply states that it was encrypted

#### Scenario: An unencrypted store is reported as unencrypted

- **WHEN** a master key is stored and no passphrase was available
- **THEN** the reply states that it was not encrypted

### Requirement: Every entry point refuses malformed input rather than guessing

Each method of this capability SHALL reject input it cannot interpret, and SHALL
distinguish an absent field from one of the wrong type.

A caller selecting a candidate supplies a value naming one. Coercing an
out-of-range or wrongly-typed selection to a default would store an identity the
user did not choose — which is unrecoverable, because the choice cannot be
recomputed.

A selection that does not name a candidate in the current set SHALL be refused,
and SHALL NOT be satisfied by any other candidate.

No input SHALL cause the module to abort. A failure on this path returns an error;
it does not crash the process serving every other call.

#### Scenario: A selection outside the current set is refused

- **WHEN** a candidate is selected that the current set does not contain
- **THEN** the attempt is refused
- **AND** no identity is stored

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

Generating a slate SHALL NOT write to storage.

A slate that persisted would record choices the user has not made, and on the
derivation-path model there is nothing to persist in any case: the paths are
reproducible from the master key. If the module stops between generating a slate
and keeping a candidate, nothing is lost, because nothing was published and no
identity existed to lose.

#### Scenario: Generating a slate writes nothing

- **WHEN** a slate is generated and no candidate is kept
- **THEN** no identity is stored
- **AND** a caller asking who the user is still finds none

#### Scenario: A discarded slate leaves no trace

- **WHEN** a slate is generated and then superseded by another
- **THEN** no record of the first remains in storage

### Requirement: Slate material in memory is cleared when discarded

Secret material held while a slate exists SHALL be cleared when that slate is
discarded or superseded.

`keystore` requires this of the root secret, the derived encryption key and the
passphrase. This extends the same obligation to material this capability holds,
because a slate is the one place several candidates exist at once — and a
discarded candidate's material has no further use, so retaining it is exposure
with no compensating benefit.

#### Scenario: Discarded slate material does not persist in its buffer

- **WHEN** a slate holding secret material is discarded
- **THEN** the buffer that held it is overwritten rather than left with it in place

### Requirement: A generated name and a mark are not settled by this capability

This capability SHALL NOT define how a display name or a visual mark is derived
from a key.

A slate carries public keys, and a name derived from a public key is computable by
anything holding one. Which words, which wordlist, which denylist and which glyph
are a separate contract, and a slate that specified them would make every change to
either a change to this one.

What this capability does require is that the values a name and a mark are derived
from — the public key and the address — are present in a slate reply, which the
requirement on reply contents states.

A reply of this capability SHALL therefore carry no display name and no visual
mark, for any candidate and for the identity in use. Carrying one would settle
here what a separate contract is to settle, and a caller written against it would
be written against a name this capability never defined.

#### Scenario: No reply carries a name or a mark

- **WHEN** a slate is generated, and separately the identity in use is asked for
- **THEN** no candidate in the slate reply carries a display name or a visual mark
- **AND** the reply describing the identity in use carries neither
- **AND** both replies still carry the public key and the address a name and a
  mark would be derived from
