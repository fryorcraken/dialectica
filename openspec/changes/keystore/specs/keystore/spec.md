## Purpose

Defines how a user's root secret is stored at rest, what unlocks it, and what is refused — so that an identity survives a restart without the secret being readable by anything that can read the file, and so that no failure on this path can hang or crash the module.

## ADDED Requirements

### Requirement: A root secret persists across restarts

The keystore SHALL persist a single root secret from which every per-Stoa identity is derived, so that a user's pseudonym in a Stoa is the same after a restart as before it.

An identity that does not survive a restart is an identity nothing can accumulate against: moderator judgement, relevance signals, and the transport's sender binding all assume a pseudonym that persists.

#### Scenario: A stored secret yields the same identity after reloading

- **WHEN** a keystore is created, then loaded again from the same location
- **THEN** the root secret recovered is the one that was stored
- **AND** the identity derived from it for a given Stoa is unchanged

#### Scenario: The stored secret is the root, not a per-Stoa key

- **WHEN** a keystore is loaded
- **THEN** what it yields is the root secret
- **AND** per-Stoa keys are derived from it rather than stored separately

### Requirement: The secret is encrypted at rest under a passphrase

When a passphrase is supplied, the keystore file SHALL NOT contain the root secret in any recoverable form without that passphrase.

The passphrase SHALL be stretched by a memory-hard key derivation function with per-keystore random salt, so that a stolen file cannot be attacked at the speed of a plain hash.

#### Scenario: The plaintext secret does not appear in the file

- **WHEN** a keystore is written under a non-empty passphrase
- **THEN** the root secret's bytes do not appear anywhere in the file

#### Scenario: The correct passphrase unlocks

- **WHEN** a keystore written under a passphrase is opened with that same passphrase
- **THEN** the root secret is recovered

#### Scenario: A wrong passphrase is refused, not approximated

- **WHEN** a keystore is opened with a passphrase other than the one it was written under
- **THEN** unlocking fails
- **AND** no key material is returned
- **AND** the failure is reported as a wrong passphrase rather than as a corrupt file

#### Scenario: Two keystores holding the same secret under the same passphrase differ

- **WHEN** the same root secret is written twice under the same passphrase
- **THEN** the two files differ
- **AND** neither reveals that the secrets are identical

### Requirement: A tampered keystore is refused rather than decrypted

The encryption SHALL be authenticated: any modification to the stored ciphertext, its nonce, its salt, or its key-derivation parameters SHALL cause unlocking to fail rather than to yield a different secret.

An unauthenticated cipher would let anyone with write access to the file silently substitute the identity the user posts under.

#### Scenario: A flipped ciphertext byte fails to unlock

- **WHEN** any byte of a keystore's ciphertext is altered
- **THEN** unlocking fails
- **AND** no secret is returned

#### Scenario: An altered salt or nonce fails to unlock

- **WHEN** a keystore's salt or nonce is altered
- **THEN** unlocking fails with the correct passphrase

#### Scenario: Altered key-derivation parameters fail to unlock

- **WHEN** a keystore's recorded key-derivation parameters are altered
- **THEN** unlocking fails rather than deriving a key under attacker-chosen parameters

### Requirement: An unencrypted keystore is an explicit state, not a silent one

The keystore SHALL support being stored without encryption, and that state SHALL be recorded in the file itself rather than inferred from a failure to decrypt.

An unencrypted keystore is a real configuration on a machine whose disk is already encrypted, and refusing it entirely pushes users to worse workarounds. Recording it explicitly is what lets a caller tell "no passphrase was ever set" from "the passphrase is wrong".

#### Scenario: An unencrypted keystore opens without a passphrase

- **WHEN** a keystore written without a passphrase is opened with no passphrase
- **THEN** the root secret is recovered

#### Scenario: The two states are distinguishable

- **WHEN** a keystore is inspected
- **THEN** whether it is encrypted is determinable without attempting to decrypt it

### Requirement: A keystore readable by others is refused

Before reading key material, the keystore SHALL check the file's permissions and SHALL refuse to load a file that is readable or writable by any user other than its owner.

Refusing is deliberate and is not a warning. A warning on this path is a message nobody sees: the module has no terminal, and its caller is a view that would have to choose to render it. A secret that has already been readable by every local process is a secret to replace, not to keep using.

The check SHALL be reported distinguishably from a missing file and from a wrong passphrase, and its message SHALL name the fix.

#### Scenario: A world-readable keystore is refused

- **WHEN** a keystore file's mode grants read access to group or other
- **THEN** loading fails
- **AND** no key material is read
- **AND** the error names the permissions as the problem and the mode required

#### Scenario: A correctly-permissioned keystore loads

- **WHEN** a keystore file is readable and writable only by its owner
- **THEN** the permission check passes

#### Scenario: A newly-written keystore satisfies its own check

- **WHEN** the keystore writes a file
- **THEN** that file's permissions are restrictive enough that loading it does not fail the check

### Requirement: The module never prompts

No keystore operation SHALL read from standard input, open a terminal, or block waiting for a user. Every passphrase SHALL be supplied by the caller.

The module is a library inside a sandboxed host with no terminal. A prompt does not degrade into a question nobody answers — it is an unbounded wait, which the caller experiences as a timeout naming a slow provider rather than a locked keystore.

#### Scenario: A locked keystore returns an error immediately

- **WHEN** a keystore requires a passphrase and none is supplied
- **THEN** the operation fails and returns
- **AND** the failure is reported rather than waited on

#### Scenario: The failure names the fix

- **WHEN** any keystore operation fails
- **THEN** the message states what the caller must do to succeed, not only what went wrong

### Requirement: No keystore failure is fatal

Every keystore operation SHALL return a failure rather than panic, for any content the file may hold — truncated, empty, oversized, structurally malformed, or arbitrary bytes.

A panic in this module aborts the whole module process, and a keystore file is not under the module's control: it may be corrupt from an interrupted write, or hostile from another local process.

#### Scenario: Arbitrary file content is refused without a panic

- **WHEN** a keystore file holds arbitrary bytes of any length
- **THEN** loading fails
- **AND** no panic occurs

#### Scenario: A truncated file is refused

- **WHEN** a keystore file is a truncation of a valid one, at any length
- **THEN** loading fails
- **AND** the failure is distinguishable from a wrong passphrase

#### Scenario: Declared lengths are not trusted

- **WHEN** a field's declared length exceeds what the file contains
- **THEN** loading fails rather than reading past the end or allocating on the declared figure

### Requirement: The file format declares its version

The keystore file SHALL begin with a version discriminant, and loading SHALL refuse a version it does not recognise.

Key-derivation parameters change as hardware does. A version discriminant is what makes an older build's refusal legible rather than a misparse of a newer file.

#### Scenario: An unknown version is refused and says so

- **WHEN** a keystore declares a version this build does not recognise
- **THEN** loading fails
- **AND** the failure is distinguishable from corruption and from a wrong passphrase

### Requirement: Writing a keystore cannot destroy an existing one

Writing SHALL be atomic: an interruption at any point SHALL leave either the previous file intact or the new file complete, and never a partially-written file in place of a valid one.

The root secret is not recoverable from anywhere else. A half-written file over a good one destroys every identity the user has.

#### Scenario: The destination is never partially written

- **WHEN** a keystore is written
- **THEN** the content is written elsewhere and moved into place as one step
- **AND** the destination at no point holds an incomplete file

#### Scenario: Writing refuses to overwrite an existing keystore by default

- **WHEN** a keystore is created at a path that already holds one
- **THEN** creation fails rather than replacing it
- **AND** the failure is distinguishable from a permissions or format error

### Requirement: An error never carries key material

No error message produced by this capability SHALL contain the root secret, a derived key, a passphrase, or any part of the ciphertext.

Error strings cross the module boundary to a view, which may log or display them. An error that quotes what it failed to decrypt turns a diagnostic into a disclosure.

#### Scenario: A wrong-passphrase error quotes neither the passphrase nor the file

- **WHEN** unlocking fails because the passphrase is wrong
- **THEN** the message contains neither the supplied passphrase nor any stored bytes

#### Scenario: A malformed-file error quotes no file content

- **WHEN** loading fails because the file is malformed
- **THEN** the message describes the structural problem without reproducing the file's bytes

### Requirement: The passphrase check is not a byte comparison

Verification that a passphrase is correct SHALL be the authenticated cipher's own tag check, which is constant-time, and SHALL NOT be an ordinary equality comparison against a stored digest.

A short-circuiting comparison against a stored value leaks how much of a guess was right, which turns an offline attack on the file into a faster one.

#### Scenario: Correctness is decided by the authentication tag

- **WHEN** a passphrase is checked
- **THEN** the decision comes from the cipher's authentication, and no separate stored verifier is compared

### Requirement: Secret material is cleared when dropped

In-memory copies of the root secret, the derived encryption key, and the passphrase SHALL be zeroized when they go out of scope.

#### Scenario: A dropped secret does not persist in its buffer

- **WHEN** a value holding secret bytes is dropped
- **THEN** its buffer is overwritten rather than left with the secret in it
