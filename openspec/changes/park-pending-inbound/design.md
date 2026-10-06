## Context

`core::delivery` has three threads that touch inbound traffic: the listener
takes `channelMessageReceived` events and hands each one over to a bounded
queue, the processor puts each one through the inbound boundary, and the worker
makes every delivery call and settles each channel open as delivery answers.
Before this change, the processor waited on a channel's open for up to
`SETTLE_LIMIT` when it met a message on a channel being opened. The proposal
says why that was wrong ("Why"). The spec states the contract that replaces it:
the five parking requirements in `specs/op-transport/spec.md`.

Two constraints shape the design more than the spec text does:

- **Everything parked is attacker-controlled and unverified, and now it is on
  disk.** Channel identifiers can be computed, so any peer can put messages on a
  channel this peer happens to be opening.
- **One thread decides every message, for every Stoa.** The spec's guarantees
  about order and "exactly once" are guarantees about how that thread's takes
  interleave with the worker's settles.

## Goals / Non-Goals

**Goals:**

- Each of the spec's three triggers lives in one named function: when a message
  is parked, when parked messages are reviewed, and how a take is ordered
  against a review. A test can call each one directly.
- Remove the wait machinery entirely, not leave it unused.

**Non-Goals:**

- A status surface for parked messages (#151's question).
- Recovering a review the module stopped part-way through. The spec leaves it
  out of scope, and Decision 6 says what it costs.
- Automating the live two-peer rerun the issue asks for. It is still a manual
  step (Risks).

## Decisions

### 1. Park the message instead of waiting. This reverses `delivery-wiring` Decision 11

`delivery-wiring`'s design (archived at
`changes/archive/2026-10-06-delivery-wiring/design.md`), Decision 11, considered
and **rejected** "a per-channel hold: park the pending channel's messages aside
and let the processor carry on with others". Its reasons were that the hold
"needs a second store of messages with its own bound, its own discard rule and
its own ordering relative to the main queue — a second copy of Decision 10's
reasoning, for a stall `SETTLE_LIMIT` bounds". The owner overruled it on issue
#199, and this change reverses it. Each of those three reasons turned out to be
a cost the wait paid anyway, or one this design does not pay:

- **The stall was not really bounded, and it could be triggered by anyone.** One
  processor serves every Stoa, so while it waited, every other Stoa's messages
  sat behind it. Once the 256-slot queue filled, arrivals on open channels were
  discarded for good. Channel ids are computable, so anyone could put a message
  on a channel this peer was opening and trigger the wait. The bound was up to
  40 s, or under 80 s when the worker asked delivery during the wait. Five
  review rounds on #190 went into bounding it rather than removing it:
  - a wait per open instead of per message;
  - the open's time restarting when delivery is asked;
  - a once-only extension per message;
  - a guard so an ask does not revive a wait that had ended;
  - a book of per-message waits that had to be cleaned up on every exit.

  All of that existed only because the processor waited.
- **The wait still lost messages.** A message whose wait ran out before
  delivery answered was refused and lost. Losing that message is the very
  failure the wait existed to prevent.
- **"A second copy of Decision 10's reasoning" is avoided by using one rule.**
  The parked messages' total bounds and the waiting queue share one shedding
  rule and one function (Decision 5). The queue's bound question is the one
  `delivery-wiring`'s Open Question 6 left to the owner, and the issue asked
  for one answer covering both.
- **"Its own ordering relative to the main queue" is solved by locking**
  (Decision 3), not by a second ordering rule.

`ChannelBook::on_take` is the parking trigger. It reads the channel's state
once, under the lock the payload was taken under, and returns `Taken::Park`
only when the channel is not open and is being opened. These tests go red
without it:

- With `Judge` always: `a_message_on_a_channel_being_opened_is_parked_not_judged`
  and every review test.
- With `Park` for an open channel too:
  `a_message_on_an_open_channel_does_not_wait_on_a_repeated_open` and
  `only_a_channel_being_opened_and_not_open_parks_what_is_taken_on_it`.

**Removed with the wait:**

- `SETTLE_LIMIT`, `OpenTime`, `Wait`, `WaitId` and `Pending`.
- `Channels::await_settled`, `Channels::asked` and `Opening::asked`.
- The processor's `settle_limit` field.
- The compile-time order `CALL_TIMEOUT < SETTLE_LIMIT`.
- 25 tests from `delivery/tests.rs`, which pinned the wait, its limit, the ask
  restart, the once-only extension and the per-message wait records. Five of
  the scenarios they stated survive in the new spec, and are rewritten against
  parking in `delivery/tests/parking.rs`: a message stored once its channel is
  reported created, one refused once its open is declined, a message on a
  channel not being opened, a message waiting for the last of two requests,
  and the clock read when the message is judged.

What is kept is `DELIVERY_CALLBACK_TIMEOUT < CALL_TIMEOUT`, as an assert and
as `this_peers_wait_on_a_creation_outlasts_deliverys_own`. The worker still
must not give up on a creation that delivery would still answer, or that
channel's parked messages are refused while delivery holds it.

### 2. Parked messages live in `parked.sqlite`, a file of their own

The issue requires "its own table or store, not the op log. Nothing unverified
may be readable as an op." The table is `parked(seq INTEGER PRIMARY KEY,
channel TEXT, payload BLOB)`, indexed on `(channel, seq)`, and stamped with
layout version 1. It has no column for the sender identifier or the event's
timestamp, so neither can be stored. Only the processor thread opens the file,
once per park or review, for the reason `delivery-wiring` Decision 13 gives for
the other stores. Schema creation still uses the op log's `BEGIN IMMEDIATE`
shape, so a second thread opening the file later does not reopen the race.

`seq` is a rowid alias. A new row is numbered past every row present, so among
the rows held, a larger `seq` was handed over later. Both orderings need only
that: the hand-over order a review decides in, and "newest" for shedding.

Alternatives considered:

- **A table in `ops.sqlite`.** The op log's `check_layout` names its own
  columns, so a new table needs a layout version bump that makes every
  existing store unopenable. And it would put unverified bytes one careless
  `JOIN` away from every read of the op log.
- **A flag on rows in the op log.** This breaks "nothing unverified may be
  readable as an op" at the root: every read would need a filter it could
  forget.
- **Memory only.** The spec requires parked messages to survive a restart.

A parked op is not visible to the op log, does not move a Stoa's Lamport clock,
and is not "already held". All three hold because the op log never sees these
rows, not because some check enforces them.
`a_parked_op_is_not_readable_as_an_op_and_does_not_move_its_stoas_clock`
observes it.

### 3. How a review reaches the processor: one lock, and reviews before payloads

The spec requires that "the taking of a message and an event that begins a
review of its channel MUST be ordered, one before the other". It also requires
that a review decides all of a channel's parked messages "before any message on
that channel taken after the event … is judged". Two choices make both true by
construction:

- **One mutex (`Channels::shared`) covers the channel book, the waiting
  payloads and the waiting reviews.**
  - `Channels::settle` changes the book and queues the review that
    `ChannelBook::settle` returns, in one critical section.
  - `Channels::take` pops a payload and reads its channel's state in another.

  So every take happens wholly before or wholly after every event.
- **`Channels::take` returns a waiting review before any waiting payload.**
  Consider a payload handed over before a settle and taken after it. It reads
  `Taken::Judge`, so the review for the parked messages must already have run.
  Since the processor is one thread, a payload parked before the event is
  written to the store before the processor takes the review that reads it.

`ChannelBook::settle` is the review trigger for events 1 and 2:

| The request settled | What `settle` returns |
|---|---|
| Held | `Review::Held` |
| Not held, it was the last unsettled request, and the channel is not open | `Review::Unopened` |
| Anything else | No review |

`ChannelBook::startup_review` is the trigger for event 3. `Delivering::start`
queues it after counting startup's opens and before subscribing, so it is the
first thing the processor runs. Its set of known channels is taken at the
event, as the requirement reads.

Alternatives considered:

- **A review marker in the payload FIFO.** A payload handed over before the
  settle but taken after it would be judged ahead of the review, which breaks
  the order clause. Markers would also count against the payloads' bound and
  could be shed by it.
- **The worker runs reviews as it settles.** Two threads would then decide
  messages, and ordering their decisions against the processor's takes needs
  another lock. The worker's delivery calls would also queue behind op-log
  appends.
- **Two locks, book and queue, as before.** A settle could land between the
  processor popping a payload and reading its state.

These tests go red if each part is removed:

- Taking payloads before reviews:
  `a_review_is_taken_before_a_payload_that_was_waiting_when_its_event_came`
  and `parked_messages_are_decided_before_later_messages_on_their_channel_in_hand_over_order`.
- A settle that queues no review: every review test.

The issue's "Order" question has this answer:

- Parked messages are reviewed in hand-over order.
- They are decided before anything taken on their channel after the event.
- Messages that arrive after the settle take their turn in the waiting queue.

### 4. The four bound values

These are the values chosen (`parked::PARK_BOUNDS`):

| Bound | Value | Why |
|---|---|---|
| Messages per channel | 256 | Matches `INBOUND_BOUND`: one channel may park as many messages as could wait to be taken at once |
| Bytes per channel | 8 MiB | A review reads one channel's parked messages into memory at once, so this caps what a review holds: 54 payloads at the 150 KiB limit. For ordinary posts the count binds first |
| Messages in all | 1024 | Four channels at their own bound |
| Bytes in all | 32 MiB | Four channels at their own bound, on disk |

Parked messages exist only while an open is in flight. The total bounds what a
peer opening several Stoas at once can be made to hold, for example at a
restart.

The ordering the spec requires is held at compile time: each per-channel bound
is at most its total, and each byte bound is at least 150 KiB. The compile-time
check cannot see a lowered `MAX_MESSAGE_BYTES`. The values themselves are not
measured: how much honest traffic one open catches has not been observed live.
A bound that is too small shows up as "discarded from the parked messages"
lines, each with the running count.

The processor carries its bounds as a field that `Processor::new` sets to
`PARK_BOUNDS`. That is the lesson of the wait's limit, which was set wrongly in
`start` and passed every test. Three tests guard it:

- `the_processor_the_running_wiring_builds_parks_by_the_pinned_bounds` reads
  the field.
- `the_running_wiring_builds_its_processor_with_the_one_constructor_and_never_changes_its_bounds`
  pins the field by reading the source as text.
- `parked::tests::the_park_bounds_are_pinned` pins the values. `cargo mutants`
  does not mutate a `const`.

### 5. One shedding rule, shared by the queue and the parked messages

`parked::shed` returns the channel holding the most, by whatever the bound
counts. Among channels tied for the most, it returns the one whose newest
message was handed over latest. The arriving message is counted with its own
channel, as that channel's newest, which makes it the newest of all.

The spec says the arrival's own channel is chosen "if it is among them". With
the arrival counted as newest, that is the same tie-break, not a second rule:
the arrival's channel always has the latest newest. When the arrival's channel
is chosen, its newest message is the arrival.

- `InboundQueue::offer` sheds this way by count.
- `plan_park` sheds by count, then by bytes, after the per-channel check.
  `plan_park` is a pure function over the held rows, and the store applies it
  in one transaction.

This changes the queue's rule from "discard the arrival" to "discard the newest
payload of the channel holding the most". When one channel holds everything, it
is the old rule exactly. The old reason for it still applies within the
flooding channel: a backlog arrives roughly oldest first, so the newest messages
are leaves of their threads. What changes is that a flood on one Stoa no longer
costs another Stoa its arrivals. Tests:
`a_full_queue_discards_the_newest_payload_of_the_channel_holding_the_most` and
`the_arriving_payloads_channel_loses_a_tie_for_the_most_waiting`, beside the
store's own tests in `parked.rs`.

### 6. Every message decided once: a review takes its messages out in the same transaction that reads them

`ParkedStore::take_channel` selects a channel's rows and deletes them in one
`BEGIN IMMEDIATE` transaction, then returns them. After that, the review decides
each one. No later review can read them again. If the read fails, nothing is
deleted and they stay parked for the channel's next review, as the spec
requires.

The alternative was to decide each message and then delete it. A failed delete
after a successful store would decide the message twice. **The cost of the
choice:** a module that stops part-way through a review loses what it had taken
out. The spec puts that case out of scope, and the alternative would re-decide
those messages instead.

`a_message_a_review_decided_is_not_decided_again` and
`parked::tests::a_review_takes_a_channels_messages_in_hand_over_order_and_leaves_none`
go red if `take_channel` does not delete.

### 7. Expiry is an event, not a time

The issue asks how long a pending message is kept when its channel never
opens. The answer is that it is kept until the channel's last request settles
unopened (review 2), or until the next startup if the module stops first
(review 3). The spec's "three events and no others" rules out a timer, and a
timer would bring back the original defect: a slow open costing an honest
message. A parked message cannot outlive the open it is waiting for by more
than one process lifetime, and the bounds cap how much can be waiting.

### 8. The receive window is judged at review

This is the spec's answer to the issue's question. `Processor::pass` reads the
clock as it judges, and a parked message is judged by its review. A long park
can therefore admit an op that was more than an hour ahead when it arrived. It
can refuse an op that was within the hour only if this peer's clock moves back.

`parked_inbound` gives the boundary an empty sender identifier and a zero
timestamp, because none was kept. The boundary decides nothing from either.

### 9. The worker settles an open before it logs the outcome

`Opening::settle(held)` settles the guard explicitly, and the worker calls it
before writing "channel open", "NOT open" or "no sender identifier could be
retained". So once a line saying an open was answered or given up can be read,
the review it began is already queued and the channel is no longer being
opened. Without this, a message sent "once the log has recorded" a given-up
open could still be parked for a moment, and "nothing is parked afterwards"
would depend on timing.
`an_open_this_peer_gives_up_without_asking_delivery_leaves_nothing_parked`
relies on it.

The `Drop` guard stays as the fallback for every other path: a panic, a worker
that is gone, or a worker that never started.

### 10. The processor runs for the life of the module, and the listener's end closes nothing

Before this change, the listener closed the queue when delivery's events ended,
and the processor drained it and stopped. Now a review can still be due after
that point: an open answered after the subscription ended has parked messages
waiting for it. So the processor keeps taking. `Channels::close` exists for
tests only, so that a processor run on the test thread can end.

### 11. Tests hold the boundary up with the module's log, not with an open

Several queue scenarios need "the boundary held up deciding one payload". An
unanswered open used to do that. Now it parks and moves on. The test
`Recorder::hold_on` makes the processor wait at a gate after it records a
chosen line: junk on an open channel, refused as undecodable. This is a test
fixture only. No production code changed for it.

## Risks / Trade-offs

- [Up to 32 MiB of attacker-chosen bytes on disk while an open is in flight]
  → The bounds cap it, a channel's share goes when its open settles, and the
  next startup clears channels it does not open. The file holds no sender
  identifier or timestamp, so it records nothing about who sent what.
- [A review holds up to 8 MiB in memory] → The per-channel byte bound. A
  startup review takes one channel at a time.
- [A module stopping part-way through a review loses what it took out]
  → Decision 6. Out of the spec's scope.
- [Every held answer opens `parked.sqlite`, even with nothing parked] → One
  SQLite open per channel answer: a few per join or startup.
- [A later build with smaller bounds could restore a store over them] → A
  per-channel excess discards arrivals until a review clears the channel, and
  the total loops evict until the totals hold again on the next park. Only a
  change to `PARK_BOUNDS` can reach this.
- [A failed park drops the message] → The spec requires it ("not held for
  another attempt"). It is logged as a storage failure.
- [A message on a channel no request has asked for is still refused on
  hand-over] → Unchanged and by design. "Being opened" starts at the request,
  and startup counts its channels before it subscribes.
- [The race this change exists for only happens against a real delivery
  node] → `cargo test` drives a fake. A live two-peer rerun under
  `lgs basecamp launch` is still needed, as it was for #176. `tasks.md` 4.3
  says so.

## Migration Plan

`parked.sqlite` is created on first use. No existing file changes. Rolling back
to a build without parking leaves the file on disk, unread. The messages in it
are then lost to that build, which is no worse than that build's own behaviour.

## Open Questions

- **The bound values are judgements.** Decision 4 says what each bounds. A live
  run with real backlog sizes could move them without changing the spec.
