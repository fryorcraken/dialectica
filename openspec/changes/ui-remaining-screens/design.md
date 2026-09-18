# Design — closing the visual gap to the design bundle

## Context

The piece was briefed as "build the screens that do not exist yet", with the
owner's ruling that data may be mocked and controls may be dead so long as
`docs/PLAN.md` records it. Measuring the tree first changed the shape of the
work substantially, and the measurement is the reason the decisions below look
different from the brief:

- `FeedScreen.qml` already carried vote arrows, a composer, the posting gate and
  pagination.
- `PostHeader.qml` already accepted `vouched` and `isModerator`.
- `FlatButton.qml` already defined `destructive`, `destructive-outline` and
  `secondary-micro` — added in anticipation of a moderation screen.
- `DVouchStamp`, `DIdentityChip` and `DStatusBar` were complete, tested, and
  instantiated by **no screen at all**.

So the gap was not "screens that do not exist"; it was **components that exist
and render nowhere**.

**That gap is now closed by `piece/ui-navigation` rather than by this piece**,
and D1 below records why this branch withdrew its own answer to it on rebase.
What survives here is the Stoa list's row treatment (D5) and the `rowTitle`
token it needs, with the spec delta that contracts half of it (D5b) and the tests
that pin it (D5c).

## Decisions

### D1 — The shared footer was withdrawn: navigation delivered the same job better

**This change built a `DScreenFooter` — pagination, an opt-in `DIdentityChip`,
and `DStatusBar`'s three lamps — and mounted it on `FeedScreen`. On rebasing
onto `origin/main` it was deleted, along with `tst_screen_footer.qml`.** It is
recorded rather than quietly dropped, because "this piece shipped no footer"
and "this piece never tried" look identical from the diff, and the reasoning is
the part worth carrying.

`piece/ui-navigation` (#123) merged first and mounted the same two components,
arranged differently and on **two counts strictly better**:

- **The chip binds the right probe.** Navigation's `FeedScreen` probes
  `who_am_i` every render, derives `hasIdentity` from it, and binds the chip to
  that. This piece's footer could only offer the chip behind an opt-in
  (`showIdentity`, default false) that no screen took up, precisely because the
  feed then held no `who_am_i` answer to bind. Navigation *has* the answer, so
  its chip renders where this one could not. See D1b for why the probe choice
  is a correctness boundary rather than a preference.
- **The lamps are on every screen, not one.** Navigation mounts one
  `DStatusBar` in `Main.qml`, declared outside every screen's `visible:`
  binding, so no state can withhold it. Its comment carries the argument this
  piece did not reach: a status indicator absent from a screen is one whose
  absence a user reads as "nothing to report", and a per-screen subset makes
  that absence ambiguous. A footer-per-screen is the arrangement that produces
  the subset.

**What ruled out keeping both:** mounting `DScreenFooter` on `FeedScreen` on
top of navigation's `Main.qml` bar puts **two sets of three lamps on screen at
once**, able to disagree — `Main.qml` binds `zoneState` from the membership
listing, and the footer left it at `degraded`. Two renderings of one machine
state is the "second source for one value" failure that `Main.qml`'s own
navigator properties were shaped to make unconstructible; reintroducing it in
the chrome would be that mistake one level out.

**What was lost, stated because it is a real cost and not nothing:** the
footer bound `storageText` to core's own failure words plus the row count
("Storage: the store is unreadable", "…3 posts held"). Navigation's bar leaves
all six tooltips unset on the ground that "no explanation is shown and none is
wrong". That is a strictly smaller claim, and a tooltip carrying core's verbatim
text is a real improvement over none — but it is an improvement to
`Main.qml`'s bar, reachable in one property binding, and not a reason to keep a
second bar alive to host it.

**What breaks without this decision:** nothing turns red, which is exactly why
it is written down. Both arrangements pass every gate in the tree; only reading
the two diffs together shows the duplicate lamps.

### D1b — The chip must bind `who_am_i`, never the posting probe

**This is the piece's finding, and it survives the withdrawal** — navigation's
`FeedScreen` now carries the same argument in its own comments, so the claim is
live in the tree even though this branch's code for it is gone.

The chip must be bound to `who_am_i`, **not** to the posting probe — verified
against `who_am_i`'s own doc comment on the trait in
`dialectica/rust-lib/src/lib.rs`, which calls it "**A different question from
`getCapabilities`**, and the two can honestly disagree: a stored identity whose
keystore permissions are too open is a real identity that cannot currently be
used. A view with only the posting probe would have to render 'you are nobody'
to a user who has an identity and a fixable problem."

*Cited by method name rather than by line.* An earlier draft of this line said
`lib.rs:258`, which was wrong when written — 258 is blank, and the sentence is
in `who_am_i`'s doc comment some thirty lines below `generate_identity_slate`,
where that number lands. The wrong number is inherited: it entered at
`2026-09-18-ui-navigation/proposal.md:48` and was copied from there into
`DIdentityChip.qml`, `DThreadScreen.qml` and `FeedScreen.qml`, which is four
copies of one miscount rather than four people counting. Those three are
byte-identical to `origin/main` and out of this piece's scope; this file's copy
was this piece's own prose, so it is fixed here. `grep -n "honestly disagree"`
does not find the sentence either — it wraps mid-phrase across two lines, which
is how the original miscount survived being checked. `who_am_i` is unique in the
trait and survives the file growing, which a line number does not.

The consequence is sharp: the chip's no-identity arm offers identity
**creation**, and `keep_identity` refuses where an identity already exists. A
chip bound to `canPost` therefore shows "create an identity" to someone who has
one — the single irreversible wrong answer available.

**This change originally bound it to `canPost === true` and shipped that
defect.** It was caught by reading `piece/ui-navigation`'s concurrent diff, not
by any gate here. The fix at the time was the `showIdentity` opt-in; the fix
that survives is navigation's, which probes `who_am_i` directly.

### D2 — Two of the status bar's three lamps have no source, and render orange

**Superseded in placement, upheld in substance.** `docs/PLAN.md` §9.2 case 2
entries 6 and 7 now carry this, written by navigation and covering it more
accurately than this piece's draft did: **zone turned out to have a source
after all** — the membership listing answers it — which this branch's PLAN.md
entry wrongly claimed it did not. Delivery remains genuinely unbindable
(`DStatusBar.qml` records that the outcome arrives asynchronously through
delivery's channel events, after the publish call has returned).

This branch's duplicate PLAN.md entry was dropped on rebase rather than merged,
because it would have contradicted entry 7 on zone. Keeping a second copy is
how the wrong one gets read.

**Considered and rejected, and still worth not doing:** binding delivery to
"the feed read succeeded". It is the obvious way to make three green lamps, and
it is a claim about *peers* derived from a fact about a *local file*.

### D3 — The vouch stamp was mounted on the feed, and then removed

**This is a scope error this change made and corrected**, recorded rather than
quietly reverted because the reasoning that caught it is the useful part.

A `DVouchStamp` was mounted in the feed's attribution row, with a session
`ownVouches` map keyed by author and a hover reveal threaded from the row. It
worked and it was tested. It was **wrong on scope**, and the argument is
`PLAN.md` §7.3's staging section: "Vouching is an extension, **scheduled after**
§7.2's interim ordering ships" — and §7.2, voting, is out of the MVP by owner
ruling 1. A vouch affordance on a feed row therefore ships the second half of a
feature whose first half was deliberately excluded.

**How it was caught, which is the part worth carrying:** by reading
`piece/ui-navigation`'s concurrent diff rather than by any gate. That piece was
adding a reachability check over `qmldir` in which every registered type is
either reachable from `Main.qml` or carries an `# UNINSTANTIATED:` line saying
why — and its line for `DVouchStamp` cites ruling 1. Two pieces had reached
opposite conclusions about the same component, and nothing either of them ran
would have reported it.

That check is now `dialectica-ui/tests/check_qml_reachable.py` on `main`, and
its `# UNINSTANTIATED:` record for `DVouchStamp` is the one this branch's
rebase kept — this piece's near-identical duplicate was dropped in favour of
it, since two records at one registration is the same second-copy problem.

**The reasoning that made it an error is the same one that kept the moderation
screen out** (D7): a scope ruling is not satisfied by the affordance being
cheap to build. The vouch stamp needs no core call at all, which is exactly what
made it feel shippable and is irrelevant to whether it is in scope.

### D5 — The Stoa list gets the row treatment but no identity chip

**This is what the piece actually ships.**

**Chosen:** the reference's row separators under every row — the last one
included, unlike the moderation lists which drop it on the last — and a 19px
serif title on the new `DTheme.rowTitle` token. A `ColumnLayout` per row rather
than a bare `RowLayout` is what gives the separator somewhere to live.

**Why a new token rather than reusing `heading` or `body`:** a row title is the
thing being chosen between, so it outweighs the prose around it without
competing with the screen's own heading. 19px sits between `heading` (25) and
`body` (15) deliberately.

**No footer here, and it is a requirement rather than an omission.** A
`DScreenFooter` was mounted on this screen and removed after
`tst_stoa_screens.qml`'s
`test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity`
failed on it. That test enforces `stoa-navigation-view` R13: one key signs in
every Stoa in this release, so anything on a per-Stoa screen that raises
identity offers an **unlinkability property the software does not have**. The
chip's no-identity arm renders "Voting, posting and replying need an identity",
which trips it.

**This does not conflict with the key block `main` has since added to this
screen.** That block mints *this machine's* key and its strings deliberately
avoid the word "identity" — which is the same distinction R13 draws, arrived at
from the other direction. A chip claiming a per-Stoa identity is what R13
forbids; naming the one key the machine signs everything with is not.

**Recorded in `DStoaListScreen.qml` itself**, because re-adding a footer is the
obvious next idea and no gate catches it by inspection — only that one test
does. The comment was rewritten on rebase: its original form said "that is why
`FeedScreen` has a footer and this screen does not", which stopped being true
when the footer was withdrawn.

### D5b — The delta contracts the row BOUNDARY and declines to contract the type

**The spec stage never ran for this piece, and `openspec validate --strict`
failed for want of a delta. Rather than declaring `skip_specs: true`, this change
adds one requirement to `stoa-navigation-view` — and deliberately covers only
half of what the code does.**

**What earns a requirement: the separator.** It is not decoration. The list can
hold two Stoas whose founding titles are byte-identical — the capability already
contracts that case, and `test_two_stoas_with_the_same_title_render_differently`
exists for it — and a row stacks a title over an address with buttons beside
them. Where one row's group ends and the next begins is therefore the same
concern as "a listed Stoa is rendered with its address, never with its title
alone", read one level out: that requirement makes the row carry the half an
attacker does not control, and this one makes it unambiguous **which row that
half belongs to**. A reader who attributes one row's address to the next row's
title has been misled about which Stoa they are opening, and the content cannot
correct them. Whitespace alone does not carry the division, because the gap
between two rows and the gap between a title and its own address are both gap.

**What does NOT earn one: the 19px title.** `stoa-navigation-view`'s Purpose puts
the visual system outside itself **by name** — "colours, type, metrics, the mark,
the 8-8-6 address abbreviation … requirements below say which component owns a
rendering decision and never restate what it renders". A requirement pinning the
row title's size would contradict the capability's own scoping sentence. So the
delta says a boundary is rendered per row and stops; it fixes neither the
boundary's thickness nor its colour nor the element that draws it, and says so in
its own text, so a later treatment that divides rows differently still satisfies
it.

**Considered and rejected: `skip_specs: true`.** It was the cheaper answer and it
would have been a false one. The marker means "this change modifies no specs
(pure refactor, tooling, docs)", and a row boundary that resolves which Stoa a
rendered address belongs to is observable behaviour on a screen whose whole
capability is about not misattributing peer-supplied strings. The previous agent
declined to paper over the gap for that reason and was right.

**What breaks without the requirement:** nothing red — which is the point. The
separator survived a rebase that deleted everything else this piece built, and
without a requirement naming it, the next person to touch this delegate has
`tst_stoa_screens.qml` telling them the count must hold and nothing telling them
why the last row is included.

### D5c — The row treatment is tested by COUNT, and the type by RELATION

**Answering "nothing tests the row treatment this piece ships", which `tasks.md`
left as an unticked row.**

*An earlier draft of this line counted the lines instead. It is written as the
behaviour now, deliberately: the raw diff of `DStoaListScreen.qml` is far larger
than any count of the behavioural change, because the reindent in `d59b479`
touches nearly every line of the delegate without altering one — `git diff -w`
is what separates the two. A figure quoted here would have sent a future reader
to `git diff origin/main...HEAD -- dialectica-ui/src/qml/DStoaListScreen.qml`,
found them a much bigger number, and left them checking whether they had the
right commit. What the tests below must cover is the separator and the row
title, and that does not change with the line count.*

**`tst_render_probe.qml` does not cover it.** It probes `DStoaListScreen`
(`test_a_populated_stoa_list_screen_paints_content`), but its assertion is that
the content area is not a single flat colour. A screen with no separator at all
paints a title, an address, an identicon and two buttons, so it passes that probe
unchanged. The probe is a floor against a screen that never reached the scene
graph, and its own spec says so; it cannot see this.

**Four separator tests, because one cannot distinguish the failures.** The
assertions are on the **count of boundaries against the count of rows**, never on
how one is drawn — matching what the delta contracts and what the capability
declines to contract:

- `test_every_rendered_row_is_separated_from_the_next` — three rows, three
  boundaries. Three rather than one deliberately: a screen drawing a single rule
  for the whole list satisfies any assertion phrased as "a separator exists".
- `test_one_row_draws_exactly_one_boundary` — the smallest non-empty list, which
  is what the drop-it-on-the-last-row convention reduces to zero.
- `test_the_row_count_and_the_separator_count_move_together` — one row and four,
  asserting the difference is three. This is the relation, established by varying
  the input rather than by trusting one fixture.
- `test_an_empty_list_draws_no_row_boundary` — the other direction, and **not a
  formality**. A separator hoisted to the list container renders on the empty
  state, where it is a rule belonging to a row that is not there. The count tests
  above are blind to that, because none instantiates an empty list.

**Each was proved able to fail, by two mutations that are the two real defects**
(measured on the split tests, 82 baseline):

| Mutation | What fails |
|---|---|
| `visible: index < visibleRows.length - 1` — the drop-it-on-the-last-row convention the requirement rules out | **79 passed, 3 failed** — 3 rows → 2 found, 1 row → 0 found, 4 rows → 3 found. The empty-list test correctly stays green |
| the separator hoisted out of the delegate to the list container | **79 passed, 3 failed** — 3 rows → 1 found, 4 rows → 1 found, and the empty list → 1 found. `test_one_row_draws_exactly_one_boundary` correctly stays green: a hoisted rule renders exactly one, which a one-row list cannot tell from a correct delegate |

Those last two sentences are the argument for the shape of the set. The empty-list
test is the only one that catches the hoist's *second* symptom, and the one-row
test is the only one that isolates the drop-last defect at its minimum — neither
is redundant with the others, and each stays green under exactly one mutation,
which is how a set of four tests says four different things.

**Why the relation test re-measures the one-row count rather than reusing the
constant.** `test_the_row_count_and_the_separator_count_move_together` builds its
own one-row screen even though `test_one_row_draws_exactly_one_boundary` already
pins that number. Asserting `afterFour - afterOne == 3` against a hardcoded `1`
would reduce the relation to a second assertion about the four-row fixture, and
two fixtures separately tuned to two literals are exactly what a relation is
supposed to rule out. Keeping the row counts different (1 and 4, neither of them
3) is the other half of that: a hoisted separator renders a fixed number and can
satisfy any single fixture whose row count it happens to equal, but not two that
disagree.

### D5d — Each separator test owns one screen's whole lifetime

`separatorsForRowCount(replies, expectedRows)` creates a screen, asserts its row
count, reads the separator count and destroys it before returning, so no two
component trees are ever live at once.

The earlier single-function form read a count from one screen, called `destroy()`
on it, then built a second through `makeList` — which reassigns `Core.bridge`.
QML's `destroy()` only **queues** the deletion onto the event loop, so the two
trees briefly coexisted and the test's correctness rested on `Repeater`
populating synchronously and on the walk beating the queued deletion. Both are
true here and the test passed reliably, but neither was stated or asserted: they
were timing assumptions the test depended on and could not detect the failure of.

Measuring inside a helper that owns the lifetime removes the overlap rather than
documenting it — CLAUDE.md's "complexity in the data structure, not the logic",
applied to object lifetime. Only the *number* outlives the screen, so nothing
walks a destroyed tree.

**What breaks without it:** nothing red today, which is why this is recorded
rather than left to the diff. Restoring the two-screens-in-one-function form
turns a passing suite into one whose green depends on Qt's deletion scheduling —
a regression that would not announce itself until some unrelated change to
`makeList` or to `Repeater` incubation made the queued deletion land earlier.

**The title is tested against the TOKENS, not against 19.** `DTheme.rowTitle` is
compared to the element's `pixelSize`, and then `rowTitle > body` and
`rowTitle < heading` are asserted as the relation the reference establishes — a
row title outweighs the prose around it without competing with the screen's own
heading. A test hardcoding 19 would fail on a reference revision that kept the
ordering, and — the worse direction — would **pass** on a `DTheme` where `body`
had been raised to 19 and the distinction collapsed. Proved able to fail by
reverting `font: DTheme.rowTitle` to `DTheme.body`, which is exactly the line
this piece changed: `Actual 15, Expected 19`.

It carries a `NO SPEC:` marker, because by D5b the type is contracted nowhere.

**What no test here can still see:** whether any of it renders under basecamp.
`qmltestrunner` instantiates with the host absent, so a layout correct in a spec
and colliding at launch is indistinguishable from one that works.

### D5e — The separator is found by `objectName`, never by geometry

**Chosen:** the separator carries `objectName: "rowSeparator"`, and the test
walker matches on that name.

**Rejected: keying the walk on geometry** — matching any `Rectangle` one pixel
high. It needs no production change at all, which is the whole of its appeal,
and it is the more dangerous option in both directions. It **over-matches**:
anything else that happens to be a 1px `Rectangle` — a future underline, a
divider, a focus ring — joins the count, and a count is the entire assertion in
each of the four separator tests. And it **survives deletion of the thing it
measures**: delete the separator, let an unrelated 1px rule take its place in
the walk, and the suite stays green while the behaviour the delta contracts is
gone. That is this repo's recorded defect family — a fixture where two
explanations give the same answer — reached by a different route.

The name is a **test seam in production code**, which is a real cost and the
reason this is a decision rather than an obvious call: `objectName` exists to be
read by something outside the component. It is paid deliberately. The
alternative is not "no seam" but "an implicit seam keyed on a coincidence of
geometry", and an implicit seam cannot be deleted honestly — nothing tells the
next person that the height they are changing is load-bearing for a test.

**What breaks without it, measured:** deleting the `objectName` line gives
**79 passed, 3 failed** against an 82 baseline — the walk finds zero, so the
three counting tests fail with `Actual 0` against 3, 1 and 4 respectively.

`test_an_empty_list_draws_no_row_boundary` **stays green**, and that is worth
stating rather than rounding to "all four". It asserts zero boundaries, and an
unnamed separator also yields zero, so the mutation and the correct behaviour
give it the same answer. It is not a weak test — it is the only one that catches
the *hoist* defect in D5c's table — but it cannot see this one, and a claim that
all four turn red would have been a fabricated number in a table of real ones.
(That claim was written here first and corrected by running it.)

The silent direction is the one to protect: *replacing* the name-keyed walk with
a geometry-keyed one leaves the suite green today and blind to the deletion case
described above. A reader who finds the `objectName` decorative and drops it for
a height match has made the tests stop measuring the separator without any gate
saying so. And the over-matching is not hypothetical in this very file:
`grep -n -B1 "Layout.preferredHeight: DTheme.hairline" dialectica-ui/src/qml/DStoaListScreen.qml`
returns **six** rectangles of exactly the separator's width and height — the row
separator plus five others at lines 271, 272, 560, 691 and 798 — differing from
it only in `color`. A geometry-keyed walk over the screen would already count
all six today, before anyone adds a seventh.

Recorded in `DStoaListScreen.qml` beside the element as well, since that is
where someone tidying an "unused" property will be standing.

### D6 — ~~The delivery sweep excludes lamp LABELS~~ — withdrawn on rebase

`tst_composer_claims.qml` gained an exclusion dropping any line that *is*
exactly `delivery`, `storage` or `zone`, because the feed's footer rendered a
lamp labelled DELIVERY into the sweep corpus and `DELIVERY` is a needle in
`test_no_gate_state_claims_delivery`.

**With the footer gone, the corpus contains no lamp label at all.** The sweep
instantiates `DComposer`, `FeedScreen` and `DPublishOutcome`, and none of the
three now holds a `DStatusBar` — navigation's lives in `Main.qml`, which the
sweep does not build.

So the exclusion was reverted rather than carried across. **An exclusion that
excuses nothing is worse than no exclusion**: it is a standing hole that a lamp
label — or anything else matching those three words on its own line — could
later slip through with every sweep still green, and nothing would flag it
because nothing would have changed. This is the same shape as the
`otherKnownDenials()` exclusion that file already records deleting, for the same
reason.

The reasoning is kept here because the *argument* for it was sound and will be
needed again the moment a sweep corpus includes a lamp: the labels are pinned
character-for-character by `tst_status_bar.qml`'s `compare(labels[0],
"DELIVERY")`, so a delivery claim cannot be smuggled into a label without that
test failing first — and the strip must be **whole-line, never substring**, or
"Your post was delivered" becomes "Your post was ed" and hides the claim from
the needle at the same time.

### D7 — No moderation screen, and no mocked counts

Both were briefed and both are refused by things that outrank a scope note.
The argument is in `proposal.md` under "What this is not" rather than repeated
here; the short form:

- **Moderation screen** — `PLAN.md` ruling 3, merged two commits before this
  branch was cut: "the MVP ships no moderation screen", blocked at the contract.
  Verified against the trait, not the document: `publish_post`, `publish_reply`
  and `publish_vote` exist and no moderation-publishing method does.
- **Counts** — `stoa-navigation-view`'s "Every number rendered is one this peer
  can actually answer" and ruling 2 (unread is out of the MVP). PLAN.md states
  explicitly that a scope note does not override a merged requirement.

## What no test here can see

**The row treatment is now covered** — four tests in `tst_stoa_screens.qml`, each
proved able to fail by mutation. D5c has what they assert and what they do not.
This section previously said nothing tested it, which was true of the tree the
rebase produced and is no longer true.

**The component suite cannot see whether any of this renders under basecamp.**
`qmltestrunner` instantiates components with the host absent, so a layout that
is correct in a spec and collides with something at launch is indistinguishable
from one that works. The `D`-prefix gate covers the name collision specifically
and nothing else.

## A note on the two concurrent pieces, because it decided this design three times

`piece/ui-navigation` and this piece both edited `FeedScreen.qml` and `qmldir`.
**Reading that branch's diff found three things this change had wrong, and no
gate here would have caught any of them** — the vouch stamp's scope (D3), the
identity chip's binding (D1b), and finally the whole footer (D1). In every case
the sibling was right and this piece was wrong, and in every case every local
gate was green on both sides.

That is worth recording as a method rather than as three incidents: when pieces
run concurrently over shared files, the other branch's reasoning is evidence
this branch cannot get from its own tests. The first two defects were the same
shape — an affordance that was *cheap to build* mistaken for one that was *in
scope* or *correctly sourced*. The third is a different and more expensive
shape: two correct implementations of one job, where the cost is not a bug but
a duplicate, and where "both passed review" would have been how it shipped.
