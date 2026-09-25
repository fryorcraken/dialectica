# Tasks

## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec delta: a test-only piece, `skip_specs: true` in `.openspec.yaml` says why; `proposal.md` is written
- [x] design + code — `dev-writer`
- [x] tests — `tester` — see "5. Tester's pass" below: one new test added
      (`tst_scaffold_values_unchanged.py`), one gap in `tst_adjudicate_ui_run.py`
      closed, and two items left explicitly unproven for the owner
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
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
- [x] 2.2 `ui-tests.yml` builds Basecamp with `lgs basecamp setup` and all three
      modules with `lgs basecamp install`, and runs sitometres against the
      installed profile root. Verified by 4.1: sitometres reported all three
      modules `in place` in the profile's module root
- [x] 2.3 `lgs left scaffold.toml's values alone` compares `tomlq -S .` before
      and after the verbs. It passed in both runs of 4.1. It has not been shown
      to fail: no run has had a verb change a value. **Tester, partial:** the
      guard's own comparison mechanism (`tomlq -S .` snapshotted, then diffed)
      is now pinned by `tst_scaffold_values_unchanged.py`, which extracts both
      real steps from `ui-tests.yml` by name and runs them against fixture
      `scaffold.toml`s: it fails on one hex digit of a pin changed, and does
      NOT fail on a comment-stripped, key-reordered, differently-quoted rewrite
      with every value identical — the shape CLAUDE.md says every `lgs
      basecamp` verb performs. So the check can tell a value change from an
      `lgs`-style rewrite. What remains unproven, because it is a fact about
      `lgs` rather than about the diff: whether `lgs basecamp setup` or
      `install` themselves ever rewrite a VALUE (as opposed to only stripping
      comments and reordering). That needs a real `lgs` run, which the owner's
      rule keeps out of local hands and which no CI run so far has had reason
      to exercise. See item 2 in "5. Tester's pass" below.

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

- [x] 4.1 The `sitometres join spec` job is green on this branch, twice on the
      same commit (cold, then warm). In both, the adjudicator printed `verdict:
      pass`, `[pass]` for each step, and `ok: all 14 steps passed`, against the
      spec's 14. https://github.com/fryorcraken/dialectica/actions/runs/36090846720
      (attempts 1 and 2)
- [x] 4.2 Measured cost is in design.md D7's table: 9m19s cold, 3m16s warm

### 5. Tester's pass

The dev-writer flagged two things it had never seen go red. One is now closed
locally; the other needs a CI run this piece does not push on its own, per the
owner's "do not push anything" instruction. Both mutations below are written
down precisely enough for someone with CI access to run them.

1. **`join.yaml`'s no-key-state step, never seen red with the create
   affordance present — still unproven.** The concern is real: nothing has
   ever exercised the case where `createTitleField`/`createStoaButton` exist in
   the tree at the same time the no-key snapshot is taken, so nobody has
   watched the `not_text:` half of that step actually catch it. This is
   structurally an e2e-only question — component tests run with the host
   absent, and the question is specifically about the interaction between
   sitometres' `text:`/`not_text:` matchers and the real inspector's element
   tree — so it cannot be closed locally, and the owner's rules keep sitometres
   and a directly-launched Basecamp out of local hands.

   **The mutation to run in CI, and the expected outcome:** in
   `dialectica-ui/src/qml/DStoaListScreen.qml`, the `Loader` guarding the
   create block reads `active: screen.machineKey.state === "held"` (line 919).
   Change it to `active: true`, leave `join.yaml` untouched, and run the
   `sitometres join spec` job. Expected: the step named "a fresh profile is
   offered a key and not a Stoa" fails, because `createTitleField` and
   `createStoaButton` now exist in the tree the `not_text:` half is matched
   against; the adjudicator should report that step as not-pass and, because
   sitometres does not continue past a failed `wait_for`, likely also report a
   short step count. Revert the one-line change immediately after the run,
   whichever way it goes, and record the observed output here — a red run
   confirms the check discriminates; a green run means the check or the
   `Loader` binding needs another look before this is trusted further.

   Reported to the owner rather than run, because running it needs a pushed
   commit and a CI run, which "do not push anything" rules out for this agent.

2. **`lgs left scaffold.toml's values alone`, never shown to fail — partially
   closed.** See 2.3 above: `tst_scaffold_values_unchanged.py` now proves the
   guard's `tomlq -S .` + `diff` mechanism distinguishes a changed value from
   an `lgs`-shaped comment/reorder rewrite, entirely locally (no `lgs`, no
   Nix, no Basecamp). What is still unproven is whether `lgs basecamp
   setup`/`install` themselves ever produce a value change for this guard to
   catch — that is a fact about `lgs`, not about the diff, and needs a real
   run of those verbs to observe. **A CI mutation that would test the real
   thing, if the owner wants it run:** temporarily add an unused key under
   `[modules.dialectica_ui]` (or any table `lgs basecamp install` rewrites) in
   `scaffold.toml` immediately before the "Read scaffold.toml before lgs
   touches it" step, structured so `install`'s own rewrite would plausibly
   normalise or drop it — expected outcome is either the guard staying green
   (evidence `lgs` really does leave values alone) or going red (evidence it
   does not, which would be a real finding about `lgs`, not about this test).
   Not run here, for the same reason as item 1.
