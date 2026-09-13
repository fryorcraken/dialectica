## Stages

- [x] spec — `spec-writer` — **no OpenSpec delta, and that half still does not
      apply.** This change removes a view element no requirement ever asked for,
      so it has no capability delta; `.openspec.yaml` sets `skip_specs: true` with
      the measurement behind it.
      **The row is ticked rather than struck because a requirement was
      nonetheless written**, on the second pass, in answer to the `spec-writer`
      boxes in `findings/architecture.md` and `findings/correctness.md`:
      `docs/UI-BRIEF.md` **rendering obligation 10**, plus the matching clause in
      the Feed section. That is where a view obligation belongs — every capability
      `openspec list --specs` reports contracts core behaviour, and there is no
      view-facing capability in the tree. Reasoning is in `proposal.md` under *The
      obligation the move narrowed*.
      **One caveat, and it is `design.md` §7 rather than a reason to reopen this
      row:** an *unmerged* delta on `piece/ui-composer` requires a
      `compose.apparatus` string. It is reported, not touched — a spec change is a
      `spec-writer`'s and needs the owner's call.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer` — `findings/design.md`. Seven boxes, all
      `dev-writer`'s, **all now closed — see §10**. The big one: **rendering
      obligation 10's decision is not in `design.md`**. Also a `UI-BRIEF.md`
      contract whose `fillHeight` promise is false in the only call shape the tree
      has — measured, `filler.h=0` in a `Main.qml`-shaped harness, and reproduced
      independently when the box was closed. §4's six figures all reproduce;
      PLAN.md is clean.
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

- [x] 2.1 `ON WHAT YOU HOLD` — survives in the brief at **constraint 1** ("a
      count of *anything* global … is unknowable. Do not show one") **and in the
      interface already**, at `FeedScreen.qml`'s empty state: "This is a fact
      about your copy, not about the Stoa."
      **Narrowed by the `spec-writer`'s second pass — the "in the interface
      already" half is true only of the empty state**, and the state where the
      screen asserts extent (the pagination control) carries no locality
      statement. The brief was incomplete rather than wrong: it covered counts and
      not non-numeric extent claims. Completed as `UI-BRIEF.md` rendering
      obligation 10; the code gap is an open `dev-writer` box in
      `findings/correctness.md`. Read that box and `proposal.md`'s *The obligation
      the move narrowed* before citing this line as evidence the obligation
      survived intact — it did not.
- [x] 2.2 `ON THE MARK` — survives in the brief at **obligation 6, layer 2 (the
      identicon)** in its four-layer list — "must never be rendered as a
      verification mark, a badge, or anything that reads as 'checked'" — **and
      structurally**, since
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
- [x] 5.3 *(`spec-writer`, second pass.)* Add **rendering obligation 10** —
      every quantity is a fact about this machine's copy, and "more" is a
      quantity — in two halves: never render a quantity the core cannot know, and
      where the interface does assert extent, that assertion must be readable as
      local. It closes the gap constraint 1 left: constraint 1 forbids a global
      *count*, and the feed's extent claim is a **control**, not a numeral.
      Includes the explicit non-obligation — a screen asserting no extent owes
      nothing, because a once-per-screen disclaimer is the mistake this change
      undoes.
- [x] 5.4 *(`spec-writer`, second pass.)* Add the matching clause to the Feed
      section: paging is an extent claim governed by obligation 10, `hasMore` is
      computed from this peer's log alone, and the screen that offers paging is
      the one that owes the locality statement. Placed there because a screen
      author works from the Feed section, which is the routing failure that let
      the apparatus ship.

## 6. Gates

- [x] 6.1 `dialectica-ui/tests/run-qml-tests.sh` — green, 0 failed, on Qt 6.10.3.
      Run it for the count; a number written here goes stale the moment a spec
      file is added, as one was for obligation 10. What the count cannot say and
      is worth recording: no test asserted apparatus content before the change,
      which is the 1.4 measurement, so nothing was lost by the deletions.
- [x] 6.2 `qmllint` clean (exit 0) on `ScreenFrame.qml`, `FeedScreen.qml`,
      `Theme.qml`, `Main.qml` and `PostHeader.qml`, with `-I` pointing at the qml
      directory so local types resolve.
- [x] 6.3 `qmlformat` parses `ScreenFrame.qml` and exits 0 — CI uses it as the
      syntax check because qmllint does not catch a syntax error.
- [x] 6.4 CI's `textFormat` gate re-measured with **its own** regex rather than a
      looser one: `grep -cE '\bText \{'` against `grep -c 'textFormat:'`, which
      balance in `FeedScreen.qml` and in every other file. Run both for the
      figures — obligation 10's sentence moved them.
      The trap worth recording is the regex, not the number: a plain
      `grep -c "Text {"` also counts `SanitisedText {` and would report an
      imbalance that is not there. The gate's `\b` excludes those by design.
- [x] 6.5 CI's layout-import gate: `ScreenFrame.qml` still uses `ColumnLayout` and
      still imports `QtQuick.Layouts`.
- [x] 6.6 No component name shadows a Qt built-in — only deletions were made, so
      the reserved-name list is untouched.

## 7. Report rather than fix

- [x] 7.1 The `compose.apparatus` requirement on unmerged `piece/ui-composer` —
      `design.md` §7. Read in full it does not need the column (it governs the
      **closed gate's** content), but the wording is contract text, so it is the
      composer piece's `spec-writer` to change. Verified that the string has no
      implementation on any branch, that `composer-view` is not among the merged
      capabilities (`openspec list --specs`), and that no `copy.json` exists in
      the tree.
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
- [x] 8.6 Gates re-run after every edit above: `run-qml-tests.sh` green, 0 failed;
      `qmllint` exit 0 on `ScreenFrame.qml`, `FeedScreen.qml` and `Main.qml`,
      with no binding-loop warning. The probe harness was a temporary
      `tst_zzprobe.qml`; it is deleted.

## 9. Obligation 10 — the extent claim, after the second review pass

- [x] 9.1 Make `FeedScreen.qml` satisfy brief rendering obligation 10. The
      pagination `RowLayout` became a `ColumnLayout` holding the button row and a
      locality sentence, so the control's existing `visible:` binding governs
      both and the claim cannot render without its qualifier. No new `visible:`
      guard: the row was always outside `readState`'s three-state invariant.
- [x] 9.2 Reject the two placements that do not hold by construction. A
      body-level `Text` mirroring `ON THIS ORDERING` needs a fourth
      slightly-different guard duplicating the control's condition; folding the
      sentence into the ordering sentence would print a paging disclaimer on
      screens that offer no paging, which obligation 10 explicitly forbids.
      Recorded in `design.md` §2.
- [x] 9.3 Prove the control in **both** states rather than reading the binding:
      with `hasMore=true`, with `hasMore=false` at `page=0`, and at `page=2`
      where Previous keeps the claim alive. Measured through `run-qml-tests.sh`
      on a throwaway spec under `tmp/probe/`, since the runner now takes a path.
- [x] 9.4 Record that two obvious instruments are blind here, because each looks
      conclusive and one is already assumed in several test-file comments:
      `visible` reports *effective* visibility and reads `false` for every
      descendant of an offscreen `TestCase`; and the card's
      paging-versus-no-paging height delta measures **identical** with the
      sentence present and replaced by a one-word string. Written into
      `design.md` §2 and the new spec's header comment.
- [x] 9.5 Add `dialectica-ui/tests/tst_feed_extent_claim.qml`, asserting the
      object-graph relation rather than a pinned literal, and prove each case can
      fail. Two mutations: deleting the sentence fails two cases; **moving it
      into the empty-state card** — the filed defect shape — leaves the presence
      check passing and is caught only by the shared-ancestor assertion, which is
      why that assertion is the load-bearing one.
- [x] 9.6 Narrow `design.md` §2's `ON WHAT YOU HOLD` row, which claimed the
      obligation survives "in the interface already" — true only of the empty
      state. The row now cites constraint 1 and obligation 10 and points at a new
      subsection recording why the repair the old wording invited had no subject
      on this screen.
- [x] 9.7 Gates after the change: `run-qml-tests.sh` green, 0 failed; `qmllint
      --unqualified disable` exit 0 on `FeedScreen.qml` and on the new spec;
      CI's `textFormat` gate balances under its own regex. `git diff` on
      `FeedScreen.qml` read back to confirm both mutations were reverted with no
      residue, and the scratch probe deleted.

## 10. The design review's seven boxes

All seven are `dev-writer`'s and all seven are closed. Outcomes and measurements
live beside the reviewer's text in `findings/design.md`; this records what was
done. Two of the seven describe a tree that had already moved — the review read
`4ab9144` and was committed at `dd0fe95`, two commits later.

- [x] 10.1 Record obligation 10's **decision** in `design.md`, which carried its
      reasoning but not its alternatives. New §2 subsection with the two rejected
      framings (a once-per-screen disclaimer; empty-screen-only is correct) and
      **what the obligation costs** — it is the first brief obligation
      dischargeable only by prose, in a change arguing obligations are things the
      interface *does*. The discriminator that makes that defensible is *who is
      addressed*, not prose-versus-structure.
- [x] 10.2 §2's `ON WHAT YOU HOLD` row and its `findings/` pointer — **already
      fixed by `2fc3689`**, which landed after the commit the review read. No edit
      made; verified by diff that both halves are gone and that
      `grep -n "findings/correctness"` over `design.md` returns nothing, so no
      citation dangles after archive deletes `findings/`.
- [x] 10.3 Correct the `ScreenFrame` contract in `docs/UI-BRIEF.md`, whose two
      middle bullets contradicted each other. **Re-measured independently before
      editing**: a `fillHeight` child gets 484 in a card with an explicit height
      and **0** in a content-sized card — including the `Main.qml`-shaped harness,
      which is the only call shape in the tree. The brief now says a content-sized
      card has no slack to give and tells an author to design a screen that grows
      downward. Two alternatives rejected in `design.md` §4, with the cost named:
      a screen wanting a full-height region cannot get one from the shell as it
      stands. `ScreenFrame.qml`'s comment carried the same unqualified figure and
      now carries the scope with it.
- [x] 10.4 Stop the `ScreenFrame` section swallowing the ten rendering
      obligations. It was the only heading between `## Non-negotiable rendering
      obligations` and `## The vote control`, so all ten nested inside it.
      Promoted to `##` **and** the obligations given `## The obligations
      themselves` — promoting alone leaves obligation 1 under the wrong heading.
      Restores `origin/main`'s structure while keeping the reading order
      `design.md` §4 argues for.
- [x] 10.5 Restore the `copy.json` `feed.orderingNote` provenance comment on the
      relocated ordering sentence — the one bundle-sourced string in the tree
      without one, against seven that keep theirs. Kept rather than dropped
      because what changed is the presentation and not the string. Recorded in
      `design.md` §3 with the rejected alternative.
- [x] 10.6 Fix 2.2's stale `UI-BRIEF.md` line range, the twin of the one
      `findings/readability.md` fixed in `design.md`. The cited sentence has now
      been measured at three different lines across three reviews (622, 641,
      **875** today) without changing, so the range is replaced by the heading.
      2.1's range is replaced too — it is still accurate, and it is the same
      instrument.
- [x] 10.7 Correct `proposal.md`'s Impact list, which said "**No test changes**"
      while the diff modified `run-qml-tests.sh`, and omitted `CLAUDE.md` and the
      added spec file. The runner's single-spec mode is now argued where a reader
      of the change folder will meet it rather than only in `git log`, with the
      cost it accepts (two modes that can drift) stated. `design.md` §6 carried
      the same false sentence and is corrected to match.
- [x] 10.8 Gates after all of the above — see §11.

## 11. Merging `main`, and the eleven notes the owner decided to delete

`main` moved a long way while this piece was in review: #67 renamed the theme
singleton, #77/#79 rewrote the QML runner, and #60/#63 added three screens.
Six files conflicted. Each resolution below was verified rather than assumed.

- [x] 11.1 **Establish which merge-tree stage is which**, because the number
      depends on merge direction and a previous brief assumed it wrongly.
      `git merge-tree --write-tree origin/main HEAD` puts `origin/main` at
      **stage 2** and the piece at **stage 3** — proved by resolving
      `origin/main:<path>` and `HEAD:<path>` with `git rev-parse` and matching
      all eight blob hashes, not by reading the documentation.
- [x] 11.2 **`ApparatusColumn.qml` and `MarginNote.qml` — modify/delete, both
      deleted.** `main`'s only change to either is #67's `Theme.` → `DTheme.`
      rename, measured by diffing the base blob against `main`'s. A rename
      applied to a file that is going away is not a reason to keep the file.
- [x] 11.3 **`run-qml-tests.sh` — took `main`'s wholesale.** `main` grew this
      branch's single-spec mode independently and added `check_bindings` and
      `--import` on top. Proved byte-identical to `origin/main`'s blob with
      `git hash-object` (`8802141`) after `checkout --theirs`. `proposal.md`'s
      Impact entry claiming this change adds the mode is corrected rather than
      left moot.
- [x] 11.4 **`CLAUDE.md`, `FeedScreen.qml`, `ScreenFrame.qml` — genuine
      three-way merges**, each keeping this piece's work and `main`'s. `CLAUDE.md`
      keeps the piece's *UI-BRIEF* row **and** `main`'s new `RUNNER.md` row;
      `ScreenFrame` keeps the one-column reshape and drops the `apparatus` alias
      `main` documented; `FeedScreen` keeps the empty tail where the three notes
      were.
- [x] 11.5 **Delete the eleven `MarginNote` instantiations `main` added**, on the
      owner's explicit decision — no migration, no rewriting as body copy.
      `DJoinScreen` (4), `DStoaListScreen` (4), `DOnboardingScreen` (3). Every
      note is recorded verbatim in `design.md` §9, including the two sentences
      that existed nowhere else, so nothing needs reconstructing from git history.
- [x] 11.6 **Rewrite `DOnboardingScreen`'s body-copy comment**, which argued from
      "the margin note stays" and pointed at "the apparatus note below" — both
      false once the notes went. The reasoning that survives (why the obligation
      is in the body, why no word count) is kept; the orphaned premises are gone.
- [x] 11.7 **Sweep the whole `dialectica-ui/` tree for stale bare `Theme.`**, not
      only the files touched. The merge left five live reads in this branch's own
      new code — `FeedScreen.qml:413,414,674,698,699` — plus five in
      `ScreenFrame.qml`, all written correctly for a pre-#67 world and stale
      after it. A stale `Theme.` resolves into basecamp's host namespace, reads
      `undefined`, and renders as an unstyled default without failing any build.
      `check_qml_names.py` now reports **ok across 29 QML files and 17 qmldir
      entries**.
- [x] 11.8 **Three test edits the merge forced**, each argued in `design.md` §9
      rather than fixed to whatever turned the suite green: one obsolete
      guard-on-a-guard deleted, one count-pin (`shown.length > 5`, broken by a
      legitimate change) replaced by the named-sentence check it stood in for,
      one `screen.apparatus.length` dereference guarded. The last was **proved
      still able to fail** by mutating the body sentence — it reported *"Found 0
      carrier(s), 0 of them in apparatus"* — and the mutation reverted.
- [x] 11.9 **Drop `MarginNote` and `ApparatusColumn` from `check_qml_names.py`'s
      `GRANDFATHERED` set**, since their components no longer exist. An exemption
      for a deleted file exempts nothing and grows a list the gate's own comments
      ask a reader to shrink — the "hand-maintained sweep lists go stale
      silently" trap. The comment above the set described it as "eleven", a count
      this change would have falsified; it now describes the set instead of
      counting it. `tst_check_qml_names.py` builds its fixtures from `Core` and
      `Identicon`, so neither name was pinned and all 17 cases still pass.
- [x] 11.10 **Correct four claims in `proposal.md` and `design.md` that the merge
      falsified**, rather than leaving them to read as current: `composer-view`
      and `stoa-navigation-view` are no longer unmerged; `compose.apparatus` is
      now settled by the merged spec, which requires the statement *"in the
      closed gate's own body"* and explicitly not in any region; and the
      `ON PUBLISHING` note is gone because #62 deleted it and discharged the
      denial in `DPublishOutcome`. Each re-measured against the merged tree.
- [x] 11.11 **Gates and suite.** `check_qml_names.py`, `tst_check_qml_names.py`,
      `check_qml_members.sh`, `tst_check_qml_members.sh` all green;
      `run-qml-tests.sh` exits **0** across 11 spec files with 0 failed.
      `qmllint` exits 0 on every changed file — its remaining "Unqualified
      access" warnings are pre-existing `Repeater`-delegate noise, confirmed by
      running the same lint against a pristine `origin/main` checkout.
      `qmlformat -n` parses every edited file, and no edited file contains the
      multi-declarator `for` init that Qt 6.8.3 miscompiles.
