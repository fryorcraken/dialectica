## Context

See `proposal.md` for why. The pure half of `op-transport` exists
(`ChannelIdentity::of`, `transport::receive`, `transport::publish`); the adapter
half did not. What shapes the approach:

- `cargo test` does not compile `dialectica/rust-lib/src/lib.rs`. Anything left
  there is untested by construction, and `nix build ./dialectica#lgx` is the only
  local gate that compiles it at all.
- The module is `"concurrency": "single"`: every dispatch and `on_context_ready`
  run on one thread, the module's Qt event loop (the generated scaffold says so).
- `dialectica/flake.nix` pins delivery at v0.2.1. Read at that tag
  (`src/delivery_module_plugin.cpp`), every channel call waits up to its own
  `CALLBACK_TIMEOUT{30}` seconds for the runtime and then answers; events are
  emitted from the runtime's callback thread whenever SDS hands one over,
  independently of any call in flight.

## Goals / Non-Goals

**Goals:** the adapter half `op-transport` and `stoa-membership` now require, with
every decision in `dialectica-core` behind a seam `cargo test` drives.

**Non-Goals:** delivery outcomes (`channelMessageSent` / `channelMessageError`),
closing channels, retrying without a restart or a repeated create/join, a status
surface, and a new wire method. `proposal.md` names each as follow-up.

## Decisions

### 1. The reliable channel, and nothing else

Delivery here is `channelCreate` / `channelSend` / `channelMessageReceived`, one
channel per Stoa. Issue #176 put the reason plainly: a plain `send` on a content
topic "would make item 7 look green while giving up the retransmission and repair
the design depends on". So the seam (Decision 2) has no `send` and no `subscribe`,
and the adapter subscribes to exactly one event.

Prior art was #30 (branch `core/transport`, closed unmerged "per the decision to
build from specs rather than from these PRs"), read and not merged. The pure half
landed in #55, whose `tasks.md` §8 deferred the adapter to "the method that needs
it" — no later change picked it up, which is why the publish sink was still a
logging no-op.

### 2. Every decision in `core::delivery`, behind a four-method seam

`core::delivery::Delivery` has four methods — `create_node`, `start_node`,
`channel_create`, `channel_send` — and the adapter implements each as one call to
the generated client. Everything else is in `dialectica-core`: the node
configuration, what counts as a decline, the order of calls, the sender
identifier, the inbound queue, how an event is processed, and every log line.
The listener's loop is in core too (`listen`), fed an iterator the adapter builds
from the SDK subscription and the generated decoder.

**Rejected: #30's shape**, statics and logic in the adapter. It put the queue, the
channel map and the ingest loop in the one file no test compiles.

**The seam has no `stop`.** `op-transport` forbids this application stopping the
node; with no method for it, the worker cannot be written to. The adapter could
still call it directly, so `the_adapter_never_stops_a_node_and_creates_one_at_one_site`
reads the adapter's source (comments stripped) and fails on any `.stop(`, and on
anything but exactly one `create_node` call and one `start` call.

### 3. One worker thread makes every delivery call; handlers only enqueue

A publish, create or join hands `Delivering` an op id or a Stoa over an `mpsc`
channel and returns. One worker thread takes actions off in order and makes each
delivery call synchronously.

- **Why the reply must not wait.** `content-authoring` forbids a reply waiting on
  delivery. A synchronous `channelSend` or `channelCreate` against an unresponsive
  delivery would hold the reply for the whole IPC timeout, and the view would
  likely time out and show a stored op as a failure.
- **Why one FIFO consumer.** It makes both ordering requirements true by
  construction: sends go out in the order publishes were answered (the sink runs
  before the reply, and handlers run one at a time), and a send enqueued after an
  open is made after that open is answered.
- **Why a worker may call.** `logos_protocol.h`: a client is owned by one thread,
  and "calls from other threads marshal onto it and block until it answers". The
  owner is the event loop, so the event loop must never wait on the worker — and
  nothing does: the handlers' `mpsc` send never blocks, and the processor and
  worker share only a mutex held for map lookups.

**Rejected: the generated `*_async` twins**, chained by their callbacks. The
callbacks run on the event loop after the method returns, so the reply would not
wait — but "send after the open is answered" becomes a state machine advanced
from callbacks, and the code that advances it lives where no test reaches.

What breaks without it: making the calls inline turns
`an_unresponsive_delivery_does_not_delay_the_publish_reply` and
`an_unresponsive_delivery_does_not_delay_a_join` red (each measures a 3 s delivery
against a 1 s bound).

### 4. Each call waits 35 s, past delivery's own 30 s

`CALL_TIMEOUT` is passed to every `*_with_timeout` call. The IPC default is 20 s;
delivery v0.2.1 waits up to 30 s on its runtime before answering. With the default,
a `channelCreate` completed at 25 s would be recorded here as unanswered — not
open — while delivery holds it open and its messages arrive and are refused as
arriving on an unknown channel. Past 30 s, delivery's answer decides. Nothing
waits on this but the worker. **No test can see this constant**: it bounds a real
IPC call, which only a live delivery has. It is argued here and in its doc, not
pinned.

### 5. The node: `channels` layer named, `logos.test`, `Edge`

`{"entryLayer":"channels","preset":"logos.test","mode":"Edge"}`.

- `entryLayer` is named because `op-transport` requires it: a node without the
  channel layer refuses every channel call.
- **`logos.test`** is the Logos Test Network, cluster 2. It is the preset whose
  maximum message size is 150 KiB — the value `transport::MAX_MESSAGE_BYTES` pins
  and the boundary refuses above — and the one #30 used. `logos.dev` is cluster 3
  with the transport's default size. Peers on different presets never meet, so
  this is interop in practice although the spec leaves it to design.
- **`Edge`** is a light node: it does not relay other peers' traffic, and it
  publishes and receives through the preset's service nodes. `Core` would make
  every dialectica peer a relay — contributing bandwidth to the mesh and not
  depending on the fleet for delivery, at a cost in bandwidth and listening ports
  on a user's machine. `Edge` matches #30 and is the lighter choice for a desktop
  module; the dependence on the fleet is the trade, raised with the owner under
  Open Questions.

`the_node_preset_and_mode_are_pinned` fails on a change to either, so a change is
a decision rather than an edit.

### 6. The node is asked for once per process, and started only when created

`Delivering::start` returns `false` and does nothing on a second call, so creation
and start are each requested at most once however often startup runs
(`node_creation_is_requested_once_however_often_startup_runs`). `StartNode` is the
first action in the worker's queue, so creation precedes every channel operation.
Doing *nothing* on the second call — no channel either — is `stoa-membership`'s
"Startup running again in the same module process MUST NOT request any channel":
the first run asked for every membership's, and every create or join since asks
for its own.

A declined creation is logged with delivery's reason and the channels are still
requested. **`start` is then not requested**, as `op-transport` now requires
("Start SHALL be requested only after delivery has accepted this application's
creation", and the declined-creation scenario's "node start is not requested").
The reason it was chosen before the spec stated it still holds: delivery v0.2.1
declines a second `createNode` with "Context already initialized", which is most
often another module in the context owning the node — and a node this application
did not create is not its to start, any more than to stop. `declined` treats a
transport failure and an unanswered call as declines too, which is the
requirement's "declines, fails or does not answer".

### 7. The sender identifier: 32 random bytes per Stoa, retained in `senders.sqlite`

Rendered `/dialectica/1/p/<hex>`. Each property the spec asks for, and why:

- **Unique per installation**, even two holding one identity. The spec-writer
  inferred this from `CLAUDE.md`'s own-send asymmetry and asked for it to be
  confirmed; it is confirmed from the SDS spec rather than inferred:
  `logos-lips/docs/anoncomms/raw/sds.md` says the sender "MUST include its own
  globally unique identifier" and a participant "SHOULD ignore the message if it
  has a `sender_id` matching its own". Two installations sharing a value could
  each discard the other's messages. Not measured against delivery.
- **Different per Stoa and not the key**, because SDS traffic carries it even for
  someone who only reads: one value per installation would link that reader
  across every Stoa it is in.
- **Stable across restarts**, because it is retained and read back.

**Rejected: #30's author-derived `sender_id(author_for_stoa)`.** Two installations
restored from one keystore compute the same value, and it is computable from a
key. **Rejected: one installation secret plus a keyed hash per Stoa.** It meets
every property, but a new Stoa then needs no write, so "a sender identifier that
cannot be retained opens no channel" could only fail on the very first Stoa.

It is its own SQLite file because `stoas.sqlite` stamps a layout version and its
`check_layout` names exactly its columns: a new table there needs a version bump
that makes every existing membership store unopenable. `SenderStore::sender_for`
reads back after `INSERT OR IGNORE`, so the value returned is always the retained
one; a store that cannot be written returns an error and the worker requests no
channel.

### 8. The send reads the op back from the log by id

The publish sink carries only the op id. The worker opens the log, reads the op,
and hands `transport::handoff`'s bytes to `channelSend` — so the payload is the
wire form as stored (for a re-publish, the bytes stored the first time), which is
what `op-transport` requires. `transport::handoff` shares one private function,
`addressed`, with `transport::publish`, so "is a channel open for this Stoa, and
which" has one answer. The sink's type did not change.

### 9. Create and join take a `joined` sink

`create_stoa` and `join_stoa` gained `joined: &mut dyn FnMut(&Address)`, called on
the success arm of `store.join` and nowhere else — the publish path's `deliver`
shape. So "after the membership is recorded" and "never on a refusal" hold by
position, and a repeated join (`Joined::AlreadyIn` is success) requests again. A
panicking sink is caught and logged, and the reply is unchanged
(`a_panicking_join_sink_does_not_change_the_reply`). This landed as its own
no-behaviour commit, with the op log path (Decision 13).

### 10. Inbound: a bounded queue of 256 that discards the arrival

The listener only offers each event to `InboundQueue` and never waits on the
boundary; the processor takes them off in arrival order.

**The count, 256**, from two bounds. Memory: a payload may be 150 KiB before the
boundary refuses it, so the worst case is ≈ 37.5 MiB in a module process sharing
its host; #30's 1024 is ≈ 150 MiB. Loss: a discard is final for this peer. #30's
justification ("a dropped op is recoverable through retransmission") does not
hold — by the time `channelMessageReceived` fires, SDS has treated the message as
delivered and will not repair it. The processor decides a typical op in about a
millisecond, so 256 waiting is a burst arriving faster than SQLite appends.

**The arrival is discarded, reversing #30.** A backlog burst arrives roughly
oldest first: dropping the oldest loses thread roots and keeps orphaned replies;
dropping the arrival loses leaves and keeps every thread it holds whole.

**The clock is read when the message is processed**, after any wait, and never
from the event's timestamp (`the_window_is_judged_by_this_peers_clock_not_the_events_timestamp`).

### 11. A message on a channel still opening waits for the answer

delivery v0.2.1 can emit `channelMessageReceived` for a channel after its runtime
created it and before the `channelCreate` answer reaches this peer. Judged in that
gap, it is refused as an unknown channel and lost (Decision 10). So the channel
map carries **pending** opens beside open ones, and the processor, meeting a
message on a channel that is pending and not open, waits for the answer (bounded
at 40 s, past `CALL_TIMEOUT`). An `Opening` guard settles the open on every path
out of the worker, a panic included, so nothing waits on an open nobody will
answer.

`op-transport` now states the behaviour: a message on a channel "whose opening
this peer has requested and delivery has not yet answered, MUST be judged only
once that open is settled", and one on any other channel identifier, an open one
included, without waiting. The wait's condition is exactly that — pending *and*
not open — so a message on an open channel whose open is being repeated, or on an
unrelated channel, is judged at once. Removing the wait turns
`a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` red.

A declined *repeat* open leaves an already-open channel open (`Channels::settle`
opens on success and never closes), which is `stoa-membership`'s "A repeated
request that delivery declines, fails or does not answer MUST leave a channel
that is already open open": delivery created it once, and nothing in this change
closes it.

### 12. The module's log is a seam, and its words are in core

`core::delivery::Journal` receives every line; the adapter's is stderr. The spec
contracts what the log says and must not say, and only a seam lets a test read
it. Every line is worded once, in `Note`. A refusal is logged by kind
(`unknown-channel`, `too-long`, `undecodable`, `fails-verification`,
`stoa-mismatch`, `ahead-of-time`, `storage`) and carries nothing the sender chose;
a discard reads "discarded … N discarded since the module started" and never
"refused".

### 13. Two threads now open `ops.sqlite`, and the op log handles it

The processor and worker open the log per action on their own threads, beside the
dispatch thread. Three things follow:

- `core::log::op_log_path_in` names the file once. The adapter spelled it at four
  sites; a fifth writer storing into a different file from the one reads open
  would receive everything and show nothing.
- **A fresh store's schema was created by whichever connection got there — twice.**
  Two connections opening a store that does not exist yet both read version 0 and
  both ran the `CREATE`. Found by `a_received_op_reaches_the_log_through_the_running_listener`
  and fixed in its own commit: `create_schema` takes `BEGIN IMMEDIATE` and re-reads
  the version under the write lock. `two_connections_opening_a_fresh_store_at_once_both_open_it`
  failed in round 0 before the fix.
- **Concurrent appends wait rather than fail**, because rusqlite's connections wait
  on a busy database. `two_connections_appending_at_once_both_store_everything`
  pins it; setting the busy timeout to zero turns both tests red with "database is
  locked".

`stoas.sqlite` and `senders.sqlite` are each opened by one thread only (dispatch,
and the worker), so neither has the race.

### Behaviour chosen during implementation, now in the spec

Each of these was chosen while the code was written, and the spec now states it.
The reasoning that chose each is here; the contract is the requirement cited.

- **`start` only after an accepted creation** — `op-transport`, "Start SHALL be
  requested only after delivery has accepted this application's creation"
  (Decision 6).
- **A second startup requests nothing, channels included** — `stoa-membership`,
  "Startup running again in the same module process MUST NOT request any
  channel" (Decision 6).
- **A message on a channel still opening is judged once the open settles** —
  `op-transport`'s inbound-boundary requirement and its four "while its channel
  opens" / "does not wait" scenarios (Decision 11).
- **A declined repeat open leaves an open channel open** — `stoa-membership`,
  "A declined repeat request leaves an open channel open" (Decision 11).
- **A failed subscription is logged and sending stays wired** — `op-transport`,
  "A peer that cannot subscribe still publishes". Refusing to wire delivery at
  all would take publishing away from a peer that can still publish.
- **A join or publish before delivery is wired is logged and dropped, not
  held** — `op-transport`, "A publish before delivery is wired is not sent when
  it is", and `stoa-membership`, "A join before delivery is wired has its
  channel requested once, by startup". Holding requests would need a second
  queue in front of the worker for a state a dispatch cannot normally reach (the
  scaffold fires `on_context_ready` before the first dispatch), and startup
  already asks for every membership's channel.
- **A message the op log cannot take is logged and not retried** —
  `op-transport`, "A message the op log cannot take is logged and not retried".
  Retrying would mean holding messages the bounded queue has already let go, and
  an op arriving again is admitted as a first arrival would be.

### How each `CLAUDE.md` module-contract trap is handled

| Trap | Handling |
|---|---|
| No handler may unwind | Handlers only enqueue. Both sinks are caught (`delivered_and_published`, `recorded_and_replied`). The worker, the processor and the listener run each item under `catch_unwind` and log a panic. Locks are taken poisoned-or-not. Threads are started with `thread::Builder`, which reports a refusal where `thread::spawn` panics. |
| `recv()` may block forever on an older SDK | The builder pin (`9f420c2`) stages an SDK whose `recv()` polls the provider's `Abandoned` status every 200 ms and fails once it is reported, so the listener's loop ends and logs "the inbound listener has ended". That needs a runtime with the status channel (logos-protocol 0.9); on an older one `recv()` parks forever — one leaked thread holding nothing another waits on. |
| `RET_STALE_WARN` (3) | Not reachable on this surface, confirmed at v0.2.1: `api_call_handler.h` and the start/stop callbacks return early on it, so each channel call answers once, terminally. This change subscribes to no outcome event, so there is nothing to count twice. |
| Own sends arrive as `channelMessageSent` | Not subscribed to. The own op is already in the log (stored before it is sent), so nothing is lost. `only_reliable_channel_receipts_reach_the_op_log` reads the adapter's subscriptions. |
| The node is a singleton per Logos Core instance | This application asks once per process (Decision 6). delivery refuses a second `createNode` ("Context already initialized"); that is logged as a decline and channels are still requested. |
| `createNode` exactly once; never `stop()` | Once per process by `Delivering`; no `stop` on the seam; the adapter-source test. |
| `messageReceived` timestamps are nanoseconds | Not subscribed to. `channelMessageReceived`'s timestamp (v0.2.1: `currentTimestampNs()` at receipt) is carried in `Arriving` and read by nothing; the window uses the host clock at processing time. |
| A decline arrives as `Ok` with an error envelope | `declined` reads `callee_error` first, then `success: false`, then treats a transport `Err` as a decline. |

## Risks / Trade-offs

- [`Edge` depends on the `logos.test` fleet's service nodes] → a desktop module
  stays light; revisit with the owner (Open Questions).
- [A slow delivery delays later sends by up to 35 s each] → replies are unaffected;
  the outbound queue is unbounded, but each entry is caused by a local call from
  the view, so it grows at the user's rate.
- [256 is a guess at burst depth] → discards are counted and each is logged with
  the running total, so a bound that is too small is visible in the log.
- [Messages on another application's channels on the shared node are logged as
  `unknown-channel` refusals] → one line each, carrying no channel id.
- [The adapter is compiled only by `nix build ./dialectica#lgx`] → it is four
  calls, one subscription and one decoder mapping; the generated names it relies
  on (`create_node_with_timeout`, `on_channel_message_received`,
  `decode_channel_message_received` and its fields) are checked by that build and
  by nothing else. Two peers exchanging an op is verified by hand with two
  `lgs basecamp launch` profiles; the automated two-peer test is follow-up.

## Migration Plan

`senders.sqlite` is created on first use; no existing file changes. The op log's
schema-race fix changes how a fresh store is created, not what is written.

## Open Questions

- **Preset and mode.** `logos.test` and `Edge` are recorded above with their
  meaning. Either can change without changing the spec — a constant and its pin —
  but both decide which network dialectica peers meet on and whether they relay,
  so the owner should confirm them.
