# spec-test re-review — adopt-specflow-plugin (#195)

Re-review of the checks as they stand at `3b32d6f8`, against issue #195 (read
fresh with `gh api`), `proposal.md`, and the first round's answered boxes in
`findings/spec-test.md`, each treated as a claim. Every mutation below was made
with `Edit` and undone with `git restore`; `git status --short` was empty
before this file was written. The plugin clone at
`/home/fryorcraken/src/fryorcraken/agent-spec-flow` is clean and was not
edited.

- [x] **`dev-writer`** — `tasks.md:39-43` (1.1, third command) — the reading rule passes an emptied `## Lessons`, and the heading count passes a duplicated heading that hides a missing one
      **Scenario:** `## Lessons` is the last required heading, and the next heading, `## spec-writer`, is not in the pattern. `grep -A2` therefore prints it as a context line, `141-## spec-writer`, which is "`<n>-` followed by text", exactly what the rule accepts. The failure shape the task describes, a second `<n>:## …` heading, never appears. Separately, the count `8` counts matching lines, not distinct names: rename `## Mutation tool` to `## Build` and the overlay has seven of the eight required headings and the count still prints `8`. The third command's reading cannot catch it either, since the renamed heading still has text beneath it; I inferred that and did not run it.
      **Measured:** (1) Deleted `None.` and its blank line under `## Lessons`. The commands printed `8`, `0`, and a last group of `139:## Lessons`, `140-`, `141-## spec-writer`, all accepted by the rule as written. (2) Renamed `## Mutation tool` to `## Build`. The first command printed `8`. The two mutations the first round named still behave as claimed. Emptying `## Mutation tool` printed `8`, `0`, then `58:## Mutation tool`, `59-`, `60:## CI gates`, matching the task's text exactly. Misspelling it as `## Mutation tools` printed `7`. Fix: in the reading rule, treat a third line starting `## ` as empty whatever its separator, and check that the eight printed headings are eight distinct names. Severity: low. Only `## Lessons` is exposed today, but it is one of the two sections whose body is a placeholder, so it is the likeliest one to be blanked.
      **Fixed** in the commit "Harden the tasks.md checks". 1.1 is now two
      commands: the `TODO` count, and the `-A2` listing (with `-x` in place of
      `^…$`) read for two things: the eight heading lines name the eight
      required headings each once, and each group's third line is text that
      is not a heading. The count of matching lines is dropped, since the
      listing subsumes it. Both of this box's mutations are written into the
      task as the failures it shows. Run, each with `Edit` and undone with
      `git restore`: on the current tree, eight different headings and eight
      text lines; with `None.` and its blank line deleted under `## Lessons`,
      the last group reads `139:## Lessons`, `140-`, `141-## spec-writer`,
      a heading in the third line; with `## Mutation tool` renamed
      `## Build`, `## Build` appears at 37 and 58 and `## Mutation tool`
      not at all.

- [x] **`dev-writer`** — `tasks.md:72-75` (2.3, worked example) — the worked numbers are stale, and run as written they fail
      **Scenario:** a re-reviewer or the `closer` copies the worked command, gets "differ: byte 1", and reads it as a modified block. The derivation above it is correct. But the sentence "Worked at this change's last edit to `CLAUDE.md`" stopped being true when `3b32d6f8` added 19 lines above the block.
      **Measured:** `grep -b -n -F "<!-- specflow:" CLAUDE.md` now prints `423:24944:` and `603:34412:`, and `wc -c CLAUDE.md` prints `34434` = 34412 + 22. So the span is 34434 − 24944 = 9490. On `SKILL.md` it prints 1796 and 11264, the same as recorded, so 11264 + 22 − 1796 = 9490. `cmp -i 24944:1796 -n 9490 CLAUDE.md <clone>/skills/sync/SKILL.md` exits 0 with no output. The task's `cmp -i 23900:1796 -n 9490 …` exits 1: "differ: byte 1, line 1". Also, `grep` on `SKILL.md` prints four hits, not two. Lines 16 and 17 (offsets 701 and 739) are the skill's prose mentioning the markers, and the begin and end lines are 40 and 220. The task should say to take the hits whose line is the bare marker. Drop the numbers or label them as an example that goes stale. Severity: low. The check itself holds. Changing one byte in the block (`pipe` to `pipf` at `CLAUDE.md:522`) made `cmp` report `differ: byte 5596, line 100` and exit 1. `git -C <clone> diff --exit-code 77fce4d -- skills/sync/SKILL.md` prints nothing, and `git -C <clone> log --oneline -3 77fce4d` shows `77fce4d specflow v0.1.0: extract …`. That pins the reference, so the same edit made to both copies cannot hide: `git diff <commit> -- <path>` compares the clone's working file against `77fce4d`. I reasoned this rather than measured it, because the clone is the owner's and I did not edit it.
      **Fixed** in the commit "Harden the tasks.md checks". The worked
      numbers are gone, and 2.3 says none is written down because any edit
      above the block moves them. The `grep` now takes `-x -F` with the two
      full marker lines, so it matches only the bare-marker lines (not
      `SKILL.md`'s prose at its lines 16 and 17) and each offset is a line
      start; that also retires the "5 bytes into the line" caution. The span
      is stated as a derivation (end offset + 22 − begin offset, the same for
      both files). Run at this commit: `CLAUDE.md` `423:24944`, `603:34412`;
      `SKILL.md` `40:1796`, `220:11264`; span 9490 on both;
      `cmp -i 24944:1796 -n 9490 CLAUDE.md <clone>/skills/sync/SKILL.md`
      exits 0 with no output; `git -C <clone> diff --exit-code 77fce4d --
      skills/sync/SKILL.md` prints nothing. Mutation, undone with
      `git restore`: one line inserted above the block moved the derived
      offsets to 24984 and 34452, which is what "derive fresh" is for. Not
      run by me: a one-byte edit inside the block, and the `cmp` against the
      shifted file. The session's permission classifier refused both as
      modifications of `CLAUDE.md`'s instructions, so the `cmp`'s failure on
      a changed byte rests on this box's own measurement (`pipe` to `pipf`,
      `differ: byte 5596, line 100`, exit 1).

- [x] **`dev-writer`** — `tasks.md:97-103` (4.2) — the widened search still enumerates spellings, and three new ones pass it
      **Scenario:** a comment cites a deleted file by its path without `.md`, by role-file prose, or with an underscore. All three survive, and the search prints nothing, which is the issue's whole check.
      **Measured:** added `` Per `.claude/agents/RUNNER` and the closer's role file, and docs/OPENSPEC_ARCHIVE. `` to `openspec/changes/relevance-votes/tasks.md:33`. The 4.2 command printed nothing. As a control I appended `RUNNER.md` to the same line, and the command printed that line (`tasks.md:35`), so the search was reading the mutated working tree. Then restored. A path-prefix form closes these three spellings and is clean today: `git grep -n -i -E "\.claude/agents/[a-z]|OPENSPEC[-_]ARCHIVE|PROJECT[-_]MANAGEMENT|role file" -- . ":!openspec/changes/archive/" ":!openspec/changes/adopt-specflow-plugin/"` prints nothing on `3b32d6f8`. The `[a-z]` keeps out the bare-directory hits at `.gitignore:49,69` and `CLAUDE.md:467`, which are legitimate. (`OPENSPEC.ARCHIVE` with `.` is wrong: it matches "openspec archive" at `relevance-votes/tasks.md:33`.) Add it, or OR it into the existing pattern. Whichever is chosen, say in 4.2 that the search covers named spellings only, beside the limit it already states for quoted wording. Severity: low. Both live citations the first round found are gone. My broad sweep over `\.claude/agents|RUNNER\b|role file|OPENSPEC.ARCHIVE|project.management|agents/[a-z-]+` found only prose about test runners and CI runners, and the legitimate hits above.
      **Fixed** in the commit "Harden the tasks.md checks". The path-prefix
      alternatives are ORed into 4.2's one pattern
      (`\.claude/agents/[a-z]|OPENSPEC[-_]ARCHIVE|PROJECT[-_]MANAGEMENT|role file`),
      and 4.2 now says the search covers the spellings it names and no
      others, why `[a-z]` follows the slash, and why `OPENSPEC.ARCHIVE` is
      wrong. Run: the widened search prints nothing on the current tree.
      With this box's line added to `relevance-votes/tasks.md` it prints that
      line (`tasks.md:34`), and the old pattern, run on the same mutated
      tree, prints nothing. Restored with `git restore`.

- [x] **`dev-writer`** — `tasks.md:110-113` (4.3 (a)) — `git diff --stat` truncates long paths, so its output cannot show which folder a file is in
      **Scenario:** another change's folder gains an edit to a file whose tail matches one of this change's. The stat line reads `.../findings/design-review.md` either way, and the reader of "lists only this change's folder" cannot tell the two apart.
      **Measured:** edited line 1 of `openspec/changes/archive/2026-09-28-ui-remaining-screens/findings/design-review.md`. 4.3(a) as written reads committed history only (`origin/main...HEAD`), so I ran its working-tree equivalent, `git diff --stat 645b4e73 -- openspec`, where `645b4e73` is `git merge-base origin/main HEAD`. It printed two identical-looking lines, `.../findings/design-review.md | 102 +++++` (this change's) and `.../findings/design-review.md | 2 +-` (the archived change's). Then restored. `git diff --name-only origin/main...HEAD -- openspec` prints full paths. Today it lists the ten `adopt-specflow-plugin/` files and `openspec/changes/relevance-votes/tasks.md`, which matches the claim. The one-line `relevance-votes` diff is the citation repoint and touches no stage block. Use `--name-only`, or `--stat=200`. Severity: low. 4.3(b) is sound. On the current tree it prints `adopt-specflow-plugin/tasks.md:1` and `relevance-votes/tasks.md:0`. With this change's `## Stages` renamed to `## Stage block` it printed `:0` for both, so a live change losing its block is caught. Renaming the archived change's `## Stages` was caught too, but only by (a), as `.../2026-09-28-ui-remaining-screens/tasks.md | 2 +-`, readable because that path's tail is unique.
      **Fixed** in the commit "Harden the tasks.md checks". 4.3(a) uses
      `git diff --name-only origin/main...HEAD -- openspec`, says why not
      `--stat`, and says to run it before the `closer`'s archive commit
      moves this folder. Run: on the current tree it lists only
      `adopt-specflow-plugin/` paths and `relevance-votes/tasks.md`. With
      line 1 of the archived `ui-remaining-screens/findings/design-review.md`
      edited, the working-tree form `git diff --name-only 645b4e73 --
      openspec` printed that file's full path beside this change's
      `findings/design-review.md`. Restored with `git restore`. 4.3(b) on the
      current tree: `adopt-specflow-plugin/tasks.md:1`,
      `relevance-votes/tasks.md:0`.

- [x] **`dev-writer`** — `tasks.md:26-28` against rows 2.1, 2.2, 3.1, 3.2 and 4.1, and the issue's "Kept" list — five ticked rows name no command and no reading, although the preamble says each row names one, and "Kept" has no row at all
      **Scenario:** a later edit drops a kept section from `CLAUDE.md`, or re-adds a deleted agent file. Nothing in `tasks.md` would catch it. The first round's "byte-identical" measurement of the kept sections lived only in `findings/spec-test.md`, which the `closer` deletes, and it is no longer true. `68508d2d` rewrote 15 lines of "Module contract traps" (`@@ -250,15 +251,18 @@`, the `qmllint` paragraph). That rewrite is a correction, not a loss, but it means "kept" is now only a reading.
      **Measured:** each holds at `3b32d6f8`, by these commands, which could be the rows' Verify lines:
      2.1: `grep -n -F -e "Keeping this file true" -e "is the owner's" -e "How to work in this repo" -e "Scratch files go in" -e "Worktrees are not scratch" -e "How to shape a change" -e "Tests are part of the change" -e "Before anything else, make the failure visible" CLAUDE.md` prints four hits, at lines 532, 567, 587 and 598. All are past the block's begin marker at 423.
      2.2: `grep -n -E "^\| " CLAUDE.md` shows exactly the `docs/SCAFFOLD.md` and `docs/SOURCES.md` rows (lines 14 and 15) before the block.
      Kept: `grep -n -E "^## " CLAUDE.md` lists "What this is", "Module contract traps", "Scaffold…" and "Security posture". Their line spans are 73, 273, 22 and 16, against main's 73, 270, 22 and 14.
      3.1: `jq . .claude/settings.json` shows `worktree.baseRef` `"head"` and `enabledPlugins` `{"specflow@agent-spec-flow": true}`, with no `extraKnownMarketplaces`.
      3.2: `git check-ignore --no-index -v .claude/specflow/PROJECT.md` exits 1.
      4.1: `git ls-files .claude/agents docs/OPENSPEC-ARCHIVE.md docs/PROJECT-MANAGEMENT.md` prints nothing.
      Add these, or say on each row that it is a reading. Severity: low.
      **Fixed** in the commit "Harden the tasks.md checks". Each row now
      names a Verify command, tightened where the proposed one could pass a
      defect. 2.1 and "Kept" are one check: `grep -n -E "^## " CLAUDE.md`
      prints exactly six named headings before the begin marker, which
      catches both a dropped kept section and a re-added generic one (the
      proposed `-F` list matched four block headings and could not see a
      kept section going). It says the kept text is not compared, and why.
      2.2 counts exactly three `^\| ` lines before the marker. 3.1 is
      `jq -c .` against one exact string. 3.2 is `git check-ignore --no-index`
      over four paths, printing exactly `.claude/settings.local.json` (`-v`
      is left out because it also prints the negation that admits
      `settings.json`). 4.1 is the proposed `git ls-files`. 1.3 gained a
      check too, and 1.2 now says it is a reading and why. Run on the current
      tree, each as stated. Mutations, each undone with `git restore` (4.1's
      with `git rm -f`): a `## Tests are part of the change, not a follow-up`
      heading added above "Scaffold" appears in 2.1's list; a
      `docs/OPENSPEC-ARCHIVE.md` row added to the table makes 2.2's lines
      four; `!.claude/specflow/` deleted makes 3.2 print
      `.claude/specflow/PROJECT.md` too; `RUNNER.md` restored into the index
      from `origin/main` makes 4.1 print it. Not mutated: 3.1, because
      `.claude/settings.json` sets `worktree.baseRef` for any agent dispatched
      while it is changed, and an exact-string comparison needs no
      demonstration that it fails on a different string.

## What holds, and what is out of scope

- **Owner rows.** The issue's fourth Check is owner row 3.4, unticked, and it
  says why it is the only end-to-end check. The marketplace and install steps
  are owner row 3.3. Neither can run inside a dispatched agent. This session
  lists no `specflow:*` agent type, as expected before 3.3.
- **1.2** ("every dialectica-specific sentence has a home") is still a
  reading, and 1.1 says so. Not reopened.
- `openspec validate adopt-specflow-plugin --strict` passes, and reports
  `skip_specs` honoured.
- The plugin's required-heading list
  (`openspec/specs/project-overlay/spec.md:20` in the clone) is the same eight
  that 1.1's pattern names.
- The #174 regression and the skipped tester step are owner decisions, and are
  not reopened here.
- **Out of my scope, for the design and architecture reviewers.**
  `3b32d6f8` adds a `CLAUDE.md` section, "Shell rules for every session",
  that the issue's "What changes" does not list. The piece also touches
  `dialectica-ui/tests/tst_render_probe.qml` and
  `dialectica/rust-lib/Cargo.toml`, which are presumably comment repoints. No
  check covers either.
