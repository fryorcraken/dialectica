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
  `CLAUDE.md`'s "Module contract traps" carried the same wrong wording and is
  corrected to match, so the two always-read files give one account of the
  gate.
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
- **`lgs basecamp launch <profile>`**, where the draft had no argument:
  `README.md`, "Building", runs `lgs basecamp launch alice`.
- **The `-p` explanation stays in `README.md`.** The overlay's Rust row keeps
  the flags in its command, and its SDK bullet points at "Building" for why
  they matter, rather than carrying a second copy of README text, the same
  choice as the staging command.
- **`Main.qml` is driven by specs.** The draft's "`Main.qml` is instantiated
  by no spec" was history: `tst_navigation.qml` and `tst_stoa_screens.qml`
  construct it, and `check_qml_members.sh`'s header says so. The QML row's
  blind spot is now the narrower true one, a component no spec constructs.
- **The `grep -c` form lives in the `build` job**, in "Stage artifacts". The
  deleted `.claude/agents/README.md` said "release job", and the overlay
  inherited the error; `release` contains no `grep`.
- **The `[basecamp.env]` switches are already set** in the tracked
  `scaffold.toml`, so the hazard says to check they survived an `lgs` verb
  rather than to set them, and points at `docs/SCAFFOLD.md` for what each
  prevents. `CLAUDE.md` names neither switch.
- **`run-qml-tests.sh` with one spec is the unprompted shape.** The removed
  `CLAUDE.md` text said so, and the block's table prices `sh <relative-path>`
  as a click in general, so the overlay names the script, `nix build …` and
  `lgs …` as the "project's own" commands the block's free row refers to.
- **The `yq` guard probes, it does not match a name**:
  `require-jq-yq.sh` feeds the `yq` on `PATH` a YAML document and refuses one
  that does not return JSON.

### #174's flow rules: what the overlay keeps, and what it cannot

Plugin v0.1.0 was extracted before `5ec12953`, which is the only commit to
`.claude/agents/` since `6a7e02b9` (`git log 6a7e02b9..origin/main --
.claude/agents/`). Searching the plugin's `skills/` and `agents/` for
`re-review`, `--admin` and `fast-forward` returns nothing. So adopting it
removes from the active flow the five headline rules:

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

#174 also added these rules. Each is gone from the active flow too. The
locations are in `.claude/agents/` at `origin/main`:

6. **A cherry-pick that stops on a conflict is aborted, and the agent is
   continued so it rebases its own branch.** The review round meets this
   every time, because six reviewers fork from one HEAD and tick adjacent
   rows (`RUNNER.md:277-291`, `spec-writer.md:47-52`). The plugin's `run`
   skill says nothing about a conflicting pick, and its "You do not write the
   work" leaves the runner no route.
7. **A reviewer holding uncommitted mutations rebases through a patch**:
   `git diff --binary --output=…`, then `git restore`, then rebase, then
   `git apply` (`RUNNER.md:293-328`). `git rebase` refuses a dirty tree, and a
   stash lands on the stack every worktree shares.
8. **What a runner commits** (`RUNNER.md:23-51`). The runner ticks no other
   agent's row. An edit the owner asks for is a dispatch. An edit an agent
   refused is reported, not made. Output an agent could not commit is
   committed by continuing that agent, or redone by a fresh one, never copied
   by the runner.
9. **Nothing lands on the piece while the review round is out**
   (`RUNNER.md:519-531`). The plugin freezes the piece only while the `tester`
   runs.
10. **Rebuilding the state after the `closer` has archived.** The change
    folder is found under `archive/` with `git ls-files`, and a re-dispatched
    `closer` does not archive again (`RUNNER.md:123-130`,
    `closer.md:75-95`, `closer.md:229-238`). The plugin's `run` and `closer`
    grep only `openspec/changes/<name>/`, although its `closer` archives
    before CI, so a red run comes back already archived.
11. **The `closer` checks the archive commit for a change to
    `openspec/specs/` and stops before CI if it finds one**, so the promoted
    contract is reviewed (`closer.md:281-293`, `311-322`).
12. **The `closer` deletes `findings/` at the start of Step 3, not in Step 1**
    (`closer.md:130-134`). An uncommitted deletion makes `git merge` refuse,
    and `git rebase` too.

Rules 1 to 4 also carry detail the list above compresses: which commits need
re-review and which are only tracking, the record forms and checks, the
red-run fix going through re-review, a fresh `spec-writer` for markers, no
markers put to the owner, a refused push as a stop with no fetch-and-merge,
and fast-forwarding to a `closer` that returns without merging. The
`agent-spec-flow` issue comment cited below lists each, with its location.

Rules 6, 7, 8 and 10 describe v0.1.0's behaviour as written. Rule 12 does
as well, because the plugin's own rebase refuses an uncommitted deletion. Rule
11 closes a gap the plugin opens. None of them depends on rules 1 to 5.

**Item 5 is in the overlay's `## closer`.** It adds a prohibition the plugin
does not make and contradicts none it does, so it is a role rule the overlay may
carry. It is the one item with a merged incident behind it: #170 records a
`closer` merging PR #165 with `--admin`.

**Items 1 to 4 are not in the overlay, because each would override a specflow
rule** — the stage-block roster, the runner's dispatch order, the `closer`'s
rebase, and the cherry-pick route. The re-review row also cannot enter through
`## Extra stages`: a row there names a project agent the runner dispatches
(`run` skill, "Dispatching"), and this row names the runner itself.

**Items 6 to 12 are not in the overlay either, mostly for a different
reason.** Rule 11's stop hands its commit to rule 1's round, which the overlay
cannot add. Rules 6 to 10 and 12 add to the plugin without contradicting it,
so the overlay could carry them; the `run` skill reads the overlay in
preflight, and the `closer` reads its `## closer` section. The owner chose to
carry them upstream instead.
Every rule then has one source, and the gap until the plugin carries them is
accepted (see "The owner accepted the regression" below).

What was considered instead:

- **Write them into the overlay anyway.** Rejected: the plugin's agents are
  told the overlay never overrides them, so an overlay rule that contradicts a
  plugin rule leaves two instructions and no precedence.
- **Keep the in-repo `RUNNER.md` and `closer.md` beside the plugin.** Rejected:
  that is the two-sources problem this change exists to end, and the plugin's
  `closer` would still be the one dispatched as `specflow:closer`.
- **Carry them upstream, in the plugin.** Chosen. The loss is tracked by an
  issue on the flow repo, `agent-spec-flow`:
  <https://github.com/fryorcraken/agent-spec-flow/issues/1>. The issue body
  lists rules 1 to 5. Its first comment
  (<https://github.com/fryorcraken/agent-spec-flow/issues/1#issuecomment-6010040773>)
  adds rules 6 to 12 and the detail under rules 1 to 4, each with its location
  and what the plugin does instead. That repo is where a rule about the runner,
  the stage roster or the `closer` belongs. The reasoning for every item is not
  lost: it is in
  `openspec/changes/archive/2026-09-27-171-workflow-rules/design.md` and
  `proposal.md`, archived with the change that made them. What is lost is the
  rule's place in the flow that runs.

This makes the issue's "No behaviour change" false for the flow as of this
tree, and the stage block that `specflow:spec-writer` writes has twelve rows
where this repo's has thirteen. Pieces already archived are unaffected; no
change is in flight with a stage block (`relevance-votes`, the one other live
change, has a `tasks.md` with no `## Stages` block).

### The owner accepted the regression, on condition it is tracked accurately

The owner **accepted** the loss of rules 1 to 4 and 6 to 12 from the active
flow, on one condition: they are tracked accurately in
<https://github.com/fryorcraken/agent-spec-flow/issues/1>. That issue is the
record of what the plugin owes, and this section is the record that the gap was
a decision, not an oversight. `/specflow:run` is not held back, and the overlay
does not carry the rules.

**The accepted risk is that unreviewed content can reach `main`.** Under plugin
v0.1.0 there are three routes:

1. The `closer` resolves a rebase conflict itself and pushes over the reviewed
   commits with `--force-with-lease` (plugin `agents/closer.md:92-98`). Rule 3
   closed this.
2. An archive commit's delta merge rewrites a live requirement in
   `openspec/specs/`, and the plugin's `closer` has no stop for that. Rule 11
   closed this.
3. A red run goes to a fixer and then straight back to the `closer` with no
   review in between (plugin `skills/run/SKILL.md:220-232`). Rule 1's
   red-run route closed this.

Each route ends in `gh pr merge --squash`. **Branch protection does not catch
any of them.** `gh api repos/fryorcraken/dialectica/branches/main/protection`
reports `required_approving_review_count: 0`, so a merge needs no review at
all. The settings that are on cannot tell reviewed content from unreviewed:
strict status checks, signed commits and `enforce_admins` all pass a green,
signed squash whatever it holds. The specs this repo protects include
op-authenticity and moderation-authorisation requirements, so content that
skips review can weaken a security contract. Route 3 is not new: `main`'s
`closer.md` also merged after a fixer, and rule 1 is what covered it. Routes 1
and 2 occur only when a rebase conflicts or an archive changes a spec.

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

While the plugin is unpublished its marketplace is a local clone, so the
entry is machine-local and `settings.json` gets only `enabledPlugins`. Each
contributor clones `https://github.com/fryorcraken/agent-spec-flow` into a
directory of their choice and registers that clone with `claude plugin
marketplace add <the directory cloned into>`, the two steps `README.md`'s
"Working with the agent flow" documents with one example directory, never a
contributor's real path. The `add` default scope is `user` (`claude plugin
marketplace add --help`), so it reaches every worktree rather than one
checkout. The owner
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
  an orphaned PR is redundant (`.claude/agents/README.md`). This rule is not
  actually dropped: the plugin already carries it in
  `skills/worktree-discipline/SKILL.md`, under "PR refs", and the overlay need
  not repeat it.

## Risks / Trade-offs

- [The flow regresses on #174's rules until the plugin carries them] → item 5
  is in the overlay. Items 1 to 4 and 6 to 12 are tracked in
  <https://github.com/fryorcraken/agent-spec-flow/issues/1>, in its body and
  its first comment, and are absent from the active flow until the plugin
  carries them. The owner accepted this on the condition that the issue tracks
  them accurately. Accepting it accepts three routes to `main` for unreviewed
  content, and branch protection does not catch them
  (`required_approving_review_count: 0`). "The owner accepted the regression"
  above names the three routes.
- [The overlay draws on `ci.yml` and branch protection, both of which move] →
  it names commands (`gh api …/protection`) rather than lists where it can,
  and names the step whose output a claim comes from.
- [A session cannot see the plugin until the marketplace entry exists] →
  `README.md` carries the command that adds it, and the issue's fourth check (a fresh
  session lists the `specflow:*` agents) is the owner's to run after merge.
