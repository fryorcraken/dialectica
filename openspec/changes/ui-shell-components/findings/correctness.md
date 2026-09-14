# Correctness review — `ui-shell-components`

Dimension: **correctness only**. Security, readability and architecture are held
by other instances.

Reviewed at `d9fe78f` against `origin/main` (three-dot), with the design bundle
at `tmp/ui-bundle-new/handoff/` as the authority. Baseline before mutating:
18 spec files, 342 assertions, 0 failures; `check_qml_members.sh` and
`check_qml_names.py` both green.

## Findings

- [ ] **`dev-writer`** — `FlatButton.qml:46` — the `kinds` lookup reaches
      `Object.prototype`, so the unknown-kind fallback does not fire for a
      prototype member name
      **Scenario:** `FlatButton { kind: "constructor" }` (likewise `"toString"`,
      `"valueOf"`, `"hasOwnProperty"`, `"__proto__"`, `"isPrototypeOf"`,
      `"propertyIsEnumerable"`). `kinds[kind]` resolves to the inherited
      `Object.prototype` function rather than `undefined`, so the guard
      `kinds[kind] !== undefined` passes, `spec` becomes a `Function`, and every
      field read off it — `spec.fill`, `spec.stroke`, `spec.textInk`,
      `spec.font`, `spec.padX`, `spec.padY` — is `undefined`. The documented
      fallback to `secondary` never happens.
      **Measured** (Qt 6.10.3, this worktree): all seven prototype names render
      `fill=#ffffff borderW=1 borderC=#000000 labelInk=#000000 iw=NaN ih=NaN`,
      against `secondary`'s `fill=#00000000 borderW=1 borderC=#26231d
      labelInk=#26231d iw=60.078125 ih=35`. Qt logs four
      `Unable to assign [undefined] to QColor/QFont` warnings per instance — a
      white-on-black button of undefined size, which is precisely the
      "invisible control that still accepts clicks" class D5 exists to close,
      reintroduced through a different door.
      **Why the suite misses it:** `tst_flat_button.qml:182`'s unknown-kind
      corpus is `["", "Primary", "destuctive", "micro", "ghost",
      "secondary micro", "undefined"]` — seven plain strings, none a prototype
      member. `test_an_unrecognised_kind_falls_back_to_secondary` passes; so do
      all 16 FlatButton assertions.
      **Severity:** moderate, and latent rather than live — every one of the 24
      `kind:` call sites in `dialectica-ui/src/qml/` is a string literal today
      (`grep -rn "kind:"`), so nothing reaches it in the current tree. It
      becomes live the moment a `kind` is bound from a model field or from
      core. The fix is a shape, not a longer denylist: an own-property check
      (`kinds.hasOwnProperty(kind)`), or seeding the table with
      `Object.create(null)`.
      **Note for `tester`:** adding prototype names to the corpus at
      `tst_flat_button.qml:182` is the regression test, and it fails before the
      fix — measured above.

## What I verified and found correct

**All three defects the author reports finding are real, and I re-measured each
rather than taking the claim.**

*The `state` shadowing.* Reproduced on Qt 6.10.3 with an isolated probe: a
`Rectangle` declaring `property string state` alongside
`states: State { name: "degraded"; PropertyChanges { target: self; marker: 99 } }`
holds `state=degraded` and leaves `marker=0`; the identical component with the
property named `lampState` gives `state=degraded marker=99`. Same assignment,
silently different behaviour, no error or warning — exactly as described. The
rename is complete: `grep` over `DStatusBar.qml` shows `lampState` at the
declaration, the binding and all three call sites, with `state` appearing only
in comments. The regression test at `tst_status_bar.qml:209` is genuinely
load-bearing, and it catches *both* spellings of the regression: reverting the
name to `state` fails it 1 of 13 on `lampState` being `undefined`, and the
subtler re-introduction (keeping `lampState` wired while adding a shadowing
`property string state` beside it) fails it 1 of 13 on its second assertion,
`the item's built-in state was written to`. That second case is the one that
matters, and the test was written to see it.

*`isPerson`.* Genuinely absent from `dialectica-ui/src/` (`grep` returns
nothing), while the bundle's `IdentityChip.qml:29` does pass it. The
undeclared-property drop is silent in QML, so
`test_the_mark_has_no_isPerson_channel` asserting `marks[0].isPerson ===
undefined` is the right shape for this.

*`width`/`height` on a layout-managed item.* Restoring the bundle's spelling at
`DStatusBar.qml:101-103` makes `check_qml_members.sh` exit 1 with two
`Quick.layout-positioning` warnings naming lines 102 and 103. The gate catches
it, and the departure from the bundle is correct.

**The two rewritten tests can now fail.** The markup assertions key on
`textFormat !== 0` rather than on the round-tripped string, which is the right
correction — `Text.text` returns its source whatever the format, so the string
form cannot see a `RichText` mutation. The colour test at
`tst_status_bar.qml:89` asserts three ways (token, hardcoded literal, and
pairwise distinctness), which closes the `both sides undefined` hole the author
describes; the literal half fails if a token is renamed, and the relation half
fails if all three are edited to one value.

**I looked for further instances of this repo's defect family — a fixture where
two explanations give the same answer — and found the tests unusually well
guarded against it.** The tree-walking helpers are the place this would
normally hide, because a walker that finds nothing reports clean. They do not:
`dotsOf` is always followed by `compare(dots.length, 3)`, and mutating the dot's
`implicitWidth` from 7 to 8 (which makes the helper's key miss) fails three
tests loudly rather than passing vacuously. `labelsOf` is likewise pinned with
`compare(labels.length, 3)`, `identiconsOf` with `compare(marks.length, 1)`, and
`everyText` with `compare(texts.length, 1)`. `tst_identity_chip.qml`'s
`visibleText` correctly propagates ancestor visibility, without which the
no-identity absence assertions would be vacuous.

**The `NO SPEC` defaults are right.** An unknown lamp state falling to
`degraded` rather than `failed` is the correct call: `failed` asserts the
machine is broken, which the UI cannot back from a string it could not parse,
where `degraded` says "I do not know" — and the bundle's Tone section forbids
claiming more than the software delivers. Falling to `ok` would be the actual
error, and `test_an_unrecognised_state_degrades_rather_than_reading_as_ok`
pins it over 13 values. `markMutedAlpha: 0.45` is asserted both as a relation
(strictly less opaque, strictly above zero) and as a literal, which survives a
retune while still catching the token going missing. The bundle asks for
`muted: true` at `examples/ModerationScreen.qml:94` and gives no value, so the
`NO SPEC` mark is honest.

**Copy strings are verbatim.** Every string I checked against `copy.json` —
`VOUCH`, `VOUCHED`, both vouch tooltips, `CURRENT IDENTITY`,
`Create an identity`, `Voting, posting and replying need an identity.`,
`DELIVERY`/`STORAGE`/`ZONE` — matches exactly, and the tests hardcode the
expected values rather than reading them off the component.

**The coverage disclaimers are accurate, not defensive.** `onClicked:
root.clicked()` is present at `FlatButton.qml:67` and `onClicked:
root.toggled()` at `DVouchStamp.qml:80`; both signal tests do emit directly and
therefore genuinely cannot see the wiring, and both say so. Removing the
assertion that would have falsely claimed it was the right call. The host
name-collision disclaimer at `tst_status_bar.qml:9-13` matches what CLAUDE.md
records, and `check_qml_names.py` passes. I found no test claiming coverage it
does not have.

## Tree state

Every mutation I made was reverted with `git checkout` and the probe
directories under `tmp/` removed; `git status --porcelain` in the review
worktree is empty and `tst_status_bar.qml` is back to 13 of 13 passing.
