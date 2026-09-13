## Stages

- [ ] ~~spec — `spec-writer`~~ — **does not apply.** This change removes a view
      element no requirement ever asked for, so it has no spec delta;
      `.openspec.yaml` sets `skip_specs: true` with the measurement behind it.
      Struck through rather than omitted, because "does not apply" and "nobody did
      this" are different states and the block exists to tell them apart.
      **One caveat, and it is `design.md` §7 rather than a reason to unstrike
      this row:** an *unmerged* delta on `piece/ui-composer` requires a
      `compose.apparatus` string. It is reported, not touched — a spec change is a
      `spec-writer`'s and needs the owner's call.
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

**On the `tests` row.** It is left unticked deliberately, and a `tester` has a
real question to ask here even though this change adds no assertion. The
question is the one §4 of `design.md` raises: **no test on `main` could see the
apparatus column, and none can see its absence either.** The three notes shipped
into the running app and all 41 QML tests stayed green throughout. A `tester`
should decide whether the ordering sentence moved in §3 is worth pinning — it is
the one piece of interface text this change creates, and the argument for pinning
it is that it was invisible to the suite in its previous location too.

## 1. Establish the true extent before changing anything

- [x] 1.1 Sweep the tree for both names rather than trusting the dispatch's list
      — `grep -rniI "apparatus\|MarginNote"` over `dialectica-ui/`, `docs/`,
      `openspec/specs/` and `dialectica/`. Found: two components, three
      instantiations in `FeedScreen`, the `ScreenFrame` shell, three `Theme`
      lines. The other four hits (`PHASE0-FINDINGS.md`, `IDENTICON.md`,
      `module-wire-contract/spec.md:302`, `rust-lib/src/lib.rs:72`) are the
      unrelated Phase 0 sense of the word — "apparatus rather than forum
      surface" — and are left alone.
- [x] 1.2 Establish that `docs/UI-BRIEF.md` does **not** present the apparatus as
      rendered UI, contrary to the dispatch's premise — verified by
      `git grep -niI "apparatus\|margin note\|marginal\|MarginNote" origin/main --
      docs/UI-BRIEF.md` returning **nothing**. Recorded in `design.md` §1, because
      it locates the defect: the column came from the bundle directly, not via the
      brief.
- [x] 1.3 Confirm `FeedScreen` is the only `ScreenFrame` user — verified by
      `grep -rn "ScreenFrame"` over `dialectica-ui/`, which returns the component,
      its `qmldir` line, and one instantiation.
- [x] 1.4 Confirm no test asserts apparatus content — verified by
      `grep -rniI "MarginNote\|apparatus\|ON THIS ORDERING\|ON WHAT YOU HOLD\|ON
      THE MARK"` over `dialectica-ui/tests/`, which returns one line in
      `tst_identicon.qml`: the word "mark" inside an unrelated comment.

## 2. Check each obligation survives before deleting its note

Done before any deletion, since a note whose obligation exists nowhere else is a
requirement being deleted by accident. The table is `design.md` §2.

- [x] 2.1 `ON WHAT YOU HOLD` — survives in the brief at constraint 1
      (`UI-BRIEF.md:86-88`, "a count of *anything* global … is unknowable") **and
      in the interface already**, at `FeedScreen.qml`'s empty state: "This is a
      fact about your copy, not about the Stoa."
- [x] 2.2 `ON THE MARK` — survives in the brief at obligation 6 layer 2
      (`UI-BRIEF.md:584-593`, "must never be rendered as a verification mark, a
      badge, or anything that reads as 'checked'") **and structurally**, since
      `PostHeader.qml` and the Stoa header both render `AddressLabel` beside
      `Identicon` unconditionally.
- [x] 2.3 `ON THIS ORDERING` — **does not fully survive**, and this is the one
      that needed action. The brief's half is intact (Feed section); the
      interface's half was carried only by this note. The ordering label "same
      order for everyone" is honest but **neutral** — it declines to claim recency
      without denying it, and a reader assumes newest-first unless told otherwise.
      Handled in 4.1.

## 3. Remove the apparatus from the view

- [x] 3.1 Delete `ApparatusColumn.qml` and `MarginNote.qml` via `git rm`.
- [x] 3.2 Drop both `qmldir` registrations. Verified nothing else names either
      type by the 1.1 sweep.
- [x] 3.3 Remove `FeedScreen.qml`'s `apparatus: [...]` block — the only
      assignment to the alias `ScreenFrame` exposed.
- [x] 3.4 Reshape `ScreenFrame.qml` from a two-column `RowLayout` into one
      anchored `ColumnLayout`, and drop the `apparatus` alias. Alternatives
      rejected in `design.md` §4.
- [x] 3.5 Give `ScreenFrame` an `implicitHeight`, which the reshape forces rather
      than chooses: the old `anchors.fill` `RowLayout` propagated no implicit size,
      so the `Rectangle` had none — while `Main.qml:42` reads exactly that value.
      Recorded as a discovered defect in `design.md` §4 rather than presented as
      part of the removal.
- [x] 3.6 Remove `Theme.apparatusWidth` (two readers, both deleted) and correct
      the two theme comments naming inks by their role in the column.
      `Theme.paperDeep` is kept as a surface token — reasoning in `design.md` §5.
- [x] 3.7 Confirm no theme token is orphaned by the deletions beyond `paperDeep`
      — verified by `grep -rn "Theme.note\|Theme.rule2\|Theme.paperDeep"`:
      `Theme.note` is used by `PostHeader.qml:61`, `Theme.rule2` by
      `VoteControl.qml` and `FeedScreen.qml:284`.

## 4. Keep the one obligation that would otherwise have gone

- [x] 4.1 Move the `ON THIS ORDERING` sentence into `FeedScreen`'s own body,
      under the heading rule, verbatim — so it becomes interface addressed to a
      user rather than margin addressed to a designer. The comment above it says
      why the neutral ordering label does not cover it, so a later reader does not
      delete it as redundant with the label.
- [x] 4.2 Reject discharging it into `docs/UI-BRIEF.md` alone — a brief
      obligation with no interface text is how the apparatus came to ship. Argued
      in `design.md` §3.

## 5. Correct `docs/UI-BRIEF.md`

Kept to the apparatus question only. Three branches edit this file concurrently
and `docs/name-shape-sweep` (#64) rewrites it wholesale, so unrelated staleness is
reported to the runner rather than fixed here.

- [x] 5.1 Add a box under *Non-negotiable rendering obligations* saying an
      obligation is a thing the interface must **do**, not text to print at the
      reader; that the bundle's `APPARATUS` column is annotation rather than
      interface; that it was built anyway and has been removed; and that
      obligations discharge structurally where they can.
- [x] 5.2 Add the matching clause to the Feed section's ordering rules: a neutral
      label is **not by itself enough**, because it declines to claim recency
      without denying it, so the screen must say plainly that this is not newest
      first.

## 6. Gates

- [x] 6.1 `dialectica-ui/tests/run-qml-tests.sh` — **41 passed**, 0 failed across
      4 spec files (13 + 12 + 7 + 9), on Qt 6.10.3. Same 41 as before the change:
      no test asserted apparatus content, which is the 1.4 measurement.
- [x] 6.2 `qmllint` clean (exit 0) on `ScreenFrame.qml`, `FeedScreen.qml`,
      `Theme.qml`, `Main.qml` and `PostHeader.qml`, with `-I` pointing at the qml
      directory so local types resolve.
- [x] 6.3 `qmlformat` parses `ScreenFrame.qml` and exits 0 — CI uses it as the
      syntax check because qmllint does not catch a syntax error.
- [x] 6.4 CI's `textFormat` gate re-measured with **its own** regex rather than a
      looser one. `grep -cE '\bText \{'` gives 13 for `FeedScreen.qml` against 13
      `textFormat:` assignments, and every other file balances. A plain
      `grep -c "Text {"` reports 15 and would have looked like a failure — the
      false positives are `SanitisedText {`, which the gate's `\b` excludes by
      design.
- [x] 6.5 CI's layout-import gate: `ScreenFrame.qml` still uses `ColumnLayout` and
      still imports `QtQuick.Layouts`.
- [x] 6.6 No component name shadows a Qt built-in — only deletions were made, so
      the reserved-name list is untouched.

## 7. Report rather than fix

- [x] 7.1 The `compose.apparatus` requirement on unmerged `piece/ui-composer` —
      `design.md` §7. Read in full it does not need the column (it governs the
      **closed gate's** content), but the wording is contract text, so it is the
      composer piece's `spec-writer` to change. Verified that the string has no
      implementation on any branch, that `composer-view` is not among the sixteen
      merged capabilities, and that no `copy.json` exists in the tree.
- [x] 7.2 The `ON PUBLISHING` delivery disclaimer — `design.md` §8. Verified
      absent from `origin/main` and from `piece/ui-composer`'s pushed tip, so
      there is nothing on this branch to preserve it into and no publish path to
      attach it to. The obligation is contracted in `UI-BRIEF.md` obligation 9 and
      in `composer-view`, and belongs to the composer screen.

## 8. Correctness and security findings

Five boxes across `findings/correctness.md` and `findings/security.md` — **three
for `dev-writer`, all closed**. The measurements live beside the reviewer's own
text in those files; this section records only what was done. The remaining two
boxes are one `spec-writer`'s and one `tester`'s and are deliberately untouched.

- [x] 8.1 Close the `fillHeight` collapse, filed as one defect in both files.
      **Reproduced first**, which is what made the fix checkable: a bare
      `Rectangle` with `Layout.fillHeight` in a `ScreenFrame { width: 1000;
      height: 600 }` measured `filler.h=0`. `body.height` is now bound to
      `Math.max(implicitHeight, root.height - 2 * Theme.cardPaddingY)` and the
      same probe measures **544**, matching `origin/main`. No binding loop: the
      binding reads `root.height` while `implicitHeight` reads
      `body.implicitHeight`.
- [x] 8.2 Measure the two alternatives rather than asserting the choice. A
      trailing `fillHeight` spacer **splits the slack with a genuine `fillHeight`
      child** (544 → 262) and adds a `spacing` gap to `body.implicitHeight`,
      inflating the card 20px so the `Flickable` scrolls past the content —
      re-breaking what `implicitHeight` fixed. Anchoring the bottom edge measures
      identically to the `Math.max` binding, which is chosen only for being
      explicit. Both rejections are recorded in `design.md` §4 with their numbers.
- [x] 8.3 Name the trade the fix accepts instead of hiding it. A card given an
      **explicit height** with no `fillHeight` child scatters its rows (two 40px
      rows at y=111 and y=393 rather than y=0 and y=60). Accepted because no
      caller does that — `Main.qml` assigns the frame no height at all, so the
      card is always sized from `implicitHeight`, where the probe measures y=0
      and y=60. `ScreenFrame.qml` carries the escape hatch in a comment.
- [x] 8.4 Extend `design.md` §4, which the review correctly found discussed only
      the `implicitHeight` half of the anchor choice and never mentioned that the
      same choice changed what `fillHeight` means for every future child. It now
      carries both halves, the review's re-measurement of the original defect
      (`implicitHeight` 0 at thirty rows, `contentHeight` 56 — the feed did not
      scroll at all), and the two rejected alternatives.
- [x] 8.5 Narrow `design.md` §2's `ON THE MARK` row to the claim it supports.
      The note carried **two** propositions; the pairing half survives
      structurally (with the review's asymmetry measurement written in) and the
      "never a proof" half has no rendered text and is accepted as undischarged
      in the interface, with the reason stated. The note is not restored — the
      reviewer explicitly did not ask for it back.
- [x] 8.6 Gates re-run after every edit above: `run-qml-tests.sh` **41 passed**,
      0 failed across 4 spec files; `qmllint` exit 0 on `ScreenFrame.qml`,
      `FeedScreen.qml` and `Main.qml`, with no binding-loop warning. The probe
      harness was a temporary `tst_zzprobe.qml`; it is deleted and the suite is
      back to its four spec files.
