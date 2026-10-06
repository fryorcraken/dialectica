# spec-test review: park-pending-inbound

Reviewed: `specs/op-transport/spec.md` (the delta), `delivery/tests/parking.rs`,
the parking and receiving sections of `delivery/tests.rs`, and `parked.rs`'s
own tests. Issue #199 read fresh with `gh issue view 199`.

Mutations run (both restored; `git status` clean afterwards, nothing left in
the tree):

1. `parked.rs` `plan_park`: swapped the order of the two entries of `measures`
   (byte bound restored before the count bound). **Survived.** Commands:
   `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core parked::`
   (17 passed) and `... -p dialectica-core delivery::tests::parking` (35 passed).
2. `delivery.rs` `InboundQueue::offer`: made each channel's "newest" its
   *first* waiting place instead of its last. **Survived.** Command:
   `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
   (1349 passed, 0 failed).
3. A positive control, not a mutation: added an assert to
   `a_parked_message_carries_no_sender_identifier_into_its_review` that the
   payload's own text IS in `parked.sqlite`. It passed, so the byte scan for the
   sender identifier is not vacuous (restored).

Clean, in prose. **The park trigger** (`ChannelBook::on_take`, the four states
including "open with a repeat request unsettled"), **the review triggers**
(`settle`: held, last-and-unopened, not-last, repeat-declined-on-open; the
startup set; "nothing but the three events") and **the exactly-once ordering**
(take returns a review before a payload; a payload taken before a settle is
parked and the settle's review follows; one taken after is judged) each have a
test that can fail for the reason it names, read against the spec scenario by
scenario. The migration in part 4 is faithful: every scenario kept by the
REMOVED blocks' Migration notes is present and verbatim save the two stated
edits, the thirteen wait scenarios are accounted for, and nothing outside
`op-transport` cites a removed requirement name. The issue's expiry question is
answered by "not the passing of time" plus every request settling; no stale
scope. Task 2.7's `grep` claim holds (no `SETTLE_LIMIT`, `await_settled` or
`OpenTime` under `dialectica/`). No `NO SPEC:` marker exists anywhere in the
parking tests.

## Findings

- [x] **`spec-writer`** — `spec.md`, "Parked messages are bounded per channel and in total", the sentence "The count bound is restored first, then the byte bound."
      **Scenario:** one arriving payload leaves both totals exceeded: the count total is full (channel A holds the most messages) and the arrival is large enough to push the byte total over (channel B holds the most bytes). Count-first discards A's newest, then B's; byte-first discards B's newest first and may stop there. Which messages are lost depends on the order, and no scenario says which.
      **Measured:** mutation 1 above (order swapped), both suites green.
      **Severity:** medium. It is a MUST on a bound that attacker traffic drives. Add a scenario with both totals exceeded, or drop the sentence.
      **Outcome (spec-writer): fixed in the spec.** The order is kept, count first, as the code has it, and is now stated as a choice procedure: "Messages are chosen for the count bound first, until it would hold, and then for the byte bound, until it would hold", the byte choices "leaving out every message already chosen". Added scenario "Over both total bounds at once, the count bound is restored first": count total full, A holds the most messages, B the most bytes, an arrival on a third channel whose overage exceeds A's newest and does not exceed B's newest; A's newest and B's newest are both discarded and the arrival is parked. Swapping the order keeps A's newest, so the scenario fails under mutation 1. The test is the `tester`'s entry below.

- [x] **`spec-writer`** — `spec.md`, both discard rules ("Inbound payloads waiting to be taken are bounded" and "Parked messages are bounded per channel and in total"): "When several channels hold the most, … otherwise the one among them whose newest … arrived latest."
      **Scenario:** two channels tie for the most and neither is the arriving payload's. The rule picks the one whose newest was handed over latest. Every scenario for the tie has the arrival's channel in it ("loses a tie for the most"); the sentence is stated and never exercised. `parked.rs` says so itself ("No scenario pins this tie-break").
      **Measured:** mutation 2 above (queue's per-channel newest made the first place), 1349 tests green.
      **Severity:** low-medium. Add a scenario per requirement, or say the second clause is a consequence of the first.
      **Outcome (spec-writer): fixed in the spec**, one scenario per requirement. The clause is not a consequence of the first, so it gets scenarios rather than a remark. Queue: "Among other channels tied for the most waiting, the one whose newest arrived latest gives up its newest". Parked: "Among other channels tied for the most parked, the one whose newest was handed over latest gives up its newest". Each fixes the interleaving so that "the first and the last" of the two tied channels' messages are on the first channel (your `a1 b1 b2 a2`), which is what makes a channel's first place and its newest disagree, so mutation 2 can fail it. The tests are the `tester`'s.

- [x] **`tester`** — `delivery.rs` `InboundQueue::offer` (waiting payloads), "otherwise the one among them whose newest waiting payload arrived latest"
      **Scenario:** bound 4, waiting `a1 b1 b2 a2`, then a payload on a third channel. `a` and `b` tie at two; `a2` arrived latest, so `a2` is discarded. With a channel's newest read as its first place `b` (or either, by hash order) is chosen. Nothing fails. The only coverage of this clause is `shed`'s unit test and `parked`'s `among_other_channels_tied…`, neither of which goes through the queue's own load calculation.
      **Measured:** mutation 2, survived.
      **Severity:** medium (the only test of the queue's tie-break is on a different function).
      **Outcome (tester): fixed.** `delivery/tests/parking.rs`
      `among_other_channels_tied_for_the_most_waiting_the_one_whose_newest_arrived_latest_loses_it`:
      bound four, `f1 s1 s2 f2` handed over, then a payload on a third channel,
      through `Channels::hand_over` and the module's log, and run the other way
      round as well, so neither a channel's name nor a map's order picks. The
      expected order taken, `f1 s1 s2 z1`, is written out from the spec's
      scenario. **The code changed since you read it:** the queue no longer has a
      load calculation of its own, because `offer` asks `shedding::choose` (the
      `dev-writer`'s fix to the architecture finding). So the mutation was
      re-aimed, twice. (a) Your mutation 2 in its new home: in
      `shedding::choose` a channel's newest left at its first place. Predicted:
      this test, `shedding::tests::among_other_channels_tied_…`,
      `parked::tests::among_other_channels_tied_…` and the table row in
      `every_parking_bound_rule_discards_…` fail. Observed: those four, this one
      with the queue taken as `[f1, s1, f2, z1]` against `[f1, s1, s2, z1]`.
      (b) What is still the queue's own, `rposition` in `InboundQueue::offer`
      changed to `position`. Predicted: this test and the two `a_full_queue_…`
      tests fail. Observed: those three. Both restored.

- [x] **`tester`** — `plan_park`, count-before-byte order
      **Scenario:** see the first `spec-writer` entry. Once the spec says which order, add the test with both totals exceeded by one arrival: it fails when the order is swapped.
      **Measured:** mutation 1, survived. `over_the_total_bytes_…` has the count total not exceeded (5 of 5), so only the byte loop ever runs.
      **Severity:** medium.
      **Outcome (tester): fixed**, at two layers, from the spec's new scenario
      "Over both total bounds at once, the count bound is restored first".
      `parked::tests::over_both_total_bounds_at_once_the_count_bound_is_restored_first`
      (the store) and its row in
      `every_parking_bound_rule_discards_what_the_spec_says_and_logs_it` (the
      processor and the log). The numbers are worked by hand in the store test's
      comment: `a` holds three 10-byte messages (the most messages), `b` two
      150-byte (the most bytes), five messages and 330 of 500 bytes; `c` brings
      200 bytes, 30 over, which is more than `a`'s newest (10) and no more than
      `b`'s (150). Count first, two messages are discarded; bytes first, one.
      Your mutation 1 (the two totals visited in reverse order). Predicted:
      `Parked { evicted: 2 }` becomes `evicted: 1`, and in the table one discard
      is logged where two are expected. Observed: `left: Parked { evicted: 1 },
      right: Parked { evicted: 2 }`, and the table's "Over both total bounds at
      once" row failed on its discard count (1 against 2). No other test in
      `parked::` or `delivery::tests::parking` failed. Restored.

- [x] **`tester`** — `parking.rs` — "Parked messages are bounded", the byte-bound and tie scenarios never reach the module's log
      **Scenario:** "A channel at its byte bound discards the arrival" and "The arriving payload's channel loses a tie for the most parked" both say "recorded as discarded from the parked messages". They are pinned in `parked.rs` against `Parking { parked, evicted }` only. A processor that discarded without logging, or logged the byte-bound discard as a refusal, would pass: only the count-bound and the total-count (non-tie) cases go through `processor.decide` and the journal.
      **Severity:** low-medium.
      **Outcome (tester): fixed.** `every_parking_bound_rule_discards_what_the_spec_says_and_logs_it`,
      a table of the spec's scenarios run through `Processor::decide` with the
      bounds shrunk on the one processor: a channel at its byte bound; the
      arrival's channel losing a tie; the tie between two other channels, and its
      mirror; both totals at once; and a payload discarded for the byte total.
      Each row's expected result is what each channel still holds, as tags,
      written out from the scenario; the discards, the "N discarded since" on the
      last one, the parks and the absence of any refusal or store follow from it
      and are asserted on the module's log. Mutation: `Processor::park` logging
      a discarded arrival as a refusal (`Note::Refused`) and counting nothing.
      Predicted: every row whose arrival is discarded fails. Observed: "A channel
      at its byte bound discards the arrival" failed (`left: 0, right: 1`
      discards), with four other tests. **A limit worth saying:** every
      `ParkOutcome::Discarded` goes through one arm of `Processor::park`, so the
      byte-bound row shows the byte bound reaches that arm and is logged, and
      cannot show a byte-specific logging branch, which does not exist; the
      store's own tests tell the byte bound from the count bound.

- [x] **`tester`** — a parked discard's record is never checked for the payload or the sender identifier
      **Scenario:** the spec says the parked-message discard's record "MUST NOT carry the payload or the sender identifier". The `zzyzx` echo checks cover the park record (`a_park_is_logged_as_a_park…`), the refusals, and the waiting-queue discard (`every_discard_is_counted…`). `a_discard_from_the_parked_messages_shares_the_running_count…` only checks `"refused"` and `"parked"` words. `Note::ParkDiscarded { total }` has no payload field today (seen in `delivery.rs:395` while choosing a mutation), so a leak needs a type change; the test is the guard against it.
      **Severity:** low.
      **Outcome (tester): fixed.**
      `a_parked_discard_is_logged_without_the_payload_the_sender_or_the_channel`,
      both ways a message is discarded from the parked ones (the arrival over its
      own channel's count bound; an earlier message for the total count). Every
      payload and sender identifier carries `zzyzx`, and the record is searched
      for it in the three spellings the other tests use, and for both channels'
      identifiers (the spec's channel-identifier rule). Mutation: the arrival's
      sender identifier appended to the discard's log line. Predicted: fails on
      the first echo. Observed: failed on `zzyzx`, the line reading "…1
      discarded since the module started zzyzx-sender-identifier". **A limit:**
      for the evicted message the processor holds only a count, never its payload
      (the store returns a number), so that row can guard what the processor
      holds and no more. Restored.

- [x] **`tester`** — "Parked messages are reviewed on three events and no others": two claims with no test of their own
      **Scenario:** (a) "A later startup in the same module process begins no review" — `node_creation_is_requested_once_however_often_startup_runs` calls `start` twice but nothing is parked, so a second startup review would find nothing to refuse and the test could not tell. It is held only by the same guard that stops node creation twice. (b) "If the parked messages cannot be read at a review" is tested for the held review only; the startup review's read failure (messages parked, store unreadable, startup refusing nothing and logging a storage failure) is not.
      **Severity:** low.
      **Outcome (tester): fixed, both.**
      (a) `a_later_startup_in_the_same_module_process_begins_no_review`: after
      the first startup (its review known to have run, because a payload taken
      after it is parked) a message is put in the parked file on a channel
      nothing opens, as an unreadable review leaves one; a second `start` must
      return `false` and refuse nothing. "Refuses nothing" is a 500 ms watch,
      which can pass falsely and never fail falsely. Removing the guard in
      `Delivering::start` fails three tests, this one on the return value, so
      that mutation does not isolate the claim; the isolating one is a second
      `start` that returns `false` as before but first queues
      `begin_startup_review` on the running wiring. Predicted: only this test
      fails. Observed: only this test, on the refusal of the orphan
      ("refused (unknown-channel)" in the log). (b)
      `a_startup_review_that_cannot_read_the_parked_messages_refuses_nothing_and_logs_it`:
      a message parked, the file unreadable at startup and readable after it;
      the storage failure is logged, nothing is refused, the message is still
      parked. Mutation: the read failure in `refuse_unknown_at_startup` silently
      returned. Predicted and observed: "timed out waiting: the startup review
      to log a storage failure". Both restored.

- [x] **`tester`** — "Messages parked on an open delivery declines are refused" and "… this peer gives up" never run through the worker
      **Scenario:** the tests stand in for the settle with `drop(opening)` and `Opening::held()`. The worker's own mapping (an error envelope, or `Err("timed out")`, becomes a dropped guard, and so an `Unopened` review) is tested separately, as "channel NOT open" in `a_declined_channel_creation_is_read_in_every_shape…` and as a later refusal in `a_message_taken_after_its_open_was_declined…`, but no test has a message already parked when a scripted `Err` or error envelope arrives. The chain park, worker decline, review, refusal is joined only by the guard's `Drop`. The give-up after asking delivery and receiving no answer is the spec's own wording and has no test beyond the `Err` shape's "not open".
      **Severity:** low. The pieces are each covered.
      **Outcome (tester): fixed.**
      `messages_parked_on_an_open_the_worker_sees_declined_are_refused_in_every_shape_of_no`:
      two valid ops parked on a channel, then `Worker::open` answered by each of
      the three shapes of no in `the_ways_delivery_says_no` (a transport failure,
      an error envelope, a failure with no reason). Asserted: the worker logs the
      channel not open with delivery's words; no refusal is logged by the worker
      itself; after the review both are refused as an unknown channel, neither
      is stored, nothing is parked. **On "this peer gives up after asking
      delivery and receiving no answer":** this peer's timeout reaches the
      worker as `Err`, which is the transport-failure row, so that wording is
      the same path and no separate test of it can be written. Mutation: the
      worker's declined arm settling as held (`opening.finish(true)`).
      Predicted: the two messages are stored, not refused. Observed:
      `refused (unknown-channel)` 0 against 2 with both `stored inbound op`
      lines in the log, and four other tests failed. Restored.

- [x] **`tester`** — `a_message_a_review_decided_is_not_decided_again`, the restart half cannot fail
      **Scenario:** the spec step is "the module is then restarted and delivery answers startup's creation … already exists". The test runs `Review::Startup([channel])` instead: a set that names the channel, so it refuses nothing and decides nothing whatever remains parked. Only the repeated held review (`held(channels.opening(&channel))`) can find a message decided twice. Restart through `Peer::restarted()` as `startup_refuses…` does, or drop that half.
      **Severity:** low.
      **Outcome (tester): fixed.** The restart half now restarts through
      `Peer::restarted()`: the Stoa is joined before, so startup asks for its
      channel, whose creation delivery answers "already exists"; a junk payload
      is then handed over, and its refusal is read only once every waiting
      review has run. The new process's log must hold nothing of the op's, no
      "already held" and exactly one refusal (the junk), and the op is still
      stored with nothing parked. **A trap met on the way:** the needle
      "already held" alone matched the worker's own "delivery already held it"
      line for the channel, so the test names the processor's line,
      `inbound op <id> already held`. Mutation: `take_channel` no longer
      deleting what it takes. Predicted: fails. Observed: failed at the
      repeated held review in the first half, so that mutation does not reach
      the restart half; to see the restart half bite alone I skipped the
      first half's repeated review in the test, re-ran the mutation and it
      failed at the restart half's "already held" assertion; the test and the
      code were then restored.

- [x] **`tester`** — name promises more than the body: `a_parked_message_carries_no_sender_identifier_into_its_review`
      **Scenario:** it parks and scans the file; no review runs, so nothing about "into its review" is observed. The scan itself is sound (positive control above). Rename it to the store property it pins.
      **Severity:** low.
      **Outcome (tester): fixed.** Renamed
      `a_distinctive_sender_identifier_is_nowhere_in_the_parked_file`; its
      comment says it holds the bytes of the file, beside the columns that
      `parked::tests::nothing_parked_carries_the_sender_identifier_or_the_timestamp`
      holds. The body is unchanged.

- [x] **`spec-writer`** — `spec.md`, "When the message chosen is the payload being parked, it is discarded and nothing more is."
      **Scenario:** the count total is full and channel A holds the most; the arriving payload is large, on channel B, which holds the most bytes. The count loop discards A's newest, the byte loop then picks B (the arrival), so the arrival is discarded too: two messages lost for one arrival, and "nothing more" reads as "after this one", not "none before it". `plan_park` does this (`_ => return Plan { park: false, evict }` keeps the evictions already chosen). The text allows both readings. Say which, and add a scenario.
      **Severity:** low-medium. The cheaper reading (no evictions when the arrival ends up discarded) is also the one that never loses a message to a payload that was lost anyway.
      **Outcome (spec-writer): fixed in the spec, with the cheaper reading.** The sentence is replaced by: "If the payload being parked is chosen, for either total bound, that payload MUST be discarded and every message already parked MUST be kept, a message chosen before it included. Otherwise every message chosen MUST be discarded, and the payload MUST be parked." This matches the per-channel rule ("nothing already parked is"). Added scenario "A payload discarded for the byte total costs no message already parked": count total full, A holds the most messages, the arrival's own channel holds the most bytes even once A's newest is left out; the arrival is discarded, everything parked before it is still parked, and no other discard is recorded. **The code does not do this today** (`plan_park` keeps the count loop's eviction); the fix is the `dev-writer`'s second entry in `security.md`, which this decides, and the test is the `tester`'s.

- [x] **`spec-writer`** — unmarked decisions pinned by tests with no scenario behind them (`NO SPEC:` is absent from every parking test)
      **Scenario:** (a) the park record carries no channel identifier (`a_park_is_logged_as_a_park…` asserts it; the spec says only "payload or sender identifier", and a channel being opened is arguably not "a channel identifier this peer has no channel open under"); (b) a parked-message store of a future or mislabelled layout is refused (`a_store_from_a_future_layout_is_refused`, `a_mislabelled_file_is_refused_at_open`), with no word on what a message does when the store will not open beyond "cannot be written/read"; (c) the four bound values 256 messages / 8 MiB per channel and 1024 / 32 MiB total (`the_park_bounds_are_pinned`), though the spec says "The values are not contracted here". Decide whether each is contract or design, and say so.
      **Severity:** low.
      **Outcome (spec-writer): decided, one each.** (a) **Contract, now said.** A parked message's channel is not open when it is taken, so the existing rule in "A message a reliable channel delivers reaches the op log only through the inbound boundary" ("a channel identifier this peer has no channel open under") already covers it; the park paragraph now says so in one pointer sentence rather than restating the rule, and the new scenario "A park is logged without text the sender chose" backs `a_park_is_logged_as_a_park…`, channel identifier included. (b) **Design.** The layout version is `design.md` Decision 2's; the spec's contract for such a store is the one it already has for any store that "cannot be written" (storage failure, not held for another attempt) or "cannot be read" at a review (storage failure, messages stay parked). The two layout tests pin the mechanism and want a `NO SPEC:` comment. (c) **Design.** The values are `design.md` Decision 4's and stay uncontracted; the spec contracts only their ordering, which "The bounds are ordered as required" holds. `the_park_bounds_are_pinned` is a known-answer guard on a design constant and wants a `NO SPEC:` comment. The comments for (b) and (c) are the `tester`'s next entry; (a) no longer needs one.

- [x] **`tester`** — add the `NO SPEC:` comment to the tests of the three choices in the entry above, so the next reader does not take them for scenario-backed
      **Severity:** low.
      **Outcome (tester): fixed.** `NO SPEC:` comments, each saying which
      design decision the test pins and what the spec says instead, on
      `the_park_bounds_are_pinned` (Decision 4; the spec says "The values are not
      contracted here"), `a_store_from_a_future_layout_is_refused` and
      `a_mislabelled_file_is_refused_at_open` (Decision 2; the spec's only word is
      the storage failure of a store that "cannot be written" or "cannot be
      read"), and, beyond your list, `the_layout_version_and_file_name_are_pinned`
      (Decision 2, the same constant's known answer). Choice (a), the park
      record's channel identifier, needs none, as the `spec-writer` decided. The
      one existing marker, on `a_review_leaves_neither_the_size_nor_the_bytes_of_what_it_took_in_the_file`,
      is kept. Comments only: nothing to fail, and `parked::tests` is green.

- [x] **`tester`** — "A parked op is not readable as an op" samples one read of the module's surface
      **Scenario:** the requirement says no reply the module gives to any call carries the parked op. `a_parked_op_is_not_readable_as_an_op…` checks the op log's length, `clock`, and `list_threads_from_request`. A call that read the parked file directly (a post, a thread or a revision read) would not be seen. Task 4.1 names this as "by construction"; it is a limit, not a defect, but the box on 4.1 is unticked and says nothing of which other reads exist.
      **Severity:** low.
      **Outcome (tester): fixed in part, and the rest rejected.** Fixed:
      `a_parked_op_is_not_readable_as_a_thread` reads a thread by the parked
      op's id through `read_thread_from_request` (a root post's id is its
      thread's), after a control that the same request for a held post returns
      it, and asserts an error with no page and none of the body. Mutation: a
      processor that parks a message and also judges it, so the op is stored
      and the row is kept. Predicted: this test fails at the thread read.
      Observed: it failed there, the reply being the page with the op in
      `items`. (My first mutation judged instead of parking and failed this test
      at its `parked_on == 1` precondition, which does not show the read can
      fail, so it was replaced.) Rejected: a test of a read that "opens the
      parked file" has nothing to call, since the module has no such read, and
      the by-construction argument is `tasks.md` 4.1's. The box on 4.1 is the
      `dev-writer`'s and the `closer`'s; I tick only my own row. The two reads now
      sampled are the feed and the thread read; I did not enumerate the module's
      other reads (`get_stoa`, the revision reads) and make no claim about them.
