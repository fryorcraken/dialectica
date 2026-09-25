# The runner's own file

Every other file in this directory is read by the agent it names. This one is
read by the session that dispatches them — the **runner**. Read it before your
first dispatch; [`README.md`](README.md) is the flow itself.

## What a runner does

**Dispatch, track state, report.** That is the whole list.

- **You do not write the work** — not the spec, code, tests or findings fixes,
  not even one small edit while an agent is being prepared.
- **You do not rebase.** The `closer` does it.
- **You sit in your piece's worktree, and you run one piece.**

Yours besides dispatching: `git worktree add --no-track` (the flag is
load-bearing — see "Create worktrees with `--no-track`"), removing each agent's
worktree once its work is cherry-picked, the reading below, and the stage
block's re-review row — the one row that records a decision of yours rather
than an agent's work (step 3 of "From the `dev-writer`'s hand-back to the
merge").

## One runner per piece, sitting in that piece's worktree

**Your HEAD is the fork point for every agent you dispatch.** With
`worktree.baseRef: "head"` (see README.md), an agent dispatched with
`isolation: "worktree"` gets a tree cut from wherever your session's HEAD is.

So **enter your piece's worktree once, with `EnterWorktree(path: <absolute
path>)`, and stay there.** A session moving *itself* is the case the tool is
built for; it is dispatched agents that cannot do it, for reasons README.md
keeps.

**Never check out a different piece's branch in this session.** Doing so
silently forks the next dispatched agent from the wrong piece — no error, no
warning, the agent just works confidently on the wrong code.

**Do not run two pieces from one session.** Start a second session in the
second piece's worktree instead.

### What you read, and what you only point at

You read exactly enough to decide the next dispatch:

| Read | For |
|---|---|
| `openspec/changes/<name>/tasks.md` — the `## Stages` block | which stage is next, and whether anyone is on it |
| `ls openspec/changes/<name>/findings/` — **the filenames** | whether a reviewer has reported, and which dimension |
| `grep -rn "^- \[ \]"` over `findings/` | whether anything is unanswered, as a count |
| the `closer`'s report | whether the piece closed, or what stopped it |

**You do not read the findings themselves, and never quote one into a brief.**
Name the file and let the agent read it. A finding carries the measurement that
backs it; a paraphrase arrives without that, and the content occupies your
context twice — once from the report, once rewritten into the next brief. That
crowding is what loses the state you are supposed to be tracking.

The same holds for `design.md`, `proposal.md`, the GitHub issue, the spec and
the code: point at them. If you are reading a diff to judge whether it is
right, that is a reviewer's dispatch, not your reading.

## Rebuild the state before you act on it

Your memory of what you dispatched does not survive a compaction and was never
the source of truth. Run these at the start of a session and after any gap —
none costs a permission prompt:

```
gh pr list --state open                 # in flight: one row per piece
git worktree list                       # which trees exist, on what branch
openspec list                           # which changes are unarchived
grep -rn "^- \[ \]" openspec/changes/<name>/tasks.md    # what is left
```

`tasks.md`'s `## Stages` block is the state. **An unticked row with no agent
running is a stage nobody is doing** — that is the whole tracking mechanism.

**A change with no stage block is invisible to it.** The grep returns nothing,
which reads exactly like every row ticked. Confirm the block exists before
trusting an empty result:

```
grep -c "^## Stages" openspec/changes/<name>/tasks.md
```

`0` means untracked, not done. Changes predating the rule answer `0`; a new one
should not, since the `spec-writer` writes the block first.

A struck-through row keeps its empty box, so read the strike, not the box.

## Is an agent still working?

**`ListAgents`.** A piece not in that list has no agent, whatever you remember.

`ListAgents` and `ScheduleWakeup` are available to an **interactive main-loop
session** — which the runner is — and not to a subagent. So a reviewer sent to
check this file cannot see them and will report them as missing; that is a fact
about who dispatched it, not an error here. `Monitor` is the fallback if you ever
find they are genuinely absent.

- **One is running → `SendMessage` it.** Never start a second with `Agent`: two
  writers on one piece fork from the same HEAD and neither sees the other's
  commits, which surfaces not as a git conflict but as a spec that moved while
  code was written against it — there is no conflict, because they were never in
  the same tree. A
  continued agent also still holds its measurements, where a new one gets your
  summary of them.
- **None running, row unticked → dispatch.**
- **Either way, set a 5–10 minute `ScheduleWakeup` or `Monitor`.** A dispatched
  agent is silent until it finishes, so a stalled one looks like a working one.

## One piece is one PR

The shape that breaks this: each stage looks like a finished unit of work, so
spec, dev and tests each get a branch and a PR — leaving the contract, the code
and the tests that prove they match in three places, none reviewable. **A stage
is not a unit of review**, and `openspec archive` runs once, as a commit on the
piece branch that rides the same PR.

- **One branch per piece: `piece/<name>`.** A branch named for a stage is the
  failure happening.
- **The `dev-writer` opens the PR**, as the last act of its first pass, having
  pushed its own commits straight to the remote `piece/<name>` ref. It does not
  wait for your cherry-pick — [`dev-writer.md`](dev-writer.md) states the
  sequence and owns it. **If you are reaching for `gh pr create`, either the
  `dev-writer` has not run yet or the PR already exists**; check with `gh pr list
  --head piece/<name>` rather than creating a second one.
- **Your cherry-pick is still yours, and it is not what puts the work on the
  remote.** The `dev-writer` has already pushed the commits; you cherry-pick so
  that *your local HEAD* carries them, because that HEAD is the fork point for
  every agent you dispatch next. Skip it and the next writer forks from a tree
  missing the previous one's work — pushed or not.
- **Count before dispatching.** `gh pr list --state open` is one row per piece;
  more rows than pieces means something opened a PR that should not have.

**No `worktree-agent-<id>` ever appears on the remote** — a harness-named branch
there is the same failure as a reviewer branch reaching it, renamed. That is a
rule about the *ref name*, not about who may push: the `dev-writer` and `closer`
both push their tip **to `refs/heads/piece/<name>`**, which creates no agent
branch on the remote. Agent branches are named by the harness rather than by you,
so you learn each one from the agent's report and cherry-pick from it.

**Do not rename or re-point a branch with an open PR.** A PR's head ref is
immutable, and every workaround loses something; open a new PR on the correctly
named branch and close the old one, saying where the work went. The README's
branch section has the specifics.

## Dispatching

**A brief points at the work; it does not contain it.** Name the piece, the
worktree and the file to read. Never paraphrase a finding: a number you carry
into a brief was measured earlier and the agent cannot tell how stale it is.

**Dispatch with `isolation: "worktree"`.** The agent then arrives in its own
tree, forked from your HEAD, with a working directory it does not have to correct
— so the brief carries no worktree instructions at all:

> Act on the findings for `dev-writer` in
> `openspec/changes/<name>/findings/`. Piece branch `piece/<name>`. Commit to
> your own branch and report its name, so the work can be cherry-picked.

**Keep `git -C` and `EnterWorktree` out of your briefs.** An agent already in the
right place needs neither, and a brief carrying them sends it hunting for a
problem it does not have. The explanation stays in README.md, where a reader who
meets the refusal can find it.

**Do not phrase an instruction in a way that invites a chain.** "`cargo test`
from `dialectica/rust-lib/`" reads as `cd dialectica/rust-lib && cargo test`,
which costs an approval click even though `cargo test` is allow-listed. Name the
directory as its own step, or give a `--manifest-path`.

**Address this repo's agents unqualified** — `code-reviewer`, not
`agent-skills:code-reviewer`. The plugin ships a similarly-described reviewer
carrying none of this repo's traps.

**What you must still ask for is the branch name.** The agent lands on a
harness-named `worktree-agent-<id>`, not on `piece/<name>`, so its commits need
cherry-picking onto the piece — and the name is assigned by the harness rather
than chosen by you. Have the agent report it rather than guessing it.

**The isolation does not always take, and the agent is told to stop when it
doesn't.** An agent dispatched with `isolation: "worktree"` has landed in the
main checkout on the piece branch instead — so every agent file now has it run
`pwd` and `git rev-parse --abbrev-ref HEAD` and refuse to mutate or commit if the
answer is wrong. **An agent reporting that is doing the right thing: do not
re-dispatch it with instructions to work around it.** There is no brief that
fixes this, because the agent cannot place itself. Check `.claude/settings.json`
still carries `worktree.baseRef: "head"`, and if the tree is genuinely wrong,
stop and raise it with the user rather than dispatching into the main checkout.

A mutating reviewer that ran without the check would break the tree your own HEAD
points at, which is why the guard is in every file rather than in the brief.

**Cherry-pick before you dispatch the next agent, and make sure your HEAD carries
it.** Every dispatch forks from *your HEAD*, so an agent launched before
the previous one's work has landed on your branch gets a tree without it. It will
then rewrite, duplicate or contradict work it cannot see, and nothing fails —
there is no conflict, because the two agents were never in the same tree. The
sequence per agent is: hand-back → cherry-pick onto `piece/<name>` → remove the
agent's tree → dispatch the next.

Reviewers are the exception that proves it: six run concurrently precisely
because they only *read* the code, so forking them all from the same HEAD is
correct. It is writers that must be serialised.

**A dispatched agent cannot be put inside a pre-existing worktree.** Not "usually
fails" — two probes measured both routes and both fail, the second one *silently*
until the agent's first Bash call. **The transcripts live in
[`README.md`](README.md)**, under "Why the prohibition is written down anyway"
and the `Works?` table beside it, in one place only because two copies of a
measurement drift.

The operational consequence is short:

- **Dispatch with `isolation: "worktree"` and let the agent be.** That route
  works completely and is what this flow runs on.
- **Do not reach for `EnterWorktree` on an agent's behalf, and do not put it in a
  brief.** The tool moves only the session that calls it, so you could not do it
  for an agent even if it were correct.
- **Do not read the first probe's refusal as a hint.** Its message names the
  precondition `isolation: "worktree"` establishes, which invites exactly the
  combination probe 2 measured failing — isolation *plus* an `EnterWorktree` call
  across into the piece tree. Isolation alone never crosses, so nothing breaks.

### The setting this depends on, and how it fails

`worktree.baseRef: "head"` lives in `.claude/settings.json`, which is **tracked**
and so travels with a clone.

**Nothing fails when it is missing.** Agents are simply cut from
`origin/<default-branch>` instead of your HEAD, hold none of the piece's commits,
and work confidently on the wrong code. No error, no warning. If an agent reports
a fork point that is not your HEAD, or reports files that should exist as
missing, check that file before investigating anything else.

It is the user's file. **Do not edit it**; if it is absent, say so rather than
creating it.

Two things this setting does *not* change, so you do not go looking for them: the
agent's branch is created with no upstream, so the `--no-track` hazard below does
not arise on it; and the setting is global, applying to every
`isolation: "worktree"` dispatch with no per-dispatch override.

### How many at once

| Stage | How many |
|---|---|
| `spec-writer` / `dev-writer` / `tester` | **one in total**, not one each — cherry-pick and commit before dispatching the next, or it forks from a HEAD without the previous one's work |
| reviewers | **six, in parallel** — a tree and a findings file each |
| `closer` | one, never beside a writer |

A full review is six agents of three types, one per stage-block row:

| Dispatch | Reads | Writes |
|---|---|---|
| `code-reviewer` × 4 — correctness, security, readability, architecture, named in the prompt | the code | `findings/<dimension>.md` |
| `spec-test-reviewer` | **spec and tests only — never the implementation** | `findings/spec-test.md` |
| `design-reviewer` | code, `design.md`, the change's GitHub issue | `findings/design-review.md` |

The last two are not smaller `code-reviewer`s:

- **`spec-test-reviewer` is blind to the implementation on purpose** — someone
  who has read the code judges tests by what the code does, which is the defect
  a spec exists to catch. Do not hand it the code to "give it context".
- **`design-reviewer`** asks whether the recorded decisions were the ones taken.
  A gap it finds is a missing `design.md` entry, not a code defect.

**Name the dimension** when launching a `code-reviewer`: one agent asked to hold
two becomes whichever it started with. A small change can take one covering all
four — but the other two are still separate dispatches, because what
distinguishes them is what they may read.

**A change with no source diff still gets all six.** Agent files, prose and
config are reviewable material; treating "no code" as an exemption is how this
flow's own adopting change nearly shipped with `code-reviewer` skipped.

**Two concurrent authors across pieces is the ceiling.** Fanning agents across
sequential work moves dependency discovery to collision time.

## Create worktrees with `--no-track`

```
git worktree add --no-track -b piece/<name> .claude/worktrees/piece-<name> origin/main
```

**The flag is what stops the piece branch being configured to push to `main`.**
Without it, `git worktree add <path> -b piece/<name> origin/main` branches from a
remote-tracking ref, and git's `branch.autoSetupMerge` default then writes
`remote = origin` and `merge = refs/heads/main` into the new branch's config. The
branch is set up to push to `main` from the moment it exists.

This is the cause of the bare-`git push`-lands-on-`main` warning that this file
and `CLAUDE.md` both carry. Measured: a branch created without the flag has
`merge refs/heads/main` in its config, and `git push origin piece/<name>` from it
was **rejected by branch protection for `refs/heads/main`**, going through only
with a fully-qualified refspec. The lesson is about branch creation, not about
the push form.

**Check it with `git config`, not `git branch -vv`.** `branch -vv` cannot catch
this: it prints `[origin/main]`, and nothing in that output tells an intended
upstream from a wrong one. The positive signal is:

```
git config --get-regexp "^branch\.<name>"
```

**returning nothing.**

A branch created this way has no upstream, so a push names the refspec in full:
`git push origin refs/heads/piece/<name>:refs/heads/piece/<name>`.

## Prune worktrees at merge time

`git worktree remove <path>` as soon as a branch is merged or abandoned. Every
stale checkout is a full copy of the repo, so a recursive grep hits each one —
and a citation from a stale copy reads exactly like one from the real tree.

`git worktree list` read against the open-PR count is the check: a tree with no
open PR and no running agent is prunable. The gap grows quietly, since nothing
fails — the greps just get less trustworthy. `git worktree prune` clears entries
whose directories are already gone.

**Check merged-ness with `gh pr list`, not `git branch --merged`** — this repo
squash-merges, so a squashed branch never looks merged to git.

**Removing each agent's worktree is yours.** An agent cannot remove its own
tree: it is standing in it, and `git worktree remove` refuses the directory you
are in. The sequence after an agent hands back is cherry-pick its commits off
its branch, then remove its tree.

**You keep a tree while something may still need reading** — re-checking a
finding against the exact tree that produced it, comparing two reviewers'
citations, recovering a mutation an agent left uncommitted — and `--force`
destroys all of it. You are the only party that knows whether any of that is
still wanted, which is why the removal is yours rather than each agent's.

Agent trees accumulate faster than piece trees, one per dispatch rather than one
per piece, so `git worktree list` is worth running at the end of each review
round rather than at merge time.

## From the `dev-writer`'s hand-back to the merge

One sequence, in order. Every step is a dispatch, so the rules above hold at
each one: one writer at a time, and cherry-pick before the next dispatch.

**1. Route what the spec left unsaid, before the `tester`.** If the
`dev-writer`'s hand-back names any `NO SPEC:` marker, or any behaviour decision
it made where the spec was silent, **dispatch the `spec-writer` next.** Point
its brief at the markers with `git grep -n "NO SPEC:"` rather than listing
them. A decision reported without a marker is in no file, so quote the
hand-back's sentence verbatim — that is the one thing here you hold the only
copy of.

Dispatch a **fresh** `spec-writer`, not the one that wrote the spec: that one's
tree was forked before the `dev-writer`'s commits existed, so it cannot see the
markers it is being asked to judge.

Then the markers and tests are brought into line with the new spec text: by the
`dev-writer` if the `spec-writer` changed the behaviour, and then the `tester`;
otherwise by the `tester` directly. Either brief names the `spec-writer`'s
commit.

**Do not put markers to the owner.** Deciding them is the `spec-writer`'s job.
Escalate only what it returns as a product decision, and state the choice: the
options, and what each would make the system do.

No marker and no reported decision: the `tester` is next.

**2. The `tester`, then the review round** — six reviewers in parallel, as "How
many at once" sets out.

**3. Every commit made after the review round is reviewed before the `closer`
runs.** That includes:

- a writer's pass answering findings;
- a rewrite or new file made on an owner instruction;
- a `spec-writer` callback;
- a fix for a red CI run (step 4 sends it back here).

What counts is **every commit that changes something which merges**. A commit
that only flips boxes — a finding's outcome in `findings/`, which is deleted
before merge, or a stage-row tick — needs no review. A commit moving reasoning
into `design.md` does.

**A green CI run and the author's own mutation runs are not review, and a
warning the owner did not answer is not consent.** Two pieces merged unreviewed
code on the same day with every gate green, one of them after a re-pass was
offered as optional and nobody answered.

**Size the re-review yourself** — it is a judgement, not a fixed rule, because
what lands after review ranges from one line to a rewrite:

- **Lanes and count.** Re-dispatch the lanes whose ground the new commits touch
  — spec-test when tests or scenarios changed, design when `design.md` did,
  correctness or security when code did. A finding answered exactly as its
  reviewer asked may need only that reviewer to confirm it. A rewrite needs all
  six again.
- **Model.** The Agent tool's `model` override sets it per dispatch. A narrow
  confirmation can run on a smaller model; a rewrite or a security-relevant
  change gets the strongest one.
- **The brief** names the commit range to read — for `spec-test-reviewer`, only
  the spec and test files in it, since it stays blind to the implementation —
  and says the reviewer's stage row is already ticked and stays so, and that
  new findings are appended as boxes to its existing findings file.

A re-review can raise findings of its own, whose fixes are commits, which need
the next round. Each round covers only what landed since the last, so rounds
shrink.

**Record the call**, in your report and in the stage block's re-review row. One
indented line per round under that row: the commit range, what landed, the
lanes and model each ran on, and why that size. A round you decide to skip gets
a line too, with its reason — a skip is a decision, and an unrecorded one looks
exactly like a forgotten one:

```markdown
- [ ] re-review: every commit after the review round — runner
      round 1 `a1b2c3d..e4f5a6b` findings pass — correctness, spec-test (role default models): two handlers and their tests changed
      round 2 `e4f5a6b..0c9d8e7` red-CI fix — skipped: `cargo fmt` whitespace only, no token changed
```

**Tick the row when no commit that merges is unreviewed.** The record and the
tick are commits you make in your own tree, on `piece/<name>`, before the next
dispatch forks from it. **If a commit lands after it is ticked — a red-CI fix
— untick it** and add the next round's line: the `closer`'s Step 1 refuses to
run while a row other than its own is unticked, and that is the only thing that
lets it see the fix was never read.

**When unsure, re-review.** The `closer` waits.

**4. The `closer`** — below.

### The `closer`, and what comes back

Dispatch it when every row above its own three is ticked or struck, the
re-review row included; it rebases if behind, archives, watches CI and merges.
Watching a run is the cheapest work in the flow and yours is the most expensive
context to spend on it.

It decides nothing and dispatches nobody. Two things come back:

**A red run.** This is the most tempting moment to break the first rule in this
file — the failing lines are in the report and the fix looks like one line. The
`closer` refused it for the reason you should: it neither read nor wrote the
change. Dispatch a fixer the ordinary way — `isolation: "worktree"`, its own tree
forked from your HEAD, its commits cherry-picked back. There is no special
dispatch shape for a fixer, and **nothing goes into the piece's own worktree but
you**: putting a dispatched agent there is the failure the whole "Dispatching"
section above measures. Who to send:

| What failed | Who |
|---|---|
| implementation, build, clippy, fmt | `dev-writer` |
| a test — wrong assertion, missing case, a test that cannot fail | `tester` |
| the contract, not the code | `spec-writer`, then the fixer — never both at once |

One writer at a time still holds: the piece is idle, not free. Give the writer
the run URL, not your reading of it. Then **the fix goes back through step 3**
— untick the re-review row and size a round for it — **and only then
re-dispatch the `closer`**. A piece merges because the fix went through step 3
and the `closer` saw it green, not because the fix looked right.

**An unticked box**, meaning a finding was never answered: it routes to whoever
the finding names.

A stale branch does *not* come back — the `closer` rebases it itself.
