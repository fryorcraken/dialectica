# Correctness review — e2e-suite-review

Scope: every file `git show --stat 8368b2f` touched, as it stands at this
piece's HEAD (`ee8e145`, fast-forwarded from `8368b2f`), plus this piece's own
diff (`git diff 8368b2f...HEAD`, which is documentation and a `CLAUDE.md` line
only — no code). Correctness dimension only.

## Findings

- [ ] **`dev-writer`** — `dialectica-ui/tests/adjudicate-ui-run.sh:48` — the
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

- [ ] **`dev-writer`** — `dialectica-ui/tests/adjudicate-ui-run.sh:56` — a spec
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
