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
