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
| `ON WHAT YOU HOLD` | every count is what this machine holds; no global total is knowable | **Brief constraint 1**, `UI-BRIEF.md:86-88`: "a count of *anything* global — members, total posts — is unknowable. Do not show one." And in the **interface already**: `FeedScreen.qml`'s empty state says "This is a fact about your copy, not about the Stoa." |
| `ON THE MARK` | the mark is recognition, never proof; the address is printed beside it | **Brief obligation 6, layer 2**, `UI-BRIEF.md:584-593`: "the identicon must never be rendered as a verification mark, a badge, or anything that reads as 'checked'." And **structurally in the interface**: `PostHeader.qml` and the Stoa header both render `AddressLabel` beside `Identicon`, unconditionally. |
| `ON THIS ORDERING` | this feed is **not** newest-first | **Brief Feed section**, `UI-BRIEF.md:331-375` — but see §3. The brief's half survived; the interface's half did not, so it was moved rather than deleted. |

Two of the three discharge **structurally**, which is the stronger form: the
address is beside every mark whether or not any text says it should be. A note
asserting the same thing adds nothing a reader acts on, which is part of why the
column was never load-bearing.

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
`body.implicitHeight + 2 * Theme.cardPaddingY`, and the layout anchors to three
edges rather than filling, so content packs to the top as
`Layout.alignment: Qt.AlignTop` used to make it.

**This is a behaviour change beyond the apparatus question and is deliberately
not presented as one of the removals.** It is recorded here rather than fixed
silently. A reviewer should read it as: the two-column form concealed a missing
height source, and a one-column form cannot.

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
