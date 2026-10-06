# Findings: correctness (`code-reviewer`)

Dimension reviewed: **correctness only**. Security, readability and architecture
were not reviewed by this instance.

Reviewed against `specs/op-transport/spec.md`: `parked.rs` and `delivery.rs`
in full, plus `transport::receive_via` and `judge`, which the review path calls.

- [ ] **`dev-writer`** — `parked.rs:407-435` (`plan_park`) — a message already
      parked is discarded to make room for a payload that is then discarded too.
      **Scenario:** bounds per-channel count 5, per-channel bytes 100, total
      count 3, total bytes 100. Channel `a` holds three 30-byte messages. A
      90-byte payload arrives for `b`. The count loop finds `a` (3) holds more
      than `b` (1 with the arrival) and evicts `a`'s newest. The byte loop then
      finds `b` (90 bytes) holds more than `a` (60) and returns `park: false`,
      keeping the eviction. Probed: `Parking { parked: false, evicted: 1 }`, and
      `a` is left with 2. One park attempt costs two messages, though the arrival
      is the one given up, and the eviction freed room nothing uses. The spec
      says "while parking it would put the parked messages over a total bound"
      and "when the message chosen is the payload being parked, it is discarded
      and nothing more is". The first reads against evicting for a payload that
      is not parked. The second can be read as covering only the bound being
      restored at that moment. **Severity:** low (needs both total bounds near
      full and one channel dominating bytes while another dominates count), but
      it is a silent loss of a message, which this change exists to prevent.
      Either plan both bounds before applying any eviction, or have the spec
      state that evictions already made stay. **Measured:** no test exercises a
      count eviction followed by a byte-bound discard of the arrival; the probe
      above was written for this review and is not in the tree.

## Areas checked and clean

**When a message is parked.** `ChannelBook::on_take` is the only decider and
`Channels::take` is its only caller. It is asked once per payload, under the
lock the payload was popped under, and returns `Park` only for not-open and
opening. A message refused at hand-over (`refused_on_hand_over`) never reaches
the queue, so it can never be parked. The open-with-a-repeat-request case
judges, as the spec requires.

**Which events review.** `Review` is constructed in exactly two places,
`ChannelBook::settle` (events 1 and 2) and `ChannelBook::startup_review`
(event 3). The request counter is balanced, because `Opening` has one
constructor, is not `Clone`, and settles in `Drop`. I found no way to make a
second review from time, a send, a repeat join, an arrival or a take. A decline
while another request is unsettled returns no review. A decline of a repeat for
an open channel returns none either. Held returns a review whatever else is
unsettled, as event 1 says. `begin_startup_review` is queued before the
subscription and before any worker open, so it is first in the queue, and a
second `start` returns before reaching it.

**Exactly once, none lost, none twice.** The book, the waiting payloads and the
waiting reviews are one mutex, and `take` returns a waiting review before any
payload. A payload taken before an event and parked is therefore decided by that
event's review, and one taken after reads the post-event state. `take_channel`
reads and deletes in one `BEGIN IMMEDIATE` transaction, so no later review can
see a row a review has taken. A failed read deletes nothing. Evictions are
deleted rows, so no review sees a discarded message. The processor is the only
thread that opens `parked.sqlite`, so there is no concurrent writer. I looked
for an interleaving that decides a message twice or drops one, and found none
outside the spec's own out-of-scope case (a module stopping mid-review).

**Order.** Parked rows come back `ORDER BY seq`. `seq` is a rowid alias without
`AUTOINCREMENT`, so a deleted top row's number can be reused. A new row is
always numbered past every row still held, which is all the order and the
shedding rule use. The waiting queue's shed removes an element and appends, so
FIFO order is kept.

**Shedding.** `shed` with the arrival counted as the newest of its own channel
gives the spec's tie-break as one rule, as the design claims. The queue rule and
the per-channel and total rules match the text, apart from the entry above.

**Reachable panics.** None on a path from untrusted input. No indexing or
`unwrap` outside `#[cfg(test)]` in `parked.rs`. `delivery.rs` locks through the
poison-tolerant `lock`, and `*requests -= 1` is guarded by `> 1`. Each
decision, park and review runs under `catch_unwind`.

**Window clock.** `Processor::pass` reads the clock when the message is judged,
so a parked message is judged at its review. The event timestamp is unused. The
parked path passes an empty sender identifier and a zero timestamp, and
`receive_via` and `judge` read neither.

**Mutation.** `cargo mutants --in-place`, scoped to the changed files. Both runs
finished; each took longer than the overlay's couple of minutes (5 and 8
minutes), but neither was abandoned.
- `parked.rs`: 50 mutants, 35 caught, 15 unviable, **0 missed**.
- `delivery.rs`, restricted with `--re` to the parking and review functions
  (`on_take`, `settle`, `startup_review`, `take`, `offer`, `review`,
  `refuse_parked`, `take_parked`, `park`, `hand_over`, `count_discard`,
  `Opening`, `is_known`): 46 mutants, 36 caught, 10 unviable, **0 missed**.
- Not mutated: the rest of `delivery.rs` (node, send, worker, notes), because of
  the time budget. Not mutable: `const` values, and the adapter in
  `src/lib.rs`, which `cargo test` does not compile.
- `cargo test -p dialectica-core --lib delivery`: 138 passed. I did not run
  `cargo clippy` or the full CI `rust` job.

## Observations that need no action

- The listener logs a waiting-queue discard after it releases the lock, and the
  processor does the same for parked discards. Two lines can reach the log in the
  opposite order to their running counts. Every count is still unique and
  correct.
- A payload taken as `Judge` on a channel that was neither open nor opening is
  looked up again in `pass`. A join answered held in the microseconds between the
  take and the judgement would store it instead of refusing it. The result is the
  harmless one, and the window is narrow.
- A park line and a parked-discard line both contain the word "parked". They are
  worded differently, which is what the spec asks for. A log filter keyed on that
  one word would catch both.
- If the startup review cannot read the store, stale rows on a channel the peer
  never opens stay until the next process's startup. That is the spec's own rule
  ("decided by the channel's next review").
- `Delivering::start` ignores `spawn`'s result for the processor, as it did
  before this change. A processor that fails to start would leave every message
  undecided and every review unrun. The failure is logged.
