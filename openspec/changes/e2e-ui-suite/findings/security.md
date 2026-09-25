# Security review — `e2e-ui-suite`

Scope: this file covers the **security** dimension only (CI supply chain,
workflow permissions/triggers, and what `Main.qml`'s new root handles expose).
Correctness, readability and architecture are covered by separate reviewer
instances.

Reviewed via `git diff origin/main...HEAD` (three dots) against the local
tree — the tree carries one commit not yet pushed to the PR's remote ref.

## Findings

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:1141`, `.github/workflows/ci.yml:1152`, `.github/workflows/ui-tests.yml:363`
      — two dependencies installed in CI with no version pin at all, in the
      same jobs that go out of their way to pin everything else.
      **Scenario:** `ci.yml`'s `ui-specs` job runs
      `npm install --no-save --silent @paradoxcomputer/sitometres@0.1.2 yaml`
      — `sitometres` is pinned, `yaml` is not; `npm install` with no version
      resolves to whatever `yaml`'s `latest` dist-tag is on the npm registry
      at the moment the job runs, on every PR, forever. The same job and
      `ui-tests.yml`'s `spec` job each separately run
      `python3 -c 'import yaml' 2>/dev/null || pip install --quiet pyyaml`,
      which pins `pyyaml` no better. There is no `package.json`, lockfile or
      `requirements.txt` anywhere under `dialectica-ui/` to fall back on —
      confirmed with `git ls-files dialectica-ui | grep -i package` (no
      output) and `git ls-files dialectica-ui/tests | grep -i package` (no
      output).
      This directly contradicts the discipline this same PR states and
      applies to its other two dependencies: design.md D8 — *"sitometres is
      `@paradoxcomputer/sitometres@0.1.2`, a released version and never a
      range"* — and `ui-tests.yml`'s own comment on `LGS_VERSION` — *"Pin the
      VERSION, never a range or `latest`: a moving ref would silently change
      the tool that gates every spec here."* A compromised or backdoored
      release of `yaml` or `pyyaml` (both widely-depended-on packages, and
      supply-chain takeovers of exactly this kind of dependency are a live
      threat class) would be pulled into every PR run and every push-to-main
      run with no PR diff to review it against. Blast radius is limited by
      there being no secrets in either workflow (verified: no `secrets.*` or
      `github.token` reference in `ui-tests.yml` or the new `ui-specs` job in
      `ci.yml`), so this is a CI-integrity gap (tampered validation/adjudication
      output, or a compromised runner used as compute) rather than a
      credential-exfiltration path — but it is real, cheap to close (pin
      `yaml@<version>` and `pyyaml==<version>`), and the PR's own stated
      policy is the yardstick it fails by.
      **Not measured against a CI run** — no CI run was triggered for this
      review (nothing was pushed, per the runner's instruction). The gap is
      established by reading the workflow text and confirming the absence of
      any lockfile, not by observing a floating install in an actual run.

## Areas checked and found clean

- **Workflow triggers and permissions.** `ui-tests.yml` sets
  `permissions: contents: read` at the top level with no job-level override,
  including the new `spec` job that uploads failure artifacts (artifact
  upload needs no elevated permission). It triggers on `pull_request`
  (fork-safe: read-only default token, no secrets) rather than
  `pull_request_target`, and on `push: [main]` and `workflow_dispatch`. No
  `secrets.*` or `github.token` reference appears anywhere in either
  `ui-tests.yml` or the new `ui-specs` job added to `ci.yml` (checked with
  `git grep`). The pre-existing `release` job in `ci.yml` (untouched by this
  diff) is the only job in the file with `permissions: contents: write`, and
  it is gated on a tag push, not on this change.
- **Third-party GitHub Action pins.** The two non-GitHub-owned actions this
  PR adds — `cachix/install-nix-action` and `nix-community/cache-nix-action`
  — are both pinned to a full commit SHA with the tag in a trailing comment,
  matching the convention the existing `ci.yml` already uses for third-party
  actions (e.g. `softprops/action-gh-release`). The GitHub-owned actions
  (`actions/checkout`, `actions/setup-node`, `actions/cache`,
  `actions/upload-artifact`) are pinned only by major-version tag, which is
  mutable — but that is the same split the rest of `ci.yml` already uses
  (first-party actions by tag, third-party by SHA), so this PR is consistent
  with an existing, deliberate project convention rather than introducing a
  new gap. Noted as an observation, not a defect of this piece.
- **Isolation of the driven Basecamp process.** design.md D9 records, and the
  workflow matches, that sitometres' throwaway `$HOME` is kept (`--real-home`
  is never passed), so the app under test sees no credential of the runner's.
  The QML inspector the run depends on is confirmed present only on the dev
  `#app` build (`the build really has the QML inspector` step, greeping the
  ELF for the inspector's listening-port string) and design.md records the
  shipping build sets `enableInspector = false` — that flag lives upstream in
  scaffold/basecamp packaging, outside this diff, so it is a claim this piece
  inherits rather than one it could newly break.
- **The new `Main.qml` read-only handles**
  (`listReadState`, `stoaCount`, `pasteFailure`, `joinState`, `joinFailure`).
  All five are projections of state the corresponding screen already renders
  on screen (confirmed by reading `dialectica-ui/tests/tst_e2e_handles.qml`,
  which pins each one against real screen state rather than a constant).
  None carries key or identity material — design.md D6 records that
  omission as deliberate ("`Main.qml` records that the navigator must never
  come to depend on one"), and the property list matches that: no property
  here is named or shaped like a secret, and `joinFailure`/`pasteFailure` are
  core-supplied error strings already surfaced to the user by the existing
  screens, not new information reachable only through the inspector.
- **YAML/JSON parsing of untrusted-shaped input.** `adjudicate-ui-run.py` uses
  `yaml.safe_load` (not `yaml.load`) and `json.load` on files whose paths are
  hardcoded by the workflow, not attacker-supplied. `validate-ui-specs.mjs`
  uses the `yaml` package's `parse()`, which does not execute code. No
  `eval`, `exec`, or unsafe deserialization in any of the new Python or JS
  files.
- **No shell-injection surface from workflow-context expressions.** Checked
  for `${{ github.event.* }}`, `github.head_ref`, `github.actor` or similar
  attacker-influenceable expressions interpolated directly into a `run:`
  block in either new/changed workflow — none found. The only expressions
  interpolated into `run:` bodies are matrix values and env vars this
  workflow itself sets from `scaffold.toml`/`lgs` output, not PR-supplied
  text.

## Branch and mutation status

Branch: `worktree-agent-a189ea6c127f04335` (confirmed with
`git rev-parse --abbrev-ref HEAD`; this is a worktree of its own, not
`piece/134-e2e-ui-suite` and not the repository root, so mutation would have
been safe here — I made none).

No files were mutated for this review: the diff for the security dimension
is entirely CI-workflow YAML, a QML property list, and small Python/JS test
utilities, and every question I had (dependency pinning, permissions,
trigger safety, what the new QML handles expose) was answerable by reading
the diff, the referenced design.md sections, and the existing
`tst_e2e_handles.qml` test rather than by breaking something to see if a
test caught it. No Rust files changed in this diff, so `cargo mutants` does
not apply here.
