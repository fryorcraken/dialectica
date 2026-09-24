## Why

The join preview is the screen where a user decides whether to trust an address
handed to them, and it currently tells them nothing about what the Stoa is
called: it calls no core method before a join, so it renders "nothing here
knows what this Stoa is called" for every reference. Issue #98 (PR #144) added
`getStoa`, which answers a Stoa's title for an `(address, record)` pair this
peer has not joined, and restated `stoa-navigation-view` against it — so the
spec now obliges the preview to render a founding title that the view does not
fetch. Issue #143 (milestone 0.0.1) closes that gap.

## What Changes

- **The preview asks `getStoa` about the reference it is showing**, with the
  address and genesis record it already holds, as soon as it is shown and
  without the user acting. Nothing is joined by asking.
- **What the reply's `title` is rendered as is decided by `isGenesisFallback`,
  and by nothing else.** A fallback reply's `title` is the founding title and
  fills the founding-title position; a non-fallback reply's `title` is a
  current title and fills the current-title position, never the founding one.
  This is what `stoa-navigation-view` already requires of any `getStoa` reply;
  this change adds the call that produces one.
- **`isGenesisFallback` is surfaced, not only used to pick a panel.** A fallback
  reply establishes only that this peer holds no moderator-set title for the
  Stoa, not that the Stoa was never renamed; the preview says so, rather than
  letting the absence of a current title read as "not renamed".
- **The description is rendered with the current title, and only there.** A
  moderator sets it in the same op as a current title, a fallback reply always
  carries an empty one, and the bundle's preview has no description position.
  So it is shown only from a non-fallback reply, and only when non-empty.
- **A blank Stoa title is invalid, everywhere a title enters or is answered**
  (owner rulings, given while this change was in progress). A title is
  **blank** when it is `""` or is made only of whitespace or zero-width
  characters. `stoa-genesis` defines the term with an exact list of thirty
  code points — the twenty-five Unicode `White_Space` code points, and U+200B,
  U+200C, U+200D, U+2060 and U+FEFF — so that the core and the view test the
  same set and a test can pin every member. This reverses the contracts that
  made an empty title legal:
  - the genesis encoding refuses a record whose title is blank, on encode and
    on decode, so creation refuses a blank title and a join, a `getStoa` or
    any other call handed such a record refuses it as undecodable;
  - the op encoding refuses a metadata op whose title is blank, so one arriving
    from a peer is refused at the transport boundary, and resolution never lets
    one bind wherever a reader holds one as an op;
  - on decode, a blank title is the last refusal reported: an input that is
    also wrong in another way (trailing bytes, for one) is reported as that
    other fault, in both the genesis and the op decoder;
  - no successful `getStoa` reply carries a blank `title`;
  - on the view, a blank `title` in a `getStoa` reply is a malformed reply, a
    blank founding title in a join reply is not an available founding title,
    a blank title is never a same-title (lookalike) match, and a blank title
    at creation is passed to the core as typed and its refusal rendered.
  Only a title made **entirely** of those characters is refused. A title that
  carries them beside at least one other character — at its edges or inside
  it — is neither refused for them nor trimmed or normalised. A character
  outside the list is not blank, even one that renders invisibly, such as a
  bidirectional control.
- **A store that already holds a blank-titled genesis record is neither
  migrated nor skipped** (owner ruling). The record fails to decode, and a
  listing that reaches it reports the failure in the module's failure shape,
  leaving the record as it was. Before 0.1.0 only development stores can hold
  one.
- **A persistent op store that already holds a blank-titled metadata op is
  treated the same way.** Its stored bytes no longer decode, so every read that
  would return it fails, reporting the decoder's reason; it is not skipped as a
  non-binding op, and nothing migrates or removes it. `getStoa` for that Stoa
  answers the error shape rather than a fallback. Before this change a peer
  could have received one only from a crafted peer, since no build publishes
  metadata ops before #125.
- **A persistent op store refuses to append an op the op encoding refuses to
  encode**, storing nothing, so it cannot write the row just described. An
  in-memory log, which holds ops as values and decodes nothing on read, stores
  such an op as it stores any other.
- **A lookup that fails is a failure the screen renders, and not a fallback.**
  The core's reason is shown; no title is rendered from it; it is not reported
  as a refused join; and the join affordance is still offered, since the join
  is a separate call the core answers for itself.
- **A reply shape that is neither success nor the error shape is a failure.**
  A reply missing `isGenesisFallback` as a boolean, or `title` or
  `description` as a string, or carrying a blank `title`, is not rendered as
  a fallback or as a rename.
- **A lookup belongs to the reference it was made for.** A title answered for
  one reference is never rendered for the next one previewed.
- **A join reply's founding title takes the founding position from a fallback
  reply's.** Where both carry one, the join's is rendered; the fallback's fills
  the position only while no successful join reply carries one that is not
  blank.
- **Out of scope, and settled by the owner on the issue:** the same-title
  (lookalike) comparison is not extended to current titles. A Stoa renamed to
  match one the user holds reports `isGenesisFallback: false`, so no founding
  title is available and the comparison does not run; that is accepted at this
  stage. This change pins that the comparison does not run against a current
  title rather than leaving it to be inferred.
- **Also out of scope:** display-point mitigation of bidirectional and
  zero-width characters in titles (`stoa-metadata`'s "A displayed title is
  never an identifier" places it on the renderer; the preview's founding title
  does not do it today either), and publishing a metadata op (issue #125). Until
  #125 lands this build publishes none, so in practice every lookup falls back
  unless an op arrives from elsewhere.

## Capabilities

### New Capabilities

None. The join preview's contract is `stoa-navigation-view`'s, and the blank
title ruling amends the capabilities that already own each title.

### Modified Capabilities

- `stoa-navigation-view`: ADDED "The preview asks the core what the Stoa is
  called, and renders the answer as what the reply says it is" (the lookup is
  made for the reference on screen, before any join and for no malformed input;
  a fallback `title` fills the founding position; a non-fallback `title` and
  its non-empty `description` fill the current position, and a fallback
  `title` gives way to a join reply's founding title; a fallback is stated
  as "no moderator-set title held here"; a current title is never compared
  against held Stoas); ADDED "A lookup that fails, or answers in no
  recognisable shape, renders no title and withdraws no join" (the core's
  reason is rendered, the outcome is neither a fallback nor a refused join, the
  join affordance remains, and an empty `title` is one of the unrecognisable
  shapes); ADDED "A lookup's answer is rendered only for the reference it was
  made for". MODIFIED "A listed Stoa is rendered with its address, never with
  its title alone" (a blank founding title still gets a row, but is no longer
  described as legal); MODIFIED "Joining shows what is being joined, and joins
  nothing until the user acts" (a blank founding title, a join reply's
  included, is not an available founding title; where a successful join reply
  and a fallback reply both carry a founding title, the join reply's is the one
  rendered); MODIFIED "No current title is
  rendered until one has been resolved" (only its citation of `stoa-metadata`'s
  renamed resolution requirement changes); MODIFIED "A Stoa already held
  whose title matches is shown as a distinct Stoa, not as a duplicate" (a
  blank title matches nothing, and titles that are not blank are compared
  untrimmed); MODIFIED "Creating a Stoa asks for a title and nothing else, and
  is always offered" (a blank title still reaches the core as typed, and its
  refusal is rendered).
- `stoa-genesis`: ADDED "A blank title is not a valid title" (defines blank by
  an exact list of thirty code points; a blank title, the empty one included,
  is refused on encode and on decode with one failure; a title with any other
  character is neither refused for its blank characters nor altered; on decode
  a blank title is the last refusal reported, so an input also carrying
  trailing bytes is refused as trailing bytes); MODIFIED "A tampered or
  truncated record is rejected" (a blank title joins the distinguishable
  decoding refusals).
- `stoa-membership`: REMOVED "A title the genesis record cannot carry is
  refused before a Stoa exists" and ADDED its replacement, "A title the genesis
  record cannot carry, a blank one included, is refused before a Stoa exists"
  (a blank title is refused, reversing the requirement that an empty one be
  accepted; a MODIFIED block cannot drop the accepting scenario, so the
  requirement is replaced); MODIFIED "Joining takes an address and the record
  it names, and verifies rather than trusts" (a record with a blank title is
  refused without a membership); MODIFIED "A joined Stoa's genesis record is
  retained, not only its address" (a retained record that fails to decode, a
  blank-titled one included, is reported as a failure and is neither skipped
  nor migrated).
- `op-format`: MODIFIED "A Stoa metadata op carries display fields and no
  policy" (a metadata op with a blank title is refused on encode and on
  decode, and on decode it is the last refusal reported; an empty or blank
  description is still valid); MODIFIED "Valid text
  is never normalised or otherwise transformed" (its preservation scenario is
  narrowed, for a metadata op's title, to one carrying at least one character
  that is not blank, which is the ruling applied).
- `stoa-metadata`: REMOVED "Current metadata resolves by last-write-wins,
  falling back to genesis" and ADDED its replacement, "Current metadata
  resolves by last-write-wins among binding ops, falling back to genesis" (a
  metadata op carrying a blank title never binds, reversing "an empty title in
  a binding op is the current title", and one stored as bytes before the
  refusal is not held as an op at all; replaced rather than modified for the
  same reason); MODIFIED "A displayed title is never an identifier" (its
  preservation scenario is narrowed to a title carrying at least one character
  that is not blank); MODIFIED "A Stoa's metadata is answerable from its address and
  genesis record, joined or not" (its citation of the replaced requirement
  follows the rename); MODIFIED "The reply carries the current metadata and
  says whether it fell back to genesis" (no successful reply carries a blank
  `title`); MODIFIED "`getStoa`
  refuses what it cannot answer, and never reports a fallback in place of a
  failure" (a record whose title is blank is refused, and a stored entry that
  does not decode, a blank-titled metadata op stored before the refusal
  included, is the error shape rather than a fallback).
- `op-log`: MODIFIED "The log records what arrived, and decides nothing about
  it" (a log that keeps ops as their encoding refuses to append an op the op
  encoding refuses to encode, and stores nothing; a log that keeps ops as
  values stores it); ADDED "A stored entry that does not decode fails every
  read that would return it" (not skipped, not reported as absent, not
  repaired or migrated, a blank-titled metadata op stored before the refusal
  included).

## Impact

- `dialectica-ui/src/qml/Core.qml`: one new wrapper naming `get_stoa`, the only
  place the method string appears.
- `dialectica-ui/src/qml/DJoinScreen.qml`: the lookup, its outcome tied to the
  reference as the join outcome already is, the two title panels driven from
  it, the fallback statement, the description, and the no-title note's copy —
  which currently says nothing knows what the Stoa is called and that only
  joining asks, both false once a lookup answers.
- `dialectica-ui/src/qml/Main.qml` if the preview's entry point is where the
  lookup is triggered.
- `dialectica-ui/tests/tst_stoa_screens.qml`: QML specs driving the preview
  against fake `getStoa` replies — fallback, non-fallback, blank `title`,
  failure, malformed shape, and a second reference after the first — and the
  existing specs that pin an empty title as legal, which now pin the reverse.
- The view needs the blank-character list too, for the lookup reply, the join
  reply, the listing row and the lookalike comparison; it is the same thirty
  code points the core tests.
- `dialectica-ui/src/qml/DStoaListScreen.qml`: the comment calling an empty
  title legal.
- **A core change**, where there was none before the ruling: the genesis
  encoder and decoder, the op encoder and decoder for the metadata kind, the
  metadata resolver, the persistent op log's append and the publish path, and
  the creation, join and `getStoa` paths in the wire layer, together with the core tests that pin an empty title as accepted. The
  wire shapes do not change; which inputs are refused does.
- **Stores already holding a blank-titled genesis record** decode differently
  once this lands, and that is decided rather than open: the record fails to
  decode, the Stoa listing reports the failure, and nothing skips or migrates
  it. A development store holding one therefore answers every listing with
  that failure. **A persistent op store holding a blank-titled metadata op**
  is decided the same way: every read that would return it fails, so
  `getStoa`, the feed and the thread for that Stoa answer the error shape until
  the row is removed by hand. It is not skipped as a non-binding op.
