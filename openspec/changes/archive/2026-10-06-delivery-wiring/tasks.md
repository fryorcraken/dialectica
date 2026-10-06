# Tasks

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
- [x] re-review: every commit after the review round — runner
      round 1 `7a2a3335..369561d1` findings passes (spec-writer ×2, dev-writer ×2, tester): spec deltas, delivery.rs/transport.rs/wire.rs/sender.rs rewrite, tests, design.md, CLAUDE.md — all six lanes; security and correctness on the strongest model (opus), readability, architecture, spec-test and design on role defaults: a rewrite of hostile-input handling needs the full set
      round 2 `369561d1..2cb71aaf` round 1 findings passes (spec-writer ×2, dev-writer ×2, tester): per-open settle deadline, receive_via boundary refactor, "already exists" wording, compile-time timeout asserts, new tests, design.md — all six lanes; security and correctness on the strongest model (opus), readability, architecture, spec-test and design on role defaults: the wait logic and boundary shape changed again
      round 3 `2cb71aaf..7462ded8` round 2 findings passes (spec-writer, dev-writer, tester): timeout-order requirement and two-open scenario, is_opening one-predicate refactor, Decision 11 re-argued (SETTLE_LIMIT kept at 40 s), three tests — all six lanes, each confirming its round-2 answers and reading the new ground; security on the strongest model (opus), the other five on role defaults: narrower than round 2, no new boundary code
      round 4 `7462ded8..58460b02` round 3 findings passes (spec-writer, dev-writer, tester): the ask restarting an open's time (spec, OpenTime/Wait state, once-only extension), Processor::new refactor, Decision 11 rewritten, timeout and adapter pins — all six lanes; security and correctness on the strongest model (opus), readability, architecture, spec-test and design on role defaults: new wait-state logic on the hostile-input path
      round 5 `58460b02..1d5e2e37` round 4 findings passes (dev-writer, tester): one-line guard in Wait::extend_from with its regression test, two prose references, five tests for the ask restart, end_wait and the ask-counted time — security, correctness, spec-test, readability and design; security and correctness on the strongest model (opus), the other three on role defaults: each confirms its round-4 answers; architecture skipped — no architecture box in play and the only code change is one guard line inside an existing function
      round 6 `1d5e2e37..3d34f15e` round 5 findings passes (spec-writer, dev-writer): new requirement that nothing of a judged wait is kept, the ask-after-end clause, doc comments, test citations, the 0.75 s figure — security, spec-test, design and readability; security on the strongest model (opus), the other three on role defaults: the new requirement bounds sender-growable state; correctness and architecture skipped — no code changed but comments
      round 7 `3d34f15e..ae44f364` round 6 findings passes (spec-writer, dev-writer): three book-hygiene scenarios scoped to the message's own wait, design.md entry for the requirement and a corrected test count — spec-test, design and readability on role defaults: prose and scenario wording only; security, correctness and architecture skipped — no code changed and the scenario edits narrow wording without changing the behaviour they pin
      round 8 `ae44f364..43844f2b` fix for the owner's failed two-peer check (dev-writer): content topic given four parts so delivery's parser accepts it, a test-only transcription of that parser, the fake delivery refusing what delivery refuses, Decision 17, a CLAUDE.md trap — all six lanes; security and correctness on the strongest model (opus), readability, architecture, spec-test and design on role defaults: a code change on the channel identity both peers must agree on, found only by the live check
      round 9 `43844f2b..87596ac4` round 8 findings passes (spec-writer, dev-writer): op-transport requirement that the content topic is one the network's rule accepts, test-only getShard step and delivery-pin test in transport.rs, live-check result recorded (7.3), QML and CLAUDE.md comment corrections — correctness, spec-test, design, readability and architecture; correctness on the strongest model (opus), the other four on role defaults: each confirms its round-8 answers; security skipped — no production code changed (the topic is unchanged, the new code is cfg(test), delivery.rs gained a doc comment only)
      round 10 `87596ac4..c0190bda` round 9 findings passes (spec-writer, dev-writer): composer-view delta dropping "delivery is not wired at all", op-transport sentence reworded, test-only lock_nodes_of refactor, scaffold.toml-to-lock pin test, design.md and docs/SCAFFOLD.md prose — correctness, spec-test, design and readability on role defaults, correctness also confirming the refactor commit changes no behaviour: each confirms its round-9 answers; security and architecture skipped — the code is cfg(test) only and the refactor is a helper extraction inside that test module
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Make room (no behaviour change)

- [x] 1.1 Name the op log file once, `core::log::op_log_path_in`, and use it at the adapter's four sites; `the_op_log_file_name_is_pinned` holds the name.
- [x] 1.2 Give `create_stoa` and `join_stoa` a `joined` sink called only after a membership is recorded, with a no-op in the adapter; the whole suite passes unchanged (commit `Make room for delivery wiring`).

## 2. Two threads on one op log

- [x] 2.1 Create a fresh store's schema once under `BEGIN IMMEDIATE`; `two_connections_opening_a_fresh_store_at_once_both_open_it` failed in round 0 before the fix and passes after it (design Decision 13).
- [x] 2.2 Pin that concurrent appends wait rather than fail: `two_connections_appending_at_once_both_store_everything`, red with a zero busy timeout.

## 3. The sender identifier

- [x] 3.1 `core::sender::SenderStore` in `senders.sqlite`: 32 random bytes per Stoa, retained before returned, layout-checked like the other stores; `sender.rs` tests cover stability across a reopen, two Stoas, two installations, not-a-key, and an unwritable store.

## 4. Outbound: the seam, the node, channels and sends

- [x] 4.1 The `Delivery` seam (four methods, no `stop`), `node_config`, `CALL_TIMEOUT` and `declined`; `the_nodes_configuration_names_the_reliable_channel_layer`, `the_node_preset_and_mode_are_pinned`, `declined_reads_delivery_s_three_shapes_of_no`.
- [x] 4.2 One worker thread: node first, then every membership's channel, then what handlers enqueue; `node_creation_precedes_every_channel_operation`, `node_creation_is_requested_once_however_often_startup_runs`, `a_declined_node_creation_does_not_stop_the_module`.
- [x] 4.3 Channel opens with the retained sender identifier; `joining_requests_the_stoas_channel_after_the_membership_is_recorded`, `creating_a_stoa_requests_its_channel`, `a_refused_join_requests_no_channel`, `a_channel_delivery_declines_does_not_fail_the_join`, `a_repeated_join_requests_the_channel_again`, `a_restarted_peer_requests_every_stoas_channel_and_no_other`, `an_unreadable_membership_record_opens_nothing_and_stops_nothing`, and the four sender-identifier tests.
- [x] 4.4 Sends read back by op id through `transport::handoff`; `an_op_published_on_an_open_channel_is_sent_as_its_stored_wire_form`, `an_op_the_peer_already_holds_published_again_is_sent_again`, `a_publish_into_a_stoa_with_no_open_channel_sends_nothing_and_opens_nothing`, `a_send_delivery_declines_leaves_the_op_published`, `sends_follow_the_order_of_their_publishes`, `a_publish_after_a_join_is_sent_on_the_channel_the_join_opened`.
- [x] 4.5 Replies never wait: `an_unresponsive_delivery_does_not_delay_the_publish_reply` and `an_unresponsive_delivery_does_not_delay_a_join` (the call held at a gate until the test releases it).

## 5. Inbound: the queue, the listener, the processor

- [x] 5.1 `InboundQueue` bounded at 256, discarding the arrival and counting discards; `the_waiting_messages_never_exceed_the_bound`, `a_full_queue_keeps_what_it_holds_and_discards_the_arrival`, `every_discard_is_counted_and_logged_apart_from_refusals`, `the_inbound_bound_is_pinned`.
- [x] 5.2 The listener loop (`listen`) and the processor, judging against this peer's clock at processing time; `an_op_another_peer_published_is_stored_unordered`, both window tests, `a_refusal_is_logged_by_kind_without_text_the_sender_chose`, `the_peer_keeps_processing_after_an_unreadable_message_and_a_refusal`, `a_received_op_reaches_the_log_through_the_running_listener`.
- [x] 5.3 A message on a channel still opening waits for the answer (now `op-transport`'s); `a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` and `…_is_declined_is_refused_after_the_answer`.

## 6. The adapter

- [x] 6.1 `DeliveryModule` (four `*_with_timeout` calls), `channel_messages` (the one subscription and the generated decoder), the start in `on_context_ready`, and the two sinks; compiled by `nix build ./dialectica#lgx`, and read by `the_adapter_never_stops_a_node_and_creates_one_at_one_site` and `only_reliable_channel_receipts_reach_the_op_log`.
- [x] 6.2 Nothing unwinds: `a_panicking_delivery_call_is_contained_and_the_next_action_runs`, `a_panicking_join_sink_does_not_change_the_reply`, `a_panic_reading_one_event_does_not_end_reception`; the worker (per action), the processor (per message) and the listener (per event, since 8.6) run under `catch_unwind`.

## 7. Verification

- [x] 7.1 `cargo test -p dialectica -p dialectica-core`, `cargo clippy … --all-targets`, and `nix build ./dialectica#lgx` green.
- [x] 7.2 Each refusal path and the bound shown red by a reverted mutation; the mutation and what it turns red are recorded beside each decision in `design.md` (Decisions 6, 10, 11 and 15), re-run under §10 where the first-pass count had gone stale.
- [x] 7.3 Two peers exchange a post under two `lgs basecamp launch` profiles (#102 items 5–7). A manual check no test here can stand in for: every test runs both sides in one process against a fake delivery. The owner ran it under Basecamp 0.2.3. The first build created the node and had every channel declined for its content topic (§14). The second build, with the four-part topic, passed: alice and bob each logged `channel open for Stoa 1024b9fd…`, and the ops `84aa65c8…` (alice→bob), `f4f72c6d…` and `d0578af7…` (bob→alice) were each `handed to the channel` by the sender and `stored inbound op` by the receiver within about 5 s, with no refusal, discard or crash. Not shown by it: the restart-reopen path, since `lgs basecamp launch` scrubs `module_data/` on every launch; a post appears on the other screen only after re-entering the Stoa (#194).

## 8. Review round

Each regression test below was seen red before its change.

- [x] 8.1 Split `transport::receive` into lookup, `judge` and `admit` (`Judged` only `judge` makes), and render panic payloads once in `wire::panic_detail`; no behaviour change, whole suite green.
- [x] 8.2 The processor judges before opening the op log and holds no lock while it does (design Decision 15): `an_unknown_channel_is_refused_as_one_before_the_op_log_is_opened`, `the_channel_book_is_not_held_while_an_op_is_appended`.
- [x] 8.3 "Already exists" opens the channel (Decision 14): `a_channel_delivery_reports_already_existing_is_open`, `a_creation_delivery_did_not_complete_in_time_opens_on_the_next_request`, `a_module_restarted_while_delivery_kept_running_has_its_channels_open`, `only_delivery_s_already_exists_answer_opens_a_declined_channel`.
- [x] 8.4 An empty `error` is no reason to decline (Decision 6): `an_empty_error_string_is_not_a_reason_to_decline`.
- [x] 8.5 Traffic on a channel neither open nor opening is refused on hand-over and takes no place (Decision 10): `traffic_on_a_channel_this_peer_is_not_opening_takes_no_place_in_the_queue`. When an open counts as being opened was left unspecified here; 9.2 builds the spec's answer.
- [x] 8.6 The pending-open wait survives a poisoned book, and its limit is injectable (Decision 11): `a_poisoned_channel_book_still_waits_for_this_channels_open`, `a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait` (passed before the change too: `SETTLE_LIMIT` already bounded the wait). The listener contains a panic per event (Decision 16): `a_panic_reading_one_event_does_not_end_reception`.
- [x] 8.7 The log says "already held" for an op that arrives again (`an_appended_twice_arrival_is_reported_already_present`, re-asserted); `Wiring` replaces the `started` flag beside an optional outbox (`a_worker_that_could_not_start_is_named_as_such_and_startup_does_not_run_again`, which cannot be shown red against the old shape, since it constructs the new one).

## 9. Spec-writer callback (`560e39f0`)

- [x] 9.1 Make room, no behaviour change: `Opening` owns its handle on the channel book so it can travel inside an action, and the message limit is one predicate, `transport::refuse_oversized`, which `judge` asks; whole suite green.
- [x] 9.2 A channel is being opened from when a create, a join or startup asks for it (Decision 11): the `Opening` guard travels inside `Action::Open`, and startup marks its opens before it subscribes. `a_message_on_a_channel_whose_open_waits_behind_another_is_judged_once_that_open_settles` (the former `NO SPEC` test, expectation flipped) and `a_restarted_peer_keeps_what_delivery_hands_over_before_startup_asks_for_its_channel` were red before it. An open never asked of delivery is settled: `a_sender_identifier_that_cannot_be_retained_opens_no_channel` now asserts it, red with the guard leaked on that path; `a_join_the_worker_cannot_take_is_given_up_and_not_left_opening` stages a worker whose end of the queue is gone, red with the refused action forgotten in `request`. Startup's guards when the OS refuses the worker thread are satisfied by construction and not shown by a test: `start` returns with them in scope, so they drop and settle, and no test here can make `thread::Builder::spawn` fail.
- [x] 9.3 A payload over the message limit, on a channel open or being opened, is refused as `too-long` on hand-over and takes no place (Decision 10): `an_oversized_payload_on_an_open_channel_takes_no_place_in_the_queue`, red before. `a_payload_at_the_limit_waits_its_turn` passed before and is red with `>=` in `refuse_oversized`; `an_oversized_payload_on_an_unknown_channel_is_refused_as_an_unknown_channel` is red with the two hand-over checks swapped.
- [x] 9.4 `SETTLE_LIMIT`'s justification restated: it bounds a stall of every Stoa and covers one call, not a queue of opens (Decision 11, Risks). No test waits the value out; the bounded-wait behaviour is `a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait`. (Superseded in part by 10.4: the value's relation to `CALL_TIMEOUT` is now checked at compile time and by a test.)

## 10. Re-review round 1, and the spec-writer's callback (`b6d35fac`)

- [x] 10.1 Make room, no behaviour change: `transport::receive_via` is the boundary's one spelling of lookup, `judge`, write; `receive` and the processor both go through it, and `judge` is private, so `Judged` certifies the channel check by construction (Decision 15). `the_boundary_looks_a_channel_up_under_the_messages_own_identifier` is red with the lookup keyed by anything but the message's channel id. `decide` is three calls: wait, `pass`, `record_decision`.
- [x] 10.2 Make room, no behaviour change: the channel book's pending entry is a `Pending` record rather than a bare count.
- [x] 10.3 The wait on a pending open is bounded once per open, not once per message (`op-transport`, Decision 11): `many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each` was red before the change (four messages held the op 1.61 s at a 400 ms limit). `an_open_whose_wait_has_expired_still_opens_its_channel_when_delivery_answers` and `a_new_request_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again` passed before it — the per-message shape satisfied both — and are red against the two ways the per-open shape can be got wrong (the open given up at expiry; a request not clearing the time). `an_open_this_peer_gives_up_without_asking_delivery_does_not_hold_a_message_up` passed before and is red with the guard leaked on the no-sender path. `a_request_made_while_a_message_waits_does_not_extend_that_messages_wait` pins the `op-transport` scenario "Requests made while a message waits do not lengthen its wait" and is red with the deadline re-read on each wake-up.
- [x] 10.4 `DELIVERY_CALLBACK_TIMEOUT < CALL_TIMEOUT < SETTLE_LIMIT` asserted at compile time; `SETTLE_LIMIT` at 10 s or `CALL_TIMEOUT` at 20 s fails the build (both tried). The asserts cannot hold delivery's real 30 s, so `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call` writes it as a literal: red with `DELIVERY_CALLBACK_TIMEOUT` at 10 s and `CALL_TIMEOUT` at 20 s, which compiles (Decision 4). Removing the bound altogether turns the six tests Decision 11 names red.
- [x] 10.5 The "already exists" recogniser the spec now states needed no code: `only_delivery_s_already_exists_answer_opens_a_declined_channel` gains the two new `stoa-membership` scenarios' rows, red for the "something else already exists" row with the match loosened to `already exists` (Decision 14).
- [x] 10.6 `design.md`: Decision 3's thread and mutex sentence corrected and the 5 s publish-behind-append wait measured and recorded; Decision 15 no longer quotes words Decision 3 lacks; the status channel is named by its feature macro, not "0.9"; mutation results re-run in place of pointers to the PR body and `findings/`.

## 11. Re-review round 2, and the spec-writer's callback (`de35f026`)

- [x] 11.1 Make room, no behaviour change: `ChannelBook::begin_wait` (then `wait_ends`) asks `is_opening` rather than spelling "pending and not open" a second time, so whether a message waits and whether it keeps waiting are one predicate; the `delivery::` tests pass unchanged.
- [x] 11.2 `design.md` Decision 11: the case against a limit sized for a queue of opens re-argued (the "grows as the square of K" claim withdrawn: each wait ends at its open's settle, so the stall is bounded by the last open's settle under either limit), `SETTLE_LIMIT` re-decided and kept at 40 s, and "K × 40 s" bounded by the last settle, as the spec's `de35f026` wording has it. No code change: the spec's new order (delivery's own < `CALL_TIMEOUT` < `SETTLE_LIMIT`) is what 10.4 already holds. Mutation claims in Decisions 6, 10 and 11 name the tests that go red and no longer carry counts; each was re-run on this tree. "No test can see" `CALL_TIMEOUT`/`SETTLE_LIMIT` corrected to "no test waits it out", with the test that holds them against delivery's 30 s named in `delivery.rs`, Decision 4 and 10.4.

## 12. Re-review round 3, and the spec-writer's callback (`d240ebdc`)

- [x] 12.1 Make room, no behaviour change: `Processor::new` is the one place a processor is built, with `settle_limit: SETTLE_LIMIT`, called by `Delivering::start` and the tests' fixtures alike (spec-test round 3, part (a)); the suite passes unchanged. The test that reads the running limit, and part (b), are the `tester`'s.
- [x] 12.2 This peer's ask of delivery starts an open's time again, and extends a message already waiting on it, once (`op-transport`, Decision 11). The channel book holds each waiting message's end (`Wait`) and the open's time as `OpenTime`; the worker marks the ask (`Opening::asked`) immediately before `channelCreate`. Red against a no-op ask before the change: `a_message_waiting_when_delivery_is_asked_is_judged_after_delivery_answers`, `an_earlier_message_on_a_queued_open_does_not_cost_an_op_that_arrives_while_delivery_is_asked`, `asking_delivery_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`, `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again` (the four new scenarios) and `the_worker_marks_its_ask_of_delivery_in_the_channel_book`. Decision 11 names what each part's removal turns red.
- [x] 12.3 `design.md` Decision 11, `SETTLE_LIMIT`'s doc and the Risks restated for the ask: the stall per start of an open's time, under twice `SETTLE_LIMIT` for an extended message, (K+1) × 35 s at startup for every K; the loss that remains (a wait that ran out before the ask); the round-3 probe; the once-only cap and the alternatives it rules out.
- [x] 12.4 Decision 4's "(re-run on the final tree)" label dropped (readability round 3).

## 13. Re-review round 4

- [x] 13.1 An ask extends only a wait that has not ended (`op-transport`: a message whose wait has expired "MUST be judged without waiting on that open"; Decision 11): `Wait::extend_from` returns when the ask is at or past the wait's end. `an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again` holds the book across the message's end and asks inside it, the correctness reviewer's probe ported; red before the fix, the message judged 1.00 s after the ask at a 1 s limit.
- [x] 13.2 `design.md` cites no piece-branch commit in Decision 11, and 11.1 names `begin_wait` (readability round 4).

## 14. The owner's first live run (7.3): delivery refused the content topic

- [x] 14.1 The content topic is `/dialectica/1/s-<hex>/proto`, four parts, which delivery's `NsContentTopic.parse` accepts with `dialectica` as application and `1` as version (design Decision 17). `the_content_topic_is_one_delivery_parses_with_dialectica_as_application` was red before the fix with the live message, `generation should be a numeric value`. The known-answer pin and the pure-function test moved with it; the channel id and sender id are unchanged, since delivery does not parse them.
- [x] 14.2 `transport::delivery_topic_rule` (test-only) transcribes the parser at `logos-delivery` `bfdb5afd`, held both ways by its own tests (the live-refused topic refused with the live message; delivery's default topic accepted). The fake delivery's `channel_create` declines a topic it refuses; with the old prefix every `delivery::` test that opens a channel and relies on it went red.

## 15. Re-review round 8

- [x] 15.1 The rule takes `getShard`'s step after the parse (correctness round 8): `delivery_topic_rule::subscribable` refuses a generation other than `0` or none with `Generation > 0 are not supported yet`, and the fake and the content-topic test consult it. `a_generation_other_than_zero_parses_and_is_then_refused_for_its_shard` was red before the fix (`subscribable("/1/…")` returned `Ok`). `CLAUDE.md`'s trap entry says "generation `0`".
- [x] 15.2 A delivery pin bump fails a test (architecture round 8): `the_transcribed_revision_is_the_one_delivery_is_locked_at` reads `dialectica/flake.lock` and asserts every `logos-delivery` node is locked at `DELIVERY_REV`, naming the files to re-read, `ALREADY_EXISTS`'s included. Red with the node match renamed, on its found-at-least-one assert.
- [x] 15.3 The owner's second live run passed; 7.3 ticked on it, and `proposal.md`, `design.md` (Decision 5, 17, Risks, Open Questions) and PR #190 say so (design round 8). The stale "delivery is not wired" comment in `DPublishOutcome.qml` corrected.
- [x] 15.4 The two tests that satisfy `op-transport`'s "The network's content-topic rule reads the content topic as dialectica version 1" say so in their comments; `DefaultContentTopic` cited at line 16 (readability round 8).

## 16. Re-review round 9

- [x] 16.1 Make room, no behaviour change: `lock_nodes_of` reads `dialectica/flake.lock`'s nodes for one repository, and the delivery pin test asks it; the `delivery_topic_rule` tests pass unchanged.
- [x] 16.2 The delivery a Basecamp installs is tied to the lock (correctness round 9): `the_scaffold_installs_the_delivery_module_the_lock_holds` reads the tracked `scaffold.toml` and asserts its `delivery_module` rev equals every `logos-delivery-module` node's in the lock. Red with `scaffold.toml`'s rev set to `0000…0000` (on the `assert_eq!`, naming the node and both revs) and with the table renamed (on its not-found message). `design.md` Decision 17 no longer files the scaffold pin as the live check's, and `docs/SCAFFOLD.md` says that pin is tied to the lock.
- [x] 16.3 Decision 17's "A pin bump fails a test" parses, names where the old prompt sat, and calls the three-file list the test's own (readability round 9).
