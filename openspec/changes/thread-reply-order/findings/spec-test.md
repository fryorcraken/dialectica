# spec-test review — thread-reply-order (#147)

## Findings

- [x] **`spec-writer`** — `openspec/specs/thread-read/spec.md` (the live spec)
      is self-contradictory right now, and will stay that way until this change
      is archived.

      **Where:** Purpose (line 8) versus the still-unarchived Requirement *"The
      items are a flat sequence in the system's order, with the root first"*
      (lines 478–533), specifically its scenario *"A reply orders after the
      reply it answers"* (lines 516–519) and the line *"The replies SHALL
      follow in the order the system's ordering rule places them"* (line 482).

      **Measured:** commit `f2527b3` ("Spec: a thread read returns its replies
      in the reverse of the ordering rule's sequence (#147)") hand-edited the
      live Purpose line from *"This capability places posts in that order"* to
      *"places a thread's replies, after its root, in the exact reverse of that
      order"* — the #147 fix, stated as already true. It left the Requirement
      body and its scenarios untouched, because a delta carries no Purpose and
      the amended Requirement text lives only in
      `openspec/changes/thread-reply-order/specs/thread-read/spec.md` until
      `openspec archive` promotes it. Right now, in one file: the Purpose says
      replies come back in the *exact reverse* of the ordering rule's sequence,
      and forty-odd lines later the live Requirement still says *"The replies
      SHALL follow in the order the system's ordering rule places them"* and
      pins *"A reply orders after the reply it answers"* with no conditions at
      all — literally the pre-#147 scenario the GH issue names as the bug,
      still standing as the authoritative Requirement text in the live spec.

      **Scenario:** a reader consulting the live contract during this review
      window — including the in-flight `ui-thread-view` change, which
      `proposal.md`'s Impact section says cites this same requirement by name —
      gets two different answers to "which order is authoritative" depending
      on which paragraph of the same file they read. `openspec validate
      --strict` passes it regardless, since it checks heading structure only.

      This is the exact pattern `docs/OPENSPEC-ARCHIVE.md` names as invalid for
      `stoa-genesis`: *"Its live spec differs from its delta by whole added
      paragraphs... so the live file was hand-edited after the delta and no
      diff ever showed it."* That doc's own "Four traps" section exists because
      this has happened here before.

      **Severity:** significant but self-resolving — `openspec archive` will
      overwrite the stale Requirement text with the delta's, and the file
      becomes consistent again. Until then it is a live, checked-in
      capability contract that contradicts itself in the same commit that
      introduced the contradiction. Worth a decision on whether to revert the
      early Purpose edit until archive (so the live file agrees with itself
      even mid-flight), or whether editing the Purpose ahead of the delta is
      accepted practice here — right now it is undocumented practice that
      reproduces a defect already written down as a trap.

      **Outcome: fixed, by neither of the two options offered.** Reverting the
      edit until archive only moves the contradiction: a delta carries no
      Purpose, so archive would promote the reversed requirement beneath the
      old *"places posts in that order"*, and the file would contradict itself
      from then on unless the `closer` hand-edited the live spec in the archive
      commit — the `stoa-genesis` shape again, now with a step someone has to
      remember. The defect is that the Purpose restated the requirement's
      direction at all, which is a second copy of one rule inside one file.
      The Purpose line now reads *"places a thread's replies, after its root,
      in a sequence taken from that order and defines none of its own; which
      sequence is stated once, in the requirement on the items' sequence, and
      not here."* It is true of the live requirement today (the rule's own
      sequence) and of the promoted one after archive (its exact reverse), so
      the file agrees with itself at every point and archive has nothing to
      hand-edit. `proposal.md`'s two mentions of the Purpose edit say so.

## Areas checked and clean

- **Scenario coverage.** Every scenario in
  `openspec/changes/thread-reply-order/specs/thread-read/spec.md`'s MODIFIED
  requirement has a test at the right layer: the unit tests in `thread.rs`
  call `read_thread` directly, and
  `a_thread_read_over_a_store_on_disk_returns_the_root_and_its_replies` in
  `end_to_end.rs` exercises the same order claim through the wire handler over
  a real `SqliteOpLog`, computing its expected order independently from the op
  ids rather than reading it back from the implementation. No scenario is
  untestable as written, and none is covered only at a layer that cannot
  observe it.
- **Mutation (part 2).** Reverted `dialectica/rust-lib/dialectica-core/src/thread.rs`'s
  `for entry in log.iter_stoa(stoa)?.into_iter().rev() {` to drop the `.rev()`
  — i.e., the exact pre-#147 "newest first" behaviour the owner's condition
  names. Ran `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core --lib thread::tests`. Result: 7 tests failed,
  65 passed, with no collateral failures elsewhere:
  - `a_reply_orders_after_the_reply_it_answers` (the #147 regression test
    itself — the owner's stated condition is met)
  - `a_reply_carrying_a_lower_counter_than_the_reply_it_answers_comes_before_it`
  - `the_lowest_counter_leads_the_replies_and_the_highest_ends_them`
  - `replies_with_equal_counters_are_reversed_rather_than_re_sorted`
  - `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`
  - `the_root_is_the_first_item_whatever_the_logs_order_puts_first`
  - `the_sequence_follows_the_counters_and_not_the_asserted_times`

  The mutation was fully reverted with `Edit` afterwards; `git diff` on the
  file shows no residual change. This is the one mutation run for this
  review (budget of one or two); it directly settles the owner's stated
  condition rather than leaving it to inspection, so a second mutation
  (e.g. `design.md`'s own ascending-`sort_by_key` claim) was not run.
- **`NO SPEC:` markers.** Two exist in the files this piece touches
  (`thread.rs:1838`, `end_to_end.rs:1877`), but `git diff main...piece/thread-reply-order`
  shows neither line is part of this change's diff — both predate #147 and are
  about unrelated behaviour (Stoa scoping, vote effects on moderation). No new
  marked or unmarked spec gap found in the code this piece adds.
- **Requirements moved between capabilities.** Not applicable — this change
  has no `REMOVED`/`ADDED` pair; it is a single `MODIFIED` requirement within
  `thread-read`.
- **Issue freshness.** Re-read fresh with `gh issue view 147 --repo
  fryorcraken/dialectica`. The issue's fixture, its two named fixes, and its
  "Not affected" section match `design.md` and `proposal.md` exactly; nothing
  here is specified against a scope the issue no longer states.
- **Scenario name preservation.** The delta keeps the name *"The sequence is
  the ordering rule's and is not re-sorted here"* while inverting its content
  (now asserting the exact reverse), and keeps *"A reply orders after the
  reply it answers"* while adding the conditions it depends on — both
  satisfy `openspec validate`'s rule against a MODIFIED block dropping a
  scenario by name, and both are the mechanism the finding above depends on
  understanding correctly.
