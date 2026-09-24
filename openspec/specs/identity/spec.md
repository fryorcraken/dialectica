# identity Specification

## Purpose

Defines what the identity in use is, how a user's per-Stoa identity is derived, what a Stoa address is derived from, and what a verification of an op does and does not establish — so that an identity is permanent, no attacker-supplied key, signature or address can crash a parser or pass as someone else's, and a pseudonym stable within a Stoa can be unlinkable across Stoas.

In this release the identity in use is one machine key, the same in every Stoa, so unlinkability across Stoas is suspended rather than provided: the same user in two Stoas is visibly one key. The per-Stoa derivation that unlinkability rests on stays built and is contracted here, and the keys it yields become the identity in use again when per-Stoa identity is restored.

## Requirements

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

### Requirement: The derivation constants are pinned against silent change

Every constant participating in a Stoa address, a signing digest or a key
derivation SHALL be pinned to an independently derived known answer.

Each is consensus-critical in the silent direction: change one byte of a prefix
or of the derivation salt, and this peer's Stoa addresses and signatures stop
matching every other peer's — with no error anywhere, because each peer remains
internally consistent. A test that recomputes an expectation from the constant it
is meant to guard cannot see this.

**The author address pin is retired rather than relaxed.** Four derivations
remain pinned — the Stoa address, the signing digest, the per-Stoa key, and the
per-Stoa key at an explicit path — and the derivation that is gone is gone
because it no longer exists, not because it stopped mattering. The record's key
count, which only the author-address preimage carried, goes with it.

The count of *pins* is one higher than the count of derivations, because the
path-taking derivation is pinned at two inputs: path 1, and path 0 where it
would collide with the pathless scheme if the salt bump were reverted. State the
requirement over derivations rather than over assertions — an implementation is
free to pin a derivation at more inputs than this spec enumerates, and a bare
count of `assert_eq!`s has already been got wrong here in both directions.

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

#### Scenario: Per-Stoa key derivation at an explicit path is pinned

- **WHEN** a key is derived from a fixed root, a fixed Stoa address and an
  explicit path, at path 0 as well as at a non-zero path
- **THEN** each equals a value derived independently of this implementation

  Path 0 is named because it is where the path-taking scheme and the pathless
  one would produce the same key if the salt separating them were reverted, so
  it is the input at which a silent merge of the two schemes would first show.
  This scenario was absent while the requirement's prose counted three pins, and
  its absence is how that undercount survived review.

### Requirement: A Stoa address's display form parses strictly

A Stoa address MUST have a display form a user can copy: its bytes as
lowercase hexadecimal, two characters per byte. Parsing MUST accept only text of
exactly that length, every character of which is a hexadecimal digit, with a
failure that distinguishes a wrong alphabet from a wrong length.

**Letter case is not part of what parsing checks.** The hexadecimal digits
include the letters `a` to `f` in either case, so parsing MUST accept text whose
letters are uppercase, or any mix of cases, as naming the same address the
display form names. Any character that is not a hexadecimal digit in either case
is outside the alphabet.

**Wherever the module reports a Stoa address, it MUST report the display form**,
whatever case the address arrived in. An address taken in with uppercase letters
and reported back is reported in lowercase.

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

#### Scenario: An address in uppercase or mixed case parses to the same address

- **WHEN** a Stoa address's display form is parsed with every letter
  uppercased, and again with only some letters uppercased
- **THEN** both results equal the address the display form names
- **AND** rendering either result gives the display form, in lowercase

#### Scenario: A reply reports an address in its display form whatever case it was asked in

- **WHEN** a call that takes a Stoa address and reports one back in its reply is
  given that address with every letter uppercased
- **THEN** the call succeeds as it would for the display form
- **AND** the address its reply reports is the display form, in lowercase

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

### Requirement: Identity does not rotate: the key behind an identity is never replaced

There SHALL be no way to replace the key behind an identity while keeping the
identity. A per-Stoa identity is derived once and is permanent.

**In this release the key behind the identity in use is the machine key, in every
Stoa** — see *In this release one machine key is the identity in every Stoa*.
Calling the operation that creates a master key while one is held MUST leave the
identity in use unchanged: the identity report MUST name the same public key
afterwards as before, and an op published afterwards MUST be signed by that same
key. The operation meant is the one `identity-onboarding` provides for a peer to
obtain its master key (*A peer with no master key can obtain one without naming
a Stoa*), not the one that reports whether a master key is held (*Whether this
peer holds a master key is reportable without creating one*), which writes
nothing by its own requirement.

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

#### Scenario: Creating a master key while one is held does not replace the identity in use

- **WHEN** a peer holding a machine key reports the identity in use for a Stoa,
  then calls the operation that creates a master key, then reports the identity
  in use for that Stoa again and publishes a post into it
- **THEN** the second report names the public key the first report named
- **AND** the post carries that same public key

#### Scenario: An identity is named by its key and by nothing beside it

- **WHEN** the values by which an identity is reported are enumerated
- **THEN** the public key is among them
- **AND** it is the only identifier among them

  Stated as "the only identifier" rather than as "no identifier that would
  survive the key changing", because the key cannot change — this requirement
  forbids it — so a scenario written over that counterfactual could never be
  run. What is checkable is how many identifiers an identity is reported by.
