# spec-test review: delivery-wiring

Reviewed at `7a2a3335` on `piece/176-delivery-wiring`. Read: the two spec deltas,
`tasks.md`, `proposal.md`'s scope section, issue #176 (body and the owner's
2026-09-29 comment), `delivery/tests.rs` in full, the tests in `sender.rs`,
`log/sqlite.rs`, `transport.rs` (the two `handoff` tests) and the `wire.rs`
sink-signature churn. The implementation was read only at the three lines a
mutation touched. Baseline: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
passed (1256 + 30 + 3, 0 failed).

## Mutations run (two; both restored, tree is clean)

1. `dialectica/rust-lib/src/lib.rs:889`, the publish handlers' sink
   `&mut |id| delivery.published(id)` changed to `&mut |_id| {}`. Ran
   `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core delivery`:
   **all 67 passed. Survived.** The line sits in the `cfg(logos_scaffold)` region
   (that region opens at `lib.rs:741`), so `cargo test` never compiles it; the
   adapter text tests do not name the sinks.
2. `dialectica/rust-lib/dialectica-core/src/delivery.rs:945`, in `Delivering::start`,
   `InboundQueue::with_bound(INBOUND_BOUND)` changed to `with_bound(usize::MAX)`.
   Same command: **all 67 passed. Survived.**

## Findings

- [ ] **`tester`** — the adapter's three sinks are unpinned: a publish sink that
      does nothing leaves every gate cargo runs green, and that is exactly the
      defect #176 was filed for ("No op ever leaves the authoring peer").
      **Where:** `delivery/tests.rs` tests every handoff by passing
      `|id| delivering.published(id)` (and `delivering.joined`) itself, in the
      test. The adapter's own wiring of those closures, `lib.rs:889` (one helper
      serving all three publish handlers) and `lib.rs:1047` / `lib.rs:1060`
      (create and join), is behind `cfg(logos_scaffold)`. The text tests
      (`the_adapter_never_stops_a_node…`, `only_reliable_channel_receipts…`,
      `the_adapter_maps_each_event_field…`) read that file but none names a sink.
      **Measured:** mutation 1 above, all 67 delivery tests passed. The `joined`
      sinks were not mutated (budget); they have the same shape and the same
      absence of a pin, so the same result is expected.
      **Fix shape:** a text pin in the family of the existing adapter tests
      asserting that the file passes `.published(` once and `.joined(` twice, and
      contains no `|_id| {}` / `|_stoa| {}` closure. Layer note: this is a
      requirement about the adapter, which only `nix build ./dialectica#lgx` or a
      source-reading test can see; say which in the test's comment.
      **Severity:** high — the central requirement, not a corner.

- [ ] **`tester`** — the inbound bound is pinned on the queue and not on the
      queue `start` builds. **Where:** `the_waiting_messages_never_exceed_the_bound`,
      `a_full_queue_keeps…`, `every_discard_is_counted…` and
      `the_inbound_bound_is_pinned` all construct `InboundQueue::with_bound(...)`
      or read the constant; `Peer::processor` also builds its queue from
      `INBOUND_BOUND` itself. Nothing floods the queue `Delivering::start` makes.
      **Measured:** mutation 2 above; 67 of 67 passed with an effectively
      unbounded queue in production. The tester recorded this limit (their item
      2); it is a gap and not an acceptable limit, because "bounded by a fixed
      count" is the requirement and the shipped path can lose it silently.
      **Fix shape:** through `start_listening`, hold a channel creation at a
      `Gate` so the processor parks on the first message, send `INBOUND_BOUND + 2`
      messages, and assert the journal carries a discard line. Every ingredient
      already exists in `taking_a_message_from_delivery_does_not_wait_on_the_boundary`.
      **Severity:** medium.

- [ ] **`tester`** — the discard's log record is not checked for the two things the
      requirement forbids in it. **Where:** `delivery/tests.rs`
      `every_discard_is_counted_and_logged_apart_from_refusals` asserts the
      discard lines lack the word `refused`; the spec says "MUST NOT carry the
      payload or the sender identifier". `arriving()` already supplies a
      distinctive sender (`a-sender-that-chose-this`) and the test never asserts
      its absence, and the payloads are single bytes.
      The same holds for the refusals other than unknown-channel: the
      table test `every_refusal_is_logged_under_its_own_name…` checks the sender
      is absent from each line but its payloads (`vec![1,2,3]`, `b"not an op"`, …)
      are not distinctive, and only `a_refusal_is_logged_by_kind…` checks a
      payload, for the unknown-channel case alone.
      **Read, not measured:** a discard line that appended the sender, or an
      undecodable-refusal line that echoed the payload, would pass. Give the
      table a distinctive payload per case and assert it absent, as the
      unknown-channel test does.
      **Severity:** medium (security property, "no text the sender chose").

- [ ] **`tester`** — "found the op already held" reaches the sink through no test.
      **Where:** `op-transport`, "Every publish that succeeds … one that stored
      the op, and one that found the op already held", scenario "Re-publishing an
      op already held sends it again". `an_op_the_peer_already_holds_published_again_is_sent_again`
      calls `delivering.published(&id)` a second time by hand, so it proves the
      worker sends twice and says nothing about whether a handler calls the sink
      when its reply is `wasNew:false`. `wire.rs:10261`
      (`appending_an_op_the_peer_already_holds_reports_already_present`) calls
      `log.append` directly and its own comment says it never reaches
      `publish_post`, so it cannot see the sink either. The tester recorded this
      as their item 4.
      **Read, not measured:** a handler that called `deliver` only when the op
      was new would pass every test in the tree.
      **Fix shape:** a wire-level test with a small `OpLog` wrapper whose `append`
      answers `Appended::AlreadyPresent`, and a recording sink; assert the sink is
      called once and the reply says `wasNew:false`. Not reachable by publishing
      the same body twice through a real log, since the counter differs.
      **Severity:** medium.

- [ ] **`tester`** — delivery's "no" is tried in one shape per call. The spec says
      "declines, fails, or does not answer" for node creation, channel creation and
      sends. Node creation and channel creation are tested only with the observed
      `Ok({error, success:false, value:null})` shape
      (`a_declined_node_creation…`, `a_channel_delivery_declines…`); sends only
      with `Err("the network said no")` (`a_send_delivery_declines_leaves_the_op_published`),
      which is not the shape real delivery was measured giving.
      `declined_reads_delivery_s_three_shapes_of_no` covers the helper, not that
      each call site routes through it.
      **Read, not measured:** a send path that treated only `Err` as a failure
      would log nothing when delivery answers with its error object, and pass.
      Run the send test with `the_observed_decline`, and one node-creation and
      one channel-creation case with `Err(..)` (including that start is still not
      requested after an `Err` creation).
      **Severity:** low-medium.

- [ ] **`tester`** — "could not subscribe" is asserted; what the spec requires the
      log to say is not. **Where:** `a_failed_subscription_leaves_sending_wired`
      matches the substring `could not subscribe`. The requirement is that the log
      record that "this peer will not receive ops from other peers". A line that
      said only the first would pass. Assert on the consequence's wording as the
      other log assertions do for theirs.
      **Severity:** low.

- [ ] **`spec-writer`** — a requirement clause with no scenario and no test: "given
      up as unanswered". **Where:** `op-transport`, "Every payload the reliable
      channel delivers passes the inbound boundary", the sentence on a message
      arriving while its open is unanswered, lists "reported created, declined,
      failed, or given up as unanswered" as the ways an open settles. The first
      three have scenarios; the fourth has none, gives no limit, and no test
      reaches it (`SETTLE_LIMIT`, 40 s, appears in the tests only in a doc
      comment). It matters more than the others: while the processor waits, the
      queue fills and arrivals are discarded, so the length of the wait is a
      property a peer can observe. Either add a scenario and say the wait is
      finite and bounded by a fixed value, or state that the limit is not
      contracted; then the `tester` needs a matching test, which will want the
      limit injectable.
      **Severity:** medium.

- [ ] **`spec-writer`** — the scenario "The sender identifier is not the author's key"
      can never fail. **Where:** `op-transport`, last requirement. The value is
      minted from a Stoa address and random bytes; no key reaches the function, so
      "compared with the public key … not that key, in any encoding" holds by
      construction, and both tests of it (`sender.rs`
      `the_identifier_is_not_a_key_in_any_encoding`, `delivery/tests.rs`
      `the_sender_identifier_is_not_the_authors_key`) compare against a key
      unrelated to anything the code sees. The tester recorded this as limit 3 and
      it is acceptable as a limit, but the spec should say so rather than carry a
      scenario that reads as measured: restate the "neither a public key … nor
      computable from one" bullet as a property of what the minting code takes
      as input, checked by reading (as the node-stop prohibition is), and drop or
      relabel the scenario. Same defect family as the stop prohibition, which the
      spec already handles honestly.
      **Severity:** low.

## Areas that were clean

- **Scenario coverage.** Every scenario in both deltas maps to a test, apart from
  the two above ("given up", and the handler half of re-publish). The order,
  do-not-wait and race scenarios are the strongest tests here: the gate-based
  ones (`an_unresponsive_delivery_does_not_delay_*`,
  `sends_made_while_an_open_is_unanswered…`, `taking_a_message_from_delivery_does_not_wait…`,
  `a_message_arriving_while_its_open_is_declined…` with its flag) read the
  answer off what has happened rather than a clock, and the tests that need a
  `sleep` fail on the wrong implementation regardless of how it lands.
- **Fixtures with two explanations.** The fake answers from its input (channel
  id echoed, send id by payload length), the declined-channel test sends its
  message after the decline and checks the refusal kind, the storage-refusal
  test drains the queue with `run` rather than watching for a time, and the
  clock test uses a clock that is wrong until the open is answered. No
  same-answer fixture found in the reading.
- **Layer.** Outbound behaviour and the inbound boundary are tested in-process
  against a fake at the right seam. The requirements only the adapter can
  see (no stop, one create site, `channelMessageReceived` only) are read off
  its source, which is the only layer that can see them; the gap is the sinks
  (first finding).
- **Race fix in `sqlite.rs`.** `two_connections_opening_a_fresh_store_at_once…`
  repeats 40 rounds behind a barrier; the tasks file says it was red before the
  fix. `the_op_log_file_name_is_pinned` and `the_layout_version_and_file_name_are_pinned`
  hardcode the names rather than reading them back.
- **`NO SPEC:` markers.** None in this change's tests; the one `NO SPEC` in
  `tasks.md` (5.3, a message waiting on an open) is now a requirement in the
  delta with three scenarios. No marker left to route.
- **Unmarked pins, for information (no action):** the tests hardcode log
  substrings and the six refusal names from `design.md` (`unknown-channel`,
  `too-long`, `undecodable`, `fails-verification`, `stoa-mismatch`,
  `ahead-of-time`, plus `storage`), and the queue size 256. The spec asks only
  that refusals be named and distinct and that the bound be fixed; the names and
  the number live in `design.md`, which is where the design-reviewer looks.
- **Requirements moved between capabilities.** None: the deltas MODIFY and ADD
  within `op-transport` and ADD to `stoa-membership`; nothing is REMOVED here.
  The MODIFIED blocks keep the live wording per `proposal.md`; I did not diff
  them against `openspec/specs/` beyond reading them for sense.
- **Self-consistency.** Read both files whole. The "before delivery is wired"
  clauses, the once-only startup, "an open channel stays open on a declined
  repeat" and "a channel is open only once delivery reports it created" do not
  contradict one another.
- **Staleness against #176.** Nothing out of scope: `entryLayer: "channels"`,
  node once and never stopped, `channelCreate` on create/join, sends through the
  handoff, a listener on `channelMessageReceived`, reliable channel throughout.
  The owner's 2026-09-29 comment asks for an automated two-peer end-to-end test
  "once this issue lands"; `proposal.md` names it as follow-up for the project
  manager to file, which matches the wording. That is the runner's to route, not
  a box here. `tasks.md` 7.3 (the manual two-profile check) is still unticked,
  and no test here can stand in for it.
