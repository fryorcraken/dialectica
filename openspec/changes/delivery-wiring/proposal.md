## Why

No op ever leaves the peer that wrote it (issue #176). A post published on one
peer never reaches a second peer in the same Stoa, which is #102's checklist
item 7 failing every time. The pure half of `op-transport` landed in #55:
`ChannelIdentity::of`, `transport::receive` and `transport::publish`. Its
`tasks.md` §8 left the adapter wiring out on purpose, to "land with the method
that needs it". No later change picked it up. The publish sink in
`dialectica/rust-lib/src/lib.rs` is still a no-op that logs "delivery is not wired
yet". Nothing calls `createNode` or `channelCreate`, and nothing listens for
`channelMessageReceived`.

**Delivery here means the reliable channel and nothing else**:
`channelCreate`, `channelSend` and `channelMessageReceived`, one channel per
Stoa, as `op-transport` requires. A plain `send` on a content topic is out of
scope. It would make item 7 pass while giving up the retransmission and repair
the design relies on.

## What Changes

- **The node.** When the module starts, it asks the delivery module to create
  and start the shared node, at most once per module process. The configuration names the
  reliable-channel layer explicitly. The module never calls `stop()`. If delivery
  declines the creation, the module logs it and keeps serving, and it still
  attempts channel opens: another module in the same context may already have
  created the node. It then does not ask for the node to be started, since a
  node it did not create is not its to start. Startup running a second time in
  one process asks delivery for nothing, channels included.
- **Opening channels.** A successful `createStoa` or `joinStoa` asks for the
  Stoa's channel to be opened, using `ChannelIdentity::of` and a sender
  identifier of this installation's own. At start, the module does the same for
  every Stoa in the membership record. A channel counts as open only once
  delivery reports that it holds it: that it created it, or that the channel
  already exists. The second answer is what delivery gives when a creation it
  stopped waiting on completed anyway, and when this module restarts while
  delivery keeps running; read as a decline, it left the channel shut for as
  long as delivery ran. The create or join reply does not wait for
  the open and does not report on it. A failed open is logged, and is attempted
  again at the next start or the next create or join of that Stoa. A repeated
  open that delivery declines leaves an already-open channel open. A create or
  join answered before startup has wired delivery asks for nothing then; startup
  asks for its channel along with every other Stoa the peer is in.
- **Publishing.** Every successful `publishPost`, `publishReply` or `publishVote`
  hands the op's stored wire form to `channelSend` on the Stoa's channel. This
  replaces the logging no-op. The op is stored before it is sent, as
  `op-transport` already requires. The reply does not wait for the send. A
  declined, failed or unanswered send is logged, and the reply still reports the
  op as published. A publish into a Stoa with no open channel sends nothing,
  opens nothing, and logs the fact. A publish answered before startup has wired
  delivery is logged and never sent, not held for later. Sends go out in the
  order their publishes were answered.
- **Receiving.** A listener passes every `channelMessageReceived` event through
  `transport::receive`. The receive window is judged against this peer's own
  clock at processing time, never against the event's timestamp, so #162's
  one-hour refusal applies on the live path. Each refusal is logged by kind. The
  log never carries the payload, the sender identifier, or a channel identifier
  this peer has not opened. A message on a channel whose open is still
  unanswered is judged once the open settles, not refused in the gap, and that
  wait is bounded by a fixed time even if delivery never answers. A message
  the op log cannot take is logged as a storage failure and not retried. A
  failed subscription is logged, and opens and sends still happen.
- **The inbound bound.** Payloads waiting for the boundary are capped at a fixed
  count. When the cap is reached, the arriving payload is discarded and the
  waiting ones are kept. This **reverses #30**, which discarded the oldest.
  Discards are counted, and each is logged with the running total, in a form
  that cannot be mistaken for a refusal. A message on a channel this peer has
  neither open nor being opened is refused when delivery hands it over and
  takes no place in the queue, so traffic on other applications' channels, or
  on any identifier a sender picks, cannot force discards of a Stoa's ops.
- **The sender identifier** differs between installations, including two holding
  the same identity. It is the same for one installation across restarts. It
  differs between two Stoas. It is made from nothing that is, or is computed
  from, a public key; that is checked by reading the code that makes it, since
  comparing an identifier with a key cannot fail when no key reaches that code.
  If it cannot be retained, the channel is not opened.

### Core API: no new wire methods

`CLAUDE.md` makes any widening of the core API a deliberate decision. This
change adds nothing to the wire surface. #30 added three methods; each is
accounted for here:

- **`joinStoaChannel`: not needed.** A caller should never have to open a channel
  separately from being in a Stoa. If it did, a view could join a Stoa and forget
  to listen, and restarting would silently stop receiving. Opening therefore
  follows from `createStoa`, `joinStoa` and module start.
- **`publishOp`: not needed.** The three existing publish methods already store
  the op and hand it off. `content-authoring` already contracts their replies,
  including the rule that no delivery outcome appears in them.
- **`transportStatus`: not added.** Its purpose was to expose the discard count.
  Here the count goes to the module's log, which is enough for #102's manual
  checks. A status surface for a view belongs with #151 (the DELIVERY lamp),
  which also needs the delivery-outcome state `op-transport` says is still owed.
  Designing one surface there is better than designing half of it here.

The phase-0 probe `deliveryChannelExists` is left as it is.

## Capabilities

### New Capabilities

None. Everything here is the adapter half of `op-transport`, plus the
Stoa-lifecycle obligation that `op-transport` assigns to `stoa-membership`.

### Modified Capabilities

- `op-transport`:
  - MODIFIED *The delivery node is shared and is never stopped by this peer*.
    The prohibitions stay: no site stops the node, and no site creates a node
    of its own. The requirement now adds one site that asks the delivery module
    to create and start its node. That site is reached at most once per
    process, names the channel layer in the configuration, runs before any
    channel operation, does not stop the module when delivery declines, and
    asks for the node's start only after delivery accepted its creation.
    Its scenario keeps its name, *No site in this application stops or creates
    a delivery node*. `openspec` will not archive a MODIFIED block that drops a
    scenario, and under the requirement's own wording ("creating a second one
    is not available") the name is still true. The shutdown and leave-Stoa
    handlers stay owed; this change supplies neither.
  - MODIFIED *A locally-authored op is stored before it is published*. One
    paragraph is added. It says the no-open-channel report is the publish path
    answering the adapter, not the module's reply to its caller, which remains
    `content-authoring`'s: the op is published because it is in the log. Once
    publishing is wired, that is the first place the two capabilities meet, and
    without the paragraph they read as contradictory.
  - ADDED *A published op is handed to its Stoa's reliable channel*.
  - ADDED *Every payload the reliable channel delivers passes the inbound
    boundary*.
  - ADDED *Inbound payloads waiting for the boundary are bounded*.
  - ADDED *The sender identifier this peer supplies is its own, stable, and says
    nothing about its author*.
- `stoa-membership`: `op-transport` says which Stoas get a channel, and when, is
  the Stoa-lifecycle capability's to decide.
  - ADDED *Creating or joining a Stoa opens its reliable channel*.
  - ADDED *Every Stoa the peer is in has its channel opened when the module
    starts*.

The MODIFIED blocks keep the live capability's `SHALL` wording. ADDED
requirements use `MUST`.

## Out of scope, and named as follow-up

- **An automated two-peer end-to-end test**: one peer publishes a post, and the
  other receives it and shows it. The owner asked for this on #176 (comment of
  2026-09-29). It is the first point at which a second peer can prove anything,
  and it automates #102's items 5–7. The by-hand check with two
  `lgs basecamp launch` profiles (`tasks.md` 7.3) has **not yet been run**;
  `design.md`'s Open Questions puts that departure from #176 to the owner.
  **Follow-up, for the project manager to file.**
- **Delivery outcomes.** `op-transport`'s three owed things are the bound, the
  in-flight record, and the never-propagated record. This change does not
  subscribe to `channelMessageSent` or `channelMessageError`. Because nothing
  counts outcomes, `RET_STALE_WARN` cannot cause a double count on this
  surface. That tick belongs to the delivery C ABI, which the module crosses on
  our behalf, and the vendored contract has no event or field that carries it.
  This belongs with #151.
- **Closing channels** on shutdown and on leaving a Stoa. This is the fourth
  thing `op-transport` says is owed, and there is still no site for it.
- **Retrying** a declined send, or a failed channel open, without a restart or
  a repeated create or join. Also retrying an inbound message the op log could
  not store, and sending an op published before startup wired delivery.
- **The node's network configuration**, such as the preset and the mode. It goes
  in `design.md` and is raised with the owner. `op-transport` already excludes it
  from the contract beyond the lifecycle.

## Open questions for the owner

The spec takes a position on each of the first four, and the code on this
branch implements it. Reversing one is still possible, but it is now a spec
change and a code change together, not an edit to this proposal. The fifth is
not in the spec. The sixth was raised in review, after the code landed; the
spec closes part of it and leaves the rest open.

1. **A join or create whose channel fails to open.** The spec has the reply
   succeed unchanged and log the failure. The alternatives are to fail the join,
   which contradicts membership being the user's choice, or to add a reply field
   saying the channel is not open, which widens the API.
2. **How private the sender identifier is.** The spec makes it unlinkable across
   Stoas and unrelated to the author key, so a reader who never posts is not
   identified by it. A single value per installation would be simpler, but it
   would link that reader's presence across every Stoa they are in.
3. **Which payload a full inbound queue discards.** The spec discards the
   arriving payload. #30 discarded the oldest.
4. **Whether re-publishing an op already held sends it again.** The spec says it
   does, which gives a user a retry after a declined send.
5. **The node's preset (`logos.test` or `logos.dev`) and mode (`Edge` or
   `Core`).** #30 used `Edge` on `logos.test`. This decides which network
   dialectica peers meet on, and whether each peer relays traffic for others.
6. **Whether one Stoa's traffic may crowd out another's in the inbound
   queue.** The queue is one fixed-count bound shared by every channel, and a
   discard is final. The spec now keeps traffic on channels this peer is not
   opening out of the queue entirely. It does not stop a peer that floods the
   channel of a Stoa this peer *is* in: channel identifiers are computable and
   Stoas are permissionless, so anyone can post into one, and while that flood
   fills the queue, valid ops arriving in every other Stoa are discarded. The
   options, each keeping memory bounded:
   - **Leave it shared** (the spec as written). Simplest, and a burst in one
     Stoa can use the whole queue; the cost is that flooding any one Stoa the
     victim is in suppresses the victim's receipt of every Stoa.
   - **Give each open channel a fixed share of the one bound.** Floods stay in
     their own Stoa. The cost is burst depth: with many Stoas open, each
     share is small, so an ordinary backlog in one busy Stoa is discarded
     sooner even while the queue is otherwise empty.
   - **When the queue is full, discard the newest payload of whichever channel
     holds the most waiting, the arrival included.** Floods stay in their own
     Stoa and a lone burst keeps the whole queue. For a single busy channel it
     is today's rule (the arrival is discarded); it changes which payload is
     discarded only when another channel is the one holding the most, so it
     amends question 3's answer rather than reversing it.

## Impact

- `dialectica/rust-lib/src/lib.rs`: the publish sink, node creation, channel
  opens, and the inbound listener. Most of the decisions can live in
  `dialectica-core`, behind a seam `cargo test` can drive: the node
  configuration, the order of delivery actions, the queue, the sender
  identifier, and how an event is processed. `cargo test` does not compile the
  adapter, so any property left only in the adapter is untested.
- The `CLAUDE.md` traps apply directly:
  - No dispatch handler, and nothing on the listener path, may unwind.
  - The listener must end and log it when the provider goes away, not block
    forever. Check which SDK revision the builder pin provides before relying
    on that.
  - Delivery can decline a call with `Ok` wrapping an error envelope, so
    `callee_error` has to run before a reply is read as success.
  - Own sends come back as `channelMessageSent`, never as
    `channelMessageReceived`.
- Persistent state gains a retained sender identifier for each Stoa.
- No wire method is added, and no reply shape changes.
