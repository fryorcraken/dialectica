## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

This change edits the contract's text. It changes no code (design.md Decision
4). The only thing left to build is the test the rewritten scenario needs, and
the tester's stage row owns it. Section 2 records what that test depends on, so
the tester does not have to work it out again.

## 1. Spec text (done by the spec-writer in `43622b9` and `c8d3105`; checked here)

- [x] 1.1 The delta `REMOVED`s *Identity does not rotate, and this is a contract
      not an omission* and `ADDED`s *Identity does not rotate: the key behind an
      identity is never replaced*, with the rewritten scenario (design.md
      Decisions 1 and 2). Verify:
      `openspec validate identity-spec-coherence --strict`.
- [x] 1.2 Neither removed name is cited anywhere, so the rename breaks no
      reference. Verify: `git grep -n -F "this is a contract not an omission"`
      and `git grep -n -F "pure function of its root"` over `openspec/specs`,
      `dialectica`, `dialectica-ui`, `docs` and `CLAUDE.md` return only the
      live spec's own headings, which archive removes.
- [x] 1.3 Both Purpose paragraphs are edited directly in `openspec/specs/`, and
      each keeps its per-Stoa text for #108 (design.md Decision 3). Verify:
      `git show 43622b9 -- openspec/specs` changes only the lines above
      `## Requirements` in each file.

## 2. The behaviour the scenario names already holds (design.md Decision 4)

- [x] 2.1 Confirm that calling the operation that creates a master key
      (`createIdentity`) while one is held replaces nothing. `mint_master_key` returns early on `keystore_path.exists()`, and
      `Keystore::create` refuses an existing file underneath it. Verify: by
      reading `wire.rs` `mint_master_key` and `keystore.rs` `Keystore::create`.
      `a_mint_over_an_existing_keystore_replaces_nothing_and_reports_it_as_not_new`
      pins guard 1.
- [x] 2.2 Confirm that no other production path can write over a held key.
      Verify: `git grep -n "write_to("` over `dialectica/rust-lib` finds no
      production caller except `Keystore::create`. `git grep -n "\.create("`
      finds only `keep_selection` and `mint_master_key` outside tests, plus the
      `seed_store` example.
- [x] 2.3 Confirm that the identity report and the signing key both read the
      keystore from disk on every call, not from the mint's reply. Verify: by
      reading `whoami_for` → `posting_identity` and the adapter's `publishing` →
      `publishing_key`.
- [x] 2.4 **Owned by the `tester` row, not this one.** A test cites *Creating a
      master key while one is held does not replace the identity in use*, and
      calls `createIdentity`, not the read-only `getMasterKey` (design.md
      Risks). It must be proved red against a build whose mint writes a fresh root over a
      held one. Deleting guard 1 alone is **not** that build: guard 2 turns the
      mint into the error shape and the identity stays unchanged (design.md
      Decision 4). The test's keystore opener must read the file, not return
      `a_master_key()`.

      Done:
      `dialectica-core/src/wire.rs`'s
      `wire::tests::creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`.
      Holds a machine key (`createIdentity`), reports the identity
      (`who_am_i`), calls `createIdentity` again, reports again, and publishes
      a post — asserting the second report names the first report's key and
      the post's author is that same key. The opener is `|| Keystore::open(&dir.keystore_path(), &Unlock::Unencrypted)`,
      which reads the file. Proved red by mutating `mint_master_key`'s
      `exists()` branch to `Keystore::generate()` + `write_to` (replacing the
      file, bypassing `Keystore::create`'s `AlreadyExists` guard) instead of
      reading the existing key back: the second `who_am_i` report assertion
      failed with the fixture's key on the left and the freshly-generated
      key on the right. Restored; `cargo test --manifest-path
      dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` is
      green (1142 + 30 passed) with the implementation unchanged from HEAD.
