# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

### 1. A failed keep leaves no master key behind (`design.md` D8)

- [x] 1.1 Re-point `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`
      at the delta's "no master key that was not there before", and watch it fail
      against the unfixed keep (it did, at that assertion, with the fixture guard
      passing).
- [x] 1.2 Add `undo_a_keystore_this_keep_wrote`, removing only a keystore the same
      keep created; verify with 1.1 and with
      `a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`.
      The flag-removing mutation for the second test was refused by the permission
      checker, so its discrimination is argued, not measured.

### 2. Core: the machine key is the identity in use (`design.md` D3–D7)

- [x] 2.1 `posting_identity`, `publishing_key`, `whoami_for`, `who_am_i` and
      `get_capabilities_from_stores` take no record (and the first three no Stoa);
      all resolve through `Keystore::identity_key`. Verify with
      `one_machine_key_posts_replies_and_votes_in_two_stoas` (issue #149's
      two-Stoa regression test) and
      `a_recorded_per_stoa_choice_does_not_change_the_key_in_use`.
- [x] 2.2 `Whoami::Identity` drops `path`; `recoveryNeedsTheRecord` is `false`.
      Verify with `the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
      and `who_am_i_reports_that_the_master_key_alone_recovers_the_identity_in_use`
      (a Stoa with a recorded choice and one without).
- [x] 2.3 Delete `NO_CHOICE_FOR_THIS_STOA`; replace the tests pinning it with
      `the_identity_in_use_needs_no_choice_recorded_for_the_stoa` and
      `the_probe_and_whoami_refuse_alike_when_no_master_key_is_stored`.
- [x] 2.4 Re-point the tests that bound a keep to `whoAmI` at the record
      (`a_kept_identity_survives_a_restart_and_is_the_one_the_record_reproduces`,
      `a_record_restored_beside_a_master_key_reproduces_the_kept_choice`).
- [x] 2.5 `the_creator_a_creation_names_is_the_identity_the_probe_reports` reaches
      the probe through `get_capabilities_from_stores`, the adapter's own call, and
      asserts the creator's post is authored by `genesis.creator`.
- [x] 2.6 Invert the seeding e2e test and `examples/seed_store.rs`, which pinned the
      three-derivations gap to fail when it closed; the founder signs with
      `wire::publishing_key` and no `chosen_paths` row is seeded. Verify with
      `the_seeding_sequence_builds_a_nested_thread_whose_author_the_probe_reports`.
- [x] 2.7 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
      -p dialectica-core`, clippy `--all-targets -D warnings`, and fmt on the
      touched files are green.

### 3. Adapter (`cfg(logos_scaffold)`, compiled only by `nix build`)

- [x] 3.1 `publishing`, `get_capabilities` and `who_am_i` stop opening the record;
      `publishing` keeps `core::stoa_of` as the envelope check ahead of the
      keystore (D7). Verify with `nix build path:./dialectica#lgx`.
- [x] 3.2 Update CI's adapter-gate comments to describe why `core::stoa_of` and the
      probe's entry point remain; every `WANTED` name is still present.

### 4. View: route to the Stoa list, withhold the onboarding screen (`design.md` D9)

- [x] 4.1 `Main.qml`: remove the `onboarding` state and the `DOnboardingScreen`
      mount; the feed's identity request reaches `acquireIdentity()`, which renders
      the list. Verify with `test_a_missing_identity_offers_the_route_to_the_stoa_list`
      and `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`.
- [x] 4.2 `qmldir` records `DOnboardingScreen` as `# UNINSTANTIATED:` with the
      reason. Verify with `check_qml_reachable.py` and
      `test_the_per_stoa_onboarding_screen_is_instantiated_nowhere` (confirmed to
      fail with a hidden `DOnboardingScreen {}` mounted).
- [x] 4.3 `DIdentityChip`'s caption names no Stoa ("Create an identity"). Verify with
      `test_the_create_affordance_names_no_stoa`.
- [x] 4.4 The full QML suite, `check_qml_names.py` and `check_qml_members.sh` are
      green.

### Satisfied by construction

- "A record of per-Stoa choices that cannot be read MUST NOT prevent posting …
  and MUST NOT change the key in use" (`identity`), and the `identity` scenario
  "An unreadable record of choices does not prevent posting". The probe, the
  report and `publishing_key` take no record, so no record state can reach them.
  In the adapter, none of the three paths calls `Self::paths`. No test can make
  an unreadable record matter to functions that are never given one. The
  adapter half is visible only in the diff and to `nix build`.

  **tester, 2026-09-24:** added
  `wire::tests::an_unreadable_record_of_choices_does_not_prevent_posting`, which
  writes garbage bytes at the identity record's own default path (confirmed
  `IdentityStore::open` refuses it), then asks the probe, the report and a
  publish through the real wire entry points and asserts all three succeed and
  name the machine key. This is not a mutation-backed test: `Keystore` carries
  no path or directory, so no signature-preserving mutation of `posting_identity`,
  `whoami_for` or `get_capabilities_from_stores` can reach a record file at all —
  the same structural argument this bullet already makes, checked directly
  rather than argued. The test is the spec's scenario made concrete, not
  evidence the scenario is reachable by a narrower fault.
