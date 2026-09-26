# spec-test review — 171-workflow-rules

Scope actually reviewed: `proposal.md`, `tasks.md`, `.openspec.yaml`, issues
#171, #170, #169, #133 (via `gh issue view`), and `git diff origin/main...HEAD
--stat` (stat only). No file under `.claude/agents/`, no `design.md`, no other
findings file and no diff body was read, per the brief's restriction.

- [x] **`dev-writer`** — `openspec/changes/171-workflow-rules/tasks.md:11-15`
      (the struck tester row) versus `proposal.md:132-139` and `tasks.md:110-114`
      (task 7.1, the `closer.md` Step 1 pathspec) — the tester row is struck on
      "no CI job, script or test reads `.claude/agents/` ... there is no
      executable behaviour to assert. ... The check that can see this change is
      the six reviewers reading the prose." That rationale is sound for the
      *wording* of the agent files, but the contract also specifies a literal,
      deterministic shell command that `closer.md` Step 1 is to run —
      `git ls-files -- "openspec/changes/<name>/tasks.md"
      "openspec/changes/archive/????-??-??-<name>/tasks.md"`, required to
      return "exactly one path back," stopping the closer on "more than one, or
      none." That claim is checkable against fixture directories without
      parsing a single word of any agent's prose — it is `git`/pathspec
      behaviour, the exact category the "no executable behaviour" rationale
      does not reach. Task 7.1 records only a one-off "Verify:" line, run once
      by the dev-writer during authoring, not a standing test.
      **Scenario:** a later edit to `closer.md` narrows or mistypes the glob
      (e.g. drops the `archive/` alternative, or gets the `????-??-??` digit
      count wrong) so an archived change now resolves to zero paths, or an
      unrelated change with a name that is a substring/superstring of another
      resolves to more than one. Per the rule the closer then "stops and
      reports," so the immediate failure is safe rather than silently wrong —
      but nothing catches the regression before a re-dispatched closer hits it
      on a real piece, and the six review lanes plus this one are prose
      readers, not executors of the command.
      **Measured:** ran the exact command shape from task 7.1 against this
      tree. `git ls-files -- "openspec/changes/171-workflow-rules/tasks.md"
      "openspec/changes/archive/????-??-??-171-workflow-rules/tasks.md"`
      returns only the live path; the same shape for `op-clock` returns only
      `openspec/changes/archive/2026-09-16-op-clock/tasks.md`; for `clock`
      (substring only) returns nothing — all three match what the proposal
      claims, so the pathspec is correct today. Separately,
      `git grep -n -F ".claude/agents" -- .github/` returned nothing, and
      listing `.github/workflows/` and `dialectica-ui/tests/` found no script
      that runs this command against fixtures — confirming no gate exists to
      catch a future regression in it. Severity: moderate — the current
      behaviour is correct and the failure mode is a stop rather than a wrong
      merge, but the "there is no executable behaviour to assert" framing used
      to strike the whole tester row is broader than the file actually is, and
      this one command is exactly the "pathspec's behaviour" example my brief
      names.

      **Deferred** — to a follow-up issue, listed in PR #174's follow-ups for
      the project manager to file, and recorded in `design.md`'s Risks ("The
      `closer`'s Step 1 pathspec has no standing test"). The finding is right
      that the command is executable and that the struck row's "no executable
      behaviour" is broader than the file. It is not added in this piece for
      two reasons. `proposal.md`'s
      Impact says this change adds no tests or CI, and the stage block's
      struck tester row is the `spec-writer`'s. And a test holding its own
      copy of the command would not fail when a role file's copy changed,
      which is the regression the scenario describes; the useful test
      extracts the command from `closer.md` and `RUNNER.md` and runs it
      against fixture names from the `lint` job, which is a piece of its own.

## Re-review `c222c37..9dc235c`

Scope actually reviewed: `git diff c222c37..9dc235c -- openspec/changes/171-workflow-rules/proposal.md` (full diff read); `tasks.md` and `.openspec.yaml` in full; `git diff c222c37..9dc235c --stat` (stat only); issues #171, #170, #169, #133 (already read in round 1, re-checked for coverage against the new text). No file under `.claude/agents/`, no `design.md`, and no other findings file was read.

- [x] **`spec-writer`** — round 1's pathspec-test finding is deferred, but the
      deferral is invisible in the contract
      **Scenario:** `proposal.md`'s "Out of scope" section is where this
      contract records every other deferred item, each with its own reasoning
      and a note for the project manager to file ("One file per stage row",
      "A clean first-round review with no box (#182)", "Why #165 was
      `BLOCKED`", etc.). Round 1's finding above — that the `closer`'s Step 1
      pathspec is a deterministic, checkable command with no standing test —
      was accepted: its own "Deferred" note says it is tracked "to a
      follow-up issue... and recorded in `design.md`'s Risks". But
      `proposal.md`, the document this `skip_specs` change designates as the
      contract, never mentions it: a reader of `proposal.md` alone has no way
      to learn this gap was found and consciously deferred rather than never
      noticed. The only trace anywhere outside `design.md` is one unlabelled
      clause in `tasks.md`'s Implementation notes, which names no follow-up
      and points only at `design.md`.
      **Measured:** `git grep -c -F "pathspec" openspec/changes/171-workflow-rules/proposal.md`
      returns no output (zero matches, over the whole file, not just the
      diff). `git grep -n -F "clean re-review trace"` matches once, at
      `tasks.md:174`, and zero times in `proposal.md`. Severity: moderate —
      matches round 1's own rating; the gap is real and was found, just not
      surfaced where the contract's own convention says a deferred gap
      belongs.

      **Fixed** (`spec-writer`, this commit). `proposal.md`'s "Out of scope"
      gains "A standing test for the git commands the role files name", which
      carries the pathspec together with the commands the next box names, says
      what was measured and where, why no test is added in this piece, and that
      it is a follow-up for the project manager in place of the pathspec-only
      one PR #174 lists. The struck tester row in `tasks.md` no longer says
      there is no executable behaviour: it says the role files' git commands
      are executable, were measured once, and points to that entry. Left for
      the `dev-writer`: `design.md`'s Risks entry "The `closer`'s Step 1
      pathspec has no standing test" widened to match, and PR #174's
      follow-up list updated.

- [x] **`spec-writer`** — new deterministic-command claims added in this range
      get no equivalent deferral
      **Scenario:** this range adds several brand-new prose claims about the
      exact output of a git command the `closer`/runner runs, each entirely
      new in `c222c37..9dc235c` (none of the four strings below appear in
      `proposal.md` before `c222c37`): the archive-commit gate
      `git diff --name-only HEAD^ HEAD -- openspec/specs/`, `git merge
      --ff-only`'s keep-or-refuse behaviour, `git show --remerge-diff <sha>`
      for a conflict resolution, and `git log --oneline --left-right
      --cherry-mark HEAD...origin/piece/<name>`'s output shape. These are the
      same class of claim round 1 flagged for the Step 1 pathspec — a
      deterministic shell command's behaviour, asserted in prose, with
      nothing that can see `.claude/agents/` at all (`tasks.md`: "no test can
      see any of these tasks"). `tasks.md`'s own verify lines (8.2, 8.4, 9.1,
      9.3, 9.7, 9.8) show each was checked once against a scratch repository
      at authoring time — the same one-off verification the pathspec had
      before round 1's finding was written. Unlike the pathspec and the
      verdict-box grep behaviour (which at least got the tasks.md mention
      above), nothing in `proposal.md` or `tasks.md` flags these four as a
      known, accepted gap; they read as settled rather than as
      measured-once-and-untested, which is an inconsistency within the
      contract's own treatment of the same risk.
      **Measured:** counted occurrences of each string in `proposal.md` at
      `c222c37` vs `9dc235c` with `git grep -c -F "<string>" <rev> --
      openspec/changes/171-workflow-rules/proposal.md` (no output means zero):
      `"openspec/specs/"` 0→8, `"ff-only"` 0→2, `"remerge-diff"` 0→1,
      `"cherry-mark"` 0→1. Severity: moderate — same reasoning as round 1's
      pathspec finding; the commands were measured once and are presumed
      correct, but no gate would catch a future edit that got one wrong, and
      that gap is currently untracked for these four while it is tracked for
      the pathspec.

      **Fixed** (`spec-writer`, this commit), in the same "Out of scope"
      entry as the box above. It names all four, plus step 1's
      `git diff --name-only -G "NO SPEC:"` and the runner's new pre-tick
      `git grep -l -F "<range>"`, and it says which fail open and which fail
      closed. A mistyped `openspec/specs/` path in the archive check lists
      nothing and lets the `closer` carry on, so an unreviewed spec change
      would merge. That makes it the first command a test should cover. A
      mistyped pathspec or pre-tick grep stops the agent instead. The
      `--ff-only`, `--remerge-diff` and `--cherry-mark` claims describe git's
      own behaviour, and the entry says a test of them would mostly re-test
      git. No test is added in this piece, for the reasons the entry gives.

## Areas checked clean

- **Issue coverage.** Every "Done when" / proposed-change bullet in #171,
  #170, #169 and #133 is addressed in `proposal.md`, including #171's
  "leaves the reviewer count and model choice to the runner's judgement, with
  the decision recorded" and #170's exact `gh pr view` field list — verified
  live: `gh pr view 174 --repo fryorcraken/dialectica --json
  mergeStateStatus,mergeable,statusCheckRollup,reviewDecision` returned valid
  data for all four fields, so none is a typo'd field name.
- **Scope.** `git diff origin/main...HEAD --stat` shows exactly the five
  `.claude/agents/*` files the proposal's Impact section names
  (`README.md`, `RUNNER.md`, `closer.md`, `spec-writer.md`, `tester.md`) plus
  the `openspec/changes/171-workflow-rules/` paperwork — no `settings.json`,
  no hooks, no `CLAUDE.md`, matching the proposal's own "Out of scope"
  section and the piece's stated authorisation boundary.
- **Owner authorisations.** The two edits the dispatch brief said were
  owner-authorised beyond the four issues' text — `tester.md`'s handling of a
  decided marker, and `closer.md` Step 3 not re-archiving on a re-dispatch —
  are each explicitly tagged "(owner-authorised)" in `proposal.md` at the
  point they're introduced, rather than presented as reads on the issues.
- **#133's deviation from the issue's literal fix.** The issue's suggested
  `git checkout 51ab7f8^ -- .claude/agents/README.md` is not run; the
  proposal explains why (the file has changed since, e.g. PR #106) and
  instead quotes the exact paragraph text from the issue. The quoted text in
  `proposal.md:160-162` matches the issue's quoted paragraph verbatim.
- **Struck spec row.** Matches `.openspec.yaml`'s `skip_specs: true` +
  `schema: spec-driven`, and `openspec/config.yaml` confirms `spec-driven` is
  the project's built-in schema name, so the declaration is well-formed.
- **`#169`'s declined optional suggestion** (keeping the spec-writer's tree
  for a callback) is addressed, not silently dropped — the proposal states it
  is declined and defers the reasoning to `design.md`, which is outside this
  review's scope.
