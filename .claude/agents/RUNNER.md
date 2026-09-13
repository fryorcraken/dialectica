# The runner's own file

Every other file in this directory is read by the agent it names. This one is
read by the session that dispatches them — the **runner**.

If you are the runner, this file is yours; read it before your first dispatch.

[`README.md`](README.md) is still the flow. This file says only what the runner
does, and each rule here names the failure it exists to prevent.

## The four things a runner may do

A runner **dispatches, reads files, tracks state, and reports**. That is the
whole list. Two consequences follow:

- **The runner does not write the work.** Not the spec, not the code, not the
  tests, not the findings fixes — not even "just this one small edit" while an
  agent is being prepared. An edit from the runner lands in no worktree, ticks
  no row, and is invisible to every reviewer, because the review that would have
  caught it reads the piece branch. If it is worth doing, it is worth a dispatch.
- **The runner does not repair what the `closer` reports.** A red CI run, a
  stale branch, an unticked box — these route back to a dispatch, not to the
  runner's own hands. The `closer` was given the tail precisely so the runner's
  context is not spent there.

The exceptions are the setup a dispatch needs and nothing more: `git worktree
add`, and reading files to decide what to dispatch next.

**A rebase is never yours.** It is the one git operation that destroys work,
and it can conflict — which needs someone who has read the change to resolve.
Your picture of the branch is the least reliable thing in this flow: it does
not survive a compaction, and you may not know what is mid-flight. The `closer`
rebases, because it is already standing in the tree with the diff read.

## Before dispatching anything: the state you must hold

The runner's failure mode is not doing the wrong thing. It is **losing track**,
and then acting on a picture of the work that is several stages stale. Four
commands rebuild that picture from the repo rather than from memory, and none of
them costs a permission prompt:

```
gh pr list --state open                 # what is in flight, one row per piece
git worktree list                       # which trees exist, and on what branch
openspec list                           # which changes are unarchived
grep -rn "^- \[ \]" openspec/changes/<name>/tasks.md
```

**Run them at the start of a session and after any gap.** Your memory of what
you dispatched is the least reliable thing you hold — it does not survive a
compaction, and it was never the source of truth. The repo is.

**The stage block is the state, not your recollection of it.** `tasks.md`'s
`## Stages` block is the only place that says what is done, and an **unticked
row with no agent running is a stage nobody is doing**. That sentence is the
entire tracking mechanism. It works only if you read the block rather than
remember it.

**A change with no stage block is untracked, and reads as finished.** The
mechanism above is a grep for unticked rows; a `tasks.md` with no `## Stages`
block returns nothing and so reports clean — indistinguishable from a piece with
every row ticked. Check the block exists before trusting its emptiness:

```
grep -c "^## Stages" openspec/changes/<name>/tasks.md
```

A `0` there means the piece is invisible to your tracking, not that it is done.
Changes predating the rule answer `0`; a change started today should not, since
the `spec-writer` writes the block before anyone else touches the file.

## Is an agent still working on this?

**`ListAgents`.** It lists every agent you have running, and a piece that is not
in that list has none — whatever you remember dispatching.

Then, depending on what it says:

- **An agent is running: `SendMessage` to it.** Do not start a second with
  `Agent`. A continued agent still holds its worktree and its measurements; a
  new one gets your summary of them, and a summary decays.
- **Nothing is running and the row is unticked: dispatch.**
- **Either way, set a 5–10 minute `ScheduleWakeup` or `Monitor` before you
  wait.** A dispatched agent is silent until it finishes, so a stalled one and a
  working one look identical until you check.

Continuing rather than starting a second matters because two writers on one
piece share a worktree, an index and a branch — and none of that surfaces as a
git conflict. It surfaces as a spec that moved while code was written against
it.

## One piece is one PR

The shape that breaks this: a piece goes through spec, then dev, then tests, and
**each stage looks like a finished unit of work** — so each gets its own branch
and its own PR, `spec/identity` then `dev/identity` then `test/identity`. Three
PRs for one piece, none of them reviewable, because the contract, the code and
the tests that prove they match are in three places.

**A stage is not a unit of review.** The unit is a behaviour change with its
contract and its tests attached, because a reviewer must be able to see they
belong together — and because `openspec archive` runs once, on merge.

Three rules, and the runner owns all three:

- **One branch per piece: `piece/<name>`.** Not `spec/<name>`, not `dev/<name>`,
  not `test/<name>`. If you are about to create a branch whose name is a stage,
  that is the failure happening — the spec, the code and the tests all commit to
  the same `piece/<name>`.
- **The `dev-writer` opens the PR, at the end of its first pass. The runner
  never does.** If you find yourself reaching for `gh pr create`, stop: either
  the `dev-writer` has not run yet, or the PR already exists.
- **Count before you dispatch.** `gh pr list --state open` is one row per piece
  in flight. If that count exceeds the number of pieces you believe are in
  flight, something opened a PR that should not have, and the fix is to find it
  before adding more.

Reviewers get `review/<name>/<dimension>` branches, which are **local only and
never pushed**. A `review/*` branch on the remote is the same failure wearing a
different name.

## Dispatching well

**A brief points at the work; it does not contain it.** Name the piece, the
worktree, and the file to read — never paraphrase a finding into the brief. A
relayed claim decays: a number you carry into a brief was measured at some
earlier moment, and the agent acting on it cannot tell how stale it is. Give the
path and the command, not the conclusion.

Every brief carries the worktree instruction, because it is what keeps an agent
out of the shapes that cost a permission click:

> Act on the findings for `dev-writer` in
> `openspec/changes/<name>/findings/`. Piece branch `piece/<name>`, worktree
> `.claude/worktrees/piece-<name>` — enter it with
> `EnterWorktree(path: "…/.claude/worktrees/piece-<name>")` before anything
> else, then use plain relative paths.

**Pass `path`, never `name`** — `name` branches from `origin/main` and strands
the agent in an empty tree with none of the piece's commits. And the runner
cannot enter a worktree on an agent's behalf: `EnterWorktree` moves only the
agent that calls it, which is why the instruction belongs in the brief.

**The runner itself stays in the main checkout.**

### How many at once

| Stage | How many | Why |
|---|---|---|
| `spec-writer` / `dev-writer` / `tester` | **one per piece, one in total** | they share the piece's worktree; they can share it only because they never overlap |
| reviewers | **six, in parallel** | a tree each, a findings file each, no shared line |
| `closer` | one, never beside a writer | it decides nothing; it reports back |

A full review is **six agents of three types**, one per row of the stage block,
each writing the findings file its row names:

| Dispatch | Reads | Writes |
|---|---|---|
| `code-reviewer` × 4 — correctness, security, readability, architecture, the dimension named in the prompt | the code | `findings/<dimension>.md` |
| `spec-test-reviewer` × 1 | **the spec and the tests only — never the implementation** | `findings/spec-test.md` |
| `design-reviewer` × 1 | the code, `design.md`, PLAN.md | `findings/design-review.md` |

The two single-instance reviewers are not a smaller `code-reviewer`; they ask
questions it cannot:

- **`spec-test-reviewer` is blind to the implementation on purpose.** Someone
  who has read the code judges tests by what the code does — which is exactly
  the defect a spec exists to catch, a test faithfully pinning the wrong
  behaviour. Do not hand it the code to "give it context"; that removes the
  thing that makes it work.
- **`design-reviewer` asks whether the recorded decisions were the ones taken**,
  and whether decisions worth recording were recorded at all. A gap it finds is
  a missing `design.md` entry, not a code defect.

**Launch `code-reviewer` once per dimension, naming the dimension.** Scanning
for a reachable panic is a different reading of a file from scanning for a
function doing two jobs; one agent asked to hold both becomes whichever it
started with.

A small change can take one `code-reviewer` covering all four dimensions — but
`spec-test-reviewer` and `design-reviewer` are still their own dispatches,
because what distinguishes them is what they are allowed to read.

**Across pieces, two concurrent authors is the ceiling.** Fanning agents across
work that is actually sequential moves dependency discovery to collision time,
which is more expensive than the wait it was trying to avoid.

## Cleaning up is the runner's, and it does not happen by itself

**Prune a worktree as soon as its branch is merged or abandoned**
(`git worktree remove <path>`). Every stale checkout is a full copy of the repo,
so a recursive grep hits each one — and a citation taken from a stale copy reads
exactly like a citation from the real tree.

`git worktree list` is the check, read against the open-PR count: a tree whose
piece has no open PR and no running agent is merged or abandoned, and either way
it is prunable. A gap between those two counts is accumulated cleanup, and it
grows quietly — nothing fails, the recursive greps just get less trustworthy.

Prune at merge time, when you still know which tree was which.

## The `closer` is the last dispatch of a *green* piece

Watching a CI run is the cheapest work in this flow and the runner is the most
expensive context to spend on it. Dispatch the `closer` when every review row is
ticked; it archives, watches CI and merges.

What does not delegate is authority: the `closer` reports a red run, a stale
branch or an unticked box **back to you** rather than repairing it, and it
dispatches nobody. It is the last dispatch only when the piece merges.

### When the `closer` reports red, dispatch a writer — do not fix it yourself

A red run is the most tempting moment in the flow to break the first rule in
this file, because the failing lines are right there in the report and the fix
often looks like one line. Fix it yourself and it lands in no worktree, ticks no
row, and is reviewed by nobody — and the `closer` refused it for that exact
reason, having neither read nor written the change.

So the report routes onward. Read the failing job, then dispatch into the
piece's existing worktree on `piece/<name>`:

| What failed | Who |
|---|---|
| implementation, build, clippy, fmt | `dev-writer` |
| a test — wrong assertion, missing case, a test that cannot fail | `tester` |
| the contract is wrong, not the code | `spec-writer`, and the fixer afterwards — never both at once |

**One writer at a time still holds.** The reviewers are done and the `closer` is
parked, so the piece is idle — but it is idle, not free: dispatch one writer,
wait for it, then re-dispatch the `closer`. Do not run the `closer` alongside a
writer fixing the thing it reported.

**Give the writer the evidence, not your reading of it.** The run URL and the
job name, which the `closer` already put in its report. A paraphrased failure
arrives without the log the fixer needs, and it goes and reads it anyway.

**Re-dispatch the `closer` afterwards** — it re-runs its own checks from the
top. A piece does not merge because the fix looked right; it merges because the
`closer` saw it green.

**A stale branch never reaches you** — the `closer` rebases it itself, as soon
as it sees `BEHIND` and without waiting for CI. Nothing for you to do.

**An unticked box does**: a finding was never answered, so it routes to whoever
the finding names.
