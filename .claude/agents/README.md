# The spec-driven flow

Role agents around [OpenSpec](https://openspec.dev)'s built-in `spec-driven`
schema. OpenSpec supplies the artifacts and their ordering; Claude Code supplies
the agents. Nothing here is custom tooling — a survey of the alternatives (Spec
Kit, Kiro, Tessl, BMAD, AgentOS) found per-role agents unserved everywhere and
declined outright by one, while subagents already give isolated context windows,
per-role models and tool limits.

## The documents, and what each is for

| Document | Question | Where it ends up |
|---|---|---|
| `docs/PLAN.md` | A short summary of what exists, and **what is not built yet** | Lives at `docs/`, edited forever |
| `proposal.md` | Why this change, which capabilities it touches | `changes/archive/<date>-<name>/` |
| `openspec/specs/` | **What** the system does — the behaviour contract | `openspec/specs/`, current |
| `design.md` | **How**, and **why this approach** (Decisions) | `changes/archive/<date>-<name>/` |
| `tasks.md` | The ordered checklist | `changes/archive/<date>-<name>/` |

### What "archived" means concretely

This is OpenSpec's own behaviour, not a convention of ours.

While a change is in flight it lives in `openspec/changes/<name>/`, and its
`specs/` holds a **delta** (`## ADDED Requirements`). `openspec archive` then:

1. **offers to merge the delta into `openspec/specs/`** — the live, current
   contract. It is a prompt, and declining it archives without promoting the
   spec, so take the sync;
2. **moves the folder** to `openspec/changes/archive/<date>-<name>/`, dated
   today unless the name already carries a date, which is never stacked.

So the change's `proposal.md`, `design.md` and `tasks.md` are moved, not
deleted: they stay in version control and stay greppable. Finding a past
decision means grepping the archive, which is what it is for.

One exception worth knowing: a change that declares `retire_capabilities` can
make archive **delete** a spec rather than merge into it. Nothing here does
that, and it takes an explicit marker.

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

**Two steps belong to whoever is running the change, not to any agent:**

- **Acting on findings.** Every reviewer ends "findings only, do not fix". A
  finding about behaviour goes back to `spec-writer`; about the code, to
  `dev-writer`; about a test, to `tester`. Re-run only the reviewers whose
  findings led to changes.
- **`openspec validate` and `openspec archive`.** Archive is where the delta is
  merged into `openspec/specs/` — skip the step, or decline its sync prompt, and
  the change ships with its spec never promoted. Do it once the change is
  otherwise done, and take the sync.

The three reviewers split deliberately, and run in parallel:

- `code-reviewer` asks **is this code correct, safe and well-shaped?**
- `spec-test-reviewer` asks **do the tests pin what the spec requires, and can
  they fail?**
- `design-reviewer` asks **did the code take the decisions that were recorded,
  and were the decisions worth recording recorded?**

**`code-reviewer` is launched once per dimension** — correctness, security,
readability, architecture — with the prompt naming which. One agent holding all
four does each worse: scanning for a reachable panic is a different reading of
the same file from scanning for a function doing two jobs, and a single pass
becomes whichever the reviewer started with. A small change can take one
instance covering all four.

So a full review is typically six agents: four `code-reviewer`, plus
`spec-test-reviewer` and `design-reviewer`.

`spec-test-reviewer` is deliberately blind to the implementation. Someone who
has read the code judges tests by what the code does, which is exactly the
failure a spec exists to catch: a test that faithfully pins the wrong behaviour.

**Give each reviewer that mutates code its own worktree.** Two sharing a tree
see each other's broken code and cannot tell it from the author's; this has
happened.

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

**Specs get reorganised as concepts generalise.** When a second instance shows
that requirements written for one capability are really about a general one,
they move — `REMOVED` from the old spec and `ADDED` to the new, verbatim, in one
change. OpenSpec has no capability move or rename, so the extraction is composed
from those primitives. Do it when the generality is demonstrated, not predicted.
