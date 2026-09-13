# Drop the apparatus column from the shipped view

## Why

The design bundle's right-hand column headed `APPARATUS` carries italic marginal
notes — `ON THIS ORDERING`, `ON WHAT YOU HOLD`, `ON THE MARK` — which explain **to
a reader of the design** why the screen behaves as it does. **The owner has
confirmed it is annotation about the design, not part of it.**

It reached the shipped QML anyway. On `origin/main` (`468e716`):

- `ApparatusColumn.qml` renders a `Theme.paperDeep` panel with a literal
  `"APPARATUS"` heading;
- `MarginNote.qml` renders one entry in it;
- `ScreenFrame.qml` — the shell **every** screen is built on — is a two-column
  `RowLayout` whose right column is that panel, unconditionally;
- `FeedScreen.qml` ends with three `MarginNote`s filling it.

So a real user of the app sees a 244px column of commentary addressed to a
designer. That is the mistake.

**The obligations those notes carry are real and are not being dropped.** Each
note restates something the interface genuinely owes the reader. They survive
because `docs/UI-BRIEF.md` already states all three, in prose, where the brief
states obligations — verified line by line in `design.md` §2 rather than
assumed. Only the marginal-column *presentation* goes.

## What Changes

- **Delete `ApparatusColumn.qml` and `MarginNote.qml`**, and their two `qmldir`
  registrations. Nothing else instantiates either.
- **Reshape `ScreenFrame.qml`** from a two-column `RowLayout` into a single
  content column. The alternative — keeping the layout and leaving the right
  column empty — is the shape this change exists to remove, and it would leave
  244px of dead width on every screen.
- **Remove `FeedScreen.qml`'s `apparatus: [...]` block**, the only assignment to
  the property `ScreenFrame` exposed.
- **Remove `Theme.apparatusWidth`** and the two theme comments that describe inks
  by their role in the column. `Theme.paperDeep` itself stays: it is a surface
  token, and a screen may want a deeper paper for an inset panel; only the
  comment claiming it is *the apparatus column* is wrong once the column is gone.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true` with the measurement behind it.

**No merged spec requires apparatus text.** The single `apparatus` hit across
`openspec/specs/` is `module-wire-contract/spec.md:302`, using the word in its
Phase 0 sense ("the probe is apparatus rather than forum surface"). The two specs
named as risks in the dispatch — `composer-view` and `stoa-navigation-view` — **do
not exist on `main`**; `openspec/specs/` holds sixteen capabilities and neither is
among them. There is no `copy.json` anywhere in the tree, so no `*.apparatus` key
is required verbatim by anything. If either spec lands later requiring apparatus
text, that is a `spec-writer`'s question and an owner's call, not this piece's.

## Impact

- **Deleted:** `dialectica-ui/src/qml/ApparatusColumn.qml`,
  `dialectica-ui/src/qml/MarginNote.qml`.
- **Modified:** `dialectica-ui/src/qml/ScreenFrame.qml`,
  `dialectica-ui/src/qml/FeedScreen.qml`, `dialectica-ui/src/qml/qmldir`,
  `dialectica-ui/src/qml/Theme.qml`, `docs/UI-BRIEF.md`.
- **No test changes.** `grep -rniI "MarginNote\|apparatus\|ON THIS ORDERING\|ON
  WHAT YOU HOLD\|ON THE MARK"` over `dialectica-ui/tests/` returns one line, in
  `tst_identicon.qml`, and it is the word "mark" inside an unrelated comment. **No
  test asserts apparatus content**, which is itself worth recording: the column
  shipped and no gate could see it.
- **No core change.** This is view-only.
- **`docs/UI-BRIEF.md` needed less correcting than expected**, and the reason is
  recorded in `design.md` §1: the brief never presented the apparatus as something
  the interface renders. The word does not appear in it. What this change adds is
  one short paragraph making that silence explicit, so the next reader of the
  bundle does not repeat the mistake.
