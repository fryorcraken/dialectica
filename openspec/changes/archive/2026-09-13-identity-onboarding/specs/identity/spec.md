## Purpose

Amends per-Stoa derivation to admit a user-chosen derivation path, which the
`identity-onboarding` capability introduces.

The requirement being modified currently forbids any input beyond the root secret
and the Stoa address, and states the reason: an identity must be reproducible from
those two alone. A user-chosen path is exactly such a third input. The
recomputability guarantee therefore moves rather than disappearing — it now rests
on the path being recorded, which `identity-onboarding` requires.

Only the one requirement changes. The rest of `identity` — address construction,
parse refusals, domain separation, the key-to-author binding, authenticity not
being authority, no rotation, and secret-key handling — is untouched.

A MODIFIED block replaces the whole requirement, scenarios included, so every
scenario the live requirement carries is restated below verbatim and the new ones
are added alongside. In particular "The same root and Stoa always yield the same
identity" is kept: the two-input derivation still exists, and a block that
renamed that scenario rather than keeping it would drop the two-input guarantee
from the contract on archive, with no error.

## MODIFIED Requirements

### Requirement: A user has one identity per Stoa, and it is permanent

A user SHALL have exactly one identity within a Stoa, and that identity SHALL be
reproducible from the user's root secret, the Stoa's address, and the derivation
path recorded for that Stoa.

Derivation SHALL depend on nothing else. A session counter, a wall clock, a device
identifier or any other value varying between runs SHALL NOT participate, because
the same root, Stoa and path must yield the same identity on any device and after
any restart. The derivation taking only a root secret and a Stoa address SHALL
remain and SHALL keep that same property, because identities already derived under
it are permanent and its inputs are what a peer can still recompute from.

The derivation path is a value the user chose at onboarding rather than one any
published value carries, so it SHALL be recorded in storage; the
`identity-onboarding` capability owns that obligation and the requirement that the
record be exportable. **This is a narrower guarantee than deriving from the root
and the Stoa alone**, and the difference is deliberate: it buys the user a choice
of identity, at the cost that the root secret is no longer sufficient on its own to
reproduce their identities.

Where a derivation scheme taking a path and one taking only the root and the Stoa
both exist, they SHALL be distinguishable, so that one scheme's identities cannot
be silently reproduced by the other. The scheme is versioned for this purpose.

A stable pseudonym is what lets anything accumulate against an identity — a
moderator's judgement, or a relevance signal — and the transport binds its sender
identifier to it for a channel's lifetime, so instability here would change the
identifier other peers know a user by.

#### Scenario: The same root and Stoa always yield the same identity

- **WHEN** a key is derived twice from one root secret and one Stoa address
- **THEN** both derivations yield the same public key

#### Scenario: The same root, Stoa and path always yield the same identity

- **WHEN** a key is derived twice from one root secret, one Stoa address and one
  derivation path
- **THEN** both derivations yield the same public key

#### Scenario: A derived identity survives storage and reload

- **WHEN** a derived key's stored form is written out and read back
- **THEN** the restored key has the same public key as the original

#### Scenario: A derived key is a usable identity, not merely a distinct one

- **WHEN** a derived key signs an op
- **THEN** the signature verifies under that key's public half

#### Scenario: Different paths for one Stoa yield different identities

- **WHEN** two keys are derived from one root secret and one Stoa address under
  two different derivation paths
- **THEN** their public keys differ

#### Scenario: The path-taking scheme does not collide with the scheme without one

- **WHEN** a key is derived from a root and a Stoa under the scheme that takes a
  path
- **AND** a key is derived from the same root and Stoa under a scheme that takes
  none
- **THEN** the two public keys differ
