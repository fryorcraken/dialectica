# Design — proving #164's two unproven checks, and the yq/jq rule

## Context

See proposal.md for why this piece exists and its three parts. The archived
`openspec/changes/archive/2026-09-25-e2e-ui-suite/` is the decision record the
merged suite is checked against; it is not edited here, and where a claim in
it turned out wrong or mis-cited, the correction is in this file.

Two facts about the CI this piece runs in shape everything below:

- **Both workflows trigger on `pull_request`, not on a push to a piece
  branch.** A break pushed to `refs/heads/piece/134-e2e-suite-review` runs only
  because the PR is open.
- **Both workflows set `cancel-in-progress: true` on a group keyed by
  `github.ref`** (`ci-…` in ci.yml, `ui-tests-…` in ui-tests.yml). For a PR
  that ref is the PR's merge ref, so every push to the piece branch cancels the
  run before it.

Part 1 of the proposal, the six reviews of what #164 merged, is the reviewers'
stage and not this one. This pass writes the CLAUDE.md line (part 3), runs the
two CI proofs (part 2), and confirms or corrects the archive's claims that
part 3 leans on.

## Goals / Non-Goals

**Goals:** each of the two checks seen red in CI for the reason it exists,
then green again on the revert, with both run URLs and predicted-versus-observed
in `tasks.md`; a branch tip with neither break in it; the owner's
yq/jq rule in CLAUDE.md.

**Non-Goals:** any change to what the checks assert; a local run of
sitometres or of `lgs basecamp setup`/`install` (the owner's rules keep both
out of local hands); the Python heredocs ci.yml had before #164 (proposal,
"Not in this piece").

## Decisions

### D1 — The scaffold guard's red run answers "does `lgs` itself change a value"

The proposal requires the record to say which of two questions the red run
answers: whether the real step fires in CI on a changed value, or whether
`lgs` itself changes a value. A break that answers the second answers the
first too, because the change it produces is observed by the real step
between the real snapshots. A break that answers only the first does not
answer the second. So the break is one `lgs`'s own rewrite acts on.

**Chosen:** add a key `lgs` does not know to `[modules.dialectica_ui]` in
`scaffold.toml`, the table archived `tasks.md` 5 item 2 suggested. Read from
the `logos-scaffold` checkout named in `docs/SOURCES.md`, whose crate version
is 0.3.1, the pin both workflows install:

- `parse_modules` (`src/config.rs`) reads `flake`, `role` and
  `standalone_app` from each module table and ignores any other key, so the
  file still parses and every verb still runs.
- `serialize_config` (`src/config.rs`) builds a fresh document from the parsed
  struct and writes those same three keys back, so the unknown key is not
  written.
- `setup` calls `save_project_config` unconditionally once it has built
  Basecamp and lgpm (`cmd_basecamp_setup` in `src/commands/basecamp.rs`).

  In that checkout the only other caller is `cmd_basecamp_modules`;
  `cmd_basecamp_install` does not rewrite the file. CLAUDE.md and both
  workflows' comments say `install` does too. The guard snapshots across both
  verbs, so the red run cannot say which of them dropped the key, and it does
  not settle that disagreement. It is recorded here as read, not resolved.

So after `setup` the file no longer holds the key, `tomlq -S .` of it differs
from the BEFORE snapshot by exactly that key, and "lgs left scaffold.toml's
values alone" should exit 1 with its own `::error::`. The value it catches is
one `lgs` removed; nothing in the job edited the file between the snapshots.

What this answers, precisely: **`lgs` 0.3.1's rewrite drops a key it does not
know, and the real step, in CI, sees that as a changed value.** It does not
say `lgs` changes a value of the committed `scaffold.toml`. Every green run of
the job is the evidence for that, one run at a time, because the guard would
fail it otherwise.

**Rejected alternatives:**

- **A step that edits `scaffold.toml` between the two snapshots.** It proves
  the step fires on a changed value and nothing about `lgs`. The proposal
  names it as answering only the first question.
- **Deleting a value `setup` fills with a default,** such as
  `[repos.lgpm].attr`, which `setup` writes back as `cli`. It is also an
  `lgs`-originated change, but ci.yml's `lint` job reads
  `cfg["repos"]["lgpm"]["attr"]` in its Python check and would go red on a
  `KeyError`, a second red in the same push for a reason unrelated to the
  guard. It also changes the input `setup` builds from, where the unknown key
  changes nothing `lgs` acts on.

The guard runs before the spec, so this run cannot also say anything about
sitometres: the job stops at the guard. That is why the two proofs are two
pushes (D3).

The red run bore this out: the diff was exactly that one key, and the step
exited 1 with its own error (`tasks.md` 3.1). It also showed something the
prediction missed. The adjudicator step runs on `always()`, found no report
because sitometres never started, and printed its missing-report diagnosis,
"sitometres was killed before it could write one (job timeout?)". The step's
failure is correct, since nothing was proved. Its stated cause is wrong
whenever an earlier step stopped the job, and archived D1 chose that wording
because "a missing-file error reads as a bug in the adjudicator". The
diagnosis now misleads in the other direction, on every run an earlier guard
stops. That behaviour is the adjudicator's, which the proposal keeps out of
any capability, so it is a review finding for this piece rather than a change
made here.

### D2 — The no-key step's break is archived `tasks.md` 5.1, unchanged

**Chosen:** `createBlockLoader`'s `active:` in `DStoaListScreen.qml` set to
`true`, `join.yaml` untouched. The step "a fresh profile is offered a key and
not a Stoa" is the only one that says the create affordance is absent, and its
`not_text:` half had never been seen to fail with the affordance present.

What the red run showed (`tasks.md` 2.1 has the run): the step failed on both
`not_text:` selectors, each naming the element it still found, after polling
to the spec's 30s default. That matches sitometres at the pinned tag (`v0.1.2`,
`6dc23e2`): `notText` fails when `resolveAll` finds any node for the selector,
and it is not one of the checks `isIrrecoverable` lets a `wait_for` give up on
early.

**One prediction was wrong, and it matters for archived D1.** The run
was expected to end its report at the failed step, because `run` builds its
`Runner` with `continueExecution: false`, and so to trip the adjudicator's
step-count condition. It did not. sitometres 0.1.2 records every step it did
not attempt as `inconclusive` ("9 later step(s) were not attempted"), so the
report held all 14 steps and the count matched. The run went red on conditions
1 and 2 (the verdict, and the failed and inconclusive steps), not on 3.

So a run that stops early after a failure does not reach the count condition
on this sitometres; the per-step condition catches it first. Archived D1 says
the count is "the one that catches" a run that "executed nothing further, and
reported a clean sheet", and that stays true: an empty or short report is what
the count guards, and `tst_adjudicate_ui_run.sh` pins it with fixtures. What
this run adds is that the real tool pads rather than truncates in the case
observed here, so the count condition is a backstop for a report sitometres did
not write this way, not the condition an early stop usually trips.

**Rejected:** breaking the spec instead, by deleting its `not_text:` list. That
proves the adjudicator counts, which D1 of the archive already proves locally,
and says nothing about whether the matcher sees the element.

The same break also reddens ci.yml's `qml` job, because
`tst_stoa_screens.qml` contracts the same absence at the component layer. Run
locally with the break applied, it turns exactly three of that file's tests red
and leaves the two other specs that instantiate the screen green; `tasks.md`
2.1 names them. That red is in the other workflow and does not stand in front
of the spec's, so it is expected rather than avoided.

### D3 — One break per push, and every run waited out before the next push

Each break is its own commit and each revert is `git revert` of it, so the
branch's net diff for the proofs is zero and the squash merge carries none of
them. The branch tip at hand-back is a revert or a documentation commit on top
of one, never a break.

**What breaks without the waiting:** both workflows cancel in progress on the
next push to the same PR (Context). Pushing the revert before the red run
finishes cancels that run, and a cancelled run is not a red one: it records
nothing about the check. So the revert is pushed only after both workflows on
the break have concluded.

Two breaks in one push would not work either: the guard stops the job before
sitometres starts, so the no-key step would never run.

### D4 — The CLAUDE.md line sits in "When you forbid a tool, name the replacement"

The proposal allows that paragraph or the costs table. **Chosen:** the
paragraph. The costs table prices shapes by what the permission checker does
with them (`|`, `$(…)`, a path after `cd`), and a `python3` call is not a shape
the checker refuses; it costs a click because the owner has to approve it. The
paragraph is about banned habits and what to reach for instead, which is what
the owner's rule is. It is phrased by cost, as that section's rules are.

### D5 — What the archive got right and what it mis-cited

Checked here because proposal part 3 rests on them.

- **Confirmed: no Python in the suite parses YAML or JSON.** Of the files
  proposal part 1 lists, only `check_qml_reachable.py` is Python, and it reads
  QML. The four shell scripts read YAML with `yq` and JSON with `jq`;
  `ui-tests.yml` reads `lgs basecamp paths --json` with `jq`.
- **Confirmed at the pinned tag: archived D12's case for keeping
  `validate-ui-specs.mjs`.** At `v0.1.2`, `src/cli.ts`'s `KNOWN_VERBS` is
  `smoke, run, inspect, init, doctor`, with no validate verb;
  `src/commands/run.ts`'s `loadSpec` parses with `YAML.parse` from `yaml` and
  then boots; `validateSpec` is exported from `src/index.ts`; and the tag's
  `package-lock.json` resolves `yaml` to 2.9.0, the version ci.yml pins.
- **Corrected: the citations.** Archived D2 and D12 cite
  `paradoxcomputer/sitometres ab6b3ea`. That commit is on a fork branch,
  `fix/inspector-probe-nix-elf-sibling`, whose `package.json` says 0.1.0; it is
  not the released 0.1.2 both workflows pin, which is tag `v0.1.2` at
  `6dc23e2`. Every claim D2 and D12 make from it holds at the tag as well
  (`boot()` builds `wanted` from the app, its declared dependencies and
  `with`, and nothing further), so no decision changes, but a later reader
  should check against the tag.
- **Corrected: CI's `sh`.** Archived `tasks.md` 6.6 and D12 say the scripts
  had not been run under dash. `tasks.md` 1.5 says they ran green "under
  `shell: /usr/bin/bash -e {0}`". Both describe the same runs. The step's
  shell is bash, but ci.yml invokes each script by its path and each one's
  shebang is `#!/usr/bin/env sh`, so the interpreter is the runner's `sh`,
  which on Ubuntu is dash. Every green `ui-specs` run since #164 is therefore a
  dash run. This is inferred from the shebang and Ubuntu's default, not printed
  by any log.

## Risks / Trade-offs

- **The `lgs` source read is a working tree, not the tag.** Its crate version
  is 0.3.1, but the checkout was read without checking it out at `v0.3.1`. →
  The red run is the observation; D1's reading is the prediction, and
  `tasks.md` records both.
- **A pull_request run tests the merge of the branch with `main`.** If `main`
  moves between a break and its revert, the green run tests a different base
  from the red one. → Each run is recorded with the head SHA it ran on.
- **R1 is also red in ci.yml's `qml` job** (D2). → Expected, recorded, and in
  a different workflow from the spec it proves.
