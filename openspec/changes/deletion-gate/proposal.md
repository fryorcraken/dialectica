# A CI gate for the deletion a branch never meant to make

## Why

**Eight times on 2026-09-13, a branch's diff against `main` carried deletions of
files it never touched.** Not one was visible to CI.

A branch cut before a change landed and never updated carries "the file without
that change" as an intentional-looking deletion. A squash merge applies it. There
is no conflict, because nobody edited the same lines twice — the branch simply
predates them. Three PRs (#36, #42, #43) reached the edge of merge that way,
each carrying ~690-705 deletions of the seven agent files,
`docs/OPENSPEC-ARCHIVE.md` and three `## Purpose` sections without which
`openspec archive` aborts and writes nothing.

**`mergeStateStatus` reported `UNKNOWN` for all three.** Not `BEHIND`, not
`DIRTY`. Nothing in the PR view showed it.

The sharpest instance was PR #60, `piece/ui-onboarding`: all four required jobs
green, and a diff that would have deleted thousands of lines across
`closer.md`, `membership.rs`, `wire.rs` and the `stoa-membership` spec. A closer
following every rule except the hand-run diff merges that.

**That instance can no longer be measured, and the reason matters.** `#60` has
since been rebased onto `733544d`, so both diff forms now report 12 deletions
and the historical 7,071 figure is not reproducible from the repository today.
It is quoted here as a session observation rather than as something a reader can
re-derive — which is exactly the kind of number this project treats as a claim
rather than evidence. What *is* reproducible, on `origin/piece/op-transport` as
this is written, is the mechanism itself:

```
git diff origin/main origin/piece/op-transport --shortstat
  31 files changed, 6577 insertions(+), 7101 deletions(-)
git diff origin/main...origin/piece/op-transport --shortstat
  14 files changed, 6449 insertions(+), 37 deletions(-)
```

Two forms, one branch, a 7,064-line disagreement. Run those two commands before
believing anything below; they are the whole argument in six lines.

### Why the four existing jobs are structurally blind to it

They test **the merged result of a branch that is internally consistent**. The
tree compiles and the suite passes *because* the deleted code is genuinely absent
and nothing references it. `cargo test` on a branch missing `membership.rs`
measures a world where that deletion is correct.

This is the same class as the two blindnesses CLAUDE.md already records —
`cargo fmt --check` not following path dependencies, and `cfg(logos_scaffold)`
code that `cargo test` never compiles. A green gate that measured nothing.

## What Changes

- **A new `Lint` step fails a pull request whose diff deletes a file the pull
  request does not claim to delete.** The claim is a `Deletes: <path>` line in
  the PR body, one per deleted path.
- The step runs in `Lint`, which finishes in seconds, rather than as a fifth job.
  `Lint` is already one of the four required checks
  (`gh api repos/<owner>/<repo>/branches/main/protection`), so a step there is
  enforced with no branch-protection change; a new job would need one, plus a
  queue slot for what is a single `git diff`.
- `lint` gains `fetch-depth: 0`, because the merge-base form needs history.

### The three details that decide whether this works

**1. The range is three-dot, not two-dot.** `origin/main...HEAD` diffs against
the merge base — what this branch deletes relative to where it forked. The
two-dot form reports what `main` has gained since, which on a correctly merged
branch produced a **6,871-deletion false alarm** and reproduces today as the
7,101-vs-37 split quoted above. A gate that cries wolf is a gate someone
disables, so this is the difference between a check that survives and one removed
in a month.

**2. A shallow clone fails in the dangerous direction.** `actions/checkout`
defaults to `fetch-depth: 1`. With no history the merge base is unresolvable, and
the obvious implementation then reports zero deletions and **passes**. The step
therefore verifies the merge base resolves and fails loudly if it does not,
rather than treating an empty result as clean.

**3. No allowlist file.** A tracked list of permitted deletions is a
hand-maintained list guarding a property a command can check — the exact shape
`every_request_taking_method()` demonstrates going stale in
`dialectica-core/src/wire.rs`, where the doc comment has to *ask* each author to
add their method because nothing enforces it. The PR body is the right place
because it is where a human already explains the change, and it cannot drift out
of sync with the diff it describes: both are read at the same moment.

### The trigger is pull requests only, and that is a limit not an oversight

The workflow also runs on pushes to `main` and on tags, where there is no pull
request and therefore no body to read. The step is scoped to the
`pull_request` event and skips otherwise. It follows that **the gate never
inspects `main` itself** — it inspects proposals to change `main`, which is
where the defect lives, but a reader should not infer from a green `main` run
that this check ran at all.

## What it does not catch, stated plainly

**Seven of the eight firings, not eight.** The exception is the duplicate
`storage_dir` on `piece/ui-stoa-list`: a textual merge that kept *both* copies of
a refactor that landed at two offsets. That is an addition, not a deletion, and a
deletion gate is blind to it. Only Build LGX caught it — both copies are
`cfg(logos_scaffold)`, so `cargo test`, clippy and `cargo fmt` compile neither,
and three of four jobs went green over a tree that could not build.

Naming this is the point rather than an apology for it. A gate whose limits are
unwritten gets trusted past them, which is how "exit 0 on a gate that measured
nothing" happens in the first place.

**It also does not replace the closer's hand-run diff.** This gate proves a
branch deletes nothing unclaimed; it does not prove the branch's diff is the
change it says it is. A branch that *adds* something it should not still passes,
and so does one whose claimed deletions are claimed wrongly — the gate checks
that a path was acknowledged, never that acknowledging it was correct. Those are
different questions and only the second needs a reader. `closer.md` step 2 stays
as written.

**And a `Deletes:` line is an assertion by whoever wrote the body**, so the gate
converts a silent deletion into a stated one rather than into a reviewed one.
That is the intended trade: it moves the cost onto the rare change that genuinely
deletes, and makes the common accidental case loud.

## Capabilities

### New Capabilities

None, and this was checked rather than assumed.

`openspec list --specs` returns fourteen capabilities, every one of them a
contract over module behaviour — `op-format`, `stoa-membership`,
`module-wire-contract` and so on. **A CI gate is not behaviour of the dialectica
module**: it asserts nothing about what a peer sends, what core answers, or what
the view renders. There is no module that could fail such a requirement and no
test in any suite that could cover it, which makes it a scenario that cannot be
tested — the defect this project's own flow doc names first.

The existing `Lint` steps are the precedent and none of them is specified: "the
UI icon is a 256x256 PNG" and "scaffold.toml kept its comments" are both
repository hygiene enforced in CI with no capability behind them. The schema
agrees explicitly, naming "tooling" alongside pure refactors and docs as the
case `skip_specs` exists for.

**The change therefore declares `skip_specs: true`** in
`openspec/changes/deletion-gate/.openspec.yaml`, which is what `openspec
validate` requires of a change with no deltas. That file also carries
`schema: spec-driven`, **without which the marker is silently ignored** and
validation fails for having no deltas — two symptoms, one cause. The first draft
of this change omitted the file entirely and did not validate.

It still gets a change folder and a full stage block, because a piece with no
spec delta still needs reviewing — and without the block there is no unticked row
to say so.

### Modified Capabilities

None.

## Impact

- `.github/workflows/ci.yml` gains one step and one `with:` key.
- `.claude/agents/closer.md` gains two things: one rule on the closer authoring
  commits to a PR it would otherwise merge, and one line on archiving a
  `skip_specs: true` change. Both are folded in here rather than split out; the
  arguments are in the last two sections.
- `docs/OPENSPEC-ARCHIVE.md` gains the same zero-delta note in its reference
  form, beside the `## Purpose` traps.
- **Every open PR is affected on its next run.** At the time of writing every
  open branch is clean under the three-dot measure — the eight firings were
  caught and rebased by hand — so this should land green rather than firing
  immediately. Verify with the two-command pair above rather than trusting that
  sentence, which goes stale the moment `main` moves.
- A genuine file move trips it, correctly: a move is a deletion plus an addition,
  and saying so in the PR body is accurate rather than a workaround.
- `docs/PLAN.md` §10's "Deliberately not built" list gains nothing, because this
  is being built. Its adjacent claim that the `qml` job proves "nothing about
  `call()`'s JSON parsing" is now stale — `dialectica-ui/tests/` exists — but
  correcting it belongs to a change that touches that job, not to this one.
- Out of scope: detecting the duplicate-symbol class above, any change to the
  other four jobs, and any change to branch protection.

## Why the `closer.md` rule ships here rather than on its own

The closer authored a commit onto a PR it was pre-approved to merge. Its file
forbids fixing code and ticking others' boxes but never anticipated the closer
*authoring*, so nothing in it was broken — the case was simply absent.

**Folding it in.** The two changes share one subject: what a closer must do
before a merge that nobody else has read. This change adds the mechanical half
(a gate that reads the diff) and the rule adds the human half (a merge the
author may not approve), and separating them would produce a two-line PR whose
justification is this proposal. The counter-argument is real — a CI change and an
agent-file change are different review surfaces, and #59 landed the closer's file
on its own — but it is weaker here than the coupling, because a reviewer reading
either half alone would ask for the other.

The same reasoning carries the archive note in the section below, which arrived
from the same closer for the same reason: it found a gap in its own checklist
and refused to fix it itself. Three documentation edits that all answer "what
must a closer do about a merge or an archive nobody else has read" are one
review surface, not three.

The rule was drafted here as:

> The closer may author a fix to its own checklist or its own file, and must
> then treat that PR as one it cannot approve — it goes to the runner like any
> other. It must say in its report which commits on the PR are its own, since
> the runner reviewing it needs to know which parts had no second reader.

**That draft is wrong and was corrected by the owner before it shipped**, on the
grounds that *"the closer is supposed to archive so commits from the closer are
expected"*. Applied literally to the archive commit it is self-defeating:
`openspec archive` is the closer's own stage row, so every piece it archived
would become a piece it could not close, and the role deadlocks on its normal
path.

The axis is not authored-or-not, it is **dispatched or discretionary**. The
archive commit is what the closer was sent to write and is mechanical — a folder
moved, a delta merged — so it disqualifies nothing. A fix to its own checklist
is something it *decided* to write, had no second reader, and does disqualify it
from approving that PR. The checkable test is "was I dispatched to write this?"
rather than "is this routine?": the first can be checked against the brief, the
second is a judgement made by the agent with the most reason to answer
conveniently.

The report clause survives the correction unchanged and applies to the
discretionary kind. It is the sharpest part: without it the rule produces a
correct handoff that the runner cannot act on, because "this PR needs a second
reader" does not say *which lines* lacked one.

See `.claude/agents/closer.md` for the wording that shipped; this section is the
record of why the first version was wrong, which the file itself does not carry.

## Archiving a change that has no spec delta

**Neither archive document mentions this case.** `grep -c "skip_specs"` and
`grep -c "schema"` over `.claude/agents/closer.md` and
`docs/OPENSPEC-ARCHIVE.md` on `origin/main` return **zero in all four**. Since
#59 moved the archive stage row to the `closer`, that agent now reaches a
zero-delta change with nothing in its checklist to say what to do.

This is live rather than hypothetical: `piece/seed-store` (PR #66) and **this
piece** are both `skip_specs: true` with no `specs/` directory, so the first
closer to meet one is the one closing one of these two.

The cost is already paid once. The drafts of this proposal asserted
`skip_specs: true` in prose while `.openspec.yaml` did not exist, and
`openspec validate deletion-gate --strict` failed with "Change must have at
least one delta" — a message that names deltas and never mentions the marker.

### What the note must say, and one thing to verify first

Three claims, each checked rather than assumed:

- **A `skip_specs: true` change archives normally.** `archive` moves the folder
  to `openspec/changes/archive/<date>-<name>/`; there is simply no delta-merge
  prompt, because `openspec show <name> --json --deltas-only` reports
  `deltaCount: 0`. Nothing extra is required.
- **`schema:` must accompany the marker or it is silently ignored**, and the
  resulting `--strict` failure presents as two problems when it is one: the
  marker reported as ignored, then a failure for having no deltas. One cause.
- **There is also a `--skip-specs` CLI flag**, whose name collides with the YAML
  key (`openspec archive --help`). This is worth naming because the closer will
  otherwise have to guess whether it is required. With the marker set correctly
  it is **not** needed — validation passes and the delta count is zero without
  it — so the note should say the file is the mechanism and the flag is not a
  substitute for it.

**One thing the `dev-writer` must verify rather than inherit from me.** The
`core-e2e` change is the only archived zero-delta precedent, and
`git log --diff-filter=A -- openspec/changes/archive/2026-09-13-core-e2e/`
shows it landing **inside** the squash merge `b85111d`, not as a separate
post-merge commit. Every earlier change archived that way too, spec-bearing
ones included — so that is a pre-#59 habit, not evidence about zero-delta
changes specifically. Do not cite `core-e2e` as showing how the archive
*commit* should be made; cite it only for the marker. Re-run that command
before writing the note.

### Placement: both files, and what stops them drifting

Put the **full note in `docs/OPENSPEC-ARCHIVE.md`**, beside the `## Purpose`
traps in "Four traps, none visible from the files" — it is the same kind of
fact, a mechanical trap invisible from the artifacts. Put **one line in
`closer.md`** that says a zero-delta change archives normally and points at the
page.

Two copies drifting is a failure this repo has named, so the split is
deliberately *not* two copies. `closer.md` already carries the pattern and its
own justification for it:

> Most of this step's traps are there and none of them are visible from the
> files; this section does not restate them, because two copies drift and the
> reader who finds the stale one cannot tell.

So the anti-drift mechanism is that **only one file carries the reasoning**.
`closer.md` gets a pointer and a single fact — enough to stop an agent
mid-task concluding a zero-delta change cannot be archived — and every
detail, the `schema:` requirement and the flag collision included, lives in
one place. A pointer cannot contradict the page it points at.

The `closer.md` line, for the `dev-writer` to place in step 5 beside the
existing "read the page in full" instruction:

> **A change with no spec delta archives normally.** `skip_specs: true` in its
> `.openspec.yaml` is the marker, there is no delta-merge prompt to take, and
> the `--skip-specs` flag is not needed when the marker is set. The marker is
> silently ignored without a `schema:` key beside it — see
> `docs/OPENSPEC-ARCHIVE.md`.
