## ADDED Requirements

### Requirement: Core exposes the derivation to a caller

Core SHALL expose a way for a caller to obtain the display name for a public key
it supplies. A caller holding a well-formed public key SHALL be able to reach that
key's name without holding any wordlist, any digest, or any part of the derivation
itself.

**This is what makes every other requirement of this capability observable.** The
derivation, the wordlists, the pins and the determinism contract are all defined
here, and until an entry point exists none of them can be exercised by anything
outside core — the capability describes behaviour no caller can reach.

**The absence of an entry point is not neutral; it forces a second
implementation.** The view runs in a sandbox with no network and no filesystem
outside its plugin directory, so it holds none of the wordlists and cannot derive
a name for itself. Without a call into core, the derivation gets written a second
time in the view, and two implementations of one scheme is exactly the "one key,
two names" divergence this capability's pinning requirements exist to prevent.

**What is exposed is the derivation and nothing more.** Reaching a name SHALL
require supplying a public key, so a caller obtains a name only for key material
it already holds. This does not relax *The name SHALL NOT travel*: a name reaches
a caller because that caller asked for one with a key in hand, and never as a
field of a reply reporting somebody else's authorship.

**The entry point SHALL take a public key**, and these requirements pin that input
and the answer, not the computation between them. Which bytes the derivation reads
and what it hashes are settled by this capability's other requirements and are not
restated here — a caller supplies a key and receives that key's name, and that
contract holds whatever those requirements come to say.

**An author address SHALL NOT be required**, in addition to or in place of the key.
A caller holding only an address cannot arrive at the right name, which this
capability already establishes; an entry point demanding one would oblige every
caller to carry a value the derivation does not use.

#### Scenario: A name is obtainable for a supplied public key

- **WHEN** a caller supplies a well-formed public key and asks for its name
- **THEN** that key's display name is returned

#### Scenario: The name returned is the derivation's name for that key

- **WHEN** a name is obtained through the entry point for a public key, and the
  derivation this capability specifies is applied to that same key
- **THEN** the two names are the same, so the entry point reaches this derivation
  rather than a second one

#### Scenario: A public key alone is enough, with no address supplied

- **WHEN** a caller asks for a name supplying a public key and no author address
- **THEN** a name is returned, so nothing beyond the key is required of a caller

#### Scenario: The same key reaches the same name on every call

- **WHEN** a name is obtained through the entry point twice for one public key
- **THEN** both calls return the same name

#### Scenario: A name is obtainable for a key belonging to no known identity

- **WHEN** a name is obtained for a well-formed public key this peer has never seen
- **THEN** a name is returned rather than a failure

#### Scenario: Obtaining a name changes nothing

- **WHEN** a name is obtained for a public key, and a name is then obtained for a
  second public key
- **THEN** the second call's name is unaffected by the first
- **AND** obtaining a name neither stores anything nor alters what any later call
  answers

### Requirement: The entry point refuses anything that is not a public key, rather than naming it

Where a caller supplies key material that is not a well-formed public key, the
entry point SHALL refuse it and SHALL NOT return a name.

**"Not a well-formed public key" is decided by the same rule that admits a key
anywhere else on this surface, and it is wider than a length check.** The entry
point SHALL refuse every input the identity layer refuses as a public key — which
includes material of the wrong length, material of the right length that is not a
valid curve point, and **a low-order point, which is the case a length check
misses.** A low-order point is the right length, decodes, and is still refused
everywhere else on this surface; an entry point that accepted one would name a key
that can never verify a signature, and would be the one place in the module where
a weak key is admitted. This requirement is stated against the identity layer's
answer rather than against a list of shapes so that it cannot drift from it.

**Failing to supply key material at all is a distinct refusal from supplying bad
key material**, and the two SHALL be distinguishable by their messages. Both are
refusals and neither is a name; a caller that cannot tell them apart cannot tell a
request it malformed from a key it should stop trusting.

**Key material that parses is the whole of what can fail.** The derivation is
total over well-formed keys, so once key material is accepted as a public key
there is nothing left that can fail. The entry point SHALL therefore report no
further error condition, and in particular none that a well-formed key belonging
to a real identity could reach: an error a caller cannot provoke is a branch a
caller handles for nothing and a reader takes as evidence of a failure that does
not exist.

A refusal SHALL NOT be reported as a name. There SHALL be no placeholder name, no
name for "unknown", and no name derived from truncated or padded input — each
would render as an ordinary participant, which is a name attributable to nobody
presented as one attributable to somebody.

A refusal SHALL NOT abort the process. Key material reaching this entry point is
attacker-controlled: it arrives inside an inbound op and is handed on by a view
rendering somebody else's authorship. A crash here takes the module down for every
caller, which makes a refusal that panics a remotely triggerable denial of service
rather than a robustness nicety.

#### Scenario: Key material of the wrong length is refused

- **WHEN** a caller supplies key material shorter or longer than a public key
- **THEN** the call is refused
- **AND** no name is returned

#### Scenario: A low-order point is refused rather than named

- **WHEN** a caller supplies a low-order point — key material of the correct
  length that decodes but that the identity layer refuses as a public key
- **THEN** the call is refused
- **AND** no name is returned, so the refusal is not a length check wearing a
  wider name

#### Scenario: The entry point admits exactly what the identity layer admits

- **WHEN** key material is supplied to the entry point, and the same material is
  offered to the identity layer's public key parse
- **THEN** the entry point returns a name in exactly the cases the identity layer
  accepts the material, and refuses in exactly the cases it refuses

#### Scenario: Absent key material is refused distinguishably from bad key material

- **WHEN** a caller supplies no key material at all, and then supplies key material
  that is not a well-formed public key
- **THEN** both calls are refused
- **AND** the two refusals carry different messages, so a caller can tell a request
  it malformed from a key it should stop trusting

#### Scenario: A refusal is not a name

- **WHEN** the entry point refuses key material for any reason
- **THEN** no name is returned in the refusal
- **AND** no placeholder or fallback name is returned in its place

#### Scenario: Arbitrary key material does not take the module down

- **WHEN** arbitrary byte strings are supplied as key material in turn
- **THEN** each call answers, rather than terminating the process
- **AND** the entry point answers a later well-formed call normally

#### Scenario: A well-formed key reaches no failure

- **WHEN** names are obtained for many distinct well-formed public keys
- **THEN** every call returns a name
- **AND** none reports a failure, so key material is the only thing that can fail
