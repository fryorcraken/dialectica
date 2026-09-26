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
  one. An agent forked after the previous tick reached the runner's HEAD
  cherry-picks cleanly; the review round's ticks, made from one HEAD on
  adjacent rows, do not (see "A cherry-pick that conflicts goes back to its
  agent"). The `closer`'s Step 1 checks that every row but its own is ticked
  or struck. That check is the only gate in the flow that can be run, rather
  than remembered.

## Goals / Non-Goals

**Goals:**

- The runner's steps from the `dev-writer`'s hand-back to the merge read as one
  numbered sequence in `RUNNER.md`, with #169 at the start and #171 at the end.
- Whether post-review work was reviewed shows up in the stage block, where the
  `closer`'s existing gate can see it. That includes the `closer`'s own
  commits, which the owner took into this piece.
- What the runner commits is stated once, as the owner ruled it.
- `closer.md` names `--admin` and branch-protection changes as forbidden.
- `.claude/agents/README.md`'s `settings.json` paragraph is back to its pre-#119
  wording. Its other changes are the branch section's account of how work
  reaches `piece/<name>`, which points to `RUNNER.md`, and the four passages
  outside that section that named the cherry-pick as every agent's route,
  which say "brought onto" or "bringing onto" the piece instead. Its
  stage-block section does not change; `proposal.md`'s Impact entry for
  `README.md` says why "one row per stage, then three rows the `closer` owns"
  still holds.

**Non-Goals:**

- Diagnosing why #165 was `BLOCKED`. See "Out of scope" in `proposal.md`.
- Changing any reviewer's or writer's role file, beyond correcting one clause
  of `dev-writer.md` that states a false premise. A re-review, a writer
  resolving a conflict and a mutating reviewer continued to rebase use the
  files as they are; the runner's brief or continuation message carries what
  differs.
- The size of the first review round. It stays at six.
- The stage-block template's shape. The review round's tick conflict is handled
  by the cherry-pick rule, not by reshaping the block.

## Decisions

### One section of `RUNNER.md` carries the whole sequence, with the `closer` as its last step

"The `closer`, and what comes back" becomes a subsection of a new section,
"From the `dev-writer`'s hand-back to the merge". That section has four
numbered steps:

1. Route `NO SPEC:`.
2. The tester and the review round.
3. Re-review.
4. The `closer`.

Every path that adds a commit after the review round — a red-CI fix, a
conflict resolution, a spec-changing archive — goes back to step 3 rather than
stating its own re-review rule.

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

Step 2 points to "How many at once" for the round's size and states no count
of its own. A count there would go stale the moment #132 tiers the round, and
it sits far from every hunk #132 touches, so reconciling the two PRs would not
surface it (`findings/architecture.md`). Step 3's sizing guidance says "the
full set" for the same reason.

### The re-review is one runner-owned row, and the runner unticks it when a later commit lands

The template in `spec-writer.md` gains one row, between the review rows and the
`closer`'s three:

```markdown
- [ ] re-review: every commit after the review round — runner
```

Under that row the runner writes one indented record line per round. The line
starts ``round <n> `<range>` ``, numbered in the order the lines are written,
and gives what landed, the lanes and models, and why; a lane run again over
the same range, fresh or continued, gets a line of its own (see "Findings go under a heading
naming the round"). A skipped round gets a line with its reason. The runner ticks the row when no commit that
merges is left unreviewed. If a later commit that needs review lands — a red-CI
fix, a conflict resolution, a spec-changing archive — the runner unticks the
row and adds the next round's line.

The runner owns this row because the decision it records is the runner's: how
big the re-review is. Recording it is tracking state, and tracking state is
already the runner's job ("Dispatch, track state, report"). It is not the work
itself, and "What a runner commits" names it as the runner's only content.

**What the row buys.** The `closer`'s Step 1 already checks that every row but
its own is ticked or struck. With this row, a missing re-review is something
that check finds, and `closer.md` needs no new check. On #165 the runner
offered a re-pass, got no answer, and sent the `closer` anyway. With this row
unticked, the `closer`'s Step 1 would have found it.

That check is worded as a requirement ("every row ticked or struck through
with a reason, except your own") under "is the piece finished?", not as an
explicit "stop". `RUNNER.md` and this file say only what it literally says
(`findings/readability.md`); strengthening `closer.md` Step 1 to say "stop"
was outside this piece's authorisation.

**Alternatives considered:**

- **Rows added per re-review round, one per lane.** Ruled out by the
  no-added-rows rule. The number of rounds is not known when the block is
  written, so rows for them would have to be added later, by the runner, which
  "What a runner commits" does not allow, and each would be one more line for
  a concurrent pick to meet.
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
`closer`'s gate can see a commit that landed after the row was ticked and is
still unreviewed. The re-dispatched `closer` checks the same block.

**What breaks without the row.** The only mechanical signal would be gone: the
re-review would be back to a rule the runner has to remember, and #171 is two
cases of that failing on one day.

**After the archive, that block is in the archive, and the untick goes there.**
The `closer` deletes `findings/` and archives in its Step 3, before it watches
CI in Step 4. So once an earlier `closer` has archived, whatever it then comes
back with, the change folder has moved to
`openspec/changes/archive/<date>-<name>/`, and the stage block with it.
`RUNNER.md` says the untick and the round's line go in that `tasks.md`.

**Both files name that condition, not a list of returns.** `closer.md` Step 1
and `RUNNER.md`'s "By then the block may have moved" give a red run and a
spec-changing archive as examples only. A list would read as complete, and it
is not: a conflict met merging `main` after the archive (Step 4 finding the
branch `BEHIND`) and a push refused after the archive commit also come back
with the block moved. A runner reading "after a red run, or…" as the whole set
would untick at the old path on either of those, where nothing reads it.

**Both readers of the block find it by exact path.** The command is:

```
git ls-files -- "openspec/changes/<name>/tasks.md" "openspec/changes/archive/????-??-??-<name>/tasks.md"
```

It names the live folder and the archived one, so one command serves a first
dispatch and a re-dispatch without the reader having to know which it is.

Measured on this tree when the change was written:

- `171-workflow-rules` returns only the live folder;
- `op-clock` returns only `archive/2026-09-16-op-clock`;
- `time-pegged-clock` returns only `archive/2026-09-25-time-pegged-clock`;
- `home-screen-key-states` returns only its own folder, not
  `archive/2026-09-24-home-screen-key-states-followup`;
- `clock`, which names no change, returns nothing.

**The suffix glob `openspec/changes/*<name>/tasks.md` was the first version,
and is ruled out.** `*` matches any prefix, so it also matches every change
whose name ends in `<name>`: `*clock` returns both `op-clock` and
`time-pegged-clock`. The date pattern pins the prefix to exactly the
`YYYY-MM-DD-` that every folder under `openspec/changes/archive/` carries
(`git ls-files -- "openspec/changes/archive/*/proposal.md"` lists them).

What breaks without the exact path: a re-dispatched `closer` greps
`openspec/changes/<name>/`, which no longer exists, sees no unticked row, and
passes Step 1. The runner's untick would sit in a file nothing reads. That is
exactly the case the untick exists for, so without the path the row would
guard every post-review commit except the one #171 names by kind. With the
suffix glob instead, a piece named `clock` would find another change's block
as well, and a `closer` taking the first line would gate on the wrong piece.

**The command appears twice, in `closer.md` Step 1 and in `RUNNER.md`'s
"Rebuild the state", and that is deliberate.** Each is the file of an agent
that has to run it, and neither agent reads the other's file as a matter of
course. A pointer from one to the other would leave the reader at the gate
without the command it is about to type. What the two copies share is only
the command. Why the block moves, because the `closer` archives before it
watches CI, is stated once, in `RUNNER.md` step 3; `closer.md` and "Rebuild
the state" say where it is, not why. What each reader does with the answer
differs, so each file states its own.

**The runner's rebuild commands otherwise fail in a way that reads as
state.** After a red run, the grep on `openspec/changes/<name>/tasks.md`
errors, `grep -c "^## Stages"` errors, and `openspec list` no longer shows the
change, next to a sentence saying `0` means untracked. So "What you read" and
"Rebuild the state" both point at the archived folder, and the latter says
how to recognise a piece in that state: an open PR whose change `openspec list`
no longer shows.

`closer.md` points to `RUNNER.md` for why the block is there rather than
restating it. It adds two things only the `closer` needs:

- **An archived folder with no `findings/` passes the findings gate.** An
  earlier `closer` deleted it, and no re-reviewer has run since: after the
  archive every re-reviewer writes its file, clean or not (see "A clean
  re-reviewer appends a ticked verdict box"). So its absence means the runner
  ran no round there, and the runner's line under the re-review row says why.
  Without this sentence, `grep` on a missing directory errors, and a `closer`
  could read the error as a failed gate. The sentence once said the absence
  meant "the re-review raised none"; that held only while a clean re-reviewer
  wrote no file, and it would now let a `closer` read a skipped round as a
  clean one.
- **More than one path back, or none, means stop and report what came back.**
  The `closer` cannot tell which block is the piece's. More than one most
  likely means something recreated the pre-archive folder after the archive,
  such as an untick or a findings file written at the old path. None means the
  name is wrong. Without this, a `closer` taking the archived path would pass
  Step 1 over a block the runner meant to leave unticked elsewhere, and a
  `closer` given a wrong name would find no rows and read that as all ticked.

### "Every commit that merges" defines what needs re-review, and tracking commits do not count

A findings pass always adds at least one commit, because the fixer flips the box
and appends the outcome. If every commit counted, every piece would owe a
re-review of box ticks. Counting only commits that change content keeps the rule
meaningful. `RUNNER.md` lists what is tracking rather than content: a box
flipped, a finding's outcome in `findings/`, a re-reviewer's findings or verdict
box in `findings/`, a stage-row tick, and the runner's own record line under the
re-review row.

- `findings/` is deleted before merge. A re-reviewer's verdict box in particular
  must not count: if it did, recording that a round was clean would itself owe
  a round.
- A stage-row tick is tracking.
- **The runner's record lines merge**, because `tasks.md` is archived with the
  change. They are still tracking: they record what the flow did, not what the
  change does. If they counted, recording round 1 would itself be a commit that
  owes round 2, and no round could ever close the row. That is why the list
  names them explicitly: "commits that change something which merges", read
  literally, covers them.

A commit that moves durable reasoning into `design.md` does count. It merges,
and it is the design-reviewer's material.

**The `closer`'s own commits are sorted the same way.** Two need review and
three do not:

- **A writer's conflict resolution needs review.** It is new content written
  after the round (see "The `closer` stops on a conflict").
- **An archive commit that changes `openspec/specs/` needs review.** It merges
  the spec delta into the live contract, and that merge is a result no
  reviewer has read in its merged form (see "The archive check").
- **A merge of `main` that stops on no conflict needs none.** It changes none
  of the piece's own lines, and what it brings in is already on `main`, so the
  squash gains nothing from it. A semantic clash — `main` renamed something the
  piece still calls — is CI's to see, and a red run comes back as one
  (`proposal.md`, "Out of scope").
- **The `closer`'s deletion of `findings/` needs none.** It removes tracking.
- **An archive commit that changes nothing under `openspec/specs/` needs
  none.** It moves the change folder and commits the deletion; a
  `skip_specs: true` change is always this case.

`RUNNER.md` states this definition because a runner needs it to decide whether
a round is owed at all.

### The re-review brief carries what differs from a first-round review

The reviewer role files describe a first-round review: tick your row, write your
findings file. In a re-review the row is already ticked and the findings file
exists. `RUNNER.md` tells the runner to put four things in the brief:

- the round and the commit range to read;
- that the reviewer's row stays as it is;
- that new findings are appended as boxes to the existing file, under a
  heading naming the round (next entry);
- that a re-review which finds nothing appends one ticked verdict box instead
  (next entry).

Appending still works with the findings gate, which only counts boxes.

**When `findings/` is already gone, the file is written afresh in the archived
folder.** The `closer` deletes `findings/` at the start of its Step 3, just
before it archives. So a re-review after a red run has no file to append to. `RUNNER.md` has the
brief say to write it under the same name, in
`openspec/changes/archive/<date>-<name>/findings/`. Same name, because the
runner reads reviewers by filename. That folder, because it is where the
re-dispatched `closer`'s Step 1 runs the gate (see the re-review row entry). A file written
anywhere else is one the gate never sees, and the finding in it would not block
the merge.

After the archive, every re-reviewer writes that file, holding its findings or
its verdict box.

**A conflict resolution is read with `git show --remerge-diff <sha>`.** The
resolution is a merge commit, and its diff against the piece is mostly `main`'s
own changes, which are already on `main` and are not the reviewer's business.
`--remerge-diff` re-runs git's own merge and diffs that result, conflict
markers included, against the committed one, so what a reviewer sees is the
resolution and nothing else. Measured with git 2.55.0 in a scratch repository:
on a merge whose one conflict was resolved to a third line, it printed exactly
the five conflict-marker lines replaced by that one line.

The alternative was editing four reviewer files. It is ruled out by scope: the
owner authorised edits for what these four issues ask for, and none asks for a
reviewer-file change.

### A clean re-reviewer appends a ticked verdict box

A re-reviewer that finds nothing appends one ticked box naming the round and
the commit range it read, and commits it itself, as it would a finding:

```markdown
- [x] **re-review round <n> `<range>`: no findings** — read <what>; clean
```

**Why a box at all.** A first-round reviewer's own tick on its stage row attests
that its review ran. A re-reviewer's row was ticked in the first round, so a
clean re-review that wrote nothing would leave no commit of the reviewer's: the
only record would be the runner's line under the re-review row, which nothing
would check against a reviewer's output (`findings/security.md`, second
finding). A
runner that wrote the line without dispatching, or a reviewer that under-read
its range, would look exactly like a clean round. The verdict box is the
reviewer's own trace, in the reviewer's own file.

**Why ticked.** The `closer`'s Step 1 runs two greps. `grep -rn "^- \[ \]"`
does not match a ticked box, so the verdict does not read as an open finding;
`grep -rc "^- \["` does count it, so a file holding only the verdict box passes
"every file non-zero". An unticked verdict would stop the `closer` as an
unanswered finding; a fresh file written after the archive with no box would
fail the non-zero check. Either sends the piece back over a review that found
nothing.

**Why one box for the round, and clean areas still in prose.** The reviewer
role files say to report clean areas in prose rather than as boxes, and the box
does not change that: it is one line saying the round ran. It takes the shape
#182 recommends for a clean first-round review — reviewer-side, one ticked
verdict box, because that "keeps the closer's gate mechanical" where the
alternative, a `closer` that accepts a zero-box file stating "no findings",
makes the gate read prose. So when #182 lands the two rules agree. #182's own
first-round case stays out of scope here: it needs the reviewer role files.

**Why the brief, not a role file.** The re-review is the runner's dispatch and
the brief already carries what differs from a first round; the reviewer files
are outside this piece's authorisation.

**What breaks without it:** a clean re-review before the archive is invisible
in `findings/`; after the archive a clean re-reviewer either writes a zero-box
file that stops the `closer`, or writes nothing, which is the untraced case
above. And `closer.md` Step 1's rule that an archived folder with no
`findings/` means no round ran after the archive would stop being true.

**Rejected: a clean re-reviewer writes nothing, and says so in its report.**
That was this piece's first version. It avoided tripping the gate, but it was
reasoned from what avoids the gate rather than from what leaves a trail, and a
report goes to the runner and is not kept.

**Findings go under a heading naming the round, and the runner checks for it
before it ticks.** A box nothing reads is a trace, not a gate. The verdict box
alone still left a round's completion attested by the runner's tick and
nothing else: two re-reviewers stalled on a permission prompt write nothing,
and a runner reading a stalled agent as finished, or rebuilding state after a
compaction, ticks the row (`findings/security.md`, re-review, second box). The
re-review row is the one stage row not ticked by the agent that did the work,
so the owner's "Agents tick their own" cannot protect it. So:

- **The runner's round lines are numbered.** Each starts
  ``round <n> `<range>` ``, numbered from 1 in the order the lines are
  written. A lane run again over a range it already had — a run the runner
  did not accept, or an agent replaced after a stall — gets a line of its
  own: the next number, the same range, the lanes it re-runs, and why. That
  holds however the lane is run again: a fresh dispatch, or the same agent
  continued with `SendMessage`, whose message then gives the new line's two
  forms whole, as a brief does. Continuing an agent gets no line only when it
  adds no review to a record already committed: finishing a round it has not
  yet recorded, such as after a stall, committing, or rebasing. Its record,
  once committed, is the round's own.
- **Every re-reviewer's file names the round, in one of two exact forms.** One
  with findings appends them under ``## Re-review round <n> `<range>` ``; a
  clean one appends the verdict box
  ``- [x] **re-review round <n> `<range>`: no findings** — read <what>; clean``.
  ``round <n> `<range>` `` is copied from the runner's line, and the brief
  gives both forms whole, not "such as", since the check below is a
  fixed-string search for them. A heading written loosely matches neither:
  `findings/architecture.md` has ``## Re-review round 1 (`c222c37..9dc235c`)``.
- **Before it ticks, the runner runs, for every unskipped round the tick
  closes,**
  ``git grep -l -F -e '## Re-review round <n> `<range>`' -e '**re-review round <n> `<range>`: no findings**' -- <change folder>/findings/``,
  on its HEAD once every lane's findings commit is there. It must list the
  file of every lane the round ran, except a lane a later round ran again
  over the same range, which that round's check covers. A lane not listed
  has not finished: the runner continues that reviewer or dispatches a fresh
  one, which gets its own line, and does not tick. The patterns are in
  single quotes because both hold backticks, which a shell expands inside
  double quotes.

**Why not the bare range.** The first version was
`git grep -l -F "<range>" -- <change folder>/findings/`. Reviewers write
ranges into prose routinely, earlier rounds' included: at `23aaac3`,
`git grep -n -F "c222c37..9dc235c"` over this piece's `findings/` returns
round 1's range on lines that are neither a heading nor a verdict box in five
of the six files (every file but `architecture.md`). One tick can close
several rounds, so a later round's reviewer citing an earlier range would count
as the earlier round's record, although that round's own reviewer wrote
nothing (`findings/security.md`, re-review `9dc235c..34fd428`, second box).
The exact forms are a heading and a box, which prose does not produce by
accident.

**Why the number.** The range cannot tell two runs of one lane over one range
apart. On this piece's round 1 the runner did not accept the Sonnet security
run and re-ran the lane on Opus; a search for the range alone is satisfied by
the Sonnet run's verdict box, whether or not the Opus run wrote anything
(`findings/security.md`, re-review `9dc235c..34fd428`, first box). With the
re-run on a line of its own, and the earlier line's check skipping the lanes a
later line re-ran, the rejected run's record satisfies nothing the re-run
must. The number also keeps consecutive rounds apart, which share an
endpoint: round N's range ends at the SHA round N+1's starts at, so a search
cut down to one SHA matches the previous round's records too.

**A continued run needs the number as much as a fresh one.** The rule first
gave a line only to a lane dispatched again, and said continuing the same agent
with `SendMessage` was not a new dispatch. But the runner's cheapest answer to
a run it rejects for "did not read X" is to continue that agent and say so, and
the agent's rejected record is already committed under the old number: the old
line's check passes on it before the continuation has done anything
(`findings/security.md`, re-review round 3 `34fd428..dc1390a`, box). So the
line is keyed to the lane being run again, not to how. Continuing an agent
gets no line only when it adds no review to a record already committed:
finishing a round it has not yet recorded, committing, or rebasing. The rebase
case is named on its own because "has not yet committed its record" would not
cover it: a reviewer continued to rebase after its cherry-pick conflicted has
committed its record, and its run was accepted, so keying on the commit alone
would give it a line for no review.

**Rejected:**

- **A separate per-dispatch tag**, the other fix `findings/security.md`
  offered. It adds a second identifier where the round line already carries
  one the runner writes anyway.
- **The two forms without the number** (``## Re-review `<range>` `` and
  ``**re-review `<range>`: no findings**``, which this piece's rounds 1 and 2
  were briefed with). They fix the prose hits, but not the re-run: both
  runs of a lane over one range write the same form.
- **Forbidding the continuation of a rejected run**, so that a lane is only
  ever run again by a fresh dispatch, which already got a line. It is also one
  rule, but it fights the cheapest step the runner reaches for when the defect
  is "did not read X", and a runner that continued anyway would reopen the
  hole, silently. Numbering the re-run closes it whichever way the lane is
  run.

**What it still cannot see.** A later reviewer who quotes an earlier round's
heading or verdict box whole, in prose, satisfies that round's check. The
forms are chosen to make that unlikely, not impossible: a finding that
discusses the forms quotes them with `<n>` and `<range>` as placeholders, as
`findings/security.md`'s outcomes do, and matches no real round.

**This piece's rounds 1 and 2 predate the number.** Their briefs gave the
un-numbered forms, so the check for those two rounds searches those forms,
and from round 3 on briefs use the numbered ones. Round 1 is the re-dispatch
case the number exists for, and it is settled without it: the Sonnet
correctness and readability runs wrote nothing, and `security.md` holds the
Opus run's own heading, ``## Re-review `c222c37..9dc235c` (Opus)``, committed
in `59619032`.

Measured on this tree at `23aaac3`:

- the un-numbered forms for round 1 (`c222c37..9dc235c`) and for round 2
  (`9dc235c..34fd428`) each list all six findings files;
- the numbered forms for round 2 list nothing, since no lane was briefed
  with them, so a check run with the wrong form fails closed;
- the numbered forms with the placeholders left in list `security.md`, whose
  outcomes quote them, so the single-quoted patterns do match text holding
  backticks and `**`;
- on an archived folder with no `findings/`, a pathspec that matches nothing
  lists nothing, so a round recorded there without a file fails closed: the
  runner cannot tick.

It prints file names only, so it stays inside what "What you read" lets the
runner read, and the table there has a row for it.

**What else was considered.** The finding offered a `closer`-side check as
well. It is deferred to the owner (`proposal.md`, "Out of scope"): the
`closer` deletes `findings/` when it archives, so after an archive it could
match only the rounds recorded since, and it would parse the runner's round
lines, whose only fixed part is their start. A runner-side check is one
command against files that already exist. **What breaks without it:** a
round's completion rests on the runner's reading of whether an agent has
finished, which is the failure a stalled agent produces; the residual, a
runner that skips the check, is in Risks. **What breaks without the number:**
a lane run again, fresh or continued, has its check satisfied by the run it
replaced. **What
breaks without the exact forms:** a later round's prose satisfies an earlier
round's check, and the runner ticks over a lane that never ran.

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
commits were brought onto the runner's branch afterwards, and never onto that
agent's `worktree-agent-<id>` branch. A continued
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

The existing sequence (hand-back → bring the commits on → remove the tree →
dispatch the next) already removes the tree, so the decision needs no change
there. It needs only the half-sentence in step 1 saying the callback is fresh.
Without it, a runner reading "continue an agent rather than starting one" in
`README.md` might try to revive the old one.

### Markers are shown to the `spec-writer` by a command scoped to the piece's diff, and unmarked decisions by quotation

`RUNNER.md` already forbids paraphrasing a finding into a brief. `NO SPEC:`
markers live in the code, so the brief names a command rather than listing
them. A behaviour decision the `dev-writer` reported without a marker exists
only in its hand-back, so there is no file to point to. The brief quotes that
sentence word for word. A quotation is not a paraphrase, and this is the one
place the runner holds the only copy.

**The command is scoped by the piece's diff:**
`git diff --name-only -G "NO SPEC:" origin/main...HEAD` lists the files where
the piece added or removed a marker line, and `git grep -n "NO SPEC:"` over
those files shows each marker.

- **An unscoped `git grep -n "NO SPEC:"` was the first version.** On this
  piece's tree at review time it returned 150 lines (`findings/correctness.md`):
  every marker already on `main`, every archived change
  that discusses one, and every role file describing the mechanism, `RUNNER.md`
  included. The real markers are a minority of it,
  so the brief would have handed the `spec-writer` the listing-by-hand it was
  meant to avoid.
- **A path scope, `-- dialectica dialectica-ui`, was ruled out.** It still
  returns markers from 24 files that are already on `main`, and it is a
  hand-maintained list of where markers may live, which goes stale silently
  the first time one lands elsewhere.
- **`-G` selects files, not hunks**, so the patch form of the same command is
  no better: on #165's squash commit it printed 47 KB. The `--name-only` form
  on that commit returns exactly `authoring.rs` and `transport.rs`, and the
  grep over those two returns four lines.

### After the `spec-writer`, the brief chooses who brings markers into line

If the `spec-writer` changed the behaviour, the code is now wrong, so the
`dev-writer` goes next and the `tester` after it. Otherwise the `tester` is next
anyway, and its brief names the `spec-writer`'s commit so it writes against the
new text.

**The brief also says which markers the `spec-writer` decided, and that each is
now closed:** reworded or removed to match the new spec text, not kept. The
runner learns which from the `spec-writer`'s hand-back. The reason is
`tester.md`, which said "Keep the markers and report each one — the spec-writer
decides". That was right when the `spec-writer` decided after the review round,
and it is out of date now that it decides before the `tester` runs. A tester
following it keeps a marker for behaviour that is now specified. The
`spec-test-reviewer` then finds the marker and routes it back to the
`spec-writer`, which is the #162 loop this change exists to end.

The durable fix is `tester.md` saying this itself, so that `RUNNER.md` points
to it rather than carrying the rule in a brief. The owner authorised that edit
for this piece, so `tester.md` now says what a decided marker becomes, and
`RUNNER.md`'s step 1 keeps only what the brief names — the commit and the
decided markers — and points to `tester.md` for the rest. The rule is stated
once.

### The brief asks the `spec-writer` to name product decisions, because nothing else makes it say so

#169 has the runner escalate "only what the `spec-writer` returns as a product
decision". Nothing under `.claude/agents/` tells the `spec-writer` to return
one: `spec-writer.md` has it decide every routed marker, with no branch for
"this is not mine to decide" (`findings/correctness.md`). Read literally, the
runner is told to act on a signal the flow never produces.

So `RUNNER.md` step 1 has the brief ask for it: which markers, if any, neither
the issue nor the specs settle, so that the choice is the owner's. The request
lives in the brief because the only file the runner reads is its own, and
`spec-writer.md` is outside this piece's authorisation beyond its one corrected
sentence.

### A hand-back naming no product decision leaves nothing for the owner, and closes only the markers it covers

What a `spec-writer` hand-back with no product decision settles is who acts
next: nothing goes to the owner. It does not settle that every marker is
closed. `RUNNER.md` step 1 has the next brief name as decided only the markers
the hand-back says new spec text now covers, or whose behaviour is to change. A
marker the `spec-writer` chose to leave in place stays open, and the brief does
not name it as decided.

**Why the two are separate.** `spec-writer.md` allows three outcomes for a
routed marker — add the requirement, say the behaviour should change, or leave
it in place, since "Leaving a marker in place is also a decision; say so rather
than ignoring it". Only the first two produce spec text a marker can cite.
`tester.md` closes a marker the brief names as decided by rewording it to cite
the scenario now covering it, or by removing it. Told that a left-in-place
marker was decided, a tester would reword it to cite a scenario that does not
exist, or remove it, and the chosen-not-specified behaviour would lose the one
marker that shows it was chosen.

**Rejected: "a hand-back that names none has decided them all".** That was this
piece's earlier wording. It joined the two questions, so a `spec-writer` that
left one marker in place and named no product decision would have had that
marker closed by the runner's brief. The spec-writer's ruling on this piece
(`f9df27f`) corrected it.

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

### The `closer` merges `main` rather than rebasing onto it

The owner chose this, over keeping the rebase, when taking
`findings/security.md`'s first finding into the piece. `closer.md` Step 2's fix
for a stale branch is now `git merge origin/main` in the `closer`'s own tree,
pushed with `git push origin HEAD:refs/heads/piece/<name>`. The rebase, its
`--force-with-lease` push and the paragraph defending it are gone, and "What
you never do" forbids force-pushing for any reason.

**Why a merge: a writer's conflict resolution has to be one commit that the
runner can bring onto its HEAD unchanged and a reviewer can read on its own.**
A merge gives exactly that — one commit, whose `--remerge-diff` is the
resolution. A rebase spreads a resolution across every replayed commit that
touched the conflicting lines, and rewrites the piece's history, which the
remote holds, so its push needs force. And a later rebase onto `main` would
drop a merge commit a writer made, raising the same conflict again.

**What else was considered:**

- **Keep the rebase, and have the `closer` resolve.** That was the text before
  this pass. The resolution is content no reviewer read, which is the finding.
- **Keep the rebase, and send the conflict to a writer who rebases.** The
  writer's rebase rewrites the piece's pushed history and needs a force-push,
  and its resolution is not one commit a reviewer can isolate.
- **Cherry-pick a writer's merge commit with `-m 1`.** Measured in a scratch
  repository: the picked commit has one parent, and merging `main`'s
  conflicting commit again afterwards stops on the same conflict. `main`'s
  commits are no longer ancestors of the piece, so git has no record that the
  conflict was ever resolved.

**Whether the branch reads flat does not matter here.** `README.md`'s
"cherry-pick rather than merge" is about the piece branch reading as a flat
sequence, and every piece squash-merges, so a merge of `main` on the piece
never reaches `main`'s history.

### The `closer` stops on a conflict, and the runner routes it to a writer

The `closer` does not resolve a conflict. When the merge of `main` stops, it
lists the conflicting paths with `git diff --name-only --diff-filter=U`, runs
`git merge --abort`, reports the paths and returns. The runner routes the
conflict by those paths with the existing red-run table, and the writer's
brief says to merge `origin/main` into its own branch and resolve there, not
to rebase. The resolution then goes through step 3 before the `closer` is
re-dispatched, with the brief naming `git show --remerge-diff <sha>`.

Measured with git 2.55.0 in a scratch repository:

- `git diff --name-only --diff-filter=U` during the stopped merge printed only
  the conflicting file — not an unrelated file deleted but not staged in the
  same working tree;
- `git merge --abort` put HEAD back on the pre-merge commit, and that
  unstaged deletion was still there afterwards;
- a plain `git merge origin/main` that merges cleanly completes without
  opening an editor when no terminal is attached, so the `closer` needs no
  `--no-edit`.

**Why not let the `closer` resolve and have the runner review it afterwards.**
The `closer` decides nothing about the content, and a resolution is a
decision about content. Its turn would end at the resolution anyway, since a
commit that needs review ends it, so resolving buys nothing but an author who
did not read the change.

### The `closer` deletes `findings/` at the start of Step 3, not in Step 1

Step 1 still runs both gates and confirms the durable reasoning is in
`design.md`, and the ownership paragraph stays there; the deletion itself
happens at the start of Step 3, immediately before `openspec archive`, and is
committed with the archive. On a re-dispatch, a re-review's `findings/` in the
archived folder is deleted at the same point in Step 3 and committed there.

**Why: Step 2's merge of `main` sits between them.** A deletion made in Step 1
is still uncommitted when Step 2 merges. Measured with git 2.55.0 in a scratch
repository, re-run on this pass: with the deletion staged by `git rm -r`,
`git merge main` refuses with `error: Your local changes to the following files
would be overwritten by merge:` naming the findings file, although `main`'s
only change was to a different file. Unstaged, the merge runs and `--abort`
keeps the deletion, but that relies on git carrying uncommitted changes across
a merge and its abort. After the move, nothing is uncommitted while Step 2 runs,
in either form.

**Why it is inside the Step 2 rewrite's authorisation.** Before this piece
Step 2 rebased, which refuses any uncommitted change, so the Step 1 deletion was
already a latent stop. Replacing the rebase with a merge made it a stop on
every first dispatch whose branch is stale, since Step 1 deleted `findings/` on
every first dispatch. The owner-authorised rewrite of Step 2 would not work
without it.

**Rejected: commit the deletion in Step 1.** It would work, but it splits the
deletion from the archive commit whose message Step 3 has explain it, so the
diff does not read as removing review evidence; one commit carrying both keeps
that explanation next to the deletion. **Rejected: tell the
`closer` to leave the deletion unstaged.** It works, measured above, but it is
a guard on how a shell command is typed, and nothing would check it.

**What breaks without it:** a stale first dispatch stops at Step 2 on an error
caused by its own housekeeping, which is not a conflict and which Step 2 does
not describe.

### The archive check applies only to an archive commit the `closer` made in the same run

Straight after Step 3's archive commit, before its push, a `closer` that made
the archive commit in that run runs
`git diff --name-only HEAD^ HEAD -- openspec/specs/`. Then it pushes, and acts
on the push and the check together:

- **push refused:** it stops and reports the refusal and the check's result,
  the files listed or that none were;
- **push accepted, files listed:** it stops before Step 4, reports the archive
  commit and the files, and returns; the runner sizes a round for that commit,
  then re-dispatches;
- **push accepted, nothing listed:** it carries on to Step 4.

The owner chose to stop when specs change, rather than to review every archive
or none.

**Why before the push.** The check was first placed after it, which left one
path where it never ran. A refused push ends the `closer`'s turn, and a
re-dispatched `closer` skips the check (below), so an archive whose push was
refused was checked by nobody: a spec-changing archive could reach `main` with
no round ever sized for it (`findings/security.md`, re-review). The check reads
only the local commit, and the push does not move HEAD, so running it first
measures the same thing. What changes is that its result exists on every path
out of Step 3, including the refusal, and `closer.md`'s "Your report" asks for
it there. On the runner's side, "A refused push" in "The `closer`, and what
comes back" has the runner untick the re-review row and record a round for the
archive before it reports the refusal to the owner, so the round is owed
whatever the owner then does about the push. Removing the move puts the check
back after a step that can end the turn, and the refused-push path is again
unchecked; nothing mechanical would notice, since no test runs `closer.md`.

Measured: on `2bda577f` (#181, which changed two specs) the command lists
`openspec/specs/feed-view/spec.md` and `openspec/specs/stoa-navigation-view/spec.md`;
on this tree's HEAD it lists nothing.

- **`HEAD^ HEAD`.** The check runs straight after Step 3's commit, so HEAD is
  the archive commit and `HEAD^` its first parent, which is the merge of
  `main` if Step 2 made one. It measures the archive and nothing else.
- **Only in the same run.** A re-dispatched `closer` finds the change archived
  and makes no archive commit, so its HEAD is whatever the runner brought onto
  the piece last — a fix, a resolution, the runner's tick — which step 3 has
  already reviewed. Run there, `HEAD^ HEAD` would measure the wrong commit:
  a reviewed spec callback at HEAD would stop the `closer` for nothing, and a
  tick at HEAD would say nothing about the archive.
- **Not every archive.** An archive that changes no spec — every
  `skip_specs: true` change, this one included — would bounce off a review
  with nothing to read.

**A re-dispatched `closer` pushes HEAD whether or not it deleted anything.**
Its tree carries every commit the runner brought onto the piece since the last
push: a fix, a conflict resolution, the runner's record lines and tick. The run
it watches in Step 4 must include them. The earlier reading of Step 3 — push
only when a re-review's `findings/` was deleted — would have had it watch a
run on a tip missing the very fix it was re-dispatched for.

### The runner fast-forwards to the `closer`, a conflict resolver and an already-pushed branch; it cherry-picks everyone else

`git cherry-pick` cannot carry a merge commit's second parent: it refuses a
merge outright (`is a merge but no -m option was given`, measured), and with
`-m` it drops the parent, as above. Both the `closer`'s branch and a conflict
resolver's can carry a merge of `main`, so the runner runs
`git merge --ff-only <branch>` for them instead. It applies because the agent
forked from the runner's HEAD and nothing moves that HEAD while the agent runs.
Measured: a fast-forward to a branch holding a resolved merge commit applies;
one to a branch that has diverged is refused with
`fatal: Not possible to fast-forward, aborting.`, and the hint above it offers
`git merge --no-ff` and `git rebase`. `RUNNER.md` tells the runner to take
neither and stop: `--no-ff` would add a merge commit that is the runner's own
content, and the runner never rebases `piece/<name>`.

For the `closer`, the fast-forward comes first whenever it returns without
having merged, before the runner writes to the stage block or dispatches
anyone. Its archive commit moved the stage block, so without the fast-forward
the archived `tasks.md` the runner unticks is not in the runner's tree, and the
next agent forks from a HEAD without the archive or the merge of `main`.

**An agent whose commits are already on the remote piece ref is fast-forwarded
to as well.** In this flow that is every `dev-writer` pass. `dev-writer.md` has
it push its tip to `refs/heads/piece/<name>` before handing back on its first
pass, so the PR opens early enough for CI, and "On the findings pass it always
does": a writer answering findings, a red-CI fixer and a pass after a
`spec-writer` callback all push. A cherry-pick copies those commits to new SHAs, and from then on
the runner's `piece/<name>` does not descend from the remote ref. Every later
push of a HEAD forked from it — the `closer`'s at Step 2 or Step 3, a
findings-pass writer's — is refused as non-fast-forward, and no agent may
force.

This is not hypothetical; it is in this piece's own reflog
(`git reflog show --date=iso piece/171-workflow-rules`). The `dev-writer`
pushed `c67aa24`..`d41d0fd`, forked from the runner's `abc7693`. At 23:32:26–27
on 2026-09-25 the runner cherry-picked them as `a255c19`, `c64f549`, `68211d9`
and `938dc55`, new SHAs even though `abc7693` was the first one's parent, and at
23:32:47 it ran `reset: moving to d41d0fd` to get back onto the pushed commits.
That reset is what `RUNNER.md` now forbids ("never rebase, reset or force-push
`piece/<name>`"), so without this rule the same mistake would have no exit but
the owner.

Measured with git 2.55.0 in a scratch repository, re-run on this pass:

- a plain `git cherry-pick` of a pushed commit whose parent is HEAD made a new
  SHA; a branch forked from it and pushed to the piece ref was refused with
  `! [rejected] HEAD -> piece (non-fast-forward)`;
- `git cherry-pick --ff` onto a HEAD that is not the commit's parent exits 0 and
  makes a new SHA with no warning, so `--ff` is not a safe substitute: it
  diverges silently in exactly the case that matters;
- `git merge --ff-only` to the pushed branch kept its SHA, and on a diverged
  branch refused with `fatal: Not possible to fast-forward, aborting.`

So the rule names the case by its condition — commits already on the remote
piece ref — with the `dev-writer` as the instance, on every pass. An earlier
version named only its first pass, which read as the complete list: a runner
following it cherry-picked every later pass, and the next `closer` push was
refused (`findings/correctness.md`, re-review, first box, reproduced in a
scratch repository down to the `=` pair the refused-push diagnostic prints).
On this piece the runner's reflog shows findings-pass commits brought on as
`merge 619e679: Fast-forward` and `cherry-pick: fast-forward`; the second was
safe only because the runner's HEAD was the commit's parent, and nothing in
the text told the runner to do either. Every `RUNNER.md`
passage that gave the route for a `dev-writer`'s commits now says
fast-forward or points to "Dispatching": the per-agent sequence, the
already-pushed paragraph, "How many at once", the red-run fixer, and both
sentences in "One piece is one PR". `README.md`'s branch section says the
`dev-writer` pushes on every pass. Passages in `RUNNER.md` and `README.md` about
every agent's commits say "brought onto" the piece rather than naming either
route, since a reviewer is still cherry-picked and a `dev-writer` never is.

**The criterion for which old-route sentences change, in every file this piece
edits:** a sentence saying the `dev-writer`'s commits, or every agent's, reach
the piece by cherry-pick is false under this rule and contradicts `RUNNER.md`,
so its route words change to "brought onto" or "brings … onto"; a sentence
true of a cherry-pick whatever the route, or about an agent that is still
cherry-picked, stays. By it, `dev-writer.md`'s four statements of its own
route changed, and only in the words naming the route: "The runner brings it
onto `piece/<name>` once you hand back"; "you do not need the runner bringing
them onto it"; "The runner brings your commits onto its own local
`piece/<name>` afterwards"; and "Bringing your commits onto the local
`piece/<name>` is the runner's". Its "is not something the runner can
cherry-pick", about commits made on the piece branch when isolation did not
take, stays: it says what the runner cannot do with commits on the wrong
branch, which holds of either route. `README.md`'s stage-block sentence stays
for the same reason (see "`spec-writer.md`'s 'neighbouring rows cannot
conflict' is corrected").

**Rejected: keep `dev-writer.md`'s route sentences because bringing its passes
on is the runner's step and changes nothing the `dev-writer` does.** That was
this piece's earlier reason. It is equally true of the `README.md` passages the
piece did correct, so it did not separate the cases; and the runner reads
`dev-writer.md` for this very sequence — `RUNNER.md`'s "One piece is one PR"
says it "states the sequence and owns it" — so the old wording would have met
a runner as the route to follow (`findings/spec-test.md` and
`findings/design-review.md`, re-review `9dc235c..34fd428`).

**A refused push goes to the owner, not to a fix of the runner's.** If one
happens anyway — the remote piece ref holds a commit the runner's HEAD lacks —
the runner cannot reconcile the two without a reset, a force-push or a merge
of its own, and "What a runner does" forbids all three. So `RUNNER.md`'s "The
`closer`, and what comes back" routes it to the owner: report
`git log --oneline --left-right --cherry-mark HEAD...origin/piece/<name>` and
wait. That command was chosen because it shows both what diverged and whether
it is only copies: in the scratch repository it printed `<` for the commit only
HEAD held and a `=` pair for a commit and its cherry-picked copy, and `>` for a
commit only the remote held. The `closer`'s side of the same event is
`closer.md`'s: a refused push at Step 2 or Step 3 stops it, with no force and
no fetch and merge of the remote piece ref, because what the remote holds that
the runner's HEAD lacks is not known to have been reviewed.

### The `closer`'s returns are given as examples, with no count

"The `closer`, and what comes back" once opened "Two things come back", and
this piece's earlier passes raised the number as returns were added, to five.
A number is a claim the reader can check, and it checked false:
`closer.md` Step 6 stops on a PR that stays `BLOCKED` with every required check
green — the very case #170 is about — and the section had no entry for it, so a
runner counting five found no route (`findings/readability.md`, re-review,
first box). `closer.md` has more stops than the section routes: a findings file
with zero boxes, more than one change folder or none, a PR body that disagrees
with the diff, isolation that did not take. So the section now says its returns
are examples, and a return it does not name comes with its evidence and is
routed by what it is. It routes six:

- **a red run, a conflict, and an archive commit that changed
  `openspec/specs/`**, each back through step 3;
- **an unticked box**, widened from "a finding never answered" to a stage row
  as well. Step 3 relies on the `closer` returning on an unticked re-review
  row, which the old gloss did not fit. Another agent's row goes back to that
  agent, continued to tick it, or to a fresh agent for the stage, since
  "Agents tick their own"; the re-review row means a round is owed;
- **a refused push**, to the owner, and when the `closer`'s report also lists
  files from the archive check, the runner first unticks the re-review row and
  records a round (see "The archive check …");
- **a PR that stays `BLOCKED` with every required check green**, to the owner
  with the `gh pr view` output the `closer` reported, with no other route and
  no diagnosis, which is out of scope here.

**Rejected: keep a count and add the missing entries.** The count would be
right until the next stop is added to `closer.md`, with nothing to notice that
it had gone wrong. Examples plus a rule for the unnamed case stay true.

### What the runner commits: its re-review row and nothing else

`findings/architecture.md`'s owner finding asked whether a runner may write an
edit itself when a dispatched agent's attempt is refused, as happened in
`d805fe4`. The owner took it into the piece and ruled: **"A runner always
delegates"** and **"Agents tick their own"**. `RUNNER.md`'s "What a runner
does" gains "What a runner commits", which states the rule once; step 3 and
the per-agent sequence point to it.

- The runner's own content is the re-review row's record lines, tick and
  untick.
- Bringing an agent's commits on — a clean cherry-pick or a fast-forward —
  adds no content of the runner's.
- Every agent ticks its own row. When a hand-back reports a stage done and
  the row is still unticked, the runner continues that agent to tick it.
- Everything else is work, delegated with no exception: a proposal, a spec,
  `design.md`, code, tests, role-file text, a finding's outcome, and another
  agent's tick. **Who commits and whether a commit needs review are separate
  questions.** A tick and a finding's outcome need no review under step 3,
  and that does not make them the runner's.

**Alternatives the owner ruled out:**

- **A standing exception for edits the owner asks for in session** — the
  finding's framing, and `d805fe4`'s practice. An owner's edit request is now
  an instruction to dispatch the agent whose file it is. An agent refused an
  edit it was briefed to make is reported to the owner, and the runner does not
  make it.
- **The runner ticking reviewers' rows to avoid the review round's conflict** —
  `a284e51`'s practice, whose message gives the conflict as its reason. The
  conflict now goes back to each agent (next entry).
- **Copying an agent's uncommitted output onto the piece** — `c43c7a7`'s
  practice, after a signing failure. It commits content the runner did not
  write under the runner's commit. The agent is continued to commit it once
  the blocker is cleared; if only the owner can clear it, the runner reports
  and waits; if the agent cannot be continued, a fresh agent redoes the stage.

Those four commits predate the ruling and stand as they are (`proposal.md`).

### A cherry-pick that conflicts goes back to its agent

The runner runs `git cherry-pick --abort`, keeps the agent's tree, and
continues the agent with `SendMessage`: it rebases its own branch onto
`piece/<name>`, resolves there, and reports back. The runner picks again. The
agent's branch is local and never pushed, so that rebase rewrites nothing
anyone else holds — unlike a rebase of `piece/<name>`, which the runner never
does.

**The review round meets this every time, and `RUNNER.md` says so**, so the
runner expects it rather than reading it as a fault. The reviewers fork from
one HEAD, each ticks its own row, and the review rows are adjacent. Measured
with git 2.55.0 in a scratch repository holding the template's stage block,
re-run on this pass: with the correctness tick picked, the security tick (next
row) stops with `CONFLICT (content): Merge conflict in tasks.md`, and the
readability tick (one unchanged row between) applies cleanly. Git conflicts on
neighbouring changed lines, not only on the same line. So picked in template
order, each landing before the next is picked, each of the second to sixth
review picks stops. **No order stops fewer
than three**: a pick is clean only if neither neighbouring row is ticked yet,
and at most three of six consecutive rows can be picked that way. Sequential
stages never meet it, since each forks after the previous tick is on the
runner's HEAD. The cost is at least three `SendMessage` round trips per review
round, one after another, each for a one-character change.

`RUNNER.md` states this without the count of six, so it stays true when #132
tiers the round; the count and the bound live here.

**What was considered and not done in this piece** (`proposal.md`, "Out of
scope", has the text for the follow-up issue):

- **One file per stage row**, the owner's suggestion. Two commits that change
  different files cannot conflict in content, in any order — the reason six
  reviewers already write six findings files at once without conflict. It
  reshapes the stage block that four role files read, and the owner cannot
  rule on a redesign while away.
- **A blank line between the template's rows.** The measurement shows one
  unchanged line between two ticks is enough, so it would work — but only
  while every later edit to the template keeps the spacing, which is a guard
  nothing checks.
- **Serialising the reviewers.** It removes the conflict by giving up the
  parallel round.

### An agent with uncommitted changes saves them as a patch around its rebase

The conflict rule above sends the review round's ticks back to their agents to
rebase, and a mutating reviewer cannot: `code-reviewer.md` and
`spec-test-reviewer.md` have it leave its mutations uncommitted, as evidence
only the runner may discard, and `git rebase` refuses a dirty tree
(`findings/correctness.md`, re-review, second box). `RUNNER.md` states the
steps by that condition, not by role, and the runner's continuation message
carries them:

1. `mkdir -p tmp`, then `git diff --binary --output=tmp/uncommitted.patch HEAD`;
2. `git restore --source=HEAD --staged --worktree -- .`;
3. `git rebase piece/<name>`, resolve, `git add`, `git rebase --continue`;
4. `git apply tmp/uncommitted.patch`, and report whether it applied.

Measured with git 2.55.0 in a scratch repository with the template's review
rows, three reviewer worktrees forked from one HEAD, each ticking its own row:

- with one reviewer holding an unstaged edit to one file and a staged edit to
  another, `git rebase piece/x` refused with `error: cannot rebase: You have
  unstaged changes.` / `error: additionally, your index contains uncommitted
  changes.`;
- `git diff --output=tmp/…` failed with `could not open 'tmp/mutations.patch'
  for writing: No such file or directory` until the directory existed, hence
  the `mkdir -p` in step 1; once it did, the patch held both edits, staged
  and unstaged, since `git diff HEAD` compares the tree with the commit;
- after step 2, `git status --porcelain --ignored` printed only `!! tmp/`: the
  tree and index were clean and the patch file, being ignored, survived;
- step 3 stopped with `CONFLICT (content): Merge conflict in tasks.md`, and
  after the file was resolved and added, `git rebase --continue` finished with
  `Successfully rebased and updated refs/heads/rev-b` (the agent shells'
  `git var GIT_EDITOR` is `true`, so it opened no editor);
- step 4 applied with no output, and `git diff HEAD` afterwards was
  byte-for-byte the saved patch; both edits came back unstaged, so the staged
  split is not kept, which a mutation does not need;
- the runner's next `git cherry-pick rev-b` applied cleanly.

**Untracked files are outside the four steps, and `RUNNER.md` says what that
means for step 4.** `git diff HEAD` does not see an untracked file, the
restore leaves it, and it does not stop the rebase, so it stays in the tree
throughout; a staged new file is in the patch but comes back untracked. A
tree holding only untracked files therefore saves an empty patch, and step 4
fails: measured with git 2.55.0 in a scratch repository under `./tmp/`
(since deleted), one committed file and one untracked `probe.txt`, the save
exited 0, the restore left `?? probe.txt`, and `git apply` exited 128 with
`error: No valid patches in input (allow with "--allow-empty")`. A reviewer
following step 4 would report that the patch did not apply, which reads as
lost evidence although nothing was lost (`findings/correctness.md`, re-review
`9dc235c..34fd428`). So `RUNNER.md` says that refusal means nothing tracked
was saved, and has the runner carry that sentence in its message.
**Rejected:** `git apply --allow-empty`, which would make step 4 succeed on an
empty patch, because the command is quoted in `proposal.md` as a plain
`git apply` and in its standing-test inventory; tying the steps to the
rebase's own refusal, which would reorder the four steps `proposal.md` sets
out; and a precondition check before step 1, which would add a fifth command
to that inventory for a case that loses nothing. The only reachable exception, an untracked file at a path
an incoming commit adds, stops the rebase before it changes anything; in the
review round the incoming commits are other lanes' ticks and findings, which a
reviewer does not hold untracked.

The patch is written by `git diff --output` rather than `> file` because a
redirect is a shape this repo's permission checker cannot analyse, so every
reviewer would stall on an approval prompt mid-round.

**Signing.** `RUNNER.md` names no signing flag: signing is off for the days the
owner is away, and the runner's message adds `--no-gpg-sign` as every brief
does. Measured in the same scratch repository with `commit.gpgsign true` and
`gpg.program` pointed at a missing binary, so any signing attempt fails fast:
`git rebase --no-gpg-sign piece/x` stopped on the conflict, and
`git rebase --continue`, with no flag of its own, completed without trying to
sign, while a plain `git commit` in the same tree failed with
`gpg failed to sign the data`. So the flag given to the rebase carries across
its `--continue`, which is where the correctness re-reviewer's run hung.

**Not chosen:**

- **Committing the mutations.** It would put them on the piece: the runner's
  next pick carries them.
- **Discarding them** (`git checkout -- .`). It destroys the evidence the
  reviewer's findings cite, which `code-reviewer.md` says only the runner may
  discard.
- **`git rebase --autostash`.** It got past the refusal in the correctness
  re-reviewer's scratch run, but whether its restore is intact was never
  measured, and when it cannot re-apply it leaves the changes in the stash
  list, which every worktree shares — the reason `CLAUDE.md` bans a bare
  `git stash`. A patch file in the agent's own ignored `tmp/` belongs to that
  tree alone, and survives a failed re-apply as a file anyone can read.

**The durable home is the reviewer role files**, which should say what a
reviewer does with its mutations when continued to rebase. They are outside
this piece's authorisation, so that edit is a follow-up for the owner
(`proposal.md`, "Out of scope"). **What breaks without the steps:** a mutating
reviewer continued to rebase is left to invent a route or stop, and every
later pick in the round waits on it.

### `spec-writer.md`'s "neighbouring rows cannot conflict" is corrected

Its sentence "That is what keeps their cherry-picks clean — git conflicts on
the same line, not on neighbouring ones" is false by the measurement above. It
was also the premise behind the runner ticking reviewers' rows. Left as it is,
the file holding the template would say the review round's picks are clean
while `RUNNER.md`, in the same piece, tells the runner they conflict. The
replacement says three things: a sequentially forked agent picks cleanly;
adjacent ticks from one HEAD conflict, because git conflicts on neighbouring
lines too; and `RUNNER.md`'s "Dispatching" says who resolves that. No row of
the template changes.

It is inside the piece's authorisation because it follows from a ruling the
owner took into the piece: it corrects that ruling's premise in the file that
stated it, and adds no rule.

**`dev-writer.md` stated the same premise, and its clause is corrected the same
way.** Its "`tasks.md`, and where your work lands" said the `dev-writer` ticks
only its own row and adds none, "so concurrent agents' cherry-picks do not
conflict" (`findings/readability.md`, re-review). That reason is false by the
same measurement and contradicted `RUNNER.md` in the same way, so the same
reasoning puts it inside the authorisation. Only that clause changed: it is now
a pointer to `spec-writer.md`'s stage-block paragraph, which says when ticks
conflict and points on to `RUNNER.md`, so the account lives in one place. The
rule itself — tick exactly one row, your own, add none — is unchanged, and the
only other `dev-writer.md` edits are its four route sentences (see "The runner
fast-forwards to …").

**`README.md`'s stage-block section is kept, although it reads close to the
same premise.** It says one row per agent instance keeps four instances off
"the same line, which is the conflict one-row-per-agent exists to prevent", and
"Each agent flips its own row and adds none, so concurrent cherry-picks
never touch the same line". Both are literally true: each tick changes its own
line and no other. Neither says the picks are clean, which is the claim
`spec-writer.md` and `dev-writer.md` made and the measurement refutes; the
reader has to supply "so they do not conflict". The criterion is the one the
route sentences follow: a false sentence changes, a true one stays. "Each
agent flips its own row" also stands as the owner ruled ("Agents tick their
own"). Whether that section should go on to say neighbouring ticks conflict
is the one-file-per-stage-row follow-up's to settle, and `proposal.md`'s
entry for it lists that section among what would change
(`findings/design-review.md`, re-review `9dc235c..34fd428`).

### `closer.md`'s signing text is deliberately not edited

The owner has switched signing off for the days they are away and called it
"temporary". Two `closer.md` passages are false for that period: Step 2's "a
signing failure is a stop-and-ask… do not reach for `--no-gpg-sign`", and
Step 6's "four green checks and a signed commit". Both become true again when
signing is back, and editing them now would need a second edit to undo it.
The Step 2 paragraph keeps its rule; the only change is the operation it
names, from "a rebase" to "the merge of `main`", because Step 2 no longer
rebases. For that period `--no-gpg-sign` lives in the runner's briefs and in
no role file.

## Risks / Trade-offs

- **[A re-review round can itself produce findings, so rounds can repeat.]** →
  Each round is sized to the commits since the last one, so rounds get smaller.
  The row is ticked when a round adds no merging commit. `RUNNER.md` does not
  cap the number of rounds: a cap would be a fixed rule, and #171 argued
  against fixed rules.
- **[The git commands the role files name have no standing test.]** Several
  are deterministic: `closer.md` Step 1's `git ls-files` pathspec and Step 3's
  `openspec/specs/` check; `RUNNER.md`'s scoped `NO SPEC:` command and step
  3's pre-tick `git grep -l -F -e … -e …` for the two exact forms; and
  "Dispatching"'s `mkdir -p tmp`, `git diff --binary
  --output=tmp/uncommitted.patch HEAD`, `git restore --source=HEAD --staged
  --worktree -- .` and `git apply tmp/uncommitted.patch`. Each was measured
  once, when it was written, and nothing re-runs them (`findings/spec-test.md`).
  They do not fail alike:
  - **fail closed**, the agent stops or reports: a mistyped `ls-files`
    pathspec returns no path or several, and the `closer` stops; a pre-tick
    pattern with a wrong character lists nothing, and the runner cannot tick;
    the save with `--binary` dropped writes `Binary files … differ` for a
    binary change, `git apply` refuses it, and the reviewer reports it — but
    step 2 has already discarded that change, so the loss is reported, not
    prevented;
  - **fail open**, every command exits 0 and the flow carries on: a mistyped
    `openspec/specs/` path lists nothing, and an unreviewed spec change would
    merge; a pre-tick search cut down to the bare range matches it in prose,
    and one cut to a single SHA also matches the previous round's records,
    so the runner ticks over lanes that never ran; and the save with `HEAD`
    dropped writes only the unstaged changes, step 2 discards the staged
    ones, and step 4 applies cleanly with the staged mutation gone
    (`findings/architecture.md` and `findings/spec-test.md`, re-review
    `9dc235c..34fd428`, both measured).

  `RUNNER.md`'s `--ff-only`, `--remerge-diff` and `--cherry-mark` claims
  describe git's own behaviour, and a test of them would mostly re-test git.
  → Deferred to the standing-test follow-up in `proposal.md`'s "Out of
  scope", listed in PR #174's follow-ups in place of the earlier
  pathspec-only one: a script under the `lint` job that extracts each command
  from the role file and runs it against fixtures, the three fail-open cases
  first. Not added here: this change adds no tests or CI, and a test holding
  its own copy of a command would not fail when a role file's copy changed.
- **[Only the runner runs the pre-tick check.]** It makes the re-review row's
  tick rest on the re-reviewers' own records, but a runner that skips it goes
  unnoticed: the `closer` reads neither the heading nor the verdict box. →
  Deferred to the owner as a design question (`proposal.md`, "Out of scope",
  "An independent check of the re-review row by the `closer`").
- **[The review round costs at least three `SendMessage` round trips.]** →
  Accepted for this piece. The one-file-per-row follow-up removes it.
- **[`tester.md` said "Keep the markers and report each one — the spec-writer
  decides".]** Once the `spec-writer` has decided before the tester runs, that
  sentence was out of date: a tester following it would keep a marker for
  behaviour that is now specified. → The owner authorised the `tester.md` edit
  for this piece. `tester.md` now says what a tester does with a marker the
  brief names as decided (reword to cite the scenario, or remove) and keeps
  "must not remove" for an **open** marker only. `RUNNER.md` step 1 carries only
  what the brief must list, and points to `tester.md` for the rest, so the rule
  is stated once. The residual risk: a runner that leaves the list out of a
  brief gets the old behaviour, and nothing checks the brief.
- **[A re-dispatched `closer` did not know Step 3 was done.]** After a red run,
  `closer.md`'s Step 1 finds the change archived, but Step 3 did not say the
  archive was already done, so a fresh `closer` could run `openspec archive` on
  a change that is no longer live. → The owner authorised the edit for this
  piece. Step 3 now opens with that case: `openspec archive` is not run again,
  and HEAD is pushed whether or not it deleted anything.
- **[`closer.md`'s signing text is false while signing is off.]** → Deliberate;
  see the Decisions entry. Do not "fix" it.
- **[PR #132 changes six of the same files under `.claude/agents/`.]** → The
  second to merge reconciles them. `proposal.md`'s "Overlap with open PR #132"
  gives the measured picture, from a trial merge
  (`git merge-tree --write-tree --name-only HEAD origin/piece/review-tiering`):
  the four conflict blocks this piece adds, the auto-merged hunks still to be
  read in place, and the command to re-run after any later edit. None of
  #132's `closer.md` hunks touches Step 2's merge of `main`. The re-review
  row's wording does not depend on lane names, so #132's merge of readability
  into correctness does not break it.
- **[Nothing mechanical checks any of this prose.]** → The six reviewers are the
  check. `tasks.md` strikes the tester row with the reason.
