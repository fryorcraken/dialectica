# Correctness review — e2e-successful-join

Scope: correctness only (this piece was reviewed dimension-by-dimension; see
`tasks.md`'s four `code-reviewer` rows for security/readability/architecture,
each a separate dispatch).

## What was checked

- Staged the SDK symlink (`nix build --inputs-from ./dialectica
  logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`) and ran
  `cargo test --manifest-path dialectica/rust-lib/dialectica-core/Cargo.toml`
  in full: every test green, including the three new tests in
  `tests/seeded_reference.rs`. Re-ran `--test seeded_reference` in isolation
  to confirm they pass standalone, not only as part of the whole-suite run.
- `cargo clippy --manifest-path dialectica/rust-lib/dialectica-core/Cargo.toml
  --all-targets -- -D warnings`: clean (covers the new test file).
- `rustfmt --check --edition 2021
  dialectica/rust-lib/dialectica-core/tests/seeded_reference.rs` directly:
  clean. (`cargo fmt -- --check` on the whole package reports unrelated
  pre-existing drift in `identity.rs` and `wire.rs`, neither touched by this
  diff — matches the known `dialectica-ci-fmt-gap` note that CI's fmt gate
  does not follow the path dependency into this crate. Not this piece's
  defect.)
- Verified `seeded-join.yaml`'s pasted reference string is the only
  occurrence of the `{"stoa":"` opening pattern in the file (`grep`), matching
  design.md D2's textual-read assumption.
- Traced every `objectName` the new spec clicks or reads
  (`pasteField`, `pasteButton`, `foundingTitleText`, `fallbackNote`,
  `joinButton`, `joinedPanel`, `joinFailurePanel`, `joinCancelButton`,
  `shareButton`, `openStoaButton`) to its declaration in `DJoinScreen.qml` /
  `DStoaListScreen.qml`; all exist except `joinCancelButton`, which is exactly
  the one name this diff adds.
- Traced every root handle the spec reads (`screenShown`, `listReadState`,
  `stoaCount`, `joinState`, `joinFailure`, `listedStoas`, `feedReadState`,
  `feedRowCount`) to `Main.qml`; all pre-exist, confirming design.md's "no new
  root handles" claim.
- Ran `dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_navigation.qml`
  as committed: 28 passed, 0 failed (includes the two new tests
  `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` and
  `test_declining_the_join_preview_leaves_the_list_with_no_join_call`).
  Confirmed the helpers the new tests use (`spec.bridgeFor`, `spec.callsTo`,
  `spec.stoaA`, `spec.visibleNamed`) all pre-exist in the file rather than
  being newly introduced and untested themselves.
- Attempted to independently reproduce the D6/tasks.md-4.1 mutation (removing
  `list.reload()` from `Main.qml`'s `onJoined`) to verify the new component
  test catches it. **The Edit was refused by the harness's own permission
  classifier** ("Modify Shared Resources") before any change was written;
  `git status` confirms the tree is unmutated. Recorded as unverified rather
  than retried through another tool, per instructions. This does not
  contradict anything: tasks.md 2.1 and 4.1 already record this exact
  mutation applied and reverted in two separate real CI runs (36238223919 red,
  36238723726 green), with matching predicted-vs-observed step numbers and
  call logs, which is stronger evidence than a local repro would have added.
- Checked the matrix/count-gate wiring: `ui-tests.yml`'s matrix gained
  `seeded-join` alongside the existing five, and the "every spec in the tree
  is in the matrix" step derives its expectation from `strategy.job-total`
  (not a hardcoded number), so the count check moves with the matrix rather
  than needing a second edit.
- Read `stoa-navigation-view` and `view-navigation`'s spec deltas against the
  new component tests and the new spec's steps; the scenarios line up with
  what is actually asserted (join-not-held reported as joined; return
  survives a successful join; joined Stoa reaches the list without a
  restart).

## Findings

None. The seeded reference decodes, hashes to the address it is paired with,
and both `get_stoa` and `join_stoa` accept it against an empty store — checked
by running the tests myself rather than trusting the recorded run. The textual
spec-reader in `seeded_reference.rs` (D2) has exactly the one blind case
design.md's Risks section already names (a reference duplicated into a
comment in an escaped form) and no other. The one behavioural code change
(`joinCancelButton`) and the one test-only addition
(`tst_navigation.qml`'s two new functions) are both consistent with what they
claim to pin, and the CI wiring changes (`ui-tests.yml` matrix,
`join.yaml`'s doc paragraph) are consistent with each other and with what
actually runs.

- [x] **none** — no correctness defects found in this piece's diff
      (`git diff 2bda577...HEAD`). See "What was checked" above for what was
      run rather than merely read.
