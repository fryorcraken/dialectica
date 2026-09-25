## Context

See `proposal.md` for why. Everything this change edits is prose that agents
read: no CI job, script or test reads `.claude/agents/`, so the only check on
it is a reviewer reading it.

Three constraints shape the approach:

- **Each agent reads its own file, and the runner reads `RUNNER.md`.** A rule
  is only followed if it is in the file of the agent that must act on it. #169
  happened because the `NO SPEC:` routing lived in `spec-writer.md` and
  `tester.md`, which the runner does not open.
- **Each rule is stated once.** Where a second file needs a rule, it points to
  the first one. Two copies drift, and the reader who finds the stale one cannot
  tell.
- **The stage block's mechanics.** Each agent ticks its own row and nobody adds
  one, so concurrent cherry-picks never touch the same line. The `closer`'s
  Step 1 refuses to merge while any row other than its own is unticked. That
  refusal is the only gate in the flow that can be checked, rather than
  remembered.

## Goals / Non-Goals

**Goals:**

- The runner's steps from the `dev-writer`'s hand-back to the merge read as one
  numbered sequence in `RUNNER.md`, with #169 at the start and #171 at the end.
- Whether post-review work was reviewed shows up in the stage block, where the
  `closer`'s existing gate can see it.
- `closer.md` names `--admin` and branch-protection changes as forbidden.
- `.claude/agents/README.md`'s `settings.json` paragraph is back to its pre-#119
  wording, and no other line of that file changes.

**Non-Goals:**

- Diagnosing why #165 was `BLOCKED`. See "Out of scope" in `proposal.md`.
- Changing any reviewer's role file. A re-review uses the reviewer files as they
  are, and the runner's brief carries what differs for a re-review.
- The size of the first review round. It stays at six.

## Decisions

### One section of `RUNNER.md` carries the whole sequence, with the `closer` as its last step

"The `closer`, and what comes back" becomes a subsection of a new section,
"From the `dev-writer`'s hand-back to the merge". That section has four
numbered steps:

1. Route `NO SPEC:`.
2. The tester and the review round.
3. Re-review.
4. The `closer`.

The red-CI path in step 4 sends the fix back to step 3 rather than stating its
own re-review rule.

**Alternatives considered:**

- **A #169 section and a #171 section.** Each would describe part of the same
  stretch between hand-back and merge. The red-CI path would then need its own
  copy of the re-review rule, because it sits under the `closer` heading.
- **Leaving "The `closer`, and what comes back" as its own section and adding
  the rest above it.** This is the same split, one heading further down. The
  sentence in it that caused #171 ("Then re-dispatch the `closer`") only reads
  correctly if a reader can see the step it now returns to.

The subsection keeps its heading, so the proposal's references to it still
resolve.

### The re-review is one runner-owned row, and the runner unticks it when a later commit lands

The template in `spec-writer.md` gains one row, between the review rows and the
`closer`'s three:

```markdown
- [ ] re-review: every commit after the review round — runner
```

Under that row the runner writes one indented record line per round. The line
gives the commit range, what landed, the lanes and models, and why. A skipped
round gets a line with its reason. The runner ticks the row when no commit that
merges is left unreviewed. If a later commit lands, such as a red-CI fix, the
runner unticks the row and adds the next round's line.

The runner owns this row because the decision it records is the runner's: how
big the re-review is. Recording it is tracking state, and tracking state is
already the runner's job ("Dispatch, track state, report"). It is not the work
itself. `RUNNER.md` lists it with the runner's other non-dispatch tasks, so it
does not read as an exception to "You do not write the work".

**What the row buys.** The `closer`'s Step 1 already refuses to run while any
row other than its own is unticked. With this row, a missing re-review blocks
the merge through the gate that exists, and `closer.md` needs no new check. On
#165 the runner offered a re-pass, got no answer, and sent the `closer` anyway.
With this row unticked, the `closer` would have stopped at Step 1.

**Alternatives considered:**

- **Rows added per re-review round, one per lane.** Ruled out by the
  no-added-rows rule. The number of rounds is not known when the block is
  written, so rows for them would have to be added later. Added rows are what
  make concurrent cherry-picks collide.
- **Pre-written rows for a fixed number of rounds, say two, each with six
  lanes.** Most would be struck on most pieces, and a third round would still
  need an added row.
- **Re-reviewers ticking their own existing rows again.** Those rows are ticked
  after the first round. A second tick has nothing to flip, so the row cannot
  show that a second round is outstanding.
- **Recording the round only in the runner's report.** #171 asks for the report
  *and* the stage block. A report goes back to the owner and is not kept in the
  repo, so the next session's runner, rebuilding state from `tasks.md`, would
  not see that a round was still owed.
- **Striking the row when nothing was reviewed after the round.** A strike
  means "this stage does not apply", and this row always applies: it asks
  whether the branch holds unreviewed code that will merge. "Nothing landed" is
  a round whose record says so, followed by a tick.

**Unticking** is new in this flow. Until now, no row went back from `[x]` to
`[ ]`. It is limited to the runner's own row, and it is the only way the
`closer`'s gate can see a red-CI fix that is still unreviewed. That fix lands
after the row was ticked, and the re-dispatched `closer` checks the same block.

**What breaks without the row.** The only mechanical signal would be gone: the
re-review would be back to a rule the runner has to remember, and #171 is two
cases of that failing on one day.

### "Every commit that merges" defines what needs re-review, and tick-only commits do not count

A findings pass always adds at least one commit, because the fixer flips the box
and appends the outcome. If every commit counted, every piece would owe a
re-review of box ticks. Counting only commits that change content keeps the rule
meaningful:

- `findings/` is deleted before merge.
- A stage-row tick is tracking.

A commit that moves durable reasoning into `design.md` does count. It merges,
and it is the design-reviewer's material.

`RUNNER.md` states this definition because a runner needs it to decide whether
a round is owed at all.

### The re-review brief carries what differs from a first-round review

The reviewer role files describe a first-round review: tick your row, write your
findings file. In a re-review the row is already ticked and the findings file
exists. `RUNNER.md` tells the runner to put three things in the brief:

- the commit range to read;
- that the reviewer's row stays as it is;
- that new findings are appended as boxes to the existing file.

Appending still works with the findings gate, which only counts boxes.

The alternative was editing four reviewer files. It is ruled out by scope: the
owner authorised edits for what these four issues ask for, and none asks for a
reviewer-file change.

### Sizing guidance comes from #171, and names no model

#171 argued against a fixed rule, and its reasoning is kept here. Post-review
changes range from a one-line fix to a rewrite, so any fixed count is wrong for
most of them. The issue gives these guides, which `RUNNER.md` carries:

- re-dispatch the lanes whose ground the new commits touch;
- a finding answered exactly as the reviewer asked may need only that reviewer;
- a rewrite needs the full set;
- a narrow confirmation can run on a smaller model;
- a rewrite or anything security-relevant gets the strongest model.

`RUNNER.md` names the Agent tool's `model` override but no model by name.
Available models change, and CLAUDE.md's "do not write down anything a command
can answer" applies. The tool's own parameter lists them.

The issue also says why a red CI run is not an exception. **A green CI run and
the author's own mutation runs are not review, and a warning the owner did not
answer is not consent.** #164's `yq` rewrite and #165's post-review spec
callback both merged with every gate green. `RUNNER.md` keeps that sentence,
because it is what a runner needs when a green run tempts it to skip the round.

### `NO SPEC:` routing: a fresh `spec-writer`, not the one that wrote the spec

#169 left one question optional: keep the `spec-writer`'s tree until the
`dev-writer`'s first pass, so a callback could continue that agent with
`SendMessage` rather than start a new one. **Decided: no. The callback is a
fresh dispatch, and `RUNNER.md` says so.**

The reason is where each tree was forked. The first `spec-writer`'s tree was
cut from the runner's HEAD before the `dev-writer` ran. The `dev-writer`'s
commits reached the piece by cherry-pick afterwards, onto the runner's branch,
and never onto that agent's `worktree-agent-<id>` branch. A continued
`spec-writer` would therefore stand in a tree without the code or the
`NO SPEC:` markers it was called back to judge. That is the failure `RUNNER.md`
already rules out for every writer: an agent forked from a HEAD missing the
previous agent's work "will then rewrite, duplicate or contradict work it
cannot see". A fresh dispatch forks from the runner's current HEAD, which has
the markers.

What continuing would have kept is the first agent's reading of the issue. That
reading is already in `proposal.md` and the spec, which a fresh agent reads. So
the only cost is re-reading two files.

Keeping the tree would also leave a full copy of the repo standing for the whole
of the `dev-writer`'s pass. CLAUDE.md's "Worktrees are not scratch" names that
as what makes a recursive grep cite a stale tree.

The existing sequence (hand-back → cherry-pick → remove the tree → dispatch the
next) already removes the tree, so the decision needs no change there. It needs
only the half-sentence in step 1 saying the callback is fresh. Without it, a
runner reading "continue an agent rather than starting one" in `README.md`
might try to revive the old one.

### Markers are shown to the `spec-writer` by command, and unmarked decisions by quotation

`RUNNER.md` already forbids paraphrasing a finding into a brief. `NO SPEC:`
markers live in the code, so the brief names `git grep -n "NO SPEC:"` rather
than listing them. A behaviour decision the `dev-writer` reported without a
marker exists only in its hand-back, so there is no file to point to. The brief
quotes that sentence word for word. A quotation is not a paraphrase, and this
is the one place the runner holds the only copy.

### After the `spec-writer`, the brief chooses who brings markers into line

If the `spec-writer` changed the behaviour, the code is now wrong, so the
`dev-writer` goes next and the `tester` after it. Otherwise the `tester` is next
anyway, and its brief names the `spec-writer`'s commit so it writes against the
new text.

### `--admin` is forbidden in Step 6, with a pointer from "What you never do"

The rule sits in Step 6, where a `closer` is standing when `gh pr merge` refuses.
It says:

- the merge-on-green authority covers `gh pr merge <n> --squash` and stops at
  branch protection;
- never `--admin`, never a write to branch protection or a ruleset;
- on `BLOCKED` with every required check green, stop and report the
  `gh pr view` output #170 names.

"What you never do" gets a one-line bullet that points to Step 6 rather than
restating it. The list is the file's summary of tempting shortcuts, and leaving
this one out of it would make the list read as complete without it.

#170's reasoning is kept in the Step 6 text because it is what a `closer` needs
at that moment. The route past a `BLOCKED` state does not depend on why the PR
was blocked. So a closer that treats `--admin` as the way past `BLOCKED` would
take the same route past a requirement that is actually failing. On #165
nothing substantive was skipped, but that was luck.

### #133: two lines restored by hand, not by `git checkout`

The issue's own fix, `git checkout 51ab7f8^ -- .claude/agents/README.md`, would
revert every later change to that file, including #106's move from `PLAN.md`
to GitHub Issues. So only the paragraph is replaced. Its original text is
taken from `git grep -n -F "user's** file" 51ab7f8^ -- .claude/agents/README.md`
rather than retyped from the issue, and restored with the original's line break.

**Two things in the replaced paragraph were not written by the owner.** One is
its pointer to CLAUDE.md's "`.claude/` is the owner's". The other is its claim
that a second copy "invited reading the rest of the directory as fair game".
Both came from a fixer answering a #131 review finding, which #133 names as the
unauthorised part. The restored text states the narrow rule again, and that is
the owner's wording. `RUNNER.md`'s "It is the user's file. **Do not edit it**"
already says the same thing, so the two files agree once more.

## Risks / Trade-offs

- **[A re-review round can itself produce findings, so rounds can repeat.]** →
  Each round is sized to the commits since the last one, so rounds get smaller.
  The row is ticked when a round adds no merging commit. `RUNNER.md` does not
  cap the number of rounds: a cap would be a fixed rule, and #171 argued
  against fixed rules.
- **[The `closer`'s rebase conflict resolution is a post-review content change
  that this change does not cover.]** Step 2 of `closer.md` has the `closer`
  resolve conflicts itself, and that code is not reviewed before the merge.
  #171 names three cases (a findings pass, an owner instruction, a red-CI fix)
  and does not name this one. → Reported in the hand-back as a candidate for a
  follow-up issue. It is not in scope here.
- **[`tester.md` says "Keep the markers and report each one — the spec-writer
  decides".]** Once the `spec-writer` has decided before the tester runs, that
  sentence is out of date. A tester following it would keep a marker for
  behaviour that is now specified. → `tester.md` is outside this change's
  authorisation, so this goes in the hand-back as a proposed owner edit. Until
  then, `RUNNER.md` has the tester's brief name the `spec-writer`'s commit.
- **[PR #132 edits the same template and the same README paragraph.]** → The
  second to merge reconciles them. `proposal.md` lists the overlapping hunks.
  The re-review row's wording does not depend on lane names, so #132's merge of
  readability into correctness does not break it.
- **[Nothing mechanical checks any of this prose.]** → The six reviewers are the
  check. `tasks.md` strikes the tester row with the reason.
