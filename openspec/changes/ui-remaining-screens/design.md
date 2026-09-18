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
token it needs.

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

The chip must be bound to `who_am_i`, **not** to the posting probe — verified at
`dialectica/rust-lib/src/lib.rs:258`, which says the two "can honestly disagree:
a stored identity whose keystore permissions are too open is a real identity
that cannot currently be used. A view with only the posting probe would have to
render 'you are nobody' to a user who has an identity and a fixable problem."

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

**Nothing in the tree tests the row treatment this piece ships.** The separator
and the `rowTitle` font are presentation, `tst_stoa_screens.qml` asserts on the
list's behaviour rather than its geometry, and this change added no test for
either. That is stated rather than left implied: the QML suite passing says
nothing about whether the separator renders under the last row.

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
