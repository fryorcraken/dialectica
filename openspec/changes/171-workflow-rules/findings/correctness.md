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

## Re-review `9dc235c..34fd428`

Correctness only. I read the range's diff to `RUNNER.md`, `closer.md`,
`README.md` and `dev-writer.md`, then read `closer.md` Steps 1 to 3 and "Your
report" and `RUNNER.md`'s "Dispatching", step 3 and "The `closer`, and what
comes back" in full, taking each as its agent would. I ran commands against
this tree and against a scratch repository under `./tmp/r2/` (git 2.55.0,
`commit.gpgsign false` set in that repository only, since deleted).

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:282-310` — the four steps
      are sent to "an agent whose tree holds uncommitted changes", but an
      untracked-only change is uncommitted and never blocks a rebase. On such
      a tree, step 4 fails and reports that the patch did not apply. Neither
      text says what that failure means.
      **Scenario:** a correctness reviewer's only mutation is a new probe
      file it never `git add`ed. Its hand-back says it left that file in its
      tree, as `code-reviewer.md` requires. Its tick conflicts in the review
      round, and the runner's continuation message carries the four steps,
      since the tree does hold an uncommitted change. Step 1 writes an empty
      patch, because `git diff HEAD` does not see untracked files. Step 2
      leaves the untracked file where it is. Step 3 rebases normally. Step 4
      exits 128. The agent does as step 4 says and reports that the patch did
      not apply. That reads as lost mutation evidence, although the file is
      still in the tree and nothing was lost. `RUNNER.md` routes no report
      of a failed apply, and it cannot tell this case from a genuine
      failure. The same happens for any agent sent the steps with a tree that
      holds nothing tracked. Severity: low. Nothing is lost, and the rebase
      itself works, but an unattended runner gets a false alarm about
      evidence it has been told only it may discard. Two readings would fix
      it: tie the steps to `git rebase`'s own refusal (`cannot rebase: You
      have unstaged changes`) rather than to "uncommitted changes", or say
      that an empty patch file means there is nothing to re-apply.
      **Measured** (scratch): with only untracked files in the tree,
      `git status --porcelain` → `?? other_lane.txt` / `?? probe_staged.txt`
      / `?? probe_untracked.txt`. `git diff --binary
      --output=tmp/u2.patch HEAD` exited 0. `git apply tmp/u2.patch` →
      `error: No valid patches in input (allow with "--allow-empty")`, exit
      128. The first run below shows an untracked file does not stop
      `git rebase`.

      **Fixed** (this commit), by the second reading. After the four steps,
      `RUNNER.md` now says an untracked file is in none of them (`git diff`
      does not see it, the restore leaves it, and it does not stop the rebase
      unless an incoming commit adds a file at its path), that a staged new
      file comes back untracked (your note), and that a tree holding nothing
      else saves an empty patch which `git apply` refuses with `error: No
      valid patches in input`, meaning nothing tracked was saved, not that
      evidence was lost. The runner carries that sentence in its message, so
      the agent reports it as such. Re-measured in a scratch repository under
      `./tmp/` (git 2.55.0, since deleted): one untracked file only, the save
      exited 0, the restore left `?? probe.txt`, and `git apply` exited 128
      with that error; a staged new file saved, restored and re-applied came
      back as `?? staged-new.txt`. Not chosen: tying the steps to the
      rebase's own refusal, which would reorder the four steps the proposal
      sets out; and `git apply --allow-empty`, since the proposal quotes step
      4 as a plain `git apply` in the steps and in the standing-test
      inventory. `design.md`'s entry on the patch around the rebase records
      both, with a third, a precondition check before step 1.

The brief's known gap, the untracked mutation: it **does not break the
rebase, and the evidence stays behind in place**, measured. The scratch tree
was a reviewer branch with a tick one row from the piece's tick. It held an
unstaged edit to `code.txt`, a staged new `probe_staged.txt` and an untracked
`probe_untracked.txt`, so `git status --porcelain --ignored` printed
` M code.txt` / `A  probe_staged.txt` / `?? probe_untracked.txt`. The patch
from step 1 held `code.txt` and `probe_staged.txt` and not the untracked
file. After step 2 the status was `?? probe_untracked.txt` / `!! tmp/`, so the
staged new file left both the index and the tree. Step 3's `git rebase piece`
stopped on `CONFLICT (content): Merge conflict in tasks.md` with the untracked
file present. It was resolved and added, and `git rebase --continue` printed
`Successfully rebased and updated refs/heads/rev.` Step 4 applied with no
output. The status was then ` M code.txt` / `?? probe_staged.txt` /
`?? probe_untracked.txt` / `!! tmp/`. One detail sits beside `RUNNER.md:309`'s
"come back unstaged": a staged *new* file comes back **untracked**. It is
still in the tree and in the patch, so nothing is lost. That is a note, not a
box. The one input that does stop the rebase is an untracked file at a path
an incoming commit adds. The scratch run gave `error: The following untracked
working tree files would be overwritten by checkout: other_lane.txt` /
`error: could not detach HEAD`. That aborts before any change, so the tree and
the patch are both intact. In the review round the incoming commits are other
lanes' ticks and findings files, which a reviewer does not hold untracked, so
I found no reachable case.

Verified against the code rather than the outcomes, and clean:

- **Round 1, first box (fast-forward every `dev-writer` pass):** closed. The
  per-agent sequence (`RUNNER.md:242-245`), the already-pushed paragraph
  (`:260-269`), "How many at once" (`:363`), the red-run fixer (`:655-657`)
  and the prune sequence (`:443-446`) all now say fast-forward, or point to
  "Dispatching". `git grep -n -i "cherry-pick" -- .claude/agents/RUNNER.md
  .claude/agents/README.md` leaves no passage that sends a `dev-writer`'s
  commits by cherry-pick. `dev-writer.md:203`'s "The runner cherry-picks
  your commits" remains. `design.md:826-828` records it as deliberate, and
  it changes nothing the `dev-writer` does. I walked every agent against the
  rule. A `spec-writer`, a `tester` and a reviewer never push, so
  cherry-picking them diverges nothing. A `dev-writer` pass forks from the
  runner's HEAD, including local-only reviewer ticks. Its push carries those
  ticks to the remote, and `--ff-only` then leaves local and remote equal.
  `--ff-only` also succeeds when a pass made no push, so the rule is safe
  even where its premise does not hold.
- **Round 1, second box (rebase with mutations in the tree):** closed, apart
  from the empty-patch case above. The steps ran end to end.
- **Round 1, third box (`README.md` push sentence):** closed.
  `README.md:170-173` names both `closer` push points and every `dev-writer`
  pass.
- **The `closer`'s Step 3 check before the push, and its three outcomes**
  (`closer.md:281-324`). The check sits after the archive commit and before
  the upstream check and the push. Nothing between them makes a commit, so
  `HEAD^ HEAD` is still the archive commit when it runs. The three outcomes
  cover every case. The first ("Push refused") carries its own "if this run
  made the archive commit" condition, so it also covers the already-archived
  path, which skips the check (`:233-237`) and otherwise goes straight to
  Step 4. "Your report" (`:478-480`) asks for the check's result with a
  refusal, and `RUNNER.md:694-697` acts on it by unticking and recording a
  round before it goes to the owner. That closes the hole a later
  `closer` would open by skipping the check. A stop at the upstream check
  after the archive commit is covered too: "Your report" asks for the files
  wherever the archive changed `openspec/specs/`, which is not conditional
  on the push.
- **The returns, with no count** (`RUNNER.md:647-650`). `git grep -n -i -E
  "(two|three|four|five|six|seven) (things|returns|outcomes) (come|that
  come)"` over `.claude/agents/` and `proposal.md` returns nothing. `closer.md`
  also stops on a zero-box findings file, on a `git ls-files` that returns more
  than one path or none, and on a wrong upstream. None of those is named in
  `RUNNER.md`, and "route it by what it is" covers each with the evidence the
  `closer` reports. The new `BLOCKED` return matches `closer.md:426-433`.
- **The pre-tick check** (`RUNNER.md:595-609`). `git grep -l -F
  "c222c37..9dc235c" -- openspec/changes/171-workflow-rules/findings/` lists
  all six: `architecture.md`, `correctness.md`, `design-review.md`,
  `readability.md`, `security.md` and `spec-test.md`. The same search for
  `9dc235c..34fd428` finds only `tasks.md:26`, the runner's record line, which
  sits outside `findings/`. So before this round no lane's file falsely
  claims it ran. A folder with no `findings/`
  (`openspec/changes/archive/2099-01-01-171-workflow-rules/findings/`) and a
  range no file holds (`zzzzzzz..yyyyyyy`) both print nothing, so the check
  fails closed. A lane's file cannot hold a round's range before that round
  exists: the range ends at a commit made after every earlier outcome was
  written.

## Re-review round 3 `34fd428..dc1390a`

Correctness only, narrowed as briefed. I read the range's diff to `RUNNER.md`,
`closer.md` and `dev-writer.md`, then `RUNNER.md`'s "What you read",
"Dispatching" and step 3 in full as the runner would, and ran the new
mechanics in a scratch repository under `./tmp/r3/` (git 2.55.0,
`commit.gpgsign false` in that repository only, since deleted).

- [x] **owner** — `openspec/changes/171-workflow-rules/tasks.md:25-26`
      (the runner's round lines), against `RUNNER.md:618-638` — this piece's
      own rounds 1 and 2 were briefed with the un-numbered forms, and the
      exception that says so lives only in `proposal.md:242-250` and
      `design.md:456-462`, which `RUNNER.md:90-92` tells the runner to point
      at, not read. A runner following `RUNNER.md` literally cannot tick this
      piece's re-review row.
      **Scenario:** the row has never been ticked (`tasks.md:24` is `[ ]`),
      so the next tick closes rounds 1, 2 and 3 ("one recorded since the row
      was last ticked"). A runner rebuilding its state from the `## Stages`
      block after a compaction, which is the case the check exists for, runs
      the numbered command for each round. Round 3 can list its lanes' files
      once this round's commits land. Rounds 1 and 2 list nothing. `RUNNER.md:634-637`
      then says: this lane "has not finished the round … continue that
      reviewer, or dispatch a fresh one for the lane — which gets its own
      line — and do not tick". That is twelve re-dispatches over two ranges
      that were reviewed and answered, or a piece that stops. It fails
      closed, so no unreviewed work merges, but the loop the runner narrowed
      this round to converge restarts. Nothing in round lines 1 and 2 says
      which form they were briefed with. Neither does the rest of the stage
      block. The only on-tree mention outside the contract is `tasks.md:344`,
      in the `dev-writer`'s checklist, below the block the runner reads.
      Remedy, and why this is not addressed to a writer: annotate round lines
      1 and 2 with the forms their briefs used (``## Re-review `<range>` ``
      and ``**re-review `<range>`: no findings**``). Those lines are the
      runner's own record ("The record, the tick and the untick are commits
      you make"), so neither writer may edit them. No role-file change is
      needed, and whoever makes the annotation flips this box. Severity:
      medium, because a literal runner does the wrong thing, and it happens
      only on this piece.
      **Measured:** on this tree, `git grep -l -F -e '## Re-review round 1
      `c222c37..9dc235c`' -e '**re-review round 1 `c222c37..9dc235c`: no
      findings**' -- openspec/changes/171-workflow-rules/findings/` prints
      nothing, and so does the same command for `round 2 `9dc235c..34fd428``.
      The un-numbered forms for round 2 list all six files. `git grep -n -i -e
      "predate" -e "un-numbered" -e "unnumbered" -e "without the number" --
      .claude/agents openspec/changes/171-workflow-rules/tasks.md` returns
      only `tasks.md:344`.

      **Fixed** in `549c09c` (the runner's commit), exactly as the remedy
      asked; verified by the `spec-writer`, who flips this box. The commit
      changes only `tasks.md`, and only round lines 1 and 2 under the
      re-review row: each keeps its text and gains "Pre-numbering round:
      briefed with the un-numbered forms, so check it with" the full
      `git grep -l -F` command for its own range, in the two un-numbered
      forms the remedy names (``## Re-review `<range>` `` and
      ``**re-review `<range>`: no findings**``), single-quoted, over this
      change's `findings/`. Round 1 carries `c222c37..9dc235c` and round 2
      `9dc235c..34fd428`, each its own line's range. Both commands, run as
      written on this tree at `fd96a3d`, list all six findings files:
      `architecture.md`, `correctness.md`, `design-review.md`,
      `readability.md`, `security.md` and `spec-test.md`. So a runner
      rebuilding its state from the stage block now reads, on the line
      itself, which check closes each of those rounds, and no longer
      re-dispatches twelve lanes. Nothing outside the runner's own lines
      changed, and no role file, as the remedy said none needed to.
      That agrees with `proposal.md`'s "This piece's rounds 1 and 2 predate
      the round number" and `design.md`'s matching paragraph, which already
      held that those rounds are checked by the un-numbered forms. The box
      was addressed to the **owner** only because no writer may edit the
      runner's round lines; no owner decision was involved, and the owner
      may object on return.

**Round 2's box (untracked-only tree) is fixed**, measured rather than taken
from the outcome. On a reviewer branch whose tick conflicts with the piece's
adjacent tick, and whose only change was an untracked `probe.txt`, I ran the
four steps as written. `mkdir -p tmp` and `git diff --binary
--output=tmp/uncommitted.patch HEAD` exited 0. After the restore, the status was
`?? probe.txt` / `!! tmp/`. `git rebase piece` stopped on `CONFLICT (content):
Merge conflict in tasks.md`, and after `add` and `--continue` printed
`Successfully rebased`. `git apply tmp/uncommitted.patch` exited 128 with
`error: No valid patches in input (allow with "--allow-empty")`, which is the
prefix `RUNNER.md:323-324` quotes. `probe.txt` was still in the tree. A staged
new file came back `?? staged-new.txt` after save, restore and apply, as the
new sentence says. The paragraph now tells the runner to carry the meaning of
that refusal in its message, which was the missing piece.

**The round-numbered pre-tick command** works as `RUNNER.md:626` gives it. The
single quotes pass backticks and `**` through: the command ran here with no
approval prompt, and it matched both forms. In the scratch repository:

- the round 1 forms listed the files holding a round 1 heading or verdict box;
- the same file set held `## Re-review round 11 …`, `**re-review round 12 …`,
  a round 1 box over a longer SHA (`…e4f5a6bf`), and prose citing
  ``round 2 `a1b2c3d..e4f5a6b` `` without the heading prefix. The round 1
  command did not list a file on any of those, and the round 2 command
  listed nothing. The closing backtick in both patterns is what ends the
  number and the range.

On the re-dispatch, a rejected run's `## Re-review round 1 …` heading did not
satisfy round 2, and appending the round 2 verdict box made round 2 list
`security.md`. On this tree, round 3's command printed nothing before any
lane of this round had committed, so it fails closed.

**A re-dispatch getting its own line** is consistent end to end. The earlier
round's check exempts a lane a later round re-ran over the same range. That
round's check needs the lane's file, and the number keeps the rejected run's
record from satisfying it. A skipped line dispatches nothing, so it cannot
trigger the exemption.

**`dev-writer.md`'s route words** match `RUNNER.md`. The four sentences now say
the runner "brings" the commits onto `piece/<name>`, and for a `dev-writer`
`RUNNER.md:242-245` makes that a fast-forward. `git grep -n -i -F
"cherry-pick" -- .claude/agents/dev-writer.md` returns only `:158`. That line
is about commits made directly on `piece/<name>` in the wrong tree, where
neither route applies, so it is accurate as it stands. The stage-row pointer
at `:131-132` is also accurate: `spec-writer.md:47-53` says when adjacent
ticks conflict and points to `RUNNER.md` for who resolves them.

**`closer.md`'s edits** are clean. The Step 3 sentence now parses. "Every stop
this file names ends your turn" matches every `stop` in the file. The only
other use, at `:160`, is `git merge` stopping on a conflict, which the next
paragraph turns into a closer stop.

Below the box threshold, in prose only:

- `RUNNER.md:621-622` runs the check "once every lane's findings commit is on
  your HEAD". A lane that stalled has no findings commit, so read literally
  the precondition never holds, the check never runs and the row stays
  unticked. That is fail-closed, and the following sentences say what to do
  about a missing lane. `proposal.md:206`'s "once the round's commits are on
  it" is the looser wording.
- The heading has a capital `Re-review` and the verdict box a lowercase
  `re-review`. `-F` without `-i` is case-sensitive, so a reviewer who
  capitalises the box fails the check and is re-dispatched. That is also
  fail-closed, and the brief gives the form whole.
