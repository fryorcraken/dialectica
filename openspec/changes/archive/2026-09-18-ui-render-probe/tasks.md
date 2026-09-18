# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ (skipped: the piece is itself a test; `dev-writer`
      verified it by mutation — twins diverging on the real bug, and all four
      screens demonstrated to fail on a real screen losing its content)
- [ ] ~~review: correctness — `code-reviewer`~~ (skipped: no reviewers
      dispatched — test infrastructure, not a screen, and the owner is waiting
      on visible UI progress)
- [ ] ~~review: security — `code-reviewer`~~ (skipped: same reason)
- [ ] ~~review: readability — `code-reviewer`~~ (skipped: same reason)
- [ ] ~~review: architecture — `code-reviewer`~~ (skipped: same reason)
- [ ] ~~review: spec-test — `spec-test-reviewer`~~ (skipped: same reason)
- [ ] ~~review: design — `design-reviewer`~~ (skipped: same reason)
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

Each box below is ticked against a command that was run on this tree, not
against an intention. Where a requirement holds by construction rather than by a
test, that is said rather than implied.

### The probe

- [x] `dialectica-ui/tests/tst_render_probe.qml` covers all four screens —
      `FeedScreen`, `DStoaListScreen`, `DJoinScreen`, `DOnboardingScreen` — each
      driven to a populated state through the faked `Core.bridge`, with the
      populated state asserted as the fixture's own precondition before the paint
      is asserted.
- [x] The `Item`-root / sibling-`TestCase` layout is used, per the spec's pinned
      shape. Measured on this tree: with `TestCase` as the file root, a red+blue
      Rectangle grabs `#ffffff` 0-differing in all three sub-shapes (declared,
      `createTemporaryObject`, and the TestCase itself).
- [x] No two subjects overlap, and the `TestCase` is parked clear of every
      subject cell.
- [x] Sampling is on a stride (4px), compared against the first sampled pixel,
      stopping at the first difference.
      `test_the_sampler_stops_early_on_paint_and_scans_when_flat` pins both
      directions arithmetically: a painted 120x80 region stops before 600
      samples, a flat one costs exactly 600, and a 1x1 region is exactly 1.

### The demonstration that it can fail

- [x] `RenderProbeHealthy.qml` and `RenderProbeShadowed.qml` differ only by
      `property var data` — the sibling repo's real `CommitView.qml` bug.
      Measured: `children.length` 2 against **0**; grabs `#1e3a5f` non-flat
      against `#ffffff` flat.
- [x] `check_probe_twins.sh` enforces the one-line difference mechanically, and
      `tst_check_probe_twins.sh` pins it in both directions over 10 cases —
      including a twin whose colours also differ (rejected) and an empty corpus
      (rejected rather than reported clean).
- [x] **The four screen probes were demonstrated to fail on a real screen
      losing its content**, not only on the synthetic twins. With
      `ScreenFrame`'s body set `visible: false`, all four go red reporting flat
      `#efe9dc`. The same mutation left an earlier whole-card version of this
      probe at 9 passed / 0 failed — which is what found the false green.

### Bounded coverage

- [x] Satisfied by construction, and said plainly rather than ticked as tested.
      The spec's fifth requirement forbids inferring correctness, layout, colour,
      text or completeness from a pass. Nothing in the probe reads any of those:
      it compares pixels for inequality and never against a reference, so there
      is no code path that could assert them. The boundary is written into the
      spec file's header and `design.md` so it is not over-read later; it is not
      a testable behaviour.

### Gates

- [x] Full QML suite green on this tree: 19 spec files, 375 tests, 0 failures.
- [x] `check_qml_names.py`, `check_qml_members.sh` green; their own tests
      (`tst_check_qml_names.py`, `tst_check_bindings.sh`) green.
- [x] `docs/PLAN.md` §10's stale `qml`-job claim replaced with a measured one —
      `tst_core_call.qml` already covers the three things it called untested.
- [x] §10's anti-false-green passage migrated into `design.md`, with PLAN.md
      pointing at it rather than keeping a second copy.
