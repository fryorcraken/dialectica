# stoa-metadata Specification

## ADDED Requirements

### Requirement: A Stoa's current metadata is carried by a signed op

A Stoa's **display metadata** — what it is called and how it describes itself
today — SHALL be carried by a signed op, separate from the genesis record.

The genesis record's title is a **founding** value: it is inside the address
preimage, so it can never change without minting a different Stoa. The metadata
op carries what the Stoa is called *today*. The two are different questions
about the same Stoa and both SHALL remain answerable.

A metadata op SHALL carry the Stoa address it applies to, its signer's public
key, a display title, and a description.

The description is a field the genesis record does not have at all, which is
part of why this op exists rather than being a mechanism for overriding genesis
values one-for-one.

#### Scenario: A metadata op names the Stoa it applies to

- **WHEN** a metadata op is encoded
- **THEN** the encoding carries a Stoa address
- **AND** the address is inside the bytes the signature covers

#### Scenario: Renaming a Stoa does not change its address

- **WHEN** a Stoa's metadata op declares a title different from its genesis title
- **THEN** the Stoa's address, computed from the genesis record, is unchanged

#### Scenario: The genesis title remains readable after a rename

- **WHEN** a metadata op declares a new title
- **THEN** the genesis record still decodes to its founding title
- **AND** the two titles are separately available

### Requirement: A metadata op carries no posting policy

A metadata op SHALL NOT carry a posting policy field.

A rename is cosmetic; a policy change is authorisation. Three things rule the
field out of *this* op rather than merely deferring it by taste:

- The reader rule for display metadata is "prefer the latest valid op, fall back
  to the genesis value". Applied to a policy, fallback means a peer that has not
  yet received a tightening op treats the Stoa as carrying its *founding*
  policy. For a title that is a stale name; for a policy it is admitting posters
  the Stoa has since excluded — the same silent widening that `stoa-genesis`
  refuses when it declines to default an unknown policy discriminant.
- A policy change is retroactive in a way a rename is not: it alters who may
  post, for everyone, including on content already published.
- The policy enum has exactly one accepted value, so a metadata op carrying one
  could not express a change. Behaviour that cannot be varied cannot be
  specified as varying.

Omitting the field SHALL NOT cost an encoding version to add later. A future
policy-changing act SHALL be expressible as a new op kind taking an unused kind
discriminant, so an older client refuses it as an unknown kind rather than
misreading it, and no existing op's encoding or address changes.

#### Scenario: The metadata op's encoding accounts for exactly its declared fields

- **WHEN** a metadata op is encoded
- **THEN** the encoding's length is fully accounted for by the version, kind,
  Stoa address, signer key, title and description
- **AND** a policy field cannot be present without that accounting failing

#### Scenario: An unknown op kind is refused rather than reinterpreted

- **WHEN** an op declares a kind discriminant this build does not recognise
- **THEN** decoding fails, naming the unrecognised kind
- **AND** it is not read as any kind this build does know

### Requirement: The op kind is inside the signed bytes

A metadata op's kind discriminant SHALL be inside the bytes its signature
covers, at the same fixed position every op kind uses.

There is one signing prefix for all ops, so a signature commits to "some
dialectica op" and not to which one. Only an unambiguously typed preimage
separates the kinds. Without it, a signature authorising one act would authorise
another — and a metadata op is a moderator-signed act, so the kind that a
signature over it could be mistaken for is a moderation.

#### Scenario: A signature over a metadata op does not verify as another kind

- **WHEN** a metadata op is signed, and its signature is attached to an op of a
  different kind with the same Stoa, signer and otherwise-identical fields
- **THEN** verification of the substituted op fails
- **AND** the original metadata op still verifies

#### Scenario: A metadata op and another kind never share a preimage

- **WHEN** a metadata op and an op of another kind are built to be as similar as
  the two field sets allow
- **THEN** their encodings differ
- **AND** their op ids differ

### Requirement: The metadata op's encoding is canonical

A metadata op SHALL have exactly one valid byte encoding. Each variable-length
field SHALL be length-prefixed, so that no two distinct ops encode to the same
bytes.

Two variable-length fields sit adjacent in this op. Without prefixes the
boundary between them is ambiguous: title `"ab"` with description `"c"` and
title `"a"` with description `"bc"` would produce identical bytes, and therefore
identical op ids for two different acts.

#### Scenario: The same op always encodes identically

- **WHEN** the same metadata op is encoded twice
- **THEN** the two byte strings are identical

#### Scenario: Ops differing in any field encode differently

- **WHEN** two metadata ops differ in the Stoa, the signer, the title or the
  description
- **THEN** their encodings differ
- **AND** their op ids differ

#### Scenario: Adjacent variable-length fields cannot be confused

- **WHEN** one op's title and description are the same concatenated text as
  another's, split at a different point
- **THEN** their encodings differ

#### Scenario: An empty title and an empty description round-trip

- **WHEN** a metadata op with an empty title and an empty description is encoded
  and decoded
- **THEN** the decoded op carries an empty title and an empty description
- **AND** empty is not confused with absent

#### Scenario: Text survives multi-byte UTF-8

- **WHEN** a metadata op's title or description contains multi-byte UTF-8
- **THEN** encoding and decoding return the same text

### Requirement: A malformed metadata op is refused at the decoding boundary

Decoding SHALL reject any input that is not the canonical encoding of a valid
metadata op: truncation at any point, trailing bytes, a length prefix
disagreeing with the input in either direction, a length prefix over the maximum
a delivered message could carry, text that is not valid UTF-8, and a signer key
that is not a valid public key.

Each SHALL be reported distinguishably, so a reader is not sent looking in the
wrong place.

A field's declared length SHALL be checked against the maximum **before** any
allocation is made on the strength of it. A four-byte length prefix can claim
far more than a delivered message could hold, and allocating on a peer's promise
is a remote memory-exhaustion lever.

Every byte here arrives from a peer and is attacker-controlled. No input shape
SHALL cause a panic: a panic in this decoder aborts the module process.

#### Scenario: Truncated input is refused

- **WHEN** a metadata op's encoding is truncated at any point
- **THEN** decoding fails
- **AND** no partially-populated op is produced

#### Scenario: Trailing bytes are refused

- **WHEN** a valid encoding is followed by extra bytes
- **THEN** decoding fails

#### Scenario: A length prefix claiming more than the input holds is refused

- **WHEN** a title or description length prefix claims more bytes than remain
- **THEN** decoding fails
- **AND** the failure is distinguishable from the input merely running out

#### Scenario: A length prefix over the message cap is refused before allocating

- **WHEN** a length prefix claims more bytes than a delivered message could carry
- **THEN** decoding fails naming the over-long field
- **AND** the refusal comes from the cap rather than from exhausting the input

#### Scenario: Text that is not valid UTF-8 is refused

- **WHEN** a metadata op's title or description bytes are not valid UTF-8
- **THEN** decoding fails
- **AND** the bytes are not lossily converted, which would map distinct inputs
  onto one op

#### Scenario: A signer key that is not a valid public key is refused

- **WHEN** a metadata op carries a signer key that is not a valid public key
- **THEN** decoding fails
- **AND** the failure is distinguishable from a malformed encoding

#### Scenario: No input shape causes a panic

- **WHEN** arbitrary or mutated bytes are decoded as a metadata op
- **THEN** every outcome is a success or a named failure

### Requirement: Verification of a metadata op answers authenticity, not authority

Verifying a metadata op SHALL establish only that it is authentically from the
key it names. It SHALL NOT be read as establishing that the signer was a
moderator of the Stoa.

Authority needs the Stoa's moderator set as of the op's position in the order,
which the op type does not have and cannot obtain. A metadata op signed by a
peer who is not a moderator is genuinely from that peer, and SHALL verify as
authentic; whether it binds is a separate question answered on read.

Conflating the two is a measured failure mode: the nearest comparable project
checks moderator authority only on the send path and never on the read path, so
any peer can forge a moderation.

#### Scenario: A metadata op signed by a non-moderator is authentic

- **WHEN** a peer who is not the Stoa's creator signs a metadata op for that Stoa
- **THEN** verification succeeds, because the op really is from that peer
- **AND** this is not a statement that the rename takes effect

#### Scenario: A metadata op signed by someone other than its named signer fails

- **WHEN** a metadata op names one signer but is signed with another key
- **THEN** verification fails

#### Scenario: A tampered metadata op does not verify

- **WHEN** a metadata op's title is changed after signing
- **THEN** verification fails

#### Scenario: A metadata op replayed into another Stoa does not verify

- **WHEN** a metadata op's Stoa address is changed after signing
- **THEN** verification fails
- **AND** it does not arrive as a rename of a Stoa its signer never addressed

### Requirement: A metadata op carries no ordering field

A metadata op SHALL NOT carry a timestamp, a Lamport value, a sequence number,
or any other field asserting its own position in an order.

Resolution between two metadata ops for one Stoa is last-write-wins by the
order the transport establishes. That order is the transport's to supply. A
value the op asserted about itself would be forgeable by the very author it is
meant to order — and for a moderator-owned field that is the difference between
"the latest rename wins" and "whoever claims the highest number wins".

This requirement constrains the op's contents. It does not define the ordering,
which needs ordering metadata the delivery contract does not presently expose.

#### Scenario: The encoding has no room for an ordering field

- **WHEN** a metadata op is encoded
- **THEN** the encoding's length is exactly accounted for by its declared fields
- **AND** an ordering field cannot be added without that accounting failing
