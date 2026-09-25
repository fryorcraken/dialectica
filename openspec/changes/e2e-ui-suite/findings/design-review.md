# Design review — `e2e-ui-suite`

Checked `design.md`'s Decisions against the code (`git diff origin/main...HEAD`),
against `proposal.md`, against `gh issue view 134`, against PR #120
(`gh pr view 120` and `origin/piece/e2e-sitometres:openspec/changes/e2e-sitometres/design.md`),
and against the named model, `radicle-logos-module/.github/workflows/ui-tests.yml`.

Overall the Decisions are in good shape: D1–D10 each map onto a specific place
in the code, the four owner-requested decisions (salvage vs. restart, the CI
cost split, `lgs`-preference in CI, never touching the owner's own Basecamp
config) are all recorded and the code matches what is recorded, and the
`scaffold.toml`-value guard, the inspector-presence check, `delivery_module`'s
`role = "dependency"` and the pin values all check out against the live repo
state. One real gap found, below.

- [ ] **`tester`** — a checked-in guard-test exists but is wired into no CI job,
      unlike every sibling it was modelled on
      `dialectica-ui/tests/tst_scaffold_values_unchanged.py` is a new file (not
      present on `origin/main`, not mentioned in `design.md`) that pins the
      "lgs left scaffold.toml's values alone" guard's own diff mechanism —
      exactly the same rationale D1 gives for `adjudicate-ui-run.py`: "a
      checked-in script with its own tests... both of this repo's QML gate
      defects shipped through review because a heredoc cannot be run without
      pushing." D1 goes on to say `adjudicate-ui-run.py`'s own tests
      (`tst_adjudicate_ui_run.py`) are "run on every PR by ci.yml's `ui-specs`
      job" — and they are (`.github/workflows/ci.yml:1149-1153`, "The run
      adjudicator catches what it claims to").
      `tst_scaffold_values_unchanged.py` gets no equivalent step anywhere.
      **Verified:** `git grep -rn "tst_scaffold_values_unchanged" -- .github/`
      returns nothing — the only references in the whole diff are in
      `tasks.md`'s prose. So the file can silently start failing (e.g. a
      future edit to either named step in `ui-tests.yml` that the test extracts
      by name) and no PR gate would ever notice, which is exactly the
      "heredoc that cannot be run without pushing" failure mode D1 exists to
      close, reopened for this one test. Either wire it into `ui-specs`
      alongside its sibling, or record in `design.md` why this one guard-test
      is deliberately not run in CI (e.g. if it needs something `ui-specs`
      does not have) — right now neither has happened, so the omission reads
      as an oversight rather than a decision.

## The four owner-requested decisions, checked individually

1. **Salvage #120 rather than restart** — recorded in `proposal.md`'s "Why"
   (not `design.md`, but that is the document table's intended split: "why
   this change" belongs in `proposal.md`). It states the choice and the
   itemised carry list under "Carried over from `origin/piece/e2e-sitometres`."
   The issue itself (#134) already leaned this way ("restarting from scratch
   should not be the default without checking what's salvageable first"), so
   the piece is not the first place this was decided — it correctly treats it
   as inherited rather than re-litigating it with fresh alternatives.

2. **CI cost split, and whether it stays** — D7, matched exactly by the code:
   the cheap `ui-specs` job lives in `ci.yml` (spec-schema validation +
   adjudicator self-tests, `ci.yml:1117-1153`), the expensive `spec` job lives
   in its own `ui-tests.yml`, and D7's measured cold/warm table
   (9m19s/3m16s, Actions run 36090846720) matches the `on:` triggers actually
   present in `ui-tests.yml` (`push: [main]`, `pull_request: [main]`,
   `workflow_dispatch`, no `paths:` filter).

3. **`lgs` preferred wherever an `lgs` verb exists, radicle's workflow as the
   model** — `ui-tests.yml`'s header explicitly frames itself as "Modelled on
   radicle-logos-module's ui-tests.yml, with one deliberate move further
   towards `lgs`," and D4 records why: radicle's raw `nix build` of Basecamp
   is not available here because `install` needs `setup`'s lgpm/profiles
   regardless, so `setup` is used instead of a bare `nix build`, and D2
   explains why `install` (not radicle's `build`) is used for the modules —
   `delivery_module` is `role = "dependency"`, which only `install` builds.
   D3 is the one place a non-`lgs` step is kept (sitometres launches Basecamp
   itself rather than `lgs basecamp launch`), and it is justified at length
   (attached-log unusability, the manifest-based QML-root lookup, matching
   radicle's own shape). No step in `ui-tests.yml` reaches for a non-`lgs`
   shape where an `lgs basecamp` verb would do the same job.

4. **Never touching the owner's own Basecamp config/data** — covered by D3
   ("Local launches go through `lgs basecamp launch <profile>`... That is why
   the full run is proven in CI and nowhere else"), the Risks section's first
   bullet, and `tasks.md`'s Implementation preamble ("No local run of the
   updated spec against a launched Basecamp was made"). `ui-tests.yml` only
   ever runs in CI; nothing in the diff invokes `sitometres run` or a raw
   Basecamp binary outside that workflow.

## Decisions vs. code, spot-checked

- D1 (adjudicator + step-count branch): matches `adjudicate-ui-run.py` and its
  wiring in both `ui-tests.yml` ("The run proved what the spec asks") and
  `ci.yml`'s `ui-specs` job.
- D2 (`install` builds all three modules, `delivery_module` is
  `role = "dependency"`): matches `scaffold.toml:27-29` and the "Build and
  install all three modules" step in `ui-tests.yml`.
- D3 (sitometres launches Basecamp itself): matches — the "Run the spec" step
  calls `npx sitometres run`, never `lgs basecamp launch`.
- D4 (Basecamp from `lgs basecamp setup`, guards for it): matches the "Build
  Basecamp and seed the profiles" step, the "lgs left scaffold.toml's values
  alone" step, and the "The build really has the QML inspector" step.
- D5/D6 (spec rewritten for no-key state, root handles): matches
  `join.yaml` and the five `readonly` properties added to `Main.qml`.
- D8 (pins): `LGS_VERSION: "0.3.1"` and `SITOMETRES: "@paradoxcomputer/sitometres@0.1.2"`
  match in both workflow files.

No code found contradicting a recorded decision, and no undocumented
consequential choice found beyond the one filed above.

**Branch:** `worktree-agent-a259a1bc46cc7c5ec` — one commit, ready to
cherry-pick onto `piece/134-e2e-ui-suite`. Tree is ready to prune once picked.
