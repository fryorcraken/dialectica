# generated-names Specification

## Purpose
Defines the derivation of a three-word display name from an identity's public key: what the derivation takes, what it guarantees, which three mechanical screens the wordlists are held to, and what a name may never be used for. A name is never published and never sent over the wire — it is recomputed by whoever holds the key, at the point of rendering — so this capability's whole subject is making one key yield one name on every peer, forever.

## Requirements

### Requirement: A display name is derived from a public key and from nothing else

A display name SHALL be a function of an identity's public key alone. The same
public key SHALL yield the same name on every peer, at every time, with no
lookup, no stored state and no published value participating.

Determinism is the whole point rather than a convenience. A name that two peers
could compute differently is one identity rendering as two people, which users
report as impersonation — so **a name is not published and cannot be**, because
a published name is a value peers could disagree about. A name is recomputed
wherever it is shown.

The derivation SHALL take the **public key**, not the address. An address is a
hash of a record kept extensible against a future key log; deriving from the key
means a name tracks the key that signs, which is what a reader is being shown.
This also keeps the question "does a name change when a key does?" a question
that arrives loudly, rather than one pre-answered by which value was hashed.

Nothing varying between runs SHALL participate: no clock, no counter, no nonce,
no device identifier, no Stoa address, and no value carried by an op.

#### Scenario: One key always gives one name

- **WHEN** a name is derived twice from the same public key
- **THEN** both derivations yield the same name

#### Scenario: A name is derivable from a public key with no address supplied

- **WHEN** a name is derived and the only input given is a public key
- **THEN** a name is returned, so no address is needed to compute one

#### Scenario: A name cannot be derived from an address

- **WHEN** the derivation's input is an author address rather than the public key
  that address was derived from
- **THEN** the name it would produce, if the address bytes were accepted as
  input, differs from the name for that key
- **AND** so a caller holding only an address cannot arrive at the right name

#### Scenario: Derivation survives a restart

- **WHEN** a name is derived for a public key, the process is restarted, and a
  name is derived for that same key again
- **THEN** the two names are equal

#### Scenario: Different keys generally give different names

- **WHEN** names are derived for many distinct public keys
- **THEN** the names are not all equal, so the derivation depends on its input

### Requirement: The name SHALL NOT travel

No reply SHALL carry a display name: not a feed row, not a thread item, not an
onboarding slate candidate, not any other reply in which core reports an author.
A name is derived by whoever holds the key, at the point of rendering.

**A derived value beside the material it derives from is two values that must
agree and could disagree**, where the recipient has no way to tell which is
wrong. A name on the wire is also a name a relay could strip or forge, which
determinism exists to make impossible. So the name does not travel, in either
direction, under any reply shape.

What a reply owes is therefore the derivation's **input**, and that obligation
belongs to each reply's own capability rather than to this one. This capability
says only that a name is never the thing carried.

**How a caller reaches the derivation is not settled here**, and no requirement
of this capability obliges core to expose one. The gap is real rather than
overlooked: the QML sandbox denies the view the network and the filesystem
outside its plugin directory, so it holds none of the wordlists and cannot derive
a name for itself — which leaves the derivation reachable by no caller until an
entry point exists. Tracked as issue #81, and out of scope here.

#### Scenario: No reply carries a display name

- **WHEN** a reply in which core reports an author is read and every field of the
  item reporting that author is enumerated
- **THEN** no field carries a display name

#### Scenario: A name does not travel as content

- **WHEN** a post arrives carrying a field that spells a display name
- **THEN** that field is not treated as the author's name anywhere
- **AND** the name for that author remains the one derived from the signing key

### Requirement: The derivation is domain-separated from every other use of the key

The hashed preimage SHALL begin with a fixed-width domain separator reserved for
name derivation, distinct from the separator every address derivation and every
signing digest uses.

Without separation the name's digest and the address's digest would be the same
function of the same key, and the two would move together. Separation is what
makes them independent functions: an attacker grinding keys for a target's name
gets an unrelated address and mark each time, and grinding for the mark gets an
unrelated name. The two must be landed together, so the costs multiply rather
than add.

**The name's independence from the mark and the abbreviated address holds by
domain separation and not by byte allocation**, and stating it the other way is a
mistake this project has made twice in two documents. The name is derived from
`H(NAME_PREFIX || public_key)`; the mark and the abbreviation both read the
**address**, a different digest of the same key. Byte 3 of one digest and byte 3
of the other are unrelated values, so there is no shared range in which the name
could overlap either. Any claim that a range of **address** bytes is "reserved
for the name" is describing a mechanism that does not exist.

The separator SHALL carry a scheme version, so that a future change to the
derivation or to the wordlists mints different names from identical keys rather
than silently colliding with this scheme's.

#### Scenario: The name digest is not the address digest

- **WHEN** a name's digest and an author address are computed from one public key
- **THEN** the two digests differ

#### Scenario: The name digest is not an undomain-separated hash of the key

- **WHEN** a name's digest for a key is compared with the plain hash of that key
- **THEN** they differ

#### Scenario: The name's digest is not the digest the other two channels read

- **WHEN** the name's digest for a key is compared, byte for byte, with that key's
  address
- **THEN** the two differ, so no byte range is shared between the name's input and
  the input the mark and the abbreviation read
- **AND** so no allocation of address bytes to the name is required or possible

#### Scenario: The separator is pinned against silent change

- **WHEN** a name is derived from fixed key material
- **THEN** it equals a value derived independently of this implementation

### Requirement: The mark and the abbreviated address read disjoint address bytes

A reader is offered three derived channels for telling identities apart: the
**name**, the **mark** derived from the address, and the **abbreviated address**
shown on screen. The name's independence is settled by the requirement above.
The mark and the abbreviation are the pair whose disjointness must be arranged,
because both read the same 32-byte address and a byte one reads is a byte the
other may also read. They SHALL read disjoint byte ranges:

- The **abbreviation** shows a head group, a middle group and a tail group of the
  address's display form. The middle group SHALL be retained: head-and-tail alone
  is the shape a vanity generator is built to defeat, and the middle group is what
  makes a convincing near-match expensive.
- The **mark** SHALL read only bytes the abbreviation does not show.

The security argument is the one that justifies domain separation, applied to a
single digest: an attacker grinding for a lookalike mark and an attacker grinding
for a lookalike abbreviation are independent searches **only when the channels
share no input**, and independent searches multiply in cost rather than adding. A
byte the abbreviation displays is worse than merely shared — it is a byte the
attacker can target while reading their progress off the screen.

Where the mark reads bytes the abbreviation already shows, those bytes tell the
reader nothing the address has not already told them, so the mark's effective
contribution is only the bytes it reads that the abbreviation hides.

#### Scenario: The mark and the abbreviation share no byte

- **WHEN** the set of address bytes the mark reads is compared with the set the
  abbreviation displays
- **THEN** the two sets are disjoint

#### Scenario: The abbreviation keeps a middle group

- **WHEN** an address is abbreviated
- **THEN** the result shows a head group, a middle group and a tail group
- **AND** the middle group is drawn from the interior rather than from either end

#### Scenario: Changing a byte the mark reads does not change the abbreviation

- **WHEN** two addresses differ only in a byte the mark reads
- **THEN** their abbreviated forms are identical
- **AND** their marks differ, so the two channels carry separate information

#### Scenario: Changing a displayed byte does not change the mark

- **WHEN** two addresses differ only in a byte the abbreviation displays
- **THEN** their marks are identical

### Requirement: A name is three drawn words in the form adjective noun of place

A name SHALL consist of exactly three drawn words in a fixed order — an adjective
from the adjective list, a noun from the noun list, and a place from the place
list — joined by the fixed connector `of` between the noun and the place, as in
*pensive aporia of lampsakos*.

**There are three slots, not four.** The connector is fixed literal text emitted
unconditionally between the noun and the place; it reads no hash bytes and carries
no entropy. This is stated because the natural reading of "three words plus a
connector" is that there are four slots.

The list sizes SHALL be exactly 8,192 adjectives, 1,024 nouns and 1,024 places,
giving `8192 x 1024 x 1024` = 2^33 = 8,589,934,592. The birthday probability for
5,000 identities in one Stoa is about 0.145%, and for 10,000 about 0.580%.

The sizes are powers of two and that is load-bearing rather than incidental:
reducing a fixed-width draw into a power-of-two list is exactly uniform, so no
word is more likely than another. Each list SHALL be **cut down to the nearest
power of two below what its source yields, never padded up to reach one**. The
direction matters — a list with headroom discards real candidates, which costs
only choice, where a padded list ships entries invented to fill it.

**The adjective slot admits any English adjective**, which is what puts 8,192
within reach of the sources and lets three slots carry the whole space. Nothing
constrains which adjectives are eligible.

#### Scenario: A derived name has three drawn words and the connector

- **WHEN** a name is derived for any public key
- **THEN** it consists of three drawn words
- **AND** the fixed connector appears between the second and third of them

#### Scenario: The slots draw from the lists they are specified to draw from

- **WHEN** names are derived for many distinct public keys
- **THEN** every first word is a member of the adjective list
- **AND** every second word is a member of the noun list
- **AND** every third word is a member of the place list

#### Scenario: The lists are the sizes the arithmetic rests on

- **WHEN** each list is counted
- **THEN** the adjective list holds exactly 8,192 entries
- **AND** the noun list holds exactly 1,024 entries
- **AND** the place list holds exactly 1,024 entries

#### Scenario: The connector is not a slot

- **WHEN** names are derived for many distinct public keys
- **THEN** the connector is the same literal text in every one of them
- **AND** it never varies with the key, so it carries no entropy

### Requirement: The three drawn words are never elided; the connector may be

A name SHALL be reproducible in full from the derivation. Core SHALL NOT
abbreviate, truncate or elide any of the three drawn words when returning a name,
and SHALL NOT return a name with a drawn word shortened mid-word.

The three drawn words are the whole of the space. Eliding one removes a slot's
worth of distinguishing content, and eliding the place removes the half of the
name that a reader uses to tell two otherwise similar names apart.

**The connector is the one exception, and it is a relaxation rather than a
loosening of the rule.** It reads no hash bytes and carries no entropy, so a
caller too cramped to render it may omit it: the information content of
*pensive aporia lampsakos* is identical and no reader is misled about who
published something. This is the only part of a name that may be dropped, and it
is droppable precisely because it is the only part that is not derived.

#### Scenario: A returned name carries all three drawn words in full

- **WHEN** a name is returned by the derivation
- **THEN** each of the three drawn words appears complete
- **AND** none is shortened, elided or replaced by an ellipsis

#### Scenario: A name without the connector names the same identity

- **WHEN** a name is compared with the same name with the connector removed
- **THEN** the three drawn words are identical
- **AND** so the two identify the same key

#### Scenario: Every entry of each list is reachable

- **WHEN** the derivation's index extraction is applied across the full range of
  its input bytes
- **THEN** every index of the adjective list is produced
- **AND** every index of the noun list is produced
- **AND** every index of the place list is produced

#### Scenario: Reduction into a list is uniform

- **WHEN** the derivation's index extraction is applied across the full range of
  its input bytes
- **THEN** every index of a list is produced the same number of times as every
  other, so no word is favoured over another

### Requirement: Each word slot reads distinct bytes, within a bounded slice

Each of the three slots SHALL take its index from bytes of the name digest that
no other slot reads, so that the three words are independent draws rather than
three views of the same bits.

Each slot SHALL take a 16-bit draw reduced into its list. This is what makes the
reduction exactly uniform: 8,192 divides 65,536 eight times and 1,024 divides it
sixty-four times, so no entry of either list is favoured.

The derivation SHALL read a **bounded** prefix of the digest. Reading on without
bound is what lets two implementations disagree about how far to read and so
produce different names for one key, which is the failure this whole scheme
exists to prevent; a bounded slice is also what makes the derivation checkable
against a fixed expected value at all.

**The bound SHALL be 6, and the three draws SHALL consume bytes `0..6`
exactly.** Two bytes feed the adjective, two the noun, two the place, and the
derivation reads no seventh byte under any input.

**The bound is exactly what the derivation consumes, and it names no byte the
derivation does not read.** A bound stated wider than the draws would record a
boundary nothing enforces — a range read by nothing, which the next reader takes
as load-bearing and designs around. A scheme that later wants more bytes is a
new scheme version by the freezing requirement below, which is where that
decision belongs rather than in a range set aside now against a use nobody has.

**The name's window and the mark's are not comparable ranges, and this must not
be read as though they were.** The name reads a prefix of the **name digest**;
the mark reads a window of the **address**, which is a different digest under a
different separator. The two are independent because of that separation and not
because their byte ranges avoid each other — so the costs of grinding for a
lookalike name and for a lookalike mark multiply rather than add, and that holds
whichever bytes either reads. A byte reservation *between* them would do no
work, which is a claim this scheme has already had to retract once.

Beyond the bound the derivation SHALL fail loudly rather than read further. With
the draws fixed at `0..6` of a 32-byte digest this path is not reachable through
any key or any digest, and it is stated as a requirement on the derivation's
shape rather than as a behaviour a test can provoke: what it forbids is an
implementation that reads on, and a second implementation that read further
would produce a different name for the same key.

**Every draw is a single unconditional reduction.** The number of bytes a name
consumes is therefore fixed rather than data-dependent, every index of every
list is reachable, and the `2^33` space is reached exactly rather than
approximately — so the collision figures in the requirement above hold without a
caveat, and no draw is ever skipped in a way that would make one word rarer than
another.

**The step that turns a digest into three words SHALL be exercisable over a
supplied digest**, separately from hashing a key. This is a testability
obligation rather than a surface to expose to callers: reaching a chosen slot
combination through a chosen *key* means grinding for one, so the pinning and
uniformity requirements below are checkable only if a digest can be supplied
directly.

#### Scenario: Each slot varies independently of the others

- **WHEN** for each slot in turn, the bytes that slot draws from are varied across
  their full range while every other byte is held fixed
- **THEN** that slot's word takes every value its list holds
- **AND** the other two slots' words do not change, so no byte feeds two slots

#### Scenario: Changing a byte one slot reads changes only that slot

- **WHEN** two digests differ only in a byte that one slot reads
- **THEN** the names derived from them differ in that slot's word
- **AND** agree in the other two slots

#### Scenario: The derivation reads no byte past its bound

- **WHEN** two digests agree on bytes `0..6` and differ on **every** byte from 6
  onward
- **THEN** the names derived from them are equal

#### Scenario: The bytes a name consumes do not vary with the digest

- **WHEN** names are derived over many digests
- **THEN** each reads the same six bytes as every other, so no input makes the
  derivation consume more

### Requirement: Nothing filters a drawn name

The derivation SHALL return the three words its draws select, whatever those
three words are. It SHALL hold no list of refused combinations, SHALL never
retry a draw, and SHALL never substitute, suppress or reorder an output for what
the words are, what they mean, what they connote, whom they name, or what they
spell together.

**Owner decision, recorded rather than argued.** The screens requirement below
forbids keeping a word out of a list. This forbids the other shape the same rule
takes — leaving every word in and refusing what they combine into. The two are
one decision and are stated as two requirements because they are two places an
implementation could put a filter.

**Exactly one rule refuses anything anywhere in this contract**, and it is the
requirement below that no wordlist entry may contain the connector as a word.
That is a rule about the shape of an entry rather than about any word's meaning,
and it exists because an entry carrying ` of ` renders as two places on one
name. Nothing else in this capability keeps anything out.

**What this buys is a scheme with no conditional path at all.** Every draw is
one unconditional reduction, so the derivation is total over well-formed keys,
consumes a fixed six bytes, reaches every index of every list, and hits the
`2^33` space exactly. A second implementation agrees with this one by reducing
three 16-bit values — there is no table of pairs it must also hold, and no
shared table that could drift between peers and rename somebody.

#### Scenario: Every combination the draws select is returned

- **WHEN** the slot-selection step is exercised over digests selecting many
  different noun–place combinations
- **THEN** each returns the words its draws selected
- **AND** none is substituted, suppressed or replaced by a second draw

#### Scenario: A name is a function of six bytes and nothing else

- **WHEN** two digests agree on bytes `0..6`
- **THEN** the names derived from them are equal, whatever the words drawn are,
  so no property of the drawn words feeds back into the derivation

### Requirement: The derivation is pinned to fixed expected names

The derivation SHALL be checkable against names stated independently of the
implementation, for fixed public keys.

This is consensus-critical in the silent direction. Change one byte of the
separator, one entry of a list, the order of two entries, or which bytes a slot
reads, and this peer's names stop matching every other peer's — with no error
anywhere, because each peer remains internally consistent. A check that asks the
implementation what it produced and agrees with the answer cannot see this;
the expected name must be written down.

**The written-down name is what pins the word count**, and this is the reason the
pin is stated as a requirement of its own rather than left to the shape
requirement above. Counting the words of whatever the implementation returned
agrees with an implementation that joins two words, or four, or three in the
wrong order — the count and the thing being counted come from one source. A
written-down name is produced independently, so a change to the number of words,
to the order of the slots, or to the connector between them fails it.

**Pinned cases SHALL span the lists rather than clustering**, reaching a low and
a high index in each of the three slots, so that a pin is evidence about the
index arithmetic and not only about one region of one list. There is one
derivation path and no second one to pin separately: with no refusal and no
retry, every key exercises the same six bytes and the same three reductions.

#### Scenario: A fixed key yields a fixed name

- **WHEN** a name is derived for a fixed public key
- **THEN** it equals a name written down independently of this implementation

#### Scenario: A pinned name is reproducible from its digest by hand

- **WHEN** a pinned case's digest is taken, its three 16-bit draws are read from
  bytes `0..6`, and each is reduced into its list by hand
- **THEN** the three words reached are the pinned name's three words, so the pin
  checks the index arithmetic rather than agreeing with the implementation that
  produced it

#### Scenario: A wordlist reordering is visible

- **WHEN** two entries of a list are exchanged and the pinned cases are derived
  again
- **THEN** at least one pinned case no longer matches its written-down name

#### Scenario: The pinned name is three words and a connector, in order

- **WHEN** a pinned case's written-down name is read
- **THEN** it is an adjective-list entry, a noun-list entry, the connector, and a
  place-list entry, in that order
- **AND** it was written down independently of the implementation, so a build
  joining a different number of words, a different order, or a different
  connector fails to match it

### Requirement: Three screens apply to every list, and no others

Every entry of every list SHALL be ASCII and lowercase; each list SHALL hold one
entry per word, per person and per place, with one spelling per entry; and every
entry SHALL be a real word, person or place rather than an invented one.

Those three — **ASCII-transliterable, deduplicated and attested** — are the only
screens that apply to a list, and all three are mechanical: **none asks what a
word means, what register it carries, or what it says about the person carrying
it.** A word SHALL NOT be kept out of a list for any other reason, and **a word
being kept out for any other reason is the mistake rather than the word.**

The long tail is deliberately in, and its cost is accepted rather than argued
away: some names will be legible and still hard to tell apart, and the mark and
the address are what carry a reader through that rather than a shorter list.

**An entry MAY contain an internal space**, because a Greek place is often named
in two words — `lokroi epizephyrioi`, `antiocheia maiandros`. A rule admitting
only single-word entries would be one this requirement forbids, and it is an
expensive one: it puts the place list's honest yield well below 1,024 by
discarding multi-word toponyms, so a census taken under it measures a pool the
contract does not describe.

ASCII is a bidi decision rather than a typographic preference. These are the one
piece of rendered text this project fully composes from a fixed list, so keeping
them ASCII means a generated name can never itself carry a bidi override or a
homoglyph — it removes the attack from this surface rather than mitigating it.
The obligation to handle bidi in everything a name is rendered *next to* is
untouched by this and belongs elsewhere.

**Attestation carries the whole of the quality bar, and it is the one screen a
test cannot check.** A fabricated Greek word reads exactly like a real one: no
reviewer catches it by reading the list, and no assertion over the list's own
contents can distinguish the two, because the only evidence that would settle it
is outside the program. The lists SHALL therefore be **built from sources rather
than from recall**, and that obligation is stated here as a requirement on how
the lists are produced, checkable by a person against a source and not by a test.
Stating it without a scenario is deliberate: a scenario here would assert
something the suite cannot fail on, which is worse than an unchecked requirement
because it reads as covered.

The sources SHALL be:

- the **adjective** list: **any English adjective**.
- the **noun** list: **any attested ancient Greek noun**, drawn from four pools on
  equal footing — abstractions, named historical Greeks, mythological figures, and
  ordinary concrete nouns. The last covers objects, animals, plants, materials,
  crafts, ships, music, measures, body parts, kinship, time and weather.
- the **place** list: **any ancient Greek or mythological place**.

Real and imagined SHALL NOT be distinguished in either Greek list or to a reader;
a place reads as an origin and a figure reads as a name, which is all either slot
does.

**The noun slot is not a technical vocabulary.** Reading it as the vocabulary of
Greek thought alone is what an earlier draft did, and it is a far narrower pool
than any attested noun — the mythological and concrete pools were absent from it
entirely and are the larger half of what the slot now draws on.

**No slot is screened for register**, the adjective slot included, so `luminous`,
`pensive` and `restless` draw alongside any other English adjective.

#### Scenario: Every entry is ASCII and lowercase

- **WHEN** every entry of every list is examined
- **THEN** each contains only ASCII characters
- **AND** each is lowercase
- **AND** each is non-empty, with no leading or trailing space and no double space

#### Scenario: A multi-word place entry is accepted

- **WHEN** the place list holds an entry naming a two-word toponym
- **THEN** it is accepted, because no screen excludes an internal space
- **AND** it draws and renders as one place, the connector `of` still preceding it

#### Scenario: No list holds a duplicate

- **WHEN** each list is compared against itself
- **THEN** no entry appears twice in a list

### Requirement: No noun entry contains the connector

No entry of the noun list SHALL contain the connector as a separate word — that
is, no noun entry SHALL contain the substring formed by a space, the connector,
and a space.

An entry may still hold an internal space; this constrains what that space may
sit beside rather than forbidding one, so a two-word entry such as
`lokroi epizephyrioi` is unaffected.

**The constraint is on the noun list only, and that is the whole of what is
decided here.** The place slot is the last word of the name, so a place carrying
the connector produces no second *of Y* after it and no ambiguity about which
place the slot supplied. Whether a place entry may carry the connector is
therefore left open rather than ruled on, and a list that happens to contain none
satisfies this requirement as written.

The reason is the *X of Y* shape rather than anything about the words. A noun
entry carrying the connector renders as *pensive zenon of kition of lampsakos*,
which reads as two places attached to one name and leaves a reader unable to tell
which of them the place slot supplied. Widening the noun slot to named historical
Greeks, mythological figures and concrete nouns is what makes this reachable:
those are exactly the entries a source is liable to supply already qualified by a
place. The constraint is on the entry's spelling and is therefore checkable
against the shipped list, unlike the attestation obligation above.

**This is the only rule in this capability that keeps anything out, and it is a
rule about one literal substring.** What a noun means is no part of whether it
is in: the bare form of a qualified entry — `zenon` where a source offered
`zenon of kition` — is in the list and draws normally.

#### Scenario: No noun entry carries the connector as a word

- **WHEN** every entry of the noun list is examined
- **THEN** none contains the connector surrounded by spaces

#### Scenario: The noun a name renders carries no connector

- **WHEN** names are derived for many distinct public keys
- **THEN** no name's second word group contains the connector, so no name reads as
  carrying two places

### Requirement: A name is never unique, never an identifier, and never numbered

A display name SHALL NOT be treated as unique, SHALL NOT be accepted anywhere an
identity is named, and SHALL NOT be disambiguated by appending a number or any
other distinguishing suffix.

Uniqueness is unavailable rather than merely unbuilt. There is no registry and no
authority to hold a namespace: a Stoa has no membership list, peers join and
leave without announcing it, and each peer knows only what has reached it. Two
peers can each believe a name is free.

Numbering is worse than leaving a collision alone. Appending a suffix requires
agreeing which of two identities was second, which is arrival order — a per-peer
fact — so two peers would number the same pair oppositely and each would be sure
the other was the impostor.

No operation SHALL accept a display name in place of an address or a key: not a
lookup, not a moderation target, not a vote target, not a request naming an
author. The address is the identity.

Collisions are rare rather than expected at the specified space, and that changes
nothing here. The rule is not a response to the rate.

#### Scenario: Two keys deriving one name each derive it unchanged

- **WHEN** two distinct public keys that derive the same name are each put through
  the derivation
- **THEN** each yields that name exactly as derived
- **AND** neither carries a number, a suffix or any other added distinguishing
  mark

A colliding pair is not found by searching the key space, which is infeasible at
the specified size. It is constructed: the name is a function of the key, so a
pair is obtained by holding the derivation's inputs fixed at a chosen name and
taking two distinct keys that reach it. What is under test is that the derivation
adds nothing when two keys meet, not the likelihood of their meeting.

#### Scenario: No method accepts a name where an identity is required

- **WHEN** a request supplies a display name in a field that names an author, a
  moderation target or a vote target
- **THEN** the request is refused rather than resolved to any identity

### Requirement: A name is not a credential, and deriving one establishes nothing

Deriving a name SHALL establish only that this is the name for this key. It SHALL
NOT be read as meaning the key is known, trusted, present in this Stoa,
moderating it, or authorised to do anything.

A name is freely reachable: anyone willing to press a regeneration button reaches
any name they like, so any argument of the form "an attacker would have to grind
for that" is unavailable. What an attacker gets is a lookalike name on a
different address; they cannot forge the address and cannot forge a signature, so
nothing they publish is attributable to the identity they imitate. The whole
attack is social, and the address is what defeats it.

Deriving a name SHALL consult no moderator set, no genesis record and no stored
state, being a function of the key alone. This follows from determinism and is
stated because a name is rendered next to moderator badges, where a reader may
read the pair as one claim.

A name SHALL be derivable for any well-formed public key, including one belonging
to no identity this peer has seen. There is nothing to look up, so there is
nothing that could fail **to be found**.

**The derivation SHALL be total over well-formed keys.** Every well-formed key
yields a name, and **malformed key material is the only failure this capability
has** — the requirement below specifies it. There SHALL be no other error
condition, and in particular no failure that a well-formed key belonging to a
real identity could reach: an error variant no input can produce is an
unreachable branch that a reader takes as evidence the failure exists, and a
caller handles a case that cannot arrive.

What this requirement forbids is a derivation that refuses a key for not being
*recognised*, which is a different thing from refusing bytes that are not a key.

#### Scenario: A name is unchanged by every surrounding state

- **WHEN** a name is derived for one key with no Stoa joined and no identity
  stored, and again with a genesis record loaded, a moderator set in force and an
  identity stored
- **THEN** the two names are equal, so no such state participates

#### Scenario: An unknown key still derives a name

- **WHEN** a name is derived for a well-formed public key this peer has never
  seen
- **THEN** a name is returned rather than a failure

#### Scenario: A name says nothing about authority

- **WHEN** names are derived for a moderator's key and for a non-moderator's key
- **THEN** neither name carries any marking distinguishing the two

### Requirement: Malformed key material is refused rather than crashed on

Deriving a name from input that is not a valid public key SHALL return a failure
rather than a name, and SHALL NOT abort the process.

A public key on this path arrives inside an inbound op and is
attacker-controlled. A panic here aborts the module process, the caller learns
only that its call timed out, and every later call reports the module as not
loaded — so a derivation that panics on a short slice is a remotely triggerable
denial of service rather than a robustness nicety.

A failure SHALL NOT be reported as a name. There SHALL be no placeholder name,
no name for "unknown", and no name derived from truncated or padded input: each
would render as an ordinary participant, which is a name attributable to nobody
presented as one attributable to somebody.

#### Scenario: A wrong-length key is refused

- **WHEN** a byte string shorter or longer than a public key is given to the
  derivation
- **THEN** it returns a failure rather than panicking

#### Scenario: Arbitrary bytes do not abort the process

- **WHEN** arbitrary byte strings are given to the derivation
- **THEN** each returns a reply rather than terminating the process

#### Scenario: A failure is not a name

- **WHEN** the derivation fails for any input
- **THEN** no name is returned
- **AND** no placeholder or fallback name is returned in its place

### Requirement: The scheme and its wordlists are versioned together and frozen

Any change to the scheme SHALL mint a new version rather than edit the current
one. A change SHALL include: removing a word from any of the three lists, adding
one, reordering a list, changing a list's size, changing the number of slots,
changing which bytes a slot reads, and changing the connector.

**Changing a list's size is a scheme change even when it looks like a
correction.** The sizes are what make the reduction unbiased, so taking the
adjective list to any value that is not a power of two introduces a bias, and
taking it to a different power of two reindexes every draw.

The mechanism is worth stating rather than asserting as a rule. The derivation
maps digest bytes to list indices, so removing one word reindexes the list and
every identity that drew at or after the removed index renders differently —
on peers that have updated and not on peers that have not. The same key then
renders as two different people depending on who is looking, which is exactly the
disagreement this scheme exists to prevent and which users report as
impersonation. A version bump makes the old and new schemes two distinct
derivations rather than two peers' answers to one question.

The first version SHALL therefore be conservative. Shipping a word that has to
come out later is not a patch: it is a migration in which everybody's name
changes at once.

Names under two scheme versions SHALL be distinguishable, so that one version's
names cannot be silently reproduced by the other.

#### Scenario: A different scheme version gives a different name for one key

- **WHEN** a name is derived for one public key under two scheme versions
- **THEN** the two names differ

#### Scenario: Removing a word renames identities that drew past it

- **WHEN** an entry is removed from a list without a version bump, and names are
  derived again for keys that drew at or after the removed index
- **THEN** those names differ from the names the same keys had before
- **AND** this is the outcome the version bump exists to prevent

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

**A request whose key material cannot be read at all is refused before the
identity layer is reached**, and this is a third class rather than a variety of
either above. Key material of a type that is not a string, key material that is
not valid hex, and key material longer than a public key's encoding admits are
each refused by the entry point itself — the identity layer is given nothing to
judge, so it renders no verdict and "exactly what the identity layer refuses"
does not reach them. Each SHALL be distinguishable from *absent* key material,
for the same reason absent and bad must be told apart: a caller that reads
"nothing supplied" when it supplied something malformed looks for the wrong bug.
The entry point MAY refuse over-long material without examining it, which is what
lets the length be bounded before anything is allocated from it.

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

- **WHEN** key material that reaches the identity layer is supplied to the entry
  point, and the same material is offered to the identity layer's public key
  parse
- **THEN** the entry point returns a name in exactly the cases the identity layer
  accepts the material, and refuses in exactly the cases it refuses

#### Scenario: A request the entry point cannot read as key material is refused before the identity layer is reached

- **WHEN** a caller supplies no key material, key material of a type that is not
  a string, key material that is not valid hex, or key material longer than a
  public key's encoding admits
- **THEN** the call is refused and no name is returned
- **AND** the refusal is the entry point's own, since the identity layer is given
  no material to render a verdict on

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
