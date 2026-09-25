# Document the SDK staging step `cargo test` needs, and use the same command in CI

## Why

The README's unit-test command fails in a fresh clone or a new worktree before it
compiles anything: `error: failed to load manifest for dependency
`logos-rust-sdk``. `dialectica/rust-lib/Cargo.toml` depends on the SDK by path
(`../logos-rust-sdk-src`). Nothing outside a Nix build puts anything at that
path, and the README does not say what does. On #91 (PR #163), every agent
dispatch that ran cargo hit this and waited for the runner to stage the
directory by hand. One of them lost its worktree, because it stopped before
editing anything and the harness deletes an unchanged worktree.

The staging method is also written down three different ways, and none of them
is in the README. CI parses `dialectica/flake.lock` with Python and builds
`github:<rev>#rust-sdk-src`. `.gitignore` tells the reader to substitute a rev
from `flake.nix` by hand. The runner links a store path. One command covers all
three, and it cannot drift from the build:

```
nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src
```

`--inputs-from ./dialectica` resolves `logos-module-builder` to the input that
`dialectica/flake.lock` pins, so the staged SDK moves with the pin and no
`flake.nix` change is needed.

## What Changes

Scope is issue #168's "What changes", as corrected by the owner's comment of
2026-09-25. The issue body was edited after that comment and already carries
most of it.

1. **README, "Building":** add the staging command before the `cargo test`
   line, with one sentence on why it is needed. The README becomes the one
   place the step is documented.
2. **`.github/workflows/ci.yml`, step "Stage the logos-rust-sdk source the
   builder pins"** in the `Rust core tests` job: replace the Python snippet that
   reads the builder rev out of `flake.lock`, and the
   `nix build "github:$rev#rust-sdk-src"` after it, with the same command as the
   README. Keep the `test -f dialectica/logos-rust-sdk-src/Cargo.toml` check
   after it. The comment block above the step argues for reading the lock, and
   ends with a parenthetical saying `.gitignore` names tag `0.2.6`. That
   parenthetical is already stale: `.gitignore` now says
   `<rev from flake.nix>`. Both go with the step and are rewritten to fit the
   new command.
3. **`.gitignore`:** the comment on `dialectica/logos-rust-sdk-src` points to
   the README. It stops carrying its own staging method (today, a
   `github:logos-co/logos-module-builder/<rev from flake.nix>#rust-sdk-src`
   command with a hand-substituted rev). The issue body describes this comment
   as naming tag `0.2.6`. It no longer does, but it is still a second method,
   so the edit stands.
4. **`CLAUDE.md`:** point to the README for staging, so that an agent in a fresh
   worktree stages its own tree (`nix build` costs no approval click). It does
   not repeat the command, because two copies drift. In the same file, fix
   `nix build .#lgx` to `nix build ./dialectica#lgx`, because the repository
   root has no flake. `git ls-files` finds `dialectica/flake.nix` and no root
   `flake.nix`.

**The relative `./dialectica` form must be run, not assumed.** The owner's
comment says only the absolute-path form was run, with
`--no-link --print-out-paths`, and that the relative form was untested. The
later body says both were measured. Where the two disagree, the comment is
taken as the standing instruction here. The `dev-writer` runs the exact
README command from the repository root, then the README's `cargo test`, and
records the result in `tasks.md`. CI running the same command is the second
witness.

**Out of scope:**

- No `flake.nix` change and no re-export of the builder's output. The owner's
  comment says that if `--inputs-from` works, the re-export is unnecessary.
  `design.md` D1 records it among the alternatives.
- The other `python3` heredocs in `ci.yml`. Only the staging step's snippet is
  replaced.
- The `.#lgx` string at `ci.yml` line 1768 and in archived changes. The issue
  names `CLAUDE.md` only. Archived changes are records and are not edited.
- Anything under `.claude/`.

**Done when** (from the issue):

- In a fresh clone, the README's steps (stage, then `cargo test …`) run the
  suite with no other setup.
- CI's `Rust core tests` job stages the SDK with the same command and passes.
- `.gitignore` and `CLAUDE.md` name no other staging method.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None.

Every edit here is to developer documentation or CI tooling. None of them
changes how dialectica behaves: no wire shape, no op, no view, no stored state.
No capability in `openspec list --specs` covers how a developer builds or tests
the repository. Writing one to carry this piece would contract the gate rather
than the system, which the archived `core-e2e` and `e2e-ui-suite` changes both
declined to do. `.openspec.yaml` therefore declares `skip_specs: true` beside
its `schema:` key, and the spec row in `tasks.md` is struck through with that
reason.

## Impact

- **Modified:** `README.md`, `.github/workflows/ci.yml` (one step and the
  comment block above it), `.gitignore` (one comment), `CLAUDE.md`.
- **Also in `ci.yml`, for the `dev-writer` to check:** the `Rust core tests`
  job's `install-nix-action` comment says the job "builds one `github:` flake
  with no relative path input" and so needs no Nix version floor. After this
  change the job evaluates `./dialectica`'s lock through `--inputs-from`, so
  the reason as worded is no longer true, even if the conclusion still holds.
  The floor exists because Nix up to 2.24 rejects a lock file containing a
  relative path input. `dialectica/flake.lock` is the core module's lock, which
  the same file says builds at every version. That comment is reworded or its
  claim re-established; it is not left describing a step that no longer
  exists.
- **Removed:** the `python3` dependency of the SDK staging step.
- **Not affected:** `dialectica/` source, `dialectica/flake.nix`,
  `dialectica/flake.lock`, `dialectica/rust-lib/Cargo.toml`, the wire contract,
  and every requirement in `openspec/specs/`.
