# Correctness review — `mvp-scope`

Scope: correctness only, per dispatch. This change touches `docs/PLAN.md` and
adds `openspec/changes/mvp-scope/{proposal,design,tasks}.md`. No code diff.
Correctness here means: is every factual claim, file:line citation, and quoted
excerpt true of this tree?

## Method

Read every citation directly against the file it names (not grepped for the
quoted phrase alone, per the task's warning about line-break false negatives —
confirmed real: `DStoaListScreen.qml:390-399`'s quote genuinely spans a line
break and a scoped grep for the full phrase returns nothing even though the
citation is correct). Cross-checked the 13-method trait table against
`dialectica/rust-lib/src/lib.rs` by listing every `fn` in the trait. Diffed
`docs/PLAN.md` against `main` in full and read every hunk. Ran
`openspec validate mvp-scope --strict` (passes, `skip_specs` honoured).

## Findings

- [ ] **`dev-writer`** — `docs/PLAN.md:3587-3590` — case-2 list item 3 cites
      `DStoaListScreen.qml` as establishing "no post count on a Stoa row, **and
      no unread count**", but the cited passage only supports the post-count
      half.
      **Scenario:** `DStoaListScreen.qml:390-399` (also cited identically in
      `proposal.md`'s "Where core cannot serve" section) says: *"no call
      answers how many posts this peer holds for a Stoa, and the thread
      listing reports whether a further page exists rather than a total."*
      That passage never mentions unread state. The file's only "unread"
      occurrence (`DStoaListScreen.qml:18,21`) is the screen's own
      `readState` enum value meaning "the listing call hasn't returned yet" —
      a load-state flag, not a per-Stoa unread-post count. PLAN.md's bullet
      says the file "records that nothing computes **either**" (post count
      and unread count), which overstates what the citation shows: it
      documents one absence, not two. The actual support for an unread
      exclusion is ruling 2 / §9.1 question 8 (`docs/PLAN.md:3253-3272`),
      which is about `listThreads` gaining a per-thread `unread` field on the
      **feed** screen — a different screen and arguably a different concept
      (per-thread flag vs. a per-Stoa aggregate count) from what item 3
      attaches it to. No citation anywhere in the tree establishes that a
      per-Stoa unread *count* was ever asked for or considered — item 3
      appears to have merged two separate rulings' absences under one
      citation that only backs one of them.
      **Measured:** `grep -n -i "unread" dialectica-ui/src/qml/DStoaListScreen.qml`
      returns exactly 2 lines (18, 21), both the `readState` property
      definition/comment, zero within or near the cited 390-399 range.
      **Severity:** low — the underlying scope ruling (unread is out of the
      MVP) is independently true and well-cited elsewhere (§9.1 question 8);
      this is a misattributed citation on a list whose whole stated purpose
      (Decision 3 in `design.md`) is that "each entry cites the file that
      establishes the absence" so the set is checkable by reading. A reader
      checking item 3's citation would not find what the bullet claims it
      finds.

## Verified clean

- **All 13 trait-method line citations** in `proposal.md`'s table
  (`get_capabilities:105`, `list_threads:121` with `includeHidden` at `:109`,
  `read_thread:150`, `create_stoa:175`, `join_stoa:193`, `list_stoas:211`,
  `generate_identity_slate:231`, `keep_identity:250`, `who_am_i:268`,
  `publish_post:288`, `publish_reply:303`, `publish_vote:316`,
  `display_name:341`) resolve exactly against
  `dialectica/rust-lib/src/lib.rs` and each is the method named.
- **`lib.rs:600`** — `publish_moderation` appears exactly once in the file,
  and only inside a comment describing a hypothetical fourth pair a
  reshaping argument "would have made" — confirmed not a real method. The
  claim that the trait exposes "no moderation-publishing method at all" is
  true: the trait's 13 content methods (plus `version`/`ping`/`panic_probe`/
  `delivery_channel_exists`, which are plumbing) contain no such method.
- **Five case-2 placeholder citations**, four of five fully verified
  (the fifth, item 3, is the finding above — its post-count half is fine,
  its unread-count half is not):
  1. No score anywhere — `feed.rs:96-98`, `wire.rs:1984`, `lib.rs:310-315`,
     `VoteControl.qml:17-23` all quote-match exactly.
  2. Exactly one ordering — `feed.rs:38-41` and `lib.rs:113-116` quote-match
     exactly.
  3. No post/unread count — post-count half verified against
     `DStoaListScreen.qml:390-399` (exact quote match, confirmed spanning a
     line break as flagged); unread-count half is the finding above.
  4. History kept, no read method — `revision.rs:11-13` quote-matches
     exactly (the three-bullet list, "History is kept" is bullet three); the
     claim that `read_thread` is the only thread-read method and no other
     trait method reads prior versions is true against the 13-method table.
  5. No moderation-publishing method — verified via `lib.rs:600` above.
- **`composer-view/spec.md:638-639`** — "SHALL render that affordance
  **inert** — present but offering no action" — exact match. Correctly used
  in `design.md` Decision 4 to show a repo-wide "inert is worse than absent"
  rule would contradict a merged requirement.
- **`stoa-navigation-view/spec.md:123`** — requirement titled "Every number
  rendered is one this peer can actually answer" — exact match.
- **`composer-view/spec.md:581`** — requirement titled "The vote control
  displays no score" — exact match.
- **`FeedScreen.qml:344-350`** — exact quote match ("worse than absent: it
  reads as a working control"). `proposal.md`'s Impact section correctly
  states this is left alone and not cited as PLAN.md authority — confirmed:
  no "inert is worse than absent" generalisation appears anywhere in the
  PLAN.md diff.
- **The grep claim in `tasks.md`** — `grep -c "teaches users the app is
  broken" docs/PLAN.md` returns exactly 1 (measured), matching the claim
  that the migrated reasoning left no second copy.
- **The pre-change citation `PLAN.md:3599-3608`** in `design.md` §6a — checked
  against `git show main:docs/PLAN.md`, lines 3599-3608 on `main` are exactly
  the "Votes are the one item that contradicts a §9.1 decision" paragraph
  described. Correct use of a pre-edit line reference to describe prior state.
- **§6's pre-existing quote** — "no moderation UI and no moderation
  op-publishing path" already exists verbatim at `main:docs/PLAN.md:1686-1687`,
  confirming the proposal's claim that §6 already carried this exclusion
  before the change.
- **Spec-side cross-checks** — `content-authoring/spec.md` contracts
  publishing a vote (requirement at line 370+, "A vote names a target and a
  direction, and both directions publish"); `composer-view/spec.md` contracts
  the control (line 581); neither spec contains the string "MVP"; no spec in
  `openspec/specs/` contains the string "unread"; `moderation-resolution`'s
  spec has no publish-moderation requirement. All consistent with
  `.openspec.yaml`'s `skip_specs: true` argument.
- **No contradiction found elsewhere in the untouched parts of PLAN.md.**
  The many upvote/downvote mentions in §7.2's body (weighting formulas,
  Appendix A) are the relevance *design* that ruling 1 explicitly leaves
  intact ("the design stays; its MVP membership does not") — not stale
  staging claims requiring an edit.
- `openspec validate mvp-scope --strict` passes: "Change 'mvp-scope' is
  valid", `skip_specs` honoured, zero deltas accepted as declared.

## Not this review's lane

Readability of the §9.1-question-8-kept-but-answered structure (an item
titled "no longer a question" remaining under a heading "What could not be
decided here") is a legibility question, not a factual-correctness one — the
bullet's own text explains why it's kept rather than struck, so it is not a
false claim. Leaving for the readability/architecture reviewer if in scope.
