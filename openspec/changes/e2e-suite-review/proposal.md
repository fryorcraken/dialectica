# Review the end-to-end suite #164 merged, and prove its two unproven checks

## Why

PR #164 (`8368b2f`) merged the join-refusal end-to-end run. Part of what it
merged was never reviewed. Its six reviews ran against a Python version of the
suite. After them, one commit rewrote the adjudicator, the tool-pin check and
the scaffold-guard test in shell, added `require-jq-yq.sh`, and changed both
workflows. That commit merged with no reviewer having read it. #164 also left
two of its checks without a red run, because only a CI run with a deliberate
break can show them failing and #164's agents were not allowed to push. Its
archived `tasks.md` section 5 records both.

The owner has also given a rule for this suite and for every agent that works
on it. In their words: "Use CLI yaml checker", "I don't understand why you are
using Python locally for yaml checks", and "I was tired of approving python
yaml scripts when yq could be used".

## What Changes

The owner set this scope. All three parts are in this piece and none of them is
follow-up.

**1. Review all the code #164 merged, and fix every finding in this PR.** The
review covers the files in `git show --stat 8368b2f`, read as they now stand
on `main`. Each target below is reviewed in full, not only this piece's diff,
and all six reviews run: correctness, security, readability, architecture,
spec-test and design. The code files are:

- `.github/workflows/ui-tests.yml`
- `.github/workflows/ci.yml`, meaning the parts #164 added or changed: the
  `ui-specs` job, the header comment, the `LGS_VERSION` comment and the "Not yet
  a job" entries (`git show 8368b2f -- .github/workflows/ci.yml`)
- `dialectica-ui/src/qml/Main.qml` (the five read-only root handles) and
  `dialectica-ui/tests/tst_e2e_handles.qml`
- `dialectica-ui/tests/adjudicate-ui-run.sh`, `tst_adjudicate_ui_run.sh`,
  `require-jq-yq.sh`, `tst_ui_tool_pins.sh` and
  `tst_scaffold_values_unchanged.sh`: the shell rewrite no reviewer read
- `dialectica-ui/tests/ui/join.yaml` and `validate-ui-specs.mjs`
- `dialectica-ui/tests/check_qml_reachable.py` (#164 changed only its
  docstring)

The archived change `openspec/changes/archive/2026-09-25-e2e-ui-suite/` is
the decision record that the code is checked against. If one of its claims turns
out to be wrong, the correction goes in this change's `design.md`. The archive
stays unedited, because it records what was decided at the time.

**2. Show both of #164's unproven checks failing in CI.** For each one, a
writer pushes a deliberate break to this piece's branch, reads the red run,
reverts the break, and records both run URLs: the red run and the green run
after the revert. Neither break may reach `main`.

- **`join.yaml`'s no-key step** ("a fresh profile is offered a key and not a
  Stoa"). The break is the create affordance being present in a fresh profile:
  `createBlockLoader`'s `active:` guard in `DStoaListScreen.qml` set to `true`,
  as archived `tasks.md` 5.1 describes. It is done when the adjudicator reports
  that step as not passing in the `sitometres join spec` job.
- **The `scaffold.toml` guard against a real `lgs` run** ("lgs left
  scaffold.toml's values alone" in `ui-tests.yml`). It is done when that step
  goes red in a run where `lgs basecamp setup` and `install` really ran, because
  a value differed. The record MUST say which question the red run answers:
  - whether the real step fires in CI on a changed value, or
  - whether `lgs` itself changes a value.

  A break injected between the two snapshots answers only the first. Only a
  value that `lgs`'s own rewrite changes answers the second.

In `ui-tests.yml` the guard step runs before the spec. A guard failure
therefore stops the job before sitometres starts. Each red run must show its
own check failing and not be hidden behind the other.

**3. YAML goes through `yq`, JSON through `jq`, and neither through Python.**
This rule covers the suite and every agent working on it. `yq` here means the
Ubuntu jq-wrapper, so filters are written in jq syntax.

- **The merged suite already meets the rule for Python.** None of its own
  Python parses YAML or JSON (`yq` is itself written in Python, and archived
  D12 says why that is outside the rule). Its one Python file, `check_qml_reachable.py`, reads
  QML. `ui-tests.yml` reads JSON with `jq`. The review in part 1 confirms this
  rather than assuming it.
- **`validate-ui-specs.mjs` stays.** It reads specs with the Node `yaml`
  library. It is not Python, and it is the one place the suite parses YAML
  without `yq`. It has to validate a spec with the parser the run itself uses.
  `yq` parses with a different library, so its answer is not the run's answer.
  sitometres also has no verb that validates without launching a Basecamp.
  Archived `design.md` D12 carries the full reasoning, and this change's
  `design.md` confirms or corrects it.
- **`CLAUDE.md` gets a line**, because the owner asked for one in this piece.
  The line names `yq` for YAML and `jq` for JSON, never Python, as the
  replacement agents must use. It goes in "How to
  work in this repo, and what Bash costs", either in the list under "When you
  forbid a tool, name the replacement" or in the costs table. The owner's
  reason: "I was tired of approving python yaml scripts when yq could be used".
  `CLAUDE.md` is injected into every session and agent, so this is how the
  rule reaches them. The `dev-writer` writes the line.

**Constraints from the owner that bind every agent on this piece:**

- Nothing touches the owner's own Basecamp config or data, meaning anything
  outside `.scaffold/`. Local Basecamp runs go through `lgs basecamp launch
  alice` and related verbs only.
- `lgs` is used in CI wherever an `lgs` verb exists.
- No local `npm install` or `pip install`.

**The NO SPEC marker at `tst_adjudicate_ui_run.sh:110` stays.** The run
adjudicator's contract is deliberately kept out of any capability (archived
`.openspec.yaml`), and its missing-report behaviour is recorded in archived
`design.md` D1. No spec exists that the marker could be resolved into. That
is a decision, not an oversight.

The PR carries `Part of #134`.

## Not in this piece

- **The next piece of #134 is wider coverage**, taken from the archived
  `proposal.md`'s follow-up list: a successful join with a seeding peer, the
  feed screen, the thread screen, the moderation screen, and key creation then
  Stoa creation.
- **The Python heredocs `ci.yml` already had before #164.** They read
  `metadata.json`, `flake.lock` and `scaffold.toml`, and they are neither #164's
  code nor part of this suite. Whether the owner's rule extends to them is the
  owner's decision. They are not ported here.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None.

This is a test-only piece. It reviews and fixes a test harness, proves two of
its checks can fail, and adds a working rule to `CLAUDE.md`. None of that
changes what dialectica does. `.openspec.yaml` declares `skip_specs: true` and
says why. If a review finding turns out to need a change to contracted
behaviour, it comes back to `spec-writer` rather than being fixed as a test
change.

## Impact

- **Modified:** `CLAUDE.md` (one line). Any of the files in part 1 that a
  finding touches.
- **Pushed and then reverted, net zero:** the two deliberate breaks, one in
  `DStoaListScreen.qml` and one in `scaffold.toml` or `ui-tests.yml`,
  depending on which question the guard's run is meant to answer.
- **Not affected:** `dialectica/` core, the Rust tree, the wire contract, and
  every requirement in `openspec/specs/`.
