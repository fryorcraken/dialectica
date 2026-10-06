# Tasks — adopt the specflow plugin (#195)

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change replaces agent
      instructions and `CLAUDE.md` prose with the specflow plugin and its
      overlay, and alters no behaviour of dialectica, so no requirement in
      `openspec/specs/` changes. Declared as `skip_specs: true` alongside
      `schema:` in `.openspec.yaml`. Owner's decision for this run: no
      `spec-writer` step; the `dev-writer` wrote `proposal.md` and this block.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] re-review: every commit after the review round — runner
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

No test can see these tasks: no CI job, script or test reads `.claude/`,
`CLAUDE.md`'s prose or the deleted docs. Each is checked by the command or the
reading named on it.

### 1. The overlay

- [x] 1.1 Add `.claude/specflow/PROJECT.md` from the issue's draft, every line
      checked against the file it came from at `645b4e73`; corrections are in
      `design.md`, "The overlay is reconciled against this tree". Verify: all
      eight required headings present, no line reading exactly `TODO`
      (`grep -c -x "TODO" .claude/specflow/PROJECT.md` prints `0`).
- [x] 1.2 Move every dialectica-specific sentence from the deleted files into
      it, or list it under `design.md`'s "Dropped, deliberately".
- [x] 1.3 `## closer` carries #174's `--admin` and `BLOCKED` rules; the four
      #174 rules the overlay cannot carry are in `design.md`.

### 2. `CLAUDE.md`

- [x] 2.1 Delete the eight generic sections the issue lists.
- [x] 2.2 "Where to look for what" keeps the `docs/SCAFFOLD.md` and
      `docs/SOURCES.md` rows only.
- [x] 2.3 Append the block from the plugin's `skills/sync/SKILL.md`. Verify:
      `cmp` from each file's `<!-- specflow:begin v0.1.0 -->` byte to the end
      of the block reports no differing byte.

### 3. Settings and `.gitignore`

- [x] 3.1 `.claude/settings.json`: add `enabledPlugins`; `worktree.baseRef`
      unchanged.
- [x] 3.2 `.gitignore`: add `!.claude/specflow/`; keep `!.claude/agents/`.
- [ ] 3.3 Owner: add the `extraKnownMarketplaces` entry to the main checkout's
      untracked `.claude/settings.local.json`. A dispatched agent cannot write
      outside its worktree; the JSON is in the PR description.

### 4. Deletions and references

- [x] 4.1 Delete the nine `.claude/agents/*.md`, `docs/OPENSPEC-ARCHIVE.md` and
      `docs/PROJECT-MANAGEMENT.md`.
- [x] 4.2 No reference to a deleted file outside `openspec/changes/archive/`.
      Verify: `git grep -n -E "agents/README\.md|RUNNER\.md|OPENSPEC-ARCHIVE|PROJECT-MANAGEMENT" -- . ":!openspec/changes/archive/"`
      prints only this change's own folder, which names them.

### 5. Hand-off

- [x] 5.1 `openspec validate adopt-specflow-plugin --strict` passes.
- [x] 5.2 Push to the remote piece ref by refspec and open the PR with
      `Closes #195`.
