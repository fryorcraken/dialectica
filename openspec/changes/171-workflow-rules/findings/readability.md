# Readability review — 171-workflow-rules

Dimension covered: **readability only** (correctness, security and
architecture are separate dispatches).

Scope checked: `.claude/agents/RUNNER.md`, `closer.md`, `spec-writer.md`,
`tester.md`, `README.md`, and `openspec/changes/171-workflow-rules/{proposal,design,tasks}.md`
— the full `origin/main...HEAD` diff (three dots). Every `git grep`/`git
ls-files` command quoted below was run in this tree; outputs are pasted
verbatim.

## Findings

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:443-444` (echoed in
      `openspec/changes/171-workflow-rules/design.md:91`) — the claim that
      "the `closer`'s Step 1 refuses to run while a row other than its own is
      unticked" is stronger than what `closer.md`'s Step 1 actually says.
      **Scenario:** a runner unticks the re-review row after a red-CI fix (per
      RUNNER.md's own instruction) and re-dispatches the `closer`, trusting
      that Step 1 will catch it because "it refuses to run" on an unticked
      row. `closer.md`'s Step 1 does say, for the *findings* gate, "Lines
      means unticked findings, which block the merge" — an explicit
      stop-the-merge statement. For the *stage block* check, right below it,
      the text only says: "An unticked row with no agent running is a stage
      nobody did, and that is precisely what the block exists to surface." It
      never says "stop", "refuse" or "block the merge" for this specific
      check — that instruction is left to be inferred from the section
      heading "is the piece finished?" and the top-level ordering. A `closer`
      reading only what closer.md's Step 1 literally states has no explicit
      instruction telling it what to do on finding the re-review row unticked
      beyond noting it, which is weaker than "refuses to run" promises. Since
      design.md cites this exact mechanism as *why the new row needs no new
      check in closer.md* ("With this row, a missing re-review blocks the
      merge through the gate that exists, and `closer.md` needs no new
      check"), the whole justification for not touching `closer.md` rests on
      a behaviour `closer.md`'s own text doesn't quite spell out.
      **Measured:** `grep -n -i "not finished\|is not done\|stage nobody\|unticked row" .claude/agents/closer.md` → only one hit, the "stage nobody did" sentence
      itself, with no adjoining "stop" or "block the merge" clause (contrast
      with the findings-gate paragraph nine lines above it, which has both).
      This is fixable inside the files this piece already touches (soften
      the RUNNER.md/design.md wording to match what `closer.md` actually
      says), so it does not require the closer.md Step 1 edit that would be
      out of this piece's authorised scope.

      **Fixed** (this commit and the one before it), as suggested, by
      softening the claim to what `closer.md` Step 1 literally says.
      `RUNNER.md` step 3 now reads "the `closer`'s Step 1 checks that every
      row but its own is ticked or struck, and that check is the only thing
      that lets it see the commit was never read". `design.md`'s Context
      bullet and "What the row buys" say "checks", not "refuses", and "What
      the row buys" now records that Step 1 is worded as a requirement rather
      than an explicit stop, and that saying "stop" there was outside this
      piece's authorisation. The PR body's matching sentence is updated too.
      Measured after the edit: `git grep -n -e "refuses to run" -e "refuses
      to merge" -- .claude/agents openspec/changes/171-workflow-rules/design.md`
      returns nothing.

- [x] **`dev-writer`** — `.claude/agents/closer.md:84` — the cross-reference
      "Step 3 of [`RUNNER.md`](RUNNER.md)'s 'From the `dev-writer`'s hand-back
      to the merge'" overloads "Step 3" with a name `closer.md` already gives
      to one of its own six top-level steps.
      **Scenario:** a `closer`, mid-way through its own `## Step 1 — is the
      piece finished?`, reads "Step 3 of RUNNER.md's [section]" and,
      because every other "Step N" it has seen so far in this file refers to
      closer.md's own numbered steps (`## Step 1` through `## Step 6`),
      has to stop and re-parse the sentence to realise "Step 3" here means
      RUNNER.md's third numbered item (which RUNNER.md itself calls "step 3"
      in lower case, never "Step 3") rather than this file's own
      `## Step 3 — archiving`. The qualifier "of RUNNER.md" disambiguates it
      on a careful read, so this is a clarity nit rather than a
      correctness defect, but it is exactly the kind of two-numbering-systems
      collision the "one sequence" restructuring in RUNNER.md was written to
      avoid internally. **Measured:** `git grep -n -F "Step 3 of" .claude/agents/`
      → one hit, `closer.md:84`; `git grep -n -F "step 3" .claude/agents/RUNNER.md
      .claude/agents/closer.md` → RUNNER.md uses lower-case "step 3" at
      lines 20, 484, 486 (its own internal numbering); closer.md's own step is
      `## Step 3 — archiving` at line 207. Suggested fix (not applied):
      lower-case "step 3" in closer.md's own cross-reference, matching how
      RUNNER.md refers to itself, or name the section instead of the number
      ("'From the `dev-writer`'s hand-back to the merge', item 3").

      **Fixed** (this commit), with the second suggestion: `closer.md` Step 1
      now reads "[`RUNNER.md`](RUNNER.md)'s "From the `dev-writer`'s
      hand-back to the merge", in its item 3, says why…", naming the section
      first and calling the number an item. Measured after the edit:
      `git grep -n -F "Step 3 of" -- .claude/agents` returns nothing. The
      other `closer.md` references this pass adds to `RUNNER.md` name a
      section ("The `closer`, and what comes back") rather than a number.

## What is clean

- **The "From the `dev-writer`'s hand-back to the merge" section reads as one
  sequence.** Items 1 (route `NO SPEC:`), 2 (tester + review round), 3
  (re-review) and 4 (the `closer`) run in order with a single opening
  sentence ("One sequence, in order...") governing all four, and item 4's
  one-line pointer flows directly into the `### The closer, and what comes
  back` subsection immediately below it — a natural continuation, not two
  documents stitched together. The size imbalance (item 3 runs to roughly 70
  lines against one-liners for items 2 and 4) is real but each short item
  correctly points elsewhere for its detail (item 2 to "How many at once",
  item 4 to its own subsection) rather than being under-explained; I do not
  think this rises to a defect.

- **Cross-references I checked resolve to what they claim.** Every pointer to
  `"From the `dev-writer`'s hand-back to the merge"` (in `closer.md` ×2,
  `spec-writer.md` ×1, `RUNNER.md` internally) matches the heading text
  exactly, including the backticks around `dev-writer`. `closer.md`'s
  pointer to "Step 3... says why the stage block and any re-review findings
  are there" correctly lands on the paragraph that states that reasoning
  ("The `closer` archives before it watches CI..."), matching design.md's own
  claim that the *why* is stated once, in RUNNER.md's step 3, and that
  `closer.md`/"Rebuild the state" say only *where*, not *why*.

- **Every `git grep` / `git ls-files` command quoted in `tasks.md` and
  `design.md` as a "Verify:" step produces exactly the output claimed**, run
  fresh in this tree:
  - `git grep -n -F "NO SPEC" -- .claude/agents/RUNNER.md` → exactly the one
    location inside the new step 1 text (no second statement of the rule
    elsewhere in the file, as task 1.1 claims).
  - `git grep -n -F -- "--admin" -- .claude/agents/closer.md` → the three
    lines the #170 fix introduces (Step 6 twice, "What you never do" once).
  - `git grep -n -F "review: design" -- .claude/agents` → one hit, confirming
    the stage-block template still has exactly one copy.
  - `git ls-files -- "openspec/changes/<name>/tasks.md" "openspec/changes/archive/????-??-??-<name>/tasks.md"`
    for `clock` (nothing), `op-clock` (only `archive/2026-09-16-op-clock/tasks.md`),
    `171-workflow-rules` (only the live folder) and `home-screen-key-states`
    (only `archive/2026-09-24-home-screen-key-states/tasks.md`, not the
    `-followup` change) all match design.md's measured claims verbatim.

- **The retraction sweep found nothing left over.** None of the example
  phrases from the brief, nor several more I tried, survive anywhere under
  `.claude/agents/`, `CLAUDE.md` or `docs/`: `"re-dispatch the \`closer\`"`
  (the one remaining hit is the new, correct usage — "and only then
  re-dispatch the `closer`" after going through step 3), `"Keep the
  markers"`, `"Two paths back"`, `"three rows the \`closer\` owns"` (still
  accurate — the new re-review row is one more row *before* the closer's
  three, as `proposal.md`'s own "Impact" section explicitly notes and
  justifies), `"to the owner as open decisions"`, `"the owner's to decide"`,
  and `"re-dispatch the \`closer\` straight after"` (the remaining
  occurrences of that phrase are in `proposal.md`'s "Why" section, correctly
  describing the *old*, now-fixed behaviour as history).

- **The `README.md` `.openspec/settings.json` paragraph restoration (#133)**
  reintroduces text that states the "it's the owner's file" rule a second
  time (rather than pointing at `CLAUDE.md`'s general rule) — on its face
  the anti-pattern CLAUDE.md warns about. I checked this is not an oversight:
  it is exactly what issue #133 asks for (the pre-#119 wording, byte-for-byte
  restored, per the issue's own quoted text), and `design.md`'s "#133: two
  lines restored by hand" section explains why the narrow duplicate is the
  owner's original, intentional wording. Not a finding.

- **Tester.md's `NO SPEC:` section reads clearly** and correctly distinguishes
  a marker the brief names as decided (closed) from any other marker (open,
  keep and report), matching `RUNNER.md` step 1 and `proposal.md`'s
  description of the same rule stated once.

## Re-review `c222c37..9dc235c`

Dimension: **readability only**. Read: `git log --oneline c222c37..9dc235c`;
`git diff c222c37..9dc235c` for `.claude/agents/RUNNER.md`, `closer.md`,
`spec-writer.md` and `README.md`; `RUNNER.md` and `closer.md` in full at HEAD;
`proposal.md` in full; `tasks.md` sections 8-9; the `design.md` passages the
retraction greps hit; `dev-writer.md:124-218` and `README.md:160-253` for the
sweep. Every command quoted was run in this tree.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:597` — "Five things come
      back" reads as the complete list of the `closer`'s returns, and it is
      not: `closer.md:417-424` has the `closer` stop and report a PR that
      stays `BLOCKED` with every required check green, and no entry in "The
      `closer`, and what comes back" covers it.
      **Scenario:** the `closer` hits the #170 case this piece exists to
      handle, stops as Step 6 says, and reports the `gh pr view … reviewDecision`
      output. The runner turns to its section for what comes back, counts five
      bold entries (red run, unticked box, conflict, spec-changing archive,
      refused push), and finds none that matches: no route, and no "goes to
      the owner" like the refused push has. The same gap exists for the
      zero-box findings file `closer.md:113-114` sends back "naming the file".
      And the "An unticked box" entry (`RUNNER.md:620-621`) glosses it as "a
      finding was never answered … whoever the finding names", which does not
      fit an unticked stage row. Step 3 (`RUNNER.md:565-567`) relies on the
      `closer` returning on exactly that: an unticked re-review row.
      **Measured:** `git grep -n -F "Five things come back" -- .claude/agents`
      → `RUNNER.md:597` only. `git grep -n -F "BLOCKED" -- .claude/agents/RUNNER.md`
      → no output. The count was "Two" at `c222c37`. This range made it a
      number the reader can check, and it checks false. `proposal.md:188-189`
      already sets the rule for this shape elsewhere: returns are "given as
      examples, not a list that reads as complete". Either drop the count,
      or add a `BLOCKED` entry and widen the unticked-box entry to stage rows.
      Severity: moderate. This is the one return #170 is about.

      **Fixed** (this commit), with both suggestions, as the `spec-writer`'s
      box below set the contract. "Five things come back" is gone: the
      section now says its returns are examples, that `closer.md` has more
      stops than it routes, and that a return it does not name comes with
      its evidence and is routed by what it is. It gains "A PR that stays
      `BLOCKED` with every required check green": to the owner, with the
      `gh pr view` output the `closer` reported, no other route and no
      diagnosis. "An unticked box" now covers a stage row: another agent's
      row goes back to that agent, continued to tick it, or to a fresh agent
      for the stage; the re-review row means a round is owed. The refused-push
      entry gains the untick-and-record step for a report that lists files
      from the archive check. Measured after the edit:
      `git grep -n -e "things come back" -- .claude/agents` returns nothing,
      and `git grep -n -F "BLOCKED" -- .claude/agents/RUNNER.md` returns the
      new entry. `design.md` has a Decisions entry, "The `closer`'s returns
      are given as examples, with no count".

- [x] **`spec-writer`** — `openspec/changes/171-workflow-rules/proposal.md:359`,
      `:369` and `:624` — the contract gives three different accounts of what
      comes back from the `closer`.
      **Scenario:** a `dev-writer` fixing the box above goes to the contract
      for the list. Line 359 says Step 4 "lists every return" and names five.
      Line 369, ten lines later, says Step 6's `BLOCKED` stop is "reported as
      before", which makes it a sixth return the "every" list leaves out. The
      Impact section at line 624 says "its four returns". The writer cannot
      tell which count the contract means, or whether leaving `BLOCKED` out
      was intended.
      **Measured:** `git grep -n -i -e "four returns" -e "every return" --
      openspec/changes/171-workflow-rules/proposal.md` → lines 359 and 624.
      `tasks.md:188` says 9.x "Supersedes … 8.3's 'four returns'", and
      `tasks.md:224` says "five returns", but the proposal was never brought
      into line. Severity: low. The fix is wording, but it decides the fix for
      the box above.

      **Fixed** (`spec-writer`, this commit). The contract now gives one
      account. Its entry on Step 4, "The `closer`, and what comes back", says
      the section gives returns **as examples, with no count**, since
      `closer.md` has more stops than this piece routes (a zero-box findings
      file, more than one change folder or none, a PR body that disagrees
      with the diff, isolation that did not take). A return the section does
      not name is reported with its evidence and routed by what it is. It
      routes six:
      - a red run, a conflict and a spec-changing archive, which go back
        through step 3;
      - an unticked box, widened to cover a stage row as well as a finding.
        Another agent's row goes back to that agent, or to a fresh one for
        the stage. The re-review row means a round is owed;
      - a refused push, which goes to the owner, and which also owes a round
        when the `closer`'s report lists files from the archive check;
      - a PR that stays `BLOCKED` with every required check green, which goes
        to the owner with the `closer`'s `gh pr view` output (#170).

      "Reported as before" is gone, and Impact's "its four returns" now reads
      "its returns given as examples with no count". For the `dev-writer`'s
      box above, that means both of its suggestions: drop "Five things come
      back", add the `BLOCKED` entry, and widen the unticked-box entry to
      stage rows.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:164` and `:182` — two
      sentences in "One piece is one PR" still give cherry-pick as the route
      for the branches the new rule fast-forwards.
      **Scenario:** line 164 says the `dev-writer` "does not wait for your
      cherry-pick", and the very next bullet (line 170) says "you fast-forward
      to its branch". Line 182 is the paragraph about the `dev-writer` and the
      `closer` pushing to the piece ref, and it ends "you learn each one from
      the agent's report and cherry-pick from it". Those are the two agents
      `RUNNER.md:240-243` tells the runner to fast-forward to instead. A runner
      reading this section in order meets the old route twice around the one
      correction. Line 182 is the more costly of the two: a cherry-pick of the
      `closer`'s branch refuses its merge of `main` (`RUNNER.md:253`), and a
      cherry-pick of pushed commits diverges silently (`:260-267`).
      **Measured:** `git grep -n -i -F "cherry-pick" -- .claude/agents/RUNNER.md`
      → 164 and 182 are the only hits that name the `dev-writer` or the
      `closer`. The remaining generic ones (201, 219, 331, 603) describe the
      default route and are correct. `proposal.md:345-348` records correcting
      this section's "you cherry-pick", so the section is in scope, and these
      two were missed. Severity: low.

      **Fixed** (this commit). Line 164 now says the `dev-writer` "does not
      wait for you to bring them on", and line 182 says "you learn each one
      from the agent's report and fast-forward to it, as 'Dispatching'
      says". The generic sentences stay generic. Two that addressed the
      `dev-writer` without naming it changed as well: the sample brief in
      "Dispatching", which is a `dev-writer` findings brief, now says "so the
      work can be brought onto the piece", and "What you must still ask for
      is the branch name" says the commits need "bringing onto the piece".
      Measured after the edit: `git grep -n -F "cherry-pick" --
      .claude/agents/RUNNER.md` returns no line naming the `dev-writer` or
      the `closer` as picked, only the generic route, the refusal messages,
      and the conflict rule.

- [x] **`spec-writer`** — `.claude/agents/dev-writer.md:131` — the premise this
      piece corrected in `spec-writer.md` survives in a second role file, and
      `proposal.md` does not account for it.
      **Scenario:** `dev-writer.md:131` says each agent ticks only its own row
      "so concurrent agents' cherry-picks do not conflict". `proposal.md:446-463`
      calls the same claim in `spec-writer.md` "false by the measurement
      above". It corrects that sentence because leaving it would contradict
      `RUNNER.md` "in this same piece". The `dev-writer.md` sentence does the
      same thing, and the proposal neither corrects it nor lists it. It is
      missing from Out of scope's "Who reads the stage block, and would
      change" (`proposal.md:509-524`), and the `dev-writer.md` exclusion at
      `:573-576` covers only the "runner cherry-picks your commits" sentence.
      Correcting it means editing a role file, which is outside the piece's
      authorisation, so that is an **owner** call. What the `spec-writer` can
      do is record it: in Out of scope, or as a sentence the `spec-writer.md`
      reasoning covers.
      **Measured:** `git grep -n -F "do not conflict" -- openspec/changes/171-workflow-rules/ .claude/agents/`
      → only `dev-writer.md:131`, so neither the proposal nor any findings
      file mentions it. Severity: low. The dev-writer is never one of the
      concurrent agents itself, but the sentence is the premise the owner's
      "Agents tick their own" ruling overturned.

      **Fixed** (`spec-writer`, this commit), by contracting the correction
      rather than recording it as an owner call. It is the same case as the
      `spec-writer.md` sentence. It states the same premise, which is false
      by the same measurement. It contradicts `RUNNER.md` in this piece in
      the same way. And the `spec-writer.md` correction is inside the
      authorisation because it follows from the owner's "Agents tick their
      own" ruling, a reason that covers every file stating that ruling's
      premise, not only the one holding the template. `proposal.md` gains an entry
      "`dev-writer.md` states the same premise, and its clause is corrected
      too". Only the clause "so concurrent agents' cherry-picks do not
      conflict" changes. It becomes a pointer to `spec-writer.md`'s
      stage-block paragraph, and the tick-one-row rule itself is unchanged.
      The introduction, "Anything else under `.claude/`" and Impact now name
      it. "Who reads the stage block, and would change" in the one-file-per-row
      follow-up now lists every role file that tells its agent to tick its
      own row, `dev-writer.md` included. The edit itself is the
      `dev-writer`'s.

**Clean in this range.** The four cross-file pointers the range added resolve
to headings that exist and say what they claim. From `closer.md` they are
Step 1's "in its item 3", Step 2's "The `closer`, and what comes back", and
the closing paragraph. The fourth is `spec-writer.md`'s and `README.md`'s
"Dispatching". The same holds for every intra-`RUNNER.md` pointer added:
"What a runner commits", "What a runner does", "Dispatching", "How many at
once" and step 4. "What a runner does" and "What a runner commits" do not
contradict: the second is a nested subsection that expands the first's single
line on the re-review row. The two lists of what "work" is (line 11 and
lines 37-38) differ in length but not in substance. "From the `dev-writer`'s
hand-back to the merge" still reads as one sequence: every loop in the
`closer` subsection goes back to step 3 by name. The retraction sweep for
`rebase`, `force-with-lease`, `The closer does it`, `deleted in Step 1`,
`only if it has a finding`, `Four things come back`, `has decided them all`
and `not on neighbouring ones` across `.claude/agents/`, `CLAUDE.md` and
`docs/` finds only the deliberate `rebase` uses (`RUNNER.md:13`, `:271`,
`:275-277` and `:626`) and history in `design.md`/`proposal.md`.
`README.md:241` ("never touch the same line") is literally true and is listed
as kept in `proposal.md:512-514`. `README.md:172` omits the `closer`'s Step 2
push, but "pushed by two agents only" still holds.

## Re-review `9dc235c..34fd428`

Dimension: **readability only**. Read: `git log --oneline 9dc235c..34fd428`;
`git diff 9dc235c..34fd428` for `RUNNER.md`, `closer.md`, `README.md` and
`dev-writer.md`; `RUNNER.md` in full at HEAD; `closer.md:200-330`;
`README.md:92-251` and `:436-455`; `spec-writer.md:38-57`;
`dev-writer.md:150-219`; `proposal.md:340-489` and `:590-886`;
`design.md:958-1014`; `tasks.md:180-289`. Every command quoted was run in this
tree.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:598` — "once the round's
      commits are on your HEAD" names the wrong commits by its most natural
      reading. The pre-tick grep needs the **re-reviewers' findings commits**
      on HEAD. But the round line template three lines up (`:591`) ties a
      round to its reviewed range, "round 1 `a1b2c3d..e4f5a6b` findings
      pass", so "the round's commits" reads as that range. Those commits are
      on HEAD before the round is even dispatched.
      **Scenario:** all six re-reviewers of a round have handed back, and the
      runner has not yet picked their findings commits. It reads `:598` as
      "the reviewed range is on HEAD", which it is, and runs the grep. The
      grep lists nothing. `:606-608` then says each unlisted lane "has not
      finished the round … continue that reviewer, or dispatch a fresh one
      for the lane". So the runner re-dispatches six finished lanes, or
      continues six agents that have nothing left to do. It fails closed, so
      nothing unreviewed merges, but a full round is wasted on one phrase.
      **Measured:** in this tree, `git merge-base --is-ancestor 34fd428 HEAD`
      succeeds, so this round's range is on HEAD. Yet
      `git grep -l -F "9dc235c..34fd428" -- openspec/changes/171-workflow-rules/findings/`,
      run before this file was written, printed nothing.
      `git grep -n -F "the round's commits" -- .claude/agents/` → `RUNNER.md:598`
      only. Suggested wording: "once each lane's findings commit is on your
      HEAD". Severity: low to moderate. The check is new in this range, and
      this is the one clause that says when to run it.

      **Fixed** (this commit), as suggested: step 3 now runs the check "once
      every lane's findings commit is on your HEAD". The same edit replaces
      the bare-range command with the spec-writer's two exact forms, so
      `git grep -n -F "the round's commits" -- .claude/agents/` now prints
      nothing.

- [x] **`dev-writer`** — `.claude/agents/closer.md:291-292` — "so run after
      the push it would never run for this archive at all" garden-paths into
      the opposite instruction. With no commas around "run after the push",
      the reader parses "so run after the push" as an imperative: *run it
      after the push*. The paragraph means the reverse: *if it were run after
      the push, it would never run*.
      **Scenario:** a `closer` skimming Step 3 for what to do reads the bold
      "before the push" heading, then this sentence, which tells it to "run
      after the push". The code block and the three-way list below make the
      right order recoverable. But the sentence exists only to justify the
      order, and read quickly it contradicts that order. This is the one
      ordering change the range made to `closer.md`.
      **Measured:** `git diff 9dc235c..34fd428 -- .claude/agents/closer.md`
      shows the sentence is new in this range. `closer.md:281-292` has "before
      the push" twice and "after the push" once, and the once is this clause.
      Suggested wording: "…and a re-dispatched `closer` skips it, so a check
      run after the push would never run for this archive at all." Severity:
      low.

      **Fixed** (this commit), with the suggested wording: "…and a
      re-dispatched `closer` skips it, so a check run after the push would
      never run for this archive at all."

- [x] **`dev-writer`** — `.claude/agents/dev-writer.md:131-132` — the one
      sentence this piece is authorised to write in `dev-writer.md`
      garden-paths, and it says more about its target than the target holds.
      "When ticks conflict all the same, and who resolves that, is
      [`spec-writer.md`]'s stage-block paragraph" opens like a subordinate
      clause ("When ticks conflict all the same, …"), so the reader expects
      an instruction after the comma. The verb only arrives at "is", nine
      words later, and turns the whole thing into a noun clause. "All the
      same" also reads as "identically" as easily as "nonetheless".
      Separately, `spec-writer.md:49-53` says *when* ticks conflict. It does
      not say *who resolves* them: it hands that on ("[`RUNNER.md`]'s
      "Dispatching" says who resolves that").
      **Scenario:** a `dev-writer` reads that it ticks one row and adds none,
      then this sentence. It re-parses the sentence, follows the link for
      "who resolves that", and finds a second pointer instead of the answer.
      **Measured:** `git diff 9dc235c..34fd428 -- .claude/agents/dev-writer.md`
      shows this sentence is the whole of the range's edit to the file.
      `proposal.md:604-606` specifies a pointer to a paragraph "which says
      when ticks conflict and points on to `RUNNER.md`", which is the more
      accurate description. Suggested wording: "never adding a row.
      [`spec-writer.md`](spec-writer.md)'s stage-block paragraph says when
      ticks conflict anyway, and where to find who resolves them." Severity:
      low.

      **Fixed** (this commit), with the suggested wording: "never adding a
      row. [`spec-writer.md`](spec-writer.md)'s stage-block paragraph says
      when ticks conflict anyway, and where to find who resolves them." It no
      longer claims the target says who resolves them.

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:282-319` — the new
      dirty-tree procedure comes before the paragraph that says when it is
      needed. So it opens on "a mutating reviewer's does" before the reader
      has learned that reviewers are the agents whose picks conflict.
      "Dispatching" now runs: a conflicting cherry-pick goes back to its agent
      to rebase (`:275-280`); a dirty tree cannot rebase, then four steps
      (`:282-310`); and only then "The review round meets that conflict every
      time" (`:312-319`). That last paragraph is the one that makes the
      procedure routine rather than rare. Its "that conflict" now reaches back
      past the 29-line procedure to `:275`.
      **Scenario:** a runner meets its first review-round pick conflict and
      reads the section in order. It sees the dirty-tree case as an edge case
      about some mutating reviewer. Only on reading on does it learn that
      every pick after the first in every review round hits it, and that
      reviewers are the agents with dirty trees. `design.md:960-961` gives
      the reasoning in the other order: "The conflict rule above sends the
      review round's ticks back to their agents to rebase, and a mutating
      reviewer cannot".
      **Measured:** `git diff 9dc235c..34fd428 -- .claude/agents/RUNNER.md`
      shows `:282-310` inserted between the two paragraphs that were adjacent
      at `9dc235c`. Suggested fix: move `:312-319` up to follow `:280`, so the
      procedure follows the paragraph that explains when it is needed. No
      wording changes. Severity: low. This is an ordering fix, not a missing
      rule.

      **Fixed** (this commit), as suggested. "The review round meets that
      conflict every time" now follows the conflicting-cherry-pick paragraph
      directly, and the dirty-tree procedure comes after it, so "that
      conflict" points at the paragraph just above it. The moved paragraph's
      wording is unchanged; one over-long line was rewrapped. The procedure
      also gained a closing note on untracked files and an empty patch
      (`findings/correctness.md`'s box in this round).

**The four round-1 fixes hold.** `git grep -n -i -e "things come back" -e "four
things" -e "five things" -e "four returns" -e "five returns" -- .claude/agents/`
prints nothing. `RUNNER.md:647-650` gives the returns as examples and says how to
route an unnamed one. `:674-677` widens "An unticked box" to stage rows, and
`:694-697` has the refused push untick the row and record a round.
`:707-709` is the `BLOCKED` entry. `design.md:844` is the Decisions entry.
`proposal.md` has none of "four returns", "every return" or "reported as
before", and `tasks.md:191` and `:242` record the supersessions. `RUNNER.md:164`
and `:183-184` no longer give the cherry-pick for the `dev-writer` or the
`closer`. `dev-writer.md:131` no longer says ticks "do not conflict", though see
the third box above for how its replacement reads.

**Stated once: clean apart from the ordering box.** Each of the three rules
this range added lives in one place. The returns-as-examples rule is in "The
`closer`, and what comes back". The pre-tick check is in step 3, with a
pointer-only row in "What you read" (`:81`). The dirty-tree rebase is in
"Dispatching". "What a runner does", "What a runner commits" and "Dispatching"
do not overlap in substance. "What a runner does" lists the runner's jobs and
points to "Dispatching" for the sequence. "What a runner commits" says which of
those produce content. "Dispatching" holds the sequence. The new "An unticked
box" entry points to "What a runner commits" rather than restating it, and step
3's "continue that reviewer, or dispatch a fresh one" covers a different
trigger, a missing re-review record rather than a missing tick. The parenthetical
"(cherry-pick, or fast-forward where "Dispatching" says)" now appears at
`RUNNER.md:19-20`, `:363` and `:444`, and at `README.md:131`. Each is a pointer,
not a restatement.

**Pointers: clean.** Each pointer the range added lands on a heading that
exists and says what it claims: `RUNNER.md:81` ("step 3 of 'From the
`dev-writer`'s hand-back to the merge'"), `:183-184`, `:363` and `:656`
("Dispatching"), `:564` ("below" → `:595`), `:605` ("How many at once" → the
table at `:369-373`, which names every findings file), `:676` ("What a runner
commits"), and `:677` ("step 3"). Also `closer.md:236` ("check below" →
`:281`), `README.md:131`, `:173` and `:189`, and `design.md:844`. The one
exception is `dev-writer.md:132`, which is the third box above.

**Retraction sweep: clean against the proposal.** Commands:
`git grep -n -i -F "cherry-pick"`, `-F "rebase"`, the count phrases above,
`-e "neighbour" -e "adjacent" -e "not conflict" -e "n't conflict" -e "same line"`
and `-e "writes no file" -e "no file" -e "only if it has a finding" -e "writes
nothing" -e "clean re-review" -e "finds nothing"`, each `-- .claude/agents/`.
- **"The dev-writer's commits are cherry-picked"** survives only in
  `dev-writer.md:173` and `:203`, and in `:187` ("the runner's cherry-pick"),
  which `proposal.md:779-782` and `:608` keep on purpose ("no other
  `dev-writer.md` text is edited"). The proposal names only the `:203`
  sentence, so a later sweep will find `:173` again and have to reach the
  same conclusion. That is a note, not a box. Every other hit is true of the
  agent it addresses (reviewers, `tester`, `spec-writer`), is generic, or is
  one of the kept README lines (`:120`, `:165`, `:170`, `:242`, and `:185`,
  which now names the fast-forward exceptions).
- **"The closer rebases":** every hit is the runner's prohibition (`:13`,
  `:273`), the agent-side rebase of its own local branch (`:277-303`), or the
  conflict resolver told not to rebase (`:682`).
- **"A clean re-review writes no file":** no hit. `:555` says the reverse.
- **"Four things" and "Five things":** no hit.
- **"Neighbouring lines don't conflict":** only the kept `README.md:242-243`.
  `RUNNER.md:314-316` and `spec-writer.md:49-52` say the reverse, and
  `closer.md:143` is about a squash merge, which is a different subject.

Stylistic only, no box: `README.md:99` ("…are brought onto the piece forks
from a HEAD…") and `:141` ("brings its commits on from that") read awkwardly
after the substitution. Several range-added lines run past the file's wrap
width (`README.md:99`, `:141`, `:173`; `RUNNER.md:612`). `RUNNER.md:675-676`
"goes back to that agent, continued to tick it" is compressed but parses in
context.

## Re-review round 4 `34fd428..dc1390a`

- [x] **re-review round 4 `34fd428..dc1390a`: no findings** — read `git log --oneline 34fd428..dc1390a`, `git diff 34fd428..dc1390a` for `RUNNER.md`, `closer.md` and `dev-writer.md`, `RUNNER.md:260-330`, `:465-519` and `:520-659`, `dev-writer.md:150-219`, `spec-writer.md:70-84`, and `tasks.md`'s re-review row; clean

Dimension: **readability only**, narrowed as briefed. Every command below was
run in this tree.

**The four round-2 fixes hold.** `RUNNER.md:621` now says "once every lane's
findings commit is on your HEAD", and `git grep -n -F "the round's commits" --
.claude/agents/` prints nothing. `closer.md:292` reads "so a check run after the
push would never run for this archive at all", which no longer parses as an
imperative. `dev-writer.md:131-132` uses the suggested wording and does not
claim that `spec-writer.md` says who resolves a conflict. In "Dispatching", "The
review round meets that conflict every time" (`RUNNER.md:282-289`) now comes
directly after the conflicting-cherry-pick paragraph (`:275-280`) and before the
dirty-tree procedure (`:291-326`), so "that conflict" points at the paragraph
above it.

**Stated once: clean.** The round-numbered heading and verdict-box forms are
defined in one place, step 3's brief bullet (`RUNNER.md:563-572`). The
fixed-string check that reads them is at `:626`, and "What you read" (`:81`)
repeats that command with a pointer back to step 3. The numbering rule and the
re-dispatch line rule live only in "Record the call" (`:599-609`), and the
exception for a re-run lane is stated once at `:629-631`.
`spec-writer.md:77-80` only points. `git grep -n -i -F "re-review round" --
.claude/agents/ CLAUDE.md docs/` returns only those RUNNER.md lines and
`spec-writer.md:78`. The single-quoted `git grep -n -F 'Re-review `'` and
`-i -F 're-review `'` over the same paths print nothing, so no un-numbered form
survives. I also ran the documented check itself for round 3 (single-quoted,
as `:622-623` instructs). It listed `architecture`, `correctness`,
`design-review`, `security` and `spec-test`, and not `readability`. That matches
the rule at `:629-631` and the round 4 line in `tasks.md:28`, and the command
ran without a prompt.

**Pointers: clean.** `:81` "step 3 of 'From the `dev-writer`'s hand-back to the
merge'" lands on `:517`. `:574` "(below)" lands on "Record the call" at `:599`,
`:575-576` "the check before you tick (below)" lands on `:618`, and `:629`
"How many at once" lands on the findings-file table at `:376-380`. The
`dev-writer.md` route words ("brings", "bringing", at `:173`, `:187`, `:203`,
`:214`) are neutral between cherry-pick and fast-forward, as the fast-forward
rule at `RUNNER.md:260-269` needs. The one `cherry-pick` left at
`dev-writer.md:158` covers commits made on the wrong branch, and
`findings/spec-test.md:208` and `findings/design-review.md:231` record that it
is kept on purpose.

Stylistic only, no box:
- `RUNNER.md:325` "carry this sentence in the message too" refers to itself,
  and read alone its "That refusal" has no antecedent. The note is only
  self-contained if the runner carries both sentences, from "A tree holding
  nothing else" on. "Carry these two sentences" would say so.
- `RUNNER.md:630-631` "that round's own check covers it": "that round" could be
  read as the round being checked. "the later round's own check" would remove
  the second reading.
- `RUNNER.md:600-601` "numbered from 1 in the order you write the lines: then
  what landed" has a colon followed by "then", which reads as a stutter.
- `dev-writer.md:188` runs past the file's wrap width after the substitution.

## Re-review round 10 `c4b1df5..842758b`

- [x] **re-review round 10 `c4b1df5..842758b`: no findings** — read `RUNNER.md`'s diff in the range, the tick paragraph whole (`:626-672`), "Record the call" (`:600-624`), step 3's brief bullet (`:558-589`) and the "What you read" table (`:76-84`); clean

**One procedure, in order: clean.** `:626-628` announces two checks before the
tick, and the bolded **First, the number check** (`:627`) and **Then the forms
check** (`:650`) put them in the order a runner performs them. Each check has
its own paragraph, and each ends with its failure action: "you do not tick"
(`:639`) and "do not tick" (`:670`). The number check paragraph (`:634-648`)
follows the order a runner needs: what to verify, the false-match caveat, the
failure and its repair, the empty-listing case, then why. I ran the command as
written against this piece's `tasks.md`. It printed lines 25-34, consecutive,
numbered 1-10 with each number once. The six-space indent claim holds for the
real stage block and for the example at `:621-623`.

**The new "What you read" row: accurate.** `:81` gives the same command as
`:631`, minus the path. It says what the listing decides ("run 1, 2, 3 … once
each") and points to step 3, as the forms row below it does. It sits above the
forms row, which is the order the two checks run in. Neither pattern contains a
backtick, so the double quotes prompt nothing.

Stylistic only, no box:
- `RUNNER.md:615` "so the check below would pass on it": the dev-writer is
  right that this is ambiguous now. Only the forms check passes on the rejected
  run's record, and the number check would fail on a repeated number. A reader
  cannot act wrongly on it, because the preceding clause states the action ("A
  continued run needs the new number"). "so the forms check below would pass on
  it" removes the second reading.
- `RUNNER.md:576-577` "the check before you tick (below) is a fixed-string
  search for them" has the same ambiguity, since two checks now precede the tick
  and both are `git grep -F`. "for them" points at the forms check, so this is
  style too. "the forms check before you tick (below)" matches the new names.
- `RUNNER.md:634-648` is one fifteen-line paragraph. Splitting it before "Put
  the line right" would separate the check from its repair. It reads correctly
  as it stands.
- `RUNNER.md:638` "the line numbers show which lines stand under the row"
  assumes the reader knows the row's own line number. The listing does not
  print it, because the row does not match the pattern. The round lines form
  one consecutive run of line numbers, so a reader can find them anyway.

Not readability, noted for the correctness lane: a duplicate early in the list
(1, 1, 2, 3) must become 1, 2, 3, 4 under "1, 2, 3 … in the order they stand".
So every later line's number changes too, and under `:641-642` every lane
briefed from one of those lines runs again. The text states this, but only by
applying `:641` to each line in turn. It never says outright that one
duplicate re-runs every round after it.

## Re-review round 11 `842758b..dd4fe18`

- [x] **`dev-writer`** — `.claude/agents/RUNNER.md:655-658` — the only remedy
      the number check now gives for a missing round line is the "nothing
      landed" form, and the sentence asserts that is the case: "The two rows on
      adjacent line numbers mean the line is not written yet: write it, as
      'Record the call' says for a piece where nothing landed."
      **Scenario:** a piece where commits did land. The runner dispatched round 1
      over `a1b2c3d..e4f5a6b`, but the round line never reached HEAD (written
      and not committed, or lost resolving a pick). At the tick the listing
      prints the re-review row and the `closer`'s row on adjacent line numbers.
      The bullet tells the runner which line is missing and how to write it:
      the skipped round-1 form, both ends at HEAD, "marked skipped because
      nothing landed". Written that way, the line is false. Worse, the forms
      check covers only rounds "not marked skipped" (`:673-674`), so the round
      that did run is never checked, and a lane that never recorded passes
      unnoticed. That is the failure the forms check exists to catch.
      At `842758b` the same paragraph said "a round whose line was lost gets
      its line back". This range dropped that, so the only cue left for a
      missing line points at the skip form.
      **Measured:** `git diff 842758b..dd4fe18 -- .claude/agents/RUNNER.md`
      removes "a round whose line was lost gets its line back" and adds
      `:655-658`. `git grep -n -i -F -e "lost" -e "gets its line back" --
      .claude/agents/RUNNER.md` finds no other statement of the repair.
      Suggested wording: "write what is missing: a line for each round that
      ran, as 'Record the call' gives it, or, if nothing landed, the skipped
      round 1 it describes." Severity: medium. A reader who follows the
      sentence as written records a false skip and turns off the forms check
      for a round that ran. It needs a missing line to trigger, which is rare.

      **Fixed** (`dev-writer`, this commit), following the `spec-writer`'s
      ruling in `2bf65c8` and items 1-2 of its list under
      `findings/security.md`'s round 11 box. The "at least one round line"
      bullet now says to write what is missing: each round that ran gets its
      line back with the number, range and lanes the runner's report
      recorded, and the forms check covers it like any other round; only
      where no round ran is it the skipped round 1 of "Record the call", and
      only if that paragraph's new `git diff` check passes; where the runner
      cannot tell, a range the check fails gets a round. In bold: never repair
      a missing line with the nothing-landed form over a range holding a
      commit that needs review, with your reason (the forms check skips a
      round marked skipped). "Record the call" now anchors that form's range
      at the commit the review round read and runs the check before the line
      is written, so in your scenario the check runs over a range starting
      at `a1b2c3d`, which holds `e4f5a6b`'s commits, and lists what landed
      there before any skip line is written.
      `git grep -n -i -F -e "lost" -e "gets its line back" --
      .claude/agents/RUNNER.md` now returns the new bullet, and the
      unrelated "evidence was lost" in "Dispatching". `design.md`'s
      number-check "at least one" bullet records the repair, its reason and
      the `842758b` sentence that `dd4fe18` dropped.

Dimension: **readability only**, narrowed as briefed. Read: `git diff
842758b..dd4fe18 -- .claude/agents/RUNNER.md`; `RUNNER.md:76-84`, `:556-718`
and `:750-789`; `spec-writer.md:50-90`; `closer.md:78-97`. Every command
quoted was run in this tree.

**One procedure, in order: clean apart from the box.** "Record the call"
(`:600-626`) gives the line's form, its number and the re-run rule, and the
sample follows. The tick paragraph (`:635-670`) comes next. The number check
states its three conditions as bullets, and each bullet ends with what to do
when the condition fails. The forms check follows (`:672-696`), and then the
commit, untick and archive paragraphs. The numbering rule is "one more than the
highest under the row", which is stated at `:604`, `:614-615` and `:660-661`.
All three wordings agree, and none of them conflicts with the forms check's
"below, not higher-numbered" (`:685-686`). I ran the command at `:641` against
this piece's `tasks.md`. It printed the re-review row at 24, round lines 25-35
with no gap in the line numbers, and the `closer`'s row at 36. No round number
repeats. A runner applying the three bullets to that output ticks correctly. No
line elsewhere in the file matched, so the out-of-rows caveat in bullet 1 was
not needed here. The `closer`'s row text, "- [ ] findings all ticked,
`findings/` deleted", is defined at `spec-writer.md:68`, and the pattern
`] findings all ticked` also matches it once ticked. `closer.md` never strikes
that row.

**Retired-rule sweep: clean.** `git grep -n -i -F -e consecutive -e "next
number" -e "1, 2, 3" -e "numbered from" -e "in the order" -e "next round" -e
"one is skipped" -e "that is skipped" -- .claude/agents/` returns only
`RUNNER.md:597`, "which need the next round", and `:702`, "add the next round's
line". Both refer to the next round, not to its number, and both stay true
under the new rule. `git grep -n -F -e "round <n>" -e "round number" -e "number
check"` and `-i -F -e "re-review row" -e "under the row"` over `.claude/`
return nothing that states the old rule. `closer.md:90` says "the runner's
line under the re-review row", which does not depend on the numbering.

**`spec-writer.md:78` is not misleading enough to box.** "collects one
indented line per re-review round beneath it" does not give the six-space form.
The spec-writer writes the row bare, though, and writes no round line. The next
sentence sends the reader to `RUNNER.md` for "what goes under it", and the
runner, who does write those lines, gets the exact form at `:601-602` along
with a check that fails when the form is wrong. "Per re-review round" also
still fits, since a re-run gets its own number and so its own round. Style: "one
line per round, in the form `RUNNER.md` gives" would stop a reader treating
"indented" as any indent.

Stylistic only, no box:
- `RUNNER.md:81`: the "What you read" row names two of the three conditions,
  every line between the rows is a round line and no number repeats. It leaves
  out "at least one round line". It points to step 3, which has all three.
- `RUNNER.md:613-614`: "**A lane you run" breaks onto a new line in the source
  after "round line." That is a wrap artefact and does not show in rendered
  output.
- `RUNNER.md:667-670`: "is why this check exists" gives the reason for the
  repeat bullet only. The check now has three conditions, so "why the repeat
  condition exists" would be more accurate.
- `RUNNER.md:647` and `:659` both use "gap". The first means gaps in `git
  grep`'s line numbers, which block the tick. The second means gaps in round
  numbers, which do not ("Order and gaps do not matter"). Bullet 1 says "line
  numbers" and bullet 3 says "number", so a careful reader can tell them apart.
  "no line of the file is missing" in bullet 1 would remove the overlap.
- `RUNNER.md:576-577`: "the check before you tick (below)", noted in round 10,
  still fits either check.

## Re-review round 12 `dd4fe18..1380d50`

- [ ] **`dev-writer`** — `.claude/agents/RUNNER.md:612-614` and `:676-678` —
      `<review>` is defined only from the runner's memory, "the HEAD you
      dispatched the review round from". The repair bullet then names the one
      case where that memory is gone, "such as after your report is lost",
      and says "the check decides". But the check cannot run without
      `<review>`, and nothing in `RUNNER.md` says where to read it from.
      "Rebuild the state" (`:96-129`) says memory "does not survive a
      compaction" and lists commands that recover the state, and none of
      them recovers this commit.
      **Scenario:** after a compaction, a runner finds the re-review row and
      the `closer`'s row on adjacent line numbers and cannot tell whether a
      round ran. It needs "the commit the review round read" and has to
      guess. A natural guess is the HEAD of the `dev-writer`'s hand-back
      answering the review findings, because that is the last point it
      remembers the review being "done". If that pass rejected the findings
      and added a paragraph to `design.md` (the case `design.md:322-334`
      exists for), the range starts after that commit. `git diff --name-only`
      lists only `findings/` and `tasks.md`, the check passes, and the runner
      writes round 1 as skipped. The bold rule at `:678-681` forbids exactly
      that outcome, but the runner cannot see it has broken the rule,
      because its wrong `<review>` made the check agree with it.
      **Measured:** `git grep -n -i -F "review round" --
      .claude/agents/RUNNER.md` shows no line saying how to find the review
      round's commit other than `:613`'s "the HEAD you dispatched the review
      round from". The contract has the way to find it, but only as an
      example: `proposal.md:225-227` and `design.md:306-308` say `c222c37` is
      "the parent of the review round's first findings commit". On this tree,
      `git log --diff-filter=A --format=%h%x20%p%x20%s --
      openspec/changes/171-workflow-rules/findings/` gives `f94f7b8d` with
      parent `c222c37b`, the earliest findings commit. So that derivation
      works, but the earliest commit matters: the latest findings-adding
      commit, `2259e7ff`, has parent `a284e514`.
      Suggested fix: add one sentence at `:613-614` saying how to recover
      `<review>` when the runner's report does not hold it, such as the parent
      of the earliest commit that added a file under the change folder's
      `findings/`. Keep "What you read" in step with it. Severity: medium.
      This is the lost-report case the bullet names itself, and a wrong
      `<review>` makes the check pass on exactly the false skip it exists to
      stop.

Dimension: **readability only**, narrowed as briefed. Read: `git diff
dd4fe18...1380d50 -- .claude/agents/RUNNER.md`; `RUNNER.md:60-129` and
`:490-729` at HEAD; `proposal.md:210-304`; `design.md:296-365`. Every command
quoted was run in this tree.

**The round-11 box is fixed.** The "at least one round line" bullet
(`:671-682`) now opens with "write what is missing". It restores each round
that ran from the runner's report, and it allows the skip form only where no
round ran and the check passes. It also forbids, in bold, a nothing-landed
repair over a range holding a commit that needs review. In my round-11
scenario, a runner holding its report writes round 1 over
`a1b2c3d..e4f5a6b` back, and the forms check then covers it.
`git grep -n -i -F -e "lost" -e "gets its line back"` finds the repair only at
`:673` and `:677`, plus the unrelated `:327`.

**Stated once, one procedure: clean apart from the box.** The anchor and the
two commands are stated once, in "Record the call" (`:610-628`). The "What you
read" row (`:81`) gives the same two commands and points to step 3. The
repair bullet points to "that paragraph's check" and does not restate it. The
never-repair rule sits only in the bullet, next to its reason. When a runner
has `<review>`, the procedure reads in order: the form and the range, then run
both commands, then what a failure means, which is an ordinary line naming
what landed. The runner would write ``round 1 `<review>..<HEAD>` `` marked
skipped correctly. "Where any round 1 starts" agrees with the sample's round
1, `a1b2c3d..e4f5a6b`.

Stylistic only, no box:
- `RUNNER.md:628-629`: no blank line separates "…in no round's range." from
  "**A lane you run again…**". In rendered Markdown, the re-run rule, which
  applies to every round, therefore closes the nothing-landed check's
  paragraph. It is bold and reads on its own, so no one would misapply it.
- `RUNNER.md:678`: "a round sized as above" points about 130 lines back, to
  "Size the re-review yourself", past "Record the call"'s own "sized as
  above". `proposal.md:293` says "sized as step 3 says", which is unambiguous.
- `RUNNER.md:622-625`: an archive commit that leaves `openspec/specs/`
  unchanged and the `closer`'s merge of `main` both need no review
  (`:535-539`). The check still fails on either, because both touch paths
  outside `findings/`. Normally the row is ticked before the `closer` runs, so
  the check never sees them. Only the lost-line repair after a red-CI round
  can meet them, and there it fails toward an extra round. That belongs to
  the correctness lane, not readability.
