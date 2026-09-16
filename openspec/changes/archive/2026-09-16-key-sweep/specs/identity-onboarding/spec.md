## MODIFIED Requirements

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
- **AND** no two share a derivation path

#### Scenario: Requesting another set yields different candidates

- **WHEN** a slate is requested, and then a slate is requested again
- **THEN** no candidate in the second set has a public key from the first

#### Scenario: Regeneration is not limited

- **WHEN** a slate is requested many times in succession
- **THEN** each request is answered
- **AND** none is refused on the ground of how many preceded it

### Requirement: A slate reply carries public values only

A reply describing candidates SHALL carry, for each candidate, values sufficient
to identify and display it, and SHALL NOT carry a secret key, a master key, a seed,
a mnemonic, or any value from which one could be reconstructed.

The view has no use for a secret: it cannot sign, because signing is the module's.
Adding a field later is a change to this contract and can be made; a secret that has
crossed the module boundary cannot be recalled. The asymmetry is the whole argument
for starting narrow, and the requirement below that each reply's field set is closed
is what keeps "later" meaning a decision rather than a side effect.

A candidate's public key SHALL be present. It is the only unforgeable way to tell
two candidates apart, and it is what every value shown beside a candidate is
derived from — the generated display name and the visual mark both read the key's
own bytes. **No candidate SHALL carry an author address**: the public key is the
sole author identifier, so an address would be a second identifier beside the one
it was derived from, where the two could disagree and a reader could not tell
which was wrong.

#### Scenario: No secret appears in a slate reply

- **WHEN** a slate is generated
- **THEN** no field of the reply contains the master key's bytes
- **AND** no field contains any candidate's secret key bytes

#### Scenario: A candidate carries an address and a public key

- **WHEN** a slate is generated
- **THEN** each candidate carries a public key
- **AND** no candidate carries an author address

  The scenario keeps its name while its content inverts, because the name is how
  the delta addresses the scenario this change alters. What it asserts is now the
  address's absence: the public key is the sole author identifier, and a
  candidate carrying both would carry a derived value beside the material it was
  derived from.

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
position selected** — the same public key, not merely a candidate derived at the
same position. A selection is made on what the user was shown, and an offering and
a keep are two separate calls, so anything the offering depended on and the keep
re-established can differ between them while every guard on the selection still
passes. When it does, the user is given a working identity they never saw, which is
the outcome the requirement on malformed input calls unrecoverable.

The reply SHALL state whether the candidate was kept, and where it was, SHALL carry
the identity kept — its public key — and where it was not, SHALL carry a reason and
no identity. This is the posting probe's shape rather than a second convention for
the same job, and it is what makes a caller able to show the user the identity they
now have without asking a second question.

#### Scenario: The identity kept is the candidate that was displayed

- **WHEN** a set of candidates is offered, and a candidate is then kept at a selected
  position
- **THEN** the identity the keep reports has the public key the offering reply
  carried at that position
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
- **AND** carries that identity's public key
- **AND** carries no author address
- **AND** carries no reason

#### Scenario: A failed keep records nothing

- **WHEN** keeping a candidate fails
- **THEN** no identity is reported as kept
- **AND** a subsequent load finds no identity that was not there before
- **AND** the reply carries a reason and no identity

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

The reply SHALL name the identity by its public key and SHALL NOT carry an author
address. The public key is the sole author identifier, and it is what the generated
display name and the visual mark are both derived from.

#### Scenario: An existing identity is reported

- **WHEN** an identity is stored and readable
- **THEN** the reply states that there is an identity
- **AND** carries its public key
- **AND** carries no author address
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

### Requirement: The fields of each reply are exactly those this capability requires

Each reply of this capability SHALL carry exactly the fields the requirements above
name for it, and SHALL carry no other field.

A closed set is what makes the requirement that no reply carry a display name or a
visual mark checkable at all: an obligation to carry *no* name cannot be met by a
reply whose field set is open, because any later field is then admissible and a name
is a later field. The same argument covers every field a separate contract owns.

**The set narrows when a requirement above stops naming a field, and that is this
requirement working rather than a separate obligation.** The author address was
named by the requirements on slate contents, on keeping, and on reporting the
identity in use; it is named by none of them now, so a reply still carrying one
carries a field no contract names and fails this requirement. Nothing here has to
forbid the address by name, which is the same reason nothing here forbids a display
name by name.

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

### Requirement: A generated name and a mark are not settled by this capability

This capability SHALL NOT define how a display name or a visual mark is derived
from a key.

A slate carries public keys, and a name derived from a public key is computable by
anything holding one. Which words, which wordlist, which denylist and which glyph
are a separate contract, and a slate that specified them would make every change to
either a change to this one.

What this capability does require is that the value a name and a mark are derived
from — the public key — is present in a slate reply, which the requirement on reply
contents states. **Both channels read the key**, so one field is sufficient to make
both computable; this previously named the public key and the address as two inputs,
which the capability owning the derivations no longer describes.

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
- **AND** both replies still carry the public key a name and a mark would be
  derived from
