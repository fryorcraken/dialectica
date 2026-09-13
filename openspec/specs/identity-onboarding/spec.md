# identity-onboarding Specification

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

## Requirements

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

Derivation SHALL remain that of the `identity` capability, which owns it. This
capability adds the path as an input and SHALL NOT define a derivation of its own.
That `identity` therefore carries a derivation taking a path alongside the one
taking none — and that the two must be distinguishable so neither silently
reproduces the other — is stated there rather than here.

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
Adding a field later is a change to this contract and can be made; a secret that has
crossed the module boundary cannot be recalled. The asymmetry is the whole argument
for starting narrow, and the requirement below that each reply's field set is closed
is what keeps "later" meaning a decision rather than a side effect.

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

**The identity kept SHALL be the candidate the offering reply displayed at the
position selected** — the same public key and the same address, not merely a
candidate derived at the same position. A selection is made on what the user was
shown, and an offering and a keep are two separate calls, so anything the offering
depended on and the keep re-established can differ between them while every guard on
the selection still passes. When it does, the user is given a working identity they
never saw, which is the outcome the requirement on malformed input calls
unrecoverable.

The reply SHALL state whether the candidate was kept, and where it was, SHALL carry
the identity kept — its address and its public key — and where it was not, SHALL
carry a reason and no identity. This is the posting probe's shape rather than a
second convention for the same job, and it is what makes a caller able to show the
user the identity they now have without asking a second question.

#### Scenario: The identity kept is the candidate that was displayed

- **WHEN** a set of candidates is offered, and a candidate is then kept at a selected
  position
- **THEN** the identity the keep reports has the public key the offering reply
  carried at that position
- **AND** has the address the offering reply carried at that position
- **AND** this holds for every position in the set

#### Scenario: A kept identity survives a restart

- **WHEN** a candidate is kept, and the stored state is then loaded afresh
- **THEN** the identity reported is the one that was kept

#### Scenario: A kept identity survives more than one restart

- **WHEN** a candidate is kept and the stored state is loaded afresh twice
- **THEN** both loads report the same identity

#### Scenario: A kept identity can sign as the identity it reported

- **WHEN** a candidate is kept and an op is then signed by the identity it yields
- **THEN** the op's author is the identity that keeping it reported

#### Scenario: A successful keep reports the identity it kept

- **WHEN** a candidate is kept
- **THEN** the reply states that it was kept
- **AND** carries that identity's address and public key
- **AND** carries no reason

#### Scenario: A failed keep records nothing

- **WHEN** keeping a candidate fails
- **THEN** no identity is reported as kept
- **AND** a subsequent load finds no identity that was not there before
- **AND** the reply carries a reason and no identity

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

- **WHEN** paths have been recorded for several Stoas through this module
- **THEN** every recorded pairing of Stoa and path can be read back
- **AND** reading them requires nothing beyond the stored record itself

### Requirement: A recorded path is one this module's derivation could have produced

The set of derivation paths this capability can offer SHALL be bounded, and a path
outside that bound SHALL be refused rather than coerced into it — both when it is
offered for recording and when it is read back from the record.

An identity derives from any path whatsoever, so a path the module did not produce
derives a working identity that is not the user's, and nothing downstream can tell
the difference. Coercing such a value — clamping, masking on read, truncating —
hands the user a usable identity they did not choose, which the requirement on
malformed input calls unrecoverable because the choice cannot be recomputed. A
refusal is the only outcome that is visible.

The bound SHALL be the same on the offering side and the reading side. Two bounds
that agree today are two things to keep in step, and the narrower of them then
describes what the module writes while the wider describes what it accepts.

**This is a bound on the range, not on the five paths a particular set offered.**
Which five those were is not recoverable once the set is superseded, and the record
outlives every set. So what is required is that a recorded path be a value this
module's derivation could have produced, which is the strongest property available
to a reader of the record alone.

A value in the record that is not a Stoa address SHALL likewise be refused rather
than padded or truncated, for the same reason: a coerced address names a different
Stoa.

**These refusals concern content the module did not write.** The requirement that
the record be fully readable is about what the module recorded through its own
interface, and nothing this module writes can be refused by these bounds — a bound
that refused a path the module can offer would be a store bricked by its own writer.
Content the module did not write reaches the record by restore, by file sync, or by
edit, and refusing it is not a contradiction of readability but its precondition:
what reads back must be what was recorded.

#### Scenario: A path outside the bound is refused on read

- **WHEN** the record holds a path outside the bound
- **THEN** reading the record reports a failure naming that
- **AND** no identity is derived from the value

#### Scenario: A path outside the bound is refused on recording

- **WHEN** a path outside the bound is offered for recording
- **THEN** the attempt is refused
- **AND** the record holds no entry for it

#### Scenario: The last path inside the bound is accepted

- **WHEN** the greatest path inside the bound is recorded and read back
- **THEN** it reads back as that value

#### Scenario: Every path a set can offer is inside the bound

- **WHEN** candidates are generated over many different sets
- **THEN** every path offered is one the record accepts

#### Scenario: A recorded Stoa that is not an address is refused

- **WHEN** the record holds, in place of a Stoa address, a value of another length
- **THEN** reading the record reports a failure naming that
- **AND** no pairing is returned for it

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

### Requirement: The derivation path is carried in the replies that name an identity

A reply describing a candidate, a reply reporting a kept identity, and a reply
reporting the identity in use SHALL each carry the derivation path that identity
derives from.

The path is the one input to an identity that no published value carries and that
the module alone holds, so a caller that cannot see it cannot show the user what
must be preserved — and the requirement that the interface be able to state that
recovery needs the record is a statement about a value the caller is otherwise
never shown. The path is not secret: the record of it "reveals nothing that a
published identity does not already reveal".

#### Scenario: A candidate carries its path

- **WHEN** a slate is generated
- **THEN** each candidate carries the derivation path it derives from

#### Scenario: A kept identity is reported with its path

- **WHEN** a candidate is kept
- **THEN** the reply carries the path of the candidate that was kept

#### Scenario: The identity in use is reported with its path

- **WHEN** the identity in use is asked for and there is one
- **THEN** the reply carries the path it derives from
- **AND** the path is the one recorded for that Stoa

### Requirement: A set of candidates and each candidate in it are nameable by the caller

A reply offering candidates SHALL carry a value identifying that set, and SHALL
carry, for each candidate, a value identifying it within the set. A request to keep a
candidate SHALL name both — alongside the Stoa the choice is for, which every method
of this capability takes.

Without the set identifier there is no way for a caller to say which set its
selection was made against, and so no way for the module to refuse a selection made
against a superseded one — which is a requirement above rather than a nicety, since
the refusal is what stops a stale choice storing a candidate the user never saw. The
per-candidate value is what makes "a selection that does not name a candidate in the
current set" a thing a caller can get wrong rather than a thing it cannot express.

#### Scenario: A set is identified and its candidates are individually nameable

- **WHEN** a slate is generated
- **THEN** the reply carries a value identifying the set
- **AND** each candidate carries a value identifying it within the set

#### Scenario: A candidate is kept by naming the set and the candidate

- **WHEN** a candidate is kept by supplying the set's identifier and that
  candidate's own
- **THEN** the request is accepted, the choice being identified by that pair
- **AND** a request naming a set that is not the current one is refused

### Requirement: The fields of each reply are exactly those this capability requires

Each reply of this capability SHALL carry exactly the fields the requirements above
name for it, and SHALL carry no other field.

A closed set is what makes the requirement that no reply carry a display name or a
visual mark checkable at all: an obligation to carry *no* name cannot be met by a
reply whose field set is open, because any later field is then admissible and a name
is a later field. The same argument covers every field a separate contract owns.

**Widening a reply remains available and remains cheap** — it is a change to this
requirement, made deliberately, rather than a field that arrives as a side effect of
producing one. What this forbids is a caller coming to depend on a field no contract
names.

#### Scenario: A candidate's field set is closed

- **WHEN** a slate is generated
- **THEN** each candidate's set of fields, compared as a whole set against the set
  the requirements name for it, is equal to it
- **AND** the reply's own set of fields, compared the same way, is equal to the set
  required of it

Comparing the set as a whole rather than asking whether each required field is
present is what this scenario turns on: presence checks pass on a reply carrying an
extra field, so they cannot establish closure.

#### Scenario: The identity replies' field sets are closed

- **WHEN** a candidate is kept, and separately the identity in use is asked for
- **THEN** each reply's set of fields, compared as a whole set, is equal to the set
  required of it for that outcome

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

### Requirement: A set of candidates retains no secret material

A set of candidates SHALL retain no candidate's secret key and no master key. Secret
material SHALL exist only for as long as deriving a candidate's public half takes,
and SHALL be cleared when that derivation is done rather than when the set is
discarded.

`keystore` requires clearing of the root secret, the derived encryption key and the
passphrase. This is the same obligation, discharged one step earlier: a set is the
one place several candidates would exist at once, so a set that held their secrets
would multiply the exposure by the number of candidates and keep it for as long as
the user is deciding — which is unbounded, because regeneration is unlimited.

**The stated obligation is what a caller and a test can check: that the set holds
none.** Whether a transient buffer inside a derivation was overwritten is not
observable from outside the derivation, so it is not stated as a scenario here.
Requiring the retention property instead is stronger where it can be checked and
silent where it cannot, rather than the other way round.

**The buffers inside derivation itself belong to `identity`**, which owns how a key
derives. A requirement about them is that capability's and is not stated here; this
capability requires only that nothing it hands on or holds carries one.

#### Scenario: A set of candidates holds no secret

- **WHEN** a set of candidates is generated
- **AND** every byte the set exposes is searched for the master key and for each
  candidate's secret key
- **THEN** none of them is found
- **AND** a value that is present in the set is found by the same search, so the
  search is known to work

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

**How this is enforced is the closed field set**, not a check for fields with
name-like spellings. A rule that forbade the two while admitting anything else would
be met by a `label`, a `nickname` or a glyph under a field name nobody anticipated;
a closed set forbids all of them without having to enumerate what a name might be
called. So a test failing because a field was added is failing this requirement as
much as the closed-set one, and the two are deliberately not independent.

#### Scenario: No reply carries a name or a mark

- **WHEN** a slate is generated, and separately the identity in use is asked for
- **THEN** no candidate in the slate reply carries a display name or a visual mark
- **AND** the reply describing the identity in use carries neither
- **AND** both replies still carry the public key and the address a name and a
  mark would be derived from
