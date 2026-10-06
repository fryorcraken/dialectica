# Design review — `ui-remaining-screens`

Read against `openspec/changes/ui-remaining-screens/{design.md,proposal.md,tasks.md}`,
the code (`DStoaListScreen.qml`, `DTheme.qml`, `tst_stoa_screens.qml`), `qmldir`,
and `docs/PLAN.md` from `origin/main`. `findings/code-review.md` was read first so
this pass does not re-verify what it already confirmed (line-count drop, the
lamp-lifetime split, the mutation numbers, the stale-citation-in-code scoping
call) — only design.md's own text is re-checked against the tree.

## Overall

The Decisions section is unusually well evidenced: every citation I checked
against the code resolved to what it claims, `qmldir` and `docs/PLAN.md` really
are byte-identical to `origin/main` (confirmed independently of
`findings/code-review.md`'s own check), `Main.qml:428` really does bind
`zoneState` from `list.readState`, and PLAN.md's case-2 list really does now
carry only delivery (entry 6) and the storage mock (entry 7) — zone is absent,
matching D2's claim that it "turned out to have a source after all." The four
separator tests and the relation test match D5c/D5d's description exactly,
including the `separatorsForRowCount` helper's lifetime shape. One real
citation defect below, one recording gap, and one suggestion.

- [x] **`dev-writer`** — `design.md:85` — a stale line-number citation, in
      design.md's own prose rather than in code this piece left untouched.
      **Scenario:** D1b quotes `dialectica/rust-lib/src/lib.rs:258` for the
      "can honestly disagree: a stored identity whose keystore permissions are
      too open…" sentence. **Verified:** line 258 is
      `fn generate_identity_slate(&mut self, request: String) -> String;` —
      an unrelated method. The quoted sentence is actually at `lib.rs:284-285`,
      inside `who_am_i`'s doc comment (confirmed by
      `grep -n "can honestly disagree" dialectica/rust-lib/src/lib.rs`).
      `findings/code-review.md` already found and correctly scoped-out this
      *same* stale number where it appears in `DIdentityChip.qml:49` and
      `DThreadScreen.qml:50` — both untouched, byte-identical to
      `origin/main`, so fixing them there is rightly a separate piece's job.
      But `design.md:85` is not inherited code: it is this change's own
      Decisions prose, written fresh by this piece's dev-writer, and nothing
      forces it to carry the same wrong number the QML happens to. A reader
      who opens `lib.rs:258` to check the claim lands on the wrong doc
      comment. One-line fix: `lib.rs:284-285`.

      **Fixed** in `design.md` D1b — but cited by **method name, not by line**,
      which is the deliberate part. The finding is correct that 258 is wrong;
      re-measuring found it is wrong in one more way than reported, and that
      the suggested replacement would itself have been short:

      - **258 is a blank line.** `generate_identity_slate` is at **257**, so
        the number in the tree pointed one past the unrelated method, not at
        it. (The finding says 258 *is* the `fn` line.)
      - **The quote spans `lib.rs:284-288`, not 284-285.** The fragment
        design.md reproduces runs to "…and a fixable problem." on 288, so
        `284-285` would have truncated it mid-sentence. Verified by reading
        `lib.rs:278-294`, which is `who_am_i`'s whole doc comment.
      - **The reviewer's own grep could not have found it**, which is why the
        finding reports that command as confirming a line it does not return:
        `grep -n "can honestly disagree"` matches nothing in `lib.rs`, because
        the phrase wraps as "…the two can honestly" / "disagree: a stored
        identity…" across 284-285. The only grep that lands is on a fragment
        within one line, e.g. `grep -n "keystore permissions are"`. That wrap
        is the likeliest reason the number went unchecked in the first place.

      **Why a method name rather than `lib.rs:284-288`.** This piece just
      dropped a line count from D5c on the grounds that figures rot, and a line
      number is that same figure — it had already rotted twice here. `who_am_i`
      is unique in the trait, survives `lib.rs` growing, and is what the
      decision is actually about. The quoted sentence is kept and extended to
      include the "**A different question from `getCapabilities`**" clause, so
      the citation is checkable by search rather than by offset.

      **Scope, re-verified rather than inherited.** `git diff --stat
      origin/main...HEAD` (three dots) shows this piece touches exactly two QML
      files, `DStoaListScreen.qml` and `DTheme.qml` — so the finding's call
      that `DIdentityChip.qml` and `DThreadScreen.qml` are out of scope holds,
      and I confirmed it rather than taking it on trust. One addition the
      finding did not have: **`FeedScreen.qml:101` carries the same stale
      number**, a third inherited copy, also untouched here and also out of
      scope. The origin is visible too —
      `openspec/changes/archive/2026-09-18-ui-navigation/proposal.md:48` — so
      all four copies descend from one miscount rather than four independent
      counts. design.md now records that provenance, which is what makes the
      remaining three cheap for a later piece to sweep in one pass.

- [x] **`dev-writer`** — `design.md` (no entry) — the choice to key the
      separator test on `objectName` rather than geometry is argued in prose
      (D5c/D5d and the code comment at `DStoaListScreen.qml:494-499`) but never
      given its own Decisions entry. **Scenario:** a reviewer scanning
      Decisions for "why is the separator named" finds nothing under a
      heading — the argument ("a walker keyed on geometry would match
      anything else that happened to be a 1px-high `Rectangle`, and would go
      on passing if this element were deleted and an unrelated rule took its
      place") is real and present, just not where the Decisions list would
      surface it on its own. This is a real alternative with a real reason
      one path was rejected — exactly what CLAUDE.md's "put the complexity in
      the data structure" and this reviewer's brief flag as the kind of
      choice that belongs recorded. Low severity: the reasoning exists and is
      findable, it is just folded into a different decision's prose rather
      than standing on its own. Suggestion, not a blocker.

      **Fixed** — accepted and promoted to its own entry, **D5e — The separator
      is found by `objectName`, never by geometry**. The suggestion is taken
      rather than argued down: this is a technology choice with a named
      rejected alternative and a failure mode, which is what a Decisions entry
      is for, and the prose it was folded into (D5d) is about *object
      lifetime* — a different subject that happened to be adjacent. The
      `objectName` paragraph is moved out of D5d rather than copied, so there
      is no second copy to drift.

      Writing it as a decision made it testable, and **the measurement
      corrected a claim I had already written**. The entry first said removing
      the `objectName` "turns all four separator tests red at once". Running it
      gives **79 passed, 3 failed** against the 82 baseline:
      `test_an_empty_list_draws_no_row_boundary` **stays green**, because it
      asserts zero boundaries and an unnamed separator also yields zero — the
      mutation and the correct behaviour give that test the same answer. D5e
      now states the measured numbers and says which test cannot see this
      defect and why. Had the entry not been written, that "all four" would
      have stayed in D5d's prose unmeasured.

      Two things the entry adds that the folded prose did not have:

      - **The over-matching is concrete, not hypothetical.**
        `grep -n -B1 "Layout.preferredHeight: DTheme.hairline"` over
        `DStoaListScreen.qml` returns **six** rectangles of exactly the
        separator's width and height — lines 271, 272, 560, 691, 798 plus the
        separator — differing only in `color`. A geometry-keyed walk would
        already count all six today.
      - **The cost is named.** `objectName` is a test seam in production code,
        which is why this is a decision and not an obvious call; the entry
        argues the alternative is not "no seam" but an implicit one keyed on a
        coincidence of geometry, which cannot be deleted honestly.

      The code comment at `DStoaListScreen.qml:490-504` is extended to carry
      the measured figure and point at D5e, so someone tidying an "unused"
      property sees what it is load-bearing for. Suite re-run after restoring
      the mutation: **82 passed, 0 failed**.

## Decisions taken vs. recorded: findings not raised

Everything else asked for in the brief checked out on inspection and is not
repeated as a finding:

- **D1's withdrawal argument** (two-lamps-disagreeing, `Main.qml` binding
  `zoneState` vs. the footer's `degraded`) is recorded with the mechanism
  named, not merely asserted, and the loss (`storageText`'s verbatim core
  wording, six unset tooltips) is stated as a real cost rather than elided.
  "Navigation's version is better" is argued on two named counts, not
  asserted.
- **The three reverted judgement calls** (`qmldir`'s `DVouchStamp` record,
  PLAN.md's case-2 entry, the lamp-label exclusion in
  `tst_composer_claims.qml`) are each recorded with a reason, and each reason
  checks out against the tree: `qmldir` and `docs/PLAN.md` are unchanged from
  `origin/main`, and PLAN.md's entry 7 (storage) does not contradict a zone
  claim because zone is no longer in the case-2 list at all.
- **The `proposal.md` correction** ("does not touch `FeedScreen`" was false)
  is left in place with the correction and the lesson stated in the same
  spot ("A sibling's Impact list is a statement of intent at writing time,
  not a boundary") — an honest in-place correction, not a quiet rewrite.
- **The dropped line count** (D5c) matches CLAUDE.md's "do not write down
  anything a command can answer": the figure is removed rather than
  qualified, and the removal is argued (a qualified number is still a number
  nobody re-measures) rather than merely asserted.
- **D5d's gap disclosure** ("nothing turns red today... the suite's green
  would depend on Qt's deletion scheduling") is at the right width — it says
  plainly that nothing currently fails, states the specific mechanism that
  would silently start mattering, and does not oversell the fix as closing a
  live bug.
- **The `NO SPEC:` marker** on the row-title type scale is correctly left as
  a marker rather than folded into a Decision — D5b explains why the
  capability's own Purpose sentence forecloses contracting it here, and
  `tasks.md` correctly leaves "which capability owns the view's type scale"
  for a `spec-writer` rather than a dev resolving it unilaterally.
- **The `lib.rs:258` citation left untouched in `DIdentityChip.qml` and
  `DThreadScreen.qml`** (out of scope) is correctly not a finding — those
  files are byte-identical to `origin/main`, and fixing an unrelated stale
  citation there would be exactly the unreviewable bundling CLAUDE.md warns
  against. (Design.md's own copy of the same number is a different matter —
  see the finding above.)

Branch: `worktree-agent-aaf7eed463840ab92`. Tree is ready to prune once this
commit is cherry-picked.
