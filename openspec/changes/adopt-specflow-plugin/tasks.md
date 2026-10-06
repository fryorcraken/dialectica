# Tasks — adopt the specflow plugin (#195)

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change replaces agent
      instructions and `CLAUDE.md` prose with the specflow plugin and its
      overlay, and alters no behaviour of dialectica, so no requirement in
      `openspec/specs/` changes. Declared as `skip_specs: true` alongside
      `schema:` in `.openspec.yaml`. Owner's decision for this run: no
      `spec-writer` step; the `dev-writer` wrote `proposal.md` and this block.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — **no code changes.** Every edit this change makes
      under `dialectica/` and `dialectica-ui/` is a comment line, and the rest
      is agent instructions and prose, so there is no behaviour for a test to
      pin. Owner's decision for this run: the `tester` step is skipped.
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
reading named on it. Every command below is one plain call from the
repository root.

### 1. The overlay

- [x] 1.1 Add `.claude/specflow/PROJECT.md` from the issue's draft, every line
      checked against the file it came from at `645b4e73`; corrections are in
      `design.md`, "The overlay is reconciled against this tree". Verify, two
      commands:
      `grep -c -x "TODO" .claude/specflow/PROJECT.md` prints `0`; and
      `grep -n -A2 -x -E "## (Test layers|Build|Mutation tool|CI gates|Never commit|Hazards|Extra stages|Lessons)" .claude/specflow/PROJECT.md`
      prints eight groups, read for two things.
      - **The eight `<n>:## …` lines name the pattern's eight headings, each
        once.** A count of matching lines cannot see this: with
        `## Mutation tool` renamed `## Build`, a count still prints `8`, and
        this output shows `## Build` twice and no `## Mutation tool`.
      - **Each group's third line is `<n>-` followed by text that is not a
        heading.** An emptied section fails it one of two ways. With
        `## Mutation tool`'s body deleted, the group reads
        `<n>:## Mutation tool`, `<n>-`, `<n>:## CI gates`. With `## Lessons`'
        `None.` deleted, it reads `<n>:## Lessons`, `<n>-`,
        `<n>-## spec-writer`: the next heading is not in the pattern, so it
        prints as a context line.

      Neither command can see whether a section's content is right; that is
      the reading 1.2 names.
- [x] 1.2 Move every dialectica-specific sentence from the deleted files into
      it, or list it under `design.md`'s "Dropped, deliberately". A reading,
      with no command: no search can tell a sentence that was moved from one
      that was lost.
- [x] 1.3 `## closer` carries #174's `--admin` and `BLOCKED` rules; the #174
      rules the overlay does not carry, and why, are in `design.md`. Verify:
      `grep -n -F -e "gh pr merge --admin" -e "BLOCKED" .claude/specflow/PROJECT.md`
      prints two lines, both between the `## closer` and `## pm` headings.

### 2. `CLAUDE.md`

- [x] 2.1 Delete the eight generic sections the issue lists, and keep the
      four it keeps: "What this is", "Module contract traps", "Scaffold: what
      `lgs` does and does not do" and "Security posture". Verify:
      `grep -n -E "^## " CLAUDE.md` prints, on lines before the begin marker
      (its line number is the first field 2.3's `grep -b` prints), exactly
      these six headings in this order: "Where to look for what", "What this
      is", "Module contract traps", "Scaffold: what `lgs` does and does not
      do", "Security posture", "Shell rules for every session".
      The next hit, `## specflow`, is on the line after the marker. A deleted
      kept section, or a generic section re-added as a heading, changes the
      list. It does not compare the kept sections' text: "Module contract
      traps" was corrected by this change (`design.md`, "The overlay is
      reconciled against this tree", the `qmllint` entry), so whether the
      rest of it survived is a reading of
      `git diff origin/main...HEAD -- CLAUDE.md`.
- [x] 2.2 "Where to look for what" keeps the `docs/SCAFFOLD.md` and
      `docs/SOURCES.md` rows only. Verify: `grep -n -E "^\| " CLAUDE.md`
      prints, on lines before the begin marker, exactly three: the
      `| Read | When |` header and the `docs/SCAFFOLD.md` and
      `docs/SOURCES.md` rows. Every later hit is one of the block's own
      tables.
- [x] 2.3 Append the block from the plugin's `skills/sync/SKILL.md`. Verify:
      the block in `CLAUDE.md` is byte-identical to the plugin's. The
      reference is `agent-spec-flow` commit `77fce4d` (v0.1.0); in a clone of
      it, first `git -C <clone> diff --exit-code 77fce4d -- skills/sync/SKILL.md`
      prints nothing. Then derive the offsets fresh every time, because any
      edit above the block moves them; none is written down here for that
      reason.
      - `grep -b -n -x -F -e "<!-- specflow:begin v0.1.0 -->" -e "<!-- specflow:end -->" CLAUDE.md`,
        and the same on `<clone>/skills/sync/SKILL.md`. Each prints two
        lines, `<line>:<offset>:<marker>`. `-x` matches only a line that is
        the bare marker, which keeps out `SKILL.md`'s own prose about the
        markers and makes each offset a line start.
      - The span is the end marker's offset, plus 22 (`<!-- specflow:end -->`
        is 21 bytes, then its newline), minus the begin marker's offset. It
        comes out the same for both files, or the blocks already differ in
        length.
      - `cmp -i <CLAUDE.md begin offset>:<SKILL.md begin offset> -n <span> CLAUDE.md <clone>/skills/sync/SKILL.md`
        prints nothing and exits 0.

### 3. Settings and `.gitignore`

- [x] 3.1 `.claude/settings.json`: add `enabledPlugins`; `worktree.baseRef`
      unchanged. Verify: `jq -c . .claude/settings.json` prints exactly
      `{"worktree":{"baseRef":"head"},"enabledPlugins":{"specflow@agent-spec-flow":true}}`,
      with no `permissions` and no `extraKnownMarketplaces` (`design.md` says
      why neither is tracked).
- [x] 3.2 `.gitignore`: add `!.claude/specflow/`; keep `!.claude/agents/`.
      Verify:
      `git check-ignore --no-index .claude/specflow/PROJECT.md .claude/agents/any.md .claude/settings.json .claude/settings.local.json`
      prints exactly `.claude/settings.local.json`: the overlay, a project
      agent and `settings.json` are admitted, and the machine-local file is
      not. Leave out `-v`, which also prints the pattern that admits
      `settings.json`.
- [ ] 3.3 Owner, after the merge: follow `README.md`'s "Working with the
      agent flow". The clone of `https://github.com/fryorcraken/agent-spec-flow`
      and `claude plugin marketplace add` on it can run any time.
      `claude plugin install specflow@agent-spec-flow --scope project` runs
      after the merge, from the root of a checkout of `main` that has it:
      there `git diff .claude/settings.json` shows nothing afterwards. Run
      before the merge from the root checkout, it would write
      `enabledPlugins` into that checkout's tracked `settings.json`. It
      changes the owner's Claude Code configuration, which a dispatched agent
      does not touch.
- [ ] 3.4 Owner, after 3.3: a session started in this repository lists the
      `specflow:*` agents and skills (the issue's fourth Check). It is the
      only check that shows `enabledPlugins` and 3.3 worked; every other check
      passes on a tree where the plugin never loads.

      Both rows are post-merge and nothing gates them: neither `closer` reads
      this section, and the archive freezes them unticked. `design.md`, "The
      marketplace entry is not in this change", says why that is accepted.

### 4. Deletions and references

- [x] 4.1 Delete the nine `.claude/agents/*.md`, `docs/OPENSPEC-ARCHIVE.md` and
      `docs/PROJECT-MANAGEMENT.md`. Verify:
      `git ls-files .claude/agents docs/OPENSPEC-ARCHIVE.md docs/PROJECT-MANAGEMENT.md`
      prints nothing.
- [x] 4.2 No reference to a deleted file outside `openspec/changes/archive/`.
      Verify, case-insensitive, over all eleven deleted names, the "agents
      README" and "role file" spellings, any path under `.claude/agents/`,
      and the two docs with `-` or `_`:
      `git grep -n -i -E "agents/README\.md|agents README|RUNNER\.md|\.claude/agents/[a-z]|OPENSPEC[-_]ARCHIVE|PROJECT[-_]MANAGEMENT|role file|(closer|code-reviewer|design-reviewer|dev-writer|spec-test-reviewer|spec-writer|tester)\.md" -- . ":!openspec/changes/archive/" ":!openspec/changes/adopt-specflow-plugin/"`
      prints nothing. This change's own folder is excluded because it names
      them. `\.claude/agents/[a-z]` needs a character after the slash, so the
      bare directory, which `.gitignore` and the block legitimately name, is
      not a hit; `OPENSPEC.ARCHIVE` would be wrong, because it matches the
      words "openspec archive". The search covers the spellings it names and
      no others. Comments that quote `CLAUDE.md` wording the block replaced
      are a different shape again; they were repointed by reading.
- [x] 4.3 Changes in flight keep working. The issue's Check names
      `openspec/changes/ui-remaining-screens/tasks.md`, which no longer
      exists: that change is archived as
      `openspec/changes/archive/2026-09-28-ui-remaining-screens/`, so the
      Check as written fails on a missing file whatever this change does.
      The corrected check has two parts.
      (a) `git diff --name-only origin/main...HEAD -- openspec` lists only
      paths under `openspec/changes/adopt-specflow-plugin/` and
      `openspec/changes/relevance-votes/tasks.md`, whose one edit repoints a
      citation of the deleted agents README and touches no stage-block line.
      `--name-only`, not `--stat`: `--stat` shortens a long path to
      `.../<tail>`, so an edit to another change's file with the same tail
      reads like one of this change's. Run it before the `closer`'s archive
      commit, which moves this change's folder under `archive/`.
      (b) For each change `openspec list` shows — `adopt-specflow-plugin` and
      `relevance-votes` — `grep -c "^## Stages" openspec/changes/<name>/tasks.md`,
      the plugin runner's own detection: `1` for this change, `0` for
      `relevance-votes`, which has had no stage block since before this piece
      and reads as untracked under either flow.

### 5. Hand-off

- [x] 5.1 `openspec validate adopt-specflow-plugin --strict` passes.
- [x] 5.2 Push to the remote piece ref by refspec and open the PR with
      `Closes #195`.
