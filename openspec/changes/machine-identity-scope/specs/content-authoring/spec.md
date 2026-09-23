## RENAMED Requirements

- FROM: `### Requirement: The author is derived from the Stoa, never supplied`
- TO: `### Requirement: The author is the module's to decide, never supplied`

## MODIFIED Requirements

### Requirement: The author is the module's to decide, never supplied

A publish operation MUST NOT accept an author, an identity, a key, or an address
as a parameter. The identity that signs MUST be decided by the module from its own
state. **In this release it is the machine key, whichever Stoa the request names**
(`identity`: *In this release one machine key is the identity in every Stoa*).

A method taking an author is a method that can be asked to sign as someone it is
not.

A request carrying a field that names an author MUST be refused rather than
ignored, so that a caller which believes it is choosing an identity is told it is
not.

#### Scenario: The published op's author is the derived identity

- **WHEN** a post is published into a Stoa
- **THEN** the op's author is the identity in use for that Stoa, which in this
  release is the machine key
- **AND** the op verifies against that identity's key

  The scenario keeps its name because the name is how this delta addresses it.
  "Derived" now means decided by the module from its own state: the identity is
  no longer derived from the Stoa, and the check is that the author is the
  identity in use there.

#### Scenario: A request naming an author is refused

- **WHEN** a publish request carries a field naming an author, an identity or a
  key
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: The signing identity is the one the probe reports

- **WHEN** the posting-capability probe reports an identity for a Stoa and a post
  is then published into that Stoa
- **THEN** the published op's author is the identity the probe reported
