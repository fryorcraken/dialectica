# Tasks

## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec delta: docs and CI tooling only, `skip_specs: true` in `.openspec.yaml` says why; `proposal.md` is written
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — no test added: the change touches neither Rust core logic nor QML, so neither existing test layer can see it. The one candidate, a gate asserting README and `ci.yml` carry the same staging command, is design.md's explicit Non-Goal ("A gate is a follow-up if drift is ever observed; it is not built speculatively here" — Risks/Trade-offs on D3), reasoning this repo has applied elsewhere (no speculative gates). Independently confirmed instead of gated: `git grep -F` of the exact command
      `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`
      finds exactly one byte-identical occurrence in `README.md` and one in
      `.github/workflows/ci.yml`, and zero in `.gitignore` or `CLAUDE.md` (issue's third "Done when"). The other two "Done when" items are checked by running the commands (task 2.1) and by CI's own first run (task 3.4), neither of which a unit or component test can substitute for.
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [x] re-review after `67a6605`: readability — `code-reviewer` — the dev-writer's answer to the readability finding reworded prose in `proposal.md` and `design.md`
- [x] re-review after `67a6605`: design — `design-reviewer` — the same commit edited D1's Alternatives in `design.md`. Not re-reviewed: correctness, security, architecture, spec-test, because the commit touches no README, CI, `.gitignore`, `CLAUDE.md` or test ground
- [x] findings all ticked, `findings/` deleted — `closer` — all six findings files were clean-pass reports (five with zero checkboxes, per the reviewers' brief to write the file even with nothing found; `readability.md`'s one finding was fixed in `67a6605` and re-reviewed clean). Scanned all six for prose outside a checkbox asking for a change before deleting: none found.
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

No spec delta, so nothing here is a spec requirement. The contract is the
issue's "Done when", and each task names the run that shows it. None of this is
reachable by `cargo test` or the QML suite: a green unit suite says nothing
about a README line or a CI step, and none of these tasks claims otherwise.

### 1. Measure the command before writing it anywhere

- [x] 1.1 Confirm the tree starts with no staged SDK. Verified:
      `git status --ignored` in the fresh agent worktree listed no ignored path
      at all, so `dialectica/logos-rust-sdk-src` and `target/` were both absent.
- [x] 1.2 Run the relative `--inputs-from ./dialectica` form from the tree root,
      exactly as the README carries it. Verified: it printed nothing, exit 0,
      and left `dialectica/logos-rust-sdk-src` →
      `/nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src`.
      `--no-link --print-out-paths` printed the same path, and so did building
      `github:logos-co/logos-module-builder/<rev in dialectica/flake.lock>`
      directly (design.md D1).
- [x] 1.3 Show `--inputs-from` is what pins it. Verified: without it the name
      does not resolve (`cannot find flake 'flake:logos-module-builder'`), and
      with a probe registry mapping the name to another flake, the probe wins
      without the flag and the lock wins with it (design.md D1).
- [x] 1.4 Show that re-running replaces a stale link, which the README's "again
      whenever the pin moves" relies on. Verified: link pointed at an unrelated
      store path, command re-run, link back on `lsdgw…` (design.md D2).

### 2. Write it once, in the README

- [x] 2.1 README "Building": the staging command before the `cargo test` line,
      with one sentence on why. Verified: from the tree in 1.1, the README's two
      commands run as written gave `dialectica` 0 tests,
      `dialectica-core` 1180 passed, `end_to_end` 30 passed, 0 failed, with no
      other setup.

### 3. Make CI run the README's line

- [x] 3.1 `ci.yml` "Stage the logos-rust-sdk source the builder pins": the
      Python lock parser and `nix build "github:$rev#rust-sdk-src" -L` replaced
      by the README's command byte for byte; `test -f …/Cargo.toml` kept.
      Verified: `git grep -F` of the command finds exactly the README line and
      the CI line, and `yq` reads the step's `run` back as those two lines
      after `set -eu`.
- [x] 3.2 The comment block above the step rewritten for the new command, and
      the stale parenthetical about `.gitignore` naming tag `0.2.6` removed,
      since `.gitignore` no longer carries any command.
- [x] 3.3 The `install-nix-action` comment in the `rust` job re-grounded
      (design.md D5). Verified: `jq "[.nodes[] | .locked.type] | unique"
      dialectica/flake.lock` gives `null` (the root), `git`, `github`, and no
      `path`.
- [ ] 3.4 CI's `Rust core tests` job stages with the new step and passes. Not
      verifiable before the PR's first run: tick it from that run, not from the
      local measurement in 2.1.

### 4. Point everything else at the README

- [x] 4.1 `.gitignore`: the comment on `dialectica/logos-rust-sdk-src` points to
      README "Building" and carries no command. Verified: `git grep -F
      rust-sdk-src -- .gitignore` finds only the ignore line itself.
- [x] 4.2 `CLAUDE.md`: a bullet telling an agent in a fresh worktree to stage its
      own tree, pointing at the README without repeating the command; and
      `nix build .#lgx` → `nix build ./dialectica#lgx`. Verified:
      `git grep -F "#lgx" -- CLAUDE.md` finds the corrected command, the
      sentence saying why `.#lgx` fails, and the unrelated
      `path:./dialectica#lgx` flake ref, and no instruction to run `.#lgx`;
      `git ls-files flake.nix` finds no root flake.
- [x] 4.3 `nix build ./dialectica#lgx` from the tree root. Verified: exit 0.
