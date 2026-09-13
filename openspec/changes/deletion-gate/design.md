## Context

See `proposal.md` — Why, for the eight firings and the 7,101-vs-37 measurement
that motivates the three-dot range.

The design-level constraints, which the proposal states but does not resolve:

- The check must run somewhere it is already enforced. `Lint` is a required
  status check today; a new job would need a branch-protection change.
- The workflow triggers on `push` to `main` and on tags as well as
  `pull_request`. Two of those three have no PR body to read.
- `actions/checkout` defaults to `fetch-depth: 1`, and the merge-base range is
  the thing that does not work without history.
- The repo's shell conventions are set by `.github/workflows/ci.yml` and
  `dialectica-ui/tests/run-qml-tests.sh`: POSIX `sh`, `set -eu`, and a
  `::error::` line naming the file when a check fails.

## Goals / Non-Goals

**Goals:**

- A pull request whose merge-base diff deletes a file fails `Lint` unless the
  PR body claims that path.
- The check fails **loudly** when it cannot measure, and the two "cannot
  measure" causes are distinguished from each other and from "measured, clean".
- The logic is executable outside GitHub Actions, so its ability to fail is
  demonstrable rather than asserted.

**Non-Goals:**

- Judging whether a claimed deletion is *correct*. The gate checks that a path
  was acknowledged; `closer.md` step 2 remains the reader.
- The duplicate-symbol class (`proposal.md` — What it does not catch).
- Any change to the other four jobs or to branch protection.

## Decisions

### 1. A script file, not an inline `run:` block

Every other check in `Lint` is inline. This one is not, and the reason is the
job the piece was given: **prove the gate can fail.** An inline block is
reachable only by pushing a branch and opening a PR, so each of the three
demonstrations would cost a round trip through GitHub, and the shallow-clone
case could not be demonstrated at all without editing the workflow to be
deliberately wrong and pushing that.

Extracted to `.github/scripts/check-claimed-deletions.sh`, taking its base ref,
head ref and PR-body file as arguments, the same logic runs against throwaway
repositories built by `.github/scripts/tests/test-check-claimed-deletions.sh` in
under a second, including a genuinely shallow clone.

Considered and rejected: inline, with the tests driving a copy of the logic. A
test against a copy of the code is the defect family this repo already
catalogues — the copy passes while the original rots.

The cost is one more file and an indirection from the workflow. Accepted,
because the alternative is a gate whose failure path is asserted rather than
observed, which is the exact thing this gate exists to prevent elsewhere.

### 2. Two independent guards, because they catch different failures

Measured against real shallow clones (`tmp/probe/` during development;
reproduced as tests):

- `git diff A...B` in a clone too shallow to see the fork point exits **128**
  with `fatal: ...: no merge base`. It does **not** print an empty list.
- `git rev-parse --is-shallow-repository` reports `true` for exactly the depths
  where that happens, and flips to `false` once the graft reaches the fork
  point — at which point the merge base is the true one. Probed at depths 1, 3,
  10 and 25 against a 21-commit `main` with a branch forking at the root commit:
  no depth produced a *wrong* merge base, only an absent one.

So the dangerous case is not git inventing a bad answer — it is **the script
turning git's refusal into a pass.** Three realistic shapes do exactly that, all
three measured:

| shape | shallow result |
|---|---|
| `run:` block with no `set -e` | exit 0, "no deletions" |
| `set -eu`, piped into `while read` | exit 0, pipe masks git's 128 |
| `set -eu`, `\|\| true` on the substitution | exit 0, "no deletions" |

The script therefore resolves the merge base as its own step and checks
`--is-shallow-repository` **before** diffing, and neither guard is redundant:

- The **shallow** guard names the cause the fix acts on (`fetch-depth`), and
  fires even in the hypothetical where a future git resolves a graft boundary as
  a merge base.
- The **merge-base** guard catches every other way the range fails — an
  unfetched base ref, an orphan branch, a typo'd ref name — which a shallow check
  alone would report as healthy.

Alternative considered: trust `set -eu` and let git's 128 propagate. Rejected —
it produces `fatal: no merge base` and nothing else, which names neither
`fetch-depth` nor this check, and the table above is the record of how easily a
later edit converts that into a pass.

**Measured, and this is the sharpest result of the piece.** Deleting the shallow
guard leaves the headline shallow test (5) still *passing*: guard 2 catches it,
because `origin/main` and `origin/feature` have no common ancestor in a
depth-1 clone. What goes red is test 6 — the same shallow clone with
`base == head`, where a commit is trivially its own merge base, so guard 2 sees
a healthy repository and **the gate passes having measured nothing.** That is
the exact silent-pass this piece exists to prevent, and only the shallow guard
stops it. Test 6 is therefore not a variation on test 5; it is the only test
that holds guard 1 up.

### 3. `fetch-depth: 0`, not a finite depth

`fetch-depth: 2` would be enough for a branch one commit behind and wrong for
every other branch, failing the gate for a reason unrelated to deletions. A
number chosen to be "usually enough" is a gate that fires on branch age. Full
history costs seconds on a repo this size, and the shallow guard means getting
it wrong is loud rather than silent.

### 4. The claim is parsed as a whole line, anchored

A `Deletes:` claim matches only at the start of a line (leading whitespace and
an optional Markdown list marker allowed), with the path being the rest of the
line trimmed. Anchoring matters because a PR body is prose that may quote this
very check — an unanchored substring search would let *"the gate wants a
`Deletes: <path>` line"* acknowledge a path named `<path>`. The existing
adapter-derivation check in `Lint` carries the same scar, and strips comments for
the same reason.

Path comparison is exact string equality against git's output, not a prefix or
glob. `Deletes: docs/` acknowledging everything under `docs/` is a wildcard in a
field where the whole point is naming what goes.

### 5. Unclaimed deletions fail; claimed-but-absent paths do not

If the body claims `Deletes: a.txt` and the diff does not delete `a.txt`, that is
reported as a note and does **not** fail. The claim is an assertion about intent,
and a stale one — a path deleted in an earlier push then restored — is not a
defect the merge should block on. Failing on it would also make the gate fire
during ordinary iteration on a PR whose body was written first.

`NO SPEC:` — there is no spec to say which way this goes; see the test of that
name.

### 6. Scoped to `pull_request` by an `if:` on the step

`if: github.event_name == 'pull_request'` rather than a guard inside the script,
so the skip is visible in the GitHub UI as a skipped step rather than as a green
one that did nothing. A reader of a `main` run should be able to see that this
check did not run — `proposal.md` is explicit that a green `main` run is not
evidence of this gate.

The body arrives via `env:` from `github.event.pull_request.body` and is written
to a file the script reads, rather than being interpolated into the script. A PR
body is attacker-controlled text: `${{ }}` interpolation into a `run:` block is
shell injection, and this is the one step in the workflow that handles untrusted
input at all.

### 7. Three of these tests were written wrong first, and all are recorded

(The third is guard 3's, in §11 — it was found by review rather than by me, and
it is the same shape as these two.)

Both are the defect family this repo already catalogues — a fixture where two
explanations give the same answer — and both were found by mutating the script
rather than by reading the tests. Neither would have been found by a green run.

- **The two-dot test had the mechanism backwards.** It made `main` *delete* a
  file after the fork, reasoning that two-dot would blame the branch. It does
  not: `--diff-filter=D` on `base head` asks what HEAD lacks that BASE has, and
  a branch that forked earlier still *has* that file. The fixture that
  discriminates is main *ADDING* a file after the fork — the branch predates it,
  therefore lacks it, therefore two-dot scores it a deletion by the branch. That
  is also the real failure: `main` gained the agent files and three branches
  were blamed for ~700 deletions each.
- **The whole-line test had the direction backwards.** The lookup is
  `grep -qxF "$deleted_path" "$claims_file"`, so the path is the pattern and the
  claims are the haystack. Dropping `-x` lets a claim that *contains* the path
  satisfy it (`Deletes: x.txt.bak` acknowledging `x.txt`), not a shorter claim
  matching a longer path.

Recorded here rather than only in the tests because the corrected fixtures look
arbitrary without the mechanism, and the natural "simplification" is to rewrite
each back to the version that proves nothing.

A third thing the mutation runs taught: **a mutation that fails to apply is
indistinguishable from a test that fails to catch it.** Two mutations silently
did not apply (shell quoting against a regex containing the same metacharacters)
and produced a green suite that read as a test defect. The mutation harness in
`tmp/probe/` therefore asserts the file changed before believing the result.
That harness is scratch and is not committed; the tests it validated are.

### 8. The workflow's own invocation is tested

The workflow passes `github.event.pull_request.base.sha` and the literal `HEAD`,
while every test was written with symbolic refs — a different path through
`rev-parse --verify "$ref^{commit}"` and through the `...` range. Test 16 covers
the form that actually runs in CI. Reading the base from the event rather than
hardcoding `origin/main` also means a PR targeting a non-`main` base is measured
against its own base.

### 9. Two git settings are pinned, because each changes the verdict

Both were found by review, and both are defaults rather than guarantees —
settable in repo, global or system config, or through `GIT_CONFIG_COUNT`, none
of which this workflow controls.

- **`--find-renames`.** Measured: `git mv big.txt moved.txt`, same branch, same
  command — with detection on the gate reports 0 deletions and exits 0; with
  `diff.renames false` in the repo config it reports `big.txt` unclaimed and
  exits 1. A gate whose answer depends on a developer's config is not a gate,
  and renames are the obvious false-positive class.
- **`core.quotePath=false`.** This one refutes a weakness recorded in the
  earlier draft. I predicted a non-ASCII path would fail in the *passing*
  direction; it does not — git C-quotes it, so the gate fails, which is safe.
  The real defect is the mirror image: deleting `café.txt` prints
  `"caf\303\251.txt"`, so a correct PR writing `Deletes: café.txt` was
  **rejected**, and the note printed told the author their correct claim matched
  nothing. That is a false positive on a legitimate change plus advice pointing
  away from the problem, and a false positive is what gets a check disabled.

### 10. The tests run in CI, reversing the earlier decision

The first version of this design left them out: nothing else in this repo tests
its CI scripts, and that convention was "not this piece's to set."

**Review refuted that on its own terms.** Two mutations survived the entire
suite — dropping `-F` from the claim lookup, so a crafted `Deletes: src/lib?rs`
acknowledges a real deletion of `src/lib.rs`, and replacing guard 3 with the
`|| true` idiom §2's own table names as dangerous. Both are silent passes. The
tests that catch them now exist, but a test nothing runs stops being true the
moment someone edits the script, and the argument for leaving it unmeasured is
precisely the argument this gate exists to refute: **an unmeasured property is
not a held property.** A gate against silently-stopped measuring that was itself
never measured is the joke `ci.yml` keeps warning about.

It runs on every event rather than only `pull_request`, because the script's
behaviour is a property of the script, not of the event. Verified it does not
depend on the developer's git config: run with `HOME`, `GIT_CONFIG_GLOBAL` and
`GIT_CONFIG_SYSTEM` pointed at empty files — no `user.name` available anywhere,
as on a GitHub runner — all 34 assertions still pass. Without that check it
would have gone red in CI for a reason having nothing to do with the gate.

### 11. Guard 3 is reachable, and finding out cost a third wrong test

Review asked the right question — dead code, or defence against a case the
fixtures do not model? The answer is the second, and it is now pinned. Measured
per guard, with one subtree object removed from the object store:

| check | result |
|---|---|
| guard 2a `rev-parse --verify base^{commit}` | exit 0 — peels a commit only |
| guard 2a `rev-parse --verify HEAD^{commit}` | exit 0 |
| guard 2b `merge-base base HEAD` | exit 0 — walks commits, not trees |
| the diff | **exit 128** — must read the tree |

That is a partially-fetched or corrupted object store: every cheap check says
the repository is healthy and only the diff disagrees.

**The first version of this test passed for the wrong reason**, which is the
third instance of this piece's recurring defect and the one I would most want a
reader to notice. It removed the *root* tree, which in a fresh clone also makes
`rev-parse --verify` fail — so the script exited 1 at **guard 2**, the assertion
went green, and the `|| true` mutation still passed 33 of 33. The exit status
was identical whichever guard fired, so the test could not tell them apart. The
fixture now removes a *subtree*, and asserts guard 2 still passes before
asserting the failure, so it cannot silently drift back to measuring guard 2.

Also measured and worth recording: removing the **blob** of the deleted file
does not reach guard 3 at all, because `--name-only` never reads file contents.
It is the obvious fixture to reach for and it proves nothing.

## Risks / Trade-offs

- **[A `Deletes:` line is an assertion, not a review]** → Intended, and stated in
  `proposal.md`. The gate converts a silent deletion into a stated one. The
  failure message says which paths need claiming, not that claiming them is
  right.
- **[The script is invoked by CI but lives outside the workflow]** → A reader of
  `ci.yml` sees a path rather than the logic. Mitigated by the step's comment
  naming what the script does and why it is not inline.
- **[A rename trips the gate]** → Only a rename git scores below its similarity
  threshold, which is the case a reader should look at anyway. The earlier
  wording here said this followed from `--diff-filter=D` not including `R`; that
  was the wrong reason and review caught it. It follows from rename *detection*
  being on, which was a default rather than a guarantee until §9 pinned it.
