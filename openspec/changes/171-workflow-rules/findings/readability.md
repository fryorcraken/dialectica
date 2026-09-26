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
