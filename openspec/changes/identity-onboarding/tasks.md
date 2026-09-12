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
- [x] 6.3 `openspec validate identity-onboarding --strict` — **passes**, after two
  defects the implementation pass found were fixed in the delta by `spec-writer`.
  Recorded rather than ticked silently, because the failures were real and the
  reason they are gone is a spec edit rather than a code one.

  1. `identity/spec.md`: the MODIFIED requirement had **renamed** the live spec's
     scenario "The same root and Stoa always yield the same identity" to "The
     same root, Stoa and path…" instead of keeping it alongside. A MODIFIED block
     replaces the whole requirement, so `openspec archive` would have **dropped**
     the two-input determinism scenario. Both scenarios are now present, and the
     requirement prose now states that the two-input derivation remains, so the
     retained scenario is in contract rather than orphaned. Confirmed by
     reverting the fix and watching `validate --strict` name the omission:
     *"MODIFIED … omits scenario(s) the current spec still has"*.
  2. `identity-onboarding/spec.md`: "A generated name and a mark are not settled
     by this capability" carried no `#### Scenario:` block. It now asserts the
     observable thing — that no reply of this capability carries a name or a mark
     field, while both still carry the public key and the address such values
     would be derived from.

     **Coverage of the new scenario is partial, and saying so is the point.**
     `the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
     asserts the whole serialised string, so a name field added to the whoami
     reply fails it. `the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
     checks each expected key is *present*, not that the key set is exactly
     those — so a `name` field added to a slate candidate would leave it green.
     The slate half of the scenario is therefore unpinned; a test asserting the
     candidate's key set exactly is `tester`'s to add.

  Both behaviours were already covered in code —
  `a_derived_stoa_key_is_deterministic` and `a_path_derived_key_is_deterministic`
  both exist and pass — so nothing was untested; what was at risk was the
  contract losing a requirement on archive.

  **A third thing blocks the archive, and it is not in this change.**
  `openspec archive identity-onboarding` aborts with *"Validation errors in
  rebuilt spec for identity … Spec must have a Purpose section"* and writes
  nothing — so this change cannot be archived at all until the live
  `openspec/specs/identity/spec.md` gains a `## Purpose`. That file has none, an
  artifact of the specs being hand-merged before the CLI was installed.

  **The fix already exists on the `docs/flow-tooling` branch**, which gives a
  Purpose to `identity`, `module-wire-contract` and `stoa-metadata` and corrects
  the flow README's claim that openspec is not installed. It is not on `main`
  yet. **`docs/flow-tooling` must land before this change is archived**; nothing
  is duplicated here, because two Purposes for one spec is two answers.

  With that Purpose temporarily in place, the archive applies cleanly and the
  rebuilt `identity` requirement carries **both** determinism scenarios, all
  three original scenarios and the three new ones — verified by running the real
  archive in a throwaway worktree, reading the merged file, then reverting
  everything the archive wrote.
- [x] 6.4 The `logos-rust-sdk-src` symlink is removed and absent from the commit
  — confirmed by `git status`.
