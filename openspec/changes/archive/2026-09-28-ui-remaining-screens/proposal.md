# Close the visual gap between the shipped screens and the design bundle

## Why

**The owner has been waiting two days to look at this UI, and the priority is a
visible, launchable app that matches the design bundle.** The instruction that
governs this piece, verbatim: "DATA CAN BE MOCKED; BUTTONS CAN BE DEAD; as long
as it's noted in the PLAN.md", and "I don't give a shit about the text in the
bundle, I want the DESIGN AKA the SCREEN AKA THE UI to be honoured."

So the deliverable is the **visual design** — layout, spacing, borders, type,
colour — and not the bundle's copy strings.

Measured against this tree, the gap is smaller than the brief assumed, and that
is worth stating because it changes what this piece is. `FeedScreen.qml` already
carries vote arrows, a composer, the posting gate and pagination;
`PostHeader.qml` already accepts `vouched` and `isModerator`; `FlatButton.qml`
already defines `destructive`, `destructive-outline` and `secondary-micro` — the
three kinds the bundle's moderation screen uses, added in anticipation of it.

What was actually missing was **the screen footer**. `DStatusBar` and
`DIdentityChip` were registered in `qmldir`, carried 27 passing test functions
between them, and were instantiated by no screen. So the components existed,
were tested, and rendered nowhere — the same class of defect
`piece/ui-navigation` found for the onboarding screen, seen from the component
side rather than the routing side.

**That gap is closed, but by `piece/ui-navigation` rather than by this piece.**
It merged first, mounting `DIdentityChip` on the feed bound to `who_am_i` and
`DStatusBar` once in `Main.qml` as chrome on every screen. This branch's own
`DScreenFooter` was withdrawn on rebase rather than merged alongside it, because
keeping both would put two disagreeing sets of three lamps on the feed at once.
design.md D1 records what was decided and what was lost.

`DVouchStamp` is in the same state and is **not** in that set: it is unmounted
on purpose, by a scope ruling. Distinguishing the two cases is what this change
got wrong first and then fixed; design.md D3 records it.

## What Changes

- **The per-Stoa row treatment on the Stoa list** — a hairline separator under
  every row including the last, and a 19px serif title on a new
  `DTheme.rowTitle` token. This is what the piece ships.
- **A recorded reason that the Stoa list carries no identity chip.**
  `stoa-navigation-view` R13 forbids this screen raising identity at all, and
  re-adding a footer here is the obvious next idea that no gate catches.
- **No post count and no unread count** on the row, for the reason under "What
  this is not" below.
- ~~**A screen footer, mounted on the feed**~~ — built, then **withdrawn on
  rebase** as superseded by `piece/ui-navigation`. design.md D1.
- ~~**No vouch stamp**~~ — one was built here and removed on scope (`PLAN.md`
  §7.3 schedules vouching *after* §7.2's ordering, and §7.2 is out of the MVP by
  ruling 1). The `# UNINSTANTIATED:` record that states this is now `main`'s,
  written by the same sibling piece; this branch's duplicate was dropped.
  design.md D3 carries the account, including that it was caught by reading that
  branch's concurrent diff rather than by any gate.

## What this is not

### It does not ship a moderation screen, and that is not a scope choice this piece made

The brief asked for screen 07. **`docs/PLAN.md` ruling 3 forbids it**, in a
commit merged two commits before this branch was cut (`a2375d4`, PR #95):
"Moderation stays out, and the MVP ships no moderation screen." The ruling is
recorded at §6 *and* §9.2 deliberately, and §6 anticipates this exact situation:
"It is blocked at the contract, not merely deferred, which is the part worth
knowing before anyone treats the screen as a phase somebody skipped."

**Verified against the trait rather than the document** — `grep "    fn "
dialectica/rust-lib/src/lib.rs` lists `publish_post`, `publish_reply` and
`publish_vote`, and no moderation-publishing method at all. So a moderation
screen has nothing to call.

This is the one place "mock it and move on" does not resolve the question,
because the thing blocking it is an owner ruling rather than a missing call, and
PLAN.md states outright that a scope note "does not override" a merged
requirement. Building the screen would overwrite a decision recorded one commit
ago by the same owner who set this piece's priority. **Reported rather than
resolved by this piece**, since reversing ruling 3 is the owner's to do and takes
one sentence if that is what is wanted.

### It renders no post count and no unread count

Both are forbidden by things that outrank a scope note, and both would otherwise
be the obvious thing to mock:

- `stoa-navigation-view`'s **"Every number rendered is one this peer can
  actually answer"** forbids a rendered count on the Stoa list, "including
  substituting another call's page length". PLAN.md's case 2 list names this
  entry and says the narrowing governs it: "the position is left empty rather
  than filled".
- **Unread counts are out of the MVP by ruling 2**, and PLAN.md is explicit that
  this one is excluded *by decision* rather than by a missing call, so it is not
  worked off by a core change.

`DStoaListScreen.qml:390-399` already records the reasoning in the position the
count would occupy. The row's **visual treatment** is this piece's business and
is honoured; the number is not.

### It does not touch `Main.qml` or the thread screen

`piece/ui-navigation` owns `Main.qml` and mounting; `piece/ui-thread-view` owns
the thread screen.

**The no-overlap claim this section originally made was wrong, and it is left
here corrected rather than deleted, because the way it was wrong is the useful
part.** It read `ui-navigation`'s proposal — which scopes itself to reachability,
says it "does not specify what any newly-reachable screen renders", and names
only `Main.qml`, `qmldir`, tests and PLAN.md in its Impact — and concluded
`FeedScreen.qml` was this piece's alone. What merged as #123 edited
`FeedScreen.qml` substantially: the identity report, the thread-open affordance,
and the footer chip. **A sibling's Impact list is a statement of intent at
writing time, not a boundary either piece is held to**, and treating it as one
is what let two pieces build the same footer twice.

## Impact

After the rebase onto `origin/main`, this piece touches three files:

- `dialectica-ui/src/qml/DStoaListScreen.qml` — the row treatment, and the
  recorded reason this screen carries no identity chip.
- `dialectica-ui/src/qml/DTheme.qml` — one new token, `rowTitle`.
- `openspec/changes/ui-remaining-screens/` — this change's own documents.

Withdrawn on rebase, each because `main` already carries the same thing or a
better one — design.md D1, D2, D3 and D6 have the arguments:

- ~~`DScreenFooter.qml`, `tst_screen_footer.qml`~~ — superseded by
  `piece/ui-navigation`'s chip and shared status bar.
- ~~`FeedScreen.qml`~~ — main's version taken wholesale.
- ~~`qmldir`~~ — main's `# UNINSTANTIATED:` record for `DVouchStamp` kept; this
  branch's duplicate dropped.
- ~~`docs/PLAN.md`~~ — main's case 2 entries 6 and 7 kept; this branch's entry
  dropped, since it wrongly denied that zone has a source.
- ~~`tst_composer_claims.qml`~~ — the lamp-label exclusion reverted, because with
  no `DStatusBar` in the sweep corpus it excuses nothing and is a standing hole.

No core change. No trait method is added, and no wire shape moves.
