# Tasks — workflow rules (#171, #170, #169, #133)

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change edits agent
      instructions under `.claude/agents/` only and alters no behaviour of
      dialectica, so no requirement in `openspec/specs/` changes. Declared as
      `skip_specs: true` alongside `schema:` in `.openspec.yaml`. The
      `spec-writer`'s work here is `proposal.md` and this block.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — prose-only change to agent instructions. No CI
      job, script or test reads `.claude/agents/`, so there is no executable
      behaviour to assert. A test pinning the new wording would fail when the
      wording changed, not when the rule was wrong. The check that can see
      this change is the six reviewers reading the prose.
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] re-review: every commit after the review round — runner
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
