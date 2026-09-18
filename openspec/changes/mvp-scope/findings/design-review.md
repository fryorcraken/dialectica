# Design review — `mvp-scope`

Scope: does the code (here, `docs/PLAN.md`) take the decisions recorded in
`design.md`, and were the decisions worth recording recorded. `design.md`
exists (this is a documentation-only piece; no new format, security boundary,
dependency or migration/performance complexity — but the writer chose to write
one anyway, appropriately, given how much reasoning this piece moves).

## Summary

The four rulings are recorded, each landed at the PLAN.md location `design.md`
says it would, and I could not find either rejected framing of ruling 4 ("inert
is worse than absent" or "inert acceptable but false claims not") anywhere in
`design.md`, `proposal.md`, `.openspec.yaml`, or the `docs/PLAN.md` diff —
Decision 4 states the rejection directly and the diff matches it. The three
named migrations (§9.2's "contradicts a §9.1 decision" paragraph, the "rendered
zero" reasoning, and §9.1 question 8's rationale) each land under the Decision
they belong to, and I verified against `origin/main`'s copy of PLAN.md that the
migrated passages existed there verbatim and are gone (struck, in the two
migrated cases; left in place, in the unread case, per Decision 2's argument
that it is reasoning about something not yet built). Decision 2's departure
from strike-and-point is argued and, on inspection of `docs/PLAN.md:3253-3271`,
correctly executed — the entry is rewritten, not struck, exactly as the
Decision describes.

Two gaps below are worth recording, and one is worth fixing before merge.

## Findings

- [ ] **`dev-writer`** — `openspec/changes/mvp-scope/design.md` (no Decision
      entry) — three runner calls made during this piece are not recorded as
      Decisions, and at least one of them meets this repo's own bar for what
      belongs here.
      **Scenario:** a future reader of `design.md` sees Decision 3 argue against
      a grep-able `// CASE 2:` marker (a mechanism for enforcing the case-2 list
      condition) but has no record that a *different* proposed mechanism — a CI
      gate checking for "worse than absent"-style phrasing intended to keep
      ruling 4's argument from being generalised into a repo rule — was also
      proposed and dropped as over-engineering for MVP scope. This is exactly
      the shape CLAUDE.md and `.claude/agents/README.md` ask to be recorded:
      "write down only what a command cannot tell you" and "a reviewer that
      spends an afternoon proving an approach impossible has produced a result
      worth as much as the review, and unwritten the next agent spends the same
      afternoon." A future agent tempted to add such a gate when case-2's list
      grows past five members re-derives the same rejection from scratch. The
      other two calls (not sweeping `FeedScreen.qml:344-350`'s comment, and not
      queueing a spec change to relax `stoa-navigation-view`'s and
      `composer-view`'s narrowing requirements) are lower-stakes: the first is
      stated in `proposal.md`'s Impact section ("left alone... a separate piece
      deals with the comment itself") though not in `design.md`'s Decisions,
      and the second is implicit in Decision 5 and the Risks section ("relaxing
      them is a spec change" / "PLAN.md cannot enforce this") — close enough to
      recorded that I would not block on those two alone. The CI-gate
      rejection is the one with no trace anywhere in the change.

- [ ] **`dev-writer`** — `openspec/changes/mvp-scope/proposal.md:206-210` vs.
      `design.md` (no Decision entry) — the decision to leave
      `FeedScreen.qml:344-350` untouched is stated in `proposal.md`'s Impact
      section but not recorded under Decisions in `design.md`, even though it
      is the site Decision 4 spends the most space arguing about (whether the
      comment's "worse than absent" claim generalises). Impact is the wrong
      section for this — it is stated as a fact about what changed, not as a
      choice with an alternative. The alternative that was available and
      rejected (sweep or annotate the comment to point at Decision 4, so a
      reader hitting `FeedScreen.qml` sees the scope note instead of an
      unqualified local claim) is not named anywhere. This is a minor gap,
      since the file is genuinely untouched (verified: `git diff
      origin/main...HEAD -- dialectica-ui/src/qml/FeedScreen.qml` is empty),
      but "a separate piece deals with the comment itself" in `proposal.md` is
      itself a decision (defer, don't touch) with no alternatives recorded.

## What is in good shape, stated so it is not re-litigated

- **Ruling 4's rejected framings are absent, in both directions.** Grepped
  `design.md`, `proposal.md`, `.openspec.yaml` and the full `docs/PLAN.md` diff
  for "inert is worse than absent," "worse than absent" as a generalised claim,
  and any variant of "false claims are not acceptable." The only occurrences of
  "worse than absent" are (a) `FeedScreen.qml`'s own untouched comment, quoted
  and explicitly scoped as local by Decision 4 and by `proposal.md`'s Impact
  section, and (b) the `docs/PLAN.md` case-2 list's citation of
  `VoteControl.qml`'s comment, which is a pointer to code, not a restated repo
  rule. Decision 4's actual recorded rule is the owner's own two-case
  contract-authority framing ("if core exists, the control MUST be wired" /
  "inert or placeholder acceptable if documented"), which is what
  `docs/PLAN.md`'s §9.2 new section states. Neither rejected intermediate
  framing survives.
- **Decision 2's departure from strike-and-point is justified and executed
  correctly.** The argument — an open question that stops being a question
  is not the same event as a claim becoming false, so PLAN's strike convention
  (reserved for false claims) doesn't fit — is sound on its own terms and
  consistent with CLAUDE.md's "Keeping this file true," which ties striking to
  superseded correctness, not to closure. Checked `docs/PLAN.md:3253-3271`
  against `origin/main`'s copy: the bullet is rewritten in place, its original
  reasoning kept verbatim, and only the "what would decide it" clause is
  struck — exactly what Decision 2 and `tasks.md` both claim.
- **The three named migrations landed cleanly, each under its Decision, with
  no second copy left in PLAN.md.** Verified against `origin/main`:
  - §9.2's "Votes are the one item that contradicts a §9.1 decision" paragraph
    existed verbatim in `origin/main`'s PLAN.md (lines 3599-3608) and is now
    struck in the working tree, with the reasoning moved to Decision 6a. The
    phrase "teaches users the app is broken" now occurs exactly once in
    `docs/PLAN.md` (inside the strike-and-point passage, quoting rather than
    restating), matching `tasks.md`'s own verification claim.
  - The "rendered zero is worse than a rendered absence" reasoning existed in
    `origin/main`'s closing paragraph and is now a pointer ("is in the
    `mvp-scope` change's `design.md`") with the full argument moved to
    Decision 6b. The one remaining `docs/PLAN.md` occurrence of "worse than a
    rendered absence" (line 3581, in the case-2 list) is a citation of
    `VoteControl.qml`'s own comment, not a restatement of the design.md
    argument — this is a citation, not a duplicate, and is fine.
  - §9.1 question 8's "why unread was left undecided" rationale is correctly
    *not* fully migrated — Decision 2 argues, correctly, that this reasoning is
    about something still not built and therefore belongs in PLAN.md per
    `.claude/agents/README.md`'s own rule ("Reasoning migrates" only applies to
    reasoning attached to a landed decision). Decision 6c states this
    explicitly and only draws the closure argument into design.md, leaving the
    "why unread needs peer-local state" reasoning where it was. This is the
    correct reading of the migration rule, not a shortcut.
- **`.openspec.yaml`'s `skip_specs: true` argument is sound**, and matches the
  actual capabilities touched: `content-authoring`, `composer-view`,
  `moderation-resolution`, `feed-view`, `stoa-navigation-view` are all
  confirmed untouched by `git diff origin/main...HEAD --stat` (only
  `docs/PLAN.md` and the change folder itself changed).
