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
- **A Stoa title of `""` is invalid, everywhere a title enters or is
  answered** (owner ruling, given while this change was in progress). This
  reverses the contracts that made an empty title legal:
  - the genesis encoding refuses a record whose title is empty, on encode and
    on decode, so creation refuses an empty title and a join, a `getStoa` or
    any other call handed such a record refuses it as undecodable;
  - the op encoding refuses a metadata op whose title is empty, so one arriving
    from a peer is refused at the transport boundary, and resolution never lets
    one bind wherever a reader meets it;
  - no successful `getStoa` reply carries an empty `title`;
  - on the view, an empty `title` in a `getStoa` reply is a malformed reply, an
    empty founding title in a join reply is not an available founding title,
    an empty title is never a same-title (lookalike) match, and an empty title
    at creation is passed to the core and its refusal rendered.
  Whether a whitespace-only title also counts as empty is **not ruled**, and
  this change does not decide it.
- **A lookup that fails is a failure the screen renders, and not a fallback.**
  The core's reason is shown; no title is rendered from it; it is not reported
  as a refused join; and the join affordance is still offered, since the join
  is a separate call the core answers for itself.
- **A reply shape that is neither success nor the error shape is a failure.**
  A reply missing `isGenesisFallback` as a boolean, or `title` or
  `description` as a string, or carrying an empty `title`, is not rendered as
  a fallback or as a rename.
- **A lookup belongs to the reference it was made for.** A title answered for
  one reference is never rendered for the next one previewed.
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

None. The join preview's contract is `stoa-navigation-view`'s, and the empty
title ruling amends the capabilities that already own each title.

### Modified Capabilities

- `stoa-navigation-view`: ADDED "The preview asks the core what the Stoa is
  called, and renders the answer as what the reply says it is" (the lookup is
  made for the reference on screen, before any join and for no malformed input;
  a fallback `title` fills the founding position; a non-fallback `title` and
  its non-empty `description` fill the current position; a fallback is stated
  as "no moderator-set title held here"; a current title is never compared
  against held Stoas); ADDED "A lookup that fails, or answers in no
  recognisable shape, renders no title and withdraws no join" (the core's
  reason is rendered, the outcome is neither a fallback nor a refused join, the
  join affordance remains, and an empty `title` is one of the unrecognisable
  shapes); ADDED "A lookup's answer is rendered only for the reference it was
  made for". MODIFIED "A listed Stoa is rendered with its address, never with
  its title alone" (an empty founding title still gets a row, but is no longer
  described as legal); MODIFIED "Joining shows what is being joined, and joins
  nothing until the user acts" (an empty founding title, a join reply's
  included, is not an available founding title); MODIFIED "No current title is
  rendered until one has been resolved" (only its citation of `stoa-metadata`'s
  renamed resolution requirement changes); MODIFIED "A Stoa already held
  whose title matches is shown as a distinct Stoa, not as a duplicate" (an
  empty title matches nothing); MODIFIED "Creating a Stoa asks for a title and
  nothing else, and is always offered" (an empty title still reaches the core,
  whose refusal is rendered).
- `stoa-genesis`: MODIFIED "A tampered or truncated record is rejected" (an
  empty title is refused on encode and on decode, distinguishably).
- `stoa-membership`: REMOVED "A title the genesis record cannot carry is
  refused before a Stoa exists" and ADDED its replacement, "A title the genesis
  record cannot carry, an empty one included, is refused before a Stoa exists"
  (an empty title is refused, reversing the requirement that it be accepted; a
  MODIFIED block cannot drop the accepting scenario, so the requirement is
  replaced); MODIFIED "Joining takes an address and the record it names, and
  verifies rather than trusts" (a record with an empty title is refused without
  a membership).
- `op-format`: MODIFIED "A Stoa metadata op carries display fields and no
  policy" (a metadata op with an empty title is refused on encode and on
  decode; an empty description is still valid).
- `stoa-metadata`: REMOVED "Current metadata resolves by last-write-wins,
  falling back to genesis" and ADDED its replacement, "Current metadata
  resolves by last-write-wins among binding ops, falling back to genesis" (a
  metadata op carrying an empty title never binds, reversing "an empty title in
  a binding op is the current title"; replaced rather than modified for the
  same reason); MODIFIED "A Stoa's metadata is answerable from its address and
  genesis record, joined or not" (its citation of the replaced requirement
  follows the rename); MODIFIED "The reply carries the current metadata and
  says whether it fell back to genesis" (no successful reply carries an empty
  `title`); MODIFIED "`getStoa`
  refuses what it cannot answer, and never reports a fallback in place of a
  failure" (a record whose title is empty is refused).

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
  against fake `getStoa` replies — fallback, non-fallback, empty `title`,
  failure, malformed shape, and a second reference after the first — and the
  existing specs that pin an empty title as legal, which now pin the reverse.
- `dialectica-ui/src/qml/DStoaListScreen.qml`: the comment calling an empty
  title legal.
- **A core change**, where there was none before the ruling: the genesis
  encoder and decoder, the op encoder and decoder for the metadata kind, the
  metadata resolver, and the creation, join and `getStoa` paths in the wire
  layer, together with the core tests that pin an empty title as accepted. The
  wire shapes do not change; which inputs are refused does.
- **Stores already holding an empty-titled record or metadata op** decode
  differently once this lands. What a listing answers for such a retained
  record is left to the requirements that already govern a retained record the
  build cannot read, and is recorded as an open question rather than decided
  here.
