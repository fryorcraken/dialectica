# The spec-driven flow

Five roles around [OpenSpec](https://openspec.dev)'s built-in `spec-driven`
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

A directory move plus a merge — nothing is deleted or compressed.

While a change is in flight it lives in `openspec/changes/<name>/`, and its
`specs/` holds a **delta** (`## ADDED Requirements`). `openspec archive` then:

1. **merges the delta into `openspec/specs/`** — the live, current contract;
2. **moves the folder** to `openspec/changes/archive/<date>-<name>/`.

Everything stays in version control and stays greppable. The archive is
organised by change, so finding a past decision means grepping it — which is
what it is there for.

### PLAN.md sheds in two directions

As a change lands, the part of PLAN.md it implements moves out:

- **Behaviour → the spec.** Struck through in PLAN.md, with a one-line summary
  that the thing exists.
- **Reasoning → `design.md`** under Decisions, and removed from PLAN.md. Someone
  investigating a past decision reads the archive; that is what it is for.

PLAN.md is left with what is **not built yet**, plus one line per built area
saying it exists — never why it works that way. Keeping a second copy of the
reasoning is the failure mode: two copies drift and the wrong one gets read.

Reasoning never goes in a spec at all. OpenSpec silently drops a REMOVED
requirement when the capability is new, with validation still passing.

**This applies to changes as they land, not as a migration.** PLAN.md's existing
~1800 lines hold plenty that would now live in a `design.md` — §2.3's SDK gaps,
§11's traps, why BIP-340 was rejected — and most of it has no change to attach
to. Leave it. It shrinks by attrition as changes touch each area.

## The roles

| Agent | Reads | Writes |
|---|---|---|
| `spec-writer` | PLAN.md (from `origin/main`) | `proposal.md`, `specs/` |
| `dev-writer` | spec, PLAN.md | `design.md`, `tasks.md`, code, tests-as-it-goes |
| `tester` | spec, inherited tests | the test suite |
| `spec-test-reviewer` | **spec + tests only** | findings |
| `design-reviewer` | code, `design.md`, PLAN.md | findings |

`spec-test-reviewer` is deliberately blind to the implementation. Someone who
has read the code judges tests by what the code does, which is exactly the
failure a spec exists to catch: a test that faithfully pins the wrong behaviour.

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

**Give each reviewer its own worktree.** Two mutation-testing reviewers sharing
a tree see each other's broken code and cannot tell it from the author's.
