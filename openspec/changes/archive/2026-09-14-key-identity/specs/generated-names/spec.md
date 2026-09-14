## MODIFIED Requirements

### Requirement: A display name is derived from a public key and from nothing else

A display name SHALL be a function of an identity's public key alone. The same
public key SHALL yield the same name on every peer, at every time, with no
lookup, no stored state and no published value participating.

Determinism is the whole point rather than a convenience. A name that two peers
could compute differently is one identity rendering as two people, which users
report as impersonation — so **a name is not published and cannot be**, because
a published name is a value peers could disagree about. A name is recomputed
wherever it is shown.

The derivation SHALL take the **public key**, and SHALL take it directly: the
name's three draws read bytes of the key itself, with no hash, no digest and no
domain separator standing between the key and the draws.

**The public key is the sole author identifier**, so there is no address to
derive from and no second derivation over the key to be separated from. The
earlier scheme hashed the key under a name separator because the key also fed an
author-address derivation, and two derivations over one key must be made
independent functions of it. With one derivation there is no counterpart, and
the separator separated the name from nothing.

Nothing varying between runs SHALL participate: no clock, no counter, no nonce,
no device identifier, no Stoa address, and no value carried by an op.

#### Scenario: One key always gives one name

- **WHEN** a name is derived twice from the same public key
- **THEN** both derivations yield the same name

#### Scenario: A name is derivable from a public key with no address supplied

- **WHEN** a name is derived and the only input given is a public key
- **THEN** a name is returned, so no other value is needed to compute one

#### Scenario: A name cannot be derived from an address

- **WHEN** any 32-byte value that is not the public key — a Stoa address among
  them — is accepted as the derivation's input in place of that key
- **THEN** the name it produces differs from the name for that key
- **AND** so a caller holding anything but the key cannot arrive at the right
  name

This scenario outlives the author address it was written for. The author address
is deleted by this change, so the specific value it named no longer exists; the
property it was pinning — that the name is a function of *the key* and of no
other value that travels beside it — is what makes the derivation checkable at
all, and it is stated here against any such value rather than dropped with the
one that prompted it.

#### Scenario: Derivation survives a restart

- **WHEN** a name is derived for a public key, the process is restarted, and a
  name is derived for that same key again
- **THEN** the two names are equal

#### Scenario: Different keys generally give different names

- **WHEN** names are derived for many distinct public keys
- **THEN** the names are not all equal, so the derivation depends on its input

#### Scenario: The name's input is the key's own bytes

- **WHEN** the bytes the derivation draws from are compared with the
  corresponding bytes of the public key it was given
- **THEN** they are equal, so no hash stands between the key and the draws

### Requirement: Each word slot reads distinct bytes, within a bounded slice

Each of the three slots SHALL take its index from bytes of the public key that
no other slot reads, so that the three words are independent draws rather than
three views of the same bits.

Each slot SHALL take a 16-bit draw reduced into its list. This is what makes the
reduction exactly uniform: 8,192 divides 65,536 eight times and 1,024 divides it
sixty-four times, so no entry of either list is favoured.

**The three draws SHALL consume key bytes `18..23` exactly** — two bytes feed the
adjective, two the noun, two the place — and the derivation SHALL read no key
byte outside that range under any input. A bounded slice is what stops two
implementations disagreeing about how far to read and so producing different
names for one key, and it is what makes the derivation checkable against a fixed
expected value at all.

**The bound is exactly what the derivation consumes, and it names no byte the
derivation does not read.** A bound stated wider than the draws would record a
boundary nothing enforces — a range read by nothing, which the next reader takes
as load-bearing and designs around.

Beyond the bound the derivation SHALL fail loudly rather than read further. With
the draws fixed at `18..23` of a 32-byte key this path is not reachable through
any well-formed key, and it is stated as a requirement on the derivation's shape
rather than as a behaviour a test can provoke: what it forbids is an
implementation that reads on, and a second implementation that read further
would produce a different name for the same key.

**Every draw is a single unconditional reduction.** The number of bytes a name
consumes is therefore fixed rather than data-dependent, every index of every
list is reachable, and the `2^33` space is reached exactly rather than
approximately — so the collision figures hold without a caveat, and no draw is
ever skipped in a way that would make one word rarer than another.

**The step that turns key bytes into three words SHALL be exercisable over a
supplied 32-byte value**, separately from being handed a public key. This is a
testability obligation rather than a surface to expose to callers: reaching a
chosen slot combination through a chosen *key* means grinding for one, so the
pinning and uniformity requirements are checkable only if the bytes can be
supplied directly.

#### Scenario: Each slot varies independently of the others

- **WHEN** for each slot in turn, the bytes that slot draws from are varied
  across their full range while every other byte is held fixed
- **THEN** that slot's word takes every value its list holds
- **AND** the other two slots' words do not change, so no byte feeds two slots

#### Scenario: Changing a byte one slot reads changes only that slot

- **WHEN** two keys differ only in a byte that one slot reads
- **THEN** the names derived from them differ in that slot's word
- **AND** agree in the other two slots

#### Scenario: The derivation reads no byte past its bound

- **WHEN** two keys agree on bytes `18..23` and differ on **every** byte outside
  that range
- **THEN** the names derived from them are equal

#### Scenario: The bytes a name consumes do not vary with the digest

- **WHEN** names are derived over many keys
- **THEN** each reads the same six bytes as every other, so no input makes the
  derivation consume more

### Requirement: The derivation is pinned to fixed expected names

The derivation SHALL be checkable against names stated independently of the
implementation, for fixed public keys.

This is consensus-critical in the silent direction. Change one entry of a list,
the order of two entries, or which bytes a slot reads, and this peer's names
stop matching every other peer's — with no error anywhere, because each peer
remains internally consistent. A check that asks the implementation what it
produced and agrees with the answer cannot see this; the expected name must be
written down.

**The written-down name is what pins the word count**, and this is the reason the
pin is stated as a requirement of its own. Counting the words of whatever the
implementation returned agrees with an implementation that joins two words, or
four, or three in the wrong order — the count and the thing being counted come
from one source. A written-down name is produced independently, so a change to
the number of words, to the order of the slots, or to the connector between them
fails it.

**A pin SHALL also fail when the name's byte window moves.** With the name
reading the key directly, a slot reading bytes `16..17` instead of `18..19`
produces a different name from the same key with nothing else changed, and the
window is not otherwise visible in any output. Pinned cases SHALL therefore be
chosen so that the bytes outside `18..23` differ between them, so that a
window that had slipped onto a neighbouring byte would read a different value
and fail the pin rather than coinciding with it.

**Pinned cases SHALL span the lists rather than clustering**, reaching a low and
a high index in each of the three slots, so that a pin is evidence about the
index arithmetic and not only about one region of one list. There is one
derivation path and no second one to pin separately: with no refusal and no
retry, every key exercises the same six bytes and the same three reductions.

#### Scenario: A fixed key yields a fixed name

- **WHEN** a name is derived for a fixed public key
- **THEN** it equals a name written down independently of this implementation

#### Scenario: A pinned name is reproducible from its digest by hand

- **WHEN** a pinned case's public key is taken, its three 16-bit draws are read
  from bytes `18..23`, and each is reduced into its list by hand
- **THEN** the three words reached are the pinned name's three words, so the pin
  checks the index arithmetic rather than agreeing with the implementation that
  produced it

#### Scenario: A wordlist reordering is visible

- **WHEN** two entries of a list are exchanged and the pinned cases are derived
  again
- **THEN** at least one pinned case no longer matches its written-down name

#### Scenario: A shifted byte window is visible

- **WHEN** a slot is altered to read the byte pair adjacent to the one it is
  specified to read, and the pinned cases are derived again
- **THEN** at least one pinned case no longer matches its written-down name

#### Scenario: The pinned name is three words and a connector, in order

- **WHEN** a pinned case's written-down name is read
- **THEN** it is an adjective-list entry, a noun-list entry, the connector, and a
  place-list entry, in that order
- **AND** it was written down independently of the implementation, so a build
  joining a different number of words, a different order, or a different
  connector fails to match it

### Requirement: The scheme and its wordlists are frozen, with no version to bump

Any change to the scheme SHALL rename every identity at once, and SHALL
therefore be taken as a deliberate migration rather than as a patch. A change
SHALL include: removing a word from any of the three lists, adding one,
reordering a list, changing a list's size, changing the number of slots,
changing which bytes a slot reads, and changing the connector.

**There is no scheme version and no way to mint one.** The name reads raw key
bytes, so there is no preimage in which a version could sit, and two schemes'
names cannot be made distinguishable. This is a one-way door and it is taken
deliberately: the earlier scheme carried a version inside its domain separator,
and removing the separator removes the seam with it.

**What this costs is smaller than it looks, and the reason is that a name is
local.** A name is never published, never travels, and is recomputed wherever it
is shown. Two peers on different builds rendering different names for one key is
therefore a client-side rendering difference, not a disagreement about anything
being agreed on — the same class of difference as two peers running different
app versions. It is not the impersonation failure that a divergence in a
published value would be.

**What survives is a recognition-aid inconsistency**, and it is real: someone
learns a name, tells another person to look for *pensive aporia of lampsakos*,
and that reader on a different build sees something else. The standing rule
already covers it — a name is never an identifier, and the public key is.

**Changing a list's size is a scheme change even when it looks like a
correction.** The sizes are what make the reduction unbiased, so taking the
adjective list to any value that is not a power of two introduces a bias, and
taking it to a different power of two reindexes every draw.

The mechanism is worth stating rather than asserting as a rule. The derivation
maps key bytes to list indices, so removing one word reindexes the list and
every identity that drew at or after the removed index renders differently — on
builds that have updated and not on builds that have not.

The lists SHALL therefore be conservative, and SHALL be treated as frozen.
Shipping a word that has to come out later is a migration in which everybody's
name changes at once, with no version to tell the two schemes apart.

#### Scenario: A different scheme version gives a different name for one key

- **WHEN** a name is derived for one public key
- **THEN** the derivation takes no version input that could be varied to
  distinguish two schemes, so its whole input is the same six key bytes
- **AND** the name carries no marker of which wordlists produced it

  **There is deliberately no wordlist change in this WHEN**, and an earlier
  version of this scenario had one. Both THENs hold without it — the absence of a
  version input and the absence of a scheme marker are standing properties of the
  derivation, not consequences of a change — so varying a wordlist here made the
  WHEN inert and invited a reader to take this scenario as covering what a
  wordlist change does. That is the next scenario's job.

#### Scenario: Removing a word renames identities that drew past it

- **WHEN** an entry is removed from a list, and names are derived again for keys
  that drew at or after the removed index
- **THEN** those names differ from the names the same keys had before
- **AND** no property of either name distinguishes which scheme produced it

#### Scenario: Changing which bytes a slot reads renames every identity

- **WHEN** a slot is altered to read different key bytes and names are derived
  again for many keys
- **THEN** the names differ from those the same keys had before

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

No operation SHALL accept a display name in place of an author's public key: not
a lookup, not a moderation target, not a vote target, not a request naming an
author. **The public key is the identity.** An author is identified by the key
that signs, and by nothing derived from it — the name and the mark are both
recognition aids computed from that key, and neither is the thing being
identified.

This is the sentence issue #80 changes rather than a restatement of the old one.
Until this change an author was identified by an address derived from the key,
and "the address is the identity" was the rule everywhere an author appeared.
With the author address deleted, a requirement still pointing at it would point
at a value no longer carried. **Stoa addresses are untouched**: a Stoa is still
identified by its address, and nothing here reaches that.

Collisions are rare rather than expected at the specified space, and that changes
nothing here. The rule is not a response to the rate.

#### Scenario: Two keys deriving one name each derive it unchanged

- **WHEN** two distinct public keys that derive the same name are each put through
  the derivation
- **THEN** each yields that name exactly as derived
- **AND** neither carries a number, a suffix or any other added distinguishing
  mark

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
for that" is unavailable. What an attacker gets is a lookalike name **on a
different public key**; they cannot forge the key and cannot forge a signature,
so nothing they publish is attributable to the identity they imitate. The whole
attack is social, and **the public key is what defeats it**.

Deriving a name SHALL consult no moderator set, no genesis record and no stored
state, being a function of the key alone. This follows from determinism and is
stated because a name is rendered next to moderator badges, where a reader may
read the pair as one claim.

A name SHALL be derivable for any well-formed public key, including one belonging
to no identity this peer has seen. There is nothing to look up, so there is
nothing that could fail **to be found**.

**The derivation SHALL be total over well-formed keys.** Every well-formed key
yields a name, and **malformed key material is the only failure this capability
has**. There SHALL be no other error condition, and in particular no failure that
a well-formed key belonging to a real identity could reach: an error variant no
input can produce is an unreachable branch that a reader takes as evidence the
failure exists, and a caller handles a case that cannot arrive.

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

## RENAMED Requirements

- FROM: `### Requirement: The scheme and its wordlists are versioned together and frozen`
- TO: `### Requirement: The scheme and its wordlists are frozen, with no version to bump`

The versioning half of the old name no longer holds: the name reads raw key
bytes, so there is no preimage in which a scheme version could sit. The freezing
half survives and is now the whole of the requirement.

## ADDED Requirements

### Requirement: The three channels read pairwise disjoint bytes of the public key

A reader is offered three derived channels for telling identities apart: the
**name**, the **mark**, and the **abbreviated public key on screen**. All three
read the same 32-byte public key. The set of key bytes each channel reads SHALL
be **pairwise disjoint** from the set each other channel reads.

**Disjointness is the whole of what makes the three channels independent, and it
is load-bearing rather than decorative.** Under the previous design the name
read a digest of the key under its own separator while the mark and the
abbreviation read an author address, a different digest of the same key — so the
three could not overlap, and their independence came from domain separation
whatever bytes each happened to read. That reasoning is withdrawn by this
change and MUST NOT be relied on: with the address gone and no hash between the
key and any channel, all three read one shared space, and two channels reading
one byte are two searches that partly coincide.

The security argument is that an attacker grinding for a lookalike name and an
attacker grinding for a lookalike mark are independent searches **only when the
channels share no input**, and independent searches multiply in cost rather than
adding. A byte the abbreviation **displays** is worse than merely shared — it is
a byte the attacker can target while reading their progress off the rendered
key, so it contributes nothing an observer could not have been handed directly.

The allocation SHALL be:

| channel | key bytes | count |
|---|---|---|
| abbreviation — head group | `0..3` | 4 |
| mark | `4..11` | 8 |
| abbreviation — middle group | `14..17` | 4 |
| name | `18..23` | 6 |
| abbreviation — tail group | `29..31` | 3 |
| **allocated** | | **25** |

Bytes `12..13` and `24..28` — seven in all — SHALL be read by no channel. They
are **unallocated rather than reserved**: no channel may be extended onto them
without this requirement changing, and nothing today depends on their value. The
budget closing with room to spare is what makes disjointness affordable; it is
not a boundary anything enforces against a use nobody has.

**The abbreviation SHALL keep a middle group**, drawn from the interior rather
than from either end. Head-and-tail alone is the shape a vanity generator is
built to defeat, and the middle group is what makes a convincing near-match
expensive. Preserving that shape is why the mark and the name are placed around
the abbreviation's groups rather than the abbreviation being narrowed to make
room.

**All three channels SHALL be measurable together, and each SHALL be measured by
varying key bytes and observing its output rather than by restating its
arithmetic.** Disjointness is a property *between* channels, so a check that can
only reach two of them checks something weaker than this requirement states. Two
of the three are rendered by the view and one is derived in the core, so
satisfying this obliges the name's byte window to be reachable from wherever the
other two are measured — and a channel's window that is read from a constant
rather than measured follows that constant wherever it moves and can never report
an overlap.

This is a requirement rather than an implementation note because it is the only
thing making the accepted cost — the derivation arithmetic existing in two
languages — safe rather than merely accepted. Without it, deleting whichever
component makes the name reachable leaves every requirement above still reading
as satisfied, `openspec validate --strict` still passing, and the pairwise check
silently reduced to the two-channel check it was before this change.

**Whatever makes the name's window reachable for this measurement SHALL NOT
render a name, and SHALL have no consumer other than the measurement.** It
reports which key bytes are read — as draw indices or equivalent — and nothing a
reader ever sees.

That restriction is what settles its obligations under *Malformed key material is
refused rather than crashed on*. That requirement forbids a name derived from
truncated or padded input, because such a name "would render as an ordinary
participant, which is a name attributable to nobody presented as one attributable
to somebody" — a hazard that exists only where a name is rendered. A measurement
apparatus renders nothing, so it SHALL instead accept any input and report
in-range values for all of it, including malformed input: it has no way to
express a refusal, and a measurement that returned a non-value for some input
would report that channel as reading no byte, satisfying disjointness with a
measurement that found nothing — the failure the non-emptiness scenario above
exists to forbid.

The two rules are therefore not in conflict: **anything that renders a name
refuses malformed input; the thing that only measures accepts it.** A change that
gives the measurement apparatus a rendering consumer moves it under the first
rule and SHALL make it refuse.

#### Scenario: The measurement apparatus renders no name

- **WHEN** whatever makes the name's window reachable for the measurement is
  examined
- **THEN** it exposes which key bytes are read, and no name
- **AND** nothing outside the measurement consumes it

#### Scenario: The measurement apparatus accepts malformed input

- **WHEN** malformed or truncated key material is given to the measurement
  apparatus
- **THEN** it reports in-range values rather than refusing
- **AND** no name is rendered from them anywhere

#### Scenario: No two channels read one byte

- **WHEN** the set of key bytes the name reads, the set the mark reads and the
  set the abbreviation displays are compared
- **THEN** each pair of those three sets is disjoint

#### Scenario: Each channel's measured set is non-empty

- **WHEN** the byte set each of the three channels reads is measured
- **THEN** each of the three sets is non-empty, so disjointness is not satisfied
  by a measurement that found nothing

#### Scenario: A byte one channel reads moves only that channel

- **WHEN** two public keys differ only in a byte that exactly one channel reads
- **THEN** that channel's output differs between them
- **AND** the other two channels' outputs are identical

#### Scenario: A displayed byte does not reach the name or the mark

- **WHEN** two public keys differ only in bytes the abbreviation displays
- **THEN** their names are equal
- **AND** their marks are identical

#### Scenario: A byte the name reads is not on screen

- **WHEN** two public keys differ only in bytes the name reads
- **THEN** their abbreviated forms are identical

#### Scenario: All three channels are measurable in one place

- **WHEN** the byte sets of the name, the mark and the abbreviation are compared
- **THEN** all three sets are obtained in one place, each by varying key bytes and
  observing that channel's output
- **AND** the name's byte window is reachable there, so removing whatever makes it
  reachable fails this requirement rather than only shrinking a test file

#### Scenario: A channel reading an unallocated byte is detected at every value

- **WHEN** a channel's output depends on an unallocated key byte, including when
  it depends on that byte only for a single one of the byte's 256 values
- **THEN** the measured byte set for that channel includes the unallocated byte

  A conditional read is the case this scenario exists for. A measurement that
  tries a fixed list of byte values reports a channel as not reading a byte it
  reads only at some value the list omits, and the disjointness and unallocated
  -byte scenarios above then hold about a set that is missing a member. The set
  each channel reads is therefore determined over the byte's whole domain.

#### Scenario: The abbreviation keeps a middle group

- **WHEN** a public key is abbreviated
- **THEN** the result shows a head group, a middle group and a tail group
- **AND** the middle group is drawn from the interior, with undisplayed bytes on
  both sides of it

#### Scenario: No channel reads an unallocated byte

- **WHEN** two public keys differ only in bytes `12..13` and `24..28`
- **THEN** their names are equal, their marks are identical, and their
  abbreviated forms are identical

## REMOVED Requirements

### Requirement: The derivation is domain-separated from every other use of the key

**Reason**: The requirement obliges the name's hashed preimage to begin with a
fixed-width domain separator distinct from the one every address derivation
uses, and it states that the name's independence from the mark and the
abbreviation "holds by domain separation and not by byte allocation". Both halves
are withdrawn by this change.

The separator existed to make the name and the author-address derivations two
independent functions of one key. Issue #80 deletes the author address, so there
is no second derivation over the key and nothing for the name to be separated
from. The owner's ruling is raw disjoint slices with no prefixes at all, so the
name reads key bytes directly and there is no preimage for a separator to begin.

The independence claim is not merely obsolete but **inverted**. It was correct
while the name read one digest and the mark and the abbreviation read another:
two digests cannot overlap, so no byte allocation between them was required or
possible, and the requirement was right to call any reserved range a mechanism
that does not exist. Under one shared 32-byte value the opposite holds — byte
allocation is the only thing keeping the searches independent — and leaving this
requirement in place would leave the spec asserting the negation of the
requirement that replaces it.

**Migration**: The replacement is *The three channels read pairwise disjoint
bytes of the public key*, above, which carries the same security argument —
grinding costs multiply rather than add — resting on byte-disjointness instead of
on domain separation. `OP_SIGNING_PREFIX` is unaffected and remains required:
it separates the signing digest and is what stops a signature over one preimage
being replayed as a signature over another. Removing the name's separator
reaches nothing on the signing path.

The scheme-version half of this requirement is carried by *The scheme and its
wordlists are frozen, with no version to bump* — which this change renames from
*The scheme and its wordlists are versioned together and frozen* (see `##
RENAMED Requirements` above) and whose text it rewrites to record that the
versioning seam is gone and what its loss costs. The post-rename title is the
one cited here, because it is the heading that exists once this change applies.

### Requirement: The mark and the abbreviated address read disjoint address bytes

**Reason**: Written against the author address, which #80 deletes, and scoped to
two channels of the three. It states that the name's independence "is settled by
the requirement above" — the domain-separation requirement removed above — and
that the mark and the abbreviation "are the pair whose disjointness must be
arranged". Once all three channels read one shared value, the name is no longer
settled elsewhere and the pair is a triple.

**Migration**: Replaced by *The three channels read pairwise disjoint bytes of
the public key*, above, which restates the same property against the public key,
extends it to all three channels pairwise, and keeps the middle-group obligation
and the security argument unchanged. The mark's byte window is unchanged in
offset; only the value it reads changes, from the address to the key.
