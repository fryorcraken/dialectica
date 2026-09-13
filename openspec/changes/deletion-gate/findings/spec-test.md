# Spec-test findings — `deletion-gate`

Reviewed at `fe86a43` in `.claude/worktrees/review-deletion-gate-spec`, a
worktree of my own so no mutation of mine could be mistaken for another
reviewer's. This piece is `skip_specs: true`, so `proposal.md` and `design.md`
were read as the contract and `.github/scripts/tests/test-check-claimed-deletions.sh`
as the tests. The suite runs green as committed: **39 passed, 0 failed.**

**Where I had to read the implementation.** Part 2 requires mutating the script,
so I read `check-claimed-deletions.sh` in full. Everything below was produced by
running the script or mutating it, never by reading it — and every mutation was
applied with `Edit`, which errors on a missing or non-unique string, so each one
provably landed before the suite was believed. The tree was restored after every
mutation and `git status` is clean; the final suite run above is the post-restore
run.

## The mutations I ran, and what each measured

| # | mutation | result |
|---|---|---|
| 1 | `grep -qxF` → `grep -qx` (drop `-F`) | **38/39** — caught |
| 2 | guard 3 → `... \| \| true` | **37/39** — caught |
| 3 | `base...head` → `base..head` | **33/39** — caught |
| 4 | guard 1 disabled (`if false && ...`) | **35/39** — caught |
| 5 | guard 2a (`rev-parse --verify` loop) removed | **39/39 — SURVIVED** |
| 6 | guard 2b (merge-base emptiness check) neutered | **39/39 — SURVIVED** |
| 7 | guards 2a **and** 2b both removed | **39/39 — SURVIVED** |
| 8 | claim anchor removed (`^[[:space:]]*[-*+]...` → `^.*`) | **39/39 — SURVIVED** |
| 9 | `[ -f "$body_file" ]` guard removed | **39/39 — SURVIVED** |
| 10 | `cannot_measure` newline fold removed **and** headline wrapped | **37/39** — caught |
| 11 | `awk` fence filter → `cat` | **37/39** — caught |
| 12 | `-c core.quotePath=false --find-renames` removed | **37/39** — caught |
| 13 | `annotation_of` helper loosened to match any `fetch-depth` line | **38/39** — caught |

Mutations 1 and 2 are the two the earlier round reported as surviving. **Both are
genuinely closed** — confirmed, not taken on report. Mutation 2 also confirms the
guard-3 fixture now reaches guard 3 rather than guard 2: the "guard 2 still
passes" pre-check stayed green while the two guard-3 assertions went red, which
is exactly the discrimination the first version of test 18 could not make.

Mutation 13 is the "measuring instrument" check. Loosening `annotation_of` toward
green turns test 22's single-line assertion red, so the helper is not freely
tunable. That is the right answer and I record it because it was the thing I most
expected to find broken.

## Defects

- [ ] **`tester`** — `.github/scripts/tests/test-check-claimed-deletions.sh:224`
      (test 8) — the anchor, which is design.md §4's entire argument, is not
      pinned: the fixture discriminates on trailing prose, not on anchoring
      **Scenario:** replace the claim-extraction regex's anchor
      `s/^[[:space:]]*[-*+]\{0,1\}[[:space:]]*Deletes:...` with `s/^.*Deletes:...`,
      so `Deletes:` matches anywhere in a line. Test 8 still passes, because its
      body puts words *after* the path — `... write a line reading Deletes:
      sub/doomed.txt in the body.` — so the extracted claim becomes
      `sub/doomed.txt in the body.` and fails to match for a reason that has
      nothing to do with the anchor. The shape the anchor actually defends is a
      mid-sentence mention where the path **ends the line**. Measured against the
      standard fixture with the mutation applied: a body of `To acknowledge a
      removal, write a line reading Deletes: sub/doomed.txt` exits **0**,
      `ok: 1 deleted path(s) ... all claimed in the PR body` — a silent pass on a
      genuine deletion. A blockquote claim (`> Deletes: sub/doomed.txt`) also
      passes at exit 0 under the mutation, though `**Deletes:** path` still fails.
      **Measured:** 39 of 39 pass with the anchor removed. This is the same defect
      family the piece already catalogues three times — a fixture where two
      explanations give the same answer — and it is the fourth instance, in the
      one test whose comment claims to pin anchoring. design.md §4 is explicit
      that anchoring is what stops a body quoting this very check; nothing
      measures it.
      **Severity:** high — a silent pass on a real deletion, and the property is
      one of the two the design names as load-bearing for claim parsing. The fix
      is a fixture whose `Deletes:` mention is preceded by prose and followed by
      nothing, asserting exit 1.

- [ ] **`tester`** — `.github/scripts/tests/test-check-claimed-deletions.sh:207`
      (test 7) — guard 2 can be **deleted entirely** with the suite green; test 7
      passes on guard 3's back
      **Scenario:** remove both halves of guard 2 — the `rev-parse --verify`
      loop and the merge-base emptiness check. All 39 assertions still pass.
      `git diff` itself exits 128 on an unresolvable ref, so guard 3 catches the
      case and the exit status is unchanged; test 7's only other assertion is a
      `case` checking that `fetch-depth` is **absent** from the output, which is
      also true of guard 3's message. So test 7 pins "exit 1, and not diagnosed as
      shallow" — strictly weaker than the "missing ref diagnosed as a missing
      ref" its own `ok` line claims.
      **Measured:** removing guard 2a alone, guard 2b alone, and both together
      each leave 39/39 green. With both gone the annotation for an unresolvable
      base ref becomes `'git diff origin/no-such-branch...origin/feature' failed
      — the object store may be incomplete.` with the detail line `The merge base
      resolved to , so this is NOT the shallow-clone case and not a missing ref:
      both of those were already checked.` That is an empty merge base printed as
      a value, and a message that **actively denies the true cause** while naming
      a false one. The failure direction stays safe (exit 1), so this is a
      diagnosis defect rather than a silent pass — but design.md §2's table
      asserts guard 2 "catches what the others miss" and claims each guard earns
      its place, and for the verdict it currently does not. This is the same shape
      as the guard-3 hole the correctness reviewer found: a guard whose only test
      passes because a later guard happens to catch the same fixture.
      **Severity:** medium — no silent pass, but design.md §2 states a measured
      claim about guard 2 that the suite does not hold up, and the fix the wrong
      message sends a reader to is the wrong fix. Test 7 should assert on the
      message content (`does not resolve to a commit`), not merely on the absence
      of `fetch-depth`.

- [ ] **`tester`** — `.github/scripts/tests/test-check-claimed-deletions.sh:294`
      (test 13) — the missing-body-file guard is unpinned; test 13 passes for an
      unrelated reason, and the untested direction is a silent pass
      **Scenario:** remove `[ -f "$body_file" ] || cannot_measure ...`. All 39
      pass. Test 13 stays green because `awk` fails on the nonexistent file,
      writes an empty `$unfenced_body`, no claims are extracted, and the branch's
      genuine deletion goes unclaimed — exit 1 for a reason that has nothing to do
      with the guard. Test 13's fixture therefore cannot tell "the body file is
      missing" from "the body claims nothing", which is **precisely the
      distinction the script's own comment says the guard exists to preserve**
      ("a missing file fails rather than reading as 'no claims made'").
      **Measured:** with the guard removed, a branch that deletes nothing plus a
      missing body file exits **0** with `ok: 0 deleted path(s) ... all claimed in
      the PR body`, having read no body at all — a gate that measured nothing
      reporting clean, which is the exact silent pass this piece exists to
      prevent. (`awk` prints a `fatal:` line to stderr, but nothing acts on it and
      the script is not `set -e`.) The test that discriminates is the one the
      guard is for: a **clean** branch with a missing body file, asserting exit 1.
      **Severity:** medium — the live workflow writes the file unconditionally, so
      this is not reachable today; it becomes reachable the moment
      `$RUNNER_TEMP` is not writable, which is one of the three things the
      correctness review explicitly could not check against a live run.

- [ ] **`tester`** — `.github/scripts/tests/test-check-claimed-deletions.sh:623`
      (test 22) — the "annotation is a single line" assertions cannot fail for the
      reason they name
      **Scenario:** `annotation_of` extracts only lines **beginning** with
      `::error::`. A multi-line headline does not produce a second `::error::`
      line — its tail becomes ordinary log output — so `ann_lines` stays 1 no
      matter how badly the headline is truncated. Measured: with the fold removed
      and the shallow headline wrapped across two source lines (reproducing the
      exact historical defect), the annotation truncates to `deletion gate cannot
      measure this branch — the checkout is a shallow clone, so the merge base
      of` and **all three "is a single line" assertions stay green**; only the two
      content assertions (`*fetch-depth*` and `*"cannot measure"*"fetch-depth"*`)
      go red.
      **Measured:** 37/39 under that mutation, with the three single-line
      assertions among the 37. So the general property those three claim to
      pin — "a future message cannot reintroduce the defect by being written the
      natural way", design.md §12 — is carried entirely by the two content
      assertions, which are specific to the shallow message. The missing-ref and
      unclaimed-deletion single-line assertions pin nothing at all: no mutation of
      their messages can make them fail.
      **Severity:** medium — the *fix* is real and test 23 does pin folding for
      the shallow path. What is not pinned is the generalisation design.md §12
      claims, so a future `cannot_measure` caller on the missing-ref or guard-3
      path can wrap its headline and lose its tail with the suite green. Asserting
      that the raw output contains no line after an `::error::` that was meant to
      be part of it — or counting lines in `$1` before folding — would close it.

- [ ] **`closer`** — this branch is itself carrying the second blind spot the
      proposal names, **live**, and merging it as-is reverts `468e716`
      **Scenario:** `piece/deletion-gate` was cut before `468e716` (op-transport,
      #55) landed, and has no commit of its own touching `docs/PLAN.md`. Its
      three-dot diff is clean — 13 files, 3,210 insertions, **zero deletions** —
      so this gate passes its own branch, correctly. But `git diff origin/main
      piece/deletion-gate -- docs/PLAN.md` shows ~156 lines of change that the
      branch never authored: every "**Specified — see the `op-transport` spec**"
      annotation removed and the struck-through text it replaced restored, plus
      `docs/UI-BRIEF.md` (-38) and the `op-transport` spec and change folder
      (-2,055). `git merge-base --is-ancestor 468e716 piece/deletion-gate` exits
      **1**, confirming the branch predates it.
      **Measured:** this is exactly `proposal.md`'s "a merge that silently reverts
      content deletes no files" and design.md §2's own ten-branch table, which
      names `468e716` as the commit whose landing put every open branch in this
      state. The gate is blind to it by design and says so; the mitigation is
      design.md §13's working practice, which is the closer's to apply.
      **Severity:** high as a merge hazard, and **not a defect in the piece** —
      the scope statement is accurate and the blind spot is documented. It needs a
      rebase onto current `main` before merge, and re-running the branch's own
      gate afterwards. I raise it as a box because it is actionable and because
      merging without it would revert a merged PR's documentation work, which is
      the failure this piece was written about.

## On the specific questions asked

**The three-dot range is genuinely pinned, not pinned on a fixture where both
forms agree.** Mutation 3 turns six assertions red, including test 4's dedicated
`arrived-later.txt` check, which names the *mechanism* — a file `main` gained
after the fork — rather than asserting a count. The author's correction of that
fixture (design.md §7) is sound and I could not make it pass under two dots.

**The two guards against an unresolvable merge base: one half holds, one does
not.** Guard 1 is exactly as claimed — disabling it leaves test 5's exit-status
assertion green (guard 2 catches it) and flips test 6's, confirming design.md
§2's sharpest result that test 6 is the only test holding guard 1 up. I
reproduced that precisely. Guard 2 is the half that does not hold; see the box
above.

**Guard 3's fixture distinguishes the subtree case from the root-tree case.** The
`|| true` mutation goes red at two assertions while the "guard 2 still passes"
pre-check stays green, which is the discrimination the first version lacked.
Closed.

**Annotation truncation: the three annotation-reading assertions are real, but
two of the three pin nothing.** See the box above. The `fetch-depth` content
assertion and test 23 do fail when folding is removed; the line-count assertions
do not.

**The scope claim is accurate and, if anything, understated.** "Seven of eight
firings", the `storage_dir` addition and the staged-merge reversion are all
recorded honestly, and the framing does not imply more protection than the
assertions establish — `proposal.md`'s "the gate catches a file vanishing, not
work vanishing" is the correct single sentence for it. I found no overclaim.

**A fourth thing the change claims that nothing tests**, beyond the three the
author states (`$RUNNER_TEMP` writability, `git fetch --no-tags origin
"$BASE_SHA"`, the description-less PR body): **the workflow wiring itself**.
`fetch-depth: 0`, `if: github.event_name == 'pull_request'`, and the `env:`-to-file
body passing are verified by tasks.md 2.1-2.3 as "verified by parsing the YAML" —
by hand, at one moment. Nothing executable pins any of them, and `fetch-depth: 0`
is the actual fix while the script's guard 1 is only the safety net. A future edit
dropping that key ships green and the gate goes red on every PR, which by the
piece's own argument is how a gate gets disabled. I did not open a box: adding a
YAML assertion is arguably scope creep for this piece and the correctness
reviewer already flagged the adjacent live-run gap. Worth a line in the PR
description so it is a known limit rather than an assumed coverage.

## `NO SPEC:` markers

Exactly one, and it is correctly handled: test 10 / script line 336 /
design.md §5, on a claim with no matching deletion being a note rather than a
failure. It is marked in all three places, reasoned in the right one, and the
decision is the right way round — failing there would fire during ordinary
iteration. **No spec gap to report**, which is the expected answer for a
`skip_specs: true` piece; there is no capability that could carry it.

I found no unmarked ones. Every behaviour the tests pin traces to a stated claim
in `proposal.md` or `design.md`.

## PLAN.md on `origin/main`

**Clean, and there is nothing to shed.** PLAN.md contains no mention of
deletions-in-a-diff, `fetch-depth`, merge base, stale branches, squash merge,
branch protection, required checks or `mergeStateStatus`. §13 "Open questions" is
entirely protocol design and answers none of this; no section states this gate as
future intent; nothing duplicates it.

Two corrections to `proposal.md`'s claims about §10, neither worth a box:

- §10 is titled **"CI"**; "Deliberately not built" is a bolded sub-list inside
  it, with three entries (no e2e job, no `doctor` job, no `install`/`launch`
  job). The proposal's "§10's 'Deliberately not built' list gains nothing,
  because this is being built" is **true but reads as if an entry were being
  retired** — there is no entry for this gate, because §10 never contemplated it.
- The proposal says §10's claim that the `qml` job proves "nothing about
  `call()`'s JSON parsing" is now stale. I checked `ci.yml`: the `qml` job runs
  `dialectica-ui/tests/run-qml-tests.sh` at line 668 and asserts a test count at
  685, and `tst_core_call.qml` exists. So the stale half is §10's closing
  sentence *"That logic is genuinely testable and currently untested"*, not the
  clause the proposal quotes. The proposal correctly defers the fix to a change
  that touches that job; I note the imprecision only so whoever does it edits the
  right sentence.

PLAN.md §10 also records the standing instruction that settles whether this piece
owes it an entry: *"Read the workflow for which paths it walks — naming them here
would be a second copy that drifts."* On that precedent the gate's syntax,
`fetch-depth: 0` and matching rules must **not** go into PLAN.md. The change
adds nothing there, which is correct.

## What was clean

**The three-dot range, guard 1, guard 3, the fence filter, `-F`, `--find-renames`
and `core.quotePath=false` are each pinned by a test that goes red when that
exact property is removed**, and in every case the red assertion names the
mechanism rather than a count or a length. Mutations 1, 2, 3, 4, 10, 11 and 12
all landed and were all caught. That is seven of the piece's load-bearing
properties measured rather than asserted, which is a better ratio than any piece
I have reviewed here.

**Test 17's two-file construction is exactly right.** Under the `-F` mutation its
exit-status assertion stays green — `sub/store.rs` is still unclaimed, so the
gate exits 1 anyway — and only the `case` on `sub/lib.rs` catches it. The test's
own comment predicts this and that is why the second file is there. Without it
the test would pass for the wrong reason.

**The `annotation_of` helper is not tunable toward green** (mutation 13).

**Test 6 is load-bearing and its comment is accurate.** It is the only assertion
that flips when guard 1 goes, and I reproduced that exactly as design.md §2
describes.

**The workflow invocation form is tested.** Test 16 covers the raw-base-SHA +
literal `HEAD` path that CI actually runs, and it matches `ci.yml` lines 116-117.

**The suite is wired into CI** (`ci.yml` line 143-144, no `if:`), so these
properties stay measured. Given three of my five findings are tests that pass for
the wrong reason, that wiring is what makes fixing them durable.
