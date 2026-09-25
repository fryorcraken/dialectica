# Security review — `e2e-ui-suite`

Scope: this file covers the **security** dimension only (CI supply chain,
workflow permissions/triggers, and what `Main.qml`'s new root handles expose).
Correctness, readability and architecture are covered by separate reviewer
instances.

Reviewed via `git diff origin/main...HEAD` (three dots) against the local
tree — the tree carries one commit not yet pushed to the PR's remote ref.

## Findings

- [x] **`dev-writer`** — `.github/workflows/ci.yml:1141`, `.github/workflows/ci.yml:1152`, `.github/workflows/ui-tests.yml:363`
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

      **Fixed** in the commit that ticks this box. There are two halves, and
      the second closes the gap differently from your suggestion.
      - `yaml` is now `yaml@2.9.0` in `ui-specs`. That is the version
        sitometres' own `package-lock.json` resolves (paradoxcomputer/
        sitometres `2fba210`), so it exists, and it is the parser sitometres
        was tested against.
      - PyYAML is no longer fetched at all, rather than being pinned to
        `pyyaml==<version>`. Both `import yaml || pip install --quiet pyyaml`
        lines are gone. ui-tests.yml names `python3-yaml` in its existing
        Ubuntu apt transaction. `ui-specs` uses the runner image's copy and
        fails on the import, by name, if the image ever drops it. The
        fallback was already not what supplied PyYAML. On Actions run
        36091870189 (the `UI spec validation` job), that step finished all
        its adjudicator runs 0.3s after starting, which is too fast for a
        PyPI download. That is inferred from timing: pip ran `--quiet`, so
        nothing was logged either way. A pinned pip install would also need
        `actions/setup-python`. My unverified understanding is that the
        system Python on ubuntu-24.04 refuses `pip install` under PEP 668.
        Either way, it adds a network fetch and a second trust root where
        none is needed. design.md D8 records the rejected alternative.
      - Residual, deferred to design.md Risks ("sitometres' own dependencies
        are resolved at run time"): sitometres' transitive dependencies
        still resolve within its declared ranges, in both `npm install` and
        `npx --yes`. Closing that needs a committed lockfile. Generating one
        needs an `npm install`, and the owner's rule keeps that out of local
        hands.
      **Not verified by a CI run.** Nothing is pushed. `tst_ui_tool_pins.py`
      and `tst_scaffold_values_unchanged.py` both parse the edited workflows
      and pass locally. Whether `npm install … yaml@2.9.0` and the apt line
      succeed on a runner is for the closer's CI run to show.

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
