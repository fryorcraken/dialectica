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

- [x] **`tester`** — the adapter's three sinks are unpinned: a publish sink that
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
      **Outcome (`tester`): fixed, at the text layer, as the fix shape said.**
      `delivery::tests::the_adapter_hands_delivery_every_recorded_membership_and_every_published_op`
      reads `lib.rs` (comments stripped, whitespace removed) and asserts, with
      hardcoded counts: `&mut|id|delivery.published(id)` once; the three publish
      handlers each once as `self.publishing(&request,core::publish_post|reply|vote)`;
      `&mut|stoa|self.delivery.joined(stoa)` twice; `self.delivery.start(` once; and
      no closure that ignores its argument (`|_id|`, `|_stoa|`, `|_|`, `|_op|`,
      `|_op_id|`). Its comment says why it is a text pin (`cfg(logos_scaffold)`,
      which `cargo test` does not compile). Mutations, each predicted red and
      observed red, and each the only test to move: the finding's own mutation
      (`&mut |id| delivery.published(id)` replaced by `&mut |_id| {}`, "should
      contain `&mut|id|delivery.published(id)` exactly 1 time(s)", got 0), and the
      join handler's sink replaced by `|_stoa| {}` ("… exactly 2 time(s)", got 1).
      The create handler's sink is the same text as the join handler's and is
      counted with it, so dropping either is the same failure; not run separately.
      A change that swapped the sink for a differently named ignoring closure
      (`|ignored| {}`) would not be caught, nor would a sink that calls something
      other than `published` on the right object; both need the adapter compiled
      and run, which is task 7.3's manual two-peer check. Restored each time.

- [x] **`tester`** — the inbound bound is pinned on the queue and not on the
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
      **Outcome (`tester`): fixed**, with the fix shape's ingredients:
      `the_queue_the_running_wiring_builds_is_bounded_at_the_pinned_count` (see the
      matching `correctness.md` box). Predicted red under `with_bound(usize::MAX)`,
      observed red ("0 of 262 discarded"), the only test to move. It uses
      `hold_the_boundary_up` (an unanswered open on a second Stoa) rather than a
      gate on the first message, and asserts the arithmetic bound
      (`>= junk - INBOUND_BOUND` discards) because whether the processor has taken
      the first message when the flood starts is not fixed.

- [x] **`tester`** — the discard's log record is not checked for the two things the
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
      **Outcome (`tester`): fixed, for the discard, for every refusal the
      processor logs, and for the two the listener logs.**
      (1) `every_discard_is_counted_and_logged_apart_from_refusals` now feeds
      messages with the sender `zzyzx-the-sender-identifier` and payloads
      `zzyzx-payload-N`, and asserts every discard line lacks `zzyzx`, its hex and
      its decimal byte list. Mutation: the discard line with the sender appended
      (`format!("{} {who}", Note::Discarded{..})`, kept in one line so it still
      counts as a discard). Predicted red, observed red
      ("… 1 discarded since the module started zzyzx-the-sender-identifier"); a first
      attempt that appended a second line went red on the count instead, which is
      not the property, and was replaced. The pre-change test passes that mutation.
      (2) `every_refusal_is_logged_under_its_own_name_and_none_carries_what_the_sender_chose`:
      distinctive payload per row, guarded so a row whose payload does not carry the
      marker fails; see the `readability.md` box for the mutation.
      (3) New `a_refusal_made_on_hand_over_is_logged_by_kind_without_text_the_sender_chose`:
      the unknown-channel and too-long refusals are logged by `hand_over`, a
      different code path from the processor's, and no test read those lines for
      an echo. Mutation: `hand_over`'s refusal line with the channel id appended
      (prefix kept, so the name-matching tests stay green). Predicted red, observed
      red, and the only test to move; the same mutation written as
      `(kind: <sender>)` also turned three dev tests red, by breaking the
      `refused (unknown-channel)` text they search for, not by the property.

- [x] **`tester`** — "found the op already held" reaches the sink through no test.
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
      **Outcome (`tester`): fixed, as the fix shape said.**
      `wire::tests::a_publish_that_finds_the_op_already_held_is_still_handed_to_delivery_on_all_three_handlers`
      wraps a `MemoryOpLog` in `AlwaysAlreadyHeld` (stores, then answers
      `Appended::AlreadyPresent`) and, for post, reply and vote, asserts the reply
      is `wasNew:false` and the recording sink got exactly `[id]` of the id the
      reply names. Mutation: `delivered_and_published` calling the sink only when
      `published.was_new()`. Predicted red, observed red ("for post: an op found
      already held must still be handed to delivery, once, left: []"), the only
      failure among the 304 `wire::` tests. The three handlers share
      `delivered_and_published`, so one mutation stands for all three; a
      per-handler bypass was not tried.

- [x] **`tester`** — delivery's "no" is tried in one shape per call. The spec says
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
      **Outcome (`tester`): fixed, wider than asked.** A table,
      `the_ways_delivery_says_no` (transport failure; the observed error envelope;
      a failure with no reason), run at every call site by
      `a_declined_node_creation_is_read_in_every_shape_delivery_says_no` (start is
      never requested, the log names the reason, channels are still requested),
      `a_declined_channel_creation_is_read_in_every_shape_delivery_says_no` (the
      channel is not open, the log names the Stoa and the reason) and
      `a_declined_send_is_read_in_every_shape_delivery_says_no` (the log says
      delivery did not take the op, and never that it was handed to the channel).
      Mutations, each predicted red and observed red: `Worker::send` reading only
      `Err` — red on the envelope row, and only that test; `channel_answer` reading
      only `Err` — red on the envelope row, and the dev's
      `a_declined_repeat_open_leaves_the_channel_open_for_the_next_message` also
      went red; node creation reading only the error envelope (the reverse shape,
      `Err` ignored) — red on the transport-failure row, and the only test to move,
      so the dev's node test did not see that one. Node creation reading only `Err`
      also turned the dev's `a_declined_node_creation_does_not_stop_the_module`
      red, so that direction was already covered. "Does not answer" reaches the
      code as the `Err` the seam's timeout gives; nothing here can tell it from a
      transport failure, so it is the first row. Restored each time.

- [x] **`tester`** — "could not subscribe" is asserted; what the spec requires the
      log to say is not. **Where:** `a_failed_subscription_leaves_sending_wired`
      matches the substring `could not subscribe`. The requirement is that the log
      record that "this peer will not receive ops from other peers". A line that
      said only the first would pass. Assert on the consequence's wording as the
      other log assertions do for theirs.
      **Severity:** low.
      **Outcome (`tester`): fixed.** `a_failed_subscription_leaves_sending_wired`
      now finds the line by the consequence's words (`will not receive ops from
      other peers`, case-insensitively), checks it carries delivery's reason, and
      also asserts node creation was requested (the spec's "node creation, channel
      creation and sends MUST still be requested"; the test had checked the other
      two). Mutation: the message reduced to `could not subscribe to
      channelMessageReceived (<why>)`. Predicted red, observed red, the only test to
      move. Restored.

- [x] **`spec-writer`** — a requirement clause with no scenario and no test: "given
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
      **Outcome (`spec-writer`): fixed.** `op-transport`, "Every payload the
      reliable channel delivers passes the inbound boundary", gains "That wait is
      bounded by a fixed time": a waiting message "MUST be judged no later than a
      fixed time after this peer began waiting on it, whether or not delivery ever
      answers the open", and is then judged against the channels open then. The
      value stays out of the spec (it is `design.md`'s, beside `CALL_TIMEOUT`).
      New scenario "A message waiting on an open delivery never answers is judged
      after a bounded wait": the create is never answered, the message is refused
      as an unknown channel while delivery still has not answered, and a valid op
      after it on an open channel is then stored. As you say, the `tester` will
      want the limit injectable to run it without a 40 s wait.
      **Follow-up (`tester`):** the scenario's test is
      `a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait`
      (limit injected, 200 ms). It had never been seen red; it is now. Two
      mutations, each predicted red and observed red as "timed out waiting: the op
      behind the unanswered open to be stored": the injected limit ignored
      (`Processor::decide` passing `SETTLE_LIMIT` to the wait), and the wait
      unbounded (`Condvar::wait` for a settle, no limit). A third mutation — a wait
      whose clock restarts on every wake-up (`wait_timeout(book, limit)` in a loop,
      giving up only when one call times out) — **survived that test and every dev
      test**, and is what the spec's "no later than a fixed time after this peer
      began waiting" forbids, since one unrelated open settling per interval
      postpones the judgement for ever. New
      `opens_settling_for_other_channels_do_not_extend_the_bounded_wait` keeps
      another channel's open settling every millisecond until the waiting message
      is refused (or 12 s pass), and is red under that mutation ("timed out
      waiting: the message to be refused while opens keep settling"), green on the
      shipped loop, and independent of when the processor began waiting.

- [x] **`spec-writer`** — the scenario "The sender identifier is not the author's key"
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
      **Outcome (`spec-writer`): fixed.** The fourth bullet of "The sender
      identifier this peer supplies is its own, stable, and says nothing about its
      author" now reads "be made from nothing that is a public key this peer holds
      or signs with, or that is computed from one", followed by a paragraph saying
      it "is checked by reading the code that makes a sender identifier, not by
      comparing an identifier with a key", as the node-stop prohibition is, and
      naming the two-installations scenario as the observable evidence beside it.
      The scenario "The sender identifier is not the author's key" is replaced by
      "Nothing a sender identifier is made from is a key" (WHEN the code that makes
      one is examined for its inputs). `sender.rs`
      `the_identifier_is_not_a_key_in_any_encoding` and `delivery/tests.rs`
      `the_sender_identifier_is_not_the_authors_key` now map to no scenario; the
      `tester` should retarget or drop them.
      **Follow-up (`tester`):** both dropped as tests of the scenario.
      `delivery::tests::the_sender_identifier_is_not_the_authors_key` is deleted (a
      note in its place says where the property is checked): no key reaches the code,
      so it could not fail. `sender::tests::the_identifier_is_not_a_key_in_any_encoding`
      is split: its format half survives as
      `a_sender_identifier_is_its_head_and_thirty_two_bytes_in_lowercase_hex`, now
      against hardcoded literals (it asserted against `SENDER_PREFIX` and
      `SENDER_BYTES`, so it agreed with any value they were changed to), labelled as
      a `design.md` pin and not a spec scenario; its key half is replaced by
      `nothing_a_sender_identifier_is_made_from_is_a_key`, which does what the spec
      now says the scenario is — reads the code that makes the identifier. It pins the
      minting function's signature (a Stoa address and nothing else), that the bytes
      come from `getrandom::fill` once, that the one thing imported from `identity`
      is `Address`, and that no key, keystore or signature word appears in the
      non-test source. Mutation: the random fill followed by the bytes being
      overwritten from a public key. Predicted red, observed red on the import and
      word checks; the dev's "two installations differ" and "two Stoas differ" tests
      went red too, because that mutation is also deterministic. What this cannot see
      is a key laundered through the Stoa address, which the spec's "checked by
      reading" cannot either. `two_installations_holding_one_identity_supply_different_sender_identifiers`
      never puts one identity in both installations: no identity reaches the code,
      so doing so would change nothing it can see; its comment now says what can
      fail it (a derivation from what the two share) instead of claiming the case.

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

## Re-review round 1 `7a2a3335..369561d1`

Reviewed at `f37c6cc4` (the range's spec and test files only, plus the two
mutated lines). Read: the two spec deltas and `proposal.md`'s diff, the whole
`delivery/tests.rs` diff, the `mod tests` of `sender.rs`, the `wire.rs` test
added for the already-held publish, `git diff` of `transport.rs` for the
`refuse_oversized` predicate the tests name, issue #176 fresh (body and the
owner's 2026-09-29 comment, state OPEN). `log/sqlite.rs` has no change in this
range. Baseline: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
exited 0.

**Confirmed fixed, each by reading the named test against the spec:** the
adapter-sink text pin and the argument-order pin; the inbound bound on the queue
`start` builds; the discard, hand-over and refusal-table echo checks (sender,
payload in three spellings); the already-held publish on all three handlers;
the three shapes of "no" at node creation, channel creation and send; the
subscription consequence's wording; the bounded wait (with the wait that a
wake-up must not restart); and the sender-identifier scenario rewritten as a
reading of the minting code. The test the spec-writer's rewrite orphaned
(`the_sender_identifier_is_not_the_authors_key`) is deleted, and its sibling in
`sender.rs` is split into a literal format pin and `nothing_a_sender_identifier_is_made_from_is_a_key`.

**Mutations run (two; both restored, `git status --short` empty afterwards):**

1. `Delivering::start`, the startup marking `let opens = self.startup_opens(..)`
   moved to after the subscription. The spec's "startup MUST count every channel
   as being opened before it checks any message" is a race, and the test says it
   is not deterministic. Ran
   `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core a_restarted_peer_keeps_what_delivery_hands_over`
   twice: **red both times**, round 0, "inbound message refused
   (unknown-channel); nothing was stored". Killed.
2. `startup_opens`, `.iter()` followed by `.take(1)` (only the first recorded
   Stoa counted as being opened). Ran
   `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core delivery::tests`:
   **5 red** (`a_restarted_peer_keeps_what_delivery_hands_over_…`,
   `a_restarted_peer_requests_every_stoas_channel_and_no_other`,
   `a_peer_in_more_stoas_than_a_page_requests_every_channel_at_startup`,
   `node_creation_precedes_every_channel_operation`,
   `a_declined_node_creation_does_not_stop_the_module`). Killed, but this
   mutation also stops the requests, so it does not isolate the marking from the
   asking; the first mutation is the one that does.

**Every new or changed scenario maps to a test that fails for the reason it
names.** By scenario: "already exists" opens (`a_channel_delivery_reports_already_existing_is_open`,
which also asserts the send goes to that channel); timeout then "already exists"
(`a_creation_delivery_did_not_complete_in_time_…`); restart keeps channels
(`a_module_restarted_while_delivery_kept_running_…`, and the gated
`a_restarted_peer_keeps_what_delivery_hands_over_…`); open waiting behind another
(`a_message_on_a_channel_whose_open_waits_behind_another_…`, which proves the
message was handed over before the queued open reached delivery by counting the
listener's reads, not by a clock); bounded wait; panic reading one message;
traffic on an unopened channel, an oversized payload on an open channel and a
payload at the limit (all three hold the boundary up on an unanswered open and
assert the refusals land while it is still held, via `answered_creates() == 1`).
No `NO SPEC:` marker appears in the range's tests.

- [x] **`spec-writer`** (then `tester`) — a requirement clause with no scenario, whose
      tests pin an internal rather than what the clause says.
      **Where:** `op-transport`, "Every payload the reliable channel delivers passes
      the inbound boundary", the two sentences "or given up by this peer without
      delivery being asked" and "an open this peer never goes on to ask delivery for
      is given up". No scenario names either. The two tests that cover it,
      `a_sender_identifier_that_cannot_be_retained_opens_no_channel` and
      `a_join_the_worker_cannot_take_is_given_up_and_not_left_opening`, assert
      `channels.is_known(..)` is false, a read of the wiring's own book. What the
      spec's clause is for is observable: a message on that channel is refused
      promptly, not after `SETTLE_LIMIT` (the tests' own comments say so, "every
      message on the channel would wait out `SETTLE_LIMIT`"). A change that kept
      the channel counted while making `is_known` answer false, or the reverse,
      would pass or fail on the internal and not on the behaviour.
      **Failure scenario:** a give-up path added later (a third reason the worker
      does not ask delivery) that forgets to settle: no scenario tells its author
      the obligation exists, and `is_known` is only checked on the two paths that
      exist today.
      **Fix shape:** a scenario (WHEN an open is requested and this peer then
      never asks delivery for it, AND a message arrives on that channel, THEN it
      is refused as arriving on an unknown channel without the wait bound
      elapsing), and a test through `decide` with the injectable limit set long,
      asserting the refusal arrives inside a short time.
      **Not measured**, read only. **Severity:** low.
      **Outcome (`spec-writer`): fixed on the spec side; box left for the
      `tester`.** New `op-transport` scenario, under "Every payload the reliable
      channel delivers passes the inbound boundary": "An open this peer gives up
      without asking delivery does not hold a message up" — with sender
      identifiers unretainable and a Stoa's channel to be opened for the first
      time, a message arriving once the log has recorded that Stoa is refused as
      an unknown channel before the fixed wait has passed, and no creation is
      requested. The trigger is the sender-identifier failure because that is
      the give-up path the spec itself names; the worker that cannot take a
      join is not spec-visible, so the tester may drive that path as well against
      the same THEN. A future give-up path is covered by the requirement's
      "an open this peer never goes on to ask delivery for is given up", which no
      scenario can enumerate in advance. The requirement text is unchanged apart
      from the settled-by list, reworded for the security entry's per-open wait.
      **Outcome (`tester`): fixed, both give-up paths now checked from a message.**
      The scenario's own path, the running wiring with an unretainable sender
      identifier (`an_open_this_peer_gives_up_without_asking_delivery_does_not_hold_a_message_up`),
      is first made to prove the open was pending — the worker is held at node
      creation and the book asked, so a join that never marked its open cannot
      pass it by being refused at once — and then the message is sent as soon as
      the log says the open was given up, and must be refused inside
      `eventually`'s ten seconds where the running wiring's wait is forty. The test
      had carried a `started.elapsed() < SETTLE_LIMIT` assertion after
      `eventually`, which could not fail independently and is dropped. The two
      `is_known` tests you named (`a_sender_identifier_that_cannot_be_retained_opens_no_channel`,
      `a_join_the_worker_cannot_take_is_given_up_and_not_left_opening`, the worker
      that cannot take a join, which the spec-writer left for this box) keep
      their book assertion and gain your fix shape: a message decided by a
      processor whose limit is a minute is refused inside ten seconds
      (`refused_without_waiting_out_the_limit`). Mutations, each restored:
      the guard forgotten on the no-sender path (`mem::forget(opening)` before the
      return) — predicted red in the wiring test and in the sender-identifier
      test, observed both (the wiring test at "timed out waiting: the message to be
      refused", 10 s; the other at its book assertion, which fires before the new
      message check); the refused action forgotten in `request` with the
      `is_known` assertion of the worker-gone test neutralised, to see the message
      check alone — predicted red, observed "timed out waiting: the message to be
      refused without waiting", so the new check can fail without the internal
      one; and `is_known` narrowed to open channels only, for the pre-check —
      observed red at "the join's open was not counted as being opened while it
      waited on the worker".

- [x] **`spec-writer`** — the rule that recognises "already exists" is unspecified,
      and its negative side has one test row.
      **Where:** `stoa-membership`, "Creating or joining a Stoa opens its reliable
      channel": "delivery answers that the channel already exists" MUST open the
      channel; "delivery answers the creation with its error shape for any other
      reason" does not. The spec does not say what makes an answer that one:
      delivery's wording is free text (`ChannelCreate failed: channel already
      exists: <id>`), and the tests build that exact string, including this
      channel's id. The only negative row in
      `only_delivery_s_already_exists_answer_opens_a_declined_channel` is
      `channel_create callback timeout`. No test says whether an answer that says
      "already exists" of a different subject or a different channel (or the
      `Context already initialized` wording `CLAUDE.md` records for node creation)
      opens this Stoa's channel, and the spec, which is the only place that can say,
      is silent.
      **Failure scenario:** a match on the substring `already exists` opens a channel
      on delivery's answer about something else; a match that also required the
      channel id would decline a genuine answer if delivery reworded. Both pass
      every test here.
      **Fix shape:** state the recognised form in the requirement (delivery's
      answer that names this channel's identifier as already existing), add a
      scenario for an "already exists" answer naming another channel (still a
      decline), and have the tester add that row. If the intent is a looser match,
      say so instead.
      **Not measured** (the recogniser's code is outside what this role reads).
      **Severity:** low.
      **Outcome (`spec-writer`): fixed, as the looser match, said so.**
      `stoa-membership`, "Creating or joining a Stoa opens its reliable channel",
      now states the recognised form: delivery's reason contains its own words
      `channel already exists`, whatever surrounds them and whether or not it
      names the channel identifier, because the request names one channel and the
      answer is about that one; a reason without those words is a decline,
      including one saying something other than a channel already exists or is
      already initialised. Requiring the identifier was not taken: it guards
      against an answer about another channel, which a request naming one
      channel does not receive, and it would decline a genuine answer if
      delivery dropped the trailing id. New scenarios: "An \"already exists\"
      answer that does not name the channel opens it" and "A decline saying
      something else already exists does not open the channel" (the `Context
      already initialized` wording among its rows). **For the `tester`:** rows
      for both scenarios, since no box of theirs carries this.
      **Follow-up (`tester`):** the dev-writer added the rows to the recogniser's
      own test (`only_delivery_s_already_exists_answer_opens_a_declined_channel`,
      on `channel_answer`), which cannot see the scenarios' THENs (a message
      stored or refused, a post sent or not). `what_delivery_says_of_a_channel_decides_whether_the_channel_opens_and_a_post_is_sent`
      is one table through the whole wiring, a peer per row: no id in the answer
      (opens: message stored, post sent), `Context already initialized` and
      "content topic already exists" (do not: message refused as unknown, post
      not sent). Mutations, restored: matching `already` instead of the
      words, red at the `Context already initialized` row; requiring the
      channel id (a second colon) in the answer, red at the no-id row.

**Areas that were clean, and one observation with no box.** Scenario coverage
for the range is complete apart from the two boxes above. The self-consistency
read found no contradiction: the hand-over refusals, the being-opened rule, the
bounded wait and the startup rule agree, and `proposal.md` repeats the
requirement text without diverging from it. Nothing in the spec is now out of
scope of #176 (reliable channel throughout; the owner's request for an
automated two-peer test remains the project manager's follow-up, and `tasks.md`
7.3 remains unrun, which `proposal.md` now says openly). Observation, no box:
the bounded wait is per message ("no later than a fixed time after this peer
began waiting on it"), so N messages behind one never-answered open stall the
processor N limits in a row; `design.md` (Decisions around lines 395-435) records
this as accepted, and no test holds more than one waiting message, which is
consistent with the spec as written.

## Re-review round 2 `369561d1..2cb71aaf`

Reviewed at `2cb71aaf` (tree at `0a8f8639`, which adds only findings). Read: the
range's diffs of `specs/op-transport/spec.md`, `specs/stoa-membership/spec.md`
and `proposal.md`; the whole `delivery/tests.rs` diff; the `mod tests` hunk of
`transport.rs` and the comment-only `arrival.rs` hunk; my round-1 boxes above.
The implementation was read only at the two lines each mutation touched, plus
the doc comment on the function they sit in. `dialectica/logos-rust-sdk-src` was
missing and staged with the `nix build --inputs-from` command; baseline
`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
exited 0 on the restored tree.

**Round-1 outcomes confirmed.** The "given up without asking delivery" box: the
scenario now exists and is tested from a message in both give-up paths
(`refused_without_waiting_out_the_limit`, limit 60 s inside `eventually`'s 10 s),
and the wiring test first proves the open pending, so a join that never marks its
open cannot pass it. The "already exists" recogniser box: two scenarios, rows
through `channel_answer` and through the whole wiring
(`what_delivery_says_of_a_channel_decides_whether_the_channel_opens_and_a_post_is_sent`,
message stored or refused and post sent or not, one peer per row, both
"something else already exists" and "already initialized" rows present).

**Mutations run (two; both restored, `git status --short` empty afterwards).**
Command for both: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core delivery::tests`.

1. `Pending::wait_ends`, `*self.wait_ends.get_or_insert_with(|| Instant::now() + limit)`
   replaced by `Instant::now() + limit` (the time kept per message, not per open).
   **Killed**, by exactly the two tests written for it: `many_messages_on_one_unanswered_open_…`
   ("the op behind the stuck open waited 4.015s: more than one wait of 1s") and
   `an_opens_time_is_the_same_for_every_message_that_waits_on_it_until_a_request_clears_it`
   ("a later message on the same open was given its own time"). The other 92 passed,
   so no other test was carrying this property.
2. `Channels::opening`, the line `pending.wait_ends = None;` deleted (a new request
   no longer starts a new wait). **Killed**, by exactly two:
   `a_new_request_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
   ("judged without waiting on the new request") and the relation test again
   ("a request for the channel did not start a new wait"). Not mutated, read only:
   the storm test `a_request_made_while_a_message_waits_does_not_extend_that_messages_wait`
   (deadline re-read on each wake-up) is a probe that can fail as designed: the
   storm outlives the wait and `eventually` has ten seconds against a 300 ms limit;
   it also asserts the refusal came no sooner than the limit, so a message that
   never waited cannot pass it.

**Scenarios added or changed, each mapped to a test that fails for the reason it
names:** many messages on one open (test 1 above); wait expired then delivery
answers (`an_open_whose_wait_has_expired_still_opens_…`: asserts the queue holds
the message after hand-over, so a refusal-on-hand-over fails there, then that it is
stored); new request after expiry (mutation 2); requests while a message waits
(the storm test); give-up without asking (above); the two "already exists"
wordings (above). Self-consistency read: the per-open time, the "a request starts a
new wait for later messages and does not lengthen a wait under way" paragraph and
the "expired" paragraph agree once "the time starts when a message first begins
waiting after the latest request" is read as one time per request. The
`transport.rs` test (`the_boundary_looks_a_channel_up_under_the_messages_own_identifier`)
is a structure pin for the existing stoa-mismatch scenario, not a new behaviour.
Nothing in the range is out of scope of #176 (reliable channel throughout); I did
not re-read the issue's comments since round 1, and its state is unchanged in what
this range touches.

- [ ] **`tester`** — "Opens left unanswered at the same time hold the messages on
      every other channel up once each, one after another" has no test, and no
      scenario. **Where:** `op-transport`, the paragraph after "That wait is bounded
      by a fixed time for each open". Every test in the range holds one channel
      stuck (`many_messages_on_one_unanswered_open_…` uses one stuck Stoa; the
      relation test reads one channel's book entry). **Failure scenario:** the time
      moved from the per-channel `Pending` to a single value shared by the
      processor (or keyed by the first channel that ever waited) makes a second
      stuck channel's messages judged early, which breaks "judged only once that
      open is settled, unless that channel's own wait expires first". Both mutation
      kills above would still hold, as would every other test here, because none
      has a second pending open with a message waiting on each. **Fix shape:** two
      stuck opens, a message on each, then a valid op on an open channel; assert
      the second stuck message is refused no earlier than the limit after it began
      waiting and the valid op is stored within about two limits, not four; or, if
      the paragraph is a description and not a requirement, the `spec-writer` says
      so and no test is owed. **Not measured** (the mutation was not run; the
      budget was spent on the two above). **Severity:** low.
      **Spec side (`spec-writer`): the paragraph is a description, and a test is
      still owed; box left for the `tester`.** The paragraph now opens "The
      following is a consequence of the requirements above and adds none", and
      says "at most once" and "never extends past the moment the last of those
      opens settles" (the round-2 security and readability reviewers showed the
      old "how long they can do so in all is that fixed time for each wait" read
      as a reachable total, which it is not once opens settle on one clock). But
      the failure scenario above does not break that paragraph: it breaks the MUST
      two paragraphs up — a message on a channel being opened "MUST be judged only
      once that open is settled … unless the wait below expires first", with the
      time started "when a message first begins waiting on the open". A time
      shared across opens judges the second stuck channel's message before its own
      wait expires, which that MUST forbids, and no scenario held two stuck opens.
      Added to `op-transport`: **"Each unanswered open's wait is its own"** — two
      unanswered opens, a message on each, then a valid op on an open channel;
      both refused as unknown while delivery has not answered, the second refused
      no sooner than the fixed time after the first was refused, and the valid op
      stored before three fixed times from when the first began waiting. That is
      your fix shape; the test is owed against that scenario.

- [x] **`spec-writer`** — the relation `delivery's own timeout < CALL_TIMEOUT <
      SETTLE_LIMIT` is pinned by a test and stated nowhere in the spec.
      **Where:** `delivery::tests::the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
      hardcodes delivery's 30 s and compares the two constants. No scenario or
      requirement says the fixed time a message may wait must exceed the longest
      this peer waits for delivery's answer to a creation, and "a fixed time" as
      the spec writes it is satisfied by one second. **Failure scenario:** a fixed
      time under delivery's own 30 s meets every requirement and scenario here (the
      scenarios use an injected limit of 100 ms to 1 s) while reopening in
      production the race the requirement exists to close: a message handed over
      before its join's creation answers is refused as an unknown channel and lost,
      the very case the "waits behind" and "racing its own join" scenarios describe.
      Only the test, which is `design.md`'s pin and maps to no scenario, holds the
      line. **Fix shape:** either say in the requirement that the fixed time
      outlasts the longest this peer waits on delivery for a creation (leaving the
      value in `design.md`), or record in the change that the relation is a design
      choice the spec deliberately does not contract. **Severity:** low.
      **Outcome (`spec-writer`): fixed, as a requirement.** Your failure scenario
      is right: without the order, "stored once delivery reports the channel
      created" holds only if delivery reports within whatever the fixed time is,
      and one second satisfied every sentence. `op-transport`, after "That wait is
      bounded by a fixed time for each open", now says: "That fixed time MUST be
      longer than the longest this peer waits for delivery to answer one channel
      creation, and that longest MUST be longer than the time delivery allows
      itself to answer one." It says what follows (a message racing the creation
      this peer is waiting on is judged only after that creation is answered or
      given up), that the order is contracted and the values are not, and that an
      open queued behind other requests is not covered by it — which is Decision
      11's stated cost, so the spec now agrees with it rather than being silent.
      New scenario **"A message's wait outlasts this peer's wait on a creation,
      which outlasts delivery's own"** (the three compared; delivery's shortest,
      the message's wait longest), which
      `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
      already satisfies as written. `proposal.md`'s "Receiving" bullet says the
      same. No code change: 30 s < 35 s < 40 s holds today, and a queue-sized
      limit, if the `dev-writer` reconsiders 40 s, would satisfy it too.
