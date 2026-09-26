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
stage. D1–D5 cover the CLAUDE.md line (part 3), the two CI proofs (part 2), and
the archive's claims that part 3 leans on. D6–D10 are the fixes to what the
reviews found in #164's code.

**Decision numbers here are this change's own.** Code that cites the merged
suite's decisions names the `e2e-ui-suite` change, because both documents will
sit under `openspec/changes/archive/` with overlapping numbers (D10).

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
failure is correct, since nothing was proved. Its stated cause was wrong
whenever an earlier step stopped the job; D6 corrects it.

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
- **Corrected: archived D1's premise for the missing-report message.** It says
  the report "is missing only when the process never reached its exit: a job
  timeout or an OOM kill". That holds for sitometres, and not for the step that
  reads the report, which runs on `always()` and so also runs when an earlier
  step stopped the job before sitometres started. D6 has the consequence.
- **Corrected: the archived proposal's coverage claim.** Archived
  `proposal.md` says every behaviour `join.yaml` asserts is already a
  requirement, and `join.yaml`'s header and the archived `.openspec.yaml` say
  the same. Two steps had no requirement behind them: "it starts on the Stoa
  list" (no requirement named the opening screen; others only ruled out the
  feed and the onboarding screen) and "it is refused and the list is still up"
  (`stoa-navigation-view` required the refusal and that no call is made, but
  nothing said the list stays up). The spec-test review found both. This
  change's `view-navigation` delta adds the two requirements, "The view opens
  on the Stoa list" and "A paste refused as not a Stoa reference leaves the
  list rendered", so the claim holds once this change is archived. The view
  already behaved that way, so no view code changes. The archive is left as
  written.

### D6 — A missing report names both causes, and the step stays on `always()`

The adjudicator cannot see why a report is missing, only that it is. On the
3.1 red run it blamed a job timeout for a run the scaffold guard had stopped
before sitometres started, and the guard's own error was two steps above.
**Chosen:** the message names both causes, "never started (an earlier step
stopped the job: see its error above)" and "killed before it could write one
(a job timeout, or OOM)", and still says nothing was proved. It remains a
`NO SPEC:` behaviour (proposal: the adjudicator is in no capability).

**Rejected:**

- **Skipping the step when sitometres never started**
  (`if: always() && steps.<run>.outcome != 'skipped'`). Today a skipped run
  step implies an earlier failure, so the job would still be red. But this
  step is the gate that says nothing was proved, and the condition makes
  whether the gate runs depend on another step's `if:`. If the run step ever
  gains a condition of its own, such as a per-spec one, a skipped run and a
  skipped gate make a green job that proved nothing. `always()` fails closed;
  the cost is a second red annotation on a run that is already red.
- **Passing the run step's outcome into the script** so it can name the one
  cause. That would add an input to the script, which the workflow has to
  supply correctly, only to choose between two sentences. The step log already
  says which step stopped the job, so the message points there.

**What breaks without it:** `tst_adjudicate_ui_run.sh`'s "names a run that
never started" goes red, and nothing else. Measured by running the suite
before the wording changed.

### D7 — A spec with no `steps:` list is a reported problem, not a jq abort

The step-count condition compares against the length of the spec's `steps:`
list, so a spec without one has to be refused rather than counted. `null`'s
length is 0, which an empty run matches, and a number's is its absolute value,
which would count `steps: 2` as two. #164 refused it by calling jq's `error()`
inside `expected=$(…)`. Under `set -eu` that ended the script with jq's exit
code 5 and a bare `jq: error`, with no `::error::` and none of the other
conditions checked. That broke the script's rule that every failing condition
is reported before exiting.

**Chosen:** the filter yields `null` for anything that is not a list, a yq
failure (a spec that does not parse) also becomes `null`, and `null` is one
more `problem`. The count comparison runs only when there is a count to compare
against. It keeps its "equal, or else a problem" form, which fails closed on a
non-integer (archived D1).

**What breaks without each part, measured:**

- With the type check replaced by a bare `length`, 4 checks go red: all three
  of "a steps: value that is not a list", where `steps: 2` then prints
  `ok: all 2 steps passed` and exits 0, and "names the cause" in the no-steps
  case, which then reports only `spec has 0`.
- With the `if ! expected=$(…)` guard removed, 2 checks go red: the
  unparseable-spec case's "names the cause" and "still reports the verdict".
  yq exits 1 there, so that case's exit check cannot tell the two apart.
- Before this change, 7 checks were red across the new cases and the
  missing-report case (D6 has one of them).

### D8 — No workflow splices an expression into a `run:` body, and a check keeps it that way

Actions substitutes a `${{ … }}` into a `run:` body's text before the shell
parses it, so a value carrying shell syntax would run as shell. Passed through
`env:`, it reaches the shell as a variable and is one word. #164's
`ui-tests.yml` spliced `${{ matrix.spec }}` into two `run:` bodies, and both
workflows spliced `${{ env.LGS_VERSION }}` into `cargo install`, while every
other value in those jobs came through `env:`.

None of this is exploitable today. The matrix is a hand-typed `[join]`, the
version is a repo literal, and both workflows run on `pull_request` with
`contents: read` and no secrets, so a fork that could change the matrix could
already add any step it liked. The risk arrives if the matrix is ever derived
rather than typed, which is the direction the "every spec in the tree is in
the matrix" step points.

**Chosen:** `REPORT` and `JUNIT` as job-level `env:` in `ui-tests.yml`'s `spec`
job, beside `SPEC`, and `"$LGS_VERSION"` in both `cargo install` steps. Plus a
check, `tst_workflow_run_bodies.sh`, run from ci.yml's `ui-specs` job, that
fails on any `${{` in any `run:` body of any workflow file. It globs
`.github/workflows/` rather than naming the two files, and fails on finding
none.

**Why a check and not only the edit:** a fix to three lines holds only until
the next step is written the old way, and nothing would notice. The rule is
total over workflow files, and the committed files contain no exception to it.

**Rejected:** limiting the check to `ui-tests.yml`, the finding's scope.
ci.yml's `build` job had the same `LGS_VERSION` splice, and a check that
covers one file of two leaves a rule that holds by accident in the other.

**What breaks without it, measured:** against the workflows as #164 left them,
the check names four steps, ci.yml's `build` "Install logos-scaffold" and
ui-tests.yml's "Install logos-scaffold", "Run the spec" and "The run proved
what the spec asks". With its `select` made to match nothing, exactly one check
goes red, the injected-splice case. Its pairing case moves the same expression
into the step's `env:` and must pass.

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
