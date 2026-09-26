# Tasks — workflow rules (#171, #170, #169, #133)

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change edits agent
      instructions under `.claude/agents/` only and alters no behaviour of
      dialectica, so no requirement in `openspec/specs/` changes. Declared as
      `skip_specs: true` alongside `schema:` in `.openspec.yaml`. The
      `spec-writer`'s work here is `proposal.md` and this block.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — prose-only change to agent instructions. No CI
      job, script or test reads `.claude/agents/`, and a test pinning the new
      wording would fail when the wording changed, not when the rule was
      wrong. The git commands the role files name are executable, and each
      was measured when it was written; a standing test for them is a
      follow-up (`proposal.md`, "Out of scope"). The check that can see this
      change is the six reviewers reading the prose.
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] re-review: every commit after the review round — runner
      round 1 `c222c37..9dc235c` findings passes, owner-authorised scope (closer merge-not-rebase, runner-commit rule), spec callbacks — all six lanes, sized for the strongest model: closer.md Step 2 rewritten, new RUNNER.md section, large proposal.md and design.md changes, force-push and merge rules are security-relevant. Correction: no model override was passed, so all six ran on Sonnet 5 (role default). design, spec-test and architecture accepted on Sonnet — decision records, contract and file placement, each re-ran its checkable claims. correctness and readability parked on banned shell shapes and were stopped; they and security re-run on Opus 5.5. Pre-numbering round: briefed with the un-numbered forms, so check it with ``git grep -l -F -e '## Re-review `c222c37..9dc235c`' -e '**re-review `c222c37..9dc235c`: no findings**' -- openspec/changes/171-workflow-rules/findings/`` — lists all six files at `cd5b755`. For security, the accepted run is the Opus section ``## Re-review `c222c37..9dc235c` (Opus)`` (`5961903`); the earlier Sonnet verdict box in the same file (`0f55b92`) was not accepted and does not count for this round.
      round 2 `9dc235c..34fd428` spec callbacks and dev-writer passes answering round 1's 11 boxes (closer's spec check before the push, returns without a count, pre-tick grep check, mutating-reviewer rebase, fast-forward every dev-writer pass, README and dev-writer.md wording) — all six lanes on Opus 5.5 (model override passed): role files, proposal.md and design.md all changed substantially, and the closer and runner rules are security-relevant. Pre-numbering round: briefed with the un-numbered forms, so check it with ``git grep -l -F -e '## Re-review `9dc235c..34fd428`' -e '**re-review `9dc235c..34fd428`: no findings**' -- openspec/changes/171-workflow-rules/findings/`` — lists all six files at `cd5b755`.
      round 3 `34fd428..dc1390a` spec callbacks and dev-writer pass answering round 2's 20 boxes (round-numbered pre-tick check, standing-test follow-up, #132 overlap, dev-writer.md route words, untracked-only tree) — all six lanes on Opus 5.5 (model override passed), narrowed: each lane confirms its own round-2 fixes, and new issues are boxed only at medium severity or above, lower in prose. Every lane raised round-2 boxes; rounds went 11 → 20, and the loop has to converge
      round 4 `34fd428..dc1390a` readability re-dispatched (Opus 5.5): the round-3 readability run parked on a permission prompt (backticks inside a double-quoted grep pattern) and was stopped having written nothing; same range and narrowing as round 3, so round 3's check skips readability
      round 5 `dc1390a..d1c8726` spec-writer on the last two round-3 boxes (re-run line rule: a continued re-run gets its own line too) and the dev-writer's RUNNER.md/design.md wording; runner's own record fix `549c09c` — security, correctness, spec-test, design on Opus 5.5 (model override passed), narrowed as round 3: security raised the box, correctness reads the rule literally, proposal.md and design.md changed. Skipped: architecture (no rule moved between files) and readability (round 4 verified it; one sentence changed since)
      round 6 `d1c8726..6d43cda` spec-writer's stale-round-number reclassification in proposal.md's standing-test follow-up, the dev-writer's matching design.md Risk, runner's own round-1 record note `80c1bcc` — spec-test and design on Opus 5.5 (model override passed), narrowed: only proposal.md and design.md changed (no .claude/ file in the range), so correctness, security, readability and architecture have no ground here and are skipped
      round 7 `6d43cda..d1d2165` spec-writer's copy rule for the pre-tick check (proposal.md) and the dev-writer's RUNNER.md tick-paragraph clause and design.md Decision/Risks — correctness, security, spec-test, design on Opus 5.5 (model override passed), narrowed: correctness reads the clause literally, security because the pre-tick check is a gate, proposal.md and design.md changed. Skipped: architecture (no rule moved between files) and readability (one clause)
      round 8 `6d43cda..d1d2165` security re-dispatched (Opus 5.5): the round-7 security run parked on a permission prompt (a backtick inside a double-quoted grep pattern) and was stopped having written nothing; same range and narrowing as round 7, so round 7's check skips security
      round 9 `d1d2165..c4b1df5` spec-writer's record-and-route of the stale-line-number residual (proposal.md) and the dev-writer's matching design.md and PR body — security, spec-test, design on Opus 5.5 (model override passed), narrowed: security confirms its round-8 box, proposal.md and design.md changed. No .claude/ file in the range, so correctness, readability and architecture are skipped
      round 10 `c4b1df5..842758b` spec-writer's number check before the forms check (proposal.md) and the dev-writer's RUNNER.md tick paragraph and "What you read" row, design.md Decision/Risks and PR body — correctness, security, readability, spec-test, design on Opus 5.5 (model override passed), narrowed: RUNNER.md changed, so correctness, security and readability read it; proposal.md and design.md changed. Skipped: architecture (no rule moved between files)
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

No test can see any of these tasks: no CI job, script or test reads
`.claude/agents/`. Each one is checked by reading the file, and the check
named on each task is that reading.

### 1. `RUNNER.md`: the hand-back-to-merge sequence (#169, #171)

- [x] 1.1 Replace "The `closer`, and what comes back" with a section, "From
      the `dev-writer`'s hand-back to the merge", that has four numbered steps.
      The `closer` keeps its heading as a subsection. Verify: one section, and
      no second statement of either rule elsewhere in the file
      (`git grep -n -F "NO SPEC" -- .claude/agents/RUNNER.md`).
- [x] 1.2 Step 1 routes `NO SPEC:` markers and unmarked behaviour decisions to a
      fresh `spec-writer` before the `tester`. It also says not to put markers
      to the owner. Verify: the step names the grep for the brief, the verbatim
      quotation for unmarked decisions, and who goes next after the spec-writer.
- [x] 1.3 Step 3 requires review of every commit that merges after the review
      round, covering all four #171 cases. It leaves the size and models to the
      runner, and says where the call is recorded. Verify: the red-CI path in
      step 4 returns to step 3 and no longer says to re-dispatch the `closer`
      straight after a fixer.
- [x] 1.4 Add recording the re-review row to "What a runner does" as the
      runner's own tracking, not the work.

### 2. `spec-writer.md`: a place for the re-review round (#171)

- [x] 2.1 Add one runner-owned re-review row to the template, above the
      `closer`'s three, and a sentence pointing to `RUNNER.md` for what goes
      under it. Verify: no existing row is changed, and the template is still
      the only copy (`git grep -n -F "review: design" -- .claude/agents`).

### 3. `closer.md`: forbid `--admin` (#170)

- [x] 3.1 Step 6 names `gh pr merge --admin` and any branch-protection or
      ruleset change as forbidden, including under merge-on-green. On `BLOCKED`
      with every required check green, the closer stops and reports the
      `gh pr view` output #170 names. Verify: `git grep -n -F -- "--admin" --
      .claude/agents/closer.md`.
- [x] 3.2 Add a bullet to "What you never do" that points to Step 6.
- [x] 3.3 Change the closing "The runner dispatches a fresh `closer` when the
      fix has landed" so it points to `RUNNER.md`'s sequence and no longer
      skips the re-review.

### 4. `README.md`: restore one paragraph (#133)

- [x] 4.1 Replace the `settings.json` paragraph under "`baseRef: "head"` is
      required" with the text of
      `git grep -n -F "user's** file" 51ab7f8^ -- .claude/agents/README.md`.
      Verify: `git diff origin/main...HEAD -- .claude/agents/README.md` shows
      that one paragraph and nothing else. `dev-writer.md` is untouched.

### 5. Hand-off

- [x] 5.1 `openspec validate --strict 171-workflow-rules` passes.
- [x] 5.2 Push to the remote piece ref by refspec, and open the PR with
      `Closes #171`, `#170`, `#169` and `#133` on their own lines.

### 6. After the `spec-writer` callback (`6c2149c`)

- [x] 6.1 `RUNNER.md` step 3, "What counts": tracking is a box flipped, a
      finding's outcome in `findings/`, a stage-row tick, or the runner's own
      record line under the re-review row. Verify: the record line is named in
      the list.
- [x] 6.2 `RUNNER.md` step 3: after the `closer` has archived, the stage block
      and `findings/` are in `openspec/changes/archive/<date>-<name>/`, and the
      untick goes there. Verify: the paragraph after the untick names the
      archived path.
- [x] 6.3 `closer.md` Step 1 finds the change folder with
      `git ls-files -- "openspec/changes/*<name>/tasks.md"` and runs both gates
      there, pointing to `RUNNER.md` for why. Verify: the pathspec returns the
      live folder for `171-workflow-rules` and the archived one for `op-clock`
      on this tree.
- [x] 6.4 `RUNNER.md`'s re-review brief: if `findings/` is gone, the reviewer
      writes the file afresh under the same name in the archived folder.
- [x] 6.5 `RUNNER.md` step 1: the brief after the `spec-writer` names the
      markers it closed, and says each is reworded or removed, not kept.
- [x] 6.6 `design.md`: the reasoning for 6.1 to 6.5 is in the Decisions
      entries they belong to, and the `closer`'s Step 3 gap is in Risks.
- [x] 6.7 `openspec validate 171-workflow-rules --strict` passes.

### 7. After the `spec-writer`'s ruling (`1244ae5`)

- [x] 7.1 `closer.md` Step 1 uses the exact pathspec
      `git ls-files -- "openspec/changes/<name>/tasks.md" "openspec/changes/archive/????-??-??-<name>/tasks.md"`
      in place of 6.3's suffix glob, and stops on more than one path or none.
      Verify: the pathspec returns nothing for `clock`, only op-clock for
      `op-clock`, and only the live folder for `171-workflow-rules`.
- [x] 7.2 `RUNNER.md` step 3's brief: a re-reviewer writes the file afresh in
      the archived folder only if it has a finding.
- [x] 7.3 `RUNNER.md` "What you read": the `tasks.md` and `findings/` rows
      point at the archived folder and "Rebuild the state". "Rebuild the
      state" gets the paragraph saying the paths have moved, with the same
      pathspec. Its four commands are unchanged. Step 3 points to that
      paragraph for finding the block rather than saying it again.
- [x] 7.4 `design.md`: the pathspec entry cites what was measured and why the
      suffix glob was dropped. It says why the command appears in both
      `closer.md` and `RUNNER.md`. The `findings/` bullet points at
      `RUNNER.md` step 3.
- [x] 7.5 `openspec validate 171-workflow-rules --strict` passes.

### 8. The `closer`'s own commits, what the runner commits, and the review round's findings

The owner took `findings/security.md`'s and `findings/architecture.md`'s owner
findings into this piece and ruled on them; `proposal.md` records the rulings.
Commands written into a role file were run in a scratch repository under
`./tmp/` (git 2.55.0) before any claim about their output was written.

- [x] 8.1 `RUNNER.md` "What a runner does": "You do not rebase" becomes "never
      rebase, reset or force-push `piece/<name>`, and do not merge `main`";
      "What a runner commits" states the owner's rulings once. Verify:
      `git grep -n -F "What a runner commits" -- .claude/agents` returns the
      heading and step 3's pointer, both in `RUNNER.md`, and the red-run text
      points to "What a runner does".
- [x] 8.2 `RUNNER.md` "Dispatching": the per-agent sequence names the
      fast-forward for the `closer` and a conflict resolver; a conflicting
      cherry-pick goes back to its agent; the review round's tick conflict is
      expected. "Removing each agent's worktree" matches. Verify: the
      messages quoted (`is a merge but no -m option was given`,
      `Not possible to fast-forward`, `CONFLICT (content): Merge conflict in
      tasks.md`) are the ones the scratch repository printed.
- [x] 8.3 `RUNNER.md` step 3 lists the `closer`'s commits by whether they need
      review, and the brief for a resolution names
      `git show --remerge-diff <sha>`. Step 4 lists all four returns, the
      fast-forward comes first, and "a stale branch does not come back" holds
      only for a clean merge of `main`. Verify:
      `git grep -n -i "rebase" -- .claude/agents` returns only the runner's
      prohibition, the refused fast-forward's hint, the agent rebasing its own
      local branch, and the writer's "not to rebase".
- [x] 8.4 `closer.md`: the order list, Step 2 (merge `origin/main`, no force,
      stop on a conflict), Step 3 (the archive check, only for an archive made
      in this run; push HEAD on a re-dispatch), Step 4's `BEHIND` pointer,
      "What you never do", "Your report" and the closing paragraph. Signing
      text unchanged except the operation it names. Verify:
      `git grep -n -e "force-with-lease" -- .claude/agents` returns nothing,
      and `git diff --name-only HEAD^ HEAD -- openspec/specs/` lists two files
      on `2bda577f` and none on this tree.
- [x] 8.5 `README.md`'s branch section and `spec-writer.md`'s stage-block
      sentence point to `RUNNER.md`'s "Dispatching". Verify:
      `git diff origin/main...HEAD -- .claude/agents/spec-writer.md` shows the
      one sentence, the re-review row and its paragraph, and no other template
      row.
- [x] 8.6 Findings addressed to the `dev-writer`: `RUNNER.md` step 2 states no
      count; step 1's `NO SPEC:` command is scoped to the piece's diff and the
      brief asks the `spec-writer` to name product decisions; `RUNNER.md` and
      `design.md` say only what `closer.md` Step 1 says; `closer.md` names
      `RUNNER.md`'s section before its item number. The clean re-review trace
      and the pathspec test are deferred, with where each now lives. Verify:
      `grep -rc "^- \[ \]" openspec/changes/171-workflow-rules/findings/`
      is zero for every file.
- [x] 8.7 `design.md`: Decisions for merge-not-rebase, stop-on-conflict, the
      archive check, the fast-forward, what the runner commits, the
      conflicting cherry-pick, the `spec-writer.md` correction, the signing
      text, the scoped `NO SPEC:` command and the product-decision request;
      the stale Context line and the stale Risks entry on the `closer`'s
      rebase are replaced.
- [x] 8.8 `openspec validate 171-workflow-rules --strict` passes.

### 9. After the `spec-writer`'s ruling on the dev-writer's decisions (`f9df27f`)

Supersedes 7.2 (a re-reviewer writes only if it has a finding) and 8.3's "four
returns". Commands written into a role file were run against this tree or a
scratch repository under `./tmp/` (git 2.55.0), since deleted, before any claim
about their output was written.

- [x] 9.1 `closer.md` Step 3: a refused push stops the `closer` and it
      reports, pointing to Step 2, which now also forbids fetching and merging
      the remote piece ref. "Your report" and the closing list name a refused
      push. Verify: a branch forked from a cherry-picked copy of pushed commits
      was refused with `(non-fast-forward)` in the scratch repository.
- [x] 9.2 `closer.md` Step 1 and `RUNNER.md`'s "By then the block may have
      moved" state the condition — an earlier `closer` archived — with returns
      as examples only.
- [x] 9.3 `closer.md`: the `findings/` deletion moves to the start of Step 3
      (both the first-pass and the re-dispatch paragraph); Step 1 keeps the
      gates, the ownership paragraph and the `design.md` check; the commit
      sentence names Step 3; the order list says so. Verify: with a staged
      deletion, `git merge` refused with `Your local changes to the following
      files would be overwritten by merge` in the scratch repository.
- [x] 9.4 `closer.md` Step 1: an archived folder with no `findings/` means an
      earlier `closer` deleted it and no re-reviewer has run since.
- [x] 9.5 `RUNNER.md` step 3's brief: a clean re-reviewer appends one ticked
      verdict box naming the range; after the archive every re-reviewer
      writes its file; the verdict box is on the needs-no-review list. Verify:
      `git grep -n -e "writes no file" -e "only if it has" -- .claude/agents`
      returns nothing.
- [x] 9.6 `RUNNER.md` step 1: "A hand-back that names none has decided them
      all" is replaced; only markers covered by new spec text or whose
      behaviour changes are named as decided.
- [x] 9.7 `RUNNER.md` "Dispatching", "What a runner does" and "One piece is
      one PR", and `README.md`'s branch table row and "The exceptions are…":
      an agent whose commits are already on the remote piece ref is
      fast-forwarded to, and README points to `RUNNER.md` without a list that
      reads as complete. Verify: `git merge --ff-only` kept the pushed SHA and
      `git cherry-pick --ff` onto a non-parent HEAD made a new SHA with exit 0,
      in the scratch repository.
- [x] 9.8 `RUNNER.md` "The `closer`, and what comes back": five returns; a
      refused push goes to the owner with
      `git log --oneline --left-right --cherry-mark HEAD...origin/piece/<name>`.
      Verify: run on this tree, and in the scratch repository it printed `<`,
      `>` and a `=` pair as the role file describes.
- [x] 9.9 `design.md`: Decisions for the verdict box, the `findings/` deletion
      point, the already-pushed fast-forward (citing
      `git reflog show --date=iso piece/171-workflow-rules`) and the corrected
      product-decision sentence; the two Risks entries replaced; stale
      "Step 1 deletes" and "raised none" text updated.
      `findings/security.md`'s second box gets its landed outcome.
- [x] 9.10 `openspec validate 171-workflow-rules --strict` passes.

### 10. After the `spec-writer`'s re-review findings (`1f62afd`)

Supersedes 9.8's "five returns" and 9.7's "the `dev-writer`'s first pass".
Commands written into a role file were run against this tree or a scratch
repository under `./tmp/` (git 2.55.0), since deleted, before any claim about
their output was written.

- [x] 10.1 `closer.md` Step 3: the `openspec/specs/` check runs straight after
      the archive commit, before the push; then push refused → stop and
      report the refusal and the check's result; accepted with files → stop
      before Step 4; accepted with nothing → Step 4. "Your report" asks for
      the check's result on a refused push. The re-dispatch paragraph's skip
      is kept, pointing at the check "below". Verify: the check's code block
      sits above the push's in Step 3.
- [x] 10.2 `RUNNER.md` "The `closer`, and what comes back": no count; returns
      are examples and an unnamed one is routed by what it is; a `BLOCKED`
      entry; "An unticked box" covers stage rows; the refused-push entry
      unticks and records a round first when the report lists archive-check
      files. Verify: `git grep -n -e "things come back" -- .claude/agents`
      returns nothing.
- [x] 10.3 `RUNNER.md` step 3's brief: findings go under a heading naming the
      range; both heading and verdict box carry the range exactly as the
      brief gives it.
- [x] 10.4 `RUNNER.md` step 3's tick rule: the pre-tick
      `git grep -l -F "<range>" -- <change folder>/findings/` for every
      unskipped round the tick closes; a lane not listed is continued or
      redispatched, and the row stays unticked. "What you read" gains a row
      for it. Verify: for `c222c37..9dc235c` on this tree the command lists
      all six findings files; for an absent range it lists nothing.
- [x] 10.5 `RUNNER.md` "Dispatching": the four steps for an agent whose tree
      holds uncommitted changes, with no signing flag. Verify: the whole
      sequence ran in the scratch repository, including a `tasks.md` conflict
      mid-rebase, and the re-applied patch matched the saved one.
- [x] 10.6 `dev-writer.md`: "so concurrent agents' cherry-picks do not
      conflict" replaced by a pointer to `spec-writer.md`'s stage-block
      paragraph. Verify: `git diff origin/main...HEAD --
      .claude/agents/dev-writer.md` shows that clause and nothing else.
- [x] 10.7 Every `dev-writer` pass is fast-forwarded to: `RUNNER.md`'s
      per-agent sequence, already-pushed paragraph, "How many at once", the
      red-run fixer, "One piece is one PR" (two sentences) and the sample
      brief; `README.md`'s branch section says the `dev-writer` pushes on
      every pass and the `closer` after merging `main` and in Step 3. Verify:
      `git grep -n -e "first pass" -- .claude/agents/RUNNER.md` returns only
      the PR-opening sentence.
- [x] 10.8 `design.md`: the archive-check entry says why the check runs before
      the push; the verdict-box entry gains the heading and the pre-tick
      check; new entries for returns without a count and for the mutating
      reviewer's rebase with its three rejected alternatives; the
      `spec-writer.md` correction entry covers `dev-writer.md`; "the one
      instance today" names every pass; Risks' standing-test entry points at
      the new Out of scope entry, and a new Risk records that only the runner
      runs the pre-tick check.
- [x] 10.9 PR #174's body: the follow-ups list the wider standing test, the
      `closer`-side re-review check, the reviewer role files on rebasing with
      mutations, and a second reader for rejected findings; the
      one-file-per-row follow-up lists every role file that ticks a row.
- [x] 10.10 The five `dev-writer` boxes in `findings/correctness.md` and
      `findings/readability.md` flipped with outcomes. Verify:
      `grep -rc "^- \[ \]" openspec/changes/171-workflow-rules/findings/`
      is zero for every file.
- [x] 10.11 `openspec validate 171-workflow-rules --strict` passes.

### 11. After the `spec-writer`'s ruling on `8cef5d1` (`6354aa8`)

Wording only: each edit changes the words naming the route an agent's commits
take, and adds no rule.

- [x] 11.1 `RUNNER.md`, closing "No `worktree-agent-<id>` ever appears on the
      remote": the runner brings each agent branch onto its HEAD, not
      fast-forwards to it.
- [x] 11.2 `README.md`: "One writer at a time", the harness-named branch
      sentence in the branch section, the sample `dev-writer` findings brief,
      "What the agent's own branch means for getting work back" and "Who
      removes the agent's tree" say "brought onto" or "bringing onto" the
      piece. The reviewer's tree "once its work is cherry-picked", "a commit
      cherry-picked rather than merged", "never by cherry-pick" and
      "concurrent cherry-picks never" are unchanged. Verify:
      `git grep -n -F "cherry-pick" -- .claude/agents/README.md` returns only
      those four and the branch table row.
- [x] 11.3 `design.md`: the Goals line on `README.md` and the fast-forward
      entry name these passages.
- [x] 11.4 `openspec validate 171-workflow-rules --strict` passes.

### 12. After the `spec-writer`'s callback on re-review `9dc235c..34fd428` (`23aaac3`)

Supersedes 10.3 and 10.4's bare-range heading and check. Every command whose
output a file now describes was run on this tree or in a scratch repository
under `./tmp/` (git 2.55.0), since deleted, before the claim was written.

- [x] 12.1 `RUNNER.md` step 3's brief: the heading and the verdict box are
      given whole, as ``## Re-review round <n> `<range>` `` and
      ``- [x] **re-review round <n> `<range>`: no findings** — read <what>; clean``,
      with ``round <n> `<range>` `` copied from the runner's line. Verify:
      `git grep -n -F "a1b2c3d" -- .claude/agents/RUNNER.md` returns only the
      two sample round lines, not a heading or box example.
- [x] 12.2 `RUNNER.md` "Record the call": each line starts
      ``round <n> `<range>` ``, numbered in the order written; a lane
      dispatched again over the same range gets its own line, a `SendMessage`
      continuation none; the sample block shows a re-run line.
- [x] 12.3 `RUNNER.md` step 3's tick paragraph and "What you read": the
      two-pattern command in single quotes; the earlier round's check skips
      lanes a later round re-ran over the same range; a fresh dispatch for a
      missing lane gets its own line; run once every lane's findings commit
      is on HEAD. Verify: on this tree the un-numbered forms list all six
      files for rounds 1 and 2, the numbered forms for round 2 list nothing,
      and the numbered forms with placeholders list `security.md`.
- [x] 12.4 `RUNNER.md` "Dispatching": the review-round conflict paragraph
      moved above the dirty-tree procedure; the procedure closes with what
      untracked files and an empty patch mean. Verify: `git apply` on an
      empty patch exited 128 with `error: No valid patches in input` in the
      scratch repository.
- [x] 12.5 `closer.md`: Step 3's "a check run after the push"; the closing
      paragraph says every stop ends the turn, with its list as examples
      including `BLOCKED`.
- [x] 12.6 `dev-writer.md`: the stage-row pointer reworded; the four route
      sentences say "brings … onto" or "bringing … onto". Verify:
      `git grep -n -i "cherry-pick" -- .claude/agents/dev-writer.md` returns
      line 158 only.
- [x] 12.7 `design.md`: the heading-and-check Decision rewritten for the
      round-numbered forms (why not the bare range, why the number, rejected
      alternatives, the residual, the transition for rounds 1 and 2, the
      measurements); the route criterion and the rejected earlier reason in
      the fast-forward entry; `README.md`'s kept stage-block sentences; the
      untracked-file case; the `NO SPEC:` entry's route words; the
      standing-test Risk reclassified; the #132 Risk pointing at the
      proposal's overlap section.
- [x] 12.8 PR #174's body: the follow-ups match the standing-test entry and
      the two self-contained owner follow-ups.
- [x] 12.9 The nine `dev-writer` boxes of re-review `9dc235c..34fd428`
      flipped with outcomes. Verify:
      `grep -rc "^- \[ \]" openspec/changes/171-workflow-rules/findings/`
      is zero for every file.
- [x] 12.10 Trial merge against `origin/piece/review-tiering` re-run after
      the `dev-writer.md` edits.
- [x] 12.11 `openspec validate 171-workflow-rules --strict` passes.

### 13. After the `spec-writer`'s line-rule ruling (`47f6008`)

Supersedes 12.2's "a `SendMessage` continuation none". Wording follows
`proposal.md`'s line-rule bullet under the re-review row.

- [x] 13.1 `RUNNER.md` "Record the call": a lane run again over a range it
      already had gets its own line, fresh or continued; a continuation gets
      none only when it adds no review to a committed record (finishing an
      unrecorded round, committing, rebasing); a continued run needs the
      number because the rejected record is committed under the old one.
      Verify: `git grep -n -e "no line" -e "not a new dispatch" --
      .claude/agents` returns only `RUNNER.md`'s new sentence.
- [x] 13.2 `RUNNER.md`'s pre-tick check: "the round ran" and "ran again" where
      it said "dispatched". Verify: `git grep -n -F "dispatched again" --
      .claude/agents/RUNNER.md` returns nothing.
- [x] 13.3 `design.md`: the stage-block summary, the numbered-lines bullet and
      the check say "run again"/"ran"; "Why the number" gains why a continued
      run needs a number and why the rebase case is named; "Rejected" gains
      forbidding the continuation of a rejected run.
- [x] 13.4 `openspec validate 171-workflow-rules --strict` passes.

### 14. After the `spec-writer`'s stale-number reclassification (`e2f2729`)

Supersedes 12.7's standing-test Risk wording. Follows `proposal.md`'s
standing-test entry as `e2f2729` left it; no `.claude/` file changes.

- [x] 14.1 `design.md`'s "The git commands the role files name have no
      standing test" Risk: the fail-closed pre-tick case narrowed to "a wrong
      character that no committed record carries"; the fail-open list gains
      the stale-number case (a re-run's check typed with the earlier line's
      number matches the earlier run's record over the same range when that
      run left one for the lane), with its measurement and the round 3
      readability refinement; "the three fail-open cases first" now four.
      Verify: `git grep -n -i "three fail" --
      openspec/changes/171-workflow-rules/design.md` returns nothing.
- [x] 14.2 `openspec validate 171-workflow-rules --strict` passes.

### 15. After the `spec-writer`'s copy rule (`16958b3`)

Supersedes 14.1's stale-number entry in the fail-open list. Follows
`proposal.md`'s pre-tick bullet and standing-test entry as `16958b3` left
them.

- [x] 15.1 `RUNNER.md`'s tick paragraph: the check's ``round <n> `<range>` ``
      is copied from that round's own line as the brief's was, not typed; one
      sentence after the command's description says a number typed from
      memory matches the rejected run's record whenever that run left one.
      "What you read" and the brief bullet unchanged. Verify:
      `git grep -n -F "copied from that round's own line" -- .claude/agents`
      returns the tick paragraph only.
- [x] 15.2 `design.md`'s pre-tick Decision: the check bullet states the copy
      rule; a new entry records it with two rejected alternatives (typed,
      measured at `c3d697e9`; a mechanical guard, which is the deferred
      `closer`-side reader); "What it still cannot see" gains the wrong-line
      case with a pointer to Risks. Verify: at `c3d697e9` the round 3 forms
      over `34fd428..dc1390a` list five files, the round 4 forms
      `readability.md` alone, and the round 5 forms over `dc1390a..d1c8726`
      include `spec-test.md` and `design-review.md`.
- [x] 15.3 `design.md` Risks: the stale-number case leaves the fail-open list
      for its own paragraph (a runtime input, answered by the copy rule); the
      follow-up's count is three again; "Only the runner runs the pre-tick
      check" covers forms copied from the wrong line. Verify:
      `git grep -n -F "four fail-open" --
      openspec/changes/171-workflow-rules/design.md` returns nothing.
- [x] 15.4 PR #174's body: step 3 says the check's forms are copied from the
      round's line; the `closer`-side follow-up names a check run with forms
      copied from the wrong line.
- [x] 15.5 `findings/design-review.md`'s round 6 box flipped with its outcome.
- [x] 15.6 `openspec validate 171-workflow-rules --strict` passes.

### 16. After the `spec-writer`'s repeated-number residual (`0fbebed`)

Follows `proposal.md`'s pre-tick bullet, standing-test entry and
`closer`-side follow-up as `0fbebed` left them; `design.md` and the PR body
only. No `.claude/` file changes, and no findings box is touched.

- [x] 16.1 `design.md`'s "What it still cannot see": the wrong number on the
      line itself follows the wrong-line case as a separate residual, with its
      measurement; "No test of the role files can see" covers both cases.
      Verify, at `291499c7`: the forms from ``round 3 `34fd428..dc1390a` ``
      list `architecture.md`, `correctness.md`, `design-review.md`,
      `security.md` and `spec-test.md`; the round lines under the re-review
      row carry 1 to 8, once each.
- [x] 16.2 `design.md` Risks: the stale-number paragraph names two residuals,
      the wrong line and a repeated or stale number on the line, both to the
      next Risk; "Only the runner runs the pre-tick check" adds a line whose
      number an earlier line already carries, seen by the deferred
      `closer`-side check only through unique and consecutive numbers.
- [x] 16.3 `design.md`'s "What else was considered": the residual list gains
      the repeated number.
- [x] 16.4 `design.md`'s copy-rule Decision: a Rejected entry for the
      `RUNNER.md` "one more than the highest" clause.
- [x] 16.5 PR #174's body: the `closer`-side follow-up names the
      unique-and-consecutive number criterion.
- [x] 16.6 `openspec validate 171-workflow-rules --strict` passes.

### 17. After the `spec-writer`'s number check (`328c192`)

Follows `proposal.md`'s two pre-tick bullets, standing-test entry and
`closer`-side follow-up as `328c192` left them, and the list for the
`dev-writer` in `findings/security.md`'s re-review round 9 box. No findings
box is touched. The number check was run on this tree, and on copies of the
stage block under `./tmp/` (since deleted), before any claim about its output
was written.

- [x] 17.1 `RUNNER.md`'s tick paragraph: the number check, its command in a
      code block, ahead of the forms check, with its rule (a repeat or skip
      means no tick, the line is put right, a lane briefed from a renumbered
      line runs again, an empty listing means a mistyped command, only lines
      directly under the row count) and one sentence of reason. "Record the
      call" unchanged. Verify: `git grep -n -F "number check" --
      .claude/agents` returns the tick paragraph only.
- [x] 17.2 `RUNNER.md` "What you read": a row for the number check over
      `tasks.md`.
- [x] 17.3 `design.md`: a Decision for the number check beside the copy
      rule, with its measurements and the round-8 routing as a rejected
      alternative; the "one more than the highest" rejection argued from
      "the next number" in "Record the call"; "What it still cannot see"
      split into the wrong-line case (a second reader only), the wrong number
      (the number check sees it) and a re-run given no line (neither check
      sees it); "What else was considered"'s residual. Verify:
      `git grep -n -e "adds no check" -e "repeating an earlier line" --
      openspec/changes/171-workflow-rules/design.md` returns nothing.
- [x] 17.4 `design.md` Risks: the standing-test entry lists the number
      check's command and its fail-closed case, and its residuals match
      `proposal.md`'s; "Only the runner runs the pre-tick checks" covers
      both checks.
- [x] 17.5 PR #174's body: step 3 names the number check first; the
      standing-test follow-up and the `closer`-side follow-up match
      `proposal.md`.
- [x] 17.6 `openspec validate 171-workflow-rules --strict` passes.

### 18. After the `spec-writer`'s unique-numbers rule (`57f4f76`)

Supersedes 17.1's number-check rule (a skip is no longer a fault, and a line
is never lowered) and 16.4's rejection of "one more than the highest". Follows
`proposal.md` as `57f4f76` left it, and the list for the `dev-writer` under
`findings/correctness.md`'s re-review round 10 first box. The number check was
run on this tree, and on scratch copies of the stage block and `findings/`
under `./tmp/` (since deleted), before any claim about its output was written.

- [x] 18.1 `RUNNER.md` "What you read": the number-check row carries the
      three-pattern command and says what it is for.
- [x] 18.2 `RUNNER.md` "Record the call": one line per round, six spaces,
      nothing else between the rows; a new line's number is one more than
      the highest under the row; the row is never struck, and a piece where
      nothing landed gets a skipped first line before the tick; a re-run gets
      a new number by the same rule; "the forms check below".
- [x] 18.3 `RUNNER.md` tick paragraph: the new command, what it prints, and
      the three conditions (every line between the rows listed, at least one
      line, no repeat) replacing the consecutive-numbers rule, the two-case
      repair and the empty-listing sentence; the upward repair and "never
      lower a number" with its reason. Verify: at `57f4f763` the command
      prints `tasks.md:24`, `:25-34` and `:35`; with seven spaces in the
      pattern it prints `:24` and `:35` alone.
- [x] 18.4 `RUNNER.md` forms check: the exception is a lane a line below it
      in the row ran again, since after a repair a number does not say where
      a line stands.
- [x] 18.5 `design.md`: the stage-block summary and the numbered-lines bullet
      take the new rule; "neighbouring rounds"; the "one more than the
      highest" rejection folded into the number-check Decision as adopted;
      that Decision gains the command, the three conditions, why the numbers
      need only be unique, why the rows are in the listing, the measurements
      and four new rejected alternatives; "What it still cannot see" gains a
      removed line and a lowered number; both Risks entries follow.
- [x] 18.6 PR #174's body: step 3, the `design.md` summary, the
      standing-test follow-up and the `closer`-side follow-up follow the
      same rule.
- [x] 18.7 `findings/correctness.md`'s round 10 `dev-writer` box flipped with
      its outcome.
- [x] 18.8 The sweep in that box's item 13 returns nothing.
- [x] 18.9 `openspec validate 171-workflow-rules --strict` passes.
