# Correctness review — machine-identity-scope

Scope: correctness only (per dispatch). Covered: whether every posting/replying/
voting/whoAmI/getCapabilities path now uses the one machine key in every Stoa;
whether the D8 keep/undo paths leave disk in the state the spec requires;
reachable panics; whether the `cfg(logos_scaffold)` adapter (`dialectica/rust-lib/src/lib.rs`)
matches core (`dialectica-core/src/wire.rs`).

## Method

- `git diff 041eaff...HEAD` read in full for `wire.rs`, `lib.rs`,
  `examples/seed_store.rs`, `tests/end_to_end.rs`, the QML views (`Main.qml`,
  `FeedScreen.qml`, `DIdentityChip.qml`, `qmldir`), `ci.yml`, and the four spec
  deltas.
- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: 1094 + 30 tests, all green, no new warnings beyond the
  pre-existing `logos-rust-sdk` ones.
- `sh dialectica-ui/tests/run-qml-tests.sh` against `tst_navigation.qml`,
  `tst_identity_chip.qml`, `tst_thread_navigation.qml`: all green (23 + 17 + 12
  passed).
- `nix build path:./dialectica#lgx`: succeeded — this is the only gate that
  compiles the `cfg(logos_scaffold)` adapter in `lib.rs`, and it built clean.
- Independently reproduced design.md's D8 mutation claim rather than taking it
  on trust: changed `if wrote_it {` to `if true {` in
  `undo_a_keystore_this_keep_wrote`, ran the `a_keep_whose*` test group. Result
  matched the design doc exactly —
  `a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`
  failed (`left: None, right: Some([...keystore bytes...])`), the other two in
  the group stayed green. Reverted immediately; `git status --short` confirmed
  a clean tree before writing this file.
- Searched for `.unwrap()`/`.expect()` added by this diff outside `mod tests`
  (which starts at `wire.rs:3410`) — none found. Diffed `lib.rs` for
  `unwrap`/`expect` — none added.
- Checked the QML side does not read the now-removed `path` field:
  `Core.qml`'s `identityFrom()` reads only `hasIdentity`, `publicKey` and
  `reason`.

## Findings

No correctness defects found. Nothing to tick.

The core signature changes (`posting_identity(keystore)`, `publishing_key
(keystore)`, `whoami_for(master)`, `who_am_i(request, master)`,
`get_capabilities_from_stores(request, open_keystore)`) all lose their Stoa/
record parameters exactly as design.md D3 describes, and every call site in
`lib.rs` was updated to match — `Self::paths` is no longer passed to
`get_capabilities`/`who_am_i`/`publishing`, and is still (correctly) used by
`keep_identity`, which D2 leaves working as before. `one_machine_key_
posts_replies_and_votes_in_two_stoas` and `a_recorded_per_stoa_choice_does_
not_change_the_key_in_use` both exercise the full six-op/two-Stoa and
kept-choice-is-ignored scenarios through the wire, and both pass.

D8's `undo_a_keystore_this_keep_wrote` correctly discriminates on `wrote_the_
keystore`/`wrote_it`, set once in `keep_selection`'s own create-vs-exists
branch: a keystore this call did not write is never removed
(`a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`),
and one it did write is removed on a subsequent record-write failure
(`a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`). Both
directions are tested and both pass; the flag's discrimination is now measured
twice (once by the tester's prior pass, once independently here) rather than
argued.

The adapter (`lib.rs`) matches core at every touched call site: `publishing`
reads `core::stoa_of` for the envelope check alone (D7) and never opens the
per-Stoa record; `get_capabilities`/`who_am_i` are handed only a keystore
opener. `nix build path:./dialectica#lgx` compiles this cleanly, which is the
only gate that does (per `design.md`'s own note that the adapter half of D3 is
visible to no test).

No reachable panic was introduced. All `unwrap()`/`expect()` additions are
inside `#[cfg(test)] mod tests`. The handlers stay wrapped in `core::guarded`,
proven by the existing "opener panics" tests, which still pass with the
reduced parameter lists.

The view routing (`Main.qml`, `FeedScreen.qml`, `DIdentityChip.qml`, `qmldir`)
matches design.md D9: the `onboarding` state, `createIdentityFor`,
`closeOnboarding` and `identityWasKept` are removed together (no orphaned
reference to any of them remains — checked via `git grep`), `acquireIdentity()`
is `enterOnly("", null)`, and `DOnboardingScreen` is recorded as
`UNINSTANTIATED` in `qmldir` and is mounted nowhere in `Main.qml`.
`tst_navigation.qml`'s
`test_the_per_stoa_onboarding_screen_is_instantiated_nowhere` passes.

Areas outside this diff (`identity_store.rs`, `keystore.rs`, `authoring.rs`)
are untouched, as design.md's Non-Goals state, and I found nothing that
depends on them changing.
