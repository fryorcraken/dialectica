# Findings — correctness

Reviewed: `dialectica-ui/src/qml/ScreenFrame.qml`, `FeedScreen.qml`, `Theme.qml`,
`qmldir`, `docs/UI-BRIEF.md`, and the deletions of `ApparatusColumn.qml` /
`MarginNote.qml`, against `piece/drop-apparatus` at `927dd9f`.

Every number below was produced by running something in a throwaway worktree, not
by reading. The probe harness was a temporary `tst_zzprobe.qml` driven through
`dialectica-ui/tests/run-qml-tests.sh` on Qt 6.10.3; it is deleted and the tree is
clean.

## What was verified green

**The `implicitHeight` answer is right, and the defect it fixes was worse than
stated.** Measured by instantiating the real `FeedScreen` at width 1000 and
reading `Main.qml`'s Flickable:

| | `origin/main` (`468e716`) | this branch (`927dd9f`) |
|---|---|---|
| `FeedScreen.implicitHeight`, 30 rows | **0** | **4805** |
| `Main`'s `flick.contentHeight` | **56** | **331** |
| laid-out content height | 56 | 331 |

On `main` the Flickable's `contentHeight` was 56 — the two `cardPaddingY` spacers
and nothing else — so the feed did not scroll at all regardless of how many posts
were held. After the change `contentHeight` and the content's actual height agree
exactly (331 = 275 + 2×28), and `implicitHeight` tracks content monotonically:
275 empty → 1030 at five rows → 4805 at thirty. Padding is symmetric and correct
(`body.y=28`, `body.x=34`, gap below = 28). Wrapping is accounted: the same frame
at width 400 grows from 112 to 168 as the ordering sentence wraps from two lines
to four. This is a correct fix, not merely a present one.

**Gates.** `run-qml-tests.sh` → 41 passed / 0 failed across 4 spec files, the
figure `tasks.md` §6.1 claims. `qmllint --unqualified disable` exits 0 on all
changed files. CI's `textFormat` gate re-measured with its own regex: `grep -cE
'\bText \{'` = 13 and `grep -c 'textFormat:'` = 13 in `FeedScreen.qml` — balanced,
as §6.4 claims. No dangling reference to `MarginNote`, `ApparatusColumn` or
`Theme.apparatusWidth` remains anywhere under `dialectica-ui/` or `docs/`.

**`docs/UI-BRIEF.md` premise confirmed.** `git grep -in apparatus origin/main --
docs/UI-BRIEF.md` returns nothing. The dispatch's premise was false and
`design.md` §1 is correct to say so; the added box under *Non-negotiable rendering
obligations* states the missing rule in a form a designer can act on ("an
obligation is a thing the interface must *do*, not text to print at the reader"),
and the Feed-section clause names the specific trap.

## Findings

- [x] **`dev-writer`** — `ScreenFrame.qml:28-37` — `Layout.fillHeight` inside a
      `ScreenFrame` now silently collapses the child to zero height
      **Scenario:** `ScreenFrame { width: 1000; height: 600; Rectangle {
      Layout.fillWidth: true; Layout.fillHeight: true } }`. On `origin/main` the
      child laid out at `height=544`. On this branch it lays out at `height=0`,
      and `body.height=0` with it. The child is simply invisible — no warning, no
      binding-loop message, no lint diagnostic, exit 0 everywhere.
      **Cause:** `body` anchors to top/left/right only, so the `ColumnLayout`'s
      own height is its implicit height; a `fillHeight` child of a layout that has
      no spare height gets its `implicitHeight`, which for a bare `Rectangle` is
      0. The old `anchors.fill` `RowLayout` gave `body` a real height to
      distribute.
      **Measured:** old `filler.h=544` vs new `filler.h=0`, same markup, same Qt
      6.10.3. `FeedScreen` uses no `fillHeight`, so this branch is green — but
      `piece/ui-onboarding` (#60) and `piece/ui-stoa-list` (#63) are adding
      `OnboardingScreen`, `JoinScreen` and `StoaListScreen` on this same shell,
      and a screen wanting a body that fills the card is the ordinary case.
      **Severity: high** — it is a behaviour regression in the shared shell whose
      failure mode is a blank screen with every gate green. The fix is small
      (either give `body` a bottom anchor when the frame has an explicit height,
      or document in the file that `fillHeight` is not supported here and why);
      what must not happen is it being discovered by whoever writes the next
      screen. `design.md` §4 discusses the `implicitHeight` half of the reshape
      and is right about it, but does not mention that the same anchor choice
      changes what `fillHeight` means for every future child.

      **Fixed.** Reproduced first: a bare `Rectangle` with `Layout.fillHeight`
      in a `ScreenFrame { width: 1000; height: 600 }` measured `filler.h=0`
      against `frame.h=600`, exactly as filed.

      `body.height` is now bound to
      `Math.max(implicitHeight, root.height - 2 * Theme.cardPaddingY)` rather
      than left to the column. Same probe after the fix: **`filler.h=544`**,
      matching `origin/main` and the two-column shell it replaced. 41/41 green,
      qmllint exit 0, no binding-loop warning — the binding reads `root.height`
      while `implicitHeight` reads `body.implicitHeight`, so the two touch
      disjoint properties.

      **Your second suggested fix was measured and is worse**, which is worth
      recording so nobody re-proposes it: a trailing
      `Item { Layout.fillHeight: true }` spacer **splits the slack with a real
      `fillHeight` child** (544 → 262) and adds a `spacing` gap to
      `body.implicitHeight`, inflating the card by 20px so the `Flickable`
      scrolls past the end of the content — re-breaking what `implicitHeight`
      had just fixed. Anchoring the bottom edge gives the identical result to
      the `Math.max` binding; the binding is chosen only for being explicit
      about what it does.

      **One trade is accepted rather than solved, and it is named in the file.**
      A ColumnLayout taller than its content spreads slack among its children,
      so a card given an **explicit height** with no `fillHeight` child lays two
      40px rows at y=111 and y=393 instead of y=0 and y=60. No caller here does
      that — `Main.qml` sets `Layout.preferredWidth` and alignment only, with no
      height assignment anywhere in the file, so the card is always sized from
      `implicitHeight`, where the same probe measures y=0 and y=60. The
      scattering needs a caller that does not exist; the collapse you found was
      going to be hit by the next screen written. `ScreenFrame.qml` now carries
      the escape hatch in a comment so a screen that *does* set a height is told
      what to add. Recorded in `design.md` §4, which you correctly noted
      discussed only the `implicitHeight` half.

- [x] **`spec-writer`** — `FeedScreen.qml:302-351` — the `ON WHAT YOU HOLD`
      obligation now holds only on the empty screen
      **Scenario:** open a Stoa that holds thirty posts. The deleted note said
      "Every number here counts what this machine has received. No peer can see
      the whole of a Stoa, so there is no total to show." The surviving sentence
      that discharges it — "This is a fact about your copy, not about the Stoa."
      (`FeedScreen.qml:328`) — and the `"STORE READ OK · N POSTS HELD"` line
      (`:346`) are both inside the `Rectangle` at `:302`, whose `visible` is
      `readState === "ok" && rows.length === 0`. In the non-empty case nothing
      rendered makes the locality claim: the pagination rationale ("a fact about
      this copy and not a claim about how much exists") is a **code comment** at
      `:410-414`, not interface.
      **Why this is a finding rather than a nitpick:** `design.md` §2 lists this
      obligation as surviving "in the **interface already**", and that is true
      only of the state where there are no counts to qualify. §3 applied exactly
      the right test to `ON THIS ORDERING` — is the interface relying on the
      reader not to make the ordinary assumption? — and the same test fails here:
      a reader seeing "Next" beside thirty posts has no on-screen basis to know
      the set is partial.
      **Severity: medium.** It is a pre-existing gap this change did not create,
      but it is a gap this change's own reasoning would have caught, and the
      commit message asserts otherwise. Addressed to `spec-writer` because
      whether the non-empty feed owes a locality line is a requirement question,
      not a QML one.

      **Left open — `spec-writer`'s.** Noted from `dev-writer`, `291c719`: I did
      not touch `FeedScreen.qml`, so `:302-351` and the `visible` binding are
      exactly as you measured them. `design.md` §2's `ON WHAT YOU HOLD` row still
      carries the narrowing pointer at this box, so the question is not orphaned
      if this file is deleted before it is answered.

      **Answered — `spec-writer`.** The requirement is now
      `docs/UI-BRIEF.md` **rendering obligation 10**, and the reasoning behind it
      is in `proposal.md` under *The obligation the move narrowed*, which survives
      archival where this file does not.

      **Your framing needed one correction and it changes the requirement.** The
      non-empty feed renders **no number at all** — `"STORE READ OK · N POSTS
      HELD"` is the screen's only count and it renders only when `rows.length ===
      0`, so it only ever reads `0`; there is no page number and no "showing 30
      of". A requirement written as "a locality line wherever a count appears"
      would therefore have had no subject on the screen you filed it against.
      What is actually unqualified is the **pagination control**: `hasMore` is
      computed by `feed::list_threads` from this peer's log alone, so "Next" means
      *this machine holds another page* and a reader reads it as *this Stoa has
      more*. That is an extent claim without being a numeral, which is precisely
      the crack the apparatus note's wording ("every **number** here") left open.

      So obligation 10 is triggered by the extent claim rather than by the screen:
      never render a quantity the core cannot know, and where the interface does
      assert extent, that assertion must be readable as local. **Rejected: a
      once-per-screen locality line** — that is a disclaimer printed at the
      reader, which is the mistake this whole change undoes.

      **The code is now wrong against a brief that is right**, so the remaining
      work is a `dev-writer` box below rather than more spec.

- [x] **`dev-writer`** — `FeedScreen.qml`, the pagination row — the paging
      control asserts extent and nothing on that screen says whose copy it is
      about
      **The requirement it fails:** `docs/UI-BRIEF.md` rendering obligation 10,
      second half — where the interface asserts extent, the assertion must be
      readable as local. Written in answer to the `spec-writer` box above; read
      that box for why the obligation is about the control rather than about a
      count.
      **Scenario:** open a Stoa this peer holds thirty-one posts for. Thirty
      render and a "Next" button appears. The screen's locality sentence — "This
      is a fact about your copy, not about the Stoa." — is inside the `Rectangle`
      gated on `rows.length === 0`, so it does not render; the pagination
      rationale ("a fact about this copy and not a claim about how much exists")
      is a **code comment** beside the `RowLayout`, which no user reads. The
      reader has an extent claim and no locality statement.
      **The shape this change already demonstrated** is the one to reach for, and
      it is named here because it is a pointer rather than a layout decision: the
      `ON THIS ORDERING` sentence became a body-level `Text` outside the state
      branches, with a comment saying which obligation it discharges. Whether
      obligation 10's sentence belongs beside the pagination row, in the body, or
      folded into the ordering sentence already there is yours — **what is settled
      is that it must render in the state where paging is offered, and must not
      become a disclaimer on screens that assert no extent.**
      **One documentation edit belongs with the code fix**, because it is your
      file and not mine: `design.md` §2's `ON WHAT YOU HOLD` row claims the
      obligation survives "in the **interface already**", which is true only of
      the empty state. Narrow it the way §2's `ON THE MARK` row was already
      narrowed — say which claim it supports. The row's other citation,
      `tasks.md` 2.1, is annotated; `findings/architecture.md` files the same
      point and is answered there.
      **Severity: medium.** Pre-existing, not created by this change; it is in
      scope because this change is what moved the obligation and because the brief
      it must now meet is completed in the same change.

      **Fixed.** The pagination `RowLayout` is now a `ColumnLayout` holding the
      button row and the locality sentence, so the existing `visible:` binding —
      `readState === "ok" && (hasMore || page > 0)` — decides both. The claim and
      its qualifier cannot render apart in either direction, and no new guard was
      added: the row was always outside `readState`'s three-state invariant.

      **Of the three placements you left to me, I took the third and rejected the
      one you pointed at.** A body-level `Text` mirroring `ON THIS ORDERING` would
      work, but it needs its own visibility condition duplicating the control's —
      a fourth slightly-different `visible:` guard, which CLAUDE.md names as the
      signal to reshape rather than to add a fifth. Nesting reuses the binding the
      control already carries, so there is no second condition to keep in step.
      Folding it into the ordering sentence was rejected for the opposite reason:
      that sentence renders unconditionally, so it would have printed a paging
      disclaimer on screens that offer no paging — the outcome your last clause
      forbids.

      **Proved in both states, and the instrument matters more than the result.**
      Measured through `run-qml-tests.sh` on a throwaway spec in `tmp/probe/`:
      with `hasMore=true` the sentence and the "Next" button resolve to the same
      governing ancestor; with `hasMore=false` at `page=0` they still do, so
      neither can be shown without the other. `page=2, hasMore=false` keeps the
      control (Previous is still an extent claim) and keeps the sentence with it.

      **Two instruments were tried first and both are gates the defect
      satisfies** — worth recording, because each looks conclusive:

      - **`visible` is unreadable here.** QML reports *effective* visibility and a
        `TestCase` is itself invisible offscreen, so every descendant reads
        `false` — including the unconditional ordering sentence. A test asserting
        `visible === true` would fail on correct code; one asserting `false` would
        pass on anything.
      - **Height cannot see the sentence.** The card's paging-versus-no-paging
        `implicitHeight` delta was measured **identical** with the real sentence
        and with it replaced by a one-word string. A height assertion would have
        reported clean across the exact mutation it was meant to catch.

      What is readable is the object graph, so that is what the test asserts.

      **Test added:** `dialectica-ui/tests/tst_feed_extent_claim.qml`, three
      cases — the sentence is present where paging is offered, it shares a
      governing ancestor with the buttons, and the no-extent state does not
      acquire it separately. **Two mutations justify them**, and the second is
      the one that matters:

      - deleting the sentence fails two of the three;
      - **moving the sentence into the empty-state card** — the defect shape you
        filed, a sentence that exists on the screen but in a state that never
        coincides with the control — leaves the presence check **passing** and is
        caught only by the shared-ancestor assertion. A presence-only test would
        have been a gate this defect satisfies, which is why the structural one
        is the load-bearing assertion rather than a nicety.

      Both mutations were run and reverted; `git diff` on `FeedScreen.qml` was
      read back to confirm no residue before committing.

      **`design.md` §2 narrowed**, as asked. The row no longer claims the
      obligation survives "in the interface already"; it now cites brief
      constraint 1 *and* obligation 10, and points at a new subsection —
      *`ON WHAT YOU HOLD` was narrower than it looked, and the gap was not a
      count* — which records that the repair the row's wording invited (a
      locality line wherever a count appears) has **no subject on this screen**,
      because the non-empty feed renders no number; that the unqualified claim is
      the control; the two placements rejected and why; and the measurement note
      above, so the next person does not re-derive that height and `visible` are
      blind here.

- [ ] **`tester`** — `dialectica-ui/tests/` — nothing pins the one piece of
      interface text this change creates, nor the `implicitHeight` it fixes
      **Scenario:** delete `FeedScreen.qml:225-233` (the "Not newest first…"
      `Text`) and the suite is 41/41 green. Revert `ScreenFrame.qml:43`
      (`implicitHeight`) to nothing and the suite is 41/41 green, while the feed
      silently stops scrolling — which is exactly how the defect survived from
      `0538c0d` to now.
      **Measured:** 41 of 41 tests pass under both mutations. `tasks.md` leaves
      the `tests` row unticked and asks the right question about the ordering
      sentence; this entry adds the second half — the `implicitHeight` is the
      more consequential of the two and is equally unpinned. A test asserting
      `frame.implicitHeight > 0` and that it grows with `rows.length` is cheap,
      needs no rendering, and would have failed on `main`.
      **Severity: medium.**

      **Left open — `tester`'s.** A test that must be *proven to fail* is not
      mine to write, and writing it badly would be worse than leaving the box.
      Two notes from `dev-writer`, `291c719`, so whoever takes it is not working
      from stale pointers:

      - **`ScreenFrame.qml:43` has moved.** The `implicitHeight` binding you name
        is now at **line 28** (`implicitHeight: body.implicitHeight + 2 *
        Theme.cardPaddingY`); the `body.height` binding is at **79**. I rewrote
        the comment block — the two bindings themselves are unchanged, so both
        mutations you describe still reproduce.
      - **The probe technique works, contrary to the comment repeated in several
        test files.** QtTest measures height with the root given a size: a
        `TestCase` with `width`/`height` set and `when: windowShown`, then
        `createTemporaryObject`. I used exactly that to re-measure the escape
        hatch (`implicitHeight` 156 plain, 176 with a trailing spacer, and rows
        at y=111/393 versus y=0/60 at `height: 600`), which is the same shape a
        test for `frame.implicitHeight > 0` and its growth with `rows.length`
        would need. Worth knowing before concluding it cannot be tested here.
      - There is now a **third** assertion worth pinning alongside your two: that
        a trailing `Item { Layout.fillHeight: true }` inflates `implicitHeight`
        by `Theme.blockGap` while `Layout.fillHeight` on a real child does not.
        That is the defect `findings/readability.md`'s first box filed, and it is
        currently prevented by a comment only. Measured numbers are in
        `design.md` §4.

- [x] **`dev-writer`** — `design.md` §2 / commit message — the `ON THE MARK`
      "never a proof" proposition has no surviving rendered text
      **Scenario:** `grep -rn "proof"` over `dialectica-ui/src/qml/` returns
      nothing. The structural half of the claim is genuinely sound and I verified
      it: `PostHeader.qml:35-38` renders `AddressLabel` with no `visible:`
      binding and no empty-string collapse, and the asymmetry runs the safe way —
      `Identicon` at `:21-26` is the one that can hide (`visible: markSize >=
      Theme.markMinDraw`), so a mark without an address cannot occur. Same at the
      Stoa header, `FeedScreen.qml:143`.
      **What does not survive** is the second proposition the note carried: that
      the mark is "a shortcut for recognition, never a proof of anything". Nothing
      on screen says it. `design.md` §2's argument — that a note asserting it adds
      nothing a reader acts on — is a defensible call, and I am not asking for the
      note back.
      **Severity: low, and this is a documentation defect rather than a code
      one.** The ask is that §2's "survives structurally" be narrowed to the claim
      it actually supports (the address is beside the mark), so the next reader
      does not cite this row as evidence that the non-proof claim is discharged
      somewhere.

      **Fixed, as asked and no further.** `design.md` §2's table row for
      `ON THE MARK` no longer asserts a single surviving obligation; it now says
      the note carried **two propositions** and points at a new subsection that
      separates them:

      - the **pairing** claim survives structurally, with your asymmetry
        measurement written in (`AddressLabel` has no `visible:` binding; the
        `Identicon` is the conditional element, so a mark cannot appear without
        an address);
      - the **non-proof** claim has no rendered text, is accepted as
        undischarged in the interface deliberately, and the reason is stated —
        the protection a reader acts on is the address being present, not prose
        telling them a glyph is not evidence.

      The subsection closes by naming the misreading to prevent: the row
      supports "the address is beside the mark" and not "the non-proof
      proposition is discharged somewhere". The note is not restored, per your
      "I am not asking for the note back".

      No code change — this box is a documentation defect and is closed as one.

## Areas that were clean

The three-state logic in `FeedScreen.reload()` is untouched by this change and
still holds: `readState` is one string computed in one place, the failure path
never writes `rows`, and a reply that is neither shape becomes a named failure
rather than an empty feed. The `Theme` edits are comment-only apart from deleting
`apparatusWidth`, whose two readers both went with it; keeping `paperDeep` as an
unreferenced surface token is argued in `design.md` §5 and is the right call for a
palette. `qmldir` lost exactly the two registrations whose files were deleted. The
width change a `fillWidth` child sees (688 → 932 at frame width 1000) is the
intended consequence of removing a 244px column and is correct.
