## MODIFIED Requirements

### Requirement: Current metadata resolves by last-write-wins, falling back to genesis

A reader MUST take a Stoa's current metadata from the **leading binding**
metadata op it holds for that Stoa, and MUST fall back to the genesis record's
founding values when it holds none.

**A metadata op binds only when all three of these hold**, and whether they
hold MUST be decided each time a Stoa's metadata is resolved:

- it is authentic — its signature verifies under the author key it names;
- that author is in the moderator set derived from the Stoa's genesis record,
  which the `moderation-resolution` capability's requirement "A Stoa's moderator
  set is derived from its genesis record" defines as the record's creator alone;
- the Stoa the op itself names is the Stoa being resolved.

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
any of the three checks MUST NOT displace an earlier binding one, and MUST NOT
be reported as the current metadata.

Only ops of the metadata kind decide current metadata. A post, revision, vote or
moderation op in the Stoa — by a moderator or anyone else — MUST NOT change it.

**The fallback values are the founding title and an empty description**, since
the genesis record carries no description. A resolution that falls back MUST
report that it did. A Stoa whose only metadata ops fail to bind MUST resolve
exactly as a Stoa with none.

A binding op's title and description MUST be taken as the op carries them,
including when either is empty. An empty title in a binding op is the current
title. It is not an absence, and it MUST NOT trigger the fallback.

Resolution MUST depend on nothing but the metadata ops held and the genesis
record. Two readers holding the same ops and the same record MUST reach the same
current metadata, whatever sequence those ops were received in.

#### Scenario: A Stoa with no metadata op shows its founding values

- **WHEN** a reader holds a Stoa's genesis record and no metadata op for it
- **THEN** the Stoa's displayed title is the founding title
- **AND** its description is empty
- **AND** the resolution reports that it fell back to the genesis values

#### Scenario: The creator's metadata op supplies the current values

- **WHEN** a reader holds a metadata op for a Stoa, authentically signed by that Stoa's creator and naming that Stoa
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

#### Scenario: An empty title in a binding op is a title, not a fallback

- **WHEN** a reader holds a binding metadata op for a Stoa whose title is empty
- **THEN** the current title is empty
- **AND** the resolution reports that it did not fall back

#### Scenario: Resolution does not depend on the sequence ops arrived in

- **WHEN** two readers hold the same metadata ops for a Stoa, binding and non-binding, appended in different sequences
- **THEN** both resolve the same current values
- **AND** both report the same answer as to whether they fell back

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

- **WHEN** a metadata op carries a title containing control or zero-width characters
- **THEN** decoding preserves them unchanged
- **AND** the obligation to render them safely rests with the renderer

## ADDED Requirements

### Requirement: A Stoa's metadata is answerable from its address and genesis record, joined or not

The module MUST offer a call, `getStoa`, that takes a Stoa address under the
field `stoa` and the genesis record that address names under the field
`genesis`. The record MUST be accepted in the same encoding that the
`stoa-membership` capability's calls report and accept, so a record taken from a
listing item or a creation reply is usable here as it is.

The call MUST verify the record against the address before resolving anything,
and MUST resolve current metadata from that record's moderator set and the ops
the peer holds, as the requirement "Current metadata resolves by last-write-wins,
falling back to genesis" defines.

**The call MUST answer for a Stoa the peer is not in.** Whether the peer has
joined the Stoa MUST NOT change the answer. Metadata ops the peer holds for an
unjoined Stoa MUST be resolved exactly as they would be for a joined one.

**The call MUST NOT change any state.** It MUST NOT record membership of the
Stoa, MUST NOT append or publish any op, and MUST NOT alter anything retained
for a Stoa the peer is in.

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

### Requirement: The reply carries the current metadata and says whether it fell back to genesis

A successful `getStoa` reply MUST carry every one of the following fields:

- `stoa`: the address that was asked for.
- `title` and `description`: the current metadata, as resolved. Where the
  resolution fell back, `title` is the genesis record's title and `description`
  is the empty string.
- `policy`: the genesis record's posting policy, reported by the stable name the
  `stoa-membership` capability uses for it — `"open"` for the policy every
  current Stoa declares. A metadata op carries no policy, so this field never
  comes from one.
- `isGenesisFallback`: a boolean. It is `true` exactly when no binding metadata
  op was held for the Stoa, and `false` whenever one was.

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

#### Scenario: The policy is reported by name from the genesis record

- **WHEN** `getStoa` is called for a Stoa created through this module
- **THEN** `policy` is `"open"`
- **AND** it is `"open"` whether or not a binding metadata op is held

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
  longer than the largest record that encoding can hold;
- the record does not verify against the address;
- the peer's op store cannot be consulted.

**A store that cannot be consulted MUST produce the error shape and MUST NOT
produce a fallback reply.** A fallback reply states that the peer holds no
binding metadata op, which it cannot know when the store is unreadable. Reporting
the founding values as `isGenesisFallback: true` would make a broken store
indistinguishable from a Stoa nobody has renamed.

A request field the call does not read MUST be ignored rather than refused, and
a request carrying one MUST be answered as though it were absent.

#### Scenario: A record that does not match the address is refused

- **WHEN** `getStoa` is called with an address and a well-formed genesis record of a different Stoa
- **THEN** the reply is the error shape
- **AND** it carries no title, description, policy or `isGenesisFallback`

#### Scenario: Malformed requests are refused in the error shape

- **WHEN** `getStoa` is called with a request that is not a JSON object, with `stoa` or `genesis` missing, with either holding a number, with an address of the wrong length or alphabet, and with a `genesis` that is not a decodable record
- **THEN** each reply is the error shape carrying a reason

#### Scenario: An unreadable store is an error, not a fallback

- **WHEN** `getStoa` is called with a valid address and record and the peer's op store cannot be consulted
- **THEN** the reply is the error shape
- **AND** it does not carry `isGenesisFallback`

#### Scenario: An unrecognised request field is ignored

- **WHEN** `getStoa` is called with a valid address and record and an additional field the call does not read
- **THEN** the call succeeds
- **AND** the reply is the one the request would have received without that field

### Requirement: Resolving a Stoa's metadata never aborts the process

Resolving current metadata, and the `getStoa` call that performs it, MUST NOT
panic for any request and for any combination of ops the op decoder accepts,
whatever their signatures, authors, kinds, Stoas, counters or field contents.

Every metadata op a peer holds arrived from a peer and is attacker-controlled. A
panic aborts the module process, which would let a single hostile op deny the
receiving peer every call, not only this one.

#### Scenario: An adversarial log resolves without aborting

- **WHEN** `getStoa` is called for a Stoa whose ops include forged, mis-signed, non-moderator, cross-Stoa and wrong-kind metadata ops, ops carrying the maximum representable counter, ops carrying no counter, and titles and descriptions at the maximum field length
- **THEN** the call returns a reply rather than aborting
- **AND** the module answers subsequent calls
