# Readability review — identity-spec-coherence (#157 / PR #160)

Dimension covered: **readability only** (correctness, security and
architecture are other reviewers' rows).

## No findings

Reviewed everything on this HEAD not on `origin/main`
(`git diff origin/main...HEAD`, three dots): `proposal.md`, `design.md`,
`tasks.md`, the `identity` spec delta, the two direct Purpose edits in
`openspec/specs/identity/spec.md` and `openspec/specs/identity-onboarding/spec.md`,
and the new test in `dialectica/rust-lib/dialectica-core/src/wire.rs`.

Checks performed, all clean:

- Markdown structure: bold (`**`) and backtick (`` ` ``) delimiter counts are
  even in every changed file (no unclosed emphasis or unclosed inline code
  spans); the "Which edits are direct, and which are delta" table in
  `proposal.md` has a consistent column count across all rows; nested bullet
  lists use consistent 2-space indentation matching the rest of the file.
- No trailing whitespace or stray tabs in the diff
  (`git diff origin/main...HEAD | grep -n ' $'` /
  `grep -nP '\t'`, both clean once discounting an empty context line).
- `cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core -- --check`
  reports four pre-existing unformatted spots (`identity.rs:1007`,
  `identity.rs:1015`, `wire.rs:6127`, `wire.rs:16701`) — none inside this
  change's diff (the new test is at `wire.rs:5463-5533`). Consistent with the
  known repo-wide gap that `cargo fmt` doesn't follow the `dialectica-core`
  path dependency (see memory `dialectica-ci-fmt-gap`); not something this
  piece introduced or is responsible for fixing.
- The new test,
  `wire::tests::creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`,
  follows the file's established test conventions: a `//`-comment block
  mirroring the scenario's WHEN/.../THEN/AND structure with `// ...` connector
  comments, a comparison-to-sibling-test explanation up front (distinguishing
  it from `a_mint_over_an_existing_keystore_replaces_nothing_and_reports_it_as_not_new`
  immediately above), and the `open` closure reuse pattern (non-capturing
  `Fn` closure passed to `who_am_i` twice) matches existing usage at
  `wire.rs:6912`. Referenced identifiers (`create_identity`, `who_am_i`,
  `publishing_key`, `publish_post`, `OnboardingDir`, `slate_request`, `by`,
  `a_stoa`, `ignored_delivery`) all resolve to real definitions with matching
  signatures.
- Terminology ("the operation that creates a master key" vs. "machine key" vs.
  "master key") is used consistently across `proposal.md`, `design.md`, the
  spec delta, and the direct Purpose edits, and matches the phrase
  `identity-onboarding` already used before this change — the design doc's own
  "why this operation" reasoning explicitly ties the naming to that existing
  phrase.
- The dense, self-referential prose style (long single-line Purpose
  paragraphs, heavy cross-citation of decisions by number) matches this
  repo's existing convention throughout `openspec/specs/` and `CLAUDE.md`
  itself; judged against that house style rather than generic prose-brevity
  taste, nothing reads as an accidental readability regression.

No boxes to check — nothing here needs a fix.
