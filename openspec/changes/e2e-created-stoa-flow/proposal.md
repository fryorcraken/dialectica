# Drive a created Stoa end to end: key, Stoa, feed, thread and moderation

## Why

#134 stays open until the end-to-end suite covers a successful join, the feed
and the thread, run against an `lgs`-installed Basecamp (owner, 2026-09-25:
splitting the remainder to 0.0.2 was considered and rejected). The two archived
suite changes, `e2e-ui-suite` and `e2e-suite-review`, list five follow-ups, and
the owner's instruction for this piece is to handle all five: a successful join,
the feed screen, the thread screen, the moderation screen, and key creation then
Stoa creation.

Reading those five against `main` changes what they cost. Four of them need no
seeder at all, because a Stoa the profile creates itself can be opened. Since
`genesis-in-replies` (#130), the creation reply and every listing item carry the
genesis record, and the view keeps it. So a fresh profile can create its
machine key, create a Stoa, open that Stoa's feed with its record, post, open the
post's thread, reply, and reach the moderation screen from the feed. That one
route covers four of the five. The fifth, a successful join, is the only one that
needs something from outside the profile.

## What Changes

### This piece: four specifications on the created-Stoa route

Four sitometres specifications, each run the way `join.yaml` runs, in
`ui-tests.yml` against a profile `lgs basecamp install` populated. Each is listed
below by what it must observe. The steps, and whether the four are four files or
fewer, longer ones, are for design.md to decide.

1. **Key creation, then Stoa creation, from a fresh profile.** The list opens in
   the no-key state. Acting on the create-key action reaches the key-held state,
   where the create affordance is present and the key block is absent. Creating
   a Stoa with a title renders the address the core returned. The listing, read
   again, holds that one Stoa, and a share is offered for its row.
2. **The feed screen.** Opening the created Stoa's row renders its feed. The
   feed read succeeded and holds nothing, as opposed to failing. With the key
   held, the posting gate is open. Publishing a post states that it was saved on
   this machine, and the feed, read again, holds one row. The way back renders
   the list.
3. **The thread screen.** Acting on that row opens its thread. The read
   succeeded and holds the root with no replies. A reply submitted there leaves
   the thread, read again, holding the root and the reply. The way back renders
   the feed.
4. **The moderation screen**, which shipped inert (#127). The feed's moderation
   affordance renders the moderation screen. The screen states that its controls
   publish nothing. Acting on one of them leaves the way out offered, and the
   way out renders the feed.

**Each spec must be seen failing in CI before it is believed.** `e2e-suite-review`
set that standard: a deliberate break pushed alone, its red run read for the
predicted reason, reverted, and both runs recorded. It applies to every spec
this piece adds. A step asserting an absence also needs a presence in the same
snapshot, as `join.yaml`'s no-key step has (`e2e-ui-suite` design.md D5), or it
passes against a screen that never rendered.

**Specification 2 is the reproduction #152 asks for.** #152 reports "genesis
record ended mid-field" on opening a Stoa, intermittently, and its first step is
"reproduce against a Stoa created fresh on current `main` … open it
immediately". Specification 2 does exactly that on every run. It also closes
`genesis-in-replies`' unticked 5.4, "create a Stoa, press Open, see a feed",
which no agent could click. If the feed step goes red for that reason, the
finding goes on #152. It is not a defect in the specification, and the step is
not weakened to pass.

### The second piece: a successful join, with its seeder

A successful join is proposed as a separate, later piece. Its PR carries
`Closes #134`. What the seeder needs is settled here, and how it is built is for
that piece's `design.md`.

**What the seeder needs to supply is a Stoa reference that verifies.** Nothing
more is needed. That means a reference in the encoding `stoa-navigation-view`
fixes (`{"stoa": …, "genesis": …}`), with these properties:

- its record decodes under the current genesis encoding: a known version, a
  32-byte creator key, a policy with a name (`open`), and a title that is not
  blank;
- its address is that record's hash;
- it names a Stoa the profile under test does not already hold, so that the join
  is a new membership and not the idempotent case;
- its founding title is one the spec can recognise, and is distinct from
  anything else the run creates.

**It is a core-level fixture, not a second Basecamp profile.** There are three
reasons, each checkable:

- **A join consults nothing but the two inputs.** `stoa-membership`'s
  "Verification consults nothing but the two inputs" says so. The preview's
  `getStoa` lookup, on a peer holding no ops for the Stoa, answers the record's
  own founding title as a fallback (`stoa-metadata`, "A peer that has never
  stored an op is answered with a fallback"). A live peer contributes nothing
  either call checks.
- **A second peer could not supply content either.** Ops never leave the peer
  that authored them (#176). The publish sink logs "delivery is not wired yet"
  and sends nothing. A seeding Basecamp that posted would therefore add nothing
  to what the joining profile receives.
- **sitometres drives one app.** A second Basecamp in the same job would be a
  second host that no spec step can observe.

This departs from the issue's wording, which says a successful join "needs a
seeding peer". The departure is deliberate, and the owner can overrule it. A
second profile under `lgs basecamp launch` becomes the right seeder once #176
lands and receipt from another peer is what is under test. That is #102's item 7,
and it is not on #134's list.

The `seed_store` example (`dialectica/rust-lib/dialectica-core/examples/seed_store.rs`) already
prints an address and its record. A creation reply from a throwaway core carries
both too. The second piece's design chooses between these, between a committed
reference and one generated per run, and how a generated one reaches a static
spec.

**Two requirements that spec will need, found while checking it against
`openspec/specs/`.** They are recorded here so the next `spec-writer` starts from
them rather than rediscovering them. Neither is added by this change.

- **The positive half of "A join is reported from the core's reply, never
  assumed".** That requirement says a join is reported **only** on a successful
  reply. It has a scenario reporting success for a Stoa already held, and one
  after a refused lookup. It has none for the ordinary case: a fallback lookup,
  then a successful join of a Stoa not held.
- **A joined Stoa reaches the list without a restart.** `view-navigation`
  contracts this for a created Stoa ("A created Stoa reaches the list without a
  restart"). For a joined one, "A Stoa joined in this session can be shared"
  presupposes the row but does not require it.

### Why two pieces and not one

- **The order is forced.** The join spec ends on the joined Stoa's feed, and
  needs the feed handles and the feed requirement that this piece adds.
- **The open design question is all in the second piece.** It is how a
  verifiable reference reaches a static spec. This piece has no open question
  about mechanism, only about layout.
- **Every spec needs its own red run in CI, waited out.** Both workflows cancel
  in progress per ref. Four proofs is already a long pass, and a fifth that
  depends on a new mechanism is the one most likely to need a second attempt.

The split is for reviewability. All five are handled, and none is left as an
issue to file.

This piece's PR carries **`Part of #134`**. The owner's comment on #134 names a
successful join as still required in 0.0.1, and this piece does not deliver one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `stoa-navigation-view`: "What is shared carries the founding record, not the
  address alone" is replaced by "What is shared carries the founding record, and
  a Stoa is shareable wherever a reply carried it", which describes what `main`
  does. A created Stoa and a listed one are shareable from the record their
  reply carried, and the screen is no longer required to account for a missing
  share. It is a REMOVED and an ADDED rather than a MODIFIED because
  `openspec validate` refuses a MODIFIED block that drops a scenario the live
  spec has, and "A Stoa just created offers no share" cannot be kept under its
  own name with the opposite content. "Joining shows what is being joined, and
  joins nothing until the user acts" is MODIFIED only to cite the new name. Its
  prose cited the old one, and nothing else in it changes.
- `feed-view`: a requirement is added. A feed that holds nothing and a feed that
  could not be read are different screens. The capability's Purpose is widened
  to cover it.

Most of this piece observes contracted behaviour against a running host. The two
deltas exist because specifications 1 and 2 walk a route the spec tree either
contradicts or does not contract.

**The contradiction.** `stoa-navigation-view` says a Stoa just created "cannot be
shared at all", because "neither the listing reply nor the creation reply carries
a genesis record". It has a scenario, "A Stoa just created offers no share".
Neither has been true since #130. `stoa-membership`'s "A reply naming a Stoa
carries the record that Stoa's address is the hash of" puts the record on both
replies. The view keeps it, and `tst_stoa_screens.qml`'s
`test_a_created_stoa_is_openable_from_the_creation_reply_alone` asserts
`canShare` is `true` for it. The same requirement also requires the screen to
"account for the absence somewhere the user can read it". That account was the
list's `ON SHARING` marginal note, which #70 removed with the annotation column
on the owner's decision (`drop-apparatus`, design.md, "four notes, all rendered
nowhere else"). No test pins it, and nothing renders it. `genesis-in-replies`
changed the core and the view, and its only delta was to `stoa-membership`, so
the view-side requirement kept its old text. Specification 2 opens a created
Stoa's feed, and `view-navigation`'s "The feed is rendered for a Stoa chosen from
the list" gives the feed a record only where the view holds one. As the spec
stands, the view holds none for a created Stoa, so the step would assert the
opposite of a live requirement.

**The gap.** Specification 2's central observation is that the feed read
succeeded and holds nothing, and did not fail. No view requirement says so. The
empty-versus-unreadable pair for the feed is named as "the existing feed
screen's" in `stoa-navigation-view`'s Purpose, and as "this view's governing
rule" in `composer-view`'s prose. `feed-read` contracts the core side, "A feed
read that fails is the error shape, and never an empty feed". `FeedScreen.qml`
cites `SPEC.md` for it, which is not a file this repository tracks. The list screen's
equivalent is a requirement ("Holding no Stoas and failing to read membership are
different screens"), and the added requirement follows its shape. `feed-view` is
the feed screen's capability, so the requirement goes there.

**Contracts that live outside `openspec/specs/`.** Specification 3 rests on
`thread-view` and specification 4 on `moderation-view`. Each exists only in an
unarchived change folder, `openspec/changes/ui-thread-view/` and
`openspec/changes/moderation-screen/`. Their code merged in #122 and #127, and
neither change was archived: `openspec list` shows both in flight, with review
and closer rows unticked. So a step asserting what the thread screen renders
cites a requirement that is in the tree but not in the live contract. This piece
does not archive them, because each change's stage block belongs to that change's
own closer. The spec-test review should check those steps against the
change-folder text. The runner should raise the missing archives with the owner,
the same way #161 did for `first-run-identity`. The routes into and out of the
thread are `view-navigation`'s and are live.

Every other observation above is already a requirement:

- the no-key state, the mint, the key-held state, the creation outcome and the
  created Stoa's address: `stoa-navigation-view`;
- the created Stoa among the rows after creation, the feed opened from a row with
  its record, the returns from the feed and from the thread, and the thread
  opened from a row: `view-navigation`;
- the posting gate, the stored-not-delivered success, and content appearing only
  once core reports it: `composer-view`.

## Impact

- **New files:** the new specifications under `dialectica-ui/tests/ui/`.
- **Modified:** `.github/workflows/ui-tests.yml`. Its matrix gains the new
  specs, and each job's count check follows it. `dialectica-ui/src/qml/Main.qml`
  gains read-only root handles for the feed, the thread and the moderation
  screen, each a projection of state that screen already owns and each pinned in
  `tst_e2e_handles.qml` (the `e2e-ui-suite` change's design.md D6). The view
  gains `objectName`s on controls the specs drive and that have none today, such
  as the list row's Open action and the composers' submit.
- **Spec:** the deltas above, and `feed-view`'s Purpose line edited in
  `openspec/specs/feed-view/spec.md`. OpenSpec ignores a Purpose in a delta for
  an existing capability, so this is the one place it can change. No view code
  changes for either delta. The view already meets both, and `tester` pins each
  scenario that is not pinned already.
- **CI cost:** each spec that is its own file is another `ui-tests.yml` matrix
  job, about 3 minutes warm and 9 minutes cold (`e2e-ui-suite` design.md D7). Any
  spec after the first repeats the key-and-Stoa prefix, because each job starts
  from a fresh profile. Four files or one chained file is design.md's call, with
  that cost in view.
- **Known limit, carried from the `e2e-ui-suite` change's Risks:** sitometres
  assigns a field's `text` rather than typing into it. `createTitleField` reacts
  through `onTextChanged`, and `DComposer`'s draft aliases its field's `text`, so
  an assignment reaches both. A field that listened only to `onTextEdited` would
  not see it.
- **Not affected:** `dialectica/` core, the Rust tree, the wire contract, and
  every other requirement in `openspec/specs/`.
