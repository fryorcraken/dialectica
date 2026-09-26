# Correctness review — e2e-suite-review

Scope: every file `git show --stat 8368b2f` touched, as it stands at this
piece's HEAD (`ee8e145`, fast-forwarded from `8368b2f`), plus this piece's own
diff (`git diff 8368b2f...HEAD`, which is documentation and a `CLAUDE.md` line
only — no code). Correctness dimension only.

## Findings

- [x] **`dev-writer`** — `dialectica-ui/tests/adjudicate-ui-run.sh:48` — the
      missing-report diagnosis names a specific cause that is wrong whenever an
      earlier step stopped the job, which is the common case in `ui-tests.yml`.
      **Scenario:** the script always prints exactly
      `"::error::no JSON report — sitometres was killed before it could write
      one (job timeout?), so nothing was proved"` when `$report` does not
      exist, with no distinction between "sitometres started and was killed"
      and "sitometres never started." In `ui-tests.yml`'s `spec` job, "The run
      proved what the spec asks" runs with `if: always()`, so it also runs
      after the earlier "lgs left scaffold.toml's values alone" guard fails
      and stops the job before "Run the spec" ever executes — the exact case
      recorded in this piece's own `tasks.md` 3.1: the real CI run
      (https://github.com/fryorcraken/dialectica/actions/runs/36142050293)
      printed this same "(job timeout?)" message when the true cause was the
      scaffold guard, not a timeout or a kill.
      **Reproduced locally:**
      `sh dialectica-ui/tests/adjudicate-ui-run.sh tmp/does-not-exist.json dialectica-ui/tests/ui/join.yaml`
      exits 1 and prints the job-timeout message unconditionally, for a report
      that never existed for any reason.
      **Measured:** `design.md` D1 already names this as "a review finding for
      this piece rather than a change made here" — this entry is that finding,
      filed as instructed. `tst_adjudicate_ui_run.sh`'s matching case (line
      114-118) only asserts the generic phrase `"nothing was proved"` is
      present and that jq's own `"Could not open"` is absent; it does not pin
      the specific "(job timeout?)" wording, so a fix that removes or
      generalises the guess would not need that test changed.
      **Severity:** low-to-moderate — never wrong about there being no
      evidence, but wrong about why, in exactly the case (an earlier guard
      failing) that this piece's proposal predicts and that CI has already hit.
      A person debugging a red `ui-tests.yml` run from this message alone is
      pointed at "job timeout?" when the real cause is a step log two entries
      above.
      **Fixed** in the commit that flips this box (the same fix answers
      `design-review.md`'s finding, ticked in the same commit). The message
      now names both causes, "never started (an earlier step stopped the job:
      see its error above)" and "killed before it could write one (a job
      timeout, or OOM)", and still says nothing was proved and exits 1. The
      step stays on `always()`; design.md D6 says why gating it on the run
      step was rejected. Test: `tst_adjudicate_ui_run.sh`'s missing-report
      case gains "names a run that never started" and "names a run that was
      killed". **Predicted** before the wording change: "never started" red,
      "was killed" green. **Observed:** the same, 1 of the 7 red checks in
      that run.

- [x] **`dev-writer`** — `dialectica-ui/tests/adjudicate-ui-run.sh:56` — a spec
      with no (or a non-array) `steps:` key crashes with jq's own raw error and
      a non-standard exit code, bypassing the script's own `::error::`
      convention that every other failure path in this file follows.
      **Scenario:** `expected=$(yq '.steps | if type == "array" then length
      else error("the spec has no steps: list") end' "$spec")` calls jq's
      `error(...)` inside a command substitution. Under `set -eu`, that
      substitution's non-zero exit aborts the script immediately, before the
      `problem()` collection logic runs, so the failure looks nothing like the
      rest of the script's diagnostics.
      **Reproduced locally:** a spec containing only `app: dialectica_ui` (no
      `steps:` key) run as
      `sh dialectica-ui/tests/adjudicate-ui-run.sh tmp/report.json
      tmp/spec-nosteps.yaml` exits **5** (jq's own exit code, not the script's
      documented `1` or the usage error's `2`) and prints the bare
      `jq: error (at <stdin>:1): the spec has no steps: list`, with no
      `::error::` prefix, no GitHub Actions annotation, and none of the other
      conditions checked.
      **Severity:** low — no real spec in the tree is missing `steps:`, and the
      run still fails (non-zero exit), so nothing passes that should not. But
      the header comment's stated design goal — "EVERY failing condition is
      reported before exiting" — silently does not apply to this one
      precondition, and a future spec author's typo (`stpes:` for `steps:`)
      would get a jq stack trace instead of the clear message every other
      branch of this script gives.
      **Fixed** in the commit that flips this box. The filter yields `null`
      for a `steps:` that is absent or not a list, a yq failure on a spec that
      does not parse becomes `null` too, and `null` is one more `problem()`,
      so every other condition is still checked and reported and the exit is
      1 with an `::error::` line. The count comparison runs only when there
      is a count, and keeps its fail-closed form (design.md D7). Tests, each
      marked `NO SPEC:`, in `tst_adjudicate_ui_run.sh`: a spec with no
      `steps:` (paired with a failing report, so "still reports the verdict"
      and "still reports the step" show nothing was cut short), `steps: 2`
      against a green two-step report, and `steps: [`. **Predicted** before
      the fix: 3 red in the no-steps case (exit 5, verdict, step; "names the
      cause" green, since jq's own error carries the phrase), 1 in the
      `steps: 2` case (exit 5), and 2–3 in the unparseable case depending on
      yq's exit code. **Observed:** 3, 1 and 2 (yq exits 1 on a parse error,
      so that case's exit check passed). Mutations after the fix, each
      restored: the type check replaced by a bare `length` turns 4 red, and
      `steps: 2` then prints `ok: all 2 steps passed` and exits 0 (predicted
      4); the `if ! expected=$(…)` guard removed turns 2 red (predicted 2).

      **`tester`'s check on "each marked `NO SPEC:`" above:** the file carries
      two `NO SPEC:` marker lines in total, not three — one above the
      missing-report case (this finding's other entry) and one above "a spec
      with no `steps:` list is reported". The second marker's own text
      explicitly covers the two cases after it, "a `steps:` value that is not
      a list is refused, not counted" and "a spec that does not parse is
      reported", which is exactly what `proposal.md`'s "The `NO SPEC:` markers
      … stay" section describes as the intended shape (one marker, three
      cases) rather than an omission. `tasks.md` 5.2 read as "three separate
      marker lines" is corrected there. No marker line is added, and no test
      case changes; this is a wording clarification only.

## Checked and clean (no finding filed)

- **The `.steps` step-count guard does not depend on any YAML 1.1/1.2
  divergence between the adjudicator's parser (`yq`, PyYAML-backed) and
  sitometres' own parser (npm `yaml`).** Ran `yq '.steps | length'
  dialectica-ui/tests/ui/join.yaml` locally (kislyuk/yq 4.1.2, jq 1.8.1,
  matching the apt package both workflows install): returns `14`, matching
  both `tasks.md`'s recorded step count and `join.yaml`'s own 14 `- name:`
  entries. `join.yaml` has no anchors, merge keys, or YAML-1.1-only scalars
  (octal/sexagesimal numbers, bare `yes`/`no`/`on`/`off`) that could make the
  two parsers disagree on array *length* — a version divergence between the
  two YAML specs changes how an ambiguous scalar's *type* is read, not how
  many items a block sequence has, so this guard is not exposed to the
  divergence the lead asked about, at least for the spec that exists today.

- **The four shell scripts (`adjudicate-ui-run.sh`, `require-jq-yq.sh`,
  `tst_adjudicate_ui_run.sh`, `tst_ui_tool_pins.sh`,
  `tst_scaffold_values_unchanged.sh`) are POSIX/dash-safe.** Grepped all five
  for `[[`, `local`, bare `==` in a shell `[ ]` test (as opposed to inside a
  quoted jq/yq filter string, where `==` is correct), arrays, `<<<`
  here-strings, `declare`/`typeset`, and `${var,,}`/`${var^^}` case
  conversion: none found. Every `==` hit is inside a jq or yq filter literal,
  not shell syntax. This machine's `/bin/sh` is bash (not dash), so running the
  suites locally does not itself prove dash compatibility — the static check
  above is what does. All three test suites (`tst_adjudicate_ui_run.sh`,
  `tst_ui_tool_pins.sh`, `tst_scaffold_values_unchanged.sh`) also ran clean
  under that bash-as-`/bin/sh`.

- **`tst_e2e_handles.qml` passes** (7/7, via
  `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_e2e_handles.qml`),
  including `test_a_failed_reload_does_not_count_the_listing_it_kept`, and
  `Main.qml`'s `stoaCount` is bound to `list.visibleRows.length` (the guarded
  listing), not `list.lastListing.length` (the unguarded one an earlier review
  in #164 flagged and D6 fixed). I could not re-verify the mutation directly:
  editing `Main.qml` to swap the binding was refused by this session's
  permission classifier ("Modify Shared Resources"), so per this task's
  instructions I did not retry or route around it. The claim that this
  mutation turns exactly that test red is therefore carried forward from
  #164's own commit message ("Binding stoaCount to list.lastListing.length
  turns this test red and no other"), not re-measured by me.

- **Neither of this piece's two deliberate CI-proof breaks remains in the
  branch's net diff**, confirmed directly rather than by trusting `tasks.md`
  4.1's claim: `git diff --stat 8368b2f HEAD -- scaffold.toml
  dialectica-ui/src/qml/DStoaListScreen.qml` prints nothing.

- **`ci.yml`'s `ui-specs` job and `ui-tests.yml`'s spec-matrix wiring**
  (spec-count-vs-matrix guard, tool-pin literals, the padded step-listing jq
  filter in `adjudicate-ui-run.sh`) were read and, where cheaply testable
  locally, exercised: the padding filter was checked against a 3-step fixture
  with varying verdict-string lengths and renders correctly aligned; the
  file-count in `dialectica-ui/tests/ui/` (1) matches the matrix (`spec:
  [join]`, 1 job). `check_qml_reachable.py`'s docstring-only change accurately
  describes the sitometres specs under `tests/ui/` as the dynamic-reachability
  half it defers to.

## Not reviewed here

- Whether sitometres 0.1.2's own behaviour (padding a truncated report with
  `inconclusive` steps rather than a short report, `validateSpec`'s exported
  verbs, etc.) matches what `design.md` D2/D5 claim — that is a claim about a
  third-party package pinned by SHA, not about dialectica's own code, and this
  piece's design-reviewer/spec-test-reviewer lanes are better placed to check
  it against the pinned tag.

## Round 2 (HEAD 532794e)

Scope: `git diff ee8e145...HEAD` (the fix pass reshaping the adjudicator, the
workflows and the scripts; the new `install-yq.sh` and
`tst_workflow_run_bodies.sh`; the `view-navigation` spec delta; the new
`tst_e2e_handles.qml` cases), read against the whole suite. Correctness
dimension only.

No new findings. Both round-1 fixes hold, verified by re-running the suites
rather than by reading the diff alone:

- **The missing-report message now names both causes**
  (`adjudicate-ui-run.sh:52`), and `tst_adjudicate_ui_run.sh`'s "names a run
  that never started" / "names a run that was killed" cases pass. Ran
  `sh dialectica-ui/tests/tst_adjudicate_ui_run.sh`: all cases green.
- **A spec with no, a non-list, or an unparseable `steps:` is now reported as
  a problem rather than crashing on jq's own error**
  (`adjudicate-ui-run.sh:70-92`, guarding the substitution with
  `if ! expected=$(…); then expected=null; fi` under `set -eu`). Ran the same
  suite's three new cases ("a spec with no `steps:` list", "a `steps:` value
  that is not a list", "a spec that does not parse"): all green. Mutation-
  tested by hand to check the guard is load-bearing and not merely decorative:
  collapsing the type check and the `if !`-guard into a bare
  `expected=$(yq '.steps | length' "$spec")` (removing both at once, a
  stronger mutation than the one `tasks.md` 5.2 records) turned 6 of the added
  checks red, including "exit 1" on the `steps: 2` case, which now passes
  clean at `ok: all 2 steps passed`. Mutation restored;
  `git status --short` confirmed clean before committing this file.
- **No workflow splices a `${{ … }}` expression into a `run:` body** (the
  `LGS_VERSION`/`REPORT`/`JUNIT` cargo/adjudicator lines in both workflows now
  read the value through a job-level `env:` var instead). Ran
  `sh dialectica-ui/tests/tst_workflow_run_bodies.sh`: reports both `ci.yml`
  and `ui-tests.yml` clean, and its two synthetic cases (a splice added to a
  fixture, then moved through `env:`) are correctly flagged/cleared.
  Confirmed by grep that no `matrix.spec` reference remains inside any `run:`
  body in `ui-tests.yml` — the surviving splices are all in `name:`, job-level
  `env:`, and `with:` (artifact naming), which the check correctly leaves
  alone.

The new `view-navigation` test material was checked against the running code,
not just the spec text: `Main.qml`'s `screenShown` (lines 103-107) and its
five main-area screens' `objectName`s (`stoaList`, `joinScreen`, `feed`,
`thread`, `moderation`, lines 323-469) match `tst_e2e_handles.qml`'s
`mainAreaScreens` table exactly, and the "no bridge to the core" case
(`Core.bridge = null`) exercises a real robustness property — `makeStandaloneMain`
does not crash and still asserts the list alone is rendered. Ran
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_e2e_handles.qml`:
8/8 pass, including the two new table tests. Also traced the "failed
master-key query with an empty listing" case that the new tests' comment says
is pinned elsewhere (`tst_stoa_screens.qml`'s
`test_the_view_supplies_no_stoa_of_its_own_before_one_is_chosen`): that test
never supplies a `get_master_key` reply, and both files' `bridgeFor` default
an unlisted method to the error shape (`{"error":"no fake reply for …"}`), so
the claim holds — it is genuinely the error-shape case, not a distinct
unconfigured-default case.

Checked and clean, no finding: `install-yq.sh` (apt-source-aside procedure
matches the two workflows' prior inline steps verbatim; `"$@" yq` passes the
job's package list through correctly); the `REPORT`/`JUNIT` env additions in
`ui-tests.yml` (job-level `env:` may reference `matrix`, and both are used
consistently at each of their call sites); the doc-only attribution edits in
`adjudicate-ui-run.sh`, `require-jq-yq.sh`, `tst_ui_tool_pins.sh`,
`tst_scaffold_values_unchanged.sh`, and `join.yaml` (each just qualifies a
`design.md` reference with which change's `design.md`, no code changed).
