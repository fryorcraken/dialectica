# Tasks

## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec delta: a test-only piece, `skip_specs: true` in `.openspec.yaml` says why; `proposal.md` is written
- [ ] design + code — `dev-writer`
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

**The full run is proven in CI and nowhere else.** No local run of the updated
spec against a launched Basecamp was made: local launches go through
`lgs basecamp launch` (owner's rule), which sitometres cannot own (design.md
D3). Every box below that depends on the run says which CI run ticked it.

### 1. Carried from PR #120

- [x] 1.1 `adjudicate-ui-run.py` and `tst_adjudicate_ui_run.py` brought over;
      `python3 dialectica-ui/tests/tst_adjudicate_ui_run.py` passes, and
      disabling the step-count branch turns exactly three of its checks red
      (design.md D1 — the docstring's attribution corrected to the measured one)
- [x] 1.2 `validate-ui-specs.mjs` brought over;
      `node dialectica-ui/tests/validate-ui-specs.mjs` reports `join.yaml: ok
      (14 steps)` against sitometres 0.1.2
- [x] 1.3 `ui-specs` job in ci.yml runs both of the above on every PR

### 2. Getting past step 1

- [x] 2.1 Diagnosis reproduced locally before any change: with
      `lgs basecamp build` output alone, Basecamp's log reads `Missing
      dependencies detected: delivery_module` / `Cannot resolve dependencies
      for: dialectica` while sitometres' hint blames `dialectica`; with a
      dev-variant `delivery_module` beside the other two, the app opened and
      steps 1–2 passed (design.md D2)
- [ ] 2.2 `ui-tests.yml` builds Basecamp with `lgs basecamp setup` and all three
      modules with `lgs basecamp install`, and runs sitometres against the
      installed profile root; verified by 4.1
- [ ] 2.3 `lgs left scaffold.toml's values alone` compares `tomlq -S .` before
      and after the verbs; verified by a CI run reaching the run step

### 3. The spec, for current `main`

- [x] 3.1 `join.yaml` rewritten: create steps removed, the no-key state asserted
      from the element tree, row-count assertions that could not fail removed
      (design.md D5); validated by 1.2
- [x] 3.2 Five `readonly` handles on `Main.qml` (design.md D6);
      `tst_e2e_handles.qml` passes, and binding each handle to a constant turns
      its test red (both mutations run, then reverted)
- [x] 3.3 Whole QML suite, `check_qml_members.sh` and `check_qml_reachable.py`
      green with the handles added

### 4. Proof

- [ ] 4.1 The `sitometres join spec` job is green on this branch: verdict pass,
      every step passed, 14 of 14 steps — the run URL goes here
- [ ] 4.2 That run's job duration recorded here, as design.md D7's measured
      cost of the `setup`-based job
