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
| [`.claude/agents/README.md`](.claude/agents/README.md) | **Before starting a change.** The spec-driven flow: which document answers which question, and the five roles. Also the test defects that have shipped here and what prevents them. |

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

  **Before sending an agent somewhere, check the directory is in scope.**
  Absolute paths fix the *analysability* problem; they do nothing for a
  directory that was never added. An agent pointed at a path outside every
  working directory stalls on every read no matter how clean its paths are.

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
- **`lgs basecamp setup` strips every comment from `scaffold.toml`.** Run
  `git diff scaffold.toml` after any `setup`.
- **The UI's icon must be a 256×256 PNG**, and the UI module must declare core
  in `dependencies` with **matching versions**.

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
