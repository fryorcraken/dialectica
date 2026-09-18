# Move the Bash-costs rules into CLAUDE.md, and revert the change that put them elsewhere

## Why

PR #119 rewrote all seven `.claude/agents/` role files, rewrote
`.claude/agents/README.md`, added `.claude/agents/BASH-COSTS.md`, added a gate
over them and wired two `lint` steps into `ci.yml`.

**That was the runner's own initiative, and the owner did not authorise it.**
`.claude/` is the owner's. Their ruling:

> *"so go and move the instructions you gave to every single agent in claude.md"*
>
> *"I saw what you added, it was to avoid perm prompts; this belongs to CLAUDE.md"*

The diagnosis behind #119 was correct — four dispatched agents in one session
cost the owner manual approval prompts by using shapes CLAUDE.md forbids. The
action was not the runner's to take, and it was also unnecessary.

**`CLAUDE.md` is injected into every Claude session and agent at startup, so a
rule there already reaches everything.** A file under `.claude/agents/` is read
only by the agent it names. Copying one rule into nine files solved a problem
that did not exist, and bought nine copies that drift.

## What changes

**The revert.** #119 is undone in full: the seven role files and
`.claude/agents/README.md` return to their prior content — including README's
"Every agent pays CLAUDE.md's Bash costs" section, which #119 had hollowed out
into a pointer; `BASH-COSTS.md` and the two gate scripts are deleted; and the
two `lint` steps for that gate come out of `ci.yml`.

The probe-twins and QML-reachability gates in the same job are separate,
authorised work and are untouched. `ci.yml` was the one conflict in the revert,
because the reachability gate landed after #119 in the same block.

**The fold into `CLAUDE.md`.** Most of `BASH-COSTS.md` was already there. Only
what the existing section genuinely lacked is added: `for`/`while` named in the
costs table, reads outside the working directories named as a cost, the rule
that relative paths are preferred *inside your own worktree*, the `Grep`/`Glob`
tools as the replacement for a loop that builds a corpus file to grep, and the
`env VAR=` prefix named against the measured `QT_QPA_PLATFORM=offscreen
qmltestrunner` incident.

**One line is corrected rather than restored.** `dev-writer.md` said *"Absolute
paths, and `Read`/`Edit`/`Write` over shell file manipulation"*, which
contradicts the worktree dispatch flow every agent now runs under. That was a
genuine defect predating #119, not part of the unauthorised restructure, so it
is fixed in place.

**A standing rule is added.** `.claude/` is the owner's: nothing there is added,
edited, deleted or restructured without their explicit request. CLAUDE.md
already said settings were the user's; a rule about one file invited reading
everything else as fair game, which is what happened. That narrower line is
generalised rather than left beside the new one.

## Spec delta

None. `.openspec.yaml` sets `skip_specs: true` alongside the required `schema:`,
with the argument. Agent instructions and permission-cost guidance are about how
work gets done here, not about what the forum does; `openspec/specs/` contracts
neither.
