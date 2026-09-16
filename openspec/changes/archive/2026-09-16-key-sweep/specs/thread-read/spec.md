<!--
`Only authentic posts in the named Stoa are returned` is deliberately NOT in this
delta. Its prose already frames authorship over keys — its forged-reply scenario
reads "whose author field names one key and whose signature was made with
another" — so it survives the sweep unchanged. Named here so that its absence
reads as a checked result rather than an omission.
-->

## MODIFIED Requirements

### Requirement: An author is reported as a public key, and never as a name

Each item SHALL report the public key that signed that op, as the whole of how
it names its author. No item SHALL report an author address.

**One field replaces two because the reason for two is gone.** The previous
contract required an address *and* a key on every item, on the ground that a
reader is shown two derived channels — a display name and a mark — reading two
different digests, neither recoverable from the other. Both channels now read the
public key's own bytes directly, under the `generated-names` requirement *The
three channels read pairwise disjoint bytes of the public key*. An address is
therefore an input to nothing a reader is shown, and carrying one would put a
second identifier beside the key it was derived from, where the two could
disagree and a recipient would have no way to tell which was wrong.

The key is what makes the item's name and mark computable at all, and it is also
what settles authorship: a name and a mark are both cheaply re-rolled to resemble
someone else's, and only the key is bound to a signature. So the field that
survives is the unforgeable one, and the derivable ones stay absent.

**No item SHALL carry a display name or a mark.** A name is a pure function of
the public key, so sending one from here would put a derived value on the wire
beside the material it derives from — and a name on the wire is one a relay could
strip or forge. Deriving names is the job of whatever renders them, and this
contract supplies the input rather than the output. The same holds for the mark.

The author field reported SHALL be that of the key that actually signed the op,
which verification has already established, and SHALL NOT be taken from an
unverified claim.

#### Scenario: Each item carries both an address and a key

- **WHEN** a thread's posts are returned
- **THEN** every item carries its author's public key
- **AND** no item carries an author address

  The scenario keeps its name while its content inverts, because the name is how
  this delta addresses the scenario it alters. The two-field contract is what
  this change ends, so what was asserted as a pair is now asserted as one field
  present and the other absent.

#### Scenario: The two fields identify the key that signed

- **WHEN** a thread containing posts by two different authors is read
- **THEN** each item's public key is the one whose signature verifies that op
- **AND** the items by different authors differ in it

#### Scenario: The two fields are distinct values and neither substitutes for the other

- **WHEN** an item's author field is compared with the public key whose signature
  verifies that op
- **THEN** they are the same value, the key being the whole of how an author is
  named
- **AND** no second author field is present for it to be distinct from

#### Scenario: No item carries a derived display name

- **WHEN** a thread is read and every field of an item is enumerated
- **THEN** the only field describing the author is the public key
- **AND** the item carries no second author-describing field, which is what a
  name, a mark or an address would have to be

## RENAMED Requirements

- FROM: `### Requirement: An author is reported as both an address and a public key, and never as a name`
- TO: `### Requirement: An author is reported as a public key, and never as a name`

The old name states the two-field contract in its first clause, so leaving it
would name the requirement after the thing this change removes.
