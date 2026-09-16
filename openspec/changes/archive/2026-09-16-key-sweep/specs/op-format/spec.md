<!--
The op ENCODING does not change. Every edit here is prose naming a value that
stops existing: the op already carries `stoa` (a Stoa address) and the author's
public key, and no author address is ever serialised. So no op already signed is
re-meant, no id changes, and nothing stored needs rewriting.
-->

## MODIFIED Requirements

### Requirement: An op is the unit that crosses the wire

An op SHALL be a signed operation, and it SHALL be the only thing that crosses the wire. A peer's view of the forum is a function of the ops it has seen, so anything not expressible as an op is not expressible at all.

An op SHALL carry the Stoa it belongs to, the public key of its author, and exactly one kind describing what it does. The kinds are: a post (which is a reply when it names a parent), a revision of one of the author's own posts, a moderator's judgement about a target, a vote on a target, and a declaration of the Stoa's current display metadata.

Creating a Stoa SHALL NOT be an op. A Stoa is a genesis record its creator publishes, and hashing that record is what creates it — see the `stoa-genesis` capability. There is nothing for a signed op to add.

Replying SHALL NOT be a separate kind. A reply is a post that names a parent, so that "is this a reply?" is one question about a field rather than a second question about which kind arrived.

The author's **public key** SHALL travel in the op, and it SHALL be the whole of how an op names its author. Verification happens on read with no directory to resolve any other identifier against, so a peer must hold the key itself in order to check the signature. **No second author identifier SHALL be carried or derivable**: the key is the author, so there is nothing beside it for a recipient to reconcile it against.

Kind discriminants SHALL be appended, never inserted among those already allocated. Inserting would re-mean every op already signed, since the discriminant is inside both the signature and the op id. A kind added later therefore costs one unused discriminant and no encoding version, and an older client meets it as an unrecognised kind rather than misparsing it.

#### Scenario: Every kind round-trips through the encoding

- **WHEN** an op of any kind is encoded and decoded
- **THEN** the decoded op equals the original in every field

#### Scenario: A reply is a post that names a parent

- **WHEN** a post op names a parent op
- **THEN** it is the same kind as a post that names none
- **AND** both round-trip through the encoding

#### Scenario: An unallocated kind discriminant is refused rather than misparsed

- **WHEN** an op declares the first discriminant above those allocated
- **THEN** decoding fails naming that discriminant
- **AND** it is not read as any allocated kind

#### Scenario: An op names its author by public key and by nothing else

- **WHEN** an encoded op's fields are enumerated
- **THEN** the author is named by a public key
- **AND** no other field identifies the author

### Requirement: An op is named by a content-derived id

An op SHALL have a 32-byte id computed from its own canonical bytes under a domain-separation prefix distinct from every address prefix. The id SHALL NOT be assigned by the transport.

A content-derived id is available wherever the op is — replaying a local store, applying a snapshot, deduplicating on ingest — none of which has a transport envelope to consult. An id assigned on delivery would be absent in exactly those settings.

This id is **not** the transport's message id. The ordering rule's tiebreak refers to that one, which arrives with the message and is the transport's to assign. Two identifiers with two jobs; a reader must not substitute one for the other.

Domain separation matters because an op id and a Stoa address are both 32 bytes, and a moderation op names one of them by value. No byte string may be a valid instance of both.

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

### Requirement: An op carries no ordering field and no per-peer state

An op SHALL NOT carry a Lamport timestamp, a transport message id, a wall-clock timestamp, a sequence number, a session counter, or any other value that varies with an individual peer's history or that its author chooses freely.

Ordering is the transport's to assign. A self-asserted Lamport value would be forgeable by exactly the author it is meant to order, which defeats the purpose of ordering; a wall clock is a field the adversary sets. Both the Lamport timestamp and the message id are recorded alongside an op rather than inside it.

An op SHALL NOT carry the transport's sender identifier. That identifier binds at channel creation as a transport self-filter and is not an author identity; the author identity in an op is the key it carries.

This omission SHALL be enforced by the encoding rather than merely documented: the encoding's length is fully accounted for by the fields that are present, so a field cannot be added without the encoding's shape visibly changing.

#### Scenario: The encoding's length accounts for every field present

- **WHEN** an op is encoded
- **THEN** its length equals exactly the sum of the version, the kind, the Stoa address, the author key, and that kind's own fields
- **AND** a timestamp or sequence number could not be added without this changing

#### Scenario: No ordering field participates in the id

- **WHEN** the same op is encoded by two peers with different local histories
- **THEN** both produce identical bytes
- **AND** both compute the same id

### Requirement: Valid text is never normalised or otherwise transformed

Decoding SHALL NOT normalise, case-fold, reorder, strip or otherwise transform text that is valid UTF-8. Rejecting text that is *not* valid UTF-8 is already required by "A malformed op is rejected at the decoding boundary"; this requirement governs what happens to text that passes that check.

Declining to transform valid text is a canonicality requirement, not an oversight. Any normalisation applied at decode would mean an accepted byte string re-encoding to something other than itself, which contradicts "An accepted encoding re-encodes to itself" and would break op-id agreement between peers — the property the whole encoding exists to provide. Two peers on builds with different Unicode tables would compute different ids for one op, silently.

**The consequence is that display text is attacker-controlled and SHALL NOT be trusted for rendering or identification.** A title may contain bidirectional controls, zero-width characters, or homoglyphs of an established Stoa's name, and while no authority check exists any peer may sign such an op. Mitigation belongs to whoever renders: strip or visibly mark bidi and zero-width controls, show the Stoa address alongside any name, and never treat a title as an identifier. The Stoa address is the identity; a name never is.

**Only the final sentence changes**, from "The address is the identity" to "The Stoa address is the identity". The requirement is about titles and its subject was always the Stoa — the preceding clause says "show the Stoa address alongside any name" — so nothing here changes meaning. The bare sentence is scoped because it is the one a later reader would cite out of context as authority for an author address, which this change deletes.

#### Scenario: Valid text is returned exactly as it arrived

- **WHEN** a text field carries valid UTF-8, including multi-byte sequences, bidirectional controls and zero-width characters
- **THEN** decoding returns those characters unchanged
- **AND** re-encoding reproduces the original bytes

#### Scenario: Canonically-equivalent text stays distinct

- **WHEN** two ops carry titles that differ only by Unicode normalisation form, such as a combining sequence against its precomposed equivalent
- **THEN** their encodings differ
- **AND** their ids differ, rather than collapsing onto one op
