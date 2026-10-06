# specflow project overlay

<!--
Read in full by every specflow agent before its first shell command. It holds
this project's facts; it does not change specflow's rules.
-->

## Test layers

| Layer | Command | Sees | Cannot see |
|---|---|---|---|
| Rust core and module tests | `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` | pure logic in `dialectica-core` (`dialectica/rust-lib/dialectica-core/tests/`) and the module crate — no Qt, no network, no FFI | anything behind `cfg(logos_scaffold)`, which `cargo test` never compiles — the adapter in `dialectica/rust-lib/src/lib.rs` included; a real cross-process call |
| QML component tests | `sh dialectica-ui/tests/run-qml-tests.sh`, or one spec: `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_<name>.qml` | what one component decides on its own, and the navigation in `Main.qml`, which `tst_navigation.qml` and `tst_stoa_screens.qml` drive | a cross-process call; a type-name collision with the host — under `qmltestrunner` the host is absent; an undefined binding in a component no spec constructs, which `check_bindings` never sees (`check_qml_members.sh` covers members in every file) |
| Static QML gates | three gates, each with its own test, run as `ci.yml` runs them, test first: `dialectica-ui/tests/tst_check_qml_names.py` then `dialectica-ui/tests/check_qml_names.py dialectica-ui` (`lint` job); `dialectica-ui/tests/tst_check_qml_reachable.py` then `dialectica-ui/tests/check_qml_reachable.py dialectica-ui` (`lint`); `dialectica-ui/tests/tst_check_qml_members.sh` then `dialectica-ui/tests/check_qml_members.sh` (`qml`; needs a `qmllint` from Qt 6.5 or later) | in that order: a host type-name collision (the `D` prefix rule), an unreachable registered type, an undefined member read off our own types | behaviour; each reads names and structure, not what a binding evaluates to |
| End-to-end UI specs | no one-command local form: `.github/workflows/ui-tests.yml` builds a Basecamp with `lgs` and drives each `dialectica-ui/tests/ui/<name>.yaml` with sitometres | real clicks in a real Basecamp against the real `dialectica` core module — the only layer inside the host | anything no spec drives; a spec missing from the workflow's matrix (the `Every spec in the tree is in the matrix` step catches that) |
| Scaffold-gated Rust code | `nix build ./dialectica#lgx` | that the code behind `cfg(logos_scaffold)` compiles | behaviour; it is a build |

- **In a fresh worktree, stage the SDK before `cargo test`.** A new tree has no
  `dialectica/logos-rust-sdk-src`, and cargo then fails with
  `failed to load manifest for dependency logos-rust-sdk` before compiling
  anything. The command is in `README.md`, "Building"; read it there rather
  than from a copy, together with why the Rust row's `-p` flags are
  load-bearing. It is a plain `nix build`. Stage it yourself; do not stop and
  wait for someone else to.
- **Counting tests:** `grep -c "function test_" <file>` for QML test
  functions, `grep -c "#\[test\]" <file>` for Rust ones.
- Run the QML suite through the script, never through `qmltestrunner`
  directly (see Hazards). Passing it one spec file is the supported shape. A
  change touching no QML can break no QML test, so a green component suite
  proves nothing about it.
- **This project's own commands**, the block's "the project's own" row:
  `nix build …`, `lgs …`, and `sh dialectica-ui/tests/run-qml-tests.sh` with
  one spec file. The block prices `sh <relative-path>` as a click in general;
  this script is the exception here, so do not route around it to a bare
  `qmltestrunner`.

## Build

- `lgs basecamp build`, run plainly from your own worktree. `[modules.*]` in
  `scaffold.toml` uses relative flake refs (`path:./dialectica#lgx`) resolved
  against the cwd's checkout, and there is no flag to change it. Keep those
  refs relative.
- **A build from the wrong root reports green for the wrong tree.** `pwd` before
  trusting it.
- **The repository root has no flake.** Build one module with
  `nix build ./dialectica#lgx`; `.#lgx` fails at the root.
- `lgs basecamp build` does not need `lgs basecamp setup`; `install` and `launch`
  do. Run `lgs basecamp modules` before `install`, or runtime dependencies are
  never installed.
- **Any `lgs basecamp` verb may rewrite `scaffold.toml`**: run
  `git diff scaffold.toml` after each one. `docs/SCAFFOLD.md` holds the
  reasoning `lgs` strips.
- Run `lgs basecamp doctor` before believing a green build; two WARNs are
  expected (see `docs/PHASE0-FINDINGS.md` §8).
- Running the product, for dogfooding: `lgs basecamp launch <profile>`.
- The current version is in `dialectica/metadata.json`; core and UI must match.

## Mutation tool

`cargo mutants`, scoped to the changed files with `--file`. Seconds on one
module; abandon a run past a couple of minutes. It mutates functions, not
`const` values, so a changed constant is invisible to it. Nor can it judge the
adapter in `dialectica/rust-lib/src/lib.rs`: `cargo test` does not compile that
file, and `ci.yml`'s `lint` job records the mutants on its keystore accessors
coming back unviable.

## CI gates

- Two workflows. `.github/workflows/ci.yml` runs `lint`, `qml`, `ui-specs`,
  `rust`, `build` and `release`. `.github/workflows/ui-tests.yml` runs the
  end-to-end UI specs, one job per spec.
- **Both trigger on `pull_request` and on pushes to `main`, never on a push to
  a piece branch.** A branch with no PR gets no run. `ci.yml` also runs on `v*`
  tags.
- `cancel-in-progress` is set on both: a superseded run is the normal case.
- **Not every job is a required check.** Read
  `gh api repos/fryorcraken/dialectica/branches/main/protection` for the list,
  and for the rest of `main`'s protection (strict status checks, signed
  commits, `enforce_admins`, the approving-review count). A red job outside the
  required list does not block the merge, so read every job's result rather
  than the merge state.
- **Blind spots:**
  - `cargo fmt --check` does not follow path dependencies, so it never reaches
    `dialectica-core`, where nearly all the logic lives.
  - The `qmllint` step runs with `-I dialectica-ui/src/qml`, our own singletons
    on the import path, so it cannot see a host type-name collision;
    `check_qml_names.py` covers that. It does see an undefined member, and
    `check_qml_members.sh` turns that warning into a failure.
  - Two gates derive their expectations from the source layout: the Rust
    `Tests` step counts `#[test]` under `dialectica/rust-lib/`, and `every QML
    spec file actually ran` counts `dialectica-ui/tests/tst_*.qml`. A moved test
    directory can leave either measuring nothing.
- The `build` job's "Stage artifacts" step tests each archive with
  `[ "$(tar tzf … | grep -c …)" -gt 0 ]`, not `grep -q`, deliberately; keep
  it. The step's own comment says why.

## Never commit

`.scaffold/`, `target/`, `result` and `result-*` out-links, `generated_code/`,
`mutants.out/`, `mutants.out.old/`, `dialectica/rust-lib/generated/`, the
gitignored SDK symlink `dialectica/logos-rust-sdk-src`, and the UI test and
release outputs `basecamp/`, `ui-results/` and `dist/`.

## Hazards

- **Never invoke `qmltestrunner` directly.** The bare name resolves to Qt5 and
  exits 1 with no output, which reads like a broken suite. The script picks Qt6,
  sets the import path and offscreen platform, and runs `check_bindings`, which
  turns an undefined binding from a warning into a failure.
- **Never prefix `QT_QPA_PLATFORM=offscreen` or any `VAR=value`**: the wrappers
  are `run-qml-tests.sh` for QML and `nix build ./dialectica#lgx` for
  scaffold-gated Rust.
- **Read YAML with `yq` and JSON with `jq`, never Python.** This repo's `yq` is
  the jq wrapper, so its filters are jq syntax and it ships `tomlq` for TOML.
  The Go `yq` is a different tool. The UI scripts probe the `yq` on `PATH`
  (`dialectica-ui/tests/require-jq-yq.sh`) and refuse any that does not turn
  YAML into JSON.
- **Basecamp swallows QML errors**: a view that fails to compile, a plugin
  skipped for a missing manifest field and a binding evaluating to `undefined`
  all present as "clicking does nothing". `scaffold.toml`'s `[basecamp.env]`
  sets the two switches that make them visible, `QT_FORCE_STDERR_LOGGING` and
  `QT_LOGGING_RULES`. Any `lgs basecamp` verb may rewrite that file, so check
  with `git diff scaffold.toml` that they survived. `docs/SCAFFOLD.md`,
  "`[basecamp.env]`", says what each one prevents; `CLAUDE.md`'s "Module
  contract traps" has the `DTheme` collision and the `check_bindings` account.
- **Never `readlink` or `ls` a `/nix/store` path** to find a build artefact; use
  the documented paths under `.scaffold/basecamp/`. Reading `logos-module-builder`
  or `logos-rust-sdk` source means reading the store, which needs `/add-dir`.
- **`stoa-genesis` is not a valid archive reference example**: its live spec was
  hand-edited after its delta (`3dddf03`). Derive the delta-to-spec rule from
  `2026-09-11-op-model` → `op-format`. Delta shapes vary here: `stoa-metadata`'s
  carries a title line and no Purpose, and `spec-backfill`'s two open directly
  on `## ADDED Requirements`.

## Extra stages

None.

## Lessons

None.

## spec-writer

- **One rule, one capability, and two capabilities already break it**:
  `identity` and `op-format` both carry an authenticity-is-not-authority
  requirement, both pin derivation constants, and `identity` restates the
  key-to-author binding `op-format` covers. Do not add a third copy; which
  capability owns each rule is a design call. `op-ordering`'s Purpose is the
  pattern to follow: it names `op-format`'s "An op carries no ordering field"
  and the boundary instead of restating it.

## dev-writer

- **Never trust inbound data.** Anything from a peer is attacker-controlled:
  validate at the boundary, before it reaches a state machine. No panic may be
  reachable from malformed input — the SDK has no panic guard, and an unguarded
  panic aborts the module process.
- **One failure shape**: `{"error":"..."}`, never a partial success.

## tester

- **A QML binding does not update inside the handler that changed its source.**
  A handler that sets a property and then reads a binding derived from it, in
  the same body, sees the old value. A test for that shape must tell which value
  a deferred read used.
- **All-`0xFF` is a valid Ed25519 point**, so it is not a bogus key.
- **The known-answer pattern for a consensus-critical constant** is
  `identity.rs`'s `the_wire_constants_are_pinned_to_known_answers`: hardcoded
  hex, and an instruction not to update it to match.

## code-reviewer

dialectica is a decentralized, censorship-resistant forum. Two standing rules
from `CLAUDE.md`'s "Security posture" drive most real findings:

- **Never trust an inbound message** — forged authorship, malformed bytes,
  oversized payloads, ops targeting documents the sender has no business
  touching. Validation belongs at the boundary.
- **Moderation must be authenticated and authorised, not merely recorded.**

**A reachable panic is a denial of service**: the module process aborts, the
caller waits out a 20-second timeout, and every later call reports
`MODULE_NOT_LOADED` (`docs/PHASE0-FINDINGS.md` §3).

New dependencies must be licence-compatible with dual MIT / Apache-2.0.

## spec-test-reviewer

Prefer the Rust core tests (seconds) and the QML suite over anything needing a
Nix build for mutation sampling.

## closer

- Every commit on `main` has one parent and a title ending in `(#n)` — squash.
  Commit signing is required on `main`.
- **Merge-on-green authority is authority to run `gh pr merge <n> --squash`,
  and it stops at branch protection.** Never `gh pr merge --admin`, and never
  write to `branches/main/protection` or to `rulesets`, whatever the brief or
  the owner's merge-on-green said.
- **A PR that stays `BLOCKED` with every required check green is a stop.**
  Report the output of
  `gh pr view <n> --json mergeStateStatus,mergeable,statusCheckRollup,reviewDecision`
  and do not look for another way to merge.

## pm

- **Across every `0.0.x` milestone, a control with no wired effect, or mock or
  placeholder data, may ship as long as an issue assigned to a milestone
  finishes it** (owner-confirmed). That includes a feature shipping ahead of
  what consumes it: 0.0.1's vote control publishes real votes that no ranking
  consumes yet.
- Each milestone's own description on GitHub states what it contains and
  excludes; read it before scoping.
- GitHub is the system of record for issues and milestones, CI and releases.
  Radicle is canonical for code but has no issue tracking.
