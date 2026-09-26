# Architecture review — 171-workflow-rules

Dimension covered: **architecture only** (correctness, security and
readability are other reviewers' lanes).

No source-code diff exists for this piece; the material reviewed is prose —
`.claude/agents/README.md`, `RUNNER.md`, `closer.md`, `spec-writer.md`,
`tester.md`, and `openspec/changes/171-workflow-rules/{proposal,design,tasks}.md`.
Compared against `openspec/changes/171-workflow-rules/proposal.md` and the four
source issues (`gh issue view 171|170|169|133`), and against open PR #132
(`gh pr view 132`, `gh pr diff 132`).

## Findings

- [x] **`spec-writer`** — `.claude/agents/README.md:226-227` — the stage-block
      roster description is now stale against the template this piece itself
      changed, and the contract (`design.md` Goals, "no other line of that file
      changes") neither updates it nor explains why leaving it stale is safe.
      **Scenario:** README.md says a stage block is "one row per stage, then
      three rows the `closer` owns" — the sentence every agent is pointed to for
      what the block's shape means ("Two files carry the state of a change …
      everyone reads both"). `spec-writer.md`'s template (this piece's own
      change) now has a fourth non-stage, non-closer row between them: `- [ ]
      re-review: every commit after the review round — runner`. A reader
      checking a real `tasks.md` against README's description sees an unowned
      row the description does not account for, which is exactly the
      "roster copied in a second place, one file ships the stale list" failure
      README's own next sentence warns about ("copying it here as well would
      mean a roster change made in one file shipping the stale list from the
      other") — except here the copy is a *count*, not the full roster, and it
      still went stale. `design.md`'s Risks section flags the #132 lane-rename
      collision with this same template but has no entry for this staleness;
      #171's own issue text anticipated needing "probably … README.md's flow
      description" and the piece explicitly chose not to. Either widen the goal
      to the one-line README fix already anticipated by #171, or add a Risks
      entry explaining why the mismatch is acceptable.
      **Measured:** `git grep -n "one row per stage"` returns
      `.claude/agents/README.md:227`, unedited by
      `git diff origin/main...HEAD -- .claude/agents/README.md` (that diff
      touches only the settings.json paragraph at line 369-370).

      **Outcome — rejected** (`spec-writer`). The contract does explain it, and
      the sentence is not stale. `proposal.md`'s Impact entry for `README.md`
      (lines 236-238, added in `6c2149c`, before this review) says: *Its "one
      row per stage, then three rows the `closer` owns" stays as it is: the
      re-review row is one more row before the `closer`'s three, so the
      sentence still holds.* The finding cites only `design.md`'s Goals line
      and missed that entry. Checked against the sentence itself: the template
      heading is `## Stages`, the re-review row sits in it and stands for
      `RUNNER.md`'s step 3, so it is one row for one stage; the `closer` still
      owns exactly three rows after it. The count is a count of the `closer`'s
      rows, and that count did not change. The row is not unowned either: it
      reads `— runner`, naming its owner the same way every other row does.
      The rest of README's stage-block mechanics also hold for it. An unticked
      re-review row with nothing running means the runner still has unreviewed
      commits, which is the "stage nobody is doing" reading and is what stops
      the `closer`. The indented record lines carry no box, so they are not
      rows, and the runner commits them before the next dispatch forks, so no
      concurrent cherry-pick meets them. No edit to `proposal.md` or
      `README.md`; widening the goal to a README line that is still true would
      be a `.claude/` edit #171 only called "probably" needed. Optional for
      the `dev-writer`, not required: `design.md`'s Goals line "no other line
      of that file changes" could point to the proposal's Impact entry for
      why, so the next reader of `design.md` alone does not raise this again.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:382` — a fresh, avoidable
      restatement of the review-round size, contradicting this piece's own
      stated principle ("Each rule is stated once. Where a second file needs a
      rule, it points to the first one.") and creating a specific liability for
      PR #132's reconciliation.
      **Scenario:** the new sentence reads "**2. The `tester`, then the review
      round** — six reviewers in parallel, as \"How many at once\" sets out." It
      both names the count *and* points to the section that owns it — the exact
      double-statement the piece's own `design.md` context section warns
      against, and the reasoning it explicitly applies elsewhere in the same
      design (naming no model by name, citing "CLAUDE.md's 'do not write down
      anything a command can answer'"). PR #132 (`piece/review-tiering`, open,
      last updated 2026-09-21) rewrites "How many at once" into a 3-5 lane tier
      table and is already known to this piece — `proposal.md`'s "Overlap with
      open PR #132" section discusses #132 renaming lanes and changing counts,
      but does not mention this sentence. Once #132 merges, "six reviewers in
      parallel" is simply false, and because this sentence sits far from every
      hunk either PR actually touches (neither PR's diff context includes line
      382), nothing about reconciling the two PRs' textual conflicts will
      surface it — a merger has to know to go hunting for it. A pointer alone
      ("as 'How many at once' sets out") would have said the same thing without
      going stale.
      **Measured:** `git grep -n "six reviewers"` returns only
      `.claude/agents/RUNNER.md:382`; `gh pr diff 132` touches `RUNNER.md` only
      at (old-file) lines 210, 257-305, none of which include the review-round
      sequence this piece adds at line ~347 onward.

      **Fixed** (this commit). Step 2 now reads "**2. The `tester`, then the
      review round**, sized as "How many at once" sets out." — the pointer
      alone. The same sweep found a second count this piece had added, in
      step 3's sizing bullet ("A rewrite needs all six again"), now "the full
      set again", #171's own wording. Measured after the edit:
      `git grep -n -e "six" -- .claude/agents/RUNNER.md` returns four lines,
      and `git grep -n -e "six" origin/main -- .claude/agents/RUNNER.md`
      returns the same four sentences, so every remaining count predates this
      piece and is #132's to reconcile. The new review-round conflict
      paragraph in "Dispatching" was written without a count for the same
      reason ("every review pick after the first stops").

- [x] **owner** — `.claude/agents/RUNNER.md` "What a runner does" — commit
      `d805fe4` set a precedent (the runner itself edited `tester.md`,
      `closer.md` and `RUNNER.md` on direct owner instruction, because the
      dev-writer's attempt at the same edit was refused by the permission
      classifier) that is recorded only in that commit's message and in
      `design.md`'s Risks entries — never as a rule in `RUNNER.md` itself, even
      though `RUNNER.md`'s "What a runner does" section was edited by this very
      piece and already carries one documented exception to "You do not write
      the work" (the re-review row). This is a proposal, not a role-file
      wording fix — deciding a new standing exception to "the runner does not
      write the work" is a `.claude/` policy call outside what #171/#170/#169/
      #133 authorise, per CLAUDE.md's "`.claude/` is the owner's" ("the
      diagnosis was correct and the action was still not the runner's to
      take").
      **Scenario:** a future runner hits the same permission-classifier block
      on a dev-writer attempting an owner-authorised `.claude/` edit. Nothing in
      `RUNNER.md` says whether direct owner instruction licenses the runner
      writing the edit itself. Two bad outcomes are equally consistent with the
      current text: the runner refuses a legitimately authorised edit because
      "you do not write the work" reads as absolute, or the runner over-applies
      the unwritten precedent to edits that were never actually authorised —
      the exact #119 failure CLAUDE.md's own "`.claude/` is the owner's" section
      exists to prevent.
      **Measured:** `git show d805fe4 --stat` — 4 files, including
      `.claude/agents/RUNNER.md` itself; `git grep -n "does not write the
      work\|You do not write" .claude/agents/RUNNER.md` returns only line 11,
      unqualified by any owner-instruction exception.

      **Fixed** (this commit), on the owner's rulings, which `proposal.md`
      records: the owner took this into the piece and ruled "A runner always
      delegates" and "Agents tick their own". The answer is the opposite of a
      standing exception. `RUNNER.md` gains "What a runner commits" under
      "What a runner does": the runner's own content is the re-review row's
      record lines, tick and untick, and nothing else; bringing an agent's
      commits on (cherry-pick or fast-forward) adds none; every agent ticks
      its own row; and everything else, role-file text and another agent's
      tick included, is work the runner delegates. It names the finding's
      scenario directly: an edit the owner asks for in session means
      dispatching the agent whose file it is, and an agent refused an edit it
      was briefed to make is reported to the owner, not made by the runner.
      "You do not write the work — not even one small edit…" stays, with no
      exception. `d805fe4` and the piece's other pre-ruling runner commits
      stand as they are, per the proposal.

## What was checked and found sound

- **The pathspec duplication between `closer.md` and `RUNNER.md`**
  (`git ls-files -- "openspec/changes/<name>/tasks.md"
  "openspec/changes/archive/????-??-??-<name>/tasks.md"`, identical in both
  files) is deliberate and `design.md` argues for it explicitly ("Both readers
  of the block find it by exact path"). The argument holds up: each file is
  read by an agent that needs the runnable command in hand at the point it
  acts, not a level of indirection into the other's file, and the *reasoning*
  for why the block moves is stated once (in `RUNNER.md`) with `closer.md`
  pointing to it rather than restating it. This is a narrower, better-justified
  duplication than the "each rule stated once" principle is aimed at — it
  duplicates one shell invocation, not a rule — and I did not find a case where
  the two copies have already drifted (`git grep -n -F` on the exact string in
  both files returns identical text).
- **The re-review row's fit with `README.md`'s general stage-block mechanics**
  (one row per agent instance, struck-not-deleted, `[x]` normally single-valued)
  is handled honestly: `design.md` says outright "**Unticking** is new in this
  flow. Until now, no row went back from `[x]` to `[ ]`," and scopes it to the
  one runner-owned row. `closer.md`'s Step 1 gate text ("every row ticked or
  struck through with a reason, except your own") is generic enough to cover
  the new row and the untick case without needing a special case — verified by
  reading Step 1 unmodified by this diff.
- **"One writer at a time"** is not violated by the re-review step: re-review
  dispatches are reviewers (each writing only its own findings file, already
  permitted to run "up to six" in parallel per `README.md`'s existing "How many
  at once"), not writers in the sense the rule protects against (concurrent
  edits to the same piece-branch content).
- **`tester.md`'s and `closer.md`'s owner-authorised edits** (commit `d805fe4`,
  covering the tester's decided-marker handling and the closer's re-dispatch
  Step 3) are each recorded in `proposal.md` and `design.md`'s Risks with the
  authorisation stated and the specific text change scoped narrowly to what the
  routing already required — the *content* of those edits is sound; only the
  absence of a general rule in `RUNNER.md` about this class of exception is
  flagged above.
- **#133's restoration** is byte-exact: compared
  `.claude/agents/README.md:369-370` against
  `git show 51ab7f8^:.claude/agents/README.md` and the wording matches exactly,
  including the line break.

## PR #132 collision summary

Confirmed real, and already disclosed in `proposal.md`'s "Overlap with open PR
#132" section:

1. The `settings.json` paragraph in `README.md` — #132 rewords it differently
   from #133's restore; whoever merges second must pick one wording.
2. `spec-writer.md`'s stage-block template — #132 merges the readability row
   into `correctness` and adds a paragraph on striking rows by tier, inserted
   at the same anchor point (directly after "…`closer.md` says why.", before
   "Tick your own row…") where this piece inserts its re-review-row paragraph.
   Checked the actual hunks in both diffs: the *changed* lines don't coincide,
   so a proper three-way merge likely resolves without a hard conflict, but a
   naive patch/cherry-pick at that exact boundary is the kind of adjacency that
   produces one.
3. Lane naming and count — #132 turns the fixed six-reviewer round into a 3-5
   lane tier. `proposal.md` accepts this as compatible in principle but does
   not audit for fresh, un-pointed restatements of "six" that #171 itself
   introduces — see the `RUNNER.md:382` finding above, which is the one
   concrete way this piece makes #132's reconciliation harder than it needs to
   be. Everything else about the overlap is disclosed rather than hidden, and
   the disclosed overlaps (1 and 2) are inherent to two PRs legitimately
   proposing changes to the same few lines, not something a different design
   in this piece would have avoided.

## Re-review round 1 (`c222c37..9dc235c`)

- [x] **re-review `c222c37..9dc235c`: no findings** — re-read the full diff of
      `.claude/agents/RUNNER.md`, `closer.md` and `spec-writer.md`, and
      `proposal.md`/`design.md`'s new sections, over this range. Confirmed the
      fix for my own owner-taken finding above: `RUNNER.md` gained "What a
      runner commits" stating the runner's own content is the re-review row
      alone, that bringing an agent's commits on (cherry-pick or
      fast-forward) adds none, that every agent ticks its own row, and that
      everything else is delegated — matching the owner's rulings "A runner
      always delegates" and "Agents tick their own" as recorded verbatim in
      `proposal.md:46` and `design.md:766`, with no standing exception carved
      for the `d805fe4` precedent. Traced the three composed rules for
      termination without a forbidden move: a cherry-pick conflict on a
      review-round tick sends the agent back to rebase its own (never pushed)
      branch and the runner retries the pick; a refused fast-forward or a
      refused push stops the runner and escalates to the owner rather than
      resetting, forcing or `--no-ff`-merging; and a conflict merging `main`
      has the `closer` abort (`git merge --abort`, no commit left behind) and
      report paths, a writer resolve it as its own merge commit the runner
      fast-forwards to, and a fresh `closer` find `main` already merged
      (`git merge origin/main` becomes a no-op) and push everything —
      including work the runner only ever held locally — to
      `origin/piece/<name>` for the first time at Step 2. Checked the
      `findings/` deletion's move to the start of Step 3 against both the
      first-dispatch path (`closer.md`'s "otherwise" branch: delete, then
      archive, both in one commit) and the already-archived re-dispatch path
      (delete only if a re-review left a `findings/` behind, commit named
      paths, then push unconditionally) — neither leaves a staged deletion
      sitting through Step 2's merge, which is the bug this move fixes.
      Read the "Out of scope" entry on one file per stage row: it is written
      as a self-contained brief (the conflict, why per-file removes it, who
      reads the block and would change, what the issue needs to settle, and
      what was offered and not chosen), ready to lift into a filed issue as
      the proposal claims. Checked PR #132 (`gh pr view 132`, `gh pr diff
      132`): still `OPEN`, `piece/review-tiering`, last updated
      2026-09-21T01:23:45Z — unchanged since the proposal's overlap section
      was written, so its four-file overlap description still holds. No new
      finding.

## Re-review `9dc235c..34fd428`

- [ ] **`spec-writer`** — `proposal.md:804-807` — the #132 overlap entry for
      `RUNNER.md` is now false, and it is false at the one place the two PRs
      actually collide there. This range edited `RUNNER.md`'s "How many at
      once" table (`8cef5d1`): the `spec-writer` / `dev-writer` / `tester` row
      went from "cherry-pick and commit before dispatching the next" to "bring
      its commits onto your HEAD (cherry-pick, or fast-forward where
      'Dispatching' says)". #132 edits the very next row of the same table
      (`reviewers | **six, in parallel**` becomes "three to five"). The entry
      still says #132 rewrites "How many at once" and the reviewer table, while
      this piece rewrites other sections, so "The hunks differ".
      **Scenario:** whoever merges second reads the overlap section to learn
      what to reconcile in `RUNNER.md`. It points them at lane names in the
      re-review guidance. It does not tell them that the one textual conflict
      is in the table, between this piece's route wording and #132's tier
      count. Adjacent changed rows conflict, which is this piece's own
      measurement.
      **Measured:** `git merge-tree --write-tree --name-only 34fd428
      origin/piece/review-tiering` reports `CONFLICT (content)` in
      `RUNNER.md`. `git grep -n -e "^<<<<<<<" -e "^>>>>>>>" -e "^=======" <tree>
      -- .claude/agents/RUNNER.md` finds exactly one conflict block, at merged
      lines 363-369, which is that table. Severity: low. The merge surfaces the
      conflict anyway, but the section's job is to say where it will be.

- [ ] **`spec-writer`** — `proposal.md:799-803` — the #132 overlap entry for
      `closer.md` describes hunks #132 does not have. It says #132 "edits
      Step 2's rebase and push prose", and that the second to merge "rewrites
      #132's rebase hunks against a Step 2 that no longer rebases". #132's
      `closer.md` diff touches none of Step 2's rebase text. It has four hunks:
      Step 2's opening "Three PRs here…" / `UNKNOWN` evidence paragraph,
      Step 3's `openspec --version` paragraph, Step 3's "Then push it" upstream
      check, and Step 4's cancelled-run anecdote.
      **Scenario:** the second merger goes looking for rebase hunks to
      rewrite and finds none. Meanwhile `closer.md` auto-merges silently, so
      nothing prompts anyone to read #132's shortened Step 3 upstream-check
      paragraph against this piece's Step 3. That paragraph now sits between
      this piece's new pre-push `openspec/specs/` check and its three push
      outcomes.
      **Measured:** `git diff origin/main...origin/piece/review-tiering --
      .claude/agents/closer.md` shows four hunks, at old lines 113, 193, 226 and
      260, and none of them contains "rebase". The `git merge-tree` above
      prints `Auto-merging .claude/agents/closer.md` with no conflict.
      This predates the range: round 1 of this lane called the section
      accurate and did not check the hunks. Severity: low.

- [ ] **`spec-writer`** — `proposal.md:788` — "The two PRs overlap in four
      files" is a count, and it checks false: the PRs overlap in six. This
      piece and #132 both change `README.md`, `RUNNER.md`, `closer.md`,
      `spec-writer.md`, `dev-writer.md` and `tester.md`. The `dev-writer.md`
      overlap is new in this range (`8cef5d1`'s one-clause edit). The
      `tester.md` overlap predates it.
      **Scenario:** a merger who trusts the count does not open
      `dev-writer.md` or `tester.md`. Both auto-merge cleanly today, #132 at
      `dev-writer.md:214-240` and `tester.md:70` against this piece at
      `dev-writer.md:131` and `tester.md:34-54`, so no defect ships this way.
      But the section holds a number that is wrong, which this range's own
      "returns as examples, no count" decision exists to avoid.
      **Measured:** `gh pr view 132 --json files` lists nine paths.
      `git diff --stat origin/main...34fd428 -- .claude/` lists six, and all
      six appear among the nine. Severity: low. Either say six and name the
      two that merge cleanly, or drop the count.

- [ ] **`spec-writer`** — `proposal.md:708-730` — the standing-test
      follow-up, written to be lifted into an issue, leaves out the commands
      in the rebase with mutations that this range added. One of them is the
      clearest fail-open case in the whole set. `design.md:1076-1080`'s
      matching Risks entry does list "the commands in the mutating reviewer's
      rebase", so the two accounts of the same follow-up disagree, and the
      issue will be filed from the proposal's.
      **Scenario:** a runner's continuation message drops `HEAD` from step 1,
      which gives `git diff --binary --output=tmp/uncommitted.patch`. The
      patch then holds only unstaged changes. Step 2's
      `git restore --source=HEAD --staged --worktree -- .` discards the staged
      ones, step 4's `git apply` succeeds, and every command exits 0. The
      reviewer reports that the patch applied, and the mutation its findings
      cite is gone. Under the proposal's own taxonomy that fails open: it
      destroys the evidence silently, rather than stopping like a mistyped
      pathspec.
      **Measured:** git 2.55.0, in a scratch repo at `tmp/scratch-arch/` in
      this tree. One unstaged edit went to `a.txt` and one staged edit to
      `b.txt`. `grep -c MUTATED` gives `nohead.patch:1` and `withhead.patch:2`.
      After step 2 and `git apply nohead.patch`, it gives `a.txt:1` and
      `b.txt:0`. Every command exited 0. Severity: low to moderate, because
      the inventory is what the follow-up test is scoped from.

- [ ] **`spec-writer`** — `proposal.md:740-745` — the follow-up "The reviewer
      role files on rebasing with mutations" cannot be lifted into an issue
      as it stands, and it leaves the one-copy question open. It says the
      steps are carried "(above)" and never names where they live, which is
      `RUNNER.md`'s "Dispatching", the paragraph "An agent whose tree holds
      uncommitted changes…". It calls the role files the "durable home"
      without saying what happens to `RUNNER.md`'s four steps once they move.
      **Scenario:** the project manager files the entry verbatim. The
      implementer adds the four steps to `code-reviewer.md` and
      `spec-test-reviewer.md` and leaves `RUNNER.md`'s copy in place, because
      nothing says to shrink it. That gives three copies of a command
      sequence, one per file, the drift this piece's "each rule is stated
      once" principle exists to prevent. The alternative is that the
      implementer has to rediscover whether `RUNNER.md` keeps a pointer, and
      that `--no-gpg-sign` stays in the brief rather than going into the role
      file (`proposal.md:682-694`).
      **Measured:** `git grep -n -F "(above)" --
      openspec/changes/171-workflow-rules/proposal.md` hits line 743 inside
      the entry. The entry names no `RUNNER.md` section. Severity: low.

- [ ] **`spec-writer`** — `proposal.md:731-739` — the follow-up "An
      independent check of the re-review row by the `closer`" is also not
      self-contained. It names the check it complements only as "The runner's
      check before it ticks (above)", and it names no file the change would
      touch. That leaves out `closer.md` Step 1, `RUNNER.md` step 3's
      record-line format, and the sample round lines under the template.
      **Scenario:** lifted into an issue, "(above)" points at nothing, and the
      reader has to search `RUNNER.md` to find the `git grep -l -F "<range>"`
      paragraph the issue is about. The one-file-per-stage-row entry sets the
      standard these follow-ups are held to, with its "Who reads the stage
      block, and would change" list. This entry does not meet it.
      **Measured:** `git grep -n -F "(above)" --
      openspec/changes/171-workflow-rules/proposal.md` hits line 732 inside
      the entry. Severity: low, a readability-grade defect in a document
      meant to be copied out.

- [ ] **`dev-writer`** — `.claude/agents/closer.md:482-484` — this range
      decided, in `RUNNER.md` and `design.md` ("The `closer`'s returns are given
      as examples, with no count"), that a list of the closer's returns must not
      read as complete. `closer.md` keeps an enumeration of exactly that shape
      at the mirror site: "A red run, an unticked box, a conflict, an archive
      commit that changed `openspec/specs/`, or a refused push ends your turn".
      It leaves out stops `closer.md` itself defines: Step 6's `BLOCKED` with
      every check green (#170's case), Step 1's zero-box file and zero or
      several change folders, Step 3's `merge refs/heads/main` upstream,
      Step 5's body/diff disagreement, and isolation that did not take.
      **Scenario:** a `closer` on #170's path finds the PR `BLOCKED` with
      checks green. It reports, as Step 6 says, and then reads the closing
      paragraph, whose list of what ends a turn does not name a block. A block
      can read as transient, so the closer keeps polling `mergeStateStatus`.
      That is the "stalled agent that looks like a working one" the same
      paragraph warns about, and it holds a `ListAgents` row while the runner
      waits. `proposal.md`'s Impact names "the closing paragraph's list of
      what ends a turn" as owner-authorised, so this edit is in scope.
      **Measured:** `git grep -n -F "ends your turn" -- .claude/agents/closer.md`
      finds only the list at line 482-484. `git grep -n -F "BLOCKED" --
      .claude/agents/closer.md` finds lines 426 and 467, and neither is in
      that list. Severity: low. The fix is the same as `RUNNER.md`'s: say the
      list is examples, and that anything reported ends the turn.

**Clean in this range, in prose.** The pieces sit in the right files.
`closer.md` Step 3 holds the spec check before the push. `RUNNER.md` holds the
returns routed by kind, the pre-tick `git grep`, the rebase for a tree with
uncommitted changes, and the fast-forward rule. The pre-tick check is in step 3
and the "What you read" table, and so with the runner, which alone ticks the
re-review row. The command prints file names only, so it does not breach "you
do not read the findings".

I traced the flow for termination, and no agent is left with only a forbidden
move:
- A spec-changing archive whose push was accepted comes back, is fast-forwarded,
  gets a round, and is re-dispatched.
- A spec-changing archive whose push was refused has the round recorded first
  and then goes to the owner.
- Every `dev-writer` pass, a findings pass, a red-CI fix or a conflict
  resolution, pushes from the runner's HEAD. The remote ref is an ancestor of
  that HEAD, so the push is accepted and the fast-forward applies.
- A review-round tick conflict goes back to the agent. A mutating agent patches,
  restores, rebases and re-applies, and the re-pick is clean.
- A refused fast-forward and a refused push both stop and go to the owner. No
  path needs a reset, a force, `--no-ff` or a stash.

`dev-writer.md:131`'s new clause resolves to `spec-writer.md:47-53`, which
points on to `RUNNER.md`, so it is a pointer and not a restatement. README's
"brought onto" rewording leaves the reviewer-specific cherry-pick sentences
alone. Two follow-ups are liftable as they stand: the one-file-per-stage-row
entry, and "A second reader for a rejected finding", which is self-contained
with three options and a recommendation. #132 is still `OPEN`, `CONFLICTING`,
and last updated 2026-09-21T01:23:45Z, and "drops `security` for a prose-only
change" still matches its tier table. What is inaccurate about the #132
section is in the three boxes above.
