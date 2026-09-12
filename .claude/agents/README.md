# The spec-driven flow

Role agents around [OpenSpec](https://openspec.dev)'s built-in `spec-driven`
schema: OpenSpec supplies the artifacts and their ordering, Claude Code the
agents. No custom tooling — subagents already give isolated context windows,
per-role models and tool limits.

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
archive, not before you start.** `openspec` is installed; run
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
| `dev-writer` | spec, PLAN.md | `design.md`, `tasks.md`, code, tests-as-it-goes |
| `tester` | spec, inherited tests | the test suite |
| `spec-test-reviewer` | **spec + tests only** | findings |
| `design-reviewer` | code, `design.md`, PLAN.md | findings |
| `code-reviewer` | code | findings |

## One piece of work is one branch and one PR

Every stage — spec, design, code, tests, review fixes — lands as **commits on one
branch, under one PR**. Agents keep their own worktrees and local branches; a
*stage* never gets its own branch and PR.

**The unit of review is a behaviour change with its contract and its tests
attached.** A reviewer must be able to see they belong together, not be told so
by whoever is orchestrating. And `openspec archive` runs once, on merge — split
across several merges, the contract lands at a different time from the code that
honours it.

### One writer at a time; reviewers in parallel

**A piece has at most one `spec-writer`, `dev-writer` or `tester` running at any
moment** — not one of each, **one in total**. All three commit to `piece/<name>`
directly, and each has its own reason why a second agent alongside it is unsafe:

**`tester` mutates implementation code it does not own.** It breaks a line, checks
a test notices, restores it. A `dev-writer` editing that file concurrently either
inherits the mutation as its own broken state or overwrites the restore — and
neither shows up as a conflict, because the two never touch git at the same moment.
A silently reverted fix is worse than a merge conflict.

**A spec must not move while code is being written against it.** That pair is about
the contract rather than the files: run them together and the implementation is
built against a contract that changed underneath it, with neither agent knowing.
This session did exactly that — a `spec-writer` reworded the envelope rule's scope
while four reviewers read the code implementing it, so their findings were against a
contract that had already moved.

That holds on the way back too. When review routes a spec gap to `spec-writer`,
**stop the fixer, let the spec land, then restart it against the new text** — a
`dev-writer` fixing code while the requirement it answers is being rewritten is the
same failure, just later in the flow.

**Reviewers run in parallel, up to six**, because each writes only its own findings
file and no two write the same path.

The exception that makes worktrees non-negotiable: a reviewer that runs `cargo
test` or `cargo mutants` **does** mutate the tree, breaking the code deliberately
to see whether a test notices. Two sharing a tree see each other's broken code and
report it as the author's; this has happened here.

**A reviewer deletes its worktree when done** — `git worktree remove <abs-path>
--force` — rather than restoring what it mutated. Restoring depends on having
tracked every edit, and `cargo mutants` breaks dozens of lines; one missed restore
ships a deliberately broken line into the piece. Deleting is unconditional and
cannot half-succeed, and the findings file is already committed elsewhere.

A `tester` has nothing to delete — it works in the piece's shared tree and its
tests are the deliverable. It proves the implementation is untouched with
`git diff --stat` instead, which should show test files only. **A missed restore
will not fail its own suite**, because the code was mutated precisely so a test
would catch it and the test's expectation was then restored to match.

Every branch rule below follows from that asymmetry, so read it that way rather
than as bookkeeping.

### Branch names say which kind of branch it is

| Name | Whose | Holds |
|---|---|---|
| Branch | Worktree | Whose | Holds |
|---|---|---|---|
| `piece/<name>` | one, shared | the three writers, in turn | **the** task branch — the one the PR is open on. Spec, code, tests and findings-fixes all commit here directly |
| `review/<name>/<dimension>` | one each | one reviewer | its findings file, nothing else — cherry-picked onto the piece, never pushed |

**`spec-writer`, `dev-writer` and `tester` share one worktree, checked out on
`piece/<name>`.** They can share it precisely because they never run at the same
time; handing each its own tree would buy nothing and add a cherry-pick to get
wrong. Reviewers get a tree each because they are the only agents that overlap.

Named for the role and not the stage, because `dev/x` invites a `test/x` beside
it — which is the shape this section exists to stop.

**Open the PR on `piece/<name>` from the first commit. It cannot be corrected
later.** A PR's head ref is immutable: `PATCH /pulls/<n> -f head=…` returns **200
and silently ignores the field**, GraphQL has no `headRefName`, and `--base`
changes the target rather than the source. GitHub's branch-rename endpoint
retargets PRs whose *base* was renamed and **auto-closes** one whose *head*
vanished. Recovering means recreating the old branch at the same SHA and
reopening — which works, but only if you notice.

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

**Only the runner pushes.** With one pusher there is no race to lose, no rebase to
retry, and no force-push to be tempted by.

**Only reviewers get a side branch**, because only reviewers run genuinely in
parallel — six at once, while a fixer may still be changing the code they are
reading. A reviewer's own branch is what stops its commit racing that. Everyone
else writes the piece one at a time and commits to it directly; a side branch there
would add a step to get wrong and misname the commits besides.

Cherry-pick rather than merge, so the task branch reads as a flat sequence rather
than six merge commits carrying six branches.

**Never `git add -A`** — commit named paths. Worktrees collect build output and a
gitignored SDK symlink, and sweeping up another agent's half-finished edit
corrupts the branch you were working on.

**Check `git branch -vv` before any git write** — a worktree created from a branch
inherits that branch's upstream, and a bare `git push` has landed commits directly
on `main` here more than once.

## Two files carry the state of a change

Each agent's own file says what it writes. These are the shapes everyone needs to
recognise, because everyone reads both.

**`tasks.md` opens with a stage block**, written once by `spec-writer` and unticked:

```markdown
## Stages
- [ ] spec — `spec-writer`
- [ ] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner
```

**One row per agent instance, not per role** — `code-reviewer` runs four times, so
it gets four rows, each ticked by the instance that did it. Do not collapse them
onto one line to save space: a shared checkbox is one nobody can tick truthfully,
and all four instances would then edit the same line, which is the conflict
one-row-per-agent exists to prevent.

Each agent flips its own row and adds none, so concurrent cherry-picks never
touch the same line. **An unticked row with no agent running is a stage nobody is
doing** — that is the whole point, and without it this session took a piece to the
edge of merge with zero reviewers and another missing four, neither visible until
someone asked.

**`findings/<dimension>.md`**, one file per reviewer — `correctness`, `security`,
`readability`, `architecture`, `spec-test`, `design-review`. **Every finding is a
checkbox**, written unticked by the reviewer:

```markdown
- [ ] **`dev-writer`** — `wire.rs:96` — what is wrong
      **Scenario:** inputs → wrong output. **Measured:** the number, if there is one.
```

Whoever acts on it flips the box and appends the outcome — **fixed** (with the test
that fails without it), **rejected** (with the argument), or **deferred** (and where
to) — without editing the reviewer's text. The runner deletes the directory before
merge, once no box is empty.

So "blocks the merge" is literal and checkable: `grep -rn "^- \[ \]"` over the
directory either returns lines or it does not.

Three consequences worth knowing whatever your role:

- **An unticked entry blocks the merge.** A file, not a convention, so a forgotten
  finding stops a PR instead of evaporating.
- **Findings stay attributable**, which is what a rejection needs: a fixer that
  disagrees knows which reviewer to argue with.
- **Never relay a finding through a brief.** Name the file. A paraphrase arrives
  without the evidence that backed it, and a report copied into the runner's
  context and then rewritten into the next brief occupies it twice — which is what
  crowds out the state the runner needs to keep.

Durable reasoning moves into `design.md` before the tracker goes. The tracker is
scaffolding; the reasoning is not.

### Handing over between agents

No agent can read another's report — everything passes through the runner, so
every hop is a chance to lose the evidence. Three rules:

**Continue an agent rather than starting one.** A message to the agent that did
the work keeps its worktree, its measurements and its reasoning. A fresh agent
gets a brief, which is a summary of those.

**Write the dead end down, not just the conclusion.** A reviewer that spends an
afternoon establishing why a trait-driven sweep is impossible here — the trait
lives in the crate that depends on core, not the reverse, and is behind a `cfg`
`cargo test` never sets — has produced a result worth as much as the review.
Unwritten, the next agent spends the same afternoon. It goes in `design.md`
beside the decision it rules out.

**State a claim's provenance when relaying one.** "A reviewer measured X" and "I
believe X" license different actions, and an agent given the second as the first
will not re-check it.

**The runner owns the last two stage rows, plus dispatching and pushing.**
`tasks.md`'s stage block is the list — read it to see what is left, because an
unticked row with no agent running is a stage nobody is doing. Dispatch by naming
the findings files rather than carrying their content, and re-run only the
reviewers whose findings led to changes.

The reviewers run in parallel and ask different questions:

- `code-reviewer` — **is this code correct, safe and well-shaped?**
- `spec-test-reviewer` — **do the tests pin what the spec requires, and can they
  fail?**
- `design-reviewer` — **did the code take the decisions that were recorded, and
  were the decisions worth recording recorded?**

**Launch `code-reviewer` once per dimension** — correctness, security,
readability, architecture — naming which in the prompt. Scanning for a reachable
panic is a different reading of a file from scanning for a function doing two
jobs, and one agent holding both becomes whichever it started with. A small
change can take one instance covering all four; a full review is typically six
agents.

**`spec-test-reviewer` is blind to the implementation.** Someone who has read the
code judges tests by what the code does — exactly the failure a spec exists to
catch: a test that faithfully pins the wrong behaviour.

**Give each reviewer that mutates code its own worktree**, in
`.claude/worktrees/`. Two sharing a tree see each other's broken code and cannot
tell it from the author's; this has happened.

## What experience has taught this flow

**A quantity in a comment is a claim, and this repo fabricates them.** One sweep
found six comments in one file arguing from premises the code disproves. The worst
were numbers, because a number reads as though someone measured it:

- *"the reason this was wrong for two years"* — the repo is **five days old**, and
  `git log -S` dates both commits to the same day. Worse, it was written **in the
  commit titled "stop three comments from saying the wrong thing"**.
- *"three orders of magnitude below 64 MiB"* — a factor of **16**, and
  self-contradicting: three orders below 64 MiB is ~67 KiB, which would refuse the
  legitimate 1.6 MB request the same comment defends.
- *"five call sites"* for a constant with **one** production use — the argument for
  the design this change had rejected.

Before writing a duration, a magnitude or a count, **get it from a command**:
`git log -S <string>` for when something appeared, `grep -c` for how many. A
plausible number nobody checks is the same failure as a fabricated citation, and it
is harder to spot because it looks like evidence.

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

**A green gate can be structurally blind.** `cargo fmt --check` does not follow
path dependencies, so it never reaches `dialectica-core` — where nearly all the
logic lives. Anything behind `cfg(logos_scaffold)` is not compiled by
`cargo test` at all. Say what a gate cannot see rather than reporting it as
passed; "exit 0" on a gate that measured nothing is worse than no gate.

**Specs get reorganised as concepts generalise**, and two capabilities asserting
one rule is the failure that prevents — both already live here. See
[`docs/OPENSPEC-ARCHIVE.md`](../../docs/OPENSPEC-ARCHIVE.md).
