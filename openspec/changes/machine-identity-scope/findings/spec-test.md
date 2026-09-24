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

- [ ] **`spec-writer`** — `dialectica-ui/tests/tst_identity_chip.qml:229` (marked `NO SPEC`)
      The `view-identity-onboarding` requirement constrains the create-identity
      affordance to name no Stoa, but the exact caption ("Create an identity")
      is the dev's choice; nothing in the spec text picks it.
      **Scenario:** a future rewrite to any other Stoa-free caption (e.g. "Set
      up an identity") is equally spec-compliant and would still pass
      `test_the_create_affordance_names_no_stoa`, which asserts the absence of
      "stoa" rather than the presence of one literal string.
      **Measured:** read only — the test's own comment names the gap and cites
      the covering test; no code was run for this one.

- [ ] **`spec-writer`** / **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:695` (marked `NO SPEC`, inside `undo_a_keystore_this_keep_wrote`, added by design.md D8)
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

Everything else in scope — the RENAMED/MODIFIED requirement pairs, the
`identity-onboarding` REMOVED-then-replaced route in `view-navigation` (a
genuine behavior change with its own new requirement and migration note, not a
silent drop), and the issue's own four asks (§"What this needs") — reads
clean against the issue re-fetched via `gh issue view 149`.
