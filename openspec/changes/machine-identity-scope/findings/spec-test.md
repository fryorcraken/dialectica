# spec-test review — machine-identity-scope

Scope: `openspec/changes/machine-identity-scope/specs/*.md` against the tests in
`dialectica/rust-lib/dialectica-core/src/wire.rs`'s `mod tests`,
`dialectica/rust-lib/dialectica-core/tests/end_to_end.rs`, and
`dialectica-ui/tests/tst_*.qml`. Implementation was not read, except the two
lines mutated below.

## Issue #149's regression test — confirmed by mutation

`wire::tests::one_machine_key_posts_replies_and_votes_in_two_stoas` exercises
**two different Stoas** (`stoas[0]`/`stoas[1]`, asserted `assert_ne!`), asks the
probe and `whoAmI` about each, publishes a post/reply/vote into each, and reads
the six ops' authors back out of the log — all through the real wire entry
points (`get_capabilities_from_stores`, `who_am_i`, `publish_post/reply/vote`),
which is the right layer for a claim about what an op is signed with.

Mutated `posting_identity` (`wire.rs:344`) from
`publishing_key(keystore).public_key().to_hex()` to a per-Stoa-scheme
derivation (`keystore.stoa_public_key(&stoa).to_hex()` against a fixed
address, since the function's signature — deliberately — carries no Stoa to
vary the derivation by). Ran
`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core
one_machine_key_posts_replies_and_votes_in_two_stoas`:

- **Before the mutation:** passed.
- **After:** failed immediately at `wire.rs:6403` (`probe["identity"] ==
  machine_key`, first Stoa, first assertion in the loop) — `left:
  "7f439b…"` (the per-Stoa-scheme key) `right: "35c1c8…"` (the machine key).
- **Reverted** (`git diff` against `HEAD` is empty on this file; confirmed
  with `git status --short` before committing).

The test cannot pass against a reintroduced per-Stoa derivation. One
mutation was run, per the effort budget; the tree carries no mutation now.

## Coverage — clean

Walked all six `identity` scenarios, the modified `identity-onboarding`
scenarios (unbacked-state report, no-path-in-reply, restore, malformed input,
second-keep refusal, nothing-stored-before-keep), the three `content-authoring`
scenarios, and all `view-navigation`/`view-identity-onboarding` scenarios
against `tasks.md`'s own test names, then read each test. All map to a test at
the right layer:

- `identity`'s six scenarios: `one_machine_key_posts_replies_and_votes_in_two_stoas`,
  `a_recorded_per_stoa_choice_does_not_change_the_key_in_use`,
  `an_unreadable_record_of_choices_does_not_prevent_posting`, and
  `the_creator_a_creation_names_is_the_identity_the_probe_reports` (a Stoa's
  creator posts as its creator — mutation-verified per its own comment: the
  probe's lookup was changed to the pathless per-Stoa scheme and this test was
  the one that failed against 956 lib tests passing).
- `content-authoring`'s "A request naming an author is refused" is covered
  more broadly than the delta's own scenario by the pre-existing
  `a_forbidden_field_is_refused_on_every_operation`, which checks `author`,
  `identity`, `key` (and two more fields) across all three publish handlers.
- `view-navigation`'s three added scenarios map 1:1 to
  `test_a_missing_identity_offers_the_route_to_the_stoa_list`,
  `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`,
  and `test_the_per_stoa_onboarding_screen_is_instantiated_nowhere`
  (`tst_navigation.qml`), each driven through `Main.qml` rather than a
  narrower screen test — the right layer for a cross-screen routing claim —
  plus the static `check_qml_reachable.py` gate reading the `qmldir`
  `# UNINSTANTIATED:` line for `DOnboardingScreen`.
- `view-identity-onboarding`'s "affordance is not a per-Stoa choice" is
  `test_the_create_affordance_names_no_stoa` (`tst_identity_chip.qml`); "a kept
  candidate is announced" / "no other outcome announces one" is
  `test_the_kept_signal_fires_only_on_a_reply_that_kept_something`
  (`tst_onboarding_states.qml`); "no stored flag stands in for the module's
  answer" is `test_both_probes_are_re_read_on_every_render`
  (`tst_navigation.qml`).

No scenario in the five spec deltas was found untestable as written, and no
test read as a fixture that agrees with itself (the `"ab"`/`"abc"`,
hash-moved, or position-only-pinned shapes named in the review brief).

## Findings

- [x] **`spec-writer`** — `dialectica-ui/tests/tst_identity_chip.qml:229` (marked `NO SPEC`)
      The `view-identity-onboarding` requirement constrains the create-identity
      affordance to name no Stoa, but the exact caption ("Create an identity")
      is the dev's choice; nothing in the spec text picks it.
      **Scenario:** a future rewrite to any other Stoa-free caption (e.g. "Set
      up an identity") is equally spec-compliant and would still pass
      `test_the_create_affordance_names_no_stoa`, which asserts the absence of
      "stoa" rather than the presence of one literal string.
      **Measured:** read only — the test's own comment names the gap and cites
      the covering test; no code was run for this one.
      **Outcome (`spec-writer`): rejected — the marker stays, as a decision.**
      The caption is copy, not behaviour: it is `copy.json`
      `common.createIdentity`, owned by the design bundle, and a spec that
      pinned the literal would fail on a reword rather than on a caption that
      misinforms. What the contract has to hold is the relation, and it does:
      the scenario requires the affordance to be offered (the presence half,
      which the test's `indexOf("Create an identity")` precondition discharges
      as a fixture guard) and to name no Stoa (the absence half). A rewrite to
      "Set up an identity" passing the suite is the intended outcome, not a
      gap. No spec change; no work for `tester` or `dev-writer`.

- [x] **`spec-writer`** / **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:695` (marked `NO SPEC`, inside `undo_a_keystore_this_keep_wrote`, added by design.md D8)
      What a keep reports when its own undo — removing the master key that
      same keep just wrote, after the *record* write failed — itself fails to
      remove the file is unspecified, and unlike its sibling branches (pinned
      by name in `tasks.md` 1.1/1.2:
      `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`,
      `a_keep_whose_record_write_fails_keeps_a_master_key_that_was_already_there`),
      no test exercises this branch at all — not even by argument.
      **Scenario:** a keep whose record write fails on a filesystem where the
      just-written keystore file cannot then be removed (e.g. a concurrent
      process holding it open, or a permissions race) reaches this line, and
      the compound reason string it composes has never been produced by a
      test — a rewording of it would ship unnoticed either way.
      **Measured:** read only; this is the code path `tasks.md` 1.2 already
      flags as un-mutation-tested for a different reason (the permission
      checker refused the flag-removing mutation for its *sibling* test), and
      this second branch is not mentioned there at all.
      **Outcome (`spec-writer` half): fixed.** The box is left unticked for the
      `tester`'s half. `specs/identity-onboarding/spec.md`, requirement
      "Keeping a candidate persists it, and keeping is one step", now names the
      one exception to "complete or change nothing": a keep that stored a master
      key where none was, then failed, MUST remove it and MUST NOT remove one
      that predates it; where storage refuses the removal, the reply MUST carry
      a reason and no identity, and the reason MUST state that a master key was
      left stored, so it differs from the reason the same failure gives when the
      removal succeeds. "A failed keep records nothing" is narrowed to "storage
      permits removing any master key the keep stored" (it contradicted the new
      case as written), and a new scenario, "A failed keep that cannot remove
      the master key it stored says so", carries the rest. The proposal's
      `identity-onboarding` bullet says so. `openspec validate
      machine-identity-scope --strict` passes. The current code already meets
      this; the only `dev-writer` work is that the `NO SPEC:` comment in
      `undo_a_keystore_this_keep_wrote` is now stale and should be replaced by
      a citation of the new scenario. **`tester` now has to pin:** with the
      master key removal refused, the reply is the refusal shape with a reason
      and no identity, and that reason is **not equal** to the reason the same
      record failure gives when the removal succeeds — assert the relation, not
      the literal. Reaching the branch needs no filesystem race:
      `undo_a_keystore_this_keep_wrote` is callable from `wire.rs`'s own
      `mod tests`, and `std::fs::remove_file` refuses a path that is a
      directory, so `wrote_it = true` against a directory path reaches it
      deterministically; the tester decides whether that layer suffices or a
      `keep_selection`-level route exists.
      **Outcome (`dev-writer` half): fixed** in this commit; the box stays
      unticked for the `tester`'s half. The `NO SPEC:` comment in
      `undo_a_keystore_this_keep_wrote` is replaced by a citation of the
      scenario "A failed keep that cannot remove the master key it stored says
      so", naming the relation it requires (the reason differs from the bare
      `record_failure` the same failure reports when the removal works). The
      function's docstring, which quoted "no master key that were not there
      before" unqualified, now carries the requirement's exception too. No
      code changed.

      **Outcome (`tester` half): fixed.** New test
      `wire::tests::a_failed_keep_that_cannot_remove_the_master_key_it_stored_says_so`,
      calling `undo_a_keystore_this_keep_wrote` directly as this box's own text
      suggested — a directory in place of the keystore path makes
      `std::fs::remove_file` fail deterministically, with `wrote_it: true`, no
      filesystem race needed. It asserts the relation the scenario requires
      rather than a literal: the returned reason is `assert_ne!` against the
      bare `record_failure.to_string()` (independently confirmed, before the
      call under test runs, to not already contain "master key" — the `Storage`
      arm's `Display` never does — so a pass cannot be explained by the fixture
      alone), and separately `assert!(reason.contains("master key"))` for the
      "states a master key was left stored" half. `Kept::Stored` is a
      `panic!`, not an unreachable branch, so a regression that started
      reporting success would fail loudly rather than mismatch a field.

      **Mutation, predicted vs. observed:** replaced the `if let Err(e) =
      std::fs::remove_file(...) { return Kept::Refused { reason: <augmented> }
      }` block with `let _ = std::fs::remove_file(keystore_path);` (swallowing
      the removal failure, falling through to the bare
      `Kept::Refused { reason: record_failure.to_string() }` every sibling
      branch already returns). Predicted: the new test fails on the
      `assert_ne!`, because both sides become the identical bare message.
      Observed: exactly that —
      `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
      dialectica-core
      a_failed_keep_that_cannot_remove_the_master_key_it_stored_says_so` failed
      with `assertion `left != right` failed` and `left`/`right` both equal to
      `"the identity record could not be read or written: boom; check the path
      and its containing directory"`. Prediction and observation agree.
      Reverted immediately after; `git diff --stat` against the commit this
      run started from shows only `wire.rs`, and the diff is exactly the new
      test (no implementation line touched) — confirmed with `git diff` and
      `git status --short` before committing.

      Full suite green after restoring: `cargo test --manifest-path
      dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` — 1095
      passed in `dialectica-core`'s lib tests (including the new one), 30 in
      `end_to_end`, 0 failed anywhere.

      No permission-classifier refusal was hit for this mutation — the Edit
      tool applied and reverted it without a block.

Everything else in scope — the RENAMED/MODIFIED requirement pairs, the
`identity-onboarding` REMOVED-then-replaced route in `view-navigation` (a
genuine behavior change with its own new requirement and migration note, not a
silent drop), and the issue's own four asks (§"What this needs") — reads
clean against the issue re-fetched via `gh issue view 149`.
