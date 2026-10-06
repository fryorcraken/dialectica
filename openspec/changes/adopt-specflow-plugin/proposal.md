## Why

The spec-driven flow in `.claude/agents/` has been extracted, generic parts
only, into a Claude Code plugin, **specflow** (repo `agent-spec-flow`,
v0.1.0). Keeping both copies means two sources for one process, which drift.
Issue #195 adopts the plugin and removes the in-repo copies; it is the owner's
request for exactly the `.claude/` changes below.

## What Changes

- **Add `.claude/specflow/PROJECT.md`**, the project overlay: every
  dialectica-specific rule the extraction left out of the plugin — test
  layers, build, mutation tool, CI gates, what never to commit, hazards, and
  per-role rules. Reconciled against this tree rather than the issue's draft,
  which predates `5ec12953` and `fe093beb`.
- **`CLAUDE.md`**: replace its generic sections ("Keeping this file true",
  "`.claude/` is the owner's", "How to work in this repo, and what Bash costs",
  the scratch and worktree sections, "How to shape a change", "Tests are part
  of the change", "Before anything else, make the failure visible") with the
  block `/specflow:sync` writes. Keep "What this is", "Module contract traps",
  "Scaffold" and "Security posture". The "Where to look for what" table keeps
  `docs/SCAFFOLD.md` and `docs/SOURCES.md` only.
- **Delete** all nine `.claude/agents/*.md`, `docs/OPENSPEC-ARCHIVE.md` and
  `docs/PROJECT-MANAGEMENT.md`. Their generic content is the plugin's agents
  and its `flow`, `worktree-discipline`, `run`, `openspec-archive` and `pm`
  skills.
- **`.claude/settings.json`**: add `enabledPlugins` for
  `specflow@agent-spec-flow`. `worktree.baseRef: "head"` stays.
- **`.gitignore`**: re-admit `.claude/specflow/`.
- The marketplace entry is machine-local while the plugin is unpublished, so
  it is not in this change: `README.md` documents the two steps each
  contributor runs once, cloning `agent-spec-flow` and running `claude plugin
  marketplace add` on the clone.

**Not a no-op for the flow.** The issue says the change is behaviour-neutral
for the flow. That held at `6a7e02b9`; since then `5ec12953` (#174) changed
the in-repo flow — a re-review round, `NO SPEC:` routed before the `tester`,
the `closer` merging `main` instead of rebasing, and no `--admin` merges — and
plugin v0.1.0 carries none of it. `design.md` says what the overlay could keep
and what it cannot; the rest is tracked by an issue on the flow repo,
`agent-spec-flow`: <https://github.com/fryorcraken/agent-spec-flow/issues/1>.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. No requirement in `openspec/specs/` changes: this is the build process,
not the system. `.openspec.yaml` declares `skip_specs: true`.

## Impact

- Agent and session instructions only. No code, test, workflow or spec file
  changes behaviour.
- After merge, the next piece starts with `/specflow:run <issue#>`, and needs
  the plugin cloned and its marketplace added on the clone, as `README.md`
  says.
