# identity Specification

## Requirements

### Requirement: A user has one identity per Stoa, and it is permanent

A user SHALL have exactly one identity within a Stoa, and that identity SHALL be
reproducible from the user's root secret and the Stoa's address alone.

Derivation SHALL depend on nothing else. A session counter, a wall clock, a
device identifier or any other value varying between runs SHALL NOT participate,
because the same root and the same Stoa must yield the same identity on any
device and after any restart.

A stable pseudonym is what lets anything accumulate against an identity — a
moderator's judgement, or a relevance signal — and the transport binds its
sender identifier to it for a channel's lifetime, so instability here would
change the identifier other peers know a user by.

#### Scenario: The same root and Stoa always yield the same identity

- **WHEN** a key is derived twice from one root secret and one Stoa address
- **THEN** both derivations yield the same public key

#### Scenario: A derived identity survives storage and reload

- **WHEN** a derived key's stored form is written out and read back
- **THEN** the restored key has the same public key as the original

#### Scenario: A derived key is a usable identity, not merely a distinct one

- **WHEN** a derived key signs an op
- **THEN** the signature verifies under that key's public half

### Requirement: Identities are unlinkable across Stoas

The same user in two Stoas SHALL be two identities that the protocol does not
relate. Deriving for two different Stoa addresses from one root SHALL yield
different keys, and two different roots in one Stoa SHALL yield different keys.

There SHALL be no way to compute a Stoa identity's public half from the root's
public half. This is the property cross-Stoa unlinkability rests on: were public
derivation available, anyone holding a user's root public key could enumerate
their pseudonym in every Stoa they participate in.

This is a privacy guarantee about the protocol and not about the world. Within a
Stoa a pseudonym is deliberately stable, and writing style, posting time and the
reply graph link a user's identities whatever the key derivation does.

#### Scenario: Two Stoas yield different identities from one root

- **WHEN** one root secret is used to derive identities for two different Stoas
- **THEN** the two public keys differ

#### Scenario: Two users in one Stoa do not collide

- **WHEN** two different root secrets are used to derive identities for one Stoa
- **THEN** the two public keys differ

#### Scenario: A per-Stoa identity is not the root identity

- **WHEN** a per-Stoa key is derived from a root secret
- **THEN** its public key differs from the public key of the root used directly

### Requirement: A public key that can never verify a signature is refused at the parse

Parsing a public key SHALL reject a byte string that is not a valid key, and
SHALL separately reject a well-formed key under which no signature can ever
verify. The two SHALL be reported distinguishably.

The second refusal is not defence in depth. Such a key parses, produces a stable
address, and carries through any record that names it — so a Stoa genesis record
naming one decodes, self-authenticates, and yields a forum whose sole moderator
can never authorise anything, which no later check distinguishes from a
legitimate Stoa. Refusing at the parse is what makes every key a caller holds
one that could in principle sign.

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

### Requirement: A wrong-length key or signature is an error, never a panic

Parsing a public key, a secret key or a signature SHALL reject input of any
length other than the one the scheme defines, and SHALL do so by returning an
error.

Every one of these values arrives inside an inbound op and is
attacker-controlled. A panic on this path aborts the module process, the caller
learns only that its call timed out, and every later call reports the module as
not loaded — so a parser that panics on a short slice is a remotely triggerable
denial of service, not a robustness nicety.

#### Scenario: A key of the wrong length is refused

- **WHEN** a byte string shorter or longer than a public key is parsed
- **THEN** parsing fails rather than panicking

#### Scenario: A signature of the wrong length is refused

- **WHEN** a byte string shorter or longer than a signature is parsed
- **THEN** parsing fails rather than panicking

#### Scenario: Malformed fields on the verification path are a refusal

- **WHEN** an op is verified with a key or a signature of any wrong length or
  with arbitrary bytes in either
- **THEN** verification reports failure rather than panicking

### Requirement: Signing is domain-separated by purpose

A signature SHALL commit to a fixed prefix naming what it is for, so that a
signature made over one kind of payload cannot be presented as a signature over
something else that happens to share bytes.

The prefix SHALL be fixed-width. A variable-length prefix concatenated with
variable-length data is how two different inputs come to share a preimage.

This prefix separates a dialectica op signature from a signature over anything
else. It does **not** separate one op kind from another, because every op is
signed under the same prefix — that separation comes from the op encoding being
unambiguously typed, and belongs to the `op-format` capability.

#### Scenario: A signed payload is not a bare hash of itself

- **WHEN** the value signed over a payload is compared with the undomain-
  separated hash of that same payload
- **THEN** they differ

#### Scenario: A signature verifies over the bytes it was made over

- **WHEN** a signature is made over some bytes and checked against those bytes
  under the signing key's public half
- **THEN** verification succeeds

#### Scenario: A signature does not verify over different bytes

- **WHEN** a signature is checked against bytes other than those it was made
  over
- **THEN** verification fails

#### Scenario: A signature does not verify under a different key

- **WHEN** a signature made by one key is checked under a different key
- **THEN** verification fails

### Requirement: An address is derived from a record, never from a bare key

An author's address SHALL be a fixed-width domain-separated hash of a record
containing the key, not of the key alone. The record SHALL carry its own key
count, so that a record holding more than one key is a different preimage rather
than an ambiguous one.

Hashing a record rather than the key is what lets a key log be added later
without every author's address changing. It costs nothing today and hashing the
raw key would foreclose it permanently.

An author address and a Stoa address SHALL be separated by their domain
prefixes, so that no byte string is ever valid as both. Both are 32 bytes and
appear in the same places, so without separation either could be presented as
the other.

#### Scenario: An address is not a bare hash of the key

- **WHEN** an author's address is compared with the plain hash of that author's
  public key
- **THEN** they differ

#### Scenario: Different keys get different addresses

- **WHEN** two different public keys are converted to addresses
- **THEN** the addresses differ

#### Scenario: An author address and a Stoa address never collide

- **WHEN** the exact preimage an author address is derived from is instead
  hashed as a Stoa address
- **THEN** the two results differ

### Requirement: The derivation constants are pinned against silent change

Every constant participating in an address, a signing digest or a key derivation
SHALL be pinned to an independently derived known answer.

Each is consensus-critical in the silent direction: change one byte of a prefix,
of the record's key count, or of the derivation salt, and this peer's addresses
and signatures stop matching every other peer's — with no error anywhere,
because each peer remains internally consistent. A test that recomputes an
expectation from the constant it is meant to guard cannot see this.

#### Scenario: An author address derivation is pinned

- **WHEN** an address is derived from fixed key material
- **THEN** it equals a value derived independently of this implementation

#### Scenario: A Stoa address derivation is pinned

- **WHEN** a Stoa address is derived from fixed bytes
- **THEN** it equals a value derived independently of this implementation

#### Scenario: The signing digest is pinned

- **WHEN** the signed value over fixed bytes is computed
- **THEN** it equals a value derived independently of this implementation

#### Scenario: Per-Stoa key derivation is pinned

- **WHEN** a key is derived from a fixed root and a fixed Stoa address
- **THEN** it equals a value derived independently of this implementation

### Requirement: An address's display form parses strictly

An address SHALL have a display form a user can copy, and parsing that form
SHALL accept only the exact form: correct alphabet and exact length, with a
failure that distinguishes the two.

This parser meets attacker-supplied content, because a Stoa address may appear
inside a post. A lenient parser that accepted a truncated or over-long address
would let two different Stoas collide in what a reader sees, and would let a
moderation action name something other than what it appears to name.

#### Scenario: An address survives its display form

- **WHEN** an address is rendered and parsed back
- **THEN** the result equals the original

#### Scenario: A wrong alphabet is refused

- **WHEN** text outside the display form's alphabet is parsed as an address
- **THEN** parsing fails reporting the alphabet, not the length

#### Scenario: A wrong length is refused on both sides

- **WHEN** text one unit shorter or one unit longer than an address is parsed
- **THEN** parsing fails reporting the length it found

### Requirement: Verification binds the key to the claimed author

Verifying an op that arrived over the wire SHALL check that the key presented
belongs to the author being claimed, in addition to checking the signature. A
valid signature under a key that is not the claimed author's SHALL be refused.

Verifying a signature proves that whoever holds a key's secret signed some
bytes, and says nothing about who they are. An attacker can generate a key, sign
anything with it, and attach whatever author address they like; only re-deriving
the address from the key catches that. The check SHALL live in the same
operation as the signature check rather than being left to a caller to perform
separately — it is the step that goes missing, because the other steps are
visibly load-bearing and this one looks like bookkeeping.

A failed verification SHALL report only that it failed. There is exactly one
thing to do with an op that does not verify — drop it — so distinguishing why
would offer a choice that does not exist.

#### Scenario: An op verifies when the key matches the claimed author

- **WHEN** an op is verified with the key whose address is the claimed author's
- **THEN** verification succeeds

#### Scenario: A validly signed op under the wrong key is refused

- **WHEN** an op signed with a valid signature under one key claims a different
  author
- **THEN** verification fails
- **AND** the signature itself still verifies under its own key, so the refusal
  is the address binding and not a broken signature

#### Scenario: A tampered op does not verify

- **WHEN** the bytes an op was signed over are changed after signing
- **THEN** verification fails

### Requirement: Authenticity is not authority

A successful verification SHALL mean exactly that this op is authentically from
the author it claims. It SHALL NOT be read as meaning the op is permitted.

Whether an author may moderate, may revise a given post, or may post at all
under a Stoa's policy each requires state an op does not carry, and each is
settled elsewhere on read. A caller that treats a successful verification as
"this op is valid" is therefore wrong, and the contract says so rather than
leaving it to be inferred.

#### Scenario: Verification consults no authority state

- **WHEN** an op is verified
- **THEN** the check uses only the claimed author, the key, the signed bytes and
  the signature
- **AND** no moderator set, genesis record or posting policy is consulted

### Requirement: Identity does not rotate, and this is a contract not an omission

There SHALL be no way to replace the key behind an identity while keeping the
identity. A per-Stoa identity is derived once and is permanent.

A key that can be discarded at will is a key nothing can be attached to:
rotation lets a user shed whatever has accumulated against their identity, and
does so indistinguishably from a legitimate compromise. Rotation waits until
standing attaches to a revocable credential rather than to a keypair, and the
record-hashed address is what keeps that door open.

#### Scenario: An identity is a pure function of its root and its Stoa

- **WHEN** an identity is derived at any time from a given root and Stoa
- **THEN** the result is the same key
- **AND** no operation exists that yields a different key for the same pair

### Requirement: A secret key cannot be copied, logged or serialised by accident

A secret key SHALL NOT be duplicable, renderable for display, or serialisable
through any interface this capability provides. Each of those is a way a secret
reaches a log line, a reply or a second copy nobody tracks, and none is needed
in order to sign.

A secret key SHALL be reconstructible from its stored form, because a keystore
must be able to hand one back.

**This does not cover memory.** A stored form handed out is a plain byte array
this capability no longer controls, and the intermediate values in generation
and derivation are ordinary stack memory. Owning a secret's lifetime is the
keystore's job; what is claimed here is only that a secret does not reach a log
or a wire through this surface.

#### Scenario: A secret key survives a storage round trip

- **WHEN** a secret key's stored form is written out and read back
- **THEN** the restored key has the same public key as the original

#### Scenario: Every stored form of the right length is a valid secret key

- **WHEN** any byte string of the secret key's length is read as a secret key
- **THEN** it is accepted
- **AND** a wrong length is the only failure this parse reports
