# Security review — delivery-wiring (#176)

Dimension: **security** only. Reviewed at `7a2a3335` against `origin/main`
(three-dot): `dialectica-core/src/delivery.rs`, `sender.rs`, the `transport.rs`
handoff split, the `log/sqlite.rs` schema-race fix, the `wire.rs` join sink, and
the adapter's `DeliveryModule` / `channel_messages` / `on_context_ready` in
`dialectica/rust-lib/src/lib.rs`, plus the generated event decoder and
`logos_rust_sdk::bytes` it relies on.

Measurements are from probe tests appended to `delivery/tests.rs` in the
reviewer's worktree only (not committed); each scenario below says what the
probe did so it can be re-run.

- [x] **`dev-writer`** — `delivery.rs:845` — `Processor::decide` opens `ops.sqlite`
      **before** the boundary looks up the channel, so the cheapest refusal pays
      the most expensive step.
      **Scenario:** a message on a channel this peer has not opened — another
      application's channel on the shared node (design.md's Risks already says
      these arrive), or any id a sender picks — goes `await_settled` → `op_log()`
      (`Connection::open`, `PRAGMA user_version`, layout check) → `receive`, which
      only then refuses it as `unknown-channel`. The boundary's own doc promises
      "a 4 MiB payload costs a length comparison rather than a parse"; here it
      costs a database open first.
      **Measured:** 2000 unknown-channel messages through `decide`: 270 ms,
      **135 µs each**; the same 2000 through `Channels::receive` with the log
      already open: 0.66 ms, **0.33 µs each** — about 400× (debug build). So
      foreign or hostile traffic drains the bounded queue ~400× slower than it
      needs to, which is what turns a burst into final discards of real ops
      (next entry). Side effect, same cause: with the op log unopenable, an
      unknown-channel message is logged `refused (storage: …)` rather than
      `refused (unknown-channel)` (probe: `break_file` on `ops.sqlite`, one
      message on `/another-app/channel` → one `storage` line, no
      `unknown-channel` line).
      **Severity:** medium — amplifies the DoS in the next entry; not a crash.
      **Direction:** decide channel and size before opening storage (open the
      log lazily, or split `receive` so the pre-storage checks run first).
      **Fixed** in `cd5d7c82` + `3b72e546`, the split: `decide` looks the channel
      up, runs every pre-storage check (`transport::judge`: size, decode, verify,
      Stoa, window), and opens the op log only to `admit` an op that passed. Your
      probe 1, ported with an assertion as
      `an_unknown_channel_is_refused_as_one_before_the_op_log_is_opened`, was red
      first (one `storage` line). And since `686553da` a message on a channel
      neither open nor opening does not reach the processor at all: it is refused
      on hand-over (the spec change your next entry led to). design.md Decisions
      10 and 15.

- [x] **`spec-writer`** — `op-transport` "Inbound payloads waiting for the
      boundary are bounded" / `delivery.rs:729` — every message delivery emits
      takes a slot in the one shared queue before anything is judged, so traffic
      on **any** channel makes discards of **every** Stoa's ops final.
      **Scenario:** channel ids are computable by anyone and Stoas are
      permissionless, so any peer can send on a Stoa this peer is in; and the
      node is shared, so other applications' channel traffic arrives too. 256
      waiting messages from one source fill the queue and each further arrival
      — including a valid op in a different Stoa — is discarded, and design
      Decision 10 records that a discard is unrepairable ("SDS has treated the
      message as delivered"). One noisy or hostile channel is a censorship lever
      over all the others, which is the property this forum exists to resist.
      **Measured:** probe offered 256 messages on `/another-app/channel`, then a
      valid op on the Agora channel: `offer` returned `Discarded(1)`.
      **Severity:** medium. The current spec text leaves no room for the obvious
      mitigations — "every message … MUST be put through the boundary" and a
      single count-bound over all of them — so this needs a spec decision, not a
      code tweak: e.g. refuse an id that is neither open nor pending before it
      takes a slot (logged as `unknown-channel`, without the id), and/or give
      each open channel its own share of the bound. design.md's Risks entry on
      foreign-channel messages records the log line they cost, not the slots.
      **Outcome (`spec-writer`): fixed in part; the rest deferred to the owner.**
      Fixed: `op-transport`, "Inbound payloads waiting for the boundary are
      bounded", now requires that a message on a channel identifier this peer has
      neither open nor being opened "MUST NOT take a place among the waiting
      payloads": it is refused as an unknown channel when delivery hands it over,
      logged as that refusal is, and neither counts towards the bound nor is
      counted as a discard. New scenario "Traffic on a channel this peer is not
      opening takes no place in the queue" is this finding's probe with the
      opposite expectation. The requirement's "MUST NOT wait on the boundary" now
      reads "deciding any payload, the refusal below included", so the hand-over
      check may not take a lock the boundary holds while it decodes, verifies and
      appends. "The waiting payloads never exceed the bound" now says its payloads
      arrive on an open channel. Deferred: a peer flooding the channel of a Stoa
      this peer *is* in still fills the shared bound and forces discards in every
      other Stoa. Which remedy (keep it shared, a fixed per-channel share, or
      discard the newest of the channel holding the most) trades burst depth
      against isolation, and one amends the spec's position on Open Question 3, so it
      is `proposal.md` Open Question 6 for the owner, with the options.

- [x] **`dev-writer`** — `delivery.rs:670` (doc) / `delivery.rs:729` — the memory
      bound "256 × 150 KiB ≈ 37.5 MiB" is claimed but not enforced by this code.
      **Scenario:** `InboundQueue::offer` checks only the count; the 150 KiB limit
      is applied by `transport::receive` after the message has waited. What
      delivery hands over is bounded only by the node's configured maximum
      message size — and `Worker::start_node` deliberately carries on over a node
      **another module created** ("Context already initialized"), whose preset
      and limit this code does not choose. `sender_id` and `channel_id` are
      likewise held at whatever length arrives.
      **Measured:** probe offered a 4 MiB payload: `Waiting`, queue length 1.
      256 of those is 1 GiB held in a process sharing its host.
      **Severity:** low (depends on delivery's limit being larger than
      `MAX_MESSAGE_BYTES`). **Direction:** either drop an over-limit payload at
      `offer` without holding it (the spec's refusal order puts
      `unknown-channel` before `too-long`, so say which it is logged as), or
      restate the doc and design Decision 10 so the bound is visibly delivery's
      rather than this queue's.
      **Fixed** (the second direction) in `89b3b552` and the docs commit:
      `INBOUND_BOUND`'s doc and design Decision 10 now say the queue bounds a
      count, that the worst case is 256 × delivery's maximum message size, and
      that it is larger on a node another module created; design.md Risks carries
      it. **Deferred** (the first direction) to the `spec-writer`: refusing an
      oversized payload on hand-over is behaviour `op-transport` does not state
      (it names only unknown channels as refused there, and "decided in the order
      they arrived" would need saying for it), so it is not built. It now lives in
      design.md's Open Questions ("For the spec-writer: oversized payloads on
      hand-over") and in this round's hand-back.
      **Spec outcome (`spec-writer`): the first direction is now required; the
      `dev-writer` builds it.** `op-transport`, "Inbound payloads waiting for the
      boundary are bounded", now says a payload larger than the message limit, on
      a channel open or being opened, "MUST NOT take a place among the waiting
      payloads": it is refused as over-long on hand-over, logged as that refusal
      is, and neither counts towards the bound nor is a discard. A message on a
      channel neither open nor being opened is still refused as an unknown
      channel whatever its size, which answers "say which it is logged as". A
      hand-over refusal is not among the waiting payloads, so "decided in the
      order they arrived" does not order it. New scenarios: "An oversized payload
      on an open channel takes no place in the queue" and "A payload at the limit
      waits its turn". Not decided: the sender identifier is still held at
      whatever length arrives (bounded only by delivery's own message maximum);
      that is in the hand-back for the owner, not in the spec.

- [x] **`dev-writer`** — `delivery.rs:1030` — the listener runs its **whole loop**
      under one `catch_unwind`, so a single panic while reading one event ends
      reception for the rest of the process; `tasks.md` 6.2 and design.md's trap
      table both say the listener runs **each item** under `catch_unwind`, which
      is false for the listener.
      **Scenario:** the iterator's `next()` is where the SDK receives the event
      and the generated `decode_channel_message_received` reads its fields — the
      one piece of the inbound path that touches event data before the queue. A
      panic there logs "panicked and was contained", then "the inbound listener
      has ended", and closes the queue: every later op from every peer is lost
      until restart, with the module otherwise healthy. I found **no** reachable
      panic in the decoder at this pin (length-checked `arr[i]`, `as_*()?`,
      `bytes::decode_lenient` is index-safe), so this is defence in depth, not a
      live exploit.
      **Severity:** low. **Direction:** catch per `next()` and keep listening, or
      correct the two claims so the listener's all-or-nothing containment is
      recorded as a decision.
      **Fixed** in `1cb7a857` (the first direction): each `next()` and its
      hand-over run under their own `catch_unwind`, the panic is logged, and the
      loop carries on; `tasks.md` 6.2 and the trap table are now true.
      `a_panic_reading_one_event_does_not_end_reception` feeds the running
      listener an event whose read panics, then a valid op; red first (the
      listener ended and the op was never stored). design.md Decision 16 records
      the cost: an iterator that panicked on every call without consuming an event
      would spin.

## Clean areas

- **What reaches the log.** Every `Note` variant was read: no line carries an
  arriving payload, sender identifier, or channel id; refusals are by kind, and
  the only free text is this peer's own storage error or delivery's decline
  reason (a local module, not a peer). `a_refusal_is_logged_by_kind…` and
  `every_refusal_is_logged_under_its_own_name…` pin the sender-chosen strings
  out, the unknown-channel id included. `Arriving`'s `Debug` is never formatted
  into a line.
- **The one-hour ahead-of-time refusal** applies at this boundary: the processor
  reads the host clock (`now_ms`, panic-free, saturating) **after** the pending
  wait and never the event timestamp; `transport::receive` judges the window
  last, before the only append.
- **The pending-open wait cannot be driven by a sender.** Only ids this peer
  itself asked to open are ever pending; an arbitrary channel id falls through
  `await_settled` at once (`!open && pending` is false), and the wait is bounded
  (40 s) and always released by the `Opening` drop guard, a panic included.
- **Panics.** The worker and processor contain each item; locks are taken
  poisoned-or-not; threads are started with `Builder`; both handler sinks are
  under `catch_unwind` inside `guarded`, and the sinks only enqueue. No
  `unwrap`, `expect`, indexing or unchecked arithmetic on the inbound path
  outside tests.
- **Sender identifier.** 32 bytes from the OS source, per Stoa, not derivable
  from any key, retained before use, and a failure to retain opens no channel.
  Within one Stoa it links nothing a poster's signing key (`publishing_key`,
  one identity key in this release) does not already link, and across Stoas it
  links nothing, which is what protects a reader who never posts. It is never
  logged.
- **SQLite concurrency.** The fresh-store race is closed by `BEGIN IMMEDIATE` and
  a re-read; concurrent appends wait on rusqlite's default busy timeout;
  `senders.sqlite` and `stoas.sqlite` each have a single opener thread.
- **Dependencies.** None added.

Observations, no action required: every inbound message produces one log line
(stored, refused or discarded) with no rate limit, so a flood grows the host's
log at roughly the flood's own rate — no amplification, but worth remembering if
logs go to a small disk. `Channels::receive` holds the channel-book lock across
the whole decode, verify and SQLite append, so an append waiting on a busy
database (for as long as rusqlite's busy timeout allows) also holds up the
worker's opens and sends.

`cargo mutants` on `sender.rs` (library tests; the `seeded_reference`
integration tests fail in mutants' copied tree because they read outside
`rust-lib`): 16 mutants, 8 caught, 7 unviable, 1 missed — `SenderError`'s
`Display` replaced by an empty string survives. That only blanks the reason in
the "no sender identifier could be retained" log line; it is not a security
property, so it is left to the tester's lane rather than boxed here.

**Follow-up (`tester`), the missed `SenderError` `Display` mutant:** closed.
`sender::tests::every_error_says_what_went_wrong_in_its_own_words` renders each
variant against a hardcoded fragment and requires the five to differ, and
`a_sender_identifier_that_cannot_be_retained_opens_no_channel` now asserts the
log line carries the store's own words. Mutation: `fmt` returning `Ok(())` at once.
Predicted red, observed red in both ("no sender identifier could be retained ()"
in the second). Restored.

## Re-review round 1 `7a2a3335..369561d1`

Dimension: **security** only. Read at `f37c6cc4` (the range's code is
`369561d1`'s): `delivery.rs` in full, the range's diffs to `transport.rs`,
`wire.rs` and `sender.rs`, design.md Decisions 10, 11 and 14, the Risks and Open
Questions, and `op-transport`'s wait requirement. I also read the SDK's
`EventSubscription` (`logos-rust-sdk-src/src/plugin.rs:461-512`) to judge the
listener's per-event containment.

**The four round-0 outcomes hold.** `Processor::decide` looks the channel up, then
calls `judge`, then opens the op log for `admit` only (`delivery.rs:1081-1092`).
`refused_on_hand_over` asks `is_known` before `refuse_oversized`, so a message is
dropped before it waits (`delivery.rs:1025-1030`). `INBOUND_BOUND`'s doc now
states the payload-bytes bound and does not overclaim it. The listener runs each
`next()` and hand-over under its own `catch_unwind` (`delivery.rs:951-971`).

- [x] **`spec-writer`** — `op-transport`, "That wait is bounded by a fixed time"
      (`specs/op-transport/spec.md:190`), as built at `delivery.rs:587-599` /
      `:1068` — the wait is given up "for that message", so each later message
      on the same still-pending channel waits the full limit again. A sender
      therefore chooses how long every Stoa's inbound processing stalls, up to
      the open's whole pending time rather than one `SETTLE_LIMIT`.
      **Scenario:** since `641cae31`, an open is pending from the request. So a
      join queued behind k worker actions against an unresponsive delivery is
      pending for up to (k+1) × 35 s. At startup the last of K Stoas waits behind
      node creation and K−1 creations, so it is pending for up to K × 35 s + 35 s.
      Channel ids are public and Stoas are permissionless, so any peer can send
      on that channel, and the payload need not be an op. With n such messages
      the one processor waits n × 40 s. For the 20th of 20 Stoas that is up to
      21 × 35 s = 735 s, and n = ⌈735 / 40⌉ = 19 messages keep every other Stoa's
      ops unjudged for about 12 minutes. Once 256 messages are waiting, every
      arrival on an open channel is discarded, and design Decision 10 records
      that such a discard is final. Honest traffic on a busy Stoa does the same
      without trying.
      **Measured:** a probe appended to `delivery/tests.rs` (reviewer's tree
      only) put 10 messages on a channel whose open is never answered, with
      `settle_limit` = 200 ms, then a valid op on an open channel. The op was
      stored after **2.01 s** (10 × 200 ms). The wait did not end at 200 ms.
      **Severity:** medium-low. It needs a delivery that answers calls slowly
      while it still hands messages over, which is the state Decision 11 already
      names. The loss is bounded by that pending window, but inside it a sender
      holds every Stoa hostage for as long as it keeps sending.
      **Direction:** let one open's wait end one time: a deadline kept per open
      (set when the first message begins waiting on it), after which later
      messages on that open are judged without waiting until a new request marks
      it pending again. This caps the stall at one `SETTLE_LIMIT` per open,
      whatever n is. The spec's "for that message" reads as ruling that out, so
      the spec has to decide it first.
      **Outcome (`spec-writer`): fixed; the `dev-writer` builds it.** The
      direction is adopted. `op-transport`, "Every payload the reliable channel
      delivers passes the inbound boundary": the paragraph is now "That wait is
      bounded by a fixed time for each open, not for each message" — the time
      starts when a message first begins waiting on the open; once it has passed
      unanswered the wait has expired, and the waiting message and every later
      one on that channel are judged without waiting, so one open holds other
      channels up for that fixed time however many messages arrive on it. A new
      paragraph, "An expired wait changes nothing else about the open", keeps the
      channel being opened (delivery's later answer still settles and opens it,
      and hand-over does not refuse its messages), and says only a later create,
      join or startup request lets a message wait on it again. The settled-by
      list no longer reuses "given up as unanswered" for the expiry; it now reads
      "given up by this peer after asking delivery and receiving no answer".
      What it trades: a message on the stuck channel taken after the expiry is
      judged at once, and refused if the open is still unanswered, where today
      it would have waited its own 40 s and might have been stored. That loss is
      confined to the channel whose open is stuck, where the per-message wait
      spread it to every Stoa through the shared queue. New scenarios: "Many
      messages on one unanswered open hold other channels up for one wait, not
      one each" (your probe, asserting under 2 × the limit for ≥ 3 messages), "An
      open whose wait has expired still opens its channel when delivery
      answers", and "A new request for a channel whose wait has expired lets a
      message wait again".

- [x] **`dev-writer`** — design.md Decision 11 (`design.md:400-406`, `:433`),
      Risks (`design.md:621-625`) and `SETTLE_LIMIT`'s doc (`delivery.rs:520-530`)
      — these say the 40 s limit "keeps the stall one call long" and that a
      message is refused "after holding every Stoa up for 40 s". Under an
      adversary both are false: the stall is n × 40 s, capped only by the open's
      pending time (entry above). The stated reason for rejecting a longer limit
      is also wrong. It says every message on a still-queued channel would hold
      the processor "one after another". But with a limit that outlasts the
      pending time, only the **first** message waits: the open has settled by
      the time the second is taken. So the worst-case stall is the pending time
      under either limit, and 40 s only shortens the honest few-message case
      while turning each later message into a refusal and a lost op.
      **Scenario:** a reader deciding whether the settle wait needs hardening
      trusts "one call long" and closes the question, and the probe above shows
      it does not hold. **Severity:** low (documentation of a security bound).
      Correct the text whichever way the spec entry above is settled.
      **Fixed** (`dev-writer`) in the commit `Bound the wait on a pending open
      once per open, not once per message`, with the code: the spec entry above
      was settled per open and the wait is now built that way, so the text
      describes a bound that holds. Decision 11 no longer says "one call long":
      it says what 40 s outlasts (one delivery call), what it bounds (one stall
      per unanswered open, whatever a sender sends — so K × 40 s for K opens
      stuck at once, bounded by this peer's memberships), what per open gives up,
      and your measurement of the per-message shape with the figures. The
      rejection of a longer limit is re-argued: your point that with a limit
      outlasting the pending time only the first message waits is recorded as
      what undid the old argument, and the new one is about one stuck open under
      per-open (a limit sized for the queue grows the startup total as K² where
      40 s grows as K). Risks and `SETTLE_LIMIT`'s doc are corrected the same
      way. `many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each`
      is your probe as a test (four messages, 400 ms): red before the change at
      1.61 s, green after.

## Round 1 clean areas

- **Pending opens cannot be created by a sender.** Only `joined` (create and join
  replies) and `startup_opens` (the membership record) make an `Opening`. An
  arbitrary channel id is refused on hand-over and never waits.
- **The listener's per-event containment cannot spin on this SDK.**
  `EventSubscription::next` is `recv().ok()` over `mpsc::recv_timeout`, which
  does not panic, so no call to `next()` panics without first consuming an event.
  Design Decision 16's stated cost does not arise at this pin.
- **What reaches the log.** New lines (`ChannelAlreadyHeld`, `NoWorker`,
  `WorkerGone`, the listener's `Panicked`) carry only this peer's own Stoa
  addresses, op ids, delivery's reasons and panic text. Hand-over refusals log
  the kind only. One thing to keep an eye on, not a finding: `panic_detail` is
  untruncated. A panic raised while reading an event with a message that
  formats event bytes would log them, once per event. I found no reachable panic
  in the generated decoder (round 0), so this is not live.
- **"Already exists" by substring.** The reason comes from delivery or the IPC
  layer. The only identifiers in a `channelCreate` call are this peer's own
  channel id (`/…/<stoa hex>`), content topic and sender identifier, so no peer
  can put that phrase into a reason.
- **Hand-over lookups.** `is_known` hashes the sender-chosen id with SipHash
  (`HashMap<String, _>`) under a lock that is held only for map operations, and
  the processor's condvar wait releases it. The listener never waits on the
  boundary.
- **`Judged` / `admit`.** `Judged`'s field is private and `judge` is its only
  constructor, so no caller can append an op that skipped a check.
- **Drop-guard safety.** `Opening::drop` → `settle` takes the book lock. No path
  drops an `Opening` while it holds that lock (`opening()` releases the lock
  before it builds the guard, and unwinding drops inner guards first), and
  `settle` has no panicking operation, so a drop during unwind cannot abort.
- **Dependencies.** None added in the range.

`cargo mutants` on `delivery.rs`, scoped with `--re` to the hand-over and wait
logic (`refused_on_hand_over`, `hand_over`, `is_known`, `await_settled`,
`channel_answer`, `listen`, `settle`, `Opening`, `opening`), `dialectica-core`
lib tests: 18 mutants, **14 caught, 4 unviable, 0 missed**. My stall probe was
in the tree during that run. It adds one test and changes no mutated function.

## Re-review round 2 `369561d1..2cb71aaf`

Dimension: **security** only. Read at `0a8f8639` (the range's code is
`2cb71aaf`'s): the range's diffs to `delivery.rs`, `transport.rs` and
`arrival.rs`, the new delivery and transport tests, design.md Decision 11 and the
Risks, and the `op-transport` / `stoa-membership` spec diffs. Probes were
appended to `delivery/tests.rs` in the reviewer's tree only and are not
committed. Each scenario below says what the probe did, so it can be re-run.

**Both round-1 outcomes hold.** The time is kept per open.
`Pending::wait_ends` is `get_or_insert_with`, so every message on one open gets
the same instant back. Only `Channels::opening` clears it, and only `joined`
(create and join replies) and `startup_opens` call that; no inbound path does.
`await_settled` reads the deadline once, before its loop, so a settle storm and a
request storm cannot extend a wait. Hand mutation: I made `Pending::wait_ends`
return `Instant::now() + limit` without storing it, which is the per-message
clock. Two tests went red:
`many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each`
(4.02 s against a 1 s limit) and
`an_opens_time_is_the_same_for_every_message_that_waits_on_it_until_a_request_clears_it`.
I restored it after. The Decision 11 text no longer says "one call long".

- [x] **`dev-writer`** — design.md Decision 11, "Rejected: a limit sized for a
      queue of opens" (`design.md:487-499`), and the Risks entry that leans on it
      (`design.md:755-758`). The rejection says that under a queue-sized limit
      "startup's K opens, each held in turn, grow as the square of K where 40 s
      grows as K". That is false. A wait ends when its open settles, and at
      startup every open settles by (K+1) × 35 s, counted from startup. The waits
      run one after another on one processor, but each ends at a fixed wall-clock
      time. So under a sized limit the stall is at most the pending window,
      (K+1) × 35 s, which is linear in K.
      Under 40 s per open it is no smaller once K ≥ 7. A sender puts one message
      on each stuck channel, in the order the opens settle. Wait m then ends at
      min(40m, (m+1) × 35), which reaches the window at m = 7, and every later
      wait runs to its own open's settle. For 20 Stoas the stall is 735 s under
      either limit. That is the round-1 headline figure, unchanged. It now takes
      20 messages, one per channel, where it took 19 on one channel.
      **Scenario:** a peer in 20 Stoas restarts against a delivery that hands
      messages over but answers no calls (Decision 11's precondition). A sender
      posts one junk payload on each of those Stoas' public channel ids. Every
      Stoa's inbound processing stalls about 12 minutes, the queue fills, and
      later arrivals are discarded for good. A queue-sized limit would stall
      exactly as long and would refuse none of the stuck channels' messages.
      So the trade the decision records (lose those messages to get a smaller
      worst-case stall) buys nothing at K ≥ 7.
      **Measured:** a probe scaled 35 s → 100 ms (`SETTLE_LIMIT` → 114 ms), with
      node creation followed by 8 Stoa opens settling at 200…900 ms. It put one
      message on each open's channel in settle order, then a valid op on an open
      channel. The op was stored after **914 ms** at the per-open 114 ms limit
      and **917 ms** at a 10 s limit. A second probe used three opens settling
      at 200/400/600 ms under a 10 s limit. The "held in turn" reading predicts
      1200 ms; the op was stored at **612 ms**.
      **Severity:** low. The bound is documented and is not violated (K × 40 s ≥
      the window). But the security argument for choosing 40 s over the
      loss-free alternative is wrong. Correct the rejection and the Risks line,
      then decide again whether 40 s is still the right value once the argument
      is true.
      **Fixed** (`dev-writer`) in the commit `Re-argue why the settle limit is
      40 s, and stop counting what grows`; `SETTLE_LIMIT` is **unchanged at
      40 s**, re-decided. The rejection now records the square-of-K argument as
      withdrawn with your reason, the stall under either limit as the last open's
      settle, and your two probes with their figures (914/917 ms; 612 ms against
      a predicted 1200). The Risks line says "up to K × 40 s …, never past the
      last of them settling", and (K+1) × 35 s once K ≥ 7 at startup.
      Why 40 s still, argued in Decision 11 "So 40 s stays": you are right that
      at startup, with every channel stuck, no fixed value from 40 s up buys a
      smaller stall once K ≥ 7, and I say so. What 40 s does buy is outside that
      case: where one open's pending time is longer than the stuck opens × 40 s —
      a peer in fewer than seven Stoas, or a join queued behind a backlog of sends
      against a hung delivery (the outbound queue is unbounded) — a queue-sized
      limit lets **one** message on that open hold every Stoa for its whole
      pending time, and 40 s holds it once, for 40 s. The price is the stuck
      channel's own messages, refused after 40 s even when delivery then answers
      held; I kept the loss on that channel rather than spread the stall to every
      Stoa through the shared queue. Two further reasons recorded: `op-transport`
      asks for "a fixed time for each open", which a queue-position limit is not
      (adopting one is a spec change), and a shorter fixed value down to
      `CALL_TIMEOUT` trims little (36 s: 720 s against 735 s at K = 20) while
      refusing more. The spec's new order (delivery's own < `CALL_TIMEOUT` <
      `SETTLE_LIMIT`) still holds, by the existing asserts and test. This is a
      judgement, and the decision says so; if the owner prefers the loss-free
      shape it goes to the `spec-writer` first.

## Round 2 clean areas

- **Nothing a peer controls can extend or restart a wait.** A peer's message
  *does* start an open's time. That is the spec's rule ("started by the first
  message to wait on it"), and all it lets a sender do is use the one wait
  early. The loss that follows stays on the stuck channel, as the round-1
  outcome's "What it trades" records. Once a wait has expired, a message on
  that channel is refused by `receive_via`'s lookup before any decode. So the
  queue slots it takes cost a hash lookup each, and the shared bound itself is
  still owner Open Question 6.
- **`receive_via`.** `judge` is private. `Judged(` is constructed only inside
  `judge`, and only `receive_via` calls `judge`, with the Stoa it looked up
  under the message's own `channel_id`. A caller cannot pair one channel's
  bytes with another channel's Stoa.
  `the_boundary_looks_a_channel_up_under_the_messages_own_identifier` pins
  this. The delivery processor's lookup copies the Stoa and releases the book
  before anything is decoded or verified.
- **"Already exists".** This range changes only the spec: `channel_answer` and
  `ALREADY_EXISTS` are the same as in round 1, and the substring rule matches
  the new `stoa-membership` wording. The round-1 finding still stands: no peer
  can put that phrase into delivery's reason.
- **Panics.** `Instant::now() + limit` would panic only for a limit near
  `Duration::MAX`. `settle_limit` is `SETTLE_LIMIT` in the running wiring and is
  set otherwise only in tests. `saturating_duration_since` cannot panic. The
  compile-time order asserts are `const` and cost nothing at run time.
- **What reaches the log.** No new `Note` in the range. `record_decision` is
  the old match, moved.
- **Dependencies.** None added.

`cargo mutants` on `delivery.rs`, scoped with `--re` to `Pending::`,
`ChannelBook::`, `Channels::opening`, `Channels::await_settled` and
`Processor::{decide,pass,record_decision}`, `dialectica-core` lib tests: 16
mutants, **11 caught, 5 unviable, 0 missed**. My three probes were in the tree
during that run. They add tests and change no mutated function.

## Re-review round 3 `2cb71aaf..7462ded8`

Dimension: **security** only. Read at `ea37a706` (the range's code is
`7462ded8`'s): the `delivery.rs` diff (`cda4827d`'s predicate and the limits'
comments), the three new or reworked tests, design.md Decision 11 and the Risks,
the `op-transport` delta and `proposal.md`. One probe was run in the reviewer's
tree only and then removed. The scenario below says what it did, so it can be
re-run.

**The round-2 outcome holds.** Decision 11 withdraws the square-of-K argument and
gives my reason. It records both probes with their figures. The Risks line now
reads "never past the last of them settling", with (K+1) × 35 s once K ≥ 7.
`SETTLE_LIMIT` is 40 s (`delivery.rs:606`). I checked the arithmetic behind the
re-decision. With a limit L, wait m ends at min(L·m, 35(m+1)). At L = 36 that
switches only at m = 35, so for K = 20 the stall is 20 × 36 = 720 s. At L = 40 it
switches at m = 7, giving 21 × 35 = 735 s. Both match the text. **Against a
sender, the stall half of the corrected argument holds.** Only this peer's
requests start or clear a wait. One message on a long-pending open holds every
Stoa for 40 s rather than for the open's whole pending time. Below 7 stuck opens,
40 s is strictly shorter than a queue-sized limit. The loss half does not hold,
as the entry below shows.

- [x] **`spec-writer`** — `specs/op-transport/spec.md:192` (and `proposal.md:72-74`,
      "never judged before that creation is answered or given up") — the new
      paragraph says the order delivery < `CALL_TIMEOUT` < fixed time means "a
      message waiting on the open whose creation is the one this peer is
      currently waiting on delivery to answer is then judged only after delivery
      has answered that creation or this peer has given up". That does not follow
      from the requirements above it. The open's time starts when a message
      *first* waits on it after the request, and an open is pending from the
      request, so it can start while the open is still queued. When the worker
      later reaches the open, the time left is 40 s minus however long it waited
      in the queue. That can be less than the call. The paragraph's exclusion
      covers only a wait that expires "before delivery is asked". It does not
      cover one that started in the queue and expires while delivery is being
      asked. That is exactly the race this wait exists for.
      **Scenario:** a peer starts up in several Stoas against a slow but working
      delivery. Node creation and the earlier Stoas' creations take 24 s between
      them, each well inside delivery's 30 s. As soon as the peer subscribes, a
      sender puts one junk payload on a later Stoa's channel id. Channel ids are
      public, and the payload need not be an op. That starts the open's 40 s at
      t = 0. At t = 24 s the worker asks delivery to create the channel. At
      t = 32 s delivery hands over an honest op on it, before its answer, which
      is the documented v0.2.1 race. At t = 40 s the wait expires and both
      messages are refused as `unknown-channel`. At t = 44 s delivery answers
      "created", 20 s after it was asked and inside its own 30 s. SDS has counted
      the honest op as delivered, so it is lost for good. With no sender
      involved, honest traffic arriving during the queue does the same. The
      sender adds control over timing: it can start every later Stoa's time at
      the earliest possible moment. Decision 11's "40 s … covers the ordinary
      cases (the open being the worker's current call)" rests on the same
      sentence. It needs no more than 5 s (40 − 35) of calls ahead of the open.
      **Measured:** a processor-level probe, scaled 40 s → 1000 ms. It opened
      the channel with `channels.opening`, queued a junk payload at t = 0 and an
      honest op at 800 ms, and settled the open `held` at 1100 ms. That models a
      call begun at 600 ms and answered 500 ms later, which is 20 s against
      delivery's own 750 ms. Result: the honest op was **not stored**, and the
      journal showed two `refused (unknown-channel)` lines.
      **Severity:** low. The loss stays on the Stoa whose open is late, and it
      needs more than 5 s of delivery calls queued ahead of that open. But the
      spec states a guarantee that its own rules break, and a sender can trigger
      the breach deliberately.
      **Direction (spec decision):** either narrow the sentence, so the guarantee
      covers only a wait that began after the worker asked delivery, and record
      the loss for an open that sat in the queue first; or restart an open's time
      when this peer asks delivery for it. That second option bounds the stall
      per call rather than per request, and reopens part of Decision 11's
      "Start an open's time at the request" alternative, so it has to be argued
      against the stall bound.
      **Outcome (`spec-writer`): the second direction, capped; the `dev-writer`
      builds it.** Narrowing would have kept the loss in exactly the race this
      wait exists for, and restarting at the ask closes it without reopening the
      unbounded stall, because only this peer's asks move the time and a message
      is extended at most once. `op-transport`, "Every payload the reliable
      channel delivers passes the inbound boundary": the fixed-time paragraph now
      says an open's time is started by the first message after a request **and
      again when this peer asks delivery to create the channel**, and nothing
      else. A new paragraph, "Asking delivery to create a channel starts the
      open's time again", says the ask restarts the time whether or not a
      message waits or the time had ended. A message waiting at the ask is judged
      no later than the fixed time after the ask, and its wait expires then. That
      extension is made once per message: a later ask during the same wait
      restarts the time for the messages after it and does not move that one's
      end. The timeout-order paragraph keeps its MUST unchanged (fixed time >
      this peer's creation wait > delivery's own) and now states what follows
      from it. A message waiting at the first ask during its wait, or beginning
      after an ask, is judged only after that creation is answered or given up.
      It then **names the loss** it does not cover: a message whose wait expired
      before delivery was asked (an open queued behind others for longer than the
      fixed time), the messages after it until the ask, and a message already
      extended once when a second ask comes. The consequence paragraph now
      bounds the stall per start of an open's time, not per request. It says a
      message extended by an ask holds the other channels up for less than twice
      the fixed time (under it before the ask, at most it after). It still never
      runs past the last settle, and only this peer's requests and asks set it.
      "An expired wait changes nothing else" and "A request … does not lengthen a
      wait already under way" now name the ask as the other thing that starts a
      wait and the one thing that moves a waiting message's end.
      New scenarios: "A message waiting when this peer asks delivery for its
      channel is judged after delivery answers" (the extension; an honest op as
      the first waiter), "An earlier message on a queued open does not cost an op
      that arrives while delivery is asked" (your probe with the opposite
      expectation), "Asking delivery for a channel whose wait has expired lets a
      message wait again", and "A second ask while a message waits does not
      extend its wait again" (the cap). Three existing scenarios gained a WHEN
      clause ruling asks out while their messages wait, since their timing claims
      assume none: "Many messages on one unanswered open …", "Each unanswered
      open's wait is its own" and "Requests made while a message waits do not
      lengthen its wait". `proposal.md` "Receiving" is rewritten to match.
      Why the stall bound survives: before the ask a waiting message is under one
      fixed time, or it would have expired. After the ask it is at most one fixed
      time, and in practice until this peer gives up the call (`CALL_TIMEOUT` <
      the fixed time) unless a second request for that channel is pending. So one
      junk message on a queued open now holds every Stoa for under 80 s where it
      held 40 s. The startup total at K ≥ 7 (the last open's settle, 735 s for 20
      Stoas) is unchanged, because no wait outlasts its own open. The cap is what
      keeps "Requests made while a message waits do not lengthen its wait" true.
      Without it, repeated joins against a hung delivery chain asks every
      ≤ 35 s, each extending the same message by 40 s, and postpone it for as
      long as they last.

## Round 3 clean areas

- **`cda4827d` is a pure refactor.** `wait_ends` now returns `None` on
  `!is_opening`, where it used to return `None` on `is_open`. The difference is
  the not-open, not-pending case. That case used to fall through to
  `pending.get_mut`, which returns `None`, so the result is the same. The two
  questions (whether a message waits, and whether it keeps waiting) now share
  one predicate. Nothing a peer controls reaches it other than the channel id
  used as a `HashMap` key.
- **The reworked decline test.** It now has a two-minute limit and runs `decide`
  on its own thread. It pins that a declined open ends the wait at once, rather
  than holding every Stoa for the whole limit. So it guards the stall bound in
  the `!open && pending` loop condition.
- **`each_unanswered_opens_wait_is_its_own…`.** It pins the per-open deadline
  against being shared across opens. A shared deadline would shorten a wait
  rather than lengthen one, so it would cost messages rather than stall time.
- **Panics, log lines, dependencies.** The range adds no arithmetic, indexing or
  `unwrap` outside tests, no new `Note`, and no dependency.

`cargo mutants` on `dialectica-core/src/delivery.rs`, scoped with
`--re "is_opening|wait_ends|await_settled"`, `dialectica-core` lib tests: 11
mutants, **8 caught, 3 unviable, 0 missed**. My probe was not in the tree during
that run.
