# Architecture review — #168 sdk-staging

Dimension covered: **architecture and readability** only (per dispatch). No
findings from the correctness, security or spec-test dimensions are claimed
here; those are separate reviewer passes.

Read issue #168 with `gh issue view 168 --json body,comments` before starting,
including the owner's 2026-09-25 comment, which corrects the body on two
points (the #91 runs did use the pinned SDK; `--inputs-from` likely needs no
`flake.nix` change) — both corrections are reflected in `proposal.md` and
`design.md` as written, verified by reading both files in full.

## What I checked

- Read `proposal.md`, `design.md`, `tasks.md`, `.openspec.yaml` in full.
- Read the full diff `git diff 8368b2f...HEAD` for `README.md`, `.gitignore`,
  `CLAUDE.md`, and the touched step + surrounding comment block in `ci.yml`.
- Staged the SDK myself with the exact README command
  (`nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`)
  from this tree's root — resolved to
  `/nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src`, matching
  design.md's D1 table.
- Ran `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
  as one plain command: **1180 passed** (dialectica-core lib), **30 passed**
  (end_to_end), 0 failed — matches tasks.md task 2.1's claimed numbers exactly.
- Ran `nix build ./dialectica#lgx` (not `.#lgx`) as one plain command: exited
  clean, `git status` stayed "nothing to commit, working tree clean"
  afterward (the SDK symlink and `result` are both gitignored, confirming
  `.gitignore` covers the build output this step produces).
- Checked CI's `install-nix-action` comment rewrite (D5) against
  `jq "[.nodes[] | .locked.type] | unique" dialectica/flake.lock`-style
  reasoning: `dialectica/flake.lock` only has `github`/`git`/root node types,
  no `path` input, so the re-grounded reason the comment now gives is
  accurate.
- Checked that `.#lgx` was fixed only in `CLAUDE.md` (as scoped) and left
  alone at its one other occurrence, `.github/workflows/ci.yml:1761` (inside
  an unrelated comment about `#app`/`--variant`), matching proposal.md's
  explicit out-of-scope note.
- Checked for duplication: the staging command appears byte-for-byte in
  exactly two places (`README.md`, `ci.yml`), and `.gitignore`/`CLAUDE.md`
  point at the README rather than repeating it — matches D4's stated design
  and the repo's own "two copies drift" principle.
- Considered whether design.md's non-goal (no drift gate between README and
  CI's copy of the command) is a reasoned trade-off or an unexamined gap: it
  is reasoned in Risks/Trade-offs, consistent with CLAUDE.md's own
  "do not refactor speculatively" / no-speculative-gates stance elsewhere in
  this repo. Not a defect.
- Considered whether the CI comment's citation "the `sdk-staging` OpenSpec
  change's design.md" (ci.yml:1279) will still resolve once the closer
  archives the change to `openspec/changes/archive/2026-09-25-sdk-staging/`
  before merge (per `docs/OPENSPEC-ARCHIVE.md`). It will: the citation names
  the change by its stable name rather than a full pre-archive path, and
  `docs/OPENSPEC-ARCHIVE.md` confirms an archived folder stays "still
  greppable" by that name. Not a defect — noting only because I checked it,
  since a stale-path citation is a known failure shape in this repo.

## Result

No architecture defects found. The change is small, single-purpose per file,
introduces no new dependency (it *removes* the `python3` dependency from the
CI staging step), avoids the two-near-identical-copies trap by having the
README be the one documented source and CI the one deliberate
byte-for-byte executor, and each new comment earns its place by explaining a
"why" (why `--inputs-from`, why no rev literal, why the `test -f` guard)
rather than restating the command. `cargo mutants` was not run: this piece
touches no Rust source (only `README.md`, `.gitignore`, `CLAUDE.md`,
`ci.yml`, and the `openspec/changes/sdk-staging/` docs), so there is nothing
in scope for it to mutate.

No findings to check off for this dimension.
