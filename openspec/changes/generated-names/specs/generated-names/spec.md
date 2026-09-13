## Purpose

Defines how a readable display name is derived from an identity's public key: what the derivation takes and guarantees, what the wordlists must contain and may not, where a name must be returned, and what a name may never be used for — so that every peer renders one identity identically, and so that a name is never mistaken for the thing that settles who published something.

## ADDED Requirements

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

The separator SHALL carry a scheme version, so that a future change to the
derivation or to the wordlists mints different names from identical keys rather
than silently colliding with this scheme's.

#### Scenario: The name digest is not the address digest

- **WHEN** a name's digest and an author address are computed from one public key
- **THEN** the two digests differ

#### Scenario: The name digest is not an undomain-separated hash of the key

- **WHEN** a name's digest for a key is compared with the plain hash of that key
- **THEN** they differ

#### Scenario: The separator is pinned against silent change

- **WHEN** a name is derived from fixed key material
- **THEN** it equals a value derived independently of this implementation

### Requirement: A name is four words, two adjectives then two nouns

A name SHALL consist of exactly four words in a fixed order: two adjectives drawn
from the adjective list, then two nouns drawn from the noun list.

The word count follows from the list sizes. The adjective list holds 256 entries
and the noun list 512, because a uniform register is held to be worth more than
a larger list: a larger adjective list would be padded with near-synonyms, and a
padded region degrades every name drawn from it. At two adjectives and one noun
the space is `256 x 256 x 512` = 2^25, and the birthday probability for 5,000
identities in one Stoa is about 31%, which fails the bar. The fourth word takes
the space to `256 x 256 x 512 x 512` = 2^34, at which 5,000 identities collide
with probability about 0.073%.

**The register cap is a choice and not a ceiling the sources impose**, and that
is recorded here because it is the premise the whole four-word shape rests on. A
list drawn without the single-voice screen would be far larger, and three words
over such a list would reach a comparable space. Four words is what follows from
keeping the register; it is not what follows from the corpus. Anything revisiting
the count revisits the register rule first, and either way it is a scheme version
bump.

The second added word SHALL be a noun rather than a third adjective, so that a
reader has two content words to latch onto rather than a longer run of modifiers.

The list sizes SHALL be exactly 256 and 512. These are powers of two and that is
load-bearing rather than incidental: reducing a fixed-width draw into a
power-of-two list is exactly uniform, so no word is more likely than another. A
list of another size would introduce a bias, and any change to a size changes the
arithmetic that justified four words.

#### Scenario: A derived name has four words

- **WHEN** a name is derived for any public key
- **THEN** it consists of exactly four words

#### Scenario: The slots draw from the lists they are specified to draw from

- **WHEN** names are derived for many distinct public keys
- **THEN** every first and second word is a member of the adjective list
- **AND** every third and fourth word is a member of the noun list

#### Scenario: The lists are the sizes the arithmetic rests on

- **WHEN** the adjective list and the noun list are counted
- **THEN** the adjective list holds exactly 256 entries
- **AND** the noun list holds exactly 512 entries

#### Scenario: Every entry of each list is reachable

- **WHEN** the derivation's index extraction is applied across the full range of
  its input bytes
- **THEN** every index of the adjective list is produced
- **AND** every index of the noun list is produced

#### Scenario: Reduction into a list is uniform

- **WHEN** the derivation's index extraction is applied across the full range of
  its input bytes
- **THEN** every index of a list is produced the same number of times as every
  other, so no word is favoured over another

### Requirement: Each word slot reads distinct bytes, within a bounded slice

Each of the four slots SHALL take its index from bytes of the digest that no
other slot reads, so that the four words are independent draws rather than four
views of the same bits.

The derivation SHALL read a **bounded** prefix of the digest. Reading on without
bound is what lets two implementations disagree about how far to read and so
produce different names for one key, which is the failure this whole scheme
exists to prevent; a bounded slice is also what makes the derivation checkable
against a fixed expected value at all.

Beyond the bound the derivation SHALL fail loudly rather than read further.

The bound SHALL leave a reserve sufficient for one complete redraw of all four
slots, which the denylist requires. The reserve SHALL be stated as part of the
scheme: it is consumed only on a refusal, so the number of bytes a name consumes
is data-dependent rather than fixed, and the bound is what keeps it finite.

**The step that turns a digest into four words SHALL be exercisable over a
supplied digest**, separately from hashing a key. This is a testability
obligation rather than a surface to expose to callers: a refused first draw, an
exhausted reserve, and a byte read past the bound are all reachable through a
chosen digest and reachable through a chosen *key* only by grinding for one. A
scheme whose failure paths can only be reached by grinding is a scheme whose
failure paths no test covers.

#### Scenario: Each slot varies independently of the others

- **WHEN** for each slot in turn, the bytes that slot draws from are varied across
  their full range while every other byte is held fixed
- **THEN** that slot's word takes every value its list holds
- **AND** the other three slots' words do not change, so no byte feeds two slots

#### Scenario: Changing a byte one slot reads changes only that slot

- **WHEN** two digests differ only in a byte that one slot reads
- **THEN** the names derived from them differ in that slot's word
- **AND** agree in the other three slots

#### Scenario: The derivation reads no byte past its bound

- **WHEN** two digests agree on every byte inside the bound and differ beyond it
- **THEN** the names derived from them are equal

#### Scenario: Exhausting the reserve fails rather than reading on

- **WHEN** the slot-selection step is exercised over a digest whose first draw and
  whose one reserved redraw both land on refused combinations
- **THEN** it reports a failure
- **AND** returns no name

Reaching this through a key would mean grinding for one, so the slot-selection
step SHALL be exercisable over a supplied digest. Without that the bound and the
loud failure are unreachable in a test and so are claims nothing can check.

### Requirement: A refused combination redraws all four slots, deterministically

The scheme SHALL carry a denylist of refused word combinations, and a derivation
landing on one SHALL redraw **all four** slots from the reserved bytes rather
than redrawing only the offending slot.

Vetting single words is insufficient because the generated *combination* is what
ships: two individually innocuous words can compose into a slur, an insult aimed
at a real group, or a claim about the person carrying the name. A pooled noun
list of thinkers and abstractions is more exposed than a list of one kind,
because a proper name beside an abstract noun can compose into a reading neither
word carries alone.

Redrawing the whole name is what keeps termination arithmetic. A refused pair is
refused for the combination, so changing one half can land on a second refused
pair and the loop's termination becomes a property of the denylist's shape rather
than of the byte budget.

The redraw SHALL be a function of the digest alone, so that every peer skips
identically. A redraw introducing fresh randomness or a nonce would break the
determinism every other requirement here rests on.

The scenarios below exercise the slot-selection step over supplied digests, for
the reason the byte-budget requirement gives: reaching a refused draw through a
key would mean grinding for one.

#### Scenario: A refused combination is never returned

- **WHEN** the slot-selection step is exercised over a digest whose first draw
  lands on a refused combination
- **THEN** the name returned is not that combination

#### Scenario: A redraw is deterministic

- **WHEN** the slot-selection step is exercised twice over one digest whose first
  draw is refused
- **THEN** both yield the same name

#### Scenario: A redraw replaces every slot

- **WHEN** the slot-selection step is exercised over a digest whose first draw is
  refused
- **THEN** all four words are the ones the reserved bytes select
- **AND** the result is not the first draw with a single slot substituted, which
  is checked by constructing the digest so that those two differ

#### Scenario: An unrefused draw ignores the reserve

- **WHEN** the slot-selection step is exercised over a digest whose first draw is
  not refused, and again over a digest differing from it only in the reserved
  bytes
- **THEN** the two names are equal, so an unrefused draw does not consult the
  reserve

#### Scenario: No name a key can reach is a refused combination

- **WHEN** names are derived for many distinct public keys
- **THEN** none of the names returned is on the denylist

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
pin is stated as a requirement of its own rather than left to the four-word
requirement above. Counting the words of whatever the implementation returned
agrees with an implementation that joins three words, or five, or four in the
wrong order — the count and the thing being counted come from one source. A
written-down name is produced independently, so a change to the number of words,
to the order of the slots, or to the separator between them fails it.

Pinned cases SHALL include a key whose first draw is refused, so that the redraw
path is pinned and not only the common one.

#### Scenario: A fixed key yields a fixed name

- **WHEN** a name is derived for a fixed public key
- **THEN** it equals a name written down independently of this implementation

#### Scenario: The redraw path is pinned

- **WHEN** the slot-selection step is exercised over a fixed digest whose first
  draw lands on a refused combination
- **THEN** the name it yields equals a name written down independently of this
  implementation
- **AND** that written-down name is the one the reserved bytes select, not the
  refused draw

#### Scenario: A wordlist reordering is visible

- **WHEN** two entries of a list are exchanged and the pinned cases are derived
  again
- **THEN** at least one pinned case no longer matches its written-down name

#### Scenario: The pinned name is four words in the specified order

- **WHEN** a pinned case's written-down name is read
- **THEN** it is four words
- **AND** its first two are entries of the adjective list and its last two are
  entries of the noun list
- **AND** it was written down independently of the implementation, so a build
  joining a different number of words fails to match it

### Requirement: The wordlists are ASCII, single-voice, and carry no verdict

Every entry of both lists SHALL be ASCII, lowercase, and a single word
containing no whitespace.

ASCII is a bidi decision rather than a typographic preference. These are the one
piece of rendered text this project fully composes from a fixed list, so keeping
them ASCII means a generated name can never itself carry a bidi override or a
homoglyph — it removes the attack from this surface rather than mitigating it.
The obligation to handle bidi in everything a name is rendered *next to* is
untouched by this and belongs elsewhere.

The lists SHALL be drawn from Greek philosophy and letters. The adjectives SHALL
be in one voice, at once geographic and temperamental. The nouns SHALL pool the
vocabulary of Greek thought with the names of thinkers, writers and makers.

**A name may describe a texture, never a verdict.** No entry SHALL assert a
quality of the person carrying it, in either direction. Excluded by category:
words that congratulate their bearer; tyranny and violence; disorder read as an
accusation; pathology and death; anything mapping onto a real group; and the
sexual and bodily. The system can be blamed for generating a name even where the
user cannot be blamed for carrying one, and "the hash chose it" is not a defence.

No entry SHALL assert authority — no word naming a magistrate, an officer or a
moderator — because a participant handed such a word has been handed apparent
standing by the wordlist, and a name is never a credential.

No entry SHALL be a term this project's own vocabulary depends on, because a
collision is worst in a feed, where every row attributes a post to one of these
names. No entry SHALL be a figure whose mere invocation is a move in a debate,
because a user rendered under one is signed by them on every post and anyone
disagreeing is visually disagreeing with them.

`stoic` SHALL appear in the adjective list and SHALL NOT appear in the noun list:
in the adjective slot it cannot be misread as naming a place.

#### Scenario: Every entry is ASCII, lowercase and one word

- **WHEN** every entry of both lists is examined
- **THEN** each contains only ASCII characters
- **AND** each is lowercase
- **AND** none contains whitespace

#### Scenario: No entry is a term of this project's own vocabulary

- **WHEN** both lists are searched for the terms this design names as its own
- **THEN** none of them appears in either list

#### Scenario: No entry is an excluded figure

- **WHEN** both lists are searched for the figures excluded as arguments rather
  than names
- **THEN** none of them appears in either list

#### Scenario: The lists hold no duplicates

- **WHEN** each list is compared against itself
- **THEN** no entry appears twice in a list

#### Scenario: One word is an adjective and never a noun

- **WHEN** the noun list is searched for the adjective that names a school
- **THEN** it is absent from the noun list
- **AND** present in the adjective list

### Requirement: A name is returned wherever core returns an author, beside the address

Every reply in which core reports who authored something SHALL carry that
author's display name alongside the author's address, and SHALL NOT carry the
name in place of the address.

This is what the caller cannot do for itself. A name derives from a public key; a
reply reporting an author reports an **address**, which is a hash and from which
no key is recoverable. So a caller holding only an address cannot compute the
name, and a reply carrying only the address has handed the caller a value it
cannot render an attribution from. Core holds the key — the signed op carries
it — so core renders the name.

The address SHALL remain present in every such reply. The name is added beside it
and never substituted for it, which is what the requirement below on what a name
is not depends on.

The name SHALL be the one this capability's derivation produces for the public
key that signed, rather than a value stored with the content or carried by any
op. A name travelling as data is a name a relay could strip or forge.

Where a reply reports no author, it SHALL carry no name. A field that would be
meaningless is omitted rather than sent as an empty or null value, which is the
partial-success shape the module's reply contract forbids.

#### Scenario: A feed row carries a name and an address

- **WHEN** a feed of threads is read
- **THEN** each row carries the author's address
- **AND** each row carries a display name

#### Scenario: The name on a row is the pinned name for the signing key

- **WHEN** a feed row is read for a post signed by one of the fixed keys whose
  name is written down
- **THEN** the row's display name equals that written-down name

This is pinned to the written-down name rather than compared against the
derivation's own output, because a row built by calling the derivation and then
checked by calling the derivation agrees with itself whatever either does — it
would pass on a row that named the wrong author's key.

#### Scenario: A row's name follows the key that signed, not the row's position

- **WHEN** two posts signed by two different fixed keys appear in one feed
- **THEN** each row carries the written-down name for the key that signed that
  post
- **AND** exchanging the two posts' order exchanges the two names with them

#### Scenario: The address is not replaced

- **WHEN** a feed of threads is read
- **THEN** every row's author address is the address of the key that signed it
- **AND** adding the name changed no other field of the row

#### Scenario: Two posts by one author render one name

- **WHEN** two posts signed by one key are read in a feed
- **THEN** both rows carry the same display name
- **AND** both carry the same author address

#### Scenario: A reply with no author carries no name field

- **WHEN** a reply that reports no author is read
- **THEN** it carries no display name field at all, rather than an empty or null
  one

#### Scenario: A name does not travel as content

- **WHEN** a post arrives carrying a field that spells a display name
- **THEN** the name rendered for its author is the one derived from the signing
  key
- **AND** the arriving field does not change it

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

#### Scenario: Two identities sharing a name are both served unchanged

- **WHEN** two distinct identities whose keys derive the same name each author a
  post, and the feed is read
- **THEN** both rows carry that name exactly as derived
- **AND** neither carries a number, a suffix or any other added distinguishing
  mark
- **AND** the two rows carry different author addresses, which is what tells them
  apart

A colliding pair is not found by searching the key space, which is infeasible at
the specified size. It is constructed: the name is a function of the key, so a
pair is obtained by holding the derivation's inputs fixed at a chosen name and
taking two distinct keys that reach it, or by exercising the feed against a
derivation stubbed to one name. What is under test is the feed's handling of a
collision, not the likelihood of one.

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

This is about lookup and not about totality, and the distinction matters because
two other requirements do specify failures. A derivation can still fail because
the input is not a well-formed key, and it can still fail because the denylist
reserve was exhausted — neither is a key being unknown, and neither is relaxed
here. What this forbids is a derivation that refuses a key for not being
recognised.

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
one. A change SHALL include: removing a word, adding a word, reordering a list,
changing a list's size, changing the number of words, changing which bytes a slot
reads, and changing the denylist.

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
