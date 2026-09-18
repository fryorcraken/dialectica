## ADDED Requirements

### Requirement: A peer with no master key can obtain one without naming a Stoa

The module SHALL provide an operation that creates this peer's master key and
that requires no Stoa.

Every other operation in this capability takes a Stoa, and a peer's first run has
none. Creating a Stoa requires a creator key and creates none; the operations
that write a master key all require a Stoa first. A peer in that state can reach
neither, and it is the state every peer starts in — so the operation that resolves
it cannot itself require a Stoa.

The operation SHALL write the master key only, and SHALL NOT record a chosen path
for any Stoa. Recording one would name a Stoa the peer has not created: creation
would then succeed while posting stayed refused for want of a choice in the real
Stoa, which is a failure that presents as a success.

The reply SHALL name the resulting key, SHALL state whether that key is protected
at rest, and SHALL state whether this call created it.

#### Scenario: A peer with no master key obtains one

- **WHEN** a peer holding no master key asks for one, naming no Stoa
- **THEN** a master key is stored
- **AND** the reply names that key

#### Scenario: The key obtained is the key a Stoa creation uses

- **WHEN** a peer obtains a master key and then creates a Stoa
- **THEN** the creation succeeds
- **AND** the creator recorded in the genesis record is the key that was obtained

#### Scenario: Creation is still refused before a key exists

- **WHEN** a peer holding no master key creates a Stoa
- **THEN** the creation is refused

#### Scenario: No per-Stoa choice is recorded

- **WHEN** a peer obtains a master key
- **THEN** no chosen path is recorded for any Stoa

#### Scenario: Protection at rest is reported

- **WHEN** a master key is stored with no passphrase available
- **THEN** the reply states that the key is not protected at rest

### Requirement: Obtaining a master key never replaces one

Where a master key already exists, the operation SHALL report it and SHALL NOT
replace, rewrite, or re-derive it. The reply SHALL distinguish a key this call
created from one that was already present.

A master key exists in exactly one place, and replacing it silently discards every
identity derived from it while the ops those identities signed remain published.
This capability already refuses that for keeping a candidate; an operation that
creates a master key must not become a second route to the same loss.

A repeated request SHALL be reported as a success rather than as a refusal.
Holding a master key is the expected state on every run after the first, so a
refusal there would make the operation one that every caller must guard against
calling — and the guard is what gets forgotten.

The protection reported for an existing key SHALL be the protection of the stored
key, not of whatever the caller currently has available. The call wrote nothing,
so the stored key is the only truthful source; reporting the caller's instead
would tell a user their key is protected on the strength of a passphrase that was
never applied to it.

#### Scenario: A second request replaces nothing

- **WHEN** a peer that already holds a master key asks for one again
- **THEN** the stored key is unchanged
- **AND** the reply names that same key

#### Scenario: A second request is a success, not a refusal

- **WHEN** a peer that already holds a master key asks for one again
- **THEN** the reply is a success
- **AND** it states that this call did not create the key

#### Scenario: An existing key's protection is reported from the key itself

- **WHEN** a peer holding an unprotected master key asks for one again while a
  passphrase is available
- **THEN** the reply states that the stored key is not protected at rest
