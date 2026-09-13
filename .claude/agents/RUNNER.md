# The runner's own file

Every other file in this directory is read by the agent it names. This one is
read by the session that dispatches them — the **runner** — which until now was
the only participant in the flow with no file of its own, and made the mistakes
that follow from that: it spawned no agent and did the work itself, it lost track
of whether an agent was still running, and it opened **fourteen PRs for five
pieces**, twelve of which were closed unmerged and redone.

Those three are one failure. **The runner's obligations were written as asides
inside documents addressed to subagents**, so the rule that would have prevented
each was in a file the runner had no reason to open. If you are the runner, this
file is yours; read it before your first dispatch.

[`README.md`](README.md) is still the flow. This file says only what the runner
does, and each rule here names the failure it exists to prevent.

## The four things a runner may do

A runner **dispatches, reads files, tracks state, and reports**. That is the
whole list. Two consequences worth stating because both have been broken:

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
Both changes in flight when this file was written predate the rule and answer
`0`; a change started today has no such excuse, and the `spec-writer` writes the
block before anyone else touches the file.

## Is an agent still working on this?

This is the question the runner got wrong most often, and the honest answer is
that **you cannot tell from your own context** — a dispatched agent returns a
report when it finishes and is otherwise silent, so "I dispatched one" and "one
is running" feel identical from the inside.

So do not infer it. **Establish it:**

1. **`ListAgents`** — this is the direct answer, and it is cheap. A piece whose
   agent is not in that list has no agent running, whatever you remember.
2. **`SendMessage` to continue it, rather than `Agent` to start a new one.** A
   continued agent still holds its worktree and its measurements; a new one gets
   your summary of them, which is the decay this flow has been bitten by
   repeatedly. If an agent is running, talk to it.
3. **Set a reminder while you wait.** A 5–10 minute `ScheduleWakeup` or `Monitor`
   is what turns "I think something is happening" into a scheduled check. Without
   it a stalled agent and a working one look the same for an hour.

**Never dispatch a second agent for a stage that already has one.** Two writers
on one piece share a worktree, an index and a branch — and none of that surfaces
as a git conflict; it surfaces as a spec that moved while code was written
against it. When in doubt, `ListAgents` first and dispatch second.

**A stage with no agent and no tick is yours to dispatch now.** That is the
signal the stage block exists to give you; the failure it caught here was a
piece that reached the edge of merge with zero reviewers.

## One piece is one PR — and the runner is who breaks this

This event is worth stating exactly, because the shape is seductive and it will
present itself again. On 2026-09-12, between 09:00 and 10:08, five pieces were
taken through spec, then dev, then tests. Each stage looked like a finished unit
of work, so each got its own branch and its own PR — `spec/identity` (#32), then
`dev/identity` (#40), then `test/identity` (#46). Fourteen PRs for five pieces;
twelve were closed unmerged and redone as `piece/*`.

Check it rather than believe it:

```
gh pr list --state all --limit 80 --json number,headRefName,state
```

The stage-named branches are still in that list, and the twelve `CLOSED` rows
are the cost.

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
relayed claim decays: four of this flow's own briefs carried numbers that were
stale or wrong by the time an agent re-derived them. Give the path and the
command, not the conclusion.

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
| reviewers | **up to six, in parallel** | a tree each, a findings file each, no shared line |
| `closer` | one, and last | it decides nothing; it reports back |

**Launch `code-reviewer` once per dimension** — correctness, security,
readability, architecture — naming the dimension in the prompt. One agent asked
to hold two dimensions becomes whichever it started with.

**Across pieces, two concurrent authors is the ceiling.** Fanning agents across
work that is actually sequential moves dependency discovery to collision time,
which is more expensive than the wait it was trying to avoid.

## Cleaning up is the runner's, and it does not happen by itself

**Prune a worktree as soon as its branch is merged or abandoned**
(`git worktree remove <path>`). Every stale checkout is a full copy of the repo,
so a recursive grep hits each one — and a citation taken from a stale copy reads
exactly like a citation from the real tree. This has already produced wrong
citations here.

`git worktree list` is the check, and the honest way to read it is against the
open-PR count: a tree whose piece has no open PR and no running agent is either
merged or abandoned, and either way it is prunable. When this file was written
that comparison gave 36 worktrees against 6 open PRs — the gap is what not
pruning at merge time accumulates to, and it is why a recursive grep from the
repo root now reads dozens of stale copies of every file.

Prune at merge time, when you still know which tree was which.

## The runner's last dispatch is the `closer`

Watching a CI run is the cheapest work in this flow and the runner is the most
expensive context to spend on it. Dispatch the `closer` when every review row is
ticked; it archives, watches CI and merges.

What does not delegate is authority: the `closer` reports a red run, a stale
branch or an unticked box **back to you** rather than repairing it, and it
dispatches nobody. A report from the `closer` is the start of your next
dispatch.
