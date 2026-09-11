# Design: op ordering, and where SDS's order actually stops

## The finding, which is most of this change

PLAN.md §13 framed the gap as a missing LIDL field. The investigation found the
metadata is lost **one layer lower than the LIDL contract**, and that nothing in
any implementation available to read carries it to an application. The change
therefore designs no ordering algorithm; it records what a peer keeps and what it
does in the meantime.

### What SDS actually maintains

LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`. The `Message` protobuf:

| Field | Line | What it is |
|---|---|---|
| `message_id` | `sds.md:114` | Unique id of the message |
| `lamport_timestamp` (optional uint64) | `sds.md:116` | "Logical timestamp for causal ordering in channel" |
| `causal_history` (repeated HistoryEntry) | `sds.md:117` | Preceding message ids this depends on; "generally 2 or 3" |
| `bloom_filter` (optional bytes) | `sds.md:118` | Received message ids in channel |
| `repair_request` | `sds.md:120` | SDS-Repair extension only |

Per participant, `sds.md:150-157`: a Lamport timestamp per channel — initialised to
epoch-ms so new joiners order correctly without syncing history (`sds.md:191`) — and
a bloom filter of received ids.

**SDS's own conflict rule is already §5.7's rule.** On delivery the participant
"MUST insert the message ID into its local log, based on Lamport timestamp"
(`sds.md:237-238`), and ties are ordered "in ascending order of message ID"
(`sds.md:246-247`). §5.7 saying "nothing new is invented" is exactly right — the
rule is not the problem, its input is.

### Where the metadata is lost

Not in the delivery module. It is already gone at the Reliable Channel API — the
spec the delivery module consumes — `logos-lips/docs/messaging/application/raw/reliable-channel-api.md`.

Its incoming path (lines 180-185) is: decrypt, apply SDS, then "**Reassemble**: Once
all segments for a message have been received, reassemble and emit a
`reliable:message:received` event." And that event, lines 309-315, is:

```
MessageReceivedEvent:
  description: "Event emitted when a complete message has been received and reassembled."
  fields:
    message:
      type: array<byte>
      description: "The reassembled message payload."
```

**One field.** No `lamport_timestamp`, no `message_id`, no `causal_history`. The
spec is explicit elsewhere (line 123) that the timestamp it does wrap "acts only as
a uniqueness salt; ordering is provided by the SDS Lamport timestamp" — so it knows
which value orders, and does not pass it on.

### And the delivery module drops even the timestamp it forwards

`logos-delivery-module` at tag `v0.2.1`, `src/delivery_module_plugin.cpp:207-215`:

```cpp
} else if (eventType == "channel_message_received") {
    std::vector<uint8_t> payloadBytes;
    if (jsonObj.contains("payload")) {
        payloadBytes = decodeBase64Payload(jsonObj["payload"]);
    }
    impl->channelMessageReceived(
        jsonObj.value("channelId", ""),
        jsonObj.value("senderId", ""),
        payloadBytes,
        timestamp);
```

where `timestamp` was assigned at `cpp:158` as `currentTimestampNs()`, defined at
`cpp:44-48` as a `CLOCK_REALTIME` read.

So `channelMessageReceived`'s `timestamp` is **this peer's wall clock at the moment
its C++ callback fired**. It is not the sender's clock, not a Lamport value, and not
even the SDS message's own timestamp. Two peers receiving the same op record
different values; one peer's values move with NTP and with scheduling. The
`message_received` branch (`cpp:180-190`) does read `msgObj.value("timestamp", 0.0)`
from the event JSON — which is why §11 records that units differ per event — but the
channel branch does not.

This matters for the write-up below: it means `timestamp` is not a weaker Lamport
value that might be pressed into service. It is unusable for ordering, and a design
that leaned on it would be reading a local clock while believing it read a shared one.

### It is not reachable by any other route

- **No other event.** Full list at `delivery_module.lidl:23-32` and `cpp:82-92`.
  `channelMessageSent` / `channelMessageError` carry a `requestId`, not an order.
- **No query.** `delivery_module.lidl:7-21` has no channel-state or log-read verb;
  `channelExists` answers "true"/"false".
- **Not inside the payload.** `channelSend` wraps `{payload: base64, ephemeral:
  false}` (`cpp:504-508`) and receive base64-decodes `payload` back. SDS's fields
  wrap our payload; they are not inside it. What we put in is what comes out.
- **`storeQuery` is the wrong tier.** It returns `{messageHash, message,
  pubsubTopic}` per message (`lidl:13`) — transport content hashes, not SDS Lamport
  values — and is marked "USE AT YOUR OWN RISK", subject to change without a
  deprecation cycle.

### The transport below has not integrated SDS yet either

`/home/fryorcraken/src/logos-messaging/logos-delivery` (the Nim fork) contains no
SDS. The only trace is an unmerged branch `origin/add/nim-sds-wiring` (`6eabb855`,
"chore: vendor nim-sds (no runtime integration yet)") touching `nimble.lock` and
`waku.nimble` only. Its receive path
(`waku/node/delivery_service/recv_service/recv_service.nim:73-86`) hashes, dedups
and emits immediately with no causal gate; its `RecvMessage` (lines 32-35) holds
`msgHash` and `rxTime` and nothing else. The chokepoint event
(`waku/events/message_events.nim:24-28`) is `{messageHash, message}`.

The JS reference client has the same shape: `docs.waku.org/docs/build/javascript/
reliable-channels.md:139-147` hands the `message-received` listener a `wakuMessage`
and expects the application to decode its own protobuf out of `payload`; a message
id comes back on **send** only (line 222), and receive-side ordering surfaces only
as aggregate `{received, missing, lost}` counters (lines 160-193).

So per-message Lamport/id is internal to SDS by design in every implementation
available to read. This is a contract question at three layers, not an oversight at
one — which is why the write-up below names two of them and why filing it is the
owner's call rather than a patch we could send.

## Decisions

### Decision: This is outcome (b), and no application-level ordering is designed

The metadata exists, is maintained, and is the documented basis of ordering; it does
not reach us. The brief's outcome (c) — designing a substitute — is therefore
declined, and declining it is the decision.

A dialectica-side Lamport clock would be the single worst thing to build here, for a
reason stronger than duplication: **it cannot be made to agree with SDS's.** SDS's
clock advances on messages we never see individually (acks, sync messages,
ephemeral traffic, `sds.md:297-322`) and is initialised from epoch-ms
(`sds.md:191`). A clock advanced only on op arrivals runs behind it, diverges
per-peer according to what each peer received and when, and produces an order that
looks authoritative and is not. When SDS's order does become reachable, the two
would disagree — and §5.7 would have two answers with nothing to choose between them.

What is built instead is the seam: a type recording what the transport said, and a
comparison over it. When the fields arrive, they are populated at the boundary and
nothing downstream changes.

### Decision: `Arrival` holds two `Option`s, and that shape IS the answer

The type could have been `{lamport: u64, message_id: [u8; 32]}` with sentinel values
for "not supplied". It is `{lamport: Option<u64>, message_id: Option<MessageId>}`
instead, and this is CLAUDE.md's "put the complexity in the data structure, not the
logic" applied to the exact case it describes.

A sentinel (`lamport: 0`) makes "did the transport order this?" a question every
call site must remember to ask, and answer the same way. The `Option` makes it
unaskable-incorrectly: a caller cannot read the Lamport value without confronting
its absence, because the compiler will not let them. The degraded path is not a
branch that might be forgotten at the fourth call site; it is the only way to get at
the value.

This is also what makes the spec's "absence is distinguishable from a real value"
requirement structural rather than documented. `Arrival::is_ordered_by_transport()`
exists so that a caller can *state* the question, but the type already guarantees
they cannot skip it.

### Decision: The degraded order is by op id, and it is named a degraded order

Three candidates for ordering ops the transport did not order:

1. **Arrival sequence** — rejected. It is per-peer by construction, which is
   precisely the silent divergence this whole change exists to avoid. Two peers that
   received the same ops in different network orders would render a thread
   differently, with no error.
2. **The delivery module's `timestamp`** — rejected, and this is the trap worth
   recording. It *looks* like the obvious fallback: it is right there in the event
   signature. But it is a local `CLOCK_REALTIME` read (`cpp:44-48, 158`), so it is
   arrival sequence wearing a timestamp's clothes — worse than candidate 1, because
   it is per-peer while appearing shared.
3. **Op id** — chosen. It is a function of the op's own bytes, so every peer holding
   the op computes the same one from the op alone, consulting nothing it received.

Op id ordering is arbitrary with respect to time, and that is accepted, because the
property being bought is *convergence*, not accuracy. Two peers agreeing on a wrong
order is a forum that renders consistently; two peers disagreeing is a forum that
cannot be reasoned about. The spec says this in as many words so nobody later
"improves" it into candidate 1 or 2.

**Unordered ops sort below ordered ones** rather than above: an op the transport
placed is better evidence than an op it did not, and the current version of a post
should not be displaced by one about which nothing is known.

### Decision: A message id without a Lamport timestamp does not order

Since the fields are independent `Option`s, `(None, Some(id))` is representable and
something must be said about it. It is treated as unordered.

The temptation is to order by message id alone — it is total, stable, and every peer
computes the same result. It is also meaningless: the id is a keccak-256 hash
(`reliable-channel-api.md:121`), and the spec is explicit that it is a tiebreak
*within* a Lamport value. Ordering by it alone yields an arbitrary order
indistinguishable in shape from a real one, which is worse than an arbitrary order
that announces itself.

Note this case is unreachable through the contract we have, which supplies neither
field. It is specified because the type permits it and an unspecified corner of a
type is where the next reader's assumption goes.

### Decision: the order is total over op ids, and callers must dedup

`cmp_ops` falls back to the op id in every branch, so once two records share an
op id there is nothing left to separate them and they compare `Equal` — even
when their `Arrival`s differ. Reachable two ways: `(None, None)` with the same
op id, and the ordered branch with equal Lamport and equal message id.

This is **not** fixed by adding another tiebreak, because there is nothing left
to tie-break on that is not per-peer. Comparing the `Arrival`s themselves would
mean ordering by transport metadata that was explicitly declared insufficient to
order (or, in the `(None, None)` case, by nothing at all). The honest resolution
is a precondition: ops are idempotent by op id (§3.1), so a store holds one
record per op id and the case does not arise.

It is stated in the spec rather than left in a doc comment because it is a
contract on **callers**, and `cmp_ops` is public. The store is being built now,
by another agent, and is exactly the caller that could assemble a list before
deduplicating. The failure would be silent in the usual way: a stability-
dependent order, differing per peer, with no assertion anywhere.

### Why the order is transitive

Worth recording because it is the law a mixed comparator most typically breaks,
and because the argument explains why this one does not.

`is_ordered_by_transport` partitions any population into two blocks. Every
transport-ordered op precedes every unordered one; each block is independently
totally ordered. **A partition into two totally-ordered blocks with a uniform
rule between them is transitive by construction** — there is no boundary for a
mixed comparator to break on, because the boundary rule consults nothing but
presence. `cmp_tiebreak`'s has-id/no-id split within one Lamport value is the
same argument one level down.

The load-bearing word is *uniform*. A boundary rule that consulted a value —
"an op with Lamport 2 loses to an unordered op" — breaks transitivity
immediately, and that is the mutation the exhaustive test was verified against
(mutation 8 in `tasks.md`). The reasoning is in the code, but reasoning is not a
gate, which is why the test exists.

## How the two-clocks trap is avoided

The brief asks this be stated explicitly, and it is the load-bearing paragraph.

Two orders that can disagree, disagreeing invisibly, is prevented **by there being
only one order**. Concretely:

1. **No second clock exists to disagree.** `Arrival` has no constructor that derives
   a Lamport value; it only records one. There is no `Arrival::next()`, no counter,
   no peer state. The only way a Lamport value enters the system is for the
   transport to have supplied it.
2. **The comparison consults nothing local.** `Arrival::cmp_ops` is a pure function
   of two `(Arrival, OpId)` pairs. It reads no clock, no arrival counter, no
   ambient state. Two peers holding the same inputs cannot produce different
   outputs, and `arrival.rs`'s tests assert this against hardcoded expectations
   rather than against a second run of the implementation.
3. **The degraded case is also transport-independent.** Op id is derived from the
   op's bytes, so even the fallback cannot diverge between peers.
4. **The absent case is visible.** Because it is `Option`, a peer can report "this
   thread is ordered by op id, not by the network" rather than silently presenting a
   degraded order as an authoritative one. The failure mode §13 warns about is
   invisibility; this is what makes it visible.

The property that remains untested and cannot be tested here is agreement with the
*real* SDS order, because no SDS order reaches this code. That is stated in
`tasks.md` as what the green gate structurally cannot see.

## What the LIDL contract must add, for the owner to file

Recorded here rather than filed, per the brief. Two layers need it, and the lower
one is the one that matters — patching only the upper leaves nothing to forward.

**1. `logos-lips` — Reliable Channel API (`reliable-channel-api.md`), `MessageReceivedEvent`, lines 309-315.**

This is the root change. The event currently has one field; it needs the SDS values
the layer already holds:

```
MessageReceivedEvent:
  fields:
    message:            array<byte>   # existing
    lamportTimestamp:   uint64        # SDS Message.lamport_timestamp (sds.md:116)
    messageId:          string        # SDS Message.message_id (sds.md:114), hex
```

Both are already computed and already the documented basis of ordering
(`reliable-channel-api.md:123`). `lamportTimestamp` should be optional in the same
sense SDS makes it optional — ephemeral messages send it unset (`sds.md:321-322`) —
so the event must be able to express "not set" rather than defaulting to 0.

**2. `logos-delivery-module` — `delivery_module.lidl:28` and `src/delivery_module_plugin.cpp:207-215`.**

Once (1) lands, forward it:

```
event channelMessageReceived(channelId: tstr, senderId: tstr, payload: bstr,
                             timestamp: int, lamportTimestamp: int, messageId: tstr)
```

with `cpp:207-215` reading `jsonObj.value("lamportTimestamp", ...)` and
`jsonObj.value("messageId", "")` from the event JSON instead of dropping them.

**A third item worth filing separately, and arguably first**, because it is a bug
rather than a missing feature: `channelMessageReceived`'s existing `timestamp` is a
local `CLOCK_REALTIME` read (`cpp:158`, `cpp:44-48`) while the sibling
`message_received` branch (`cpp:180-190`) forwards the message's own timestamp. A
consumer reasonably reads the two as the same kind of value. Either forward the
message's timestamp or rename the parameter; as it stands it silently reports
receive-time as though it were message-time.

**Not blocking us today.** Until (1) and (2) land, `Arrival::unordered()` is what
the boundary constructs, the degraded order applies, and a peer can say so.

## What this change deliberately does not build

- **The store / op log.** A later change, and another agent's. This type is what it
  will key on; building the store here would bake an ordering decision into a schema
  as a side effect, which is what landing the type separately prevents.
- **The revision and moderation resolvers.** They consume this order. §5.7's rule
  needs the target op to check authorship and the moderator set to check authority —
  both store questions.
- **Any change to the op format.** `an_op_carries_no_ordering_fields` still passes
  untouched; this change is the other half of the sentence in its comment.
- **The boundary wiring** that constructs an `Arrival` from a `channelMessageReceived`
  event. There is nothing to construct it from beyond `Arrival::unordered()`, and a
  decode path with no fields to decode is better written when the fields exist.
