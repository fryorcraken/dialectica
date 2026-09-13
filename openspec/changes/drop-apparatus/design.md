# Design — dropping the apparatus column

## 1. The brief needed less correcting than the dispatch expected, and why that matters

The dispatch asked me to stop `docs/UI-BRIEF.md` "presenting the apparatus as
something the interface renders". **It never did.** Measured, not assumed:

```
git grep -niI "apparatus\|margin note\|marginal\|MarginNote" origin/main -- docs/UI-BRIEF.md
```

returns nothing. The word does not occur in the file.

That is worth recording rather than quietly skipping, because it locates the
defect precisely. The apparatus did **not** reach the QML by way of a brief that
asked for it. It reached the QML directly from the design bundle, whose marginal
notes look — to someone implementing screen by screen — exactly like every other
element in the mockup. The brief was silent, and silence was read as permission.

So the correction the brief needs is not a retraction. It is the sentence it was
missing: **an obligation is a thing the interface must do, not text to print at
the reader, and the bundle's apparatus column is annotation rather than
interface.** That is added as a box under *Non-negotiable rendering obligations*,
which is the section a designer reads when deciding what a screen owes.

## 2. Where each apparatus obligation lives now

The three notes were checked one at a time against the brief **before** deleting
anything, because a note whose obligation exists nowhere else is a requirement
being deleted by accident. Line numbers are `origin/main`'s.

| Note | The obligation | Where it survives |
|---|---|---|
| `ON WHAT YOU HOLD` | every count is what this machine holds; no global total is knowable | **Brief constraint 1** — "a count of *anything* global — members, total posts — is unknowable. Do not show one" — and **brief rendering obligation 10**, which this change added because the note's wording did not cover the feed's actual claim. In the **interface**, two separate sentences carry it, each beside the claim it qualifies: see below. |
| `ON THE MARK` | **two propositions** — see below; they do not survive equally | **Brief obligation 6** — *"A generated name is never unique and never an identifier — the address is."* — and specifically **layer 2, the identicon**, in its four-layer list. Carries both. In the **interface**, only the pairing half survives. |
| `ON THIS ORDERING` | this feed is **not** newest-first | **Brief Feed section**, `UI-BRIEF.md:331-375` — but see §3. The brief's half survived; the interface's half did not, so it was moved rather than deleted. |

### `ON THE MARK` carried two propositions, and only one survives on screen

Narrowed after review, which found the original wording here claimed more than it
supports. The note said the mark is "a shortcut for recognition, **never a proof
of anything** — which is why the address is printed beside it". Two claims:

- **The pairing claim — the address is beside the mark.** This survives
  **structurally**, which is the stronger form: `PostHeader.qml:35-38` renders
  `AddressLabel` with no `visible:` binding and no empty-string collapse, while
  the `Identicon` at `:21-26` *is* conditional (`visible: markSize >=
  Theme.markMinDraw`). The asymmetry runs the safe way — an address can appear
  without a mark, a mark cannot appear without an address. Same at the Stoa
  header, `FeedScreen.qml:143`. The review verified this rather than trusting it.
- **The non-proof claim — the mark proves nothing.** This has **no surviving
  rendered text**: `grep -rn "proof"` over `dialectica-ui/src/qml/` returns
  nothing.

**The second is accepted as undischarged in the interface, deliberately.** The
brief states it (obligation 6 and its four-layer list, which is explicit that the
identicon "is nonetheless forgeable in exactly the way the name is"), and the
protection a reader actually needs is the address being *present* — which the
structural half delivers. A line of prose telling a reader that a glyph is not
proof is not something they act on; showing them the address is.

So the claim this row supports is **"the address is beside the mark"**, not "the
non-proof proposition is discharged somewhere". Anyone citing this table for the
latter is citing it wrongly, which is why the distinction is written out.

### `ON WHAT YOU HOLD` was narrower than it looked, and the gap was not a count

An earlier version of the row above claimed this obligation survived "in the
**interface already**", citing the empty state's "This is a fact about your copy,
not about the Stoa." Review found that sentence and the `"STORE READ OK · N POSTS
HELD"` line both sit inside the `Rectangle` gated on `rows.length === 0`, so a
feed showing thirty posts made no locality claim at all. The row was overclaiming.

**The obvious repair was the wrong one, and measurement is what showed that.**
The natural reading — "a locality line wherever a count appears" — has no subject
on a non-empty feed, because **the non-empty feed renders no number**. `"STORE
READ OK · N POSTS HELD"` is the screen's only count and it renders only when the
list is empty, so it only ever reads `0`. There is no page number, no "showing 30
of", no reply count.

**What is unqualified is the pagination control, and it is an extent claim
without a numeral.** `hasMore` comes from `feed::list_threads`, computed from this
peer's log alone, so "Next" means *this machine holds another page* — while a
reader meeting a full page and a "Next" button reads it as *this Stoa has more*.
That is precisely the crack the note's own wording left open: it said "every
**number** here counts what this machine has received", so when the assertion
stopped being a numeral it stopped being covered.

So the requirement is **triggered by the extent claim rather than by screen
state**, and it is written as brief rendering obligation 10: never render a
quantity the core cannot know; where the interface *does* assert extent, that
assertion must be readable as local; and **a screen asserting no extent owes
nothing** — the last clause being what keeps this from becoming a disclaimer
printed once per screen, which is the mistake this whole change undoes.

**Taken: the sentence lives inside the pagination control's own layout.** The
`RowLayout` became a `ColumnLayout` holding the buttons and the sentence, so one
`visible:` binding — the one the row already had — decides both. The claim and
its qualifier cannot render apart, in either direction.

**Rejected: a body-level `Text` with its own visibility condition**, which is the
shape `ON THIS ORDERING` uses in §3. It would work, but it adds a fourth
slightly-different `visible:` guard to a screen that computes `readState`
precisely so at most one state renders by construction — and CLAUDE.md names a
fourth guard as the signal to reshape rather than to add a fifth. Nesting reuses
the existing binding instead of duplicating its condition, so there is no second
place to keep in step.

**Rejected: leaving it as the code comment beside the `RowLayout`**, which is
where this reasoning already lived and which no user opens. That is the same
failure as leaving an obligation in the brief alone.

**The architecture review's premise dissolved rather than being answered.** It
filed this as "whichever answer you choose, the current shape makes it awkward",
on the assumption the trigger was screen state and a fourth guard was needed. The
pagination row was always outside `readState`'s three-state invariant and already
carried its own binding, so the reshape costs no new condition.

**A note for whoever tests this.** `visible` is not a readable signal here: QML
reports *effective* visibility, and a `TestCase` is itself invisible offscreen, so
every descendant reads `false` whatever its own binding says. Height is worse than
useless — the paging-versus-no-paging card height delta was measured identical
with the sentence present and with it replaced by a one-word string, so a height
assertion is a gate the defect satisfies. What is readable is the object graph:
the sentence and the buttons sharing one governing ancestor is the property that
makes "renders where paging is offered, and only there" true by construction.

## 3. The one obligation that would have vanished, and the decision taken

`ON THIS ORDERING` is the exception, and it is the case the dispatch warned
about.

The ordering control's label is `"same order for everyone"`. That label is
**honest** — it satisfies the brief's rule against labelling an ordering "new",
"latest" or "recent", and it is true of this fallback specifically. But it is
**neutral**. It declines to claim recency; it does not deny it.

The denial matters because a reader meeting a forum feed assumes newest-first
unless told otherwise. A neutral label leaves the interface relying on the reader
not to make the ordinary assumption — which is not a thing an interface may rely
on. On `main` the only place that assumption was corrected was the apparatus
note, so deleting the column would have removed the correction and left nothing.

**Decision: the sentence moves into the feed's own body**, under the heading
rule, in `Theme.note` — verbatim, since its wording was already reviewed. It is
now interface addressed to a user rather than margin addressed to a designer,
which is the distinction this whole change turns on.

**Rejected: leaving it to `docs/UI-BRIEF.md` alone.** A brief obligation with no
interface text is precisely how the apparatus came to ship in the first place —
the brief said what was owed, the bundle showed a way to say it, and the way got
built. Discharging an obligation into a document nobody running the app reads is
not discharging it.

The brief's Feed section gains the matching clause, so the next designer is told
that a neutral label is not by itself enough.

## 4. Reshaping `ScreenFrame`, and a defect the reshape exposed

`ScreenFrame` was a `RowLayout` of two columns: the content, and the apparatus
panel at a fixed `Theme.apparatusWidth` of 244px. Removing the panel leaves a
choice.

**Rejected: keep the `RowLayout`, drop only the panel.** That is a one-column
`RowLayout`, which is a `ColumnLayout` with a misleading name, and it leaves the
`apparatus` property alias pointing at nothing. It also keeps the shape the
change exists to remove.

**Rejected: keep the column but render it empty.** 244px of dead width on every
screen, and an invitation for the next screen to fill it back up.

**Taken: one anchored `ColumnLayout`**, with the `apparatus` alias gone.

The reshape then forced a question the two-column form had hidden.
`anchors.fill: parent` on the old `RowLayout` meant the layout's implicit size
did **not** propagate to the `Rectangle`, so `ScreenFrame` had **no
`implicitHeight` at all** — while `Main.qml:42` reads exactly that value to set
the `Flickable`'s `contentHeight`. Both landed in the same commit (`0538c0d`), so
this has been true since the screen was written.

With one column there is nothing else that could know the card's height, so the
reshape cannot avoid answering: `implicitHeight` is now
`body.implicitHeight + 2 * Theme.cardPaddingY`.

**This is a behaviour change beyond the apparatus question and is deliberately
not presented as one of the removals.** It is recorded here rather than fixed
silently. A reviewer should read it as: the two-column form concealed a missing
height source, and a one-column form cannot.

Review measured the defect and it was **worse than this section first claimed**:
on `origin/main` the feed's `implicitHeight` was **0 with thirty rows**, and
`Main.qml`'s `Flickable.contentHeight` was **56** — the two padding spacers and
nothing else. **The feed did not scroll at any row count.** After the change,
`contentHeight` and the laid-out content agree exactly (331), and the value tracks
content monotonically: 275 empty → 1030 at five rows → 4805 at thirty.

### The anchor choice also changed what `fillHeight` means, which this section missed

Added after review. The first version of this reshape anchored `body` to **three**
edges — top, left, right — reasoning that `implicitHeight` above already carried
the card's height and the column should size itself. That is correct for the
height, and it **silently broke a case no screen on this branch exercises**.

A `ColumnLayout` with no bottom constraint has height equal to its own implicit
height, so it has **no spare space to distribute**, and a child declaring
`Layout.fillHeight: true` falls back to its `implicitHeight` — 0 for a bare
`Rectangle`. Measured at Qt 6.10.3, same markup: **544 high on `origin/main`, 0 on
the three-edge form.** No warning, no binding loop, qmllint exit 0, 41/41 green.
The failure mode is a blank region on a screen where every gate passes.

`FeedScreen` uses no `fillHeight`, so the branch was honestly green — but #60, #62
and #63 are all building screens on this shell, and a body that fills the card is
the ordinary case. The text those screens owe a reader (a seed-phrase permanence
warning, a closed-gate reason, a publish outcome that must not claim delivery) is
exactly what would have vanished. **An obligation discharged by a zero-height
element is an obligation not discharged.**

**Taken: bind `body.height` to `Math.max(implicitHeight, root.height - 2 *
cardPaddingY)`.** A `fillHeight` child gets the real slack (544 again), and the
binding reads `root.height` while `implicitHeight` reads `body.implicitHeight`, so
the two touch disjoint properties and cannot loop. Qt reports none.

**Two alternatives were tried and measured before being rejected**, which is the
only reason the trade below is stated with confidence rather than asserted:

- **Anchor the bottom edge.** Fixes `fillHeight` identically, and is the same
  trade — it is not a *better* answer, just a less explicit one.
- **Add a trailing `Item { Layout.fillHeight: true }` spacer to absorb slack.**
  Worse on two counts, both measured: it **splits the space with a genuine
  `fillHeight` child** (544 → 262), and its `spacing` gap enters
  `body.implicitHeight`, inflating the card by 20px so the `Flickable` scrolls
  past the end of the content — re-breaking the very thing `implicitHeight` fixed.

**The trade this accepts, stated plainly.** A `ColumnLayout` taller than its
content distributes slack *among its children*. So in a card given an **explicit
height** with no child claiming that slack, two 40px rows land at y=111 and y=393
rather than stacked at y=0 and y=60 — the top-packing the three-edge form gave
away for free, which this binding does not recover.

It is accepted rather than solved because **no caller here gives a card an
explicit height**: `Main.qml` sets `Layout.preferredWidth` and alignment only
(verified — the file contains no `height` assignment to the frame), so the card is
always sized from `implicitHeight`, where content and card agree and nothing
scatters (measured: y=0 and y=60). The scattering needs a caller that does not
exist; the `fillHeight` collapse was going to be hit by the next screen written.
`ScreenFrame.qml` carries the escape hatch in a comment, so a screen that does set
an explicit height is told what to add rather than left to diagnose it.

### The two escape hatches are not equivalent, and the comment used to offer both

Added after review. The escape hatch originally offered a choice —
`Layout.fillHeight` on a child, **or** a trailing `Item { Layout.fillHeight:
true }` — which contradicted this same section's rejection of the trailing
spacer a few paragraphs earlier. Both fix the scatter; only one is free.
Measured in one run, two 40px rows in a `ScreenFrame`:

| Form | `implicitHeight` | rows at `height: 600` |
|---|---|---|
| no slack-absorbing child | 156 | y=111, y=393 — the scatter |
| trailing `Item { Layout.fillHeight: true }` | **176** | y=0, y=60 |
| `Layout.fillHeight` on a real child | **156** | y=0, y=60 |

The 20px delta is `Theme.blockGap` exactly: the spacer is a layout child, so it
takes a `spacing` gap that enters `body.implicitHeight` and inflates the card —
re-breaking what `implicitHeight` exists to fix, on a screen where the scatter
looks solved. The comment now names `fillHeight`-on-a-real-child as the fix and
says what the spacer costs, rather than presenting them as alternatives.

**The general shape of the defect**: a reader who hits a symptom takes whichever
remedy needs least judgement. Offering two and warning about neither means the
cheaper-looking one gets picked.

### Where the shell's contract lives

Also after review. `ScreenFrame`'s contract lived only inside `ScreenFrame.qml` —
`grep -rn ScreenFrame` over `docs/` returned nothing — so the rule was reachable
only by someone who already had a reason to open the shell. A screen author
starting `MyScreen.qml` with `ScreenFrame { … }` has no such reason until
something has already gone wrong, and the failure mode is silent.

Four new call sites across three in-flight branches happen to satisfy the
contract, but by what those authors wrote rather than by anything telling them.
The evidence that the rule was in the wrong place is that a second author
independently re-derived "the apparatus is annotation, not load-bearing" on
`piece/ui-onboarding` and, having derived it, kept the margin note anyway — two
authors, the same conclusion, no shared place to record it.

**Taken: state the contract in `docs/UI-BRIEF.md`**, under *What `ScreenFrame`
gives you*, which this change already edits and which the repo designates as the
live statement of what a screen owes. Three supporting edits make it reachable:
the brief's opening now names the QML implementer as a second audience,
CLAUDE.md's "Where to look for what" row now sends a screen author there
**before writing a screen**, and `ScreenFrame.qml`'s header points at the
section and asks for the two to be kept in step.

**Rejected: leave it in the component's comment block.** That is the arrangement
that produced the gap. It also hides the rule from anyone reading the brief to
decide what a screen owes, which is the audience that most needs it.

**Rejected: a spec delta.** The contract is about how a QML shell is used, not
about observable forum behaviour; `.openspec.yaml` sets `skip_specs: true` for
this change and nothing here alters that.

The comment block shrank in the same pass: four passages narrated the two-column
shape this replaced, which `git log` and this section already carry. What is kept
is the two things neither answers — why `body.height` is bound at all, and what
to do when a card scatters.

## 5. `Theme.paperDeep` stays; `Theme.apparatusWidth` goes

`apparatusWidth` had exactly two readers, both deleted, so it goes with them.

`paperDeep` is now unreferenced but **stays**. It is a surface token in a palette
— a deeper paper for a panel inset in a card — and the next inset panel will want
it. What was wrong was only its comment, which named it as *the apparatus
column*. The same applies to `accent`, whose comment listed "apparatus rules"
among its uses.

Deleting a palette token because today's screens happen not to use it is a
different change from this one, and would make the palette a record of current
usage rather than a designed set.

## 6. What this change does not touch

- **No spec delta.** `.openspec.yaml` sets `skip_specs: true` with the
  measurement. See §7 for the one spec that *does* name an apparatus string.
- **No test changes.** No test on `main` asserts apparatus content — measured by
  grep over `dialectica-ui/tests/`, which returns one line, in `tst_identicon.qml`,
  and it is the word "mark" inside an unrelated comment. **That is itself a
  finding**: the column shipped, and no gate could see it.
- **No core change.** View-only.
- **No unrelated staleness fixed.** Several things in `UI-BRIEF.md` invite
  editing; all are left alone. Three branches are editing this file concurrently
  and one rewrites it wholesale, so a sweep here would be a sweep nobody can
  review.

## 7. A spec on an unmerged branch requires an apparatus string — reported, not decided

`piece/ui-composer` (PR #62) carries a spec delta requiring an apparatus string:

> `openspec/changes/ui-composer/specs/composer-view/spec.md:79`
>
> "The view SHALL state that no compose box is shown and why, using the bundle's
> `compose.apparatus` string, so the absence reads as a decision rather than as a
> missing feature."

**Read in full, this requirement does not need the column.** It sits inside the
requirement *"A closed gate shows the reason verbatim and offers a fix"*, and
what it requires is that **the closed gate** state why no compose box is shown.
It names `compose.apparatus` only as the source of the wording. A sentence in the
closed gate's own body discharges it exactly — which is the same move §3 makes
for the ordering note, and it is the better answer on its own merits: a statement
about why *this gate* is closed belongs in the gate, not in a margin.

**But the wording "the bundle's `compose.apparatus` string" is contract text, and
changing a contract is a `spec-writer`'s job with the owner's call.** So this is
reported and not touched. Nothing in this change edits `openspec/specs/` or any
other change's delta.

Three facts bound the risk, all measured:

- **Nothing is merged.** `composer-view` does not exist in `openspec/specs/`,
  which holds sixteen capabilities and not that one. The requirement is in an
  in-flight delta.
- **The string has no implementation anywhere.** `git grep "compose.apparatus"`
  over `piece/ui-composer`'s pushed tip finds it in the spec and the proposal, and
  in **no QML file**. There is no `copy.json` in the tree at all, on any branch.
  So no code is being broken; a requirement is awaiting an implementation that has
  not been written.
- **The test said to block this is not on any branch.** The dispatch named
  `tst_vote_and_gate.qml::test_the_apparatus_string_is_the_bundles_and_is_verbatim`.
  `piece/ui-composer`'s pushed tip carries four test files and none is that one;
  `git grep "apparatus"` over its `dialectica-ui/tests/` returns nothing. It
  exists in a tester's working tree that has not been pushed.

So this change cannot fail that test and cannot edit that spec. **It is the
composer piece's decision** whether the requirement keeps naming a bundle string
whose column no longer exists. This design note is the hand-off.

## 8. The delivery disclaimer, and why it is not this change's to preserve

The dispatch asked that the `ON PUBLISHING` note's obligation — that the
interface positively denies knowing anything about delivery — be preserved in the
screen's own body rather than in the brief alone.

**That note does not exist on this branch.** `git grep "ON PUBLISHING"` over both
`origin/main` and `piece/ui-composer`'s pushed tip returns nothing; it is in the
same unpushed working tree as the test above. On `main` there is no publish path,
no compose box and no submit control at all — `FeedScreen` only reads.

So there is nothing here to preserve it into, and adding a delivery disclaimer
would mean writing a denial about an affordance this branch does not have. That
is not honesty; it is text about a button that is not on the screen.

**The obligation is real and is already contracted**, twice over, so it is not
resting on a note:

- `docs/UI-BRIEF.md` obligation 9 — "A successful publish means 'saved here', not
  'posted'" — which is unchanged by this change.
- `composer-view`'s requirement *"A successful publish claims local storage and
  never delivery"*, with three scenarios, on the branch where publishing lands.

The right place for the interface text is the composer screen, which is where the
success message that must not claim delivery will be written. Recorded here so
the composer piece inherits it rather than discovering it.
