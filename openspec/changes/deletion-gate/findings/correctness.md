# Correctness findings — `deletion-gate`

Reviewed at `94ef230` in `.claude/worktrees/piece-deletion-gate`. The suite runs
green as committed (23 passed, 0 failed). Every entry below was produced by
running the script or mutating it, not by reading it; each mutation was applied
with `Edit` (which errors on a missing string), so every mutation provably
landed before the suite was believed.

## Defects

- [ ] **`tester`** — `.github/scripts/check-claimed-deletions.sh:164` — the `-F`
      in `grep -qxF` is load-bearing and no test covers it
      **Scenario:** replace `grep -qxF -- "$path" "$claimed_list"` with
      `grep -qx -- "$path" "$claimed_list"` — the deleted path becomes a basic
      regular expression instead of a literal. On a branch deleting `src/lib.rs`
      and `src/store.rs`, a body reading only `Deletes: src/lib?rs` is then
      accepted as claiming `src/lib.rs` (`src/store.rs` stays unclaimed, which
      proves the match was the regex one and not a coincidence). A PR body can
      thereby acknowledge a path that does not exist while satisfying the gate
      for one that does.
      **Measured:** 23 of 23 tests pass under this mutation. The script's own
      comment at line 162 states the reason `-F` is there ("a path containing
      `.` or `[` would otherwise match things it should not"); nothing asserts
      it. This is the same defect family as the `-x` case the author already
      corrected in test 15 — test 15 pins `-x` and leaves `-F` unpinned.
      **Severity:** high — it is a silent pass on a genuine deletion, the exact
      failure direction the piece exists to prevent.

- [ ] **`tester`** — `.github/scripts/check-claimed-deletions.sh:126-130` — the
      third guard (a failing `git diff`) is never exercised, and can be replaced
      by the idiom design.md names as dangerous with the suite still green
      **Scenario:** replace the whole `if ! git diff ... ; then cannot_measure
      ... fi` block with
      `git diff --diff-filter=D --name-only "$base_ref...$head_ref" >
      "$deleted_list" 2>/dev/null || true`. That is shape three from design.md
      §2's own table ("`set -eu`, `|| true` on the substitution → exit 0, no
      deletions"), and it converts any future `git diff` failure into "0 deleted
      paths, all claimed" — a pass having measured nothing.
      **Measured:** 23 of 23 tests pass under this mutation. Guards 1 and 2
      intercept every case the suite constructs, so no fixture ever reaches this
      branch. The piece's central argument is that each guard must be shown to
      earn its place (§2 does exactly that for guards 1 and 2, with test 6 named
      as the one holding guard 1 up); guard 3 has no such test.
      **Severity:** high — an untested guard against the precise idiom the
      design identifies as the silent-pass mechanism.

- [ ] **`dev-writer`** — `.github/scripts/check-claimed-deletions.sh:126` — the
      verdict depends on `diff.renames`, a git config the script does not pin
      **Scenario:** a branch whose only change is `git mv big.txt moved.txt`.
      With git's default rename detection the gate reports `0 deleted path(s)`
      and exits 0. Run `git config diff.renames false` in the same clone and the
      same invocation reports `big.txt` as an unclaimed deletion and exits 1.
      Nothing in the script fixes which behaviour applies — `diff.renames` can
      be set in repo, global or system config, or via `GIT_CONFIG_COUNT`/
      `GIT_CONFIG_KEY_0`, none of which the workflow controls.
      **Measured:** both verdicts reproduced in one clone, differing only by
      that config line. design.md §Risks states the rename trade-off as a
      property of `--diff-filter=D` ("`--diff-filter=D` does not include `R`");
      it is in fact a property of rename *detection* being on, which is a
      default and not a guarantee. Pass `--find-renames` (or `--no-renames`)
      explicitly so the documented behaviour is the one that runs.
      **Severity:** medium — on the GitHub runner today the default holds, so
      this is not a live silent pass; it is a gate whose answer a developer's
      config changes, and the design records the wrong reason for the behaviour.

- [ ] **`tester`** — `.github/scripts/tests/test-check-claimed-deletions.sh:224`
      (test 8) — the anchoring test only covers the same-line case, and a claim
      inside a fenced code block counts as a real claim
      **Scenario:** a PR body whose three lines are a triple-backtick fence
      open, then `Deletes: sub/doomed.txt`, then a fence close — i.e. the
      convention quoted in a Markdown code block — satisfies the gate for a
      genuine deletion of `sub/doomed.txt`. Exit 0, "all claimed in the PR body".
      Test 8 constructs the mention *inline within a sentence*, which the
      leading-anchor regex rejects for a different reason (the line does not
      start with `Deletes:`), so it passes without covering the case design.md
      §4 actually describes: *"a PR body that is prose that may quote this very
      check"*. A body documenting the convention — which any PR introducing or
      amending this gate would naturally contain — acknowledges its example path.
      **Measured:** reproduced against the standard fixture; blockquote (`>`),
      bold (`**Deletes:**`), numbered-list and HTML-comment forms all correctly
      fail, so the fence is the specific gap rather than a general looseness.
      **Severity:** medium — it needs a body shaped a particular way, but that
      shape is the one the design singles out as the hazard.

## Refutations and confirmations of the author's four flagged weaknesses

- [ ] **`dev-writer`** — `.github/scripts/tests/test-check-claimed-deletions.sh:269`
      (test 12) — flagged weakness 1 is **refuted in the author's stated
      direction and replaced by a real usability defect**: a non-ASCII deletion
      cannot be claimed at all
      **Scenario:** the author believed test 12's assumption (that
      `git diff --name-only` prints such paths bare) would fail in the *passing*
      direction for a non-ASCII path. It does not. With `core.quotePath` at its
      default, a deleted `café.txt` prints as `"caf\303\251.txt"` — quoted, with
      literal backslash escapes — so the gate **fails** (exit 1), which is the
      safe direction. The defect is the other side: a correct PR that genuinely
      means to delete `café.txt` and writes `Deletes: café.txt` is **rejected**,
      and the note "the body claims 'Deletes: café.txt', but the diff does not
      delete that path" tells the author their correct claim was wrong. The
      only body that passes is `Deletes: "caf\303\251.txt"`, which the failure
      message does print — so it is recoverable, but only by copying a form no
      author would write. `-c core.quotePath=false` on the diff makes the
      printed path the real one.
      **Measured:** reproduced end to end; `git -c core.quotePath=false diff
      --diff-filter=D --name-only` prints `café.txt` where the default prints
      `"caf\303\251.txt"`. A filename containing a newline is quoted the same
      way and therefore also fails safe rather than splitting the read loop.
      **Severity:** low — no repo path today is non-ASCII, and the failure is in
      the safe direction; but the design's claim about test 12 is wrong in a way
      worth correcting, since the recorded reasoning is what the next reader
      trusts.

## What was clean

**The shallow-clone argument is exactly as the author states it, and I could not
break it.** Disabling guard 1 (`if false && [ "$(git rev-parse
--is-shallow-repository ...)" = "true" ]`) leaves test 5's exit-status assertion
green — guard 2 catches it, because `origin/main` and `origin/feature` have no
common ancestor at depth 1 — and turns exactly two assertions red: test 5's
"shallow failure names fetch-depth", and test 6's exit status, which flips from
1 to `ok: 0 deleted path(s) ... all claimed in the PR body`. That is the
measured silent pass, and test 6 is the only test that fails on it. The claim
that removing either guard leaves a real hole is confirmed.

**The three-dot range is correctly tested.** Mutating `...` to `..` turns five
assertions red including test 4's dedicated two-dot-leak check, which reports
`arrived-later.txt` — the file main gained after the fork. The author's
correction of that fixture (§7) is sound: the fixture discriminates.

**The workflow's handling of the PR body is correct and is the only untrusted
input in the file.** `PR_BODY` reaches the script through `env:` and a file,
never through `${{ }}` inside a `run:` block. The only other interpolations in
`ci.yml` are `github.ref` (line 26), `runner.os` and `env.LGS_VERSION` (1045,
1052), none attacker-controlled. `printf '%s' "$PR_BODY"` is quoted, so a body
containing `$(...)`, backticks, `;` or newlines is written verbatim. I found no
shell injection.

**The `pull_request` merge-ref case works, and better than the design claims.**
`actions/checkout` on a `pull_request` event checks out `refs/pull/N/merge`, a
merge of the head into the base, so `BASE_SHA...HEAD` has the base tip as its
own merge base and the three-dot form degenerates to two-dot — which is
*correct* here, because it reports what the merged result deletes. Built the
real fixture (base tip + `refs/pull/1/merge`, main having gained a file after
the fork): the branch's deletion is caught, the file main added is not blamed.
Detached HEAD and a raw base SHA both resolve.

**Claim-parsing edge cases behave.** Trailing whitespace, a tab separator and a
Markdown bullet are accepted; blockquote, bold, lowercase `deletes:`, an HTML
comment, a nested double bullet, a numbered list and a bare `Deletes:` with no
path are all correctly rejected. An empty `PR_BODY` and a body of the literal
string `null` both fail on an unclaimed deletion. CRLF trimming works. A path
whose name ends in a space cannot be claimed (the trim eats it) — pathological
enough that I did not open a box, but it is a case where the failure message
prints a claim that can never satisfy the gate.

**On flagged weaknesses 2, 3 and 4.** §5's `NO SPEC:` call — a stale claim being
a note rather than a failure — is the right one and is reasoned in the right
place; failing there would fire during ordinary iteration and gives an attacker
nothing. Weakness 3 (tests not run by CI) is not itself a correctness defect,
but it is what makes the two surviving mutations above durable: with no gate on
the gate, a future edit that drops `-F` or collapses guard 3 into `|| true`
ships green. Weakness 4 I could not check — see below.

## What I could not check

The live GitHub Actions behaviour: whether `$RUNNER_TEMP` is writable at that
point in the job, whether `git fetch --no-tags origin "$BASE_SHA"` succeeds
against GitHub's server (it needs `uploadpack.allowReachableSHA1InWant`, which
GitHub sets, but I verified this against no live run), and the exact shape
`github.event.pull_request.body` takes for a PR with no description. All three
need a real Actions job; every local emulation I built agrees with the author's
expectations, and each failure mode I could construct aborts the step loudly
under `set -eu` rather than passing.
