# Workflow rules: review everything that merges, route `NO SPEC:` first, forbid `--admin`, restore one README paragraph

Closes #171, #170, #169 and #133. All four are `workflow` issues, and all four
edit files under `.claude/agents/`. The owner authorised those edits for this
piece, limited to what the four issues ask for and to four additions the owner
authorised in session, each marked **owner-authorised** below. One sentence in
`spec-writer.md`, and one clause in `dev-writer.md` stating the same premise,
are corrected as well: both are measured false, and left alone either would
contradict a rule this piece adds under an owner ruling. So are the passages in
`README.md` and `dev-writer.md` that name the cherry-pick as the route for the
`dev-writer`'s commits, which the owner-authorised fast-forward rule makes
false. The entries for them say why that is inside the authorisation.

## Why

On 2026-09-25 two pieces merged with code no reviewer had read. On #134 (PR
#164), four scripts and two workflows were rewritten on an owner instruction
after the six reviews were in. On #162 (PR #165), a spec callback, two
`design.md` rewrites, doc rewording and a new test all landed after review.
`RUNNER.md` never says that post-review work needs review. For a red CI run it
says the opposite: re-dispatch the `closer` straight after the fixer (#171).

On #162 the `dev-writer` handed back two `NO SPEC:` markers. The runner put them
to the owner as open decisions. Nothing in `RUNNER.md` says they go to the
`spec-writer`: that routing is written only in `spec-writer.md` and `tester.md`,
which the runner does not read. So the tester and all six reviewers worked
against a spec that said nothing on either point, and a second round of
writing followed (#169).

The same piece's `closer` got a `BLOCKED` PR with every required check green,
and merged it with `gh pr merge --admin`. Neither the brief nor `closer.md`
said that "merge on green" stops at overriding branch protection (#170).

Separately, #131 reverted #119's unauthorised rewrite of the agent files, and
one reworded paragraph got through. It is the `settings.json` paragraph in
`.claude/agents/README.md`, which a fixer changed while answering a #131 review
finding, without the owner's authorisation (#133).

This piece's own review round found two more gaps, both addressed to the owner
because they reach beyond the four issues. The `closer`'s own commits reach
`main` unreviewed: it resolves a rebase conflict by hand, and its archive
commit merges a spec delta into the live contract, and #171's re-review rule
covers neither (`findings/security.md`). And `RUNNER.md` says without exception
that the runner does not write the work, although on this piece the runner
committed role-file edits on the owner's direct instruction (`d805fe4`),
stage-row ticks on reviewers' behalf (`a284e51`, `e7e2bbd`), and findings files
copied from reviewers' trees (`c43c7a7`). Nothing says which of those a runner
may do (`findings/architecture.md`). The owner took both into this piece, and
ruled on the second: **"A runner always delegates"**, and **"Agents tick their
own"**. None of those four commits is a runner's to make.

## What Changes

The rules the runner follows from the `dev-writer`'s hand-back to the `closer`
read as **one sequence in `RUNNER.md`**. #169 covers the step before the tester
and #171 covers everything after the review round. Neither rule is copied into
another file.

- **#169: route `NO SPEC:` to the `spec-writer` before the `tester`.**
  `RUNNER.md` says that when the `dev-writer`'s hand-back names a `NO SPEC:`
  marker, or a behaviour decision it made where the spec said nothing, the
  runner dispatches the `spec-writer` next, before the `tester`. The
  `dev-writer` or `tester` then brings the markers and tests into line with the
  new spec text. The runner does not put markers to the owner. It escalates
  only what the `spec-writer` returns as a product decision, and says what the
  choice is. Specifically:
  - **The callback is a fresh `spec-writer` dispatch**, not a continuation of
    the one that wrote the spec. #169's optional suggestion, keeping that
    agent's tree until the `dev-writer`'s first pass so it could be continued,
    is **declined**; `design.md` gives why.
  - **The brief points at markers by a command scoped to the piece's diff**,
    rather than listing them: `git diff --name-only -G "NO SPEC:"
    origin/main...HEAD` names the files where the piece added or removed a
    marker line, and `git grep -n "NO SPEC:"` over those files shows each
    marker. An unscoped `git grep -n "NO SPEC:"` is not the command: it also
    returns every marker already on `main` and every file that describes the
    mechanism. **A behaviour decision reported without a marker is quoted
    word for word** from the hand-back, since no file holds it.
  - **The brief asks the `spec-writer` to name any product decision**: the
    markers, if any, that neither the issue nor the specs settle, so the
    choice is the owner's. `spec-writer.md` gives it no such category, so the
    brief is what makes it say so.
  - **A hand-back that names no product decision leaves nothing for the
    owner. It does not make every marker closed.** A marker is decided, in
    the sense `tester.md` closes, when the hand-back says new spec text now
    covers it or that the behaviour is to change. A marker the `spec-writer`
    chose to leave in place ("leaving a marker in place is also a decision",
    `spec-writer.md`) is not closed, and the next brief does not name it as
    decided: a tester told it was decided would reword it to cite a scenario
    that does not exist, or remove it.
  - **After the `spec-writer`, the `dev-writer` goes next if the behaviour
    changed; otherwise the `tester` does.** Either brief names the
    `spec-writer`'s commit and the markers it decided.
  - **`tester.md` says what a decided marker becomes** — the owner authorised
    this edit for the piece in session, beyond the four issues' text, because
    `tester.md`'s "Keep the markers and report each one" contradicts the new
    routing. A marker the brief names as decided is closed: reworded to cite
    the scenario now covering it, or removed. Any other marker stays open, and
    "must not remove" covers only a test carrying an open one. `RUNNER.md`
    points to `tester.md` rather than restating this.
  - With no marker and no reported decision, the `tester` is next, as before.
- **#171: every change made after the review round is reviewed before the
  `closer` runs.** `RUNNER.md` requires review of every commit added to the
  piece after the review round, before the `closer` is dispatched. That covers:
  - a writer's pass answering findings;
  - a rewrite or new file made on an owner instruction;
  - a spec callback;
  - a fix for a red CI run.

  "The `closer`, and what comes back" no longer tells the runner to re-dispatch
  the `closer` straight after a fixer: a red-CI fix goes through re-review
  first. The runner decides how big the re-review is: which lanes, how many
  reviewers, and which model each runs on, using the Agent tool's `model`
  override. It records that call, and any decision to skip a re-review, in its
  report and in the piece's `tasks.md`. When unsure, it re-reviews.
  Specifically:
  - **What needs review is every commit that changes something which merges.**
    A commit that only records tracking needs none: a box flipped, a finding's
    outcome written into `findings/`, a re-reviewer's findings or verdict box
    in `findings/` (which the `closer` deletes before the merge), a stage-row
    tick, or the runner's own record line under the re-review row. A commit that moves reasoning into
    `design.md` does need review.
  - **`RUNNER.md` names no model**, only the Agent tool's `model` override.
  - **A re-review brief carries four things:** the round and the commit range
    to read (for `spec-test-reviewer`, only the spec and test files in it);
    that the reviewer's stage row is already ticked and stays ticked; that new
    findings are appended as boxes to that reviewer's existing findings file,
    **under a heading naming the round**, in exactly this form:

    ```markdown
    ## Re-review round 3 `a1b2c3d..e4f5a6b`
    ```

    and that **a re-review which finds nothing appends one ticked verdict box
    instead**, naming the same round and saying it found nothing, in exactly
    this form:

    ```markdown
    - [x] **re-review round 3 `a1b2c3d..e4f5a6b`: no findings** — read <what>; clean
    ```

    ``round <n> `<range>` `` is copied from the runner's own line for the round
    under the re-review row (below). Either way, the reviewer's file names the
    round once the round is done. The heading and the box carry it exactly as
    the brief gives it, since the check before the tick is a fixed-string
    search for those two forms. The brief gives both forms whole, not "such
    as": a heading written loosely, such as
    ``## Re-review round 1 (`c222c37..9dc235c`)`` on this piece, matches
    neither. `RUNNER.md` gives that example with placeholders,
    ``## Re-review round <n> (`<range>`)``, so that no role file carries
    this piece's SHAs; it fails to match for the same reason, the
    parenthesis.
    The reviewer writes and commits that box itself, as it would a finding.
    It is the reviewer's own record that the round ran: a re-reviewer's stage
    row is already ticked, so without the box a clean round leaves nothing
    but the runner's line under the re-review row. The box and the heading
    are what the runner checks before it ticks the row (the check under the
    re-review row below); no other gate reads them. The box is ticked, so the `closer`'s
    `grep -rn "^- \[ \]"` passes it, and it is a box, so `grep -rc "^- \["`
    counts it. Clean *areas* stay in prose, as the reviewer role files say;
    the verdict box is one line for the round, not a box per area.
    `RUNNER.md`'s brief bullet gives the runner that reason in two sentences,
    so a brief it writes does not drop the tick or the box.
  - **If the `closer` has already deleted `findings/`**, which it does before
    archiving, the file is written afresh at the same name in the change's
    folder as it now stands, the archived one, holding the re-reviewer's
    findings or its verdict box. Every re-reviewer writes it, clean or not.
    A fresh file with no box would fail the `closer`'s "every file non-zero"
    check; a verdict box is what keeps a clean one from doing so.
- **#171: the stage-block template gets a place to record a re-review round.**
  The template lives in `spec-writer.md`, which `README.md` names as its only
  copy. It gains one row, placed after the review rows and before the
  `closer`'s three:

  ```markdown
  - [ ] re-review: every commit after the review round — runner
  ```

  - **The row is the runner's.** It records the runner's sizing decision, not
    an agent's work. `RUNNER.md` lists it among the runner's own tasks.
  - **The runner writes one indented line under it per round:** it starts
    ``round <n> `<range>` ``, numbered from 1 in the order the lines are
    written, and goes on to what landed, the lanes and the model each ran on,
    and why that size. A round the runner skips also gets a line, with the
    reason. These lines are not rows: they carry no box.
  - **A lane run again over a range it already had gets a line of its
    own**: the next number, the same range, the lanes it re-runs, and why (a
    run the runner did not accept, or an agent replaced after a stall). That
    holds however the lane is run again: a fresh dispatch, or the same agent
    continued with `SendMessage`, whose message then gives the new line's
    two forms whole, as a brief does. Continuing an agent gets no line only
    when it adds no review to a record already committed: finishing a round
    it has not yet recorded, such as after a stall, committing, or rebasing.
    Its record, once committed, is the round's own. The number is what
    tells two runs of one lane over one range apart, which the range
    cannot: on this piece's round 1, the runner did not accept the Sonnet
    security run and re-ran the lane on Opus, and a search for the range
    alone is satisfied by the Sonnet run's verdict box
    (`findings/security.md`, re-review `9dc235c..34fd428`, first box). A
    continued run needs the number as much as a fresh one: the rejected
    run's record is already committed under the old number, so the check
    would pass on it before the continuation had done anything
    (`findings/security.md`, re-review round 3 `34fd428..dc1390a`, box).
  - **The row is never struck.** A round with nothing to review gets its line
    and then a tick.
  - **The runner ticks it when no commit that merges is unreviewed**, and
    **unticks it when any commit that needs review lands after the tick**: a
    red-CI fix, a writer's conflict resolution, an archive commit that
    changed `openspec/specs/`, or any other commit step 3 lists. This is the
    only row in the stage block that ever goes from `[x]` back to `[ ]`.
  - **Before it ticks the row, the runner checks that every lane of every
    round the tick closes left its own record.** A round the tick closes is
    one recorded since the row was last ticked and not marked skipped. For
    each, run on the runner's HEAD once the round's commits are on it, with
    ``round <n> `<range>` `` copied from that round's own line under the
    re-review row, not typed:

    ```
    git grep -l -F -e '## Re-review round <n> `<range>`' -e '**re-review round <n> `<range>`: no findings**' -- <change folder>/findings/
    ```

    It must list the findings file of every lane the round ran (the
    file names in `RUNNER.md`'s "How many at once"), except a lane that a
    later line ran again over the same range: that line's own check
    covers it. The command prints file names, not findings, so it stays
    within what the runner reads. A lane whose file is not listed has not
    finished the round, however finished its agent looks or whatever its
    hand-back said: the runner continues that reviewer, or dispatches a
    fresh one for the lane (which gets its own line, above), and does not
    tick. The row is the one stage row not ticked by the agent that did the
    work, so this check is what makes its tick rest on the re-reviewers' own
    records rather than on the runner's reading of whether an agent has
    finished; a stalled agent looks exactly like a finished one. Specifically:
    - **It searches the heading and the verdict box, not the bare range.**
      Reviewers write ranges into prose routinely, including earlier rounds'
      ranges: `git grep -n -F "c222c37..9dc235c"` over this piece's
      `findings/` returns it on lines that are neither a heading nor a
      verdict box in five of the six files (at `6f17bebf`). One tick can close several
      rounds, so with a bare-range search a later round's reviewer citing an
      earlier range would count as the earlier round's record, although that
      round's own reviewer wrote nothing (`findings/security.md`, re-review
      `9dc235c..34fd428`, second box).
    - **The round number is part of both forms.** It separates two runs of
      one lane over one range (the line rule above), and it keeps
      consecutive rounds apart, which share an endpoint: round N's range
      ends at the SHA round N+1's starts at.
    - **The number and range are copied from the round's line, as the
      brief's are.** The brief's two forms are copied from that line, so the
      check then searches exactly what the reviewer was told to write. A
      number typed from memory is one way the check fails open: a check for a
      lane run again, typed with the earlier line's number, matches the
      earlier run's record over the same range whenever that run left one
      for the lane, and the runner ticks before the re-run has written
      anything. That is the record the line rule's number exists to set
      aside. Measured at `c3d697e9` over `34fd428..dc1390a`: round 3's forms
      list five files, every lane's but `readability.md`, and the forms
      copied from round 4's line list `readability.md` alone. The round 3
      readability run wrote nothing, so this piece's round 4 would have
      failed closed under round 3's number; a lane whose earlier run did
      write, as round 1's Sonnet security run did, would not.
    - **Single quotes, not double.** Both patterns contain backticks, which
      a shell expands inside double quotes.
    - **What it still cannot see:** a later reviewer who quotes an earlier
      round's heading or verdict box whole, in prose, satisfies that round's
      check. The forms are chosen to make that unlikely, not impossible.
      And the check searches whatever forms it is given: copied from the
      wrong line, such as the neighbouring line over the same range or the
      previous round's line, it passes on that line's records. At
      `c3d697e9`, round 5's forms list `spec-test.md` and `design-review.md`
      among others, the two lanes round 6 ran, so a round 6 check run with
      round 5's start would pass whatever round 6 had written. **Nor can it
      see a wrong number on the line itself**, which is a separate residual
      from copying from the wrong line. A re-run's line written by taking the
      previous line as a template, with its number left unchanged, repeats
      that line's number; the brief, the reviewer's heading and the check are
      all copied from it, so they agree, and the check passes on the earlier
      run's record over the same range, whenever that run left one for the
      lane, before the re-run has written anything. The copy rule moves the
      number's source from the runner's memory to the line; it does not check
      the line. Measured at `9e6dde2f`, and again at `291499c7`: the forms
      from a second line starting ``round 3 `34fd428..dc1390a` `` list
      `architecture.md`, `correctness.md`, `design-review.md`, `security.md`
      and `spec-test.md`, every one from the first run (`findings/security.md`, re-review round 8
      `6d43cda..d1d2165`, box). "Numbered from 1 in the order the lines are
      written" already fixes each line's number, the n-th line carrying n,
      so a repeated or stale number breaks a stated rule, as skipping the
      check does. No test of the role files can see either case, because the
      command there carries `<n>` and `<range>` as placeholders; only a second
      reader of the round lines can ("An independent check of the re-review
      row by the `closer`", in "Out of scope"), and the wrong number only by
      checking that the numbers under the row are unique and consecutive,
      since forms derived from a line with a repeated number are satisfied by
      the earlier run's record.
  - **This piece's rounds 1 and 2 predate the round number.** Their briefs
    gave ``## Re-review `<range>` `` and
    ``**re-review `<range>`: no findings**``, so the check for those two
    rounds searches those forms, not numbered forms copied from their lines;
    each of their lines names its own check. Measured at `6f17bebf`: both
    list all six findings files for each round. Round 1 is the re-dispatch
    case the number exists for, and it is settled without it: the Sonnet correctness
    and readability runs wrote nothing, and `security.md` holds the Opus
    run's own heading, ``## Re-review `c222c37..9dc235c` (Opus)``, committed
    in `59619032`. From round 3 on, briefs use the numbered forms.
  - **The runner commits the record lines, the tick and any untick itself**, in
    its own tree on `piece/<name>`, before the next dispatch forks from it.
    They are the only content the runner commits: see "What the runner
    commits" below.
  - **The `closer` is dispatched only when every row above its own three is
    ticked or struck**, this row included. The `closer`'s existing Step 1
    checks that every row but its own is ticked or struck, and an unticked
    box ends its turn ("Your report"), so that check is what lets it see an
    unticked re-review row.
  - **After the `closer` has archived the change, the stage block is in
    `openspec/changes/archive/<date>-<name>/tasks.md`.** The runner's untick
    goes there, and a re-dispatched `closer`'s Step 1 reads the block there.
    What decides this is that an earlier `closer` archived, not what it came
    back with. Several returns follow an archive: a red run; an archive
    commit that changed `openspec/specs/`; a conflict met merging `main`
    after the archive, when Step 4 found the branch `BEHIND`; and a push
    refused after the archive commit. Where `closer.md` or `RUNNER.md` says
    when the block has moved, it names that condition, with any returns
    given as examples, not a list that reads as complete. In detail:
    - **The runner reads the block there, and `findings/` beside it**, for as
      long as the PR is open. `RUNNER.md`'s "What you read" table and
      "Rebuild the state" say so, since both otherwise name only
      `openspec/changes/<name>/`, and a piece archived but not merged no
      longer appears in `openspec list`.
    - **The `closer`'s Step 1 finds the folder by exact path**, not by
      assuming it:
      `git ls-files -- "openspec/changes/<name>/tasks.md" "openspec/changes/archive/????-??-??-<name>/tasks.md"`.
      Exactly one path back: it runs both gates in that folder. More than one,
      or none, it stops and reports what came back: it cannot tell which block
      is the piece's. More than one most likely means something recreated the
      pre-archive folder after the archive, such as an untick or findings file
      written at the old path. None means the name is wrong.
    - **An archived folder with no `findings/` passes the findings gate.** An
      earlier `closer` deleted it, and no re-reviewer has run since: every
      re-reviewer after the archive writes a file, clean or not (above), so
      its absence means the runner ran no round there, which its line under
      the re-review row records. A `grep` error for the missing directory is
      not a failed gate.
    - **A re-dispatched `closer` does not archive again.** `closer.md` Step 3
      says so (owner-authorised in session): `openspec archive` is not rerun,
      but Step 3's deletion, commit and push still apply. A re-review's
      `findings/` in the archived folder is deleted and committed in Step 3,
      and HEAD is pushed either way, as the entry on the `closer`'s own
      commits below sets out, so the run it watches includes everything the
      piece now carries.
- **#170: forbid `--admin` in `closer.md`.** The file names `gh pr merge
  --admin`, and any change to branch protection, as things the `closer` never
  does, including when the owner has granted merge-on-green. Branch protection
  here covers rulesets too: no write to `branches/main/protection` and none to
  `rulesets`. Step 6 carries the rule, and "What you never do" points to it
  rather than restating it. If the PR stays
  `BLOCKED` with every required check green, the `closer` stops and reports the
  output of
  `gh pr view <n> --json mergeStateStatus,mergeable,statusCheckRollup,reviewDecision`.
  It does not look for another way to merge.
- **#133: restore one paragraph in `.claude/agents/README.md`.** Only the
  `settings.json` ownership paragraph under "`baseRef: "head"` is required"
  goes back to its pre-#119 wording, quoted in the issue:

  > It is the **user's** file. Do not edit it on your own initiative;
  > machine-local settings belong in `settings.local.json`, which stays ignored.

  The issue's own fix, `git checkout 51ab7f8^ -- .claude/agents/README.md`, is
  **not** run. The file has changed a lot since then, including PR #106's move
  from `docs/PLAN.md` to GitHub Issues, and that command would revert those
  changes too. The `dev-writer.md` hunk the issue names as owner-approved is
  left alone.
- **The `closer`'s own commits go through the re-review step (owner-authorised,
  from `findings/security.md`'s first finding).** Step 3 of `RUNNER.md`'s
  sequence covers every commit made after the review round, the `closer`'s
  included. The `closer` still never waits for a review: a commit of its own
  that needs one ends its turn, and it comes back to the runner the way a red
  run does. Specifically:
  - **The `closer` brings `main` in by merging it, not by rebasing onto it**
    (the owner chose this over a rebase). Step 2's fix for a stale branch
    becomes `git merge origin/main` in the `closer`'s own tree, pushed to the
    piece ref by refspec with no force. The rebase and its
    `--force-with-lease` push are removed, and "What you never do" forbids
    force-pushing for any reason. The reason is the conflict rule below
    ("The `closer` does not resolve a conflict"): a writer's conflict resolution has to be one commit that the runner
    can bring onto its HEAD without rewriting it, and that a reviewer can read
    on its own. A later rebase would drop that merge commit and raise the same
    conflict again.
  - **A refused push stops the `closer`.** If any push of its HEAD to the
    piece ref is refused, in Step 2 or Step 3, the remote piece ref holds a
    commit its branch does not. The `closer` stops and reports. It does not
    force, and it does not fetch and merge the remote piece ref either: what
    the remote holds that the runner's HEAD lacks is not known to be
    reviewed. `closer.md` states this for both pushes. Its "Your report"
    asks for the refusal git printed, and its closing list of what ends a
    turn names a refused push, since a refused push is a return like the
    others.
  - **`BEHIND` in Step 4 means merge `main` now, by Step 2's route**,
    conflict rule included, and then Step 4 against the new run. It replaces
    Step 4's "stop and report", which contradicted Step 2's own instruction to
    merge the moment `BEHIND` shows and its account of arriving there from
    Step 4. A stale branch with no conflict does not come back to the runner
    (below), from Step 2 or from Step 4.
  - **The `closer` deletes `findings/` in Step 3, after Step 2, not in
    Step 1.** Step 1 runs both gates and confirms the durable reasoning is in
    `design.md`; the deletion itself happens at the start of Step 3,
    immediately before `openspec archive`, and is committed with the archive.
    On a re-dispatch, a re-review's `findings/` in the archived folder is
    deleted at the same point and committed in Step 3. The reason is Step 2's
    merge: a deletion made in Step 1 is still uncommitted when it runs.
    Staged, it makes `git merge` refuse (`Your local changes to the following
    files would be overwritten by merge`) although `main` never touches those
    files, measured by the `dev-writer` in a scratch repository. Unstaged, the
    merge runs and `--abort` keeps it, but that relies on git carrying
    uncommitted changes across a merge and its abort. Moving the deletion
    after the merge means nothing is uncommitted while Step 2 runs, in either
    form. This edit is inside the owner-authorised Step 2 rewrite: without it,
    Step 2's merge can fail on the first dispatch whenever the branch is
    stale, since `closer.md` as it stands has Step 1 delete `findings/` on
    every first dispatch. `closer.md` Step 1 keeps one sentence of that
    reason (a staged deletion makes `git merge` refuse), so a later edit does
    not move the deletion back; the rest is in `design.md`.
  - **A merge of `main` that stops on no conflict needs no review.** It changes
    none of the piece's own lines, and it adds nothing to the squash: what it
    brings in is already on `main`.
  - **The `closer` does not resolve a conflict.** When the merge stops on one,
    it lists the conflicting paths with `git diff --name-only --diff-filter=U`,
    runs `git merge --abort`, reports the paths and returns. This replaces
    Step 2's "A conflict is yours to resolve". The runner routes the conflict
    as it routes a red run, by what the conflicting paths are (`RUNNER.md`'s
    table: implementation to the `dev-writer`, a test to the `tester`, the
    contract to the `spec-writer` and then the fixer). The writer's brief says
    to merge `origin/main` into its own branch and resolve the conflict in
    that merge commit, not to rebase. The resolution is a commit made after
    the review round, so it goes through step 3 before the `closer` is
    re-dispatched. The re-review brief names the merge commit, and says to
    read it with `git show --remerge-diff <sha>`, which shows only what the
    resolution changed from git's own merge.
  - **An archive commit that changes the live contract is reviewed** (the
    owner chose to stop when specs change). **Straight after Step 3's
    archive commit, before its push**, the `closer` runs
    `git diff --name-only HEAD^ HEAD -- openspec/specs/`. The check reads
    the local commit and the push does not move HEAD, so running it first
    measures the same thing; run after the push, it would never run when the
    push is refused, since a refused push ends the turn, and a re-dispatched
    `closer` skips it (below), so a spec-changing archive would reach `main`
    with no round ever sized for it. Then the `closer` pushes:
    - **Push refused:** it stops and reports the refusal **and the check's
      result**, the files it listed or that it listed none. `closer.md`'s
      "Your report" asks for both. `closer.md` handles a refused push in
      Step 3 once, for both kinds of run, so it asks for the check's result
      only if this run made the archive commit: a re-dispatched `closer` runs
      no check (below) and reports the refusal alone.
    - **Push accepted, and the check listed any file:** it stops before
      Step 4, reports the archive commit and the files, and returns.
    - **Push accepted, and the check listed nothing:** it carries on.

    A change with `skip_specs: true` always lists nothing, since archiving it
    only moves the change folder and commits the `findings/` deletion. For an
    archive that listed files, the runner sizes a round for the archive
    commit like any other, whichever way the `closer` came back with it, then
    re-dispatches the `closer`. That `closer`
    finds the change archived and does not archive again, by the Step 1 and
    Step 3 rules for a re-dispatch above. **The check applies only to an
    archive commit the `closer` made in the same run.** A re-dispatched
    `closer` makes none, so it skips the check: its HEAD is whatever the runner
    brought onto the piece last, which step 3 has already reviewed. Step 3's
    re-dispatch paragraph in `closer.md` names the check it skips as the one
    below it, since the check no longer sits at the end of the step; the skip
    itself is unchanged.
  - **A re-dispatched `closer` pushes its HEAD at Step 3 whether or not it
    deleted anything.** Its tree carries every commit the runner brought onto
    the piece since the last push: a fix, a conflict resolution, the runner's
    record lines and tick. The run it watches in Step 4 must include them.
    This replaces the reading of Step 3's re-dispatch paragraph under which
    the push happens only when a `findings/` was deleted.
  - **The runner brings the `closer`'s commits onto its HEAD by fast-forward.**
    When a `closer` returns without having merged, whatever else it reports,
    the runner first runs
    `git merge --ff-only <the closer's branch>` in its own tree. Only then
    does it write to the stage block or dispatch the next agent. Without this,
    the archived `tasks.md` the runner unticks in is not in its tree, and the
    next agent forks from a HEAD without the archive commit or the merge of
    `main`. A conflict resolver's branch is brought on the same way, since
    `git cherry-pick` cannot carry a merge commit's second parent.
    **So is any agent's branch whose commits are already on the remote piece
    ref**, which in the current flow is every `dev-writer` pass, not only
    the first: `dev-writer.md` has it push its tip to
    `refs/heads/piece/<name>` before handing back on the first pass, and
    "On the findings pass it always does", which covers a writer answering
    findings, a red-CI fixer and a pass after a `spec-writer` callback. So
    `RUNNER.md` names the case by its condition, and gives the
    `dev-writer` as the instance, not its first pass. A
    cherry-pick that is not a fast-forward copies those commits to new
    SHAs, so the runner's `piece/<name>` no longer descends from the remote
    ref, and every later push of a HEAD forked from it, the `closer`'s
    included, is refused, since no agent may force. Measured on this piece's
    reflog: the `dev-writer`'s pushed `c67aa24`..`d41d0fd` were
    cherry-picked into new commits at 23:32 on 2026-09-25, and the runner
    then ran `reset` to `d41d0fd`, which `RUNNER.md` now forbids.
    `git merge --ff-only` either keeps the SHAs or refuses; it never
    diverges silently, which `git cherry-pick --ff` does when HEAD is not
    the commit's parent. `RUNNER.md`'s "One piece is one PR" bullet on
    bringing the `dev-writer`'s pushed commits onto the runner's HEAD says
    so too, pointing to "Dispatching": it said "you cherry-pick", which
    this rule contradicts. `RUNNER.md`'s other sentences that give the
    cherry-pick as the route for an agent's commits in general say
    "brought onto the piece" or "bringing onto the piece" instead, which
    covers both routes: the sample `dev-writer` findings brief in
    "Dispatching", and "What you must still ask for is the branch name".
    So does the sentence closing "No `worktree-agent-<id>` ever appears on
    the remote", which is about every agent branch: it says the runner
    brings each one onto its HEAD as "Dispatching" says, not that it
    fast-forwards to each, since most agents' branches are still
    cherry-picked. If a
    fast-forward refuses, the runner stops and reports, and takes neither of
    the hints git prints with the refusal: `git merge --no-ff` would make a
    merge commit that is the runner's own content, and `git rebase` rewrites
    `piece/<name>`. It never rebases, resets or force-pushes `piece/<name>`,
    and it does not merge `main` itself.
  - **`RUNNER.md` step 3 names what the `closer` adds.** Its list of what
    needs review gains a writer's conflict resolution and an archive commit
    that changes `openspec/specs/`. Its list of what needs none gains a merge
    of `main` with no conflict, the `closer`'s deletion of `findings/`, and an
    archive commit that changes nothing under `openspec/specs/`.
  - **Step 4, "The `closer`, and what comes back", gives the `closer`'s
    returns as examples, with no count.** `closer.md` has more stops than
    this piece routes: a findings file with zero boxes, more than one change
    folder or none, a PR body that disagrees with the diff, isolation that
    did not take. A count reads as the complete list and is checkably false.
    The section routes six returns, and says a return it does not name is
    reported to the runner with its evidence, which the runner routes by
    what it is:
    - **A red run**, **a conflict** and **an archive commit that changed
      `openspec/specs/`** go back through step 3.
    - **An unticked box** is either a finding never answered, which routes
      to whoever the finding names, or a stage row. Another agent's row goes
      back to that agent, continued to tick it, or to a fresh agent for the
      stage. The re-review row means a round is owed: step 3.
    - **A refused push goes to the owner:** the remote piece ref holds a
      commit the runner's HEAD lacks, which the fast-forward rule above
      exists to prevent, and the runner cannot reconcile the two without a
      reset, a force-push or a merge of its own, all of which it never does.
      It reports the output of
      `git log --oneline --left-right --cherry-mark HEAD...origin/piece/<name>`
      and waits. **If the `closer`'s report also lists files from the
      archive check**, the runner first unticks the re-review row and records
      a round for the archive commit, as for a spec-changing archive, so the
      round is owed whatever the owner does about the push.
    - **A PR that stays `BLOCKED` with every required check green goes to
      the owner** (#170), with the `gh pr view` output the `closer` reported.
      The runner does not look for another route to the merge either, and
      does not diagnose the block (see "Out of scope").

    "A stale branch does not come back" now holds only for a merge of `main`
    with no conflict.
  - **`README.md`'s branch section says how work reaches `piece/<name>`.** Its
    table's `piece/<name>` row says the runner cherry-picks every agent's
    commits, and the section says "Cherry-pick rather than merge". Both gain
    the fast-forward cases above, by pointing to `RUNNER.md` rather than
    restating them, and neither enumerates them in a way that reads as
    complete. The same section's two other statements of the old route,
    the `worktree-agent-<id>` row's "cherry-picked onto the piece" and "every
    agent's commits are cherry-picked onto it", say "brought onto" instead,
    so the section does not contradict its own pointer. So does its
    sentence on the harness-named branch, "the runner picks from that",
    which becomes "the runner brings its commits on from that".
  - **`README.md`'s other statements of the old route change the same way.**
    Four passages outside the branch section say an agent's commits are
    cherry-picked onto the piece, which is false for every `dev-writer`
    pass under the fast-forward rule above:
    - "One writer at a time", on a second writer launched "before the
      first's commits are cherry-picked";
    - the sample `dev-writer` findings brief in "Handing over between
      agents", "so the work can be cherry-picked onto the piece", which
      `RUNNER.md`'s copy of the same brief no longer says;
    - "What the agent's own branch means for getting work back", "commits
      still need a cherry-pick onto `piece/<name>`";
    - "Who removes the agent's tree", "after cherry-picking the work off".

    This is the same case as the `dev-writer.md` clause under "What the
    runner commits" below: left as they are, they contradict `RUNNER.md` in
    this piece, and correcting them adds no rule. Each changes only the words
    that name the route, to "brought onto" or "bringing onto" the piece.
    Sentences that are true of a cherry-pick stay: the reviewer's tree
    removed "once its work is cherry-picked" (reviewers are cherry-picked),
    the orphan-branch check's "a commit cherry-picked rather than merged",
    and the stage-block section's "concurrent cherry-picks never touch the
    same line", which is literally true and is left to the follow-up in
    "Out of scope".
  - **`dev-writer.md`'s own statements of the route change the same way.**
    The criterion is the one above, and it holds in every file this piece
    edits: a sentence saying the `dev-writer`'s commits, or every agent's,
    reach the piece by cherry-pick is false under the fast-forward rule and
    contradicts `RUNNER.md`, so its route words change; a sentence true of a
    cherry-pick whatever the route, or about an agent that is still
    cherry-picked, stays. `dev-writer.md` is the `dev-writer`'s own file
    describing its own passes, so it meets that test more directly than any
    `README.md` passage. Four sentences change, each only in the words that
    name the route, to "brings … onto" or "bringing … onto" the piece:
    - "Where your commits go": "The runner cherry-picks it onto
      `piece/<name>` once you hand back";
    - "The PR is yours": "you do not need the runner's cherry-pick to get
      there";
    - the same section, after the push sequence: "The runner cherry-picks
      your commits onto its own local `piece/<name>` afterwards";
    - the same section, on not checking out `piece/<name>`: "A local
      cherry-pick is the runner's."

    "Where your commits go"'s "is not something the runner can
    cherry-pick", about commits made from the piece branch when isolation
    did not take, stays: it says what the runner cannot do with commits on
    the wrong branch, which holds of either route. An earlier version of
    this proposal kept all four on the ground that bringing the passes on is
    the runner's step and changes nothing the `dev-writer` does; that is
    equally true of the `README.md` passages, so it did not separate the two
    cases (`findings/spec-test.md`, re-review `9dc235c..34fd428`, first
    box).
- **What the runner commits (owner-authorised, from
  `findings/architecture.md`'s third finding; ruled by the owner: "A runner
  always delegates" and "Agents tick their own").** `RUNNER.md`'s "What a
  runner does" states it once, and step 3, the per-agent sequence in
  "Dispatching" and the red-run text point there:
  - **The runner's own content is its re-review row and nothing else:** the
    record lines, the tick and the untick. That is the only stage-block row
    it touches.
  - **Bringing an agent's commits onto `piece/<name>` adds no content of the
    runner's.** A clean cherry-pick, or the fast-forward above for the
    `closer`, a conflict resolver and an agent whose commits are already on
    the remote piece ref, carries commits an agent made. That stays the
    runner's job.
  - **Every agent ticks its own stage row.** The runner does not tick a row
    for another agent, including when the agent's hand-back reports its stage
    done and the row is still unticked. It keeps that agent's tree and
    continues the agent with `SendMessage` to tick the row and commit it.
    `README.md`'s "Each agent flips its own row" stands unchanged.
  - **Everything else is work, and the runner delegates all of it, with no
    exception.** Work is a proposal, a spec, `design.md`, code, tests,
    role-file text, a finding's outcome, and another agent's tick. The last
    two need no review under step 3, but needing no review does not make a
    commit the runner's: who commits and whether a commit is reviewed are
    separate questions. An edit the
    owner asks for in the session is an instruction to dispatch the agent
    whose file it is, not to make the edit, however small it is. When a
    dispatched agent is refused an edit it was briefed to make, as the
    `dev-writer` was before `d805fe4`, the runner reports the refusal to the
    owner and does not make the edit itself.
  - **Copying an agent's uncommitted output onto the piece is work, not
    tracking.** It commits content the runner did not write, under the
    runner's commit. When an agent reports that it wrote its output but could
    not commit it, the runner keeps that agent's tree and continues the agent
    with `SendMessage` to commit it, once whatever stopped the commit is
    cleared. If only the owner can clear it, such as a permission prompt or a
    credential, the runner reports that to the owner and waits. If the agent
    cannot be continued, the runner dispatches a fresh agent for the stage,
    which redoes it rather than copying the old agent's file. The signing
    failure behind `c43c7a7` does not arise while signing is off: the owner
    has switched the requirement off for the days they are away, and briefs
    tell agents to commit with `--no-gpg-sign`. Once signing is back, a
    signing failure is again a blocker only the owner can clear, handled as
    above: the runner reports it and waits. See "Out of scope" for why no
    role file changes for this.
  - **A cherry-pick that stops on a conflict is not the runner's to
    resolve.** The runner runs `git cherry-pick --abort`, keeps the agent's
    tree, and continues that agent with `SendMessage`: it rebases its own
    branch onto `piece/<name>`, resolves the conflict there, and reports
    back. The runner then cherry-picks again. The agent's branch is local and
    never pushed, so the rebase rewrites nothing anyone else holds. If the
    agent cannot be continued, the runner dispatches a fresh agent for the
    stage.
  - **An agent whose tree holds uncommitted changes is told how to rebase
    without losing them.** A mutating reviewer's tree does: `code-reviewer.md`
    and `spec-test-reviewer.md` tell it to leave its mutations uncommitted,
    as evidence only the runner may decide to discard, and `git rebase`
    refuses a dirty tree (`error: cannot rebase: You have unstaged changes.`,
    measured by the correctness re-reviewer). The runner's continuation
    message carries the steps, as a re-review brief carries what differs
    from a first-round review, and `RUNNER.md` states them by that condition,
    not by role:
    1. save the uncommitted changes as a patch file under the tree's own
       `tmp/`, which is gitignored, written by a git command rather than a
       shell redirect;
    2. restore the tree to its last commit, index included;
    3. rebase onto `piece/<name>` and resolve the conflict;
    4. re-apply the patch, and report whether it applied.

    The mutated state survives, in the tree and in the patch file. Not
    chosen: committing the mutations, which would put them on the piece;
    discarding them, which destroys the evidence the reviewer's findings
    cite; and `git rebase --autostash`, which got past the refusal in the
    correctness re-reviewer's scratch run but whose restore is unmeasured,
    and which stores its stash in the stash list, shared with every worktree
    (the reason `CLAUDE.md` bans a bare `git stash`), when it cannot
    re-apply it. The `dev-writer` measures the sequence in a scratch
    repository before writing its commands into `RUNNER.md`, as it has every
    other command in this piece. The commands it measured end to end are
    these: for step 1, `mkdir -p tmp` and then
    `git diff --binary --output=tmp/uncommitted.patch HEAD`, since `git diff`
    does not create the directory; for step 2,
    `git restore --source=HEAD --staged --worktree -- .`; and for step 4, a
    plain `git apply tmp/uncommitted.patch`. The changes come back unstaged
    whether or not they were staged before, a staged new file coming back
    untracked, and the patch file stays in `tmp/` either way. **An untracked
    file is outside the four steps, and `RUNNER.md` says so:** `git diff`
    does not save it, step 2 leaves it in place, and it stays in the tree
    throughout. It does not stop the rebase unless an incoming commit adds a
    file at its path; then `git rebase` refuses before it starts
    (`error: The following untracked working tree files would be
    overwritten by checkout`), with no rebase in progress and the file
    untouched. **A tree whose only changes are untracked saves an empty
    patch**, and step 4's `git apply` refuses it
    (`error: No valid patches in input`, exit 128). `RUNNER.md` says that
    refusal means there was nothing tracked to re-apply, not that evidence
    was lost, and has the continuation message carry that sentence, so the
    agent reports it as such. The commands stay as above for this case:
    step 4 stays a plain `git apply`, with no `--allow-empty` and no check
    added before step 1. Measured by the `spec-writer` with git 2.55.0 in a
    scratch repository: an untracked file at a path the piece's commit
    added refused the rebase as quoted; one at a path nothing incoming
    touched survived the save, restore, rebase and apply; the save wrote an
    empty file and the apply refused it as quoted; and a staged new file
    came back from the apply untracked. `RUNNER.md` names no signing flag: while
    signing is off the runner's message adds `--no-gpg-sign` to the rebase,
    as every brief does, since the correctness re-reviewer's
    `git rebase --continue` hung on the signing prompt. The durable home for
    these steps is the reviewer role files: see "Out of scope".
  - **The review round meets that conflict every time, and `RUNNER.md` says
    so** where it sets out the per-agent sequence, so the runner expects it
    rather than reading it as a fault. The six reviewers fork from one HEAD,
    each ticks its own row, and the six review rows are adjacent. Measured on
    this pass with git 2.55.0, in a scratch repository holding the template's
    stage block: two such ticks cherry-picked one after the other conflict
    when their rows are adjacent (`CONFLICT (content): Merge conflict in
    tasks.md`), and merge cleanly with one unchanged row between them. Picked
    in template order, each of the second to sixth review cherry-picks stops.
    No order stops fewer than three: a pick is clean only if neither
    neighbouring row is ticked yet, and at most three of six consecutive rows
    can be picked that way. Sequential stages never meet it, since each forks
    after the previous tick is on the runner's HEAD. This piece does not
    change the template to avoid the conflict: see "Out of scope".
  - **`spec-writer.md` stops saying that ticks on neighbouring rows cannot
    conflict.** Its sentence "That is what keeps their cherry-picks clean —
    git conflicts on the same line, not on neighbouring ones" is false by
    the measurement above. It is also the premise behind the runner ticking
    reviewers' rows: `a284e51`'s message gives the conflict as the reason,
    and the owner's ruling "Agents tick their own" now forbids that practice.
    Left as it is, the file holding the template would say the review
    round's cherry-picks are clean while `RUNNER.md`, in this same piece,
    tells the runner they conflict. Only that sentence changes, and no row
    of the template. What replaces it says three things: an agent forked
    after the previous tick reached the runner's HEAD cherry-picks cleanly;
    ticks on adjacent rows by agents forked from the same HEAD, which is the
    review round, conflict when cherry-picked one after another, because git
    conflicts on neighbouring lines as well as on the same line; and
    `RUNNER.md` says who resolves that. This edit is inside the piece's
    authorisation because it follows from a ruling the owner took into this
    piece: it corrects the ruling's premise in the file that stated it, and
    adds no rule.
  - **`dev-writer.md` states the same premise, and its clause is corrected
    too.** Its "`tasks.md`, and where your work lands" says the `dev-writer`
    ticks exactly one stage row, its own, "never adding a row, so concurrent
    agents' cherry-picks do not conflict". That reason is false by the same
    measurement, and it contradicts `RUNNER.md` in this piece exactly as
    `spec-writer.md`'s sentence did, so the same reasoning puts it inside the
    authorisation. Only the clause after "never adding a row" changes: it
    becomes a pointer to `spec-writer.md`'s stage-block paragraph, which says
    when ticks conflict and points on to `RUNNER.md`, so the account stays in
    one place. The rule itself (tick exactly one row, your own, and add none)
    is unchanged. The only other `dev-writer.md` edits are the four route
    sentences under the fast-forward rule above.
  - **"You do not write the work — not even one small edit while an agent is
    being prepared" stays, and gains no exception.**
- **This piece's own history predates the ruling, and is not redone.** Four
  runner commits on this piece are of kinds the ruling now excludes:
  - `d805fe4`: `tester.md`, `closer.md`, `RUNNER.md` and `design.md` edits
    made on the owner's direct instruction, after the `dev-writer`'s attempt
    was refused;
  - `c43c7a7`: four findings files copied from reviewers' trees after a
    signing failure;
  - `a284e51` and `e7e2bbd`: stage-row ticks for six reviewers.

  They were made before the owner ruled, and they stand as they are: no
  history rewrite, no revert, no redo. `d805fe4` landed before the review
  round, so the six reviewers read its edits. The owner may raise these
  commits separately; this piece does not.

### Out of scope

- **One file per stage row, so that no two agents' ticks can conflict.** The
  owner suggested this in answer to the measured conflict above. It changes the
  stage block's design, which the owner cannot rule on while away, so this
  piece keeps the template and handles the conflict by the cherry-pick rule
  above. It is a follow-up for the project manager to file, and this entry is
  written to be lifted into that issue as it stands:
  - **The conflict.** The stage block is one list in one `tasks.md`. The six
    reviewers of a review round fork from the same runner HEAD, and each
    ticks its own row. The runner then cherry-picks their commits onto
    `piece/<name>` one after another, and each commit changes a file the
    earlier picks have already changed. Measured with git 2.55.0 in a scratch
    repository holding the template's stage block: a tick on a row next to
    one already picked stops with `CONFLICT (content): Merge conflict in
    tasks.md`, and a tick with one unchanged row between it and the picked
    one applies cleanly. Git conflicts on neighbouring changed lines, not only
    on the same line. The six review rows are adjacent, so in template order
    the second to sixth picks each stop, and no order stops fewer than three.
    Each stop costs a `SendMessage` round trip, one after another, for the
    agent to rebase onto the piece and resolve a one-character change.
    Sequential stages never meet it: each agent forks after the previous
    tick is on the runner's HEAD.
  - **Why separate files remove it by construction.** A cherry-pick merges
    path by path, and two commits that change different files cannot
    conflict in content, in any order. With one file per stage row, the way
    `findings/<dimension>.md` is one file per reviewer, each agent's tick
    changes a file no other agent touches. The six reviewers already write
    six findings files at once, with no conflict, for the same reason.
  - **Who reads the stage block, and would change:**
    - `spec-writer.md`: the template, and the paragraph above it on why
      ticks stay clean.
    - `README.md`'s stage-block section in "Two files carry the state of a
      change": the block's shape, "Each agent flips its own row and adds
      none, so concurrent cherry-picks never touch the same line", a struck
      row keeping its empty box, and `openspec archive` counting a struck row
      as incomplete.
    - `closer.md` Step 1: the stage-block gate, and the `git ls-files`
      command that finds `tasks.md` in the live or the archived change
      folder.
    - `RUNNER.md`'s "What you read" table and "Rebuild the state": the
      `tasks.md` rows and their greps, `grep -rn "^- \[ \]"` and
      `grep -c "^## Stages"`, and the paragraph on where the block is once
      the change is archived. Also `RUNNER.md` step 3, whose re-review row
      carries the runner's record lines, tick and untick.
    - Every role file that tells its agent to tick its own row:
      `dev-writer.md`'s "`tasks.md`, and where your work lands", `tester.md`,
      `code-reviewer.md`, `spec-test-reviewer.md` and `design-reviewer.md`.
  - **For the issue to settle:** how a struck row and its reason are written
    when a row is a file; whether `openspec archive`, which reads `tasks.md`,
    still warns about an incomplete stage; and what the check for "an
    unticked row with no agent running" becomes.
  - **Offered with it, and not chosen for this piece:** a blank line between
    the template's rows, which the measurement shows would stop the conflict
    but holds only while every later edit to the template keeps the spacing;
    and serialising the reviewers, which removes the conflict by giving up
    the parallel round.
- **`closer.md`'s signing text, and `--no-gpg-sign` in role files.** The
  owner has switched signing off for the days they are away, and said both
  are "temporary". So two passages in `closer.md` are false for that period
  and are **not edited**: Step 2's paragraph that says commit signing is
  required on `main`, that a signing failure is a stop-and-ask, and not to
  reach for `--no-gpg-sign` or `commit.gpgsign false`; and Step 6's "it
  requires four green checks and a signed commit". Both become true again
  when signing is restored, and editing them now would mean a second edit to
  undo it then. Do not "fix" either. For that period `--no-gpg-sign` is in
  the runner's briefs and in no role file. The Step 2 paragraph is carried
  into the rewritten Step 2 with its rule unchanged. The only change is the
  operation it names, from "a rebase" to the merge of `main`, because
  Step 2 no longer rebases.
- **A clean first-round review with no box (#182).** `code-reviewer.md` and
  `spec-test-reviewer.md` tell a reviewer to report clean areas in prose, not
  as boxes, and `closer.md`'s Step 1 sends back any findings file with zero
  boxes. So a first-round review that finds nothing stops the `closer`. This
  piece does not fix that: it needs the reviewer role files, which are outside
  its authorisation, and #182 carries it. The re-review verdict box above
  lives in the runner's brief rather than a role file, and takes the shape
  #182 recommends (one ticked verdict box, clean areas still in prose), so the
  two agree when #182 lands. On this piece, `findings/design-review.md` is
  such a file; it gets its box from the `design-reviewer` in this piece's
  re-review round, not from a role-file change.
- **Why #165 was `BLOCKED`** (#170's closing paragraph). This piece only stops
  a closer from going around a block. It does not diagnose one.
- **A standing test for the git commands the role files name.** Several are
  deterministic, and each was measured once, when it was written (`tasks.md`'s
  Verify lines, `design.md`). Nothing re-runs them, so a later edit that
  mistypes one is caught only when a real piece meets it. They are, by file:
  - `closer.md` Step 1's
    `git ls-files -- "openspec/changes/<name>/tasks.md" "openspec/changes/archive/????-??-??-<name>/tasks.md"`,
    and Step 3's `git diff --name-only HEAD^ HEAD -- openspec/specs/`;
  - `RUNNER.md` step 1's `git diff --name-only -G "NO SPEC:" origin/main...HEAD`,
    and step 3's pre-tick
    ``git grep -l -F -e '## Re-review round <n> `<range>`' -e '**re-review round <n> `<range>`: no findings**' -- <change folder>/findings/``;
  - `RUNNER.md` "Dispatching", the steps for an agent whose tree holds
    uncommitted changes: `mkdir -p tmp`,
    `git diff --binary --output=tmp/uncommitted.patch HEAD`,
    `git restore --source=HEAD --staged --worktree -- .` and
    `git apply tmp/uncommitted.patch`;
  - `RUNNER.md`'s `git merge --ff-only <branch>` keeping a pushed SHA or
    refusing, step 3's `git show --remerge-diff <sha>` for a resolution, and
    step 4's `git log --oneline --left-right --cherry-mark HEAD...origin/piece/<name>`
    on a refused push.

  They do not fail alike, and that sets the order a test covers them in:
  - **Fail closed: the agent stops or reports.** A mistyped `ls-files`
    pathspec returns no path or several, and the `closer` stops. A pre-tick
    pattern with a wrong character that no committed record carries lists
    nothing, and the runner cannot tick. The save step with `--binary`
    dropped writes `Binary files a/<f> and b/<f> differ` for a binary
    change, and `git apply` then refuses it
    (`error: cannot apply binary patch to '<f>' without full index line`,
    exit 1), so the reviewer reports that the patch did not apply; step 2
    has already discarded that change from the tree, so the loss is
    reported, not prevented.
  - **Fail open: every command exits 0 and the flow carries on.** Three,
    and they are the ones a test covers first:
    - a mistyped `openspec/specs/` path lists nothing, and the `closer`
      carries on past an archive that changed the live contract, so an
      unreviewed spec change would merge;
    - a pre-tick search cut down to the bare range matches it wherever a
      reviewer wrote it in prose, and one cut down to a single SHA also
      matches the previous round's records, since consecutive rounds share
      an endpoint. The runner ticks with lanes that never ran. Measured at
      `34fd428`, before any round-2 lane had written:
      `git grep -l -F "9dc235c"` over `findings/` listed all six files,
      every one matched by round 1's records (`findings/spec-test.md`,
      re-review `9dc235c..34fd428`, third box);
    - the save step with `HEAD` dropped,
      `git diff --binary --output=tmp/uncommitted.patch`, writes only the
      unstaged changes. Step 2 then discards the staged ones, step 4's
      `git apply` succeeds, and the reviewer reports that the patch applied
      with the staged mutation gone, which is the evidence its findings
      cite. Measured with git 2.55.0 in a scratch repository, one unstaged
      and one staged edit: every command exited 0, and afterwards only the
      unstaged edit was in the tree (`findings/architecture.md` and
      `findings/spec-test.md` measured the same, re-review
      `9dc235c..34fd428`).
  - The `--ff-only`, `--remerge-diff` and `--cherry-mark` claims describe
    git's own behaviour, and a test of them would mostly re-test git.
  - **A stale round number also fails open, and is not a command defect.**
    A pre-tick search for a lane run again, typed with the earlier line's
    number, matches the earlier run's record over the same range whenever
    that run left one for the lane, and the runner ticks before the re-run
    has written anything (measured at `80c1bcc8` and again at `c3d697e9`;
    `findings/spec-test.md`, re-review round 5 `dc1390a..d1c8726`, box). The
    line rule above makes two numbers over one range routine. But the
    command in `RUNNER.md` carries `<n>` and `<range>` as placeholders, so
    no role file can hold a wrong number: the number is the runner's input
    when it runs the check, and a test that runs the role file's command
    against fixtures stays green whatever the runner types. This piece
    answers it in the pre-tick check itself, whose round and range are
    copied from that round's own line (above). That leaves two residuals,
    each the same gap as a runner skipping the check, and both belong to the
    `closer`-side follow-up below:
    - a runner copying from the wrong line;
    - a line that itself carries a repeated or stale number, such as a
      re-run's line templated from the previous one with the number left
      unchanged. The copy rule carries that number into the brief and the
      check, so all three agree and the check passes on the earlier run's
      record. The follow-up sees it only by its number criterion, not by
      matching forms.

  Not added here: this change adds no tests or CI (Impact), and a test
  holding its own copy of a command would not fail when a role file's copy
  changed. The useful test extracts each command from the role file and runs
  it against fixtures from the `lint` job, which is a piece of its own. It is
  a follow-up for the project manager to file, in place of the pathspec-only
  follow-up PR #174 lists, and the struck tester row in `tasks.md` says so.
- **An independent check of the re-review row by the `closer`.** A follow-up
  for the owner, written to be lifted into an issue as it stands:
  - **The gap.** The stage block's re-review row
    (`- [ ] re-review: every commit after the review round — runner`) is the
    one stage row not ticked by the agent that did the work. `RUNNER.md`,
    in step 3 of "From the `dev-writer`'s hand-back to the merge", has the
    runner run, before it ticks that row,
    ``git grep -l -F -e '## Re-review round <n> `<range>`' -e '**re-review round <n> `<range>`: no findings**' -- <change folder>/findings/``
    for every round the tick closes, with ``round <n> `<range>` `` copied
    from that round's line, and not tick while the findings file of any
    lane the round ran is missing. That makes the tick rest on the
    re-reviewers' own records, but the runner both runs the check and ticks
    the row, so a runner that skips the check, runs it with forms copied
    from the wrong line, or writes a round line whose number an earlier line
    already carries, goes unnoticed: `closer.md` Step 1 checks only that
    every row but the `closer`'s own is ticked or struck.
  - **What a `closer`-side check would do:** in Step 1, check that the round
    numbers under the re-review row are unique and consecutive, 1, 2, 3 in
    the order the lines stand; and match each round line under the row
    against the findings files, by the same two forms. Matching alone cannot
    see a repeated number: it derives each round's forms from that round's
    line, so a line repeating an earlier number yields forms the earlier
    run's record satisfies (`findings/security.md`, re-review round 8
    `6d43cda..d1d2165`, box). The number check reads only the line's start,
    its one fixed part, so it needs no fixed format for the rest of the
    line. On this piece's `tasks.md`, at `291499c7`, the eight round lines
    carry 1 to 8, once each.
  - **Why it is not a one-line addition.** The `closer` deletes `findings/`
    at the start of Step 3, just before `openspec archive`, so a
    re-dispatched `closer` has files only for the rounds recorded since
    that archive, and needs a way to tell those rounds from the earlier
    ones. And it would parse the runner's round lines, whose only fixed
    part is their start, ``round <n> `<range>` ``; the lanes and models
    that follow are free prose.
  - **Who reads or writes what would change:** `closer.md` Step 1;
    `RUNNER.md` step 3's paragraph on recording the call, which sets the
    round-line format, its sample lines, and its paragraph on ticking the
    row; `spec-writer.md`'s paragraph under the template on the re-review
    row; and `RUNNER.md`'s "What you read" row for the pre-tick command.
  - **For the issue to settle:** how the `closer` tells rounds recorded
    since the archive from those before; how much of the round line becomes
    a fixed format (the lanes, at least, for the `closer` to know which
    files to expect); and whether a missing file stops the `closer` or is
    reported.
- **The reviewer role files on rebasing with mutations in the tree.** A
  follow-up for the owner, written to be lifted into an issue as it stands:
  - **The gap.** `code-reviewer.md` and `spec-test-reviewer.md` tell a
    mutating reviewer to leave its mutations uncommitted, as evidence only
    the runner may discard, and say nothing about being continued to rebase
    its own branch after its cherry-pick conflicted, which the review round
    meets every time. `git rebase` refuses a dirty tree.
  - **Where the steps live now:** `RUNNER.md`'s "Dispatching", in the
    paragraph "An agent whose tree holds uncommitted changes cannot rebase
    as it stands", and the runner's continuation message carries them. Save
    the changes (`mkdir -p tmp`, then
    `git diff --binary --output=tmp/uncommitted.patch HEAD`); restore the
    tree and index (`git restore --source=HEAD --staged --worktree -- .`);
    rebase onto `piece/<name>` and resolve; re-apply
    (`git apply tmp/uncommitted.patch`) and report whether it applied.
  - **The change: move them, do not copy them.** The steps describe what the
    reviewer does in its own tree, so their durable home is the reviewer role
    files. They go into `code-reviewer.md` and `spec-test-reviewer.md`, and
    `RUNNER.md`'s paragraph shrinks to the condition (a tree holding
    uncommitted changes cannot rebase) and a pointer to those files, which
    the continuation message then cites rather than carries. Left in
    `RUNNER.md` as well, the same command sequence would sit in three files.
  - **Signing stays out of the role files.** The runner's message adds
    `--no-gpg-sign` to the rebase while the owner has signing switched off,
    and no role file names it, since the owner called that period
    temporary (PR #174's proposal, "Out of scope", on `closer.md`'s signing
    text).
  - **For the issue to settle:** whether each of the two role files carries
    the steps, or one carries them and the other points to it.
- **A second reader for a rejected finding.** A writer can close a finding
  as **rejected** in a commit that touches only `findings/`. Step 3 counts
  that as tracking, so no reviewer reads the rejection, and the `closer`
  reports a rejection it finds unconvincing but does not judge it. This is
  not #171's gap. #171 asks that every change on the branch be reviewed, and
  a rejection changes nothing that merges: the defect it declines to fix was
  read by the reviewer who raised it, and the rejection's argument lives in
  `findings/`, which the `closer` deletes. The gap also predates this piece.
  It is a follow-up for the owner, with three options:
  - (a) step 3 counts a commit recording a **rejected** outcome as needing
    review by the lane that raised the finding, which confirms the rejection
    or re-opens the box;
  - (b) the `closer` lists every rejected outcome in its report, and the
    runner relays them to the owner;
  - (c) leave it.

  Recommended: (a). It reuses the re-review round, and it puts the
  rejection in front of the one reader who measured the defect.
- **A semantic clash between the piece and a clean merge of `main`**, such as
  a function `main` renamed that the piece still calls. The merge needs no
  review, as above; CI is what sees such a clash, and a red run comes back
  as one.
- **Changing the size of the first review round.** Every reviewer row stays. A
  change with no source diff still gets all six reviewers. Only the re-review
  after that round is left to the runner's judgement.
- **Anything else under `.claude/`.** That means `settings.json`, hooks, and
  rewording in any role file beyond what these four issues, the
  owner-authorised additions and the `spec-writer.md`, `dev-writer.md` and
  `README.md` corrections above ask for. The writer and reviewer role files
  are not edited for the `closer`'s commits: a conflict resolver and a
  re-reviewer of an archive commit get what differs in the runner's brief, as
  every re-reviewer does. Nor are they edited for a mutating reviewer's
  rebase, whose steps the runner's continuation message carries. In
  `dev-writer.md`, only the stage-row clause and the four route sentences
  above change. No role file is edited for signing, as the entry
  on `closer.md`'s signing text above says. `CLAUDE.md` is not edited
  either.

### Overlap with open PR #132 (`piece/review-tiering`)

Both PRs change six files under `.claude/agents/`: `README.md`, `RUNNER.md`,
`closer.md`, `spec-writer.md`, `dev-writer.md` and `tester.md`. #132 changes
nine paths in all (`gh pr view 132 --json files`), and this piece changes those
six under `.claude/` (`git diff --stat origin/main...HEAD -- .claude/`). The
second to merge reconciles them. #132 is `CONFLICTING` with `main` and was last
updated on 2026-09-21. It was cut before the PLAN.md-to-Issues change: its diff
still has the `design-reviewer` reading `PLAN.md`.

Where they meet was measured by a trial merge, which writes no files:
`git merge-tree --write-tree --name-only HEAD origin/piece/review-tiering`, on
2026-09-27, with this piece at `6f17bebf` and #132 at `118b4ec`. Run with
`origin/main` in place of `HEAD`, the same command gives the conflicts #132
already has with `main` (`README.md`'s `PLAN.md` line, and `CLAUDE.md`), so
those are not listed here. Re-run it after any later edit to a file in the
list; the dev-writer's route edits in `dev-writer.md` land a few lines from
#132's hunk there.

- **Conflicts this piece adds**, one block each:
  - **`RUNNER.md`'s "How many at once" table.** This piece rewords the
    `spec-writer` / `dev-writer` / `tester` row's route ("bring its commits
    onto your HEAD (cherry-pick, or fast-forward where 'Dispatching'
    says)"), and #132 changes the next row, the reviewers, from "six, in
    parallel" to "three to five". Adjacent changed rows conflict, as this
    piece measured for the stage block. The merged table needs this piece's
    route wording and #132's count.
  - **`spec-writer.md`, the paragraph after the template.** This piece adds
    the paragraph on the re-review row, and #132 adds one on striking review
    rows by tier, at the same place. Both are new paragraphs, and the merge
    keeps both. #132's edit to the template itself (readability merged into
    correctness) auto-merges, and the merged template still has the
    re-review row after `review: design` and before the `closer`'s three.
  - **`README.md`, the `settings.json` paragraph.** #133, which this piece
    closes, restores the pre-#119 wording, "It is the **user's** file. Do not
    edit it on your own initiative; …"; #132 rewrites the same paragraph to
    "It is the owner's; machine-local settings go in `settings.local.json`".
    #133 is the owner's issue asking for the restored wording.
  - **`README.md`, "What the agent's own branch means for getting work
    back".** This piece changes "need a cherry-pick onto `piece/<name>`" to
    "need bringing onto"; #132 condenses the paragraph and says "Every
    agent's commits are cherry-picked onto `piece/<name>`", which is the
    route this piece corrects. The merged text says "brought onto", not
    "cherry-picked".
- **Auto-merged, and still to be read in place:**
  - **`closer.md`.** #132's hunks are four: Step 2's opening paragraph on a
    stale branch ("Three PRs here…" and the `UNKNOWN` evidence), Step 3's
    `openspec --version` paragraph, Step 3's "Then push it" upstream-check
    paragraph, and Step 4's cancelled-run anecdote. None touches Step 2's
    merge of `main`, which this piece rewrote, or #170's Step 6 and "What
    you never do". But #132's shortened "Then push it" paragraph lands in the
    Step 3 this piece restructured, between the `openspec/specs/` check run
    before the push and the push's three outcomes, and nothing in the merge
    prompts anyone to read it there.
  - **`dev-writer.md`.** #132 changes "six reviewers" to "every reviewer"
    and shortens the upstream-check paragraph, both under "The PR is yours",
    and shortens one sentence under "When you are acting on review
    findings"; this piece changes the stage-row clause in "`tasks.md`, and
    where your work lands" and the four route sentences, three of them under
    "The PR is yours".
  - **`tester.md`.** #132 adds one parenthetical to the "hardcoded
    expectation" paragraph; this piece changes what a decided marker
    becomes. No shared lines.
- **Lane names.** Any lane names this piece uses in its re-review guidance
  are the six on `main`. #132 merges readability into correctness, so the
  second to merge checks them.
- **The same subject, compatible rules.** #132 decides how many lanes the first
  review round gets, tiered by what the change contains. #171 decides how much
  re-review follows that round, by the runner's judgement. The two can coexist,
  but #132 as written drops `security` for a prose-only change, and this piece
  is itself prose-only and keeps all six.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change edits agent instructions and no system behaviour, so
`.openspec.yaml` declares `skip_specs: true` alongside its `schema:` key.

## Impact

- `.claude/agents/RUNNER.md`: the hand-back sequence (#169, #171; step 1's
  diff-scoped marker command and its request for product decisions; step 3's
  re-review brief, including the clean re-reviewer's verdict box and the
  heading naming the round, both in exact forms; round lines numbered, and a
  line of its own for a lane run again over the same range, fresh or
  continued; the
  runner's check before it ticks the re-review row, searching those two
  forms with the round and range copied from the round's line, with its row
  in "What you read"), "The `closer`, and what comes back" (#171; its returns
  given as examples with no count, including a `BLOCKED` PR and an unticked
  stage row, and a refused push that also reports a spec-changing archive;
  owner-authorised),
  "What a runner does" (the re-review row, #171; owner-authorised and
  owner-ruled: the runner's own content is that row alone, it always
  delegates, agents tick their own rows, an uncommitted output file goes back
  to its agent, and the fast-forward and "You do not rebase"), step 3's lists
  of what needs review (owner-authorised), the per-agent sequence in
  "Dispatching" (cherry-pick, or fast-forward for the `closer`, a conflict
  resolver and an agent whose commits are already on the remote piece ref;
  a refused fast-forward stops the runner; a cherry-pick that conflicts goes back to its agent, and the
  review round's ticks are expected to conflict; an agent whose tree holds
  uncommitted changes saves and re-applies them around its rebase;
  owner-authorised), the "One
  piece is one PR" bullet that said the runner cherry-picks the
  `dev-writer`'s pushed commits, and the sentences that gave the cherry-pick
  as the route for an agent's commits in general (owner-authorised, with the
  fast-forward rule), and where "What you read" and "Rebuild the state" find
  the stage block once the change is archived (#171).
- `.claude/agents/spec-writer.md`: the stage-block template (#171); and the
  one sentence above it claiming ticks on neighbouring rows cannot conflict,
  corrected (following from the owner's ruling "Agents tick their own").
- `.claude/agents/dev-writer.md`: one clause, "so concurrent agents'
  cherry-picks do not conflict", replaced by a pointer to `spec-writer.md`
  (following from the same ruling); and four sentences naming the
  cherry-pick as the route for the `dev-writer`'s commits, which say
  "brings … onto" or "bringing … onto" instead (following from the
  fast-forward rule, as the `README.md` passages do). Nothing else in the
  file.
- `.claude/agents/closer.md`: Step 6 and "What you never do" (#170); Step 1,
  for where a re-dispatched `closer` reads the stage block after the archive
  (#171); Step 3, which a re-dispatched `closer` does not re-archive in
  (#171, owner-authorised); and the closing paragraph, which points to
  `RUNNER.md`'s re-review step rather than restating it (#171). Owner-authorised
  as well: the order list, Step 1 and Step 3 (the `findings/` deletion moves
  from Step 1 to the start of Step 3, as part of the Step 2 rewrite), Step 2
  (merge `main`, stop on a conflict or a refused push), Step 3 (the
  `openspec/specs/` check run before the push, a stop after an archive that
  changes `openspec/specs/`, a stop on a refused push reporting the check's
  result, and push HEAD on a re-dispatch), Step 4's pointer to Step 2, "What
  you never do" (no force-push, no conflict resolution), "Your report"
  (including the check's result on a refused push), and the closing
  paragraph's list of what ends a turn. Not edited: the signing text in
  Step 2 and Step 6, except where Step 2's paragraph names the operation
  (see "Out of scope").
- `.claude/agents/tester.md`: what a marker the brief names as decided becomes,
  and "must not remove" narrowed to open markers (#169, owner-authorised).
- `.claude/agents/README.md`: one paragraph (#133); and, owner-authorised, the
  branch section's account of how work reaches `piece/<name>`, which points
  to `RUNNER.md`; and the four passages outside that section naming the
  cherry-pick as the route for any agent's commits, which say "brought onto"
  or "bringing onto" instead (following from the fast-forward rule, as the
  `dev-writer.md` clause does). Nothing in its stage-block section changes. "Each agent
  flips its own row" stands, as the owner ruled. "One row per stage, then
  three rows the `closer` owns" also stays: the re-review row is one more
  row before the `closer`'s three, so the sentence still holds.

No code, tests, CI workflow or spec changes.
