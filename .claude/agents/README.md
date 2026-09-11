# The spec-driven flow

Five roles around [OpenSpec](https://openspec.dev)'s built-in `spec-driven`
schema. OpenSpec supplies the artifacts and their ordering; Claude Code supplies
the agents. Nothing here is custom tooling — a survey of the alternatives (Spec
Kit, Kiro, Tessl, BMAD, AgentOS) found per-role agents unserved everywhere and
declined outright by one, while subagents already give isolated context windows,
per-role models and tool limits.

## The documents, and what each is for

| Document | Question | Lifetime |
|---|---|---|
| `docs/PLAN.md` | Why the system is built this way; what is not built yet | Permanent |
| `proposal.md` | Why this change, and which capabilities it touches | Archived with the change |
| `openspec/specs/` | **What** the system does — the behaviour contract | Permanent |
| `design.md` | **How**, and **why this approach** (Decisions) | Archived with the change |
| `tasks.md` | The ordered checklist | Archived with the change |

Two rules keep them from drifting:

- **PLAN.md sheds as specs are written.** Behaviour it described as forthcoming
  is struck through and pointed at the spec. It shrinks toward intent alone.
- **Reasoning that outlives a change must reach PLAN.md.** `design.md` is
  archived, so a trap or constraint left only there is lost. Per-change
  reasoning stays in `design.md`; system-level reasoning goes to PLAN.md.

Reasoning never goes in a spec. OpenSpec silently drops a REMOVED requirement
when the capability is new, with validation still passing.

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
