# Tasks

## Stages

- [x] ~~spec — `spec-writer`~~ — **no spec delta, struck rather than deleted.**
      A CI gate asserts nothing about module behaviour, so there is no
      capability to contract and no test in any suite that could cover such a
      requirement; the change declares `skip_specs: true` in its
      `.openspec.yaml`. The row stays visible because a missing row reads as an
      oversight and the next reader cannot tell which. `spec-writer` did run,
      and owns `proposal.md`, this block and that marker file — see
      proposal.md's Capabilities section for why a capability would be wrong
      rather than merely absent.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] CI green, PR merged — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`

## Implementation

## 1. The check itself

- [x] 1.1 Write `.github/scripts/check-claimed-deletions.sh`, taking base ref,
      head ref and a PR-body file, using the three-dot `base...head` range.
      Verified by `test-check-claimed-deletions.sh` tests 1-3: an unclaimed
      deletion exits 1 naming the path, a claimed one exits 0, a branch
      deleting nothing exits 0.
- [x] 1.2 Guard the shallow-clone case so it fails rather than passing.
      Verified by tests 5, 6 and 10 against a real `--depth 1` clone; test 6
      is the one that matters, and removing the guard makes it go red while
      test 5 still passes (measured — see design.md, Decisions §2).
- [x] 1.3 Guard unresolvable refs and a missing body file separately from the
      shallow case, so the message names the cause. Verified by tests 7 and 13,
      including that a missing ref is NOT reported as a shallow-clone problem.
- [x] 1.4 Parse `Deletes:` claims anchored to line start, allowing a Markdown
      list marker, and compare paths whole-line. Verified by tests 8, 9, 14, 15
      — test 15 fails under `grep -F` without `-x`.

## 2. Wiring it into CI

- [x] 2.1 Add the step to the existing `Lint` job, scoped with
      `if: github.event_name == 'pull_request'`. Verified by parsing the YAML:
      the lint job has 11 steps, step 1 carries the `if` and the two `env` keys.
- [x] 2.2 Set `fetch-depth: 0` on the `lint` checkout. Verified by the same
      parse; without it the gate's own shallow guard fires, which is the
      intended safety net rather than the intended state.
- [x] 2.3 Pass the PR body through a file written from `env:`, never
      interpolated into the shell. Verified by reading the step: `PR_BODY`
      reaches the script only as `$RUNNER_TEMP/pr-body.txt`.
- [x] 2.4 Pass the base as `github.event.pull_request.base.sha` with `HEAD`,
      and test that exact form. Verified by test 16, which was added after
      noticing every other test used symbolic refs.

## 3. Proving it can fail

- [x] 3.1 Run the suite against a deliberately two-dot implementation and
      confirm test 4 goes red. Done — the first version of test 4 stayed green
      under it and was rewritten; the mechanism is recorded in the test's own
      comment.
- [x] 3.2 Run the suite with the shallow guard removed and confirm test 6 goes
      red. Done — 17 passed, 2 failed.
- [x] 3.3 Run the suite with `grep -x` dropped and confirm test 15 goes red.
      Done — the first version of test 15 stayed green under it and was
      rewritten.

## 4. Acting on review findings

- [x] 4.0a Pin `--find-renames` and `core.quotePath=false` on the diff. Verified
      by tests 19 and 20; reverting either flag alone turns exactly its own test
      red, measured.
- [x] 4.0b Strip fenced code blocks before extracting claims, so a documented
      example is not an assertion. Verified by tests 21a/b/c, the third of which
      pins that a real claim after a fence still counts.
- [x] 4.0c Pin `-F` in the claim lookup with a test. Verified by test 17, which
      deletes two files so a pass cannot be a coincidence.
- [x] 4.0d Rewrite test 18 so it reaches guard 3. The first version removed the
      root tree and exited at guard 2, leaving the `|| true` mutation green at
      33/33; it now removes a subtree and asserts guard 2 passes first. Verified
      by applying the reviewer's exact mutation and watching test 18 go red.
- [x] 4.0e Run the suite in `Lint`, reversing the earlier decision. Verified by
      parsing the YAML (step 2, no `if:`) and by running the suite with `HOME`
      and both `GIT_CONFIG_*` pointed at empty files, so it cannot depend on a
      developer's git identity as CI cannot.

## 4b. Acting on readability and architecture findings

- [x] 4b.1 Make every `::error::` annotation single-line and carry its own fix.
      A workflow command is newline-delimited, so the shallow message truncated
      at "so the merge base of" and `fetch-depth: 0` never reached the reader.
      Fixed structurally: `cannot_measure` folds newlines in its headline.
      Verified by tests 22-23, which assert on the first line of each `::error::`
      rather than on stdout; restoring the old message turns them red while
      tests 5 and 7 stay green.
- [x] 4b.2 Give guard 3 a `── Guard 3 ──` banner, so the name five documents use
      resolves in the source. Verified by `grep -n "Guard 3"`.
- [x] 4b.3 One `mktemp -d` and one trap, replacing two hand-synced trap lists.
- [x] 4b.4 Rename `uncommented_body` to `unfenced_body` — nothing here strips a
      comment — and split the merged comment block into Step 1 (fences) and
      Step 2 (claims), each above its own code.
- [x] 4b.5 Document the cwd as the fourth input in the usage block, with the
      argument for documenting rather than parameterising and the trigger to
      revisit.
- [x] 4b.6 Retitle design.md §2 to three guards and fold §11's guard-3 content
      into it; retitle §7; renumber §8b to §9 and shift 9/10/11 up.
- [x] 4b.7 Remove the stale `5,034` rather than updating it — the sentence is
      about ambiguity, so no figure was load-bearing there.
- [x] 4b.8 Replace proposal.md's pruned-branch verification command with a loop
      over whatever `origin/piece/*` exists. Verified by running it.

## 4c. The second blind spot

- [x] 4c.1 Record that a merge silently reverting content is invisible to this
      gate, beside the `storage_dir` case. Reconstructed the shape rather than
      taking it on report: 254 lines reverted, file still present,
      `--diff-filter=D` empty, gate reports `ok: 0 deleted path(s)` and exits 0.
      Also confirmed `merge-base --is-ancestor` exits 0 on the reverted commit,
      so reachability is no evidence the content survived.
- [x] 4c.2 Restate the scope as one property — *the gate catches a file
      vanishing, not work vanishing* — since both known blind spots are that,
      and the next will be a third shape of it. In `proposal.md` and repeated in
      the script header, which already carried the limits.
- [x] 4c.3 Record the generalisation in design.md §13: check the base you commit
      against is still the base you computed against, which covers a rebase
      `--continue` and a cherry-pick as well as a merge. **No guard added** —
      deliberately, with the argument for why in the same section.

## 5. The documentation edits

- [x] 4.1 Add the closer-authoring rule to `closer.md`'s "What you never do",
      on the **dispatched-or-discretionary** axis: the archive commit is the
      closer's job and disqualifies nothing, discretionary work disqualifies it
      from approving that PR, and the report names which commits are its own
      discretionary work. The first draft was on an authored-or-not axis and
      would have deadlocked the role on its own archive commit; corrected by the
      owner before it shipped, and `proposal.md` carries why.
- [x] 4.2 Add the zero-delta pointer to `closer.md` step 5, beside the existing
      "read the page in full" instruction.
- [x] 4.3 Add the full zero-delta note to `docs/OPENSPEC-ARCHIVE.md`. Its three
      claims re-verified rather than inherited: `deltaCount: 0` and
      `validate --strict` passing without `--skip-specs`, the flag's existence
      in `archive --help`, and `core-e2e` archiving inside squash merge
      `b85111d`.
