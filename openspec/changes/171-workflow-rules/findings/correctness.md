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

## Re-review `c222c37..9dc235c`

Correctness only. I read `RUNNER.md`, `closer.md`, `README.md`,
`spec-writer.md` and `dev-writer.md` (unchanged, but the agent RUNNER.md's
new fast-forward rule is about) as the agent each addresses, and walked the
paths in the brief. Commands were run against this tree or a scratch
repository under `./tmp/p171/` (bare remote, runner clone, agent worktrees),
since deleted. Scratch commits used `--no-gpg-sign`: without it, the first
cherry-pick failed with `gpg: signing failed: Timeout` after pinentry launched
on `/dev/pts/3`.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:240-243` and `:258-262`
      — the fast-forward rule names "the `dev-writer`'s first pass" as the
      agent whose commits are already on the remote piece ref. But the
      `dev-writer` pushes on every pass, so a runner following the text
      cherry-picks its later passes, and the local and remote piece refs
      diverge.
      `dev-writer.md:221-223` has the dev-writer push on its findings pass
      ("On the findings pass it always does: commit, push, never open a
      second"). The same sequence runs whenever the dev-writer is dispatched
      again: after a `spec-writer` callback changes behaviour (RUNNER.md step
      1, `:452-453`), as a red-CI fixer (`:610`), and as a conflict resolver.
      Read literally, RUNNER.md gives a cherry-pick for all of them except the
      conflict resolver. `:258-260` says "in this flow, the `dev-writer`'s
      first pass", which reads as the complete list. `:331` says "cherry-pick
      and commit" for the `dev-writer` row. `:602-603` tells the runner to
      dispatch a red-CI fixer with "its commits cherry-picked back".
      `README.md:170-172` says the `dev-writer` pushes "at the end of its first
      pass". The piece's own `design.md` disagrees with itself here. `:716-718`
      counts "a findings-pass writer's" push among the pushes a copy would
      break. `:741-742` then calls the first pass "the one instance today".
      **Scenario:** after the review round, the `dev-writer`'s findings pass
      forks from the runner's HEAD (reviewer ticks included, not pushed). It
      pushes `HEAD:refs/heads/piece/<name>`, which fast-forwards the remote,
      and hands back. The runner is not looking at a first pass, so it
      cherry-picks and gets a copy with a new SHA. Next the review round runs
      and the runner records it. Then the `closer`'s Step 3 push, or the
      Step 2 push after merging `main`, is refused. The `closer` stops and the
      runner parks the piece on the owner (RUNNER.md `:635-646`). The
      rule was written to prevent exactly this, but its example sends the
      runner into it. The same thing happens on the red-CI path through
      `:602-603`.
      **Measured:** this piece did it. `git rev-parse
      origin/piece/171-workflow-rules` → `619e6799…`, which is the
      `dev-writer`'s findings-pass commit "Add the clean re-review verdict box,
      move the closer's findings/ deletion to Step 3, and fast-forward to
      already-pushed branches". `git branch -r --contains 749cb13b` (an earlier
      findings-pass commit) → `origin/piece/171-workflow-rules`. The reflog
      shows the runner brought these on with `merge 619e679: Fast-forward`
      and `cherry-pick: fast-forward`. Those are safe here only because HEAD
      was the parent, and the text did not tell the runner to do it.
      Scratch reproduction: the runner made a local-only tick `b594425`. The
      dev-writer then committed `cd018a4` on it and ran `git push origin
      HEAD:refs/heads/piece/x` → `b09b282..cd018a4  HEAD -> piece/x`. The
      runner ran `git cherry-pick --no-gpg-sign dw` → `[piece/x 74c69c4]`.
      The closer forked, committed, and ran `git push origin
      HEAD:refs/heads/piece/x` → `! [rejected]        HEAD -> piece/x
      (non-fast-forward)`. After `git fetch origin`, RUNNER.md's diagnostic
      `git log --oneline --left-right --cherry-mark HEAD...origin/piece/x`
      printed `= 74c69c4 dev-writer findings pass` / `= cd018a4 dev-writer
      findings pass`. The diagnostic itself works as documented.
      Severity: high on the findings pass, which every piece with a
      `dev-writer` finding reaches. The owner has to step in, and the runner
      may not repair it.

      **Fixed** (this commit), as the contract now reads: `proposal.md` names
      every `dev-writer` pass. `RUNNER.md`'s per-agent sequence says "every
      `dev-writer` pass"; the already-pushed paragraph names the `dev-writer`
      by condition, pushing "on every pass — a findings pass, a red-CI fix or
      a pass after a `spec-writer` callback as much as its first"; "How many
      at once" says to bring commits on by cherry-pick or fast-forward "where
      'Dispatching' says"; the red-run fixer's commits are "brought back as
      'Dispatching' says (a `dev-writer` fixer has pushed, so fast-forward to
      it)"; and "One piece is one PR" no longer says "cherry-pick" for the
      `dev-writer` or the `closer` (`readability.md`'s re-review third box).
      `README.md:170-172` is the third box below. `design.md`'s ":741-742 the
      one instance today" now names every pass and cites this box. Measured
      after the edit: `git grep -n -e "first pass" -- .claude/agents/RUNNER.md`
      returns only "opens the PR, as the last act of its first pass", which is
      true.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:273-278` — the
      conflict rule tells the agent to rebase its own branch onto
      `piece/<name>`. `code-reviewer.md:178` tells a mutating reviewer to leave
      its mutations uncommitted in its tree, and `git rebase` refuses to run
      in a dirty tree. The brief gives the reviewer no way round this that the
      rest of the flow allows.
      **Scenario:** first review round, picked in template order. RUNNER.md
      `:280-287` says every pick after the first stops. The correctness
      reviewer left a hand-mutated line in its tree, as its file tells it to,
      and RUNNER.md `:418` expects that ("recovering a mutation an agent left
      uncommitted"). The runner aborts the pick of the security reviewer's
      tick and continues that reviewer with `SendMessage` to rebase. `git
      rebase piece/<name>` refuses. git's hint says to "commit or stash". Both
      are ruled out:
      - a commit would put the mutation on the piece;
      - `CLAUDE.md:293-295` bans a bare `git stash`;
      - `git checkout -- .` destroys the evidence `code-reviewer.md` says only
        the runner may discard.

      The agent is left to invent a route or stop. Every later pick then waits
      on it.
      **Measured** (scratch): three reviewers forked from one HEAD and each
      ticked an adjacent row. `git cherry-pick --no-gpg-sign rev-sec` →
      `[piece/x 6035cba] tick security`. Then `git cherry-pick --no-gpg-sign
      rev-read` → `CONFLICT (content): Merge conflict in tasks.md`, which
      confirms `:283-286`. After `git cherry-pick --abort`, with an
      uncommitted edit in `code.txt` on the reviewer's branch, `git rebase
      piece/x` → `error: cannot rebase: You have unstaged changes.` /
      `error: Please commit or stash them.` `git rebase --autostash piece/x`
      got past that (`Created autostash: 118f62e`) and stopped at the
      expected `tasks.md` conflict. I did not finish that round-trip:
      `--continue` hung on the signing prompt, because the rebase state had
      recorded `-S` from `commit.gpgsign`. So whether the mutation is
      restored intact is unmeasured.
      Severity: medium. It stalls the first review round for any
      reviewer that mutated, and it cannot be fixed in `code-reviewer.md`
      under this piece's authorisation.

      **Fixed** (this commit) on the `RUNNER.md` side, with the role-file
      home deferred to the owner (`proposal.md`, "Out of scope", "The
      reviewer role files on rebasing with mutations in the tree", and PR
      #174's follow-ups). "Dispatching" now has, after the conflict rule, the
      steps the runner's continuation message carries for an agent whose
      tree holds uncommitted changes: `mkdir -p tmp` and
      `git diff --binary --output=tmp/uncommitted.patch HEAD`;
      `git restore --source=HEAD --staged --worktree -- .`; rebase onto
      `piece/<name>` and resolve; `git apply tmp/uncommitted.patch`, reporting
      whether it applied. Measured end to end in a scratch repository under
      `./tmp/` (git 2.55.0, since deleted), including the `tasks.md` conflict
      mid-rebase: the dirty rebase refused as you measured; `git diff
      --output` refused until `tmp/` existed; after the restore only the
      ignored `tmp/` remained; the rebase stopped on the conflict and
      continued; the patch re-applied and `git diff HEAD` matched it exactly,
      both edits now unstaged; the runner's next pick applied cleanly. Your
      unmeasured `--autostash` restore is recorded as not chosen, with the
      shared stash list as the reason. The signing hang is answered too, and
      `RUNNER.md` names no flag: with signing forced on and `gpg.program`
      pointed at a missing binary, `git rebase --no-gpg-sign` followed by a
      flagless `git rebase --continue` completed without trying to sign,
      while a plain `git commit` failed. `design.md` has the entry.

- [x] **`dev-writer`** — `.claude/agents/README.md:170-172` — "the
      `closer` pushes it again after the archive commit" no longer matches
      `closer.md`. The closer now pushes in two more places:
      - Step 2, `:179-183`, after merging `main` and before any archive;
      - Step 3, `:233-235`, on a re-dispatch that makes no archive commit
        ("push HEAD as below whether or not you deleted anything").

      This piece rewrote line 131 of the same file to cover how commits reach
      the piece, but left this sentence alone.
      **Scenario:** after a refused push, the runner uses README's branch
      section to work out which pushes can have moved the remote ref. It rules
      out the `closer`'s Step 2 push, because README says the closer pushes
      only after an archive. Its report to the owner then misattributes the
      commit the remote holds. Severity: low. This is prose that contradicts
      the role file, not a wrong command.
      **Measured:** `git grep -n -F "push" -- .claude/agents/` shows
      `README.md:172` "`closer` pushes it again after the archive commit"
      alongside `closer.md:182` and `:233`, which are the two other push
      points.

      **Fixed** (this commit). The sentence now reads: the `dev-writer`
      pushes "at the end of every pass, and opens the PR on its first; the
      `closer` pushes it after merging `main` and in its Step 3, whether or
      not it made an archive commit — [`closer.md`](closer.md) says when".
      That covers both push points you name and the first box's every-pass
      correction, and points to `closer.md` rather than enumerating in a way
      that could go stale. Measured after the edit:
      `git grep -n -F "pushes it again after the archive" -- .claude/agents`
      returns nothing.

Clean, and checked by running the command where there was one:

- `git diff --name-only -G "NO SPEC:" origin/main...HEAD` works as RUNNER.md
  step 1 says. On this tree it lists eight files, all prose belonging to this
  piece.
- Tick conflicts on adjacent rows happen exactly as RUNNER.md `:280-287` and
  `spec-writer.md:47-53` describe.
- The refused-push diagnostic prints `=` for a copied pair, as documented.
- `GIT_EDITOR` resolves to `true` in agent shells (`git var GIT_EDITOR` →
  `true`), so the closer's bare `git merge origin/main` and a resolver's
  `rebase --continue` cannot hang on an editor, even though `test -t 0` and
  `test -t 1` both succeed.
- git 2.55.0 supports `git show --remerge-diff`.

The closer's paths are consistent with each other and with RUNNER.md:

- the first dispatch through a clean merge of `main`;
- a merge conflict, meaning abort, report the paths, the runner fast-forwards
  to an unchanged branch, a writer merges, and the merge is re-reviewed;
- the `findings/` deletion moved to the start of Step 3;
- the spec-changing archive check, where HEAD is the archive commit at that
  point even after a Step 2 merge;
- the re-dispatch that finds the change archived.

On the red-run path after the archive, the runner fast-forwards to the
`closer` first, and the untick lands in the archived `tasks.md`. It breaks
only through the first box above.

The runner's re-review round on this piece used ticked verdict boxes
(`git log` shows `0f55b92d`, `9bbaf7da`). That matches RUNNER.md `:521-537`
and passes the `closer`'s two gates as designed.

**Owner, outside the authorised scope:** the second box is best closed in
`code-reviewer.md`, by saying what a reviewer does with its mutations when it
is continued to rebase. That file is not this piece's to edit, so the box is
addressed to the RUNNER.md side. Whether the role file should change is the
owner's call.
