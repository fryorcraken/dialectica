# Design review — deletion-gate

Scope: does the code take the decisions `design.md` records, and were the
decisions worth recording recorded? Not code quality, not test coverage — four
sibling sheets own those.

**The design record is, on the whole, sound.** Every decision the brief named as
load-bearing is both recorded and taken, and three of them I verified by
execution rather than by reading:

- **The three-dot range** is used at the only place it matters
  (`check-claimed-deletions.sh:259`, `"$base_ref...$head_ref"`), and the workflow
  passes `github.event.pull_request.base.sha` with `HEAD` into it. The figures
  re-derive: `git diff --shortstat 468e716 origin/piece/publish-envelope` gives
  `2065 insertions(+), 5067 deletions(-)` against `468e716...` giving `130`, and
  the six named files are exactly what `--diff-filter=D --name-only` lists.
  `authoring` still shows 26 and `generated-names` 13.
- **Three guards, each load-bearing.** I applied both mutations myself. Replacing
  guard 3's `if !` with `|| true` leaves **37/39**, test 18 red, and the script
  printing `ok: 0 deleted path(s)` — the silent pass §2 describes, observed.
  Neutering guard 1 leaves **35/39**, test 6 red on its verdict while test 5's
  exit-status assertion stays green, which is §2's sharpest claim, confirmed.
- **`git merge --squash` of a behind-branch keeps what `main` gained.** The
  correction the brief flagged as mattering most is right. Reconstructed in a
  throwaway repo: branch forks, `main` then adds `gained-by-main.txt` and
  appends to `shared.txt`, branch commits unrelated work. `--is-ancestor main
  feature` exits 1 (behind), two-dot names `gained-by-main.txt`, three-dot is
  empty — and after `git merge --squash feature`, `gained-by-main.txt` survives
  and `shared.txt` keeps main's appended line. It is a three-way merge against
  the merge base, as `proposal.md:187-195` now says.
- **`"strict": true`** verifies: `gh api repos/fryorcraken/dialectica/branches/main/protection`
  reports it on required checks, with `Lint` among the four contexts and
  `enforce_admins` enabled. The declined gate arm's first ground is real.
- `--find-renames` and `core.quotePath=false` are pinned on the diff and tests
  19/20 cover them; the full suite is **39 passed, 0 failed**.

**On the declined gate arm for case 3: the refusal is correct and well recorded.**
Both grounds hold independently — branch protection genuinely enforces it at
merge time, and an arm would have fired on ten healthy PRs, which is the false
alarm the piece's own central argument is against. It is recorded in three places
(`proposal.md` "Why this is not a second arm", `design.md` Risks, the script
header) plus relocated to `closer.md` step 2 with the `--is-ancestor` command and
the reason exit 1 makes a diff stat ambiguous. Nobody reopening this can claim it
was decided in passing.

**On the scope section's two properties: they hold, and the #69 framing is the
right use of the evidence.** "Catches a file vanishing, not work vanishing" and
"measures only against the fork point" do cover all three cases, and stating
properties rather than enumerating cases is the correct call — a fourth case will
be another shape of one of them. Keeping #69's own instance as the argument rather
than a footnote is right: it is evidence that a written-down limit is still a
limit that gets missed, which is precisely an argument about how far the scope
section can be relied on. I would not soften it.

Four boxes below. One is a contradiction between the code's user-facing message
and the document that governs it; three are gaps.

---

- [ ] **`spec-writer`** — `proposal.md:9` and `.github/workflows/ci.yml:75` state
      a premise this change's own later section corrects, and I disproved it by
      execution
      Both say a branch cut before a change landed carries the file's absence "as
      an intentional-looking deletion" and then that **"A squash merge applies
      it."** For that case — the behind-branch phantom, which is what the
      surrounding sentences describe ("the branch simply predates them") — a
      squash merge does **not** apply it. `proposal.md:187-195` already says so
      in its own words: *"a genuine squash-merge does not revert what `main`
      gained... it is a three-way merge against the merge base."* The correction
      landed in the case-3 section and never propagated back to the Why section
      or the workflow comment, so the document now asserts and denies the same
      mechanism eleven lines apart in file terms and 180 lines apart in reading
      order.
      **Verified, both directions, in throwaway repos.** Behind-branch, three-dot
      clean, `git merge --squash` — `gained-by-main.txt` survives and main's
      appended line survives, so nothing was applied. Then a branch that really
      deletes `doomed.txt` after the fork while `main` moves on — three-dot names
      it and `git merge --squash` **does** apply it, `doomed.txt` is gone.
      So the true statement is narrower and still enough to motivate the gate: a
      squash merge applies whatever the **three-dot** diff deletes, which is
      exactly what this gate measures. What it does not do is apply a phantom.
      This matters beyond tidiness because the phantom-is-applied belief is what
      would make someone reach for the case-3 gate arm that the piece spent a
      whole section declining — the two are the same claim, and only one copy has
      been corrected. `proposal.md` is `spec-writer`'s file; the `ci.yml` comment
      is `dev-writer`'s to follow.
      **Not a finding against the script's own message** (`check-claimed-deletions.sh:370`),
      which says the same sentence but fires only on three-dot deletions, where
      run2 shows it is true.

- [ ] **`dev-writer`** — `design.md:298` forward-references a Decisions entry
      that was never written, and the decision it names is a real one
      §9 opens *"The fence-stripping in §Decisions above follows a shape this
      repo already has"* — but there is no fence-stripping entry above it, or
      anywhere. `grep -n fence design.md` returns four hits, all inside §9 or
      incidental. §4 records what a claim is (anchored, whole-line, exact string)
      and never mentions fences; §5 records claimed-but-absent. So §9 argues about
      *how* to strip without any entry recording *that* stripping happens or why.
      The decision is not minor — it changes what counts as a claim, and the
      script's own comment gives it 28 lines (`check-claimed-deletions.sh:268-295`),
      which by this repo's rule is the tell that it was a decision. It carries
      everything a Decisions entry wants and none of it is in `design.md`: the
      constraint (a PR body documenting this convention contains a fenced
      `Deletes:` example — *"Measured before fixing: a body of 'here is how it
      works', a fence, `Deletes: doomed.txt`, a closing fence, satisfied the gate
      for a genuine deletion of that file"*), the alternative ruled out
      (anchoring alone, which the same comment shows is insufficient), and a
      deliberate fail-safe direction with a real alternative — an unclosed fence
      swallows the rest of the body, *"deliberately the safe direction: claims go
      missing, so the gate FAILS a deletion rather than accepting one."*
      That last is the sharpest omission: a reader could plausibly have chosen
      fail-open, and `design.md` does not tell them it was chosen.
      Fix is either a new entry or folding it into §4 beside anchoring, and
      correcting §9's dangling reference either way.

- [ ] **`dev-writer`** — both new scripts depart from the shell convention
      `design.md:64-66` states they follow, and the departure is unrecorded
      `design.md` lists among its design-level constraints: *"The repo's shell
      conventions are set by `.github/workflows/ci.yml` and
      `dialectica-ui/tests/run-qml-tests.sh`: POSIX `sh`, `set -eu`, and a
      `::error::` line naming the file when a check fails."* Both new scripts use
      **`set -u`**, not `set -eu` — `check-claimed-deletions.sh:70` and
      `tests/test-check-claimed-deletions.sh:16`. `run-qml-tests.sh:20` is
      `set -eu`, so the stated convention is real and this is a departure from it.
      **Verified:** `grep -n "^set " ` across the three files.
      It is almost certainly the right call in both, which is why it deserves
      recording rather than fixing. In the test script `set -e` would be actively
      wrong: the suite runs the gate expecting exit 1 more often than exit 0, and
      `-e` would abort at the first intentional failure instead of reporting
      `N passed, M failed`. In the gate itself every failure path is handled
      explicitly through `cannot_measure`, and `-e` could abort mid-message.
      But `design.md` §2 builds its entire three-shapes table on `set -eu`
      behaviour (*"Alternative considered: trust `set -eu` and let git's 128
      propagate"*), so a reader arrives at the source expecting `set -eu`, finds
      `set -u`, and has nothing telling them whether that was reasoned or
      overlooked. One entry naming the constraint (a test harness must survive its
      own expected failures), the alternative (`set -eu`, as the convention and as
      §2's own frame), and the cost (every command's exit status is now the
      author's to check, with no net) closes it.

- [ ] **`closer`** — the `6,871` figure has no reproducing command and its only
      flag is inside a **ticked** box that `findings/` deletion will remove
      The brief describes this figure as *"deliberately left unticked rather than
      invented around"*. It is not unticked. It is discussed at
      `findings/readability.md:306-309`, inside box `[x]` at line 266 — *"The
      **6,871** figure you flag at line 70 is a real remaining gap. I did not
      touch it: it is a session observation from before this piece, no command
      reproduces it... Flagging it rather than closing it."*
      **Verified:** `grep -rn "^- \[ \]" findings/` returns eleven boxes and none
      is this one; the `6,871` mentions are at `readability.md:286` and `:306`,
      both under the `[x]` at `:266`.
      That flag therefore lives only in a file the closer's own stage row deletes
      (*"findings all ticked, `findings/` deleted"*), while the figure itself
      survives in three places that ship: `proposal.md:86`, the script header at
      `check-claimed-deletions.sh:189`, and the test header at
      `test-check-claimed-deletions.sh:135` — the last two as a load-bearing
      comment (*"THE REGRESSION TEST FOR THE 6,871-DELETION FALSE ALARM"*).
      So the merge state is: an unsourced number in shipped source, and the only
      record that it is unsourced deleted in the same step. This repo's rule is
      that a number in a comment is a claim, and its memory records citations
      getting fabricated precisely where they are most persuasive.
      Not asking anyone to invent a command — declining to do that was correct.
      Either carry the honesty into the shipped text (*"a session observation
      from before this change; no command reproduces it — the reproducible pair
      is in `design.md`'s ten-branch table"*), or replace the three call sites
      with the pair that does reproduce, 5,067-vs-130. One or the other must
      happen before `findings/` goes.

---

## Checked and clean, recorded so nobody re-does it

- **Decisions taken as recorded.** §1 (script not inline) — the file exists and
  the shallow case is genuinely only reachable that way. §3 (`fetch-depth: 0`,
  not finite) — set on the `lint` checkout. §4 (anchored whole-line claim) — the
  `sed` anchors at line start with optional list marker, and `grep -qxF` is exact.
  §5 (claimed-but-absent is a note) — the second loop prints and does not set
  `unclaimed`. §6 (`if:` on the step) — on the step, not inside the script, and
  the body arrives via `env:` to a file. §8 (workflow's own invocation tested) —
  test 16 uses a raw base SHA with `HEAD`. §10 (two git settings pinned) — both
  on the diff line, tests 19/20 green. §11 (tests run in CI) — the second step
  has no `if:`, so it runs on every event, and its reversal is argued at length in
  `design.md` §11, `ci.yml`'s comment and `tasks.md` 4.0e. That reversal is
  recorded well enough that undoing it would require arguing against three copies
  of the same paragraph; I would leave all three.
- **No allowlist file** — confirmed absent, and §"3. No allowlist file" in
  `proposal.md` names the constraint, the alternative and the precedent
  (`every_request_taking_method()`), which is a complete entry.
- **`docs/PLAN.md` from `origin/main`** carries nothing this change contradicts.
  I searched the whole file for `deletion`, `Deletes:`, `three-dot`, `two-dot`,
  `merge base`, `fetch-depth`, `mergeStateStatus`, `is-ancestor` — no hits. §10
  (CI) is the relevant section and its claims stay true: the job list is still
  `lint / qml / rust / build / release` because this adds *steps* to `Lint`
  rather than a fifth job, its "Deliberately not built" list does not name this,
  and §9's *"CI exists covering §10's four jobs"* is unaffected. `PLAN.md` is
  untouched by the branch, correctly. §10's anti-false-green principle is the
  reasoning this change **applies** rather than acts on and removes, so there is
  nothing to migrate out of `PLAN.md` — no reasoning is duplicated across the two.
  `proposal.md`'s note that §10's stale `qml`-job claim belongs to a different
  change is the right call.
- **The annotation-folding mechanism has no test behind it** — every
  `cannot_measure` headline is a single-line string, so `tr '\n' ' '` is never
  exercised, and `design.md` §12's *"A future message cannot reintroduce the
  defect"* is a structural claim resting on an untested line. **Not a box from
  me:** `findings/spec-test.md:128-153` already carries it as an open `tester`
  box with the same diagnosis and a remedy. Flagged here only so the two sheets
  are seen to agree.
- **`design.md`'s ten-branch table has partly decayed, and that is acceptable as
  written.** Three of "the other eight" have since merged `main` in, so they now
  report 0 two-dot (`thread-read`, `ui-composer`, `ui-onboarding`), and
  `drop-apparatus` shows 2 deletions under *both* forms because they are genuine
  — it is a "drop" piece. The blanket row *"the other eight, each: 6 / 0"* is
  therefore no longer true of all eight. **No box:** the section labels itself
  *"Re-measurable as long as those branches exist"*, the headline claim still
  reproduces on five branches with `origin/main` unmoved at `468e716`, and
  `proposal.md` was already rewritten to instruct against whatever is open rather
  than against pinned names. Pinning per-branch numbers harder would repeat the
  rot the same document diagnoses twice. Worth knowing if someone re-derives it
  later and finds four rows short.
- **`design.md` §2 slightly overstates one measurement.** It says removing the
  shallow guard leaves *"the headline shallow test (5) still passing"*. Test 5's
  exit-status assertion passes; its **second** assertion, "shallow failure names
  fetch-depth", goes red, along with two of test 22's. Measured: 35 passed, 4
  failed, not 37/2. The substantive point — that guard 2 masks guard 1 on the
  verdict and only test 6 holds guard 1 up — is exactly right, so this is a
  wording slip inside a correct argument, not a false claim. Noted without a box.
