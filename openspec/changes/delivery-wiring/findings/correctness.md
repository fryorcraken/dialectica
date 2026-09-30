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

- [ ] **`tester`** — `delivery.rs:945` (`Delivering::start`) — the running wiring's
      queue bound is unpinned. This is the tester's own limit 2, now measured.
      **Scenario:** changing `InboundQueue::with_bound(INBOUND_BOUND)` to
      `with_bound(usize::MAX)` in `start` removes the bound `op-transport` requires
      ("bounded by a fixed count"). A flood of arrivals is then held in memory
      without limit (150 KiB each) instead of being discarded at 256.
      **Measured:** all 57 of 57 `delivery::` tests pass under that mutation.
      (`with_bound(0)` is caught, by 4 tests.) The bound tests build their own queue,
      and `the_inbound_bound_is_pinned` pins the constant, not what `start` uses.

- [ ] **`tester`** — `delivery.rs:378-390` (`Stores::memberships`) — paging past the
      first page of memberships is untested, and a wrong step hangs startup on the
      dispatch thread.
      **Scenario:** a peer in more than `MEMBERSHIP_PAGE` (100) Stoas starts. Under
      `page *= 1`, `start` loops forever inside `on_context_ready`, and the module
      never answers another call. Under `page -= 1`, it underflows `usize` (a
      panic in debug builds).
      **Measured:** `cargo mutants` MISSED both `replace += with *=` and
      `replace += with -=` at `delivery.rs:388:18`. No test holds more than a few
      memberships (`git grep MEMBERSHIP_PAGE` finds no use in `delivery/tests.rs`).

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
