# Correctness review — `e2e-created-stoa-flow`

Scope: correctness only (this reviewer was dispatched for one dimension).
Security, readability and architecture are each a separate reviewer's row.

## What was checked

- The D8 fix (`Main.qml` withholding `feed.stoaAddress` on
  `feed.stoaGenesis === root.chosen.genesis`, and `thread.threadId` on the
  matching pair for `reading`), traced through every reachable transition:
  opened from the list, back from a thread, back from moderation, and the
  (unreachable) direct re-open case.
- The `FeedScreen.visibleRows` reshaping (D3) and the seven new `Main.qml` root
  handles, against `tst_e2e_handles.qml`.
- The four new sitometres specs (`create.yaml`, `feed.yaml`, `thread.yaml`,
  `moderation.yaml`) for step-count consistency with `tasks.md`'s predictions
  and for any step that could send a read without the record.
- The D9 `wait_for:` fix and whether the same layout-shift race could reach
  `create.yaml`'s controls (it can't: `createStoaButton` isn't conditionally
  positioned by draft content the way a composer's submit is).
- That every PROOF BREAK recorded in `tasks.md` section 8.1 leaves no residue:
  `git diff fe093be...HEAD` on `DModerationScreen.qml`, `DStoaListScreen.qml`,
  `FeedScreen.qml` matches exactly what design.md/tasks.md describe as kept.
- Input handling for the values that flow into the new guard:
  `rememberGenesis`/`genesisFor` always hand `chosen.genesis` a string (never
  `undefined`), so the guard's `===` comparison can't wedge on a type
  mismatch; two different Stoas can't collide on the same genesis (the
  address is the hash of the genesis bytes).

## What was mutated, and reverted

- `Main.qml`: removed the thread's guard (`threadId: root.reading !== null ?
  root.reading.rootOp : ""`, dropping the `stoaAddress`/`stoaGenesis` check),
  and ran `tst_navigation.qml`. Result: **26 passed, 0 failed** — no test
  reddens. This reproduces, rather than contradicts, what `design.md` D8
  already discloses under "What breaks without it, measured": *"The thread's
  guard is invisible in the committed binding order"*. Given the same real
  QML engine drives both the component suite and the e2e run, this also means
  `thread.yaml` cannot currently distinguish the guard's presence from its
  absence — the thread half of D8 is a defensive addition against an
  ordering QML does not promise, not a fix for an observed thread-side bug
  (only the feed side was ever reproduced live, in UI tests run
  36213442819). This is disclosed accurately by the author, not a hidden
  defect, so I'm not opening a checkbox for it — recording it here since it's
  exactly the class of question ("any state where the withheld trigger never
  fires") this review was asked to chase, and independent verification is
  itself worth having on record.
- All mutations reverted; `git diff` and `git status` confirm a clean tree
  before committing this file.

## Findings

None. I could not find a route onto the feed or thread that sends a read
without the record, nor a state where the withheld trigger never resolves.
The four new specs' step counts match `tasks.md`'s predictions exactly (12,
18, 23, 15), the matrix and its "every spec is in the matrix" check are
consistent, and no Rust/core code is touched (confirmed via `git diff --stat`).
