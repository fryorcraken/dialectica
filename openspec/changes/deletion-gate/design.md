## Context

See `proposal.md` — Why, for the eight firings and the 7,101-vs-37 measurement
that motivates the three-dot range.

### The near miss that happened after this design was written

`proposal.md`'s argument for the three-dot range rests on a single historical
false alarm. There is now a second instance, and it is the stronger one because
it is a *population* rather than an anecdote.

When `468e716` (op-transport, #55) landed on `main`, **every one of the ten open
piece branches would have reported deletions under the two-dot form, and all ten
were clean.** Re-measurable as long as those branches exist, with
`git diff --diff-filter=D --name-only origin/main <branch>` against the same
range with `...`:

| branch | files "deleted" two-dot | three-dot |
|---|---|---|
| `piece/authoring` | 26 | 0 |
| `piece/generated-names` | 13 | 0 |
| the other eight, each | 6 | 0 |

#68 (`piece/publish-envelope`) is the sharpest: 5,067 lines deleted two-dot
against 130 three-dot — a 4,937-line phantom. The six files it "deleted" are
`dialectica-core/src/transport.rs`, the four `op-transport` change documents and
`openspec/specs/op-transport/spec.md`: entirely content the branch predates. A
two-dot gate would have demanded that author claim deletion of a live spec they
never touched.

**Ten red pull requests at once, none defective, is how a gate gets disabled in
its first week.** That is a sharper risk to this piece than any deletion it
might miss, and it is why the range is load-bearing rather than fastidious.

### The false alarm and the real defect are indistinguishable by inspection

This answers the obvious objection to automating any of this — *why not just run
a diff and look?*

A five-thousand-line deletion count reads exactly the same whether the branch
rotted or `main` grew. A human eyeballing either number reaches the same alarm
and **cannot tell from the output which they are looking at**; the two cases
differ only in which range produced them. So a reviewer running the wrong range
does not get a confused answer, they get a confident wrong one — and
`mergeStateStatus` reported `UNKNOWN` for all three of the original PRs, so
there was no second signal to catch it.

(No figure is quoted in that sentence on purpose. It once read "5,034
deletions", which was already superseded by the 5,067 measured two sections
above — a number that is *about* ambiguity is not made truer by pinning it, and
can only rot. The reproducible pair is in the table above.)

The three-dot range is the only thing that separates the two, which is why this
is a gate rather than a note in a checklist.

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

### 2. Three independent guards, because they catch different failures

The script refuses to answer unless it could actually look, and it does that in
**three** places, each catching what the others report as healthy:

| guard | checks | catches what the others miss |
|---|---|---|
| 1 | `--is-shallow-repository` | a clone with no fork point; names `fetch-depth` |
| 2 | both refs resolve, merge base exists | unfetched ref, orphan branch, typo |
| 3 | the `git diff` itself succeeds | an unreadable tree in a healthy-looking repo |

Guards 1 and 2 are argued below. **Guard 3's reachability was doubted in review
and is established further down in this same section**, because the question
"is it dead code?" is only answerable once the first two are understood.

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

**Guard 3 is reachable, and finding out cost a third wrong test.** Review asked
exactly the right question — dead code, or defence against a case the fixtures
do not model? The answer is the second. Measured per guard, with one subtree
object removed from the object store:

| check | result |
|---|---|
| guard 2a `rev-parse --verify base^{commit}` | exit 0 — peels a commit only |
| guard 2a `rev-parse --verify HEAD^{commit}` | exit 0 |
| guard 2b `merge-base base HEAD` | exit 0 — walks commits, not trees |
| the diff | **exit 128** — must read the tree |

That is a partially-fetched or corrupted object store: every cheap check says
the repository is healthy and only the diff disagrees.

**The first version of that test passed for the wrong reason**, which is the
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

Guard 3 now carries a `── Guard 3 ──` banner in the script. It previously had
none, so five documents named a thing the source did not label — and it was
precisely the guard that survived a mutation with the suite green, which is to
say the thing hardest to see in the source was the thing the tests were
blindest to.

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

### 7. Two of these tests were written wrong first, and both are recorded

(There is a third, guard 3's, recorded in §2 beside the guard it belongs to
rather than here — it was found by review rather than by me. All three are the
same shape.)

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

### 9. Strip the quoting construct once, up front — not per-arm

The fence-stripping in §Decisions above follows a shape this repo already has,
and it is worth naming as the pattern rather than leaving each author to
rediscover it. A text gate that bans some string must first remove the places
where that string can legitimately appear *as prose* — comments in source, code
fences in Markdown — and there are two ways to do it:

- **Once, up front, producing a clean buffer every subsequent search runs
  against.** `ci.yml`'s adapter check does this (`re.sub(r"^\s*//.*$", ...)`
  before any ban is applied, with a comment explaining that the gate was firing
  on its own explanatory paragraph). This piece's `awk` fence filter is the same
  shape: one pass, then every claim lookup sees only real content.
- **Per-arm, with an exclusion bolted onto each individual search.** The closer
  reports that #67's `no QML type name collides with the host` gate took this
  route and carries documented false positives for block comments and trailing
  comments, because its exclusion only catches a *leading* `//`.

I have verified the first shape in this file and in my own script; **#67 is on a
branch not merged here, so that half is the closer's measurement and not mine.**
The structural argument stands on its own either way: an up-front strip is one
place to be right, and a new ban inherits it for free, where a per-arm exclusion
must be repeated correctly at every site and silently is not. It is the same
"complexity in the data structure, not the logic" rule CLAUDE.md states, applied
to text.

**One qualification, from the architecture review, which read both gates rather
than taking my account of one.** The sibling is not the pure exemplar this
section first implied: twelve lines after the up-front strip it bolts a named
per-arm exemption onto the clean buffer
(`code.replace("keystore.stoa_key(&stoa)", "<exempt: publish path>")`), a
deliberately temporary single-call-site patch. So the real precedent is *"strip
once up front, then one documented exception"* rather than *"strip once and
never touch the buffer again"*. Recorded because a third author copying this
pattern will meet that exemption and should not conclude the comparison was
wrong.

If a third such gate is written, this is the shape to copy.

### 10. Two git settings are pinned, because each changes the verdict

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

### 11. The tests run in CI, reversing the earlier decision

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

### 12. Every failure message must survive as a one-line annotation

Found in review, and it is this piece's own thesis arriving one layer up: the
gate correctly refused to measure, and then **could not tell anyone how to fix
it.**

A GitHub workflow command is newline-delimited — `::error::` consumes text up to
the first `\n`, and the rest becomes ordinary log output. The annotation is what
GitHub surfaces on the "Files changed" tab and in the check summary, and it is
all a reader sees unless they open the raw job log. The shallow message spanned
four source lines, so the annotation was:

```
deletion gate cannot measure this branch — the checkout is a shallow clone, so the merge base of
```

Truncated mid-clause, with `fetch-depth: 0` on the next line and therefore
invisible. A red gate whose advice is truncated gets diagnosed wrongly, and a
gate diagnosed wrongly gets disabled — the same ending this piece exists to
prevent, reached by a different road.

**The fix is structural rather than per-message.** `cannot_measure` now takes a
one-line headline plus optional detail lines and folds any newline in the
headline to a space, so a caller that wraps for source readability still emits
one annotation line. A future message cannot reintroduce the defect by being
written the natural way. Every other `::error::` in `ci.yml` is single-line, so
this was the file's only departure from a convention that already existed.

**My tests were structurally unable to see this**, which is the part worth
recording. Test 5 asserts `fetch-depth` appears in the *combined stdout*; the
truncation happens between stdout and the annotation, so the suite passed while
the surface a human reads did not carry the fix. Test 22 now extracts what
GitHub would keep — the first line of each `::error::` — and asserts on that.
Its honest limit, stated in the test: it emulates the documented newline rule,
it does not observe a rendered annotation panel. A live Actions run is still the
only way to see the real thing.

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
  being on, which was a default rather than a guarantee until §10 pinned it.
