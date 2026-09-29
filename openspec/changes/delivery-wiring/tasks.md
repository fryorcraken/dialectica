# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] re-review: every commit after the review round — runner
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
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
- [x] 5.3 A message on a channel still opening waits for the answer (`NO SPEC`); `a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` and `…_is_declined_is_refused_after_the_answer`.

## 6. The adapter

- [x] 6.1 `DeliveryModule` (four `*_with_timeout` calls), `channel_messages` (the one subscription and the generated decoder), the start in `on_context_ready`, and the two sinks; compiled by `nix build ./dialectica#lgx`, and read by `the_adapter_never_stops_a_node_and_creates_one_at_one_site` and `only_reliable_channel_receipts_reach_the_op_log`.
- [x] 6.2 Nothing unwinds: `a_panicking_delivery_call_is_contained_and_the_next_action_runs`, `a_panicking_join_sink_does_not_change_the_reply`; the listener, worker and processor each run items under `catch_unwind`.

## 7. Verification

- [x] 7.1 `cargo test -p dialectica -p dialectica-core`, `cargo clippy … --all-targets`, and `nix build ./dialectica#lgx` green.
- [x] 7.2 Each refusal path and the bound shown red by a reverted mutation (listed in the PR body).
- [ ] 7.3 Two peers exchange a post under two `lgs basecamp launch` profiles (#102 items 5–7). A manual check no test here can stand in for: every test runs both sides in one process against a fake delivery.
