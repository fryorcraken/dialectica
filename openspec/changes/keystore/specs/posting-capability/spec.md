## Purpose

Defines the probe a view asks before showing any affordance that would publish something, so that a compose box is never rendered when submitting it would fail.

## ADDED Requirements

### Requirement: A caller can ask whether it may post, before trying

The module SHALL expose a capability probe that answers whether posting is currently possible, without publishing anything and without changing any state.

A view cannot discover this any other way: it has no filesystem access and no network access of its own. Without the probe its only test is to attempt a post, which means the failure arrives after the user has typed.

#### Scenario: The probe reports capability without side effects

- **WHEN** the probe is called
- **THEN** it answers whether posting is possible
- **AND** nothing is published, created, or modified

#### Scenario: The probe is callable repeatedly

- **WHEN** the probe is called more than once with no intervening change
- **THEN** each call returns the same answer

### Requirement: The answer carries an identity or a reason, never both and never neither

A successful probe SHALL report that posting is possible together with the identity that would post. An unsuccessful probe SHALL report that posting is not possible together with a reason.

A reply carrying an identity and a reason together, or neither, is not a state the caller can act on. This is the general failure the wire contract forbids — a result that is partly a success — applied to the one call whose entire job is to be trusted.

#### Scenario: Able to post

- **WHEN** the probe finds an unlocked identity
- **THEN** the answer states posting is possible
- **AND** carries the identity
- **AND** carries no reason

#### Scenario: Unable to post

- **WHEN** the probe finds no usable identity
- **THEN** the answer states posting is not possible
- **AND** carries a reason
- **AND** carries no identity

### Requirement: The reason names the fix

When posting is not possible, the reason SHALL state what the user or operator must do to make it possible — not only what was found to be wrong.

"No key at this path; create one" is actionable. "Unlocked: false" is not, and a view can do nothing with it but show it.

The reasons SHALL be distinguishable from one another, at minimum: no keystore exists, the keystore exists but is locked and no passphrase was supplied, the supplied passphrase was wrong, the keystore's permissions are too open, and the keystore is unreadable or malformed.

#### Scenario: No keystore

- **WHEN** no keystore exists at the location consulted
- **THEN** the reason says so and says that one must be created

#### Scenario: Locked with no passphrase available

- **WHEN** a keystore exists, is encrypted, and no passphrase is available
- **THEN** the reason says so and names how a passphrase is supplied

#### Scenario: Wrong passphrase

- **WHEN** a passphrase is available but does not unlock the keystore
- **THEN** the reason says the passphrase was rejected
- **AND** is distinguishable from no passphrase having been supplied

#### Scenario: Permissions too open

- **WHEN** the keystore's permissions are too open for it to be loaded
- **THEN** the reason says so and names the required mode

#### Scenario: Malformed keystore

- **WHEN** the keystore cannot be parsed
- **THEN** the reason says so
- **AND** is distinguishable from a wrong passphrase

### Requirement: The identity reported is the one that would sign

When posting is possible, the identity reported SHALL be the one an op published now would be attributed to, derived from the key that would actually sign it.

An identity reported from a different source than the signing key is a display that can disagree with reality — the user sees one handle and posts under another.

#### Scenario: The reported identity matches the signing key

- **WHEN** the probe reports an identity and an op is then signed
- **THEN** the op's author is the identity the probe reported

### Requirement: The probe never prompts and never hangs

The probe SHALL return promptly in every state, and SHALL NOT read from standard input, open a terminal, or wait for a user.

The probe is the first call a view makes. A probe that can block makes the entire interface unrenderable rather than making one button unavailable.

#### Scenario: A locked keystore answers rather than waits

- **WHEN** the probe encounters a keystore it cannot unlock
- **THEN** it returns an answer saying posting is not possible

### Requirement: The probe is a probe, not a build flag

Whether posting is possible SHALL be determined from the state the probe actually finds, and SHALL NOT be a compile-time constant, a configuration setting, or any value that can disagree with the live state.

A gate that is not a probe eventually disagrees with the thing it gates, and the direction that costs the user is "the button was there and submitting lost the draft".

#### Scenario: The answer tracks the state

- **WHEN** the state the probe consults changes
- **THEN** a subsequent call reflects the change

### Requirement: The probe answers, it does not fail

Every call SHALL produce an answer about capability. A state that prevents posting SHALL be reported as not being able to post, with a reason — not as the call itself failing.

A caller that must handle both "cannot post" and "could not determine whether you can post" has two negative branches, and the second has no sensible rendering. Collapsing them means a view has exactly one thing to check.

#### Scenario: An unreadable keystore is an answer, not a failure

- **WHEN** the keystore cannot be read at all
- **THEN** the probe answers that posting is not possible, with a reason

#### Scenario: A malformed request is still answered

- **WHEN** the probe is called with input it cannot interpret
- **THEN** it does not panic
