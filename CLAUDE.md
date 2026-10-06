# Working on this repository

## Where to look for what

This file is the always-relevant part: what dialectica is, the traps that have
bitten changes here, and — in the specflow block at the end — how to work in it
without costing the user a click per command.

As the project grows, move trigger-specific material into `docs/` and link it
from a table here, so that this file stays the thing worth reading in full:

| Read | When |
|---|---|
| [`docs/SCAFFOLD.md`](docs/SCAFFOLD.md) | **Before changing a value in `scaffold.toml`**, or when a build, `install` or `launch` misbehaves. Every entry whose purpose is not visible from its value — why two `[repos.*]` tables exist for a zone we do not use, which pairs of `attr` values deadlock `install`, and what the settings under `[basecamp.env]` and `[basecamp.profiles.*]` are each preventing. It lives here because `lgs` deletes every comment in that file. |
| [`docs/SOURCES.md`](docs/SOURCES.md) | **When a claim about the surrounding Logos ecosystem needs re-checking against source** — which local checkout has the delivery API, the SDS spec, the LEZ private-account construction, and which of those checkouts are stale working trees that will mislead if read directly. |

The specflow block at the end carries its own table, for the flow, the runner
and the project overlay.

## What this is

**Dialectica is a decentralized forum built on the Logos stack.** The name is
Greek — dialectic, reasoned argument between positions — and the logo is the
Delta (Δ).

Its organising concept is the **Stoa**: a sub-forum that anyone may create and
moderate. The two halves of that are deliberately in tension, and the whole
design follows from holding both:

- **Permissionless Stoa creation.** Creating a Stoa needs no approval step and
  no registration with a central service, because there is no global registry
  to register with — a Stoa is a genesis record its creator publishes.
- **Good moderation and curation *within* a Stoa.** A Stoa's moderators can
  and should shape it. A forum where nothing can ever be removed is not a
  forum, it is a firehose.

Whenever a design decision here looks arbitrary, check it against that pair
first — most of them are the pair, applied.

### The core/UI split is forced, not stylistic

Two modules in one repo:

- **`dialectica/`** — the core module. All network, storage, cryptography,
  state and moderation logic.
- **`dialectica-ui/`** — the QML view. Forwards everything to core.

Basecamp sandboxes the QML engine with a deny-all network access manager and no
filesystem access outside the plugin directory. A view therefore *cannot* fetch
or read anything itself. This is a platform constraint, not a preference: work
that touches the network or disk belongs in core, always.

### SDS is transport: use its API as it is, and build what we need above it

**SDS is HTTP or TCP in this stack, and the application layer is ours.** TCP
retransmits, orders within a connection and reports a failed transfer — and it
still cannot tell you your file is half-written, because it does not know what
a complete file is. Only the application does.

So: **SDS repairing what it can see is not a substitute for dialectica knowing
what a complete thread is.** Ordering, recency and causality at forum scope are
dialectica's to build, carried inside the signed op preimage where a relay can
neither forge nor strip them.

The trap this exists to prevent is treating a missing transport field as a
blocker. It is not — an application that can only order its own content while
the transport hands it ordering metadata breaks the moment that transport
changes, and SDS is LIP-109 at *raw*, the weakest maturity tier, with an API
marked Developer Preview. **File the upstream gap; do not wait on it, and do
not design around it.**

`openspec/changes/archive/2026-09-16-op-clock/design.md`'s "The layering rule
this change is a consequence of" works this through, including a claim about
it that was wrong and how the error happened.

### The core API is the deliverable

The core module's API is the part of this project to be most deliberate about.
It is a contract that outlives any particular UI, and widening it is a decision
to make on purpose rather than a side effect of needing one more field.

Conventions, inherited from the Logos module ecosystem:

- Every method takes a `std::string` and returns a `std::string`, both JSON.
- Failure is **always** `{"error":"..."}`. Never a partial success shape.
- Paginated calls take `(page, perPage)` and return
  `{"items":[...],"page":N,"hasMore":bool}`.
- JSON shapes are source-independent, so a view renders without branching on
  where the data came from.

Keeping the wire contract narrow is what keeps dependency churn behind a wall.

## Module contract traps

These are structural and bite at build time, not review time.

- **`interface: "universal"` scans the impl header's `public:` section** to
  derive the RPC dispatch table. A public constructor that takes parameters —
  *even all-defaulted ones* — is scanned as a zero-arg method named after the
  class, and miscompiles. Keep exactly one genuinely parameterless
  constructor; do dependency injection through a private
  `setDependenciesForTest()`.
- **Sibling flake refs must be one line.** `dialectica.url = "path:../dialectica"`
  — scaffold's override parser is line-level and will not see a multi-line
  `inputs.x = { url = …; }` form.
- **`follows` wiring is mandatory.** Any sub-flake pulling in a module that
  itself depends on `logos-module-builder` must add
  `inputs.<dep>.inputs.logos-module-builder.follows = "logos-module-builder";`
  or `flake.lock` gets two builder nodes, the stale one silently wins under
  `--override-input`, and the build fails with `no 'main' field in
  metadata.json`.
- **Dual remotes: Radicle and GitHub, and GitHub is load-bearing.** Radicle is
  the canonical home — a peer-to-peer forum hosted solely on a single
  centralised platform would sit oddly with its own design — but GitHub
  cannot be dropped: Actions runs CI, and the Logos module catalogue is
  hosted on **GitHub Releases**, so `lgpd` and `lgpm` fetch packages from
  there and a module not released on GitHub cannot be installed by the
  standard tooling. Both remotes get every push; releases are cut on GitHub
  tags. Two consequences of being a monorepo — two modules under one git repo,
  where the catalogue expects one module per submodule:
  - **The catalogue's release action must support a `module_path` pointing
    *inside* a submodule.** Older versions fail checkout with "pathspec did
    not match any file(s) known to git" for this layout. Pin
    `_release-module.yml` to a fixed tag rather than a moving one, so the
    release pipeline cannot change under the catalogue without a commit here
    saying so.
  - **`release-all.yml` auto-discovery cannot see two modules in one repo** —
    it reads module paths straight from `.gitmodules` submodule paths.
    `dialectica` and `dialectica_ui` each need their own
    manually-triggered workflow, not the umbrella.
- **On Linux, set `runtime_dir` to the session's real one** (e.g.
  `/run/user/1000`) in `[basecamp.profiles.<n>]`. The in-profile `xdg-tmp`
  default overflows the 108-byte `sun_path` cap and **every module segfaults**
  at "Failed to register module for remote access". Short is not enough — it
  must be the real one, or basecamp starts with no display; `docs/SCAFFOLD.md`
  says why.
- **`lgs basecamp` rewrites `scaffold.toml` and strips every comment — and
  not only on `setup`.** A plain `lgs basecamp modules`, which reads like a
  query, deleted 77 lines of comments. Assume **any** `lgs basecamp` verb
  rewrites the file, and run `git diff scaffold.toml` after every one — a verb
  can change a value too, not just drop a comment.

  **Do not answer this by re-adding comments.** That was the workaround, it
  failed repeatedly, and it cost a permission click per verb to maintain.
  The reasoning lives in `docs/SCAFFOLD.md`, where nothing strips it.

- **`lgs` builds whichever checkout it is run from, worktrees included.**
  `[modules.*]` uses **relative** flake refs (`path:./dialectica#lgx`),
  resolved against `scaffold.toml`'s own directory — and `scaffold.toml` is
  tracked, so each worktree has its own. There is no `--directory` flag and
  none is needed: the cwd decides.

  **Keep those refs relative.** An absolute path would pin every worktree's
  build to one checkout, which is the failure this note exists to prevent —
  and it fails *silently*, with a green build of the wrong tree.

  The awkward part is reaching the worktree at all, since `cd <dir> && lgs …`
  is the shape that costs a permission prompt. Run `lgs` from a shell already
  in the worktree, or accept the one prompt — **do not conclude `lgs` cannot
  target a worktree**, which is the wrong lesson and was drawn once already.
- **The UI's icon must be a 256×256 PNG**, and the UI module must declare core
  in `dependencies` with **matching versions**.

- **`lgs basecamp install` does not install a module's declared
  `dependencies`.** It builds the `[modules.*]` project sources and never reads
  the `dependencies` array in `metadata.json`. `lgs basecamp modules` is the
  verb that captures runtime dependencies; run it first, then `install`. Skip
  it and the module fails to load with `Cannot resolve dependencies for:
  <name>`, which presents as a launcher tile that does nothing when clicked —
  while the build stays green and `modules --show` lists the dependency it
  never installed.
- **`pgrep basecamp` finds nothing while Basecamp is running.** The launcher is
  `.LogosBasecamp.elf` and each module is a separate `.logos_host.elf`, both
  under the dynamic loader. Read the PID from `<profile>/launch.state` instead
  (`lgs basecamp paths <profile>` locates it). Requested as a first-class verb
  in `logos-co/scaffold#268`.
- **Run `lgs basecamp doctor` before believing a green build.** It catches pin
  drift and split basecamp/lgpm pin sets that no build failure surfaces.
  Expect two WARNs on this repo: the basecamp/lgpm split and the delivery pin
  are both deliberate (see `docs/PHASE0-FINDINGS.md` §8), which is also why
  there is no `doctor` CI job — an always-red gate trains people to ignore it.
- **Pin `logos-module-builder` ≥ 0.2.5** — earlier builders deliver empty
  binary event payloads. Assert non-empty payloads in a test.
- **A panic in a dispatch handler aborts the module process, not merely
  poisons a lock.** Measured, not inferred: `failed to initiate panic, error
  5`, SIGABRT, and every later call gets `MODULE_NOT_LOADED` — the caller
  having first waited out a 20s timeout that names nothing
  (`docs/PHASE0-FINDINGS.md` §3). The SDK has no panic guard, so no handler
  may unwind; dialectica's own guard is what stands between those two
  outcomes.
- **`recv()` on an event subscription may block forever** on an older SDK rev
  that lacks subscription status — a dead provider hangs the listener thread
  permanently. Check what the builder's pin delivers before relying on a
  timeout.
- **Handle `RET_STALE_WARN` (3)** from the delivery C ABI: a non-terminal
  "still running" tick every ~5s, always followed by a terminal OK/ERR.
  Ignoring it double-counts completions.
- **`createNode` exactly once per context.** The delivery node is a singleton
  per Logos Core instance; `stop()` kills traffic for every module using it.
  Contracted in the `op-transport` spec; kept here because it presents as a
  runtime failure in someone else's module.
- **`messageReceived`'s timestamp is nanoseconds**; every other delivery event
  is ISO-8601 (delivery bug #26).
- **`messageReceived` fires for your own messages; `channelMessageReceived`
  does not** — own sends come back as `channelMessageSent`. The consequence is
  contracted in the `op-transport` spec ("A peer's own published op is not
  received back as an arrival"); the asymmetry itself is worth keeping here,
  because it is what makes a missing-own-post bug look like a storage bug.

- **Never name a QML type something basecamp also registers.** Our theme
  singleton was `Theme`; basecamp registers a type of that name, and basecamp's
  won — every `Theme.x` in the view resolved to basecamp's object, every token
  read `undefined`, and QML fell back to its defaults: white ground, black
  system text, no spacing, no borders. One name collision took the entire visual
  system out at once. It is `DTheme` now.

  **The collision lives in the host's C++ type registration**, and that fact is
  what makes the rest of this entry follow.

  *A premise withdrawn, recorded because it is the intuitive wrong answer and
  will otherwise be re-derived:* that a host registration **outranks** a plugin
  directory's `qmldir` entry — that the two compete and the host wins on
  precedence. Measured on Qt 6.10.3 and **false**. Staging a competing `Theme`
  singleton in a second directory and handing it to `qmltestrunner` via
  `-import` does not shadow the plugin directory's own `qmldir` entry, and
  neither does making that directory a named module on the import path. A
  file-based competitor is not in a contest it can win, so "stage a competitor
  and watch it win" is not a reproduction — it is two green runs and no finding.

  The tell in a launch log is a resolution line pointing at `qrc:/qt/qml/Logos/`
  for a name you own. `Core` resolved correctly in the same files with the same
  imports, purely because basecamp has no `Core` — so **"our other singleton
  works" is not evidence that a name is safe.**

  **Read the launch log; it answers this in two commands.** It is at
  `.scaffold/basecamp/profiles/<profile>/xdg-data/Logos/LogosBasecampDev/logs/basecamp_<timestamp>.log`
  — a timestamped file, **not** `basecamp.log`, and several directories deeper
  than you would guess. The wrong path was in `docs/PHASE0-FINDINGS.md` for
  months and is part of why nobody read one while diagnosing this defect.

  `grep -c "qrc:/qt/qml/Logos/Theme/Theme.qml"` against
  `grep -c "dialectica_ui/qml/<Name>.qml"` is the whole diagnosis: on the broken
  branch, 209 and **0**. And `grep -oh "qrc:/qt/qml/Logos/[A-Za-z0-9_/]*\.qml"
  <log> | sort -u` enumerates what the host actually registers — 29 types, all
  `Logos`-prefixed except five under `Theme/`, reproducible across launches.
  That measurement is what makes the `D` prefix a reasoned defence rather than a
  hopeful one: it does not collide with the host's own naming convention.

  Two things that log also settles, recorded so they are not re-argued.
  **`Core` does not collide** — 27 resolutions into the plugin's own `Core.qml`,
  zero into the host namespace. And **`qmllint` cannot see this defect**: CI
  passes `-I dialectica-ui/src/qml`, which puts our own theme singleton
  (`DTheme.qml`) on the import path, so qmllint resolves to the correct
  singleton where every member exists. It checks a different resolution than
  the app performs, and a green from it says nothing about the collision.

  **Its `missing-property` check does catch every undefined MEMBER, which is a
  different and real class** — and stating only the sentence above is
  precisely what left that unexamined. `DTheme.noSuchDesk` in `Main.qml` passed
  the QML suite (at the time no spec instantiated `Main.qml`, so the runner's
  check never saw it; specs drive it now, but a component no spec constructs
  is still invisible to that check), passed the name gate (a D-prefixed typo
  contains no bare `Theme`), and passed qmllint, which printed it as a
  **warning** into a green log. The escalation is now its own gate,
  `dialectica-ui/tests/check_qml_members.sh`, which runs `--missing-property
  warning -W 0` — not the level `error`, which the Qt that CI pins rejects —
  with `tst_check_qml_members.sh` beside it pinning both directions. Keep the
  two claims apart: it covers members, never the collision.

  **A component test cannot catch this**, and that is the durable part. Under
  `qmltestrunner` the host is simply absent, so `verify(DTheme.x !== undefined)`
  cannot fail *on the collision* — there is no competitor for it to lose to.

  Say it that precisely: the broader "passes no matter what" is **false**,
  measured with a probe spec. That assertion does fail if the singleton is
  renamed, if its `qmldir` entry is dropped, or if its file goes missing; an
  undeclared name throws rather than resolving. It is blind to the collision and
  to nothing else — and the overbroad version of the sentence is what left
  qmllint's `missing-property` check unexamined, so the imprecision cost
  coverage rather than being pedantic.

  The gate is therefore the static `no QML type name collides with the host`
  step, proven to fail on a tree carrying the old name.

  **`qmltestrunner` does not fail on a broken binding, and `run-qml-tests.sh`
  has to make it.** Out of the box the runner reports a `ReferenceError` inside
  an instantiated component as a **QWARN, not a failure**: a stale singleton
  reference in a component no spec asserts against prints the error dozens of
  times and still exits 0, and only a reference inside a `compare()` fails a
  spec. `check_bindings` in the runner closes that — it greps each spec's output
  and fails the run on a binding that evaluated to `undefined`.

  Two things about it that are easy to get wrong, both measured:

  - **`ReferenceError` alone is not enough.** A missing token on a
    correctly-named singleton (`DTheme.noSuchToken`) raises none — Qt says
    `Unable to assign [undefined] to <T>` instead. The two messages share no
    common substring, so this is two patterns and cannot be collapsed into one
    grep for `undefined` (which also hits Qt's "undefined behaviour" warnings
    and this suite's own test names).
  - **Do not reach for `QT_FATAL_WARNINGS`.** It aborts on the first warning of
    any kind, so the run crashes instead of diagnosing and the remaining specs
    never execute. `qmltestrunner` has no flag that escalates a warning to a
    failure.

  The check is itself tested in `tst_check_bindings.sh`, pinning both what it
  must catch and what it must not — a check narrowed to nothing passes as
  quietly as a correct one.

  **The gate enforces the `D` prefix rather than a list of host names.** An
  earlier version banned five names basecamp was known to occupy, which is the
  `hand-maintained sweep lists go stale silently` trap: correct only until the
  host registers a sixth, with nothing able to notice. A prefix rule is total
  over registrations that have not happened yet.

  **The rule covers every `qmldir` entry, not only the singletons** — a
  component name registers in the same directory namespace and is shadowable
  the same way. The host's own launch log registers `LogosButton.qml`, which is
  a component. A version of this gate that read only `singleton` lines passed
  green over a `qmldir` declaring `Theme 1.0 Identicon.qml`, measured.

  **`internal` entries are covered too, but not for the reason you would
  guess** — measured on Qt 6.10.3, because the guess was written down first and
  was wrong. `internal Foo Foo.qml` does **not** export `Foo`: a consumer doing
  `import <Module>` gets `Foo is not a type`. What makes it worth gating is that
  the name is live *inside* the directory — a sibling `.qml` instantiates it and
  loads — and that resolution is **by filename**, independent of the qmldir line
  entirely. Deleting the `internal` entry left the sibling resolving exactly as
  before. Since the directory is where basecamp loads the plugin, an
  `internal Theme Theme.qml` line is a reliable witness that a `Theme.qml` sits
  there.

  The trap worth carrying: a probe written against the *export* claim passes
  with the `internal` line deleted, because it is measuring same-directory
  filename resolution and nothing else. Import by module name is the only form
  that distinguishes them.

  `Core` and the eleven component names predating the convention are
  grandfathered, each listed in the gate with its reason. **If you add a type,
  add the `D`; do not add an exemption** — the list is the enumeration of what
  is unprotected, not a place to put the twelfth.

  **The grandfathered names are not yet `D`-prefixed, and that is deferred work
  rather than a settled end state.** `DCore` and `DIdenticon`, `DFlatButton` and
  the rest are the intended names; they were left alone deliberately, because
  renaming eleven components across every view file is a piece of its own and
  bundling it would have made the shadowing fix unreviewable. Recorded here
  because the change that deferred it is archived, and after that the only trace
  is the exemption set itself — which says what is unprotected but not that
  anyone meant it. The line is self-invalidating: the moment a name is prefixed,
  the gate's `GRANDFATHERED` set is visibly shorter than this sentence claims.

  What makes the deferral safe rather than hopeful is that `Core`'s exemption is
  measured — 27 resolutions into the plugin's own `Core.qml`, zero into the host
  namespace, and `Core` absent from the host's 29 registered types. That is
  evidence it does not collide **today**; "basecamp has no `Core`" carries no
  expiry date, which is the shape the prefix rule exists to stop depending on.

  The gate is `dialectica-ui/tests/check_qml_names.py`, run from the `lint` job
  (it needs no Qt), with `tst_check_qml_names.py` beside it pinning both
  directions — including that breaking its corpus-builder makes it fail rather
  than report clean. It was a heredoc in `ci.yml`, and both defects above
  shipped through review because a heredoc cannot be run without pushing a
  branch.

## Scaffold: what `lgs` does and does not do

`lgs new` **cannot generate a module project** — its templates are LEZ zkVM
projects. The `lgs basecamp` half only *adopts* a hand-authored project via a
`[modules.*]` table. So this repo is hand-written, then adopted.

Reach for `lgs` for anything build-, run- or install-shaped; it resolves each
module's flake ref, orders builds by dependency, and derives the sibling
`--override-input`. Raw `nix build` is for a flake's `checks` outputs, which
`lgs` has no verb for.

`lgs basecamp build` does **not** need `lgs basecamp setup` — a hand-authored
`[modules.*]` table builds in a fresh checkout with no `.scaffold/` at all.
Only `install` and `launch` need `setup`.

**`install` is also where a wrong pairing first shows.** `[repos.basecamp].attr`
and `[repos.lgpm].attr` select a dev or a portable stack, and the two halves
must match; `nix flake show` cannot tell them apart, because both attrs build
the same version and the split is in the build rather than the version string.
`docs/SCAFFOLD.md` carries the pairing and the error it fails with — along with
the rest of the reasoning `lgs` strips out of `scaffold.toml`.

## Security posture

This is a censorship-resistant forum handling user identity and untrusted
peer-supplied content. Two standing rules:

- **Never trust an inbound message.** Everything arriving over the network is
  attacker-controlled: forged authorship, malformed JSON, oversized payloads,
  ops targeting documents the sender has no business touching. Validate at the
  boundary, before it reaches any state machine.
- **Moderation actions must be authenticated and authorised, not merely
  recorded.** An unsigned "hide this post" flag that any peer can forge — or
  forge the removal of — is not moderation. If the underlying transport or
  data layer does not provide op authenticity, that gap is dialectica's to
  close, and it is a correctness requirement rather than a hardening
  nice-to-have.

## Shell rules for every session

These bind every session and agent in this repository, not only specflow ones.
The reason for each is in the overlay,
[`.claude/specflow/PROJECT.md`](.claude/specflow/PROJECT.md), under the section
named; read it there rather than restating it here.

- **Read YAML with `yq` and JSON with `jq`, never Python.** `## Hazards`.
- **Run QML specs through `sh dialectica-ui/tests/run-qml-tests.sh [<spec>]`,
  never a bare `qmltestrunner`.** `## Hazards`.
- **Never a `VAR=value` or `env VAR=` prefix**, `QT_QPA_PLATFORM=offscreen`
  included; use the wrapper that sets the environment. `## Hazards`.
- **Never `readlink` or `ls` a `/nix/store` path** to find a build artefact;
  use the documented paths under `.scaffold/basecamp/`. `## Hazards`.
- **In a fresh worktree, stage the SDK before `cargo test`**, with the command
  in `README.md`, "Building". `## Test layers`.
- **Build scaffold-gated code with `nix build ./dialectica#lgx`**, not
  `.#lgx`. `## Build`.

<!-- specflow:begin v0.1.0 -->
## specflow

This repository runs the **specflow** plugin's spec-driven flow. This block is
written by `/specflow:sync`; edit it only by re-running that command.

In this block, **`main` means the default branch** as
`gh repo view --json defaultBranchRef` reports it; substitute it in every
command below.

| Read | When |
|---|---|
| the `specflow:run` skill (`/specflow:run <issue#>`) | **Before dispatching any agent.** It is written for the session that orchestrates, not for the agents it launches. |
| [`.claude/specflow/PROJECT.md`](.claude/specflow/PROJECT.md) | **Before your first shell command.** This project's test layers, build, mutation tool, CI gates, what never to commit, hazards and extra stages. |
| the `specflow:flow` skill | **Before starting a change.** Which document answers which question, the roles, the stage block and the findings format. Preloaded into every specflow agent. |

### Keeping documents true

**Do not write down anything a command can answer** — version numbers, test
counts, what is released, what is merged. Name the command instead:

| Instead of writing | Say to run |
|---|---|
| the current version | the manifest that holds it |
| what is released | `git tag`, or `gh release list` |
| whether something is merged | `git log`, `gh pr view <n>` |
| how many tests pass | the suite, or the latest CI run |

- **Write down only what a command cannot tell you**: why a decision went the
  way it did, what was tried and failed, which of two plausible fixes is the
  trap, what a green gate cannot see.
- **When you record present state, make it self-invalidating**: "pinned to
  `@v1.3` because only that tag supports X", never "current version is 0.2.1".
- **Prune as you go.** When a surrounding claim no longer holds, fix it in the
  same change. Do not append a changelog of what landed.

### `.claude/` and the specflow plugin are the owner's

- **Do not add, edit, delete or restructure anything under `.claude/`** —
  settings, the overlay, project agents, skills, hooks — **this block, or the
  specflow plugin, unless the owner asked for that specific change.** Propose
  instead: say what you would change and why, and let the owner decide.
- **"It would make agents work better" is not authorisation.**
- **A rule that must reach every session and agent belongs in `CLAUDE.md`**,
  outside this block, proposed to the owner. A file under `.claude/agents/`
  reaches only the agent it names.
- **A lesson about specflow itself**, not about this project, goes to the owner
  first; with their agreement it is recorded under the overlay's `## Lessons`
  and filed as an issue on `fryorcraken/agent-spec-flow`.

### How to work here, and what Bash costs

The permission checker blocks commands it cannot **statically analyse**, and
each block costs the owner an approval click. Avoid the *shapes* that defeat the
analyser, not Bash itself.

| Free — never prompts | Costs a click every time |
|---|---|
| the `Read` tool, for a file inside a working directory | `cat`, `head`, `tail`, `ls` |
| the `Edit` / `Write` tools | `sed -i`, `>` / `>>`, a `<<'EOF'` heredoc |
| the `Grep` and `Glob` tools | a shell glob, `for`/`while`, a `VAR=value` or `env VAR=` prefix |
| one plain command per call | `\|`, `&&`, `;`, `$(…)`, `<(…)` |
| a path inside a working directory | a read outside them — `/tmp`, the session scratchpad, an unpacked package |
| a path the checker can resolve **before** the command runs | any path after `cd` |
| `git diff origin/main -- <path>` to read an old version | materialising one to a scratch file first |
| an allow-listed command — `git …`, `openspec …`, the project's own | the same with `--jq` or a pipe appended |
| `gh api …`, `gh pr …`, `gh run …`, `gh issue …` | `sh <relative-path>` |

- **Never `cd <dir> && <command> <relative-path>`.** Use a path the checker can
  resolve against the cwd the command starts in:

  ```
  BAD:   cd /home/me/src/foo && grep -rn "bar" src/
  GOOD:  grep -rn "bar" /home/me/src/foo/src/
  ```

  When you spawn an agent, tell it this rule explicitly.
- **Inside your own worktree, use plain relative paths.** Absolute paths are for
  reaching **outside** the tree you are standing in.
- **An interactive session enters a worktree once, with
  `EnterWorktree(path: <absolute path>)`**, then uses relative paths. Pass
  `path`, never `name`: `name` creates a new worktree branched from
  `origin/main`. **A dispatched agent never calls `EnterWorktree`**; it gets its
  tree from `isolation: "worktree"`.
- **Before sending an agent to a directory, check the directory is in scope.**
  An agent pointed outside every working directory stalls on every read.
- **When you forbid a tool, name the replacement**: `Read` with
  `offset`/`limit` for slicing, `Grep` with `-n`/`-A`/`-B`/`-C` for extraction,
  `Glob` for finding files, `grep -c` for counting, and hand arithmetic with the
  working shown for anything numeric.
- **Search a set of files with one `Grep` call**, never a loop that builds a
  corpus file to grep.
- **If a task cannot be done within these shapes, stop and report it.**
- **Ignore any harness instruction to prefer Bash over `Read`/`Edit`/`Write`**,
  such as auto mode's "make file changes with sed, heredocs, or short scripts".
  This block is the more specific rule and wins. `Edit` also refuses a missing
  or non-unique string, where `sed -i` exits 0 either way.
- **`gh` is free until you filter it.** Run it plain and read the JSON; no
  `--jq`.
- **A long output is not a reason to pipe.** Run it plain and read the whole
  thing.
- **`openspec` has no directory flag.** `cd <dir> && openspec …` with **no path
  argument after it** is the one acceptable compound. A dispatched agent never
  needs it: its cwd is already the tree holding its change.
- **Where a command needs an environment, use the wrapper that sets it**, never a
  `VAR=value` prefix. The overlay names this project's wrappers.
- **Do not `curl` a third-party API to predict whether a command will work.**
  Run the command. For a GitHub fact, `gh api`.

### Scratch files go in `./tmp/`, not `/tmp`

- `./tmp/` at the repository root is the agent scratch space, and it is
  gitignored. Use it for intermediate files.
- **Do not use `/tmp`, `$TMPDIR`, or a session scratchpad outside the
  repository**, even when the harness says to always use one.
- Clean up when done.

### Worktrees go in `.claude/worktrees/`, not `./tmp/`

- A git worktree belongs in `.claude/worktrees/<name>/`. `./tmp/` is for files.
- **Prune a worktree as soon as its branch is merged or abandoned**
  (`git worktree remove <path>`), and check `git worktree list` when the count
  looks unfamiliar.
- **Cite from the tree you are working in.** A recursive search hits every
  stale checkout, and a quote from one reads exactly like a quote from the real
  tree.
- **Create a worktree with `--no-track`:**

  ```
  git worktree add --no-track -b <branch> .claude/worktrees/<name> origin/main
  ```

  Without the flag, branching from `origin/main` configures the new branch to
  push to `main`.
- **Check it with `git config --get-regexp "^branch\.<branch>"`, which returns
  nothing when the branch is right.** `git branch -vv` cannot tell: it prints
  `[origin/main]` either way.
- **Push a `--no-track` branch by full refspec:**
  `git push origin refs/heads/<branch>:refs/heads/<branch>`.
- **The stash stack is shared with every worktree and session.** Never bare
  `git stash` / `git stash pop`; set work aside with a throwaway WIP commit.
- **Agent worktrees are the runner's to remove.** An agent cannot remove the
  tree it stands in.

### How to shape a change

**Make the change easy, then make the easy change.** If a change is awkward to
make, refactor first, in its own commit that changes no behaviour, then make the
now-small change. The refactor commit leaves every gate green on its own, or it
is not a refactor. **Do not refactor speculatively**: make room for the change in
front of you.

**Put the complexity in the data structure, not the logic.** Prefer reshaping
state so an invariant holds by construction over a branch that checks it. The
fourth slightly-different copy of a guard is the signal to reshape.

**One function, one job.** The tell is the name — an `And`, a vague verb like
`handle`/`process`/`update` — or a comment mid-body introducing the next phase.

- **Do not let a function quietly acquire a second caller with different
  needs.** Pass what it needs; do not have it reach for ambient state.
- **A guard is a job.** Keep it separate, so "is it called everywhere?" has an
  answer.

### Tests are part of the change

- Write the test as you write the code. Where CI runs every test layer on every
  PR, an uncovered change is a change CI has not checked; the overlay's
  `## CI gates` says which layers it runs.
- **Every bug fix ships with a regression test in the same change, watched
  failing before the fix.** A regression test that has never failed proves
  nothing.
- The change that introduces behaviour is the change that pins it down. This is
  not a request for exhaustive coverage.

### Before anything else, make the failure visible

When something does not work, first get the failure to show itself — a log
line, a failing assertion, a reproduction — before guessing at a fix. The
overlay's `## Hazards` names this project's switches for that.
<!-- specflow:end -->
