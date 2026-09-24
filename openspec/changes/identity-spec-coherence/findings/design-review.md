# Design review — identity-spec-coherence

No findings.

Checked against GitHub issue #157 (`gh issue view 157 --repo fryorcraken/dialectica`),
`design.md`, `proposal.md`, `tasks.md`, and `git diff origin/main...HEAD`.

What was verified, not just read:

- **Decision 4's two guards** (`mint_master_key`'s `keystore_path.exists()` branch
  in `wire.rs:1331`, and `Keystore::create`'s `AlreadyExists` refusal underneath
  `write_to` in `keystore.rs:940-955`) match the code exactly, including the
  layering claim that deleting guard 1 alone does not produce a build that
  replaces the key (guard 2 catches it). The existing regression test
  `a_mint_over_an_existing_keystore_replaces_nothing_and_reports_it_as_not_new`
  (`wire.rs:5415`) carries exactly the mutation evidence design.md cites. The new
  test `creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`
  is a genuinely different check (report + signature, not reply + file bytes),
  uses a file-reading opener rather than a fixture, and `tasks.md` §2.4 records
  the mutation that proved it red (rewiring the `exists()` branch to `generate` +
  `write_to`) with a plausible, specific failure description.
- **Decision 1's "only production caller" claim** — `git grep -n "\.create("` over
  `dialectica/rust-lib` shows production callers only at `wire.rs:982`
  (`keep_selection`), `wire.rs:1349` (`mint_master_key`), and
  `examples/seed_store.rs:390`; every other hit is inside `#[cfg(test)]` code.
  Matches the design's claim precisely.
- **Decision 2's rename** (REMOVED "Identity does not rotate, and this is a
  contract not an omission" / ADDED "Identity does not rotate: the key behind an
  identity is never replaced") — the delta's carried-over requirement prose and
  the untouched scenario "An identity is named by its key and by nothing beside
  it" match the live spec byte-for-byte apart from the one new paragraph the
  proposal describes. `git grep -F` for both old names across
  `openspec/specs dialectica dialectica-ui docs CLAUDE.md` returns only the live
  spec's own headings (the ones being removed), confirming the rename breaks no
  reference. The alternatives considered (keep the old name on new content;
  delete the scenario outright) are real alternatives with stated costs, not
  strawmen — "keep the old name" is rejected because the name itself states the
  false claim, which is a specific, checkable reason.
- **Decision 3's direct-edit/delta split for the two Purpose paragraphs** —
  `git show 43622b9 -- openspec/specs` touches only the lines above
  `## Requirements` in both files, exactly as `tasks.md` §1.3 claims. The
  reasoning (a delta's Purpose is ignored for an existing capability, so #149
  could not have fixed this) is checkable against the OpenSpec archive behaviour
  named in `docs/OPENSPEC-ARCHIVE.md`, and the choice to keep per-Stoa text
  rather than delete it is justified against #108 rather than asserted.

Against the issue: the issue's "What this needs" asks to rewrite the scenario
(with two suggested options), drop or justify the THEN clause, and bring both
Purpose paragraphs in line with 0.0.1 while keeping their per-Stoa content for
#108. All three are done, and Decision 1 explicitly evaluates the issue's own
two suggested rewrites before taking a third, narrower one — the departure from
the issue's literal wording is argued, not silent. The rename (REMOVED/ADDED
under a new name) is not something the issue asked for; design.md Decision 2
justifies it as forced by `openspec` 1.13.0's refusal of both simpler shapes,
which is a tooling constraint external to the issue's scope rather than an
unexplained expansion of it.

No code changes ship (test-only diff to `wire.rs`), matching the proposal's
explicit non-goal, and the "no rotation behaviour changes" claim is consistent
with the diff stat (`git diff origin/main...HEAD --stat`): only `wire.rs`
(tests), `openspec/specs/identity/spec.md`,
`openspec/specs/identity-onboarding/spec.md`, and the change's own
`openspec/changes/identity-spec-coherence/` files are touched.
