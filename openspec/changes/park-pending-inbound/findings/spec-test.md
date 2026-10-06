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

- [ ] **`spec-writer`** — `spec.md`, "Parked messages are bounded per channel and in total", the sentence "The count bound is restored first, then the byte bound."
      **Scenario:** one arriving payload leaves both totals exceeded: the count total is full (channel A holds the most messages) and the arrival is large enough to push the byte total over (channel B holds the most bytes). Count-first discards A's newest, then B's; byte-first discards B's newest first and may stop there. Which messages are lost depends on the order, and no scenario says which.
      **Measured:** mutation 1 above (order swapped), both suites green.
      **Severity:** medium. It is a MUST on a bound that attacker traffic drives. Add a scenario with both totals exceeded, or drop the sentence.

- [ ] **`spec-writer`** — `spec.md`, both discard rules ("Inbound payloads waiting to be taken are bounded" and "Parked messages are bounded per channel and in total"): "When several channels hold the most, … otherwise the one among them whose newest … arrived latest."
      **Scenario:** two channels tie for the most and neither is the arriving payload's. The rule picks the one whose newest was handed over latest. Every scenario for the tie has the arrival's channel in it ("loses a tie for the most"); the sentence is stated and never exercised. `parked.rs` says so itself ("No scenario pins this tie-break").
      **Measured:** mutation 2 above (queue's per-channel newest made the first place), 1349 tests green.
      **Severity:** low-medium. Add a scenario per requirement, or say the second clause is a consequence of the first.

- [ ] **`tester`** — `delivery.rs` `InboundQueue::offer` (waiting payloads), "otherwise the one among them whose newest waiting payload arrived latest"
      **Scenario:** bound 4, waiting `a1 b1 b2 a2`, then a payload on a third channel. `a` and `b` tie at two; `a2` arrived latest, so `a2` is discarded. With a channel's newest read as its first place `b` (or either, by hash order) is chosen. Nothing fails. The only coverage of this clause is `shed`'s unit test and `parked`'s `among_other_channels_tied…`, neither of which goes through the queue's own load calculation.
      **Measured:** mutation 2, survived.
      **Severity:** medium (the only test of the queue's tie-break is on a different function).

- [ ] **`tester`** — `plan_park`, count-before-byte order
      **Scenario:** see the first `spec-writer` entry. Once the spec says which order, add the test with both totals exceeded by one arrival: it fails when the order is swapped.
      **Measured:** mutation 1, survived. `over_the_total_bytes_…` has the count total not exceeded (5 of 5), so only the byte loop ever runs.
      **Severity:** medium.

- [ ] **`tester`** — `parking.rs` — "Parked messages are bounded", the byte-bound and tie scenarios never reach the module's log
      **Scenario:** "A channel at its byte bound discards the arrival" and "The arriving payload's channel loses a tie for the most parked" both say "recorded as discarded from the parked messages". They are pinned in `parked.rs` against `Parking { parked, evicted }` only. A processor that discarded without logging, or logged the byte-bound discard as a refusal, would pass: only the count-bound and the total-count (non-tie) cases go through `processor.decide` and the journal.
      **Severity:** low-medium.

- [ ] **`tester`** — a parked discard's record is never checked for the payload or the sender identifier
      **Scenario:** the spec says the parked-message discard's record "MUST NOT carry the payload or the sender identifier". The `zzyzx` echo checks cover the park record (`a_park_is_logged_as_a_park…`), the refusals, and the waiting-queue discard (`every_discard_is_counted…`). `a_discard_from_the_parked_messages_shares_the_running_count…` only checks `"refused"` and `"parked"` words. `Note::ParkDiscarded { total }` has no payload field today (seen in `delivery.rs:395` while choosing a mutation), so a leak needs a type change; the test is the guard against it.
      **Severity:** low.

- [ ] **`tester`** — "Parked messages are reviewed on three events and no others": two claims with no test of their own
      **Scenario:** (a) "A later startup in the same module process begins no review" — `node_creation_is_requested_once_however_often_startup_runs` calls `start` twice but nothing is parked, so a second startup review would find nothing to refuse and the test could not tell. It is held only by the same guard that stops node creation twice. (b) "If the parked messages cannot be read at a review" is tested for the held review only; the startup review's read failure (messages parked, store unreadable, startup refusing nothing and logging a storage failure) is not.
      **Severity:** low.

- [ ] **`tester`** — "Messages parked on an open delivery declines are refused" and "… this peer gives up" never run through the worker
      **Scenario:** the tests stand in for the settle with `drop(opening)` and `Opening::held()`. The worker's own mapping (an error envelope, or `Err("timed out")`, becomes a dropped guard, and so an `Unopened` review) is tested separately, as "channel NOT open" in `a_declined_channel_creation_is_read_in_every_shape…` and as a later refusal in `a_message_taken_after_its_open_was_declined…`, but no test has a message already parked when a scripted `Err` or error envelope arrives. The chain park, worker decline, review, refusal is joined only by the guard's `Drop`. The give-up after asking delivery and receiving no answer is the spec's own wording and has no test beyond the `Err` shape's "not open".
      **Severity:** low. The pieces are each covered.

- [ ] **`tester`** — `a_message_a_review_decided_is_not_decided_again`, the restart half cannot fail
      **Scenario:** the spec step is "the module is then restarted and delivery answers startup's creation … already exists". The test runs `Review::Startup([channel])` instead: a set that names the channel, so it refuses nothing and decides nothing whatever remains parked. Only the repeated held review (`held(channels.opening(&channel))`) can find a message decided twice. Restart through `Peer::restarted()` as `startup_refuses…` does, or drop that half.
      **Severity:** low.

- [ ] **`tester`** — name promises more than the body: `a_parked_message_carries_no_sender_identifier_into_its_review`
      **Scenario:** it parks and scans the file; no review runs, so nothing about "into its review" is observed. The scan itself is sound (positive control above). Rename it to the store property it pins.
      **Severity:** low.

- [ ] **`spec-writer`** — `spec.md`, "When the message chosen is the payload being parked, it is discarded and nothing more is."
      **Scenario:** the count total is full and channel A holds the most; the arriving payload is large, on channel B, which holds the most bytes. The count loop discards A's newest, the byte loop then picks B (the arrival), so the arrival is discarded too: two messages lost for one arrival, and "nothing more" reads as "after this one", not "none before it". `plan_park` does this (`_ => return Plan { park: false, evict }` keeps the evictions already chosen). The text allows both readings. Say which, and add a scenario.
      **Severity:** low-medium. The cheaper reading (no evictions when the arrival ends up discarded) is also the one that never loses a message to a payload that was lost anyway.

- [ ] **`spec-writer`** — unmarked decisions pinned by tests with no scenario behind them (`NO SPEC:` is absent from every parking test)
      **Scenario:** (a) the park record carries no channel identifier (`a_park_is_logged_as_a_park…` asserts it; the spec says only "payload or sender identifier", and a channel being opened is arguably not "a channel identifier this peer has no channel open under"); (b) a parked-message store of a future or mislabelled layout is refused (`a_store_from_a_future_layout_is_refused`, `a_mislabelled_file_is_refused_at_open`), with no word on what a message does when the store will not open beyond "cannot be written/read"; (c) the four bound values 256 messages / 8 MiB per channel and 1024 / 32 MiB total (`the_park_bounds_are_pinned`), though the spec says "The values are not contracted here". Decide whether each is contract or design, and say so.
      **Severity:** low.

- [ ] **`tester`** — add the `NO SPEC:` comment to the tests of the three choices in the entry above, so the next reader does not take them for scenario-backed
      **Severity:** low.

- [ ] **`tester`** — "A parked op is not readable as an op" samples one read of the module's surface
      **Scenario:** the requirement says no reply the module gives to any call carries the parked op. `a_parked_op_is_not_readable_as_an_op…` checks the op log's length, `clock`, and `list_threads_from_request`. A call that read the parked file directly (a post, a thread or a revision read) would not be seen. Task 4.1 names this as "by construction"; it is a limit, not a defect, but the box on 4.1 is unticked and says nothing of which other reads exist.
      **Severity:** low.
