# Correctness review — `e2e-ui-suite`

Scope: correctness dimension only (this instance was dispatched for
correctness; security, readability and architecture are covered elsewhere).

## Method

Read the full diff (`git diff origin/main...HEAD`), read GitHub issue #134,
and cross-checked every claim the diff makes against the actual QML/Python it
points at:

- Verified every `objectName` the new spec (`dialectica-ui/tests/ui/join.yaml`)
  and the new probe (`dialectica-ui/tests/tst_e2e_handles.qml`) reference
  exists in `DStoaListScreen.qml` / `DJoinScreen.qml` / `Main.qml`.
- Verified every core method name (`list_stoas`, `get_master_key`, `get_stoa`,
  `join_stoa`) the fake bridge in `tst_e2e_handles.qml` stubs matches
  `Core.qml`'s actual wire method names.
- Ran `python3 dialectica-ui/tests/tst_adjudicate_ui_run.py` and
  `python3 dialectica-ui/tests/tst_scaffold_values_unchanged.py` directly —
  both pass genuinely (not by construction: `tomlq -S`/`diff`/`pyyaml` are
  actually invoked).
- Ran `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_e2e_handles.qml`
  and `dialectica-ui/tests/tst_navigation.qml` — both green, confirming the new
  `Main.qml` handles don't regress existing navigation coverage.
- Mutation-tested the five new `Main.qml` root handles by hand (QML, not Rust,
  so `cargo mutants` does not apply — no Rust files are in this diff).
- Manually counted `join.yaml`'s steps (14) against design.md D5's claim of 14,
  and confirmed the CI matrix (`spec: [join]`, one entry) against the one file
  in `dialectica-ui/tests/ui/`.

Areas checked and found clean, stated in prose rather than as boxes: the
`adjudicate-ui-run.py` three-condition logic (verdict / step-count / per-step
pass) is correct against its own paired fixtures and against the "more steps
than expected" direction, which is the direction easiest to get wrong with a
careless `<` instead of `!=`. The `Main.qml` handle bindings
(`listReadState`, `pasteFailure`, `joinState`, `joinFailure`) are wired to the
correct source properties on the correct screens, verified both by reading
the source and by mutating each one to a constant and watching
`tst_e2e_handles.qml` catch it. The `ui-specs`/`ui-tests.yml` split and the
"every spec is in the matrix" bidirectional count check are logically sound.
The `role = "dependency"` claim in `ci.yml`'s rewritten "NOTHING INSTALLS…"
comment is backed by a real, pre-existing lint assertion (`ci.yml:479-482`),
not a new unverified claim.

## Findings

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_e2e_handles.qml` — the
      `stoaCount` handle's pinning test cannot distinguish the guarded
      projection (`list.visibleRows.length`, what `Main.qml` actually binds)
      from an unguarded one (`list.lastListing.length`) in the one failure
      scenario the file tests.

      **Scenario:** `DStoaListScreen.qml:30-37` documents that `lastListing`
      is deliberately left holding the previous successful listing across a
      *failed reload* ("a failure must not blank a good listing underneath a
      banner") — i.e. after a Stoa is listed once and a later reload fails,
      `lastListing.length > 0` while `readState === "failed"`. `visibleRows`
      is the guard that turns that into 0 for any reader outside the screen;
      it is exactly the property `stoaCount` is supposed to key off, per
      design.md D6 ("None is a copy") and the file's own docstring ("a handle
      bound to the wrong source … fails here").

      `test_a_failed_listing_is_not_read_as_ok` only exercises the *first*
      listing failing (a fresh screen, `lastListing` still at its initial
      `[]`), so `lastListing` and `visibleRows` agree (both 0) and the test
      cannot tell which one `Main.qml` is reading from.

      **Measured:** with `Main.qml`'s
      `readonly property int stoaCount: list.visibleRows.length` changed to
      `list.lastListing.length` (a plausible regression — it's the "obvious"
      thing to write before recalling the readState guard), all 6 checks in
      `tst_e2e_handles.qml` still pass, including
      `test_a_failed_listing_is_not_read_as_ok`. Reverted after measuring;
      the tree is currently clean (confirmed with `git diff --stat`).

      A fix is a fixture that lists successfully once (populating
      `lastListing`), then fails a reload, and asserts `stoaCount === 0` in
      that state specifically — the case `visibleRows` exists to cover and
      the only one that can tell the two properties apart.

      Severity: medium. Not a shipped defect — `Main.qml` currently binds the
      right property — but the regression test that exists specifically to
      pin these bindings against drift has a gap in exactly the failure mode
      (a stale-but-nonempty copy read past its guard) this codebase has
      shipped before (per CLAUDE.md's "dialectica's one test defect family").

      **Fixed** in the commit that ticks this box: new
      `test_a_failed_reload_does_not_count_the_listing_it_kept` lists two
      rows, switches the fake's `list_stoas` reply to an error, calls
      `list.reload()`, and asserts `listReadState === "failed"`,
      `list.lastListing.length === 2` (the precondition that makes the case
      discriminate), then `stoaCount === 0`. Mutation, run by me on this
      tree: `stoaCount: list.lastListing.length`. Before this change, all 6
      checks stayed green (your measurement, confirmed: the five pre-existing
      tests still pass under the mutation). After it, the new test fails
      (`Actual 2, Expected 0`, `tst_e2e_handles.qml:110`) and nothing else
      does. Reverted, and 7/7 are green on the real binding. Recorded in
      design.md D6 and tasks.md 3.2.

## Confirmed correct by direct mutation (no action needed, recorded for the record)

- `joinFailure: join.failure` → constant `""`: caught
  (`test_the_join_handles_follow_the_join_screen` fails with a clear diff).
- `listReadState: list.readState` → constant `"ok"`: caught
  (`test_a_failed_listing_is_not_read_as_ok` fails with a clear diff).

Both mutations were reverted before this file was written; the working tree
carries no leftover mutation.
