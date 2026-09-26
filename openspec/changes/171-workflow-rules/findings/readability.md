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
