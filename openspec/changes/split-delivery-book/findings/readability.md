# Findings — readability (code-reviewer)

- [ ] **`dev-writer`** — `openspec/changes/split-delivery-book/design.md`, Decision 9 — "Five links" is six
      **What is wrong:** the decision opens "Five links in the moved docs pointed
      at items that stay in `delivery.rs`" and then names `channel_answer`,
      `Opening`, `Action::Open`, `Processor::review`, `refused_on_hand_over` and
      `Channels`. That is six targets, and six reference definitions are added
      (`book.rs` lines 43-45 and 84, `queue.rs` lines 28 and 62).
      `tasks.md` 2.3 and 3.4 (four and two) and the PR body's item 3 list six too.
      **Scenario:** a later reader counting definitions against the decision finds
      one more than it says and cannot tell whether a link was added unrecorded.
      **Severity:** low; a prose count, a genuine inaccuracy, not a style choice.
      **Measured:** `git grep -n -F "]: super::"` over `book.rs` and `queue.rs`
      gives six lines.

- [ ] **`dev-writer`** — `delivery.rs:963` — the heading `// ─── Inbound: the bounded queue ───` now heads no queue
      **What is wrong:** the queue left, and the heading now sits over `Arriving`
      and `parked_inbound`, so a reader scanning for the queue in this file lands
      under a heading that promises it and finds the event type instead.
      **Scenario:** `delivery.rs` is searched by section heading for "the
      queue"; the heading leads to `Arriving` and a one-line helper, and the
      queue is in `queue.rs`, which nothing here says.
      **Severity:** low; a stylistic preference, not a defect. It is already
      recorded in `design.md` Risks and in the PR body, and rewording it is not on
      the owner's closed list of residue, so the fix is the owner's to allow;
      rejecting it with that argument is a fair outcome.

Clean, in prose:

- The two `//!` headers say what the file holds and that the lock is
  `Channels`'s, as decision 7 asks, in five lines each.
- The reference-definition approach keeps every moved doc line byte-identical, and
  the definitions sit at the foot of the one doc comment that uses them.
- `#[cfg(test)] use std::collections::HashMap;` has no comment saying the tests
  take it through the glob. It matches the `#[cfg(test)] use
  crate::parked::ParkBounds;` beside it, which has none either, and removing it
  fails compilation of `delivery/tests.rs` at once, naming the type, so the next
  person is told rather than misled. Not worth a box.
- `rustfmt --check --edition 2021` reports no diff in `book.rs` or `queue.rs`.
  It does report drift in `delivery.rs` and `delivery/tests.rs`, at lines this
  piece did not touch (the CI formatting step cannot see this crate, as the
  overlay records); it is not this piece's to fix.
