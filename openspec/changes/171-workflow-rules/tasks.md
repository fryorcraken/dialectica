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
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
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
