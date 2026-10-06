# Design review: park-pending-inbound

Read against issue #199 (read fresh), `design.md`, `parked.rs` and `delivery.rs`.

**In good shape.** Every question under the issue's "What the design has to
settle" has a Decision: bounds (4, 5), expiry (7), receive window (8), order (3),
where it lives (2), what is removed (1). The code takes each one: `ChannelBook::on_take`,
`settle` and `startup_review` match the Decision 3 table; the single `Channels::shared`
lock and "review before payload" in `Channels::take` are as recorded; `take_channel`
reads and deletes in one `BEGIN IMMEDIATE` transaction (6); the worker settles
before it logs (9); `Processor::new` is the one constructor and sets `PARK_BOUNDS` (4);
`PARK_BOUNDS` holds the recorded values and the four compile-time asserts exist.
The removed names (`SETTLE_LIMIT`, `await_settled`, `OpenTime`, `WaitId`,
`settle_limit`, `asked`) no longer appear in the tree, and the "25 tests removed"
figure matches (25 `#[test]` removed from `delivery/tests.rs`). Decision 1 argues
the reversal of `delivery-wiring` Decision 11 point by point. No code contradicts a
recorded decision, and nothing contradicts the issue.

What follows is gaps and thin entries, not contradictions.

## Decided in the code, not recorded

- [ ] **`dev-writer`** — `parked.rs:384-436` (`plan_park`) — a park can evict a
      held message and then discard the arrival anyway. The count loop evicts the
      newest message of the channel holding the most; the byte loop then runs, and
      if `shed` picks the arrival's own channel it returns `Plan { park: false, evict }`
      with the count loop's evictions still in `evict`. `ParkedStore::park` applies
      them, so an honest parked message is deleted to make room for an arrival that
      is not parked.
      **Scenario:** total count full across four channels, total bytes nearly full,
      a 150 KiB arrival on a fifth channel. Count loop evicts channel A's newest
      (small); byte loop finds the arrival's channel holds the most bytes and
      discards the arrival. A is one message poorer for nothing.
      Spec text ("When the message chosen is the payload being parked, it is
      discarded and nothing more is") can be read either way, and Decision 5 says
      nothing about it. Either record it as accepted, with the cost, or make
      `plan_park` apply no eviction when the arrival is finally discarded. No test in
      `parked.rs` pins which (the byte test `over_the_total_bytes_...` evicts and
      parks). **Verified by reading**; I did not run a case.

- [ ] **`dev-writer`** — `design.md` Decision 5 — the per-channel rule is not
      argued. `plan_park` discards the arrival when its own channel would exceed
      the count or byte bound, and evicts nothing. The issue asks for "a stated rule
      for what is dropped when it is full"; Decision 5 covers only the totals. The
      alternatives (drop the channel's oldest to keep the freshest, or its newest)
      and what ruled them out are absent. The queue's "a backlog arrives roughly
      oldest first, so the newest are leaves" argument is made for the queue; say
      whether it carries over to a channel at its own bound.

- [ ] **`dev-writer`** — `delivery.rs:719-724` — reviews are never discarded and
      are not counted against any bound: "each is one of this peer's own requests
      settling". This is a choice with a real alternative (a bounded review queue)
      and a cost (the review queue grows with settles, though bounded by requests
      this peer made). It is justified only in a doc comment. Add it to Decision 3.

- [ ] **`dev-writer`** — `design.md` Decision 6 — the cost named ("a module that
      stops part-way through a review loses what it had taken out") is not the
      whole cost of delete-then-decide. A review that meets an op log that will not
      open (`Storage` refusal, `delivery.rs:1536`) has already deleted every message
      it took, so one unopenable op log loses the channel's whole parked backlog,
      and a panic in `judge` loses that message too. The spec requires "not
      decided again", so this is by contract, but the Decision should name it, since
      it is the same mechanism and the issue says "No honest message is lost
      because an open was slow".

## Entries that are thin

- [ ] **`dev-writer`** — `design.md` Decision 7 (expiry) — "kept until the
      channel's last request settles unopened, or until the next startup" omits a
      path. When a Held review cannot read the store (`take_parked` logs
      `ReviewUnreadable`, returns nothing), the messages stay parked on a channel
      that is now open and has no later event in this process. They wait for the
      next startup's open to be answered. "Cannot outlive the open ... by more than
      one process lifetime" is still true, but the entry should say a read failure
      is the one case where a message outlives its open, and that the bounds are what
      cap it.

- [ ] **`dev-writer`** — `design.md` Decision 4 — the four values have a reason
      (what each is "four channels at their own bound") but no alternative and no
      measurement, and say so ("judgements"). That is honest. What is missing: why
      256 per channel when a restart parks messages for several Stoas at once and
      the total is only four channels' worth, i.e. what happens to a fifth Stoa's
      honest backlog at a restart. The cost row is the Risks line only.

- [ ] **`dev-writer`** — `design.md` Decision 10 — the processor now never stops,
      so a review is run after the listener ends. No test is named, and I found none
      in `delivery/tests.rs` or `delivery/tests/parking.rs` that ends the event
      stream and then settles an open expecting a review. The existing tests at
      `tests.rs:2294` and `:2456` only check the listener ends and is logged. If the
      processor stopped with the listener again, nothing would go red. Name the
      test that guards it, or add one.

- [ ] **`dev-writer`** — `design.md` Decisions 3 and 4 — mutation evidence. The
      Decisions that are guards say which tests go red, which is good, but `tasks.md`
      3.2 says surviving mutants are "argued in the PR". Reasoning about surviving
      mutants on `plan_park`, `shed` and the `on_take` / `settle` seams is reasoning
      this change acted on and belongs under the Decision it guards, since the PR
      body is not archived. Record each survivor and its argument, or say there are
      none.

## Reasoning to keep somewhere the next toucher will look

- [ ] **`dev-writer`** — `CLAUDE.md` / overlay `## Hazards` — traps of the built
      subsystem that live only in `design.md` (archived with the change): Decision 9
      (settle before you log, or "nothing is parked afterwards" depends on timing),
      Decision 3 (every take and every settle under one lock; a second lock
      reopens the race), and Decision 11 (tests hold the boundary up with
      `Recorder::hold_on`, not with an open, since an open no longer holds anything).
      `delivery.rs` carries the first two in doc comments, which is where the next
      toucher of that file will look, so only Decision 11's trap and the "a changed
      `PARK_BOUNDS` is invisible to `cargo mutants`" note are missing from a
      trigger-specific document. Suggest the overlay's `## Hazards` gain the
      second, one line, or `tests.rs`'s `Recorder` doc point at it (it already
      explains `hold_on`; if so, tick this with that argument).
