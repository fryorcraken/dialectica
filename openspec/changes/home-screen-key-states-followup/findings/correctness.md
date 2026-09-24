# Correctness review — home-screen-key-states-followup

Dimension covered: **correctness only** (per the dispatching prompt). Readability,
architecture and security are each another instance's row.

No findings. Every area in scope was checked and found correct, with the
verification recorded below rather than asserted. This file has no checkboxes,
which is deliberate — nothing here needs anyone to act on it.

## What was checked, and how

**The `2602da0` merge conflict resolutions.** The brief named two files with
textual conflicts: `openspec/specs/stoa-navigation-view/spec.md` and
`dialectica-ui/tests/tst_stoa_screens.qml`.

- Counted `### Requirement:` headers in both parents (`f9617cc`: 20, `98ff9a4`:
  18) and in the merge result (23), then listed every title in each with
  `git grep -n "^### Requirement:"` and diffed the three lists by eye. Every
  requirement from both parents is present exactly once in the merge (18 of
  98ff9a4's carried through, with "always offered" replaced by f9617cc's
  key-gated version as the commit message describes, plus this piece's four
  key-state requirements). No requirement was silently dropped or duplicated.
- Read the merged key-gated requirement
  ("Creating a Stoa asks for a title and nothing else, and is offered only
  once the core reports a key") in full and compared it against `f9617cc`'s
  text: byte-identical, confirming the merge commit's own claim.
- Read the two blank-title create tests in the merged test file
  (`test_every_blank_title_reaches_the_core_rather_than_being_refused_here`,
  `test_the_placeholder_is_never_submitted_as_a_title`). Both correctly answer
  `get_master_key` with a held-key reply before exercising `create_stoa`,
  which is required since #155 made the create affordance exist only in the
  key-held state — a real, non-trivial resolution, not a mechanical merge.
- Diffed `Core.qml`, `DStoaListScreen.qml` and `DJoinScreen.qml` between
  `f9617cc` (pre-merge tip of this piece) and `2602da0` (post-merge). #154's
  additions to all three are purely additive (new functions/properties/panels
  in `Core.qml` and `DJoinScreen.qml`; comment-only changes in
  `DStoaListScreen.qml`). None of the three Loaders that gate this piece's
  key-state blocks (`keyBlockLoader`, `keyUnreadableLoader`,
  `createBlockLoader`, `keyLineLoader`) were touched by the merge.
- Confirmed `pasteSection` (the fixture `4b29627`'s test protects) is still
  declared unconditionally, outside every Loader, in the merged
  `DStoaListScreen.qml`.
- Confirmed `DJoinScreen.qml` has no reference to `machineKey`, `get_master_key`
  or any key-state name — it is genuinely independent of the key-state
  machinery, so the merge had no seam to get wrong between the two.

**How this piece interacts with #153 and #154 on `main`.** Read `6aa76d7`,
`59dbd0c`, `fca9199`, `18ec847`, `4296bc5` in full (the mid-review composition
with #153) and `f9617cc`, `2602da0` (the mid-review composition with #154).

- Verified in `Main.qml` that `acquireIdentity()` and `closeFeed()` are
  word-for-word the same body (`root.enterOnly("", null)`), and that
  `DStoaListScreen` is mounted with `visible: root.screenShown === "list"` —
  so both really do produce the same feed→list transition and the same
  `onVisibleChanged` re-showing. `18ec847`'s
  `test_following_the_route_makes_no_call_of_its_own`, which relies on this
  equivalence, is therefore testing a real invariant, not a coincidence.
- **Mutated** `acquireIdentity()` to call `Core.whoAmI()` before entering the
  list (the exact regression `18ec847`'s commit message describes) and reran
  `tst_navigation.qml`. Result matched the commit's claim exactly:
  `test_following_the_route_makes_no_call_of_its_own` failed
  (`Actual: [who_am_i,get_master_key]`, `Expected: [get_master_key]`), and
  `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`
  stayed green, since `who_am_i` isn't one of its three named calls. Reverted
  immediately after observing the failure; `git diff --stat` after revert
  showed no changes.
- **Mutated** `createBlockLoader`'s gate in `DStoaListScreen.qml` from
  `active: screen.machineKey.state === "held"` to
  `active: screen.machineKey.state !== "no-key"` (wrongly including the
  could-not-be-read state) and reran `tst_stoa_screens.qml`. Three tests failed
  as expected
  (`test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words`,
  `test_a_reply_claiming_a_key_without_naming_one_is_not_the_key_held_state`,
  `test_the_create_affordance_is_not_instantiated_when_no_key_is_held`),
  confirming the could-not-be-read/no-key/held distinction the merged create
  gate depends on is still under real test pressure after the merge. Reverted;
  `git diff --stat` confirmed clean.
- Read `f9617cc`'s and `2602da0`'s reconciliation of the blank-title rule
  (#154's "blank" definition replacing this piece's "empty") end to end. The
  live spec's language is consistent throughout — no leftover reference to
  "empty title MUST be accepted" anywhere in the merged
  `stoa-navigation-view/spec.md`.

**What the pre-rebase round could not have seen.** All ten commits
`2480536^..aac415b` were read in full (not just the two the brief flagged as
"small finding fixes"). Nothing in `4b29627` (paste-availability test),
`9efae86` (archive), `6aa76d7`/`59dbd0c` (the #153 navigation-test fix),
`fca9199`/`18ec847`/`4296bc5` (the view-navigation spec split and its test),
or `f9617cc`/`2602da0` (the #154 reconciliation and merge) introduces a
correctness defect. Each fix is backed by a measured before/after (mostly
already recorded in the commit messages, and re-verified above by mutation
rather than taken on faith).

## Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`:
  1141 + 30 passed, 0 failed.
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`:
  136 passed, 0 failed (both before mutation and after revert).
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_navigation.qml`:
  24 passed, 0 failed (both before mutation and after revert).
- `sh dialectica-ui/tests/run-qml-tests.sh` (full suite, no argument): 25 spec
  files, all green. Two pre-existing `QWARN`s in `tst_vote_and_gate.qml`
  (`SanitisedText.qml:30:5: Unable to assign QString to int`) are unrelated to
  this piece — the tests they occur in still PASS, and `SanitisedText.qml` is
  untouched by any commit in scope.
- `lgs basecamp build`, run plainly from the worktree root: succeeded,
  producing both `lgx` and `lgx-portable` artefacts for `dialectica` and
  `dialectica_ui`.

No gate was skipped or could not be run.
