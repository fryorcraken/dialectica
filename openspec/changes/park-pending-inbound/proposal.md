## Why

The one inbound processor serves every Stoa, and today it stops and waits when it
meets a message on a channel whose open delivery has not answered yet. While it
waits, every other Stoa's messages sit behind it, the shared queue fills, and
arrivals on open channels are discarded for good. Channel identifiers are
computable, so any peer can trigger that wait. A message whose wait runs out
before delivery answers is still refused and lost. The owner has ruled that the
wait is the wrong design: a message that cannot be judged yet is **parked** in
storage, and reviewed once its channel's open settles. The processor never waits
on an open.

## What Changes

- **A message on a channel being opened is parked, not waited on.** Whether a
  message is parked, judged or refused is decided once, from its channel's state
  when the boundary takes it. A parked message is neither stored nor refused, and
  the boundary moves straight on to the next message.
- **Parked messages are reviewed on exactly three events**: delivery reports it
  holds the channel (each parked message is then judged as a fresh arrival
  would be); the channel stops being opened without being open (each is refused
  as an unknown channel); and the module's first startup in a process (parked
  messages on a channel startup is not opening are refused as an unknown
  channel). No timer and no arrival triggers a review.
- **Every message is decided exactly once**, however close to a settle it is
  taken: judged when taken, or parked and decided by exactly one review, or
  discarded under a bound. Messages on one channel are decided in the order they
  were handed over, parked or not.
- **The receive window is judged at review**, against this peer's clock then.
- **Parked messages are bounded** in count and in payload bytes, per channel and
  in total. A channel over its own bound loses the arrival; over a total bound,
  the channel holding the most loses its newest message, the arrival included,
  the count total before the byte total. An arrival that is discarded costs no
  message already parked.
- **The waiting-payloads queue takes the same discard rule.** **BREAKING** for
  the queue's behaviour: a full queue no longer always discards the arrival; it
  discards the newest payload of whichever channel holds the most, which is the
  arrival whenever one channel holds them all.
- **Parked messages are kept apart from the op log and survive a restart.** They
  keep the payload and channel identifier, never the sender identifier or the
  event timestamp, are never readable as an op, and never move a Stoa's Lamport
  clock.
- **Removed**: the bounded wait on an open, its fixed time, the ask restarting
  that time, the once-only extension, and the requirement that no record of a
  message's wait is kept. The order "this peer's wait on a creation outlasts
  delivery's own" stays; the fixed time above it goes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `op-transport`. A MODIFIED block cannot drop or rename a scenario, so two
  requirements are replaced under new names rather than amended:
  - "Every payload the reliable channel delivers passes the inbound boundary"
    becomes "A message a reliable channel delivers reaches the op log only
    through the inbound boundary": the wait paragraphs and their scenarios go,
    and the definition of a channel being opened stays.
  - "Inbound payloads waiting for the boundary are bounded" becomes "Inbound
    payloads waiting to be taken are bounded", with the
    newest-of-the-largest-channel discard rule.
  - "Nothing of a message's wait on an open is kept once it is judged" is
    removed.
  - Five requirements are added: when a message is parked; the three events
    that review parked messages; that every message is decided exactly once;
    how parked messages are bounded; and where and how they are kept.

  The two old names are cited only inside `op-transport`, by requirements this
  change replaces or removes. Code comments and test names citing them are the
  `dev-writer`'s to update.

## Impact

- `dialectica/rust-lib/dialectica-core/src/delivery.rs`: the channel book's
  per-open time, per-message waits and ask extension go; the processor parks and
  reviews; the queue's discard rule changes.
- A new persistent store for parked messages, apart from the op log. Its file or
  table is a design choice.
- Tests pinning the wait, its limit, the ask restart, the once-only extension and
  the per-message wait records are removed or rewritten against the new
  requirements.
- No new wire method and no change to the module's call surface. A status
  surface for parked messages is not part of this change.
- Left to `design.md`: the four bound values, where parked messages are stored,
  and how a review is handed to the processor.
- A live two-peer rerun is needed, since the race parking exists for occurs only
  against a real delivery node.
