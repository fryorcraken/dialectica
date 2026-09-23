## ADDED Requirements

### Requirement: Whether this peer holds a master key is reportable without creating one

The module MUST provide an operation that reports whether this peer holds a
master key. The operation MUST require no Stoa, and MUST NOT create, write,
replace or modify any stored key or any record of a chosen path.

The reply to that operation MUST take one of two shapes:

- **A key is held:** the reply states that a master key is held, names that key
  by its public key, and states whether the stored key is protected at rest.
- **No key is held:** the reply states that no master key is held, and carries
  no key and no protection field.

These are the only fields the reply carries. "The fields of each reply are
exactly those this capability requires" applies to it as to every other reply of
this capability.

The key named MUST be the key this peer would sign with. It is the same key the
operation that creates a master key reports for an existing key, and the same
key a Stoa created by this peer records as its creator. The protection reported
MUST be read from the stored key itself, not from whatever passphrase the
caller currently has available.

**A master key that is present but cannot be read MUST be reported as a
failure, and MUST NOT be reported as no key held.** This covers a keystore that
is refused as tampered, refused for its permissions, or encrypted with no
passphrase available to open it. The failure MUST carry the reason, in the
keystore's own vocabulary. Reporting such a store as holding no key would invite
the caller to offer a new key to a peer that has one. The operation that creates
a key would then refuse, because it never replaces one. The user would end up
asked to create a key they already hold and unable to do it.

A malformed request MUST be refused with the failure shape, as for every entry
point of this capability.

#### Scenario: A peer with no master key is reported as holding none

- **WHEN** no master key is stored and the operation is called, naming no Stoa
- **THEN** the reply states that no master key is held
- **AND** it carries no key and no protection field

#### Scenario: Asking writes nothing when no key is held

- **WHEN** no master key is stored and the operation is called
- **THEN** no master key is stored afterwards
- **AND** no chosen path is recorded for any Stoa

#### Scenario: A held key is reported with its public key and its protection

- **WHEN** a master key is stored with no passphrase and the operation is called
- **THEN** the reply states that a master key is held
- **AND** it names that key's public key
- **AND** it states that the key is not protected at rest

#### Scenario: Asking leaves a held key untouched

- **WHEN** a master key is stored and the operation is called
- **THEN** the stored keystore is byte-for-byte unchanged

#### Scenario: The key reported is the key the peer creates Stoas with

- **WHEN** a master key is stored, the operation is called, and a Stoa is then
  created by this peer
- **THEN** the key the operation named is the creator key the new Stoa records

#### Scenario: An existing key's protection is reported from the key itself

- **WHEN** an unprotected master key is stored and the operation is called while
  a passphrase is available
- **THEN** the reply states that the stored key is not protected at rest

#### Scenario: A key that cannot be read is a failure, not an absence

- **WHEN** a master key is stored but the keystore is refused, for example
  because its permissions let others read it
- **THEN** the reply is the failure shape carrying the keystore's reason
- **AND** it does not state that no master key is held

#### Scenario: The reply's field set is closed

- **WHEN** the operation is called with a key held, and separately with none held
- **THEN** each reply's set of fields, compared as a whole set, is equal to the
  set this requirement names for that outcome
