## Context

Issue #195 carries a draft of the overlay, drafted from `6a7e02b9`, the commit
plugin v0.1.0 was extracted from. This change was made on `645b4e73`. Between
the two, `5ec12953` (#174) rewrote `RUNNER.md`, `closer.md` and the stage
block, `fe093beb` added the `yq`/`jq` rule to `CLAUDE.md`, `f32c23a8` added the
SDK-staging rule, and `8368b2f1` added the end-to-end UI workflow. Every line of
the draft was checked against the tree it replaces, and every
dialectica-specific sentence in a file this change deletes was given a home or
listed below as dropped.

The plugin's `flow` skill fixes what the overlay may do: **"The overlay adds
facts and role rules. It never overrides a specflow rule, except by striking a
stage under `## Extra stages`."** That sentence decides most of what follows.

## Goals / Non-Goals

**Goals:** the overlay states this tree's facts, not `6a7e02b9`'s; no
dialectica-specific rule is lost silently; `CLAUDE.md`'s block is exactly what
`/specflow:sync` would write, so a later sync replaces it in place.

**Non-Goals:** changing the plugin, or publishing the marketplace entry.
Lessons about specflow go to the owner first (the block's own rule), and the
owner files them on `agent-spec-flow`.

## Decisions

### The overlay is reconciled against this tree, not copied from the issue

Corrections to the draft, each checked against the file named:

- **`nix build ./dialectica#lgx`, not `.#lgx`** (Test layers, Hazards). The
  repository root has no flake: `git ls-files flake.nix` returns nothing, and
  the removed `CLAUDE.md` text said `.#lgx` "fails there". The draft carried
  the wrong form into both places; the overlay now says it once in `## Build`
  as well, because a wrong build command fails at the first use.
- **Two test layers added.** The draft listed three. `8368b2f1` added the
  end-to-end UI layer (`.github/workflows/ui-tests.yml`, sitometres driving a
  real Basecamp), the only layer inside the host. The static QML gates
  (`check_qml_names.py`, `check_qml_members.sh`, `check_qml_reachable.py`) were
  already in `ci.yml`, and are where a host type-name collision is caught,
  which no other layer sees. The e2e row says it has no one-command local
  form rather than inventing one.
- **CI gates: two workflows, and required checks are a subset.** The draft
  named `ci.yml` alone. `ui-tests.yml` also runs on `pull_request`.
  `gh api …/branches/main/protection` lists four required contexts, and the
  UI jobs are not among them, so the overlay tells the `closer` to read every
  job's result rather than the merge state. It names the command, not the
  list, because the list is a command's answer.
- **The `qmllint` blind spot restated precisely.** The draft said
  "`qmllint --missing-property error` is run with our own `Theme.qml` on the
  import path". The `qmllint` step runs `--unqualified disable -I
  dialectica-ui/src/qml`; `check_qml_members.sh` is what escalates, with
  `--missing-property warning -W 0`, because Qt 6.8.3 rejects the level
  `error` (`ci.yml`'s Qt pin comment). The singleton is `DTheme`, not `Theme`.
- **The two layout-derived gates are named**: the Rust `Tests` step's
  `#[test]` count and `every QML spec file actually ran`. "Two gates" without
  names left the reader to find them.
- **Never commit gains `basecamp/`, `ui-results/` and `dist/`**, which
  `.gitignore` lists as UI test and release outputs.
- **Hazards gain the `yq`/`jq` rule** (`fe093beb`, after the draft) and the
  SDK-staging rule moves into `## Test layers` (`f32c23a8`). The staging
  command stays in `README.md` only, as the removed text required, because
  `ci.yml` runs the README's line byte for byte.
- **Mutation tool** gains that `cargo mutants` cannot judge the adapter in
  `dialectica/rust-lib/src/lib.rs`, which `ci.yml`'s `lint` job records.
- **The current-version pointer**, `dialectica/metadata.json`, is in `## Build`:
  the block's own table says "the manifest that holds it" and the overlay names
  the manifest.

### #174's flow rules: what the overlay keeps, and what it cannot

Plugin v0.1.0 was extracted before `5ec12953`, and searching its `skills/` and
`agents/` for `re-review`, `--admin` and `fast-forward` returns nothing. So
adopting it removes from the active flow:

1. **The re-review round** — every commit landing after the review round is
   reviewed before the `closer`, recorded on a runner-owned
   `re-review: every commit after the review round — runner` stage row.
2. **`NO SPEC:` markers routed to a fresh `spec-writer` before the `tester`**,
   and the `tester` closing markers a brief names as decided.
3. **The `closer` merges `main` rather than rebasing, never force-pushes, and
   never resolves a conflict** — it aborts and reports. Plugin v0.1.0's
   `closer` rebases, pushes `--force-with-lease`, and resolves conflicts in
   files the piece touches.
4. **The runner fast-forwards** to a `closer`, a conflict resolver and every
   `dev-writer` pass instead of cherry-picking, so its `piece/<name>` never
   diverges from the remote piece ref.
5. **No `gh pr merge --admin`, and no write to branch protection or rulesets**,
   even under merge-on-green; a PR `BLOCKED` with every required check green is
   a stop.

**Item 5 is in the overlay's `## closer`.** It adds a prohibition the plugin
does not make and contradicts none it does, so it is a role rule the overlay may
carry. It is the one item with a merged incident behind it: #170 records a
`closer` merging PR #165 with `--admin`.

**Items 1 to 4 are not in the overlay, because each would override a specflow
rule** — the stage-block roster, the runner's dispatch order, the `closer`'s
rebase, and the cherry-pick route. The re-review row also cannot enter through
`## Extra stages`: a row there names a project agent the runner dispatches
(`run` skill, "Dispatching"), and this row names the runner itself.

What was considered instead:

- **Write them into the overlay anyway.** Rejected: the plugin's agents are
  told the overlay never overrides them, so an overlay rule that contradicts a
  plugin rule leaves two instructions and no precedence.
- **Keep the in-repo `RUNNER.md` and `closer.md` beside the plugin.** Rejected:
  that is the two-sources problem this change exists to end, and the plugin's
  `closer` would still be the one dispatched as `specflow:closer`.
- **Carry them upstream, in the plugin.** Chosen. The loss is tracked by an
  issue on the flow repo, `agent-spec-flow`, which is where a rule about the
  runner, the stage roster or the `closer` belongs. The reasoning for all five
  items is not lost: it is in
  `openspec/changes/archive/2026-09-27-171-workflow-rules/design.md` and
  `proposal.md`, archived with the change that made them. What is lost is the
  rule's place in the flow that runs.

This makes the issue's "No behaviour change" false for the flow as of this
tree, and the stage block that `specflow:spec-writer` writes has twelve rows
where this repo's has thirteen. Pieces already archived are unaffected; no
change is in flight with a stage block (`relevance-votes`, the one other live
change, has a `tasks.md` with no `## Stages` block).

### `docs/PROJECT-MANAGEMENT.md` is deleted, not reduced

Everything in it that is dialectica's is in the overlay's `## pm`: the
owner-confirmed `0.0.x` dead-button policy, reading each milestone's own
description, and GitHub as the system of record while Radicle has no issue
tracking. The rest is the plugin's `pm` skill, near verbatim, or history:
`docs/PLAN.md`'s retirement and #105, and the closing-keyword gap the file says
is already closed. A reduced file would hold three bullets the overlay already
holds, and two copies drift.

### `docs/OPENSPEC-ARCHIVE.md` is deleted; its facts move

- The `stoa-genesis` reference trap goes to `## Hazards`, as the issue said,
  with the two delta shapes that are this repo's (`stoa-metadata`,
  `spec-backfill`), which the plugin's skill only describes generically.
- The existing duplicated rule — `identity` and `op-format` both asserting
  authenticity-is-not-authority — and `op-ordering` as the pattern to follow go
  to a new `## spec-writer` section. It is a standing fact about this repo's
  specs, and the plugin's `spec-writer` has the generic "one rule, one
  capability" rule but cannot know where it is already broken.

### The `CLAUDE.md` block is the sync skill's text, byte for byte

It is copied from `skills/sync/SKILL.md` in the plugin checkout and appended
at the end of the file, which is where `/specflow:sync` puts a block when
neither marker is present. Checked with `cmp` from each file's begin marker to
the end of the block: no differing byte. A block written by hand would be
replaced by the next sync anyway, and differences would show only then.

### `.gitignore` keeps `!.claude/agents/`

The directory is empty after this change. The line stays because the plugin's
own `/specflow:init` writes it, for project agents an overlay row may name; an
agent added there with the line gone would be silently missing from every
worktree, the failure shape the `settings.json` comment above it records.

### The marketplace entry is not in this change

While the plugin is unpublished its marketplace is a local checkout, so the
entry is machine-local and `settings.json` gets only `enabledPlugins`. Each
contributor registers the marketplace with `claude plugin marketplace add
<checkout path>`, the command `README.md`'s "Working with the agent flow"
documents; its default scope is `user` (`claude plugin marketplace add
--help`), so it reaches every worktree rather than one checkout. The owner
runs it; a dispatched agent does not change the owner's Claude Code
configuration. The README note goes when the plugin is published and the entry
moves into `settings.json` as a pinned `github` source.

### Dropped, deliberately

Dialectica-specific sentences in deleted text with no home in the overlay:

- **Stories behind kept rules**: the nine-file rewrite behind "`.claude/` is the
  owner's"; the mistyped absolute path behind "relative paths inside your
  worktree"; the ~27 checkouts under `tmp/`; the agent that lost its worktree
  on #91; six findings recovered from a reflog after a branch rename; the "wrong for two years"
  comment. The rules are in the block or the plugin; the plugin's
  `DECISIONS.md` (11) keeps stories out of agent-facing text.
- **`blockReadsOutsideWorkingDirectories` is on.** A fact about this machine's
  settings, which `settings.local.json` answers.
- **"Address this repo's agents unqualified."** Inverted by the plugin, whose
  agents are addressed `specflow:<role>`.
- **`keystore` and `posting-capability`'s AEAD contradiction**, the
  `OPENSPEC-ARCHIVE.md` example of a cross-capability contradiction. Resolved
  history; the plugin keeps the rule.
- **`git log --oneline origin/<orphan> --not origin/piece/<name>`** for proving
  an orphaned PR is redundant (`.claude/agents/README.md`). Generic, and not in
  the plugin: a candidate for upstream, not for the overlay.

## Risks / Trade-offs

- [The flow regresses on #174's five rules until the plugin carries them] →
  item 5 is in the overlay; items 1 to 4 are tracked by an issue on the flow
  repo, `agent-spec-flow`, and are absent from the active flow until the plugin
  carries them.
- [The overlay draws on `ci.yml` and branch protection, both of which move] →
  it names commands (`gh api …/protection`) rather than lists where it can,
  and names the step whose output a claim comes from.
- [A session cannot see the plugin until the marketplace entry exists] →
  `README.md` carries the command that adds it, and the issue's fourth check (a fresh
  session lists the `specflow:*` agents) is the owner's to run after merge.
