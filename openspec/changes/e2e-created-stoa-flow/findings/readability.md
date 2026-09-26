# Readability review — `e2e-created-stoa-flow`

Dimension covered: **readability only** (correctness, security and
architecture are other reviewers' rows).

## Scope read

Full diff `git diff fe093be...HEAD`: `Main.qml`, `FeedScreen.qml`,
`DComposer.qml`, `DThreadScreen.qml`, `DStoaListScreen.qml`, the four new
sitometres specs (`create.yaml`, `feed.yaml`, `thread.yaml`,
`moderation.yaml`) and the `join.yaml` prose edit, the new/extended QML tests
(`tst_e2e_handles.qml`, `tst_navigation.qml`, `tst_composer.qml`,
`tst_stoa_screens.qml`), `ui-tests.yml`'s one-line matrix change, `design.md`
D1–D9, `proposal.md`, `tasks.md`, and the two spec deltas
(`feed-view`, `stoa-navigation-view`) plus the live `feed-view` Purpose edit.
No Rust file changed in this piece (`git diff fe093be...HEAD --stat -- '*.rs'`
returns nothing), so `cargo mutants` does not apply here.

## No checkbox findings

I did not find a defect to raise as a blocking checkbox. What follows is why,
stated so the absence is checkable rather than asserted.

- **Naming is consistent and load-bearing.** The seven new `Main.qml` handles
  (`listedStoas`, `createdStoa`, `feedReadState`, `feedRowCount`,
  `feedCanPost`, `threadReadState`, `threadItemCount`) each read a name
  already used by the screen they project (`FeedScreen.visibleRows`,
  `DThreadScreen.items`), so the vocabulary difference between "rows" and
  "items" tracks a real difference in the underlying screens rather than an
  inconsistency introduced here. The `objectName`s added
  (`openStoaButton`, `<kind>DraftField`, `<kind>SubmitButton`,
  `readThreadArea`, `moderateArea`) each say what they are for in an adjacent
  comment, and each is exercised by a component test that would fail if the
  name moved to the wrong element (`tst_composer.qml`'s
  `test_the_field_and_the_submit_are_named_by_kind`,
  `tst_e2e_handles.qml`'s five mutation-backed tests).
- **Comments earn their place.** Every comment I read in the diff states a
  "why" a command cannot — the binding-order race in `Main.qml`/`FeedScreen.qml`/
  `DThreadScreen.qml` (D8), why `moderateArea`/`readThreadArea` are named on the
  `MouseArea` and not the label (D4, and it is not idle: `moderation.yaml`'s
  and `feed.yaml`'s own comments restate the concrete failure — a click aimed
  at the label resolving to the wrong handler), why the submit control needs a
  `wait_for` before the click (D9). None of the ones I checked merely restate
  the following line.
- **The self-referential binding in `Main.qml` (D8) is unusual but not
  under-explained.** `stoaAddress: root.chosen !== null && feed.stoaGenesis
  === root.chosen.genesis ? root.chosen.stoa : ""` makes the address binding
  depend on the record binding of the same element, which is a genuinely
  non-obvious QML idiom on first read. `design.md` D8 gives it a 30-line
  treatment (what went wrong, why the two rejected alternatives — re-reading
  on both halves, `Qt.callLater` — don't work, and why the "one object"
  reshape is deferred rather than done here), and the same reasoning is
  repeated at the binding site and in `FeedScreen.qml`/`DThreadScreen.qml`
  where the trigger side lives. This is a case where the pattern is
  legitimately hard to read and the comments are doing real work rather than
  papering over a shape that should have been reshaped; I read it as
  accounted for, not overlooked, and the file names correctly (a third screen
  reading a pair) is the trigger for the reshape design.md already commits to.
- **The four-file duplication (D1) is a recorded trade-off, not an oversight.**
  `create.yaml`, `feed.yaml`, `thread.yaml` and `moderation.yaml` repeat an
  eight-step key-and-Stoa prefix verbatim. That is the kind of "fourth
  slightly-different guard" CLAUDE.md asks a reviewer to flag — except
  `design.md` D1 already weighed the one-chained-file alternative, measured why
  it fails (a report that goes `inconclusive` past the first failure, hiding
  three of four screens behind one break), and named the cost in runner
  minutes. I have nothing to add to that trade-off, so I am not opening a box
  for it.
- **Decision citations name their change.** Every `design.md D<n>` citation
  inside this piece's own `design.md`, `proposal.md` and `tasks.md` that
  points at a *different* change's design.md names it (`` `e2e-ui-suite`
  change's design.md D6 ``, `` `e2e-suite-review` design.md D2 ``, etc. —
  checked with `grep -n "design.md D" design.md`); a bare `design.md D8` inside
  this change's own files is a same-file self-reference, which is the
  expected shape rather than a violation of the citation rule.
- **The spec deltas read cleanly as prose.** `stoa-navigation-view`'s
  REMOVED/ADDED pair states its own reason for the split (OpenSpec refuses a
  MODIFIED block that drops a scenario) rather than leaving a reader to infer
  it, and the migration note is explicit about what carries across unchanged.
  `feed-view`'s new requirement and the Purpose-line edit read as a single
  coherent addition.

## What I did not check

Correctness of the D8 binding-order fix itself (does it actually close the
race under every QML evaluation order), whether the moderation/thread specs'
citations to unarchived change folders are themselves accurate, and any
security implication of the e2e handles being read-only surface — those are
the other reviewers' rows.
