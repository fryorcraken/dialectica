## MODIFIED Requirements

### Requirement: A public key that can never verify a signature is refused at the parse

Parsing a public key SHALL reject a byte string that is not a valid key, and
SHALL separately reject a well-formed key under which no signature can ever
verify. The two SHALL be reported distinguishably.

The second refusal is not defence in depth. Such a key parses and carries
through any record that names it — so a Stoa genesis record naming one decodes,
self-authenticates, and yields a forum whose sole moderator can never authorise
anything, which no later check distinguishes from a legitimate Stoa. Refusing at
the parse is what makes every key a caller holds one that could in principle
sign.

Refusal SHALL be confined to keys that can never verify: a key produced by
ordinary generation SHALL always parse.

#### Scenario: A byte string that is not a valid key is refused

- **WHEN** 32 bytes that are not a valid public key are parsed
- **THEN** parsing fails

#### Scenario: A key that can never verify a signature is refused

- **WHEN** a well-formed public key under which no signature can ever verify is
  parsed
- **THEN** parsing fails
- **AND** the failure is distinguishable from "this is not a key"

#### Scenario: A legitimate key is unaffected

- **WHEN** a freshly generated key's bytes are parsed
- **THEN** parsing succeeds

#### Scenario: Such a key verifies nothing even when obtained around the parse

- **WHEN** a key that can never verify a signature is held despite the parse
  guard, and any signature is checked under it
- **THEN** verification fails

### Requirement: The derivation constants are pinned against silent change

Every constant participating in a Stoa address, a signing digest or a key
derivation SHALL be pinned to an independently derived known answer.

Each is consensus-critical in the silent direction: change one byte of a prefix
or of the derivation salt, and this peer's Stoa addresses and signatures stop
matching every other peer's — with no error anywhere, because each peer remains
internally consistent. A test that recomputes an expectation from the constant it
is meant to guard cannot see this.

**The author address pin is retired rather than relaxed.** Three pins remain
because three derivations remain; the fourth is gone because the derivation it
guarded is gone, not because it stopped mattering. The record's key count, which
only the author-address preimage carried, goes with it.

#### Scenario: An author address derivation is pinned

- **WHEN** the values an author is identified by are enumerated
- **THEN** no address derivation over a public key is among them, there being no
  author address to pin

  The author address is deleted by this change, so this scenario pins its
  absence rather than its value. It is kept rather than dropped because a pin
  that simply disappeared would leave no evidence that the constant it guarded
  was retired on purpose, and the next reader adding an author-side derivation
  would find nothing saying one had been removed.

#### Scenario: A Stoa address derivation is pinned

- **WHEN** a Stoa address is derived from fixed bytes
- **THEN** it equals a value derived independently of this implementation

#### Scenario: The signing digest is pinned

- **WHEN** the signed value over fixed bytes is computed
- **THEN** it equals a value derived independently of this implementation

#### Scenario: Per-Stoa key derivation is pinned

- **WHEN** a key is derived from a fixed root and a fixed Stoa address
- **THEN** it equals a value derived independently of this implementation

### Requirement: A Stoa address's display form parses strictly

A Stoa address SHALL have a display form a user can copy, and parsing that form
SHALL accept only the exact form: correct alphabet and exact length, with a
failure that distinguishes the two.

This parser meets attacker-supplied content, because a Stoa address may appear
inside a post. A lenient parser that accepted a truncated or over-long address
would let two different Stoas collide in what a reader sees, and would let a
moderation action name something other than what it appears to name.

**The requirement is stated over a Stoa address because that is the only kind
there is.** It previously said "an address" while the same type served authors
too; naming the Stoa explicitly is what stops a later reader taking the narrower
scope for a case the requirement lost.

#### Scenario: An address survives its display form

- **WHEN** a Stoa address is rendered and parsed back
- **THEN** the result equals the original

#### Scenario: A wrong alphabet is refused

- **WHEN** text outside the display form's alphabet is parsed as a Stoa address
- **THEN** parsing fails reporting the alphabet, not the length

#### Scenario: A wrong length is refused on both sides

- **WHEN** text one unit shorter or one unit longer than a Stoa address is parsed
- **THEN** parsing fails reporting the length it found

### Requirement: Verification establishes that the key carried by the op signed it

Verifying an op that arrived over the wire SHALL establish that the op's bytes
were signed by the secret half of the public key the op carries. An op whose
signature does not verify under that key SHALL be refused.

**The author is the key.** An op names its author by carrying that author's
public key, so there is no separate claimed identifier for a key to be checked
against and no step that could bind one to the other. The forgery the previous
contract described — sign with your own key while naming somebody else's
identifier — is not expressible: naming a different author means carrying a
different key, and the signature then fails under it.

This is a narrowing of what verification claims, and it is stated rather than
left implicit. The previous requirement obliged a re-derivation check in the same
operation as the signature check, on the reasoning that the check was the step a
caller doing the work by hand would forget. With one value doing both jobs there
is nothing left to forget, and an implementation retaining a check would be
comparing a value to itself.

A failed verification SHALL report only that it failed. There is exactly one
thing to do with an op that does not verify — drop it — so distinguishing why
would offer a choice that does not exist.

#### Scenario: An op verifies when the key matches the claimed author

- **WHEN** an op signed by a key is verified, and the op carries that key
- **THEN** verification succeeds

  The op's carried key **is** the author it claims, so "matches" is satisfied by
  construction rather than by a comparison. The scenario keeps its name because
  it is the case this delta alters; what it checks is that an honestly signed op
  verifies.

#### Scenario: A validly signed op under the wrong key is refused

- **WHEN** an op's carried public key is replaced with a different well-formed
  key, leaving the signature as it was
- **THEN** verification fails
- **AND** the same signature still verifies under the original key, so the
  refusal is the signature not matching the carried key rather than a malformed
  signature

  **This is the one scenario whose mechanism changes.** It previously separated
  a valid signature from a mismatched author identifier, and the refusal came
  from the address binding. There is no identifier to mismatch now: substituting
  the key *is* claiming a different author, and the signature check is what
  refuses it. The scenario is testable exactly as written and its second clause
  is what keeps it honest — without it, a build that refused every op would pass.

#### Scenario: A tampered op does not verify

- **WHEN** the bytes an op was signed over are changed after signing
- **THEN** verification fails

#### Scenario: Verification takes no author identifier beside the key

- **WHEN** an op is verified
- **THEN** the values consulted are the op's signed bytes, the public key the op
  carries, and the signature
- **AND** no separate author identifier is supplied or derived

### Requirement: Identity does not rotate, and this is a contract not an omission

There SHALL be no way to replace the key behind an identity while keeping the
identity. A per-Stoa identity is derived once and is permanent.

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

#### Scenario: An identity is a pure function of its root and its Stoa

- **WHEN** an identity is derived at any time from a given root and Stoa
- **THEN** the result is the same key
- **AND** no operation exists that yields a different key for the same pair

#### Scenario: An identity is named by its key and by nothing beside it

- **WHEN** the values by which an identity is reported are enumerated
- **THEN** the public key is among them
- **AND** it is the only identifier among them

  Stated as "the only identifier" rather than as "no identifier that would
  survive the key changing", because the key cannot change — this requirement
  forbids it — so a scenario written over that counterfactual could never be
  run. What is checkable is how many identifiers an identity is reported by.

## RENAMED Requirements

- FROM: `### Requirement: Verification binds the key to the claimed author`
- TO: `### Requirement: Verification establishes that the key carried by the op signed it`

The old name asserts the property this change removes. There is no claimed author
beside the key to bind it to, so a requirement still called "binds the key to the
claimed author" would name a step its own body says does not exist.

- FROM: `### Requirement: An address's display form parses strictly`
- TO: `### Requirement: A Stoa address's display form parses strictly`

The unqualified name was accurate while one type served both kinds of address.
With only the Stoa kind left, the unqualified name reads as a requirement that
lost a case rather than one whose whole subject was always the Stoa address — the
rationale it already gives names only the Stoa.

## REMOVED Requirements

### Requirement: An address is derived from a record, never from a bare key

**Reason**: The public key is now the sole author identifier, so there is no
author address for this requirement to constrain. Every sentence and all three of
its scenarios were about the author address specifically: that it hashes a record
carrying a key count rather than a bare key, and that its domain prefix keeps it
from colliding with a Stoa address. With the value deleted, the requirement
describes a derivation that does not run.

It also stood in direct contradiction with the `generated-names` requirement *A
display name is derived from a public key and from nothing else*, which states
that the public key is the sole author identifier and that there is no address to
derive from. Two live capabilities answering the same question oppositely is the
condition this removal ends.

**Migration**: An author is identified by its public key. Where an author address
was derived, held, compared or displayed, the public key's own bytes take its
place — both values are 32 bytes and render as 64 hex characters, so no field
changes shape, length or type. A caller holding a previously-reported author
address cannot convert it to the key, the derivation having been one-way; it must
re-read the author from the reply that now carries the key.

**Stoa addresses are not affected.** A Stoa is still identified by its address,
that address is still a domain-separated hash of its genesis record, and the
requirement *A Stoa address's display form parses strictly* above continues to
govern its display form. The domain separation between an author address and a
Stoa address is not preserved but retired: with only one kind of address left,
there is no second kind for it to be separated from.
