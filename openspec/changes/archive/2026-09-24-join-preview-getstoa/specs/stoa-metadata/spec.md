## REMOVED Requirements

### Requirement: Current metadata resolves by last-write-wins, falling back to genesis

**Reason**: The owner ruled that a Stoa title of `""`, or one made only of
whitespace or zero-width characters, is invalid in every respect. This
requirement made an empty title in a binding op the current title, and carried a
scenario, "An empty title in a binding op is a title, not a fallback", asserting
it. A MODIFIED block cannot drop a scenario, so the requirement is replaced
rather than amended.

**Migration**: Replaced by "Current metadata resolves by last-write-wins among
binding ops, falling back to genesis", added in this same change. Its text is
this requirement's with these edits: a fourth binding condition, that the op's
title is not blank as the `stoa-genesis` capability defines blank, the empty
title included, together with a statement that a blank-titled op stored as
bytes before the `op-format` capability refused one is not an op the reader
holds, since reading it fails; "three" becoming "four" where the checks are counted; the
paragraph taking an empty title as the current title narrowed to the
description; a new paragraph stating that no resolution produces a blank current
title; and the empty-title scenario replaced by four — a blank-title op does not
bind, does not displace a binding op, a title with one visible letter among
blank characters binds unaltered, and an empty description in a binding op is
still a description. Every other clause and scenario is unchanged. The
requirements citing this one by name are amended in this change to cite its
replacement.

## ADDED Requirements

### Requirement: Current metadata resolves by last-write-wins among binding ops, falling back to genesis

A reader MUST take a Stoa's current metadata from the **leading binding**
metadata op it holds for that Stoa, and MUST fall back to the genesis record's
founding values when it holds none.

**A metadata op binds only when all four of these hold**, and whether they
hold MUST be decided each time a Stoa's metadata is resolved:

- it is authentic — its signature verifies under the author key it names;
- that author is in the moderator set derived from the Stoa's genesis record,
  which the `moderation-resolution` capability's requirement "A Stoa's moderator
  set is derived from its genesis record" defines as the record's creator alone;
- the Stoa the op itself names is the Stoa being resolved;
- its title is not blank — blank having the meaning, and the exact list of
  blank characters, that the `stoa-genesis` capability's requirement "A blank
  title is not a valid title" gives it, under which the empty title is blank.
  The `op-format` capability refuses to encode or decode a metadata op with a
  blank title; this condition holds resolution to the same rule for any such op
  a reader nonetheless holds, meaning one its op log returns from a read. A
  blank-titled metadata op stored as bytes before that refusal is not held in
  that sense and is not an op that fails to bind: the `op-log` capability's
  requirement "A stored entry that does not decode fails every read that would
  return it" makes the read fail, and "`getStoa` refuses what it cannot answer,
  and never reports a fallback in place of a failure" says what `getStoa`
  answers then.

A reader MUST NOT treat a metadata op as binding because it was stored, because
it was accepted on an earlier resolution, or because the peer that relayed it
vouched for it. An authentic signature establishes who signed, and nothing about
whether the signer may rename the Stoa.

**"Leading" is the position the `op-ordering` capability's rule gives**: the
higher Lamport counter carried in the op's own signed bytes first, equal
counters broken by ascending op id, and an op carrying no counter after every op
that carries one. This capability MUST NOT define an ordering or a tiebreak of
its own, and resolution MUST NOT consult any transport-supplied ordering
metadata, any recorded arrival, or the op's author-asserted wall-clock.

Ops that do not bind MUST NOT take part in the ordering. A later op that fails
any of the four checks MUST NOT displace an earlier binding one, and MUST NOT
be reported as the current metadata.

Only ops of the metadata kind decide current metadata. A post, revision, vote or
moderation op in the Stoa — by a moderator or anyone else — MUST NOT change it.

**The fallback values are the founding title and an empty description**, since
the genesis record carries no description. A resolution that falls back MUST
report that it did. A Stoa whose only metadata ops fail to bind MUST resolve
exactly as a Stoa with none.

A binding op's title and description MUST be taken as the op carries them, with
no character removed, including the blank characters a title that is not blank
also carries. An empty description in a binding op is the current description.
It is not an absence, and it MUST NOT trigger the fallback.

**A resolution MUST NOT produce a blank current title**, the empty title
included. A binding op's title is not blank by the fourth condition above, and a
founding title is not blank because the `stoa-genesis` capability refuses a
record whose title is blank.

Resolution MUST depend on nothing but the metadata ops held and the genesis
record. Two readers holding the same ops and the same record MUST reach the same
current metadata, whatever sequence those ops were received in.

#### Scenario: A Stoa with no metadata op shows its founding values

- **WHEN** a reader holds a Stoa's genesis record and no metadata op for it
- **THEN** the Stoa's displayed title is the founding title
- **AND** its description is empty
- **AND** the resolution reports that it fell back to the genesis values

#### Scenario: The creator's metadata op supplies the current values

- **WHEN** a reader holds a metadata op for a Stoa, authentically signed by that Stoa's creator, naming that Stoa, and carrying a title that is not blank
- **THEN** the current title and description are the op's
- **AND** the resolution reports that it did not fall back

#### Scenario: A later rename supersedes an earlier one

- **WHEN** a reader holds two binding metadata ops for one Stoa, one carrying a higher counter than the other
- **THEN** the current values are those of the op carrying the higher counter

#### Scenario: A later rename wins whatever the two op ids are

- **WHEN** a reader holds two binding metadata ops for one Stoa where the op carrying the higher counter has the higher op id, and again where it has the lower op id
- **THEN** in both cases the current values are those of the op carrying the higher counter

#### Scenario: A metadata op by anyone but the creator does not bind

- **WHEN** a reader holds a metadata op for a Stoa that verifies and is signed by a key other than the Stoa's creator, and no other metadata op
- **THEN** the Stoa's current title is the founding title
- **AND** the resolution reports that it fell back to the genesis values

#### Scenario: A metadata op forging the creator's authorship does not bind

- **WHEN** a reader holds a metadata op for a Stoa that names the Stoa's creator as its author but whose signature was made by another key, and no other metadata op
- **THEN** the Stoa's current title is the founding title
- **AND** the resolution reports that it fell back to the genesis values

#### Scenario: A non-binding later op does not displace a binding earlier one

- **WHEN** a reader holds a binding metadata op for a Stoa and a metadata op for the same Stoa carrying a higher counter that fails to bind
- **THEN** the current values are those of the binding op

#### Scenario: A creator's metadata op naming another Stoa does not rename this one

- **WHEN** one key has created two Stoas and has signed a metadata op naming only the second
- **THEN** the first Stoa resolves to its founding values and reports that it fell back
- **AND** the second Stoa resolves to the op's values

#### Scenario: An op carrying a counter leads one carrying none

- **WHEN** a reader holds two binding metadata ops for one Stoa, one carrying a counter and one carrying none
- **THEN** the current values are those of the op carrying a counter
- **AND** this holds whichever of the two has the lower op id

#### Scenario: Ops of other kinds do not change current metadata

- **WHEN** a reader holds no metadata op for a Stoa, and holds posts, votes and moderation ops in that Stoa signed by its creator
- **THEN** the Stoa resolves to its founding values and reports that it fell back

#### Scenario: A metadata op carrying a blank title does not bind

- **WHEN** a reader holds, for a Stoa, a metadata op authentically signed by that Stoa's creator and naming that Stoa whose title is the empty string, and no other metadata op — and again when that op's title is U+0020 U+200B U+3000 instead
- **THEN** in each case the Stoa's current title is the founding title
- **AND** the resolution reports that it fell back to the genesis values

#### Scenario: A later metadata op carrying a blank title does not displace a binding one

- **WHEN** a reader holds a binding metadata op for a Stoa, and a metadata op for the same Stoa signed by its creator, carrying a higher counter and an empty title — and again when that later op's title is U+0020 U+200B U+3000 instead
- **THEN** in each case the current values are those of the binding op
- **AND** the resolution reports that it did not fall back

#### Scenario: A title with one visible letter among blank characters binds unaltered

- **WHEN** a reader holds, for a Stoa, a metadata op authentically signed by that Stoa's creator and naming that Stoa whose title is U+0020 U+200B, the letter `a`, U+3000 U+FEFF, and no other metadata op
- **THEN** the current title is those five characters in that order, with none removed at either edge
- **AND** the resolution reports that it did not fall back

#### Scenario: An empty description in a binding op is a description, not a fallback

- **WHEN** a reader holds a binding metadata op for a Stoa whose description is empty
- **THEN** the current description is empty
- **AND** the current title is the op's
- **AND** the resolution reports that it did not fall back

#### Scenario: Resolution does not depend on the sequence ops arrived in

- **WHEN** two readers hold the same metadata ops for a Stoa, binding and non-binding, appended in different sequences
- **THEN** both resolve the same current values
- **AND** both report the same answer as to whether they fell back

## MODIFIED Requirements

### Requirement: A displayed title is never an identifier

A Stoa's displayed title SHALL NOT be treated as identifying it. Two Stoas may
carry identical titles, and a title is chosen freely by whoever signed the op.

Display text reaches a reader exactly as its author wrote it — the encoding
validates UTF-8 and deliberately applies no normalisation, because normalising
would break the canonicality every peer's agreement on op ids depends on. A
title may therefore contain bidirectional controls, zero-width characters, or
homoglyphs of an established Stoa's name. Only a Stoa's moderator can set its
current title, and nothing stops a moderator choosing such a title, just as
nothing stopped the creator choosing one as the founding title.

Whoever renders a title SHALL therefore mitigate at the point of display:
strip or visibly mark bidi and zero-width controls, show the Stoa address
alongside any name, and never resolve or match a Stoa by title. The Stoa
address is the identity.

#### Scenario: Two Stoas may share a displayed title

- **WHEN** two distinct Stoas declare the same current title
- **THEN** both remain distinct Stoas with distinct addresses
- **AND** neither is treated as the other

#### Scenario: Display text is preserved rather than sanitised in the data layer

- **WHEN** a metadata op carries a title containing control or zero-width characters alongside at least one character that is not blank, as the `stoa-genesis` capability's requirement "A blank title is not a valid title" defines blank
- **THEN** decoding preserves them unchanged
- **AND** the obligation to render them safely rests with the renderer

### Requirement: A Stoa's metadata is answerable from its address and genesis record, joined or not

The module MUST offer a call, `getStoa`, that takes a Stoa address under the
field `stoa` and the genesis record that address names under the field
`genesis`. The record MUST be accepted in the same encoding that the
`stoa-membership` capability's calls report and accept, so a record taken from a
listing item or a creation reply is usable here as it is.

The call MUST verify the record against the address before resolving anything,
and MUST resolve current metadata from that record's moderator set and the ops
the peer holds, as the requirement "Current metadata resolves by last-write-wins
among binding ops, falling back to genesis" defines.

**The call MUST answer for a Stoa the peer is not in.** Whether the peer has
joined the Stoa MUST NOT change the answer. Metadata ops the peer holds for an
unjoined Stoa MUST be resolved exactly as they would be for a joined one.

**The call MUST NOT change any state.** It MUST NOT record membership of the
Stoa, MUST NOT append or publish any op, and MUST NOT alter anything retained
for a Stoa the peer is in.

**Initialising an empty op store for a peer that has none is not a change of
state under this requirement.** A store initialised that way holds no ops, and
no call of this module answers differently for it than for a peer that has no
store. A peer that has never stored an op MUST be answered as holding no
metadata op for the Stoa — a fallback reply — and MUST NOT be refused because no
op store existed before the call.

#### Scenario: A joined Stoa's metadata is answered

- **WHEN** a peer that has created a Stoa calls `getStoa` with that Stoa's address and the record its creation reported
- **THEN** the call succeeds
- **AND** the reply's `stoa` is the address that was asked for

#### Scenario: A Stoa the peer has not joined is answered from the ops it holds

- **WHEN** a peer that is not in a Stoa holds a binding metadata op for it, and calls `getStoa` with that Stoa's address and genesis record
- **THEN** the call succeeds
- **AND** the reply carries that op's title and description

#### Scenario: Asking about a Stoa does not join it

- **WHEN** a peer calls `getStoa` for a Stoa it is not in, and then lists its Stoas
- **THEN** that Stoa is not in the listing

#### Scenario: A record reported by a listing is accepted as it is

- **WHEN** a Stoa is listed and the address and record from that item are supplied to `getStoa`
- **THEN** the call succeeds
- **AND** the record is not refused as malformed or as failing to match the address

#### Scenario: A peer that has never stored an op is answered with a fallback

- **WHEN** `getStoa` is called with a valid address and record on a peer that has no op store yet
- **THEN** the call succeeds
- **AND** `title` is the genesis record's title and `isGenesisFallback` is `true`

### Requirement: The reply carries the current metadata and says whether it fell back to genesis

A successful `getStoa` reply MUST carry every one of the following fields:

- `stoa`: the address that was asked for, in the display form the `identity`
  capability's requirement "A Stoa address's display form parses strictly"
  defines — lowercase, whatever case the request spelled it in.
- `title` and `description`: the current metadata, as resolved. Where the
  resolution fell back, `title` is the genesis record's title and `description`
  is the empty string.
- `policy`: the genesis record's posting policy, reported by the stable name the
  `stoa-membership` capability uses for it — `"open"` for the policy every
  current Stoa declares. A metadata op carries no policy, so this field never
  comes from one.
- `isGenesisFallback`: a boolean. It is `true` exactly when no binding metadata
  op was held for the Stoa, and `false` whenever one was.

**`title` MUST NOT be blank in any successful reply**, whether or not the
resolution fell back — blank as the `stoa-genesis` capability's requirement "A
blank title is not a valid title" defines it, so neither the empty string nor a
string made only of the blank characters it lists.

**The reply MUST NOT carry the founding title in a field of its own.** The
founding title is in the genesis record the caller supplied, and the
`stoa-membership` capability's calls are what report it under a name of its own;
this call leaves that to them. A `title` in a reply whose `isGenesisFallback` is
`true` is the founding title. A `title` in a reply whose
`isGenesisFallback` is `false` is a current title, and the reply does not report
the founding title beside it.

**`isGenesisFallback` is the only field that says whether a resolution fell
back, and a caller MUST NOT need to infer that from the values.** A `title` equal
to the genesis record's title does not establish a fallback, because a binding op
may carry the founding title. An empty `description` does not establish one
either, because a binding op may carry an empty description.

The title and description MUST be reported exactly as the op or the record
carries them, with no character removed, replaced or added.

#### Scenario: A Stoa with no binding metadata op reports its founding values as a fallback

- **WHEN** `getStoa` is called for a Stoa the peer holds no binding metadata op for
- **THEN** `title` is the genesis record's title
- **AND** `description` is the empty string
- **AND** `isGenesisFallback` is `true`

#### Scenario: A renamed Stoa reports its current title

- **WHEN** `getStoa` is called for a Stoa whose leading binding metadata op carries a title different from its founding title
- **THEN** `title` is the op's title
- **AND** `isGenesisFallback` is `false`

#### Scenario: A binding op carrying the founding title is not reported as a fallback

- **WHEN** `getStoa` is called for a Stoa whose leading binding metadata op carries exactly the founding title and an empty description
- **THEN** `title` is the genesis record's title
- **AND** `description` is the empty string
- **AND** `isGenesisFallback` is `false`

#### Scenario: No successful reply carries a blank title

- **WHEN** `getStoa` succeeds for a Stoa that falls back, for a Stoa whose leading binding metadata op carries a title that is not blank, for a Stoa whose only creator-signed metadata op carries an empty title, and for a Stoa whose only creator-signed metadata op carries the title U+0020 U+200B U+3000
- **THEN** in each reply `title` is a string containing at least one character that is not a blank character
- **AND** the third and fourth replies' `title` is the founding title and their `isGenesisFallback` is `true`

#### Scenario: The policy is reported by name from the genesis record

- **WHEN** `getStoa` is called for a Stoa created through this module
- **THEN** `policy` is `"open"`
- **AND** it is `"open"` whether or not a binding metadata op is held

#### Scenario: An address asked for in uppercase is reported in its display form

- **WHEN** `getStoa` is called with a Stoa's address with every letter uppercased, and that Stoa's genesis record
- **THEN** the call succeeds
- **AND** `stoa` is the address in lowercase

#### Scenario: Every field is present on every successful reply

- **WHEN** `getStoa` succeeds, once for a Stoa that falls back and once for a Stoa that does not
- **THEN** both replies carry `stoa`, `title`, `description`, `policy` and `isGenesisFallback`

#### Scenario: No reply carries the founding title in a field of its own

- **WHEN** `getStoa` succeeds, once for a Stoa that falls back and once for a Stoa whose leading binding metadata op carries a title and a description that both differ from its founding title
- **THEN** neither reply carries a `foundingTitle` field
- **AND** no field of the second reply holds the founding title

#### Scenario: Display text is reported unaltered

- **WHEN** `getStoa` is called for a Stoa whose leading binding metadata op carries a title and description containing bidirectional and zero-width control characters
- **THEN** `title` and `description` carry those characters unchanged

### Requirement: `getStoa` refuses what it cannot answer, and never reports a fallback in place of a failure

Every failure of `getStoa` MUST be reported in the module's single error shape,
carrying the reason, with none of the success fields beside it.

The call MUST fail when:

- the request is not a JSON object;
- `stoa` or `genesis` is absent, or is not a string;
- `stoa` is not a well-formed Stoa address;
- `genesis` is not a record the genesis encoding decodes, including when it is
  longer than the largest record that encoding can hold, and including when its
  title is blank — the empty string, or a string made only of the blank
  characters the `stoa-genesis` capability lists;
- the record does not verify against the address;
- the peer's op store cannot be consulted. A peer that has no op store yet is
  not this case; the requirement "A Stoa's metadata is answerable from its
  address and genesis record, joined or not" says how it is answered;
- a read of the Stoa's ops reaches a stored entry the op store cannot decode,
  as the `op-log` capability's requirement "A stored entry that does not decode
  fails every read that would return it" defines. This includes a metadata op
  whose title is blank that was stored before the `op-format` capability
  refused one.

**A store that cannot be consulted MUST produce the error shape and MUST NOT
produce a fallback reply.** A fallback reply states that the peer holds no
binding metadata op, which it cannot know when the store is unreadable. Reporting
the founding values as `isGenesisFallback: true` would make a broken store
indistinguishable from a Stoa nobody has renamed.

**A stored entry that does not decode MUST likewise produce the error shape,
carrying the reason the decoding gave, and MUST NOT produce a fallback reply.**
The call MUST NOT answer as though the entry were absent, or as though it were a
metadata op that fails to bind, and MUST NOT migrate, rewrite or remove it.

A request field the call does not read MUST be ignored rather than refused, and
a request carrying one MUST be answered as though it were absent.

#### Scenario: A record that does not match the address is refused

- **WHEN** `getStoa` is called with an address and a well-formed genesis record of a different Stoa
- **THEN** the reply is the error shape
- **AND** it carries no title, description, policy or `isGenesisFallback`

#### Scenario: Malformed requests are refused in the error shape

- **WHEN** `getStoa` is called with a request that is not a JSON object, with `stoa` or `genesis` missing, with either holding a number, with an address of the wrong length or alphabet, and with a `genesis` that is not a decodable record
- **THEN** each reply is the error shape carrying a reason

#### Scenario: A record whose title is blank is refused, not answered as a fallback

- **WHEN** `getStoa` is called with a record that is otherwise a well-formed genesis record but whose title is the empty string, together with the address computed by hashing that record's bytes — and again with such a record whose title is U+0020 U+200B, with the address computed the same way
- **THEN** each reply is the error shape
- **AND** each reason names the title as blank
- **AND** neither carries a title, description, policy or `isGenesisFallback`

#### Scenario: A record whose title has one visible letter among blank characters is answered

- **WHEN** `getStoa` is called for a Stoa the peer holds no binding metadata op for, whose genesis record's title is U+0020 U+200B, the letter `a`, U+3000 U+FEFF
- **THEN** the call succeeds
- **AND** `title` is those five characters in that order, with none removed at either edge
- **AND** `isGenesisFallback` is `true`

#### Scenario: An unreadable store is an error, not a fallback

- **WHEN** `getStoa` is called with a valid address and record and the peer's op store cannot be consulted
- **THEN** the reply is the error shape
- **AND** it does not carry `isGenesisFallback`

#### Scenario: A blank-titled metadata op stored before the refusal is an error, not a fallback

- **WHEN** `getStoa` is called with a valid address and record, on a peer whose persistent op store holds, for that Stoa, an entry stored as it would have been before the `op-format` capability refused a blank title — a metadata op signed by the Stoa's creator whose title is the empty string — and no other op
- **THEN** the reply is the error shape
- **AND** its reason names the title as blank
- **AND** it carries no title, description, policy or `isGenesisFallback`
- **AND** the stored entry afterwards is the entry stored before the call

#### Scenario: An unrecognised request field is ignored

- **WHEN** `getStoa` is called with a valid address and record and an additional field the call does not read
- **THEN** the call succeeds
- **AND** the reply is the one the request would have received without that field
