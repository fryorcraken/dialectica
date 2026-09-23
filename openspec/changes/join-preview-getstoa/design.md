## Context

`proposal.md` says why. This records how, and why each choice beat the others.

Two halves meet here, and they were one change only because the owner's blank
title rulings landed while the view change was being specified.

- **The view** already had everything the preview needs except the call.
  `DJoinScreen` keeps its join outcome tied to the reference it was made for,
  via an `outcome` object and a `currentOutcome` that filters by `(stoa,
  genesis)`. `Main.qml` ships one reused screen and rebinds its two strings.
  `getStoa` has existed since #98 (PR #144). No view called it.
- **The core** accepted an empty title everywhere, and several specs pinned
  that as legal. The genesis record's `canonical_bytes` was already fallible
  (the 1024-byte cap). The op's `canonical_bytes` was total, and `SignedOp::verify`,
  `Op::id` and `Op::sign` all compute over it.

## Goals / Non-Goals

**Goals**

- One definition of "blank" in the core, used by the genesis codec, the op
  codec and the metadata resolver, and a copy in the view that tests the same
  thirty code points.
- The preview renders what `getStoa` answers, in the position
  `isGenesisFallback` decides, and nothing it did not answer.

**Non-Goals**

- The lookalike comparison stays over founding titles. See decision 11.
- No display-point mitigation of bidi or zero-width characters. The proposal
  scopes it out.
- No migration of a store that already holds a blank-titled record. See
  decision 6.
- Publishing a metadata op is #125's.

## Decisions

### 1. The blank set is a frozen list of thirty code points, not a Unicode property

`stoa::BLANK_CHARACTERS` is a `[char; 30]`, and `Core.blankCodeUnits` in the
view holds the same thirty values. `is_blank_title` is "every character is in
the list", so the empty title is blank because it has no character that is not.

**Why not `char::is_whitespace`.** It follows Unicode's `White_Space` property
*as of the Rust toolchain that built the peer*. A record's validity decides
whether a Stoa exists at all, so two peers on toolchains with different Unicode
tables would disagree about which Stoas exist, with no error anywhere. That has
happened to this property before: U+180E MONGOLIAN VOWEL SEPARATOR was
`White_Space` until Unicode 6.3 and is not now. The spec names U+180E as
not blank for exactly that reason, and a live property would have made that
scenario depend on the toolchain.

**Why not a regex in the view.** JavaScript's `\s` is a third set. It includes
U+FEFF and excludes U+0085 and the other four zero-width characters, so the
view would disagree with the core in both directions. The view walks a list
because the core walks a list.

**Why WORD JOINER is in and the bidi controls are out.** The ruling covers
"whitespace or zero-width characters". U+2060 WORD JOINER is the zero-width
no-break character Unicode recommends in place of U+FEFF, so leaving it out
would leave a one-character route around the rule. The bidirectional controls
(U+200E, U+200F, U+202A–U+202E, U+2066–U+2069) are formatting that changes how
neighbouring text is drawn. They are not empty space, and stripping or marking
them is the renderer's job under `stoa-metadata`'s "A displayed title is never
an identifier". A title of U+200E alone is therefore valid, and the spec pins
that.

**Why one failure rather than "empty" and "whitespace".** The ruling says a
whitespace-only title is invalid "in the same way as `""`". A caller has nothing
different to do for the two, so `GenesisError::BlankTitle` and
`OpError::BlankTitle` each cover both, and a test pins that every blank title
produces the same failure.

**Why only an entirely-blank title is refused.** Blank characters beside a
visible one are left as given: not refused, trimmed or normalised. Trimming
would change the record's bytes and therefore its address, and normalising text
is what `op-format`'s canonicality requirement forbids.

### 2. The op's layout and the op's encoding are two functions

`Op::canonical_bytes` stays total. It is the layout of whatever the struct
holds, and `id()`, `sign()` and `verify()` compute over it. `Op::encode` is the
encoding: the guard `Op::check_admitted`, then the layout. `Op::decode` runs the
same guard on what it built, and `SignedOp::to_bytes` goes through `encode`.
Commit 1 of this change made that split with no behaviour change. The blank
rule is then one guard with two callers.

**Why the split, rather than making `canonical_bytes` fallible.** The
spec-writer routed one question here: `stoa-metadata`'s resolver must refuse a
blank-title metadata op "a reader nonetheless holds", and its scenarios need an
authentically signed blank-title op in a log to set up. Three alternatives were
considered, and each breaks that.

- **A title newtype that cannot be blank.** Mistakes become unrepresentable,
  which is normally the right move here. It also makes the resolver's scenarios
  impossible to construct, so the fourth binding condition would be untestable
  code. The spec requires the condition *and* its scenarios, so this reading of
  the spec is not available.
- **`canonical_bytes` returning `Result`.** Then `verify()` of a blank-title op
  returns `false`, because there are no bytes to check a signature against. The
  op fails the *authenticity* check before reaching the blank one. Deleting the
  resolver's blank condition would then leave every test green: the condition
  would be masked, not tested. It would also make `id()` fallible, and
  every log keys on it.
- **Refusing only on decode.** The spec requires the encoder to refuse too. It
  would also repeat the defect `op.rs`'s `MAX_FIELD_LEN` comment already
  describes: an encoder producing bytes its own decoder refuses.

With the split, a test builds the op by struct literal, signs it with `sign()`,
and holds it in a `MemoryOpLog`. The only thing refusing it is the resolver's
blank check.

**What breaks without the guard in `encode`** (measured):
`a_metadata_op_with_a_blank_title_is_refused_on_encode`,
`a_blank_titled_op_can_still_be_built_signed_and_verified_in_memory`,
`an_op_with_no_encoding_is_refused_and_nothing_is_written` and
`a_blank_titled_metadata_op_is_not_published_or_stored`. **Without it in
`decode`:** `a_metadata_op_with_a_blank_title_is_refused_on_decode` and
`a_blank_titled_metadata_op_from_a_peer_is_refused_at_the_boundary`.

### 3. `SignedOp::to_bytes` is fallible, and the SQLite log refuses at the write

If `to_bytes` stayed total, a blank-title op could be written to the SQLite log.
`decode_entry` reads rows through `SignedOp::from_bytes`, which refuses it, so
every later read of that Stoa would fail as `CorruptEntry`: the store would
poison itself. This is the same trap as the over-cap body that `authoring.rs`
guards with `body_within_cap`. So `append` encodes first and answers
`OpLogError::Unencodable` without writing. `transport::publish` does the same
before its append, as `PublishError::Unencodable`.

**`MemoryOpLog` does not refuse, and that difference is deliberate.** It holds
`SignedOp` values, not bytes, so it has no read path that could be poisoned.
It is the "reader that nonetheless holds one" the resolver requirement is
written for. No production path hands a log a blank-title op anyway: the
transport decodes, which refuses, and `authoring` builds no metadata op.

**What breaks without the check in `append`:**
`an_op_with_no_encoding_is_refused_and_nothing_is_written`.

### 4. The resolver's fourth condition is checked before authority

`binding_metadata` tests the kind, then blankness, then `Moderators::authorises`.
The two string tests are free and `authorises` verifies a signature, which is the
ordering argument the kind filter already makes. The condition duplicates the
decoder's refusal on purpose. The spec asks for it so that resolution
does not depend on every path into a log having gone through `decode`.

**What breaks without it:** `a_metadata_op_carrying_a_blank_title_does_not_bind`,
`a_later_blank_titled_op_does_not_displace_a_binding_one`, and the wire-level
`a_creator_signed_blank_titled_metadata_op_is_answered_as_a_fallback`, and only
those. Every other path to a blank-title entry goes through `decode`.

### 5. The genesis record checks blankness after the structure

`Genesis::canonical_bytes` refuses a blank title after the length cap.
`Genesis::decode` refuses it after `cursor.finish()`, once the input has been
shown to be exactly one record. A blank title with trailing bytes therefore
reports `TrailingBytes`: the structural fault comes first, which is the
answer that points at the right half of the problem. The op decoder does the same,
through `check_admitted` on the built op.

Nothing in `wire.rs` checks titles, because the codec answers every path:

- creation's `genesis.address()` fails with `{"error":"title: title is blank…"}`
  before the store is opened;
- a join, a `getStoa`, and `listThreads`/`readThread` go through
  `genesis_for`'s decode and fail with `{"error":"genesis: title is blank…"}`;
- a listing reaches a retained record through `decode_row` and fails as
  `CorruptEntry`, carrying the decoder's reason.

**On join and `getStoa` the refusal is made twice, and only the listing can
tell the decode half apart.** `genesis_for` decodes, and then
`Membership::verified` re-encodes the record, which the encoder refuses too.
Measured by deleting each check in turn:

- Without the check in `Genesis::decode`, join and `getStoa` still refuse,
  through `verified`, whose reason also names the title as blank. What goes red
  is `a_retained_blank_titled_record_is_reported_neither_skipped_nor_migrated`
  and the `stoa.rs` decode tests. Without the decode check a listing would
  report a blank-titled row as "does not hash to that address", which is the
  wrong reason.
- Without the check in `Genesis::canonical_bytes`, creation succeeds with an
  empty title, and seven tests go red, among them
  `a_blank_title_creates_nothing_and_says_it_is_blank` and
  `a_blank_titled_record_cannot_be_joined`.

### 6. A retained blank-titled record is reported, not migrated or skipped

This is the owner's ruling, and it is decision 5 applied to what is already on
disk. The record's bytes still hash to its address, and the decoder now refuses
them. `MembershipStore::list` already reports a row that does not decode as
`CorruptEntry` and leaves it as it was, so nothing new was written for this case.
Before 0.1.0 only development stores can hold such a record. A store that does
answers every listing with the failure until the row is removed by hand.

### 7. The view looks up the reference on screen, keyed to that reference

`DJoinScreen.lookUp()` runs on `Component.onCompleted` and whenever
`stoaAddress` or `stoaGenesis` changes. It makes no call while either is empty,
which is how "no reference" reaches this screen. Malformed paste text never
reaches it at all, because `DStoaReference.parse` refuses it on the list.

The reply is stored the way the join outcome already is:
`lookup = {stoa, genesis, …}`. A `currentLookup` returns it only while that
pair is still on screen. This is the join outcome's argument, for the same
impersonation reason. A title answered for one reference and rendered over
another lends a trusted name to an untrusted address.

**Synchronous, and what that costs.** `Main.qml` binds the two strings
separately, so switching straight from reference A to reference B changes one
and then the other. That makes one extra lookup, for B's address and A's
record. The core refuses the pair as a mismatch, and the answer is keyed to a
pair that is never on screen, so it is never rendered. The shipped navigator
cannot produce the case: `previewing` passes through `null` between two pastes.
`Qt.callLater` would avoid the extra call. It was not used because it makes
every assertion in the suite wait on the event loop, and a callback outliving a
destroyed screen is a warning that the `check_bindings` gate turns into a
failure.

### 8. `isGenesisFallback` alone decides which panel `title` fills

The issue asks that the flag be "surfaced or at least not discarded", because it
distinguishes a current name from "this may be stale". So it does two things.

- It picks the panel. A fallback `title` becomes the founding title (so the
  existing panel, caption and lookalike comparison apply to it). A
  non-fallback `title` becomes `currentTitle`, which fills the current-title
  panel and never the founding one.
- A fallback is also *stated*, in `fallbackNote`. The statement is that this
  machine holds no moderator-set title, and that a rename it has not received
  would look the same. That is all a fallback establishes. Rendering
  the founding title with no note would let its presence read as "not
  renamed".

`foundingTitle` and `currentTitle` are now both derived rather than assigned.
The `currentTitle` doc comment used to say nothing on this build could supply
one. That stopped being true with `getStoa`, and the comment now names the
lookup.

Where a join has succeeded, its reply's founding title is used first. A
fallback lookup's title is used where the join supplied none that is not blank.
The two can only disagree if the core answers the same record two ways.

### 9. The description is shown only from a non-fallback reply, and only when non-empty

The issue says "and `description` where the design has room for it". The design
bundle's join screen has a founding panel and a current panel and no
description position. A description is set in the same metadata op as a
current title, so it goes in the current panel under its own caption. A fallback
reply always carries `""` (the genesis record has no description), so nothing is
lost by never reading it there. Reading it from a fallback reply would render a
value no moderator set.

### 10. A failed lookup is rendered as its own failure, and the join stays offered

The error shape, an unreachable core, and a malformed reply all render in
`lookupFailurePanel` with their reason, and fill neither title position. The
panel is not `joinFailurePanel`. The join button stays. The view cannot tell
failure causes apart without parsing the core's error text. A lookup refused
for a mismatched record would make the join fail too, while one refused for an
unreadable op store would not, and the join is a separate call the core answers
for itself. Withdrawing the join on a lookup failure would be the view
guessing at the core's answer.

### 11. The lookalike comparison stays over founding titles

This is the owner's scope note on #143. A Stoa can be founded under any title
and then renamed to match one the user holds. `getStoa` then reports
`isGenesisFallback: false`, so no founding title is available and the
comparison does not run. That is accepted for now: the creator's key, the
identicon and whether the user already joined it are enough signal at this
stage, and on-chain Stoas after 0.1.0 are what will fix a Stoa's identity.
`lookalikes` reads only `foundingTitle`, which a non-fallback reply never
fills. So a current title cannot reach the comparison, and
`a_current_title_is_not_compared_against_held_stoas` pins that rather than
leaving it to be inferred.

**A blank held title cannot match, by construction.** `foundingTitle` is never
blank: a blank join title and a blank lookup title are both discarded before it.
Two equal strings are both blank or both not, so no held Stoa with a blank title
can equal it. The comparison takes no separate guard, and none should be added:
it would have no test that could fail.

### 12. A misshapen `getStoa` reply is a failure, a blank `title` included

`Core.stoaMetadataFrom(reply)` normalises one reply into
`{ok: true, isGenesisFallback, title, description}` or `{ok: false, error}`.
A reply missing `isGenesisFallback` as a boolean, or `title` or `description`
as a string, or carrying a blank `title`, is `ok: false`, and the error is one
of the view's own that names what was wrong. This follows `DStoaListScreen`'s
rule that a success with no items array is a failure. A reply the view cannot
place in a panel is not rendered in either. A blank `title` is included because
`stoa-metadata` says no successful reply carries one, so one that does is not a
reply the contract describes.

It lives on `Core` beside `capabilityFrom` and `identityFrom`, the other
normalisers, and so does `isBlankTitle`: the join screen needs the blank
test for the join reply too, and `Core` is the singleton every screen already
imports. A new singleton would have been one more `qmldir` entry for the name
gates to police, for one function.

**Walking UTF-16 code units is exact for this list.** All thirty blank code
points are in the Basic Multilingual Plane. A character outside it is a
surrogate pair, and no surrogate is in the list, so such a character is never
blank. That is the correct answer.

### 13. Creation passes a blank title to the core unchanged

The view has no title check at creation. The core is the only place titles are
checked, and a second check in the view would be a second implementation of
decision 1's list that could drift. The field sends what was typed, and the
refusal is rendered as the core wrote it.

### 14. A listed Stoa with a blank title still gets its row

The core no longer lists one: decision 6 makes such a listing a failure. If a
listing carries one anyway, the row still renders with its address and no
substitute title. A missing row would be a Stoa the user is in and cannot reach.
Only the comment claiming an empty title is legal has changed.

## Risks / Trade-offs

- **A stored blank-titled metadata op in a SQLite log fails every read of that
  Stoa** → reported to the spec-writer. `stoa-metadata` says such an op "never
  binds", which reads as "resolves to the fallback". But a SQLite log cannot
  hand the op to the resolver at all: `decode_entry` refuses it, so `getStoa`,
  the feed and the thread for that Stoa all answer `CorruptEntry`. No build
  publishes a metadata op before #125, so only a crafted peer could have
  delivered one before this change. This is a finding about the spec, not a
  choice made here.
- **Two copies of the blank list, one per language.** Each side's tests pin
  every member and the two non-members the spec names, so drift shows up as a
  red test on whichever side moved.
- **An extra lookup on a direct A-to-B rebinding** (decision 7). It cannot
  happen through the shipped navigator.

## Migration Plan

None. The wire shapes are unchanged; only which inputs are refused changes.
A development store holding a blank-titled genesis record answers every listing
with a failure until that row is removed. That is the owner's ruling.
