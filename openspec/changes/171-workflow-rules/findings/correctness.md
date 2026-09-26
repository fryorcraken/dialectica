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

## Re-review round 5 `dc1390a..d1c8726`

- [x] **re-review round 5 `dc1390a..d1c8726`: no findings** — read `git diff dc1390a..d1c8726` of `RUNNER.md`, `proposal.md`, `design.md` and `tasks.md`, `RUNNER.md` "Record the call" through the pre-tick check as a runner would, and ran the pre-tick check for rounds 1 to 4 on this tree; clean

Correctness only, narrowed as briefed. Nothing reaches medium.

**Round 3's owner box is closed.** At `07f0a5f`, the commands that
`tasks.md:25` and `:26` now carry on round lines 1 and 2 each list all six
findings files. The numbered command for round 3 lists five, with no
`readability.md`. Round 4's line re-runs readability over the same range, so
round 3's check skips that lane, and round 4's command lists `readability.md`.
A runner rebuilding from the stage block therefore reads each round's check on
the line itself and can tick rounds 1 to 4 without re-dispatching anything. The
row is still unticked only because round 5's four lanes are in flight.

**The restated rule, walked case by case** (`RUNNER.md:604-616`, check at
`:625-645`):

- *Stall, then continuation.* No record is committed, so the continuation is
  "finishing a round it has not yet recorded" and gets no line. It writes the
  original round's form, and that round's check needs its file. Right line,
  right check. The same holds for continuing an agent to commit
  (`RUNNER.md:47-52`).
- *Rejected run, then continuation.* The record is committed and the
  continuation adds review, so it gets the next number over the same range.
  The old round's check skips the lane because a later round ran it again. The
  new round's check fails until the continuation writes the new form, so the
  rejected record satisfies nothing. Right on both counts. If the rejected run
  had not committed yet, it gets no line, and the old round's check can pass
  only on a commit that already holds the continuation's review. That is also
  right.
- *Rebase continuation.* The record is committed and nothing is added, so it
  gets no line and keeps the round's own record. That round's check lists the
  file once the pick lands. Right.
- *Fresh re-dispatch.* It gets a line, the earlier round skips the lane, and
  its own check needs the new form. Right. Round 4 on this piece is a live
  instance, and it checks out as above.

Below the box threshold, in prose only:

- `RUNNER.md:610-611` and `proposal.md:192-193` read "finishing a round it has
  not yet recorded, such as after a stall, committing, or rebasing". This
  parses either as three cases or as three examples of "not yet recorded", and
  the second reading is false for a rebase. `design.md:453` gives the three
  cases without "such as", which is clearer. The governing clause, "adds no
  review to a record already committed", gives the right answer under both
  readings, so no outcome changes.
- "Committed" does not say where: on the agent's branch or on the runner's
  HEAD. It only matters if a runner picks a rejected run's commit, then loses
  track of a continuation it recorded no line for, and ticks. That needs the
  line to be skipped as well, which is the "runner skips a step" residual
  already in Risks.
- `RUNNER.md` does not say that a round line may carry its own check. The
  round 1 and 2 annotations still work, because they sit on the line the
  runner reads before it runs the check, and they only apply to this piece.

## Re-review round 7 `6d43cda..d1d2165`

- [x] **re-review round 7 `6d43cda..d1d2165`: no findings** — read `RUNNER.md`'s tick paragraph (626-648) literally against `tasks.md`'s round lines 1-7, ran every round's check at `077d8be`, and read `16958b3` and `d1d2165` in `proposal.md` and `design.md`; clean

I followed the tick paragraph as a runner would. The re-review row has never
been ticked (`git log -G 'x\] re-review: every commit'` over `tasks.md` returns
nothing), so the tick closes all seven rounds, none of them marked skipped. For
each round I copied ``round <n> `<range>` `` from the start of its line into the
command in single quotes:

| Round | Forms copied from the line | Listed | Lanes the line ran | Verdict |
|---|---|---|---|---|
| 1 | numbered, `c222c37..9dc235c` | nothing | six | fails closed (see below) |
| 1 | the line's own un-numbered check | all six files | six | passes |
| 2 | numbered, `9dc235c..34fd428` | nothing | six | fails closed (see below) |
| 2 | the line's own un-numbered check | all six files | six | passes |
| 3 | `34fd428..dc1390a` | architecture, correctness, design-review, security, spec-test | six, readability re-run by 4 | passes by the exception clause |
| 4 | `34fd428..dc1390a` | readability | readability | passes |
| 5 | `dc1390a..d1c8726` | correctness, design-review, security, spec-test | those four | passes |
| 6 | `d1c8726..6d43cda` | design-review, spec-test | those two | passes |
| 7 | `6d43cda..d1d2165` | nothing (before any lane of this round committed) | four | correctly withholds the tick |

A runner following the text therefore ticks correctly. It ticks once round 7's
four lanes have committed and not before. It never ticks early on any round. The
copy rule is also what makes round 4 come out right: typed with round 3's
number, readability would have had to be excepted by hand. The paragraph's
substitution is unambiguous, because every round line from 3 on starts with
exactly ``round <n> `<range>` ``, character for character, which is the
substring both patterns carry.

The measurements `proposal.md` and `design.md` cite reproduce at this HEAD.
Round 3's forms list five files (every lane's but readability). Round 4's list
readability alone. Round 5's include `spec-test.md` and `design-review.md`, so
a round 6 check copied from round 5's line would pass on round 5's records.

Below the box threshold, in prose only:

- **Rounds 1 and 2 under the literal text.** `RUNNER.md` says to copy
  ``round <n> `<range>` `` from the line, and for these two rounds that gives
  numbered forms nobody was briefed with. Those forms list nothing, so the
  paragraph says "do not tick". The failure is closed rather than open. Each of
  the two lines names its own check, and those checks list all six files. The
  annotation sits on exactly the text the runner is told to copy from, so a
  runner cannot reach the copy step without reading it. This is the same point
  as the round-5 prose above and only applies to this piece. Low.
- **The optional sentence (`RUNNER.md:641-643`) on its own.** It is correct but
  incomplete when read alone:
  - It states the match but not the consequence: the lane's file is then
    listed, and the runner ticks before the re-run has written anything. A
    reader has to take that from the previous sentence, "It must list…".
  - The subject is too broad. "A number typed from memory" matches the rejected
    record only when it is the wrong number. A number remembered correctly is
    harmless. The "such as" clause carries the real case.
  - "The earlier line's" needs "for a lane run again" to find its referent. The
    sentence does supply that, so it resolves.
  - "Rejected run" has its antecedent at `RUNNER.md:613`, which says the same
    thing for continued runs: "the check below would pass on it before the
    continuation had done anything".

  So the sentence adds the fresh-dispatch case to a reason already stated. It
  can be understood without its neighbours, but only just. One possible
  wording: "Typed from memory, the earlier line's number for a lane run again
  matches the rejected run's record whenever that run left one, and the check
  passes before the re-run has written anything." That is a readability
  preference, not a defect, and no outcome changes. Low.
- **Two words for the same thing.** `proposal.md`'s wrong-line example says "a
  round 6 check run with round 5's start", and `design.md` says "round 5's
  forms". "Start" is the piece's own term for the ``round <n> `<range>` ``
  prefix of a line (`RUNNER.md:600`, and design's "whose only fixed part is
  their start"). A reader could still take it as the range's start SHA, which
  would give `dc1390a..6d43cda`, a form that fails closed and so disproves the
  example. The design wording is the unambiguous one. Low.

## Re-review round 10 `c4b1df5..842758b`

- [x] **`spec-writer`** — `proposal.md:221-227`, mirrored at
      `RUNNER.md:638-644` — for a skip, "a line with the wrong number gets
      the number it should carry" lowers numbers, and a lowered number is one
      an earlier line already carried, so the forms check for it can pass on
      that earlier line's record. This is the failure the number check was
      added to stop, and the rule's own reason says so: "a record written
      under a repeated number cannot be told from the earlier run's".
      **Scenario:** the runner writes the lines 1 to 6, then
      ``round 8 `R` `` (correctness and security), then
      ``round 9 `R` `` (a security re-run, because the round 8 security run
      was not accepted). No line was lost; the runner mistyped 7 as 8. The
      rejected security run left ``## Re-review round 8 `R` `` in
      `security.md`. The number check lists 6, 8, 9, which is a skip. Nothing
      was lost, so "gets its line back" does not apply, and the runner does
      what the other remedy says: position 7 becomes `round 7`, position 8
      becomes `round 8`. Both lanes run again under their new numbers. Round
      7's forms check skips security, because a later round ran it again over
      `R`. Round 8's forms, copied from the corrected line, are
      ``round 8 `R` ``, and the rejected run's heading matches them, so
      `security.md` is listed before the round 8 re-run has written anything.
      The number check now shows 1 to 8 once each, correctness finishes round
      7, and the runner ticks while the security re-run is still running or
      has stalled. A skip after a lost line can be fixed the same way by
      mistake, because the text does not say how a runner tells a lost line
      from a mistyped number.
      **Why only the skip:** a repeat is fixed by raising numbers. Each
      raised number was carried only by a *later* line, and the round's own
      "except a lane a later round ran again over the same range" already
      removes those lanes from its check. I worked through 1, 2, 2, 3 →
      1, 2, 3, 4 with a shared range and it fails closed. Lowering is the
      direction that reuses a number an earlier record carries.
      **Possible fix, for the spec-writer to choose:** fix a skip by adding a
      line under the missing number, restoring it or recording that no round
      ran under it, and never lower a number. Or state as the rule itself
      that no line is given a number any committed heading or verdict box
      already carries. Either one closes the case, and `RUNNER.md` follows
      the fix.
      **Severity:** medium. It fails open, and it fails open by following
      the remedy the text gives. Reached by reasoning through the rule: I
      built no fixture tree.

      **Fixed** (`spec-writer`, this commit), at the root rather than by a
      third patch to the repair. Every record is stamped with its line's
      number, so what the forms check needs is that no number is given
      twice, not that the numbers run 1, 2, 3. `proposal.md` now says:
      - **A new line takes one more than the highest number already under
        the row**, 1 for the first. Lines are only added below the last,
        and a line's number changes only to repair a repeat. Numbers must be
        unique, not consecutive, and a gap is harmless: a number no line
        carries is one no brief gave and no record carries.
      - **The number check fails on a repeat, not on a skip or on order.**
        Each line whose number a line above it already carries is repaired
        by raising it to one more than the highest under the row, and its
        lanes run again under the new number. **The runner never lowers a
        number**, with your scenario as the reason.
      - **"A lane a later line ran again" now means a line below it in the
        row.** After a repair a line's number no longer says where it
        stands, and positional order is what the exception needs.
      Your second fix option, restoring a lost line or recording "no round
      ran under 7", would have kept gap detection, but it needs a
      placeholder line with no range, and it still cannot tell a lost line
      from a mistyped number. So a removed line is now a residual neither
      check sees, beside a re-run given no line: it breaks the rule that
      lines are only added. So is a number lowered anyway, since it lists
      once. Both are recorded in "What it still cannot see".
      **Measured**, on copies under `./tmp/` searched with `--no-index`
      (since deleted). Your scenario: `findings/` held the rejected run's
      ``## Re-review round 8 `R` `` heading in `security.md`, with a
      concrete range for `R`, and a round 8 verdict box in
      `correctness.md`. Round 9's forms listed nothing, and round 8's listed
      `correctness.md` and `security.md`. So the old repair, lowering the
      re-run to 8, ticks on the rejected run, and leaving 6, 8, 9 as written
      holds the tick until the re-run writes. A line templated with its
      number left at 2 printed `round 2` on two adjacent lines. A mid-row
      repair, 1, 2, 5, 3, 4, lists each number once.
      This lands together with `findings/spec-test.md`'s round 10 box (the
      indent), and the list below covers both.

      **For the `dev-writer`**, one list for both boxes. Your own box below
      is part of item 2.
      1. `RUNNER.md` "What you read" (`:81`): the number-check row gets the
         new command,
         `git grep -n -F -e "] re-review: every commit" -e "      round " -e "] findings all ticked"`
         over `tasks.md`. It is for whether every line between the
         re-review row and the next row is a round line, and whether any
         round number repeats.
      2. `RUNNER.md` "Record the call" (`:600-617`):
         - "One indented line per round … numbered from 1 in the order you
           write the lines" becomes one line per round, indented by exactly
           six spaces as in the sample, with nothing else between the row
           and the next row. Its number is one more than the highest
           already under the row, 1 for the first. Lines are only added
           below the last, and a line's number changes only to repair a
           repeat.
         - The re-run sentence's "the next number" becomes "a new number,
           one more than the highest under the row".
         - Add the case from `proposal.md`'s "The row is never struck":
           when nothing that merges lands after the review round, the row
           still gets round 1, both ends of the range at your HEAD
           (``round 1 `<sha>..<sha>` ``), marked skipped because nothing
           landed, before the tick. That is your box below.
         - Optional, from the prose note: "so the check below would pass on
           it" becomes "the forms check below".
      3. `RUNNER.md` tick paragraph, the number check (`:626-648`):
         - The command becomes the one above. Say it prints the re-review
           row, the round lines and the next row, each with its line
           number.
         - Replace "The six spaces are a round line's indent under the row",
           the "1, 2, 3 … in the order they stand" requirement, the
           two-case repair and the empty-listing sentence at `:644-645`
           with the three conditions: every line between the two rows is
           listed, with no gap in the line numbers, and a missed line is
           put into the form; at least one round line stands between them,
           since two rows on adjacent line numbers mean the line is not
           written yet, and a listing missing either row means a mistyped
           command; and no number repeats.
         - The repair: each repeating line below the first gets one more
           than the highest under the row, and a lane briefed from it runs
           again, with forms copied from the repaired line. Never lower a
           number, with one sentence of reason: a lowered line can land on
           a number another line carried, and its forms check passes on
           that line's records.
         - Keep the closing sentence on the templated re-run.
      4. `RUNNER.md` forms check (`:661-663`): "except a lane a later round
         ran again over the same range: that round's own check covers it"
         becomes a line below it in the row, and "that line's own check".
         Add that after a repair a number does not say where a line stands.
      5. `design.md`, the stage-block summary (`:101-105`): "numbered in the
         order the lines are written" becomes one more than the highest.
         Add the six-space single line, and the round 1 line when nothing
         lands.
      6. `design.md`, "The runner's round lines are numbered" (`:391-401`):
         the same numbering rule, and "the next number" as in item 2.
         `:416`'s "a later round ran again" as in item 4. `:442`'s
         "consecutive rounds" becomes "neighbouring rounds", as
         `proposal.md` now says.
      7. `design.md`, "Rejected: a clause … one more than the highest"
         (`:503-513`): this is now the rule, so the entry cannot stay under
         Rejected. Fold it into the number-check Decision (item 8) as
         adopted. The reason: numbering by position made the repair for a
         gap a lowering, and lowering fails open (this box). The rule is
         now the only thing that fixes a new line's number, not a
         restatement of "the next number".
      8. `design.md`, the number-check Decision (`:515-566`):
         - The command, the three conditions and the upward repair, as in
           item 3.
         - Why the numbers need only be unique: every record is stamped
           with its line's number, and a repeat is the only thing that
           lets a forms check pass on another line's record.
         - Why the rows are in the listing: `findings/spec-test.md`'s
           round 10 box, a line the pattern misses leaving the listed
           numbers looking clean.
         - "It reads only the line's start" becomes "It matches only the
           fixed start, six spaces and `round `".
         - "It fails closed on a mistyped command" becomes: a round pattern
           matching no round line leaves a gap between the rows; a missing
           row means a mistyped command; a pattern too short lists more
           lines, and those outside the rows are not round lines.
         - "A lane briefed from a line whose number changes runs again"
           stays, for the raised number.
         - Replace the measurement paragraph (`:550-556`) with
           `proposal.md`'s, at `5745a7de` and on the scratch copies. Drop
           "with round 7's line deleted, it goes from round 6 straight to
           round 8" as evidence of what the check sees.
         - Rejected alternatives:
           (a) keeping consecutive numbering and repairing a gap by adding
           a line for the missing number: it keeps lost-line detection but
           needs a placeholder line with no range, and it still cannot tell
           a lost line from a mistyped number (this box);
           (b) fixing the indent in the line rule alone: a broken rule
           still fails open silently;
           (c) the rows in the listing alone: the gap shows, but there is
           no form to repair the line to;
           (d) a looser pattern for any indent: it misses a wrapped or
           misspelt line all the same, and shows nothing when it does.
      9. `design.md`, "What it still cannot see" (`:596-601`): "keeps the
         numbers unique and consecutive" becomes "unique". Add the two
         rule-breaking residuals from `proposal.md`: a round line removed
         from under the row, and a number lowered anyway. Each lists clean.
      10. `design.md` Risks, the standing-test entry (`:1362-1417`): the
          number check's command; its fail-closed case as in item 8; and in
          the stale-number paragraph, "repeated or stale" becomes
          "repeated". Add the gap sentence and the two rule-breaking
          residuals, as `proposal.md`'s entry now has them.
      11. `design.md` Risks, "[Only the runner runs the pre-tick checks.]"
          (`:1427-1440`): the `closer`-side number criterion becomes every
          line under the row is a round line and no number repeats,
          replacing "unique and consecutive, 1, 2, 3 in the order the lines
          stand".
      12. PR #174's body:
          - Step 3's "records the call as a numbered line" says the number
            is one more than the highest under the row, the line is
            indented six spaces, and a piece where nothing lands still gets
            round 1, skipped.
          - Step 3's number check: the new command; every line between the
            rows listed, at least one, and no number twice; a repeat raised
            to one more than the highest with its lanes run again; numbers
            never lowered; a gap is harmless.
          - The `design.md` summary bullet on the number check says numbers
            are unique rather than consecutive, one more than the highest
            is adopted rather than rejected, the repair is upward, and the
            rows bracket the listing.
          - The standing-test follow-up: the number check's command; its
            fail-closed sentence as in item 8; the gap sentence and the
            rule-breaking residuals.
          - The `closer`-side follow-up: "confirms the numbers under the
            row run 1, 2, 3 … once each" and "unique and consecutive, 1,
            2, 3 in the order the lines stand" become every line under the
            row listed and no number repeated. "Reads only the line's
            start" becomes "matches only the line's fixed start".
      13. Check that no copy of the old wording is left:
          `git grep -n -e "in the order you write" -e "next number" -e "unique and consecutive" -e "gets its line back" -e "number it should" -- .claude/agents openspec/changes/171-workflow-rules/design.md`
          should return nothing once items 1 to 11 land.

- [x] **`dev-writer`** — `RUNNER.md:644-645` — "An empty listing means the
      command was mistyped, not that there are no rounds: every tick follows
      at least one round line" is false for a piece where nothing lands after
      the review round, and `RUNNER.md` never tells the runner to write a line
      in that case.
      **Scenario:** every review lane finds nothing, and no writer pass,
      callback or owner rewrite follows. No commit needs review, so "Tick the
      row when no commit that merges is unreviewed" holds right away. "Record
      the call" asks for "one indented line per round", and a skipped round
      gets a line. But there was no round to run or to skip, so the runner
      has written no line. The number check lists nothing. The text tells the
      runner that it mistyped the command. Retyping the command gives the
      same empty listing, and nothing in `RUNNER.md` shows a way forward. The
      runner either stalls on a finished piece, or it improvises by striking
      the row, which the `closer`'s Step 1 accepts, or by inventing a line.
      `proposal.md:204-205` already has the sentence that makes "every tick
      follows at least one round line" true: "The row is never struck. A
      round with nothing to review gets its line and then a tick."
      `git grep -n -e "never struck" -e "nothing to review" -- .claude/agents`
      finds neither phrase in any role file, so the runner never reads that
      sentence.
      **Fix:** carry proposal.md:204-205 into "Record the call". If nothing
      lands after the review round, the row still gets a line (round 1,
      skipped: nothing landed) before it is ticked. If the spec-writer
      decides that "a round with nothing to review" does not cover this
      piece, the box is theirs.
      **Severity:** medium. It fails closed, but the runner misdiagnoses the
      cause, and the text has no rule that lets a correct piece close.

      **Contract confirmed** (`spec-writer`; the box stays open for the
      `dev-writer`). "A round with nothing to review" does cover this case,
      and `proposal.md`'s "The row is never struck" now says so in terms.
      When nothing that merges lands after the review round, the runner
      still writes round 1, with both ends of its range at its own HEAD when
      it writes the line (``round 1 `<sha>..<sha>` ``), marked skipped
      because nothing landed, and then ticks. So every tick follows at least
      one round line. The number check's empty case changed too, with the
      `findings/spec-test.md` round 10 fix: the listing now prints the row
      and the next row. Two rows on adjacent line numbers mean the line is
      not written yet, and the runner writes it. A listing missing either
      row means a mistyped command.
      The `RUNNER.md` sentence to replace is `:644-645`: "An empty listing
      means the command was mistyped, not that there are no rounds: every
      tick follows at least one round line." The line itself goes into
      "Record the call" (`:600-617`). Both are items 2 and 3 of the list in
      the box above.

      **Fixed** (`dev-writer`, this commit). "Record the call" now says
      **"The row is never struck."** and that when nothing that merges lands
      after the review round the row still gets round 1, both ends of its
      range at the runner's HEAD (``round 1 `<sha>..<sha>` ``), marked
      skipped because nothing landed, and then the tick. The empty-listing
      sentence is gone. The number check's second condition replaces it: the
      two rows on adjacent line numbers mean the line is not yet written, and
      the runner writes it as "Record the call" says; a listing that lacks
      either row means the command was mistyped, or run on a file with no
      stage block. So the finished piece in your scenario has a way forward
      that the text names. `git grep -n -e "never struck" -e "nothing
      landed" -- .claude/agents` now returns `RUNNER.md`'s "Record the call"
      and the number check's second bullet, and `git grep -n -F "empty
      listing" -- .claude/agents` returns nothing. Measured on a scratch copy
      of the stage block under `./tmp/` with no round line (since deleted):
      the new command prints the two rows on lines 2 and 3. No test can see
      this, since nothing runs `RUNNER.md`; the check is reading it.

**Tick paragraph, applied to this tree.** At `b5c3786b`, `git grep -n -F "      round " -- openspec/changes/171-workflow-rules/tasks.md`
lists `tasks.md:25-34`. Those are the ten lines directly under the row at
`:24`, and they carry 1 to 10, once each, in order. No implementation-checklist
line starts the same way, so the listing has no line from anywhere else. The
row has never been ticked (`git log -S "[x] re-review: every commit"` over
`tasks.md` returns nothing), so the tick closes all ten rounds. I did not run
the forms check as ten single-quoted commands. I listed every heading and
verdict box in `findings/` instead, with the backtick-free prefixes
`## Re-review ` and `**re-review `, and matched each round's copied
``round <n> `<range>` `` against those lines by eye:

- Rounds 1 and 2 use their lines' own un-numbered forms, and each lists all
  six files.
- Round 3 lists five files, all but readability, which round 4 ran again.
- Round 4 lists readability.
- Round 5 lists the four lanes it ran.
- Round 6 lists design-review and spec-test.
- Round 7 lists correctness, design-review and spec-test. Security is
  excepted, because round 8 ran it again.
- Round 8 lists security.
- Round 9 lists design-review, security and spec-test.
- Round 10 lists nothing yet, which correctly holds back the tick until this
  round's five lanes commit.

A runner following the text ticks this piece correctly.

Below the box threshold, in prose only:

- **"so the check below would pass on it"** (`RUNNER.md:615`). There are now
  two checks below. Only the forms check reads records, so it is the only
  check that could "pass on" the rejected run's record, and no runner would
  act differently. "The forms check below" would remove the question. Low.
- **"directly under the re-review row"** needs the row's line number, and the
  listing does not print the row. The runner reads the stage block anyway
  ("What you read"), so this costs a glance, not a wrong action. Low.
- **A renumbered lane's line.** "Runs again under the new number" puts the
  re-run under the corrected line. "Record the call" says a lane run again
  over a range it already had gets "a line of its own — the next number".
  A runner following either instruction still ticks correctly, because under
  the second the later line's check covers the lane and the corrected line's
  check excepts it. So the two instructions only lead to different record
  keeping. Low.
- **`design.md`'s "What it still cannot see"** says a finding that discusses
  the forms quotes them "with `<n>` and `<range>` as placeholders, as
  `findings/security.md`'s outcomes do". This tree has two whole quotations of
  the real round 3 forms: `findings/security.md:815` and
  `findings/spec-test.md:413`. Both files also have their own round 3 records,
  so no check on this piece passes on a quotation alone. But the claim that
  this is unlikely is weaker than the paragraph says. The paragraph is outside
  this range, apart from the word "forms". Low.

## Re-review round 11 `842758b..dd4fe18`

- [x] **re-review round 11 `842758b..dd4fe18`: no findings** — read `git diff 842758b..dd4fe18` of `RUNNER.md`, `proposal.md` and `design.md`, `RUNNER.md` "Record the call" through the forms check literally as a runner would, ran the new number check on this tree and walked a repeat, a gap, mis-indented and wrapped lines, and the nothing-landed case on scratch copies; clean

Correctness only, narrowed as briefed. Nothing reaches medium.

**Round 10's first box (lowering) is closed**, measured. On a scratch stage
block holding lines 1 to 6, ``round 8 `1111111..2222222` `` (correctness and
security) and ``round 9 `1111111..2222222` `` (security re-run), with
`findings/` holding the rejected run's round 8 heading and box in
`security.md` and a round 8 verdict box in `correctness.md`: the new number
check lists the row, eight round lines on consecutive line numbers and the
next row, with no repeat, so it passes, and the gap in the round numbers is
harmless as the text now says. Round 8's forms list `correctness.md` and
`security.md`, and security is excepted because the line below re-ran it over
the same range. Round 9's forms list nothing, so the tick is withheld until the
re-run writes. `RUNNER.md` no longer gives any repair that lowers a number:
"Never lower a number, to close a gap or to repair a repeat" is in the third
bullet, and `git grep -n -e "in the order you write" -e "next number" -e "unique
and consecutive" -e "gets its line back" -e "number it should"` over
`.claude/agents`, `proposal.md` and `design.md` returns nothing.
`design.md:557`'s "1, 2, 3 … in the order the lines stand" is the rejected
history, described as such.

**Round 10's second box (empty listing) is closed.** "Record the call" now says
"The row is never struck" and gives the skipped round 1 line for a piece where
nothing landed. On a scratch block with no round line, the command printed the
two rows on adjacent line numbers (3 and 4). The second bullet reads that as "the
line is not written yet: write it". Once written, the line is skipped, so the
forms check does not run for it ("not marked skipped"), and the tick follows.
The "empty listing" sentence is gone.

**The number check on this tree** (`git grep -n -F -e "] re-review: every
commit" -e "      round " -e "] findings all ticked"` over this change's
`tasks.md`, at `dd4fe18` plus `6324524`) prints the row at `:24`, round lines
at `:25-35` carrying 1 to 11 once each, and the next row at `:36`. There is no
gap, no repeat, and nothing listed outside the rows. All three bullets hold.
Round 10's forms list the five lanes it ran, and round 11's list nothing yet,
which correctly holds back the tick.

**The repair rules, walked on scratch copies** (under `./tmp/`, searched with
`--no-index`, since deleted):

- *Repeat.* A line templated from the one above with `round 2` left in place
  printed `round 2` on lines 4 and 5. A wrapped checklist line starting
  `      round the corner` was listed at line 10, outside the rows, as the first
  bullet anticipates. Raising the lower line to 3 and giving `security.md` the
  templated run's round 2 heading: round 3's forms list nothing, so the tick
  waits for the re-run. Round 2's forms list `security.md`, but round 2 excepts
  security because the line below re-ran it. So the check gives the right
  answer. Without the repair, the round 2 forms, copied from the templated
  line, pass on the rejected record, which is the case the check exists for.
- *Mis-indented and wrapped lines.* Between rows at lines 2 and 9, a
  four-space line, a tab line and a wrapped line's continuation left lines 4,
  5 and 8 unlisted. Those are gaps, so the tick is withheld as the first
  bullet says.
- *Nothing landed:* as above.

Below the box threshold, in prose only:

- **"In turn" is in `proposal.md` but not in `RUNNER.md`.** `proposal.md` says
  each repeating line "is repaired, in turn". `RUNNER.md`'s third bullet omits
  "in turn". So a runner facing `1, 2, 2, 2` that computes every raise from
  the first listing gives both lower lines 3, which is a repeat again. This
  fails closed. The check runs again before any tick, so it sees 3 twice, and
  the extra cost is one more re-run of that lane. Low.
- **A skipped round 1 wrongly written.** The second bullet reads two adjacent
  rows as "the line is not written yet: write it, as 'Record the call' says
  for a piece where nothing landed". A runner that lost track of rounds after a
  compaction, and ran none although commits landed, would write a skipped
  round 1 and tick. That runner has already failed the row's own precondition,
  "no commit that merges is unreviewed", and no dispatched round can lack a
  line, since its brief's forms are copied from the line. So this is the
  runner-skips-a-step residual, not a new hole. `proposal.md`'s wording ("so
  the runner writes it") does not make the nothing-landed inference. Low.
- **More than six spaces still matches.** An eight-space round line is listed,
  because `-F` matches the six spaces as a substring. The line rule says
  "exactly six" but the check does not enforce it. That is harmless: the forms
  are copied from the listed line either way, and the line is still read. Low.
- **A raise computed while a line is still unlisted.** The listing leaves out
  a mis-indented line, so the "highest under the row" a runner reads off the
  listing can miss that line's number. A raise can then land on it.
  `design.md:569-571` says a raise "cannot land on a used number". That holds
  once every line is listed. Because the first bullet sends the runner to
  "list again" after putting the line into form, the repeat then shows, so
  this fails closed. Low.

## Re-review round 12 `dd4fe18..1380d50`

- [x] **`spec-writer`** — `proposal.md:223-226`, mirrored at `RUNNER.md:612-614`
      and `RUNNER.md:676-678` — `<review>` is defined only as "the HEAD the
      runner dispatched the review round from", which is a memory of a
      dispatch. `RUNNER.md:98-99` says that memory "does not survive a
      compaction and was never the source of truth", and nothing in
      `tasks.md` records it: the review rows and the re-review row carry no
      sha until a round line is written. The one git-derived way to recover
      it, the parent of the review round's first findings commit, appears
      in two places, and neither is a rule. `proposal.md:225-227` gives it
      as this piece's own illustration. `proposal.md` "The nothing-landed
      check's `<review>` is runner input" and `design.md` Risks give it as
      what a *second reader* compares against. `RUNNER.md` never gives it,
      and `RUNNER.md:92-94` tells the
      runner to point at `design.md` and `proposal.md` rather than read them.
      So the lost-report branch this range adds ("Where you cannot tell
      whether a round ran, such as after your report is lost, the check
      decides") sends the runner to a check whose one input is the thing it
      has lost. A runner following the text is stuck there, or supplies a
      `<review>` of its own.
      **Scenario:** a runner dispatches the review round from `R`
      (`c222c37`), the findings land, the rows are ticked (`e7e2bbdd`), a
      writer pass lands `ae59b43c` touching `proposal.md`, and the session
      compacts. The fresh session rebuilds state as `RUNNER.md:96-121` says,
      finds the re-review row with no round line beneath it, and reaches
      the "at least one round line" bullet. It cannot tell whether a round
      ran, so the check decides, but it has no `R`. The `<HEAD>` in the same
      range is its own HEAD, and "the HEAD you dispatched from" reads as a
      HEAD too, so the ready value is `ae59b43c`.
      `git diff --name-only ae59b43c ae59b43c` prints nothing. The check
      passes, it writes ``round 1 `ae59b43c..ae59b43c` `` skipped because
      nothing landed, and it ticks. That is the empty range this change
      exists to remove, reached by following the text rather than breaking
      it. With the true `R` the same run fails: measured on this tree,
      `git diff --name-only c222c37 ae59b43c` lists `proposal.md` beside
      the six findings files and `tasks.md`. A later-than-true `<review>`
      fails open, as `design.md` Risks itself says. But Risks files it with a
      runner who "breaks a stated rule", and this runner breaks none: the
      text never gave it a way to comply.
      **Measured:** the recovery exists and needs no memory.
      `git log --oneline --diff-filter=A -- openspec/changes/171-workflow-rules/findings/`
      lists `f94f7b8d` as its oldest entry (the design review), and
      `git log --oneline -1 f94f7b8d~1` gives `c222c37b`, the true `R`.
      **Possible fix, for the spec-writer to choose:** give the runner the
      recovery in the contract, and have `RUNNER.md` state it where
      `<review>` is defined: the parent of the oldest commit adding a file
      under the change's `findings/`. Or record `R` when the review round is
      dispatched, for example on the re-review row, so that it is state in
      `tasks.md` and not a memory. `RUNNER.md` follows either.
      **Severity:** medium. It fails open on the recovery path the range
      introduces, and it reproduces the defect the range closes.
      **Outcome (`spec-writer`): accepted, fixed in `proposal.md` by the
      first of the two fixes offered.** A new bullet under the range rule,
      "The runner reads `<review>` from the repository, never from memory",
      makes the derivation the definition, not a recovery: `<review>` is
      the second field of the last line of
      `git log --diff-filter=A --format="%h %p %s" -- openspec/changes/<name>/findings/`,
      always, including when the report still holds it, so there is one
      procedure and no memory in it. The bullet contracts the pre-archive
      pathspec (measured in `tmp/r12spec/`, since deleted: after a
      findings deletion, an archive move and a re-reviewer's fresh file,
      the archived pathspec gives the archive commit, the pre-archive one
      the base), the last line and not the first (the first line's parent
      here is `a284e514`, the tick commit), and that an empty listing
      means no round 1 line. The repair bullet's "the check decides" now
      says over a range from the derived `<review>`, not a HEAD it
      remembers or can see. Re-measured on this tree: the last line is
      `f94f7b8d c222c37b`. Recording `R` on the re-review row at dispatch
      was not taken: a line under the row that is not a round line fails
      the number check's first condition, and it would be one more value
      the runner types. `RUNNER.md` and `design.md` are the `dev-writer`'s
      to follow.

Read `2bf65c8` (`proposal.md`) and `1380d50` (`RUNNER.md` "What you read",
"Record the call", the "at least one round line" bullet, and `design.md`'s new
entry, Risks and the "What it still cannot see" paragraph) literally, as a
runner would, and measured both check commands on this piece's history:

- `git diff --name-only c222c37 e7e2bbdd` lists the six findings files and
  `tasks.md`, and the `tasks.md` diff over the same range is exactly the six
  review-row flips from `[ ]` to `[x]`. So the check passes and a skipped
  round 1 is right, since nothing that merges landed.
- `git diff --name-only c222c37 ae59b43c` adds `proposal.md`, so the check
  fails and an ordinary round is owed, which is also right.
- `git diff --name-only ae59b43c ae59b43c` prints nothing. The old both-ends
  form would have passed over a branch holding the `proposal.md` commit.

Given the right `<review>`, both commands decide correctly in all three cases.
The repair ordering in the "at least one round line" bullet is also sound: a
round that ran is put back as that round, the skipped form is used only where
no round ran and the check passes, and a failed check means a round. That
ordering fails closed.

Below the box threshold, in prose only:

- **A clean merge of `main` after the review round fails the check.**
  `git diff <review> HEAD` compares trees, so `main`'s own paths are listed
  and the runner owes a round for them. That fails closed, and it costs one
  round the runner could size as a skip with a reason. Low.
- **"Only boxes flipped" passes a flipped implementation-checklist box in
  `tasks.md`.** A writer who ticks a checklist box has also changed a path
  the first command lists, so the pair still fails closed. Low.
- **A lost report where rounds did run.** The check fails over
  `<review>..HEAD`, so the runner re-reviews everything since the review
  round as a new round 1. Its range ends at a HEAD that already holds the
  earlier rounds' findings commits, so it cannot equal a lost round's range,
  and the forms check cannot pass on an old record. This fails closed. The
  cost is a full re-review, where the ranges in the findings headings could
  have rebuilt the lost lines. Low.

## Re-review round 13 `1380d50..c3bda2b`

- [x] **re-review round 13 `1380d50..c3bda2b`: no findings** — read `a0ce38f` (`proposal.md`, the new "reads `<review>` from the repository" bullet and the `--no-renames` rationale) and `c3bda2b` (`RUNNER.md` "What you read", "Record the call" and the "at least one round line" bullet; `design.md`'s new entries, Alternatives, Risks and the fail-open list), walked as a runner that has lost its report, and measured the derivation on this tree and in a scratch repository; clean

The round-12 box is fixed, measured. On this tree the derivation
`git log --diff-filter=A --format="%h %p %s" -- openspec/changes/171-workflow-rules/findings/`
prints three lines, and the last is `f94f7b8d c222c37b`, so `<review>` is
`c222c37`. A runner that has lost its report, following "at least one round
line" into "Record the call", now reads that value instead of the HEAD it can
see. From it, `git diff --no-renames --name-only c222c37b ae59b43c` lists
`proposal.md` beside the six findings files and `tasks.md`, so the check fails
and the runner owes a round and does not tick, which is right. Over
`c222c37b..e7e2bbdd` the same command lists only the six findings files and
`tasks.md`, so a skipped round 1 and a tick are right there too. The empty
`ae59b43c..ae59b43c` range from the round-12 scenario cannot arise from the
text any more.

The three cases asked about, measured in `tmp/r13c/` with git 2.55.0 (since
deleted):

- **A file moved into `findings/` from outside it** reads as an add under the
  pathspec: a pre-review commit that moved `notes.md` into
  `openspec/changes/x/findings/` was listed as an `A`. This agrees with
  `design.md`'s scratch measurement. After the review round it is a newer
  line and does not change the last line.
- **A findings file added before the review round** becomes the last line, so
  `<review>` is earlier than the dispatch HEAD (`25c2647`, where the true
  value was `36cb90c`). The range then holds the pre-review implementation
  commits, the check fails, and the runner owes a round. That fails closed.
- **A re-dispatch after the archive.** After a commit deleting `findings/` and
  moving the change folder, a re-reviewer writing at the pre-archive path
  added a newer line, `80a2949`, and the last line was unchanged. One writing
  in the archived folder is outside the pathspec. Either way the derived value
  stands.

The one way to get a value later than the true one is for the review round's
first findings commit to be missing from an `A` listing. The flow rules this
out: every reviewer writes a new file, the runner commits nothing between the
dispatch and the first pick, and a pick is a cherry-pick or a fast-forward,
never a merge commit.

Below the box threshold, in prose only:

- **An earlier-than-true `<review>` is not a strict superset.** The check is a
  tree diff, not a union of commits. If a pre-review findings add makes
  `<review>` early, a post-review commit that reverts a change made between
  that add and the true dispatch HEAD cancels out of the early-based diff,
  while it is a real change against the reviewed tree. This needs a findings
  file that no role writes before the review round, plus that exact revert.
  I reasoned it and did not measure it. Low.
- **After the archive the check always fails.** With `--no-renames` the
  archive move lists every file of the change folder twice, once deleted and
  once added, so a nothing-landed round 1 written after an archive (only a
  lost-line repair reaches this) always costs a full round. It fails closed.
  Low.
- **"Every later round's findings files are adds too"** (`RUNNER.md`
  "Record the call", `proposal.md`, `design.md`) is the stated reason for
  reading the last line, but later rounds before the archive append to
  existing files, which is a modification. On this tree the first line,
  `2259e7ff`, is the spec-test review, which belongs to the review round
  itself and was picked after a tick. The rule is right and its reason is
  imprecise. Low, wording only.

## Re-review round 14 `c3bda2b..e5dcce4`

- [x] **`spec-writer`** — `proposal.md:394-425`, followed by `RUNNER.md:723-748`
      — the chain condition has no terminal state for a round line whose range
      overlaps the chain, so a piece holding one can never tick without
      breaking "never change an existing line's range". The chain follows "the
      line whose range starts at" a commit, which assumes one such line, and
      passes over only a line with the *same* range as one followed. A line
      whose start lies before the chain point it belongs at, or which shares a
      start with another line but ends elsewhere, is neither followed nor
      passed over, whatever is added below it. The bullet then diagnoses it as
      "some commits lie in no round's range" and says to write a line for "the
      missing range", but no commit is missing and there is no such range.
      **Scenario:** on a copy of this stage block, the round 14 line is
      templated from round 13 with only its end updated:
      ``round 14 `1380d50..e5dcce4` ``. The listing prints the two rows and
      fourteen round lines, 1 to 14 once each, so the first three conditions
      pass. The chain runs c222c37 → 9dc235c → 34fd428 → dc1390a (4 passed
      over) → d1c8726 → 6d43cda → d1d2165 (8 passed over) → c4b1df5 → 842758b →
      dd4fe18 → 1380d50 → c3bda2b, then stops, because no line starts at
      `c3bda2b`. Round 14 is neither followed nor passed over, because its range
      differs from round 13's. Every commit in `c3bda2b..e5dcce4` is inside
      round 14's range, so nothing is missing. There are only two repairs. Line
      15 `c3bda2b..e5dcce4` (sized, or skipped as covered) extends the chain to
      `e5dcce4`, and round 14 stays neither. Choosing round 14 at `1380d50`
      instead of round 13 leaves round 13 neither. Either way the condition
      fails for good. The runner is left with two options, both bad: a piece
      that cannot close, with the owner away, or editing round 14's range,
      which is the fail-open move the rule forbids. The same state is reached
      in two other ways. One is a round 1 whose start is a remembered HEAD
      earlier than the derived `<review>`. The other is a mid-chain gap
      repaired with the tail bullet's "record the next round, starting at the
      chain's end" instead of the gap bullet. On a copy with round 9 written
      ``round 9 `0fbebed..c4b1df5` ``, the chain stops at `d1d2165`, and the
      tail check from there also fails (it lists `proposal.md` and `design.md`
      among others). Taking the tail repair gives ``round 15 `d1d2165..<HEAD>` ``,
      and rounds 9 to 14 are then permanently neither. The template slip is
      the start-side twin of the number slip the number check exists for
      ("a re-run's line templated from the previous one"), and it is at least
      as likely. **Severity:** medium. The condition fails closed, but the
      tick is then impossible, and the only way out is to break the rule's own
      "never". **Needs:** a rule for a line that covers commits already on the
      chain. For example, a line whose range lies entirely within the span the
      chain has covered is passed over, and "the line starting at X" becomes
      the one reaching furthest. Or the gap bullet says to repair every break
      before the tail check runs, and defines the chain's end only once every
      line is followed or passed over.
      **Measured:** listings of `tmp/r14-correctness/chain-overlap.md` and
      `chain-gap.md` (since deleted; `git grep --no-index`, the number check's
      three patterns) pass the first three conditions. Walking the chain by hand gives the
      stops above. `git log` over `d1d2165..c4b1df5` shows `0fbebed` (a
      `spec-writer` commit) as the one commit the gap copy's round 9 leaves
      out. For that copy, the gap bullet's own repair, ``round 15
      `d1d2165..0fbebed` ``, chains 1 … 7, 15, 9 … 14 to `e5dcce4`, which is
      right.

      **Outcome (`spec-writer`): fixed in the contract, by removing
      mechanism.** `proposal.md`'s chain condition no longer follows one line
      per step. The chain reaches `<review>` and then the end of every line
      whose range starts at a commit it reaches, however many start there, so
      a line templated from the one before with only its end changed, or a
      re-run, extends the chain like any other. "Passed over", "every line is
      followed or passed over" and the gap diagnosis are gone: a line the
      chain never reaches needs no repair, and the one condition left is the
      tail check, run from an end the chain reaches. If it fails, the next
      line starts at that end, either a round to HEAD or, where an unreached
      line starts later, a line ending at that start; both are sound and
      both terminate, so no state needs a range edited. Your overlap copy
      reaches `e5dcce4` through round 14 and passes; your round-9 copy fails
      the tail from `d1d2165` and passes after either repair. The reasoning,
      including why the tail check alone is sound and why "furthest end" was
      not taken, is in `design.md` under "The ranges chain from `<review>`",
      "Adopted: the chain is what it reaches". Of the low notes, the gap end
      is answered by the same sentence (the smaller repair ends at the
      unreached line's start); the skipped-line edit and the `closer` return
      are left as written. `RUNNER.md:723-748` still carries the old text; the
      `dev-writer` brings it into line.

On this tree the condition gives the right decision. `<review>` derives as
`c222c37b` (the last line of the derivation is `f94f7b8d c222c37b`). Rounds
1, 2, 3, 5, 6, 7 and 9 to 14 are followed, and 4 and 8 are passed over as
repeats of 3 and 7. The chain ends at `e5dcce4`.
`git diff --no-renames --name-only e5dcce4 HEAD` lists only `tasks.md`, and the
`tasks.md` diff is the round 14 line added, so the tail is tracking only, with
the round-line allowance. All four number-check conditions hold. The tick
still waits on the forms check for round 14, whose five lanes have not yet
recorded, so not ticking at `015f497` is right. On the off-by-one copy (round 9
starting at `0fbebed`), the gap bullet's repair is unambiguous for a single
break. It gives the one extra line above and chains to `e5dcce4`.

Step 2's rule fits the flow, and I found nothing in `RUNNER.md` that
contradicts it. The first pick of a review round always applies cleanly,
because the reviewer forked from the runner's HEAD. So with nothing landed,
its parent is the dispatch HEAD, and the derivation's last line is exactly the
first picked findings add. Later picks conflict and are rebased by their
agents onto a HEAD that already holds that add, which does not move it. A
reviewer's own row tick rides its findings commit, so "commit nothing" leaves
the runner nothing it has to commit mid-round. A red CI run during the round
can only be against the `dev-writer`'s pushed tip, because the runner never
pushes, and step 2 holds that writer back. The `closer` needs every row
ticked, so it cannot be out during the round. A re-review round needs no such
hold: a commit landing during one lies after the chain's end, and the tail
check owes a round for it.

Below the box threshold, in prose only:

- **The gap bullet does not say where the missing range ends.** With several
  lines neither followed nor passed over (six in the round-9 copy), the gap
  runs from the chain's end to the start of the unfollowed line nearest it in
  ancestry. The runner has to work that out. Low.
- **A skipped line's forms are never checked, and neither is its range.** An
  existing round line whose range is edited in a later tracking commit shows in
  the tail's `tasks.md` diff as a round line, which the allowance admits. For
  an unskipped line, the forms check then fails, because reviewers were briefed
  with the old range, so this fails closed. For a skipped line, nothing sees
  it. Reaching it takes breaking "never change an existing line's range". Low.
- **"A commit they list"** asks the runner to map the tail's paths back to
  commits. After a `closer` return that holds a merge of `main`, a findings
  deletion and an archive, the rule does not say whether that is one skipped
  line or one per commit. Either chains. Low.
