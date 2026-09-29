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

- [ ] **`dev-writer`** — `delivery.rs:845` — `Processor::decide` opens `ops.sqlite`
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

- [ ] **`spec-writer`** — `op-transport` "Inbound payloads waiting for the
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

- [ ] **`dev-writer`** — `delivery.rs:670` (doc) / `delivery.rs:729` — the memory
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

- [ ] **`dev-writer`** — `delivery.rs:1030` — the listener runs its **whole loop**
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
