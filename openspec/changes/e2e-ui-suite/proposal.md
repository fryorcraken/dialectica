# Run the join-refusal end-to-end spec green, salvaged from PR #120

## Why

Every UI gate in this repo runs with the Basecamp host absent, and the
`Theme`/`DTheme` collision showed what that costs: the whole visual system went
out with every gate green, because the defect lived in the host's type
registration. PR #120 built a layer that drives the assembled application
through Basecamp's QML inspector with sitometres, and stalled before its one
specification ever passed. This piece salvages that work and gets it green.

It is **not a restart**. The harness, the report adjudicator and its tests, the
schema validator and the reasoning in #120's unarchived `e2e-sitometres` change
are carried forward. What is new is making the run pass against current `main`,
which has moved under it.

## What Changes

**Done means one thing:** the existing join-refusal specification runs green end
to end in CI on this branch, meeting the three conditions #120 set for believing
a green:

1. the report's verdict is a pass;
2. every step in the report passed;
3. the report's step count equals the specification's.

The third is the one that cannot be dropped. Without it, a run that opened the
app, executed nothing and reported an empty sheet satisfies the first two.

Carried over from `origin/piece/e2e-sitometres`:

- the sitometres specification `dialectica-ui/tests/ui/join.yaml`, updated for
  current `main` (below);
- the report adjudicator and its tests, Python there and ported to shell here
  (`dialectica-ui/tests/adjudicate-ui-run.sh`, `tst_adjudicate_ui_run.sh`;
  design.md D12 says why);
- the schema validator `dialectica-ui/tests/validate-ui-specs.mjs`, which parses
  every specification without building a host;
- the expensive workflow `.github/workflows/ui-tests.yml`, which builds a
  Basecamp with the inspector, builds the modules, runs the specification and
  adjudicates the report;
- corrections to the `ci.yml` comments that still argue from the premise that
  dialectica has no e2e harness.

**First, get past the step-1 failure #120 stalled on.** It is diagnosable from
the failing run `gh pr checks 120` reports (Actions run 35304753094). That run
never launched Basecamp. It failed in sitometres' preparation about a second after
starting, with `"dialectica_ui" depends on "delivery_module", which could not be found`.
`lgs basecamp build --variant lgx` had put two artefacts into the app directory,
`01-dialectica.lgx` and `02-dialectica_ui.lgx`, and no `delivery_module`. So the
comment in `join.yaml` claiming that "`scaffold.toml` builds all three into one
APP_DIR" is contradicted by that run's own log. #120's `tasks.md` describes
the earlier runs, which failed after 120s on "could not load the core module".
The last run failed differently and earlier. CLAUDE.md's entry on
`lgs basecamp install` not installing declared `dependencies` covers the same gap
from the install side.

That run also printed "no basecamp log was found". That was expected, because
Basecamp never started. So #120's `--user-dir` fix for keeping the log has not
yet been shown to work on a run that launched the app.

**Stale against current `main`, and must be updated rather than carried:**

- **The create-refusal steps cannot run.** #120 wrote them before #128 and #155.
  `stoa-navigation-view` now requires a fresh profile to open in the
  **no-key state**. In that state the create affordance is *not instantiated*:
  its title field and its action are absent from the element tree. #120's four
  steps typed into a create field and asserted the refusal, and on a CI profile
  that field does not exist. The no-key state renders the key block and the
  paste section instead, so the paste-and-join path is still reachable.
- **#120's design finding D4 is superseded.** It found `DOnboardingScreen`
  unreachable, so that no path in the assembled app could mint a key. The screen
  is still unmounted, but on purpose now (`machine-identity-scope`), and the
  machine key is created on the Stoa list. Do not carry D4 forward as an open
  finding.
- **#120's view edits are superseded.** `main` already names the controls this
  flow drives, under different names from #120's: `pasteField`, `pasteButton`
  (#120 used `previewButton`), `joinButton`, `createTitleField` (#120 used
  `createField`), `createStoaButton`, `createKeyButton`. `Main.qml` has become a
  five-screen navigator built on `enterOnly`. #120's root-level aliases
  (`stoaCount`, `listReadState`, `joinState` and the rest) are not on `main`, and
  whether the specification needs them is for design.md to decide.
- **The join preview now asks the core for the Stoa's title** (#154). For the
  unverifiable reference the specification pastes, that lookup fails.
  `stoa-navigation-view` requires such a failure to render no title and to leave
  the join action in place. So the refusal path is still reachable, with one more
  core call on the way.
- **`docs/PLAN.md` no longer exists.** #120 edited it and moved its sitometres
  pinning reasoning out of it. Those edits cannot be carried, and their reasoning
  belongs in design.md.

**Open decision, not settled here: CI cost and cadence.** #120 ran the cheap
schema validation in `ci.yml` on every PR, and the expensive run in its own
workflow on `pull_request` from the start (its D10). The expensive run builds
Basecamp from source. Whether that split and that trigger are still right is for
the `dev-writer` to decide and record in design.md's Decisions. It is not a spec
matter and is not assumed here.

The PR carries `Part of #134`, not `Closes #134`. The issue asks for wider
coverage than this piece delivers.

## Out of scope: follow-up coverage

These are for the owner to turn into issues. Each needs something this piece does
not build.

- **A successful join.** It needs a reference whose founding record hashes to
  its address, which means a seeding peer or an out-of-band seeder. The
  `seed-store` example is a candidate source.
- **The feed screen.** It can only be reached through a held Stoa, so it depends
  on the item above, or on the one below.
- **The thread screen** (#97), which is reached from a feed row.
- **Key creation and then Stoa creation.** *Not in the owner's list; added
  here.* #128 and #155 made this newly reachable on a fresh profile: create the
  machine key on the list, and the create affordance appears. It needs no seeder,
  so it may be the cheapest route to the feed.
- **The moderation screen**, which shipped inert (#127). The issue names it as a
  target once it is stable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None.

This is a test-only piece. It adds a way to observe the contracted view against a
running host, and observing a contract differently does not alter it. Every
behaviour `join.yaml` asserts is already a requirement: `stoa-navigation-view`
contracts the no-key state, the paste field, malformed-versus-unjoinable
failures, preview-before-join and the refused join, and `view-navigation`
contracts the screen selection. `.openspec.yaml` declares `skip_specs: true` and
records why.

**#120's `e2e-ui-harness` capability is not carried forward as a capability.**
It contracted the gate rather than dialectica. That is the thing the archived
`core-e2e` change declined to do, and README.md's test-only-piece rule names
integration targets and regression suites as adding no requirement. Its
reasoning is not lost. Its obligations become design.md Decisions: the three
adjudication conditions, `--strict`, the inspector-presence check, the
matched host and module pair, the host pin taken from `scaffold.toml` alone,
evidence kept from a failed run, and a spec list derived from the directory.
The adjudicator's own tests pin the checkable ones.

## Impact

- **New files:** `dialectica-ui/tests/ui/join.yaml`,
  `dialectica-ui/tests/adjudicate-ui-run.sh`,
  `dialectica-ui/tests/tst_adjudicate_ui_run.sh`,
  `dialectica-ui/tests/validate-ui-specs.mjs`, `.github/workflows/ui-tests.yml`.
- **Modified:** `.github/workflows/ci.yml`. It gets the validation job and loses
  the stale "no e2e harness" claims at its header, in the `LGS_VERSION`
  rationale in the `build` job, and in the "Not yet a job" section. The view
  changes only if design.md finds the specification needs a name or a root-level
  handle that `main` lacks.
- **New dependency:** `@paradoxcomputer/sitometres`, pinned to an exact released
  version.
- **Not affected:** `dialectica/` core, the Rust tree, the wire contract and
  every requirement in `openspec/specs/`.
