# Correctness review — identity-spec-coherence

Dimension covered: **correctness** only (as instructed). No findings.

## What was checked

- Read `proposal.md`, `design.md`, `tasks.md`, the delta
  (`openspec/changes/identity-spec-coherence/specs/identity/spec.md`), and the
  direct Purpose edits to `openspec/specs/identity/spec.md` and
  `openspec/specs/identity-onboarding/spec.md`.
- Verified the `REMOVED` block's text is a byte-for-byte match of the live
  requirement still present in `openspec/specs/identity/spec.md` (lines
  422–458), which is what lets `openspec archive` find and delete it.
- Ran `openspec validate identity-spec-coherence --strict` — passes, as
  claimed in tasks.md 1.1.
- Re-ran the two `git grep -F` searches tasks.md 1.2 cites, scoped exactly as
  written (`openspec/specs dialectica dialectica-ui docs CLAUDE.md`): both the
  old requirement name and the old scenario name resolve only to the
  soon-to-be-removed heading/scenario in the live spec. No dangling reference.
- Verified design.md Decision 4's guard-layering claim by reading
  `mint_master_key` (`wire.rs`) and `Keystore::create`/`write_to`
  (`keystore.rs`): `mint_master_key`'s `exists()` branch reads the file back
  and writes nothing; `Keystore::create` refuses `AlreadyExists` underneath
  it; `write_to` unconditionally replaces the file. This matches the
  design.md text exactly.
- Verified tasks.md 2.2's `git grep` claims about `write_to(` and `.create(`
  call sites: outside tests and the `seed_store` example, the only production
  callers are `mint_master_key` (`wire.rs:1349`) and `keep_selection`
  (`wire.rs:982`).
- Built the whole workspace (`cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`) after
  recreating the gitignored `logos-rust-sdk-src` symlink: **1142 + 30 tests
  pass**, matching the count tasks.md 2.4 records, and the new test
  `wire::tests::creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`
  is present and green.
- **Reproduced the red proof myself**, independently of the tester's
  claim: edited `mint_master_key`'s `exists()` branch to generate a fresh
  keystore and call `write_to` (replacing the held key) instead of reading it
  back, exactly as design.md Decision 4 and tasks.md 2.4 describe. The new
  test failed as predicted:
  ```
  assertion `left == right` failed: the second report must name the public key the first report named
    left: "2ce77df6f78073c2d53692d369afea08e72789995d1f13cf3f72b61164785507"
   right: "315140f4719b7be4835474ef5acd7731d9daf63c872c09e03cc30fa38f034e04"
  ```
  Restored the file immediately afterward; `git diff --stat` on `wire.rs`
  shows no residual change.
- Checked the new scenario's WHEN/THEN/AND against the test body line by
  line: the ordering (hold a key → report → create again → report again →
  publish), the two observations (report and signed-post author), and the
  "names the same public key" / "carries that same public key" assertions
  all correspond.
- Checked the cross-reference in the new requirement's prose ("the operation
  `identity-onboarding` provides ... not the one that reports whether a
  master key is held, which writes nothing by its own requirement") against
  `identity-onboarding`'s own text: "the operation that creates a master
  key" (line 806) and the `getMasterKey`-equivalent requirement's own MUST
  NOT-write clause (line 791–792) both match as claimed.
- Checked the "Left alone" claim in proposal.md — the phrase "the key a
  request for this peer's master key reports" — occurs exactly three times
  in the untouched requirement *In this release one machine key is the
  identity in every Stoa*, and that requirement is outside this change's
  diff.

## Clean

Every checkable claim in proposal.md, design.md and tasks.md that a
correctness review can verify — the guard layering, the grep scopes, the
test's fidelity to the scenario text, the red/green proof, and
`openspec validate --strict` — held up under independent reproduction. No
correctness defect found in the spec prose, the delta, the direct Purpose
edits, or the new test.
