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
