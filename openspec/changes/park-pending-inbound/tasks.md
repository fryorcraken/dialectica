## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. The parked-message store

- [x] 1.1 Add `parked.rs`: `parked.sqlite`, a table of `seq`, `channel` and
  `payload` and nothing else, layout version 1, `park`, `take_channel`,
  `channels`; verified by `parked::tests` (layout, reopen, the file's
  columns, a failed write or take changing nothing)
- [x] 1.2 Add the four bounds as `PARK_BOUNDS`, with compile-time ordering
  and `plan_park`'s discard rule; verified by the known-answer pin, the
  ordering test against a literal 150 KiB, and one store test per discard
  rule
- [x] 1.3 Add the one shedding rule, for the queue and the parked totals both
  (`shedding::choose`, moved out of `parked.rs` in 5.2); verified by
  `shedding::tests`

## 2. The seams in `delivery.rs`

- [x] 2.1 Replace `Pending`, `OpenTime`, `Wait` and `await_settled` with a
  `ChannelBook` of request counts, and give it `on_take` (the parking
  trigger), `settle` returning the `Review` it begins, and `startup_review`;
  verified by the three seam tests in `delivery/tests/parking.rs`
- [x] 2.2 Put the channel book, the waiting payloads and the waiting reviews
  under one lock (`Channels`), with `take` returning a review before any
  payload; verified by
  `a_payload_taken_before_a_settle_is_parked_and_the_settles_review_comes_next`
  and `a_review_is_taken_before_a_payload_that_was_waiting_when_its_event_came`
- [x] 2.3 Change the queue's discard rule to the newest payload of the channel
  holding the most; verified by the three `a_full_queue…` and tie tests
- [x] 2.4 Have the processor park, review (held, unopened, startup) and judge,
  each under its own `catch_unwind`, and stay alive after the listener ends;
  verified by the review tests in `delivery/tests/parking.rs`
- [x] 2.5 Have the worker settle an open before it logs the outcome; verified
  by `an_open_this_peer_gives_up_without_asking_delivery_leaves_nothing_parked`
- [x] 2.6 Queue the startup review in `Delivering::start` after startup's opens
  are counted and before the subscription; verified by
  `startup_refuses_a_parked_message_on_a_channel_it_does_not_open` and
  `a_message_parked_before_a_restart_waits_for_and_is_stored_by_startups_open`
- [x] 2.7 Remove `SETTLE_LIMIT`, the ask restart, the once-only extension and
  the per-message wait records, and the tests that pinned them, keeping
  `DELIVERY_CALLBACK_TIMEOUT < CALL_TIMEOUT`; verified by `grep` finding none
  of the removed names under `dialectica/`, and by
  `this_peers_wait_on_a_creation_outlasts_deliverys_own`

## 3. Gates

- [x] 3.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core` green from this worktree
- [x] 3.2 `cargo mutants` over `parked.rs`, `shedding.rs` and the seams in
  `delivery.rs`, with each surviving mutant either killed by a test or argued
  under the Decision it guards: none survived; the runs are recorded in
  `design.md` Decisions 3 and 5
- [x] 3.3 `nix build ./dialectica#lgx` green: the adapter compiles against the
  changed `Delivering::start`

## 4. What no local test layer can see

- [ ] 4.1 Satisfied by construction, not by a test: **a parked message is never
  readable as an op**, because the op log never sees a parked row. The rows
  are in another file, which no op-log read opens. A test can only observe
  that the reads it knows of return nothing, and
  `a_parked_op_is_not_readable_as_an_op_and_does_not_move_its_stoas_clock`
  does that for the op log, the Lamport clock and a module read
- [ ] 4.2 Satisfied by construction: **no sender identifier or timestamp is
  kept**, because `ParkedStore::park` takes neither and the table has no
  column for them; `nothing_parked_carries_the_sender_identifier_or_the_timestamp`
  pins the columns
- [ ] 4.3 A live two-peer rerun under `lgs basecamp launch`. The race parking
  exists for happens only against a real delivery node, and `cargo test`
  drives a fake. This is the owner's, as #176's was

## 5. Review findings addressed to `dev-writer`

- [x] 5.1 A payload discarded for a total bound evicts nothing (`ParkOutcome`,
  `Plan`, `Total::make_room`); verified by
  `a_payload_discarded_for_the_byte_total_evicts_nothing_the_count_total_chose`
  and `a_count_eviction_is_not_kept_for_a_payload_the_byte_total_then_discards`
- [x] 5.2 One shedding rule in `shedding.rs`, the arrival's "newest of all"
  written once as `Newest::Arrival`; verified by `shedding::tests` and the
  queue's and store's tie tests
- [x] 5.3 A taken payload is judged on its one reading
  (`Taken::Judge(Option<Address>)`); verified by
  `a_payload_taken_on_an_unknown_channel_is_refused_though_its_channel_opens_before_it_is_judged`
- [x] 5.4 `parked.sqlite` keeps neither the size nor the bytes of what a review
  took (`secure_delete`, `auto_vacuum = FULL`); verified by
  `a_review_leaves_neither_the_size_nor_the_bytes_of_what_it_took_in_the_file`
- [x] 5.5 The running wiring parks by `PARK_BOUNDS` and never closes the queue,
  by construction (a `#[cfg(test)]` field and flag); verified by the non-test
  build refusing to compile a `start` that sets the bounds, and by
  `a_review_due_after_deliverys_events_end_is_still_run`
- [x] 5.6 Readability: `Opening::finish`, the `drain` barrier, the review split
  per event, one discard loop, `ensure_schema`, `take_channel` without a block,
  `spawn_processor`, the startup order in `start`'s doc, design citations that
  name their change
- [x] 5.7 `design.md`: the flood lockout and the per-park cost under Risks,
  the per-channel rule, reviews unbounded, Decision 6's costs, Decision 7's
  unreadable-review path, a fifth Stoa at restart, mutation evidence
- [x] 5.8 Deferred with a destination: the per-park cost (#206), the shared
  store skeleton (#204), the `delivery.rs` split (#205)
