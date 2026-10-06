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

- [x] **`dev-writer`** — `parked.rs:384-436` (`plan_park`) — a park can evict a
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
      **Fixed** in `09eca672`. The spec-writer chose the second reading: no
      eviction when the arrival is finally discarded.
      - **The code.** `plan_park` returns `Plan::Discard` with no evictions the
        moment either total chooses the arrival. `ParkOutcome::Discarded`
        cannot carry any.
      - **The tests.** Two pin it:
        `parked::tests::a_payload_discarded_for_the_byte_total_evicts_nothing_the_count_total_chose`
        (through the store) and
        `a_count_eviction_is_not_kept_for_a_payload_the_byte_total_then_discards`
        (a table over `plan_park`).
      - **The record.** The cost the old code paid and why the rule is now this
        are in `design.md` Decision 5.

- [x] **`dev-writer`** — `design.md` Decision 5 — the per-channel rule is not
      argued. `plan_park` discards the arrival when its own channel would exceed
      the count or byte bound, and evicts nothing. The issue asks for "a stated rule
      for what is dropped when it is full"; Decision 5 covers only the totals. The
      alternatives (drop the channel's oldest to keep the freshest, or its newest)
      and what ruled them out are absent. The queue's "a backlog arrives roughly
      oldest first, so the newest are leaves" argument is made for the queue; say
      whether it carries over to a channel at its own bound.
      **Fixed.** `design.md` Decision 5 has a new paragraph, "The per-channel
      rule discards the arrival, and nothing already parked".
      - **Against dropping the channel's newest:** within one channel the
        arrival is the newest anyway, so the two differ only in churning a
        slot.
      - **Against dropping the oldest:** it favours whoever sends last, so a
        flood that keeps arriving pushes out an honest backlog parked before
        it.
      - **What it costs:** the lockout by a flood that arrives first is the new
        first entry under Risks, held for the owner (security finding 1).
      - **The "leaves" argument:** the paragraph on "newest" now says it is an
        unmeasured heuristic on SDS's order (architecture finding 6). So it is
        not offered as the reason here.

- [x] **`dev-writer`** — `delivery.rs:719-724` — reviews are never discarded and
      are not counted against any bound: "each is one of this peer's own requests
      settling". This is a choice with a real alternative (a bounded review queue)
      and a cost (the review queue grows with settles, though bounded by requests
      this peer made). It is justified only in a doc comment. Add it to Decision 3.
      **Fixed.** `design.md` Decision 3 has a new paragraph, "Reviews are never
      discarded and count towards no bound".
      - **What bounds them:** the requests this peer made (one per create, join
        or startup Stoa), not anything a peer sends.
      - **The alternative, a bounded review queue,** would need a drop rule. A
        dropped review leaves its channel's parked messages undecided until the
        next startup, which is the loss parking exists to prevent.

- [x] **`dev-writer`** — `design.md` Decision 6 — the cost named ("a module that
      stops part-way through a review loses what it had taken out") is not the
      whole cost of delete-then-decide. A review that meets an op log that will not
      open (`Storage` refusal, `delivery.rs:1536`) has already deleted every message
      it took, so one unopenable op log loses the channel's whole parked backlog,
      and a panic in `judge` loses that message too. The spec requires "not
      decided again", so this is by contract, but the Decision should name it, since
      it is the same mechanism and the issue says "No honest message is lost
      because an open was slow".
      **Fixed.** `design.md` Decision 6's single cost is now a list, "The costs
      of the choice".
      - **The module stopping mid-review** (out of scope).
      - **An op log that will not open at a review loses the channel's whole
        parked backlog.** The spec's "not parked afterwards" requires it,
        `a_parked_op_the_op_log_cannot_take_at_its_review_is_logged_and_not_parked_again`
        pins it, and the entry names it as the one path where a broken disk,
        not a slow open, costs honest messages.
      - **A panic judging one parked message loses that message.**
      - **Also in Risks:** the op-log case is joined to the mid-review stop.

## Entries that are thin

- [x] **`dev-writer`** — `design.md` Decision 7 (expiry) — "kept until the
      channel's last request settles unopened, or until the next startup" omits a
      path. When a Held review cannot read the store (`take_parked` logs
      `ReviewUnreadable`, returns nothing), the messages stay parked on a channel
      that is now open and has no later event in this process. They wait for the
      next startup's open to be answered. "Cannot outlive the open ... by more than
      one process lifetime" is still true, but the entry should say a read failure
      is the one case where a message outlives its open, and that the bounds are what
      cap it.
      **Fixed.** `design.md` Decision 7 now says it.
      - **The one path:** a held review that cannot read the store leaves the
        messages parked on a channel that is open.
      - **What decides them then:** nothing in the process, unless the Stoa is
        joined again. Otherwise the next startup's open of that channel decides
        them, or that startup's review refuses them if the peer has left.
      - **The cap** is still the bounds.
      - **The pin** is
        `parked_messages_a_review_could_not_read_are_decided_by_the_channels_next_review`,
        through the repeated-join path.

- [x] **`dev-writer`** — `design.md` Decision 4 — the four values have a reason
      (what each is "four channels at their own bound") but no alternative and no
      measurement, and say so ("judgements"). That is honest. What is missing: why
      256 per channel when a restart parks messages for several Stoas at once and
      the total is only four channels' worth, i.e. what happens to a fifth Stoa's
      honest backlog at a restart. The cost row is the Risks line only.
      **Fixed.** `design.md` Decision 4 has a new paragraph, "What a fifth Stoa
      costs at a restart".
      - **What happens.** Past four channels being opened at once, the total
        binds before the per-channel bounds do. The shedding rule then levels
        the channels: the one holding the most gives up its newest to each
        arrival on a channel holding less.
      - **Worked through:** eight Stoas being opened together keep 128 messages
        each, and an arrival on a channel already at that share is the one
        discarded. So a fifth Stoa is not shut out; it is levelled.
      - **The trade:** 256 against 1,024 lets one busy Stoa use a quarter of
        the store, while no Stoa below its share can be starved. A larger total
        buys more per Stoa at a restart, for more attacker-chosen bytes on disk.

- [x] **`dev-writer`** — `design.md` Decision 10 — the processor now never stops,
      so a review is run after the listener ends. No test is named, and I found none
      in `delivery/tests.rs` or `delivery/tests/parking.rs` that ends the event
      stream and then settles an open expecting a review. The existing tests at
      `tests.rs:2294` and `:2456` only check the listener ends and is logged. If the
      processor stopped with the listener again, nothing would go red. Name the
      test that guards it, or add one.
      **Fixed** in `09eca672` with a new test, named in Decision 10.
      - **The test:**
        `parking::a_review_due_after_deliverys_events_end_is_still_run`
        - It starts a running peer with one open unanswered.
        - It parks a message on that open.
        - It drops the event feed and waits for "inbound listener has ended".
        - Only then does it release delivery's answer.
        - It requires the op stored and nothing parked.
      - **Red by hand:** with `channels.close()` added after the listener's
        "ended" line, the old behaviour, it timed out waiting for the store.
      - **The flag is now test-only.** `Channels::closed` is `#[cfg(test)]`, so
        the running wiring's build has no way to close the queue at all.

- [x] **`dev-writer`** — `design.md` Decisions 3 and 4 — mutation evidence. The
      Decisions that are guards say which tests go red, which is good, but `tasks.md`
      3.2 says surviving mutants are "argued in the PR". Reasoning about surviving
      mutants on `plan_park`, `shed` and the `on_take` / `settle` seams is reasoning
      this change acted on and belongs under the Decision it guards, since the PR
      body is not archived. Record each survivor and its argument, or say there are
      none.
      **Fixed.** Each Decision now has a "Mutation evidence" paragraph, from
      runs I made on this pass's code, both `--in-place` and narrowed:
      - **Decision 5:** `shedding.rs` and `parked.rs` gave 48 mutants, 32
        caught, 16 unviable, 0 missed.
      - **Decision 3:** the `delivery.rs` functions this pass changed gave 15
        mutants, 9 caught, 3 timed out, 3 unviable, 0 missed. The three
        timeouts are hangs, each a detection.
      - **No mutant survived either run.** Decision 3 cites the correctness and
        security reviews' runs for the seams this pass did not touch, by commit.
      - **Decision 5 records what mutation cannot see:** the `const` bounds,
        and the missing no-eviction property.
      - **The spec-test review's two hand-made survivors** are recorded with
        where each is now held, and are the `tester`'s.
      - **`tasks.md` 3.2** now points at these paragraphs rather than the PR.

## Reasoning to keep somewhere the next toucher will look

- [x] **`dev-writer`** — `CLAUDE.md` / overlay `## Hazards` — traps of the built
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
      **Rejected**, on the argument you offered.
      - **The `hold_on` trap.** It is documented where a test author meets it.
        `Recorder`'s doc in `delivery/tests.rs` says it is "a place to hold up
        whichever thread records a given line", and `hold_on`'s own doc says how
        it holds. The tests that need "the boundary held up" all reach it
        through `hold_the_boundary_up` or `handed_over_while_opening_then_settled`,
        whose bodies call it. Anyone writing a new such test copies one of those,
        not an unanswered open, because an unanswered open no longer holds
        anything up. The tests themselves would show that at once, since
        nothing would be held.
      - **The constants note.** The overlay's `## Mutation tool` already says
        `cargo mutants` "mutates functions, not `const` values, so a changed
        constant is invisible to it". That is the general hazard, and
        `the_park_bounds_are_pinned` carries the specific one in its own
        comment.
      - **Why not the overlay.** A third copy there would be the
        "one rule, one place" breach the overlay warns about, and the overlay is
        the owner's file.
