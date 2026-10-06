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
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
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
      `design.md`, "The overlay is reconciled against this tree". Verify,
      three commands:
      `grep -c -E "^## (Test layers|Build|Mutation tool|CI gates|Never commit|Hazards|Extra stages|Lessons)$" .claude/specflow/PROJECT.md`
      prints `8`;
      `grep -c -x "TODO" .claude/specflow/PROJECT.md` prints `0`; and
      `grep -n -A2 -E "^## (Test layers|Build|Mutation tool|CI gates|Never commit|Hazards|Extra stages|Lessons)$" .claude/specflow/PROJECT.md`
      shows every heading's third line as `<n>-` followed by text. An emptied
      section shows a second `<n>:## …` heading there instead: with
      `## Mutation tool`'s body deleted, the first two still print `8` and
      `0`, and the third prints `58:## Mutation tool`, `59-`, `60:## CI gates`.
      None of the three can see whether a section's content is right; that is
      the reading 1.2 names.
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
      of the block reports no differing byte. The reference is
      `agent-spec-flow` commit `77fce4d` (v0.1.0); in a clone of it, first
      `git -C <clone> diff --exit-code 77fce4d -- skills/sync/SKILL.md`
      prints nothing. Then derive the offsets fresh, since any edit above the
      block moves them:
      - `grep -b -n -F "<!-- specflow:" CLAUDE.md` and the same on
        `<clone>/skills/sync/SKILL.md`. Search for the whole marker, from
        `<!--`: this `grep` prints the offset of the match, not of the line,
        so a pattern starting at `specflow:` lands 5 bytes into the line.
      - The span is the begin offset to the end of the end-marker line, which
        is 22 bytes (`<!-- specflow:end -->` and its newline).
      - `cmp -i <claude begin>:<skill begin> -n <span> CLAUDE.md <clone>/skills/sync/SKILL.md`
        prints nothing and exits 0.

      Worked at this change's last edit to `CLAUDE.md`: `CLAUDE.md` begin
      23900, end line 33368, so 33368 + 22 = 33390, its byte length, and the
      span is 33390 − 23900 = 9490; `SKILL.md` begin 1796, end line 11264, so
      11264 + 22 − 1796 = 9490. `cmp -i 23900:1796 -n 9490 …` exits 0.

### 3. Settings and `.gitignore`

- [x] 3.1 `.claude/settings.json`: add `enabledPlugins`; `worktree.baseRef`
      unchanged.
- [x] 3.2 `.gitignore`: add `!.claude/specflow/`; keep `!.claude/agents/`.
- [ ] 3.3 Owner: follow `README.md`'s "Working with the agent flow": clone
      `https://github.com/fryorcraken/agent-spec-flow`, run
      `claude plugin marketplace add` on that clone, then
      `claude plugin install specflow@agent-spec-flow --scope project` from
      the repository root. It changes the owner's Claude Code configuration,
      which a dispatched agent does not touch.
- [ ] 3.4 Owner, after the merge: a session started in this repository lists
      the `specflow:*` agents and skills (the issue's fourth Check). It is the
      only check that shows `enabledPlugins` and 3.3 worked; every other check
      passes on a tree where the plugin never loads.

### 4. Deletions and references

- [x] 4.1 Delete the nine `.claude/agents/*.md`, `docs/OPENSPEC-ARCHIVE.md` and
      `docs/PROJECT-MANAGEMENT.md`.
- [x] 4.2 No reference to a deleted file outside `openspec/changes/archive/`.
      Verify, over all eleven deleted names and the "agents README" spelling,
      case-insensitive:
      `git grep -n -i -E "agents/README\.md|agents README|RUNNER\.md|OPENSPEC-ARCHIVE|PROJECT-MANAGEMENT|(closer|code-reviewer|design-reviewer|dev-writer|spec-test-reviewer|spec-writer|tester)\.md" -- . ":!openspec/changes/archive/" ":!openspec/changes/adopt-specflow-plugin/"`
      prints nothing. This change's own folder is excluded because it names
      them. Comments that quote `CLAUDE.md` wording the block replaced are a
      different shape the search cannot see; they were repointed by reading.
- [x] 4.3 Changes in flight keep working. The issue's Check names
      `openspec/changes/ui-remaining-screens/tasks.md`, which no longer
      exists: that change is archived as
      `openspec/changes/archive/2026-09-28-ui-remaining-screens/`, so the
      Check as written fails on a missing file whatever this change does.
      The corrected check has two parts.
      (a) `git diff --stat origin/main...HEAD -- openspec` lists only this
      change's folder and `openspec/changes/relevance-votes/tasks.md`, whose
      one edit repoints a citation of the deleted agents README and touches no
      stage-block line.
      (b) For each change `openspec list` shows — `adopt-specflow-plugin` and
      `relevance-votes` — `grep -c "^## Stages" openspec/changes/<name>/tasks.md`,
      the plugin runner's own detection: `1` for this change, `0` for
      `relevance-votes`, which has had no stage block since before this piece
      and reads as untracked under either flow.

### 5. Hand-off

- [x] 5.1 `openspec validate adopt-specflow-plugin --strict` passes.
- [x] 5.2 Push to the remote piece ref by refspec and open the PR with
      `Closes #195`.
