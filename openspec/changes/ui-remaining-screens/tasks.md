# Tasks

## Stages

- [ ] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

**The spec stage did not run for this piece.** It was dispatched straight to
implementation under a speed-to-visible-app instruction, so `proposal.md` and
`design.md` were written alongside the code rather than from a spec. The row is
left unticked rather than struck: what this change renders is genuinely
unspecified, and the two `NO SPEC:` markers below are what a spec-writer would
need to pick up.

## Implementation

- [x] `DStoaListScreen` — the reference's row separator under every row, and the
      19px serif row title (`DTheme.rowTitle`, new token).
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
- [ ] **Nothing tests what this piece now ships.** The row separator and the
      `rowTitle` font are presentation; `tst_stoa_screens.qml` asserts on the
      list's behaviour rather than its geometry, and no test here covers either.
      Left unticked rather than struck: this is a real gap for the `tester`
      stage, not a stage that does not apply. design.md's "What no test here can
      see" states it.

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

**Unspecified behaviour this piece ships and nothing marks:** whether a Stoa row
carries a separator, and at what weight its title renders. Neither is in any
spec; both were taken from the design reference. A `spec-writer` picking this up
would need to decide whether the reference is contract or suggestion.
