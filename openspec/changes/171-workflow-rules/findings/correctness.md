# Correctness review — 171-workflow-rules

Scope: correctness only, as instructed. I walked the five required paths (first
pass with no marker; a `NO SPEC:` hand-back and an unmarked silent-spec
decision; the review round then a writer pass; an owner instruction after
review; a red CI run after archiving, and a second red run) against the
literal text of the changed role files, and ran every command the new text
gives against this tree.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:358` — the `NO SPEC:` marker
      grep the new Step 1 tells the runner to hand the `spec-writer` is
      unscoped, and this tree already returns heavy noise from it.
      **Scenario:** a future piece's `dev-writer` hand-back names one new
      `// NO SPEC:` marker it left in, say,
      `dialectica/rust-lib/dialectica-core/src/foo.rs`. Per Step 1, the
      runner "point[s] its brief at the markers with `git grep -n "NO SPEC:"`
      rather than listing them" — with no scope given (no path, no diff
      range). A `spec-writer` or runner running that literal command from the
      repo root does not get "the markers"; it gets every historical mention
      of the phrase across the whole repo, and has to pick the one real hit
      out of the flood by hand — exactly the listing-by-hand the instruction
      says to avoid. Existing archived pieces scope the same idiom
      (`git grep -n "NO SPEC:" -- dialectica/rust-lib`, in
      `2026-09-25-feed-row-contract/tasks.md:84`); the new Step 1 text drops
      the scope precedent it could have followed. Worse, this piece's own
      prose is now part of the noise for every future run of this command,
      since `RUNNER.md`, `tester.md`, `spec-writer.md`, `dev-writer.md`,
      `spec-test-reviewer.md` and `README.md` all now contain the literal
      string `NO SPEC:` in prose describing the mechanism itself.
      **Measured:** `git grep -n "NO SPEC:"` run from the repo root on this
      tree returns 150 lines, spanning six `.claude/agents/*.md` files, this
      piece's own `proposal.md`/`design.md`/`tasks.md`, dozens of archived
      changes' `design.md`/`tasks.md`/`proposal.md`, two other in-flight
      changes' `findings/`, and the ~45 lines that are actual live `// NO
      SPEC:` code/test markers in `dialectica/` and `dialectica-ui/`. The
      real markers are a small minority of the output.

      **Fixed** (this commit). Step 1 now scopes by the piece's own diff
      rather than by path: `git diff --name-only -G "NO SPEC:"
      origin/main...HEAD` lists the files where this piece added or removed a
      marker line, and `git grep -n "NO SPEC:"` over those files shows each
      one. A path scope was the first option and was dropped: scoped to
      `dialectica dialectica-ui`, `git grep -c -F "NO SPEC:"` still returns
      24 files of markers already on `main`, and a path list is one more
      hand-maintained list. Measured: on #165's squash commit,
      `git diff --name-only -G "NO SPEC:" 0177eb11^ 0177eb11` returns
      exactly `authoring.rs` and `transport.rs`, the two files whose diff
      touches a marker line, and `git grep -n "NO SPEC:" 0177eb11 --` those
      two files returns four lines. Over this piece's own range it returns
      only this piece's prose files, which is the honest answer for a piece
      whose subject is the mechanism. `git diff` without `-U0` is not
      offered: `-G` selects files, not hunks, so the patch form printed 47 KB
      for the same commit.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:376-378` — "escalate only
      what it returns as a product decision" names a signal the `spec-writer`
      is never told to produce.
      **Scenario:** a `NO SPEC:` marker is routed to a fresh `spec-writer` per
      Step 1. The `spec-writer` decides it, per its own unmodified mandate in
      `spec-writer.md` ("For each, decide whether the choice was right, then
      either add the requirement or say the behaviour should change" — no
      branch for "this is not mine to decide"). Its hand-back is read by the
      runner, which — per the new RUNNER.md text — must distinguish "what it
      returns as a product decision" (escalate to the owner, with the options
      stated) from an ordinary decision (apply it, no escalation). Nothing in
      `spec-writer.md`, or anywhere else under `.claude/agents/`, defines what
      a hand-back marking something as a "product decision" looks like, so a
      `spec-writer` read literally has no way to produce that signal and a
      runner read literally has no way to recognise it in a hand-back that
      never uses the phrase. The likely outcome is that nothing is ever
      escalated (the `spec-writer` always just decides), which happens to be
      safe, but the instruction as written asks the runner to act on a
      distinction the flow gives it no way to observe.
      **Measured:** `git grep -n "product decision" .claude/agents/
      openspec/changes/171-workflow-rules` returns exactly two lines,
      `RUNNER.md:377` and `proposal.md:45` — both stating the rule, neither
      one telling the `spec-writer` how to invoke it. `spec-writer.md` and
      `tester.md` (also touched by this piece) contain zero occurrences. This
      traces to issue #169's own text, which contains the same "returns as a
      product decision" phrase with the same missing mechanism — so the gap
      predates this diff's wording, but implementing #169 was this piece's
      job, and closing that gap was in scope for it.

      **Fixed** (this commit), in the file the runner reads, since
      `spec-writer.md` is outside this piece's authorisation beyond its one
      corrected sentence. Step 1 now says the `spec-writer`'s own file gives
      it no such category, so the runner's brief asks for it: name which
      markers, if any, neither the issue nor the specs settle, so that the
      choice is the owner's. A hand-back that names none has decided them
      all. That gives the runner something observable to act on, and the
      signal now exists because the brief creates it. Measured after the
      edit: `git grep -n -i "product decision" -- .claude/agents` still
      returns only `RUNNER.md`, which is the point — the rule and the request
      that produces its input are in the one file whose reader sends the
      brief.

## Areas checked and clean

- **The pathspec that finds the change folder after an archive**
  (`git ls-files -- "openspec/changes/<name>/tasks.md"
  "openspec/changes/archive/????-??-??-<name>/tasks.md"`, duplicated
  byte-for-byte in `RUNNER.md:94` and `closer.md:78`) is correct. Run against
  this tree it returns exactly the live folder for `171-workflow-rules`, the
  single archived folder for `op-clock`, `time-pegged-clock` and
  `home-screen-key-states` (not the `-followup` sibling), and nothing for
  `clock` — matching every claim `design.md` makes about it, including the
  ruled-out suffix-glob alternative (`openspec/changes/*clock/tasks.md`
  does return both `op-clock` and `time-pegged-clock`, confirming why it was
  dropped) and the archived-folder date-prefix format (`git ls-files --
  "openspec/changes/archive/*/proposal.md"` shows all 47 archived folders
  use the `YYYY-MM-DD-` prefix the glob assumes).
- **The `--admin` prohibition and `BLOCKED` handling in `closer.md`** reads
  correctly against Step 6 and "What you never do," and the `gh pr view`
  fields it names (`mergeStateStatus,mergeable,statusCheckRollup,
  reviewDecision`) are valid — checked by running the exact command against
  this PR.
- **The red-CI loop structurally closes**, including a second red run: Step
  4's fixer path returns to Step 3 (untick the re-review row, size and run a
  round, re-tick), and only then re-dispatches the `closer`, whose Step 1
  re-finds the same archived folder (single path, per the pathspec above) and
  whose Step 3 correctly skips re-running `openspec archive` while still
  committing/pushing a `findings/` deletion if Step 1 made one. The closer's
  own findings-gate grep is re-run fresh against the actual directory
  contents on every dispatch, so a stale tick on the closer's own
  "findings all ticked" row from an earlier dispatch cannot mask a new,
  unanswered finding that a re-review round added afterward.
- **The tester.md changes are internally consistent**: "closed" markers
  (reworded or removed) versus "open" markers (kept, reported, and covered by
  "must not remove") is an exhaustive, non-overlapping split, and the
  regression-test protection is preserved unchanged alongside it.
- **The #133 restoration is an exact match.** The replaced paragraph in
  `README.md:369-370` is byte-identical to both the issue's quoted text and
  `git show 51ab7f8^:.claude/agents/README.md` lines 374-375, including the
  original line break, and `dev-writer.md` is untouched as claimed.
- I did not find a contradiction between `RUNNER.md`'s new sequence and the
  unmodified parts of `closer.md`, `spec-writer.md` or `tester.md` beyond the
  two gaps above — in particular, the "fresh `spec-writer`" instruction does
  not conflict with `spec-writer.md`'s generic "called back" language, since
  that language is written role-generically and works the same whether the
  dispatch is a continuation or a fresh instance.
- No source diff, so `cargo mutants` and the dependency/CI-gate checks in the
  "Also check" section do not apply — confirmed `.github/workflows/ci.yml`
  contains no reference to `openspec/` or `.claude/agents/` that this change
  could break.
