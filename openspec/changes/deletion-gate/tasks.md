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
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
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
