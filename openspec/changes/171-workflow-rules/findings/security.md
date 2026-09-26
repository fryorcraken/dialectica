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
