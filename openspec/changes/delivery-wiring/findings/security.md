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

- [ ] **`dev-writer`** — design.md Decision 11 (`design.md:400-406`, `:433`),
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
