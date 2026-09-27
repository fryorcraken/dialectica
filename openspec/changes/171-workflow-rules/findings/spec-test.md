# spec-test review — 171-workflow-rules

Scope actually reviewed: `proposal.md`, `tasks.md`, `.openspec.yaml`, issues
#171, #170, #169, #133 (via `gh issue view`), and `git diff origin/main...HEAD
--stat` (stat only). No file under `.claude/agents/`, no `design.md`, no other
findings file and no diff body was read, per the brief's restriction.

- [x] **`dev-writer`** — `openspec/changes/171-workflow-rules/tasks.md:11-15`
      (the struck tester row) versus `proposal.md:132-139` and `tasks.md:110-114`
      (task 7.1, the `closer.md` Step 1 pathspec) — the tester row is struck on
      "no CI job, script or test reads `.claude/agents/` ... there is no
      executable behaviour to assert. ... The check that can see this change is
      the six reviewers reading the prose." That rationale is sound for the
      *wording* of the agent files, but the contract also specifies a literal,
      deterministic shell command that `closer.md` Step 1 is to run —
      `git ls-files -- "openspec/changes/<name>/tasks.md"
      "openspec/changes/archive/????-??-??-<name>/tasks.md"`, required to
      return "exactly one path back," stopping the closer on "more than one, or
      none." That claim is checkable against fixture directories without
      parsing a single word of any agent's prose — it is `git`/pathspec
      behaviour, the exact category the "no executable behaviour" rationale
      does not reach. Task 7.1 records only a one-off "Verify:" line, run once
      by the dev-writer during authoring, not a standing test.
      **Scenario:** a later edit to `closer.md` narrows or mistypes the glob
      (e.g. drops the `archive/` alternative, or gets the `????-??-??` digit
      count wrong) so an archived change now resolves to zero paths, or an
      unrelated change with a name that is a substring/superstring of another
      resolves to more than one. Per the rule the closer then "stops and
      reports," so the immediate failure is safe rather than silently wrong —
      but nothing catches the regression before a re-dispatched closer hits it
      on a real piece, and the six review lanes plus this one are prose
      readers, not executors of the command.
      **Measured:** ran the exact command shape from task 7.1 against this
      tree. `git ls-files -- "openspec/changes/171-workflow-rules/tasks.md"
      "openspec/changes/archive/????-??-??-171-workflow-rules/tasks.md"`
      returns only the live path; the same shape for `op-clock` returns only
      `openspec/changes/archive/2026-09-16-op-clock/tasks.md`; for `clock`
      (substring only) returns nothing — all three match what the proposal
      claims, so the pathspec is correct today. Separately,
      `git grep -n -F ".claude/agents" -- .github/` returned nothing, and
      listing `.github/workflows/` and `dialectica-ui/tests/` found no script
      that runs this command against fixtures — confirming no gate exists to
      catch a future regression in it. Severity: moderate — the current
      behaviour is correct and the failure mode is a stop rather than a wrong
      merge, but the "there is no executable behaviour to assert" framing used
      to strike the whole tester row is broader than the file actually is, and
      this one command is exactly the "pathspec's behaviour" example my brief
      names.

      **Deferred** — to a follow-up issue, listed in PR #174's follow-ups for
      the project manager to file, and recorded in `design.md`'s Risks ("The
      `closer`'s Step 1 pathspec has no standing test"). The finding is right
      that the command is executable and that the struck row's "no executable
      behaviour" is broader than the file. It is not added in this piece for
      two reasons. `proposal.md`'s
      Impact says this change adds no tests or CI, and the stage block's
      struck tester row is the `spec-writer`'s. And a test holding its own
      copy of the command would not fail when a role file's copy changed,
      which is the regression the scenario describes; the useful test
      extracts the command from `closer.md` and `RUNNER.md` and runs it
      against fixture names from the `lint` job, which is a piece of its own.

## Re-review `c222c37..9dc235c`

Scope actually reviewed: `git diff c222c37..9dc235c -- openspec/changes/171-workflow-rules/proposal.md` (full diff read); `tasks.md` and `.openspec.yaml` in full; `git diff c222c37..9dc235c --stat` (stat only); issues #171, #170, #169, #133 (already read in round 1, re-checked for coverage against the new text). No file under `.claude/agents/`, no `design.md`, and no other findings file was read.

- [x] **`spec-writer`** — round 1's pathspec-test finding is deferred, but the
      deferral is invisible in the contract
      **Scenario:** `proposal.md`'s "Out of scope" section is where this
      contract records every other deferred item, each with its own reasoning
      and a note for the project manager to file ("One file per stage row",
      "A clean first-round review with no box (#182)", "Why #165 was
      `BLOCKED`", etc.). Round 1's finding above — that the `closer`'s Step 1
      pathspec is a deterministic, checkable command with no standing test —
      was accepted: its own "Deferred" note says it is tracked "to a
      follow-up issue... and recorded in `design.md`'s Risks". But
      `proposal.md`, the document this `skip_specs` change designates as the
      contract, never mentions it: a reader of `proposal.md` alone has no way
      to learn this gap was found and consciously deferred rather than never
      noticed. The only trace anywhere outside `design.md` is one unlabelled
      clause in `tasks.md`'s Implementation notes, which names no follow-up
      and points only at `design.md`.
      **Measured:** `git grep -c -F "pathspec" openspec/changes/171-workflow-rules/proposal.md`
      returns no output (zero matches, over the whole file, not just the
      diff). `git grep -n -F "clean re-review trace"` matches once, at
      `tasks.md:174`, and zero times in `proposal.md`. Severity: moderate —
      matches round 1's own rating; the gap is real and was found, just not
      surfaced where the contract's own convention says a deferred gap
      belongs.

      **Fixed** (`spec-writer`, this commit). `proposal.md`'s "Out of scope"
      gains "A standing test for the git commands the role files name", which
      carries the pathspec together with the commands the next box names, says
      what was measured and where, why no test is added in this piece, and that
      it is a follow-up for the project manager in place of the pathspec-only
      one PR #174 lists. The struck tester row in `tasks.md` no longer says
      there is no executable behaviour: it says the role files' git commands
      are executable, were measured once, and points to that entry. Left for
      the `dev-writer`: `design.md`'s Risks entry "The `closer`'s Step 1
      pathspec has no standing test" widened to match, and PR #174's
      follow-up list updated.

- [x] **`spec-writer`** — new deterministic-command claims added in this range
      get no equivalent deferral
      **Scenario:** this range adds several brand-new prose claims about the
      exact output of a git command the `closer`/runner runs, each entirely
      new in `c222c37..9dc235c` (none of the four strings below appear in
      `proposal.md` before `c222c37`): the archive-commit gate
      `git diff --name-only HEAD^ HEAD -- openspec/specs/`, `git merge
      --ff-only`'s keep-or-refuse behaviour, `git show --remerge-diff <sha>`
      for a conflict resolution, and `git log --oneline --left-right
      --cherry-mark HEAD...origin/piece/<name>`'s output shape. These are the
      same class of claim round 1 flagged for the Step 1 pathspec — a
      deterministic shell command's behaviour, asserted in prose, with
      nothing that can see `.claude/agents/` at all (`tasks.md`: "no test can
      see any of these tasks"). `tasks.md`'s own verify lines (8.2, 8.4, 9.1,
      9.3, 9.7, 9.8) show each was checked once against a scratch repository
      at authoring time — the same one-off verification the pathspec had
      before round 1's finding was written. Unlike the pathspec and the
      verdict-box grep behaviour (which at least got the tasks.md mention
      above), nothing in `proposal.md` or `tasks.md` flags these four as a
      known, accepted gap; they read as settled rather than as
      measured-once-and-untested, which is an inconsistency within the
      contract's own treatment of the same risk.
      **Measured:** counted occurrences of each string in `proposal.md` at
      `c222c37` vs `9dc235c` with `git grep -c -F "<string>" <rev> --
      openspec/changes/171-workflow-rules/proposal.md` (no output means zero):
      `"openspec/specs/"` 0→8, `"ff-only"` 0→2, `"remerge-diff"` 0→1,
      `"cherry-mark"` 0→1. Severity: moderate — same reasoning as round 1's
      pathspec finding; the commands were measured once and are presumed
      correct, but no gate would catch a future edit that got one wrong, and
      that gap is currently untracked for these four while it is tracked for
      the pathspec.

      **Fixed** (`spec-writer`, this commit), in the same "Out of scope"
      entry as the box above. It names all four, plus step 1's
      `git diff --name-only -G "NO SPEC:"` and the runner's new pre-tick
      `git grep -l -F "<range>"`, and it says which fail open and which fail
      closed. A mistyped `openspec/specs/` path in the archive check lists
      nothing and lets the `closer` carry on, so an unreviewed spec change
      would merge. That makes it the first command a test should cover. A
      mistyped pathspec or pre-tick grep stops the agent instead. The
      `--ff-only`, `--remerge-diff` and `--cherry-mark` claims describe git's
      own behaviour, and the entry says a test of them would mostly re-test
      git. No test is added in this piece, for the reasons the entry gives.

## Re-review `9dc235c..34fd428`

Scope actually reviewed: `proposal.md` in full at `34fd428`, and specific
strings in it at `9dc235c` via `git grep -n -F <rev>` (the full range diff
was too long to display, so the per-string comparisons below stand in for it);
`tasks.md` and `.openspec.yaml` in full; `git diff 9dc235c..34fd428 --stat`
(stat only: no test file in the range); issues #171, #170, #169, #133 read
fresh with `gh issue view`. No file under `.claude/agents/`, no `design.md`,
and no other findings file was read. The only findings content touched was
`git grep -l` file-name output over `findings/`, used in the measurement in the
third box.

Round 1's two boxes are verified as fixed. `proposal.md:708-730` ("A standing
test for the git commands the role files name") names the Step 1 pathspec, the
`openspec/specs/` check, `--ff-only`, `--remerge-diff`, `--cherry-mark`, the
`NO SPEC:` command and the pre-tick grep. `tasks.md:11-17`'s struck tester row
no longer says there is no executable behaviour, and points to that entry.

- [x] **`spec-writer`** — `proposal.md:779-782` versus `proposal.md:454-470`:
      `dev-writer.md`'s "The runner cherry-picks your commits onto its own
      local `piece/<name>` afterwards" is kept, while this range corrects
      `README.md`'s four passages because they state the same route.
      **Scenario:** at `9dc235c` the fast-forward covered only "the
      `dev-writer`'s first pass" (`9dc235c` `proposal.md:334`, and `:575`
      "bringing its first pass on by fast-forward"), so the sentence was still
      true of every findings pass. This range widens the fast-forward to
      "every `dev-writer` pass, not only the first" (`:372-379`), so the
      sentence is now false on every pass. In the same range, `:454-470`
      corrects four `README.md` passages on exactly that ground: each "say[s]
      an agent's commits are cherry-picked onto the piece, which is false for
      every `dev-writer` pass under the fast-forward rule ... left as they are,
      they contradict `RUNNER.md` in this piece". `dev-writer.md`'s sentence
      meets that test more directly than any of the four, because it is the
      `dev-writer`'s own file describing its own passes. The reason given for
      keeping it, that it "changes nothing the `dev-writer` does", is equally
      true of the `README.md` passages, so the contract does not state a
      criterion that separates the two cases. After the merge, `RUNNER.md`
      says `git merge --ff-only` and `dev-writer.md` says cherry-pick, for the
      same commits. The practical risk is low. A `dev-writer` that believes
      its pushed commits get cherry-picked has no reason to keep their SHAs,
      and one that amends or rebases after pushing gets a refused `--ff-only`,
      which fails closed. **Needed:** either apply the `README.md` rule to this
      sentence, which is inside the authorisation by the same reasoning as
      `:598-608`, or state the criterion that keeps it. **Measured:**
      `git grep -n -F "first pass" 9dc235c -- <proposal>` returns `:334` and
      `:575` as quoted above; at `34fd428` `:779` reads "bringing its passes
      on by fast-forward". Severity: low. It is a contradiction between two
      shipped role files, and the contract sets out to prevent exactly that.

      **Fixed** (`spec-writer`, this commit), by applying the `README.md` rule.
      The reason given for keeping the sentence did not separate the cases,
      as the finding says. The proposal now states one criterion for every
      file this piece edits: a sentence saying the `dev-writer`'s commits, or
      every agent's, reach the piece by cherry-pick has its route words
      changed; one true of a cherry-pick whatever the route, or about an agent
      still cherry-picked, stays. Under it `dev-writer.md` has four such
      sentences, not one, all now contracted: "The runner cherry-picks it
      onto `piece/<name>` once you hand back" ("Where your commits go"), and
      under "The PR is yours" "you do not need the runner's cherry-pick to get
      there", "The runner cherry-picks your commits onto its own local
      `piece/<name>` afterwards" and "A local cherry-pick is the runner's."
      Kept, with the reason: "is not something the runner can cherry-pick",
      about commits made from the piece branch when isolation failed, which
      holds of either route. The "Out of scope" and Impact entries, and the
      opening paragraph's account of what is corrected, say the same. The edit
      itself is the dev-writer's.

- [x] **`spec-writer`** — `proposal.md:708-730` (the standing-test entry)
      does not name the mutating reviewer's save/restore/re-apply commands,
      which this range adds (`:555-560`), and they fail open.
      **Scenario:** `RUNNER.md` is to carry `mkdir -p tmp`,
      `git diff --binary --output=tmp/uncommitted.patch HEAD`,
      `git restore --source=HEAD --staged --worktree -- .` and
      `git apply tmp/uncommitted.patch`. The contract states what they do:
      "The mutated state survives, in the tree and in the patch file", and
      "the changes come back unstaged whether or not they were staged
      before". None of these strings is in `proposal.md` at `9dc235c`, and
      the entry names none of them. Their failure is not like the pathspec's.
      Suppose a later edit drops the trailing `HEAD` from the `git diff`, or
      drops `--binary`. The patch then silently leaves out staged changes, or
      binary ones, and step 2's `restore --staged --worktree` discards those
      changes. The reviewer's mutated state, which the contract says is
      evidence only the runner may discard (`:529-533`), is destroyed with
      every command exiting 0. By the entry's own ranking ("fails open ... is
      the first a test should cover") this belongs beside the
      `openspec/specs/` check, not outside the list. (`--diff-filter=U`,
      `git merge --abort` and `git cherry-pick --abort` are also unnamed, but
      they predate this range, and a mistype there stops the agent.)
      **Measured:** `git grep -n -F -e "uncommitted.patch" -e "git restore"
      -e "git apply" 9dc235c -- <proposal>` returns nothing. At `34fd428` it
      returns `:556`, `:558` and `:559`. Staging this findings file before
      committing it: `git diff --stat` printed nothing, and
      `git diff --stat HEAD` listed the file. So the form without `HEAD`
      leaves out exactly the staged changes that step 2 then discards.
      Severity: moderate.

      **Fixed** (`spec-writer`, this commit), with one part corrected by
      measurement. The entry now lists the four commands by file and section,
      and the save with `HEAD` dropped is one of the three fail-open cases a
      test covers first, beside the `openspec/specs/` check. Measured in a
      scratch repository under `./tmp/` (git 2.55.0, since deleted): one
      unstaged and one staged edit, save without `HEAD`, restore, apply; every
      command exited 0 and only the unstaged edit came back. Dropping
      `--binary` does not fail silently, though: the patch holds `Binary files
      a/i.png and b/i.png differ`, and `git apply` refuses it (`error: cannot
      apply binary patch to 'i.png' without full index line`, exit 1). So the
      reviewer reports the failure, though step 2 has already discarded the
      change. The entry files that case under fail-closed and says the loss is
      reported, not prevented.

- [x] **`spec-writer`** — `proposal.md:718-720` says "a mistyped pre-tick
      grep lists nothing, and the runner cannot tick ... Both fail closed".
      That is false for one plausible mistype, and in that case the check
      fails open.
      **Scenario:** consecutive rounds share an endpoint. Round N's range ends
      at the SHA where round N+1's starts, and the brief's heading format
      (`:125-127`) writes both SHAs into every lane's file. So if the runner
      searches for the new round's start SHA instead of the full `<a>..<b>`
      string (a truncated copy, or a hand-typed range), the previous round's
      headings match. Every lane's file is listed, and the runner ticks with
      no lane having run the new round. That is the outcome the check exists
      to prevent: "a stalled agent looks exactly like a finished one"
      (`:189-191`). The entry uses the fail-open/fail-closed split to decide
      which command a test covers first. So misclassifying this one puts a
      fail-open gate at the back of the queue, and the entry's "Both fail
      closed" reads as settled. **Needed:** correct the classification. The
      spec-writer may also want the check to match the heading or verdict-box
      form rather than the bare range; that is a design call, and this finding
      does not presume it. **Measured** on this tree at `34fd428`, where no
      round-2 lane had yet written anything:
      `git grep -l -F "9dc235c..34fd428" 34fd428 -- openspec/changes/171-workflow-rules/findings/`
      printed nothing (correct: the tick must wait), and
      `git grep -l -F "9dc235c" 34fd428 -- openspec/changes/171-workflow-rules/findings/`
      listed all six files (`architecture.md`, `correctness.md`,
      `design-review.md`, `readability.md`, `security.md`, `spec-test.md`), all
      matched by round 1's `c222c37..9dc235c` headings. Severity: moderate.

      **Fixed** (`spec-writer`, this commit), both the classification and the
      check. "Both fail closed" is gone. The entry now says a pre-tick pattern
      with a wrong character lists nothing (fail closed), and one cut down to
      the bare range matches it in prose, or to a single SHA also matches the
      previous round's records, since consecutive rounds share an endpoint
      (fail open, first in the queue beside the `openspec/specs/` check). It
      cites your measurement. The check itself now searches the heading and
      verdict-box forms carrying ``round <n> `<range>` ``, not the bare range
      (`findings/security.md`'s two boxes in this round), so the round number
      keeps consecutive rounds' records apart and only a truncation that drops
      it fails open.

Clean in this round, in prose. On internal consistency after the four
callbacks, the returns are consistent. `:412-419` gives no count to
`RUNNER.md`, and the six it routes match the list. `tasks.md` 10.x supersedes
9.8's "five" explicitly. The fast-forward routes are consistent: the `closer`,
a conflict resolver, and an agent whose commits are already on the remote ref
appear alike at `:363-381`, `:485-488` and Impact `:840-842`. Where the spec
check runs is consistent too: after the archive commit and before the push, at
`:325-333`, `:350-356`, Impact `:864-866` and `tasks.md` 10.1. The four
owner-authorised additions the header counts are each marked (`:92`, `:231`,
`:259`, `:477`). The later additions (the verdict box and heading, the
pre-tick check, returns without a count) sit under #171 or under a marked
addition. The `README.md` passages are called corrections that follow from the
fast-forward rule, both in the body and in Impact. Nothing in the range reaches
outside the four issues' scope as stated fresh today.

## Re-review round 3 `34fd428..dc1390a`

- [x] **re-review round 3 `34fd428..dc1390a`: no findings** — read `proposal.md` in full, `tasks.md`, `.openspec.yaml`, the range's stat and issues #171, #170, #169 and #133; clean

In full: `proposal.md` at HEAD. `git diff dc1390a HEAD
--stat` shows only a one-line `tasks.md` change since `dc1390a`, so HEAD's
`proposal.md` is `dc1390a`'s. The range diff itself was too long to display
and went to a file outside this worktree, which was not read. Also read:
`tasks.md` and `.openspec.yaml` in full; `git diff 34fd428..dc1390a --stat`
(stat only); issues #171, #170, #169 and #133, read fresh with `gh issue
view` (all open, no comments, bodies as in round 1). No file under
`.claude/agents/`, no `design.md`, and no other findings file was read. The
only findings content touched was `git grep -l` file-name output, used in the
measurement below.

All three round-2 boxes are fixed in the contract:

- **`dev-writer.md`'s route sentences are contracted.** `proposal.md:536-563`
  states the criterion once for every file the piece edits. It lists the four
  sentences that change and the one that stays ("is not something the runner
  can cherry-pick", with its reason), and records why the earlier reason was
  dropped. The opening paragraph (`:9-12`), `:714-715`, Out of scope
  (`:973-974`) and Impact (`:1092-1098`) all say "the stage-row clause and the
  four route sentences", and nothing else in `dev-writer.md`. The stat shows
  `dev-writer.md` changed in the range, and `tasks.md` 12.6 records the edit.
- **The standing-test entry names the patch commands and the fail-open
  cases.** `:825-829` lists `mkdir -p tmp`,
  `git diff --binary --output=tmp/uncommitted.patch HEAD`, `git restore
  --source=HEAD --staged --worktree -- .` and `git apply
  tmp/uncommitted.patch`. `:858-867` puts the save with `HEAD` dropped among
  the three fail-open cases. `:839-844` files the dropped `--binary` case
  under fail closed, with the loss reported but not prevented.
- **The pre-tick "fails closed" claim is corrected.** "Both fail closed" is
  gone. `:838-839` keeps only the wrong-character case as fail closed.
  `:850-857` classifies a pattern cut down to the bare range or to one SHA as
  fail open, and cites the round-2 measurement. The check itself now searches
  the round-numbered heading and verdict-box forms (`:203-241`).

Internal consistency after the range. The heading and verdict-box forms at
`:128-139` are fixed-string substrings of what the pre-tick command at `:209`
searches, and the backtick after the number keeps `round 1` from matching
`round 10`. The runner's line format (`:181-183`) supplies the
``round <n> `<range>` `` both forms copy (`:142`), and `tasks.md:25-27`
follows it. The re-dispatch line rule (`:186-195`) and the check's exception
for a lane re-run over the same range (`:213-215`) agree. The transition for
rounds 1 and 2 (`:242-250`) is consistent with `tasks.md:25`, which records
round 1's re-runs inside one line because the per-re-run line rule came
later. The claim at `:245-246` ("Measured at `6f17bebf`: both list all six
findings files for each round") re-ran for round 2's un-numbered forms:
`git grep -l -F -e '## Re-review `9dc235c..34fd428`' -e '**re-review
`9dc235c..34fd428`: no findings**' 6f17bebf -- openspec/changes/171-workflow-rules/findings/`
listed all six files. The count of four owner-authorised additions still
matches the markings at `:94`, `:290`, `:318` and `:564`. The #132 overlap
section's "six files under `.claude/agents/`" matches the Impact list. The
range's stat touches only `RUNNER.md`, `closer.md` and `dev-writer.md` under
`.claude/`: no `settings.json` and no hooks. Nothing in the range reaches
outside the four issues' scope as stated today.

Below medium, so in prose rather than boxed:

- `tasks.md` 4.1's Verify ("`dev-writer.md` is untouched") and 10.6's
  ("shows that clause and nothing else") are false after 12.6. Section 12
  names only 10.3 and 10.4 as superseded. These are historical task records,
  and 12.6 states the current edit, so nothing acts on the stale lines. Low.
- The opening paragraph (`:9-12`) describes the `README.md` passages as
  naming the cherry-pick as the route "for the `dev-writer`'s commits". The
  body's criterion (`:537-540`) and `:513-516` say "the `dev-writer`'s
  commits, or every agent's", and the `README.md` passages are about any
  agent's commits. The body is the operative text. Low.

## Re-review round 5 `dc1390a..d1c8726`

- [x] **`spec-writer`** — `proposal.md:844-847` (standing-test entry, fail
      closed) versus `proposal.md:199-203` (the line rule, added in this range)
      — the entry says "A pre-tick pattern with a wrong character lists
      nothing, and the runner cannot tick". That is false for one character,
      the round number, whenever the wrong number is an earlier run's over the
      same range. In that case the check fails open.
      **Scenario:** the line rule now gives a same-range re-run its own line
      whether it is a fresh dispatch or a `SendMessage` continuation, so two
      numbers over one range is a routine case, not a rare one. This piece
      has one already: rounds 3 and 4 share `34fd428..dc1390a`
      (`tasks.md:27-28`). The runner types or copies the number by hand. If it
      runs the check for the re-run with the earlier line's number, the
      rejected run's committed record matches, the lane's file is listed, and
      the runner ticks although the re-run has written nothing. `:199-203`
      states exactly this mechanism ("the rejected run's record is already
      committed under the old number, so the check would pass on it before the
      continuation had done anything"). But the standing-test entry files every
      wrong-character pattern under fail closed, and lists only the bare-range
      and single-SHA truncations under fail open (`:858-865`). The entry uses
      that split to decide which commands a test covers first, so this case
      sits in the wrong queue. It is the same misclassification as round 2's
      "Both fail closed" box above, and the fix there ("only a truncation that
      drops it fails open") left this case out.
      **Needed:** move the stale-number case to the fail-open list, and narrow
      the fail-closed sentence to a wrong character that no earlier record
      over the range carries. Whether the check itself should guard against
      it, for example by having the runner copy the number from the new line
      rather than type it, is a design call this finding leaves open.
      **Measured**, on my own findings file only (it carries round 3's verdict
      box for `34fd428..dc1390a`; round 4 re-ran another lane over that range):
      `git grep -l -F -e '## Re-review round 3 `34fd428..dc1390a`' -e '**re-review round 3 `34fd428..dc1390a`: no findings**' -- openspec/changes/171-workflow-rules/findings/spec-test.md`
      lists `spec-test.md`. The same command with `round 4` lists nothing.
      So if spec-test had been the lane re-run in round 4, a check typed with
      round 3's number would have passed before the re-run wrote anything,
      and the correct one would have held the tick. Severity: medium. It is a
      fail-open gate classified as fail closed, in the entry written to be
      lifted into the follow-up issue.
      **Outcome (`spec-writer`): accepted, fixed in `proposal.md`'s
      standing-test entry.** Re-measured at `80c1bcc8`: the round 3 forms over
      `34fd428..dc1390a` list five files, every lane but `readability.md`,
      and the round 4 forms list `readability.md` alone. One refinement to the
      box: the stale number fails open only when the earlier run over the
      range left a record for that lane. This piece's own round 4 would have
      failed closed, because the round 3 readability run wrote nothing. A lane
      whose rejected run did write, as round 1's Sonnet security run did,
      fails open. That is the case the line rule's number exists for, so it
      belongs on the fail-open list. Changes: the fail-closed sentence now
      reads "a wrong character that no committed record carries". The
      fail-open list gains the stale-number case, with the measurement and
      the refinement, and its count goes from "Three" to "Four". The design
      call the box leaves open (whether the runner should copy the number
      from the new line rather than type it) is not taken here: it is
      `design.md`'s, and the entry is a follow-up. `design.md`'s matching
      Risks entry ("The git commands the role files name have no standing
      test", fail closed/fail open lists and "the three fail-open cases
      first") carries the same misclassification. That is listed for the
      `dev-writer`, not edited here.

Read: the `proposal.md` range diff in full, and `proposal.md` at HEAD in full
(`d1c8726`'s proposal is HEAD's; `07f0a5f` changes only `tasks.md`).
`tasks.md` and `.openspec.yaml` in full. `git diff dc1390a..d1c8726 --stat`,
stat only. Issues #171, #170, #169 and #133, read fresh with `gh issue view`:
all open, no comments, bodies unchanged. No file under `.claude/agents/`, no
`design.md`, and no other findings file was read. The measurement above
searched my own file only.

**Consistency after `47f6008`.** The line rule (`:186-203`), the pre-tick
exception (`:220-223`, "a lane that a later line ran again") and Impact
(`:1074-1076`, "fresh or continued") agree. The one place the pre-tick
paragraph continues a reviewer, a lane whose file is not listed (`:226-227`),
is the "finishing a round it has not yet recorded" exception, so it
correctly gets no line, and the fresh dispatch beside it does. The other
continuations the proposal names fall inside the exception too: committing
uncommitted output (`:601-609`, "committing") and rebasing after a
conflicting cherry-pick (`:616-623`, "rebasing"). `tasks.md` 13.x states it
supersedes 12.2's "a `SendMessage` continuation none", and 13.2 replaces
"dispatched" with "ran" as `:220-222` and `:894` now read. The closer-side
follow-up's gap statement (`:893-894`) was reworded the same way. No
requirement moved between files in this range.

Below medium, so in prose rather than boxed:

- **The closer-side follow-up omits the re-run exception** (`:887-901`). It
  quotes the runner's rule as "not tick while the findings file of any lane
  the round ran is missing", with no "except a lane a later line ran again",
  and has the `closer` "match each round line ... against the findings
  files". A check filed from that text as it stands would stop on this
  piece's own history: round 3's readability run wrote nothing and was
  re-run as round 4 (`tasks.md:28`). It fails closed, since the `closer`
  stops and reports, and the entry is a follow-up, not a rule. But it is
  written "to be lifted into an issue as it stands", so the exception
  belongs in its "For the issue to settle" or in its gap statement. Low.
- **The exception's examples mix two cases** (`:191-194`). "Finishing a round
  it has not yet recorded, such as after a stall, committing, or rebasing"
  puts rebasing under "not yet recorded", but a rebase after a conflicting
  cherry-pick happens because the record is already committed. Both cases
  meet the operative test, "adds no review to a record already committed",
  so the rule's outcome is right. Only the "such as" grouping reads wrong.
  Low.

**Standing-test coverage of the rule's checkable claims.** The one checkable
claim new in this range is `:199-203`'s: a continued re-run sharing the old
number would pass the check. That is a claim about the pre-tick command,
which the entry names (`:830-832`), but it is about a failure mode the entry
misclassifies (the box above). The rest of the line rule is judgement the
runner exercises, not a command's output.

## Re-review round 6 `d1c8726..6d43cda`

- [x] **re-review round 6 `d1c8726..6d43cda`: no findings** — read `proposal.md`'s range diff, its standing-test entry (`:823-896`) and the line rule and pre-tick check (`:120-259`) at HEAD, `tasks.md`, `.openspec.yaml`, the range stat, issues #171, #170, #169 and #133; clean

Read: `git diff d1c8726..6d43cda -- openspec/changes/171-workflow-rules/proposal.md`
in full, which is `e2f2729`'s edit only (`git show --stat e2f2729`: `proposal.md`
and my own findings file). `proposal.md` at HEAD `:120-259` and `:800-930`.
`tasks.md` and `.openspec.yaml` in full. `git diff d1c8726..6d43cda --stat`,
stat only. Issues #171, #170, #169 and #133 read fresh with `gh issue view`:
all open, no comments; #171's and #169's bodies re-read and unchanged in
substance. No file under `.claude/agents/`, no `design.md`, and no other
findings file was read.

**Round 5's box is fixed.** Both halves of its "Needed" landed:

- The fail-closed sentence (`:845-847`) now reads "a pre-tick pattern with a
  wrong character that no committed record carries lists nothing", which
  excludes the stale-number case by its own qualifier.
- The fail-open list gains it (`:866-877`), and the count moves from "Three"
  to "Four". There are four bullets: the `openspec/specs/` path, the
  bare-range and single-SHA truncation, the stale number, and the save step
  with `HEAD` dropped. `git grep -n -i -F 'three fail'` over `proposal.md`
  and `tasks.md` finds no stale count in `proposal.md`; the one hit is
  `tasks.md` 14.1 quoting the old `design.md` wording it replaces.

The spec-writer's refinement is right and narrower than my box: the stale
number fails open only when the earlier run over the range left a record for
that lane. The bullet says so ("whenever that run left one for the lane"), and
closes with this piece's counter-example (round 3's readability run wrote
nothing, so round 4 under round 3's number fails closed). Re-measured on my
own file only, at `80c1bcc8`: the round 3 forms over `34fd428..dc1390a` list
`spec-test.md`, and the round 4 forms list nothing. That agrees with the
bullet's "the round 3 forms list every lane's file but `readability.md`, and
the round 4 forms list `readability.md` alone". I did not re-run it over the
other five files.

**Consistency with the rest of `proposal.md`.** The bullet says the same thing
as the line rule's closing sentence (`:200-203`: "the rejected run's record is
already committed under the old number, so the check would pass on it before
the continuation had done anything"), and its "as with round 1's Sonnet
security box" matches `:196-199` and `tasks.md:25`. "Every command exits 0"
holds for this case: `git grep -l` exits 0 when it lists a file. The bullet's
citation points at my round-5 box, whose Outcome paragraph holds the
`80c1bcc8` measurement. `tasks.md` 14.1 carries the matching `design.md`
Risk edit to the `dev-writer`, which is outside what I may read. Nothing in
the range touches `.claude/`, and nothing reaches outside the four issues.

Below medium, so in prose rather than boxed:

- **A wholly stale line also fails open and is not enumerated.** A pre-tick
  check for round N+1 typed with round N's number *and* range lists every
  lane round N recorded, and the runner ticks. The fail-closed sentence's
  qualifier ("that no committed record carries") already excludes it, so it
  is not misclassified the way round 5's case was, but the fail-open list's
  "Four" reads as complete and names only the same-range case. A test
  fixture for the stale-number bullet covers it by the same mechanism (a
  pattern equal to a committed record's form). Low.
- **`:247-249`'s "What it still cannot see" names one residual.** The
  stale-number residual is stated by the line rule (`:200-203`) and the
  standing-test entry, not in that list. The operative text is consistent;
  a reader of the check's own bullets alone would not see it. Low.

## Re-review round 7 `6d43cda..d1d2165`

- [x] **re-review round 7 `6d43cda..d1d2165`: no findings** — read `proposal.md`'s range diff and `proposal.md` at HEAD in full, `tasks.md`, `.openspec.yaml`, the range stat, issues #171, #170, #169 and #133; clean

Read: `git diff 6d43cda..d1d2165 -- openspec/changes/171-workflow-rules/proposal.md`
in full, and `proposal.md` at HEAD in full. `tasks.md` and `.openspec.yaml` in
full. `git diff 6d43cda..d1d2165 --stat`, stat only. Issues #171, #170, #169
and #133 read fresh with `gh issue view --json body,comments,state`: all open,
no comments, bodies unchanged. No file under `.claude/agents/`, no `design.md`,
and no other findings file was read. The measurements below searched my own
file only.

**The split is consistent across all six sites.** The copy rule now appears
in the pre-tick bullet (`:214-216`, "copied from that round's own line under
the re-review row, not typed"), in its own sub-bullet (`:247-260`), in the
rounds 1–2 bullet (`:278-280`, whose lines each name their own check, as
`tasks.md:25-26` do), in the standing-test entry's stale-number paragraph
(`:917-918`), in the `closer`-side follow-up (`:936-941`) and in Impact
(`:1121-1122`). They all say the same thing. The standing-test entry's
fail-open list is back to "Three", and it has three bullets: the
`openspec/specs/` path, the bare-range or single-SHA truncation, and the save
step with `HEAD` dropped. The fail-closed sentence keeps "that no committed
record carries", which leaves out the stale number, and the new paragraph at
`:906-920` takes that case with its reason: the number is the runner's input
and not the command's, so no fixture test of the role file's command can see
it. Nothing contradicts that paragraph. The residual it leaves, forms copied
from the wrong line, is named in the same terms in three places: "What it
still cannot see" (`:266-275`), `:918-920` and the follow-up's gap statement
(`:940-941`). All three send it to the `closer`-side follow-up.
`git grep -n -i -e 'four fail' -e 'Four,' -e 'three fail'` finds no stale
count in `proposal.md`. The only hits are `tasks.md` 14.1 and 15.3, which are
history and verify lines. `tasks.md` 15.1–15.4 match the proposal's wording.

**Round 6's two prose items are answered.** "What it still cannot see" now
names the wrong-line residual, the previous round's line included, which
covers the wholly stale line I noted.

**Measured** at `c3d697e9`, on `findings/spec-test.md` only: the round 3 and
round 5 forms each list it, and the round 4 forms list nothing. That agrees
with `:255-258` and `:269-271` as far as this one file can show. I did not
re-run the checks over the other five files.

Below medium, so in prose rather than boxed:

- **"round 5's start" (`:271`) reads two ways.** It means the fixed start of
  round 5's line, ``round 5 `dc1390a..d1c8726` ``, which is how `:950-951`
  uses "start". It could also be read as round 5's start SHA, which is a
  different failure: the single-SHA truncation. Low.
- **The stale-number paragraph's citation (`:910-911`) covers only half its
  claim.** It cites my round 5 box for measurements "at `80c1bcc8` and again at
  `c3d697e9`". That box's Outcome holds only the `80c1bcc8` measurement. The
  `c3d697e9` one is in the proposal's own bullet at `:255`. Low.
- **Round 5's item on the `closer`-side follow-up still stands.** `:937-938`
  quotes the rule as "not tick while the findings file of any lane the round
  ran is missing" and does not include the exception for a lane a later line
  ran again. It fails closed. Low.

## Re-review round 9 `d1d2165..c4b1df5`

- [x] **re-review round 9 `d1d2165..c4b1df5`: no findings** — read `proposal.md`'s range diff and `proposal.md` at HEAD in full, `tasks.md`, `.openspec.yaml`, the range stat; clean

Read: `git diff d1d2165..c4b1df5 -- openspec/changes/171-workflow-rules/proposal.md`
in full, `proposal.md` at HEAD in full (1216 lines), `tasks.md` and
`.openspec.yaml` in full, and `git diff d1d2165..c4b1df5 --stat`, stat only.
The stat touches no file under `.claude/`. No file under `.claude/agents/`, no
`design.md` and no other findings file was read. The issues were not re-read
this round. The brief narrowed it to the contract's internal consistency, and
round 7 read all four fresh.

**The three places agree.** "What it still cannot see" (`:271-293`), the
standing-test entry's stale-number paragraph (`:936-945`) and the
`closer`-side follow-up (`:965-979`) all state the residual the same way. It
is a separate residual from copying from the wrong line. The copy rule carries
the line's number into the brief, the heading and the check, so all three
agree. Matching forms cannot see it. Only the follow-up's number criterion
can. All three route it to the `closer`-side follow-up, and all three call it
the same gap as skipping the check.

**The numbering rule it cites fixes the number.** The rule is `:181-183`,
"numbered from 1 in the order the lines are written", plus `:186-187`, "the
next number" for a re-run. Together they make "unique and consecutive" a
restatement of an existing rule, not a new one, as `:285-287` says.

**Impact is consistent.** `0fbebed` changes no role file, and the stat
confirms no `.claude/` path in the range. Impact's `RUNNER.md` entry ("round
lines numbered") needs no change. The number criterion is in "Out of scope"
only, and nothing in Impact or "What Changes" claims a `closer.md` edit for it.

**Measured** on this tree (`94f913f6`). I ran
`git grep -l -F -e '## Re-review round 3 ' -e '**re-review round 3 ' --
openspec/changes/171-workflow-rules/findings/` (trailing space, no backtick)
to avoid a quoting prompt. It listed `architecture.md`, `correctness.md`,
`design-review.md`, `security.md` and `spec-test.md`, which matches `:281-283`.
One of `spec-test.md`'s matches is my own round 5 prose quoting the round 3
form whole (`:413` of this file). That is the quoted-heading residual the
proposal already names at `:263-265`. `tasks.md:25-33` carries the round
numbers 1 to 9 once each, in order. That extends `:978-979`'s "1 to 8 at
`291499c7`" and does not contradict it.

Below medium, so in prose rather than boxed:

- **`:942-943` drops the qualifier.** The standing-test bullet says the
  check "passes on the earlier run's record" without the "whenever that run
  left one for the lane" that `:277-278` and `:252-253` carry. The round 3
  readability run shows the qualifier matters. A repeated "round 3" line for
  round 4's readability re-run would have failed closed, because round 3's
  readability run wrote nothing. Low.
- **"A repeated or stale number" is broader than the harm.** A repeated
  number over a *different* range makes forms no earlier record carries, so
  the check fails closed and only the numbering rule is broken. The operative
  sentences tie the harm to the same-range example, so this is a reading
  risk, not a contradiction. Low.
- **"In the order the lines stand" (`:970`) and "in the order the lines are
  written" (`:182-183`) coincide only if lines are appended.** Nothing states
  that lines are appended, though "under it" implies it. Low.

## Re-review round 10 `c4b1df5..842758b`

- [x] **`spec-writer`** — `proposal.md:215-218` (the number check) versus
      `proposal.md:181` (the line rule). The number check needs a round line
      to be indented by six spaces or more, but no rule requires that.
      `:218` says "The six spaces are a round line's indent under the row" as
      though it were a fact. The line rule says only "one indented line under
      it". `git grep -n -i -F "indent"` over `proposal.md` returns `:181`,
      `:218` and `:928`, and none of them fixes the width. The `-F` pattern
      has no anchor, so it matches any line holding six spaces followed by
      `round `. A line with fewer spaces, or with a tab, is not listed.
      **Scenario:** the runner writes a line by hand rather than copying the
      one above, types the number from memory, and uses a four-space indent:
      `    round 2 <same range>` for a re-run of round 2's lane. The listing
      leaves that line out, and the lines it does print carry 1, 2, …
      consecutively, so the number check passes. The line numbers it prints
      are contiguous, so nothing in the output shows that the line under
      them was skipped. The forms check copies ``round 2 `<range>` `` from
      the unlisted line. It passes on the rejected run's record, and the
      runner ticks before the re-run has written anything. That is the case
      the number check was added to catch (`:231-235`), and here it fails
      open. The case the contract measures, a line templated from the one
      above, keeps the indent and is caught. This is the other route to a
      wrong number that the contract names: typed rather than copied.
      **Measured:** in a scratch copy under `./tmp/` (since deleted), I made a
      stage row followed by `      round 1 …`, `      round 2 …` and
      `    round 2 …` (four spaces), and ran
      `git grep --no-index -n -F "      round " -- tmp/<copy>`. It printed
      lines 2 and 3, carrying 1 and 2, and did not print line 4. The
      standing-test entry (`:928-930`) files only the opposite direction,
      the command's indent being too long, which fails closed. It does not
      say what happens when the data's indent is too short.
      **Needed:** either fix the indent in the line rule (six spaces, as
      every line in `tasks.md:25-34` has), or have the number check require
      the runner to confirm that every line between the row and the next row
      appears in the listing. Severity: medium. The check fails open without
      any error, for a line format the contract does not forbid, and the fix
      is one sentence.

      **Fixed** (`spec-writer`, this commit), by both options, since each
      covers what the other cannot. The line rule now fixes the form: one
      line, indented by exactly six spaces as `RUNNER.md`'s sample lines
      are, with nothing but round lines between the row and the next row.
      And the number check lists the rows as well as the round lines, so a
      line that breaks the rule shows as a gap:
      `git grep -n -F -e "] re-review: every commit" -e "      round " -e "] findings all ticked" -- <change folder>/tasks.md`.
      The round lines' line numbers must run without a gap between the two
      rows, or the runner does not tick; it puts the line into the form and
      lists again. The rule alone would still fail open silently when it was
      broken, and the rows alone would show a gap but give no form to repair
      it to.
      Measured, on copies under `./tmp/` searched with `--no-index` (since
      deleted): your scenario plus a tab-indented line printed the rows at
      lines 2 and 7 and round lines 3 and 4, so lines 5 and 6 are a gap;
      the old command printed lines 3 and 4 and nothing to show a line was
      missed. On this tree at `5745a7de`, the new command prints the row at
      `tasks.md:24`, rounds 1 to 10 at `:25-34`, and the next row at `:35`;
      with seven spaces in the round pattern it prints the two rows alone.
      This lands together with `findings/correctness.md`'s round 10 first
      box: numbers now only need to be unique, a new line takes one more
      than the highest under the row, and a repeat is repaired upward.
      Your prose notes are answered by the same change. Renumbering no
      longer cascades, because only the repeating line changes. Order is no
      longer required. A wrapped round line leaves a gap, which fails
      closed. The follow-up's "reads only the line's start" now says it
      matches the fixed start, six spaces and `round `. And the fail-closed
      entry no longer says a mistyped word lists nothing: a pattern that
      matches no round line leaves a gap between the rows, and one too short
      lists more lines, not fewer.

      **For the `dev-writer`:** see the list under
      `findings/correctness.md`'s round 10 first box, which covers both
      boxes.

Read: `git diff c4b1df5..842758b -- openspec/changes/171-workflow-rules/proposal.md`
in full; `proposal.md` at HEAD at `:150-380`, `:880-1030` and `:1185-1209`;
`tasks.md`, `.openspec.yaml` and this file in full; and
`git diff c4b1df5..842758b --stat`, stat only. The brief narrowed this round
to internal consistency, so I did not re-read the issues. I read no file
under `.claude/agents/`, no `design.md` and no other findings file.

**The contract is consistent across the four sites `328c192` touched.** The
number check comes first, at `:211-242`, and the forms check follows at
`:243`. "What it still cannot see" (`:295-328`) splits the residuals into
three cases:

- a wrong line, which needs a second reader;
- a wrong number, which the number check sees;
- a re-run given no line of its own, which neither check sees.

The standing-test entry (`:962-982`) makes the same split, and the
`closer`-side follow-up (`:990-1015`) repeats it. Both send only "skips
either check" and "forms copied from the wrong line" to the second reader.
That is correct, because a `closer` running the same two checks would not
see a re-run given no line either. Impact (`:1198-1201`) names both checks
and their "What you read" rows.

The number check's command is in the standing-test list (`:911-912`), in the
same form as at `:215` and `:997`. Its fail-closed case is under "Fail
closed" (`:928-930`), not under fail open. No fail-open case for the command
itself is missing: a command mistyped with fewer spaces lists extra lines,
and `:229-231` has the runner filter those by line number.

**Measured:**

- The command at `94840b52` printed `tasks.md:25-33`, carrying 1 to 9 once
  each and nothing else, which matches `:236-237`.
- At HEAD it prints `:25-34`, carrying 1 to 10.
- With a seven-space indent it prints nothing, which matches `:928`.
- With a one-space pattern, `git grep -c` reports 27 matching lines, so the
  too-short direction over-lists rather than failing open.

Below medium, so in prose rather than boxed:

- **Renumbering cascades further than the reason given for it.** `:224-227`
  re-runs every lane briefed from a line whose number changes, because "a
  record written under a repeated number cannot be told from the earlier
  run's". Suppose a repeat sits in the middle of the rounds, for example
  1, 2, 2, 3, 4, which can happen because many rounds can pass under one
  unticked row (this piece has had ten). Then "in the order they stand" renumbers every
  later line. Their lanes re-run although their records were never
  ambiguous. The same is true of a repeated number over a different range.
  This fails closed and only costs time. Low.
- **Out of order is required but its consequence is not stated.** `:219-220`
  requires 1, 2, 3 in the order the lines stand, but `:220-221` says "does
  not tick" only for a repeat or a skip. Lines numbered 1, 3, 2 contain
  neither. Low.
- **Two claims are broader than the command.** "Reads only the line's start"
  (`:1019`) describes an unanchored substring match. "Whose word is
  mistyped, lists nothing" (`:929`) is false for a truncated word:
  `      roun` lists the same lines, which does no harm. Low.
- **A wrapped round line could be listed as a round.** If a round line is
  wrapped and a continuation line under the row starts with six spaces and
  `round `, it is listed as a round line and reads as a bad number. This
  fails closed. Low.

## Re-review round 11 `842758b..dd4fe18`

- [x] **re-review round 11 `842758b..dd4fe18`: no findings** — read `proposal.md`'s range diff in full, `proposal.md` at HEAD `:160-420` and `:970-1140`, `tasks.md`, `.openspec.yaml`, this file and the range stat; clean

Read: `git diff 842758b..dd4fe18 -- openspec/changes/171-workflow-rules/proposal.md`
in full; `proposal.md` at HEAD at `:160-420` and `:970-1140`, plus Impact
as the range diff shows it; `tasks.md`, `.openspec.yaml` and this file in
full; `git diff 842758b..dd4fe18 --stat`, stat only. The brief narrowed this
round to internal consistency, so I did not re-read the issues. I read no
file under `.claude/agents/`, no `design.md` and no other findings file. The
stat shows `RUNNER.md` as the only `.claude/` path in the range: no
`settings.json`, no hooks.

**Round 10's box is closed in the contract.** It offered two fixes, and both
landed. First, the line rule (`:181-187`) now fixes the form: a single line,
indented by exactly six spaces, with nothing but round lines between the row
and the next row. Second, the number check (`:231-253`) lists both rows, and
it requires the round lines' line numbers to run without a gap between them.
My scenario, a four-space `round 2` re-run line, now leaves a gap, and the
runner does not tick. The stale "the six spaces are a round line's indent"
sentence is gone. The fail-closed entry (`:1002-1008`) no longer claims that
a mistyped word lists nothing.

**Measured:**

- On this tree, the number check prints the row at `tasks.md:24`, round lines
  1 to 11 at `:25-35` with each number once, and the next row at `:36`. It
  matches no other line in the file.
- At `5745a7de`, a seven-space round pattern prints only `:24` and `:35`,
  which matches `:283-286`.

**The numbering rewrite is consistent at every site the brief named:**

- **Numbering bullet (`:188-196`) and number check (`:254-271`).** Both say
  that numbers must be unique, that a repeat is repaired upward, and that a
  number is never lowered. "Order and gaps do not matter" (`:254`) agrees
  with "unique, not consecutive" (`:194`).
- **Forms-check exception (`:310-315`).** It now reads "a line below it", with
  the reason that a repaired line keeps its place.
- **Tick conditions.**
  - The skipped round-1 line (`:215-221`) is what makes "at least one round
    line" (`:249-251`) hold on every tick.
  - The forms check skips a round marked skipped (`:301`), so a skipped line
    is never searched.
  - The row patterns start at `] `, so they match a ticked row as well as an
    unticked one. That covers an untick after a tick.
- **"What it still cannot see" (`:353-401`).** It adds two runner-breaks-a-rule
  residuals: a round line that was removed, and a number that was lowered.
  It also explains why a gap is harmless. That explanation holds, because a
  line's number only ever moves up and the line it moved from still carries
  the old number. So every number that has a record under it is still
  carried by some line, unless a line was removed or a number lowered, and
  those are exactly the two residuals.
- **Standing-test entry (`:984-987`, `:1002-1008`, `:1053-1065`).** The number
  check appears there in the same form as at `:232` and `:1080`. The entry
  names the gap case, and it sends the remove and lower cases to "What it
  still cannot see".
- **`closer`-side follow-up (`:1079-1107`).** It uses the new command and the
  new conditions, and it says the check matches "the line's fixed start".
- **Impact (`:1280-1289`).** It names the fixed indent, one more than the
  highest, the skipped round-1 line, the rows in the listing, and "repaired
  upward and never by lowering".

Below medium, so in prose rather than boxed:

- **The follow-up drops "at least one".** The gap statement (`:1081-1082`)
  gives the runner's three conditions. The "What a `closer`-side check would
  do" bullet (`:1093-1095`) gives only two: every line is a round line, and
  no number repeats. With no lines under the row, both hold vacuously. So a
  second reader built from that bullet alone would pass a tick that had no
  round line at all, which is the one case where a runner that skipped the
  number check has nothing for the forms check to run over. The bullet
  calls itself "the runner's two checks run again", so the condition is
  implied. It is a follow-up, not a rule. Low.
- **"No number is ever given twice" and "carries a number only one line has
  carried" (`:191-193`) hold only while the rule is followed.** When a repeat
  is repaired, the lane that was briefed from the repeated line may already
  have written under a number that two lines carried. The number-check
  bullet deals with that case, so this is wording. Low.
- **"Marked skipped" (`:301`) now matters in a second case, and the phrase
  still has two readings.** From round 5 on, every round line in this piece
  that ran lanes also says "Skipped: architecture" (`tasks.md:29-35`). A
  runner that reads that as the round being marked skipped would not run
  the forms check for it. The sentence was not changed in this range, and
  the natural reading is the whole round. Low.
- **A line with more than six spaces is still listed.** The pattern is an
  unanchored substring, so a seven-space round line matches, even though the
  line rule says "exactly six". It is then checked like any other round line,
  so nothing fails open. Low.
- **A round line misplaced below the `closer`'s row** is listed, and
  `:246-248` then tells the runner it "is not a round line". If the runner
  follows that, the round's lanes are never checked. This is the same
  runner-breaks-a-rule class as a removed line, and the listing shows the
  misplaced line. Low.

## Re-review round 12 `dd4fe18..1380d50`

- [x] **`spec-writer`** — `proposal.md:1041-1043` versus `:1071-1093` (the
      standing-test entry). This range adds the nothing-landed check's two
      commands to the entry's list of commands, but not to its fail-closed
      or fail-open split. The fail-open list still says "Three, and they are
      the ones a test covers first", and still has three bullets. Both new
      commands have a mistype that fails open, with every command exiting 0.
      **Scenario:** the second command carries `-- <change folder>/tasks.md`.
      The runner copies that pathspec onto the first command, or narrows it
      to `-- <change folder>`, and runs
      `git diff --name-only <review> HEAD -- <change folder>/tasks.md`. A
      pathspec limits the listing to the paths it matches, so a red-CI fix
      to `RUNNER.md` or to source is never listed. The check passes. The
      runner writes round 1 as skipped because nothing landed, over a range
      that holds unreviewed code. The forms check skips that line, and the
      number check lists one line (`:448-451`). The row is ticked, and an
      unreviewed commit reaches the `closer`. That is the same outcome as
      the `openspec/specs/` bullet that heads the fail-open list: an
      unreviewed change merges. Similarly, the second command with a
      mistyped change folder prints nothing, and "must show only boxes
      flipped" (`:238-239`) is met because nothing is shown. The entry uses
      its split to decide which command a test covers first. So as it
      stands, the check behind the one line that claims the branch holds
      nothing unreviewed goes to the back of the queue, and "Three" reads as
      complete. My round 5 box was the same defect for the stale number.
      **Measured**, over this round's own range: `git diff --name-only
      dd4fe18 1380d50` lists nine paths, including `.claude/agents/RUNNER.md`,
      `design.md` and `proposal.md`. The same command with
      `-- openspec/changes/171-workflow-rules/tasks.md` appended lists only
      `tasks.md`, which passes the claim. `git diff --stat dd4fe18 1380d50 --
      openspec/changes/171-workflow-rule/tasks.md` (folder name one letter
      short) prints nothing and reports no error.
      **Needed:** classify the two commands. Add the pathspec-narrowed first
      command, and the empty second command, to the fail-open list, and
      update its count. `design.md`'s matching Risks entry is the
      `dev-writer`'s to follow. Severity: medium. It is a fail-open gate left
      out of the triage, in the entry that is written to be lifted into the
      follow-up issue.
      **Outcome (`spec-writer`): accepted, fixed in `proposal.md`'s
      standing-test entry.** The fail-open list goes from "Three" to
      "Seven", and a sentence says the last four belong to the
      nothing-landed check and end in a skipped round 1 over unreviewed
      work, which merges. Added, each measured: the first command narrowed
      by a pathspec (on this tree, over `dd4fe18..1380d50`: nine paths
      unnarrowed, `tasks.md` alone with the `tasks.md` pathspec, eight
      without `RUNNER.md` narrowed to the change folder); the first command
      with `--no-renames` dropped, which the security lane's round 12 box
      added to the command (measured in `tmp/r12spec/`, since deleted); the
      second command with the folder one letter short, which prints
      nothing and no error; and the new `<review>` derivation run over the
      archived folder, which gives the archive commit (scratch repository).
      The derivation with a mistyped name prints nothing, so it goes on the
      fail-closed list. The command list now names the derivation and the
      `--no-renames` form. `design.md`'s matching Risks entry, including its
      "three fail-open cases" at the deferral, is the `dev-writer`'s.

Read: `git diff dd4fe18..1380d50 -- openspec/changes/171-workflow-rules/proposal.md`
in full. `proposal.md` at HEAD at `:100-480`, `:960-1250` and `:1330-1370`.
`tasks.md`, `.openspec.yaml` and this file in full. `git diff dd4fe18..1380d50
--stat`, stat only, plus the file-name listings in the box above. The brief
narrowed this round to internal consistency, so I did not re-read the issues.
I read no file under `.claude/agents/`, no `design.md` and no other findings
file. The stat shows `RUNNER.md` as the only `.claude/` path in the range: no
`settings.json` and no hooks.

**The rest of `2bf65c8` is consistent.** The range rule (`:222-234`) and step
3's definition of tracking (`:117-122`) agree. The check's allowed paths
(`:236-241`) are exactly `findings/` and `tasks.md`'s box flips. `design.md`
is named as needing review, as `:121-122` says. The fallback is a line in
the ordinary form, sized as step 3 says. The lost-line repair (`:283-300`)
agrees with the line rule. A restored line keeps the number the report
recorded. That is not a new line, so "one more than the highest" does not
apply, and a gap it leaves is harmless by `:194-196`. The nothing-landed form
is used only where no round ran and the check passes. A round the runner
cannot account for falls to the check. "What it still cannot see"
(`:448-454`), the stale-input paragraph (`:1121-1126`), the `closer`-side
follow-up (`:1157-1161`) and Impact (`:1348-1352`) all describe the same
rule. In the standing-test entry, the two new commands sit under the
`RUNNER.md` bullet, beside step 3's pre-tick commands. `<review>` is filed
as runner input, beside the stale round number, not as a command defect,
which is where it belongs. `tasks.md` 19.1-19.7 match.

Below medium, so in prose rather than boxed:

- **The second reader runs only one of the two commands.** The `closer`-side
  bullet (`:1157-1159`) and "What it still cannot see" (`:452-453`) give the
  second reader `git diff --name-only` "as the runner did". The runner also
  ran the `tasks.md` diff. `tasks.md` is archived and merges, so a commit that
  edits it beyond box flips needs review. A commit that touches only
  `tasks.md` and was misread as tracking passes a second reader built from
  that bullet. That text is a follow-up, not a rule. Low.
- **`<review>` and "the parent of the review round's first findings commit"
  (`:1125-1126`, `:225-227`) are the same commit only if the runner commits
  nothing between dispatching the review round and bringing its first
  findings commit on.** That holds on this piece. The comparison is offered
  as a second reader's check, and the gap between the two runs in the safe
  direction, since an earlier start holds more commits, not fewer. Low.
- **The stale-input sentences are appended to a bullet headed "A stale round
  number also fails open"** (`:1096`, `:1121-1126`). The words "in the same
  way" carry the link, but a reader scanning the bullet heads for fail-open
  cases will not find `<review>` there. Low.

## Re-review round 13 `1380d50..c3bda2b`

- [x] **`spec-writer`** — `proposal.md:242-246`, the premise under the
      derived `<review>`. The derivation takes the parent of the oldest
      commit that adds a findings file. It justifies that with "The reviewers'
      commits are the first to land after the review round is dispatched:
      the runner commits nothing between". That reason does not cover the
      case. By the contract's own definition, `:848-852`, bringing an agent's
      commits onto `piece/<name>` "adds no content of the runner's". So a
      runner that commits nothing can still land commits between dispatch
      and the first reviewer pick. Nothing in the proposal forbids that.
      Every `dev-writer` pass is pushed and then fast-forwarded to
      (`:707-714`), and step 3 lists a red-CI fix as post-review work
      (`:105-108`).
      **Scenario:** the runner dispatches the six reviewers from `X`. CI goes
      red on the PR, and a fixer's commit `F` is fast-forwarded onto the
      piece before any reviewer's commit is brought on. The reviewers read
      `X`, not `F`. The oldest findings add is then a child of `F`, so the
      derived `<review>` is `F`, and the range `F..HEAD` leaves `F` out. The
      runner later loses its report, which is the case this round's change
      was written for (`:264-268`, `:348-352`). The row's lines are gone, so
      "the check decides". `git diff --no-renames --name-only F HEAD` lists
      only findings and `tasks.md`, and the check passes. Round 1 is written
      as skipped because nothing landed. The forms check skips that line,
      and the number check lists one line. The row is ticked, and `F`
      merges unreviewed. The `closer`-side second reader in "Out of scope"
      (`:1261-1268`) derives the same `F` with the same command, so its
      "range starts late" comparison cannot see this either. My round 12
      prose note said the gap between these two values "runs in the safe
      direction". That held while `<review>` was the dispatch HEAD. Now that
      `<review>` is defined as the parent of the first findings commit, the
      gap runs the other way: the derived value is later than the commit
      the review round read.
      **Measured:** on this tree the derivation gives `c222c37b`, as
      `:269-272` states. This piece had no mid-round landing. The scenario
      above is reasoned from the text, not run.
      **Needed:** one of the following. Make the premise a rule: nothing is
      brought onto the runner's HEAD between dispatching the review round
      and bringing on its first reviewer commit, so a mid-round fix waits.
      Or add the mid-round landing to "What it still cannot see" and to the
      standing-test entry's runner-input paragraph (`:1221-1229`), beside a
      runner that reads the first line. Severity: medium. It fails open in
      the lost-report repair, which is the one use the derivation was added
      for, and the second reader offered for it shares the blind spot.
      **Outcome (`spec-writer`): accepted, fixed in `proposal.md` by your
      first option, the rule.** From dispatching the review round until
      every one of its reviewers' commits is on the runner's HEAD, the
      runner brings no other commit onto the piece, commits nothing, and
      dispatches no writer, so your red-CI fixer waits for the round.
      `RUNNER.md` states it once, in step 2. The derivation paragraph
      (`proposal.md` under the range rule) now rests on that rule, not on
      "the runner commits nothing", and cites this box and the matching
      security box for what goes wrong without it. The rule broken is
      recorded as a residual in "What it still cannot see" and in the
      standing-test entry's runner-input paragraph, beside removing a
      re-run's line and lowering a number, as a stated rule neither check
      nor the `closer`-side follow-up sees. With the rule kept, the derived
      commit is the dispatch HEAD, so the gap your round 12 note weighed
      does not arise in either direction.

Read: `git diff 1380d50..c3bda2b -- openspec/changes/171-workflow-rules/proposal.md`
in full. `proposal.md` at HEAD in full. `tasks.md`, `.openspec.yaml` and my
round 12 section of this file. `git diff 1380d50..c3bda2b --stat`, stat only:
`RUNNER.md` is the only `.claude/` path in the range, so no `settings.json`
and no hooks. I read nothing under `.claude/agents/`, no `design.md` and no
other findings file. I re-ran three of the contract's measurements, all of
which print file names or commit subjects only. `git log --diff-filter=A
--format="%h %p %s" -- openspec/changes/171-workflow-rules/findings/` lists
three commits, the last `f94f7b8d c222c37b`, as `:269-272` says.
`git diff --no-renames --name-only dd4fe18 1380d50 --
openspec/changes/171-workflow-rules` lists eight paths, none of them
`RUNNER.md`. `git diff dd4fe18 1380d50 --
openspec/changes/171-workflow-rule/tasks.md` prints nothing.

**Round 12's box is fixed.** The fail-open list says "Seven" and has seven
bullets. The last four (`:1166-1192`) all belong to the nothing-landed check,
as the lead-in says: the pathspec-narrowed first command, `--no-renames`
dropped, the second command with a mistyped folder, and the derivation over
the archived folder. The command list (`:1103-1108`) names the derivation and
the `--no-renames` form. The mistyped derivation is filed as fail-closed
(`:1129-1131`), which agrees with the range rule's empty-listing bullet
(`:257-259`). Two of my three round 12 lows are also resolved. The second
reader now runs both commands (`:511-512`, `:1263`), and `<review>` is no
longer compared with a remembered value.

**The rest of `a0ce38f` is consistent.** "Three rules" (`:221`) matches the
three bullets under it: the range, the derivation, and the check. The repair
bullet (`:348-352`) runs the check from the derived `<review>`. "What it
still cannot see" (`:507-516`), the runner-input paragraph (`:1221-1229`),
the `closer`-side follow-up (`:1257-1269`) and Impact (`:1457-1461`) all
describe the same derivation and the same two commands.

Below medium, so in prose rather than boxed:

- **The reason for "the last line, not the first" (`:254-256`) is not what
  happens.** It says "every later round's findings files are adds too", but
  a re-reviewer appends to its existing file, which is a modification. On
  this tree the two later adds are first-round lanes' files that landed late
  (`c43c7a71` and `2259e7ff`). The rule itself is right. Low.
- **After the archive, the check can never pass.** With `--no-renames`, the
  archive commit lists every pre-archive file as deleted and every archived
  file as added. So a lost line repaired after an archive always gets a
  sized round, even when nothing unreviewed landed. That fails closed. Low.
- **The fail-open lead-in (`:1141-1142`) says each of the four ends "over a
  range holding unreviewed work".** For the seventh case the range misses
  that work instead: it starts at the archive commit, after the work.
  `:1224-1227` words the same case correctly. Low.
- **"The range from it" in the `closer`-side follow-up (`:1263-1264`)** does
  not say whether the range ends at the line's recorded end or at the
  current HEAD. On a re-dispatched `closer`, a range running to the current
  HEAD would take in the archive commit, and the previous note applies.
  This is follow-up text, and "For the issue to settle" already names the
  archive boundary. Low.

## Re-review round 14 `c3bda2b..e5dcce4`

- [x] **re-review round 14 `c3bda2b..e5dcce4`: no findings** — read `proposal.md` in full and its range diff, `tasks.md`, `.openspec.yaml`, this file, the range stat and issue #171; clean

Read: `git diff c3bda2b..e5dcce4 -- openspec/changes/171-workflow-rules/proposal.md`
in full, and `proposal.md` at HEAD, with the whole of `:1-916` and
`:1180-1623` read in full. `tasks.md`, `.openspec.yaml` and this file in full.
`git diff c3bda2b..e5dcce4 --stat`, stat only: `RUNNER.md` is the only
`.claude/` path in the range, so no `settings.json` and no hooks. I read
issue #171 fresh (open, no comments) to judge the scope of the new step-2
rule. I read nothing under `.claude/agents/`, no `design.md` and no other
findings file. Two commit-subject listings, below, are the only other
output I read.

**Round 13's box is closed in the contract.** It took the box's first
option. "Nothing lands on the piece while the review round is out"
(`:124-135`) forbids all three routes: bringing any other commit onto
`piece/<name>`, committing, and dispatching a writer. It holds from dispatch
until every reviewer's commit is on HEAD, so a red-CI fixer waits for the
round. It also explains why "the runner commits nothing" did not cover the
case, which was the gap my box named. The derivation paragraph (`:254-262`)
now rests on that rule, and it cites my box and the security box for what
happens without it. The broken rule is recorded as a residual in two
places: "What it still cannot see" (`:591-595`) and the standing-test
entry's runner-input paragraph (`:1302-1305`). Both say the second reader
derives the same commit, so neither claims the `closer`-side follow-up
would see it. Impact (`:1543-1544`) names step 2's rule. The rule is not in
#171's text, but #171 asks for a place to record re-review rounds, and the
derived `<review>` behind the round-1 line depends on this rule. I read it
as inside the authorisation.

**The contract is consistent after `5203661`:**

- **"Four".** The number check's lead-in (`:334-344`) now says "numbers
  and ranges" and "Four things must hold". Four bullets follow: every line
  listed, at least one, no repeat, and the ranges chain.
- **The chain bullet (`:394-425`) agrees with the rules it draws on.** A
  gap is repaired by a new line below the last line with the next number,
  and an existing range is never edited. That matches "only ever added"
  and "never lowers". Passing over a same-range line matches the re-run
  line rule. The tail check lets round lines through in the `tasks.md`
  diff, which matches step 3's list of tracking. The skipped line for a
  clean merge of `main` or the archive commit matches step 3's no-review
  list.
- **Measurements re-checked.** The chain measurement (`:427-447`) holds
  against `tasks.md:25-37`. Rounds 4 and 8 repeat 3 and 7, and the chain
  runs from `c222c37` to `c3bda2b`. `git log --oneline --reverse
  c3bda2b..ab53b41c` lists six commits: the round 13 record and five
  re-reviewers' commits, as `:430-432` says. With round 14's line, the
  chain now reaches `e5dcce4`.
- **"What it still cannot see" (`:562-598`).** A removed line is now
  narrowed to one that repeated a range. The broken review-round rule and
  a missed untick are added. Each says which second reader, if any, would
  see it.
- **The standing-test entry (`:1226-1228`) is correct.** The three
  fail-open cases that mistype the nothing-landed check's two commands
  also blind the chain's tail check. The fourth case, the archived-folder
  derivation, is not among the three, which is right. For the chain, that
  derivation starts at a commit where no line starts, so it fails closed.
- **The follow-up and Impact agree.** The `closer`-side gap statement
  (`:1333-1334`) and "would do" (`:1347-1351`) carry the chain to the
  `closer`'s own HEAD. Impact (`:1560-1562`) carries the fourth condition.

**The `1f62afd4` wording (the dev-writer's note).** The note is right.
`git log --oneline --reverse 9dc235c..1f62afd4` lists nine commits before
`1f62afd4`. All of them are tracking: the round 1 record, its correction,
and seven re-reviewers' commits (`456e1cfa` to `59619032`). So `1f62afd4`
is the first commit after round 1 that needs review. It is not "the first
commit that landed after round 1" (`:436-437`). None of the conclusions
depends on the phrase:

- the chain still stops at `9dc235c`;
- `git diff 9dc235c 1f62afd4` still holds the `spec-writer`'s commit;
- the round 14 repair still chains.

Low, so in prose. The fix is one word: "the first commit that needed review
after round 1".

Below medium, so in prose rather than boxed:

- **"Adds no command beyond the two the range rule names" (`:395-396`)
  undercounts.** The chain starts at a derived `<review>`, so it also runs
  the `git log` derivation, which is a third command. The two `git diff`
  commands belong to the check rule, not to the range rule proper. The tail
  bullet's "the range rule's two commands" has the same wording issue. Low.
- **Removing the original of a same-range pair is a different failure from
  the one described.** Suppose round 2 ran correctness and security over
  `B..C`, and round 3 re-ran security over `B..C`. If line 2 is removed,
  line 3 is followed and the chain holds. Round 3's forms check then covers
  security alone, so round 2's correctness lane is never checked. "When it
  repeated a range" (`:563`) can be read to cover this, but the explanation
  after it describes only removing the re-run. This is the same
  runner-breaks-a-rule class, since lines are only ever added. Low.
- **The `closer`-side "would do" bullet has two stale clauses.** Its
  round-1 derive-and-compare clause (`:1352-1356`) is now covered by the
  chain it asks for in the same sentence. And "the runner reads the number
  after it" (`:1369-1370`) now also reads the range. This is follow-up
  text. Low.

## Areas checked clean

- **Issue coverage.** Every "Done when" / proposed-change bullet in #171,
  #170, #169 and #133 is addressed in `proposal.md`, including #171's
  "leaves the reviewer count and model choice to the runner's judgement, with
  the decision recorded" and #170's exact `gh pr view` field list — verified
  live: `gh pr view 174 --repo fryorcraken/dialectica --json
  mergeStateStatus,mergeable,statusCheckRollup,reviewDecision` returned valid
  data for all four fields, so none is a typo'd field name.
- **Scope.** `git diff origin/main...HEAD --stat` shows exactly the five
  `.claude/agents/*` files the proposal's Impact section names
  (`README.md`, `RUNNER.md`, `closer.md`, `spec-writer.md`, `tester.md`) plus
  the `openspec/changes/171-workflow-rules/` paperwork — no `settings.json`,
  no hooks, no `CLAUDE.md`, matching the proposal's own "Out of scope"
  section and the piece's stated authorisation boundary.
- **Owner authorisations.** The two edits the dispatch brief said were
  owner-authorised beyond the four issues' text — `tester.md`'s handling of a
  decided marker, and `closer.md` Step 3 not re-archiving on a re-dispatch —
  are each explicitly tagged "(owner-authorised)" in `proposal.md` at the
  point they're introduced, rather than presented as reads on the issues.
- **#133's deviation from the issue's literal fix.** The issue's suggested
  `git checkout 51ab7f8^ -- .claude/agents/README.md` is not run; the
  proposal explains why (the file has changed since, e.g. PR #106) and
  instead quotes the exact paragraph text from the issue. The quoted text in
  `proposal.md:160-162` matches the issue's quoted paragraph verbatim.
- **Struck spec row.** Matches `.openspec.yaml`'s `skip_specs: true` +
  `schema: spec-driven`, and `openspec/config.yaml` confirms `spec-driven` is
  the project's built-in schema name, so the declaration is well-formed.
- **`#169`'s declined optional suggestion** (keeping the spec-writer's tree
  for a callback) is addressed, not silently dropped — the proposal states it
  is declined and defers the reasoning to `design.md`, which is outside this
  review's scope.

## Re-review round 15 `e5dcce4..bad7c88`

- [x] **`spec-writer`** — `proposal.md:419-425` (and `design.md:977-981`,
      "never whether a commit is read") — the gap repair may be "skipped with
      its reason", and nothing checks that reason, so on the off-by-one the
      text names as its ordinary case the specified check passes with the
      left-out commit unreviewed. "Either line is sound" is new in this range
      and is false for the skipped form. A skipped line extends the chain like
      any other, the forms check skips a round marked skipped, and the
      nothing-landed commands are required only for a skipped round 1
      (`:362-364`). The off-by-one arises exactly when the runner believes the
      round covered the left-out commit, so "covered by round `<n>`" is the
      reason it will write; round 14's correctness box itself offered
      "skipped as covered" as a repair. `design.md`'s own "a covering claim
      nothing checks is how a gate fails open" is the argument, applied only
      to unreached lines.
      **Scenario:** a copy of this stage block with round 13 written
      ``round 13 `a0ce38f1..c3bda2b` `` (one commit late: `a0ce38f1` is the
      `spec-writer` commit, and a two-dot range leaves it out). The chain
      reaches `1380d50` and stops, and the tail fails from there, which is
      right. The runner takes the text's second repair,
      ``round 16 `1380d50..a0ce38f1` skipped: round 13's start was one commit
      late; its lanes read this range``. The listing passes the first three
      conditions (lines 3-18, numbered 1 to 16 once each). The chain now
      reaches `a0ce38f1`, then `c3bda2b`, `e5dcce4` and `bad7c88`. The tail
      from `bad7c88` passes and the forms check skips round 16, so the row
      ticks. But round 13's lanes read `git diff a0ce38f1 c3bda2b`, which does
      not contain `a0ce38f1`'s `proposal.md` change. That change merges with
      no round having read it. **Severity:** medium. It is a fail-open path
      that the text offers as a sound option, not one that needs a rule
      broken. **Needs:** allow a gap line to be skipped only when the
      nothing-landed check's two commands pass over its range. They are
      already in "What you read", so no new command is needed. Otherwise the
      line is sized. Alternatively, drop "or skipped with its reason" from the
      gap repair. A gap holding only tracking commits then gets a sized
      no-lane line, as a nothing-landed round 1 does.
      **Measured:** `tmp/r15-spec-test/d2-skip-repair.md` (since deleted)
      through the number check's listing with `git grep --no-index`: sixteen
      round lines, contiguous and unique. `git diff --no-renames --name-only
      1380d50 a0ce38f1` lists `proposal.md` beside the findings files and
      `tasks.md`, so the nothing-landed check over the repair's range fails
      and would have refused the skip. `git diff --no-renames --name-only
      bad7c88 HEAD` lists `tasks.md` only, and its diff is the round 15 line,
      so the tail passes from `bad7c88`.
      **Outcome (`spec-writer`): accepted, fixed in `proposal.md` by your
      second option, extended to both repairs.** "Or skipped with its
      reason" is gone. Every line written because the tail check failed,
      the round to HEAD and the round ending at an unreached line's start
      alike, is sized as step 3 says, runs at least one lane, and is never
      marked skipped. The round to HEAD skipped as "covered by round `<n>`"
      had the same hole. In your scenario, ``round 16
      `1380d50..a0ce38f1` `` now has to run a lane, and that lane reads
      `a0ce38f1`'s `proposal.md` change. "Either line is sound" and
      `design.md`'s "never whether a commit is read" now hold as written,
      since both repairs are read. The soundness argument now says a skipped
      line's coverage rests on its reason, which nothing checks. A gap
      holding only tracking gets a sized line with a lane, not a no-lane
      line. A round sized to no lanes is a skip in all but name, and it
      passes the forms check vacuously, so "at least one lane" is part of
      the rule. The gate on the nothing-landed commands was rejected
      because it adds a branch to save one lane. The only skipped lines
      left after a failed tail check are a clean merge of `main` and an
      archive commit that changed nothing under `openspec/specs/`, each on a
      line of its own. Recorded in `design.md` as "Adopted: neither repair
      is ever skipped". The `dev-writer` brings `RUNNER.md:740-751` into
      line.

**Q1: can the specified check fail?** I ran the number check's listing
(`git grep --no-index -n -F` with its three patterns) over four copies in
`./tmp/r15-spec-test/`, since deleted. I derived `<review>` with the
contract's `git log --diff-filter=A` (last line `f94f7b8d c222c37b`), walked
the chain by hand, and ran the tail's two commands against this repository.
All four copies pass the first three conditions.

- **(a) Correct block** (rounds 1-15 as at HEAD): the chain reaches every end
  through `bad7c88`. From `bad7c88` the tail is `tasks.md` alone, and its diff
  is the round 15 line, so the check **passes**.
- **(b) No line for the last commits** (round 15 dropped): the chain reaches
  no further than `e5dcce4`. From there the tail lists `.claude/agents/RUNNER.md`,
  `design.md` and `proposal.md`. Every other reached end is earlier and fails
  too. The check **fails**, which is right.
- **(c) Templated from the line before, only the end changed**
  (``round 15 `c3bda2b..bad7c88` ``): rounds 14 and 15 both start at
  `c3bda2b`, so both `e5dcce4` and `bad7c88` are reached, and the tail from
  `bad7c88` passes. That is right, because round 15's range contains
  everything. Round 14's dead end is gone.
- **(d) Off-by-one mid-chain** (round 13 as `a0ce38f1..c3bda2b`): the chain
  stops at `1380d50`. The tail from there lists `proposal.md`, `design.md` and
  `RUNNER.md`, so the check **fails**, which is right. A sized repair (a
  round `1380d50..HEAD`, or `1380d50..a0ce38f1` sized) then chains to
  `bad7c88` and passes. The skipped repair also passes, which is the box
  above.

Below the box threshold, in prose only:

- **The verdict on a correct block depends on which reached end the runner
  picks.** On (a), the tail from `e5dcce4` (also reached) lists `RUNNER.md`,
  `design.md` and `proposal.md`, and "If they fail … the runner writes the
  next line" (`:415-416`) then owes a round `e5dcce4..HEAD` that duplicates
  round 15. `design.md:977-978` accepts this ("changes only how much is read
  again"), and the condition's wording reads as "some reached end". But the
  proposal never says a fail from one end is overridden by a pass from
  another, or which end to try first. One clause would close it: "it passes
  if the commands pass from any end the chain reaches; try the latest line's
  end first". It fails closed, so the cost is a wasted round. Low.

**Q2: is the measured paragraph true?** Yes, for everything I re-ran:

- `git diff --no-renames --name-only e5dcce4 bb6ed4e2` lists the five
  round-14 findings files and `tasks.md`, and the `tasks.md` diff is the round
  14 line added (`:450-454`).
- `git diff --no-renames --name-only 9dc235c 1f62afd4` lists `proposal.md`
  (`:444-445`).
- `git diff --no-renames --name-only c3bda2b ab53b41c` lists the five findings
  files and `tasks.md`.
- `git log` shows six commits from `c3bda2b` to `ab53b41c` (`:433-435`).
- Round 14 is at `tasks.md:38` at HEAD, so "fourteen lines at
  `tasks.md:25-38`" holds for `bb6ed4e2`.
- The round-14 templated copy's claim, that the tail passes from `e5dcce4`,
  follows from the first result.

Read: the `e5dcce4 bad7c88` diff of `proposal.md` and `design.md`,
`proposal.md:150-470` and `:540-614`, `tasks.md:1-60`, round 14's sections of
`findings/correctness.md` and this file, and issue #171 (open, no comments,
not updated since before round 14). Nothing under `.claude/agents/`, and no
`RUNNER.md` hunk.

## Re-review round 16 `bad7c88..b5ceed2`

- [x] **`spec-writer`** — `proposal.md:423-437` with `:498-500`, `:606-622`
      and `:1376-1380` (and `design.md:1018-1031`) — "neither is ever marked
      skipped" is a runner rule nothing checks, and unlike every other
      unchecked runner rule in this contract it is neither disclosed nor in
      the second reader's scope. The only thing that separates a forbidden
      skipped repair from a permitted skipped line is the free-prose reason
      on the line. The number check reads only the fixed start, the chain
      treats a skipped line like any other, the tail passes past it, and the
      forms check skips "a round … marked skipped" (`:500`). The residual
      list (`:606-615`) names only "a nothing-landed line written without its
      own check", which is round 1. The `closer`-side check in "Out of scope"
      (`:1376-1380`) re-runs the check only for "a round 1 marked skipped".
      Everywhere else a rule the runner can break unseen is listed, as with
      the lowered number, the review-round rule and the forgotten untick,
      and routed to that second reader. This one is not, so the follow-up
      built from "Out of scope" would ship without it. `design.md:975` now
      says "coverage by a skipped line rests on its reason, which nothing
      checks". `:1018-1022` then keeps the merge and archive skip as "not the
      same hole", because its reason is "a fact about that one commit". That
      holds only for a line whose range is that one commit, and nothing
      checks the range either. So a repair skipped under the permitted label
      ("skipped: archive commit, nothing under `openspec/specs/`") passes
      every check exactly as the forbidden form does.
      **Scenario:** my round-15 scenario, re-run under the new text on
      `tmp/r16-spec-test/skipped.md`. That is the stage block with round 13
      written ``round 13 `a0ce38f1..c3bda2b` ``, rounds 14-16 as at HEAD, and
      ``round 17 `1380d50..a0ce38f1` skipped: round 13's start was one commit
      late; its lanes read this range``. The runner breaks the bolded rule,
      the row still ticks, and `a0ce38f1`'s `proposal.md` change merges
      unread. **Severity:** medium. This is not a new mechanism gap. The
      contract's own convention (record the residual, route it to the second
      reader) was not applied to the rule this round added. **Needs:**
      record-and-route, not a new runner check. (1) Add one sentence to the
      "Nor does either check see" list: a line after round 1 marked skipped,
      whatever its reason, is skipped by the forms check and extends the
      chain, so a repair written as skipped, or any range labelled a clean
      merge or archive, passes both checks. (2) Widen the `closer`-side check
      at `:1376`: for every skipped line other than round 1, `git log
      --format="%h %p %s" <range>` lists exactly one commit, and that commit
      is a merge of `main` or an archive commit whose spec diff lists nothing.
      Optionally, add one clause to `:433-437` saying a skipped line's range
      is that one commit, since "a line of its own" says so only by
      implication.
      **Measured:** the number-check listing
      (`git grep --no-index -n -F -e "] re-review: every commit" -e "      round " -e "] findings all ticked"`)
      on `tmp/r16-spec-test/skipped.md` prints the two rows at 4 and 22 and
      round lines 5-21, which is contiguous. It prints 1 to 17 once each,
      with 3/4 and 7/8 sharing ranges, as on this tree. The chain by hand
      from the derived `<review>` `c222c37b` (the derivation's last line is
      `f94f7b8d c222c37b`): rounds 1-12 reach `1380d50`, round 17 starts there
      and reaches `a0ce38f1`, round 13 then reaches `c3bda2b`, and 14, 15 and
      16 reach `e5dcce4`, `bad7c88` and `b5ceed2`. `git diff --no-renames
      --name-only b5ceed2 HEAD` lists `tasks.md` alone, and that diff is the
      round 16 line, so the tail passes. The forms check is not run for round
      17, because the line is marked skipped. So all four number-check
      conditions and the forms check pass. `git diff --no-renames
      --name-only 1380d50 a0ce38f1` lists `proposal.md`. `git log
      --format="%h %p %s" 1380d50..a0ce38f1` lists seven commits, none of
      them a merge. The one-commit check in (2) would therefore refuse this
      line.
      **Fixed** (`spec-writer`), by record-and-route with no new in-piece
      check. (1) `proposal.md:620-631` adds to the "Nor does either check
      see" list a skipped line after round 1 that breaks the tail check's
      rules: a repair marked skipped, whatever its reason, or a merge or
      archive line whose range ends past that commit; it names why both
      checks pass it and routes it to the `closer`-side check. (2) That
      follow-up in "Out of scope" (`proposal.md:1401-1408`) now covers every
      skipped line other than round 1: its range must end at a clean merge
      of `main` or an archive commit that changed nothing under
      `openspec/specs/`, and the nothing-landed check's two commands must
      pass over the range from the line's start to that commit's first
      parent. That form rather than the box's `git log ... <range>` lists
      one commit: a two-dot range ending at a merge lists every commit the
      merge brought from `main`, and a line starting at a reached end also
      holds the runner's tick, so a correct line would fail it; your round
      17 line still fails, since `1380d50..a0ce38f1` ends at no merge or
      archive. (3) The optional clause is taken in the security form
      (`proposal.md:433-442`): the skipped line's range ends at the commit
      its reason names. The `dev-writer` brings `RUNNER.md:752-756` and
      `design.md:1018-1031` into line.

**Q1: is the round-15 box closed?** Yes, for a runner that follows the text.
Under `:423-424` the repair has to be a sized round with at least one lane.
On `tmp/r16-spec-test/sized.md`, where round 17 `1380d50..a0ce38f1` is sized
with a spec-test lane, the listing and chain come out the same as the
skipped copy's: rows at 4 and 22, 1-17 unique, and the chain reaches
`b5ceed2`. The forms check now has to run for round 17, and on this tree
`git grep -l -F -e '## Re-review round 17 `1380d50..a0ce38f1`' -e '**re-review round 17 `1380d50..a0ce38f1`: no findings**' -- openspec/changes/171-workflow-rules/findings/`
lists nothing. The runner therefore cannot tick until a lane briefed on
`1380d50..a0ce38f1` has written a record, and that range's diff holds
`a0ce38f1`'s `proposal.md` change. "Either line is sound" and
`design.md`'s "never whether a commit is read" now hold as written. What is
left is the box above: the prohibition is unchecked and undisclosed. By the
contract's own standard, a rule broken unseen is acceptable when it is
recorded and routed, so the missing record is what makes this medium, not
the absence of a check.

**Q2: the verify commands of `tasks.md` 21.3 and section 22.**

- **21.3:** the derivation's last line is `f94f7b8d c222c37b`, as claimed.
  The number-check listing on the real `tasks.md` prints the row at 24,
  round lines 25-40 (1 to 16, unique) and the next row at 41. The chain by
  hand reaches `9dc235c`, `34fd428`, `dc1390a` (rounds 3 and 4, identical
  ranges), `d1c8726`, `6d43cda`, `d1d2165` (7 and 8, identical), `c4b1df5`,
  `842758b`, `dd4fe18`, `1380d50`, `c3bda2b`, `e5dcce4` and `bad7c88`, and
  round 16 now carries it on to `b5ceed2`. The row's "rounds 1 to 15 through
  `bad7c88`" was true at `b5ceed2`, where it was written, and HEAD only
  extends it. **Holds.**
- **22.1:** `git grep -n -F -e "skipped with its reason" -e "such as a clean merge" -- .claude/agents/`
  prints nothing, as claimed. This was the only contact with that directory,
  and it returned no lines. It pins only that the old wording is gone, not
  that the new rule is there. For a verify step that is weak, but it is
  the implementation reviewers' job to judge, so it gets no box.
- **22.2:** `git grep -n -F -e "chain's end" -e "chain broken" -- openspec/changes/171-workflow-rules/design.md`
  prints nothing, as claimed. **One miscount:** the row says "the chain's
  end" was reworded "at its five sites". The same grep at `bad7c88` prints
  five lines, but only four are "chain's end" (`:1006`, `:1009`, `:1916`,
  `:1944`). The fifth is "chain broken" (`:1016`), which the row counts
  separately. Low, prose only.

Below the box threshold, in prose only:

- **`proposal.md:1250-1252` was not carried along with `design.md:1993`.**
  `design.md`'s copy of the fail-open preamble now says the mistyped
  commands "also let it miss a commit landed after that end". The proposal
  still says "from the chain's end, so the three that mistype them also let
  it pass over a commit landed after the last round". "The chain's end" is
  defined at `:411-412`, so that half is fine. But "after the last round"
  is the pre-round-15 model, in which the tail ran from the last line, and
  "pass over" is the verb round 15 removed from the chain. Low.

Read: the `bad7c88 b5ceed2` diff of `proposal.md`, `design.md` and
`tasks.md`, `proposal.md:300-640` and `:1230-1270` and `:1360-1404`,
`design.md:1015-1034`, `tasks.md:1-48`, this file's round-15 section, and
issue #171 (open, no comments, `updatedAt` 2026-09-25, unchanged since round
15). Nothing under `.claude/agents/` was read. The 22.1 grep over it
returned no lines. No `RUNNER.md` hunk was read.

## Re-review round 17 `b5ceed2..d806700`

- [x] **`spec-writer`** — `proposal.md:1401-1404` (mirrored at
      `design.md:2134-2140` and in PR #174's `closer`-side follow-up) — the
      new `closer`-side check for a skipped line after round 1 runs "the
      nothing-landed check's two commands over the range from the line's
      start to that commit's first parent". Those two commands are defined
      at `:293-297`, and the second requires the `tasks.md` diff to "show
      only boxes flipped". Only the tail check (`:411-415`) adds the
      exception that the diff "may also show round lines under the re-review
      row", and the new text does not carry it. A skipped line after round 1
      starts at an end the chain reaches (`:437`). That round's own line is
      recorded after its end (`:409-410`), so it always lies in
      `<start>..<commit>^1`. As written, the check fails every correct
      skipped merge or archive line after round 1. That is the same defect
      the spec-writer gave for declining the one-commit form: "a correct line
      would fail it".
      **Scenario:** a clean merge of `main` lands on this piece right after
      round 16's records, i.e. with first parent `9777492a`. The runner
      writes ``round 17 `b5ceed2..<merge>` skipped: clean merge of main``,
      which is correct under `:433-442`. The `closer`-side check finds the
      range ends at a merge and passes that condition. Then over
      `b5ceed2..9777492a` the second command shows a `+      round 16 …`
      line, which is not a box flip, so the correct line is refused. Any
      follow-up lifted from this text "as it stands" (PR body) is red on
      every piece that merges `main` after round 1. **Severity:** medium.
      The criterion is unusable for the case it was written for, and the fix
      is one clause: "with the tail check's allowance for round lines under
      the row", or "run the tail check's commands".
      **Measured:** `git diff --no-renames --name-only b5ceed2 9777492a`
      lists the five findings files and `tasks.md`, so the first command
      passes. `git diff b5ceed2 9777492a -- openspec/changes/171-workflow-rules/tasks.md`
      shows one added line, round 16's round line, so the second command as
      defined at `:296-297` fails. With the round-line allowance both pass,
      and the check still refuses both forbidden forms. My round-16 repair
      `1380d50..a0ce38f1` fails the end condition:
      `git log --format="%h %p %s" -1 a0ce38f1` shows one parent and a
      spec-writer subject, so it is no merge and no archive. A repair marked
      skipped whose range does end at a real merge still lists the unreviewed
      commit's path in the first command over `<start>..<merge>^1`.
      **Outcome (spec-writer): fixed by removal, not by the one clause.**
      The criterion, and with it the "nothing-landed check's two commands
      over … first parent" procedure, is gone from `proposal.md`. Its
      three lanes found three edges in one paragraph for a check this piece
      does not build. The follow-up's bullet "Skipped lines after round 1"
      (`proposal.md:1424-1443`) now says only what the check must refuse and
      what it must accept (including the `closer`'s merge and archive
      lines), and leaves the procedure, round-line allowance included, to
      the issue ("For the issue to settle", `:1456-1463`). The `dev-writer`
      brings `design.md`'s Risk (`:2134-2145`) and the PR body's
      `closer`-side follow-up, which carry the removed text verbatim, into
      line.

**Q1: is my round-16 box closed?** The record-and-route part is closed.
`proposal.md:620-631` adds the skipped line after round 1 to the "Nor does
either check see" list in the same form as its siblings. That form is: what
the runner breaks, why the number, chain and forms checks all pass it, the
finding it came from, "Reaching this takes a runner breaking a stated rule",
and the routing to the `closer`-side check. `:1401-1408` widens that check,
and `design.md:1213-1224`, `:1257-1261` and the Risk at `:2107-2145` match it.
The optional clause was taken in the stronger security form (`:437-442`).

I re-ran round 17 `1380d50..a0ce38f1`, marked skipped, against the new text.
The in-piece checks still pass it, as round 16 measured and as the contract
now records. The routed `closer`-side check, as specified, fails it at the
end condition, because `a0ce38f1` is neither a merge nor an archive. So the
box is closed for the forbidden cases. The unticked box above is about the
permitted case, which the same text refuses too.

**Is the reason for declining my one-commit form right?** Yes, on both
halves.

- **A merge's range lists more than one commit.** I measured this in the
  scratch repo `tmp/r17-spec-test`: base, then `main1` and `main2` on
  `main`, then `round-end` and `record-round-line` on `piece`, then
  `merge --no-ff main`. `git -C tmp/r17-spec-test log --format="%h %p %s" 85b948f..HEAD`
  lists four commits: the merge, `record-round-line`, `main2` and `main1`.
  Even from the merge's first parent, `ac22faa..HEAD`, it lists three: the
  merge and both `main` commits.
- **A line starting at a reached end holds more than the one commit.** On
  this tree, `b5ceed2..9777492a` holds the round-16 record commit and five
  findings commits. The spec-writer calls this "the runner's tick", which is
  loose wording: what is always there is the round's record and its
  findings, with the tick there only if the row was ticked first. The
  conclusion holds either way. My form was wrong.

**Q2: `tasks.md` section 23 verify commands.**

- **23.1:** `git grep -n -F -e "ending at that commit" -- .claude/agents/`
  prints exactly one line, `.claude/agents/RUNNER.md:756`, which is
  consistent with "stated once". I did not open the file, so I cannot
  confirm by reading that `:756` is the tail bullet. It is the line round
  16's outcome pointed the dev-writer at (`RUNNER.md:752-756`). The command
  shows only that the phrase appears once. **Holds** as far as a grep can
  show.
- **23.2:** there is no command. Against the diff: the "Kept" bullet now says
  "bounded on both sides". It gives the start-side argument with its round-14
  citation, then the end-side untick-and-fix scenario and why the start is
  not narrowed to `<first parent>..<sha>` (`design.md:1025-1050`). "Why the
  tail check alone is sound" starts at `:960`, and its `:977-979` gives why
  the two skippable lines end at their commit and where a broken rule is
  routed. **Holds.**
- **23.3:** `git grep -n -F -e "skipped line after round 1" -- openspec/changes/171-workflow-rules/design.md`
  prints `:1213`, `:1260` and `:2112`. By heading positions from
  `git grep -n -e "^#"`, these fall under "What it still cannot see"
  (`:1142`), "What else was considered" (`:1250`) and the Risk "Only the
  runner runs the pre-tick checks" (`:2107`). **Holds.** The Risk's text
  about not requiring a one-commit range is at `:2141-2145`.
- **23.4:** PR #174's body, from `gh pr view 174 --json body`, says each
  skipped line "ends at that commit, never at HEAD or any later commit".
  The standing-test follow-up and the `closer`-side follow-up both name "a
  skipped line after round 1 that breaks the tail check's rules". The
  `design.md` summary carries the end-side argument. **Holds.** The body also
  carries the box above verbatim ("run the nothing-landed check's two
  commands over the range from the line's start to that commit's first
  parent"), so the fix has to reach it too.
- **23.5:** `openspec validate 171-workflow-rules --strict` prints "Change
  '171-workflow-rules' is valid". **Holds.**

Below the box threshold, in prose only:

- **A skipped line that starts too early is not named in the residual's
  enumeration.** `:620-623` lists "a repair marked skipped, whatever its
  reason, or a line … whose range ends past that commit". A third form
  breaks the same rule: a skipped line `E..M`, with `M` a clean merge, where
  a commit that needs review lies between the reached end `E` and `M`. Its
  end is right and nothing labels it a repair. `:433-437` forbids it only
  through "a line of its own". The general clause "breaks the tail check's
  rules" covers it, and the widened `closer`-side check refuses it, because
  the first command over `E..M^1` lists the commit's path. Low.
- **The round-1 half of the `closer`-side check has no stated end.**
  `:1395-1396` runs the two commands "over the range from it" and does not
  say to what. If that means the `closer`'s HEAD, the round-1 line itself
  and every later round line are in the `tasks.md` diff, and the box above
  recurs. This text is from an earlier round and this round did not change
  it, so it is only noted. Low.

Read: the `b5ceed2 d806700` diff of `proposal.md`, `design.md` and
`tasks.md`, `proposal.md:284-323`, `:380-459`, `:580-649` and `:1370-1419`,
this file's round-16 section, PR #174's body, and issues #171, #170, #169
and #133. All four are open with no comments. #171, #170 and #133 were last
updated 2026-09-25 and #169 on 2026-09-26, so none changed since round 16.
Nothing under `.claude/agents/` was read. The 23.1 grep over it printed one
line, and no `RUNNER.md` hunk was read. My tree holds the scratch repository
`tmp/r17-spec-test`, which is gitignored and not committed.

## Re-review round 18 `d806700..b33f0f5`

- [x] **`spec-writer`** (then `dev-writer` for `design.md:1228` and `:2159`
      and PR #174's body, which carries the clause twice) —
      `proposal.md:1428-1430`, mirrored at `:624-626` — the refuse clause
      "holds before it a commit that needs review and lies in no other
      line's range" counts a *skipped* line as an other line. Two skipped
      lines whose ranges overlap then vouch for each other, and the commit
      that needs review is refused by neither. Overlap between the two
      permitted lines is allowed by the rules. Each starts "at an end the
      chain reaches" (`:437-438`), so after ``round 2 `E..M` `` skipped for
      the `closer`'s merge, ``round 3 `E..A` `` skipped for its archive is
      a permitted line. Design-review's round-17 box shows this shape
      passing every rule, and `design.md`'s Risk now says a permitted line's
      range holds "any other skipped commit before it".
      **Scenario:** round 1 ends at `E` and the runner ticks. A writer
      commit `W` that needs review lands, and the runner does not untick.
      That is the disclosed residual at `:646-649`, which the text routes to
      this follow-up. The `closer` merges `main` cleanly (`M`) and archives
      with no spec change (`A`). The runner writes ``round 2 `E..M` ``
      skipped: `closer`'s merge, and ``round 3 `E..A` `` skipped: archive.
      Both are permitted shapes, and both hold `W` before their commit. For
      line 2, `W` lies in line 3's range. For line 3, `W` lies in line 2's
      range. So the clause refuses neither. The chain reaches `A`. The tail
      from `A` is tracking only, as security's round-17 branch `p2` measured
      for the one-line form. The follow-up built from this text accepts the
      stage block, and `W` merges unread. With a single line ``M..A``, the
      clause would refuse ``E..M`` as intended. So the defect is in the
      clause's wording, not its idea.
      **Fix, for the `spec-writer` to choose:** "lies in the range of no line
      that is not marked skipped". Or name what counts as covering, a line
      some lane read. The issue then settles whether a commit under a
      permitted judgement skip "needs review". That is the question
      `:1460-1463` already hands it. **Severity:** medium. This text is
      written "to be lifted into an issue as it stands". The clause is the
      only part of the follow-up that sees a commit landed after the tick and
      before the `closer`'s merge, since the chain passes over it. As
      worded, the clause fails open on a shape the rules permit, and it
      fails silently.
      **Measured:** range semantics on this tree, with stand-ins for `M` and
      `A`. `git log --format="%h %p %s" d806700..b33f0f5` and
      `git log --format="%h %p %s" d806700..38844e3d` both list `5bca33f0`,
      the spec-writer commit, which needs review. Whenever `M` is an ancestor
      of `A`, every commit in `E..M` is also in `E..A`. No scratch repository
      was built, because the brief forbids `git -C` into another tree.
      **Outcome (`spec-writer`):** took the suggested qualifier, and nothing
      else. In `proposal.md`, both copies of the refuse clause now end "holds
      before it a commit that needs review and lies in the range of no line
      that is not marked skipped": "What it still cannot see" (`:624-627`)
      and the `closer`-side follow-up's "Skipped lines after round 1"
      (`:1429-1431`). "Other" is dropped because the line under test is
      itself marked skipped, so it can never count as its own cover. In the
      scenario, `E..M` and `E..A` are both skipped, so neither covers `W` and
      the clause refuses both. Whether a commit under a permitted judgement
      skip "needs review" stays with the issue, as `:1462-1465` already
      hands it. **Left for the `dev-writer`:** mirror the same replacement,
      "lies in no other line's range" becoming "lies in the range of no line
      that is not marked skipped", at `design.md:1228` and `:2159`, and at
      both of PR #174's body copies of the refuse list.

**Q1: is my round-17 box closed?** Yes, by removal. The first-parent procedure
is gone, so the missing round-line allowance has nothing left to apply to.
`proposal.md:1424-1443` states only what the check must refuse and accept,
and `:1456-1463` hands the procedure to the issue.

The refuse list names every case this piece's reviewers found:
- a repair marked skipped, whatever its reason (spec-test rounds 15 and 16);
- a merge of `main` that is not the `closer`'s (security round 17);
- a merge or archive line whose range runs past its commit (security
  round 16);
- a merge or archive line that holds an unreviewed commit before its commit
  (security round 17's `W`, and my round-17 low note on a line starting too
  early).

The accept list names every permitted form: the judgement skip (readability
round 17), the `closer`'s merge when it stopped on no conflict, and the
spec-free archive, each on a line of its own. The indistinguishable case, a
repair marked skipped under a judgement-style reason, is named as open.

The only gap is the box above: the fourth refuse form's qualifier fails open
when two lines overlap.

**Q2: `tasks.md` section 24 verify commands.** Each one was run as the row
names it.
- **24.1:** `git grep -n -F -e "stopped on no conflict" -- .claude/agents/RUNNER.md`
  prints `:549` and `:754`. `git grep -n -F -e "clean merge" -- .claude/agents/`
  prints nothing. Both match the row's claim. I did not open the file, so
  that `:549` is step 3's list and `:754` is the tail bullet rests on round
  17's line references. **Holds** as far as a grep can show.
- **24.2:** `git grep -n -F -e "first parent" -- …/design.md` prints `:1052`
  and `:1604`. The Risk "Only the runner runs the pre-tick checks" starts at
  `:2125`, and the next Risk starts at `:2184`. Neither hit falls inside the
  Risk. **Holds.**
- **24.3:** "reason on the line" prints nothing. "A forbidden skip can read
  exactly" prints `:1231`. That line lies inside "What it still cannot see",
  which runs from `:1153` to "What else was considered" at `:1268`.
  **Holds.**
- **24.4:** the grep prints `:977` ("skipped at the tail check") and `:1043`
  ("The start-side argument"). **Holds.**
- **24.5:** `findings/readability.md`'s round-17 `dev-writer` box is `[x]`
  and carries an outcome. **Holds.**
- **24.6:** read with `gh pr view 174 --json body` (`updatedAt`
  2026-09-27T04:43:48Z). The `closer`-side follow-up carries the refuse and
  accept lists, the note that `proposal.md` sets no procedure, and the new
  open question. The skippable pair names the `closer`'s merge when it
  stopped on no conflict. The removed first-parent criterion is gone.
  **Holds.** Both of the body's copies of the refuse list carry the clause
  from the box above.
- **24.7:** `openspec validate 171-workflow-rules --strict` prints "Change
  '171-workflow-rules' is valid". **Holds.**

Below the box threshold, in prose only:

- **`proposal.md:1399-1401` gives the chain more credit than it has.** It
  says the check's "chain to its own HEAD also sees a commit that landed
  after the runner's tick with no untick", and `:646-649` says the same.
  That is true only for a commit after the last skipped line. A post-tick
  commit before the `closer`'s merge sits under the skipped merge line, and
  the chain passes over it, as security's round-17 `p2` measured. What sees
  it is the skipped-line clause, which the box above says fails open. Low.
  It is pre-existing text, and the box's fix makes the combined check true.
- **An archive commit with extra edits is still accepted.** Security's
  round-16 low note was that "changed nothing under `openspec/specs/`" is the
  wrong fact for "needs no review", because an archive commit that also
  edits a file outside the moved folder meets it. The accept list uses the
  same words, so the follow-up would accept that commit. Low. It needs a
  `closer` breach.
- **The reasons for dropping the first criterion survive only in
  `design.md`.** `proposal.md:1439-1443` cites the three round-17 findings
  files for why the criterion "refused lines the rules allow", and the
  `closer` deletes those files. The concrete traps are the round's own
  record in the range, another skipped commit in the range, and the
  judgement skip. `design.md`'s Risk states them, and it is archived. An
  issue written from `proposal.md` alone would not know them, though it
  would meet them. Low.

Read: the `d806700 b33f0f5` diff of `proposal.md`, `design.md` and
`tasks.md`, `proposal.md:380-459`, `:580-659` and `:1370-1469`, the round-16
and round-17 sections of this file, `findings/security.md` from round 16,
`findings/readability.md`'s round 17, `findings/design-review.md` from round
16, PR #174's body, and issues #171 (body and comments), #170, #169 and #133
(state and comments). All four issues are open with no comments, and their
`updatedAt` is unchanged since round 17. Nothing under `.claude/agents/` was
opened. The 24.1 greps over it printed two lines and nothing, and no
`RUNNER.md` hunk was read. No mutation was run, because the change is prose.
This round added no scratch to my tree.
