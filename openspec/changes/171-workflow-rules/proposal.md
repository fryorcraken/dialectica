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
  choice is.
- **#171: every change made after the review round is reviewed before the
  `closer` runs.** `RUNNER.md` requires review of every commit added to the
  piece after the review round, before the `closer` is dispatched. That covers:
  - a writer's pass answering findings;
  - a rewrite or new file made on an owner instruction;
  - a spec callback;
  - a fix for a red CI run.

  "The `closer`, and what comes back" no longer tells the runner to re-dispatch
  the `closer` straight after a fixer. The runner decides how big the
  re-review is: which lanes, how many reviewers, and which model each runs on,
  using the Agent tool's `model` override. It records that call, and any
  decision to skip a re-review, in its report and in the piece's `tasks.md`.
  When unsure, it re-reviews.
- **#171: the stage-block template gets a place to record a re-review round.**
  The template lives in `spec-writer.md`, which `README.md` names as its only
  copy. Where and how that place is written is the `dev-writer`'s call. Two
  existing rules constrain it. Nobody adds a row, which keeps concurrent
  cherry-picks from colliding on one line. And the runner does not write the
  work.
- **#170: forbid `--admin` in `closer.md`.** The file names `gh pr merge
  --admin`, and any change to branch protection, as things the `closer` never
  does, including when the owner has granted merge-on-green. If the PR stays
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
- **Keeping the `spec-writer`'s tree until the `dev-writer`'s first pass.** #169
  marks this optional, and the owner's scope for this piece does not include
  it.
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

- `.claude/agents/RUNNER.md`: the hand-back sequence (#169, #171) and "The
  `closer`, and what comes back" (#171).
- `.claude/agents/spec-writer.md`: the stage-block template (#171).
- `.claude/agents/closer.md`: Step 6 and/or "What you never do" (#170).
- `.claude/agents/README.md`: one paragraph (#133).

Two nearby sentences describe the runner's own sequence and may now be
inaccurate. Neither should become a second copy of the rule:

- `closer.md`'s closing paragraph: "The runner dispatches a fresh `closer` when
  the fix has landed."
- `README.md`'s "one row per stage, then three rows the `closer` owns".

If the new template makes either one wrong, the fix is to point to `RUNNER.md`
or `spec-writer.md`, not to restate the rule.

No code, tests, CI workflow or spec changes.
