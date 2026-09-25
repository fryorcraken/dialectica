# Correctness review — #168 sdk-staging

Dimension covered: **correctness only** (security, readability and
architecture are other reviewers' rows).

Read `gh issue view 168 --json body,comments` before reviewing, including the
owner's 2026-09-25 comment that corrects the body (the local runs on #91 did
use the pinned SDK; `flake.nix` likely needs no change; `--inputs-from
./dialectica` is the fix). The piece's proposal.md and design.md already
incorporate that correction, and nothing in the diff contradicts it.

## What I verified, and how

This piece touches no Rust source (`git diff 8368b2f...HEAD --name-only`:
`README.md`, `.github/workflows/ci.yml`, `.gitignore`, `CLAUDE.md`, and the
`openspec/changes/sdk-staging/` files only), so there is no code path to
mutate and `cargo mutants` does not apply. I instead re-ran, from scratch in
this worktree, every command the change documents or runs in CI, rather than
reading them for plausibility:

1. **Staged the SDK** with the exact README/CI command:
   `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`.
   Succeeded; `readlink dialectica/logos-rust-sdk-src` gives
   `/nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src`, byte-for-byte
   the store path recorded in design.md's "Measured" table and in the issue's
   owner comment. `test -f dialectica/logos-rust-sdk-src/Cargo.toml` passes,
   matching the CI step's own guard.
2. **Ran `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`.**
   `dialectica` 0 tests, `dialectica-core` 1180 passed, `end_to_end` 30 passed,
   0 failed — exactly the counts tasks.md task 2.1 records.
3. **Ran `nix build ./dialectica#lgx`** (not `.#lgx`, since the repo root has
   no flake — confirmed with `git ls-files flake.nix` returning nothing).
   Succeeded (`result` → `logos-dialectica-module-lib-lgx-0.1.0`); the one
   line of output was an ignored eval-cache SQLite-busy warning, not a build
   failure.
4. **Cross-checked every factual claim the diff and its comments make against
   the tree**, not just against each other:
   - `dialectica/rust-lib/Cargo.toml:37` really has
     `logos-rust-sdk = { path = "../logos-rust-sdk-src" }`, so the README's
     stated failure mode (`failed to load manifest for dependency
     logos-rust-sdk`) is the real one.
   - `jq "[.nodes[] | .locked.type] | unique" dialectica/flake.lock` gives
     `[null, "git", "github"]` — confirms the rewritten `install-nix-action`
     comment's claim (D5) that the lock has no relative `path` input, so the
     job needs no Nix version floor for that reason.
   - `yq '.jobs.rust.steps[] | select(.name == "Stage the logos-rust-sdk
     source the builder pins") | .run'` reads back exactly `set -eu` + the
     README's staging line + the `test -f` guard — CI runs the README's line
     byte for byte, as D3 and task 3.1 claim.
   - `git grep -n "rust-sdk-src"` across the touched files finds exactly one
     occurrence of the full command in `README.md` and one in `ci.yml`, and
     none in `.gitignore` or `CLAUDE.md` — confirms the issue's third "Done
     when" and task 4.1/4.2's grep claims.
   - The only other `.#lgx` occurrence in `ci.yml` (line ~1761) is a comment
     naming a module attribute suffix, not a command to run, so the
     proposal's claim that it is correctly out of scope holds.
   - `git grep -n "python3" -- .github/workflows/ci.yml` shows the ten other
     `python3` heredocs in the file are untouched, consistent with the
     proposal's scope note that only the staging step's snippet is replaced.

## Result

No correctness defects found. Every measured claim in `proposal.md`,
`design.md` and `tasks.md` reproduced exactly when re-run independently in
this worktree, and the code (CI YAML, README prose, `.gitignore` comment,
CLAUDE.md bullet) is internally consistent with those claims. This is a
docs/CI change with no state machine, no peer input and no Rust logic, so the
"never trust an inbound message" and reachable-panic concerns this repo
usually centers correctness review on do not apply here — noted rather than
silently skipped.

No unticked boxes in this file: nothing found here needs the `dev-writer` to
act. This file's presence records that the correctness dimension was
reviewed, not skipped.
