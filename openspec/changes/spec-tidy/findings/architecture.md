# architecture review — spec-tidy

Scope: `git diff d57edaf..HEAD` (equivalently `origin/main...HEAD`), i.e. this
piece's own commits (excludes #180's `position-and-index` content, which
arrived only through the `cb46319d` merge and is not this piece's).

## What was checked

**Capability boundary — `view-navigation` as single home for screen entry/exit.**
Compared the `REMOVED` blocks in `thread-view` and `moderation-view` against the
`MODIFIED`/`ADDED` blocks in `view-navigation` line by line:

- `thread-view`'s "The thread screen is reached from the feed and can be left
  again" (live, pre-archive) folds cleanly into `view-navigation`'s existing
  "A thread is opened from a feed row and can be left": every paragraph and
  every scenario is accounted for — either moved verbatim (way-out-survives-a-
  refused-read, no-thread-screen-before-chosen, no-thread-identifier-of-its-own),
  folded as an extra `AND` onto "Acting on a feed row opens its thread" (the
  thread-read naming the row's thread id and Stoa), or correctly dropped as a
  duplicate of an already-live scenario ("The feed is reachable again from the
  thread" vs. the pre-existing, unchanged "The feed is reached again from the
  thread" — same outcome, confirmed by direct comparison).
- `moderation-view`'s "The screen is reachable, and leaving it returns where the
  user was" moves to `view-navigation` as `ADDED`, text and all three scenarios
  intact, with only the documented renamings ("the screen" → "the moderation
  screen", self-reference → "this capability") plus one added sentence handing
  what the screen renders back to `moderation-view`. Confirmed the `ADDED`
  heading has no prior live match (`git grep` came up empty before the delta
  applies) — this is a genuine addition, not a silent rename collision.
- Both moves use the pattern `docs/OPENSPEC-ARCHIVE.md` prescribes (`REMOVED`
  from the source capability, `ADDED`/folded-`MODIFIED` at the destination),
  applied consistently rather than picking a shortcut for one and not the
  other.
- No leftover overlap found between `thread-view`/`moderation-view` and
  `view-navigation` after the fold: each now states only what it renders,
  citing `view-navigation` for the route. No overlap surfaced between
  `feed-view` and `feed-read` either (`feed-view`'s corrected paragraph now
  correctly states what `feed-read` contracts about a feed row carrying no
  time, rather than the previous garbled "not widened by this change").

**Mechanics — requirement-name stability.** Spot-checked heading matches for
every `MODIFIED` delta against the live spec it targets
(`feed-view`, `generated-names` ×2, `moderation-resolution`, `stoa-genesis`,
plus the `view-navigation` fold): all match the live `### Requirement:` line
character for character. No `REMOVED`+`ADDED` pair was used where a `MODIFIED`
would do — the two capability moves are the only `REMOVED`/`ADDED` pairs in the
whole change, and both are genuine capability-to-capability moves, not a
same-capability rename dressed up as remove-and-add.

**The `cb46319d` merge resolution** (`git show --remerge-diff cb46319d --
openspec/specs/identity-onboarding/spec.md`), explicitly flagged as in scope:
clean conflict resolution. Both sides' requirements ("A peer with no master key
can obtain one without naming a Stoa" from this piece's `first-run-identity`
archive, and "A malformed `index` in a keep request is refused with a message
naming `index`" from #180) are kept with no textual change to either, ordered
as the merge commit message states. They govern different operations and do
not contradict.

**Coverage.** Confirmed via `git log` that all five changes proposal.md claims
as archived (`sqlite-projection`, `first-run-identity`, `ui-thread-view`,
`ui-remaining-screens`, `moderation-screen`) have corresponding archive
folders and correction/archive commit pairs on the branch, matching the SHAs
proposal.md and design.md cite. Did not re-derive the full list of live
capabilities against every shipped feature to check for a capability with no
live spec at all — see Noted limit below.

## Noted limit

I did not finish an exhaustive independent enumeration of `openspec/specs/`
against everything shipped on `main`, to positively confirm no other
shipped-but-unarchived capability exists beyond the five this piece names.
What I did check (the git-log cross-reference of the five named archives, and
the shape of each promotion) found nothing wrong, and `relevance-votes`
staying unarchived is explicit and matches the owner's instruction as recorded
in `design.md` §2. A full sweep for a sixth unarchived change was cut short by
time; if one is wanted, `git log --oneline main` cross-referenced against
`openspec/changes/` (excluding `archive/` and `spec-tidy/`) is the check.

## Verdict

- [x] **none** — capability boundaries, coverage of the five named archives,
      and delta mechanics (stable requirement names, `MODIFIED` used wherever
      it would do, `REMOVED`+`ADDED` reserved for genuine capability moves)
      are all clean. See "Noted limit" above for the one check left
      unfinished, which found nothing but wasn't run to completion.
