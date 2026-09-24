# Readability review — `home-screen-key-states`

Dimension covered: **readability only** (naming, functions with two jobs,
comments arguing from disproven premises, unmeasured numbers in comments, dead
code). Correctness, security and architecture are separate reviewers' lanes
and are not covered here.

## What I checked

- Full piece diff: `git diff origin/main...HEAD` (three dots) across all 13
  changed files.
- Ran `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
  -p dialectica-core`: 1099 + 30 tests, all pass.
- Ran `sh dialectica-ui/tests/run-qml-tests.sh
  dialectica-ui/tests/tst_stoa_screens.qml`: 116 tests, all pass.
- Ran `lgs basecamp build` from the tree root: builds clean, adapter compiles.
- Attempted `cargo mutants` scoped to `wire.rs` and `keystore.rs` with
  `--file` under a few path spellings (workspace-relative,
  crate-relative, and via each crate's own `--manifest-path`); every
  invocation returned "Found 0 mutants to test" with no error, which reads as
  a path-resolution quirk of the tool in this tree rather than a real "no
  mutable code" result. I did not chase it further — the effort ceiling for
  a quick per-file scan was already spent, and mutation coverage is more
  naturally the correctness reviewer's instrument. Not a readability finding.
- Searched (`git grep`) for dead references to every renamed QML identifier
  from the old identity-mint flow (`keyState`, `keyFailure`, `keyHeld`,
  `createIdentityButton`, `identityKeyLabel`, `identityUnencryptedWarning`,
  `identityFailureText`, `screen.createIdentity()`): none found. The rename
  from the old `keyState`/`keyHeld`/`createIdentity()` shape to the new
  `machineKey`/`askKeyState()`/`createMachineKey()` shape left nothing
  dangling.
- Checked the new Rust doc-comment intralinks (`[who_am_i]`,
  `[get_capabilities]`, `[mint_master_key]`) resolve to real functions: they
  do.
- Spot-checked whether the one naming defect below is a pattern or an
  isolated slip, by sampling Rust test names in the same commit
  (`a_key_whose_permissions_are_too_open_is_a_failure_not_an_absence`,
  `every_refusal_but_not_found_is_the_error_shape_in_the_keystores_words`) and
  the sibling QML test
  (`test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity`).
  All of those read correctly as what they assert. The defect below is
  isolated to one function.

## Findings

- [ ] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:3511` —
      `test_no_key_state_raises_identity` is named backwards from what it
      asserts and undersells its own scope
      **Scenario:** the function name has no negation in it, so it reads as
      an affirmative claim — "the no-key state raises identity" — exactly the
      opposite of what the body checks
      (`verify(!/\bidentity\b/i.test(shown), "no key state may raise
      identity: " + shown)`, i.e. it must **not** raise identity). A reader
      scanning a CI failure list, a `qmltestrunner` summary, or this file's
      table of contents sees "raises identity" and reasonably reads it as
      "this state is known to say 'identity', and this test pins that" — the
      opposite of the actual guarantee. The name is also narrower than the
      body: the loop asserts the "no identity" property over all **three**
      key states (`noKeyReply()`, `heldKeyReply(...)`, and the
      could-not-be-read error shape), not only the no-key one the name
      implies.
      The piece's own convention for this exact shape of test does it right
      two ways: the sibling
      `test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity`
      (same file, line 2604, pre-existing) uses "neither…claims", a
      grammatically negative construction; and the new Rust tests in
      `wire.rs` (e.g. `a_key_whose_permissions_are_too_open_is_a_failure_not_an_absence`)
      spell the "not X" out explicitly. This one function breaks with both.
      A plausible rename, following the file's own idiom: something like
      `test_no_key_state_claims_identity` won't fix the inversion (still
      reads affirmative) — it needs an explicit negative, e.g.
      `test_none_of_the_key_states_claim_identity`.
      **Severity:** low — the test itself is correct and currently passes
      (verified: `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_stoa_screens.qml` → `PASS :
      qmltestrunner::StoaScreens::test_no_key_state_raises_identity()`,
      116/116 passed). This is purely a scan-and-trust-the-name hazard, not a
      functional defect.

## Areas checked and clean

- **Naming.** `machineKey`/`noKey()`/`heldKey()`/`unreadableKey()`/
  `keyFromQuery()`/`askKeyState()`/`createMachineKey()` on the QML side, and
  `MasterKey`/`master_key_from`/`get_master_key`/
  `open_from_env_with_protection` on the Rust side, are all named for what
  they do and match the established sibling naming (`mint_master_key` /
  `Minted` is the direct precedent for `master_key_from` / `MasterKey`, and
  the doc comment says so explicitly). No misleading names found other than
  the one test above.
- **Functions with two jobs.** `keyFromQuery` has three return arms but they
  are one job (classify one reply into one of three states); the QML
  `Component.onCompleted` / `onVisibleChanged` pair looks at first glance like
  duplicated logic but is the minimal correct pattern for QML's
  create-once/show-many lifecycle, and the comment above it says why both
  hooks are needed rather than one. `get_master_key` in `wire.rs` is a
  thin guard-and-dispatch wrapper around `master_key_from`, matching the
  `mint_master_key`/`create_identity` split already in the file.
- **Comments arguing from disproven premises.** None found. Every comment I
  checked against the code it describes matched — including the `Core.qml`
  comment on `getMasterKey`'s "an unreadable keystore is `ok: false` here,
  never `hasMasterKey:false`" claim, verified against `wire.rs`'s
  `master_key_from` (only `KeystoreError::NotFound` maps to `NotHeld`; every
  other error is propagated as a failure), and the `hasMachineKey`-in-the-
  comment vs. `machineKey`-as-property-name apparent mismatch, which checks
  out: the comment is explicitly describing the issue's original flag name
  before it says the implementation widened it to an object (confirmed
  against `proposal.md:31` and `design.md:180`, which use the same "issue
  asks for `hasMachineKey`" framing).
- **Unmeasured numbers in comments.** No new magic numbers with unsupported
  claims. Layout constants (`+ 2 * 16`, margins) are plain layout arithmetic,
  not measurement claims.
- **Dead code.** None found. The old identity-mint UI block, its properties
  (`keyState`, `keyFailure`, `keyHeld`) and its tests were fully replaced, not
  left alongside the new code; grepped for every old identifier with none
  surviving.

## Hand-off

- Branch: `worktree-wf_42717d0c-339-3` (confirmed via `git rev-parse
  --abbrev-ref HEAD`; this is the harness-assigned worktree branch, not
  `piece/home-screen-key-states`).
- Mutations left in the tree: none. I made no source edits — only ran the
  test/build commands above and wrote this findings file. `git diff --stat
  ad9d867` after committing this file shows only the findings file added.
- The tree is ready to prune once this commit is cherry-picked.
