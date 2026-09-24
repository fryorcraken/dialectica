# Security review — home-screen-key-states-followup

Scope: security dimension only, as directed. Tree forked from `0cbe1d4` (PR
#155 merged to `main`), which already carries #153 (`93f1ac3`) and #154
(`98ff9a4`). Focus: what the six-reviewer round on the pre-rebase piece never
saw — the ten commits `2480536..aac415b` that landed after that round, and
how this piece's screen now composes with #153's machine-key-as-identity
change and #154's `getStoa`/blank-title work.

No findings. Every priority in the brief was checked against the merged tree,
against the full test suites, and — for the two claims most worth
distrusting on paper alone — against a hand-written mutation. Details below,
so an absence of boxes here is checkable rather than asserted.

## What was checked, and how

**1. `get_master_key` writes nothing and leaks no key material or
passphrase.**

- Read `dialectica/rust-lib/dialectica-core/src/wire.rs:1442-1476`
  (`get_master_key`, `master_key_from`) and
  `dialectica/rust-lib/dialectica-core/src/keystore.rs:398-431`
  (`open_from_env_with_protection`). The opener reads the file once, derives
  `encrypted` from the bytes' own protection tag *before* any passphrase
  lookup (`protection_of` then `unlock_for(encrypted)`), and no `unlock`
  parameter is threaded in from a caller's configuration — so the reported
  `encrypted` cannot be the caller's `DIALECTICA_PASSPHRASE` setting mistaken
  for the file's own state, which is the exact confusion the doc comment
  says it avoids.
- Confirmed the adapter (`dialectica/rust-lib/src/lib.rs:1004-1017`) passes no
  `unlock` into `get_master_key`'s opener — only `default_path_in(&dir)`.
- `KeystoreError`'s `Display` impl (`keystore.rs:591` onward) was read
  variant-by-variant: none carries ciphertext or passphrase bytes, matching
  the comment above the enum ("checked by a test").
- Ran the full Rust suite (see Gates below); it includes
  `keystore::tests::no_error_message_carries_key_material_or_a_passphrase`,
  `keystore::tests::the_environment_decides_the_unlock_only_after_the_file_has_spoken`,
  and `keystore::tests::the_file_holds_no_standalone_passphrase_verifier`, all
  passing.
- **Mutated** `master_key_from` to also treat
  `KeystoreError::PermissionsTooOpen` as `NotHeld` (the trap the function's own
  doc comment names: a present-but-unreadable key silently reported as no key,
  which would invite a doomed "create a key you already hold"). Caught:
  `wire::tests::a_key_whose_permissions_are_too_open_is_a_failure_not_an_absence`
  failed immediately, asserting the error shape it expected against the
  wrongly-substituted `{"hasMasterKey":false}`. Reverted; `git diff --stat
  0cbe1d4` is empty again.
- Confirmed `get_master_key` performs no write: the only handler paths are
  `Request::parse` (read) and the injected `open` closure (read); no `create`,
  `write_to`, or identity-record call is reachable from it.

**2. Attacker-influenced strings (`getStoa` title/description, core error
text) reaching this piece's screen unsanitised.**

- This piece's own screen, `DStoaListScreen.qml`, does not call `getStoa` at
  all — that call belongs to #154's `DJoinScreen.qml` (the join/preview
  screen), which had its own review round and is out of this piece's surface.
  Grepped `DStoaListScreen.qml` for `getStoa`/`get_stoa`: no hits.
- Everywhere this piece's screen renders a string that did not originate as a
  literal — `screen.failure` (a core error from `list_stoas`),
  `screen.createFailure` / `createFailureText` (a core error from
  `create_stoa`, including the blank-title refusal), `mintFailureText`
  (`screen.machineKey.refusal`, a core error from `create_identity`), and
  every `foundingTitle` rendered in the row `Repeater` — the `Text` element
  carries `textFormat: Text.PlainText`. Confirmed by reading every occurrence
  in `DStoaListScreen.qml` (34 instances, all `Text.PlainText`, none
  `RichText`/`StyledText`).
- `test_a_title_containing_markup_is_rendered_as_literal_characters` and
  `test_looked_up_text_is_not_interpreted_as_markup` in
  `tst_stoa_screens.qml` pass, pinning this for the title path.
- These are core-module-originated error strings (not peer-forged wire
  content) in every case on this screen — `list_stoas`, `create_stoa`,
  `create_identity`, `get_master_key` all answer the local module, not a
  remote peer — so there is no attacker-controlled string reaching this
  screen at all, in addition to the plain-text rendering being a second
  layer.

**3. A blank or lookalike title appearing as created.**

- `create()` (`DStoaListScreen.qml:337-387`) passes the typed title through
  unmodified to `Core.createStoa`, deferring entirely to the core
  (`create_stoa`, `wire.rs:2525-2583`), which refuses a blank title via
  `Genesis::address()` / `canonical_bytes()` (`stoa.rs`) before the
  membership store is even opened — `BlankTitle` is checked against the
  frozen 30-character `BLANK_CHARACTERS` list shared by the genesis codec,
  the metadata op codec, and the resolver.
- On `!reply.ok`, `screen.createState = "failed"` and `screen.created = null`
  — there is no code path that sets `created`/`"created"` without both
  `reply.ok` and a non-empty `reply.value.stoa` string. So a refused blank
  title cannot render as created; it renders `createFailureText` with the
  core's unreworded refusal instead.
- "Lookalike title" (same-title / homoglyph comparison against Stoas already
  held) is a `stoa-navigation-view` requirement that belongs to the
  join/preview flow (#154's `DJoinScreen.qml`), which compares a *previewed*
  reference against Stoas already held. This piece's create flow has no
  preview step and no same-title comparison logic at all — grepped
  `DStoaListScreen.qml` for `sameTitle`/`lookalike`: no hits. There is
  nothing on this screen a lookalike-title attack could act on; the concern
  does not apply to this piece's surface. (Not a defect — noting it as
  checked and out of scope, per the brief's instruction to say when
  something predates or belongs to another piece.)
- `dialectica-ui/tests/tst_stoa_screens.qml`'s
  `test_every_blank_title_reaches_the_core_rather_than_being_refused_here`,
  `test_the_placeholder_is_never_submitted_as_a_title`, and
  `test_a_creation_success_carrying_no_address_is_a_failure` all pass.

**4. Whether the unencrypted warning can be suppressed when the key really is
unencrypted.**

- `DStoaListScreen.qml:1182`:
  `visible: screen.machineKey.encrypted === false` — strict equality, and
  `heldKey()` (`DStoaListScreen.qml:158-164`) only ever sets `encrypted` to a
  real boolean or `null` (never a truthy-but-wrong value), so `=== false`
  cannot be defeated by type coercion. `encrypted` is threaded from
  `get_master_key`'s reply, which is itself derived from the file's own
  protection byte (point 1 above) rather than from configuration.
- **Mutated** the visibility condition from `=== false` to `=== null` (would
  suppress the warning even when the core answers `encrypted:false`). Caught:
  4 of 136 `tst_stoa_screens.qml` tests failed —
  `test_the_key_held_state_renders_its_copy_verbatim` ("missing verbatim:
  Stored unencrypted…") and
  `test_the_mint_reports_protection_from_its_own_reply` (Actual 0, Expected
  1) among them. Reverted; `git diff --stat 0cbe1d4` is empty again.
- No code path on this screen sets `encrypted` from anything other than the
  `get_master_key`/`create_identity` reply, so there is no separate
  client-side "suppress" flag to check for.

**5. A panic reachable from a malformed request to the new or merged handlers
in `wire.rs`.**

- `get_master_key`, `get_stoa` (#154's, now merged) and `create_stoa` are all
  wrapped in `guarded()` (`wire.rs:63-79`), which `catch_unwind`s the handler
  body and converts any panic — string or non-string payload — into
  `{"error":"panic in <method>: <detail>"}` rather than letting it unwind
  across the `extern "C"` dispatch boundary (which the module doc says
  aborts the process, PHASE0-FINDINGS §3).
- Read each handler body for `unwrap`/`expect`/indexing/arithmetic on
  request-derived data: `get_master_key` reaches only `Request::parse` (which
  returns `Result`) and the injected opener; `create_stoa` reads `title` via
  `parsed.get("title")` with an explicit three-way match (string / wrong-type
  / missing), and refuses an over-length or blank title through
  `genesis.address()`'s `Result` before any store write; `get_stoa` parses,
  verifies the genesis record against the address, and explicitly reports
  (not `unwrap`s) a decode failure from `Founding::of`, with a comment noting
  exactly why: "a panic here aborts the module process."
- **Mutated** `get_master_key` to add an unreachable-by-construction
  `parsed.get("nonexistent-field").unwrap()` after the existing parse, to
  confirm the guard actually intercepts a panic on this piece's specific
  handler rather than the claim being untested. Ran
  `a_peer_with_no_master_key_is_reported_as_holding_none_and_nothing_else`:
  the panic fired (visible in stdout,
  `Option::unwrap() on a None value`), the Rust test process did **not**
  abort, and the handler returned
  `{"error":"panic in get_master_key: called \`Option::unwrap()\` on a
  \`None\` value"}}` — the test then failed on the ordinary assertion
  mismatch, not a crash. This is the guard behaving as documented. Reverted;
  `git diff --stat 0cbe1d4` is empty again.
- Ran the full Rust suite, which includes
  `wire::tests::hostile_publish_input_is_never_a_panic` and
  `end_to_end::an_over_cap_genesis_title_is_refused_before_it_can_name_a_stoa`
  / `a_page_past_the_end_is_an_empty_page_rather_than_a_panic_or_a_wrapped_first_page`
  — all passing.

## The ten post-review commits (`2480536..aac415b`), read individually

- `4b29629` — adds a QML test for "pasting stays available when the key
  state could not be read," mutation-proven by the commit's own author
  (`visible: … !== "unreadable"` on `pasteSection`, reverted). No security
  concern: this is a UI-availability fix, not an auth/validation change. Read
  and agreed with the mutation evidence in the commit message.
- `9efae86` — archive/promote. No code change; mechanical move plus deletion
  of the pre-rebase `findings/` directory (correctly, since every box there
  was ticked).
- `59dbd0c`, `6aa76d7` — fix a navigation test's fixture
  (`test_a_missing_identity_offers_the_route_to_the_stoa_list`) that broke
  because #153 removed the per-Stoa key writer the original key-state design
  relied on, and record the reasoning in `design.md` Decision 8/15. Read the
  full diff: the fixture now states `get_master_key: {"hasMasterKey":false}`
  consistently with its own `who_am_i` answer. No security implication —
  this is a test correctly catching a state-machine composition the
  pre-rebase review could not have seen, since #153 merged mid-review.
- `fca9199`, `18ec847`, `4296bc5` — settle and then test a boundary dispute
  between `view-navigation` ("the route makes no call of its own") and
  `stoa-navigation-view` ("the list asks `get_master_key` on every showing").
  Read the amended requirement text and the new
  `test_following_the_route_makes_no_call_of_its_own`, which compares the
  full set of bridge calls after the identity route against a `closeFeed()`
  control (same `enterOnly("", null)` body) rather than counting three named
  methods — closing exactly the "an unnamed extra call passes silently" gap a
  named-method count would have left. This is a scoping/spec-consistency fix,
  not a security defect; the call in question (`get_master_key`) is
  read-only and its authorization is unchanged.
- `f9617cc`, `2602da0` — reconcile this piece's blank-title copy with #154's
  stricter blank-title rule (`join-preview-getstoa`, which forbids creating a
  Stoa with a blank title, superseding this piece's original "empty title
  MUST be accepted"). Read both diffs and the merge's conflict resolution in
  `stoa-navigation-view/spec.md` and `tst_stoa_screens.qml`. The resulting
  live spec and code (confirmed above, priority 3) correctly refuse a blank
  title through the core rather than the view, with no view-side special
  case that could diverge from `stoa-membership`'s enforcement. This is the
  interaction the brief calls out by name, and it holds.
- `aac415b` — ticks the archive/findings-gate rows on the (now-archived)
  `tasks.md`. No code change.

None of the ten commits touch key handling, passphrase handling, or
markup/text-format rendering, and none weakens any of the five priorities
above — each was read against the actual diff, not inferred from its message.

## Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: **1141 + 30 passed, 0 failed** (unit tests +
  `end_to_end.rs`), on the clean merged tree.
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`:
  **136 passed, 0 failed**.
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_navigation.qml`:
  **24 passed, 0 failed**.
- `sh dialectica-ui/tests/run-qml-tests.sh` (full suite, 25 spec files): **all
  passed, 0 failed** (three QWARN lines in `tst_vote_and_gate.qml` are the
  test's own deliberate malformed-input assertions, not failures).
- `lgs basecamp build`: succeeds, both `lgx` and `lgx-portable` artefacts
  produced for `dialectica` and `dialectica_ui`.
- `cargo mutants` was not run: the brief's shell-shape constraints (no pipe,
  no redirect) make its usual invocation awkward, and the two most
  security-relevant claims in this piece's diff were instead mutated by hand
  with `Edit` (both above, both caught and reverted) — a more targeted use of
  the same budget than an unscoped `--file wire.rs` run would have been
  against a file this large.

## What was not re-litigated

Findings belonging to #153 or #154 alone, with no interaction with this
piece, were not searched for beyond what surfaced while reading the merge
diff and the post-review commits above — both PRs had their own review
rounds, and the brief's focus is this piece's post-review delta and its
composition with them. Nothing found while reading either PR's diff here
looked like an unreviewed security defect; the one substantive interaction
(the blank-title reconciliation, `f9617cc`/`2602da0`) is covered under
priority 3 above rather than deferred.
