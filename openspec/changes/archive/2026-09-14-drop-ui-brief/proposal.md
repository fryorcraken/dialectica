# Delete `docs/UI-BRIEF.md` and repair the prose that cited it

## Why

`docs/UI-BRIEF.md` was written for an external designer who cannot read the
code. It has been designed against, and the owner has settled that it is now a
liability rather than an asset: **a new design bundle is the UI authority.**

This is the owner's decision and is not re-argued here. What this change owes is
that the repository is left *true* after the deletion, which is not automatic:
twenty-one live citations point at the file, and six of them are load-bearing
sentences that delegate a claim outward rather than merely name a source.

**The brief is deleted as a document, not relocated as a set of requirements.**
Where a rule already lives in an openspec spec or a test, it stays there
untouched. Where it lived only in the brief, it goes with the brief. No
replacement document is created.

## What Changes

- **Delete `docs/UI-BRIEF.md`.**
- **`CLAUDE.md`** — remove its row from the "Where to look for what" table.
- **`docs/PLAN.md`** — six citations, two of them load-bearing. Each paragraph
  is rewritten to be true with the brief gone. Where PLAN.md was delegating a
  claim outward, the claim is either stated in PLAN.md or dropped; `design.md`
  §2 says which, one by one.
- **`docs/IDENTICON.md`** — the sentence citing "UI-BRIEF obligation 6" makes
  its own argument instead.
- **Source and test comments** naming the brief are rewritten to stand alone.
  Each explains *why* the code is as it is, so none is simply deleted.

**No behaviour changes.** No QML rendering changes, no Rust logic changes, and
no test assertion changes. One assertion *message* string is reworded
(`tst_screen_frame_geometry.qml`), which is prose inside a failure message and
not a condition.

**Nothing under `openspec/changes/archive/` is touched.** Twenty-three
citations live there. An archive records what was decided at the time; editing
it would falsify history, and a reader who finds a dead link in an archived
change learns something true — that the document existed then.

## Impact

- No spec delta. `grep -rn "UI-BRIEF\|UI brief"` over `openspec/specs/` returns
  nothing: no live requirement cites the brief. See `.openspec.yaml`.
- The `ScreenFrame` implementer contract loses its prose home and is
  consolidated into `ScreenFrame.qml`'s own comments — see `design.md` §3,
  which is the one place this change moves material rather than deleting it.
