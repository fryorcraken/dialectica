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

- [ ] **`spec-writer`** — `FeedScreen.qml:302-351` — the `ON WHAT YOU HOLD`
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
