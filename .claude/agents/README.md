# The spec-driven flow

Role agents around [OpenSpec](https://openspec.dev)'s built-in `spec-driven`
schema: OpenSpec supplies the artifacts and their ordering, Claude Code the
agents. No custom tooling — subagents already give isolated context windows,
per-role models and tool limits.

> **If you are the session dispatching these agents, read
> [`RUNNER.md`](RUNNER.md) first — it is written for you, and this file is not.**
>
> Everything below is addressed to the agent it names.

## The documents, and what each is for

| Document | Question | Where it ends up |
|---|---|---|
| `docs/PLAN.md` | A short summary of what exists, and **what is not built yet** | Lives at `docs/`, edited forever |
| `proposal.md` | Why this change, which capabilities it touches | `changes/archive/<date>-<name>/` |
| `openspec/specs/` | **What** the system does — the behaviour contract | `openspec/specs/`, current |
| `design.md` | **How**, and **why this approach** (Decisions) | `changes/archive/<date>-<name>/` |
| `tasks.md` | The ordered checklist | `changes/archive/<date>-<name>/` |
| `findings/<dimension>.md` | What each reviewer found, and what was done about it | **Deleted before merge** — durable reasoning moves to `design.md` first |

A change in flight lives in `openspec/changes/<name>/`, and its `specs/` holds a
**delta**. `openspec archive` merges the delta into `openspec/specs/` and moves
the folder to `changes/archive/<date>-<name>/` — moved, not deleted, so finding a
past decision means grepping the archive.

**Archiving has enough traps to be worth its own page:
[`docs/OPENSPEC-ARCHIVE.md`](../../docs/OPENSPEC-ARCHIVE.md). Read it before you
archive, not before you start.** Archiving is the `closer`'s, and it runs as a
commit on the piece branch before CI and the merge — [`closer.md`](closer.md)
says why that ordering. `openspec` is installed; run
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
| `dev-writer` | spec, PLAN.md | `design.md`, `tasks.md`, code, tests-as-it-goes, **the PR** |
| `tester` | spec, inherited tests | the test suite |
| `spec-test-reviewer` | **spec + tests only** | findings |
| `design-reviewer` | code, `design.md`, PLAN.md | findings |
| `code-reviewer` | code | findings |
| `closer` | `tasks.md`, `findings/`, CI, the PR | deletes `findings/`, the archive commit |

## One piece of work is one branch and one PR

Every stage — spec, design, code, tests, review fixes — lands as **commits on one
remote branch, under one PR**.

Local branches are fine and every dispatched agent gets one. What never happens
is a *stage* reaching the remote on its own: only `piece/<name>` is ever pushed, so
`origin/<anything-else>` is a mistake, and a second PR on one piece is the failure
this section exists to stop.

**The unit of review is a behaviour change with its contract and its tests
attached.** A reviewer must be able to see they belong together, not be told so
by whoever is orchestrating. And `openspec archive` runs once, on merge — split
across several merges, the contract lands at a different time from the code that
honours it.

### One writer at a time; reviewers in parallel

**A piece has at most one `spec-writer`, `dev-writer` or `tester` running** — not
one of each, **one in total**. Each gets its own worktree, so this is not about
sharing a tree; it is about what the *next* agent forks from. Every dispatch is
cut from the runner's HEAD, so a second writer launched before the first's
commits are cherry-picked forks from a HEAD that does not contain them, and the
two diverge silently. Two further reasons, neither of which surfaces as a git
conflict either:

- **A spec must not move while code is written against it.** Run the pair together
  and the implementation answers a contract that changed underneath it, with
  neither agent knowing. This session did it — a scope reworded while four
  reviewers read the code implementing it. It holds on the way back too: when
  review routes a spec gap, stop the fixer, land the spec, restart the fixer
  against the new text.
- **`tester` mutates implementation code it does not own**, restoring after each
  mutation. A concurrent writer inherits the broken state or overwrites the
  restore, and neither is touching git when it happens.

**Reviewers run in parallel, three to five of them** — [`RUNNER.md`](RUNNER.md)'s
tier table says how many, read off what the change contains — each writing only
its own findings file. They get a worktree each because a reviewer running
`cargo mutants` breaks dozens
of lines to see whether a test notices — two sharing a tree read each other's
breakage as the author's, which has happened here.

**The runner creates each reviewer's worktree and names it in the dispatch**, and
the reviewer does not remove it — the runner does, once its work is
cherry-picked. Ambiguity here is silent and destructive both ways: an agent that
assumes it must make its own may instead mutate the piece's tree, and one that
assumes the tree is its to delete may `--force`-remove the state another finding
cites, or leave a full copy of the repo behind per dispatch by assuming somebody
else will clean up. **A reviewer that was not given a worktree path stops and
says so** rather than choosing either fallback.

### Branch names say which kind of branch it is

| Branch | Worktree | Whose | Holds |
|---|---|---|---|
| `piece/<name>` | the runner's | the runner, and the PR | **the** task branch, and the only one ever pushed. Work reaches it two ways: the runner cherry-picks every agent's commits onto its local copy, and the `dev-writer` and `closer` push a refspec to the remote copy |
| `worktree-agent-<id>` | one per dispatch | one agent | **local only, and named by the harness** — whatever that agent committed, cherry-picked onto the piece and never pushed |
| `main` | — | nobody | **no agent ever pushes here.** It takes commits through a PR only |

**Every dispatched agent gets its own worktree and its own branch**, cut from the
runner's HEAD. No agent stands in the piece's tree, so every agent's commits are
cherry-picked onto it — writers exactly as reviewers.

**The branch name is the harness's, not the runner's.** There is no
`review/<name>/<dimension>` to predict, so an agent reports the name it actually
landed on (`git rev-parse --abbrev-ref HEAD`) and the runner picks from that. A
name nobody recorded is work nobody can find.

**The PR is opened on `piece/<name>` and nothing else. Whichever ref it is opened
on, it is stuck with — and every workaround loses something.** A PR's head ref is
immutable:
`PATCH /pulls/<n> -f head=…` returns **200 and silently ignores the field**, and
`--base` changes the target, not the source. The rename endpoint
(`POST /branches/<old>/rename`) does follow open PRs — but it **auto-closes** one
whose *head* vanished, and if the target name already exists you must delete that
ref first, at which point the rename recreates it **at the old branch's tip and
silently drops anything the deleted ref held**. Six reviewers' findings went that
way here and were recovered only because the commit was still in a local reflog.

**Consolidating branches is not finished until the orphaned PRs are closed.**
Folding a branch into the piece leaves its PR open, describing work that now
lives somewhere else — so the PR list stops being a count of work in progress,
which is the only thing that makes a work-in-progress limit checkable. Before
closing one, prove it is redundant:

```
git log --oneline origin/<orphan> --not origin/piece/<name>
```

Empty means contained. **Non-empty means fold it first** — a commit cherry-picked
rather than merged shows here even though its content is in, so read the commits
rather than the count. Say in the closing comment where the work went, and keep
the branch.

**`piece/<name>` is pushed by two agents only, and never by cherry-pick.** The
`dev-writer` pushes it at the end of its first pass and opens the PR there; the
`closer` pushes it again after the archive commit. Both push a **refspec to the
remote piece ref** rather than checking the branch out — it is checked out in the
runner's worktree, and git refuses a branch checked out elsewhere. **When and how
the `dev-writer` does it is [`dev-writer.md`](dev-writer.md)'s**, stated once
there; this line points rather than restates, because two copies drift.

**Nobody pushes `main`, and the archive commit is not an exception.** It takes
commits through a PR only — `enforce_admins` is on, and a direct push is rejected
with `GH006`. The archive is the one that reads as though it might be exempt,
being a bookkeeping commit; it is not, and it goes onto the piece branch like
everything else. This page and `closer.md` both used to say otherwise, until a
closer tried it.

Cherry-pick rather than merge, so the task branch reads as a flat sequence rather
than six merge commits carrying six branches.

**Never `git add -A`** — commit named paths. Two reasons, and they are not the
same rule:

- **Sweeping up another agent's half-finished edit corrupts the branch you were
  working on.** This is the one that matters, because it is silent: the commit
  looks like yours, and the agent whose work you took has no way to see that it
  left.
- **A worktree collects build output that is not yours to commit** —
  `.scaffold/`, `target/`, `result-*` out-links, `./tmp/` scratch, a gitignored
  SDK symlink, and whatever is added to that list next. Noise, which a reviewer
  spots.

**This is the canonical copy of the artefact list**; each agent file states the
rule and points here rather than repeating the list, which is the part that
changes.

**Check `git config --get-regexp "^branch\.<name>"` before any git write, and
expect it to return nothing.** A bare `git push` has landed commits directly on
`main` here more than once, and the cause is the creation command: `git worktree
add <path> -b piece/<name> origin/main` branches from a remote-tracking ref, so
`branch.autoSetupMerge` writes `merge = refs/heads/main` into the new branch's
config. `git worktree add --no-track` is the fix and `RUNNER.md` carries it; a
branch made with the flag returns nothing from that `git config` call.

**`git branch -vv` does not catch this**, so do not reach for it as the check: it
prints `[origin/main]`, and nothing in that output distinguishes an intended
upstream from a wrong one.

A `--no-track` branch has no upstream, so push the refspec in full:
`git push origin refs/heads/piece/<name>:refs/heads/piece/<name>`.

CLAUDE.md's "Worktrees are not scratch" has the rest — in particular that the
stash stack is shared with every other worktree, so never bare `git stash pop`.

## Two files carry the state of a change

Each agent's own file says what it writes. These are the shapes everyone needs to
recognise, because everyone reads both.

**`tasks.md` opens with a stage block**, written once by `spec-writer` and
unticked: one row per stage, then three rows the `closer` owns. **The roster
itself lives in [`spec-writer.md`](spec-writer.md)**, which is the agent that
writes it into `tasks.md`; copying it here as well would mean a roster change
made in one file shipping the stale list from the other.

**One row per agent instance, not per role** — `code-reviewer` runs once per
lane, so it gets one row per lane, each ticked by the instance that did it. Do
not collapse them onto one line to save space: a shared checkbox is one nobody
can tick truthfully, and every instance would then edit the same line, which is
the conflict one-row-per-agent exists to prevent.

Each agent flips its own row and adds none, so concurrent cherry-picks never
touch the same line. **An unticked row with no agent running is a stage nobody is
doing** — that is the whole point — **unless it is struck through**, which is how
a stage says it does not apply and why the row is struck rather than deleted: a
deleted row and a skipped stage look identical, and a struck one says which. **A
struck row keeps its empty box, so read the strike, not the box.** One
consequence to expect rather than debug: `openspec archive` counts a struck row
as incomplete and warns before continuing, because the box really is empty. That
is the price of keeping `[x]` single-valued, and it is the right way round — a
tool that counts a skipped stage beats one that cannot tell it from a finished
one. Without this whole mechanism, a piece here once reached the edge of merge
with zero reviewers and another missing four, neither visible until someone
asked.

**A piece with no behaviour change still gets a change folder and a stage block.**
A test-only piece — an integration target, a regression suite — adds no
requirement, so it has no spec delta and its spec row is struck through with that
reason — declared as `skip_specs: true` **alongside a `schema:` key** in the
change's `.openspec.yaml`, because the marker on its own is reported as "not
valid change metadata, so the marker is not honored". It still needs reviewing,
and without the block there is no unticked row to say so: the signal that
catches a missing reviewer is absent exactly where it is easiest to skip one.
The first such piece here reached review with no `openspec/changes/<name>/` at
all, so a reviewer had no row to tick and said so.

**`findings/<lane>.md`**, one file per reviewer — `correctness-readability`
(hyphen in the filename, `+` in its stage-block row), `security`,
`architecture`, `spec-test`, `design-review`, less whichever lanes the tier
drops. **Every finding is a checkbox**, written unticked by the reviewer:

```markdown
- [ ] **`dev-writer`** — `wire.rs:96` — what is wrong
      **Scenario:** inputs → wrong output. **Measured:** the number, if there is one.
```

Whoever acts on it flips the box and appends the outcome — **fixed** (with the test
that fails without it), **rejected** (with the argument), or **deferred** (and where
to) — without editing the reviewer's text. The `closer` deletes the directory
before merge, once no box is empty.

So "blocks the merge" is literal and checkable: `grep -rn "^- \[ \]"` over the
directory either returns lines or it does not.

Three consequences worth knowing whatever your role:

- **An unticked entry blocks the merge.** A file, not a convention, so a forgotten
  finding stops a PR instead of evaporating.
- **The gate only sees checkboxes.** `grep -rn "^- \[ \]"` reports a file of
  headings as clean, so an entry written any other way is invisible to it — this
  has already happened, with forty findings including four high-severity defects
  reading as done. Before trusting an empty result, check the files have boxes at
  all: `grep -rc "^- \[" findings/` should be non-zero for every one.
- **Findings stay attributable**, which is what a rejection needs: a fixer that
  disagrees knows which reviewer to argue with.
- **Never relay a finding through a brief.** Name the file. A paraphrase arrives
  without the evidence that backed it, and a report copied into the runner's
  context and then rewritten into the next brief occupies it twice — which is what
  crowds out the state the runner needs to keep.

Durable reasoning moves into `design.md` before the tracker goes. The tracker is
scaffolding; the reasoning is not.

### Handing over between agents

**Read the files another agent wrote** — findings, `design.md`, `tasks.md`. What
does not reach you is its *report*, which returns to the runner; so anything an
agent needs passed on must be in a file, not in a report.

**A brief points at the work; it does not contain it.** A dispatch is which piece
and which file. **It carries no worktree instructions at all**, because the agent
is dispatched with `isolation: "worktree"` and arrives in a correct tree already:

> Act on the findings for `dev-writer` in
> `openspec/changes/wire-request-envelope/findings/`. Piece branch
> `piece/wire-request`. Commit to your own branch and say what it is called, so
> the work can be cherry-picked onto the piece.

**Keep `git -C <worktree>` and `EnterWorktree` out of the briefs** — an agent
already in the right place needs neither, and a brief carrying them sends it
looking for a problem it does not have. What the brief must say is where the
commits end up.

### How an agent gets the right tree: `isolation: "worktree"`

**Dispatch with `isolation: "worktree"` and no `EnterWorktree` call.** The agent
arrives in its own worktree with a working directory that needs no correcting:
relative paths resolve and every Bash command runs.

**A dispatched agent must not call `EnterWorktree`** — not "try it and fall
back". Who calls it decides the outcome:

| | Works? |
|---|---|
| a dispatched agent entering a **pre-existing** worktree (`EnterWorktree`) | **no** — refused outright at the repository root, and from an isolated tree it appears to succeed while every Bash call is then refused |
| a dispatched agent placed in **its own** fresh worktree (`isolation`) | **yes** |
| a **session moving itself** into a worktree (`EnterWorktree`) | **yes** — the case the tool is built for |

The second row is the dangerous one: it *looks* like it worked, and nothing goes
wrong until the first Bash call. So do not reach for `EnterWorktree` on top of
`isolation: "worktree"` — isolation alone is the whole mechanism, and an agent
that improvises around a refusal reaches for `env -C` or `cd &&`, an approval
click each and neither needed.

**Every agent verifies it arrived, because the isolation does not always take.**
An agent dispatched this way has landed in the main checkout on the piece branch
instead. So each agent file has it run `pwd` and `git rev-parse --abbrev-ref
HEAD` first, and **stop and report** rather than mutate or commit when the branch
is `piece/<name>` or the path is the repository root. A mutating reviewer without
that check breaks the tree the runner's HEAD points at.

The check is in the agent files rather than in the brief because it is a
precondition on the agent's own tools, not a fact about the piece — and because a
brief the runner forgets to write leaves the guard off exactly when it is needed.

#### `baseRef: "head"` is required

By default the agent's tree is cut from `worktree.baseRef: "fresh"` —
`origin/<default-branch>` — which holds **none** of the piece's commits. An agent
reviewing or extending a piece would be reading the wrong code.
`.claude/settings.json` fixes the fork point to the runner's HEAD:

```json
{ "worktree": { "baseRef": "head" } }
```

**That file is tracked, so it arrives with a clone.** `.gitignore` excludes
`.claude/*` but re-admits it by name, for the reason recorded beside the rule:
ignored, it reached no fresh checkout, and **nothing failed when it was absent** —
agents were silently cut from `origin/main` and no error said so. If an agent
reports a fork point that is not your HEAD, check this file before looking
anywhere else.

Like everything under `.claude/`, it is the owner's — CLAUDE.md's "`.claude/` is
the owner's" carries the rule, and this line points rather than restating it,
because a second copy is what invited reading the rest of the directory as fair
game. What is local to this section: machine-local settings belong in
`settings.local.json`, which stays ignored.

#### What the agent's own branch means for getting work back

The agent lands on a harness-named branch, `worktree-agent-<id>` — **not** the
piece branch. So commits still need a cherry-pick onto `piece/<name>`, exactly
the step reviewers already perform for findings, with one difference worth
noticing: the branch name is assigned by the harness rather than being the
`review/<name>/<dimension>` the runner chose, so **read it rather than assuming
it** (`git rev-parse --abbrev-ref HEAD`).

This applies to writers as much as reviewers: no agent stands in the piece's
tree, so nobody commits straight to the piece branch.

#### Tools that resolve their root from the cwd

Two tools here cannot be pointed at another tree: **`openspec`** resolves its
root from the cwd and has no `-C`, `--directory` or `--root`; **`lgs basecamp
build`** resolves `scaffold.toml`'s relative module refs against the root it was
invoked from.

**Placing the agent correctly is what makes both work**, and that is the
strongest practical argument for this dispatch shape: a tool that takes its root
from the cwd is right whenever the cwd is right. An agent in its own tree runs
`openspec` and `lgs` plainly, with no workaround and no compound command.

The hazard they share is worth keeping in view, because it is what makes a wrong
cwd expensive rather than merely inconvenient: **a wrong-tree success is
indistinguishable from a right-tree one in the output.** `lgs basecamp build`
from the wrong root does not fail — it reports a green build of code you did not
write. A tool that refused would be harmless. So before trusting either against a
tree you have not verified, `pwd` and `git rev-parse --abbrev-ref HEAD` cost
nothing.

And the rule that outlives any dispatch shape: **never report a result you did
not obtain against the tree in question.** An unrun gate reported as run is worse
than a red one, because the row gets ticked either way.

#### Who removes the agent's tree

**An agent cannot remove its own worktree, because it is standing in it.** `git
worktree remove` refuses the directory you are in, so **tree removal belongs to
the runner.** An agent's last act is to report its branch name and that its tree
is ready to prune; the runner removes it after cherry-picking the work off.

**The reason the runner keeps a tree is that it may still need reading**: to
re-check a finding against the exact tree that produced it, to compare two
reviewers' citations, or to recover a mutation the reviewer left behind. Once
`--force` has run, the evidence behind the finding is gone — and the runner is
the only party that knows whether any of that is still wanted.

That the agent *cannot* delete it is the better shape: the invariant holds by
construction rather than by every agent remembering an instruction. See CLAUDE.md
on putting complexity in the data rather than the logic.

An agent that hands back does not step out of its tree first — there is no
`ExitWorktree` step in this flow.

**If you are writing out what a finding says, you have the wrong shape.** The
reviewer already wrote it with the measurement behind it; a restatement puts a
paraphrase in front of the evidence, which is how a wrong claim reached two agents
here in one day. Reports work the same way in reverse: path, count, who each entry
is for.

Two corollaries. **Continue an agent rather than starting one** — it still holds
its worktree and its measurements, where a new one gets your summary of them. And
**write the dead end down**: a reviewer that spends an afternoon proving an
approach impossible has produced a result worth as much as the review, and
unwritten the next agent spends the same afternoon. It goes in `design.md`, beside
the decision it rules out.

**The runner owns dispatching; the `dev-writer` opens the PR at the end of its
first pass (see [`dev-writer.md`](dev-writer.md) for the sequence); the `closer`
owns the last three stage rows.** `tasks.md`'s stage block is the list — read it to
see what is left, because an unticked row with no agent running is a stage nobody
is doing.

**How the runner does that is [`RUNNER.md`](RUNNER.md), not this section** — how
to tell whether an agent is still running, how many to launch at once, and why
one piece is one PR. It is kept there rather than restated here because two
copies of a rule drift and the wrong one gets read.

The reviewers run in parallel and ask different questions:

- `code-reviewer` — **is this code correct, safe and well-shaped?**
- `spec-test-reviewer` — **do the tests pin what the spec requires, and can they
  fail?**
- `design-reviewer` — **did the code take the decisions that were recorded, and
  were the decisions worth recording recorded?**

**Launch `code-reviewer` once per lane** — correctness+readability, security,
architecture — naming which in the prompt. One agent holding two becomes
whichever it started with. [`RUNNER.md`](RUNNER.md)'s tier table says how many
lanes a given change gets.

**`spec-test-reviewer` is blind to the implementation.** Someone who has read the
code judges tests by what the code does — exactly the failure a spec exists to
catch: a test that faithfully pins the wrong behaviour.

### Every agent pays CLAUDE.md's Bash costs

This applies to every role, and the reviewers most of all, because they run
suites and mutations in a loop. **Read CLAUDE.md's "How to work in this repo,
and what Bash costs" before the first shell command.**

The rule that catches agents most often is **never chain**: `cd somewhere &&
cargo test` prompts *even though* `cargo test` is allow-listed, because the
permission checker cannot statically analyse a compound command, so no rule
applies to it. Run one plain command per call — `cd` alone in its own call is
free, and the Bash tool's directory persists between calls.

**A long output is not a reason to pipe.** This is the most common way the rule
gets broken by someone who knows it: appending `| tail -30` to keep the output
manageable turns a call the checker would have approved into a prompt, which is
the opposite of what the pipe was for. Run it plain and read the whole thing.
Likewise `gh` is free until you filter it — adding `--jq` costs a click where the
plain call costs nothing.

**Address this repo's agents unqualified** — `code-reviewer`, not
`agent-skills:code-reviewer`. The plugin ships a similarly-described reviewer,
and it carries none of this repo's traps.

**One shell trap worth not re-learning**, because it makes a gate silently
passing: `tar tzf … | grep -q` exits 141, since `grep -q` closes the pipe at the
first match and `tar` dies of SIGPIPE. Under a bare `set -eu` that is invisible,
and it becomes a spurious failure the moment anyone adds `-o pipefail`. Use
`[ "$(… | grep -c …)" -gt 0 ]`, which consumes all the output. `ci.yml`'s release
job carries this one, verbatim, because a port of it once reverted the fix.

**A change with no source diff still gets reviewed.** That is not an exemption,
and treating it as one is how this flow's own adopting change nearly shipped with
the `code-reviewer` step skipped entirely. Agent instruction files, config and the
prose in `CLAUDE.md` and `docs/` are all reviewable material.

## What experience has taught this flow

Each of these is in the agent files because it cost something here.

**A number in a comment is a claim, and this repo fabricates them.** One sweep
found six comments in a single file arguing from premises the code disproves, and
the worst were quantities, because a quantity reads as though someone measured it:
*"wrong for two years"* in a repo five days old — written, no less, in the commit
titled "stop three comments from saying the wrong thing". A plausible number
nobody checks is a fabricated citation that looks like evidence.

**So this is a step, not a caution: run the command before you write the
number.** `grep -c "function test_" <file>` for QML test functions, `grep -c
"#\[test\]"` for Rust ones, `git log -S` for a duration — whichever answers the
claim you are about to make. It is one call, and it is the difference between a
measurement and a guess that reads like one. This is CLAUDE.md's "do not write
down anything a command can answer" applied to the thing an agent writes most
often: a count in a report, a task list or a doc comment.

**When you correct a stale number, measure it fresh — do not apply the delta a
reviewer quoted.** The reviewer's figure was measured at some earlier moment and
a branch moves, so a quoted delta can introduce a second wrong claim while fixing
the first. Re-run the command against the tree in front of you.

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

**Silent failure is this codebase's house style, and it must be designed
against.** Basecamp swallows QML errors, so a view that fails to compile, a
plugin skipped for a missing manifest field, and a binding evaluating to
`undefined` all present identically as "clicking the app does nothing" — the
`Theme`/`DTheme` name collision took the entire visual system out this way, with
every gate green. `qmllint --missing-property error` printed the same class of
defect as a warning into a green log. CLAUDE.md's "Module contract traps" has
the full account and the gates it produced
(`check_qml_names.py`, `check_qml_members.sh`); do not re-derive it from a
second copy here.

**Specs get reorganised as concepts generalise**, and two capabilities asserting
one rule is the failure that prevents — both already live here. See
[`docs/OPENSPEC-ARCHIVE.md`](../../docs/OPENSPEC-ARCHIVE.md).
