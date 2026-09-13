# Reading `scaffold.toml`

`scaffold.toml` cannot explain itself. **Every `lgs basecamp` verb rewrites the
file and deletes every comment in it** — `setup`, `install`, and even a plain
`modules --show`, which reads like a query. A warning comment saying so was
itself deleted, twice. So the file holds bare values and this document holds the
reasoning; check `git diff scaffold.toml` after any `lgs basecamp` verb anyway,
because a verb can also change a *value* you did not ask it to.

This is the "why" for entries whose purpose is not visible from the value. For
the current values, read the file. For the two pins `lgs basecamp doctor` warns
about on every run, read [`PHASE0-FINDINGS.md`](PHASE0-FINDINGS.md) §8 — that is
where the decision lives, and repeating it here would be a second copy to drift.

## `[repos.lez]` and `[repos.spel]` are schema furniture, not dependencies

`lgs` refuses a `scaffold.toml` without `[repos.lez]` — *"invalid scaffold.toml:
missing [repos.lez]"* — even for a project that only ever runs `basecamp build`.
The validator is shared with the LEZ zkVM project shape and does not know a
module-only project has no zone to talk to.

So `lez` and `spel` satisfy the schema and are fetched by no verb this project
runs. **Do not read them as dialectica depending on the execution zone** — it
does not; PLAN.md §7.1 keeps LEZ firmly in "later". They are pinned anyway,
because an unpinned entry that nothing fetches is still an entry that would
fetch something unexpected the day a verb does reach it.

`[wallet]`, `[framework]`, `[localnet]` and `[circuits]` are the same: written by
the tooling to satisfy its own shape, not choices this project made.

`[repos.basecamp]` and `[repos.lgpm]` are the real ones — `install` and `launch`
build them.

## The dev/portable split is a pairing, and mixing halves deadlocks `install`

`[repos.basecamp].attr` and `[repos.lgpm].attr` must select the **same side** of
a split that no version string records.

- `app` (basecamp) and `cli` (lgpm) are the **dev** stack, which is what this
  repo uses. A dev basecamp wants `<host>-dev` package variants.
- `bin-bundle-dir-inspector` (basecamp) and `cli-portable` (lgpm) are the
  **portable** stack, which wants bare `<host>` variants.

Mix them and `install` deadlocks: the module builder emits `linux-amd64-dev`,
and a portable lgpm rejects it with

```
Package does not contain variant for platform: linux-x86_64 (package provides: linux-amd64-dev)
```

The trap is that **`nix flake show` cannot tell the two apart**. Both attrs build
the same version — the dev/portable difference is in the build, not the version
string — so nothing short of running `install` reveals a mismatch. A correct dev
pair reports `all linux-amd64-dev variants present ✓` from `launch`.

## `[modules.*]`: what each key is for

Both project modules are adopted by hand rather than generated. `lgs new` cannot
scaffold a module project — its templates are LEZ zkVM projects — so these tables
*are* the whole adoption. A hand-authored table builds in a fresh checkout with
no `.scaffold/` at all; only `install` and `launch` need a prior
`lgs basecamp setup`.

- **The table key must match the module's own `name` in its `metadata.json`.**
  Nothing checks this at write time.
- **`flake` stays relative** (`path:./<dir>#lgx`), resolved against
  `scaffold.toml`'s own directory. `scaffold.toml` is tracked, so each worktree
  has its own, and the cwd `lgs` runs from is what decides which checkout gets
  built. An absolute path would pin every worktree's build to one checkout, and
  it would fail *silently* — a green build of the wrong tree.
- **`role = "dependency"` is what gets a module installed rather than merely
  built.** `delivery_module` is written by `lgs basecamp modules`, not by hand:
  that verb reads `dependencies` out of `dialectica/metadata.json` and captures
  the module satisfying it. `lgs basecamp install` alone builds only the
  `role = "project"` entries and never consults a metadata `dependencies` array,
  so **without a prior `lgs basecamp modules`, delivery is absent at runtime**
  and the core fails to load with `Cannot resolve dependencies for: dialectica`.

  That failure **presents as a launcher tile that does nothing when clicked**,
  while the build stays green and `modules --show` lists the dependency it never
  installed.

## `[basecamp.env]`: two settings that exist to make failure visible

Both are there so a failure has a symptom at all; neither changes behaviour.

- `QT_FORCE_STDERR_LOGGING` — without it Qt's own logging vanishes into the
  platform log sink, and neither `lgs basecamp launch` nor the profile log
  captures it.
- `QT_LOGGING_RULES = "qt.qml.import.debug=true"` — surfaces QML import errors,
  which basecamp does not log itself.

The reason the second matters: **a `ui_qml` plugin that fails to load looks
exactly like one that was never clicked** — and so does a missing or wrongly
sized icon (PLAN.md §8.1), and so does a missing core dependency. Three distinct
causes, one symptom. Read
`.scaffold/basecamp/profiles/<n>/basecamp.log` to tell them apart rather than
guessing which.

## `[basecamp.profiles.*].runtime_dir` must be the session's *real* one

QtRO module sockets live under `XDG_RUNTIME_DIR`, and a unix socket path is
capped at 108 bytes (`sun_path`). Scaffold's default in-profile `xdg-tmp` path
overflows it, and **every module then segfaults** at *"Failed to register module
for remote access"* — basecamp's own bundled ones included. macOS gets a short
`/tmp/lgs-<profile>` by default; Linux keeps the long in-profile path, so each
profile here sets `runtime_dir` explicitly.

**Short is not sufficient — it must be the session's real runtime dir.** Qt finds
the Wayland compositor socket (`wayland-0`) through `XDG_RUNTIME_DIR`, so a
private short directory like `/tmp/lgs-alice` fixes the segfault and then starts
basecamp with no display, and it exits immediately. Two failures, one setting.

The value in the file is this machine's; find yours with `echo $XDG_RUNTIME_DIR`.
