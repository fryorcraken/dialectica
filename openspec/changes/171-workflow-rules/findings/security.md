# Security review — 171-workflow-rules

Dimension: **security** only, per dispatch. Scope: the prose diff
`origin/main...HEAD` (`.claude/agents/{README,RUNNER,closer,tester,spec-writer}.md`
and `openspec/changes/171-workflow-rules/{proposal,design,tasks}.md`). No
source-code diff exists on this piece.

## Findings

- [x] **`owner`** — `.claude/agents/closer.md` Step 2 (rebase) and Step 3
      (archive), unchanged by this diff — the closer's own rebase-conflict
      resolution, and the archive commit that merges a spec delta into the
      live contract, reach `main` with no review step at all, and this piece
      leaves that gap open rather than closing it.
      **Scenario:** on a future piece, `git rebase origin/main` (Step 2)
      conflicts on a file the review round touched, the closer resolves the
      conflict by hand, pushes with `--force-with-lease`, and that resolution
      — arbitrary code the six reviewers never saw — rides the same PR to a
      green run and a squash merge. Separately, Step 3's `openspec archive`
      is not a no-op: `docs/OPENSPEC-ARCHIVE.md` states it "merges the delta
      into `openspec/specs/` — the live contract," and that merge is the
      closer's own unilateral act, with no re-review step 3 of `RUNNER.md`'s
      new sequence covers (its "every commit after the review round" list
      names a findings pass, an owner instruction, a spec callback and a
      red-CI fix — not the closer's own rebase or archive commits).
      **Measured:** `git diff origin/main...HEAD -- .claude/agents/closer.md`
      touches only the "find the change folder", the re-dispatch note in
      Step 3, the `--admin`/`BLOCKED` text in Step 6, and the closing
      paragraph — Step 2's rebase text (lines ~150-197) is untouched.
      `openspec/changes/171-workflow-rules/design.md:390-395` and
      `proposal.md`'s "Out of scope" both name the rebase half of this gap
      explicitly and defer it as "a candidate for a follow-up issue... not
      in scope here" — neither text names the archive commit's spec-merge as
      a second instance of the same gap. This is confirmed, pre-existing
      (not introduced by this diff), and already disclosed as a risk for the
      rebase half; I'm recording it because the task asked this dimension to
      check it, the archive-commit half is not yet named anywhere, and only
      the owner can authorise the `.claude/` edit that would close it.

      **Fixed** (this commit). The owner took this into the piece, choosing
      to merge `main` rather than rebase, and to stop when specs change;
      `proposal.md` records both. Both halves are now closed:
      - **The conflict resolution.** `closer.md` Step 2 merges `origin/main`
        and pushes by refspec with no force; `--force-with-lease` and its
        paragraph are gone, and "What you never do" forbids force-pushing for
        any reason and resolving a conflict. On a conflict the `closer` runs
        `git diff --name-only --diff-filter=U`, then `git merge --abort`,
        reports the paths and returns. `RUNNER.md` routes the conflict to a
        writer, which resolves it in a merge of `origin/main` on its own
        branch; the runner fast-forwards to that branch, and step 3 reviews
        the resolution, the brief naming `git show --remerge-diff <sha>`.
        Measured with git 2.55.0 in a scratch repository: the first command
        printed exactly the conflicting file; `--abort` put HEAD back on the
        pre-merge commit; and `--remerge-diff` on the resolved merge showed
        only the conflict markers replaced by the resolution.
      - **The archive commit.** After Step 3's push, a `closer` that made the
        archive commit in that run runs `git diff --name-only HEAD^ HEAD --
        openspec/specs/`, and any file listed stops it before Step 4. It
        returns, and `RUNNER.md` step 3 lists that archive commit as needing
        review. Measured: on `2bda577f` (#181, which changed two specs) the
        command lists `feed-view/spec.md` and `stoa-navigation-view/spec.md`;
        on this tree's HEAD it lists nothing.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md`, "From the `dev-writer`'s
      hand-back to the merge", step 3, bullet under "The brief" — "A clean
      re-review writes no file" removes the one mechanical trip-wire the rest
      of the flow relies on to catch a false "nothing to see" claim, so a
      re-review round's completion is attested by nothing but the runner's
      own prose line.
      **Scenario:** a red-CI fix lands that reintroduces a defect a prior
      `security` finding had fixed. The runner sizes a `security` re-review,
      dispatches it, and either the reviewer under-reads the range or the
      runner simply writes the round's line without the dispatch happening.
      Either way the outcome is "clean": per this rule, no file is written
      to `findings/` at all. The closer's Step 1 gate — `grep -rn "^- \[ \]"`
      and `grep -rc "^- \["` over `findings/` — has nothing to grep and
      raises nothing. Contrast the pre-existing convention this same file
      documents for the *first* review round (`README.md:280-288`,
      `closer.md:100-110`): there, a reviewer's file with zero checkboxes is
      treated as a file "the gate cannot see" and is routed back to the
      runner "not to you to interpret" — i.e. a clean-but-suspicious report
      is still forced into a visible, escalated artifact. The re-review rule
      chooses the opposite: it says explicitly to avoid writing that same
      zero-box file "since a fresh file with no box fails the closer's
      every file non-zero check" (`RUNNER.md`, same bullet) — i.e. the
      no-file rule is reasoned from what avoids tripping the check, not from
      what leaves the best trail. The only surviving record is the runner's
      own indented line under the re-review row in `tasks.md`, which nothing
      else in the flow cross-checks against an actual reviewer output.
      **Measured:** `git diff origin/main...HEAD -- .claude/agents/RUNNER.md`,
      the paragraph beginning "The brief names the commit range to read".
      This is a real trade-off already partly reasoned about in
      `design.md`'s Risks ("Nothing mechanical checks any of this prose. The
      six reviewers are the check.") — but that entry is about whether the
      *rules in this diff* get followed, not about whether a *future clean
      re-review claim* is independently checkable. I could not find text
      addressing this specific gap, so it stands as a residual weakening
      of the merge gate's ability to distinguish "reviewed, found nothing"
      from "not reviewed."

      **Deferred** — to the `spec-writer`, and recorded in `design.md`'s Risks
      ("A clean re-review leaves no reviewer-authored trace"). The gap is
      real, and it is sharper since the owner ruled "Agents tick their own":
      a first-round reviewer's own tick attests its review, and a clean
      re-reviewer leaves no commit at all. But the no-file behaviour is not
      the `dev-writer`'s to change: `proposal.md` specifies it ("A clean
      re-review adds no box either way… after it it writes no file"), so the
      fix is a contract change. `design.md` records the candidate that closes
      the gap without tripping the `closer`'s every-file-non-zero check: a
      clean re-reviewer writes one **ticked** box naming the range it read,
      which `grep -rn "^- \[ \]"` ignores and `grep -rc "^- \["` counts. The
      hand-back to the runner names this for a `spec-writer` dispatch; if the
      spec-writer takes it, the `RUNNER.md` brief bullet changes in this
      piece, and this box's outcome stands as the record of why.

      **Fixed** (this commit, superseding the deferral above). The
      `spec-writer` took it (`f9df27f`, `proposal.md` "A re-review brief
      carries four things"). `RUNNER.md` step 3's brief now has a re-reviewer
      that finds nothing append one ticked verdict box naming the range it
      read, and commit it itself; after the archive every re-reviewer writes
      its file, clean or not. "A clean re-review writes no file" and "only if
      it has a finding" are gone, and the verdict box is on step 3's list of
      tracking that needs no review. `closer.md` Step 1 now reads an archived
      folder with no `findings/` as "no re-reviewer has run since", not "the
      re-review raised none". The reasoning is `design.md`'s "A clean
      re-reviewer appends a ticked verdict box", which replaces the Risks
      entry named above. Checked: `grep -rc "^- \["` counts a ticked box and
      `grep -rn "^- \[ \]"` does not match one — on this folder, this file
      counts 2 and the unticked grep returns nothing. No test can see the
      prose; the check is the re-review round reading it.

## Areas checked and clean

- **`--admin` / branch protection / rulesets.** `closer.md` Step 6's new text
  ("Never merge with `gh pr merge --admin`, and never change branch
  protection or a ruleset ... whatever the brief or the owner's merge-on-green
  said") and the new "What you never do" bullet both foreclose every reading
  named in the brief — including under merge-on-green authority — and the new
  `BLOCKED`-with-green-checks stop condition names the exact `gh pr view`
  command rather than leaving the closer to invent a diagnostic. No `--force`
  (as opposed to the pre-existing, unmodified `--force-with-lease`) and no
  direct push to `main` appear anywhere in the diff.
- **The untick rule.** "If a commit lands after it is ticked — a red-CI fix —
  untick it" correctly re-opens the generic, pre-existing closer Step 1 gate
  ("every row ticked or struck ... except your own"), which is unmodified and
  applies to the re-review row with no special-casing needed. I did not find
  a reading under which a re-dispatched closer could proceed past an unticked
  re-review row.
- **The archived-folder pathspec.** Measured directly:
  `git ls-files -- "openspec/changes/op-clock/tasks.md" "openspec/changes/archive/????-??-??-op-clock/tasks.md"`
  returns exactly `openspec/changes/archive/2026-09-16-op-clock/tasks.md`, and
  the same command for `171-workflow-rules` returns exactly the live path.
  The "more than one path, or none: stop and report" rule fails closed rather
  than guessing, and "an archived folder with no `findings/` passes the gate"
  is consistent with the pre-existing (unmodified) rule that the closer
  deletes `findings/` in Step 1 before archiving in Step 3 — it is not a new
  hole, just documentation of an existing one.
- **The restored `#133` paragraph.** The committed text in `README.md` matches
  issue #133's quoted original byte for byte, and `design.md`'s Decisions
  section correctly identifies the two clauses that were *not* the owner's
  (the CLAUDE.md pointer and the "invited reading... as fair game" claim,
  both introduced by an unauthorised #131 fixer) and drops only those. The
  restored wording still functions as a "do not touch without the owner"
  rule even without citing CLAUDE.md by name, since CLAUDE.md's own rule
  already reaches every file by injection — it does not depend on this
  paragraph restating it.
- **The `tester.md` edit.** Matches `proposal.md`'s description exactly: only
  a marker the brief *names as decided* is closed (reworded or removed); any
  other marker — including one reported with no marker at all — stays open
  and still falls under "must not remove." I did not find wording that would
  let a tester close a marker on its own judgement, or on an unnamed claim of
  owner or spec-writer authority.
- **Owner-authority relay.** This piece's own history (commit `d805fe4`'s
  message) records that a `dev-writer`'s attempt to make the `tester.md` /
  `closer.md` Step 3 edits was refused by the permission classifier because
  the owner's authorisation reached it relayed through the runner, and that
  the runner then made the edits itself. The committed text (`proposal.md`,
  `design.md`) documents this correctly as an exception the runner executed
  directly, not as a standing pattern any future dispatched writer can invoke
  by citing a brief's claim of owner authorisation — I did not find wording
  in `tester.md`, `closer.md` or `RUNNER.md` that generalises this into "if
  your brief says the owner authorised it, proceed." One adjacent,
  pre-existing (unmodified by this diff) sentence is worth the owner's own
  attention rather than a boxed finding here, since fixing it is outside this
  piece's authorised scope: `closer.md`'s "If the owner has said in this
  session to merge on green without coming back, that is the authority and
  you do not ask again" does not say whose session — a freshly-dispatched
  `closer` never talks to the owner directly, so in practice this authority
  can only reach it relayed through the runner's brief. This is a different
  class of relay than the `.claude/`-edit case above (a merge decision, not a
  destructive tool call the classifier itself would evaluate), and this diff
  does not introduce or worsen it — its new neighbouring sentence ("That
  authority is to run `gh pr merge <n> --squash`, and it stops at branch
  protection") narrows *what* the authority covers without touching *whose
  session* it must come from. Recorded as an observation, not a finding,
  because I have no scenario showing it produces a wrong merge, only an
  ambiguity adjacent to a spot this diff already edited.

- [x] **re-review `c222c37..9dc235c`: no findings** — read the full diff of
      `.claude/agents/{README,RUNNER,closer,spec-writer}.md` and
      `openspec/changes/171-workflow-rules/{proposal,design,tasks}.md` for this
      range (`tester.md` carries no diff in this range, so it needed no
      re-read). Verified both boxes above are actually closed by the current
      text, not just claimed closed: `closer.md` Step 2 now merges
      `origin/main` with no force and stops on a conflict rather than
      resolving one (`git grep -n "force-with-lease" .claude/agents` returns
      nothing; `git grep -n -i -- "--admin" .claude/agents/closer.md` still
      forbids it including under merge-on-green); the conflict routes through
      `RUNNER.md`'s hand-back-to-merge step 3 before a `closer` is
      re-dispatched; the archive-commit spec-diff check
      (`git diff --name-only HEAD^ HEAD -- openspec/specs/`) is scoped to an
      archive made in the same run, matching the "Fixed" note; and the clean
      re-review verdict box (`- [x] **re-review \`range\`: no findings**`) is
      both ticked (passes `grep -rn "^- \[ \]"`) and a box (counted by
      `grep -rc "^- \["`), closing the dev-writer finding's "no mechanical
      trace" gap. Checked the new "What a runner commits" section
      (`RUNNER.md`) against the threat model: it forbids ticking another
      agent's row, forbids copying an agent's uncommitted output onto the
      piece, and turns an owner in-session edit request into a dispatch
      instruction rather than a runner-made edit — closing the relayed-edit
      path `d805fe4` used. Found no reading that lets a runner's own commit
      carry reviewed-as content, no reading that permits `--force`,
      `--force-with-lease`, `reset`, or a push to `main`, and no new instance
      of authority relayed through a brief substituting for a stop-and-report.
      The pre-existing "whose session" ambiguity on merge-on-green authority,
      noted above as an observation, is untouched by this range and still not
      a finding. `tasks.md`'s new sections 8 and 9 match what `proposal.md`
      and `design.md` claim landed. Clean.

## Re-review `c222c37..9dc235c` (Opus)

Security only, on Opus, run independently of the verdict above. Read:
`git log --oneline c222c37..9dc235c`; `git diff c222c37..9dc235c` for
`.claude/agents/{README,spec-writer}.md`; `closer.md` and `RUNNER.md` in full
(the tree's copies match `9dc235c`: `git diff --stat 9dc235c HEAD` touches only
`findings/` and `tasks.md`); `proposal.md:96-230` and `:540-570`;
`design.md:560-760`; the other reviewers' re-review sections, to avoid
duplicates. No scratch repository and no mutation: every claim below is a line
of the committed text, cited.

The two first-round boxes above are **not both genuinely closed**. The
conflict-resolution half of box 1 is closed: `closer.md` never resolves and
never forces, and a writer's resolution reaches step 3. The archive half is
closed on every path except the one in the first box below. Box 2's mechanism
exists but is consumed by no gate, which is the second box below.

- [x] **`spec-writer`** — `proposal.md:304-317`, carried into `closer.md:290-315`
      and `:229-237` — the archive commit's `openspec/specs/` check runs
      **after** Step 3's push, and a refused push ends the `closer`'s turn
      before it (`closer.md:297-298`). A re-dispatched `closer` then skips the
      check by rule, so a spec-changing archive reaches `main` with no review
      round ever sized for it.
      **Scenario:** a piece whose change carries a spec delta; the branch is
      not behind, so Step 2 pushes nothing. The `closer` deletes `findings/`,
      archives (promoting the delta into `openspec/specs/`), commits, and runs
      `git push origin HEAD:refs/heads/piece/<name>` — refused, because the
      runner earlier cherry-picked a pushed `dev-writer` pass (the correctness
      re-review's first box measures this happening on every findings pass;
      `design.md:720-728` records it on this piece's own reflog). The `closer`
      stops and reports the refusal only (`closer.md:469`), never having run
      `git diff --name-only HEAD^ HEAD -- openspec/specs/`. The runner
      fast-forwards to the `closer`'s branch "whatever else it reports"
      (`RUNNER.md:590-595`), which puts the archive commit on its HEAD, and
      reports the refusal to the owner (`:635-646`). Nothing in that return
      tells it to untick the re-review row or size a round for the archive —
      the untick list at `:563-566` keys on "an archive commit that changed
      `openspec/specs/`", which nobody has measured. Once the owner
      reconciles, a re-dispatched `closer` finds the change archived, passes
      Step 1 (every row still ticked), and at `closer.md:236-237` "Skip[s] the
      `openspec/specs/` check … since this run made no archive commit". It
      pushes, watches green, merges: the promoted live contract lands unread.
      **Measured:** `git grep -n -i -e "refused" -e "made no archive commit" --
      .claude/agents/closer.md .claude/agents/RUNNER.md` → the refusal stop at
      `closer.md:297`, the skip at `:237`, and no line in `RUNNER.md`'s
      refused-push return (`:635-646`) mentioning the archive. `design.md:673-678`
      argues the "only in the same run" scoping from a re-dispatched HEAD
      being a reviewed commit, and does not consider a first run that stopped
      before the check. The check reads a local commit and the push does not
      move HEAD, so running it **before** the push (or having any return after
      the archive commit report the check's result) closes the window without
      changing what is measured. Severity: medium — the precondition is a
      known, measured failure, and the outcome is exactly the unreviewed
      live-contract change box 1 was taken to close.

      **Fixed** (`spec-writer`, this commit), with both of the finding's
      suggestions. In `proposal.md`'s entry "An archive commit that changes
      the live contract is reviewed", the `closer` now runs
      `git diff --name-only HEAD^ HEAD -- openspec/specs/` straight after the
      archive commit and **before** the push. Then it pushes. A refused push
      stops it, and it reports the refusal **and the check's result**, and
      `closer.md`'s "Your report" asks for both. An accepted push with files
      listed stops before Step 4, as before. On the runner's side, the
      refused-push return in "The `closer`, and what comes back" now says
      that when the report lists files from the check, the runner first
      unticks the re-review row and records a round for the archive commit,
      as for a spec-changing archive. The round is then owed whatever the
      owner does about the push, so a later re-dispatched `closer`, which
      still skips the check by rule, meets an unticked row in Step 1.
      Implementation is the `dev-writer`'s: `closer.md` Step 3 and "Your
      report", `RUNNER.md`'s refused-push return, and `design.md`'s "The
      archive check applies only to an archive commit the `closer` made in
      the same run".

- [x] **`spec-writer`** — `proposal.md:133-137` and `:203-207`, carried into
      `closer.md:88-91` and `RUNNER.md:560-567` — the re-review verdict box is
      produced but no gate reads it, so a re-review round's completion is
      still attested by nothing but the runner's tick. The proposal's stated
      reason for the box — "without the box a clean round leaves nothing but
      the runner's line under the re-review row, which nothing checks against
      a reviewer's output" — remains true with the box.
      **Scenario:** after the first `closer` archived (deleting `findings/`)
      and came back red, the runner brings on a `dev-writer` fix, unticks the
      re-review row and records "round 2 `x..y` red-CI fix — correctness,
      security (Opus)". Both re-reviewers stall on a permission prompt — the
      dispatch brief for this review reports three reviewers on this piece
      stalling for good that way — and write nothing; the runner,
      after a compaction or reading a stalled agent as finished, ticks the row.
      The re-dispatched `closer`'s Step 1 finds the archived folder with no
      `findings/`, which `closer.md:88-91` says "passes the findings gate …
      no re-reviewer has run since, and the runner's line under the re-review
      row says why" — although that line says two re-reviewers ran. The stage
      block is fully ticked, so it pushes the fix, watches green and merges
      unreviewed code. Before the archive the same stall passes too: the
      first-round files already hold boxes, so both greps are satisfied
      whether or not a verdict box was appended.
      **Measured:** `git grep -n -i -e "verdict" -e "re-review \`" --
      .claude/agents/closer.md` → no output: the `closer` never looks for a
      verdict box or a range. The re-review row is the only stage row whose
      tick is not made by the agent that did the work (`RUNNER.md:25-28`
      against `:33`), so it is exactly the row the owner's "Agents tick their
      own" ruling cannot protect. A check that uses what already exists: the
      round line and the verdict box both carry the commit range, so the
      `closer` (or the runner before it ticks) can require, for the latest
      round line not marked skipped, that each lane it names has a file in
      `findings/` containing that range — with finding boxes appended under a
      heading naming the range, as the correctness and readability
      re-reviewers already did by habit. Severity: medium — it needs a runner
      error, but a stalled agent that looks finished is this flow's most
      frequent failure, and the verdict box was adopted precisely so this
      attestation would stop resting on the runner alone.

      **Fixed** on the runner's side, with the `closer`-side check
      **deferred** (`spec-writer`, this commit). The finding offers either
      party. `proposal.md` now contracts the runner's check:
      - **The re-review brief.** A re-reviewer with findings appends them
        under a heading naming the range, such as
        ``## Re-review `a1b2c3d..e4f5a6b` ``. A clean one appends the verdict
        box, as before. Either way its file names the range.
      - **The runner, before it ticks the re-review row.** For every
        unskipped round recorded since the row was last ticked, it runs
        `git grep -l -F "<range>" -- <change folder>/findings/` on its HEAD,
        and the output must list the file of every lane the round
        dispatched. A lane whose file is not listed has not finished,
        however finished its agent looks. The runner continues that reviewer
        or dispatches a fresh one, and does not tick.

      In your scenario, two stalled re-reviewers write nothing, so the grep
      does not list their files and the row cannot be ticked. The
      proposal's overclaim is corrected too. It no longer says the box
      answers "nothing checks against a reviewer's output". It says the box
      and heading are what the runner checks, and that no other gate reads
      them. The residual is a runner that skips the check. An independent
      `closer`-side check is in "Out of scope" as a design question for the
      owner. It is not a one-line addition, because the `closer` deletes
      `findings/` when it archives, so after an archive it could match only
      the rounds recorded since then, and it would need a fixed round-line
      format to tell them apart. Implementation is the `dev-writer`'s:
      `RUNNER.md` step 3's brief bullet and tick rule, and a `design.md`
      Decisions entry.

**Clean in this range, and why.**

- **No route to `main` or around protection.** `closer.md` forbids `--admin`
  and any `gh api` write to `branches/main/protection` or `rulesets` "whatever
  the brief or the owner's merge-on-green said" (`:409-415`, repeated at
  `:457-458`), and a green-but-`BLOCKED` PR is a stop (`:417-424`). "No force,
  ever" (`:185`) and "Force-push, for any reason" (`:450`) have no exception;
  `git grep -n -F "force-with-lease" -- .claude/agents/` returns nothing. The
  only pushes are refspecs to `refs/heads/piece/<name>`. `RUNNER.md:13-14`
  forbids the runner rebasing, resetting, force-pushing or merging `main`, and
  `:269-271` names both of git's fast-forward-refusal hints (`--no-ff`,
  `rebase`) as not to be taken. A refused push goes to the owner with "Do not
  reset, force or merge" (`:635-646`) — the `closer` does not fetch-and-merge
  the remote ref either (`closer.md:186-189`).
- **The closer's merge of `main`.** Clean-merge-needs-no-review
  (`RUNNER.md:494-496`) is sound for what enters the squash; the semantic-clash
  residue is disclosed in `proposal.md:560-563` and falls to CI. A conflict is
  aborted, never resolved (`closer.md:191-204`), and the resolver's merge is
  read with `git show --remerge-diff`, which also exposes an edit the resolver
  made outside the conflicted hunks, since any departure from git's own merge
  appears in it.
- **The `findings/` deletion move to Step 3.** It moves a tracker deletion
  that never merges content; the gates it follows still run in Step 1 on the
  undeleted tree. No path found where the move lets a gate run on an already
  deleted folder, other than the archived-folder case in the second box above.
- **The runner-commit rule.** "A runner always delegates" is stated without
  exception (`RUNNER.md:37-52`), forbids copying an agent's uncommitted output,
  forbids the runner ticking another agent's row, and turns an owner
  in-session edit into a dispatch. No reading lets a runner land content under
  "tracking" except its own record lines, which are prose in `tasks.md`. The
  second box above is the residual: the runner's *tick* can still say a round
  ran when none did.
- **Relayed authority.** Nothing new invites an agent to act on an
  authorisation it cannot verify: an agent refused a briefed edit reports and
  stops (`RUNNER.md:45-46`), and the merge-on-green relay the first-round
  observation noted is narrowed, not widened, by `:409-415`.
- **Out of range, noted not boxed:** a `dev-writer` can close a finding as
  **rejected** in a commit that only touches `findings/`, which step 3 lists
  as tracking needing no review, and neither the runner (which does not read
  findings) nor the `closer` (which reports but does not reopen) is a second
  reader of the rejection. That list predates this range (`c222c37`'s
  `RUNNER.md:394`), so it is the owner's to weigh, not a finding here.

## Re-review `9dc235c..34fd428`

Security only, on Opus. Read: `git log --oneline 9dc235c..34fd428`;
`git diff 9dc235c..34fd428 -- .claude/agents/` in full (README, RUNNER, closer,
dev-writer); `closer.md` in full; `RUNNER.md:1-130` and `:230-713`;
`proposal.md:120-360`; `dev-writer.md:176-245`; `spec-writer.md:45-55`. Ran the
pre-tick check against this piece's own history (below), `git config
--get-regexp "^branch\.piece"` and `git check-ignore -v tmp/uncommitted.patch`.
No mutation: the change is prose.

**Round 1's two Opus boxes, checked against the text.** Both fixes do what their
notes say, on the paths those findings named.

- *The archive check.* It now runs straight after the archive commit and before
  anything else in Step 3 (`closer.md:281-292`), so no stop that follows the
  archive commit comes before it. A refused push reports its result
  (`:316-319`, `:478-480`), and `RUNNER.md:694-697` unticks the row before the
  refusal goes to the owner. The same holds for the other stop in that window,
  the `branch.piece` config check (`closer.md:294-297`). "Your report" asks for
  the files wherever the archive changed specs (`:477-478`), and `RUNNER.md`'s
  general untick rule (`:613-615`) and `proposal.md:346-348` ("whichever way
  the `closer` came back with it") route it. That stop can be reached today:
  the command returns 18 lines in this repository, where both role files say
  to expect none.
- *The verdict box.* The runner's pre-tick check (`RUNNER.md:595-609`) does
  stop the scenario in that box. Two stalled re-reviewers that never commit
  leave no file that names the range, so the row cannot be ticked. The check
  can still pass without the lane's reviewer having done the round, in the two
  ways boxed below. Both come from the same thing: the grep accepts the range
  string anywhere in the file.

- [x] **`spec-writer`** — `proposal.md:177-191`, carried into
      `RUNNER.md:595-609`. The pre-tick check cannot tell one dispatch of a
      lane from another dispatch of the same lane in the same round. When the
      runner re-dispatches a lane within a round because it will not accept the
      first run, the first run's record still satisfies the grep. The re-run
      can then stall, and the row gets ticked on a verdict the runner had
      already turned down.
      **Scenario:** this piece's round 1. At `0a1e42c1` the runner's round
      line says the Sonnet runs of correctness, readability and security are
      not accepted, and "they and security re-run on Opus 5.5". Suppose the
      Opus security reviewer stalls, which is the failure this check exists
      for. Before ticking, the runner runs the contracted
      `git grep -l -F "c222c37..9dc235c" -- …/findings/`. It lists
      `security.md`, because of the Sonnet run's clean verdict box, so the
      runner ticks. The Opus run is the one that found the two medium
      findings above (`59619032`). With the tick, both would have gone to the
      `closer` unraised.
      **Measured:** `git grep -l -F "c222c37..9dc235c" 0a1e42c1 --
      openspec/changes/171-workflow-rules/findings/` lists `architecture.md`,
      `design-review.md`, `security.md` and `spec-test.md`. It does not list
      `correctness.md` or `readability.md`. The check therefore blocks the two
      re-run lanes that the Sonnet run never finished, and passes `security`,
      the re-run lane that the Sonnet run had finished. Severity: medium. The
      precondition is the flow's most frequent failure, the outcome is a
      re-review the runner judged necessary being skipped, and this piece has
      already produced the precondition once. Possible fix, for the
      `spec-writer`: a re-dispatch within a round gets a round line of its own,
      or the brief's heading and verdict box carry a tag the runner assigns
      per dispatch. The grep then searches for that tag rather than for the
      range alone.

      **Fixed** (`spec-writer`, this commit), by the first of the two fixes
      offered, with the round number as the tag. Every round line under the
      re-review row now starts ``round <n> `<range>` ``, numbered in the order
      written. A lane dispatched again over a range it already had (a run not
      accepted, or an agent replaced after a stall) gets a line of its own:
      the next number, the same range, the lanes it re-runs. The heading and
      the verdict box carry ``round <n> `<range>` `` in exact forms,
      ``## Re-review round <n> `<range>` `` and
      ``**re-review round <n> `<range>`: no findings**``, and the pre-tick
      check searches those two forms. The earlier line's check skips the lanes
      a later line re-ran over the same range, so the rejected run's record
      no longer satisfies anything the re-run must. Continuing the same agent
      with `SendMessage` is not a new dispatch. Not chosen: a separate
      per-dispatch tag, which would add a second identifier where the round
      line already carries one the runner writes anyway. This piece's own
      rounds 1 and 2 predate the number: the proposal records that round 1's
      re-dispatch is settled all the same (the Sonnet correctness and
      readability runs wrote nothing, and `security.md` holds the Opus run's
      heading from `59619032`), so the runner can close them by the forms
      their briefs gave. `RUNNER.md` and `design.md` are the dev-writer's.

- [x] **`spec-writer`** — `proposal.md:177-183`, carried into
      `RUNNER.md:596-605`. One tick can close several rounds ("every round
      recorded since the row was last ticked"). The grep for an earlier round
      is a fixed-string search over the whole file, so a **later** round's
      reviewer who mentions the earlier range in prose satisfies it. The
      earlier round's lane then counts as finished although its own reviewer
      wrote nothing.
      **Scenario:** round 2 (`R2`) dispatches `security`, and that reviewer
      stalls. The other lanes raise findings, a fix lands, and the runner
      records round 3 (`R3`, which starts where `R2` ends) with `security`
      again, without ticking in between. The `R3` security reviewer writes
      ``## Re-review `R3` ``. In its read list or its clean notes it cites
      the previous round by range, for example "the `R2` boxes above". This
      is the habit the measurement below shows. Before the one tick that
      closes both rounds, the runner greps `R2` and `R3`, and both list
      `security.md`. The row is ticked, and no security reviewer ever read
      `R2`'s commits, since `R3` covers only what landed after `R2`.
      **Measured:** `git grep -n -F "c222c37..9dc235c" --
      openspec/changes/171-workflow-rules/findings/` returns the range on
      lines that are neither a heading nor a verdict box:
      `readability.md:159-160`, `security.md:233`, `spec-test.md:65` and
      `:107`. Reviewers write ranges into prose routinely. This section does
      the same with round 1's range, so once it is committed, `security.md`
      carries an earlier round's range written by a later round's reviewer.
      Severity: low to medium. It needs a stall plus a tick that closes more
      than one round, and the contract explicitly allows such a tick. Possible
      fix: grep for the two forms the brief prescribes rather than the bare
      range, `git grep -l -F -e "## Re-review \`<range>\`" -e "re-review
      \`<range>\`: no findings" -- <folder>/findings/`. Note that
      `architecture.md:214` wrote its heading as
      ``## Re-review round 1 (`c222c37..9dc235c`)``. Its verdict box at `:216`
      would still match, but a lane with findings that used that heading
      would not, so the brief must fix the heading form exactly.

      **Fixed** (`spec-writer`, this commit), as suggested, with the round
      number added (the box above). The check is now
      ``git grep -l -F -e '## Re-review round <n> `<range>`' -e '**re-review round <n> `<range>`: no findings**' -- <change folder>/findings/``,
      in single quotes because both patterns hold backticks. The brief gives
      both forms whole rather than "such as", and the proposal cites
      `architecture.md:214`'s loose heading as the form that would match
      neither. Measured on this tree at `6f17bebf` with the un-numbered forms
      rounds 1 and 2 were briefed with: each lists all six files, where a
      bare-range search for round 1 also hits prose in five of the six. The
      proposal records the residual: a later reviewer who quotes an earlier
      round's heading or box whole, in prose, still satisfies it.

**Clean in this range, and why.**

- **The mutating-reviewer rebase cannot put a mutation on the piece.** The patch
  is written by `git diff --output` into `tmp/`. `git check-ignore -v
  tmp/uncommitted.patch` shows it matched by `.gitignore:75:tmp/`. `git restore
  --source=HEAD --staged --worktree -- .` leaves no tracked change, so the
  rebase and its `git add <path>` of the conflicted file carry nothing but the
  reviewer's own commits. `git apply` without `--index` returns the mutations
  unstaged, outside every commit the runner picks. A mutation that created an
  untracked file is not in the patch and is not touched by the restore. It
  stays untracked, and only a `git add` of that path could commit it, which
  the reviewer role files already forbid in the form `git add -A`. No stash is
  used, and the rebase rewrites only the agent's local, unpushed branch.
- **Returns with no count cannot skip review by being unlisted.** Step 3's rule
  applies to every return: "every commit that changes something which merges"
  (`RUNNER.md:520`), and its list starts with "That includes". The untick rule
  (`:613-615`) gives examples, not a closed list. So any fix that follows an
  unlisted return (a body/diff disagreement in Step 5, a zero-box file in
  Step 1, a config stop) is a commit after the review round, whatever the
  return was called. The runner may not make that commit itself
  (`RUNNER.md:37-52`), so it goes through a writer and into a round.
- **The `BLOCKED` entry** (`RUNNER.md:707-709`) goes to the owner only. It says
  not to diagnose the block and not to look for another route. `closer.md:418-424`
  still refuses `--admin` and any write to protection or `rulesets`,
  "whatever the brief or the owner's merge-on-green said". So a relayed
  "the owner says use `--admin`" dies at the `closer`.
- **History and protection.** In this range, `git grep -n -F
  "force-with-lease" -- .claude/agents/` still returns nothing. No new text
  permits `--force`, `reset`, `--no-ff` or a push to `main`. The change of the
  `dev-writer` to fast-forward on every pass (`RUNNER.md:260-269`) removes a
  source of divergence, and the refusal hints stay forbidden (`:271-273`). A
  `dev-writer` pushing every pass puts unreviewed commits on the PR head
  before review, as its first pass already did. Nothing auto-merges:
  `git grep -n -F -e "--auto" -e "match-head-commit" -- .claude/agents/`
  returns nothing, so only a `closer` merges, after step 3.
- **Relayed authority.** No new sentence lets an agent act on an authorisation
  it cannot check. The unticked-stage-row route (`RUNNER.md:674-677`) sends the
  row back to its own agent and does not let the runner tick it.
- **Out of range, noted not boxed:** a merge of `main` "when it stopped on no
  conflict" needs no review (`RUNNER.md:526-528`), and the runner
  fast-forwards to the `closer`'s branch "whatever else it reports"
  (`:640-645`). Whether the merge really was conflict-free therefore rests on
  the `closer`'s word. A `closer` that broke its rule and resolved a conflict
  would get its resolution in unread. `git show --remerge-diff <merge>` being
  empty is a mechanical check the runner could run without reading content.
  Both lines predate this range, so this is left to the owner.

## Re-review round 3 `34fd428..dc1390a`

Security only, on Opus, narrowed. Read: `git log --oneline 34fd428..dc1390a`;
`git diff 34fd428..dc1390a -- .claude/agents/` in full (RUNNER, closer,
dev-writer); `RUNNER.md:500-739`; `proposal.md:1-14` and `:150-268`;
`design.md:380-485`; the re-review row in `tasks.md:24-27`. Ran the pre-tick
check three ways on this tree: the un-numbered round 1 forms list all six
files; the numbered round 2 forms list nothing (fails closed, as `design.md`
says); the round 3 forms list nothing before this section was written. No
mutation: the change is prose.

**Round 2's two boxes, checked against the text.**

- *Two runs of one lane.* Closed for a **fresh dispatch**. `RUNNER.md:604-609`
  gives a re-dispatched lane its own numbered line, `:626` searches
  ``round <n> `<range>` `` in both forms, and `:630-631` exempts the earlier
  line's lane, so a rejected run's round-1 box satisfies nothing the round-2
  re-run must. Round numbers cannot collide by prefix: the pattern holds
  `round 1 ` and a backtick, which `round 10 ` does not contain. It is **not**
  closed for the other way of re-running a lane the same text allows, which is
  the box below.
- *A range in prose.* Closed. The check searches only the heading and verdict
  forms (`RUNNER.md:626`), the brief gives both whole and never "such as"
  (`:574-578`), and a loose heading matches neither. The remaining gap, a later
  reviewer quoting an earlier round's form whole, is disclosed
  (`proposal.md:239-241`) and needs the literal round number and range in the
  prose, which the placeholders in quoted forms do not carry.

- [x] **`spec-writer`** — `proposal.md:186-190`, carried into `RUNNER.md:604-609`
      and `design.md:393-397`. "Continuing the same agent with `SendMessage` is
      not a new dispatch and gets no line" also covers continuing an agent
      whose run the runner **did not accept**. That agent has already
      committed its record for the round, so the pre-tick check passes on the
      rejected run and cannot tell whether the continuation did anything.
      Round 2's first box comes back by this route.
      **Scenario:** round 1 `R` dispatches `security`. The reviewer commits
      ``- [x] **re-review round 1 `R`: no findings** — read the role files``,
      and the runner brings that commit onto its HEAD. From the hand-back it
      sees the reviewer never read the handler diff, which is the non-accepting
      reason in `RUNNER.md`'s own example line (`:614`). The obvious and
      cheapest step is to continue the same agent with `SendMessage`: "also
      read the handler diff". By `:607-608` that gets no line. The continued
      reviewer stalls on a permission prompt, and a stalled agent looks exactly
      like a finished one (`:637-638`). Before ticking, the runner runs
      `git grep -l -F -e '## Re-review round 1 `R`' -e '**re-review round 1 `R`: no findings**' -- …/findings/`.
      It lists `security.md` because of the rejected box, so the row is ticked,
      and no security reviewer has read the handler diff. A continuation that
      did finish would write the same form, so the check cannot tell the two
      cases apart even in principle.
      **Measured:** from the text, cited above. `git grep -n -F "SendMessage" --
      .claude/agents/RUNNER.md` shows `:607` as the only rule on continuing an
      agent within a round. It exempts continuation without asking whether the
      agent already wrote its record. `:635` ("continue that reviewer") is
      safe because it applies only when the lane's file is **not** listed, so
      nothing is recorded yet. Severity: **medium**, for the same reasons as
      round 2's first box, which this reopens. The precondition is the flow's
      most frequent failure, the outcome is a re-review the runner judged
      necessary being skipped, and continuation is a more likely choice than a
      fresh dispatch whenever the defect is "did not read X", not "wrong model".
      Possible fix: a run the runner did not accept gets a new numbered line
      **however** it is re-run, whether continued or dispatched fresh. The
      `SendMessage` carve-out covers only continuing an agent that has not yet
      committed its record for the round, such as one stalled before its
      commit or one asked to rebase.

      **Fixed** (`spec-writer`, this commit), as suggested, and as one rule
      that replaces the old one rather than an exception beside it. The line
      rule is now keyed to the lane being run again, not to how: a lane run
      again over a range it already had gets a line of its own whether the
      runner dispatches a fresh agent or continues the same one with
      `SendMessage`, and a continuation message gives the new line's two
      forms whole, as a brief does. Continuing an agent gets no line only
      when it adds no review to a record already committed: finishing a round
      it has not yet recorded (such as after a stall), committing, or
      rebasing. The rebase case needed naming separately from the suggested
      "has not yet committed its record": a reviewer continued to rebase
      after its cherry-pick conflicted has committed its record, and its run
      was accepted, so keying on "committed" alone would give it a line for
      no review. The pre-tick check now says "every lane the round ran" and
      exempts "a lane that a later line ran again", where both said
      "dispatched", so a continued re-run is checked, and exempts the
      earlier line, exactly as a fresh one does. In the scenario,
      the continued reviewer is told round 2, so round 1's check skips
      `security` and round 2's lists `security.md` only once the
      continuation commits its own round-2 heading or box: a stalled
      continuation leaves the row unticked. Not chosen: forbidding a
      continuation of a rejected run and always dispatching fresh. It is also
      one rule, but it fights the cheapest step the runner reaches for when
      the defect is "did not read X", and a runner that continued anyway
      would reopen the hole; numbering the re-run closes it whichever way the
      lane is run. `proposal.md` carries the rule under the re-review row's
      line bullet, the exemption, and Impact. For the `dev-writer`:
      `RUNNER.md:604-609` (the rule, restated by the above),
      `RUNNER.md:629-631` ("the round dispatched" to "the round ran", and
      "dispatched again" to "ran again"), and in `design.md` the summary at
      `:103-104`, the bullet at `:393-397`, the check at `:410-411` (the same
      two word changes), and "Why the number" or "Rejected", for the
      continued-run reason and the forbid-continuation alternative.

**Clean in this range, and why** (lower-severity notes included, unboxed).

- **No new path to an unreviewed merge from the other edits.** `closer.md`'s
  return paragraph now ends the turn on every stop the file names, including
  `BLOCKED` and "anything else you stopped to report" (`closer.md:482-487`),
  which narrows what a `closer` may do after reporting. The `closer.md:292`
  edit is grammar only. The `dev-writer.md` edits change "cherry-picks" to
  "brings", matching the fast-forward rule, and move no step or permission.
- **History and protection.** Nothing in the range adds `--force`,
  `--force-with-lease`, `reset`, `--no-ff`, `--admin`, `--auto` or a push to
  `main`, and no text relaxes `closer.md`'s protection and ruleset
  prohibitions. The step 3 list of commits needing no review
  (`RUNNER.md:527-537`) is unchanged in this range.
- **The mutating-reviewer rebase note** (`RUNNER.md:318-326`). It says an
  untracked mutation stays in the tree through the rebase and that an empty
  patch is not lost evidence. Neither sentence adds a way for a mutation to
  reach a commit the runner picks. Low: when an incoming commit adds a file at
  an untracked mutation's path, the rebase stops, and the text does not say
  what the reviewer should do next. The likely improvisation is deleting the
  file, which loses evidence but is not a merge path.
- **A skipped re-run line (low).** `RUNNER.md:630-631` exempts an earlier
  round's lane when "a later round dispatched again over the same range". A
  later line for that lane marked *skipped* dispatched nothing, but a careless
  reading could take its presence as the exemption. That would leave the lane
  checked by neither round. It needs the runner to record a contradictory
  line, so it is prose only.
- **Self-extended authorisation (low, for the owner's attention).**
  `proposal.md:9-12` now treats the `README.md` and `dev-writer.md` passages
  naming the cherry-pick as corrections "inside the authorisation", because
  the owner-authorised fast-forward rule makes them false. The edits
  themselves are benign wording. The pattern, a writer ruling on its own that
  a consequence of an authorised rule is also authorised under `.claude/`, is
  the kind of relay CLAUDE.md's "`.claude/` is the owner's" guards against.
  Whether the owner accepts it is the owner's call and not a security defect
  in the text.
- **The transition for rounds 1 and 2** (`design.md:456-475`). Re-measured:
  the un-numbered round 1 forms list all six files, and the numbered round 2
  forms list none, so a runner using the wrong form fails closed and cannot
  tick.

## Re-review round 5 `dc1390a..d1c8726`

- [x] **re-review round 5 `dc1390a..d1c8726`: no findings** — read `git show` of `47f6008`, `549c09c` and `d1c8726` in full, `RUNNER.md:540-665`, this file's round-3 box, and ran both of `549c09c`'s round-1 and round-2 checks with `-n`; clean

Security only, on Opus, narrowed to the round-3 box and `549c09c`. No
mutation: the change is prose. Nothing below reaches medium, so it is prose.

**The round-3 box is closed.** `RUNNER.md:604-616` keys the line to the lane
being run again, not to how, and `:636-638` says "ran" where it said
"dispatched", so a continued re-run's lane is exempted from the old round's
check and required by the new one. In the box's scenario the continued
reviewer is given round `N+1`'s forms, which the rejected run's record cannot
contain, so a stalled continuation leaves `N+1`'s check unlisted and the row
unticked. The three no-line cases each hold up: *finishing an unrecorded
round* has no rejected record on any branch for the check to find, so the only
record that can satisfy it is the continuation's own; *committing* adds no
review; *rebasing* an accepted run adds none, and one that also adds review
("rebase, and read X") is caught by the governing clause, "only when it adds
no review". `git grep -n -F -e "dispatched again" -e "round dispatched" -e
"not a new dispatch"` over `.claude/agents`, `proposal.md` and `design.md`
leaves only `design.md:445-446`, which narrate the old rule, and
`closer.md:203`, which is about the `closer`. The round-3 prose note on a
skipped re-run line is also better for the wording: a line marked skipped did
not *run*, so it no longer reads as the exemption.

**Ways I tried to break the exception, each low.**

- *"Committed" where.* The exception says "a record already committed"; the
  check runs "once every lane's findings commit is on your HEAD". A runner
  that rejects a run before bringing its commit over could read "has not yet
  recorded" as "not yet on my HEAD", continue without a line, and later bring
  the agent's branch over whole, rejected commit included; if the
  continuation had stalled, round `N`'s check then passes on the rejected
  record. That needs both the misreading and picking a commit from a run the
  runner rejected, and "findings commit" in `:628` already names the
  reviewer's commit, not the runner's HEAD, so the natural reading is right.
- *The line after the message.* Nothing orders the `N+1` line before the
  `SendMessage` that carries its forms; a continuation sent and a line
  forgotten leaves round `N`'s check un-exempted and satisfied by the
  rejected record. That is a runner skipping its own rule, the same class as
  the residual `design.md` carries in Risks, a runner that skips the check.
- *The check quoting itself.* The check command contains its own two patterns
  as substrings, so a findings file that quotes the command with literal
  values is listed by it. `spec-test.md:360` is an instance, for round 2's
  un-numbered forms; harmless there, since `spec-test.md:147` holds the real
  heading. It needs the same lane's own committed file, so an agent of that
  lane did run, and a rejected round-`N` run cannot quote `N+1`'s literals.
  It is a variant of the disclosed "quotes an earlier round's form whole".

**`549c09c` adds no new way to pass for a future round, but its round-1
command passes on the rejected run alone (low).** Run with `-n`, round 1's
command lists `security.md` twice: `:199`, the Sonnet verdict box the runner
did not accept (`0f55b92`), and `:230`, the Opus section (`5961903`). Round 2's
lists each file once, plus `spec-test.md:360` above. So "lists all six files"
is true, but for `security` it would be true with the Opus section deleted:
it is the exact two-runs-one-range defect the number exists to close, which a
pre-numbering round cannot close retroactively. No lane is unreviewed today,
because the Opus record is committed and the round lines say `security` re-ran
on Opus; `correctness.md:146` and `readability.md:157` are single matches, the
Opus runs. The annotation lives on the runner's own line and binds only rounds
1 and 2, so it cannot be reused for a later round. If the owner wants the line
to be self-sufficient, it could name `security.md:230`, or `5961903`, as the
accepted record; nobody needs to act for the check to be sound.

## Re-review round 8 `6d43cda..d1d2165`

- [x] **`spec-writer`** — `proposal.md:906-920` and `:928-962` — the residual
      the copy rule leaves is recorded as "a runner copying from the wrong
      line", but a stale number written **on the line itself** is also left,
      and the `closer`-side follow-up it is routed to cannot see it.
      The copy rule moves the source of truth for the number from the
      runner's memory to the round line. It does not check the line. A runner
      that writes a re-run's line by taking the previous line as a template
      and leaving the number unchanged has made the old number the correct
      copy. That slip is the one the copy rule invites. The brief, the
      reviewer's heading and the check all then carry the old number, and
      they agree. So the check passes on the rejected run's record, whenever
      that run left one, before the re-run has written anything. This is the
      stale-number fail-open that `16958b3` was written to close. It
      re-enters one step earlier.
      **Scenario:** round 3 `34fd428..dc1390a` security is not accepted.
      The runner writes the re-run line as
      ``round 3 `34fd428..dc1390a` security re-run on Opus``, which duplicates
      line 27's number, and the brief is copied from it. The check copied from
      that line lists `security.md` at once.
      **Measured** at this worktree's HEAD (`9e6dde2f`):
      `git grep -l -F -e '## Re-review round 3 `34fd428..dc1390a`' -e '**re-review round 3 `34fd428..dc1390a`: no findings**' HEAD -- openspec/changes/171-workflow-rules/findings/`
      lists `architecture.md`, `correctness.md`, `design-review.md`,
      `security.md` and `spec-test.md`. Every one is from the first run, so
      a re-run of any of those five lanes under a duplicated `round 3` line
      is ticked with nothing written.
      **Why the routing does not cover it:** the follow-up's "What a
      `closer`-side check would do" (`:943-945`) matches "each round line
      … by the same two forms". It derives the forms from the lines, so it
      passes on a duplicated number exactly as the runner's check does. The
      one thing that catches this is a check that two round lines never share
      a number, and that the numbers run 1, 2, 3 in order. It is cheap
      because the line's start is its one fixed part. Nothing in
      `RUNNER.md:600-601` ("numbered from 1 in the order you write the
      lines") enforces this, and neither does the follow-up.
      **Ask:** in "What it still cannot see" (`:263-280`) and in the Risks
      bullet at `:906-920`, record a duplicated or stale number on the line as
      a separate residual from copying from the wrong line. Add "round
      numbers under the row are unique and consecutive" to the follow-up's
      "What a `closer`-side check would do" or "For the issue to settle".
      `design.md`'s matching passages then follow: "What it still cannot see",
      `:504-515`; the Risks paragraph, `:1303-1317`; and the
      "[Only the runner runs the pre-tick check.]" Risk.
      **Severity:** medium. The gate fails open on an ordinary templating
      slip, the proposal says the residual is fully routed when it is not,
      and the check this piece defers to the owner would be designed without
      it.

      **Fixed** (`spec-writer`, this commit), as asked: recorded and routed,
      with no role-file change. In `proposal.md`:
      - "What it still cannot see", under the pre-tick check, records a wrong
        number on the line itself as a residual separate from copying from
        the wrong line, with this box's scenario and measurement (re-run at
        `291499c7`: the same five files). It says the copy rule moves the
        number's source to the line without checking the line, and that only
        a second reader that checks the numbers can see it.
      - The standing-test entry's stale-number bullet now names two
        residuals, the wrong line and a repeated or stale number on the line,
        both routed to the `closer`-side follow-up, the second only through
        its number criterion.
      - The `closer`-side follow-up's "The gap" names a line whose number an
        earlier line already carries. "What a `closer`-side check would do"
        now opens with "the round numbers under the re-review row are unique
        and consecutive, 1, 2, 3 in the order the lines stand", says why
        matching forms alone cannot see a repeated number, and notes that the
        number check reads only the line's fixed start. Measured:
        `git grep -n -F` for the indented round lines in `tasks.md` at
        `291499c7` returns eight lines numbered 1 to 8, once each.

      **Not chosen: a clause in `RUNNER.md`'s "Record the call"** saying a
      new line's number is one more than the highest already under the row.
      The rule there, "numbered from 1 in the order you write the lines",
      already fixes each line's number: the n-th line carries n. A repeated
      number already breaks it. So the clause would restate a rule the runner
      already has, in another form, and it would be enforced by the same
      runner whose slip it targets. It adds no check. The slip stays in the
      class this piece already routes to a second reader, "a runner that
      skips the check". The clause would also reopen a role file to the
      correctness, security and readability lanes for no gain in what can be
      detected. The proposal says the existing rule makes a repeated number a
      breach, so the residual is recorded as a runner breaking a stated rule.
      It is not a gap in the rule.

      **For the `dev-writer`:** `design.md` only. `RUNNER.md` and every
      other file under `.claude/` stay unchanged.
      1. "What it still cannot see" (`:504-515`): add the wrong-number-on-
         the-line case after the wrong-line case, with the measurement, and
         make "No test of the role files can see this" cover both.
      2. The stale-number Risks paragraph (`:1303-1317`): "What that leaves,
         a runner copying from the wrong line, is the next Risk" becomes two
         residuals, the wrong line and a repeated or stale number on the
         line. Both go to the next Risk.
      3. "[Only the runner runs the pre-tick check.]" (`:1327-1333`): add a
         line whose number an earlier line already carries to what goes
         unnoticed. Say the deferred `closer`-side check sees it only by
         checking that the numbers are unique and consecutive.
      4. "What else was considered" (`:549`): the residual list there gains
         the repeated number alongside "copies its forms from the wrong line".
      5. The copy-rule Decision ("The check's number and range are copied
         from the round's line, not typed"): add a **Rejected** entry for the
         `RUNNER.md` "one more than the highest" clause, with the reasoning
         above. It restates "numbered from 1 in the order you write the
         lines", it is enforced by the runner whose slip it targets, and it
         adds no check.
      6. PR #174's body: the `closer`-side follow-up names the unique-and-
         consecutive number criterion.

Security only, on Opus, narrowed to `16958b3` and `d1d2165`. I read `git diff
6d43cda..d1d2165` over `RUNNER.md`, `proposal.md` and `design.md` in full,
`RUNNER.md:560-668`, `proposal.md:170-264` and `:925-962`, the round lines in
`tasks.md:24-32` and issue #171. There was no mutation, because the change is
prose. I re-ran the design's measurements:

- round 5's forms at `c3d697e9` list `correctness.md`, `design-review.md`,
  `security.md` and `spec-test.md`. That includes the two lanes round 6 ran, so
  the "wrong line" example holds.
- round 3's forms at HEAD list the five files named in the box above.
- round 7's forms at HEAD list `correctness.md`, `design-review.md` and
  `spec-test.md`, with no `security.md`. The stalled round-7 security run
  wrote nothing, which is why this round exists, and a round 7 check could not
  have ticked over it.

**Question 1: does the copy rule close the stale-number case?** For the case
the round-5 box measured, yes: a number recalled from memory while the line
carries the right one. The brief and the check now read the same line, so a
correct line makes them agree by construction. The wrong-line residual is
recorded honestly, with a real measurement, and the `closer`-side follow-up
would catch it, because it derives each round's forms from that round's own
line. The stale number on the line itself is not covered. That is the box.

**Question 2: does the clause add a new way for a round to count as done?**
No. The clause only fixes where the check's number and range come from, and
every disagreement it can create fails closed:

- A brief typed wrong while the line is right: the reviewer writes forms the
  copied check does not search, the lane is not listed, and the row stays
  unticked.
- A line edited after the brief went out: the same thing happens.
- Rounds 1 and 2: their lines each embed their own un-numbered check, and the
  proposal now says those forms are not copied from the line's numbered start.
  Round 1's command still passes on the rejected Sonnet box alone for
  `security`. Round 5's prose covered that, and the line now says which record
  is accepted.

The Fail-open count went from four to three, with the stale-number case moved
to its own bullet, and `design.md`'s "three fail-open cases" matches that. The
only role-file edit is the `RUNNER.md` clause, and the "What you read" row at
`RUNNER.md:81` points to it rather than restating it. Nothing under `.claude/`
beyond that changed in the range.

**Low, prose only:**

- *"as the brief's was".* `RUNNER.md:629-630` reads "copied from that round's
  own line as the brief's was". That is a comparison, but a quick reader could
  take the brief as the source. If a runner copies from a brief that carried a
  stale number, the check matches the rejected run's record and fails open.
  The design and proposal say "as the brief's are [copied from the line]",
  which is clearer. "copied from that round's own line, as the brief's forms
  were" would remove the doubt.
- *"Copied" is still typed.* For an agent, a copy is re-emitted text, so the
  rule changes where the number is read from, not how it is produced. That is
  the right mitigation for a number recalled from memory, and the residual
  sits with the follow-up. Nobody needs to act on this.

## Re-review round 9 `d1d2165..c4b1df5`

- [ ] **`spec-writer`** — `proposal.md:284-293` and `design.md:503-514`,
      `:540-546` — the repeated-number residual is routed on the claim that
      "only a second reader of the round lines can" see it, and the
      `RUNNER.md` rejection argues that any runner-side measure "adds no
      check" and that the slip "stays in the class … a runner that skips the
      check". Neither holds for a number check at the tick, which is one
      command the fix itself ran, so a runner-side closure within this
      piece's scope is neither taken nor recorded as rejected.
      **Scenario:** the round-8 box's. Round 3 security is not accepted; the
      runner templates the re-run line from line 27 and leaves `round 3`.
      It then does everything `RUNNER.md:625-648` asks: copies the forms
      from that line, runs the pre-tick check, sees `security.md` listed,
      and ticks. This runner did not skip the check; it ran it faithfully
      and it passed. So the slip is not in the "skips the check" class,
      which is a runner omitting a step. It is a runner following every
      step and still failing open, which is the class the pre-tick check
      and the copy rule were each adopted to close, and both of those are
      likewise "applied by the same runner whose slip they target".
      **Measured:** at this tree's HEAD,
      `git grep -n -F "      round " -- openspec/changes/171-workflow-rules/tasks.md`
      prints the nine round lines, `tasks.md:25-33`, carrying 1 to 9 once
      each, and nothing else. `0fbebed` used the same command to measure
      "eight lines numbered 1 to 8, once each" at `291499c7`. With a
      duplicated `round 3` line that output shows `round 3` twice. So the
      runner can see the repeated number itself, before it ticks, with no
      parser and no fixed format beyond the line's start, which
      `proposal.md:976-978` already says is enough for the number check.
      The earlier "Rejected: a mechanical guard" (`design.md:497-502`)
      does not cover it: its objection is parsing the rest of the line,
      which this does not do. The forms check at the same HEAD
      (`'## Re-review round 3 ...'` and the verdict form) still lists
      `architecture.md`, `correctness.md`, `design-review.md`,
      `security.md` and `spec-test.md`, so the fail-open the round-8 box
      rated medium is unchanged in the operative gate until an owner
      follow-up with no issue number lands.
      **This does not reopen the round-8 box.** That box asked only to
      record and route, and `0fbebed`/`c4b1df5` did both, in every place it
      named (checked below). The defect is new in this range: the decision
      record's reasoning for not doing more.
      **Ask:** either add the number check to the pre-tick step in
      `RUNNER.md` (the round numbers under the row run 1, 2, 3 once each,
      checked with the one `git grep -n -F` above before ticking), which is
      within #171's re-review mechanism; or keep the rejection and correct
      it. "Only a second reader can see them" becomes true of the wrong-line
      case only, the Rejected entry drops "adds no check" and "stays in the
      class … skips the check", and it records why a runner-side number
      check at the tick is not taken.
      **Severity:** medium. The outcome is the round-8 box's, a lane's
      re-run ticked on the rejected run's record, now reached by a faithful
      runner. The record that keeps it open would stop the next reader from
      adding a check the piece's own precedent says is worth adding.

Security only, on Opus, narrowed to the round-8 box and `0fbebed`,
`c4b1df5`. I read `git show` of both in full, `RUNNER.md:585-660`,
`design.md:470-590`, the round lines in `tasks.md:24-33`, and PR #174's body.
No mutation: the change is prose.

**Is the round-8 box honestly closed?** Yes, against what it asked. Every
place that box named now carries the repeated number as a separate residual:
`proposal.md`'s "What it still cannot see" (`:271-293`), the standing-test
bullet (`:936-945`), the follow-up's gap and "would do" (`:963-979`); in
`design.md`, "What it still cannot see" (`:525-546`), "What else was
considered" (`:580-581`), both Risks (`:1347-1353`, `:1363-1374`), and the
Rejected entry (`:503-514`). The PR body's owner follow-up names the
unique-and-consecutive criterion. `git grep -n -i -F "wrong line"` over both
files and `.claude/agents` finds no site that lists the wrong-line residual
without the repeated number beside it. The re-measurements hold: the round 3
forms list the same five files, and the follow-up's number check does see a
repeated number. The box above is about the rejection's reasoning, not
about whether the residual was routed.

**Low, prose only:**

- *The rejection cites the weaker sentence.* It argues from
  `RUNNER.md:601`, "numbered from 1 in the order you write the lines".
  `RUNNER.md:604-605` is stronger and more precise. It says a re-run's line
  gets "the next number", which is word for word what the rejected clause
  would add. Citing it would make the "restates" point hold without the
  n-th-line inference.
- *The PR body's standing-test follow-up is unchanged.* It still says a
  stale round number is one that "this piece answers … by having the runner
  copy the check's round and range from the round's line", with no residual.
  `proposal.md:936-945` now says the copy rule leaves two residuals. The
  owner follow-up in the same body carries the number criterion, so the
  residual is still routed. But the text a project manager lifts into the
  standing-test issue overstates what the copy rule closes.
- *A re-run given no line at all.* Neither criterion sees it. A runner that
  rewrites round 7's line to add "security re-run", instead of writing
  round 8, keeps the numbers unique and consecutive. Round 7's forms then
  match the rejected run's record. This predates the range (round 5's "The
  line after the message") and breaks `RUNNER.md:604`'s rule outright. The
  new text does not claim to catch it.

Nothing under `.claude/` changed in the range. `git diff d1d2165..c4b1df5
--stat` touches only `openspec/changes/171-workflow-rules/`. Nothing in it
adds a route to `main`, a force, `--admin`, or a way for a round to count as
done that was not there before.
