# Ship the moderation screen, inert, and amend what forbade it

## Why

**The owner reversed their own ruling.** `docs/PLAN.md` ruling 3 said *"Moderation
stays out, and the MVP ships no moderation screen"*, and a previous agent
correctly refused to build screen 07 on the strength of it and asked. The owner
has now decided the MVP ships the screen, with its controls calling nothing,
because they want to see it.

That reversal is the whole authority for this change. Nothing here is a
rediscovery of an argument the ruling settled, and nothing here claims the ruling
was wrong when it was made: the trait still exposes no moderation-publishing
method, so a moderation control still has nothing to call. What changed is the
answer to a scope question the owner owns — whether an inert screen ships — not
the answer to a contract question.

Two consequences follow, and the second is the one worth stating in a proposal
because it is where this change could have gone quietly wrong.

**A screen is not enough on its own.** The owner also asked for the Stoa-list row
counts the design shows (`31 posts · 3 unread`). Those are forbidden by a
**merged requirement** rather than by a scope note:
`stoa-navigation-view`'s *"Every number rendered is one this peer can actually
answer"* forbids a rendered count and names *"substituting another call's page
length"* explicitly, and PLAN.md's own §9.2 states that a scope note in that file
does not override a merged requirement. So the counts need a spec delta. Ruling 2
(unread counts out of the MVP) needs the same, because the design's row renders
one.

**The amendments are narrowings, and they say what would undo them.** Each keeps
the prohibition that had a permanent reason and relaxes only the one that was a
statement about what core does not compute *yet*. A reader who arrives later, sees
a placeholder count on a row, and wants to know whether it was reasoned or waved
through finds the reasoning in the requirement itself, together with the
condition — a core call answering the number — that restores the stricter form.

## What changes

- **`docs/PLAN.md` ruling 3 is amended, not deleted.** The original reasoning
  stays; the reversal is recorded beside it, with what the owner decided and what
  it costs. §6's restatement is amended in step, because both sites are reached
  from different directions and a reader who finds only one of them is a reader
  who is misinformed.
- **`stoa-navigation-view` gains an amended "Every number rendered is one this
  peer can actually answer"**, permitting a clearly-marked placeholder count in
  the MVP and keeping every permanent prohibition intact.
- **A new `moderation-view` capability** contracts what screen 07 renders and —
  more load-bearing than what it renders — what its inert controls MUST NOT
  claim. An inert destructive control is a new hazard rather than a neutral
  placeholder, and that is the part a contract is for.
- **`DModerationScreen.qml`** is added, mounted from `Main.qml` and routed to
  from a Stoa's feed, so the screen is one the owner can reach and look at.
- **`docs/PLAN.md` §9.2's case-2 list** gains an entry per placeholder shipped
  here, per the owner's standing rule.

## What this change does not do

It **builds no moderation**. No op is published, nothing is hidden, no feed is
filtered by anything this screen shows. `moderation::resolve` and the
`moderation-resolution` spec are untouched — they were built, tested and merged
before this change and they stay exactly as they were.

It also does **not** widen the `Dialectica` trait. The absence of
`publish_moderation` is what makes the controls inert, and closing that gap is a
core change with its own contract to write.

## Impact

- **Affected specs:** `stoa-navigation-view` (one requirement amended),
  `moderation-view` (added).
- **Affected code:** `dialectica-ui/src/qml/` — one new screen, `qmldir`,
  `Main.qml`, `DStoaListScreen.qml`, `DTheme.qml`, `FeedScreen.qml`.
- **Affected docs:** `docs/PLAN.md` §6, §9.2.
- **Not affected:** `dialectica/`, `dialectica-core/`. No Rust changes.
