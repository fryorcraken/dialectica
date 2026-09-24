## Context

`read_thread` (`dialectica-core/src/thread.rs`) takes `OpLog::iter_stoa`'s
sequence, which is already in `cmp_ops` order, and filters it into a thread
page. It moves the root to the front and keeps everything else in the order it
arrived in. `cmp_ops` puts the **higher** counter first, so the replies came
back newest first.

That breaks `thread-read`'s scenario *"A reply orders after the reply it
answers"*. **The defect was found by running the code, not by reading it.** The
dev-writer for #100 (the archived `reply-count` change, `design.md` Risks,
"Found while implementing, outside this change") built a root at counter 1, a
reply to it at counter 2, and a reply to that reply at counter 3.
`read_thread` returned `[root, counter 3, counter 2]`. No test pinned the
scenario, so the mismatch shipped with every gate green.

#147 offered two fixes: reverse the thread read's replies, or amend the scenario
if newest first was the intended presentation. The owner chose the first on
2026-09-24 (see `proposal.md`, Why). This design is about how to reverse without
creating a second ordering rule.

## Goals / Non-Goals

**Goals:**

- The replies come back in the exact reverse of the rule's sequence. The root
  stays first.
- `thread.rs` still contains no comparison. The order still comes from one
  place, `cmp_ops`.
- The test pinning the order fails against newest first. That was a condition
  of the owner's decision.

**Non-Goals:**

- Changing `cmp_ops` or `op-ordering`. See Decision 2.
- Changing the feed's `latestReply`. See Decision 4.
- Fixing the pre-existing `cargo fmt` drift in `identity.rs` and `wire.rs`.
  It is on `main` already and touches nothing here.

## Decisions

### 1. Walk the rule's sequence backwards; do not sort the replies again

The loop in `read_thread` now reads `log.iter_stoa(stoa)?.into_iter().rev()`.
That is the whole behaviour change. Each loop iteration (verify, check the kind,
walk the chain, resolve the item) looks at one entry and nothing else. So the
direction of the walk changes where each reply lands and nothing more. The root
is still moved to index 0 by `items.insert(0, …)`, and `Placed::at` still
assigns positions after that move. Positions therefore follow the new order with
no further change.

Alternatives considered:

- **Sort the replies by ascending counter.** This is the obvious way to write
  "oldest first", and it is wrong in two places. The rule's tiebreaks are not
  symmetric: equal counters fall back to ascending op id, and ops without a
  counter go after every op that has one. An ascending re-sort that keeps the
  ascending op-id tiebreak puts equal counters in the wrong order, and it puts
  counter-less ops in ascending order when they should be descending. The
  result is a second ordering rule inside `thread.rs` that disagrees with the
  first one. Two peers running two rules still agree with each other, so
  nothing reports an error.
- **`sort_by(|a, b| cmp_ops(b, a))`.** This gives the same result, because
  `cmp_ops` is a total order and the log holds each op id once. But it puts a
  comparator call into a module whose header says it has none. It also re-sorts
  a sequence the store has already sorted. Reversing costs O(n) and leaves the
  "no `sort`, no `cmp`" rule true.
- **Collect the replies apart from the root and call `.reverse()` on them.**
  This is correct, but it needs a second buffer and a separate root variable.
  Walking backwards does the same job in one line and keeps the root handling
  as it was.

**This is a guard. Measured:** swapping the backwards walk for
`sort_by_key(|e| (counter, id))` (an ascending re-sort with the ascending
op-id tiebreak) turns exactly three `thread.rs` tests red:

- `replies_with_equal_counters_are_reversed_rather_than_re_sorted`
- `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`
- `the_root_is_the_first_item_whatever_the_logs_order_puts_first`

`the_lowest_counter_leads_…` and `a_reply_orders_after_…` stay green under that
mutant, because on distinct counters the two orders agree. That is why the
tie and counter-less scenarios exist.

### 2. The rule stays as it is; only this reader reverses it

Changing `cmp_ops` to put the lowest counter first would fix the thread read
and break every other reader of the rule. `revision::current_version` takes
the first valid revision in the rule's order as the current version.
`moderation::resolve` is decided by the leading binding op, and its one
exception, the `Hide` preference, applies only to ops without a counter. The
feed's `latestReply` is defined as the rule's "places first". So the rule
keeps its direction and the thread read presents it in reverse. The spec
states the same thing: *"This capability MUST NOT define an order of its own"*.
Reversing the rule's sequence does not define a new order.

### 3. The asserted-times test is rebuilt so its fixture can tell the orders apart

`the_sequence_follows_the_counters_and_not_the_asserted_times` had three replies
whose asserted times ran exactly opposite to their counters. That meant the
counter order was always the same as one of the two time orders. Before #147 it
was the same as ascending time. After the reversal it would have been the same
as descending time. **So the test could never tell "by counter" from "by time"
in one of the two directions.** It is the recorded fixture-defect family again:
two explanations that give the same answer.

The new fixture gives counters 1/2/3 the times 2025/2026/2024. With those
times, the counter order, ascending time and descending time are three
different sequences. The test also asserts that its counter answer differs from
both time orders, so an edit that makes two of them coincide again fails there,
under its own name.

### 4. `latestReply` is unchanged, and afterwards it is the thread read's last reply

This passage comes from #147's "Not affected" and from the `reply-count` design's
Risks. The feed's `latestReply` is defined by `op-ordering`'s "places first",
which is `cmp_ops`'s first entry, the highest counter. It did not depend on how
#147 was resolved and it is not touched here. After this change the feed's
latest reply is the **last** reply in the thread read. Each is still correct
under its own spec. `the_count_agrees_with_the_thread_read` in `feed.rs` compares
membership and not position, so it is unaffected. It passes unchanged.

## Risks / Trade-offs

- **[A reply can come before the reply it answers]** Reversing the rule puts an
  answer after its parent only where the answer's counter is the greater, and
  two cases break that:
  - *No counter.* A reply from an older build is placed by descending op id,
    which is a hash, ahead of every counted reply, so it comes before a counted
    parent. Between two counter-less replies the hashes decide: the on-disk test
    `a_thread_read_over_a_store_on_disk_returns_the_root_and_its_replies` derives
    its expectation from the ids for that reason.
  - *An over-bound parent.* This build's publish path can produce it too.
    `clock_from_counters` (`arrival.rs`) holds an op whose counter is more than
    `ADVANCE_BOUND` above the clock but does not advance the clock to it, and
    `cmp_ops` still orders the op by that counter. An honest peer answering it
    signs one above its own clock, which is lower. Any author can reach this by
    choosing a counter over the bound, and their reply then sits last in the
    thread, after every answer to it. The bound, and ordering by an over-bound
    counter, are `op-ordering`'s; this change only reverses what they produce.

  → Accepted, and specified: `thread-read` states the conditions *"A reply
  orders after the reply it answers"* depends on, requires the reversed
  sequence regardless, and forbids moving a reply after its parent (scenario
  *"A reply carrying a lower counter than the reply it answers comes before
  it"*). A parent-aware fix-up would also have been a comparison inside
  `thread.rs`, the second ordering rule Decision 1 exists to prevent.
- **[A long thread's newest replies are on its last page]** Oldest first means
  page 0 holds the root and the earliest replies. The read reports `hasMore` but
  no total, so a reader who wants the newest replies has to page to the end. →
  This follows from the owner's decision. Pagination is unchanged and out of
  scope (`proposal.md`).
- **[Callers relying on the old sequence]** A breaking change in reply order
  only. → The in-flight `ui-thread-view` spec renders "the order the read
  returned", so a view built to it needs no change.

## Migration Plan

None needed. No op format, stored state, request shape or wire field changes.
Only the order of replies in a thread read changes. Rolling back means reverting
the code.
