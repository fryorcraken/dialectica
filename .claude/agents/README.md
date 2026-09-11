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

#### Archiving by hand, when the CLI is not installed

`openspec` is not installed here (`openspec --version` → 127), so the merge has
been done by hand. The transformation, for a capability `openspec/specs/` does
not yet hold, is exactly two edits:

1. prepend `# <capability> Specification` and a blank line;
2. rename `## ADDED Requirements` to `## Requirements`.

Everything else is carried across byte-for-byte. Verify by diffing the promoted
file against the delta and confirming those are the only hunks.

Four things about that rule cost time to establish, and none is visible from
the files themselves:

- **`stoa-genesis` is NOT a valid reference example.** Its live spec differs
  from its delta by whole added paragraphs and `SHALL` → `MUST` rewrites. Both
  landed in the *same commit* (`3dddf03`), so the live file was hand-edited
  after the delta was written and no diff ever showed it. Deriving the
  transformation from that pair yields a rule that is simply wrong. Derive it
  from `2026-09-11-op-model` → `op-format`, which matches exactly.
- **Deltas vary in shape.** Some carry a `## Purpose`; `stoa-metadata`'s
  carries a title line and no Purpose; `spec-backfill`'s two carry **neither**
  and open directly on `## ADDED Requirements`. The transformation still
  applies — but "prepend a title before the Purpose" is not the rule, and a
  merge that assumes a Purpose is present will mangle the ones without.
- **A `MODIFIED` requirement replaces the WHOLE block, scenarios included** —
  not just its prose. A merge that swapped the paragraphs and left the old
  scenarios underneath produces a requirement whose scenarios contradict it,
  with no error. When `stoa-metadata-op` renamed the field cap's boundary pair,
  taking only the prose would have left two scenarios describing a limit the
  requirement no longer words that way.
- **A `MODIFIED` heading that matches no requirement must stop the merge.**
  This is the failure the whole discipline exists to catch: `openspec archive`
  finds nothing to modify and **silently applies nothing**, losing a
  requirement with no error. Check every MODIFIED heading against the target's
  actual `### Requirement:` lines before merging, and report a miss rather than
  guessing at what was meant.

**Archive in merge order**, oldest first — a later change's `MODIFIED` delta
must apply to the text an earlier change's `ADDED` delta produced. Derive the
order from `git log --name-status --diff-filter=A -- openspec/changes`, which
maps each change folder to the commit that introduced it; do not guess it from
folder names. The archive date is the **merge** date, from that commit, not the
date you are doing the sweep.

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

**Two capabilities asserting one rule is the failure that reorganisation
prevents, and it is already here.** `identity` and `op-format` both carry an
authenticity-is-not-authority requirement, both pin derivation constants, and
`identity` restates the key-to-author binding `op-format` covers. Two copies
drift, and the reader who finds the stale one has no way to tell. Which
capability owns each rule is a design call, not a sweep's to make; it is
recorded here so whoever next touches either finds it stated rather than
rediscovers it.

The contrast is the evidence that the discipline works when applied:
`op-ordering` faced exactly this against `op-format`'s "An op carries no
ordering field", **declined to restate it, and said so in its Purpose** —
naming the other capability's requirement and the boundary between them.
`spec-backfill` did not, and produced the duplication above. The cost of
writing that sentence is one paragraph; the cost of not writing it is a
contract with two answers.

**A contradiction between two capabilities is invisible until they are merged.**
`keystore` and `posting-capability` shipped in one change, each internally
consistent, jointly demanding a distinction the design deliberately does not
provide: a wrong passphrase told apart from a tampered ciphertext, which the
AEAD tag structurally cannot do. Nothing caught it, because each delta was
reviewed alone and neither had ever been read beside the other. Archiving is
the first moment they sit in one contract — so it is the moment to read them
together, and a sweep that finds such a pair should resolve it rather than
leave the next reader to inherit a contract that contradicts itself.
