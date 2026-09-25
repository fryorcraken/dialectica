# Workflow rules: review everything that merges, route `NO SPEC:` first, forbid `--admin`, restore one README paragraph

Closes #171, #170, #169 and #133. All four are `workflow` issues, and all four
edit files under `.claude/agents/`. The owner authorised those edits for this
piece, limited to what the four issues ask for.

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
    `spec-writer`'s commit, and says the markers it decided are closed: each
    is reworded or removed to match the new spec text, not kept as an open
    question for the review round.
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

### Out of scope

- **Why #165 was `BLOCKED`** (#170's closing paragraph). This piece only stops
  a closer from going around a block. It does not diagnose one.
- **Re-review of commits the `closer` makes itself**: its rebase, including any
  conflict it resolves, and its archive commit. The re-review rule covers
  commits made before the `closer` is dispatched, and a red-CI fix made after
  it. `design.md`'s Risks names the rebase gap as a follow-up candidate.
- **Editing `tester.md`**, whose "Keep the markers and report each one" is out
  of date once the `spec-writer` has decided a marker before the `tester` runs.
  None of the four issues asks for a `tester.md` change, so it goes to the
  owner as a proposal. Until then, the `tester`'s brief carries the
  difference, as the #169 item above says.
- **Changing the size of the first review round.** Every reviewer row stays. A
  change with no source diff still gets all six reviewers. Only the re-review
  after that round is left to the runner's judgement.
- **Anything else under `.claude/`.** That means `settings.json`, hooks, and
  rewording in any role file beyond what these four issues ask for. `CLAUDE.md`
  is not edited either.

### Overlap with open PR #132 (`piece/review-tiering`)

The two PRs overlap in three files, and the second to merge must reconcile
them. #132 is `CONFLICTING` with `main` and was last updated on 2026-09-21. It
was cut before the PLAN.md-to-Issues change: its diff still has the
`design-reviewer` reading `PLAN.md`.

- **The same text.** #132 rewrites the `settings.json` paragraph that #133
  restores, to "It is the owner's; machine-local settings go in
  `settings.local.json`". It also edits the stage-block template in
  `spec-writer.md`, where this piece adds the re-review place: #132 merges the
  readability row into correctness and adds a paragraph on striking review
  rows by tier.
- **Nearby text in `RUNNER.md`.** #132 rewrites "How many at once" and the
  reviewer table. This piece rewrites "The `closer`, and what comes back" and
  adds the hand-back sequence. The hunks differ, but any lane names this piece
  uses in its re-review guidance are the six on `main`. #132 would rename them.
- **The same subject, compatible rules.** #132 decides how many lanes the first
  review round gets, tiered by what the change contains. #171 decides how much
  re-review follows that round, by the runner's judgement. The two can coexist,
  but #132 as written drops `security` for a prose-only change, and this piece
  is itself prose-only and keeps all six.

`closer.md` hunks do not overlap. #132 edits the rebase, push and CI-watch
prose. #170 edits Step 6 and "What you never do".

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change edits agent instructions and no system behaviour, so
`.openspec.yaml` declares `skip_specs: true` alongside its `schema:` key.

## Impact

- `.claude/agents/RUNNER.md`: the hand-back sequence (#169, #171), "The
  `closer`, and what comes back" (#171), the re-review row in "What a
  runner does" (#171), and where "What you read" and "Rebuild the state" find
  the stage block once the change is archived (#171).
- `.claude/agents/spec-writer.md`: the stage-block template (#171).
- `.claude/agents/closer.md`: Step 6 and "What you never do" (#170); Step 1,
  for where a re-dispatched `closer` reads the stage block after the archive
  (#171); and the closing paragraph, which points to `RUNNER.md`'s re-review
  step rather than restating it (#171).
- `.claude/agents/README.md`: one paragraph (#133). Its "one row per stage,
  then three rows the `closer` owns" stays as it is: the re-review row is one
  more row before the `closer`'s three, so the sentence still holds.

No code, tests, CI workflow or spec changes.
