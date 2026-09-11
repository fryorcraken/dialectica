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

### Requirement: A keystore cannot demand unbounded work

The key-derivation parameters SHALL be recorded in the file, so that a file written under one build's cost still opens under another's. Because they are recorded, they are attacker-chosen, and loading SHALL therefore refuse a recorded cost that exceeds a bound this implementation fixes.

The bound SHALL be on the **total work the parameters imply**, not on each parameter separately. Bounding each parameter individually does not bound their product: parameters each inside their own limit can together demand many times the work of any one of them at its limit.

The refusal SHALL happen before any memory is allocated or any iteration performed, and SHALL be distinguishable from a malformed parameter and from a wrong passphrase.

A file that can make an unlock outlast the caller's timeout wedges the module with no cancellation and no error — the caller is told only that the call timed out, which points at a slow provider rather than at a keystore somebody edited. This is not a denial of service the module can recover from, so it must be refused rather than survived.

#### Scenario: A cost above the bound is refused

- **WHEN** a keystore records a key-derivation cost above the implementation's bound
- **THEN** loading fails
- **AND** the failure is distinguishable from a malformed parameter and from a wrong passphrase

#### Scenario: The refusal precedes the work

- **WHEN** a cost above the bound is refused
- **THEN** no memory has been allocated for key derivation
- **AND** no derivation iteration has been performed

#### Scenario: Parameters individually within their limits are still bounded together

- **WHEN** a keystore records parameters that are each within their individual limits but together imply work beyond the bound
- **THEN** loading fails
- **AND** the combination is not accepted merely because no single parameter exceeded its own limit

#### Scenario: A harder but bounded cost is still honoured

- **WHEN** a keystore records a cost greater than this build writes but within the bound
- **THEN** the derivation is attempted
- **AND** the keystore opens if the passphrase is correct

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

### Requirement: A keystore in a directory others can write to is refused

Loading SHALL also refuse when the keystore's containing directory is writable by any user other than its owner.

A keystore's own permissions do not protect it from someone who can write to the directory holding it: that user can delete it and put their own in its place, or place something at a name a subsequent write will touch. The file's mode says nothing about either.

The check SHALL be on write access only. A directory others may read discloses that a keystore exists, which is not a secret — the path is a documented convention.

This SHALL be reported distinguishably from the file's own permissions being too open, because the fix is a different change to a different path.

#### Scenario: A group- or world-writable directory is refused

- **WHEN** the keystore's directory grants write access to group or other
- **THEN** loading fails
- **AND** the error names the directory, not the keystore file, as the problem

#### Scenario: A readable but not writable directory is accepted

- **WHEN** the keystore's directory grants read access to others but not write access
- **THEN** loading proceeds

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

### Requirement: No intermediate file may be captured by another process

Any path a keystore's bytes are written to before reaching their destination SHALL be unguessable, and SHALL be created only if nothing already occupies it.

Writing to a predictable intermediate path is a disclosure of the root secret, not merely untidy. Anyone able to write to the **containing directory** — which is a weaker capability than writing to the keystore itself, and is the same attacker the permission check exists to defend against — can place a symlink at a predictable name and have the secret written through it to a location and mode of their choosing. The write then completes normally, so nothing observable to the user indicates it happened.

Setting restrictive permissions at creation time does not prevent this: those permissions apply only when the file is genuinely created, and an existing path is opened with whatever permissions it already has.

#### Scenario: A pre-placed symlink cannot capture the secret

- **WHEN** a symlink is placed at the path a write would stage through
- **AND** a keystore is then written
- **THEN** no key material reaches the symlink's target
- **AND** the symlink's target is not truncated or modified

#### Scenario: The intermediate path is not predictable

- **WHEN** two writes stage to the same destination
- **THEN** they use different intermediate paths

#### Scenario: An occupied intermediate path is not reused

- **WHEN** anything already exists at the intermediate path
- **THEN** the write fails rather than opening or truncating it

### Requirement: The permissions checked are the permissions of the bytes read

The permission check and the read SHALL be performed against the same opened file, not against the same path name resolved twice.

Two resolutions of one name are two different files as far as an attacker is concerned: what is checked can be replaced before what is read. A check performed on a path also follows symlinks, so it can describe a file other than the one whose contents are used.

#### Scenario: A keystore is read at most once per operation

- **WHEN** an operation needs both the keystore's protection state and its contents
- **THEN** the file is read once and both answers come from those bytes

#### Scenario: Oversized input is refused without being read into memory

- **WHEN** the file at the keystore path is larger than any valid keystore
- **THEN** loading fails
- **AND** the whole file is not read into memory first

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
