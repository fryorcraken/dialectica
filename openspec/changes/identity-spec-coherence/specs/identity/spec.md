## REMOVED Requirements

### Requirement: Identity does not rotate, and this is a contract not an omission

**Reason**: Its scenario "An identity is a pure function of its root and its
Stoa" contradicted three other scenarios in this capability and could not be
tested. Its **AND** clause, "no operation exists that yields a different key for
the same pair", is false against "Different paths for one Stoa yield different
identities", "The path-taking scheme does not collide with the scheme without
one" and "A per-Stoa identity is not the root identity". It was also a claim about
every operation in the API, which no test can check. Its **THEN** clause repeated
"The same root and Stoa always yield the same identity". A MODIFIED block cannot
drop or rename a scenario, so the requirement is replaced rather than amended.
The scenario's name states the false claim, so keeping that name for new content
was not an option.

**Migration**: Replaced by "Identity does not rotate: the key behind an identity
is never replaced", added in this same change. Its text is this requirement's
with these edits: one new paragraph stating that in this release the key behind
the identity in use is the machine key in every Stoa, and that calling the
operation that creates a master key while one is held MUST leave the identity in
use unchanged, as reported and as signed; and the scenario "An identity is a pure
function of its root and its Stoa" replaced by "Creating a master key while one
is held does not replace the identity in use", which checks that paragraph. Its old **THEN**
clause is dropped, not carried over, because it is covered by "The same root and
Stoa always yield the same identity". Every other paragraph, and the scenario "An
identity is named by its key and by nothing beside it" with its note, is
unchanged. No requirement or test cited this requirement or the removed scenario
by name.

## ADDED Requirements

### Requirement: Identity does not rotate: the key behind an identity is never replaced

There SHALL be no way to replace the key behind an identity while keeping the
identity. A per-Stoa identity is derived once and is permanent.

**In this release the key behind the identity in use is the machine key, in every
Stoa** — see *In this release one machine key is the identity in every Stoa*.
Calling the operation that creates a master key while one is held MUST leave the
identity in use unchanged: the identity report MUST name the same public key
afterwards as before, and an op published afterwards MUST be signed by that same
key. The operation meant is the one `identity-onboarding` provides for a peer to
obtain its master key (*A peer with no master key can obtain one without naming
a Stoa*), not the one that reports whether a master key is held (*Whether this
peer holds a master key is reportable without creating one*), which writes
nothing by its own requirement.

A key that can be discarded at will is a key nothing can be attached to:
rotation lets a user shed whatever has accumulated against their identity, and
does so indistinguishably from a legitimate compromise. Rotation waits until
standing attaches to a revocable credential rather than to a keypair.

**The affordance that was being held open has been given up, deliberately.** The
author address hashed a *record* containing the key rather than the key itself,
so that the record could later grow into a key log and an identity could rotate
while its identifier survived. With the author address deleted, an identity *is*
its key and there is no identifier that could outlive one. Rotation, if it ever
arrives, therefore arrives as a credential layer above the keypair rather than as
a longer record beneath the same identifier. Nothing shipped depended on the
affordance, and this requirement already forbids what it was reserved for; it is
recorded because a reader finding rotation unbuilt should find the reason it is
now harder, rather than infer that nobody considered it.

#### Scenario: Creating a master key while one is held does not replace the identity in use

- **WHEN** a peer holding a machine key reports the identity in use for a Stoa,
  then calls the operation that creates a master key, then reports the identity
  in use for that Stoa again and publishes a post into it
- **THEN** the second report names the public key the first report named
- **AND** the post carries that same public key

#### Scenario: An identity is named by its key and by nothing beside it

- **WHEN** the values by which an identity is reported are enumerated
- **THEN** the public key is among them
- **AND** it is the only identifier among them

  Stated as "the only identifier" rather than as "no identifier that would
  survive the key changing", because the key cannot change — this requirement
  forbids it — so a scenario written over that counterfactual could never be
  run. What is checkable is how many identifiers an identity is reported by.
