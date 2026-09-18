## Context

See `proposal.md` for motivation and the measurement.

The constraint that shapes everything below: **`CLAUDE.md` and `README.md` both
warn that two copies of a rule drift and the wrong one gets read.** The obvious
fix — paste the forbidden-shapes list into all seven role files — is the shape
this repo has explicitly ruled out, and it would have produced seven copies to
maintain of a list that will certainly change.

The counter-constraint is the property that was missing: **an agent that reads
only its own role file must come away knowing the forbidden shapes and their
replacements.** A pure pointer satisfies the no-duplication rule and fails this
one, because an agent that never follows the link learns nothing.

## Goals / Non-Goals

**Goals:**

- Every role file teaches the forbidden shapes and the replacement for each.
- One canonical list, so a new forbidden shape is added in one place.
- A check that catches the eighth role file added later with no pointer.

**Non-Goals:**

- Changing the permission configuration itself. The approval prompts are
  correct behaviour by a checker doing its job; the defect is that agents were
  not told what defeats it.
- Enumerating every possible unanalysable shape. The list covers the measured
  incidents plus the shapes `CLAUDE.md` already names.
- Renaming or restructuring the role files.

## Decisions

### A canonical list plus a summarising pointer, not a pure pointer and not ten copies

**Chosen:** `.claude/agents/BASH-COSTS.md` holds the full table. Each role file
gets a uniform ~12-line section carrying (a) the link, (b) a one-sentence
summary naming every forbidden shape, (c) the tool replacements, (d) the
stop-and-report fallback, and (e) one role-specific line naming that role's most
likely trap.

The duplication is deliberate and **bounded to the part that does not change**.
The *shapes* are stable — `|`, `&&`, `$(…)` and loops will not stop costing a
click. What changes is the detail: which wrapper script to use, which flag on
`gh` is free, a newly discovered trap. That detail lives only in `BASH-COSTS.md`.

**Alternatives considered:**

- **A pure pointer** ("read `BASH-COSTS.md`"). Rejected: it fails the stated
  property. An agent under instruction-following pressure, mid-task, reaching
  for a loop, does not stop to open a linked file — and the four measured
  incidents are exactly agents not consulting a document that existed
  (`CLAUDE.md` had the rules the whole time). The failure mode being fixed *is*
  "the rule was one indirection away".
- **The full table in every role file.** Rejected on the repo's own
  two-copies-drift rule, and because seven copies of a fifteen-row table is a
  maintenance surface that will go stale asymmetrically — the worst outcome,
  since an agent cannot tell which copy is current.
- **Putting the list in `README.md` only**, extending the existing "Every agent
  pays CLAUDE.md's Bash costs" section. Rejected: this was checked first, as
  instructed. `README.md` is addressed to a reader of the *flow*, and a
  dispatched agent's brief points at the piece and the findings directory, not
  at `README.md`. It is also 32KB — the largest file in the directory — so the
  rules would sit far from what any single agent reads. `README.md` now points
  at `BASH-COSTS.md` instead of carrying its own partial copy, which removes a
  drift source rather than adding one.

### The gate identifies role files by `name:` frontmatter, not by a list

**Chosen:** `check_agent_bash_costs.sh` enumerates `.claude/agents/*.md` and
treats any file with `name:` in its first six lines as a role file.

This is `CLAUDE.md`'s "complexity in the data structure, not the logic" and the
memory note that **hand-maintained sweep lists go stale silently**. A gate
carrying `for f in spec-writer.md code-reviewer.md …` is correct until someone
adds an eighth role, at which point it passes green over the exact defect it
exists to catch. Deriving the set from the property that makes a file
dispatchable means a new role file is covered the moment it exists.

It also gets the exemptions right for free: `README.md` and `RUNNER.md` have no
frontmatter, so they are exempt *by what they are* rather than by being named.
`RUNNER.md` is addressed to the orchestrating session, which reads `CLAUDE.md`
directly.

**Alternative considered:** a hardcoded list of the seven current files.
Rejected for the reason above — it is the defect class this repo has most often
shipped.

### The gate checks for the summary, not only the link

**Chosen:** three representative phrases must be present — `one plain command
per call`, `stop and report it`, `./tmp/` — alongside the `BASH-COSTS.md`
reference.

**What breaks without it:** the "pointer with none of the summary" case in
`tst_check_agent_bash_costs.sh` turns red. That case is the subtle one, because
a file containing `See BASH-COSTS.md` *looks* finished to a human skimming the
diff while delivering none of the property. A link-only check would be a gate
the defect satisfies — worse than no gate, because it closes the question.

Three phrases rather than all fifteen rows: pinning every row would make the
gate fail on a reworded list, training people to edit the gate rather than fix
the file. Three is enough to distinguish a real section from a bare link, which
is the only distinction that matters here.

### The gate fails when it finds no role files

**What breaks without it:** the "a directory with no role files at all" case in
the test. If the frontmatter match ever breaks — a format change, a path typo —
every file is skipped and the gate reports `OK` having measured nothing. This
repo's most-documented defect class is a check that cannot fail, and a check
that silently narrows to zero is that defect wearing a green tick. The `checked`
counter and its guard are what make the green mean something.

### A check is worth it here, rather than being over-engineering

The brief asked for an explicit argument either way. **A check earns its place**,
for one reason specific to this defect: the thing being protected is *itself* a
set of instructions to agents, and the failure is silent and delayed. Nothing
goes red when a role file lacks the rules — an agent simply costs the user
clicks weeks later, and the connection back to the missing section is not
obvious. That is precisely the profile the repo builds gates for.

The cost is low: the gate needs no Qt, no Rust and no network, and runs in the
existing `lint` job in milliseconds.

## Risks / Trade-offs

**A reworded summary could fail the gate on a correct file** → the gate checks
three short, load-bearing phrases rather than whole sentences, and the failure
message names which phrase is missing and says to copy the section from any
passing role file. A red here is diagnosed from the message alone.

**The duplicated summary can drift from `BASH-COSTS.md`** → accepted, and
bounded by what is duplicated: only the shape names, which are the stable part.
Any detail that changes lives in exactly one place. The gate's phrase check also
pins the summary's load-bearing content, so drift in those cannot be silent.

**An agent may treat the summary as complete and never open the canonical
list** → partly intended. The summary is designed to be sufficient for the
common case; the link carries the wrapper-script names, the `gh --jq` rule, the
SIGPIPE trap and the exceptions. A role-specific final line in each section
names that role's most likely trap, which is the case where the summary alone
would not have been enough.

**`dev-writer.md` said "Absolute paths"** — a direct contradiction of every other
role file and of the worktree dispatch flow, both of which call for relative
paths inside your own tree. Corrected in this change to point at the new
section. Worth noting as a finding about the old text rather than a silent
edit: it is plausibly where a dispatched `dev-writer` got the habit that
produced the typo'd absolute path in the measured incident.

## Migration Plan

Not applicable — no deployed artifact and no data. The files are instructions
read at dispatch time, so the change takes effect for the next agent dispatched
after merge.
