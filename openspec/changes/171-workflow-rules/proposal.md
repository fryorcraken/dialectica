# Workflow rules: review everything that merges, route `NO SPEC:` first, forbid `--admin`, restore one README paragraph

Closes #171, #170, #169 and #133. All four are `workflow` issues, and all four
edit files under `.claude/agents/`. The owner authorised those edits for this
piece, limited to what the four issues ask for and to four additions the owner
authorised in session, each marked **owner-authorised** below.

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
tracking on reviewers' behalf (`a284e51`), and findings files copied from
reviewers' trees (`c43c7a7`). Nothing says which of those a runner may do
(`findings/architecture.md`). The owner took both into this piece.

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
  - **The brief points at markers by command**, `git grep -n "NO SPEC:"`,
    rather than listing them. **A behaviour decision reported without a marker
    is quoted word for word** from the hand-back, since no file holds it.
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
    outcome written into `findings/`, a stage-row tick, or the runner's own
    record line under the re-review row. A commit that moves reasoning into
    `design.md` does need review.
  - **`RUNNER.md` names no model**, only the Agent tool's `model` override.
  - **A re-review brief carries three things:** the commit range to read (for
    `spec-test-reviewer`, only the spec and test files in it); that the
    reviewer's stage row is already ticked and stays ticked; and that new
    findings are appended as boxes to that reviewer's existing findings file.
    If the `closer` has already deleted `findings/`, which it does before
    archiving, the file is written afresh at the same name in the change's
    folder as it now stands, the archived one, **and only when the reviewer
    has a finding**. A clean re-review adds no box either way: before the
    archive it appends nothing, and after it it writes no file, saying so in
    its report. A fresh file with no box would fail the `closer`'s
    "every file non-zero" check over a review that found nothing.
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
    **unticks it when such a commit lands after the tick**, such as a red-CI
    fix. This is the only row in the stage block that ever goes from `[x]` back
    to `[ ]`.
  - **The runner commits the record lines, the tick and any untick itself**, in
    its own tree on `piece/<name>`, before the next dispatch forks from it.
  - **The `closer` is dispatched only when every row above its own three is
    ticked or struck**, this row included, so an unticked re-review row stops
    the `closer` at its existing Step 1 gate.
  - **After the `closer` has archived the change, the stage block is in
    `openspec/changes/archive/<date>-<name>/tasks.md`.** The runner's untick
    for a red-CI fix goes there, and a re-dispatched `closer`'s Step 1 reads
    the block there. In detail:
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
    - **An archived folder with no `findings/` passes the findings gate.** The
      earlier `closer` deleted it and no re-reviewer has had a finding since.
      A `grep` error for the missing directory is not a failed gate.
    - **A re-dispatched `closer` does not archive again.** `closer.md` Step 3
      says so (owner-authorised in session): `openspec archive` is not rerun,
      but Step 3's commit and push still apply. A `findings/` deleted in
      Step 1 is committed, and HEAD is pushed either way, as the entry on the
      `closer`'s own commits below sets out, so the run it watches includes
      everything the piece now carries.
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
  - **The `closer` brings `main` in by merging it, not by rebasing onto it.**
    Step 2's fix for a stale branch becomes `git merge origin/main` in the
    `closer`'s own tree, pushed to the piece ref by refspec with no force. The
    rebase and its `--force-with-lease` push are removed, and "What you never
    do" forbids force-pushing for any reason. The reason is the conflict rule
    two bullets down: a writer's conflict resolution has to be one commit that the runner can
    bring onto its HEAD without rewriting it, and that a reviewer can read on
    its own. A later rebase would drop that merge commit and raise the same
    conflict again.
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
  - **An archive commit that changes the live contract is reviewed.** After
    Step 3's push, if `git diff --name-only HEAD^ HEAD -- openspec/specs/`
    lists any file, the `closer` stops before Step 4: it reports the archive
    commit and returns. An archive commit that changes nothing under
    `openspec/specs/` carries on to Step 4. A change with `skip_specs: true`
    is always that case, since archiving it only moves the change folder and
    commits the `findings/` deletion. The runner sizes a round for the archive
    commit like any other, then re-dispatches the `closer`. That `closer`
    finds the change archived and does not archive again, by the Step 1 and
    Step 3 rules for a re-dispatch above.
  - **A re-dispatched `closer` pushes its HEAD at Step 3 whether or not it
    deleted anything.** Its tree carries every commit the runner brought onto
    the piece since the last push: a fix, a conflict resolution, the runner's
    record lines and tick. The run it watches in Step 4 must include them.
    This replaces the reading of Step 3's re-dispatch paragraph under which
    the push happens only when Step 1 deleted a `findings/`.
  - **The runner brings the `closer`'s commits onto its HEAD by fast-forward.**
    When a `closer` returns without having merged, whatever else it reports,
    the runner first runs
    `git merge --ff-only <the closer's branch>` in its own tree. Only then
    does it write to the stage block or dispatch the next agent. Without this,
    the archived `tasks.md` the runner unticks in is not in its tree, and the
    next agent forks from a HEAD without the archive commit or the merge of
    `main`. A conflict resolver's branch is brought on the same way, since
    `git cherry-pick` cannot carry a merge commit's second parent. If a
    fast-forward refuses, the runner stops and reports. It never rebases,
    resets or force-pushes `piece/<name>`, and it does not merge `main`
    itself.
  - **`RUNNER.md` step 3 names what the `closer` adds.** Its list of what
    needs review gains a writer's conflict resolution and an archive commit
    that changes `openspec/specs/`. Its list of what needs none gains a merge
    of `main` with no conflict, the `closer`'s deletion of `findings/`, and an
    archive commit that changes nothing under `openspec/specs/`.
  - **Step 4, "The `closer`, and what comes back", lists all four returns:** a
    red run, an unticked box, a conflict, and an archive commit that changed
    `openspec/specs/`. Each of the last two goes back through step 3, as a
    red-CI fix does. "A stale branch does not come back" now holds only for a
    merge of `main` with no conflict.
  - **`README.md`'s branch section says how work reaches `piece/<name>`.** Its
    table says the runner cherry-picks every agent's commits, and it says
    "Cherry-pick rather than merge". Both gain the fast-forward case above, by
    pointing to `RUNNER.md` rather than restating it.
- **What the runner commits itself (owner-authorised, from
  `findings/architecture.md`'s third finding).** `RUNNER.md`'s "What a runner
  does" states it once, and step 3 and the red-run text point there:
  - **Tracking is the runner's to commit.** That is:
    - the re-review row's record lines, tick and untick;
    - a stage-row tick for an agent whose hand-back reports its stage done and
      whose output is on the piece, where the agent did not tick the row
      itself (as `a284e51` did for five reviewers at once). `README.md`'s
      "Each agent flips its own row" gains this case, by pointing to
      `RUNNER.md`;
    - an agent's output file that the agent wrote but could not commit, copied
      byte for byte from its tree, unread, with the commit message naming the
      tree it came from (as `c43c7a7` did after a signing failure).
  - **Work is not the runner's to commit.** Work is a proposal, a spec,
    `design.md`, code, tests, role-file text, or a finding's outcome. The one
    exception is an edit the owner tells the runner itself to make, in the
    session, as in `d805fe4`. An authorisation the runner relays for a
    dispatched agent's edit is not that instruction. Neither is a brief, a
    finding, or the edit looking small. Such a commit says in its message that
    the owner instructed it directly. The runner makes it only while no writer
    is running on the piece. It is work, so when it lands after the review
    round it goes through step 3 like any other commit.
  - **"You do not write the work — not even one small edit while an agent is
    being prepared" stays.** The exception is the owner's instruction, never
    the runner's own initiative.

### Out of scope

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
  rewording in any role file beyond what these four issues and the
  owner-authorised additions above ask for. The writer and reviewer role files
  are not edited for the `closer`'s commits: a conflict resolver and a
  re-reviewer of an archive commit get what differs in the runner's brief, as
  every re-reviewer does. `CLAUDE.md` is not edited either.

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

- `.claude/agents/RUNNER.md`: the hand-back sequence (#169, #171), "The
  `closer`, and what comes back" (#171; its four returns, owner-authorised),
  "What a runner does" (the re-review row, #171; what the runner commits, the
  fast-forward, and "You do not rebase", owner-authorised), step 3's lists of
  what needs review (owner-authorised), the per-agent sequence in
  "Dispatching" (cherry-pick, or fast-forward for the `closer` and a conflict
  resolver, owner-authorised), and where "What you read" and "Rebuild the
  state" find the stage block once the change is archived (#171).
- `.claude/agents/spec-writer.md`: the stage-block template (#171).
- `.claude/agents/closer.md`: Step 6 and "What you never do" (#170); Step 1,
  for where a re-dispatched `closer` reads the stage block after the archive
  (#171); Step 3, which a re-dispatched `closer` does not re-archive in
  (#171, owner-authorised); and the closing paragraph, which points to
  `RUNNER.md`'s re-review step rather than restating it (#171). Owner-authorised
  as well: the order list, Step 2 (merge `main`, stop on a conflict), Step 3
  (stop after an archive that changes `openspec/specs/`, and push HEAD on a
  re-dispatch), Step 4's pointer to Step 2, "What you never do" (no
  force-push, no conflict resolution), "Your report", and the closing
  paragraph's list of what ends a turn.
- `.claude/agents/tester.md`: what a marker the brief names as decided becomes,
  and "must not remove" narrowed to open markers (#169, owner-authorised).
- `.claude/agents/README.md`: one paragraph (#133); and, owner-authorised, the
  branch section's account of how work reaches `piece/<name>` and the stage
  block's "Each agent flips its own row". Its "one row
  per stage, then three rows the `closer` owns" stays as it is: the re-review
  row is one more row before the `closer`'s three, so the sentence still
  holds.

No code, tests, CI workflow or spec changes.
