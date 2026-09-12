Every box below is checked because the work was done AND the verification named
in it was run. Where reality differed from the plan, the task carries a note
rather than a tick alone — a checklist that hides a correction is worse than one
that never made the claim.

## 1. Derivation takes a path

- [x] 1.1 Add `derive_stoa_key_at_path(root, stoa, path)` to `identity.rs` under a
  new salt constant `/dialectica/2/Identity/Stoa`, leaving `derive_stoa_key`
  byte-identical — verified: `the_wire_constants_are_pinned_to_known_answers`
  passes with its original four assertions untouched.
- [x] 1.2 Extend that pinned test with a hardcoded expectation for the new
  derivation at a fixed root, Stoa and path — verified by watching it FAIL twice:
  once with the salt reverted to version 1, once with the path encoded
  little-endian. Both expected values were computed with `openssl kdf`, and that
  invocation was first validated by reproducing the existing version-1 pinned
  value exactly — which is what makes them independent rather than merely
  plausible.
- [x] 1.3 Add tests for determinism, different paths giving different keys, and
  the two schemes not colliding at path 0.

  **Correction worth recording:** the non-collision test does NOT fail when the
  salt is reverted, and the code comment now says so. Reverting the salt still
  leaves the two schemes distinct, because the path-taking one appends four bytes
  to the HKDF info. That is precisely why the salt bump is argued in `design.md`
  as a *decision* rather than relied on as a mechanism — and why 1.2's pinned
  assertion, not this test, is what guards it.
- [x] 1.4 Give `Keystore` a `stoa_key_at_path` / `stoa_public_key_at_path` /
  `stoa_address_at_path` trio — verified by
  `a_path_taking_keystore_identity_is_the_one_that_signs`, which signs and checks
  `verify_authored_op` against the reported address, with a negative case so the
  assertion is not vacuous.

## 2. The path record on disk

- [x] 2.1 Add `identity_store.rs` with a two-column SQLite table and its own
  `PRAGMA user_version` — verified: a fresh file creates, version 9999 is refused
  as `UnknownLayoutVersion` naming both numbers, a version stamped over a missing
  table is refused as `LayoutDoesNotMatchItsVersion`, and a table missing the
  `path` column is refused too. No `unwrap`/`expect` outside `#[cfg(test)]`.
- [x] 2.2 Implement `record_path`, `path_for(stoa)` and `all_paths()` — verified
  across a real file reopened twice, two Stoas recorded separately, and every
  pairing read back against a hardcoded expected set.

  Also verified by mutation: replacing `INSERT` with `INSERT OR REPLACE` makes
  `a_second_choice_for_one_stoa_is_refused_and_changes_nothing` fail, and making
  `path_from_row` clamp with `as u32` makes
  `a_stored_path_outside_u32_is_refused_rather_than_clamped` fail.
- [x] 2.3 Verify nothing machine-local is stored — verified by
  `reading_the_record_back_needs_nothing_beyond_the_record`, which reads the file
  with a SECOND connection sharing nothing with the writer and asserts the table
  holds exactly the two named columns.
- [x] 2.4 Feed the store arbitrary and truncated file content — verified. Note
  that SQLite treats a zero-length file as a fresh database, so the empty case
  *opens*; the test therefore also exercises every read on an accepted file,
  which is what makes the assertion about not panicking rather than about not
  opening.

## 3. The slate

- [x] 3.1 Add `onboarding.rs` with `SLATE_SIZE = 5`, a `SlateNonce`, and
  `derive_path` masking the top bit — verified by watching both the pinned
  constants test and `every_derived_path_is_below_two_to_the_thirty_one` FAIL
  with the mask removed. The two pinned values were computed with `openssl dgst`
  and the mask applied by hand; both digests have the top bit set, so a mask that
  was accidentally a no-op cannot hide.
- [x] 3.2 Implement `Slate::generate` walking the index forward until five
  distinct paths are held — verified: no two candidates share a path, a public
  key or an address, and a nonce reproduces an identical slate.
- [x] 3.3 Verify two generated slates differ — verified over eight regenerations
  rather than one pair, since a single pair differing could be luck.
- [x] 3.4 Verify a slate's values carry no secret — verified twice, by a byte
  search and by taking every exposed value as key material and checking none
  signs as any candidate. Both tests carry a positive control, so a broken search
  fails rather than passing silently.

  **This task found a real fixture defect.** The byte search failed on its first
  run because the master key and the nonce were both `[7u8; 32]`, so the search
  found the nonce and reported it as the master key — two explanations, one
  answer. The fixture now uses a distinct master key and asserts the two differ.
- [x] 3.5 The secret material held while deriving a slate is in a `Zeroizing`
  buffer.

  **No test asserts this, deliberately, and that is worth stating rather than
  ticking past.** `keystore.rs`'s own `generate` records that review found its
  explicit `bytes.zeroize()` deletable with the whole suite green, because a
  stack local after its function returns is not observable from a test. The same
  applies here. What was done instead is the shape that cannot be forgotten: the
  derived key is moved straight into the `Zeroizing` with no plain binding for
  anyone to omit a wipe on. A test claiming to cover this would be the coverage
  claim the keystore change was criticised for.

## 4. The wire methods

- [x] 4.1 Add `generate_identity_slate` — verified: the count is reported as a
  hardcoded 5 (not read back from the array), offering a `count` field changes
  nothing, malformed input is the error shape with no result beside it, and the
  reply's JSON key names are pinned.

  Additionally verified that a refused request does NOT call `remember`, so a
  malformed call cannot supersede a slate the user is still looking at.
- [x] 4.2 Add `keep_identity` — verified: an out-of-range selection is refused
  and writes neither store, a superseded nonce is refused, and no live slate at
  all is refused. Verified by mutation: removing the nonce comparison makes
  `a_selection_against_a_superseded_slate_is_refused_rather_than_satisfied` fail.
- [x] 4.3 Verify the write order and the second-keep refusal.

  **The write-order half was a gap this task found.** Reversing the two writes
  left the entire suite green — measured, not assumed — so
  `a_keep_whose_keystore_write_fails_records_no_path` was written for it, and
  watched to fail with the order reversed (`left: Some(11140858), right: None`).
  Without it, `design.md`'s central claim about atomicity would have been
  unpinned.
- [x] 4.4 Verify the encryption report — verified in both directions, and against
  the FILE via `Keystore::is_encrypted` rather than only against the argument, so
  a reply whose boolean had drifted from what was written would fail.
- [x] 4.5 Add `who_am_i` — verified: all four states of `design.md`'s table
  produce pairwise-distinct reasons, identity and reason are never both present
  or both absent, the reported public key derives the reported address, and
  `recoveryNeedsTheRecord` is carried.
- [x] 4.6 Verify no new method aborts — verified over eleven arbitrary inputs
  against all three methods, plus a panicking keystore dependency and a panicking
  record dependency.

## 5. The adapter and the module contract

- [x] 5.1 Add the three methods to the `DialecticaModule` trait and forward each
  into `core`, holding the live slate nonce as a second field — verified: the
  field is an `Option`, so `Default` still supplies the one genuinely
  parameterless constructor `interface: "universal"` requires, and no
  constructor parameter was added.
- [x] 5.2 Verify each adapter body is a forward plus the lookups.

  **Two helpers were extracted rather than repeating four lines a fifth time**,
  which is CLAUDE.md's "fourth slightly-different guard" signal: `storage_dir`
  (the persistence-path lookup, previously duplicated in two handlers and heading
  for five) and `master_key` (open-or-mint). `protection_from_env` went into
  `keystore.rs` rather than the adapter, so the passphrase byte handling is
  decided once — a second copy would eventually map two different passphrases
  onto one key.

  The adapter is not reachable by `cargo test` at all (PLAN.md §2.3), so these
  are verified by reading rather than by a test, which is the reason the bodies
  are kept this thin.

## 6. Gates

- [x] 6.1 Full suite: **546 passed, 0 failed** (475 before this change), with no
  warnings from either of this repo's crates. The warnings in the output are the
  SDK's own and predate this change.
- [x] 6.2 `cargo fmt --check` exits 0 and `cargo clippy -p dialectica-core
  --all-targets -- -D warnings` exits 0. Clippy caught three lints in the new
  tests (`bool_assert_comparison` twice, `unnecessary_to_owned` once), now fixed.
- [ ] 6.3 `openspec validate identity-onboarding --strict` — **DOES NOT PASS, and
  both failures are in the spec rather than in this change.** Left unchecked
  rather than ticked, because ticking it would be a false claim.

  1. `identity/spec.md`: the MODIFIED requirement renames the live spec's
     scenario "The same root and Stoa always yield the same identity" to "The
     same root, Stoa and path…" instead of keeping it alongside. A MODIFIED block
     replaces the whole requirement, so `openspec archive` would **drop** the
     two-input determinism scenario — which is exactly the silent-loss failure
     the agents README documents.
  2. `identity-onboarding/spec.md`: "A generated name and a mark are not settled
     by this capability" carries no `#### Scenario:` block.

  Neither is this change's to fix — a delta is `spec-writer`'s artifact. Both
  behaviours ARE covered in code: `a_derived_stoa_key_is_deterministic` and
  `a_path_derived_key_is_deterministic` both exist and pass, so nothing is
  untested; what is at risk is the contract losing a requirement on archive.
- [x] 6.4 The `logos-rust-sdk-src` symlink is removed and absent from the commit
  — confirmed by `git status`.
