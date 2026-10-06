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
waits on this but the worker. **No test waits it out**: it bounds a real IPC
call, which only a live delivery has. Its value is argued here and in its doc,
and `op-transport` now requires the order delivery's own < this < `SETTLE_LIMIT`
without fixing the values. Two guards hold that order. Compile-time asserts
beside `SETTLE_LIMIT` hold the constants against each other, so `CALL_TIMEOUT`
set back to 20 s fails the build (Decision 11). They cannot hold delivery's real
30 s, since `DELIVERY_CALLBACK_TIMEOUT` lowered with it still compiles;
`the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
writes the 30 s as a literal and is red with `DELIVERY_CALLBACK_TIMEOUT` at
10 s and `CALL_TIMEOUT` at 20 s, which compiles.

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
- **Measured live, not by a test: an `Edge` node on `logos.test` receives
  `channelMessageReceived` on a reliable channel.** That is the whole feature,
  and only task 7.3's two live peers could show it: the owner's second run had
  each peer store the other's ops (Risks). Nothing in this change's tests can.

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
ever open. What breaks without each: making `declined` ignore the `error` field
(its `callee_error` filter never matching) turns
`declined_reads_delivery_s_three_shapes_of_no` and every "already exists" test
red, among others (a count is left out on purpose: it grows as tests are added);
dropping the empty-string filter turns
`an_empty_error_string_is_not_a_reason_to_decline` red.

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
`InboundQueue::offer`'s bound check turns that one,
`the_waiting_messages_never_exceed_the_bound` and
`the_queue_the_running_wiring_builds_is_bounded_at_the_pinned_count` red.

**The clock is read when the message is processed**, after any wait, and never
from the event's timestamp. Reading `now_ms` from `message.timestamp` in
`Processor::pass` turns
`the_window_is_judged_by_this_peers_clock_not_the_events_timestamp` red, among
many others (most refuse a valid op as ahead of a clock of a few milliseconds);
`the_clock_is_read_after_the_wait_for_an_open_and_not_before_it` pins the "after
any wait" half.

**This decision and Decision 11 interact**: while the processor waits on a
pending open, the queue behind it fills, and arrivals past 256 are discarded.
Decision 11 says what that costs and why it is accepted.

### 11. A message on a channel still opening waits for the answer

delivery v0.2.1 can emit `channelMessageReceived` for a channel after its runtime
created it and before the `channelCreate` answer reaches this peer. Judged in that
gap, it is refused as an unknown channel and lost (Decision 10). So the channel
book carries **pending** opens beside open ones, and the processor, meeting a
message on a channel that is pending and not open, waits for the open to settle,
for at most `SETTLE_LIMIT` (40 s) **from each start of the open's time**, which
the first waiting message and this peer's ask of delivery each start — below.

`op-transport` states the behaviour: a message on a channel that "is not open, but
is being opened, MUST be judged only once that open is settled", and one on any
other channel identifier, an open one included, without waiting; and the wait "is
bounded by a fixed time for each open, not for each message". The wait's
condition is exactly pending *and* not open, so a message on an open channel
whose open is being repeated, or on an unrelated channel, is judged at once.
Removing the wait turns
`a_message_arriving_while_its_channel_opens_is_judged_after_the_answer` red. The
limit is a field of `Processor`, set to `SETTLE_LIMIT` by `Processor::new`, the
one place a processor is built — `Delivering::start` and the tests' fixtures
alike, so a test reading the limit reads the running module's (spec-test
re-review round 3). The bounded cases shorten it on what `new` returns. Removing
the bound altogether — no deadline, the wait ending only on a settle — turns
red, each timing out in `eventually`:
`a_message_waiting_on_an_open_delivery_never_answers_is_judged_after_a_bounded_wait`,
`opens_settling_for_other_channels_do_not_extend_the_bounded_wait`,
`many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each`,
`each_unanswered_opens_wait_is_its_own_and_not_one_shared_across_opens`,
`an_open_whose_wait_has_expired_still_opens_its_channel_when_delivery_answers`,
`a_new_request_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`,
`a_request_made_while_a_message_waits_does_not_extend_that_messages_wait`,
`asking_delivery_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
and `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again`.

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
given-up open is no longer counted as being opened.
`an_open_this_peer_gives_up_without_asking_delivery_does_not_hold_a_message_up`
states the same through the running wiring and a message, as the spec's scenario
does: refused as an unknown channel well inside `SETTLE_LIMIT`. It too is red with
the guard leaked on the no-sender return (the refusal never comes inside ten
seconds). Forgetting the action a gone
worker refuses turns `a_join_the_worker_cannot_take_is_given_up_and_not_left_opening`
red. Startup's guards when the OS refuses the worker thread have no test — no
test here makes `thread::Builder::spawn` fail — and rely on `start` returning
with them in scope.

**The time is kept per open, not per message.** Each pending open in the book
carries its time, an `OpenTime`, and each message waiting on it a `Wait` holding
its own end. The first message to wait after a request fixes the time's end, that
instant plus the limit, and every later message is given that same end, so once it
has passed they are judged at once, against what is open then — refused while the
open is still unanswered. That is the spec's expired wait, and it needs no state
of its own: an end in the past is it. Nothing else about the open changes: it
stays pending, so hand-over still admits its messages to the queue, and delivery's
answer, when it comes, settles and opens it. A new create, join or startup request
puts the time back to not started, so the next message waits again, for a fresh
`SETTLE_LIMIT`; it moves no waiting message's end. What breaks without each part:

- keeping the time per message (`OpenTime::end` giving every message now plus the
  limit) turns
  `many_messages_on_one_unanswered_open_hold_other_channels_up_for_one_wait_not_one_each`
  red (the op behind four stuck messages waited 4.02 s at a 1 s limit, where the
  test allows under two) and
  `an_opens_time_is_the_same_for_every_message_that_waits_on_it_until_a_request_clears_it`;
  when the per-open shape was first built, four messages held the op 1.61 s at a
  400 ms limit;
- giving the open up when its time passes (removing it from the book) turns
  `an_open_whose_wait_has_expired_still_opens_its_channel_when_delivery_answers`
  red — the next message is refused on hand-over — and with it
  `a_request_made_while_a_message_waits_does_not_extend_that_messages_wait` and
  `asking_delivery_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`;
- a request not putting the time back turns
  `a_new_request_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
  red — the message is judged at once and refused — and
  `an_opens_time_is_the_same_for_every_message_that_waits_on_it_until_a_request_clears_it`;
- the waiter reading the open's time on each wake-up in place of its own end
  turns `a_request_made_while_a_message_waits_does_not_extend_that_messages_wait`
  red — a stream of requests postpones the waiting message for as long as it
  lasts — and `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again`.
  What a request made *during* a wait does to that wait is the `op-transport`
  scenario "Requests made while a message waits do not lengthen its wait", which
  that test cites.

**This peer's ask of delivery starts the open's time again, and extends a waiting
message once.** `op-transport`, "Asking delivery to create a channel starts the
open's time again", added by the spec-writer after the round-3 security
re-review. The worker marks the ask
in the book (`Opening::asked`) immediately before `channelCreate`, and it does two
things there: the open's time becomes *started at the ask*, whether or not it had
ended, so the next message to wait ends the fixed time after the ask; and every
message waiting at that moment whose end has not yet passed gets a new end, the
fixed time after the ask, in place of the one it began with — unless an earlier
ask already gave it one. A message whose end has passed is not extended, even
while its wait is still in the book ("Why only a wait that has not ended" below).

*Why the ask, the finding it answers.* An open is pending from its request, so its
time can start while the open is still queued behind other calls. When the worker
then reaches it, the time left is `SETTLE_LIMIT` less the time spent queued, which
can be shorter than the call — so the message the wait exists for, one racing
delivery's answer, could be refused mid-call and lost. The old promise that 40 s
"covers the ordinary cases (the open being the worker's current call)" broke once
more than 5 s (40 − 35) of calls sat ahead of the open. The security re-review
(round 3) measured it scaled down: limit 1000 ms, a junk payload waiting from 0,
an honest op handed over at 800 ms, the open settled held at 1100 ms (a call begun
at 600 ms and answered 500 ms later), and the op was **not stored**; both messages
were refused `unknown-channel`. `an_earlier_message_on_a_queued_open_does_not_cost_an_op_that_arrives_while_delivery_is_asked`
is that probe with an ask at 600 ms and the opposite expectation. Restarted at the
ask, every message waiting at the (first) ask or beginning after it is judged only
once the call is answered or given up, because `SETTLE_LIMIT` outlasts
`CALL_TIMEOUT`.

*Why once per message.* Without the cap, repeated joins against a hung delivery
chain asks every ≤ 35 s, each extending the same waiting message by 40 s, and
postpone it — and every other Stoa behind it — for as long as the joins last. The
cap is what keeps "Requests made while a message waits do not lengthen its wait"
true now that an ask can move an end. It is `Option::take` on the `Wait`'s
remaining extension: a second ask finds nothing to take.

*Why only a wait that has not ended.* A message's `Wait` stays in the book past
its end until the waiter re-takes the lock, and the ask can take the lock first —
the waiter wakes at its end and queues behind whoever holds the book (the
listener's `is_known`, a settle, a hand-over), an ordinary interleaving. Extended
then, a wait that had expired would run a full limit again, against the spec's
"the message waiting then ... MUST be judged without waiting on that open", and
the "less than twice `SETTLE_LIMIT`" below would not hold: the stall would be the
overshoot plus a limit. So `Wait::extend_from` extends only when the ask is before
the wait's end, reading the end as the waiter does (no time left is ended). The
correctness re-review (round 4) found it with a probe that holds the book across
the end and asks inside it. *Rejected: have the waiter drop its own `Wait` at its
end* — it cannot, since dropping needs the lock the ask is holding; the check has
to be where the ask is applied.

*What still loses a message.* One whose wait ended before this peer asked delivery
at all — an open queued behind more than `SETTLE_LIMIT` of other calls — and the
channel's messages after it until the ask; and a message already extended once,
when a later ask comes while it still waits. The spec names both.

What breaks without each part:

- the ask not extending waiting messages turns
  `a_message_waiting_when_delivery_is_asked_is_judged_after_delivery_answers`,
  `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again` (its message
  is no longer waiting at the second ask) and
  `the_worker_marks_its_ask_of_delivery_in_the_channel_book` red;
- the ask not starting the open's time again turns
  `asking_delivery_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
  red;
- the extension uncapped (every ask moving the end) turns
  `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again` red — refused
  1.71 s after the first ask at a 1 s limit, where the test allows 1.4;
- the extension given to a wait whose end has passed turns
  `an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again` red —
  judged 1.00 s after the ask at a 1 s limit, where the test allows 0.75;
- the worker not marking its ask turns
  `the_worker_marks_its_ask_of_delivery_in_the_channel_book` red.

`an_earlier_message_on_a_queued_open_does_not_cost_an_op_that_arrives_while_delivery_is_asked`
is red only with both halves gone (as it was, against a no-op ask, before this was
built): the extension alone keeps the junk waiting until the open settles, and the
restart alone gives the op a wait of its own. Five of the tests named in this
paragraph and the list above were each red against a no-op ask before the change:
the four new scenarios' —
`a_message_waiting_when_delivery_is_asked_is_judged_after_delivery_answers`,
`an_earlier_message_on_a_queued_open_does_not_cost_an_op_that_arrives_while_delivery_is_asked`,
`asking_delivery_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
and `a_second_ask_while_a_message_waits_does_not_extend_its_wait_again` — and the
worker's, `the_worker_marks_its_ask_of_delivery_in_the_channel_book`. The sixth,
`an_ask_after_a_messages_wait_has_ended_does_not_make_it_wait_again`, is not one of
them: it came after the ask was built, and holds the guard in `Wait::extend_from`,
whose removal is what turns it red (its bullet above). A no-op ask extends nothing,
so it is not the mutation that test answers.

*Why the book holds each waiting message's end, and not the waiter.* The
extension is from the **first** ask after a message began waiting, and two asks
can land between two of the waiter's wake-ups (an ask, a call declined at once,
the next queued open for the same channel asked). A waiter that looked at "the
latest ask" would take the second's time, which is what the cap forbids. Applied
by the ask itself, under the book's lock, to every `Wait` registered then, "first"
is exact. *Rejected: a log of asks per open that each waiter indexes from where it
began* — exact too, but it grows with every ask for as long as the open is
pending, and an index means nothing once the pending entry is removed and made
again.

*Why the open's time has three states* (`NotStarted`, `StartedAt(ask)`,
`Ends(instant)`). The ask knows when it happens, not the limit a message waits by,
so it cannot fix an end; the next message to wait does, from the ask. Once fixed,
the end is kept. *Rejected: store only the start and add each reader's limit* —
an ended time would then be ended or not depending on who read it, and
`a_new_request_for_a_channel_whose_wait_has_expired_lets_a_message_wait_again`
(whose expiry runs at 100 ms and whose next reader waits by 10 s) would pass with
the request's reset removed. *Rejected: move the limit into the book* — it is the
processor's, set in one place (`Processor::new`), and a second copy in the book
could disagree with it.

*A wait gone from the book ends the wait.* It goes only with the open's pending
entry, when every request it waited on has settled, which is what the message
waits for; a request made since gives a new wait only to the messages after it.
The waiter's loop is reached this way only when the open settles and is asked for
again between two of its wake-ups, which no test here stages.

*Why each wait leaves the book when its message is judged.* `op-transport`,
"Nothing of a message's wait on an open is kept once it is judged". The
correctness and security re-reviews (round 4) found `end_wait` pinned by no test;
the tests that answered them pinned a property no requirement stated, and the
spec-writer made it one after the spec-test re-review (round 5). Each waiting
message is a `Wait` in its open's pending entry, keyed by the `WaitId` that
`ChannelBook::begin_wait` hands the waiter from a counter on the book.
`ChannelBook::end_wait` removes it by that id as `Channels::await_settled`
returns, after the loop, whichever way the loop ended: the open settled, the
message's end passed while it waited, or the end had already passed when it
began, so it never slept. Because the ids come from one counter, `end_wait` on a
pending entry a later request made again removes nothing of another message's.

The constraint is that nothing else removes a `Wait` soon enough.
`Channels::settle` removes the pending entry, and every `Wait` in it, only when
the open's last request settles, and a stuck open is pending for a long time —
up to (K+1) × `CALL_TIMEOUT` at startup, and for many `CALL_TIMEOUT`s for a join
queued behind a backlog in the unbounded outbound queue (Risks). Kept until then,
each message a sender put on that channel would add an entry — cheapest after the
time has ended, when each is judged at once — so the sender would choose how far
the book grew, and every later ask would walk all of those entries under the
book's lock (`Pending::asked`). Removed as each wait ends, with the one processor judging one
message at a time, the book holds at most one message's wait: the spec's stated
consequence.

*Rejected: no record per message* — the ask extending a count of waiters, or one
`ends` shared by every message waiting on the open. There would be nothing to
remove, but it cannot hold the once-only cap, because which messages an ask has
already extended is a fact about each message ("Why the book holds each waiting
message's end, and not the waiter", above).

The cost is a map insert and remove per waiting message, each under the book
lock the waiter already holds at that moment, and a structural obligation:
`end_wait` runs once after the loop, so every exit from the loop must fall
through to it. As `await_settled` is written, every exit is a `break` or the
loop's condition failing, and its one early `return` is before `begin_wait` has
made a `Wait`; a `return` added inside the loop would leak one entry per message
that took it.

What breaks without it: `end_wait` doing nothing — a mutant every `delivery::`
test survived when the correctness re-review ran `cargo mutants` in round 4 —
turns red, one test per way the wait ends,
`a_message_that_waited_its_opens_time_out_leaves_no_wait_in_the_book`,
`messages_judged_at_once_after_an_opens_time_has_ended_leave_no_wait_in_the_book`
and
`a_message_whose_open_settles_held_leaves_no_wait_while_another_request_is_pending`
(measured by the tester when it wrote them, and again by the correctness
re-review in round 5). Each reads the open's waits through a test helper that
answers only while the open is still pending, so the pending entry going away
cannot pass as "no wait left".

**Why per open: the security re-review measured the per-message shape.** The
first version started the clock afresh for every message (`started` was local to
`await_settled`), which the spec's earlier wording ("no later than a fixed time
after this peer began waiting on it") asked for. Since an open is pending from the
request, one queued behind k other calls against an unresponsive delivery is
pending for up to (k+1) × 35 s, and channel ids are public, so any peer can put
messages on it; the payload need not even be an op. n such messages held the one
processor — every Stoa — for n × 40 s, capped only by how long the open stayed
pending: the reviewer's probe put 10 messages on a never-answered open at a
200 ms limit and the valid op behind them waited 2.01 s. For the 20th of 20 Stoas
at startup that is up to 21 × 35 s = 735 s pending, which ⌈735 / 40⌉ = 19
messages on that one channel filled, with every arrival on an open channel
discarded for good once the queue was full; an open pending longer (one queued
behind a backlog of sends, whose queue is unbounded) stretched further with every
message. The sender chose the stall. The spec was changed to bound the time per
open, and this is that. What it removes is a sender stretching **one** stuck
open to its whole pending time; what it does not remove at startup, where every
channel is stuck at once, is below.

**What per open gives up.** A message on the stuck channel taken after the time
has passed is judged at once and refused, where the per-message shape would have
given it its own 40 s and might have stored it. That loss is confined to the
channel whose open is stuck; the per-message shape spread its cost to every Stoa
through the shared queue.

**What still bounds the stall, then: two bounds, and the smaller holds.**

- **One `SETTLE_LIMIT` per start of an open's time.** Each start holds every Stoa
  up at most once, for at most `SETTLE_LIMIT`, however many messages arrive on the
  channel, and only this peer's requests and asks start one, so no sender adds
  starts. A message the ask extends holds every Stoa up for **less than twice
  `SETTLE_LIMIT`**: under it before the ask, or its wait would have expired, and at
  most it after. In practice the part after the ask is shorter: the call is
  answered or given up within `CALL_TIMEOUT`, settling the open, unless another
  request for the channel is still pending. So one junk message on a queued open
  now holds every Stoa for under 80 s, and in practice under 40 + 35 = 75 s, where
  before the ask it held 40 s.
- **The last open's settle.** No wait outlasts its own open, and the waits run
  one after another on one wall clock, so each ends by a fixed instant, the
  moment its open settles. The stall therefore never extends past the moment the
  last of those opens settles, `op-transport`'s own words for it. This is not a
  sum of pending times: with opens settling at 70 s, 105 s, … the wait on the
  last ends when it settles, however long the waits before it took.

At startup against a delivery that answers nothing, open m of K is asked at
m × 35 s and settles by (m+1) × 35 s, counted from startup (node creation
first). A sender putting one message on each stuck channel, in the order they
settle, has the processor reach the m-th message when open m − 1 settles, which is
when the worker asks for open m; the ask gives that message an end 40 s later,
past open m's settle 35 s later, so every wait runs to its own open's settle. The
stall is therefore (K+1) × 35 s **for every K**: 735 s for 20 Stoas, unchanged,
and for fewer than 7 Stoas somewhat longer than the min(40m, (m+1) × 35) it was
before the ask (for K = 1, 70 s where it was 40 s). The security re-review
measured the ask-less shape scaled down, with 35 s → 100 ms and `SETTLE_LIMIT` →
114 ms: node creation, then 8 opens settling at 200…900 ms, one message on each in
settle order, then a valid op on an open channel. The op was stored after 914 ms
at the per-open limit and 917 ms at a 10 s one. Three opens settling at 200, 400
and 600 ms under a 10 s limit stored it at 612 ms, where waits summed in turn
would have predicted 1200 ms. Those probes made no ask, so they measured the
last-settle bound alone, which the ask does not move.

**`SETTLE_LIMIT` is 40 s: past `CALL_TIMEOUT`, and started again by the ask, so
it outlasts the call it waits on.** A message waiting on an open when the worker
asks delivery for it (its first ask while it waits), or beginning to wait after, is
judged only once that call is answered or given up, however long the open waited
in the queue first. It does **not** save a message whose wait ended before the
ask: an open queued behind node creation and earlier Stoas' creations, each up to
35 s against an unresponsive delivery, can see its time run out in the queue, and
that message and the channel's messages after it until the ask are refused and
lost. The order delivery's own 30 s < `CALL_TIMEOUT` < `SETTLE_LIMIT`, on which
this and Decision 4 rest, is `op-transport`'s requirement (the values are left to
design), and two guards hold it. Two compile-time assertions beside
`SETTLE_LIMIT` hold the constants against each other: setting `SETTLE_LIMIT` to
10 s, or `CALL_TIMEOUT` back to the IPC default of 20 s, fails the build (both
tried). Before them, both changes together passed every test, which the
correctness re-review measured, because no test waits out either value.
`the_call_timeout_outlasts_deliverys_own_and_the_settle_limit_outlasts_the_call`
holds them against delivery's own 30 s as a literal, which the asserts cannot
(Decision 4).

**Rejected: a limit sized for a queue of opens** (35 s × the calls ahead of the
open, plus its own). Its case has been argued wrongly twice, and both are
recorded because each is the intuitive reading.

- *Withdrawn, first:* that under the per-message clock every message on a
  still-queued channel would hold every Stoa up for that long, one after another.
  The security re-review showed that with a limit outlasting the pending time only
  the first message waits, since the open has settled by the second.
- *Withdrawn, second:* that under per open, startup's K opens held in turn would
  grow as the square of K under a queue-sized limit where 40 s grows as K. Each
  wait ends when its open settles, a fixed instant on one wall clock, so the
  total is the last open's settle, (K+1) × 35 s, under either limit (above), and
  the round-2 readability re-review worked it by hand and the security
  re-review measured it.

What the two limits actually differ in:

- **How far one stuck open can hold every Stoa.** A queue-sized limit lets a
  single message on the deepest pending open hold every Stoa for that open's
  whole pending time. 40 s holds it for 40 s if the open is not asked for in that
  time, and otherwise until its call ends (under 80 s, above). The two agree when
  a sender puts a message on every stuck channel at startup, for any K now that the
  ask extends each (above). They differ where a message begins waiting on an open
  more than 40 s before the worker asks delivery for it: a message on only a deep
  channel at startup, or on a join made behind a backlog of sends against a hung
  delivery. The outbound queue is unbounded (Risks), so that join's open can be
  pending for many `CALL_TIMEOUT`s, and under a queue-sized limit one message on
  it holds every Stoa that long.
- **What 40 s loses.** A message that began waiting on an open more than 40 s
  before the worker asked delivery for it is refused and lost, with the channel's
  messages after it until the ask, even when delivery then answers the open held;
  a queue-sized limit would keep them. That needs delivery to be handing messages
  over while its answers to this peer's calls take long enough to queue past
  40 s.
- **The spec asks for a fixed time.** `op-transport` bounds the wait by "a fixed
  time for each open" and orders it against `CALL_TIMEOUT`; a limit that varies
  with an open's queue position is not one, and adopting it would be a spec
  change.

**So 40 s stays, re-decided on those grounds.** Under a sender who puts a message
on every stuck channel at startup, every fixed value above `CALL_TIMEOUT` gives
the same stall, the last open's settle, because the ask carries each waiting
message past its own call; a longer one buys no smaller stall, and a shorter one
down to `CALL_TIMEOUT` trims nothing there (the figure of 720 s at a 36 s limit,
against 735 s, belonged to the shape before the ask). The value only chooses,
for a message that begins waiting on an open more than the limit before that
open is asked of delivery, between holding every Stoa up longer and refusing
more of the stuck channel's messages. 40 s takes the short hold, and keeps one
stuck channel's cost on that channel instead of spreading it to every Stoa
through the shared queue (Decision 10). A longer value would put every Stoa's
arrivals at risk — discarded once 256 are waiting — to save messages on a channel
whose delivery is already not answering. It is `CALL_TIMEOUT` plus a margin of
5 s: with the ask restarting the time, the margin is what lets a message extended
at the ask outlast a call that runs to its timeout and is then answered a little
late by the IPC. That is a judgement, not a measurement.

**The wait is its own loop, not `Condvar::wait_timeout_while`.** Once the book's
mutex is poisoned — by a contained panic under it — `wait_timeout_while` returns
`Err` at its first wake-up, so the wait would end when *any* open settled and the
message would be refused in exactly the gap this exists to close. The loop takes
the guard back from a poisoned wake-up and ends only on its own condition or the
limit. What breaks without it: `a_poisoned_channel_book_still_waits_for_this_channels_open`
goes red.

**What the wait costs.** While the processor waits on one slow open — up to
`SETTLE_LIMIT` for each start of its time, however many messages arrive on it,
and under twice that for a message the ask extends — messages on every *other*,
open channel wait behind it, and once 256 are waiting, arrivals are discarded for
good.

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
- **Keep the clock per message** — the first version. Rejected above: a sender
  chose the stall by choosing how many messages to send.
- **Start an open's time at the request rather than at the first waiting
  message.** Still rejected. It would bound the K-opens-at-startup total too, but
  an open queued behind others would then often expire before delivery was even
  asked, refusing messages a delivery that was merely slow would have let
  through. The ask restart takes the part of it that helps — a time that covers
  the call — from the ask, where what it covers is one call bounded by
  `CALL_TIMEOUT`, not the queue ahead of it.
- **Narrow the spec's promise instead of restarting at the ask** (the other
  direction the security re-review offered, round 3): keep the time from the first
  message and guarantee only waits that began after the ask. The spec-writer
  rejected it in its callback after that round: it keeps the loss in exactly the
  race this wait exists for.
- **Let every ask extend a waiting message.** Rejected by the once-only cap
  above: asks chained against a hung delivery would postpone one message, and
  every Stoa behind it, for as long as they came.

The stall is bounded by `SETTLE_LIMIT` per start of an open's time and happens
only while this peer is opening a channel, which it does at start, on create and
on join; accepted over a second queue.

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

**The spec now states this recogniser**, so the match is the contract and not only
this design's reading: `stoa-membership`'s "Creating or joining a Stoa opens its
reliable channel" says an answer reports the channel already exists when
delivery's reason contains `channel already exists`, whatever surrounds it and
whether or not it names the channel identifier, and that a reason without those
words — something else "already exists", or "Context already initialized" — is a
decline. It needed no code change. The spec-writer took the looser match on
purpose: requiring the channel id in the reason guards against an answer about
another channel, which a request naming one channel does not receive, and would
decline a genuine answer if delivery ever dropped the trailing id. Both of its new
scenarios are rows in `only_delivery_s_already_exists_answer_opens_a_declined_channel`
at the level of `channel_answer`.

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
the `AlreadyHeld` arm. A delivery pin bump fails
`the_transcribed_revision_is_the_one_delivery_is_locked_at` (Decision 17), whose
message names `channel_lifecycle.nim` among what to re-read.

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

### 17. The content topic has four parts, because delivery parses it

```text
content topic:  /dialectica/1/s-<64 hex chars>/proto     (was /dialectica/1/s/<hex>/proto)
channel id:     /dialectica/1/c/<64 hex chars>           (unchanged)
sender id:      /dialectica/1/p/<64 hex chars>           (unchanged)
```

**Found live, not by a test.** The owner's two-peer check under Basecamp 0.2.3
created the node and then declined every channel, on both peers:
`ChannelCreate failed: failed to subscribe to content topic: invalid format:
generation should be a numeric value`. Every test in this change was green,
because the fake delivery accepted any string as a topic. The rerun on the
build carrying this decision passed (Risks).

**The rule.** `channelCreate` subscribes to the topic before it builds the
channel (`logos_delivery/channels/api/channel_lifecycle.nim:46-48` at
`logos-delivery` `bfdb5afd`, the rev delivery v0.2.1 locks); the subscription
resolves the topic's shard by autosharding, which parses it first.
`NsContentTopic.parse` (`logos_delivery/waku/waku_core/topics/content_topic.nim:60-123`)
takes a leading `/` and then exactly **four** non-empty parts,
`/<application>/<version>/<topic-name>/<encoding>`, or exactly **five**,
`/<generation>/<application>/<version>/<topic-name>/<encoding>`, whose first must
parse as an integer; any other count is refused. A parsed topic then meets
`getShard` (`waku_core/topics/sharding.nim:32-51`, reached through
`subscription_manager.nim`'s `getShardForContentTopic`), which takes generation
`0` or none and refuses any other with `Generation > 0 are not supported yet`.
LIP-23 states the four-part
form (`logos-lips/docs/messaging/informational/draft/23/topics.md:141`) and the
sharding RFC the five-part one (`docs/messaging/core/raw/relay-sharding.md:230-231`).
The old topic split into five, `dialectica` was read as a generation, and the
parse failed. **The channel id and the sender id are not parsed**:
`library/channels_api/channel_api.nim:19-23` wraps each as an opaque
`ChannelId` / `SdsParticipantID` and the manager only keys a table by the first,
so neither format is at risk from this rule.

**It also made the spec's own rationale false until now.** `op-transport` keeps
the `/dialectica/1/` prefix because "autosharding places a topic by hashing only
the application and version segments" — and it does
(`waku_core/topics/sharding.nim:20-30`, `getGenZeroShard`), but only of the
parsed fields. Read as five parts, the old topic's application would have been
`1` and its version `s`. With four, they are `dialectica` and `1`, which is what
the spec says.

**Chosen: keep the `s` discriminant inside the name segment, `s-<hex>`.** It
keeps what the archived `op-transport` design gave the `s` for — a topic says it
names a Stoa, so a later `(stoa, thread)` split has a sibling form, and the
three adjacent strings `channelCreate` takes (`…/c/…`, `…/s-…`, `…/p/…`) are told
apart by eye in a log (Decision 7) — at no cost to the parse.

**Rejected:**

- **`/0/dialectica/1/s/<hex>/proto`** — the five-part form with generation 0,
  which parses and shards the same. `op-transport` requires the topic to *begin
  with the literal* `/dialectica/1/`, so this is a spec change, and it puts a
  generation in a channel identity whose spec forbids one, even a constant.
- **`/dialectica/1/<hex>/proto`**, the bare address as the name. Parses, but
  drops the discriminant for nothing.
- **Changing the channel id to match** (`/dialectica/1/c-<hex>`). Delivery does
  not parse it, so the change would buy nothing and move a second pinned value.

**No migration.** The known-answer pin in `transport.rs` changed, which its own
comment says not to do lightly: two builds disagreeing there partition silently.
Here no build ever had a channel on the old topic — no node accepted it — so
there is no network on it to strand. The archived `op-transport` design
(`2026-09-13-op-transport/design.md`, "The content topic is PLAN.md §4.1's
format") records the old form; this entry supersedes it.

**The guard is the transcription, and it runs in the fake.**
`transport::delivery_topic_rule` (test-only) transcribes both steps with
delivery's messages verbatim: `parse` is `NsContentTopic.parse`, and
`subscribable` is `getShard(ContentTopic)` — the parse, then the generation
refusal. Its own tests hold it in both directions: it refuses the exact topic
the live node refused with the live message (an accept-everything rule — what
the fake was — fails that), it accepts delivery's own `DefaultContentTopic` in
both forms (a refuse-everything rule fails that), and
`a_generation_other_than_zero_parses_and_is_then_refused_for_its_shard` holds
the shard step (it was red, `Ok(Parsed { generation: Some(1), … })`, while
`subscribable` was the parse alone). Two places consult `subscribable`:

- `the_content_topic_is_one_delivery_parses_with_dialectica_as_application`
  asserts each derived topic is accepted, with no generation, `dialectica` as
  application and `1` as version. It failed before the fix with the live
  message. With the refusal test above it is `op-transport`'s scenario "The
  network's content-topic rule reads the content topic as dialectica version 1".
- The fake delivery's `channel_create` declines a topic the rule refuses, in the
  observed decline shape and with delivery's wording, ahead of any scripted
  reply. With the old prefix, every `delivery::` test that opens a channel and
  then relies on it went red; so a topic delivery cannot parse can no longer
  reach the running wiring through a green suite.

*Why the shard step and not a narrower sentence.* The correctness re-review
found `CLAUDE.md` and the rule both accepting `/1/…`, which a live node declines
in `getShard`. Narrowing only the prose would have left the fake more lenient
than the node it models — the defect this decision exists for, moved one
function along. No topic built here has a generation, so nothing shipped was
affected.

What breaks without it: setting `TOPIC_PREFIX` back to `/dialectica/1/s/` turns
the test above, `the_derivation_is_pinned_to_a_known_answer`,
`the_derivation_is_a_pure_function_of_the_address` and those `delivery::` tests
red. Removing the check from the fake leaves only the `transport::` tests to
catch it.

**A pin bump fails a test.** The rule is a transcription at one rev, and a fake
that agrees with an old parser hides a new one's refusal exactly as the
accept-everything fake did. `delivery_topic_rule::DELIVERY_REV` names the rev,
and `the_transcribed_revision_is_the_one_delivery_is_locked_at` reads
`dialectica/flake.lock` with `include_str!` and asserts every `logos-delivery`
node there is locked at it; its message names `content_topic.nim`,
`sharding.nim` and `channel_lifecycle.nim` (Decision 14's wording) as what to
re-read. It also asserts it found at least one such node: without that, a
renamed input would leave the loop checking nothing and the test passing over
any lock. Measured with the match renamed: it fails on that assert. It is
`cfg(test)`, so the `lgx` build never reads the path. Before it, the only prompt
was "a pin bump should re-read" in three places that no command triggered — the
repo's own `.lidl` gap (`ci.yml`) in a second place.

**What it still cannot see.** It sees the lock move; it does not do the re-read.
Whoever bumps the pin can update `DELIVERY_REV` without reading a line, and the
test goes green again. It also cannot see a Basecamp whose delivery module was
built from another rev than this lock's, which is the live check's to find.

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
- **The wait on an open is bounded once per open, not once per message** —
  `op-transport`, "That wait is bounded by a fixed time for each open, not for
  each message" and "An expired wait changes nothing else about the open". Built
  per message first; the security re-review measured the stall a sender could
  choose, and the spec was changed before the code (Decision 11).
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
| `recv()` may block forever on an older SDK | The builder pin (`9f420c2`) stages an SDK whose `recv()` polls the provider's `Abandoned` status every 200 ms and fails once it is reported, so the listener's loop ends and logs "the inbound listener has ended". That needs a runtime with the status channel — one whose `logos_protocol.h` defines `LOGOS_PROTOCOL_HAS_CLIENT_SUBSCRIPTION_STATE` (symbol `lp_client_set_subscription_status_cb`), which is not the same as "0.9": both 0.9 cuts report MINOR 9 and the first lacks it (the header at `4638634`, the rev `dialectica/flake.lock` pins, says so). On a runtime without it `recv()` parks forever — one leaked thread holding nothing another waits on. |
| `RET_STALE_WARN` (3) | Not reachable on this surface, confirmed at v0.2.1: `api_call_handler.h` and the start/stop callbacks return early on it, so each channel call answers once, terminally. This change subscribes to no outcome event, so there is nothing to count twice. `CLAUDE.md`'s entry now says which surface still sees it. |
| Own sends arrive as `channelMessageSent` | Not subscribed to. The own op is already in the log (stored before it is sent), so nothing is lost. `only_reliable_channel_receipts_reach_the_op_log` reads the adapter's subscriptions. |
| The node is a singleton per Logos Core instance | This application asks once per process (Decision 6). delivery refuses a second `createNode` ("Context already initialized"); that is logged as a decline and channels are still requested. |
| `createNode` exactly once; never `stop()` | Once per process by `Delivering`; no `stop` on the seam; the adapter-source test. |
| `messageReceived` timestamps are nanoseconds | Not subscribed to. `channelMessageReceived`'s timestamp (v0.2.1: `currentTimestampNs()` at receipt) is carried in `Arriving` and read by nothing; the window uses the host clock at processing time. |
| Delivery parses the content topic; a fake that takes any string hides it | The topic has four parts (Decision 17); the fake declines what delivery's parse and shard step refuse, and a test fails when the lock moves delivery off the transcribed rev. |
| A decline arrives as `Ok` with an error envelope | `declined` reads `callee_error` first (an empty string is no reason), then `success: false`, then treats a transport `Err` as a decline (Decision 6). |

## Risks / Trade-offs

- [`Edge` depends on the `logos.test` fleet's service nodes] → a desktop module
  stays light; revisit with the owner (Open Questions).
- [A slow delivery delays later sends by up to 35 s each] → replies are unaffected;
  the outbound queue is unbounded, but each entry is caused by a local call from
  the view, so it grows at the user's rate.
- [A slow open stalls inbound processing for every Stoa for up to `SETTLE_LIMIT`
  per start of its time, however many messages arrive on it — under twice that
  for a message this peer's ask extends, in practice `SETTLE_LIMIT` plus
  `CALL_TIMEOUT` — and several opens stuck at once never past the last of them
  settling: at startup against a delivery that answers nothing, (K+1) × 35 s for
  K Stoas, under this limit or any longer one] → only this peer's requests and
  asks start an open's time, so a sender cannot lengthen it, and the peer's own
  memberships bound K. Decision 11 records why a second queue was not worth it,
  why 40 s over a limit sized for the queue, and why an ask extends a waiting
  message only once.
- [An open queued behind others can stay pending longer than `SETTLE_LIMIT`
  before this peer asks delivery for it, so a message that began waiting on it
  that long before the ask is refused and lost, with the channel's messages after
  it until the ask] → needs delivery to hand messages over while not answering
  calls, at start or on a join made behind a slow open. Every message waiting at
  the ask or after it is judged only once the call ends (Decision 11). Decision 11
  says why the limit covers one call and not a queue of them, and why the time is
  per open and started again by the ask.
- [256 is a guess at burst depth] → discards are counted and each is logged with
  the running total, so a bound that is too small is visible in the log.
- [Messages on another application's channels on the shared node are logged as
  `unknown-channel` refusals] → one line each, on hand-over, carrying no channel
  id, and taking no place in the queue (Decision 10).
- [The already-exists match depends on delivery's wording] → Decision 14; a pin
  bump fails a test that names the file to re-read (Decision 17).
- [The fake's content-topic rule is a transcription of delivery's source at one
  rev] → Decision 17; a pin bump fails that test, and the re-read is a person's.
- [The adapter is compiled only by `nix build ./dialectica#lgx`] → it is four
  calls, one subscription and one decoder mapping; the generated names it relies
  on (`create_node_with_timeout`, `on_channel_message_received`,
  `decode_channel_message_received` and its fields) are checked by that build and
  by nothing else. **Two peers exchanging ops was verified live**, by the owner
  under Basecamp 0.2.3 (`lgs basecamp launch alice` / `bob`), on the second
  build. The first created the node and had every channel declined for its
  content topic (Decision 17), the kind of defect only that check sees. On the
  build with the four-part topic, alice and bob each logged `channel open for
  Stoa 1024b9fd…`; the alice→bob op `84aa65c8…` and the bob→alice ops
  `f4f72c6d…` and `d0578af7…` were each `handed to the channel` by the sender
  and `stored inbound op` by the receiver within about 5 s, with no refusal,
  discard or crash. What that run could not show: a post reaches the other
  peer's screen only after the Stoa is re-entered (#194, out of scope here); the
  DELIVERY lamp stays orange, deliberately unbound (#151); and `lgs basecamp
  launch` scrubs `module_data/` on every launch, so the restart-reopen path
  (Decision 14) cannot be exercised live through `lgs`. The run also filed #191,
  #192 and #193.

## Migration Plan

`senders.sqlite` is created on first use; no existing file changes. The op log's
schema-race fix changes how a fresh store is created, not what is written.

## Open Questions

- **Preset and mode.** `logos.test` and `Edge` are recorded above with their
  meaning. Either can change without changing the spec — a constant and its pin —
  but both decide which network dialectica peers meet on and whether they relay,
  so the owner should confirm them.
- **Live verification is done by hand; automating it is follow-up.** #176 says
  "Verification has to be two live peers under `lgs basecamp launch`". Every
  test here drives both sides in one process against a fake delivery, and a live
  run needs a person to click each profile's tile. The owner ran it: the first
  build failed on the content topic (Decision 17), the second passed, and task
  7.3 is ticked on that run (Risks has what it showed and what it could not).
  The restart-reopen path stays unverified live while `lgs basecamp launch`
  scrubs `module_data/`. The automated two-peer test the owner asked for on #176
  is follow-up.
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
  oversized-payload refusal left it for the owner, as `proposal.md`'s seventh
  open question; not decided here.
