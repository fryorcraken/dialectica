# Readability review — `mvp-scope`

Scope: readability only, per dispatch. Covers `docs/PLAN.md`'s diff and the
three new `openspec/changes/mvp-scope/{proposal,design,tasks}.md` files against
CLAUDE.md's "Keeping this file true" and the review brief's five checks
(self-invalidating claims, anything a command could answer, one-claim-one-place,
findability of the four-ruling structure, and the case-2 list's add/remove
clarity).

## Findings

- [x] **`dev-writer`** — `openspec/changes/mvp-scope/proposal.md:91-108` — a
      13-row, line-number-pinned table of `Dialectica` trait methods sits one
      sentence after an instruction to read the trait "**rather than from any
      list**," and reproduces exactly the shape `design.md` (Decision 3,
      `.openspec.yaml`) names as this repo's `hand-maintained sweep lists go
      stale silently` trap and deliberately keeps out of `docs/PLAN.md`'s
      case-1 rule (`docs/PLAN.md:3536-3540`, "a list of methods in a document
      is this repo's `hand-maintained sweep lists go stale silently` trap...
      Read the current surface from the `Dialectica` trait ... rather than
      from any list here").
      **Scenario:** a future trait method is added, or `lib.rs` is reformatted
      and every downstream line number shifts. `docs/PLAN.md` stays correct
      (it carries no such table), but `proposal.md`'s archived table silently
      goes stale — the one artefact in this change that copies the anti-pattern
      is the one written to be non-editable after archiving. Currently
      accurate (verified against `dialectica/rust-lib/src/lib.rs`, all 13 line
      numbers match today), so this is a design inconsistency rather than a
      present factual error: `proposal.md` contradicts both its own sentence
      and the rest of the change's stated principle.
      Severity: low-moderate — cosmetic today, a silent-rot trap the day the
      trait changes, in a file this project's own convention treats as
      permanently archived once merged.

      **Fixed.** The table now carries a caveat immediately above it saying it
      is a snapshot, naming the trap by the same name the surrounding prose
      uses, and directing the reader to the trait for the current surface. It
      also states why the table is in `proposal.md` and deliberately not in
      `docs/PLAN.md` — it exists to show case 1 already has a substantial
      subject, which is the argument that section is making, and PLAN.md
      states the rule against such a list and keeps none.

      **Kept rather than deleted, which is a judgement call worth naming.**
      Deleting it would have removed the contradiction outright. The table is
      load-bearing for the section's argument, and `proposal.md` archives
      rather than staying live, so the staleness has a fixed endpoint: nobody
      is expected to maintain it, and the caveat tells a reader who finds it
      later not to trust it. The architecture reviewer filed the same table
      independently and asked for "at minimum... the same one-line caveat
      PLAN.md carries"; this is that, and it is recorded in both files.

- [x] **`dev-writer`** — `docs/PLAN.md:1692-1694` — the new sentence "it is
      restated because §9.2's MVP list is where a screen author looks **and it
      did not say so**" is falsified by this same diff: §9.2 (`docs/PLAN.md:3497`)
      is edited in this very change to add "moderation (§6) — **including any
      moderation screen**." The clause describes a state the diff itself
      erases.
      **Scenario:** a reader in six months opens §6, reads "§9.2 ... did not
      say so," then opens §9.2 and finds that it does say so — because this
      change made it say so. The justification for restating the fact reads
      as false the moment both edits are read together, even though each edit
      individually is correct.
      Severity: low — does not misstate current MVP scope, but is a
      self-contradicting justification clause landing in the same commit as
      the fact it justifies, which is avoidable by past-tensing it ("did not
      say so, before this change").

      **Fixed**, by past-tensing as suggested, plus one clause the suggestion
      did not include. §6 now reads: "§9.2's MVP list is where a screen author
      looks and, before the change that added this note, it did not say so.
      §9.2 now says it too; both sites are kept because each is reached from a
      different direction."

      The added second sentence is the part worth flagging: past-tensing alone
      fixes the false clause but leaves a reader who checks §9.2 wondering
      whether the duplication is an oversight. Saying both sites are
      deliberate closes that, and it is Decision 1's access-pattern argument
      (a reader arrives at one section and never scrolls to the other) applied
      at the one site where the two notes could read as redundant.

- [x] **`dev-writer`** — `docs/PLAN.md:3587-3593` (mirrored in
      `openspec/changes/mvp-scope/proposal.md:159-162`) — case-2 list entry 3,
      "No post count on a Stoa row, and no unread count," folds two different
      categories into one bullet. A Stoa post count is a genuine case-2 item
      (a control the mockup wants, core cannot currently serve, documented so
      it can be "worked off later" per Decision 3's own rationale). Unread is
      not that: ruling 2 (`docs/PLAN.md:3520-3521`, §9.1 question 8) already
      excludes it outright, by owner decision, not by a temporary core gap —
      there is nothing here for a future core change to "serve" the way
      entries 1, 2 and 4 name a call that doesn't exist yet. Folding it into
      the case-2 list mixes "excluded by decision" with "excluded because core
      can't do it yet," which is exactly the distinction ruling 4's two-case
      rule exists to keep apart (case 1 vs. case 2 is precisely "can core serve
      this or not," and ruling 2 is a third thing — "core could serve part of
      this, but the MVP declined the feature entirely").
      **Scenario:** a later reader uses the case-2 list as their "what's left
      to work off" checklist (as Decision 3 in `design.md:96-97` says it's
      for) and, on seeing "unread" there, either (a) starts work to "serve" it
      by wiring an unread field into `listThreads`, missing that ruling 2 is a
      standing scope decision requiring its own re-ruling, not a core gap to
      close — or (b) removes the whole bullet once post-count is served,
      accidentally also removing the pointer that unread was ever considered.
      Severity: low — the substance is correct and ruling 2 elsewhere states
      the exclusion unambiguously; this is a classification clarity issue at
      one list entry, present identically in both `proposal.md` and
      `docs/PLAN.md`.

      **Fixed**, and this reading is what shaped the fix rather than merely
      being satisfied by it. The correctness reviewer reached the same entry
      from the citation side (the cited passage supports only the post-count
      half); this finding names why the citation could not have supported the
      other half — unread is a third category, excluded by decision rather
      than by a missing call, so no citation to a core file could establish
      it.

      Entry 3 is now "No post count on a Stoa row" alone, in both
      `docs/PLAN.md` and `proposal.md`. Rather than dropping unread silently —
      which is scenario (b), losing the pointer that it was ever considered —
      the entry carries a following paragraph stating that unread is not on
      this list, that it belongs to ruling 2, and the operational
      consequence: every entry here is removed by core growing a call, and
      unread is not, so reopening it is a fresh scope ruling. That is aimed
      directly at scenario (a), the reader who starts wiring an `unread` field
      into `listThreads`.

- [x] **`dev-writer`** — `openspec/changes/mvp-scope/tasks.md:46` — "Every
      citation re-read against this tree rather than relayed from the
      proposal (see the report's verification list)" points at "the report,"
      which per `.claude/agents/README.md` ("What does not reach you is its
      *report*, which returns to the runner... anything an agent needs passed
      on must be in a file, not in a report") is never persisted anywhere a
      future reader — or another agent — can open.
      **Scenario:** a reviewer or a future maintainer follows this pointer
      looking for the verification list and finds nothing: no such list
      exists in `proposal.md`, `design.md`, `.openspec.yaml`, or `tasks.md`
      itself. The citation is unfollowable already, not only after this
      conversation's transcript is gone.
      Severity: low — the underlying claim (citations were re-read against
      the tree) is corroborated by spot-checking several citations in this
      review (`lib.rs:600`, `feed.rs`, spec line numbers all verified
      accurate), so nothing false is asserted; the pointer itself is simply
      dead on arrival.

      **Fixed by removing the pointer, not by writing the list it pointed
      at.** The line now says no verification list is kept, and why: the
      citations are themselves the checkable artefact — each names a file and
      a line, so a reader re-runs the check by reading them, where a separate
      list would be a second copy drifting against the citations it describes.
      The dead "see the report" reference is named in the replacement text as
      what it was, with the reason it could never have resolved.

      **Writing the list was considered and rejected.** It would have made
      the pointer followable and is the more obvious fix. It fails on this
      repo's own rule about second copies, and on the same trap as finding 1
      above: a hand-written list of thirteen-plus citations goes stale against
      the tree with nothing able to notice, which is a worse artefact than no
      list at all.

## Areas checked and found clean

- **Self-invalidating claims, `docs/PLAN.md` itself.** The living document
  never states the case-2 list's size ("five") anywhere — the list is left to
  be counted by a reader, and the instruction to "remove an entry when core
  grows to serve it" (`docs/PLAN.md:3572-3573`) is self-invalidating in the
  correct direction: an entry whose citation no longer holds is checkable
  against the tree. The "five members" / "five verified entries" language
  appears only in `design.md` (Decisions 3, four occurrences), which is
  historical rationale for a decision made when the set had five members —
  consistent with this repo's convention that `design.md` records
  decision-time state and PLAN.md carries the parts required to stay true.
  Not a violation.
- **Anything a command could answer.** No test counts, version numbers, or
  merge-status claims were introduced. The "four rulings" and "two-case rule"
  counts are the change's own fixed structure (naming what this piece is),
  not mutable repo state, so they are not the kind of count CLAUDE.md asks to
  be replaced by a command.
- **One claim, one place.** Checked by grep across the diff: the migrated
  reasoning phrases ("teaches users the app is broken," "rendered zero is
  worse than a rendered absence," the peer-local/never-published unread
  rationale) each appear exactly once in `docs/PLAN.md`, matching
  `tasks.md`'s own verification claim and `design.md`'s stated migration
  policy. The §6/§9.2 moderation-screen exclusion and the §7.2/§9.2 voting
  exclusion are pointer-plus-index pairs (full statement in one place,
  one-line pointer elsewhere), not duplicated reasoning — consistent with
  Decision 1's rejected-alternative argument.
- **Findability of the four-ruling structure.** `### 9.2 The MVP, as scoped by
  the owner` → `#### Four owner rulings, and the one consequence they share`
  → `#### Case 2: where core cannot serve what the bundle asks` is a clean,
  specific heading chain; PLAN.md has no table of contents to fall out of
  sync, and "an unwired control is a defect rather than a phase"
  (`docs/PLAN.md:3535`) is the kind of sentence a grep for "unwired" or
  "wired" would land on directly.
- **Case-2 list's add/remove criterion, in general.** The addition criterion
  (core cannot serve the control; documenting it is what makes an inert
  control acceptable) is stated once, at the rule (`docs/PLAN.md:3547-3552`),
  and the list header points back to it rather than restating it
  (`docs/PLAN.md:3570-3573`). The removal criterion is explicit and
  self-invalidating ("remove an entry when core grows to serve it... rather
  than leaving it to be disproved"). Only the one classification gap at entry
  3 (above) undercuts this.
- **Citations spot-checked against the tree**, not relayed: `lib.rs:600`
  (`publish_moderation` hypothetical), the 13-row trait-method table's line
  numbers, `stoa-navigation-view/spec.md:123`, `composer-view/spec.md:581` and
  `:638-639` — all accurate as of this tree.

Not reviewed: correctness of the scope decisions themselves, security,
architecture — out of this dimension's lane per the dispatch.
