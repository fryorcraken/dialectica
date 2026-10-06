# Re-review — correctness — `20643177..3b32d6f8`

Dimension: **correctness** only. Scope: the commits after the first review
round (`git diff 20643177..HEAD --stat -- . ":!openspec/changes/adopt-specflow-plugin/findings"`),
the answered boxes in `findings/*.md` read as claims, and PR #196's body
(`gh pr view 196`). Plugin checkout `agent-spec-flow` at `756e76a`, clean
(`git -C <clone> status --short` prints nothing).

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/tasks.md:72-75` — 2.3's worked example says it was "worked at this change's last edit to `CLAUDE.md`" and gives begin 23900, end line 33368, byte length 33390. That stopped being true at `3b32d6f8` ("List the six shell rules"), which added the shell-rules section above the block and is now the last edit to `CLAUDE.md`.
      **Scenario:** a re-reviewer or the `closer` checks the example before trusting the procedure. `wc -c CLAUDE.md` prints 34434, not 33390. `grep -b -n -F "<!-- specflow:" CLAUDE.md` prints 24944 and 34412, not 23900 and 33368. `cmp -i 23900:1796 -n 9490 …`, the command the example says "exits 0", now compares the wrong bytes. The procedure above the example is correct; only the example is stale, and it is stated as a fact about the current tree. This is the block's "Keeping documents true" rule: say which commit the numbers were read at, or drop them.
      **Measured:** at `3b32d6f8`, `CLAUDE.md` begin 24944, end line 34412, 34412 + 22 = 34434 = `wc -c`. `SKILL.md` begin 1796, end line 11264. Span 9490 on both sides. `cmp -i 24944:1796 -n 9490 CLAUDE.md <clone>/skills/sync/SKILL.md` exits 0 with no output, so the block itself is still byte-identical. `git show --stat 3b32d6f8` touches `CLAUDE.md` but not `tasks.md`. Severity: low.
      **Fixed** in the commit "Harden the tasks.md checks", by the box's
      second option: the numbers are dropped, and 2.3 says none is written
      down because any edit above the block moves them. The procedure now
      derives every value, with `grep -b -n -x -F` on the two full marker
      lines so `SKILL.md`'s prose mentions are not matched. Re-run at this
      commit: `CLAUDE.md` 24944 and 34412, `SKILL.md` 1796 and 11264, span
      9490 on both, and `cmp -i 24944:1796 -n 9490` exits 0 with no output,
      so the block is still byte-identical. `findings/rereview-spec-test.md`
      box 2 has the mutation runs.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:163-165` against `:179` — design.md contradicts itself on whether rule 11 depends on rule 1.
      **Scenario:** lines 163-165 say "Rules 6, 7, 8 and 10 describe v0.1.0's behaviour as written. Rule 12 does as well … Rule 11 closes a gap the plugin opens. **None of them depends on rules 1 to 5.**" "Them" includes rule 11, which the previous sentence names. Line 179 says "Rule 11's stop hands its commit to rule 1's round, which the overlay cannot add". That dependency is why rule 11 is the one item 6–12 the overlay could not carry. The issue comment design.md cites agrees with line 179: rule 11 is "the closer-side stop that makes [rule 1] happen". A reader asking whether rule 11 could ship upstream without rule 1 gets both answers from the same section. Fix: narrow "None of them" to rules 6–10 and 12, or say rule 11 depends on rule 1.
      **Measured:** read both lines at HEAD. The issue comment's rule 11 "Why it matters" says: "Rule 1 says such commits are reviewed. This rule is the closer-side stop that makes it happen." Severity: low.
      **Fixed** in the commit "Classify #174's rules 6 to 12 by their
      dependence on rules 1 to 5". The paragraph is now three bullets, in the
      issue comment's classification: 6, 7, 8, 10 and 12 stand alone; 9 is
      justified by rule 1's record but useful without it; 11 feeds rule 1's
      round, quoting the comment's "closer-side stop" line. The later
      paragraph's "Rule 11's stop hands its commit to rule 1's round" now
      agrees with it. `findings/rereview-design.md` box 1 is the same defect.

- [ ] **`dev-writer`** — PR #196 body, the `CLAUDE.md` bullet under "What changes" — the reason for "Shell rules for every session" is cited as "(owner's decision, `findings/architecture.md` box 1)". After the squash, no commit on `main` will contain that file.
      **Scenario:** the `closer` deletes `findings/` before the archive commit (the stage row "findings all ticked, `findings/` deleted"), and the squash writes the PR body into `main`'s history. The commit message then cites a file that never existed on `main`. A reader following it finds nothing, while the decision is recorded in a file that survives: `design.md`, "Six shell rules are listed in `CLAUDE.md`, outside the block", which quotes the owner. Point the body there.
      **Measured:** `gh pr view 196` shows "owner's decision, `findings/architecture.md` box 1". `git grep -n -F "findings/"` over `design.md`, `proposal.md`, `tasks.md`, `CLAUDE.md`, the overlay and `README.md` finds only rule 12's text and the closer's stage row. So the PR body is the only durable text that cites a findings file. Severity: low.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:349-351` — the sentence "`/specflow:init` merges a `permissions.allow` block … and the marketplace entry into the tracked `settings.json`" is false for the setup this repo's README prescribes.
      **Scenario:** README's step 2 registers a cloned directory (`claude plugin marketplace add ~/src/agent-spec-flow`), so the marketplace source is `directory`. For that source, `init` writes the entry to `.claude/settings.local.json`, not to `settings.json`, "since a local path is machine-local". Only a `github` source, or no registered entry at all, goes into the tracked file. The section's conclusion still holds, because the allowlist does land in tracked `settings.json` and that is the decision's point. But the marketplace half tells the owner that running `init` here would also commit a marketplace entry, which it would not. Fix: say "and, for a `github` source, the marketplace entry", or drop the marketplace half.
      **Measured:** plugin `skills/init/SKILL.md:65-73` at `756e76a` (unchanged since `77fce4d` apart from `be56964`'s 4 lines): `"source": "directory"` → "written to `.claude/settings.local.json` instead". Severity: low.
      **Fixed** in the commit "Record the plugin repository's real trust
      state, and the marketplace constraint". Read plugin
      `skills/init/SKILL.md` § 1 at `756e76a` (lines 65-73). The section now
      says `init` merges the allowlist into tracked `settings.json`, and that
      the marketplace entry goes there only for a `github` source or none
      registered; for the `directory` source README's route produces, it
      goes to `.claude/settings.local.json`, quoting "since a local path is
      machine-local". The section's conclusion is unchanged.

## Claims checked and true

The first-round "Fixed" claims were each checked against the tree. All hold.

- **Overlay** (`68508d2d`). The QML row: `tst_navigation.qml:112` and
  `tst_stoa_screens.qml:2423` declare `Component { … Main {} }`, and
  `check_qml_members.sh:26-27,36` covers "every file in src/qml". The static
  gates row: `ci.yml` runs names at 268/271 and reachable at 313/316, both in
  `lint` (59–717), and members at 1010/1013 in `qml` (718–1131). The Qt 6.5
  floor is `check_qml_members.sh:113,132`. The `tar tzf … | grep -c` lines are
  at `ci.yml:1689,1693`, inside `build` (1469–1707); the comment at 1685 says
  why. The `yq` probe is `require-jq-yq.sh:28`. The two switches are in
  `scaffold.toml:57-59`, and `docs/SCAFFOLD.md:91` heads "`[basecamp.env]`".
  `CLAUDE.md` names neither switch. The `## pm` vote-control line comes from
  `origin/main:docs/PROJECT-MANAGEMENT.md:63-65`. The "project's own commands"
  bullet comes from `origin/main:CLAUDE.md:83,218`. The 1.1 Verify prints `8`,
  `0` and eight `<n>-` text lines. Its negative example (body deleted →
  `58:`, `59-`, `60:## CI gates`) matches the line layout when lines 60–66 are
  removed. I reasoned this rather than measured it: an `Edit` in my own tree
  was refused by the auto-mode classifier.
- **`CLAUDE.md`** (`68508d2d`, `3b32d6f8`). `git grep -F "missing-property error"`
  over `CLAUDE.md` and the overlay prints nothing. `DTheme.qml` is tracked and
  `Theme.qml` is not. `check_qml_members.sh:179` runs `--missing-property
  warning -W 0`, and `:155-159` with `ci.yml:842` (`version: "6.8.3"`) give the
  pinned Qt that rejects `error`. The `qmllint` step passes
  `-I dialectica-ui/src/qml` (`ci.yml:1042`). The six shell rules each point
  at a section that holds the rule: `## Hazards` lines 106–128 (QML runner,
  `VAR=`, `yq`/`jq`, `/nix/store`), the SDK bullet in `## Test layers`, and
  `## Build`'s "no flake" bullet. `README.md:75` is the staging command, and
  `ci.yml:1281` runs it byte for byte. The block is byte-identical: `cmp -i
  24944:1796 -n 9490` exits 0, and `git -C <clone> diff --exit-code 77fce4d --
  skills/sync/SKILL.md` prints nothing.
- **Repointed comments** (`b240b4b6`). Every hunk in the `.rs`, `Cargo.toml`
  and `.qml` diff is a `//`, `///`, `//!` or `#` comment line. No code line
  changed. Each new target exists. "has an answer" and "over a branch that
  checks it" are the block's current wording. The overlay's Hazard heading is
  `PROJECT.md:118`. The `flow` skill has "Never write a scenario that cannot be
  tested" (`:259`) and "Say what a gate cannot see rather than reporting it as
  passed" (`:263`). `README.md` "Building" holds the staging command.
  `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_render_probe.qml`:
  11 passed, 0 failed. **`cargo test` was not run.** The README's SDK-staging
  `nix build` was refused by the auto-mode classifier, so the Rust suite's
  result for this range is unmeasured here. The diff being comment-only is
  established by reading every hunk.
- **`tasks.md` checks.** The 4.2 search prints nothing. The 11 names match
  `git diff --name-only --diff-filter=D origin/main...HEAD`. 4.3(a): `git diff
  --stat origin/main...HEAD -- openspec` lists only this change's folder and
  `relevance-votes/tasks.md`. 4.3(b): `openspec list` shows the two changes,
  and `grep -c "^## Stages"` prints `1` and `0`. 3.3 and 3.4 match README's
  three steps. The plugin and marketplace names match
  `.claude-plugin/plugin.json` and `marketplace.json`, and `.claude/settings.json`
  enables `specflow@agent-spec-flow`. `claude plugin marketplace add --help`
  gives `user` as the default scope. Whether `install --scope project` leaves
  `settings.json` unchanged I could not check: `claude plugin install --help`
  was refused by the classifier, and README already hedges it with "should".
- **`design.md` against its sources.** `git log 6a7e02b9..origin/main --
  .claude/agents/` lists only `5ec12953`. The `RUNNER.md`, `closer.md` and
  `spec-writer.md` line citations at `origin/main` land on the passages named.
  Two end one line early (`RUNNER.md:23-51`, whose bullet ends at 52, and
  `spec-writer.md:47-52`, whose sentence ends at 53); they still find the text,
  so I filed no box. Plugin `closer.md:92-98` and `run/SKILL.md:220-232` at
  `77fce4d` are the force-push/conflict and red-run passages. `77fce4d..756e76a`
  touches only `skills/init/SKILL.md`, `DECISIONS.md`, `LESSONS.md` and
  `README.md`. The plugin tracks no `hooks/`, `.mcp.json`, `bin/` or
  `settings.json`. All five plugin commits are `G`-signed, and the repo is
  public. The cache path is `DECISIONS.md:59`. The orphan proof is
  `worktree-discipline/SKILL.md:110-120`. `moderation-resolution`'s spec cites
  no issue. Protection gives `required_approving_review_count: 0`, strict, signed
  and `enforce_admins`. Issue comment `6010040773` exists and lists rules 6–12.
  `origin/main:CLAUDE.md:55` says "seven role files plus a CI gate".
- **No contradiction found** between `design.md` and the overlay, README or the
  PR body, apart from the findings above. The PR body contains no `/home/` path.
  `openspec validate adopt-specflow-plugin --strict` reports the change valid.

Not boxed, because it is outside the tree: the issue comment on
`agent-spec-flow#1` cites rule 5 as "dialectica's overlay, `PROJECT.md:185-192`".
At HEAD, `## closer` is at `PROJECT.md:193` and its rules are at 195–204. The
owner accepted the regression on condition that the issue tracks the rules
accurately, so the owner may want to correct that line number, or cite the
section name instead.
