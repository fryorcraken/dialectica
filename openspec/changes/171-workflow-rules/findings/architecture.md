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

- [ ] **`dev-writer`** — `.claude/agents/RUNNER.md:382` — a fresh, avoidable
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

- [ ] **owner** — `.claude/agents/RUNNER.md` "What a runner does" — commit
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
