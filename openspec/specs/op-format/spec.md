# op-format Specification

## Purpose
Defines what an op contains, how it encodes to bytes, what a signature over it commits to, and what a decoder does with hostile input — so that every peer agrees on what was signed and on what to call it.

## Requirements

### Requirement: An op is the unit that crosses the wire

An op SHALL be a signed operation, and it SHALL be the only thing that crosses the wire. A peer's view of the forum is a function of the ops it has seen, so anything not expressible as an op is not expressible at all.

An op SHALL carry the Stoa it belongs to, the public key of its author, and exactly one kind describing what it does. The kinds are: a post (which is a reply when it names a parent), a revision of one of the author's own posts, a moderator's judgement about a target, and a vote on a target.

Creating a Stoa SHALL NOT be an op. A Stoa is a genesis record its creator publishes, and hashing that record is what creates it — see the `stoa-genesis` capability. There is nothing for a signed op to add.

Replying SHALL NOT be a separate kind. A reply is a post that names a parent, so that "is this a reply?" is one question about a field rather than a second question about which kind arrived.

The author's **public key** SHALL travel in the op, not merely their address. Verification happens on read with no directory to resolve an address against, so a peer holding only an address could not check the signature. The address SHALL be recoverable from the key rather than carried separately.

#### Scenario: Every kind round-trips through the encoding

- **WHEN** an op of any kind is encoded and decoded
- **THEN** the decoded op equals the original in every field

#### Scenario: A reply is a post that names a parent

- **WHEN** a post op names a parent op
- **THEN** it is the same kind as a post that names none
- **AND** both round-trip through the encoding

### Requirement: The encoding is canonical

An op SHALL have exactly one valid byte encoding, and decoding SHALL accept only that encoding. Every variable-length field SHALL be length-prefixed, so that no two distinct ops encode to the same bytes.

Canonical encoding is what makes an op id agree between peers. Two peers holding the same op MUST compute the same bytes for it, and no accepted byte string MAY re-encode to anything other than itself — otherwise one op would have two ids, and the deduplication every other part of the system relies on would be unsound.

#### Scenario: The same op always encodes identically

- **WHEN** the same op is encoded twice
- **THEN** the two byte strings are identical

#### Scenario: Distinct ops never share an encoding

- **WHEN** two ops differ in any field — Stoa, author, or any field of any kind
- **THEN** their encodings differ
- **AND** their ids differ

#### Scenario: Adjacent variable-length fields cannot be confused for one another

- **WHEN** one post has body "ab" with an attachment "c", and another has body "a" with an attachment "bc"
- **THEN** their encodings differ
- **AND** their ids differ

#### Scenario: An accepted encoding re-encodes to itself

- **WHEN** a byte string is accepted by the decoder
- **THEN** encoding the resulting op reproduces that byte string exactly

#### Scenario: An empty variable-length field is a value, not an absence

- **WHEN** a post carries an empty body and an empty attachment list
- **THEN** it encodes and decodes as such
- **AND** is not confused with a field that is missing

### Requirement: A signature commits to the op kind

The op's kind SHALL be inside the bytes that are signed. A signature over an op of one kind SHALL NOT be a valid signature over an op of any other kind, whatever the two have in common.

There is a single signing domain prefix for all ops, so the cryptographic layer alone cannot distinguish a signature over a vote from a signature over a moderation action. The separation therefore has to come from the encoding: if the kind is part of the signed bytes, two ops of different kinds cannot share a preimage, and the property holds structurally rather than by anyone remembering to check it.

This is a security boundary. Without it, a peer who has published a vote has published a signature that an attacker could re-present as authorising a moderation action against the same target.

#### Scenario: Two ops differing only in kind do not share a signed preimage

- **WHEN** a moderation op and a vote op carry the same Stoa, the same author, the same target, and discriminants that coincide in every other byte
- **THEN** their signed byte strings differ

#### Scenario: A signature over one kind does not verify as another

- **WHEN** the signature from a validly signed vote is attached to a moderation op that is otherwise identical
- **THEN** verification of the moderation op fails
- **AND** the original vote still verifies, so the failure is not because both are broken

### Requirement: A signature commits to the Stoa

The Stoa SHALL be inside the bytes that are signed, and it SHALL be the Stoa's address rather than any channel identifier.

An op lifted from one Stoa's channel and replayed on another's MUST fail verification rather than arriving as a valid post its author never addressed there. Carrying the address rather than a channel id also keeps channel identity out of payloads, so that splitting one channel per Stoa into one per thread later is a routing change rather than a migration of every op ever published.

#### Scenario: An op replayed into another Stoa does not verify

- **WHEN** a validly signed op has its Stoa replaced with a different Stoa's address
- **THEN** verification fails

#### Scenario: An op carries no channel identifier

- **WHEN** an op is encoded
- **THEN** the encoding's length is fully accounted for by the Stoa address, the author key, the version, the kind, and that kind's own fields
- **AND** no channel identifier is among them

### Requirement: Verification answers authenticity, not authority

Verifying a signed op SHALL answer exactly one question: is this op authentically from the author it claims? It SHALL NOT be read as answering whether the op is permitted.

Specifically, verification SHALL succeed for an authentic op even when:

- the signer is not a moderator of the Stoa and the op is a moderation action;
- the signer is not the author of the post a revision claims to supersede;
- the Stoa's posting policy would not admit the signer.

Each of those needs state an op does not carry — the Stoa's moderator set at this op's position in the order, the target op, the genesis record — and each is settled on read. Conflating the two is a measured failure mode: the nearest comparable project checks moderator authority only on the send path and never on the read path, so any peer can forge a moderation there.

A caller that treats a successful verification as "this op is valid" is therefore wrong, and the contract says so rather than leaving it to be inferred.

#### Scenario: A moderation op from a non-moderator is authentic

- **WHEN** a peer who is not a moderator signs a moderation op with their own key
- **THEN** verification succeeds
- **AND** this is not a statement that the moderation binds

#### Scenario: A revision of someone else's post is authentic

- **WHEN** a peer signs a revision naming a post they did not author
- **THEN** verification succeeds
- **AND** whether the revision is valid remains a question for whoever holds the target op

#### Scenario: An op signed by a different key than it claims is rejected

- **WHEN** an op naming one author is signed by a different key
- **THEN** verification fails, even though the signature itself is well-formed

#### Scenario: A tampered op does not verify

- **WHEN** any field of a signed op is changed after signing
- **THEN** verification fails

### Requirement: An op is named by a content-derived id

An op SHALL have a 32-byte id computed from its own canonical bytes under a domain-separation prefix distinct from every address prefix. The id SHALL NOT be assigned by the transport.

A content-derived id is available wherever the op is — replaying a local store, applying a snapshot, deduplicating on ingest — none of which has a transport envelope to consult. An id assigned on delivery would be absent in exactly those settings.

This id is **not** the transport's message id. The ordering rule's tiebreak refers to that one, which arrives with the message and is the transport's to assign. Two identifiers with two jobs; a reader must not substitute one for the other.

Domain separation matters because an op id, an author address and a Stoa address are all 32 bytes, and a moderation op names one of them by value. No byte string may be a valid instance of two of them.

#### Scenario: An op id is domain-separated from a bare hash

- **WHEN** an op's id is compared with the undomain-separated hash of the same canonical bytes
- **THEN** they differ

#### Scenario: An op id is domain-separated from a Stoa address

- **WHEN** the identical byte string is hashed as an op id and as a Stoa address
- **THEN** the two results differ

#### Scenario: The id changes with every field

- **WHEN** two ops differ in any field
- **THEN** their ids differ

#### Scenario: The id derivation is pinned against silent change

- **WHEN** a fixed op, built from fixed key material, has its id computed
- **THEN** the id equals a value derived independently of this implementation
- **AND** a change to the prefix or the layout makes this fail rather than pass quietly

#### Scenario: An id survives its display form

- **WHEN** an op id is rendered as lowercase hex and parsed back
- **THEN** the result equals the original

#### Scenario: A malformed id is refused rather than truncated into a valid one

- **WHEN** text that is not exactly 32 bytes of hex is parsed as an op id
- **THEN** parsing fails
- **AND** the failure distinguishes "not hex" from "the wrong length"

### Requirement: A malformed op is rejected at the decoding boundary

Decoding SHALL reject any input that is not the canonical encoding of a valid op, and SHALL do so before the input reaches any state machine. Every byte reaching the decoder arrived from a peer and is attacker-controlled.

Each of these SHALL be refused, and each SHALL be reported distinguishably from the others: truncated input, trailing bytes, a length prefix disagreeing with the input, an unrecognised version, an unrecognised op kind, an unrecognised moderation action, an unrecognised vote direction, an optional-field tag that is neither absent nor present, text that is not valid UTF-8, and an author key that is not a valid public key.

A decoder that says only "invalid" sends the reader looking in the wrong place. An unrecognised version in particular means "a newer client wrote this", which is a different thing to tell a user than "this is corrupt".

No rejected input SHALL yield a partially-populated op.

#### Scenario: Truncated input is refused at every length

- **WHEN** an op's encoding is truncated at any length, for every kind
- **THEN** decoding fails
- **AND** no partially-populated op is produced

#### Scenario: Trailing bytes are refused

- **WHEN** a complete op's encoding is followed by any extra byte
- **THEN** decoding fails as trailing bytes, distinguishably from truncation

#### Scenario: A length prefix claiming more than the input holds is refused

- **WHEN** a variable-length field's prefix claims more bytes than remain
- **THEN** decoding fails as a length mismatch, distinguishably from truncation

#### Scenario: An unknown version is refused and names itself

- **WHEN** an op declares a version this build does not recognise
- **THEN** decoding fails reporting that version

#### Scenario: An unknown op kind is refused

- **WHEN** an op declares a kind discriminant this build does not recognise
- **THEN** decoding fails reporting that discriminant

#### Scenario: Text that is not valid UTF-8 is refused

- **WHEN** a post's body bytes are not valid UTF-8
- **THEN** decoding fails
- **AND** the bytes are not lossily converted, which would map distinct inputs onto one op

#### Scenario: An author key that is not a valid public key is refused

- **WHEN** an op carries an author key that is not a valid public key
- **THEN** decoding fails
- **AND** the failure is distinguishable from a malformed encoding

#### Scenario: Hostile input is never a panic

- **WHEN** arbitrary byte strings, including every single-byte mutation of a valid op and inputs of unrelated lengths, are decoded
- **THEN** each returns a result rather than panicking

### Requirement: An unknown discriminant is refused, never defaulted

Where a byte selects among named alternatives — the version, the op kind, the moderation action, the vote direction, an optional field's presence tag — a value this build does not recognise SHALL be refused. It SHALL NOT be mapped onto any recognised value, and no alternative SHALL be treated as a default.

The cost of defaulting is asymmetric and security-relevant. Reading an unrecognised moderation action as "hide" applies a moderation nobody performed; reading it as "unhide" silently drops one that was. Reading any non-zero presence tag as "present" lets a sender and a decoder disagree about the format while both proceed, which is how two peers derive different ids for what they each believe is one op.

Refusing means an older build cannot display an op it does not understand. That is the recoverable direction: not showing something is fixable, misrepresenting a moderation decision is not.

#### Scenario: An unknown moderation action is refused rather than defaulted

- **WHEN** a moderation op carries an action discriminant this build does not recognise
- **THEN** decoding fails reporting that discriminant
- **AND** the op is not read as a hide, nor as an unhide

#### Scenario: An unknown vote direction is refused

- **WHEN** a vote op carries a direction discriminant this build does not recognise
- **THEN** decoding fails reporting that discriminant

#### Scenario: An invalid presence tag is refused rather than read as present

- **WHEN** an optional field's tag is neither the absent value nor the present value
- **THEN** decoding fails reporting that tag
- **AND** the field is not read as present

#### Scenario: A presence tag is a tag, not a sentinel value

- **WHEN** an optional op id field is absent
- **THEN** absence is encoded as a tag rather than as a reserved id value
- **AND** every 32-byte id remains representable as a present value

### Requirement: A field longer than the transport allows is refused before allocating

Decoding SHALL refuse any length prefix exceeding the maximum a single message can carry, and SHALL do so before allocating on the strength of that prefix. The limit applies to a variable-length field's byte length and to a list's element count alike.

A four-byte length prefix can claim four gibibytes. The transport caps a message at a network-wide validation limit that cannot be raised unilaterally, so a field larger than that could never have arrived legitimately — and a decoder that reserved memory on a hostile peer's promise before discovering the input was short would be a remote memory-exhaustion lever. Discovering the shortfall afterwards is too late; the allocation has already happened.

The refusal SHALL be reported as an over-long field, distinguishably from running out of input, so that the cap is demonstrably what rejected it.

The limit SHALL be pinned at its boundary rather than only at a value far beyond it. A test claiming a length thousands of times the limit demonstrates that some bound exists while leaving its position entirely unconstrained, so the limit could drift by any smaller amount undetected.

This requirement bounds a **single field**, and deliberately claims nothing about the size of a whole decoded op. Several fields each under the limit can sum past what one message carries. Establishing that an op fits in a message needs the transport frame, which a decoder handed a byte slice cannot observe — it cannot tell whether those bytes arrived in one message, came from local storage, or were assembled by a caller. That check belongs at the transport boundary.

#### Scenario: The field-length limit is refused at its boundary

- **WHEN** a field's length prefix claims exactly one byte more than the limit allows
- **THEN** decoding fails reporting an over-long field

#### Scenario: A field at exactly the limit is not refused by the cap

- **WHEN** a field's length prefix claims exactly the limit
- **THEN** the cap does not reject it
- **AND** any failure that follows is about the input available, not the limit

#### Scenario: An over-long field length is refused by the cap, not by running out

- **WHEN** a body's length prefix claims more than the limit allows
- **THEN** decoding fails reporting an over-long field
- **AND** the failure is not merely "the input ended"

#### Scenario: An over-long list count is refused by the cap

- **WHEN** an attachment list's count claims more than the limit allows, with no elements behind it
- **THEN** decoding fails reporting an over-long field
- **AND** no capacity is reserved on the strength of the count

### Requirement: The signature trails the bytes it covers

The wire form of a signed op SHALL be the op's canonical bytes followed by the fixed-width signature, so that the signed preimage is a prefix of the message.

A decoder therefore never has to skip over the signature to find the bytes it covers, and there is no carve-out to remember about which region is signed.

The signature SHALL NOT be a field of the op itself. An op containing its own signature would have to define whether the signature covers itself; keeping them separate makes the preimage exactly "the op".

Decoding the wire form SHALL NOT verify it. A caller must be able to decode an op in order to decide what to do with a bad signature, so decoding and verifying are separate acts.

#### Scenario: The signed preimage is a prefix of the wire form

- **WHEN** a signed op is written to its wire form
- **THEN** the wire form begins with exactly the op's canonical bytes
- **AND** its length is that plus the fixed signature width

#### Scenario: A signed op survives a wire round trip

- **WHEN** a signed op of any kind is written to its wire form and read back
- **THEN** the result equals the original
- **AND** it still verifies

#### Scenario: A wire form too short to hold a signature is refused

- **WHEN** input shorter than the signature width is read as a signed op
- **THEN** decoding fails rather than underflowing

#### Scenario: A wire form with trailing bytes is refused

- **WHEN** a valid wire form is followed by an extra byte
- **THEN** decoding fails

### Requirement: An op carries no ordering field and no per-peer state

An op SHALL NOT carry a Lamport timestamp, a transport message id, a wall-clock timestamp, a sequence number, a session counter, or any other value that varies with an individual peer's history or that its author chooses freely.

Ordering is the transport's to assign. A self-asserted Lamport value would be forgeable by exactly the author it is meant to order, which defeats the purpose of ordering; a wall clock is a field the adversary sets. Both the Lamport timestamp and the message id are recorded alongside an op rather than inside it.

An op SHALL NOT carry the transport's sender identifier. That identifier binds at channel creation as a transport self-filter and is not an author identity; the author identity in an op is the key it carries and the address derived from it.

This omission SHALL be enforced by the encoding rather than merely documented: the encoding's length is fully accounted for by the fields that are present, so a field cannot be added without the encoding's shape visibly changing.

#### Scenario: The encoding's length accounts for every field present

- **WHEN** an op is encoded
- **THEN** its length equals exactly the sum of the version, the kind, the Stoa address, the author key, and that kind's own fields
- **AND** a timestamp or sequence number could not be added without this changing

#### Scenario: No ordering field participates in the id

- **WHEN** the same op is encoded by two peers with different local histories
- **THEN** both produce identical bytes
- **AND** both compute the same id

### Requirement: A moderation op names a target and an action

A moderation op SHALL carry a target and an action, rather than the action being a choice of op kind. The actions are hide and its inverse, unhide.

The action is a field because a moderation certificate signed by several moderators signs one `(target, action, ...)` tuple — a tuple needs the action inside it, not spread across which envelope arrived.

The inverse SHALL exist. Ordering moderation ops on the same target by last-write-wins is not an ordering over a set of one, and a moderation system with no correction path makes every mistake permanent.

Hiding SHALL be understood as changing what conforming peers render. It cannot unpublish anything, and this capability does not claim it can.

#### Scenario: Hide and unhide are distinguishable in the encoding

- **WHEN** a hide op and an unhide op share a target, Stoa and author
- **THEN** their encodings differ
- **AND** their ids differ

#### Scenario: A moderation op names its target by op id

- **WHEN** a moderation op is encoded and decoded
- **THEN** the target is recovered as the op id it named

### Requirement: A vote records its direction

A vote op SHALL carry a target and a direction, and both directions SHALL be representable.

Votes are collected from the first version and read by nothing, so that the history accumulates before scoring exists and scoring later arrives without a version bump. Recording the direction keeps the decision about what to *count* in the scorer, where it can change, rather than in the wire format, where changing it costs a version.

#### Scenario: Both vote directions are distinguishable in the encoding

- **WHEN** an up vote and a down vote share a target, Stoa and author
- **THEN** their encodings differ

### Requirement: The encoding declares its version

The encoding SHALL begin with a version discriminant, and decoding SHALL reject a version it does not recognise.

The version is the op format's own and SHALL NOT be shared with any other format's version. Two formats that version together mean a change to one invalidates the other for no reason.

#### Scenario: The version is the first byte

- **WHEN** an op of any kind is encoded
- **THEN** the version discriminant is the first byte
- **AND** the kind discriminant is the second

#### Scenario: An unknown version is distinguishable from corruption

- **WHEN** an op declares an unrecognised version
- **THEN** the failure names the version rather than reporting a malformed op

### Requirement: An unresolvable attachment does not invalidate an op

An op MAY carry attachment references. This capability SHALL validate only that the references decode; whether a reference resolves to any stored bytes is outside it.

An unresolvable reference is a fetch outcome, not a validation failure. A post whose attachment cannot be retrieved is still a valid, verifiable post, and a reader renders the attachment as missing rather than discarding the post or the thread.

#### Scenario: An op with attachment references round-trips

- **WHEN** a post carrying attachment references is encoded and decoded
- **THEN** the references are recovered in order
- **AND** nothing about their resolvability is asserted

#### Scenario: An op with no attachments is equally valid

- **WHEN** a post carries an empty attachment list
- **THEN** it encodes, decodes and verifies like any other
