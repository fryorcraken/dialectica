# Tasks

## Stages

- [ ] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

**The spec stage did not run for this piece.** It was dispatched straight to
implementation under a speed-to-visible-app instruction, so `proposal.md` and
`design.md` were written alongside the code rather than from a spec.

**A delta now exists, and the row stays unticked anyway.** `openspec validate
--strict` was failing for want of one; `specs/stoa-navigation-view/spec.md` adds
a single requirement covering the row boundary, written by the `dev-writer` to
unblock validation and argued in design.md D5b — including why it contracts the
boundary and deliberately not the type, and why `skip_specs: true` would have
been the false answer. **That is a dev's minimal delta, not the spec stage**: no
`spec-writer` has read PLAN.md against this piece or reviewed what else the
screen should contract, so the row is left for the agent that owns it. Ticking it
would claim a review that did not happen.

**The `tests` row is likewise not ticked.** Four tests now cover the row
treatment (design.md D5c), each proved able to fail by mutation, but they were
written by the `dev-writer` alongside the code. The `tester` stage — writing from
the spec, blind to the implementation — has not run.

## Implementation

- [x] `DStoaListScreen` — the reference's row separator under every row, and the
      19px serif row title (`DTheme.rowTitle`, new token). Verified against the
      reference itself rather than against the earlier description of it: screen
      08 draws `border-bottom:1px solid #d8d0bf` on all three rows, the last
      included, and sets each title at `font-size:19px`. `#d8d0bf` is
      `DTheme.rule`.
- [x] `DStoaListScreen` — the delegate's inner `RowLayout` reindented to its
      actual nesting level. Wrapping the row in a `ColumnLayout` left ~97 lines
      at the old indent, reading as though the row and the separator were
      siblings of the delegate rather than its children. **Its own commit**,
      changing no behaviour, so the separator work stayed reviewable: `git diff
      -w` over it shows comment reflow and nothing else.
- [x] `DStoaListScreen` — `objectName: "rowSeparator"` on the separator, so the
      tests key on intent rather than on a geometry a stray 1px `Rectangle`
      would also match.
- [x] `openspec/changes/ui-remaining-screens/specs/stoa-navigation-view/spec.md`
      — the missing delta. One `## ADDED` requirement, the row boundary only.
      `openspec validate ui-remaining-screens --strict` now passes.
- [x] `DStoaListScreen` — the recorded reason this screen carries no identity
      chip (`stoa-navigation-view` R13), rewritten on rebase so it no longer
      refers to a feed footer that was withdrawn.
- [x] ~~`DScreenFooter.qml`, and its mount on `FeedScreen`~~ — **withdrawn on
      rebase onto `origin/main`.** `piece/ui-navigation` (#123) merged first and
      does the same job better: its chip binds `who_am_i` rather than needing an
      opt-in nothing took, and its `DStatusBar` sits in `Main.qml` so the lamps
      reach every screen. Keeping both would put two disagreeing sets of three
      lamps on the feed at once. design.md D1 carries the full account,
      including what was lost.
- [x] ~~`qmldir` — the `# UNINSTANTIATED:` reason on `DVouchStamp`~~ — **`main`
      already carries one**, written by the same sibling piece. This branch's
      near-identical duplicate was dropped rather than merged: two records at one
      registration is a second copy that can drift.
- [x] ~~`docs/PLAN.md` §9.2 case 2 — the two unbound lamps~~ — **`main` already
      carries entries 6 and 7**, and more accurately: zone turned out to have a
      source (the membership listing), which this branch's draft denied. Dropped
      rather than merged, because it would have contradicted entry 7.

## Tests

- [x] ~~`tst_screen_footer.qml`~~ — deleted with the footer it covered. Its 12
      functions had been proved able to fail by mutation (`deliveryState: "ok"`
      turned the no-source-lamp assertion red; a constant `storageState` failed
      the both-directions assertion), so this is a withdrawal of scope rather
      than a retreat from weak tests.
- [x] ~~`tst_feed_vouch.qml`~~ — deleted with the vouch stamp it covered. It
      had found a real defect on the way (the degenerate-author shared slot),
      which is why the removal is a scope decision rather than a retreat: the
      code was correct and out of scope, not broken.
- [x] **The row treatment is covered, by count and by relation.** Four tests in
      `tst_stoa_screens.qml`: three asserting the number of boundaries against
      the number of rows (three rows, one-versus-four, and the empty list), and
      one asserting the title's type against the `DTheme` tokens rather than
      against a hardcoded 19. design.md D5c has what each covers and why three
      separator tests rather than one.
- [x] **Each proved able to fail, by the two mutations that are the two real
      defects.** Dropping the separator on the last row — the convention the
      requirement rules out — turns the two count tests red (3 rows → 2 found,
      1 row → 0 found) and correctly leaves the empty-list test green. Hoisting
      the separator out of the delegate turns **all three** red, the empty-list
      one included, which is why it is a third test rather than folded in.
      Reverting `font: DTheme.rowTitle` to `DTheme.body` — exactly the line this
      piece changed — fails the type test with `Actual 15, Expected 19`.
- [x] `tst_render_probe.qml` **does not already cover this**, checked rather
      than assumed. It probes `DStoaListScreen`, but asserts only that the
      content area is not one flat colour; a screen with no separator at all
      still paints a title, an address, an identicon and two buttons, so it
      passes that probe unchanged.

## Not done, and why

- [ ] ~~Screen 07, the moderation screen~~ — **refused**, not deferred.
      `docs/PLAN.md` ruling 3 (merged in `a2375d4`, two commits before this
      branch) says the MVP ships no moderation screen, and it is blocked at the
      contract: the `Dialectica` trait exposes no moderation-publishing method.
      Verified against `dialectica/rust-lib/src/lib.rs`, not against the
      document. Reversing this is the owner's call and takes one sentence.
- [ ] ~~The vouch stamp on a feed row~~ — **built, then removed on scope.**
      PLAN.md §7.3: vouching is "scheduled after §7.2's interim ordering ships",
      and §7.2 is out of the MVP by ruling 1. Caught by reading
      `piece/ui-navigation`'s concurrent diff, which declares the same component
      UNINSTANTIATED for the same reason — not by any gate. design.md D3 carries
      the full account.
- [ ] ~~Mocked post and unread counts on the Stoa row~~ — **refused.**
      `stoa-navigation-view` forbids the first ("Every number rendered is one
      this peer can actually answer", including substituting another call's page
      length) and ruling 2 excludes the second from the MVP. PLAN.md states that
      a scope note does not override a merged requirement.

## `NO SPEC:` markers left behind

**None remain in the tree.** Both markers this change left behind went with the
footer, and they are listed here because a marker that vanishes in a rebase is
unspecified behaviour that stops being visible rather than stops existing:

- ~~`tst_screen_footer.qml`'s
  `test_the_bool_property_coerces_and_is_not_itself_the_guard`~~ — deleted with
  the file. It pinned a **measured** fact worth not re-deriving: a QML `bool`
  property coerces `"true"` to `true` and `0` to `false`, so declaring a
  property `bool` is *not* a guard against a raw probe field being bound to it.
  Whatever normalises the probe is. This applies to navigation's
  `DIdentityChip.hasIdentity` binding exactly as it did to the footer's.
- ~~The footer's arrangement~~ — moot; there is no footer. **But the underlying
  gap is still open and now belongs to `Main.qml`:** nothing contracts whether
  an unbound lamp must be orange rather than absent. `DStatusBar` carries its
  own `NO SPEC:` on that default, and navigation's bar is now a second consumer
  of it.

**One marker is now live in the tree**, and it is the half of the row treatment
the delta deliberately does not contract:

- `tst_stoa_screens.qml`'s
  `test_a_row_title_is_set_apart_from_body_prose_without_reaching_a_heading` —
  **the type a row title is set at is contracted nowhere.**
  `stoa-navigation-view`'s Purpose puts "colours, type, metrics, the mark"
  outside that capability by name, so a requirement pinning the size would
  contradict the capability's own scoping sentence. The test therefore pins the
  **relation** the reference establishes — `rowTitle` above `body`, below
  `heading` — rather than the literal 19. design.md D5b has the argument.

  What a `spec-writer` still has to decide: whether the design reference is
  contract or suggestion, and if contract, which capability owns the view's type
  scale — because `stoa-navigation-view` says in its own text that it does not.

**The separator is no longer unspecified.** It is contracted by this change's
delta, for the reason in design.md D5b: it resolves which Stoa a rendered address
belongs to, on a screen whose capability is about not misattributing
peer-supplied strings.
