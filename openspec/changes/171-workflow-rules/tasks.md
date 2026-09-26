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
