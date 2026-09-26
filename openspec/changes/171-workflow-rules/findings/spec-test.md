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

- [ ] **`spec-writer`** — `proposal.md:242-246`, the premise under the
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
