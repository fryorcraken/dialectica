# Architecture review — identity-spec-coherence

Scope: architecture only (structure, module/spec boundaries, "one function one
job", where complexity lives, comment justification). Correctness, security and
readability are covered by other reviewer instances.

No findings. The change is prose/spec structure plus one added test, and both
hold up structurally:

- **Delta shape (`REMOVED` + `ADDED` under a renamed heading, not `MODIFIED`).**
  Verified this is not an invented workaround: `openspec/changes/archive/
  2026-09-24-join-preview-getstoa/specs/stoa-membership/spec.md` uses the exact
  same shape (Reason + Migration prose, `REMOVED` block followed by an `ADDED`
  block under a new but overlapping name) for the same reason — a tool that
  refuses both `MODIFIED` dropping a scenario and `REMOVED`+`ADDED` under one
  name. This change follows an established precedent rather than introducing a
  new escape hatch.
- **Direct-edit vs. delta separation.** `proposal.md`'s table keeps the two
  Purpose paragraph edits (direct, in `openspec/specs/`) structurally disjoint
  from the requirement delta (`openspec/changes/.../specs/identity/spec.md`).
  Confirmed by reading both: the delta only ever touches requirement blocks
  (`### Requirement:` through its last scenario), and the Purpose paragraphs sit
  above `## Requirements`, which `REMOVED`/`ADDED` cannot reach. No overlap, so
  archive cannot double-apply or silently revert either edit.
- **Cross-capability reference stays inside the stated boundary.** The new
  paragraph in `identity`'s added requirement ("the operation that creates a
  master key ... The operation meant is the one `identity-onboarding` provides
  for a peer to obtain its master key") uses the exact phrase
  `identity-onboarding`'s own requirement uses for that operation (see
  `openspec/specs/identity-onboarding/spec.md:806`, "the operation that creates
  a master key"). It names the operation without restating its mechanics, which
  is what `identity-onboarding`'s own Boundary paragraph ("this capability owns
  only the act of *acquiring* an identity, and deliberately restates none of the
  three") requires of a caller from the other side. Not a new coupling pattern —
  the pre-existing requirement *In this release one machine key is the identity
  in every Stoa* already reaches across the same boundary the same way.
- **The new test's shape matches the scenario's own compound WHEN/THEN/AND**,
  not a function doing two jobs. `wire.rs`'s
  `creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`
  chains mint → report → mint again → report → publish in one function, which
  reads like "one function, two jobs" out of context — but design.md's Decision
  1 gives the reason: the scenario checks two independent observations (the
  identity report and a signed op) precisely because a build could diverge on
  one without the other, and splitting them into two tests would not let a
  single assertion compare "before" against "after" across both. The sibling
  test five lines above it (`a_mint_over_an_existing_keystore_replaces_...`)
  checks a different, narrower claim (the mint's own reply and the file's raw
  bytes) and is deliberately kept separate rather than merged in — two jobs,
  correctly left as two functions.
- **Idiom consistency.** The new test's use of `slate_request()`,
  `MemoryOpLog::new()`, `crate::op::OpId::from_hex(...).unwrap()`, and the
  `who_am_i`/`create_identity`/`publish_post` helpers all match the file's
  existing conventions (checked against dozens of other call sites in the same
  file). It is not introducing a new pattern or a parallel helper that
  duplicates existing ones.
- Confirmed the suite is green as tasks.md 2.4 claims: `cargo test
  --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core` passed 1142 + 30, matching the recorded count.

No dependency changes (`Cargo.toml` untouched), no code changes outside the one
test (confirmed by `git diff origin/main...HEAD --stat`), so there is nothing
else in this change's architecture surface to review.
