## ADDED Requirements

### Requirement: In this release one machine key is the identity in every Stoa

In this release the identity in use in every Stoa MUST be this peer's machine
key: the key a request for this peer's master key reports, and the key a Stoa
this peer creates names as its creator. It MUST be the same key in every Stoa.

Every op this peer publishes — a post, a reply and a vote — MUST be signed by the
machine key, whichever Stoa it is published into. Wherever the posting-capability
probe or the report of the identity in use names an identity, for any Stoa, it
MUST be the machine key.

Which key is in use MUST NOT depend on a per-Stoa choice:

- a peer holding a usable machine key MUST be able to post, reply and vote in a
  Stoa for which no choice was ever recorded;
- a choice recorded for a Stoa MUST NOT change the key in use there;
- a record of per-Stoa choices that cannot be read MUST NOT prevent posting,
  replying or voting, and MUST NOT change the key in use.

Where no machine key is held, no identity is in use in any Stoa; the probe, the
identity report and a publish each report that as their own requirements
describe.

**Per-Stoa identity — a different key in each Stoa — is out of scope for this
release.** The derivation it will use stays built and is contracted by this
capability's derivation requirements; none of them makes a per-Stoa key the
identity in use.

#### Scenario: The same key posts, replies and votes in two Stoas

- **WHEN** a peer holding a machine key, with no per-Stoa choice recorded for
  either Stoa, publishes a post, a reply and a vote into one Stoa, and a post, a
  reply and a vote into a second, different Stoa
- **THEN** all six ops carry the same public key
- **AND** that key is the one a request for this peer's master key reports

#### Scenario: The probe and the identity report name the machine key in two Stoas

- **WHEN** the posting-capability probe and the identity report are each asked
  about two different Stoas by a peer holding a machine key
- **THEN** all four answers name the same public key
- **AND** it is the one a request for this peer's master key reports

#### Scenario: A Stoa's creator posts as its creator

- **WHEN** a peer creates a Stoa and then publishes a post into it
- **THEN** the post's author is the creator key the Stoa's genesis record names

#### Scenario: A recorded per-Stoa choice does not change the key in use

- **WHEN** a choice is recorded for a Stoa by keeping a candidate there, and a
  post is then published into that Stoa
- **THEN** the post is signed by the machine key
- **AND** it is not signed by the key the recorded choice derives
- **AND** the probe and the identity report for that Stoa name the machine key

#### Scenario: No per-Stoa choice is needed to post

- **WHEN** a peer holding a machine key, with no choice recorded for any Stoa,
  asks the probe about a Stoa and then publishes a post into it
- **THEN** the probe reports that posting is possible
- **AND** the publish succeeds

#### Scenario: An unreadable record of choices does not prevent posting

- **WHEN** a peer holds a machine key and the record of per-Stoa choices cannot
  be read
- **THEN** the probe reports that posting is possible and names the machine key
- **AND** a post published into a Stoa succeeds and is signed by the machine key

## RENAMED Requirements

- FROM: `### Requirement: Identities are unlinkable across Stoas`
- TO: `### Requirement: The per-Stoa derivation keeps identities unlinkable across Stoas`

## MODIFIED Requirements

### Requirement: A user has one identity per Stoa, and it is permanent

A user MUST have exactly one identity within a Stoa, and that identity MUST be
permanent. **In this release it is the machine key, the same in every Stoa** —
see *In this release one machine key is the identity in every Stoa*.

The per-Stoa derivation, which per-Stoa identity will use and which this release
keeps built, MUST yield a key reproducible from the user's root secret, the
Stoa's address, and a derivation path.

Derivation MUST depend on nothing else. A session counter, a wall clock, a device
identifier or any other value varying between runs MUST NOT participate, because
the same root, Stoa and path must yield the same key on any device and after any
restart. The derivation taking only a root secret and a Stoa address MUST remain
and MUST keep that same property, because keys already derived under it are
permanent and its inputs are what a peer can still recompute from.

The derivation path is a value the user chose at onboarding rather than one any
published value carries, so it MUST be recorded in storage; the
`identity-onboarding` capability owns that obligation and the requirement that the
record be exportable. **This is a narrower guarantee than deriving from the root
and the Stoa alone**, and the difference is deliberate: it buys the user a choice
of identity, at the cost that the root secret is no longer sufficient on its own to
reproduce their per-Stoa keys.

Where a derivation scheme taking a path and one taking only the root and the Stoa
both exist, they MUST be distinguishable, so that one scheme's keys cannot be
silently reproduced by the other. The scheme is versioned for this purpose.

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

### Requirement: The per-Stoa derivation keeps identities unlinkable across Stoas

The per-Stoa derivation MUST yield keys the protocol does not relate across
Stoas. Deriving for two different Stoa addresses from one root MUST yield
different keys, and two different roots in one Stoa MUST yield different keys.

There MUST be no way to compute a per-Stoa key's public half from the root's
public half. This is the property cross-Stoa unlinkability rests on: were public
derivation available, anyone holding a user's root public key could enumerate
their pseudonym in every Stoa they participate in.

**In this release the identity in use is not a per-Stoa key, so this property
protects no user.** The same user in two Stoas is one key — the machine key — and
anyone reading both Stoas can see that it is. The property is suspended, not
abandoned: the derivation it rests on stays built and pinned, and it becomes a
property of the identity in use again only when per-Stoa identity is restored,
which is out of scope for this release.

This is a privacy guarantee about the protocol and not about the world. Within a
Stoa a pseudonym is deliberately stable, and writing style, posting time and the
reply graph link a user's identities whatever the key derivation does.

#### Scenario: Two Stoas yield different identities from one root

- **WHEN** one root secret is used to derive per-Stoa keys for two different
  Stoas
- **THEN** the two public keys differ

#### Scenario: Two users in one Stoa do not collide

- **WHEN** two different root secrets are used to derive per-Stoa keys for one
  Stoa
- **THEN** the two public keys differ

#### Scenario: A per-Stoa identity is not the root identity

- **WHEN** a per-Stoa key is derived from a root secret
- **THEN** its public key differs from the public key of the root used directly
