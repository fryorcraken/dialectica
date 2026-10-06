# Design review — adopt-specflow-plugin (#195)

Read: the diff against `origin/main`, `design.md`, `proposal.md`, issue #195
(fresh, `gh api`), PR #196's body, the deleted files at `origin/main`, the flow
files' history since `6a7e02b9`
(`git log 6a7e02b9..origin/main -- .claude/agents/ CLAUDE.md` lists only
`5ec12953`, `fe093beb`, `f32c23a8`), and the plugin checkout at `be56964`.

## What holds

Every decision the issue states is honoured: the overlay is added with all
eight required headings and no `TODO` line; the eight generic `CLAUDE.md`
sections are gone and the four kept sections are untouched (no diff hunk falls
between the old lines 318 and 683); the "Where to look for what" table keeps
only `docs/SCAFFOLD.md` and `docs/SOURCES.md`; all nine agent files and both
docs are deleted; `settings.json` gains only `enabledPlugins`; `.gitignore`
re-admits `.claude/specflow/`; the marketplace entry is not tracked and is in
the PR body; nothing else under `.claude/` changed. The two departures — the
"No behaviour change" premise and deleting rather than reducing
`PROJECT-MANAGEMENT.md` — are argued, with alternatives, in `design.md`.

Claims in `design.md` checked against the tree and found true: the root has
no flake; `ci.yml` runs `lint`, `qml`, `ui-specs`, `rust`, `build`, `release`;
`ui-tests.yml` triggers on `pull_request` and `push` to `main` with
`cancel-in-progress`; required contexts are four (`Lint`, `QML lint`,
`Rust core tests`, `Build LGX`) with no UI job; Qt 6.8.3 rejects the level
`error` (`ci.yml:760-766`, `999-1002`); the `#[test]` count and
`every QML spec file actually ran` steps exist; `basecamp/`, `ui-results/`,
`dist/` are in `.gitignore`; the keystore-mutants note is at `ci.yml:509-518`;
the UI scripts refuse the Go `yq` by name (`require-jq-yq.sh:32`); the plugin's
`skills/` and `agents/` contain no `re-review`, `--admin` or `fast-forward`;
items 1–4 do contradict plugin rules (`tester.md:28,33` keeps every marker,
the `spec-writer` roster has twelve rows, the `closer` rebases with
`--force-with-lease`, the flow cherry-picks); `/specflow:init` does write
`!.claude/agents/` (`init/SKILL.md:87`); the block matches `sync/SKILL.md` at a
constant 360-line offset; #170 records the `--admin` merge of #165;
`relevance-votes` has no `## Stages` block and `ui-remaining-screens` is
archived (`2026-09-28`); the #174 reasoning is archived where `design.md` says.

## Findings

- [ ] **`dev-writer`** — `design.md:71-122` — the list of #174 rules that adopting the plugin removes is incomplete: it names five, and #174 (`5ec12953`, the only flow commit after `6a7e02b9` besides the `yq` and SDK-staging ones) added at least four more runner rules that plugin v0.1.0 does not carry and the overlay does not keep. `proposal.md:33-38` and the PR body ("five flow rules") repeat the count, and the PR asks the owner to decide on the basis of it.
      **Scenario:** the first `/specflow:run` review round. Six reviewers fork from one HEAD and tick adjacent stage rows; the runner cherry-picks them as the plugin's `run` skill says, and the second pick stops with `CONFLICT (content): Merge conflict in tasks.md` — measured in `openspec/changes/archive/2026-09-27-171-workflow-rules/design.md:1816-1824`. The plugin's `run`, `flow` and `worktree-discipline` skills say nothing about a conflicting cherry-pick (a search for `conflict` across the plugin's skills and agents hits only `closer.md:96-97`), and the plugin's "You do not write the work" leaves the runner no route. On `origin/main` the rules that answered this were `RUNNER.md:277-328` (abort, continue the agent to rebase its own branch; expect it every review round; the dirty-tree patch procedure for a mutating reviewer). Also gone, uncounted: "What a runner commits" (`RUNNER.md:23-51` — every agent ticks its own row, an owner-requested edit is a dispatch, a refused edit is reported), "Nothing lands on the piece while the review round is out" (`RUNNER.md:519-531`), and rebuilding state after the `closer` has archived (`RUNNER.md:123-130`). By `design.md:93-96`'s own test for item 5 — adds a rule the plugin does not make, contradicts none it does — at least the abort-and-continue rule and "What a runner commits" could live in the overlay. Either carry them, or list each with why it was ruled out, and correct the count in `proposal.md` and the PR body.
      **Measured:** `git diff 6a7e02b9 origin/main -- .claude/agents/RUNNER.md` shows each passage cited above as added lines; `grep -rn -i "conflict"` over the plugin's `skills/` and `agents/` returns `closer.md:96` and `:97` only.

- [ ] **`dev-writer`** — `design.md:168-187` — "Dropped, deliberately" misses dialectica-specific text that is now silently gone. None of it needs restoring, but each needs a line saying it was dropped and why, or the next reader cannot tell a decision from a loss:
      (1) the live instance of the issue-number-citation rule, `moderation-resolution`'s "The deciding moderation is named" (`.claude/agents/README.md:43-45` at `origin/main`). It no longer exists — `openspec/specs/moderation-resolution/spec.md:242-246` cites no issue — so it is resolved history, the same reason the `keystore`/`posting-capability` entry gives;
      (2) the owner-confirmed scope of the `0.0.x` policy, "a feature may ship ahead of what consumes it", and its example: 0.0.1's vote control publishes real votes that no ranking consumes yet (`docs/PROJECT-MANAGEMENT.md`, "Standing policy"). `PROJECT.md:196-198` keeps the dead-button half and drops this half;
      (3) the old costs table's naming of `nix build …` and `lgs …` as free. The block's row now says "the project's own" (`CLAUDE.md:465`), and neither the overlay nor anything else names which commands those are. The reason in the `blockReadsOutsideWorkingDirectories` entry applies, but the entry has to say so;
      (4) the stories list at `design.md:172-177` reads as an enumeration but leaves out others that went: the ~690–705-deletion stale PRs and the 6,871-deletion false alarm (`closer.md`), the shepherd that watched a cancelled Build LGX run, "forty findings" read as done, "openspec is not installed" reaching five agents, and two pieces merging unreviewed code in one day (`RUNNER.md:557-560`). Make it a category ("every incident narrative, for example …") rather than a list.
      **Scenario:** a later reader looking for the vote-control precedent, or for which local commands cost no click, finds nothing in the overlay and no "Dropped" line, so cannot tell whether the drop was deliberate.

- [ ] **`dev-writer`** — `design.md:160-166` — unrecorded: `.claude/settings.json` departs from what `/specflow:init` writes. Init merges a `permissions.allow` block (`git`, `gh pr/run/issue/api`, `gh repo view`, `openspec`, `grep`, `pwd`) into the **tracked** `settings.json` (`init/SKILL.md:35-64`, plugin `DECISIONS.md` #15). This repo keeps its allowlist in the untracked `settings.local.json` (`.gitignore:60-63`), and this change follows the repo, not the plugin. That is a reasonable choice, but nothing records it.
      **Scenario:** the owner later runs `/specflow:init`, for example to re-check headings or `gh auth`. It merges an allowlist into the tracked `settings.json`, so every clone gets it, against `.gitignore`'s stated rule, and nothing fails. Record that the allowlist stays machine-local here, and whether `/specflow:init` may ever run in this repo or only `/specflow:sync`.

- [ ] **`dev-writer`** — `design.md:28-69` — "Corrections to the draft" leaves out two overlay lines that differ from the issue's draft. (1) `PROJECT.md:50` gives `lgs basecamp launch <profile>`, where the draft has no argument. The correction is right (`README.md:60` runs `lgs basecamp launch alice`) but is not listed. (2) `PROJECT.md:24-25`, "The `-p` flags are load-bearing", is not in the draft or in any deleted file. It restates `README.md:84-86`, which makes a second copy of README text. `design.md:62-64` keeps the SDK-staging command in `README.md` only, so the two choices need reconciling or the duplicate needs a reason.
      **Scenario:** the section reads as the complete set of departures ("Corrections to the draft, each checked against the file named"), so a reviewer comparing the overlay with the draft finds two changes nobody accounts for.
