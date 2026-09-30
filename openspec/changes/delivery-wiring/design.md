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
  independently of any call in flight. v0.2.1's `flake.lock` pins
  `logos-delivery` at `bfdb5afd`; claims below about delivery's runtime are read
  at that rev (`git -C ~/src/logos-messaging/logos-delivery show bfdb5afd:<path>`),
  not at the checkout's working tree.

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
reads the adapter's source (comments stripped) and fails on any `.stop`, and on
anything but exactly one `create_node` call and one `start` call. What breaks
without it: adding a `.stop()` call, or a second `create_node` or `start` call,
anywhere in `lib.rs` turns that test red; nothing else can see the adapter.

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
  nothing does: the handlers' `mpsc` send never blocks. The event loop (the
  dispatch thread, marking an open in `Delivering::joined` and at startup), the
  worker, the listener and the processor share the channel book's mutex, and the
  listener and the processor share the inbound queue's; neither is ever held while
  a payload is decoded, verified or appended (Decision 15), so none of them waits
  on another's decision. That the dispatch thread is among the book's users is
  why that rule protects replies and not only the worker's opens.
- **The one wait a reply can meet, and its length.** A publish appends to
  `ops.sqlite` on the dispatch thread, and the processor appends inbound ops to the
  same file (Decision 13). SQLite takes one writer at a time, so a publish reply
  can wait behind an inbound append — for at most the connection's busy timeout,
  after which the append fails as "database is locked" and the publish is
  refused. That timeout is rusqlite's own default, **not a value this change
  sets**: 5 s, measured here by holding a write lock on the op log from a second
  connection and timing an append (it failed "database is locked" at 5.01 s).
  An inbound append is one row, so the wait is ordinarily far shorter; 5 s is what
  bounds it.

**Rejected: the generated `*_async` twins**, chained by their callbacks. The
callbacks run on the event loop after the method returns, so the reply would not
wait — but "send after the open is answered" becomes a state machine advanced
from callbacks, and the code that advances it lives where no test reaches.

What breaks without it: making the calls inline turns
`an_unresponsive_delivery_does_not_delay_the_publish_reply` and
`an_unresponsive_delivery_does_not_delay_a_join` red. Each holds the delivery call
at a gate the test has not opened and asserts the reply came back while delivery
had still not answered, so neither reads a clock.

The worker's startup state is one value, `Wiring` — not started, started with no
worker (the OS refused the thread), or running — rather than a `started` flag
beside an optional outbox, whose fourth combination logged every request after a
refused worker as "has not started".

### 4. Each call waits 35 s, past delivery's own 30 s

`CALL_TIMEOUT` is passed to every `*_with_timeout` call. The IPC default is 20 s;
delivery v0.2.1 waits up to 30 s on its runtime before answering. With the default,
a `channelCreate` completed at 25 s would be recorded here as unanswered — not
open — while delivery holds it open and its messages arrive and are refused as
arriving on an unknown channel. Past 30 s, delivery's answer decides. Nothing
waits on this but the worker. **No test can see this constant**: it bounds a real
IPC call, which only a live delivery has. It is argued here and in its doc, not
pinned.

Delivery's own 30 s is not the end of a creation, though: its runtime completes
the channel after the callback has given up, and says so only when asked again
(Decision 14).

### 5. The node: `channels` layer named, `logos.test`, `Edge`

`{"entryLayer":"channels","preset":"logos.test","mode":"Edge"}`.

- `entryLayer` is named because `op-transport` requires it: a node without the
  channel layer refuses every channel call.
- **`logos.test`** is the Logos Test Network, cluster 2, with a 150 KiB maximum
  message size — the value `transport::MAX_MESSAGE_BYTES` pins and the boundary
  refuses above — and the preset #30 used. `logos.dev` is cluster 3 with the
  transport's default size, which is also 150 KiB. Read at `logos-delivery`
  `bfdb5afd`: `logos_delivery/waku/factory/networks_config.nim` (`LogosTestConf`,
  `LogosDevConf`) and `waku_core/message/default_values.nim`. Two clusters are two
  networks, so this is interop in practice although the spec leaves it to design.
  A later `logos-delivery` checkout (`4a85db1b`) names cluster 2 for both presets,
  so this is a claim about the rev v0.2.1 pins and will need re-reading when the
  pin moves.
- **`Edge`** is a light node: it does not relay other peers' traffic, and it
  publishes and receives through the preset's service nodes. `Core` would make
  every dialectica peer a relay — contributing bandwidth to the mesh and not
  depending on the fleet for delivery, at a cost in bandwidth and listening ports
  on a user's machine. `Edge` matches #30 and is the lighter choice for a desktop
  module; the dependence on the fleet is the trade, raised with the owner under
  Open Questions.
- **Not measured: that an `Edge` node on `logos.test` receives
  `channelMessageReceived` on a reliable channel at all.** That is the whole
  feature, and only task 7.3's two live peers can show it. Nothing in this
  change's tests can.

`the_node_preset_and_mode_are_pinned` fails on a change to either, so a change is
a decision rather than an edit.

### 6. The node is asked for once per process, and started only when created

`Delivering::start` returns `false` and does nothing on a second call, so creation
and start are each requested at most once however often startup runs. `StartNode`
is the first action in the worker's queue, so creation precedes every channel
operation. Doing *nothing* on the second call — no channel either — is
`stoa-membership`'s "Startup running again in the same module process MUST NOT
request any channel": the first run asked for every membership's, and every
create or join since asks for its own. What breaks without the guard:
`node_creation_is_requested_once_however_often_startup_runs` goes red.

A declined creation is logged with delivery's reason and the channels are still
requested. **`start` is then not requested**, as `op-transport` now requires
("Start SHALL be requested only after delivery has accepted this application's
creation", and the declined-creation scenario's "node start is not requested").
The reason it was chosen before the spec stated it still holds: delivery v0.2.1
declines a second `createNode` with "Context already initialized", which is most
often another module in the context owning the node — and a node this application
did not create is not its to start, any more than to stop.

**What counts as a decline** is decided once, in `declined`: a transport `Err`
(timeout, provider gone), an object whose `error` is a non-empty string, or an
object whose `success` is `false`. The envelope is read before the value because
delivery answers a decline as `Ok` at the IPC level with an error envelope in the
body. An **empty** `error` is no reason: `StdLogosResult.error` defaults to `""`
and logos-cpp-sdk's `lpPushExpr` serialises it verbatim, so a success can arrive as
`{"success":true,"value":…,"error":""}`; counted as a reason, no channel would
ever open. What breaks without each: making `declined` stop reading the error
envelope turned three tests red (the mutation list in PR #190's first pass);
dropping the empty-string filter turns `an_empty_error_string_is_not_a_reason_to_decline`
red.

**The cost of "unanswered is a decline" for the node.** A `createNode` delivery
accepted but answered after 35 s is recorded here as declined, so `start` is never
requested, and no channel on that node carries traffic until the module restarts.
Accepted: it needs delivery's runtime to take longer than its own 30 s callback to
create a node, and requesting `start` on a node this application may not have
created is the thing Decision 6 exists to avoid. Unlike a channel (Decision 14),
a node has no "already exists" answer to recover through: a second `createNode`
says "Context already initialized", which is also what another module's node says.

### 7. The sender identifier: 32 random bytes per Stoa, retained in `senders.sqlite`

Rendered `/dialectica/1/p/<hex>`. Each property the spec asks for, and why:

- **Unique per installation**, even two holding one identity. The spec-writer
  inferred this from `CLAUDE.md`'s own-send asymmetry and asked for it to be
  confirmed; it is confirmed from the SDS spec rather than inferred:
  `logos-lips/docs/anoncomms/raw/sds.md` says the sender "MUST include its own
  globally unique identifier" and a participant "SHOULD ignore the message if it
  has a `sender_id` matching its own". Two installations sharing a value could
  each discard the other's messages. Not measured against delivery.
- **Different per Stoa and made from no key**, because SDS traffic carries it
  even for someone who only reads: one value per installation would link that
  reader across every Stoa it is in. `SenderStore::sender_for` takes a Stoa
  address and nothing else, so no key reaches the code that mints one — the
  property `op-transport` now states as checked by reading that code.
- **Stable across restarts**, because it is retained and read back.

**The format.** 32 bytes because the one property with no coordinator behind it
is global uniqueness: 256 random bits make a collision between any two
installations negligible, and it is the width of every other identifier here, so
no new entropy argument is needed. The `/dialectica/1/p/` head, rather than bare
hex, for the reason `transport.rs` structures the channel id: the node is shared
with every other application, and a bare hex string claims nothing about what
minted it. `p` for *participant*, SDS's own word, beside the channel id's `c` and
the topic's `s`, so the three adjacent strings `channelCreate` takes are told
apart by eye in a log. It is a `SenderId` newtype so it cannot be passed where a
channel id or topic is wanted.

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
channel. What breaks without that: opening a channel with an unretained id turns
`a_sender_identifier_that_cannot_be_retained_opens_no_channel` red.

### 8. The send reads the op back from the log by id

The publish sink carries only the op id. The worker opens the log, reads the op,
and hands `transport::handoff`'s bytes to `channelSend` — so the payload is the
wire form as stored (for a re-publish, the bytes stored the first time), which is
what `op-transport` requires. `transport::handoff` shares one private function,
`addressed`, with `transport::publish`, so "is a channel open for this Stoa, and
which" has one answer. The sink's type did not change. What breaks without the
open-channel check: sending with no open channel turns
`a_publish_into_a_stoa_with_no_open_channel_sends_nothing_and_opens_nothing` red.

### 9. Create and join take a `joined` sink

`create_stoa` and `join_stoa` gained `joined: &mut dyn FnMut(&Address)`, called on
the success arm of `store.join` and nowhere else — the publish path's `deliver`
shape. So "after the membership is recorded" and "never on a refusal" hold by
position, and a repeated join (`Joined::AlreadyIn` is success) requests again. A
panicking sink is caught and logged, and the reply is unchanged
(`a_panicking_join_sink_does_not_change_the_reply`). This landed as its own
no-behaviour commit, with the op log path (Decision 13). What breaks without the
position: calling the sink before verification turns
`a_refused_join_requests_no_channel` red.

### 10. Inbound: a bounded queue of 256 that discards the arrival

The listener offers each event to `InboundQueue` and never waits on the boundary;
the processor takes them off in arrival order.

**Traffic on a channel this peer is neither holding nor opening takes no place.**
The listener asks the channel book whether the id is open or pending, and refuses
it as `unknown-channel` there, before the queue — as `op-transport` now requires.
The node is shared, so other applications' channel traffic arrives here, and any
peer may send on an identifier it picks; queued, that traffic filled the queue and
made every Stoa's ops the ones discarded, which the security review measured
(256 foreign messages, then a valid op: `Discarded(1)`). What breaks without it:
`traffic_on_a_channel_this_peer_is_not_opening_takes_no_place_in_the_queue` goes
red. "Being opened" starts when a create, a join or startup asks for the channel,
not when the worker asks delivery (Decision 11).

**A payload over the 150 KiB message limit takes no place either**, on a channel
open or being opened: it is refused as `too-long` on hand-over, as `op-transport`
now requires. The queue bounds a count, so while an oversized payload waited for
the boundary to refuse it, the queue's memory was the bound times the largest
message the *node* carries — and the node may be one another module created with
a larger maximum. Refused on hand-over, the payload bytes waiting are at most
256 × 150 KiB = 38,400 KiB = 37.5 MiB, this application's own figure. The
predicate is `transport::refuse_oversized`, the one `judge` asks, split out in a
no-behaviour commit so the two refusals cannot disagree about a payload of
exactly the limit. The unknown channel is asked first, so a message on a channel
neither open nor opening is refused as that whatever its size. What breaks
without each: dropping the size check turns
`an_oversized_payload_on_an_open_channel_takes_no_place_in_the_queue` red (it was
red before the check existed); `>=` in `refuse_oversized` turns
`a_payload_at_the_limit_waits_its_turn` red; asking the size before the channel
turns `an_oversized_payload_on_an_unknown_channel_is_refused_as_an_unknown_channel`
red.

**Rejected: a byte bound on the queue** beside the count. It bounds memory whatever
arrives, but a second bound needs its own discard rule and its own log line, and
the spec's bound is a count; refusing what the boundary would refuse anyway, one
step earlier, costs neither.

What remains shared is the bound itself across the Stoas this peer *is* in: a
flood on one of them can still force discards in the others. That is the owner's
(`proposal.md` Open Question 6), and this change implements the spec as written.

**The count, 256**, from two bounds. Memory: at most 256 × 150 KiB = 37.5 MiB of
payload waits, since an oversized payload is refused on hand-over (above); #30's
1024 would be 150 MiB by the same sum. That figure is payload bytes: the sender
identifier each message carries is held at whatever length arrives (Open
Questions). Loss: a discard is final for this peer. #30's
justification ("a dropped op is recoverable through retransmission") does not
hold — by the time `channelMessageReceived` fires, SDS has treated the message as
delivered and will not repair it. 256 is meant as a burst deeper than the
processor ever falls behind by; **how fast the processor decides an op has not
been measured**, so that is a judgement and not a figure.

**The arrival is discarded, reversing #30.** A backlog burst arrives roughly
oldest first: dropping the oldest loses thread roots and keeps orphaned replies;
dropping the arrival loses leaves and keeps every thread it holds whole. What
breaks without it: discarding the oldest turned
`a_full_queue_keeps_what_it_holds_and_discards_the_arrival` red, and `>=` → `>` in
the bound check turned three bound tests red (PR #190's first-pass mutation list).

**The clock is read when the message is processed**, after any wait, and never
from the event's timestamp. Judging the window by the event's timestamp turned
seven tests red, both window tests among them (PR #190's first-pass mutation
list); `the_clock_is_read_after_the_wait_for_an_open_and_not_before_it` pins the
"after any wait" half.

**This decision and Decision 11 interact**: while the processor waits on a
pending open, the queue behind it fills, and arrivals past 256 are discarded.
Decision 11 says what that costs and why it is accepted.

### 11. A message on a channel still opening waits for the answer

delivery v0.2.1 can emit `channelMessageReceived` for a channel after its runtime
created it and before the `channelCreate` answer reaches this peer. Judged in that
gap, it is refused as an unknown channel and lost (Decision 10). So the channel
book carries **pending** opens beside open ones, and the processor, meeting a
message on a channel that is pending and not open, waits for the open to settle,
bounded by `SETTLE_LIMIT` (40 s).

`op-transport` states the behaviour: a message on a channel that "is not open, but
is being opened, MUST be judged only once that open is settled", and one on any
other channel identifier, an open one included, without waiting; and the wait "is
bounded by a fixed time … whether or not delivery ever answers the open". The
wait's condition is exactly pending *and* not open, so a message on an open
channel whose open is being repeated, or on an unrelated channel, is judged at
once. Removing the wait turns
`a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` red. The
limit is a field of `Processor`, `SETTLE_LIMIT` in the running wiring, so
`a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait`
runs the bounded case with a short one.

**An open is pending from the request, not from the call.** `op-transport` says a
channel "is being opened from the moment a create, a join or the module's startup
asks for it, not from the moment delivery is asked", and that startup counts
every Stoa's channel as being opened "before it checks any message delivery
hands over". The reason is a restart: a module restarted while delivery kept
running is handed its Stoas' messages from the moment it subscribes, while its
own requests for those channels still wait behind node creation and each other.
**Rejected: pending from the worker's call to delivery.** It needs no guard to
travel between threads, but every message handed over while its open waits
behind another request is refused as an unknown channel and lost — at a restart,
that is every Stoa's traffic until the worker reaches its open.

It is built as a data shape, not a rule each path follows. The `Opening` guard —
which marks the open pending when made and settles it when dropped — is made
where the request is made (`Delivering::joined`, and startup's
`startup_opens`), and travels to the worker **inside** `Action::Open`. So every
way an open can fail to reach delivery settles it by dropping it: the worker
finds no sender identifier and returns; the send to the worker is refused and
hands the action back inside its error; the worker never started, so startup's
guards drop with its return; a panic unwinds through it. That is the spec's "an
open this peer never goes on to ask delivery for is given up", held by
ownership. No open is marked for a request with no worker to take it, because
`request` only builds the action once it has an outbox. Startup marks its opens
before it calls `subscribe`.

What breaks without each:
`a_message_on_a_channel_whose_open_waits_behind_another_is_judged_once_that_open_settles`
and `a_restarted_peer_keeps_what_delivery_hands_over_before_startup_asks_for_its_channel`
were red while the worker made the guard as it called delivery (the op never
stored). Moving startup's marking to after the subscription turned the second
red too, but only because the listener won a race against the dispatch thread —
the message is ready the instant the subscription exists — so that reordering is
*likely* caught, not certainly. Leaking the guard on the no-sender return turns
`a_sender_identifier_that_cannot_be_retained_opens_no_channel` red: it asserts the
given-up open is no longer counted as being opened. Forgetting the action a gone
worker refuses turns `a_join_the_worker_cannot_take_is_given_up_and_not_left_opening`
red. Startup's guards when the OS refuses the worker thread have no test — no
test here makes `thread::Builder::spawn` fail — and rely on `start` returning
with them in scope.

**`SETTLE_LIMIT` bounds a stall of every Stoa, not an open.** There is one
processor, so while it waits, messages on every other channel wait behind it and
the queue fills. 40 s is past `CALL_TIMEOUT`, so it outlasts **one** delivery
call: an open that is the worker's current call — a join, or a creation racing
its own answer, the ordinary cases — always settles within it. It does **not**
outlast an open queued behind others, which can now be pending for node creation
plus every earlier Stoa's creation, each up to 35 s against an unresponsive
delivery. A message on such a channel is judged when the limit passes — measured,
as the spec says, from when the processor began waiting on *that message* —
against what is open then, which is a refusal and a lost op.

**Rejected: a limit sized for a queue of opens** (35 s × the Stoas ahead of it).
The wait is per message, so every message on a still-queued channel would hold
every Stoa up for that long, one after another, and 256 of them fill the queue
into discards on channels that are open. A limit that covers one call keeps the
stall one call long; what it gives up is messages on channels whose opens are
queued behind an unresponsive delivery, a state in which delivery is handing
messages over while not answering calls.

**The wait is its own loop, not `Condvar::wait_timeout_while`.** Once the book's
mutex is poisoned — by a contained panic under it — `wait_timeout_while` returns
`Err` at its first wake-up, so the wait would end when *any* open settled and the
message would be refused in exactly the gap this exists to close. The loop takes
the guard back from a poisoned wake-up and ends only on its own condition or the
limit. What breaks without it: `a_poisoned_channel_book_still_waits_for_this_channels_open`
goes red.

**What the wait costs.** While the processor waits on one slow open — up to
`SETTLE_LIMIT` per message — messages on every *other*, open channel wait behind
it, and once 256 are waiting, arrivals are discarded for good.

**Alternatives, and why not:**

- **Refuse as an unknown channel without waiting.** Loses the message in the gap
  every time delivery's event beats its answer — the ordinary case for a busy
  channel, not an edge. The spec now forbids it.
- **A per-channel hold**: park the pending channel's messages aside and let the
  processor carry on with others. It keeps other Stoas flowing during a slow open,
  but needs a second store of messages with its own bound, its own discard rule
  and its own ordering relative to the main queue — a second copy of Decision
  10's reasoning, for a stall `SETTLE_LIMIT` bounds.
- **Requeue the message at the back**: breaks "decided in the order they
  arrived", and spins the processor while the open is pending.

The stall is bounded by `SETTLE_LIMIT` per message and happens only while this
peer is opening a channel, which it does at start, on create and on join;
accepted over a second queue.

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
"refused". An op that arrives again reads "already held; nothing new was stored",
not "stored" — #102's items 5–7 are checked by reading this log by hand, so a line
that said "stored" for a duplicate would be read as a fact. What breaks without
the kind-only wording: carrying the refusal text and channel id turns
`a_refusal_is_logged_by_kind_without_text_the_sender_chose` red.

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
  on a busy database — for up to 5 s, rusqlite's default busy timeout, which
  nothing here sets or changes (Decision 3 has the measurement, and what it
  bounds: a publish reply waiting behind an inbound append).
  `two_connections_appending_at_once_both_store_everything` pins the waiting;
  setting the busy timeout to zero turns both tests red with "database is
  locked".

`stoas.sqlite` and `senders.sqlite` are each opened by one thread only (dispatch,
and the worker), so neither has the race; `sender.rs`'s `create_schema` says so,
and names the op log's as the model if that stops holding.

### 14. "Already exists" opens the channel, recognised by delivery's wording

`stoa-membership` requires an answer that the channel already exists to open it,
exactly as a report of creation does. Delivery gives that answer in two cases
this peer really meets:

- **A creation that outlived delivery's callback.** v0.2.1's `callApiRetValue`
  answers `"channel_create callback timeout"` after its own 30 s while the Nim
  `createReliableChannel` carries on and creates the channel. A later
  `channelCreate` for the same id is answered
  `"ChannelCreate failed: channel already exists: <id>"`.
- **A module restart under a running delivery.** Delivery keeps every channel, so
  startup's creation of each Stoa's channel gets the same answer.

Read as a decline, either left the channel shut for as long as delivery ran: every
message on it refused as an unknown channel, every send skipped, with delivery
holding the channel open throughout. The correctness review's probe reproduced
the first.

**Chosen: match the words.** `channel_answer` reads a decline whose reason
contains `"channel already exists"` (`ALREADY_EXISTS`) as `AlreadyHeld`. Read at
`logos-delivery` `bfdb5afd`: `channel_lifecycle.nim` answers
`err("channel already exists: " & channelId)` exactly when the manager holds the
id, and `library/channels_api/channel_api.nim` prefixes `"ChannelCreate failed: "`.
Matched as a substring, because the prefix and the trailing id belong to the C API
and the manager and neither is what says the channel is held.

**Rejected: confirm with `channelExists` after a decline.** It would survive a
rewording, but widens the four-method seam (Decision 2) with a fifth call whose
reply shape (`"true"`/`"false"` as a string, per `wire::channel_exists_reply`) is a
second contract to get right, and it adds an IPC round trip to every decline. It
also does not help the timeout case any sooner: right after the callback gives up,
the runtime may not have finished, so `channelExists` would say `false` too.

**What a rewording costs, and why that is the cheaper direction.** If delivery
changes the words, the answer reads as a decline again and the channel stays shut
until delivery restarts — the defect this fixes, silently back. A false positive
(some other error containing the phrase) would count a channel open that delivery
does not hold: its sends are then declined by delivery and logged, and nothing
arrives on it. So the match is kept to the manager's own phrase, and the tests pin
the verbatim string: `a_channel_delivery_reports_already_existing_is_open`,
`a_creation_delivery_did_not_complete_in_time_opens_on_the_next_request`,
`a_module_restarted_while_delivery_kept_running_has_its_channels_open` and
`only_delivery_s_already_exists_answer_opens_a_declined_channel` all go red without
the `AlreadyHeld` arm. A delivery pin bump should re-read `channel_lifecycle.nim`.

### 15. The processor judges before it opens the op log, and holds no lock while it does

The boundary's order — look the channel up, `judge`, then the one write — is
written once, in `transport::receive_via`, which takes the lookup and the write
as parameters. `transport::receive` is `receive_via` over an `OpenChannels` and a
log it is handed; the processor's `pass` is `receive_via` with a lookup that
copies the channel's Stoa out of the book and releases the lock, and a write that
opens the op log only for an op that passed. `judge` is private and only
`receive_via` calls it, with the Stoa it looked up under the message's own
channel identifier; `Judged` has a private field and only `judge` makes one. So
`admit` cannot be handed an op that skipped a check, or one judged against
another channel's Stoa: "every check runs before anything is written" holds by
the type, not by the caller's order.

Two findings forced the lock and the late open. The book's mutex was held across
verification and the SQLite append, so an append waiting on a busy database held
up the worker's opens and sends, against Decision 3's claim that nothing the
event loop or the worker needs is held while a payload is decided. And the log was
opened before the channel was looked up, so the cheapest refusal paid a database
open (measured by the security review at ≈ 400× the refusal alone), and with the
log unopenable a message on a channel nobody opened was logged as a storage
failure. What breaks without it: `the_channel_book_is_not_held_while_an_op_is_appended`
and `an_unknown_channel_is_refused_as_one_before_the_op_log_is_opened` go red.

**Rejected: the processor composing the three steps itself**, as it first did
after the split. It worked, but it was a second spelling of the boundary's
order that none of the boundary's ~55 tests ran: a step added to `receive` would
have been covered by every one of them and absent from the running module. And
`judge` took the channel's Stoa as a bare address beside the message, so the tie
between the two was the caller's convention. The architecture re-review found
both. `transport::tests::the_boundary_looks_a_channel_up_under_the_messages_own_identifier`
pins that the lookup is keyed by the message's own channel identifier.

**Rejected: a snapshot of the open set per message.** It releases the lock too,
but copies a map per message on the path whose rate a sender chooses.

### 16. The listener contains a panic per event, not per loop

The iterator's `next()` is where the SDK receives an event and the generated
decoder reads its fields — the one piece of the inbound path that touches event
data before the queue. Containing a panic around the whole loop ended reception
for every Stoa, until restart, over one event. Each `next()` and its hand-over run
under their own `catch_unwind`, and the loop carries on — the analogue of
`op-transport`'s "a delivered message whose fields cannot be read MUST be
discarded and recorded", and the reception keeps going. No panic is known to be
reachable in the decoder at this pin (the security review read it); this is the
containment the trap table claims. What breaks without it:
`a_panic_reading_one_event_does_not_end_reception` goes red.

The cost: an iterator that panicked on every call *without consuming an event*
would spin, logging. The SDK's receives the event before decoding it, so the next
call takes the next event.

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
  `op-transport`'s inbound-boundary requirement and its "while its channel
  opens" / "does not wait" / "bounded wait" scenarios (Decision 11).
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
- **An oversized payload is refused on hand-over** — `op-transport`, "Inbound
  payloads waiting for the boundary are bounded" (Decision 10). Proposed by the
  security review's deferred finding, put to the spec-writer, and adopted.

### How each `CLAUDE.md` module-contract trap is handled

| Trap | Handling |
|---|---|
| No handler may unwind | Handlers only enqueue. Both sinks are caught (`delivered_and_published`, `recorded_and_replied`). The worker runs each action, the processor each message and the listener each event under its own `catch_unwind` and logs a panic (Decision 16). Locks are taken poisoned-or-not, the pending-open wait included (Decision 11). Threads are started with `thread::Builder`, which reports a refusal where `thread::spawn` panics. |
| `recv()` may block forever on an older SDK | The builder pin (`9f420c2`) stages an SDK whose `recv()` polls the provider's `Abandoned` status every 200 ms and fails once it is reported, so the listener's loop ends and logs "the inbound listener has ended". That needs a runtime with the status channel (logos-protocol 0.9); on an older one `recv()` parks forever — one leaked thread holding nothing another waits on. |
| `RET_STALE_WARN` (3) | Not reachable on this surface, confirmed at v0.2.1: `api_call_handler.h` and the start/stop callbacks return early on it, so each channel call answers once, terminally. This change subscribes to no outcome event, so there is nothing to count twice. `CLAUDE.md`'s entry now says which surface still sees it. |
| Own sends arrive as `channelMessageSent` | Not subscribed to. The own op is already in the log (stored before it is sent), so nothing is lost. `only_reliable_channel_receipts_reach_the_op_log` reads the adapter's subscriptions. |
| The node is a singleton per Logos Core instance | This application asks once per process (Decision 6). delivery refuses a second `createNode` ("Context already initialized"); that is logged as a decline and channels are still requested. |
| `createNode` exactly once; never `stop()` | Once per process by `Delivering`; no `stop` on the seam; the adapter-source test. |
| `messageReceived` timestamps are nanoseconds | Not subscribed to. `channelMessageReceived`'s timestamp (v0.2.1: `currentTimestampNs()` at receipt) is carried in `Arriving` and read by nothing; the window uses the host clock at processing time. |
| A decline arrives as `Ok` with an error envelope | `declined` reads `callee_error` first (an empty string is no reason), then `success: false`, then treats a transport `Err` as a decline (Decision 6). |

## Risks / Trade-offs

- [`Edge` depends on the `logos.test` fleet's service nodes] → a desktop module
  stays light; revisit with the owner (Open Questions).
- [A slow delivery delays later sends by up to 35 s each] → replies are unaffected;
  the outbound queue is unbounded, but each entry is caused by a local call from
  the view, so it grows at the user's rate.
- [A slow open stalls inbound processing for every Stoa for up to `SETTLE_LIMIT`
  per message waiting on it] → Decision 11 records why a second queue was not
  worth it.
- [An open queued behind others can stay pending longer than `SETTLE_LIMIT`, so a
  message on its channel is refused and lost after holding every Stoa up for
  40 s] → needs delivery to hand messages over while not answering calls, at
  start or on a join made behind a slow open. Decision 11 says why the limit
  covers one call and not a queue of them.
- [256 is a guess at burst depth] → discards are counted and each is logged with
  the running total, so a bound that is too small is visible in the log.
- [Messages on another application's channels on the shared node are logged as
  `unknown-channel` refusals] → one line each, on hand-over, carrying no channel
  id, and taking no place in the queue (Decision 10).
- [The already-exists match depends on delivery's wording] → Decision 14; a pin
  bump should re-read it.
- [The adapter is compiled only by `nix build ./dialectica#lgx`] → it is four
  calls, one subscription and one decoder mapping; the generated names it relies
  on (`create_node_with_timeout`, `on_channel_message_received`,
  `decode_channel_message_received` and its fields) are checked by that build and
  by nothing else. **Two peers exchanging an op has not been verified live**:
  task 7.3 is open, and issue #176 names that check as the verification this
  change needs (Open Questions).

## Migration Plan

`senders.sqlite` is created on first use; no existing file changes. The op log's
schema-race fix changes how a fresh store is created, not what is written.

## Open Questions

- **Preset and mode.** `logos.test` and `Edge` are recorded above with their
  meaning. Either can change without changing the spec — a constant and its pin —
  but both decide which network dialectica peers meet on and whether they relay,
  so the owner should confirm them.
- **Live verification (a departure from issue #176, put to the owner).** #176
  says "Verification has to be two live peers under `lgs basecamp launch`". That
  has not been run: every test here drives both sides in one process against a
  fake delivery, and a live run needs a person to click each profile's tile. The
  change is otherwise complete against its spec, so it is offered for merge with
  task 7.3 open, for the owner to accept or to hold the merge until 7.3 is run.
  The automated two-peer test the owner asked for on #176 is follow-up.
- **`proposal.md`'s questions 1–4 are settled by the spec, and implemented**: a
  failed open leaves the join's reply unchanged and is logged (Decisions 9 and
  11); the sender identifier is per installation and per Stoa (Decision 7); a
  full queue discards the arrival (Decision 10); a re-publish sends again
  (Decision 8, and `tasks.md` 4.4). Each is reversible only as a spec change and
  a code change together. Question 5 is the preset and mode above.
- **Question 6, shared-queue fairness, is the owner's.** The spec closes the
  part about channels this peer is not opening (Decision 10); a flood on a Stoa
  this peer *is* in still shares the bound with every other Stoa. Not
  pre-empted here.
- **The sender identifier's length is the owner's.** A message's sender
  identifier is held while it waits at whatever length delivery handed over,
  bounded only by the largest message the node carries, so the 37.5 MiB in
  Decision 10 is payload bytes and not every byte a waiting message holds. The
  spec does not bound it, and the spec-writer's round that adopted the
  oversized-payload refusal left it for the owner (`findings/security.md`); not
  decided here.
