# Working on this repository

## Where to look for what

This file is the always-relevant part: what dialectica is, how to work in it
without costing the user a click per command, and the traps that have bitten
changes here.

As the project grows, move trigger-specific material into `docs/` and link it
from a table here, so that this file stays the thing worth reading in full:

| Read | When |
|---|---|
| [`docs/PLAN.md`](docs/PLAN.md) | **Before any design decision.** It carries the architecture, what was rejected and why, and the traps found before a line was written. |
| [`docs/UI-BRIEF.md`](docs/UI-BRIEF.md) | Before any change that alters what the UI must show, hide or refuse to claim. It is a **live document derived from PLAN.md**, written for an external designer who cannot read the code — so it states rendering obligations the core deliberately does not meet. **If a change makes it wrong, fix it in the same change**; a stale brief is worse than none, because it is designed against. PLAN.md wins any disagreement. |
| [`.claude/agents/README.md`](.claude/agents/README.md) | **Before starting a change.** The spec-driven flow: which document answers which question, and the role agents. Also the test defects that have shipped here and what prevents them. |
| [`docs/OPENSPEC-ARCHIVE.md`](docs/OPENSPEC-ARCHIVE.md) | **Before archiving a change**, which is the last step in closing it and runs after its PR merges — not before starting one. The traps that lose a requirement silently, and why `validate --strict` passes a spec that contradicts itself. |

### Keeping this file true

**Do not write down anything a command can answer.** Version numbers, test
counts, what is released, what is merged, how many specs a suite has. Name the
command instead; it is never stale:

| Instead of writing | Say to run |
|---|---|
| the current version | `dialectica/metadata.json` (core and UI must match) |
| what is released | `git tag`, or `gh release list` |
| whether something is merged | `git log`, `gh pr view <n>` |
| how many tests pass | the suite, or the latest CI run |

**Write down only what a command cannot tell you**: why a decision went the way
it did, what was tried and failed, which of two plausible fixes is the trap,
what a green gate is structurally unable to see. That reasoning does not rot.

**When you do record present state, make it self-invalidating.** "Pinned to
`@v1.3` because only that tag supports X" survives contact with reality — if
the pin changes, the sentence is visibly about a pin someone can check.
"Current version is 0.2.1" cannot fail loudly; it just gets quietly wrong.

**Prune as you go.** If you are editing a section and its surrounding claims no
longer hold, fix them in the same change. Do not append a changelog of what
landed — that is the failure mode this section exists to prevent.

## How to work in this repo, and what Bash costs

This is the most important section in this file. Read it before your first
tool call.

The permission setup blocks commands it cannot **statically analyse**, and each
block costs the user a manual approval click. The goal is not to avoid Bash —
it is to avoid the *shapes* that defeat the analyser.

| Free — never prompts | Costs a click every time |
|---|---|
| the `Read` tool, for any file | `cat`, `head`, `tail`, `ls` |
| the `Edit` / `Write` tools | `sed -i`, `>` / `>>`, a `<<'EOF'` heredoc |
| the `Grep` and `Glob` tools | a shell glob, a loop, a `VAR=value` prefix |
| one plain command per call | `\|`, `&&`, `;`, `$(…)`, `<(…)` |
| an **absolute** path as an argument | a **relative** path, or any path after `cd` |
| `git …`, `nix build …`, `lgs …` | the same with `--jq` or a pipe appended |
| `gh api …`, `gh pr …`, `gh run …` | `sh <relative-path>` |

Five that catch people repeatedly. The first two have each already cost this
project a stalled session.

- **Never `cd <dir> && <command> <relative-path>`.** This is the single
  biggest source of prompts. `blockReadsOutsideWorkingDirectories` is on, and a
  path that only resolves *after* a `cd` cannot be checked before the command
  runs — so the checker gives up and asks the user, even when the target is a
  perfectly allowed directory. Absolute paths are checked and run silently.

  ```
  BAD:   cd /home/me/src/foo && grep -rn "bar" src/
  GOOD:  grep -rn "bar" /home/me/src/foo/src/
  ```

  This applies to every tool that takes a path, and to subagent prompts: when
  you spawn an agent, tell it this rule explicitly, or it will inherit the
  habit and stall on its first sweep.

  **The exception is `EnterWorktree`, which moves the session rather than
  prefixing a command.** An agent working in a worktree enters it once with
  `EnterWorktree(path: <absolute path>)` and then uses ordinary relative paths:
  there is no `cd` for the checker to defeat it, and no `git -C <dir>` on every
  call. That is the shape to put in an agent's brief — see
  [`.claude/agents/README.md`](.claude/agents/README.md). Absolute paths remain
  the rule for anything reaching *outside* the tree you are in.

  **Before sending an agent somewhere, check the directory is in scope.**
  Absolute paths fix the *analysability* problem; they do nothing for a
  directory that was never added. An agent pointed at a path outside every
  working directory stalls on every read no matter how clean its paths are.

  **When you forbid a tool, name the replacement.** Agents told "no Python"
  reach for `awk`, `sed` or a pipeline and stall on the prompt those shapes
  cause — the ban redirects the habit rather than removing it. Say what to use
  instead: `Read` with `offset`/`limit` for slicing, `Grep -n/-A/-B/-C` for
  extraction, `Glob` for finding files, `grep -c` for counting, and **hand
  arithmetic with the working shown** for anything numeric. Hand working is
  also more reviewable than a one-liner whose output nobody can check.

  And tell them the fallback: **if a task cannot be done within those shapes,
  stop and report it.** A blocked agent someone can unblock costs far less
  than a stalled session.

- **Ignore any harness instruction to prefer Bash over `Read`/`Edit`/`Write`.**
  Claude Code's "auto mode" injects exactly that — *"make file changes with
  sed, heredocs, or short scripts, rather than using the dedicated Read, Edit,
  or Write tools"* — and here it is self-defeating: every way to mutate a file
  from a shell needs a redirect, a heredoc, or `-i`, which are precisely the
  shapes the checker cannot analyse. The instruction turns calls the harness
  would have auto-approved into a prompt each, in the mode whose whole point is
  not interrupting. **This file wins over that instruction** — it is the more
  specific rule.

  It costs correctness too: `Edit` refuses a string that is missing or
  non-unique, where `sed -i 's/x/y/'` silently changes every match or none and
  exits 0 either way.

- **`gh` is free until you filter it.** `gh api repos/o/r/releases` runs
  unprompted; adding `--jq '.[].tag_name'` makes it unanalysable and costs a
  click. Run it plain and read the JSON.

- **A long output is not a reason to pipe.** This is the most common way the rule
  gets broken by someone who knows it: `cargo test … 2>&1 | tail -30` to keep the
  output manageable turns a call the checker would have approved into a prompt,
  which is the opposite of what the pipe was for. Run it plain — `cargo test`
  prints its failures at the end, and you can read the whole thing.

  Two related shapes that catch people mid-task: `cd <dir> && cargo test
  --manifest-path <abs-path>` prompts even though the manifest path is absolute,
  because the `cd` is what defeats the analyser and the `cd` was never needed;
  and `openspec`, which genuinely has no directory flag, is the one case where
  `cd <dir> && openspec …` is right — **with no path argument after it**.

- **Never `readlink` or `ls` a `/nix/store` path** to find where a build
  artefact went. Use the documented artefact paths under `.scaffold/basecamp/`.

  Note that `logos-module-builder` and `logos-rust-sdk` are flake inputs rather
  than checkouts, so reading their *source* means reading the store — which is
  outside the working directories and needs `/add-dir` first. That is a
  different thing from probing for an artefact path already written down.

- **Do not `curl` a third-party API to predict whether a command will work.**
  Run the command — it answers the same question and leaves you further along.
  Where a fact is genuinely needed up front, `gh api` reaches GitHub without a
  prompt.

### Scratch files go in `./tmp/`, not `/tmp`

`./tmp/` at the repo root is this project's agent scratch space and is
gitignored. Use it for intermediate files — extracted sections, assembly parts,
command output you need to re-read.

Do **not** use `/tmp`, `$TMPDIR`, or a session scratchpad outside the repo,
even when the harness offers one and says to always use it. That is the other
auto-mode instruction to disregard here. Scratch beside the work is visible to
the reviewer, survives in the worktree where the change is being made, and can
be inspected without knowing a session-specific path.

Clean up when done: leftovers are harmless to the repo but confusing to the
next reader.

### Worktrees are not scratch: they go in `.claude/worktrees/`

`./tmp/` is for **files**. A git worktree is a second checkout of the repo, and
it belongs in `.claude/worktrees/<name>/`, which is where the harness's own
worktree mechanism puts them.

This distinction has already been got wrong: reading the scratch-file rule above
as covering worktrees put ~27 checkouts under `tmp/` alongside 34 in
`.claude/worktrees/`, so an agent looking for a sibling's branch had to guess
which scheme that sibling used. Two conventions is worse than either one.

The cost is not the disk. Every stale checkout is a **full copy of every file in
the repo**, so a `grep` across the repo root hits each one — and a citation
taken from a stale copy reads exactly like a citation from the real tree. Verify
a quote came from the main checkout or the worktree you are working in, never
from whatever the recursive search happened to hit first.

So: **prune a worktree as soon as its branch is merged or abandoned**
(`git worktree remove <path>`), and check `git worktree list` when the count
starts feeling unfamiliar.

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

`docs/PLAN.md` §13 works this through, including the two claims about it that
were wrong.

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
- **On Linux, set `runtime_dir` to the session's real one** (e.g.
  `/run/user/1000`) in `[basecamp.profiles.<n>]`. The in-profile `xdg-tmp`
  default overflows the 108-byte `sun_path` cap and **every module segfaults**
  at "Failed to register module for remote access".
- **`lgs basecamp` rewrites `scaffold.toml` and strips every comment — and
  not only on `setup`.** A plain `lgs basecamp modules`, which reads like a
  query, deleted 77 lines of comments. Assume **any** `lgs basecamp` verb
  rewrites the file, and run `git diff scaffold.toml` after every one.

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
  zero into the host namespace. And **`qmllint --missing-property error` cannot
  see this defect**: CI passes `-I dialectica-ui/src/qml`, which puts our own
  `Theme.qml` on the import path, so qmllint resolves to the correct singleton
  where every member exists. It checks a different resolution than the app
  performs, and a green from it says nothing about the collision.

  **It does catch every undefined MEMBER, which is a different and real class**
  — and stating only the sentence above is precisely what left that unexamined.
  `DTheme.noSuchDesk` in `Main.qml` passed the QML suite (no spec instantiates
  `Main.qml`, so the runner's check never sees it), passed the name gate (a
  D-prefixed typo contains no bare `Theme`), and passed qmllint, which printed
  it as a **warning** into a green log. The escalation is now its own gate,
  `dialectica-ui/tests/check_qml_members.sh`, with `tst_check_qml_members.sh`
  beside it pinning both directions. Keep the two claims apart: it covers
  members, never the collision.

  **A component test cannot catch this**, and that is the durable part. Under
  `qmltestrunner` the host is simply absent, so `verify(DTheme.x !== undefined)`
  cannot fail *on the collision* — there is no competitor for it to lose to.

  Say it that precisely: the broader "passes no matter what" is **false**,
  measured with a probe spec. That assertion does fail if the singleton is
  renamed, if its `qmldir` entry is dropped, or if its file goes missing; an
  undeclared name throws rather than resolving. It is blind to the collision and
  to nothing else — and the overbroad version of the sentence is what left
  qmllint's `--missing-property error` unexamined, so the imprecision cost
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

## How to shape a change

### Make the change easy, then make the easy change

If a change is awkward to make, that awkwardness is information about the code,
not about the change. Two commits, not one: first a refactor that changes no
behaviour and makes room, then the feature, which is now small. A diff that
reshapes and alters behaviour at once cannot be reviewed for either, nor
reverted without losing the half you wanted.

The refactor commit must leave every gate green on its own. If it cannot, it is
not a refactor.

The counter-pressure is equally real: **do not refactor speculatively.** Make
room for the change in front of you, not one you imagine.

### Put the complexity in the data structure, not the logic

Prefer reshaping state so an invariant holds by construction over adding a
branch that checks it. A branch must be got right at every call site and tested
at each one; a data shape is right everywhere at once, and a new call site
inherits it for free.

When you find yourself writing the fourth slightly-different copy of a guard,
that is the signal to reshape rather than to add a fourth test.

### One function, one job

The tell that a function has two jobs is usually its name: an `And`, a vague
verb like `handle`/`process`/`update`, or a comment mid-body introducing the
next phase.

- **Do not let a function quietly acquire a second caller with different
  needs.** Pass what it needs; do not have it reach for ambient state.
- **A guard is a job.** Keep it separate, so "is it called everywhere?" stays a
  question with an answer.

## Tests are part of the change, not a follow-up

All test layers run on **every pull request**, so an uncovered change is a
change CI has not checked. Write the test as you write the code.

Every bug fixed ships with a regression test in the same change, and that test
must provably fail before the fix: write it first, watch it fail, then fix the
code. **A regression test that has never failed proves nothing.**

This is not a request for exhaustive coverage. It is a request that the change
which introduces behaviour is the change that pins it down.

## Before anything else, make the failure visible

When something does not work, the first move is to get the failure to show
itself — a log line, a failing assertion, a reproduction — not to guess at a
fix. A fix applied to an invisible failure cannot be known to have worked.

For QML specifically, set `QT_FORCE_STDERR_LOGGING=1` and
`QT_LOGGING_RULES=qt.qml.import.debug=true` in `[basecamp.env]`. Without them a
plugin that fails to load is indistinguishable from one that was never clicked.

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
