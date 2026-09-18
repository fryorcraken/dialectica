# Design — an end-to-end UI suite driven through the QML inspector

## What this builds

A sitometres suite under `dialectica-ui/tests/ui/`, a workflow that runs it
against a real Basecamp, and a cheap schema job in `ci.yml` that parses every
spec without building anything.

The reference implementation is radicle-logos-module's, and **matching it was a
deliberate goal rather than an accident of copying**: the owner's question was
whether dialectica uses sitometres the way radicle does. It does. Where this
repo diverges, the divergence and its reason are recorded below under D7–D10;
an undocumented divergence would be the defect.

## Decisions

### D1 — The dev `#app` host with dev `.#lgx` modules, and no `--variant`

**Chosen:** build `github:logos-co/logos-basecamp/<pin>#app`, build modules with
`lgs basecamp build --variant lgx`, and pass sitometres no `--variant` at all.

**Rejected:** `#bin-bundle-dir-inspector` with `--variant linux-amd64`, which is
what the runner's original brief named.

That shape is radicle's **former** one and is superseded.
`ui-tests.yml:381-424` records the move and the three things it bought, and the
reasoning transfers to this repo unchanged:

- the inspector is a compile-time feature that is off in the *shipping* outputs
  (`appDistributed` passes `enableInspector = false`), **not** off in the dev
  build — `#app` has it;
- sitometres' `hostVariant()` already defaults to `linux-amd64-dev`, which is
  exactly what `.#lgx` produces, so the override that the portable bundle
  demanded would now *break* the pairing rather than establish it;
- it removes `lgs basecamp setup` from the job entirely, which matters here for
  a reason radicle does not have: CLAUDE.md records that **any** `lgs basecamp`
  verb rewrites `scaffold.toml` and strips every comment. A workflow that called
  `setup` would be a workflow that silently deleted this repo's most
  trap-carrying file on every run.

**What breaks without the pairing:** the host declines to load the module with a
single `not supported on this platform` warning, the interface then renders
nothing, and the run fails by timing out on its first step with nothing in
sitometres' own output naming the cause. It is symmetrical — either half alone
reproduces it — which is why the two build steps sit next to each other with one
comment covering both rather than being described separately.

### D2 — `LGS_VERSION: "0.3.1"` survives, but one clause of its reasoning does not

`ci.yml`'s `build` job justifies the released pin with two arguments. The first
is now false:

> `--inspector` builds a Basecamp with the QML inspector compiled in, which
> sitometres needs to drive the UI. **Dialectica has no e2e harness, so no job
> here calls `setup` at all.**

Dialectica now has an e2e harness. **The pin survives anyway, and for a better
reason than the one that expired**: under D1 no job calls `setup` either, so the
`--inspector` flag is not wanted by anything. The premise changed from "we have
no harness" to "the harness does not need that flag", and the conclusion is
unchanged.

The clause is corrected in place rather than deleted, because the second
argument — that `build --print-output` is inert on this path — is untouched and
still carries the pin on its own.

### D2a — Sitometres is pinned to a released version, not a git commit

`@paradoxcomputer/sitometres@0.1.2`, which `npm view` reports as the current
`latest`. Pin the **version**, never a range or `latest`: a moving ref would
silently change the tool that gates every spec here.

**Radicle pins a fork commit, and that is a workaround rather than a pattern to
copy.** It exists for an inspector-probe bug — released sitometres refused the
*bundle* with "no Basecamp with the QML inspector compiled in" even though the
inspector was there, because the probe looked for `bin/.LogosBasecamp` while
nix's `dirBundler` ships `bin/.LogosBasecamp.elf` for a bundle
(paradoxcomputer/sitometres#1). **The dev `#app` chosen in D1 ships
`bin/.LogosBasecamp`**, the spelling the released probe has always looked for,
so that bug is not on this path at all. Radicle itself has since moved back to
a plain version for the same reason.

*(This paragraph was PLAN.md's, and is moved here rather than copied: two copies
drift and the wrong one gets read.)*

### D3 — One specification, covering the join flow, because it is the only flow a
bare runner can complete

This is the largest divergence from radicle and the one most worth reading.

**Radicle's specs talk to a real seed node over the network.** Dialectica's core
does not need one for the flow chosen here, and that asymmetry decides the
suite's shape. Measured against `wire.rs` and `rust-lib/src/lib.rs`:

| call | needs | reachable on a fresh CI profile |
|---|---|---|
| `list_stoas` | the membership store only | yes |
| `join_stoa` | the membership store only | yes |
| `create_stoa` | **a keystore key**, via `creator_key_in` | **no** |

`create_stoa` resolves its creator through `keystore::creator_key_in(&dir)`,
which returns `KeystoreError::NotFound` — *"no keystore found; create one before
posting"* — when the profile has no keystore. A fresh runner has none.

So the covered flow is **paste a reference → preview it → join → it appears in
the listing**, which needs neither a key nor a network, and the create path is
covered by asserting its *failure is presented*, which is what the spec's
"failure text is asserted by presence, not by wording" requirement asks for
anyway.

### D4 — The keystore finding: onboarding is not merely unreachable, it is the
only identity-minting path

The brief asked me to verify `DOnboardingScreen` is unreachable before relying
on it. It is, and the consequence is larger than dead code.

Measured: `DOnboardingScreen` and `DStatusBar` appear in
`dialectica-ui/src/qml/qmldir` and in no `.qml` file outside it. `Main.qml` is a
three-screen navigator whose `screenShown` resolves to `"list"`, `"join"` or
`"feed"`, with no `StackView` and no fourth branch.

`generateIdentitySlate` and `keepIdentity` are declared on the `Core` singleton
and called from exactly one file — `DOnboardingScreen.qml`. **So the assembled
application has no path that creates a keystore**, and therefore a fresh install
can never create a Stoa: the create button is always offered (deliberately, per
`DStoaListScreen`'s comment, because the posting probe has no Stoa to ask about
at creation time) and always fails.

This is precisely the class of defect the proposal argues an e2e run exists to
catch, arriving on the first run: `tst_onboarding_states.qml` runs 50 test
functions against that screen, all passing, against a screen the application
never mounts. A component suite cannot distinguish a screen that works from a
screen that works and is unreachable.

**Not fixed here.** Mounting onboarding is a view change with its own contract
questions — when it shows, what dismisses it, what happens to a peer that
already has a key — and bundling it would make this piece unreviewable. It is
recorded as a finding for a later piece. What this piece does is make it
*observable*: the create-failure assertion below fails the moment onboarding is
mounted and the keystore starts existing, which is the right way round — the
suite notices the day the finding is addressed.

### D5 — Stable names go on the element that receives the interaction

The spec requires a driven element to carry its own stable name, on the element
receiving the interaction rather than an ancestor. Two shapes in this view need
care, and they need *different* care:

- **`TextInput` inside a wrapper `Rectangle`.** `createField` and `pasteField`
  carry an `id` only, which is private to the component and invisible from
  outside. The `objectName` goes on the **`TextInput`**, not on the wrapping
  `Rectangle`: sitometres resolves a typing target by requiring the resolved
  node itself to be an editable type, so a name on the Rectangle would resolve
  to the wrapper and fail to accept text.
- **`FlatButton`.** It is a `Rectangle` containing a `Text` and an unnamed
  `MouseArea`. An `objectName` on a `FlatButton` instance sits on the Rectangle.
  Radicle's `docs/e2e.md` records the matching trap from the other direction —
  *"naming a Repeater delegate matched one element instead of four, because the
  delegate Item is not clickable — its MouseArea is"*. Here the name is still
  placed on the `FlatButton` instance, because sitometres walks from a matched
  node to the nearest enclosing container that holds a mouse handler; the
  button's own `MouseArea` is that handler. **What must not happen is naming a
  layout that contains several buttons**, which would let the walk pick a
  sibling's handler.

Names added, each on a control a covered step drives: the two text fields, the
"Look at it first" preview button, and the per-row "Open" button.

### D6 — Root-level read-only aliases for assertions

Radicle's specs assert against `root.repoCount`, `root.navView`,
`root.composerItem.body` — properties its view root exposes for exactly this
purpose. Dialectica's `Main.qml` root exposes `screenShown`, `chosen` and
`previewing`, which covers navigation but says nothing about what the list holds.

`state:` expressions evaluate against the app's QML root, so an assertion about
the listing needs a root-level handle. Added as `readonly` aliases that compute
from what is already there rather than as new state — a second writable copy of
the listing is exactly the drift `visibleRows` was reshaped to prevent.

### D7 — Divergence: no `with:` list

Radicle's `browse.yaml` declares `with: [radicle]` because two modules must be
findable from one `--app-dir` search root. Dialectica's UI declares one
dependency (`dialectica`), and `lgs basecamp build` collects both into
`.scaffold/basecamp/lgx` the same way, so the single `--app-dir` is enough and
`with:` adds nothing. Recorded because its *absence* would otherwise read as an
oversight against the reference.

### D8 — Divergence: no seeded profile, and therefore no `--env`

Radicle's `write` and `local` specs need `RAD_HOME` pointing at a profile seeded
by a Rust example, because half its module reads `~/.radicle`. Dialectica's
covered flow writes only to the module's own storage dir under sitometres'
throwaway `$HOME`, so there is nothing to seed and nothing to point at. The
isolation requirement is satisfied **by construction**: sitometres gives every
run a throwaway `$HOME` and the spec's writes cannot reach outside it.

That is worth stating precisely rather than ticking: the requirement holds
because no mechanism exists by which the run could touch a person's profile, not
because a cleanup step removes what it wrote.

### D9 — Divergence: `calls:` is not asserted, for the same reason radicle does
not assert it

Radicle's `browse.yaml` carries a long note that QML dispatches through a QtRO
replica and that hop is not logged as a `LogosAPIClient` invocation, so
sitometres reports "no backend calls at all" while the data plainly arrives.
Dialectica's view reaches core the same way. So assertions are on effect via
`state:`, never on `calls:` — which is also what the spec's "assert the effect a
call produced rather than that the call happened" requirement requires.

### D10 — Divergence: the run is on `pull_request` from the start

Radicle's workflow was push-only on an assumption that measured false, and moved
to `pull_request` afterwards. Dialectica's covered flow needs no seed node and
no network for its assertions, so its variance is lower than radicle's from the
outset, and the expensive part — building Basecamp — is identical and equally
cacheable. Starting where radicle ended up avoids re-running an experiment whose
answer is written down.

### D11 — The three gate conditions, and why the third carries the weight

`--strict` is required because **sitometres exits 0 on INCONCLUSIVE by design**
(`docs/e2e.md:25`). A harness that reports green without running is worse than no
harness.

`--strict` alone is not enough, which is why the adjudication step reads the JSON
report rather than the exit code. Three conditions, all required:

1. the report's verdict is a pass;
2. every step in the report is a pass;
3. the step count in the report equals the step count in the spec.

**The third is the one that cannot be dropped.** Without it a run that started
the application, executed nothing further and reported an empty sheet satisfies
the first two and reads as a pass.

**Measured, not asserted.** Disabling the `len(steps) != expected` branch in
`adjudicate-ui-run.py` turns exactly three checks in
`tst_adjudicate_ui_run.py` red — the `stopped early` case and the two
count-related assertions in `every failing condition is reported` — while every
other check stays green, and the stopped-early fixture prints
`ok: all 2 steps passed`. That last line is the silent green this guard exists
to prevent, and it is what a reviewer should reproduce before deleting the
branch as redundant.

`set -o pipefail` before the `| tee`, for the reason radicle's comment gives:
without it the step's exit status is `tee`'s, which is always 0, so a failing run
reports as a pass.

### D12 — The schema job is separate from the run, and derives its file list
from the directory

A malformed spec is detectable in seconds without a host or a module, and the
same mistake found after a Basecamp build is the same information arriving much
later. So `ui-specs` lives in `ci.yml` and runs on every PR.

The list of specs it validates is **globbed from the directory**, never
maintained beside it. Radicle's `docs/e2e.md` records what the hand-maintained
alternative costs: `SPEC` was once hardcoded to `browse.yaml` and three specs sat
in the tree running nowhere. A spec outside the list runs nowhere, passes
nothing, and is indistinguishable from a spec that passes.

## What this layer structurally cannot see

Carried from `docs/e2e.md` because it applies here unchanged, and stated so that
a later author does not try to cover one of these and quietly fail to:

- **Window geometry.** The run is on the `offscreen` platform; the step
  vocabulary takes no size and no CLI flag sets one. Geometry belongs at the
  component layer, where a fixture owns it — `tst_screen_frame_geometry.qml` is
  already that layer here.
- **The system clipboard.** It does not function offscreen inside the host. This
  repo's `DClipboardSink` is a hidden `TextEdit` rather than a platform API,
  which is why the copy affordances are covered at the component layer and are
  not asserted end to end.
- **Whether a given core call was made** — D9.
