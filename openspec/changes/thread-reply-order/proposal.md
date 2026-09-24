## Why

A thread read returns a reply before the reply it answers (#147). `thread-read`
contracts two scenarios that cannot both hold: *"A reply orders after the reply it
answers"*, and *"The sequence is the ordering rule's and is not re-sorted here"* —
and `op-ordering`'s rule places the **higher** counter first, so a reply written
after another is placed ahead of it. The code followed the second scenario, no test
pinned the first, and the mismatch shipped with every gate green.

The owner decided (2026-09-24): **a thread's replies read oldest first**, in the
causal sense the counter gives — a reply comes after what its author had seen. The
code changes to meet *"A reply orders after the reply it answers"*, and the
scenario that contradicts it is the one amended.

## What Changes

- The thread read's replies follow the **exact reverse** of the sequence
  `op-ordering`'s rule places them in: the reply the rule places last comes first,
  and the one it places first comes last. The root stays the first item of the
  first page.
- The read still defines no order of its own and compares no value itself. It
  reverses the rule's sequence rather than applying a second comparison, so two
  peers holding the same ops still return the same sequence.
- Two consequences of the reversal are stated explicitly, because a re-sort by
  ascending counter would get them wrong: replies with equal counters come in
  **descending** op id, and a reply carrying no counter comes **before** every
  reply that carries one.
- *"A reply orders after the reply it answers"* states what it depends on: both
  replies carry counters, and the answered reply's counter advanced the
  answering peer's clock. Where either carries no counter, or the answered
  reply's counter exceeded `op-ordering`'s advance bound, the answer can carry
  the lower counter and come first; the read returns the reversed rule's
  sequence regardless and does not move a reply after its parent. A scenario
  pins that case.
- The scenario *"The sequence is the ordering rule's and is not re-sorted here"*
  keeps its name and now asserts the exact reverse of the rule's sequence;
  `openspec validate` refuses a MODIFIED block that drops a scenario by name. The
  requirement it sits in keeps its heading, which the in-flight `ui-thread-view`
  change cites.
- `thread-read`'s Purpose, which said this capability "places posts in that
  order", is corrected in place, since a Purpose is not carried by a delta.
- **BREAKING** for any caller relying on the old reply sequence. The in-flight
  `ui-thread-view` change's spec renders "the order the read returned", so a view
  built to it follows without change.

Out of scope:

- The feed's `latestReply` keeps `op-ordering`'s "places first" — the highest
  counter — and is unchanged. It becomes the **last** reply of a thread read, and
  both remain correct under their own specs.
- `op-ordering`'s rule, and every other reader of it (`post-revision`,
  `moderation-resolution`, `feed-read`), are unchanged.
- Pagination, positions, hiding and the root's placement are unchanged in
  behaviour; only which replies fall on which page follows from the new order.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `thread-read`: the requirement *"The items are a flat sequence in the system's
  order, with the root first"* changes the reply order from the ordering rule's
  sequence to its exact reverse, amends the contradicting scenario, and adds
  scenarios pinning the order against newest-first, against a re-sort on ties,
  for replies carrying no counter, and for an answer carrying a lower counter
  than the reply it answers. *"A reply orders after the reply it answers"*
  gains the conditions it depends on. The Purpose line is corrected in the live
  spec directly.

## Impact

- `dialectica/rust-lib/dialectica-core/src/thread.rs` — `read_thread`'s reply
  sequence; existing tests that assert the log's relative order among replies
  (`the_root_is_the_first_item_whatever_the_logs_order_puts_first`,
  `the_sequence_follows_the_counters_and_not_the_asserted_times`) assert the old
  direction and change with it.
- The wire reply of the thread read (`wire.rs`) changes in reply order only; no
  field, shape or error changes.
- No op format, stored state or request shape changes.
