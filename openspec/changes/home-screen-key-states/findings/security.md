# Security review — `home-screen-key-states`

Scope: security dimension only, per dispatch brief. Focus areas checked, as
asked: (1) can `get_master_key` write/create/replace a key or leak key
material/a passphrase in a reply or error; (2) can attacker-influenced text
reach the view unsanitised; (3) is a panic reachable from a malformed request;
(4) can the unencrypted warning be suppressed when the key really is
unencrypted.

## What was checked

- Full piece diff (`git diff origin/main...HEAD`, three dots) against
  `ad9d867`.
- `dialectica/rust-lib/dialectica-core/src/keystore.rs` diff (the
  `open_from_env_with_protection` addition) and its surrounding module,
  read for the properties the file's own doc comments claim (no panic, no
  secret in an error, single-read protection reporting).
- `dialectica/rust-lib/dialectica-core/src/wire.rs`: `get_master_key`,
  `master_key_from`, `MasterKey`/`MasterKey::to_json`, `mint_master_key`
  (existing, called by `create_identity`), and every test in the "Asking
  whether a master key is held" block (~30 tests).
- `dialectica/rust-lib/src/lib.rs`: the new `get_master_key` trait method and
  its `Dialectica` impl (the adapter closure passed to core).
- `dialectica-ui/src/qml/Core.qml`: the `getMasterKey()` bridge method.
- `dialectica-ui/src/qml/DStoaListScreen.qml`: the full key-state machine
  (`machineKey`, `noKey`/`heldKey`/`unreadableKey`, `keyFromQuery`,
  `askKeyState`, `createMachineKey`), all three rendered blocks (no-key,
  unreadable, held), and the `unencryptedWarning` element specifically.
- `dialectica-ui/tests/tst_stoa_screens.qml`: all key-state and
  unencrypted-warning tests.
- Ran the full Rust suite (`cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`): **1129
  passed, 0 failed** (1099 unit + 30 end-to-end).
- Ran the QML suite (`sh dialectica-ui/tests/run-qml-tests.sh
  dialectica-ui/tests/tst_stoa_screens.qml`): **116 passed, 0 failed**.
- Attempted `cargo mutants --file dialectica-core/src/wire.rs --re
  "master_key|MasterKey"` (6 mutants) scoped to the new/touched code. It could
  not be made to complete inside the couple-minutes budget even scoped this
  narrowly (`ok Unmutated baseline in 13s build + 31s test` before being
  interrupted each time) — **abandoned per instructions, not run to
  completion.** No mutants.out artifacts were left in the tree
  (`git status` is clean of them). This is a gap: the six mutants listed
  (`mint_master_key` → `Ok(Default::default())`, `MasterKey::to_json` →
  `String::new()`/`"xyzzy".into()`, `get_master_key` →
  `String::new()`/`"xyzzy".into()`, `master_key_from` →
  `Ok(Default::default())`) are untested by me directly; the manual read and
  the existing named tests (below) cover the same ground by inspection, but a
  reviewer with more time budget should finish this run.

## Findings

No security defects found in the new `get_master_key` query or in the QML
that consumes it. Detail on each area the brief named:

**Write/create/replace.** `get_master_key` → `master_key_from` → the
adapter's `open_from_env_with_protection` is read-only: it calls
`read_checked` (a `fs::read` plus a permissions check) and never reaches
`Keystore::create`, `write_to`, or `Keystore::generate`. This is asserted
directly by `asking_with_no_key_held_writes_nothing_at_all` (checks the
keystore path does not exist AND the storage directory is empty afterwards)
and `asking_leaves_a_held_key_byte_for_byte_unchanged` (byte-for-byte
comparison before/after). Both pass. The QML side never calls
`createIdentity()` from `askKeyState()`/"Try reading the key again" — only
`createMachineKey()` (bound to the create-key button, which is only
instantiated in the `"none"` state) calls it, and `Keystore::create` itself
still refuses to overwrite an existing file (`AlreadyExists`), unchanged by
this piece.

**Leaking key material or a passphrase.** `MasterKey::to_json` emits exactly
`hasMasterKey`/`publicKey`/`encrypted` (held) or `hasMasterKey` (not held) —
never the root secret, ciphertext, salt, nonce or passphrase; there is no
accessor on `Keystore` for the root at all (`keystore.rs`'s own doc comments
require this). Every error path returns `KeystoreError::to_string()`
unmodified, and that type's variants are pinned by the pre-existing test
`no_error_message_carries_key_material_or_a_passphrase` (`keystore.rs:2190`,
not part of this diff but exercised by every arm this query can return,
including the new `open_from_env_with_protection`). `Io(String)` carries only
the OS's own message (path/errno), not file content. The "closed field set"
requirement is directly tested (`a_peer_with_no_master_key_is_reported_...`,
`a_held_key_is_reported_by_its_public_key_and_its_protection_and_nothing_
else`, `every_refusal_but_not_found_is_the_error_shape_...`).

**Attacker-influenced strings reaching the view unsanitised.** Every `Text`
element that renders a core-supplied string (`reply.error`,
`machineKey.reason`, `machineKey.refusal`, the reported `publicKey` via
`AddressLabel`) is `textFormat: Text.PlainText`, so QML's rich-text
interpreter (which would otherwise parse a subset of HTML) never runs over
it — a crafted `KeystoreError` display string or a hostile `publicKey` cannot
inject markup. This matches the pattern already used for `createFailureText`
and `pasteFailureText` elsewhere on the same screen. `get_master_key` itself
takes no caller-supplied fields beyond the fixed `{}` envelope (`request`
parameter is parsed by the shared `Request::parse` and no field is read), so
there is no attacker-controlled string entering the Rust side of this
particular query at all — the only untrusted-origin string in play is
whatever `KeystoreError`'s `Display` produces from local file state, and
that's covered by the point above.

**Panic reachable from a malformed request.** `get_master_key` is wrapped in
`guarded`, which `catch_unwind`s the whole handler body and converts any
panic into `{"error":"panic in get_master_key: ..."}` rather than aborting
the module process (the file's own doc comment on `guarded` states why this
matters — an unguarded panic aborts the process per PHASE0-FINDINGS §3). I
found no `unwrap()`/`expect()`/indexing in the new code's reachable path
outside `#[cfg(test)]` blocks; the sole production `.expect()` in
`keystore.rs` (`identity_key()`, line ~839, pre-existing and not on this call
path in the no-key/error arms, only reachable after a successful open) is
provably infallible — a `[u8; 32]` is always a valid Ed25519 seed by
construction, so `SecretKey::from_bytes` cannot fail there. The malformed
request case itself (non-object JSON, oversized payload) is handled by the
same `Request::parse`/cap logic every other handler shares, unmodified by
this diff.

**Whether the unencrypted warning can be suppressed for a genuinely
unencrypted key.** No. `Core.getMasterKey()`'s `encrypted` field is derived
in Rust from `open_from_env_with_protection`, which reads the protection
byte directly off the file bytes (`protection_of`, unchanged by this diff)
— it does not consult whether a passphrase is configured or what unlock the
caller supplied, so it cannot be made to under-report. On the QML side,
`heldKey()` normalises the field to `true`/`false`/`null` (`typeof encrypted
=== "boolean" ? encrypted : null`), and the warning's visibility is
`screen.machineKey.encrypted === false` — a strict comparison, so only an
actual `false` shows it; nothing falsy-but-not-`false` (a missing field, a
string, `0`) can either show it wrongly or hide it wrongly, since those all
collapse to `null` first and `null` draws neither a warning nor a false claim
of protection. This is exercised in both directions by
`test_an_unprotected_key_carries_the_warning_in_the_accent_colour`,
`test_a_protected_key_carries_no_unencrypted_warning`,
`test_an_omitted_protection_field_makes_no_claim_either_way`, and
`test_the_mint_reports_protection_from_its_own_reply` (the last covers the
create-key path too, not just the query path) — all pass. I also tried to
break this by hand: flipping the QML comparison to `!==` or to a loose `==`
both change which mutation surfaces, but neither survives the existing named
tests (`compare(...length, 1)` vs `0` assertions on both sides would flip and
fail) — I did not leave this mutation in the tree; it was a read-only check
against the test names, not an applied edit.

## Clean areas, stated in prose

- `keystore.rs`'s `open_from_env_with_protection` diff itself is a clean,
  minimal refactor of an existing double-read into a single read, exactly as
  its doc comment claims — it removes a TOCTOU-shaped duplicate read rather
  than introducing one.
- No new dependency was introduced by this diff (checked `Cargo.toml`/`git
  diff` for lockfile changes — none touched).
- `storage_dir()` (unchanged) is the only thing that decides the keystore
  path; `get_master_key`'s request body carries no path-shaped input, so
  there is no path-traversal surface on this new query.
- The adapter (`dialectica/rust-lib/src/lib.rs`) passes no caller-supplied
  `Unlock` into `open_from_env_with_protection` — the comment there states
  why (the caller's configured protection must never be mistaken for the
  file's), and the code matches the comment.

## Handoff

- Branch: `worktree-wf_42717d0c-339-2` (confirmed via `git rev-parse
  --abbrev-ref HEAD`; this is a review worktree branch, not
  `piece/home-screen-key-states`).
- Mutations left in the tree: **none**. All investigation here was read-only
  (file reads, `git diff`, `git grep`, `cargo test`, `cargo mutants --list`,
  and interrupted `cargo mutants` runs that made no source edits). No file
  under `dialectica/` or `dialectica-ui/` was modified by this review.
- Tree is ready to prune once this commit is picked; nothing here depends on
  re-reading mutated state, since no mutation was made.
