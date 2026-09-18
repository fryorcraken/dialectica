# Add an end-to-end UI suite driven through the QML inspector

## Why

Dialectica has no end-to-end gate, and the reason it has none has expired.

The absence was deliberate and its re-entry condition was written down in two
places — `.github/workflows/ci.yml`'s "Not yet a job" section and `docs/PLAN.md`'s
"Deliberately not built" list. Both say the same thing: *the view is four buttons,
a matrixed e2e workflow over zero specs would be a job that cannot fail*, and it
**"arrives with the specs."**

The specs have arrived. Measured against this tree rather than quoted from those
comments:

- `openspec list --specs` reports **21 capabilities**. Four of them contract the
  view — `stoa-navigation-view` (17 requirements), `view-identity-onboarding`
  (18), `composer-view` (15) and `feed-view` (3): **53 view requirements** that
  did not exist when the deferral was written.
- `dialectica-ui/src/qml/` holds **23 files** — 22 QML types plus `qmldir` — not
  one. `grep -c "function test_"` over `dialectica-ui/tests/` totals **328** test
  functions across **18** spec files.

So the premise died and the comments did not notice. Correcting them is part of
this change, under CLAUDE.md's "Keeping this file true": a stale comment that
argues for an absence is worse than no comment, because it answers the question
"should we add this?" with a reason that is no longer true.

The second reason is sharper than staleness, and it is what makes an e2e suite
worth its runtime rather than merely affordable. **Every existing UI gate runs
with the host absent.** `qmltestrunner` instantiates components with no Basecamp
around them, and this repo has already paid for what that cannot see: the
`Theme`/`DTheme` collision took out the entire visual system with every gate
green, because the collision lives in the host's C++ type registration and no
component test can observe it. The repo's answer was `check_qml_names.py` — a
**static proxy** for a runtime defect, enforcing a `D` prefix because the real
property could not be measured.

A sitometres run has the host present. It observes the property the proxy stands
in for, directly, against the real application. That is a class of defect this
repo currently has no gate for at all, only a heuristic.

## What Changes

- **A new end-to-end UI capability.** A suite of sitometres specs drives a real
  Basecamp through its QML inspector, exercising the flows the view actually
  offers, and a CI workflow runs them and adjudicates the result.
- **A cheap schema-validation gate, decoupled from the expensive run.** Parsing
  every spec against the sitometres schema costs seconds and catches a malformed
  spec without building a Basecamp. It runs on every PR; the full run is a
  separate workflow.
- **Correction of three stale claims**, each currently arguing from the dead
  premise. `ci.yml`'s header ("dialectica's UI is ONE QML file with four buttons
  and no backend"; "dialectica has no e2e harness yet"), `ci.yml`'s "Not yet a
  job" entry, and `docs/PLAN.md`'s "Deliberately not built" entry. A fourth is
  flagged for the `dev-writer` rather than pre-judged here — see Impact.

- **Stable names added to the controls a covered flow must drive.** Measured,
  the view carries 22 `objectName` values and they sit almost entirely on
  *output* elements. Both text fields on the Stoa list — the create field and the
  paste field — carry an `id` only, which is private to the component and cannot
  be selected from outside it. Those two fields are the entry point to every
  other flow, so naming them is a prerequisite rather than a nicety.

Scope is **MVP**, in the owner's words: a small number of specs that genuinely
run beats a matrix that does not.

**The screens the suite can cover are three, not four.** The brief named the Stoa
list, the join preview, onboarding and the feed. Measured against this tree:
`Main.qml` is a three-screen navigator — `screenShown` resolves to `"list"`,
`"join"` or `"feed"` — and `DOnboardingScreen` appears nowhere outside
`qmldir`. It is declared as a type and instantiated by nothing. `DStatusBar` is
in the same position.

That finding is itself the argument for the change, and it is worth stating
plainly: `tst_onboarding_states.qml` instantiates `DOnboardingScreen {}` directly
and runs 50 test functions against it, all passing, against a screen the
application never mounts. A component suite cannot distinguish a screen that
works from a screen that works and is unreachable. An end-to-end run can only
ever assert what the assembled application presents, which is precisely why it
catches this class.

Onboarding is therefore **out of scope for this suite** — not deferred, but
unreachable, and a specification driving it could never run. Whether it *should*
be reachable is a separate question this change does not answer.

## Capabilities

### New Capabilities

- `e2e-ui-harness`: What an end-to-end run must establish before its result may
  be believed, and what counts as a pass. Covers the adjudication of a sitometres
  report (verdict, per-step outcome, step count), the preconditions that make a
  run meaningful at all (the driven binary really carries the inspector; the
  module variant and the host build are a matched pair), the single-source
  derivation of the pinned Basecamp revision, and the isolation obligation on any
  spec that mutates state.

This is a new capability rather than an addition to an existing one, and the
boundary is deliberate. The four view capabilities own **what the view must
render** — they are contracts on the application. This one owns **what a run of
the harness must prove before its green may be trusted** — a contract on the
gate. `openspec list --specs` shows no existing capability covering test
infrastructure, and folding gate obligations into `stoa-navigation-view` would
put two unrelated subjects behind one name.

### Modified Capabilities

None. No requirement in `openspec/specs/` changes: this change adds a way to
observe the contracted behaviour against a running application, and observing a
contract differently does not alter it. The view capabilities say what the
screens must do whether or not anything drives them.

## Impact

- **New**: a sitometres spec directory under `dialectica-ui/tests/`, and a CI
  workflow that builds a Basecamp with the inspector, builds the modules, runs
  the specs and adjudicates the report.
- **Modified**: `.github/workflows/ci.yml` (the stale header claims, the "Not yet
  a job" entry, and the new cheap validation job) and `docs/PLAN.md` (the
  "Deliberately not built" entry).
- **New dependency**: `@paradoxcomputer/sitometres`, **pinned to its latest
  release**. Radicle pins a git commit only to work around a probe bug in
  published 0.1.0; that is a workaround, not a pattern to copy.
- **Flagged for `design.md`, not decided here.** `ci.yml`'s `build` job pins
  `LGS_VERSION: "0.3.1"` and argues for it partly on the ground that
  *"Dialectica has no e2e harness, so no job here calls `setup` at all."* That
  clause is invalidated by this change, so the pin's reasoning must be re-checked
  even if the pin itself survives. It likely does — see the note below. The
  `dev-writer` should verify and record the outcome rather than inherit either
  answer.

**One inherited lesson is superseded and must not be transcribed.** The banked
advice to build the portable module variant and override the harness's variant
flag describes radicle's **former** shape. Radicle has since moved to the dev host
build, which already carries the inspector, with dev modules and no variant
override at all — its workflow records the three things that simplification
bought, including that it stops calling `setup` (which rewrites `scaffold.toml`
and strips every comment, a trap this repo carries independently).

What survives the supersession is the invariant underneath it, and that is what
the spec contracts: **the host build and the module build must be a matched
pair**, and a mismatch presents as the interface rendering nothing and the run
timing out on its first step, with nothing in the harness's output naming the
cause. Either half alone reproduces it. The spec states the pairing; which pair
to use is the `dev-writer`'s to decide and record.
- **Not affected**: `dialectica/` core, the Rust tree, and the wire contract.
  Nothing in this change alters shipped behaviour.
