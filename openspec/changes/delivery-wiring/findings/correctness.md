# Correctness review — delivery-wiring

Dimension: **correctness only**. Reviewed at `7a2a3335` (`git diff origin/main...HEAD`):
`dialectica-core/src/delivery.rs`, `sender.rs`, the `transport.rs` / `wire.rs` /
`log/sqlite.rs` diffs, and the adapter in `dialectica/rust-lib/src/lib.rs`, read
against delivery v0.2.1 (`git -C ~/src/logos-co/logos-delivery-module show
v0.2.1:src/delivery_module_plugin.cpp`, `src/api_call_handler.h`) and
logos-delivery's `createReliableChannel` (`logos_delivery/channels/api/channel_lifecycle.nim`
at `4a85db1b`). `cargo mutants` was run on `delivery.rs` (60 mutants: 38 caught,
14 unviable, 2 timeouts, 6 missed).

- [x] **`dev-writer`** (and `spec-writer` for the promise in `stoa-membership`) —
      `delivery.rs:601-607` (`Worker::open`) with `declined` at `delivery.rs:129` —
      a channel delivery already holds can never become open again, so the recovery
      the log line and the spec promise cannot happen.
      **Why:** delivery v0.2.1's `callApiRetValue` gives up after its own 30 s and
      answers `"channel_create callback timeout"` while the Nim
      `createReliableChannel` carries on and creates the channel anyway. A later
      `channelCreate` for the same id is answered
      `"ChannelCreate failed: channel already exists: <id>"`
      (`channel_lifecycle.nim:42-43`). `declined` reads both as declines, so
      `Opening` settles `created: false` both times and `ChannelBook.open` never
      gets the id. The same thing happens when the dialectica module process restarts
      while delivery stays up: every Stoa's open at startup is answered "already
      exists".
      **Scenario:** fake delivery answers the first create with
      `{"error":"channel_create callback timeout","success":false,"value":null}` and
      the second with `{"error":"ChannelCreate failed: channel already exists:
      /dialectica/1/c/<hex>",…}`. Join, then join again, as `Note::ChannelDeclined`
      tells the user to. A valid op then arrives on that channel id. Result:
      `inbound message refused (unknown-channel)`, and the op is lost for good,
      because SDS has already treated it as delivered. Every later publish logs
      `not sent: no channel is open`. Delivery has the channel open throughout.
      **Measured:** a probe test in the reviewer tree
      (`reviewer_probe_channel_delivery_already_holds_never_opens`) fails as
      described. The channel stays dead for the whole of delivery's lifetime.
      **Severity:** high. Nothing else in the process can get the Stoa's delivery
      back.
      Two possible fixes, and the choice is a design decision: treat "already
      exists" as created (it matches delivery's own "Creates (or re-opens)" docstring,
      but depends on the wording of a message string), or confirm with
      `channelExists` after a decline, which widens the four-method seam.
      `stoa-membership`'s "requested again at the next module start, or when the
      Stoa is next created or joined" should then say what that request does when
      delivery already has the channel.
      **Spec outcome (`spec-writer`): fixed; box left for the `dev-writer`.**
      `stoa-membership`, "Creating or joining a Stoa opens its reliable channel",
      now says a channel is open once delivery reports that it holds it, "either
      that it created the channel, or that the channel already exists", and "An
      answer reporting that the channel already exists MUST open the channel,
      exactly as a report that delivery created it does"; a decline for any other
      reason is still not open, and the re-request sentence now says it opens the
      channel when delivery reports holding it. New scenarios: "A channel delivery
      reports already existing is open", "A creation delivery did not complete in
      time opens on the next request" (this finding's probe sequence), and, under
      the startup requirement, "A module restarted while delivery kept running has
      its channels open". "A channel delivery declines does not fail the join" and
      `op-transport`'s "…refused once delivery declines the open" now exclude the
      already-exists answer, and the pending-open sentence lists it as a way an
      open settles. The requirement is phrased over what delivery reports, so it
      admits either mechanism the finding names (reading the "already exists"
      answer, or confirming with `channelExists`); which one is the `dev-writer`'s
      Decision. Confirmed against source: `createReliableChannel` answers
      `err("channel already exists: " & channelId)` exactly when the manager holds
      the id (`logos-delivery` `4a85db1b`, `channel_lifecycle.nim`).
      **Fixed** (code, `dev-writer`) in `5fb435f9`: `channel_answer` reads a
      decline whose reason contains `"channel already exists"` as `AlreadyHeld`,
      and the worker opens the channel on it exactly as on `Created`. Matching the
      words, not `channelExists`: design.md Decision 14 has why, and what a
      rewording would cost. The wording is also at `bfdb5afd`, the
      `logos-delivery` rev delivery v0.2.1's `flake.lock` pins (the rev above is a
      later checkout). Red before the change: this probe, ported as
      `a_creation_delivery_did_not_complete_in_time_opens_on_the_next_request`,
      and the new scenarios' tests
      `a_channel_delivery_reports_already_existing_is_open` and
      `a_module_restarted_while_delivery_kept_running_has_its_channels_open`, plus
      `only_delivery_s_already_exists_answer_opens_a_declined_channel`. The two
      tests whose scenarios now exclude the answer already declined with "no
      reliable channel manager", so they needed no change.

- [x] **`dev-writer`** — `delivery.rs:134` (`declined`, via `wire::callee_error`) —
      a success envelope that carries an empty `error` string is read as a decline.
      **Scenario:** `declined(&Ok(json!({"success":true,"value":"r-1","error":""})))`
      returns `Some("")`, so the channel is logged `channel NOT open …: ;` and never
      opened, and a send is logged as declined. `callee_error` returns any string,
      `""` included, and it runs before `success` is checked. `StdLogosResult.error`
      is a `std::string` that defaults to `""`, and logos-cpp-sdk's
      `generator_lib.cpp:1385-1387` (`lpPushExpr`) serialises it verbatim as
      `{"success":…,"value":…,"error":<error>}`. Only the cdylib path
      (`lidl_gen_cdylib.cpp:699`) turns an empty error into `null`. I could not tell
      from source which of the two paths delivery v0.2.1's replies take. The one
      live reply on record (`the_observed_decline`) is a failure, and no success
      envelope has been observed (task 7.3 is unticked).
      **Measured:** probe `reviewer_probe_success_with_empty_error_is_a_decline`
      fails with `left: Some("") right: None`.
      **Severity:** medium, and conditional. If delivery emits that shape, no channel
      ever opens and no op is ever sent, with every gate green. An explicit
      `success: true` should outrank an empty `error`, or an empty `error` should
      not count as a reason.
      **Fixed** in `52726819`: an empty `error` counts as no reason (the second
      option, so it also covers an envelope with no `success` field);
      `success: false` beside it still declines. The probe, ported as
      `an_empty_error_string_is_not_a_reason_to_decline`, was red first
      (`left: Some("")`). design.md Decision 6. `wire::callee_error` itself is
      unchanged, because `channelExists` (an existing wire method outside this
      piece) also reads it; that method has the same exposure, reported to the
      runner rather than changed here.

- [x] **`tester`** — `delivery.rs:945` (`Delivering::start`) — the running wiring's
      queue bound is unpinned. This is the tester's own limit 2, now measured.
      **Scenario:** changing `InboundQueue::with_bound(INBOUND_BOUND)` to
      `with_bound(usize::MAX)` in `start` removes the bound `op-transport` requires
      ("bounded by a fixed count"). A flood of arrivals is then held in memory
      without limit (150 KiB each) instead of being discarded at 256.
      **Measured:** all 57 of 57 `delivery::` tests pass under that mutation.
      (`with_bound(0)` is caught, by 4 tests.) The bound tests build their own queue,
      and `the_inbound_bound_is_pinned` pins the constant, not what `start` uses.
      **Outcome (`tester`): fixed.**
      `delivery::tests::the_queue_the_running_wiring_builds_is_bounded_at_the_pinned_count`
      floods the queue `start` builds — through `start_counted`, the listener and
      `hand_over`, with the processor held up by an unanswered open — with
      `INBOUND_BOUND + 5` junk messages on an open channel. It asserts the bound's
      arithmetic (at most 256 waiting and one held, so at least 5 discarded), that
      every discard line reads `queue full at 256;`, that the last line's running
      count equals the discards seen, and that every non-discarded message is decided
      once the boundary is released. Mutation: `with_bound(usize::MAX)` in `start`.
      Predicted red, observed red ("0 of 262 discarded"); it was the only test
      that moved of the 78 `delivery::` tests. A bound of 0 or of any other number
      would read "full at N" and fail the same test. Restored.

- [x] **`tester`** — `delivery.rs:378-390` (`Stores::memberships`) — paging past the
      first page of memberships is untested, and a wrong step hangs startup on the
      dispatch thread.
      **Scenario:** a peer in more than `MEMBERSHIP_PAGE` (100) Stoas starts. Under
      `page *= 1`, `start` loops forever inside `on_context_ready`, and the module
      never answers another call. Under `page -= 1`, it underflows `usize` (a
      panic in debug builds).
      **Measured:** `cargo mutants` MISSED both `replace += with *=` and
      `replace += with -=` at `delivery.rs:388:18`. No test holds more than a few
      memberships (`git grep MEMBERSHIP_PAGE` finds no use in `delivery/tests.rs`).
      **Outcome (`tester`): fixed.**
      `a_membership_record_of_more_than_a_page_is_read_to_the_end` joins
      `2 * MEMBERSHIP_PAGE + 5` Stoas, reads `Stores::memberships` on a thread the
      test waits on for 5 s (so a step that never ends fails the test, not the
      suite), and compares the sorted result with the sorted addresses joined;
      `a_peer_in_more_stoas_than_a_page_requests_every_channel_at_startup` asserts
      the same through `start` (one creation per membership, none twice). Three
      mutations of `page += 1` at `delivery.rs:388`, each predicted red and
      observed red: `-= 1` (panic "attempt to subtract with overflow", test sees a
      disconnected thread), `*= 1` (timeout, "reading the memberships never ended"),
      `+= 2` (`105` listed, `205` expected). Restored each time. The endless
      variant grows a vector while the thread spins, which is why the limit is
      5 s and not longer.

- [x] **`dev-writer`** — `delivery.rs:471-476` (`Channels::await_settled`) — once
      the book mutex is poisoned, the wait ends at the first wake-up of any kind
      rather than when this channel settles.
      **Scenario:** a panic raised while `Channels::receive` or `handoff` holds the
      book lock (for example, inside `transport::receive` on a hostile payload)
      poisons the mutex. `lock()` recovers the guard, but `wait_timeout_while`
      returns `Err` after its first internal wait, and the `let _ =` discards it.
      From then on, a message on a channel whose open is pending is judged as soon as
      any other channel's open settles, or on a spurious wake-up. It is refused as
      `unknown-channel` and lost, which is the race Decision 11 exists to close.
      This contradicts `lock`'s doc ("taking the guard back is the answer that keeps
      delivery running").
      **Severity:** low. It needs an earlier contained panic under the lock. The fix
      is to loop on the condition and recover the guard from the `Err`, rather than
      discarding the result.
      **Fixed** in `88636ff4`, as described: its own loop over `wait_timeout`,
      taking the guard back from a poisoned wake-up, ending only on this channel's
      condition or the limit. `a_poisoned_channel_book_still_waits_for_this_channels_open`
      poisons the book, parks a message on a pending open, settles a different
      open, and asserts nothing was judged; red first ("judged before its own
      open settled"). design.md Decision 11.

## Clean, in prose

- **Node lifecycle.** `createNode` is requested once per process
  (`started` flag), before any channel operation, with `entryLayer: "channels"` named. `start` is only
  requested after an accepted creation, and there is no `stop` anywhere on the
  seam or in the adapter. This matches delivery v0.2.1: the second `createNode`
  answers "Context already initialized", and `start` is dispatch-only.
- **`callee_error` first / `RET_STALE_WARN`.** v0.2.1's `replyTrampoline` and the
  start/stop callbacks drop `RET_STALE_WARN`, so each call answers once.
  `declined` reads the error envelope before the value. (Finding 2 is the one
  shape it gets wrong.)
- **Own sends.** Only `on_channel_message_received` is subscribed to. v0.2.1 emits
  own sends as `channel_message_sent`, which nothing here reads.
- **Event mapping (tester limit 1).** v0.2.1 emits
  `channelMessageReceived(channelId: tstr, senderId: tstr, payload: bstr (base64-decoded), timestamp: int64 ns)`. That
  matches the lidl, and the generated decoder's types are enforced by
  `nix build ./dialectica#lgx`. The name-to-name pairing is pinned by text. Whether
  the C++ provider's `bstr` reaches Rust in the tagged `{"_bytes":…}` form the
  strict decoder needs cannot be settled without task 7.3's two-peer run. No
  defect was found in the mapping itself.
- **Store-then-send.** `append` is a single autocommit `INSERT`, and the sink runs
  after it returns, so the worker's read-back by id sees the committed row.
  Re-publishes go through the sink too.
- **Ordering and send-waits-for-open (tester limit 5).** A single FIFO worker makes
  it structural. `sends_made_while_an_open_is_unanswered_wait_for_it_and_keep_their_order`
  would fail for a send made before the open was answered, because the op would
  be logged `not sent` and `sent == [root, child]` would fail. Nothing is hidden
  behind the limit.
- **The pending-open wait.** The condition is exactly "pending and not open". The
  `Opening` drop guard settles on every exit path, panics included. The worker is
  the only opener and handles one action at a time, so at most one open is ever
  pending. The per-id count therefore never exceeds 1, which is why the
  `+= → *=` mutant at `delivery.rs:442` survives. That mutant is equivalent,
  not a gap.
- **Op-log `BEGIN IMMEDIATE`.** Re-reading `user_version` under the write lock
  closes the double-`CREATE`. `Created::Already` is then checked against
  `LAYOUT_VERSION` and `check_layout` like any other open. Concurrent appends
  are single statements and wait on the busy timeout. No deadlock: the dispatch
  thread never takes the book lock, and the worker takes it only after its own
  log read.
- **Restart.** Channels are requested from the membership record alone at start.
  Sender ids are read back after `INSERT OR IGNORE`, so they are stable. Only
  finding 1 affects what a restart achieves.
- **Panics.** No `unwrap`, `expect` or indexing on a non-test path in `delivery.rs` or
  `sender.rs`. Worker, processor and listener items all run under
  `catch_unwind`.

## Re-review round 1 `7a2a3335..369561d1`

Dimension: **correctness only**. Read `git diff 7a2a3335 369561d1` for
`delivery.rs`, `transport.rs`, `wire.rs` and `sender.rs`, with the adapter's
`channel_messages` / `on_context_ready` in `dialectica/rust-lib/src/lib.rs` and
delivery v0.2.1's `delivery_module_plugin.cpp` and `api_call_handler.h`. The code
reviewed is at `f37c6cc4`, identical to `369561d1` outside `tasks.md`. All five
earlier outcomes hold. Suites green: `cargo test … -p dialectica -p dialectica-core`
(1286 + 30 + 3), the `delivery::` tests serially (`--test-threads=1`), and
`nix build ./dialectica#lgx`.

- [x] **`tester`** — `delivery.rs:542` (`Channels::opening`, the per-channel
      count) — the count is untested, and it now matters: two opens of one channel
      can be pending at once.
      **Why this supersedes round 0.** My clean-prose entry above ("at most one
      open is ever pending … the `+= → *=` mutant … is equivalent, not a gap")
      was true while the worker made the guard as it called delivery. Since
      `560e39f0`/`641cae31` the guard is made when the request is made, and it
      waits in the worker's queue. So a startup open plus a join, or two joins,
      are pending together. `ChannelBook`'s own doc says so: "a repeated join can
      put a second open in the queue before the first is answered".
      **Scenario:** a peer joins Stoa S. Delivery answers the create
      `"channel_create callback timeout"` after its 30 s, and its runtime then
      creates the channel anyway. During those 30 s the peer joins S again, so a
      second open is queued. Under `*entry.or_insert(0) *= 1`, the first
      decline's `settle` finds a count of 0 and removes S from `pending` while
      the second open is still unanswered. Messages delivery now hands over on S
      are refused `unknown-channel` on hand-over and lost for good, until the
      second open's "already exists" opens the channel. With the shipped `+= 1`
      they pass hand-over and wait for the second open, as `op-transport` requires
      ("It stays so until delivery's answer settles it").
      **Measured:** `cargo mutants` MISSED `replace += with *= in
      Channels::opening`. All 84 of 84 `delivery::` tests pass with it applied by
      hand. A reviewer probe in my tree
      (`reviewer_probe_a_second_pending_open_keeps_the_channel_opening_after_the_first_declines`)
      makes two `opening`s for one channel, drops the first, and asserts that the
      channel `is_known` and that a message waits and is stored once the second is
      `held`. The probe is green on the shipped code and red under the mutation
      ("the second open is still unanswered, so the channel is still being
      opened").
      **Severity:** low. The code is correct today. The gap is that nothing
      stops a regression to a flag, which is the shape round 0 called harmless.
      **Outcome (`tester`): fixed**, two tests, at the two layers the count can be
      seen from. `a_channel_asked_for_twice_is_being_opened_until_both_requests_settle`
      asks the book, for each order the two requests can settle in: after one
      settles the channel is still known, after both it is not (so it also fails a
      count that never comes down). `a_message_waits_for_the_last_of_two_requests_for_its_channel`
      is your probe as a message sees it, with its `sleep` replaced by a flag raised
      just before the second request is answered, read when `decide` returns.
      Mutations, each restored: `pending.requests += 1` to `*= 1` — predicted red in
      both, observed red in both, and a third the prediction missed, the dev-writer's
      `a_request_made_while_a_message_waits_does_not_extend_that_messages_wait`,
      which under this mutant passed without ever waiting (the storm's requests
      removed the pending entry, so the message was refused at once) until its
      refusal was required to come no sooner than the limit; and
      `saturating_sub(1)` to `saturating_sub(0)` — predicted red at the "both
      settled" assertion of the first test only, observed exactly that. This
      supersedes the round-0 entry's "equivalent, not a gap" for the `*=` mutant.

- [x] **`tester`** — `delivery.rs:96` (`CALL_TIMEOUT`) and `delivery.rs:531`
      (`SETTLE_LIMIT`) — the ordering both docs rest on is pinned nowhere:
      delivery's own 30 s < `CALL_TIMEOUT` < `SETTLE_LIMIT`.
      **Scenario:** `CALL_TIMEOUT` set back to the IPC default of 20 s. A
      `channelCreate` that delivery completes at 25 s is then declined here, and
      the Stoa is shut until the next request. That is `CALL_TIMEOUT`'s own doc
      example. Or `SETTLE_LIMIT` set below `CALL_TIMEOUT`. A message racing its
      own join's creation, answered after the limit and before the call timeout,
      is then refused `unknown-channel` and lost. That is the race Decision 11
      exists to close, and `SETTLE_LIMIT`'s doc says it is "always settled within
      it".
      **Measured:** with `CALL_TIMEOUT = 20 s` and `SETTLE_LIMIT = 10 s` together,
      all 1319 of 1319 tests pass (1286 + 30 + 3). `cargo mutants` cannot see a
      `const`. The docs name both relations, and one assertion over the two
      constants and a hardcoded 30 s would hold them.
      **Severity:** low. Nothing is wrong at the shipped values.
      **Outcome (`tester`): fixed, on top of the dev-writer's compile-time asserts
      (`DELIVERY_CALLBACK_TIMEOUT < CALL_TIMEOUT < SETTLE_LIMIT`), which stay.**
      Those compare the constants with each other, so they agree with whatever
      `DELIVERY_CALLBACK_TIMEOUT` says delivery's timeout is. The gap they leave:
      `DELIVERY_CALLBACK_TIMEOUT` lowered to fit a shortened `CALL_TIMEOUT`.
      `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
      is the assertion your fix shape names, with the 30 s written in the test
      (delivery v0.2.1's `CALLBACK_TIMEOUT{30}`) rather than read from the code.
      Mutation: `DELIVERY_CALLBACK_TIMEOUT` to 10 s and `CALL_TIMEOUT` to 20 s
      together. Predicted: the build compiles (10 < 20 < 40 satisfies both
      asserts) and only this test is red. Observed exactly that: "CALL_TIMEOUT
      (20s) does not outlast delivery's own 30s". Restored. The two single edits
      your finding describes (`CALL_TIMEOUT` back to 20 s alone, `SETTLE_LIMIT`
      under `CALL_TIMEOUT`) fail the build through the dev-writer's asserts, as
      the dev-writer recorded, and were not re-run.

### Clean in this round

- **"Already exists" opens the channel** (`5fb435f9`). `channel_answer` maps a
  decline containing `channel already exists` to `AlreadyHeld`, and `Worker::open`
  calls `held()` on it exactly as it does on `Created`. This holds after a
  timed-out create (the next join, `a_creation_delivery_did_not_complete_in_time_…`)
  and after a restart under a running delivery. On a restart, the startup open is
  queued behind a `createNode` that v0.2.1 declines at once with "Context already
  initialized". The worker still asks for the channels after that decline, and
  each is answered "already exists". v0.2.1's `"channel_create callback timeout"`
  wording is confirmed at `api_call_handler.h:159`.
- **The `Opening` guard.** Every path settles it, and none settles it twice. The
  paths:
  - it is made in `request` only under `Wiring::Running`;
  - a refused `send` hands the `Action` back inside `SendError`, which is dropped;
  - a worker that exits drops its `Receiver`, which drops what is still queued;
  - a panic in `perform` unwinds through the guard owned by `open`;
  - the no-sender return drops it;
  - a failed worker spawn drops `opens` with `return true`.

  Settling happens at the end of `open`, before the next action, so a `Send`
  queued after the open sees the channel open. No guard is dropped while its
  thread holds the book lock, so there is no self-deadlock.
- **Startup order.** `startup_opens` runs before `subscribe()`, and `Channels`
  exists only from `start`, so a join made before startup cannot mark an open that
  nothing will settle.
- **`judge` / `admit`.** `receive` is unchanged in behaviour: the channel is
  checked, then `judge`'s size, decode, verify, Stoa and window checks, then the
  one append. `Judged`'s private field makes skipping `judge` unrepresentable.
  The book lock is held only for map operations and for `handoff`'s encode. It is
  never held across a decode, a verify, an op-log open or an append.
- **Hand-over.** Unknown channel first, then `refuse_oversized`, the same
  predicate `judge` uses. A check-then-offer race only moves a message to the
  processor's own unknown-channel refusal, which the spec permits.
- **The settle-limit wait and its clock.** It uses a monotonic `Instant` taken per
  message, and `checked_sub` ends it at the limit, so wake-ups caused by other
  opens do not extend it. A poisoned wake-up recovers the guard. The loop's
  condition is exactly "pending and not open", and the processor's clock is read
  after the wait.
- **`Wiring`.** `NoWorker` is set before any step can fail, so a second `start`
  is refused in every state but `NotStarted`.
- **Per-event containment.** Each `next()` and its hand-over is its own
  `catch_unwind`. The adapter's iterator is `subscription.map(decode)`, so a
  panicking decode has already consumed its event and the loop moves on.
- `Stderr::record` → `()` is MISSED by `cargo mutants`. It is the adapter's log
  sink, and no in-crate test can read stderr, so I opened no box for it.
- **`cargo mutants` coverage was partial.** The run on `delivery.rs` (73 mutants,
  `delivery::` tests, 120 s timeout) was stopped after 32: 18 caught, 10
  unviable, 2 timeouts (the two `Stores::memberships` endless-loop mutants, which
  are expected) and 2 missed (the two above). Several mutants were each hitting
  the timeout, so the run went well past the time the role allows. The 41
  unreached mutants, from `Channels::settle` onwards, are not reported on here.

## Re-review round 2 `369561d1..2cb71aaf`

Dimension: **correctness only**. Read `git diff 369561d1 2cb71aaf` for
`delivery.rs`, `transport.rs` and `arrival.rs`, each of the two refactor commits
on its own (`7c9d6cd6`, `ff3eba85`), and the spec deltas for `op-transport` and
`stoa-membership`. Reviewed at `0a8f8639`, which is `2cb71aaf` plus tracking.
Suites green: `cargo test … -p dialectica -p dialectica-core` (1297 + 30 + 3),
and `nix build ./dialectica#lgx`.

Both round-1 outcomes hold, re-measured:
- `pending.requests += 1` → `*= 1` turns
  `a_channel_asked_for_twice_is_being_opened_until_both_requests_settle`,
  `a_message_waits_for_the_last_of_two_requests_for_its_channel` and
  `a_request_made_while_a_message_waits_does_not_extend_that_messages_wait` red.
- `DELIVERY_CALLBACK_TIMEOUT` = 10 s with `CALL_TIMEOUT` = 20 s compiles, and
  only `the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
  goes red. `SETTLE_LIMIT` = 10 s alone fails the build at the
  `CALL_TIMEOUT < SETTLE_LIMIT` assert.

- [x] **`tester`** — `delivery.rs:554` (`ChannelBook::is_opening`, the loop
      condition of `Channels::await_settled`). Nothing pins that a message is
      judged promptly once its open is declined. Under the mutation, a declined
      open holds every Stoa up for the whole settle limit.
      **Scenario:** apply `!open.is_open(id) && pending.contains_key(id)` →
      `!open.is_open(id) || pending.contains_key(id)`. A join's `channelCreate`
      is declined (for example `"Context not initialized"`, or v0.2.1's "no
      reliable channel manager"), and a message is waiting on that open. The
      decline removes the pending entry. The channel is not open, so the
      condition stays true, and the processor sits until the open's deadline,
      40 s in the running wiring. Every message on every other channel waits
      behind it. The `op-transport` requirement is that the message is judged
      "once that open is settled". The spec gives the fixed time as an upper
      bound on the wait, not as the time the wait lasts after a decline. The
      same mutant keeps a message waiting after its channel opens while a
      second request for it is still pending (open and pending, so still
      "opening").
      **Measured:** `cargo mutants` on the round's functions (16 mutants: 10
      caught, 5 unviable, 1 missed) MISSED this one. Applied by hand, all 1330
      tests pass. The core suite's wall time goes from 22.2 s to 40.03 s,
      because `a_message_arriving_while_its_open_is_declined_is_refused_after_the_answer`
      uses the processor's default `SETTLE_LIMIT`, waits out all 40 s, and
      still passes. It asserts that the refusal came *after* the answer, not
      that it came *soon after* it.
      **Severity:** low. The shipped `&&` is correct. The gap is that the
      "declined → judged at once" half of the condition is unpinned, and a
      regression there turns every declined open into a whole-module stall.
      Shape of a fix: in the decline test, set a long limit and require that
      `decide` returns well inside it, or time the refusal against the answer.
      **Outcome (`tester`): fixed**, by the first fix shape.
      `a_message_arriving_while_its_open_is_declined_is_refused_after_the_answer`
      now sets the processor's limit to 120 s and runs `decide` on a thread of its
      own, and after the open is dropped requires `decide` to have finished inside
      `eventually`'s ten seconds; the "after the answer, not before it" flag half
      is kept and is read off the same thread's return value. No stopwatch: the
      only clock is `eventually`'s ten seconds against a limit twelve times as
      long, so the right code fails only on a machine that cannot refuse one
      message in ten seconds. The name is kept because `tasks.md` 5.3 cites its
      suffix.
      **Mutation, as applied:** `&&` → `||` in `ChannelBook::is_opening`. Predicted:
      the decline test red, "timed out waiting: the message to be refused once the
      open is declined". **Observed: that test red with exactly that message, and
      also** `a_message_on_an_open_channel_does_not_wait_on_a_repeated_open`
      (timed out waiting: "the op to be stored without waiting"), the same mutant's
      other half (open and pending, so "still opening"), which the box says no
      test holds. **That test is red on this tree, where the box measured all 1330
      passing**; I could not reproduce that claim with the same mutation, so the
      box's "second half unpinned" is not borne out, and the decline half was the
      only one missing. Whole `delivery::tests` wall time under the mutation is
      10.2 s (the two ten-second timeouts), not the 40 s the old test spent passing.
      Restored with `git checkout`.

### Clean in this round

- **The two refactors change no behaviour.** `ff3eba85` swaps the bare count
  for `Pending { requests }` and moves the loop condition into `is_opening`
  unchanged. `7c9d6cd6` routes `receive` and the processor through
  `receive_via` in the same order as before: lookup under the message's own
  channel id, then `judge`, then the write. The clock is still read after the
  wait. The op log is still opened only for an op that passed. The outcome
  logging moved verbatim into `record_decision`. `judge` going private
  compiles, so nothing else called it.
- **The deadline's lifecycle.**
  - It is set once per open, by the first message that finds the channel
    pending and not open (`get_or_insert_with`). It is read under the same
    lock as the loop's first check, so a message that starts the time always
    waits on it.
  - It is shared: later messages get the same instant back, and a past instant
    returns at once. The open stays pending, so hand-over still admits the
    channel and delivery's answer still settles it.
  - Each request clears it (`Channels::opening`), including one given up at
    once. The spec allows this: that is still a request.
  - A waiting message keeps the local deadline it read, so it is never
    extended.
  - A decline or a hold that brings the count to 0 removes the whole record,
    so a later request starts clean.

  Mutations, each restored:
  - dropping the reset in `opening` turns two tests red;
  - re-reading the deadline on each wake-up turns one red;
  - a per-message clock turns two red;
  - removing the pending entry at expiry turns two red.

  `Instant + limit` cannot overflow at the shipped 40 s, and only tests set
  another value.
- **Every path that settles an open** still goes through the `Opening` drop:
  - the worker's answer (`held` on created or already exists);
  - a decline;
  - a panic in `perform`;
  - no sender identifier;
  - a refused send to the worker;
  - a worker that exits;
  - a failed worker spawn.

  `opening` releases the book lock before the guard exists, so no drop
  self-deadlocks. `settle` notifies on every call, so a waiter re-checks
  `is_opening` against the new state.
- In prose, not a box: `receive_via`'s doc says a caller "cannot pair one
  channel's message with another channel's Stoa". That holds only for an
  honest `stoa_of` closure, because one that ignores its argument can return
  any Stoa. Both callers are honest, and
  `the_boundary_looks_a_channel_up_under_the_messages_own_identifier` pins
  the key. This is a doc precision point, not a defect.

## Re-review round 3 `2cb71aaf..7462ded8`

Dimension: **correctness only**. Read `git diff 2cb71aaf 7462ded8` for
`delivery.rs` (the `is_opening` refactor `cda4827d`, the doc and assert comments),
`delivery/tests.rs` (the reworked decline test and the new
`each_unanswered_opens_wait_is_its_own_and_not_one_shared_across_opens`), the
`op-transport` spec delta, and the arithmetic in design.md Decision 11. Reviewed at
`ea37a706`. `cargo test … -p dialectica -p dialectica-core` green (1298 + 30 + 3).

Round-2 box, re-measured. The tester's disagreement is right about the tree it
names and my round-2 number was right about mine, and the two differ only by this
round's refactor. `&&` to `||` in `ChannelBook::is_opening`:
- on the current tree, `a_message_arriving_while_its_open_is_declined_is_refused_after_the_answer`
  and `a_message_on_an_open_channel_does_not_wait_on_a_repeated_open` are both red
  (93 of 95 `delivery::` tests pass; the suite takes 10.2 s, the two ten-second
  timeouts);
- with `wait_ends`'s guard put back to `self.open.is_open(..)` (the round-2 shape)
  and the same `||`, only the decline test is red and the repeated-open test passes.
  So at round 2 that half was genuinely unreachable from the loop condition alone,
  and the refactor, by making the guard and the loop one predicate, made it
  reachable. The decline half I asked for is pinned either way.

- [x] **re-review round 3 `2cb71aaf..7462ded8`: no findings** — read the `is_opening` refactor, the reworked decline test, the new per-open-wait test, the `op-transport` delta and Decision 11's arithmetic; clean

### Clean in this round

- **`cda4827d` changes no behaviour.** Old guard: return `None` if open, else
  `pending.get_mut(..).map(..)`, which is `None` when not pending. New guard: return
  `None` when `!(!open && pending)`, that is open or not pending, else `get_mut`,
  which is then always `Some`. Same result for all four (open, pending) states; the
  `get_mut().map` can no longer produce `None`, and the function keeps its
  `Option` return for the `None` of the guard. The clock, the lock and the loop are
  untouched.
- **The shared-deadline defect is caught.** The new
  `each_unanswered_opens_wait_is_its_own_…` is red under a mutation that makes
  `ChannelBook::wait_ends` return the earliest deadline set on any pending open
  ("refused 1.372µs after the first's: it was not given a wait of its own of 1s"),
  and it is the only one of 95 `delivery::` tests that moves. A wait of two limits
  instead of one would also fail its `took < 3 * limit` bound (four limits). Its
  half-limit lower bound, where the scenario says the full fixed time, is stated in
  the test's own comment and leaves nothing a correct build could fail on: `eventually`
  polls every 10 ms.
- **The reworked decline test** is sound. The open is made before `decide`'s thread
  starts, so the channel is pending when it reads; the flag is set before the open
  is dropped, so a refusal made without waiting returns with it down; the limit of
  120 s against `eventually`'s 10 s makes a wait that outlives the decline a
  timeout and not a two-minute test.
- **The spec delta matches the code.** `CALL_TIMEOUT` (35 s) is the timeout
  `lib.rs` passes to `channel_create_with_timeout`, delivery's own is 30 s and
  `SETTLE_LIMIT` is 40 s, so the new requirement's order holds and its
  "open behind other requests" exclusion is what the code does. "At most that
  fixed time", "no more than that fixed time for each wait a request starts" and
  "never extends past the moment the last of those opens settles" each follow from
  `await_settled`: the deadline is per `Pending` and set once, a message past it
  returns at once, and every wait ends when `is_opening` goes false.
- **Decision 11's startup arithmetic is right.** Waits run one after another,
  wait m ends at min(start_m + 40, (m+1) × 35), so the ends are 40m while 40m <
  (m+1) × 35, equal at m = 7 (both 280 s), and (m+1) × 35 from there. 20 Stoas:
  21 × 35 = 735 s; ⌈735 / 40⌉ = 19.

## Re-review round 4 `7462ded8..58460b02`

Dimension: **correctness only**. Read `git diff 7462ded8 58460b02` for
`delivery.rs` (`OpenTime`, `WaitId`/`Wait`, `Pending::asked`,
`ChannelBook::{begin_wait, wait_ends, end_wait}`, `Channels::asked`, the reworked
`await_settled`, `Opening::asked` and the worker's call before `channel_create`,
`Processor::new`) and `delivery/tests.rs`, against the `op-transport` delta in
`d240ebdc`. Reviewed at `adba1a22`. `cargo test … -p dialectica -p dialectica-core`
green (1305 + 30 + 3).

- [ ] **`dev-writer`** — `delivery.rs:609` `Wait::extend_from` — an ask landing
      after a message's end has passed, but before the processor has re-taken the
      book, revives that message's wait for a full limit
      **What is wrong:** `extend_from` moves `ends` to `asked + limit` whenever the
      extension is unused, without checking that `ends` is still ahead of `asked`.
      `await_settled` wakes from `wait_timeout` at its end and must re-take the book
      before it reads `wait_ends` again; if the worker's `Channels::asked` takes the
      book first, which is an ordinary interleaving on a loaded machine or behind any
      other holder of the book (listener's `is_known`, a `settle`, a `handoff`), the
      message's `Wait` is still in `pending.waits`. It is extended, and the processor
      goes back to sleep for another limit.
      **Why it is a defect:** the spec delta says "Once the open's time has ended with
      the open unanswered, the wait on it has expired: the message waiting then …
      MUST be judged without waiting on that open". It also derives "A message whose
      wait an ask extends holds them up for less than twice that fixed time: less than
      it before the ask, **since its wait would otherwise have expired**". Both
      assume the ask reaches only a wait that has not ended. Here a wait that has
      ended is made to wait again. That holds every other Stoa up for up to (end
      overshoot + one limit), which is past the "less than twice" the spec claims.
      design.md:589 and :705 and the `SETTLE_LIMIT` doc make the same claim. The
      fix below makes all of them true, so no prose needs to change.
      **Scenario (probe, reproducible):** settle limit 300 ms; open requested, not
      asked; a valid op begins waiting at t0. At t0+150 ms the test thread takes
      `channels.book` and holds it to t0+450 ms. The message's end at t0+300 ms passes
      while the processor is blocked re-taking the book. Still holding the book, the
      test calls `Pending::asked(Instant::now())`, which is the same body
      `Channels::asked` runs once it holds the lock, then releases. At t0+600 ms the
      message is **still undecided**. It is then stored when the open is reported
      held. Per the MUST above it should have been judged, and refused, at once after
      its end. The probe failed on the tree as it stands ("was still waiting half a
      limit after it; stored: true").
      **Measured fix:** a guard `if asked >= self.ends { return; }` at the top of
      `extend_from` makes the probe pass, and all 103 `delivery::` tests stay green.
      No test in the suite pins this edge, so the fix needs a regression test that
      holds the book across the end as the probe does. The fix also leaves the
      extension unused on that path, which is harmless: the message is judged on
      that wake-up.
      **Severity:** low. The window is scheduler-sized unless something else holds the
      book at the moment of the end, only this peer's ask opens it, and a sender cannot
      aim at it. It is still a reachable violation of a MUST this range added, and
      the fix is one line.
- [ ] **`tester`** — `delivery.rs:667` `ChannelBook::end_wait` — nothing pins
      that a decided message's `Wait` leaves the book
      **Measured:** `cargo mutants --file dialectica-core/src/delivery.rs` scoped to
      this range's functions found 18 mutants: 11 caught, 6 unviable, and **1 missed**.
      The missed one is `replace ChannelBook::end_wait with ()`, under which every
      `delivery::` test passes.
      **Why it matters:** without `end_wait`, every message that waits on an open
      leaves its `Wait` in `pending.waits` until the whole `Pending` is removed.
      A `Pending` can live a long time: at startup against a slow delivery, an open
      queued behind K others stays pending for up to (K+1) × `CALL_TIMEOUT`. For that
      whole time, each message a sender puts on that channel, each judged at once
      after the time has ended, adds one entry that is never freed. So a sender
      chooses how much the book grows, and every later ask walks all of those entries
      under the book lock. The code is right today. The gap is that a refactor
      dropping the call would ship green.
      **Wanted:** a test that decides a message on a pending open, both after it
      waited its time out and after the open settled held with another request still
      pending, and then asserts the open's `waits` is empty. `a_message_waits_on`
      already reads that map.

### Clean in this round

- **Every transition of `OpenTime` matches the delta.** A request (`opening`) sets
  `NotStarted`. An ask sets `StartedAt(ask)` whatever the state, including after an
  `Ends` that has passed, which is "whether or not its time had already ended". The
  first message to begin waiting fixes `Ends`: `now + limit` from `NotStarted`,
  `ask + limit` from `StartedAt`, the kept instant from `Ends`. So a passed `Ends`
  judges every later message at once until a request or an ask. Nothing else writes
  `time`, so "nothing else starts it" holds. `settle` leaves `time` alone while
  other requests stay pending, which is the "open behind other requests" loss the
  delta names.
- **An extension never shortens a wait.** Every begin-wait instant and every ask
  instant is read under the book lock, so they are ordered, and a wait's first end
  is at most its start plus one limit. An ask made while the wait is still ahead
  therefore gives an end no earlier than the one it replaces. That is why
  `Channels::asked` needs no `notify_all`: a waiter that wakes at its old end
  re-reads the later one and sleeps again. The once-only cap is `Option::take`, and
  a message that begins waiting after an ask keeps its extension for the next ask,
  which is what the delta's "the end the first ask made while it waits moved it to"
  allows.
- **The "wait gone from the book" path is right, and reachable only as the doc
  says.** `wait_ends` returns `None` while `is_opening` holds only if the `Pending`
  holding this wait was removed, when its last request settled, and a new request
  re-inserted it with an empty `waits`. A held settle opens the channel and ends the
  loop on `is_opening` first. So `None` means the open this message waited on has
  settled, not held, and the message is judged, as "judged only once that open is
  settled" requires. `end_wait` on the new entry removes nothing, because `WaitId`s
  are unique.
- **No `Wait` leaks.** `begin_wait` inserts only on the path that reaches the loop,
  every exit of the loop falls through to `end_wait`, and nothing between them can
  panic at the running limit. `Instant + 40 s` does not overflow.
- **Lock scope.** `Channels::asked` holds the book for a map lookup and a loop over
  that open's waits. The worker holds no other lock when it calls it, and calls it
  after the sender lookup and immediately before `channel_create`. That is every
  `channel_create` call site: `git grep -F channel_create` finds one outside the
  seam's trait and test doubles.
- **The rest of the new code is pinned.** The other 11 viable mutants in
  `OpenTime::end`, `Wait::extend_from`, `Pending::asked`, `begin_wait`,
  `wait_ends`, `Channels::asked`, `await_settled` and `Opening::asked` are all
  caught, including `extend_from`/`Pending::asked` → `()` and each `+` → `-`.
- **`a6fa2dde` changes no behaviour.** `Processor::new` sets the same six fields
  with `settle_limit: SETTLE_LIMIT`. `Delivering::start` passes the same four
  handles and `clock`, and both test fixtures did the same, so the tree before
  and after builds identical processors.
- **Nothing from earlier rounds is undone.** The per-open time kept on each
  `Pending` (round 2's shared-deadline test still green), the request not moving a
  waiting message's end (`a_request_made_while_a_message_waits_…` green), the
  poisoned-book loop (`into_inner` kept) and the `is_opening` single predicate all
  survive the rework.
