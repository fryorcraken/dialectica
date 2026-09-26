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
