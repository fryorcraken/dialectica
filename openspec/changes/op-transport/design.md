## Context

See `proposal.md` — Why. What shapes the approach, and is not in the proposal:

**The adapter is not compilable by `cargo test`.** `dialectica/rust-lib/src/lib.rs`
is gated on `cfg(logos_scaffold)`, which only the builder sets, because the file
`include!`s a scaffold calling `lp_*` symbols undefined in a test binary. So
`modules().delivery_module.channel_send(..)` cannot appear anywhere a test can
reach, and anything reachable only from that file is untestable. Every existing
handler is split the same way — `wire.rs`'s "the two halves of the delivery bridge
that CAN be tested" is the precedent, written in Phase 0 for exactly this reason.

**The delivery contract is `dialectica/contracts/delivery_module.lidl`**, and it
was read for this change rather than summarised. The four methods and four events
this capability uses:

```text
channelCreate(channelId: tstr, contentTopic: tstr, senderId: tstr) -> result
channelSend(channelId: tstr, payload: bstr) -> result
channelClose(channelId: tstr) -> result
channelExists(channelId: tstr) -> result

channelMessageReceived(channelId: tstr, senderId: tstr, payload: bstr, timestamp: int)
channelMessageSent(channelId: tstr, requestId: tstr, timestamp: int)
channelMessageError(channelId: tstr, requestId: tstr, error: tstr, timestamp: int)
messagePropagated(requestId: tstr, messageHash: tstr, timestamp: int)
```

Two facts about that list decide most of this design. `channelMessageReceived`
carries **no Lamport timestamp and no message id**, which is why every arrival is
`Arrival::unordered()`. And a send's outcome arrives on a **separate event after
the call returns**, which is why publishing cannot report delivery.

**`op-format`'s cap is per-field and explicitly declines to bound a decoded op's
total size**, on the grounds that "a decoder handed a byte slice cannot see the
frame the bytes arrived in". Its doc comment names the transport boundary as the
right home for the total bound. This capability is that frame, and the spec's
requirement "An oversized payload is refused, against a limit pinned at 150 KiB"
is that bound arriving.

## Goals / Non-Goals

**Goals:**

- Channel identity derivable by every peer from the Stoa address alone, pinned
  against silent change by a known-answer test.
- A receive boundary that is a pure function of its inputs and lives entirely in
  `dialectica-core`, so every refusal is testable without a node.
- Every refusal in the spec's enumerated list reported as a distinct value, not a
  string a caller has to match on.
- No panic reachable from any byte string of any length.

**Non-Goals, beyond the spec's own scope exclusions:**

- **No core API method.** The proposal excludes "any core API method shape", so
  this change adds no `DialecticaModule` trait method and no JSON handler. What it
  adds is the typed boundary those methods will call. The adapter wiring lands
  with the method that needs it.
- **No `senderId` derivation.** §5.2's per-Stoa identity supplies it and is out of
  the MVP (§9.2); the boundary takes it as a parameter and stores it nowhere.
- **No node configuration** — which network, which mode, which entry nodes.

## Decisions

### The channel identity is one type with two fields, and neither is a `String` the caller assembles

`ChannelIdentity { channel_id, content_topic }`, returned by one function of one
`&Address`. The alternative was two free functions, `channel_id(&Address)` and
`content_topic(&Address)`.

One type, because the spec's requirement is that both are pure functions of the
same input and that **nothing else participates**. Two functions can each be
called with a different address; one constructor taking one address cannot. It
also means "derive the channel identity" is a single act with a single call site
per caller, so there is no second site that could derive one half from something
else.

The fields are private with accessors, so a caller cannot construct a
`ChannelIdentity` from hand-assembled strings and hand it to a send. The only
route to one is the derivation. That is the requirement "Channel identity is a
pure function of the Stoa address" made structural rather than checked — and the
spec says why it has to be structural: *"its violation produces no error"*.

### The content topic is PLAN.md §4.1's format, and the channel id is a distinct prefix over the same address

```text
content topic:  /dialectica/1/s/<64 hex chars>/proto
channel id:     /dialectica/1/c/<64 hex chars>
```

The content topic form is §4.1's verbatim (`/dialectica/1/s/<hex>/proto`), which
matters because §4.2 establishes that autosharding hashes only
`application` + `version` — so the `/dialectica/1/` prefix is what places every
dialectica topic on one shard, and inventing a different prefix here would be a
routing change disguised as a naming choice.

The channel id carries a **different discriminant byte in the same position**
(`c` against `s`) rather than being the bare hex or a copy of the topic. Three
reasons:

1. **They are not the same namespace.** The topic is disclosed to filtering,
   storage and forwarding peers; the channel id is an application-chosen
   rendezvous string. Giving them one value would make a later change to either
   a change to both.
2. **A bare hex address as a channel id is indistinguishable from anything else
   32 bytes long.** Two applications sharing one node would collide.
3. **§4.5's deferred split is `(stoa, thread)`.** A channel id with a structured
   prefix has somewhere to put a thread segment; a bare hex string does not.

**No epoch, no counter, no generation, in either.** The spec forbids it and gives
the argument; what is worth recording here is that a *deterministic* epoch was
considered and rejected too, and PLAN.md §4.3 has the reasoning: it is "a
rendezvous problem at each boundary — a value that changes must change for
everyone at once", carried forever to route around a defect that belongs
upstream. The withdrawn `#4116` workaround is the instance.

**Hex, lowercase, via `Address::to_hex`.** Not base64 or base32: the address is
already displayed as hex everywhere in this crate (`Address::to_hex`,
`OpId::to_hex`), and a second encoding for the same bytes is a second thing to
get wrong. Case matters because the string is compared byte-for-byte by peers;
`to_hex` is lowercase and is the only producer.

### Open channels are a map from channel id to Stoa address, and that shape is the Stoa-mismatch check

```rust
pub struct OpenChannels { by_id: HashMap<String, Address> }
```

The receive boundary needs two answers: *is this channel open?* and *which Stoa
is it for?* A `HashSet<String>` answers the first and not the second, which would
leave the Stoa-mismatch refusal with nothing to compare against — and that
refusal is the spec's whole answer to an op copied onto another Stoa's channel.

A map keyed by channel id rather than by address, because the inbound event
supplies a channel id and nothing else. Keying by address would mean re-deriving
every open channel's identity per message to find which one matched, which is a
scan where a lookup will do, and a scan is where "compares only a prefix" bugs
live.

**`open()` takes a `ChannelIdentity` and stores the address it was derived from.**
So the map's invariant — that a channel id maps to the Stoa it was derived for —
holds by construction: there is no way to insert a pair the derivation did not
produce. The alternative, `open(channel_id: String, stoa: Address)`, lets a caller
register a mismatched pair, and every later Stoa check would then be comparing
against a lie.

### One `Refusal` enum, and each variant is a spec bullet

The spec requires five refusals "each reported distinguishably from the others",
and gives the reason: five different causes, five different responses. So the
boundary returns `Result<Admitted, Refusal>` with one variant per cause, in the
`stoa.rs`/`op.rs` house style — an enum with a `Display` that renders without
Rust syntax, and a test asserting pairwise-distinct rendering.

`Refusal::Undecodable(OpError)` **carries** the decoder's own error rather than
flattening it to a string. `op-format` already distinguishes eleven ways a byte
string is not an op, and collapsing them here would discard the distinction one
layer after the code that made it — the same "a boundary reporting only 'invalid'
sends the reader looking in the wrong place" the spec argues for at this level.

### The size check runs before the decode, and the constant is `MAX_MESSAGE_BYTES = 150 * 1024`

The spec requires the check apply "to the payload as received, before it is
decoded", and the ordering is not cosmetic: a decoder handed 4 MiB does work
proportional to the input before refusing, and this is the one boundary where the
input size is an attacker's free parameter.

`op.rs` already has `MAX_FIELD_LEN = 150 * 1024` for the same network limit, and
this is deliberately **a second constant rather than a re-export**. They are two
different bounds that happen to share a value today: one bounds a single field
inside an op, the other bounds a whole message. `op.rs` says so explicitly — its
cap "does NOT bound the total size of a decoded op, and must not be read as doing
so". Aliasing them would make a later change to either silently change the other,
and the spec pins this one's value against drift on its own terms.

### Publish stores first, then hands the bytes out — and the store failure is the only one this function can have

The spec's ordering requirement, implemented as: append to the log; if that fails,
return `PublishError::NotStored` and hand out nothing; otherwise return a
`Publishable` carrying `op.to_bytes()` for the caller to send.

**The send is not in this function**, which is the shape rather than an omission.
`publish` returns the bytes and the channel to send them on; whoever performs the
send is the only place a handoff failure exists. So there is no branch here that
could undo the append, and the spec's "a send failure does not remove the op" is
structural — the scenario "There is no route by which a handoff failure could
unpublish the op" is what records that, and it is checkable by reading the
function rather than by driving it.

That also means an earlier draft of this section was wrong in a way worth naming:
it said "if *that* fails, report the failure and leave the op stored", describing
a send this function does not perform. The reconciled spec forbids reporting a
handoff failure as a failed publish at all, and `content-authoring` owns the reply
that reports it.

The ordering asymmetry is still the requirement. Publishing first loses the op
whenever the store fails after a successful send — the op is on the network, other
peers hold it, and its author does not. Sending first is therefore never correct.

**`PublishError` has two variants because the two failures call for opposite
responses**, and the spec requires a publish with no open channel be
"distinguishable from a transport failure on an open channel":

- `NoChannel { stoa, id }` — **the op exists** and did not go out. Do not retry,
  do not discard.
- `NotStored(OpLogError)` — **the op does not exist**. It carries the store's own
  error rather than flattening it, for the same reason `Refusal::Undecodable`
  does: the layer below already distinguished the causes.

A boolean could not carry that, and neither could one variant. **Nothing reached
`NotStored` from a test until `AppendFailsLog` existed** — `MemoryOpLog::append`
cannot fail — so the distinction was contracted, implemented, and unexercised;
`tester`'s finding measured it and the fixture closes it.

**The op is stored even when there is no channel**, which reads as odd and is the
spec's explicit instruction: *"The op SHALL still be stored, on the same
reasoning: the author authored it."*

### The arrival is `Arrival::unordered()`, constructed at this boundary and nowhere else

`arrival.rs` already named this function's purpose — "**What the boundary
constructs today**, because `channelMessageReceived(channelId, senderId, payload,
timestamp)` supplies neither field. Named rather than reached by passing two
`None`s, so that grepping for it finds every place the contract's gap is being
absorbed." This change is the boundary that comment was written for.

The `timestamp` parameter is accepted and dropped. It is in the signature because
the event carries it and a boundary that did not take it would be hiding the
field rather than refusing it; it reaches nothing, and the test
`the_timestamp_reaches_nothing_that_is_recorded` is what holds that.

**Not `from_parts(None, None)`.** Reaching the same value the general way would
make the absence look like a recording of what arrived, where `unordered()` is a
named statement that the transport supplied nothing. The greppability is the
point.

**Do not re-open the question of where the ordering metadata went.** The
`op-ordering` change proved it unreachable by any other route, and the four
exhausted routes are recorded in its `design.md` so that this boundary does not
spend the afternoon again: no other event carries an order
(`channelMessageSent`/`Error` carry a `requestId`); the contract has no
channel-state or log-read verb; SDS's fields wrap our payload rather than sitting
inside it, so `channelSend`'s `{payload, ephemeral}` envelope round-trips exactly
what we put in; and `storeQuery` returns transport content hashes, not Lamport
values, from an API marked use-at-your-own-risk. The loss point is one layer below
the delivery module — the Reliable Channel API's `MessageReceivedEvent` has
exactly one field, the reassembled payload — so the delivery module cannot forward
what it was never given. The upstream gap is recorded for the owner to file,
against both layers, and is not blocking.

**And the `timestamp` trap specifically.** It is measured, not inferred:
`delivery_module_plugin.cpp` assigns it `currentTimestampNs()`, a `CLOCK_REALTIME`
read taken when the receiving peer's callback fired. `op-ordering`'s design ranks
it as a *worse* ordering candidate than raw arrival sequence, "because it is
per-peer while appearing shared". The sibling `message_received` branch does read
the message's own timestamp from the event JSON, which is what makes the two look
like the same kind of value; the channel branch does not.

### The delivery-outcome obligation is out of scope here, and this is the seam it attaches to

The owner's decision that a publish reports success once the op is in the log
makes a failed delivery something that must surface somewhere — otherwise a loud
failure becomes a silent one. `channelMessageError` and `messagePropagated` are
the events that carry the facts.

**The spec now contracts the obligation.** The requirement "A successful publish
is a statement about the local log and nothing more" states it, and names the
three things owed: the bound, what a peer records for an op in flight, and what it
records for one that never propagated. It also says none of them is discharged
here. That replaced the `// NO SPEC:` marker this section used to point at, so the
behaviour a publish *does* have is specified rather than chosen, and the gap is a
tracked one.

**Not built here, and here is the seam.** Both events are asynchronous and arrive
keyed by `requestId` — the value `channelSend` returns — so an outcome tracker
needs three things this path does not have: a map from `requestId` to the op it
sent, state holding that map across calls, and a clock to bound the wait. Those
make it a component beside this boundary rather than a branch within it, which is
a scoping decision and not an impossibility. What it attaches to:

- **`Publishable` carries the `id` and the `channel_id`**, which is exactly the
  pair a tracker needs to key an outcome back to an op: whoever performs the send
  holds the `requestId` the send returned and the `Publishable` it sent, so the
  association is available at the one call site that has both. Nothing further up
  has to re-derive anything.
- **`publish` returns rather than sends**, so the tracker sits at the caller and
  needs no change here. Adding one would mean giving this function a clock and a
  mutable map, which is what makes it a component rather than a branch.
- **Nothing promises delivery.** No field of `Publishable` carries an outcome, so
  a tracker adds a fact rather than correcting a claim.

**The one thing genuinely ruled out** is that a *publish's reply* could carry the
answer, and that is a consequence of the event timing rather than of this
design: the outcome arrives after the call has returned. A later capability
reports it on its own surface, not by widening this one's return type.

This crate has deliberately never held a clock, which is the part of the above
worth checking before building it: the tracker is the first thing here that needs
one, and where that clock lives is its decision to record, not this one's.

**The rendering obligation went to `docs/UI-BRIEF.md` now rather than waiting**,
as its obligation 7: a successful publish means "saved here", not "posted". PLAN
§9.2 had said the brief would need this once the three owed things were answered,
and that deferral was the wrong half to act on. The brief is designed against by
someone who cannot read the code, so a brief silent on the point leaves a designer
free to render success as *sent* — which is the failure the requirement exists to
prevent, reintroduced at the only layer a user sees.

What is splittable here is that the obligation has two halves with different
readiness. The **prohibition** is true today and complete: do not say sent,
delivered or posted, and *do not design an in-flight state*, because no call
produces the signal a spinner would wait on and a spinner that cannot resolve is
worse than none. The **positive** half — what a view shows for an op in flight
versus one that never propagated — needs the three owed things first, and is
additive to the prohibition rather than a replacement for it. Writing only the
first half is what let this land without inventing behaviour.

### What is left in the adapter, and why it is three lines

The gated adapter gets no logic. Per handler: derive or look up, call the one
`modules().delivery_module.*` method, hand the reply to a `core` function that
decides what it means. `wire.rs`'s `channel_exists_reply` and `callee_error` are
the existing instances of that last step and are reused rather than re-written —
`callee_error` already handles delivery's `{"error":…,"success":false,"value":null}`
envelope, which was observed live and is not in the contract's type.

**Nothing in the adapter is compiled by `cargo test`**, so any property that
holds only there is untested by definition. Said plainly rather than covered by a
test that does not reach it.

## Risks / Trade-offs

**[Two peers derive different channel identities and never meet]** → The
derivation is a pure function with no local input, so there is no value that
could differ. Pinned by a hardcoded known-answer test, derived independently of
the implementation, in the `identity.rs`/`stoa.rs` house style: if it fails, do
not update the expected value.

**[A hostile payload panics and aborts the module process]** → Every indexing,
slicing and arithmetic operation on the receive path was hunted. The size check
uses `len() >`, the decode is `op-format`'s (already fuzzed by its own suite over
every prefix of a valid op), and the boundary has no arithmetic. Pinned by a test
over empty, single-byte, single-byte-mutated, and over-long payloads, plus
non-UTF-8 channel and sender identifiers.

**[`Op::canonical_bytes` is infallible while `Op::decode` refuses a field over
153,600 bytes]** → A known `main` bug: a 153,601-byte body appends `Ok(Stored)`
and then bricks every read on that store. **This boundary does not widen it, and
partly closes it on the inbound side**: a payload over 150 KiB is refused before
it is decoded, and an op whose fields sum past what decodes is refused by the
decoder rather than stored. The remaining exposure is a *locally authored* op
that encodes and does not decode, which reaches the log through the publish path
without passing a decoder. Not this change's to fix — the fix is making
`canonical_bytes` fallible, in `op-format` — and named here so it is not
rediscovered as a transport bug. Reported.

**[The size bound and `op-format`'s field cap drift apart]** → They are separate
constants on purpose (see Decisions), so drift is possible by design. Each is
pinned to its own hardcoded value, and this one's spec requirement is that it
equal the transport's stated limit.

**[`OpenChannels` is in-memory, so a restart forgets which channels were open]**
→ Correct and intended: the spec's lifecycle requirement has every channel closed
on shutdown, so there is nothing to remember. Which Stoas a peer *holds* is the
Stoa-lifecycle capability's, and re-opening on start is that capability's job.

**[A refusal partially applies]** → Nothing is written before every check has
passed: the boundary is a sequence of guards over borrowed data, and the single
`log.append` is the last statement. There is no intermediate mutation to roll
back, which is CLAUDE.md's "keep handler bodies free of partial mutation" rather
than a rollback path.

## Open Questions

None that would change the specs, the approach or the tasks. The delivery-outcome
obligation is routed to `spec-writer` as unspecified behaviour rather than left
open here, because it is a decision about observable behaviour and belongs in the
spec.
