# Workflow rules: review everything that merges, route `NO SPEC:` first, forbid `--admin`, restore one README paragraph

Closes #171, #170, #169 and #133. All four are `workflow` issues, and all four
edit files under `.claude/agents/`. The owner authorised those edits for this
piece, limited to what the four issues ask for and to four additions the owner
authorised in session, each marked **owner-authorised** below. One sentence in
`spec-writer.md` is corrected as well: it is measured false, and left alone it
would contradict a rule this piece adds under an owner ruling. The entry for
it says why that is inside the authorisation.

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
  - **A re-review brief carries four things:** the commit range to read (for
    `spec-test-reviewer`, only the spec and test files in it); that the
    reviewer's stage row is already ticked and stays ticked; that new
    findings are appended as boxes to that reviewer's existing findings file;
    and that **a re-review which finds nothing appends one ticked verdict box
    instead**, naming the commit range it read and saying it found nothing,
    for example:

    ```markdown
    - [x] **re-review `a1b2c3d..e4f5a6b`: no findings** — read <what>; clean
    ```

    The reviewer writes and commits that box itself, as it would a finding.
    It is the reviewer's own record that the round ran: a re-reviewer's stage
    row is already ticked, so without the box a clean round leaves nothing
    but the runner's line under the re-review row, which nothing checks
    against a reviewer's output. The box is ticked, so the `closer`'s
    `grep -rn "^- \[ \]"` passes it, and it is a box, so `grep -rc "^- \["`
    counts it. Clean *areas* stay in prose, as the reviewer role files say;
    the verdict box is one line for the round, not a box per area.
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
  - **The runner writes one indented line under it per round:** the commit
    range, what landed, the lanes and the model each ran on, and why that size.
    A round the runner skips also gets a line, with the reason. These lines are
    not rows: they carry no box.
  - **The row is never struck.** A round with nothing to review gets its line
    and then a tick.
  - **The runner ticks it when no commit that merges is unreviewed**, and
    **unticks it when any commit that needs review lands after the tick**: a
    red-CI fix, a writer's conflict resolution, an archive commit that
    changed `openspec/specs/`, or any other commit step 3 lists. This is the
    only row in the stage block that ever goes from `[x]` back to `[ ]`.
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
    reviewed. `closer.md` states this for both pushes.
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
    every first dispatch.
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
    owner chose to stop when specs change). After
    Step 3's push, if `git diff --name-only HEAD^ HEAD -- openspec/specs/`
    lists any file, the `closer` stops before Step 4: it reports the archive
    commit and returns. An archive commit that changes nothing under
    `openspec/specs/` carries on to Step 4. A change with `skip_specs: true`
    is always that case, since archiving it only moves the change folder and
    commits the `findings/` deletion. The runner sizes a round for the archive
    commit like any other, then re-dispatches the `closer`. That `closer`
    finds the change archived and does not archive again, by the Step 1 and
    Step 3 rules for a re-dispatch above. **The check applies only to an
    archive commit the `closer` made in the same run.** A re-dispatched
    `closer` makes none, so it skips the check: its HEAD is whatever the runner
    brought onto the piece last, which step 3 has already reviewed.
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
    ref**, which in the current flow is the `dev-writer`'s first pass: it
    pushes its tip to `refs/heads/piece/<name>` before handing back. A
    cherry-pick that is not a fast-forward copies those commits to new
    SHAs, so the runner's `piece/<name>` no longer descends from the remote
    ref, and every later push of a HEAD forked from it, the `closer`'s
    included, is refused, since no agent may force. Measured on this piece's
    reflog: the `dev-writer`'s pushed `c67aa24`..`d41d0fd` were
    cherry-picked into new commits at 23:32 on 2026-09-25, and the runner
    then ran `reset` to `d41d0fd`, which `RUNNER.md` now forbids.
    `git merge --ff-only` either keeps the SHAs or refuses; it never
    diverges silently, which `git cherry-pick --ff` does when HEAD is not
    the commit's parent. If a
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
  - **Step 4, "The `closer`, and what comes back", lists every return:** a
    red run, an unticked box, a conflict, an archive commit that changed
    `openspec/specs/`, and a refused push. A conflict and a spec-changing
    archive go back through step 3, as a red-CI fix does. **A refused push
    goes to the owner:** the remote piece ref holds a commit the runner's
    HEAD lacks, which the fast-forward rule above exists to prevent, and the
    runner cannot reconcile the two without a reset, a force-push or a merge
    of its own, all of which it never does. It reports the output of
    `git log --oneline --left-right --cherry-mark HEAD...origin/piece/<name>`
    and waits. "A stale branch does not come back" now holds only for a
    merge of `main` with no conflict. `closer.md` Step 6's `BLOCKED` stop is
    unchanged and reported as before.
  - **`README.md`'s branch section says how work reaches `piece/<name>`.** Its
    table's `piece/<name>` row says the runner cherry-picks every agent's
    commits, and the section says "Cherry-pick rather than merge". Both gain
    the fast-forward cases above, by pointing to `RUNNER.md` rather than
    restating them, and neither enumerates them in a way that reads as
    complete. The same section's two other statements of the old route,
    the `worktree-agent-<id>` row's "cherry-picked onto the piece" and "every
    agent's commits are cherry-picked onto it", say "brought onto" instead,
    so the section does not contradict its own pointer.
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
- **A semantic clash between the piece and a clean merge of `main`**, such as
  a function `main` renamed that the piece still calls. The merge needs no
  review, as above; CI is what sees such a clash, and a red run comes back
  as one.
- **Changing the size of the first review round.** Every reviewer row stays. A
  change with no source diff still gets all six reviewers. Only the re-review
  after that round is left to the runner's judgement.
- **Anything else under `.claude/`.** That means `settings.json`, hooks, and
  rewording in any role file beyond what these four issues, the
  owner-authorised additions and the `spec-writer.md` correction above ask
  for. The writer and reviewer role files
  are not edited for the `closer`'s commits: a conflict resolver and a
  re-reviewer of an archive commit get what differs in the runner's brief, as
  every re-reviewer does. `dev-writer.md`'s "The runner cherry-picks your
  commits onto its own local `piece/<name>` afterwards" is not edited either:
  bringing its first pass on by fast-forward is the runner's step, stated in
  `RUNNER.md`, and changes nothing the `dev-writer` does. No role file is edited for signing, as the entry
  on `closer.md`'s signing text above says. `CLAUDE.md` is not edited
  either.

### Overlap with open PR #132 (`piece/review-tiering`)

The two PRs overlap in four files, and the second to merge must reconcile
them. #132 is `CONFLICTING` with `main` and was last updated on 2026-09-21. It
was cut before the PLAN.md-to-Issues change: its diff still has the
`design-reviewer` reading `PLAN.md`.

- **The same text.** #132 rewrites the `settings.json` paragraph that #133
  restores, to "It is the owner's; machine-local settings go in
  `settings.local.json`". It also edits the stage-block template in
  `spec-writer.md`, where this piece adds the re-review place: #132 merges the
  readability row into correctness and adds a paragraph on striking review
  rows by tier.
- **The same text in `closer.md`.** #132 edits Step 2's rebase and push prose
  and the CI-watch prose. This piece replaces Step 2's rebase with a merge of
  `main`, so the second to merge rewrites #132's rebase hunks against a Step 2
  that no longer rebases. #170's Step 6 and "What you never do" edits do not
  overlap #132.
- **Nearby text in `RUNNER.md`.** #132 rewrites "How many at once" and the
  reviewer table. This piece rewrites "The `closer`, and what comes back" and
  adds the hand-back sequence. The hunks differ, but any lane names this piece
  uses in its re-review guidance are the six on `main`. #132 would rename them.
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
  re-review brief, including the clean re-reviewer's verdict box), "The
  `closer`, and what comes back" (#171; its four returns, owner-authorised),
  "What a runner does" (the re-review row, #171; owner-authorised and
  owner-ruled: the runner's own content is that row alone, it always
  delegates, agents tick their own rows, an uncommitted output file goes back
  to its agent, and the fast-forward and "You do not rebase"), step 3's lists
  of what needs review (owner-authorised), the per-agent sequence in
  "Dispatching" (cherry-pick, or fast-forward for the `closer`, a conflict
  resolver and an agent whose commits are already on the remote piece ref;
  a refused fast-forward stops the runner; a cherry-pick that conflicts goes back to its agent, and the
  review round's ticks are expected to conflict; owner-authorised), and where "What you read" and "Rebuild the state" find
  the stage block once the change is archived (#171).
- `.claude/agents/spec-writer.md`: the stage-block template (#171); and the
  one sentence above it claiming ticks on neighbouring rows cannot conflict,
  corrected (following from the owner's ruling "Agents tick their own").
- `.claude/agents/closer.md`: Step 6 and "What you never do" (#170); Step 1,
  for where a re-dispatched `closer` reads the stage block after the archive
  (#171); Step 3, which a re-dispatched `closer` does not re-archive in
  (#171, owner-authorised); and the closing paragraph, which points to
  `RUNNER.md`'s re-review step rather than restating it (#171). Owner-authorised
  as well: the order list, Step 1 and Step 3 (the `findings/` deletion moves
  from Step 1 to the start of Step 3, as part of the Step 2 rewrite), Step 2
  (merge `main`, stop on a conflict or a refused push), Step 3 (stop after an
  archive that changes `openspec/specs/`, stop on a refused push, and push
  HEAD on a re-dispatch), Step 4's pointer to Step 2, "What you never do" (no
  force-push, no conflict resolution), "Your report", and the closing
  paragraph's list of what ends a turn. Not edited: the signing text in
  Step 2 and Step 6, except where Step 2's paragraph names the operation
  (see "Out of scope").
- `.claude/agents/tester.md`: what a marker the brief names as decided becomes,
  and "must not remove" narrowed to open markers (#169, owner-authorised).
- `.claude/agents/README.md`: one paragraph (#133); and, owner-authorised, the
  branch section's account of how work reaches `piece/<name>`, which points
  to `RUNNER.md`. Nothing in its stage-block section changes. "Each agent
  flips its own row" stands, as the owner ruled. "One row per stage, then
  three rows the `closer` owns" also stays: the re-review row is one more
  row before the `closer`'s three, so the sentence still holds.

No code, tests, CI workflow or spec changes.
