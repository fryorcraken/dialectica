# Archiving an OpenSpec change

Read this when you are **archiving** — the last step in closing a change, run
**after** its PR merges, by the `closer`. `.claude/agents/README.md` is the
flow; this is the one step with enough mechanical detail to be worth its own
page.

Archiving after the merge rather than before is deliberate: `archive` rewrites
the live contract in `openspec/specs/`, so running it on the piece branch folds
a contract promotion and the code into one squashed commit that cannot be
reverted in halves — and running it before CI is green promotes a contract for
code that may never land.

`openspec` is installed. **Run `openspec --version` rather than believing any
document about it** — including this one. This file once recorded the CLI as
absent (exit 127); the absence was real, the sentence outlived it, and "openspec
is not installed" reached five agents in one day on that basis.

## What archive does

While a change is in flight it lives in `openspec/changes/<name>/`, and its
`specs/` holds a **delta** (`## ADDED Requirements`). `openspec archive`:

1. **offers to merge the delta into `openspec/specs/`** — the live contract. It
   is a prompt, and declining it archives without promoting the spec, so take
   the sync;
2. **moves the folder** to `openspec/changes/archive/<date>-<name>/`, dated
   today unless the name already carries a date, which is never stacked.

`proposal.md`, `design.md` and `tasks.md` are moved, not deleted — still in
version control, still greppable. Finding a past decision means grepping the
archive.

A change declaring `retire_capabilities` makes archive **delete** a spec instead
of merging into it. It takes an explicit marker; nothing here does it.

**`archive` aborts and writes nothing if the target spec has no `## Purpose`.**

## The transformation, and why to diff it

For a capability `openspec/specs/` does not yet hold, exactly two edits:

1. prepend `# <capability> Specification` and a blank line;
2. rename `## ADDED Requirements` to `## Requirements`.

Everything else carries across byte-for-byte. **The CLI does not report what it
changed**, so diff the promoted file against the delta and confirm those are the
only hunks. A merge nobody diffed is a merge nobody verified.

## Four traps, none visible from the files

- **`stoa-genesis` is NOT a valid reference example.** Its live spec differs from
  its delta by whole added paragraphs and `SHALL` → `MUST` rewrites, both landed
  in the *same commit* (`3dddf03`) — so the live file was hand-edited after the
  delta and no diff ever showed it. Derive the rule from `2026-09-11-op-model` →
  `op-format`, which matches exactly.
- **Deltas vary in shape.** Some carry a `## Purpose`; `stoa-metadata`'s carries
  a title line and none; `spec-backfill`'s two carry neither and open on
  `## ADDED Requirements`. A merge assuming a Purpose mangles those.
- **A `MODIFIED` requirement replaces the WHOLE block, scenarios included.**
  Swapping the prose and leaving the old scenarios produces a requirement whose
  scenarios contradict it, with no error.
- **A `MODIFIED` heading that matches nothing must stop the merge.** Check every
  heading against the target's actual `### Requirement:` lines, character for
  character, and report a miss rather than guessing. v1.13.0 does catch a
  MODIFIED block that *omits* a scenario the live spec still has, naming it — but
  do not rely on the tool to catch a heading that matches nothing.

**A spec can contradict itself and `validate --strict` will pass it.** Twice
now: a requirement whose prose admitted only a three-input derivation while a
restored scenario asserted two, and two readings of a null field both reaching
one field and disagreeing. Re-read the whole requirement after editing it.

**Archive in merge order**, oldest first — a later `MODIFIED` must apply to the
text an earlier `ADDED` produced. Derive the order from
`git log --name-status --diff-filter=A -- openspec/changes`; do not guess from
folder names. The archive date is the **merge** date from that commit.

## A change with no spec delta

Not every change contracts module behaviour. A CI gate, a pure refactor, a
docs-only change asserts nothing about what a peer sends or what core answers,
so there is no capability to promote — and everything above this section is
about promoting one. **Such a change archives normally**; the folder moves to
`openspec/changes/archive/<date>-<name>/` and there is simply no delta-merge
prompt to take, because there is no delta.

The three things that are not visible from the files:

- **`skip_specs: true` in the change's `.openspec.yaml` is the marker**, and it
  is what makes `validate --strict` accept zero deltas. Without it the failure
  is `Change must have at least one delta` — a message that names deltas and
  never mentions the marker, so the natural response is to go and write a spec
  the change does not need.
- **`schema:` must sit beside it or the marker is silently ignored.** That
  presents as two problems when it is one: the marker reported as ignored, then
  a validation failure for having no deltas. One cause, two symptoms — fix the
  missing `schema:` key and both go.
- **There is also a `--skip-specs` CLI flag** (`openspec archive --help`), whose
  name collides with the YAML key. It is **not** a substitute and not required:
  with the marker set correctly, `validate --strict` passes and
  `openspec show <name> --json --deltas-only` reports `deltaCount: 0` without
  the flag. The file is the mechanism.

**`core-e2e` is a precedent for the marker and not for the archive commit.** It
is the only archived zero-delta change, and
`git log --diff-filter=A -- openspec/changes/archive/2026-09-13-core-e2e/`
shows it landing *inside* its squash merge `b85111d` rather than as a separate
post-merge commit. Every earlier change archived that way too, spec-bearing ones
included, so that is a pre-#59 habit rather than anything about zero-delta
changes — and it contradicts the ordering the closer now owns, where the archive
follows the merge as its own commit. Cite it for the marker only.

## The root comes from the cwd

Every command reports it — `openspec list --json` ends with
`"root": {"path": …, "source": "nearest"}` — and "nearest" is literal: it walks
up from the current directory to the first `openspec/` it finds. There is **no
`--directory`, `-C` or `--root`**. `--store` takes a registered kebab-case store
id, not a path.

Agents work in worktrees, so this bites immediately: run from the main checkout
and a change in a worktree is simply not listed. Run `openspec` from inside the
worktree — `cd <dir> && openspec …` with no path argument after the `cd` is a
shape the permission checker accepts. **Check the reported root before concluding
a change is missing or the CLI is broken.**

## Reading two capabilities together

**A contradiction between capabilities is invisible until they are merged.**
`keystore` and `posting-capability` shipped in one change, each internally
consistent, jointly demanding a distinction the design deliberately does not
provide: a wrong passphrase told apart from a tampered ciphertext, which an AEAD
tag structurally cannot do. Each delta was reviewed alone and neither had been
read beside the other.

Archiving is the first moment they sit in one contract, so it is the moment to
read them together. A sweep finding such a pair should resolve it, not leave the
next reader a contract with two answers.

**Two capabilities asserting one rule is already here.** `identity` and
`op-format` both carry an authenticity-is-not-authority requirement, both pin
derivation constants, and `identity` restates the key-to-author binding
`op-format` covers. Two copies drift and the reader who finds the stale one
cannot tell. Which capability owns each rule is a design call, not a sweep's.

The contrast shows the discipline works: `op-ordering` faced this against
`op-format`'s "An op carries no ordering field", **declined to restate it, and
said so in its Purpose** — naming the other requirement and the boundary.
`spec-backfill` did not, and produced the duplication above.

## Moving requirements between capabilities

When a second instance shows that requirements written for one capability are
really about a general one, they move: `REMOVED` from the old spec and `ADDED` to
the new, verbatim, in one change. OpenSpec has no move or rename, so the
extraction is composed from those primitives. Do it when the generality is
demonstrated, not predicted.
