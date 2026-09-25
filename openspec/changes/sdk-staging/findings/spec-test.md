# spec-test review — sdk-staging (#168)

Read issue #168 fresh with `gh issue view 168 --json body,comments`, including
the owner's 2026-09-25 comment, before writing anything below.

## Scope note

This change has `skip_specs: true` (`.openspec.yaml` correctly pairs it with
`schema: spec-driven`, so the marker is honoured) and adds no test — the
`tests` stage row is struck through in `tasks.md`. There is no spec to walk
scenario-by-scenario (§1 of my brief) and no spec-delta move to check (§4).
What is checkable, and what I checked, is whether the strike is justified and
whether the verification recorded in `tasks.md` actually holds against the
issue's "Done when" and against the tree.

## What I verified independently (not just re-read)

- **The strike's premise** — "neither existing test layer can see it" — holds.
  The diff (`git diff 8368b2f...HEAD`) touches only `README.md`, `.gitignore`,
  `CLAUDE.md`, `.github/workflows/ci.yml`, and the change folder; nothing
  under `dialectica/rust-lib/src` or `dialectica-ui/`. Neither `cargo test`
  nor the QML suite executes README prose, a `.gitignore` comment, or a
  workflow YAML step, so no unit/component layer could pin this even if asked
  to.
- **The one candidate test (a README/ci.yml drift gate) is a recorded
  Non-Goal**, not an excuse invented at test time: `design.md`'s
  Risks/Trade-offs under D3 says explicitly "a gate is a follow-up if drift is
  ever observed; it is not built speculatively here," consistent with this
  repo's stated no-speculative-gates practice (also cited for `core-e2e` /
  `e2e-ui-suite`).
- **The `git grep -F` claim** (tasks.md, tests row and task 3.1): ran
  `git grep -F` for the exact command
  `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`.
  Confirmed one byte-identical occurrence in `README.md`, one in
  `.github/workflows/ci.yml` (also present in `design.md`/`proposal.md`,
  which the claim never denies), and, separately, `git grep -F rust-sdk-src --
  .gitignore CLAUDE.md` found only the ignored-path line and a prose mention
  of the path — no staging command in either file. Matches the issue's third
  "Done when" ("`.gitignore` and `CLAUDE.md` name no other staging method").
- **Task 3.1's `yq` claim**: `yq -r '.jobs.rust.steps[] | select(.name ==
  "Stage the logos-rust-sdk source the builder pins") | .run' .github/workflows/ci.yml`
  reads back exactly the README's command after `set -eu`, plus the
  `test -f … /Cargo.toml` guard — matches.
- **Task 3.3's `jq` claim**: `jq "[.nodes[] | .locked.type] | unique"
  dialectica/flake.lock` returns `[null, "git", "github"]` — matches exactly
  (no `path` node), which is what the reworded Nix-version comment in
  `ci.yml` now asserts.
- **Task 1.1 / freshness**: `git status --ignored` in this tree (before
  staging) showed nothing ignored at all — no `dialectica/logos-rust-sdk-src`,
  matching the claimed starting state.
- **Ran the README's own steps from scratch, per the dispatch instructions**:
  - `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`
    — exit 0, no output, and `git status --ignored` afterwards showed the
    out-link at `dialectica/logos-rust-sdk-src`.
  - `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
    -p dialectica-core` — `dialectica`: 0 tests; `dialectica-core`: 1180
    passed, 0 failed; `end_to_end`: 30 passed, 0 failed. Byte-for-byte what
    task 2.1 and the tester's strike commit claim.
  - `nix build ./dialectica#lgx` — exit 0.
  This independently satisfies the issue's first "Done when" item (fresh
  clone/worktree, README steps run the suite with no other setup) — this
  worktree had never had the SDK linked before I ran the command myself.
- **Second "Done when" item** (CI's `Rust core tests` job stages with the new
  step and passes): confirmed live via
  `gh pr view 175 --json state,statusCheckRollup,headRefName,baseRefName` —
  `Rust core tests` shows `conclusion: SUCCESS` on PR #175, against commit
  `c658303` (the commit carrying the `ci.yml` step change). Task 3.4 in
  `tasks.md` is still unticked ("tick it from that run, not from the local
  measurement") even though that run has already happened and passed — not a
  soundness defect in what's recorded, just a checkbox that is now stale and
  could be ticked by whoever picks this up next; it does not block the
  `tests` stage strike, which is what I was asked to judge.
- **Issue vs. comment reconciliation**: `proposal.md` states the owner's
  2026-09-25 comment (relative `./dialectica` form "untested" at comment time)
  is taken as the standing instruction over the later-edited issue body
  (which claims both forms were measured), and then has task 1.2 actually run
  the relative form and record its result rather than just trusting either
  claim. Read the comment myself via `gh issue view 168 --json
  body,comments`; this matches what `proposal.md` and `design.md` report, and
  the independent run above reproduces the same outcome.
- **Out-of-scope items respected**: `git grep -n "\.#lgx" -- .github/workflows/ci.yml
  CLAUDE.md` shows `ci.yml`'s unrelated `.#lgx` comment (line 1761, about DEV
  `#app` modules) is untouched, and `CLAUDE.md`'s is fixed to
  `./dialectica#lgx` — matches the stated scope boundary. No `.claude/` files
  changed (`git diff 8368b2f...HEAD --stat -- .claude` is empty).

## Findings

None block the merge. The `tests` stage strike is justified by a real
structural gap (no test layer executes README/CI/.gitignore prose), backed by
a design decision recorded before the fact (not rationalized after), and every
measurement recorded in `tasks.md` that I could independently re-run
reproduced exactly what is claimed.

## Not reached

Part 2 of my brief (mutate a test, watch it fail) does not apply: this piece
adds no test file to mutate. The verification runs above (staging, `cargo
test`, `nix build ./dialectica#lgx`) are what stand in its place, per the
dispatch instructions, and are recorded above rather than as a mutation.
