# The shared shell three screens sit on

## Why

A new UI design bundle (`tmp/ui-bundle-new/handoff/`, gitignored) is the
authority for the interface. It ships eleven QML files; the impl already carries
eight in some form. The three it does not — `StatusBar`, `VouchStamp`,
`IdentityChip` — are the ones **every screen's footer and every post row need**,
so five or six screen pieces are blocked behind them.

They are taken as one piece rather than one each because they share their
dependencies: the same three status colours, the same two button kinds, and the
same `Identicon` change. Splitting them would have meant three pieces each
touching `DTheme` and `FlatButton`, which is three merge conflicts and no
review benefit.

**It builds no screen**, and that is the constraint shaping the whole piece: a
component whose only proof of correctness is "the screen that uses it looks
right" has no proof here, because that screen does not exist yet. Each component
is therefore tested by direct instantiation, against its own properties and its
rendered children.

## What Changes

- **`DStatusBar`** — three lamps in the fixed order DELIVERY, STORAGE, ZONE,
  each with a tooltip. An unrecognised state renders **degraded**, never ok, and
  so does an unbound one.
- **`DVouchStamp`** — outlined VOUCH / filled VOUCHED, visible without hover
  once vouched, and **not drawn at all while this machine has no identity**.
  No count and no property that could hold one.
- **`DIdentityChip`** — the footer chip saying who you are posting as, or that
  you are nobody here yet. Labels the mark, because position alone cannot say
  whose it is.
- **`DTip`** — a tooltip pinned to `Text.PlainText`. Not in the original scope:
  review found that a `ToolTip`'s default content item renders markup, and the
  attached form cannot be fixed in place. See `design.md` D8.
- **`DTheme`** gains `statusOk`/`statusDegraded`/`statusFailed` and
  `markMutedAlpha`; two comments stop describing a column `#70` deleted.
- **`FlatButton`** gains `destructive-outline` and `secondary-micro`, and its
  three parallel ternary chains become one table. An unrecognised kind now
  renders as `secondary` rather than invisibly — a behaviour change, argued in
  `design.md` D5.
- **`Identicon`** gains `muted`, as a root opacity that cannot reach a drawing
  channel.
- **CI** gains a step banning attached `ToolTip` bindings, because the existing
  `textFormat` gate is structurally unable to count one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true` with the measurement behind it:
no merged requirement mentions any of these components, and the two spec files
matching `tooltip|textFormat|PlainText` match on the cryptographic sense of
"plaintext".

**There is no view-facing capability for a shared component to be a requirement
of.** Every capability `openspec list --specs` reports contracts core behaviour
or a named screen. The contract this piece is built against is the design
bundle, which is gitignored and is not a spec this repo owns — so a delta here
would have to invent a requirement describing a QML component's constructor
surface, which is not a behaviour the system has.

**Four behaviours are chosen rather than specified**, and each is marked
`NO SPEC:` in the code and in the test that pins it: an unrecognised lamp state
degrading, an unbound lamp state degrading, an unrecognised button kind falling
back to `secondary`, and the `markMutedAlpha` value. A spec-writer grepping
`NO SPEC:` finds the decision beside its pin. `skip_specs` says this change
writes no requirement; it does not say the behaviour is settled.

## Impact

- **Added:** `dialectica-ui/src/qml/DStatusBar.qml`, `DVouchStamp.qml`,
  `DIdentityChip.qml`, `DTip.qml`, and four `qmldir` registrations.
  `dialectica-ui/tests/tst_status_bar.qml`, `tst_vouch_stamp.qml`,
  `tst_identity_chip.qml`, `tst_identicon_muted.qml`.
- **Modified:** `dialectica-ui/src/qml/DTheme.qml`, `FlatButton.qml`,
  `Identicon.qml`, `dialectica-ui/tests/tst_flat_button.qml`,
  `.github/workflows/ci.yml`.
- **No core change.** This is view-only.
- **`PostHeader.qml` is deliberately untouched**, and it renders a rival
  `YOU VOUCHED` chip for the same fact `DVouchStamp` now stamps. The bundle
  resolves that by replacement; this piece builds the replacement and leaves the
  original standing, so `DVouchStamp` has no consumer yet. `design.md` D9 names
  who inherits the removal.
- **The DELIVERY lamp has no honest source**, and `docs/PLAN.md` records the
  signal as unbuilt rather than merely elsewhere. `design.md` D11, and
  `DStatusBar.qml`'s header for the screen author who meets the property.
