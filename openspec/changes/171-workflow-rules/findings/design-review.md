# Design review — 171-workflow-rules

## Method

Read `design.md` in full, all four issues (`gh issue view 171/170/169/133
--repo fryorcraken/dialectica --json body,comments`), and the diffs of every
changed file under `.claude/agents/` (`git diff origin/main...HEAD -- <path>`,
three dots). Re-ran every command `design.md` cites a result for, rather than
trusting the quoted output.

## Verified: measurements in design.md are accurate

Two `design.md` entries make falsifiable claims about what a command returns
on this tree. Both were re-run and matched exactly:

- The five `git ls-files` results under "Both readers of the block find it by
  exact path" (`171-workflow-rules` → only the live folder; `op-clock` → only
  `archive/2026-09-16-op-clock`; `time-pegged-clock` → only
  `archive/2026-09-25-time-pegged-clock`; `home-screen-key-states` → only its
  own folder; `clock` → nothing). **Measured:** ran all five, output identical
  to the entry's claims.
- The suffix-glob counterexample: `git ls-files -- "openspec/changes/*clock/tasks.md"`
  returns both `archive/2026-09-16-op-clock/tasks.md` and
  `archive/2026-09-25-time-pegged-clock/tasks.md`. **Measured:** ran it,
  matches.

## Verified: each Decisions entry against the role-file text it describes

Walked every entry against the actual diff, not a summary of it:

- "One section of `RUNNER.md` carries the whole sequence" — matches: the new
  "From the `dev-writer`'s hand-back to the merge" section has exactly the four
  numbered steps named, with "The `closer`, and what comes back" kept as its
  subsection heading.
- "The re-review is one runner-owned row, and the runner unticks it" — matches
  `spec-writer.md`'s added template row and `RUNNER.md` step 3's tick/untick
  text, including the two-line worked example.
- "Every commit that merges" definition — matches step 3's "What counts"
  paragraph verbatim in substance (tracking list: box flipped, findings
  outcome, stage-row tick, runner's own record line).
- "The re-review brief carries what differs" — matches step 3's brief bullet,
  including the `findings/`-already-deleted case and the "only if it has a
  finding" qualifier.
- "Sizing guidance comes from #171, and names no model" — matches; `RUNNER.md`
  names the Agent tool's `model` override and no specific model.
- "`NO SPEC:` routing: a fresh `spec-writer`" — matches step 1's "Dispatch a
  fresh `spec-writer`... that one's tree was forked before the `dev-writer`'s
  commits existed".
- "Markers are shown by command, and unmarked decisions by quotation" —
  matches step 1's `git grep -n "NO SPEC:"` instruction and the
  quote-verbatim instruction for unmarked decisions.
- "After the `spec-writer`, the brief chooses who brings markers into line" —
  matches, including that `RUNNER.md` now points to `tester.md` rather than
  restating what a decided marker becomes. `tester.md`'s actual wording
  ("Reword it to cite the scenario... or remove it") matches the design
  entry's summary of it.
- "`--admin` is forbidden in Step 6, with a pointer from 'What you never do'"
  — matches `closer.md`'s new Step 6 paragraph and "What you never do" bullet,
  word for word on the substantive points (never `--admin`, never touch
  branch protection or a ruleset, stop-and-report `gh pr view` output on
  `BLOCKED` with every check green).
- "#133: two lines restored by hand" — matches; the restored `README.md`
  paragraph is character-for-character the text quoted in issue #133's body.

No drift found between any Decisions entry and the code it describes.

## Verified: the specific questions this review was asked to check

- **Why re-review size is left to judgement** — recorded, under "Sizing
  guidance comes from #171, and names no model": post-review changes range
  from a one-line fix to a rewrite, so a fixed count would be wrong for most
  of them; the issue's own guides are carried into `RUNNER.md`.
- **Why the untick is limited to one row** — recorded, under "Unticking is
  new in this flow": it is the only row the runner (rather than an agent)
  owns, and it is the mechanism by which the `closer`'s existing Step 1 gate
  can see a red-CI fix that hasn't been reviewed yet.
- **Why the pathspec appears twice** — recorded in full, under "The command
  appears twice... and that is deliberate": each copy sits in the file of the
  agent that has to run it (`closer.md` Step 1, `RUNNER.md`'s "Rebuild the
  state"), neither agent reads the other's file as a matter of course, and
  only the command is duplicated — the reasoning for why the block moves is
  stated once, in `RUNNER.md`.
- **Why #169's optional point was declined** — recorded, under "`NO SPEC:`
  routing: a fresh `spec-writer`, not the one that wrote the spec": the
  callback dispatch's tree would be forked from before the `dev-writer`'s
  commits and so cannot see the markers being judged; keeping the tree also
  leaves a stale full checkout standing for the whole `dev-writer` pass
  (citing CLAUDE.md's "Worktrees are not scratch").
- **Why #165's `BLOCKED` is out of scope** — recorded: `design.md`'s
  Non-Goals points to "Out of scope" in `proposal.md`, which states it there
  ("This piece only stops a closer from going around a block. It does not
  diagnose one.").
- **The owner-authorised exceptions in `d805fe4`** — both are recorded, each
  citing the owner's authorisation explicitly: the `tester.md` decided-marker
  rule (under "After the `spec-writer`, the brief chooses who brings markers
  into line", and again in Risks/Trade-offs) and `closer.md` Step 3's
  re-dispatch-skips-archive rule (under "Both readers of the block find it by
  exact path", and again in Risks/Trade-offs).

## Verified: design.md's reasoning against what the issues actually say

Re-read all four issues fresh via `gh issue view --json body,comments` rather
than from `design.md`'s paraphrase.

- #171's text on sizing, its two named 2026-09-25 incidents (#134/PR #164 and
  #162/PR #165), and "A green CI run and the author's own mutation runs are
  not review, and a warning the owner did not answer is not consent" are
  carried into `design.md` and `RUNNER.md` without distortion.
- #170's account of the `BLOCKED` merge, its stated hypothesis (unverified)
  about `requiresApprovingReviews`/`enforce_admins`, and its proposed fix
  match `design.md`'s "`--admin` is forbidden" entry. `design.md` correctly
  keeps #170's diagnosis question ("why #165 was `BLOCKED`") out of scope
  rather than asserting an answer to it.
- #169's account (marker routing lived only in `spec-writer.md`/`tester.md`,
  not `RUNNER.md`) and its optional suggestion (keep the `spec-writer`'s tree)
  are both reflected accurately, including the optional point being decided
  against with reasoning, not silently dropped.
- #133's restored paragraph, its account of how the drift got in (a #131
  review finding used as unauthorised license to edit `.claude/`), and which
  hunk stays (`dev-writer.md`'s owner-approved absolute-paths fix) all match.

## Areas not flagged

The Decisions section's format is consistent throughout: each entry states
what was chosen, the constraint, alternatives considered with what ruled each
out, and cost/what it forecloses. The guard-style entries (the re-review row,
the exact-pathspec entry) carry worked "what breaks without it" sections in
place of mutation-test evidence, which is the closest equivalent available for
a change with no source diff and no test suite that reads `.claude/agents/` —
`tasks.md` states this explicitly and does not claim mutation coverage it
doesn't have.

No findings to report.

## Round 2

- [x] **re-review `c222c37..9dc235c`: no findings** — read `design.md` in full
      (the new merge-not-rebase, closer-stops-on-conflict, spec-changing-archive,
      fast-forward, "what the runner commits", clean-verdict-box,
      findings/-deletion-in-Step-3 and corrected-product-decision entries), the
      role-file diffs over the range (`RUNNER.md`, `closer.md`, `README.md`,
      `spec-writer.md`), `proposal.md`'s diff over the range, and the four issues
      fresh via `gh issue view --json body,comments`. Re-ran the reflog
      command (`git reflog show --date=iso piece/171-workflow-rules`) and
      confirmed the cited SHAs and timestamps (`abc7693`, `c67aa24..d41d0fd`,
      the four cherry-picks at 23:32:26–27, the `reset` to `d41d0fd` at
      23:32:47) match exactly. Checked the numbered-step structure in
      `RUNNER.md`'s new section against `design.md`'s claim (four steps,
      `**1.**`–`**4.**`, with `### The \`closer\`, and what comes back` as
      step 4's subsection) — matches. Checked the findings-gate grep commands
      in `closer.md` (`grep -rn "^- \[ \]"`, `grep -rc "^- \["`) against
      `design.md`'s citation of them for the verdict-box mechanism — matches.
      Grepped for stale rebase-era phrasing in `closer.md`, `RUNNER.md` and
      `README.md` (`closer.*rebase`) — none found. Every Decisions entry added
      or changed in this range has a matching, word-for-word implementation in
      the role files, including its rejected alternatives; nothing in `design.md`
      is stale against a later ruling in this same range. Clean.

## Re-review `9dc235c..34fd428`

Read `design.md` in full, the range's diff of `.claude/agents/` (`RUNNER.md`,
`closer.md`, `dev-writer.md`, `README.md`), `spec-writer.md`'s stage-block
paragraph, `proposal.md`'s Impact and "Out of scope" entries for the role
files, and #171, #170, #169 and #133 fresh (`gh issue view --json
body,comments`; none has comments, and none contradicts a decision). Re-ran:
the pre-tick `git grep -l -F "c222c37..9dc235c"` over `findings/` (all six
files, as `design.md:401-403` says); the `git ls-files` pathspec for
`op-clock`, `time-pegged-clock`, `home-screen-key-states` and `clock`
(matches `design.md:180-187`); and the archive check on `2bda577f` (the two
specs `design.md:732-734` names).

The new and changed entries are taken as recorded: the archive check before
the push (`closer.md:281-324`, and the refused-push untick at
`RUNNER.md:691-697`); the range heading and pre-tick check
(`RUNNER.md:553-609`, with the "What you read" row at `:81`); returns as
examples with `BLOCKED` routed (`RUNNER.md:647-709`); the mutating reviewer's
patch-around-rebase steps, with `--autostash`, commit and discard rejected
(`RUNNER.md:282-310`); the fast-forward on every `dev-writer` pass
(`RUNNER.md:242-269`, `:163-174`, `:363`, `:657`); and the README
"brought onto" passages. Three findings, all on the reasoning about which
statements of the old cherry-pick route were corrected and which kept.

- [x] **`dev-writer`** — `design.md:447-452` — a Decision still names the
      cherry-pick as the `dev-writer`'s route, which a later entry in this
      same range says it never is.
      **Scenario:** the "`NO SPEC:` routing" entry argues "The `dev-writer`'s
      commits reached the piece by cherry-pick afterwards, onto the runner's
      branch". `design.md:825` now says "a reviewer is still cherry-picked and
      a `dev-writer` never is", and `RUNNER.md:169-172` has the runner
      fast-forward to it. The argument (the first `spec-writer`'s branch never
      receives those commits) survives with "were brought onto the runner's
      branch"; only the named mechanism is stale, and it is the one
      mechanism this piece spent a Decision retiring.
      **Measured:** `git grep -n -F "reached the piece by cherry-pick" --
      openspec/changes/171-workflow-rules/design.md` returns line 449.

      **Fixed** (this commit), as suggested: the entry now says the
      `dev-writer`'s commits "were brought onto the runner's branch
      afterwards, and never onto that agent's `worktree-agent-<id>` branch".
      The argument is unchanged. The same search now returns nothing.

- [x] **`dev-writer`** — `design.md:826-828` — the justification for leaving
      `dev-writer.md`'s "The runner cherry-picks your commits" is weak: it
      would equally have left the passages this piece did correct, and the
      runner is pointed at that sentence.
      **Scenario:** the ground given is that fast-forwarding "is the runner's
      step, stated in `RUNNER.md`, and changes nothing the `dev-writer` does".
      That is also true of the `dev-writer.md:131` clause and the four
      `README.md` passages this range corrected, which were corrected because
      "left as they are, they contradict `RUNNER.md` in this piece, and
      correcting them adds no rule" (`proposal.md:467-470`). By that test
      `dev-writer.md:173` ("The runner cherry-picks it onto `piece/<name>`")
      and `:203-205` ("The runner cherry-picks your commits onto its own local
      `piece/<name>` afterwards — that is for *its* HEAD") qualify too. And it
      is not only the `dev-writer` who reads them: `RUNNER.md:165-166` sends
      the runner to `dev-writer.md` for this very sequence ("states the
      sequence and owns it"), where it meets the route the reflog incident at
      `design.md:788-796` shows a runner following. Either correct those
      sentences under the same authorisation as `:131` (with `proposal.md:779-782`
      following, which is the `spec-writer`'s), or record in this entry what
      distinguishes them from the corrected passages.
      **Measured:** `git grep -n -i "cherry-pick" -- .claude/agents/dev-writer.md`
      returns lines 158, 173, 187, 203 and 214; 173 and 203 state the route.

      **Fixed** (this commit), by the first option: the sentences are
      corrected. The `spec-writer`'s callback (`23aaac3`) contracted the
      correction under the `README.md` criterion, after `findings/spec-test.md`
      raised the same point. `dev-writer.md`'s four route sentences (at 173,
      187, 203 and 214) now say "brings … onto" or "bringing … onto", each
      changed only in the words naming the route; 158's "is not something the
      runner can cherry-pick", about commits made on the wrong branch, stays,
      since it holds of either route. `git grep -n -i "cherry-pick" --
      .claude/agents/dev-writer.md` now returns line 158 only. In `design.md`
      the fast-forward entry now states the criterion once for every file the
      piece edits, lists the four sentences, and records the old reason as
      rejected because it did not separate `dev-writer.md` from the `README.md`
      passages and because `RUNNER.md` sends the runner to `dev-writer.md` for
      this sequence.

- [x] **`dev-writer`** — `design.md:1046-1055` — the entry correcting
      `dev-writer.md`'s false-premise clause does not record why `README.md`'s
      stage-block section, which states the neighbouring premise, is kept.
      **Scenario:** `README.md:238-242` says one row per agent avoids "the
      conflict one-row-per-agent exists to prevent" and that "concurrent
      cherry-picks never touch the same line". Both are literally true and
      both read as "so the ticks do not conflict", which is the premise this
      entry calls false. The reason for keeping them ("literally true", the
      owner's "Each agent flips its own row" standing, left to the
      one-file-per-row follow-up) is only in `proposal.md:471-476` and
      `:880-883`; `design.md`'s Goals (`:39-42`) mention only "one row per
      stage, then three rows". A reader who meets the corrected
      `dev-writer.md` clause and the uncorrected README sentence has to go to
      the proposal to learn the asymmetry was chosen. A gap, not a
      contradiction.
      **Measured:** `git grep -n -F "never touch the same line" --
      openspec/changes/171-workflow-rules/design.md` returns nothing;
      the same search over `proposal.md` returns line 659, and
      `"literally true"` returns `proposal.md:475`.

      **Fixed** (this commit). The `spec-writer.md` correction entry in
      `design.md` gains a paragraph on `README.md`'s stage-block section,
      quoting both sentences, and says why they stay: each is literally true
      (a tick changes its own line and no other), neither says the picks are
      clean, which is what `spec-writer.md` and `dev-writer.md` claimed; the
      criterion is the one the route sentences follow (a false sentence
      changes, a true one stays); "Each agent flips its own row" stands by the
      owner's ruling; and whether that section should say neighbouring ticks
      conflict is the one-file-per-stage-row follow-up's, whose entry lists
      the section. `git grep -n -F "never touch the same line" --
      openspec/changes/171-workflow-rules/design.md` now returns that
      paragraph.

Nothing else is stale: every Risk that has been fixed says so with its
mitigation, no Decision is superseded by a later callback, and every
measurement I re-ran holds.

## Re-review round 3 `34fd428..dc1390a`

- [x] **re-review round 3 `34fd428..dc1390a`: no findings** — read `design.md` in full, the range's diff of `RUNNER.md`, `closer.md` and `dev-writer.md`, `RUNNER.md:238-282`, the three round-2 boxes above against HEAD, and #171, #170, #169, #133 fresh; clean

All three round-2 fixes hold at HEAD: `git grep -n -F "reached the piece by
cherry-pick"` over `design.md` returns nothing; `git grep -n -i "cherry-pick"`
over `dev-writer.md` returns line 158 only, the one sentence the fast-forward
entry (`design.md:902-917`) says stays; and `design.md:1179-1192` records why
`README.md:238-243`'s stage-block sentences are kept.

Each Decision the range changed matches the role-file text it describes. The
round-numbered forms and pre-tick check (`design.md:382-491`) match
`RUNNER.md`'s brief bullet, "Record the call" and the tick paragraph, including
the lane re-run getting its own line, the later-round exception, the single
quotes, and the "What you read" row. The fast-forward criterion
(`design.md:852-926`) matches `RUNNER.md:242-269` and the four reworded
`dev-writer.md` sentences quoted word for word. The `spec-writer.md`
correction entry's account of `dev-writer.md:131` matches the new pointer
wording. The untracked-file paragraph (`design.md:1093-1114`) matches the
moved `RUNNER.md` passage, including the one exception (an incoming commit
adding a file at the untracked path). The reclassified Risks name commands
that are in the role files as quoted. `closer.md`'s "Then return" now names
`BLOCKED` among its stops, which agrees with the "returns as examples" entry.

## Re-review round 5 `dc1390a..d1c8726`

- [x] **re-review round 5 `dc1390a..d1c8726`: no findings** — read the range's diff of `RUNNER.md`, `design.md` and `proposal.md`, `findings/security.md`'s round 3 box and outcome text, and `RUNNER.md:238-321`'s cherry-pick-conflict and rebase passages; clean

**The restated rule takes the recorded decision, word for word.** The
`design.md` bullet (`:391-401`) and `RUNNER.md:604-612` differ only in person
("however the lane is run again" against "however you run it again"); every
other clause is identical, including the `SendMessage` message giving "the new
line's two forms whole, as a brief does", the carve-out list "finishing a round
it has not yet recorded, such as after a stall, committing, or rebasing", and
"Its record, once committed, is the round's own". `RUNNER.md`'s added sentence
"A continued run needs the new number as much as a fresh one …" is the new
`design.md` paragraph's reason in brief, and `proposal.md:183-203` carries the
same text. The pre-tick check's two word changes ("the round ran", "a later
round ran again") match `design.md:414-415`; the summary at `design.md:103-104`
and "What breaks without the number" (`:513-515`) now say "fresh or continued".
`git grep` for "dispatched again", "re-dispatch" and "not a new dispatch"
across the role files, `design.md` and `proposal.md` finds none keyed to a
review lane except the new paragraph's account of the old rule, so the sweep is
complete.

**The rebase reasoning is consistent with the contract.** `proposal.md` states
the rule keyed on "adds no review to a record already committed" and names
rebasing, without saying why rebasing needed naming; `design.md:452-457`
supplies the reason from `findings/security.md`'s outcome text. The two agree:
the proposal's key is not "has not yet committed", so the rebase case is
covered by the key itself and the design paragraph only explains why the
security reviewer's suggested key would not have covered it. The scenario it
cites matches the flow elsewhere in this piece: reviewers are still
cherry-picked (`design.md:926`), and a conflicting cherry-pick sends the agent
back to rebase its own branch via `SendMessage` (`RUNNER.md:275-279`,
`design.md:1039-1045`) after its record is committed.

Below medium, unboxed:

- In the restated sentence, "such as after a stall" sits inside the
  three-item list, so "finishing a round it has not yet recorded, such as after
  a stall, committing, or rebasing" can be read as three examples of a stall.
  `design.md:453` gives the same list without the aside and reads cleanly.
- The new Rejected bullet names what ruled forbidding continuation out but not
  what the chosen rule costs: every continuation for more review now needs a
  round line and a message carrying both forms whole, which is the runner's
  cheapest step made slightly less cheap.
- `proposal.md` says "a later line ran again" where `RUNNER.md` and
  `design.md` say "a later round ran again". Pre-existing wording, same
  meaning.

## Re-review round 6 `d1c8726..6d43cda`

- [x] **`dev-writer`** — `design.md:1261-1270` and `:1276-1283`: the Risk's
      remedy cannot reach the case this range added to it, and the open design
      call behind that case is not recorded. The standing-test follow-up is
      "a script … that extracts each command from the role file and runs it
      against fixtures, the four fail-open cases first". Three of the four are
      defects in a role file's copy of a command: a mistyped
      `openspec/specs/` path, a pattern cut to the bare range or one SHA, and
      the save with `HEAD` dropped. A test that runs the role file's copy
      turns red on each. The stale-number case is not one. `RUNNER.md:633`'s
      command carries `<n>` as a placeholder, so the role file cannot hold a
      wrong number. The number is the runner's own input at the moment it
      runs the check, and a fixture test of the role file's text stays green
      whatever the runner types. So the `→` line calls it deferred to a test
      that structurally cannot see it, and it is listed first in that test's
      queue. `findings/spec-test.md`'s round 5 box left one mitigation open,
      the runner copying the number from the new line rather than typing it,
      and its outcome says that call "is `design.md`'s". The 14.1 pass did
      not take it, reject it, or defer it, so `design.md` records no
      alternative. `RUNNER.md:574` already tells the runner to copy the
      number from its line for the *brief*. `:625-634` does not say where the
      check's number comes from.
      **Why medium:** it is the one fail-open case that the flow's own rule
      makes routine (two numbers over one range, `tasks.md` rounds 3 and 4).
      The only record of it points at a remedy that cannot fire, so a reader
      would believe it handled once the follow-up lands.
      **Needed:** in this Risk, split the stale-number case from the three
      role-file cases. Say that it is a runtime input no standing test of the
      role files covers. Record the design call either way: take it (a
      `RUNNER.md` clause saying the check's ``round <n> `<range>` `` is
      copied from that round's own line), or defer it to the owner with what
      rules the alternative out. `proposal.md:853-854` ("Four, and they are
      the ones a test covers first") carries the same framing. If the split
      lands, the `spec-writer` should follow it there.
      **Verified:** `RUNNER.md:633` holds `<n>`, not a value. At `80c1bcc8`
      the round 3 forms over `34fd428..dc1390a` list five files and the
      round 4 forms list `readability.md` alone, which matches the Risk's
      measurement.

      **Fixed** (this commit), by taking the call, as the `spec-writer`
      contracted in `16958b3`. `RUNNER.md`'s tick paragraph now says the
      check's ``round <n> `<range>` `` is copied from that round's own line
      as the brief's was, not typed, with one sentence on what a typed number
      matches; `:574` and the "What you read" row are unchanged. In
      `design.md` the Risk's fail-open list is back to three role-file
      cases; the stale-number case is its own paragraph, saying it is a
      runtime input no fixture test of the role files covers, that the copy
      rule answers it, and that copying from the wrong line is the next Risk,
      "Only the runner runs the pre-tick check", which now names it. The
      follow-up's count reads "three". The pre-tick Decision's check bullet
      states the copy rule, and a new entry records it with two rejected
      alternatives: leaving it typed (re-measured at `c3d697e9`: round 3's
      forms over `34fd428..dc1390a` list five files, round 4's
      `readability.md` alone), and a mechanical guard, which needs a
      round-line parser or a second reader, the deferred `closer`-side
      check. The first unboxed note is answered too: "What it still cannot
      see" now names the wrong-line case (round 5's forms at `c3d697e9`
      include `spec-test.md` and `design-review.md`, re-measured) with a
      pointer to Risks. The second note's analogy is gone with the
      fail-open bullet that carried it.
      `git grep -n -F "four fail-open"` over `design.md` now returns nothing.

Read: `git diff d1c8726...6d43cda` in full. `design.md:370-530` (the pre-tick
decision, "What it still cannot see", the rounds 1 and 2 measurements and
"What breaks without the number"). `design.md:1232-1320` (Risks).
`RUNNER.md:565-650`. `findings/spec-test.md:265-295` and its section
headings, and `findings/architecture.md`'s section headings. Issues #171,
#170, #169 and #133, read fresh: all open, no comments, nothing on round
numbering.

**The edited Risk matches `proposal.md`'s standing-test entry.** The
fail-closed sentence is identical ("a wrong character that no committed
record carries"). The fail-open list has the same four cases in the same
order. The stale-number text agrees, including the refinement: round 4 would
have failed closed because the round 3 readability run wrote nothing. The
count "four" in `design.md:1281` matches `proposal.md:853`'s "Four".

**Citations are accurate.** The separate citation splits the old joint
attribution. The truncation cases now cite `findings/spec-test.md` alone.
Its `9dc235c..34fd428` section (`:147`) holds that measurement at `:275-282`,
and `findings/architecture.md`'s section of that round has no truncation
measurement. The `HEAD`-dropped case keeps both files:
`architecture.md:357` and `spec-test.md:218` are each inside their file's
`9dc235c..34fd428` section. The stale-number citation (`spec-test.md` round 5
box, `:381` on) holds the `80c1bcc8` re-measurement, and I re-ran both
commands at `80c1bcc8` with the same result.

**The two passages left alone were right to be left.** `design.md:493-494`
says the numbered forms for round 2 list nothing, so a check with the wrong
form fails closed. That is a wrong form that no committed record carries,
which is exactly the narrowed fail-closed case. It is still true at HEAD: the
numbered round 2 check lists nothing. "What breaks without the number"
(`:513-515`) describes the decision's absence, not a mistyped value, and
stays true.

Below medium, in prose:

- "What it still cannot see" (`:475-479`) names only an earlier round's form
  quoted whole. The stale number is now a second thing the check cannot see,
  and `:438-440` ("the rejected run's record satisfies nothing the re-run
  must") holds only when the runner uses the new line's number. The
  skipped-check residual gets a pointer to Risks from `:512`. This one gets
  none. A one-line pointer would do, and the box's split may be the natural
  place to add it.
- In the stale-number case, "as with round 1's Sonnet security box" is an
  analogy rather than an instance: that record and the Opus one share the
  un-numbered forms, so no number was involved. The design's own citation
  for it is `:437`. The wording reads as an illustration, so this is minor.
- `proposal.md:848` ("dropped writes `Binary files a/<f> and b/<f> differ`
  for a binary change, and `git apply` then refuses it") was not rewrapped
  after the edit and runs past the file's width. Cosmetic.

## Re-review round 7 `6d43cda..d1d2165`

- [x] **re-review round 7 `6d43cda..d1d2165`: no findings** — read `git diff 6d43cda..d1d2165` of `RUNNER.md`, `design.md` and `proposal.md`, `design.md:400-555` at HEAD, `tasks.md`'s round lines, and #171, #170, #169, #133 fresh; clean

**The round-6 box is fixed.** The stale-number case is out of the fail-open
list and is its own paragraph (`design.md`, Risks), saying it is a runtime
input no fixture test of the role files covers. The follow-up count is
"three" in both `design.md` and `proposal.md`, and `git grep -n -F "four
fail-open"` returns only this file and `tasks.md`'s verify step. The call is
recorded as taken, in a new Decision (`design.md:477-502`) with the chosen
rule, the constraint (the number sets a rejected run aside only if the check
uses the new line's number), and two rejected alternatives, each with what
ruled it out. The first unboxed note from round 6 is answered at `:508-515`.

**RUNNER.md takes the decision as recorded.** The tick paragraph's clause
("copied from that round's own line as the brief's was, not typed") is the
`design.md` bullet's wording (`:413-415`) less "under the re-review row". The
one-sentence reason sits after the command's description, where the new
Decision says it does. Copying works on the round lines as they are:
`tasks.md:25-31` each carry ``round <n> `<range>` `` literally, and rounds 1
and 2 carry their own un-numbered check, which is what `proposal.md`'s "each
of their lines names its own check" and `design.md:517-519` both say.

**`design.md` and `proposal.md` agree** on the rule, the rejected typed
number, the wrong-line residual and its home (the `closer`-side follow-up),
and the Risk split. Re-measured at `c3d697e9`: round 3's forms over
`34fd428..dc1390a` list five files without `readability.md`; round 4's list
`readability.md` alone; round 5's over `dc1390a..d1c8726` list
`correctness.md`, `design-review.md`, `security.md` and `spec-test.md`. All
three match both documents.

Below medium, in prose:

- `proposal.md`'s "What it still cannot see" says a round 6 check "run with
  round 5's start would pass"; `design.md:513` says "round 5's forms".
  "Start" names nothing defined. It reads as a slip for "forms".
- The new Decision says the clause makes the check search what the reviewer
  was told to write "by construction". That holds only for a correct copy.
  The wrong-line paragraph a few lines down names the exception, so a reader
  is not misled, but the phrase claims more than the entry delivers.
- The new Decision has no cost line. The cost is small (the runner copies
  rather than types, which it already does for the brief), and saying so
  would make the entry complete.

## Re-review round 9 `d1d2165..c4b1df5`

- [x] **re-review round 9 `d1d2165..c4b1df5`: no findings** — read `git diff d1d2165...c4b1df5` of `design.md` and `proposal.md`, `design.md:470-590`, `RUNNER.md:546-648`, `findings/security.md`'s round-8 box and `tasks.md`'s round lines; clean

**The two documents agree on all five.** "What it still cannot see": the same
residual, scenario and measurement (``round 3 `34fd428..dc1390a` `` forms at
`9e6dde2f` and `291499c7`, five files), the same "n-th line carries n"
argument, and both end on the unique-and-consecutive criterion. The
stale-number Risk: `design.md` and `proposal.md` both name two residuals, the
wrong line and a repeated or stale number on the line, both routed to the
`closer`-side follow-up. "[Only the runner runs the pre-tick check.]": both add
the repeated-number line to what goes unnoticed and say the deferred check
sees it only by the number criterion; `proposal.md` adds the fixed-start note
and the 1-to-8 measurement, which is detail and not a disagreement. "What else
was considered" (`design.md:580-581`) gains the third residual. The Rejected
entry has no counterpart in `proposal.md`, which is right: a rejected
alternative belongs in the Decision, and `proposal.md` carries the premise it
rests on ("already fixes each line's number").

**The Rejected entry's reasoning is sound in its main argument.** A clause
restating the numbering rule, applied by the runner whose slip it targets, adds
no check; that holds. Below medium, in prose:

- The entry cites the weaker of two rules. `RUNNER.md:604-606` already says a
  lane run again "gets a line of its own — the next number, the same range",
  which is the templated re-run case exactly. Citing it would show the slip
  breaks an explicit rule for that very case, not only an inference from
  "numbered from 1".
- "Reopen a role file to the correctness, security and readability lanes" is
  unmeasured, as the dev-writer flagged, and this piece's own record half
  contradicts it: round 7 (`tasks.md:31`) re-reviewed a one-clause `RUNNER.md`
  edit with correctness and security and skipped readability ("one clause").
  "Correctness and security" is what the record supports. It is also a
  review-process cost, not a design reason, and the entry does not rest on it;
  dropping the sentence would lose nothing.
- Round 7's note stands unfixed: `proposal.md` still says "run with round 5's
  start would pass" where `design.md` says "round 5's forms".

## Re-review round 10 `c4b1df5..842758b`

- [x] **re-review round 10 `c4b1df5..842758b`: no findings** — read `git diff c4b1df5 842758b` of `RUNNER.md`, `design.md` and `proposal.md`, `design.md:460-640`, `RUNNER.md:590-679`, `findings/security.md`'s round-8 and round-9 boxes, issue #171, and re-ran the number check at `94840b52`, `328c1929` and `842758b`; clean

**RUNNER.md takes the decision as recorded.** The Decision says the rule sits
once in the tick paragraph, ahead of the forms check, with a row in "What you
read"; `RUNNER.md:626-648` and `:81` are exactly that, and the row describes
the check's purpose without restating the rule. Every clause of the Decision
and its four sub-bullets has its counterpart in the tick paragraph: the
command, directly-under-the-row, repeat or skip means no tick, put the line
right, a lane briefed from a renumbered line runs again with forms from the
corrected line, an empty listing means a mistyped command, and the one-sentence
reason (the copy rule carries a templated number into brief, heading and forms
check alike). "Record the call" (`:600-617`) is unchanged, as the round-9 box
asked. `design.md` and `proposal.md` agree on the rule, the residuals (wrong
line and a skipped check left for a second reader; a re-run given no line seen
by neither check) and the `closer`-side follow-up as a second reader. The only
difference is that `design.md` cites a second measurement commit, `328c1929`,
which is detail.

**Measurements re-run.** The number check lists nine lines at `94840b52`,
`328c1929` and `842758b` (`grep -c`), all round lines under the row, carrying 1
to 9; the seven-space pattern lists nothing at `842758b`. Issue #171 asks the
runner to record each re-review in the stage block; a check that the record's
numbers are well-formed is inside that, and nothing in the issue contradicts
it.

**The rejected alternative is honest.** It describes round 8's answer as it was
(`findings/security.md:842-875`: recorded as a residual, routed to the
`closer`-side check, classed with a runner that skips the check), and the
ground for rejecting it is round 9's scenario, which holds: a runner that
templates the line and then runs every step skips nothing. The `closer`-side
criterion is kept, correctly re-described as a second reader for a runner that
skipped the number check. **The revised "one more than the highest" rejection
is sound**: every misreading of "the next number", including "next after the
round being re-run", produces a repeat or a skip, which the number check now
lists, so a second phrasing adds no detection.

Below medium, in prose:

- "Word for word" (`design.md:507`) overstates: "one more than the highest" and
  "the next number" are the same rule in different words. "Already says" is
  what the argument needs.
- The new Decision has no cost line. The cost is the runner's judgement over
  the listing: which printed lines stand under the row, and, for a skip,
  whether a line was lost or mistyped (`RUNNER.md:639-641` gives both remedies
  and no way to tell them apart). Both fail closed, since no tick happens
  until the numbers run, so the cost is small; saying so would complete the
  entry.
- "It reads only the line's start" (`design.md:535`) is looser than the
  command: `-F` matches the six spaces and `round ` anywhere in a line, so a
  more deeply indented line would be listed too. The sub-bullet about lines
  "elsewhere in the file" covers the effect, and the line numbers settle it.
- Round 7's and round 9's note on `proposal.md`'s "round 5's start" still
  stands; this range rewrote the paragraph around it and kept the phrase.

## Re-review round 11 `842758b..dd4fe18`

- [x] **re-review round 11 `842758b..dd4fe18`: no findings** — read `RUNNER.md` step 3 ("Record the call", the number check, the forms check) and its "What you read" row, `design.md`'s re-review row Decision, "Findings go under a heading naming the round" through "What it still cannot see", and Risks, and `proposal.md`'s diff over the range; clean

**RUNNER.md takes the decision as recorded.** "One more than the highest already
under the row, 1 for the first" (`RUNNER.md:604`, `design.md:103-104`,
`design.md:396-397`, `proposal.md:188-189`); lines only added below, a number
changed only to repair a repeat; a repeat repaired upward to one more than the
highest, never lowered (`RUNNER.md:659-665`, `design.md:526-530`); the
three-condition listing bracketed by the two rows (`RUNNER.md:641-658`,
`design.md:515-525`); the positional "below, not higher-numbered" exception
(`RUNNER.md:684-686`, `design.md:423-426`); and round 1 skipped with both ends
at HEAD for a piece where nothing lands (`RUNNER.md:609-612`, `design.md:107-109`).
`proposal.md` states each of these the same way, and its Impact and closer-side
follow-up entries were updated from "unique and consecutive" to "no number
repeats". No other role file states a numbering rule (`git grep` for
`round <n>` and `highest` in `spec-writer.md`, `dev-writer.md`, `closer.md` and
`README.md` returns nothing).

**The reversal is recorded honestly.** `design.md:555-575` says what the rule
was (number by position, repair to the position's number), gives the failing
case with concrete numbers (6, 8, 9 lowered to 7, 8 lands the re-run on the
rejected run's number), cites the finding that found it, and names the earlier
rejection of this very clause and the ground it was rejected on: as a
restatement of the rule then in force, which stops being true once numbering by
position is gone. That is the right reason for the earlier reasoning no longer
holding. My own round 10 prose called that rejection sound; it was sound only
against the rule then in force, and this entry says so. What the reversal costs,
losing a gap as a lost-line signal, is recorded as a residual at
`design.md:678-687` and in Risks at `design.md:1510-1512`, with the argument that
the old signal came with a fail-open repair.

**The new rejected alternatives hold.** The indent-only and rows-only
alternatives each fail on the case the other half covers, and the text says
which. Routing the repeat to the `closer`-side check stays rejected on the
ground round 10 accepted.

Below medium, in prose:

- The placeholder alternative (`design.md:621-625`) is rejected thinly. "It
  still cannot tell a lost line from a mistyped number" is the reason, but the
  entry does not say why that matters under this alternative: the placeholder
  repair is safe for a mistyped number and fails open for a lost line, because
  it records "no round ran" over lanes that did run. One clause would make the
  rejection self-standing.
- `proposal.md:1022` still says "consecutive rounds share an endpoint". It
  means rounds adjacent in time, and the range changed the same phrase to
  "neighbouring" at `proposal.md:335`; left as is, it now sits beside a rule
  whose point is that numbers need not be consecutive.
- Round 10's note on `-F` matching anywhere now bears on `design.md:588`, "it
  matches only the fixed start": a round line indented by eight spaces contains
  the six-space pattern and is listed, so the check does not enforce "exactly
  six". The effect is benign, since a listed line has its number read, but the
  sentence claims more than the command checks.

## Re-review round 12 `dd4fe18..1380d50`

- [x] **re-review round 12 `dd4fe18..1380d50`: no findings** — read the range's diff of `RUNNER.md`, `design.md` and `proposal.md`, design.md's "Every commit that merges" entry and issue #171 fresh, and re-ran the entry's measurements; clean

**RUNNER.md follows the decision as recorded, and design.md and proposal.md
agree.** The range rule (`<review>..<HEAD>`, with `<review>` the HEAD the
review round was dispatched from), the two `git diff` commands, the
"What you read" row, and restoring a missing line as the round that ran all
match `design.md`'s "The nothing-landed round 1 records its evidence" and its
at-least-one bullet. They also match `proposal.md`'s two sub-bullets and
at-least-one bullet, with the same two findings cited. The residual
("What it still cannot see"), the two Risks and the `closer`-side
follow-up say the same thing in both documents.

**"Or any other change to `tasks.md`" is a rewording.** Both documents already
say the second command "must show only boxes flipped", and that the line is
written "only once `git diff` over that range shows tracking alone". So a failed
second check already means no skip. RUNNER.md only says the consequence out
loud, which the documents state for the first command. The line adds no new
trigger.

**The measurements reproduce.** `git log --oneline -1 f94f7b8d~1` gives
`c222c37b`. `git diff --name-only c222c37 e7e2bbdd` lists the six findings
files and `tasks.md`. Over `c222c37 ae59b43c` the same command adds
`proposal.md`. And `git grep -n -F "both ends"` at `dd4fe18` finds exactly
one statement in each of the three files, none with a reason. So the claim
that "no recorded decision is reversed" holds.

**The rejected alternatives hold, and the judgement-skip decision is
recorded.** It is the third alternative, "Forbidding a judgement skip of round
1", and #171 backs it. The issue asks for the size of a re-review to be left
to judgement, with "a decision to skip a re-review ... visible rather than
silent". A judgement skip names what landed over a range that holds it. Only
the nothing-landed line asserts a fact that a diff can refute. Rejecting
`git log --oneline` because subjects are where the misreading starts is the
right reason.

Below medium, in prose:

- The check fails closed on some commits that the definition calls tracking,
  yet RUNNER.md and design.md both say a failure "means a commit that needs
  review is in the range". Two examples:
  - A clean merge of `main` between `<review>` and HEAD. `design.md:283`
    says it needs no review, but it puts `main`'s paths into `--name-only`.
  - A non-box `tasks.md` edit, such as a round line, which `design.md:265`
    calls tracking.

  In the nothing-landed case neither is likely before round 1, and a false
  failure costs only an ordinary round. "Means a commit that may need review
  is in the range; size it as step 3 says" would be accurate, and a merge of
  `main` could still be sized as a skip on judgement.
- The entry's own form lacks a stated cost: it forecloses nothing, and it adds
  two commands per nothing-landed piece. One clause under a "Cost" label would
  complete the entry. The cost is visible from the rule, so this is a
  suggestion.

## Re-review round 13 `1380d50..c3bda2b`

- [ ] **`dev-writer`** — `design.md` "Why the derived commit is the dispatch
      HEAD" (and `proposal.md`'s matching paragraph under the `<review>` rule)
      rests on a premise that `RUNNER.md` never states, and the derivation
      fails open where the premise does not hold. The inference is "the runner
      commits nothing between [dispatching the review round and the first
      findings commit], and a reviewer commits only its findings file and its
      own stage row". The second half is true of the reviewer role files. The
      first is a description of how this piece happened to go, not a rule:
      `RUNNER.md` "Record the call" gives only the `git log` command and the
      last-line reading, and `git grep -n -F -e "commits nothing" -e "nothing
      between"` over `RUNNER.md` at `c3bda2b` finds nothing. Nothing forbids
      the case #171 names first: the PR is open during review (the
      `dev-writer` opens it, `RUNNER.md:165`), a red CI run gets a fixer, the
      fixer pushes straight to `piece/<name>`, and the runner fast-forwards
      onto it before cherry-picking any reviewer's commit. The oldest findings
      add then sits on the fix, `<review>` is the fix commit, the fix lies
      *before* the range, and `git diff --no-renames --name-only <review> HEAD`
      lists only `findings/` and `tasks.md`: round 1 is written skipped and the
      fix merges unreviewed. That is the lost-report case the derivation exists
      for, since a runner that remembered the fix would not write a
      nothing-landed line. So "a range starting there misses nothing that
      needs review" holds only if *nothing that merges* lands between dispatch
      and the first findings commit. That condition is wider than "the runner
      commits nothing", because a fast-forward onto a writer's push is not a
      runner commit. Either make it a rule in `RUNNER.md` (bring nothing onto
      your HEAD between dispatching the review round and bringing on its first
      findings commit), or name it in the standing-test Risk as a fourth runner
      input beside "supplies a value of its own" and "reads the first line". As
      written, the inference sounds like a property of the flow when it is a
      property of one ordering.

Below medium, in prose:

- **RUNNER.md follows the decision as recorded.** The derivation command, the
  last-line reading, the pre-archive pathspec kept after the archive, the empty
  listing meaning no `<review>`, `--no-renames` with its reason, the "What you
  read" row, and the lost-line repair running from the derived `<review>`
  rather than a remembered or visible HEAD all match `design.md`'s three rules
  and `proposal.md`'s sub-bullets. The two documents agree. They differ in one
  small place: `proposal.md`'s same-commit measurement still reads
  `git diff --name-only ae59b43c ae59b43c`, while `design.md` records it re-run
  with `--no-renames`. The result is empty either way.
- **The fail-open count is consistent.** Three earlier cases plus four for the
  nothing-landed check make seven in both documents, and the mistyped-name
  derivation is correctly classed as fail-closed.
- **The inference fails in the safe direction in one case.** If any commit
  before the review round added a file under `findings/`, the oldest add is
  earlier than the dispatch HEAD. The range then holds the writer's code
  commits, and the check fails closed, costing an ordinary round. No entry
  says so, but no merge follows from it.
- **The rejected alternatives hold.** Deriving only on a lost report gives one
  value two sources. The archived pathspec is measured to give the archive
  commit. Recording `<review>` on a line under the row is correctly ruled out
  by the number check's first condition. The entry does not consider the
  variant that puts the SHA inside the re-review row's own text, which the
  number check would not see. The "one more typed value" reason covers that
  variant too, so this is a gap in completeness rather than in soundness.
