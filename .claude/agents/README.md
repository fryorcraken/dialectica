# The spec-driven flow

Role agents around [OpenSpec](https://openspec.dev)'s built-in `spec-driven`
schema: OpenSpec supplies the artifacts and their ordering, Claude Code the
agents. No custom tooling — subagents already give isolated context windows,
per-role models and tool limits.

## The documents, and what each is for

| Document | Question | Where it ends up |
|---|---|---|
| `docs/PLAN.md` | A short summary of what exists, and **what is not built yet** | Lives at `docs/`, edited forever |
| `proposal.md` | Why this change, which capabilities it touches | `changes/archive/<date>-<name>/` |
| `openspec/specs/` | **What** the system does — the behaviour contract | `openspec/specs/`, current |
| `design.md` | **How**, and **why this approach** (Decisions) | `changes/archive/<date>-<name>/` |
| `tasks.md` | The ordered checklist | `changes/archive/<date>-<name>/` |
| `findings.md` | What review found, and what was done about each | **Deleted before merge** — durable reasoning moves to `design.md` first |

A change in flight lives in `openspec/changes/<name>/`, and its `specs/` holds a
**delta**. `openspec archive` merges the delta into `openspec/specs/` and moves
the folder to `changes/archive/<date>-<name>/` — moved, not deleted, so finding a
past decision means grepping the archive.

**Archiving has enough traps to be worth its own page:
[`docs/OPENSPEC-ARCHIVE.md`](../../docs/OPENSPEC-ARCHIVE.md). Read it before you
archive, not before you start.** `openspec` is installed; run
`openspec --version` rather than believing any document about it, this one
included.

### A spec must never cite a PLAN section number

`openspec/specs/` is read on its own. A requirement citing "§5.7" points at a
`docs/PLAN.md` heading that the reader does not have open, that carries no
stable number, and that PLAN.md sheds as changes land. Cite by requirement
name, or restate the substance in one clause.

One instance is live in `moderation-resolution` ("The deciding moderation is
named"), inherited from its delta and left alone by the sweep that found it —
an archive sweep that also edits prose is a sweep nobody can review.

### PLAN.md sheds in two directions

As a change lands, the part of PLAN.md it implements moves out:

- **Behaviour → the spec.** Struck through in PLAN.md, with a one-line summary
  that the thing exists.
- **Reasoning → `design.md`** under Decisions, and removed from PLAN.md. Someone
  investigating a past decision reads the archive; that is what it is for.

PLAN.md is left with what is **not built yet**, plus one line per built area
saying it exists — never why it works that way. Keeping a second copy of the
reasoning is the failure mode: two copies drift and the wrong one gets read.

Reasoning never goes in a spec at all — a spec is a behaviour contract, and
prose rationale in one is prose nobody will maintain.

**This applies to changes as they land, not as a migration.** PLAN.md today
holds plenty that would now live in a `design.md` — §2.3's SDK gaps, §11's
traps, why BIP-340 was rejected — and most of it has no change to attach to.
Leave it. It shrinks by attrition as changes touch each area.

## The roles

| Agent | Reads | Writes |
|---|---|---|
| `spec-writer` | PLAN.md (from `origin/main`) | `proposal.md`, `specs/` |
| `dev-writer` | spec, PLAN.md | `design.md`, `tasks.md`, code, tests-as-it-goes |
| `tester` | spec, inherited tests | the test suite |
| `spec-test-reviewer` | **spec + tests only** | findings |
| `design-reviewer` | code, `design.md`, PLAN.md | findings |
| `code-reviewer` | code | findings |

## One piece of work is one branch and one PR

Every stage — spec, design, code, tests, review fixes — lands as **commits on one
branch, under one PR**. Agents keep their own worktrees and local branches; a
*stage* never gets its own branch and PR.

**The unit of review is a behaviour change with its contract and its tests
attached.** A reviewer must be able to see they belong together, not be told so
by whoever is orchestrating. And `openspec archive` runs once, on merge — split
across several merges, the contract lands at a different time from the code that
honours it.

## Findings go in `openspec/changes/<name>/findings.md`

Written by the reviewer, ticked by the fixer, deleted before merge.

**A reviewer's final report is a pointer, not a copy.** Write the findings to the
file, then report only: the path, how many entries, and who each is for. The
fixer reads the file; the runner reads the pointer and dispatches.

Two reasons, and the second is why this is a rule rather than a preference:

- A paraphrase arrives without the evidence that backed it. **Never relay a
  finding through a brief** — name the file.
- A report copied into the runner's context, then rewritten into the next brief,
  occupies it twice. That is what crowds out the state a runner needs to keep,
  and it is how a piece of work ends up with no agent on it.

**Reviewers append, never rewrite another entry.** Each one carries: who it is
for (`spec-writer`, `dev-writer` or `tester`), the defect, a failure scenario
concrete enough to reproduce (inputs → wrong output, or the mutation that
survives), `file:line`, and the measurement where there is one.

**The fixer ticks in the commit that addresses it**, so the claim and the change
are one diff. Record one of three outcomes:

- **Fixed** — the commit, and the test that fails without it.
- **Rejected** — with the argument. Reviewers are wrong sometimes; a rejection is
  a legitimate outcome, but argue it rather than closing it silently.
- **Deferred** — and where it now lives. A finding that leaves this file without
  landing somewhere durable was dropped, not deferred.

**An outstanding entry blocks the merge.** That is why this is a file and not a
convention — a forgotten finding now stops a PR instead of evaporating.

**Move durable reasoning into `design.md` before deleting.** "The creator key
cannot moderate the Stoa it creates" is a recorded decision, not a task; the
tracker is scaffolding, the reasoning is not. Deleting the file is the last
commit.

### Handing over between agents

No agent can read another's report — everything passes through the runner, so
every hop is a chance to lose the evidence. Three rules:

**Continue an agent rather than starting one.** A message to the agent that did
the work keeps its worktree, its measurements and its reasoning. A fresh agent
gets a brief, which is a summary of those.

**Write the dead end down, not just the conclusion.** A reviewer that spends an
afternoon establishing why a trait-driven sweep is impossible here — the trait
lives in the crate that depends on core, not the reverse, and is behind a `cfg`
`cargo test` never sets — has produced a result worth as much as the review.
Unwritten, the next agent spends the same afternoon. It goes in `design.md`
beside the decision it rules out.

**State a claim's provenance when relaying one.** "A reviewer measured X" and "I
believe X" license different actions, and an agent given the second as the first
will not re-check it.

**Two steps belong to whoever is running the change, not to any agent:**

- **Routing findings and launching the fixer.** The reviewer writes to
  `findings.md` and the fixer ticks there, so the runner's job is to dispatch,
  not to carry the content: name the file in the brief and let the agent read the
  reviewer's own words. Re-run only the reviewers whose findings led to changes.
- **`openspec validate` and `openspec archive`.** Archive is where the delta is
  merged into `openspec/specs/` — skip the step, or decline its sync prompt, and
  the change ships with its spec never promoted. Do it once the change is
  otherwise done, and take the sync.

The reviewers run in parallel and ask different questions:

- `code-reviewer` — **is this code correct, safe and well-shaped?**
- `spec-test-reviewer` — **do the tests pin what the spec requires, and can they
  fail?**
- `design-reviewer` — **did the code take the decisions that were recorded, and
  were the decisions worth recording recorded?**

**Launch `code-reviewer` once per dimension** — correctness, security,
readability, architecture — naming which in the prompt. Scanning for a reachable
panic is a different reading of a file from scanning for a function doing two
jobs, and one agent holding both becomes whichever it started with. A small
change can take one instance covering all four; a full review is typically six
agents.

**`spec-test-reviewer` is blind to the implementation.** Someone who has read the
code judges tests by what the code does — exactly the failure a spec exists to
catch: a test that faithfully pins the wrong behaviour.

**Give each reviewer that mutates code its own worktree**, in
`.claude/worktrees/`. Two sharing a tree see each other's broken code and cannot
tell it from the author's; this has happened.

## What experience has taught this flow

Each of these is in the agent files because it cost something here.

**A test must assert against something the implementation did not produce.**
Three tests have shipped that could not fail for the reason they named:

- comparing `"ab"` with `"abc"` to prove a length prefix mattered — they differ
  either way;
- mutating a byte and asserting a hash moved — a property of SHA-256, not of the
  encoding;
- `assert_eq!(bytes[0], VERSION_1)` — asking the implementation what it wrote,
  and agreeing.

The fix is a hardcoded expectation. See
`identity.rs::the_wire_constants_are_pinned_to_known_answers`.

**`cargo mutants` is a complement, not a substitute.** It found a real gap in
7 seconds (`Policy::to_byte` replaced by a constant survived the suite) but
cannot see the defect above, because it mutates functions and not `const`
values.

**Mark unspecified behaviour in the code.** When the spec is silent and the dev
chooses, the test carries `// NO SPEC: <what was chosen>`. Without a marker a
reasonable default becomes permanent by accident.

**Never write a scenario that cannot be tested.** A field with one variant
cannot be varied through the API; behaviour that does not exist yet cannot be
covered. Describe what is checkable, or say it is out of scope.

**Read PLAN.md from `origin/main`.** A change was once designed against a §4.3
that had been rewritten to say the opposite.

**A green gate can be structurally blind.** `cargo fmt --check` does not follow
path dependencies, so it never reaches `dialectica-core` — where nearly all the
logic lives. Anything behind `cfg(logos_scaffold)` is not compiled by
`cargo test` at all. Say what a gate cannot see rather than reporting it as
passed; "exit 0" on a gate that measured nothing is worse than no gate.

**Specs get reorganised as concepts generalise**, and two capabilities asserting
one rule is the failure that prevents — both already live here. See
[`docs/OPENSPEC-ARCHIVE.md`](../../docs/OPENSPEC-ARCHIVE.md).
