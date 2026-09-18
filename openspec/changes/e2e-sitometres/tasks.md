# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

### Prerequisites in the view

- [x] `objectName` on both `TextInput`s in `DStoaListScreen` — on the
      `TextInput` itself, not the wrapping `Rectangle`, because a typing target
      must resolve to an editable node (design.md D5)
- [x] `objectName` on the preview button and the per-row Open button
- [x] Root-level `readonly` aliases on `Main.qml` for what a `state:` assertion
      must read, computed from existing state rather than duplicated (D6)

### The suite

- [x] `dialectica-ui/tests/ui/join.yaml` — 19 steps, parsed against the real
      schema. Covers paste → preview → refused join, and the create-failure
      assertion (D3, D4)
- [x] Every assertion on `state:`, none on `calls:` (D9)
- [x] Failure text asserted by presence and non-emptiness, never by wording

### The gates

- [x] `ui-specs` job in `ci.yml` — globs the directory, parses each spec against
      the sitometres schema, reports the step count per file, needs no host (D12).
      Proven to fail: a spec with a typo'd action key was rejected by name,
      exit 1, while the good spec still reported its count
- [x] `.github/workflows/ui-tests.yml` — pin resolved from
      `[repos.basecamp].pin`, dev `#app` host, dev `.#lgx` modules, no
      `--variant` (D1)
- [x] The built binary really carries the inspector, asserted before the run
- [x] `--strict`, and `set -o pipefail` before the `| tee` (D11)
- [x] Adjudication from the JSON report: verdict, every step, and step count,
      reporting every failing condition rather than the first (D11). Its own
      tests pin all four, and disabling the count branch turns exactly three
      of them red
- [x] A missing report is named as "nothing was proved", not as a missing file
- [x] Evidence uploaded on failure only

### Open: the spec does not yet pass, and why that is not a ticked box

- [ ] **The `sitometres join spec` job is RED.** Three runs; every step up to
      the run passes — the inspector assertion, the matched pair, and
      `lgs left scaffold.toml alone` — and then step 1 fails after 120s with
      `dialectica_ui declares the core module "dialectica", and Basecamp could
      not load it`. The remaining 18 steps report inconclusive.

What the three runs established, so the next person does not repeat them:

- `with: [dialectica]` was missing and is now present — the header line moved
  from `resolving` to `staging dialectica_ui, dialectica`, so the staging half
  is fixed. **It was necessary and not sufficient**: the same message persists.
- The failure is NOT the variant mismatch D1 warns about, despite presenting
  identically (app never opens, step 1 times out). Both halves are dev.
- **The first two runs could not be diagnosed at all**, because sitometres
  reaps its throwaway user-dir — and the Basecamp log with it — on the clean
  exit a failing spec produces. `--user-dir` now keeps it (D7a). The next run
  is the first that can say *why* the module did not load.

The honest reading: everything downstream of the report is proven — the
adjudicator and its guards, the schema validation, and the evidence upload,
each exercised for real by these runs. **The suite itself has not yet passed
once**, so nothing here should be read as saying the view works end to end.

### Corrections to stale claims

- [x] `ci.yml` header (the count clause was itself off by one and is now a
      command rather than a number), and the `LGS_VERSION` clause invalidated
      by this change (D2)
- [x] `ci.yml`'s "Not yet a job" entry — already rewritten by `spec-writer`;
      verified true against this implementation rather than re-edited
- [x] `docs/PLAN.md`'s "Deliberately not built" entry, and §10's `setup` claim,
      which this change makes load-bearing across two workflows. The sitometres
      pinning reasoning moved out of PLAN.md into design.md D2a
