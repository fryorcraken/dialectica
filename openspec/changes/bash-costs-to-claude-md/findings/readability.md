# Readability review — `bash-costs-to-claude-md`

Scope: readability only, per dispatch. Covers `CLAUDE.md`'s new/changed
prose and the accompanying `tasks.md` claims about it (the brief asked me to
verify the "checked line by line, added only what was missing" claim, which
is a readability-of-the-record question, not a design or correctness one).

## Findings

- [ ] **`dev-writer`** — `CLAUDE.md:80` and `CLAUDE.md:234-237` — the
      "reads outside the working directories cost a click" fact is now stated
      twice, in prose that doesn't cross-reference either occurrence.
      **Scenario:** the costs table's new row (`CLAUDE.md:80`, added by this
      change per `tasks.md`'s "Reads outside the working directories are a
      named cost" item) reads: `a path inside a working directory` (free) vs.
      `a read outside them — /tmp, the session scratchpad, an unpacked
      package` (costs a click). Twenty lines later, in the "Scratch files"
      section (also touched by this change), a new paragraph says: "It is
      also the only one you can read back without paying for it: those
      locations are outside the working directories, so **reading** a file
      you put there costs a click — as does grepping a package you unpacked
      anywhere else. Both have already happened." Same claim (`/tmp` and the
      scratchpad cost a click to read back), same two measured instances,
      stated independently in two places added in the same commit, with no
      link between them. `tasks.md`'s "fold into CLAUDE.md" section frames
      the exercise as checking `BASH-COSTS.md` against the *existing* section
      line by line to avoid re-adding a duplicate — but this duplicate was
      manufactured fresh, within `CLAUDE.md` itself, between two sections
      both edited in this diff, so that check never had a chance to catch it.
      This is exactly the failure CLAUDE.md's own "Keeping this file true"
      section warns about ("two copies drift and the wrong one gets read").
      **Severity:** low-to-moderate — not misleading today (both copies agree),
      but it is the seed of the drift the file's own policy exists to prevent,
      and it was introduced by the very change whose stated purpose was
      consolidating this material into one place.

- [ ] **`dev-writer`** — `openspec/changes/bash-costs-to-claude-md/tasks.md:54-56`
      — the claim "Everything else in `BASH-COSTS.md` was checked against
      that section and found to be a duplicate; it is not re-added" is not
      accurate — at least one non-duplicate item was dropped, not folded.
      **Scenario:** `BASH-COSTS.md`'s cost table (as it existed at `51ab7f8`)
      has the row `materialising an old version of a file to diff it → git
      diff origin/main -- <path>, one plain command`. This has no
      counterpart anywhere in the pre-#119 `CLAUDE.md` (`git show
      51ab7f8^:CLAUDE.md`, checked directly — the "How to work" section
      never mentions diffing an old version of a file) and is absent from
      the final `CLAUDE.md` on this branch (`grep -n "diff origin/main"
      CLAUDE.md` returns nothing). So this is neither a duplicate that was
      correctly skipped, nor an addition that was folded — it was simply
      lost, while the commit message and `tasks.md` both assert the fold was
      checked "line by line" against the existing section. A reader trusting
      that claim (which is exactly what the claim is for — it is the record
      that lets a future editor skip re-deriving the diff) will believe the
      table is complete when it is missing a real, useful row.
      **Severity:** moderate — the missing content itself is minor (one
      table row with an easy replacement to rediscover), but the false
      completeness claim is the more important defect: it is precisely the
      kind of unverifiable "trust me, I checked" assertion CLAUDE.md's
      "Write down only what a command cannot tell you" principle is meant to
      keep out, and here the checkable claim ("checked line by line") does
      not hold up against a checkable fact (`grep` for the dropped content).

## What was clean

- **The `.claude/` ownership rule** (`CLAUDE.md:46-63`) reads unambiguously
  against the failure it exists to prevent. It states the correct-diagnosis/
  wrong-actor distinction explicitly ("The diagnosis was correct and the
  action was still not the runner's to take") and forecloses the "it would
  help" justification by name, rather than leaving it to be inferred from a
  general "don't touch config" rule. A future agent re-deriving the same
  correct diagnosis that produced #119 is told in as many words that the
  diagnosis is not the authorisation.
- **The `settings.json` generalisation** (`CLAUDE.md:130-138`) is genuinely
  a pointer to the new section, not a second assertion of a parallel rule —
  "That file, like everything under `.claude/`, is the owner's — see
  '`.claude/` is the owner's' below before editing it." It does sit at the
  tail of a paragraph that's really about the `baseRef: head` mechanism, so
  a reader has to track "that file" back three sentences to `.claude/
  settings.json`; a little dense, but not a correctness problem and not
  worth a checkbox on its own.
- **The self-invalidating-claims check** turned up nothing new to flag. The
  file's existing conventions (naming a command instead of a number, the
  self-invalidating-pin pattern) are followed by the new prose; I did not
  find a new present-state claim in this diff that can go stale silently.
  Counts like "seven role files" and "nine-file rewrite" (`CLAUDE.md:55,63`)
  describe a specific past event (PR #119), not current repository state, so
  they don't fall under "anything a command can answer" — they can't rot,
  because they're not claims about now.
- **"Five that catch people repeatedly" → "The ones that catch people
  repeatedly"** (`CLAUDE.md:85`) is fixed as the author reports; I found no
  other hardcoded count of a list whose length could silently drift in what
  this change touched.
- **The costs-table split** (`CLAUDE.md:80-81`) correctly separates two
  previously-conflated phenomena (working-directory scope vs. cd-defeats-the-
  checker) into two rows, and each row's prose bullet below it matches what
  the row claims. The only friction is that row 80's right-hand cell ("a read
  outside them — /tmp, ...") is a *scope*, not a *shape*, where every other
  row in that column is a Bash construction (`cat`, `sed -i`, `|`, `for`) —
  a reader skimming the table for "which command shapes cost a click" could
  misread that row. Noted for awareness; not filed as its own box since it's
  a minor asymmetry rather than a wrong or misleading claim, and it's
  subsumed by the duplication finding above regardless.
