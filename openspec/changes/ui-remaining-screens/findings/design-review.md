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

- [ ] **`dev-writer`** — `design.md:85` — a stale line-number citation, in
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

- [ ] **`dev-writer`** — `design.md` (no entry) — the choice to key the
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
