# Design — the join-refusal spec, run green end to end

## Context

See proposal.md for why this layer exists and what "done" means. PR #120 left
a harness that had never passed: its last CI run (Actions run 35304753094)
failed about a second into sitometres' preparation with `"dialectica_ui"
depends on "delivery_module", which could not be found`, because
`lgs basecamp build` had put `01-dialectica.lgx` and `02-dialectica_ui.lgx`
into the app directory and nothing else. `main` has also moved under the
spec: a fresh profile now opens in the no-key state, which does not
instantiate the create affordance #120's spec typed into.

Two owner instructions shape this piece and are constraints rather than
choices:

- **`lgs` is preferred in CI wherever an `lgs basecamp` verb exists**, as in
  radicle-logos-module; any step that is not an `lgs` verb carries its reason
  here.
- **Nothing touches the owner's own Basecamp configuration or data.** Local
  launches go through `lgs basecamp launch <profile>`. That is why the full
  run is proven in CI and nowhere else (see Risks).

## Goals / Non-Goals

**Goals:** the existing join-refusal spec passes in CI under the three
conditions below; the harness that produces that green can be believed; its
cost on every PR is a decision recorded here.

**Non-Goals:** wider coverage (proposal.md lists it); a local `run-e2e.sh`;
any change to core, the wire contract or a requirement.

## Decisions

### D1 — What a run must prove before its green is believed

A run passes only when **the report's verdict is a pass, every step in the
report passed, and the report's step count equals the spec's.** `--strict` is
passed because sitometres exits 0 on INCONCLUSIVE by design, and even with it
the exit code cannot see a run that opened the app, executed nothing further,
and reported a clean sheet. The third condition is the one that catches that;
without it the first two pass on an empty run.

**Chosen:** a checked-in adjudicator, `dialectica-ui/tests/adjudicate-ui-run.py`,
with its own tests, run on every PR by ci.yml's `ui-specs` job. **Rejected:**
radicle's inline heredoc, which is the same logic but cannot be run without
pushing a branch — both of this repo's QML gate defects shipped through review
for exactly that reason.

**What breaks without the count branch, measured** (last re-run with the
condition replaced by `False and …`, after the "more steps" case landed):
disabling `len(steps) != expected` turns exactly five checks in
`tst_adjudicate_ui_run.py` red — both of the stopped-early case's, both of the
"more steps than the spec" case's added by `tester` (the other direction of the
same inequality: a phantom or double-logged step, not merely a dropped one),
and "reports the count" in the every-condition case — and the stopped-early
fixture then prints `ok: all 2 steps passed`. (#120's design stated a count of
three but attributed two of them to the every-condition case; the
stopped-early/every-condition attribution above is the measured one, and the
script's docstring says it. The count moved from three to five when the tester
added the "more steps" case, which #120 and this piece's first pass had not
tested.)

**A missing report is a failure that says nothing was proved.** None of the
three conditions covers this case, so it is a chosen behaviour, marked
`NO SPEC:` in `tst_adjudicate_ui_run.py`. sitometres writes the report from a
`finally`, so the report is missing only when the process never reached its
exit: a job timeout or an OOM kill. The adjudicator exits 1 and says that,
instead of crashing on a missing-file traceback. Both outcomes fail the step.
The difference is the diagnosis: "nothing was proved (job timeout?)" names the
likely cause, and a missing-file traceback reads as a bug in the adjudicator.
**Rejected:** exit 0 with a warning, because an absent report is the absence
of evidence, which is the one thing this script exists not to pass. **What
breaks without the guard, measured:** replacing the `os.path.exists` check with
`False and …` turns exactly two checks red, "names the real cause" and "is
not a traceback". "exit 1" stays green, because the traceback exits 1 too, so
the message is all the guard adds.

`set -o pipefail` precedes the `| tee`, or the step's status is `tee`'s, which
is always 0.

### D2 — lgs builds and installs; sitometres drives

This is the fix for #120's step-1 failure, and the largest change from #120.

**Chosen:** `lgs basecamp setup`, then `lgs basecamp install`, then sitometres
with `--app-dir` and `--user-dir` both set to the profile's module root (from
`lgs basecamp paths alice --json`) and `--basecamp` set to the binary `setup`
built.

- `install` builds every `[modules.*]` entry, dependencies first, and installs
  the results into the seeded profiles with lgpm. `delivery_module` is
  `role = "dependency"`, which `install` builds and `lgs basecamp build` does
  not (`build --module delivery_module` refuses: *"no project module
  `delivery_module`"*). That difference is the whole of #120's step-1 failure.
  The pin it builds is `scaffold.toml`'s own, so no second copy of it exists.
- sitometres discovers all three modules under the module root
  (`plugins/<name>/`, `modules/<name>/` — the layout lgpm installs and
  sitometres reads), and because each one's source already is its staging
  destination it copies nothing (`stageUserDir`'s in-place branch). Basecamp
  runs against exactly what lgs installed, which is also how a user's Basecamp
  receives modules — closer to the real thing than sitometres unpacking an
  `.lgx` itself.

**Rejected alternatives:**

- **#120's `lgs basecamp build` plus sitometres staging from its output.**
  Two artefacts of the three the app needs; the failure above.
- **`nix build` of `[modules.delivery_module].flake` into the app directory.**
  It works — measured locally, with a dev-variant delivery `.lgx` beside the two
  lgs-built ones the app opened and the core answered `list_stoas` and
  `get_master_key` — but it hand-assembles an app directory where an `lgs` verb
  exists, which the owner's instruction rules out.
- **Hand-authoring a CI-only profile, or editing `scaffold.toml` in the job,**
  to make `lgs basecamp build` treat delivery as a project module. It would make
  the run exercise a module set the repository does not declare.

**`with: [dialectica, delivery_module]` stays in the spec, and #120's reasoning
for it was wrong.** #120's D7 said naming `dialectica` was what made the core
staged, citing sitometres' hint *"Stage it with --with dialectica"*. sitometres
already resolves the app's own declared dependencies (`boot()` builds `wanted`
from the app, `app.manifest.dependencies` and `with`, at
paradoxcomputer/sitometres `ab6b3ea`, `src/session.ts`); it does NOT walk a
second hop, which is why `delivery_module` must be named. And that hint is not a
diagnosis: measured locally, it appears whenever the core fails to load, and
Basecamp's own log named the real cause — `Missing dependencies detected:
delivery_module`, `Cannot resolve dependencies for: dialectica`. Naming
`delivery_module` is what makes a missing one fail in the first second by name
instead of timing out at step 1 after 120s. `dialectica` is kept for
readability; it changes nothing.

**Three causes, one presentation**, carried from #120 because it is the most
useful thing to know when step 1 fails: a missing core, a missing or
unloadable `delivery_module`, and a dev/portable variant mismatch all present
as "the app never opens". Read Basecamp's log, which the workflow uploads on
failure, not sitometres' hint.

### D3 — sitometres launches Basecamp, not `lgs basecamp launch`

The one CI step that stays outside `lgs` where a verb exists, and why.

**Chosen:** sitometres spawns the `setup`-built Basecamp itself. **Rejected:**
`lgs basecamp launch alice` with `sitometres run --attach`. sitometres reads
Basecamp's stdout live only for a process it owns. Attached, it can read only
log FILES, which Basecamp flushes on rotation or exit, so it treats the log as
unusable, and the default per-step "no new QML errors in dialectica_ui" check
is suppressed. That check is the one that sees a binding broken only inside the
host, which is the defect class this layer exists for (the `Theme`/`DTheme`
collision). An attached run also has no manifest, so sitometres locates the
app's QML root by guessing at the dock's single QML type instead of by the
declared `view`. Radicle launches the same way.

`launch` would also scrub the profile and replay the module set, which is
redundant after `install`, and it would take `XDG_RUNTIME_DIR` from
`[basecamp.profiles.alice].runtime_dir`. That value is the owner's machine's
`/run/user/1000` (docs/SCAFFOLD.md), and it is not the runner's.

### D4 — Basecamp comes from `lgs basecamp setup`, where radicle uses `nix build`

Radicle builds `#app` with a raw `nix build`, because `lgs` has no verb that
builds a Basecamp without `setup`'s profile seeding and radicle had no use for
the profiles. Here `install` needs `setup`'s lgpm and profiles, so they are
used rather than incidental, and the owner's instruction decides it.
`[repos.basecamp].attr = "app"` is the dev build, whose inspector is compiled
in, so v0.3.1's `setup`, which has no `--inspector` flag, needs none.

The cost is a clone of `logos-basecamp` at the pin and an lgpm build, both on
top of what #120 paid.

**Two guards come with it:**

- **`lgs left scaffold.toml's values alone`.** Both verbs rewrite the file
  (CLAUDE.md). The stripped comment does not matter on a runner. A changed
  VALUE would matter, because the run would then exercise a Basecamp or module
  set the repository does not declare. So the job snapshots `tomlq -S .
  scaffold.toml` before any `lgs basecamp` verb and diffs it after. #120
  compared text with `git diff`, which `setup` would fail on every run by
  stripping the header comment.
- **The inspector-presence check.** It greps the sibling `.LogosBasecamp` ELF
  for the needle sitometres uses. It is carried from #120 and radicle, where
  the shipping `#bin-bundle-dir` is the control that proves the probe
  discriminates. Without it, a future Basecamp that turns the inspector off
  for `#app` fails as sitometres refusing the binary, or not at all.

`basecamp_bin` is read from `.scaffold/state/basecamp.state`, the key=value
file `setup` writes and `install` reads. No verb prints it; `lgs basecamp paths`
prints profile paths only. If a later scaffold adds a verb for it, the step
should use that verb instead.

### D5 — The spec, updated for the no-key state

- **The create steps are gone.** A fresh profile holds no machine key, and
  `stoa-navigation-view` requires that state not to instantiate the create
  affordance. #120's four create steps typed into a field that must not exist.
- **The no-key state is asserted from the element tree:**
  `text: {objectName: createKeyButton}` together with `not_text:` on
  `createTitleField` and `createStoaButton`, in one `wait_for`. The positive
  half is what makes the negative half evidence. A lone `not_text` passes
  against an empty tree, a screen that never rendered, or a scope that matched
  nothing.
- **"The refused join enrolled nobody" is no longer asserted by counting rows,
  and neither is "previewing joined nothing".** The list re-reads membership
  only on creation and after a successful create or join, so after a preview
  or a refused join nothing re-reads it. `stoaCount === 0` there cannot move
  whatever the core did, and it passes against a core that enrolled the peer
  anyway. What can fail after the preview is `joinState === 'previewing'`,
  because a preview that joined would read `failed`, since the core refuses
  this reference. That is the assertion kept.
- **#120's D4 is superseded**, not carried. `DOnboardingScreen` is unmounted
  on purpose (`machine-identity-scope`), and the machine key is created on the
  Stoa list.

The spec is 14 steps: `node dialectica-ui/tests/validate-ui-specs.mjs` reports
the count, and the adjudicator compares against it.

### D6 — Root handles for the suite, on `Main.qml`

`state:` expressions are evaluated against the app's QML root. The spec reads
five `readonly` projections that `Main.qml` now exposes: `listReadState`,
`stoaCount`, `pasteFailure`, `joinState` and `joinFailure`. Each one binds to
state a screen already owns. None is a copy.

**Rejected:** addressing the screens by id from the expression
(`list.readState`). An id is private to its file by QML's rules. Whether the
inspector's evaluation context resolves ids is not documented by sitometres,
whose README says to expose a root property. And renaming an id is not treated
as an interface change by anyone. **Rejected:** #120's `createState` and
`createFailure` aliases, because there is no create affordance to alias in the
state the suite runs in. **Deliberately absent:** any key or identity handle.
`Main.qml` records that the navigator must never come to depend on one, and the
suite reads the key state from the element tree instead (D5).

**What breaks without them:** every `state:` step but `screenShown`. Their
projection is pinned by `tst_e2e_handles.qml`. Measured: binding
`listReadState`, `stoaCount`, `pasteFailure`, `joinState` or `joinFailure` to a
constant, or `joinFailure` to a merely non-empty value, turns that handle's
test red. The fixtures are input-dependent for that reason: two rows, a failing
listing, and a refusal compared by text.

**`stoaCount` needs a failed RELOAD, not a failed first read.** It must read
`visibleRows`, the guarded projection, and not `lastListing`, which keeps the
previous good page across a failed reload on purpose. On a first read that
fails, both are empty, so a fixture built only on that case cannot tell them
apart. Measured by the correctness review and again here: with `stoaCount`
bound to `list.lastListing.length`, every test the file had before
`test_a_failed_reload_does_not_count_the_listing_it_kept` stayed green, and
that test alone goes red (`Actual 2, Expected 0`). It also asserts
`lastListing` still holds two rows after the failure. That is the precondition
that makes the case discriminate, so if the screen ever blanks its kept
listing, the test fails there instead of going quietly blind.

### D7 — CI cost and cadence: the split stays, and the expensive job runs on PRs

**Chosen:** keep #120's split. The cheap `ui-specs` job in ci.yml (the spec
schema plus the adjudicator's tests, in seconds, needing no host) runs on every
PR. The expensive `ui-tests.yml` runs on `pull_request`, on pushes to `main`,
and on demand, with no path filter.

- **The split** exists because the two halves fail for different reasons at
  very different prices. A malformed spec is a seconds-long finding, and it
  should not wait behind a Basecamp build.
- **On PRs, not post-merge only** (#120's D10). The defect this layer exists
  for is introduced by a PR, and a post-merge signal arrives after the damage.
  Radicle started push-only on a cost assumption that measured false (3m14s
  warm, 8m43s cold, because the cold cost is mostly the closure download, which
  the store cache removes) and moved to `pull_request`. #120's last run reached
  the spec about 6.5 minutes after the job started, Basecamp build included.
- **No path filter.** What can break this run is the view, the core, either
  flake lock, `scaffold.toml`'s pins and the workflow itself. A path list
  enumerating those is a hand-maintained list, and it goes stale by excluding
  the one path nobody thought of.
- **Known cold-price cases:** a PR from a fork gets its own cache scope, and
  the first run after a `[repos.basecamp].pin` bump is cold by construction,
  since the rev is in the cache key. The key also hashes `scaffold.toml`, so an
  lgpm or delivery pin bump gets a fresh key while still restoring from the
  same rev's prefix.

**Measured on this branch**, same commit, cold and then warm (Actions run
36090846720, attempts 1 and 2):

|                         | cold (no store cache) | warm (cache restored) |
|---|---|---|
| whole job               | 9m19s                 | 3m16s                 |
| `lgs basecamp setup`    | 4m31s                 | 59s                   |
| `lgs basecamp install`  | 3m37s                 | 43s                   |
| the 14 spec steps       | 9.7s                  | 9.6s                  |

The warm figure matches radicle's (3m14s). Moving Basecamp's build from a raw
`nix build` to `setup` (D4) did not change the warm cost in any way that
matters. A ci.yml run on the same commit took 7m16s for `Build LGX`, so on a
warm cache this job is not what a PR waits on.

### D8 — Pins, and the spec list

- **sitometres is `@paradoxcomputer/sitometres@0.1.2`**, a released version
  and never a range. The same version validates in `ui-specs` and runs in
  `ui-tests.yml`, so the schema checked is the one enforced. Radicle once pinned
  a fork commit to work around an inspector probe that missed a bundle's
  `.LogosBasecamp.elf` (paradoxcomputer/sitometres#1). The dev `#app` ships
  `bin/.LogosBasecamp`, the spelling the released probe finds, so that bug is
  not on this path. (This was PLAN.md's paragraph. PLAN.md is gone and this is
  now the only copy.)
- **`lgs` is v0.3.1** from crates.io, in step with ci.yml's `build` job.
- **The two shared pins are two literals kept equal by a check.**
  sitometres appears in ci.yml's `ui-specs` job and in ui-tests.yml, and lgs
  appears in ci.yml's `build` job and in ui-tests.yml. Each is a job-level
  `env:` value. `dialectica-ui/tests/tst_ui_tool_pins.py`, run by `ui-specs`,
  fails when the two copies differ, when either is not an exact version, when
  either is missing, or when a `run:` body writes a version itself. That last
  case matters because a literal in a `run:` body would bypass the `env:` value
  the check compares. Before this, only a comment kept them in step. If the
  sitometres copies drift, `ui-specs` validates against a schema the run does
  not enforce, and both jobs stay green.
  **Rejected:** a single file both workflows load into `$GITHUB_ENV`. It moves
  the version out of the job that uses it, and it adds a load step to each job
  whose correctness only a CI run can show. ui-tests.yml also reads
  `env.LGS_VERSION` in a cache key, which would then work only for steps after
  the load, an ordering trap. **Rejected:** repository variables (`vars.*`),
  which live outside the repo, so a bump appears in no diff. **Rejected:**
  ci.yml reading ui-tests.yml's value at run time, which is proven only in CI
  and ties one workflow's execution to the other's layout.
  **What breaks without it, measured:** changing ui-tests.yml's `SITOMETRES`
  to `0.1.3` turns the check's "the workflows as committed" case red, and so
  does writing `@paradoxcomputer/sitometres@0.1.3` back into the `ui-specs`
  `run:` body. Its other six cases each start from the real pair with exactly
  one change applied, so the check's power to fail is itself tested.
- **The spec validator's `yaml` is `yaml@2.9.0`**, the version sitometres' own
  `package-lock.json` resolves (paradoxcomputer/sitometres `2fba210`). Left
  unversioned, npm would install whatever `latest` was on the day.
- **PyYAML comes from the runner image or Ubuntu's archive, never PyPI.** An
  `import yaml || pip install pyyaml` fallback is what this rules out, because
  the fallback fetches whatever PyPI serves on the day. ui-tests.yml names
  `python3-yaml` in its apt transaction. `ui-specs` relies on the
  image and fails on the import, by name, if the image ever drops it. That
  failure is wanted: the alternative is a silent fetch. The evidence that the
  image supplies it is inferred from timing, not logged, because pip ran with
  `--quiet`. On Actions run 36091870189 (the `UI spec validation` job), the
  step running `import yaml || pip install` finished all its adjudicator runs
  0.3s after it started. That is too fast for a PyPI download, so the import
  must have succeeded. **Rejected:** `actions/setup-python` plus
  `pip install pyyaml==6.0.2`. It pins a version, but it adds an action and a
  network fetch where no fetch is needed. The package index is also a second
  trust root, beside the image the job already trusts.
- **The spec list is derived from the directory.** The validator globs
  `tests/ui/*.y{a,}ml` and fails on zero files. The matrix in ui-tests.yml is
  the one hand-kept list, and each job asserts that the directory holds exactly
  as many specs as the matrix has jobs. A spec outside the matrix runs nowhere
  and is indistinguishable from a spec that passes, which is the state radicle
  sat in with three specs.

### D9 — Isolation and evidence

sitometres' throwaway `$HOME` is kept and `--real-home` is not passed, so the
app sees no credential of the runner's. The user-dir is the profile's module
root, which is persistent rather than throwaway, so Basecamp's log survives a
failing run. A throwaway user-dir is reaped on the clean exit a failing spec
produces, which is why #120's first two runs had no log. #120's `--user-dir`
fix was never exercised on a run that launched the app. This one reads the
log's directory from `lgs basecamp paths` (`app_logs_dir`) and uploads it on
failure, and it warns when it finds none rather than uploading an empty
directory.

### D10 — `calls:` is not asserted, and #120's reason for that was wrong

#120's D9 said the QML-to-core hop is not logged, so `calls:` would see
nothing. On this Basecamp that is false: the local run printed `calls:
dialectica.list_stoas, dialectica.get_master_key` on step 1. `calls:` is still
not asserted, for a different reason: every requirement this spec observes is
about an effect, and asserting that a call happened would pass against a view
that made the call and rendered the wrong result.

## Risks / Trade-offs

- **The full run is proven only in CI.** Locally, sitometres launches Basecamp
  itself, which conflicts with the owner's rule that local launches go through
  `lgs basecamp launch`. Before that rule was given, three local runs were made
  against a sitometres throwaway or a `./tmp/` user-dir. They established D2's
  diagnosis and nothing else was run. → CI is the proof. tasks.md says so,
  and no box claims a local green.
- **On a machine with a Basecamp install, sitometres can stage the wrong copy
  of a dependency.** `collectWithDependencies` also scans
  `~/.local/share/Logos/LogosBasecamp[Dev]`, and it prefers an installed copy
  of equal version with a newer mtime. A nix-store copy's mtime is the epoch,
  so it always loses. Measured locally: the run staged an installed
  portable-variant `delivery_module`, and Basecamp refused it as `installed for
  variant 'linux-amd64' which is not supported`. A runner has no such install.
  → It matters to anyone who later writes a local runner. It is upstream
  behaviour, not filed from here.
- **`setup` pays for an lgpm build and a Basecamp clone on every run.** → The
  store cache covers the nix half. Warm, `setup` took 59s (D7).
- **sitometres' own dependencies are resolved at run time.** Both jobs pin
  sitometres and ui-specs pins `yaml`, but without a lockfile, anything
  sitometres depends on resolves within sitometres' declared ranges on the day.
  `npx --yes` in ui-tests.yml does the same. Closing that needs a committed
  `package-lock.json`, and generating one needs an `npm install`, which the
  owner's rule keeps out of local hands. → Accepted for this piece. The
  exposure is bounded because neither workflow has a secret (security
  review). A lockfile is the fix if that changes.
- **`basecamp.state` is scaffold's internal file.** A scaffold release could
  rename it. → The locate step fails by name if `basecamp_bin` is not an
  executable, and the `lgs` pin is exact.

- **The paste steps do not type.** In CI, sitometres reported for both `type:`
  steps *"key events did not reach it; assigned the property instead, so
  onTextEdited did not fire"*. So the spec proves that `pasteField`'s `text`
  reaches `DStoaListScreen.pasted` through `onTextChanged`, and the preview and
  join that follow. It does not prove that a keystroke lands in the field. If
  the field ever moves to `onTextEdited`, step 9's assignment would no longer
  reach `pasted`, and step 11 would fail. That is the right way round: the run
  would go red rather than stay green on a field nobody can fill. → Accepted
  for this piece. Real keystrokes offscreen inside the host are sitometres'
  problem to solve, not this view's.

## What this layer structurally cannot see

Carried from #120 and radicle's `docs/e2e.md`, so that a later author does not
try to cover one of these and quietly fail to:

- **Window geometry.** The run is offscreen and no step or flag sets a size.
  `tst_screen_frame_geometry.qml` is that layer here.
- **The system clipboard.** It does not function offscreen inside the host, so
  the copy affordances stay covered at the component layer.
- **What the core did beyond what the view re-reads.** See D5: after a refused
  join, nothing on screen reflects the core's membership until something
  reloads the list.
