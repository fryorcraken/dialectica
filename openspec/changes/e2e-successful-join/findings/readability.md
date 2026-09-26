# Readability review — e2e-successful-join

Scope: readability only, per dispatch. Covered the full three-dot diff
(`git diff 2bda577f7cf70e84f5fb59265f81c2f621224011...HEAD`): proposal.md,
design.md (D1–D6), tasks.md, the two spec deltas
(`stoa-navigation-view`, `view-navigation`), `seeded-join.yaml`,
`seeded_reference.rs`, the `tst_navigation.qml` additions, `DJoinScreen.qml`'s
one-line `objectName` change, `join.yaml`'s paragraph swap, and the
`ui-tests.yml` matrix line.

## What I checked and how

- Read every changed file in full, not just the hunks.
- Verified every decision citation in design.md and proposal.md against the
  named archived change (`e2e-ui-suite`, `e2e-created-stoa-flow`,
  `e2e-suite-review`), by opening each cited section and checking it says
  what the citation claims it says. All resolved and matched: D1/D2 and D1/D8
  of `e2e-ui-suite`, D1/D2 of `e2e-created-stoa-flow`, D2/D3 of
  `e2e-suite-review`, and the "second piece" section of the
  `e2e-created-stoa-flow` proposal.
- Cross-checked the seeded title and address literal for consistency across
  `seeded-join.yaml`, `seeded_reference.rs`, and `tasks.md`'s truncated
  quotes (`git grep` for both strings) — all three agree.
- Spot-checked the "measured" pass-count claims in `tasks.md` (lines 54, 120,
  123: `tst_stoa_screens.qml` 136, `tst_navigation.qml` 26 passed/1 failed,
  `tst_render_probe.qml` 11) by actually running
  `sh dialectica-ui/tests/run-qml-tests.sh <file>` for two of the three files.
  My first pass (grepping `function test_`) suggested the recorded counts
  were inflated by one or two; running the real tool showed why: Qt's
  `qmltestrunner` totals include `initTestCase()`/`cleanupTestCase()` as
  counted PASS entries even when the file only defines per-test `init()`/
  `cleanup()`. `tst_render_probe.qml` actually totalled 11 (9 `test_`
  functions + 2), and `tst_stoa_screens.qml` totalled 136, both matching
  `tasks.md` exactly. The `tst_navigation.qml` figure (26 passed + 1 failed =
  27) fits the same formula against the 25 `test_` functions present at the
  cited commit `661ac181` (verified with `git show 661ac181:... | grep -c`).
  So the recorded counts are accurate — my initial suspicion was a defect in
  my own counting method, not in the piece.
- Tried to verify a mutation-based claim (design.md D1: flipping `VERSION_1`
  in `stoa.rs` turns the three seeded tests red on "unknown genesis record
  version 1") and was refused by the permission classifier ("Modify Shared
  Resources") on an `Edit` to `dialectica/rust-lib/dialectica-core/src/stoa.rs`,
  and a `sed -n` read of an archived design.md was refused the same way. I did
  not pursue a workaround. That claim, and a few sibling "measured" claims
  about `dialectica-core`'s internals, stay unverified by me; correctness
  review is better placed to chase them since it can mutate the crate.

## Findings

None. The diff is unusually small and disciplined for what it proves: one
`objectName` plus a three-line comment in `DJoinScreen.qml`, a one-line
matrix edit, a paragraph swap in `join.yaml`'s existing "does NOT cover" note,
one new spec file, one new Rust test file, and two new component tests. Every
new comment states a "why" rather than restating the adjacent code
(`seeded_reference.rs`'s module doc in particular earns its length: it tells
a future reader what a red run there means and what NOT to do about it,
which is exactly the kind of comment CLAUDE.md asks for). Naming is
consistent with the surrounding files (`joinCancelButton` alongside
`copyReferenceButton`'s existing comment style). The two new QML tests reuse
existing helpers (`spec.stoaA`, `spec.bridgeFor`, `spec.visibleNamed`,
`callsTo`) rather than inventing parallel ones, and each carries a comment
explaining why the assertion is shaped the way it is (e.g. the fake's listing
answering from whether a join was made, so a null implementation can't pass).
Decision citations throughout proposal.md and design.md name their source
change and section, and every one I checked resolves to a real, matching
passage in an archived change — none of the "earlier pieces standardised
that" citations here are dangling or repurposed.

- [x] **none** — no readability defects found in this piece's diff; see notes
      above on what was checked, including the two "measured" pass-count
      claims verified by actually running the QML suite rather than trusting
      a grep count.
