## Purpose

Defines how a Stoa's signed ops move between peers: how the channel every peer in a Stoa must find is identified, what publishing a locally-authored op means, what an inbound payload must survive before it reaches storage, and what the transport tells a receiving peer that the peer must not believe.

The boundary with three neighbouring capabilities is drawn deliberately, and the substance is not restated here.

`op-format` owns what an op is, how it encodes, and what its decoder refuses. This capability requires that an inbound payload be put through that decoder before anything else looks at it, and does not repeat the decoder's own refusal list.

`identity` owns authenticity — whether an op is from the author it claims, and the binding of the presented key to that claimed author. This capability requires that check on the receive path and names nothing about how it works.

`op-ordering` owns what orders two ops, including the degraded order that applies when the transport supplied no ordering metadata. This capability contracts only what a receiving peer records about an arrival, and that this transport supplies nothing to record.

`op-log` owns what happens once an op is appended — that the log stores it, decides nothing about it, holds one entry per op identifier, and does not verify a signature on append. **That is not in tension with this capability's refusals, and the boundary is worth stating because it reads as though it were.** `op-log` contracts what the log does with an op it is handed; this capability contracts which inbound payloads become an op that is handed to it. A log that refused on append would be deciding validity at a point where the peer's knowledge is incomplete, which is why it does not; a transport boundary that appended whatever arrived would be storing forgeries on behalf of any peer that sent them, which is why it does not. Nothing here requires the log to check anything, and nothing in `op-log` licenses this boundary to skip a check.

**What is outside this capability**, named rather than covered because most of it needs a running delivery node and two live peers to observe, and a requirement nothing can check is worse than an acknowledged gap:

- **That two peers deriving one Stoa's channel identity actually meet.** The derivation being a pure function of the Stoa address is contracted below and is checkable; that two processes on a network consequently exchange ops is a property of the node.
- **That a published op reaches another peer** — retransmission of an unacknowledged message, recovery of messages missed while offline, and how far back a repairing peer reaches. That is the transport's own reliability, and none of it is observable from this capability's surface.
- **That closing a channel released anything on the network.** The release is reference-counted across channels on a content topic and its failures are not surfaced to the caller, so a peer cannot observe the outcome. What is contracted below is that a peer does not claim otherwise.
- **Whether a hashed content topic buys the anonymity intended.** That a Stoa's title does not appear in a topic is checkable; the size of the set a hashed topic hides a peer within is a property of the deployed network.
- **The node's configuration** — which network, which mode, which entry nodes — beyond the lifecycle contracted below.
- **Attachments and any content addressed outside an op.** A payload here is one op's wire form.
- **What a channel is opened for.** Which Stoas a peer holds, what joining one means, and where a Stoa's genesis record is stored belong to the Stoa-lifecycle capability. This capability contracts only that an inbound op never causes a peer to hold a Stoa it did not already hold.

## ADDED Requirements

### Requirement: A Stoa's ops travel on one reliable channel per Stoa

A Stoa's ops SHALL travel on exactly one reliable channel, and that channel SHALL carry nothing but signed ops of that Stoa.

The channel SHALL be identified by a channel identifier and a content topic, both derived from the Stoa's address. The channel identifier names the conversation every peer in the Stoa must join; the content topic names what the underlying network filters on.

A thread identifier, a parent op identifier, an author, or any other property of an individual op SHALL NOT appear in either the channel identifier or the content topic. Every such value is already inside the signed op.

The content topic SHALL NOT contain a Stoa's human-readable title or any other human-readable name. A content topic is disclosed to peers that serve filtering, storage and forwarding, so a readable name in one links a network address to an interest.

#### Scenario: One Stoa yields one channel identifier and one content topic

- **WHEN** the channel identity for a Stoa address is derived
- **THEN** exactly one channel identifier and one content topic are produced

#### Scenario: Two Stoas do not share a channel

- **WHEN** channel identities are derived for two different Stoa addresses
- **THEN** the two channel identifiers differ
- **AND** the two content topics differ

#### Scenario: A Stoa's title does not appear in its content topic

- **WHEN** a content topic is derived for a Stoa whose genesis title is a known string
- **THEN** that string does not appear in the content topic
- **AND** the content topic is derived from the Stoa's address alone

#### Scenario: No per-op value reaches the channel identity

- **WHEN** channel identity is derived for one Stoa, and ops of several threads and several authors are published on it
- **THEN** the channel identifier and content topic are the same for every one of them

### Requirement: Channel identity is a pure function of the Stoa address

The channel identifier and the content topic SHALL each be a pure function of the Stoa's address. Nothing else SHALL participate: not a session counter, not an epoch, not a local sequence number, not a device identifier, not a wall-clock reading, not the number of times this peer has opened the channel, and not the peer's own identity.

**This requirement exists because its violation produces no error.** Two peers computing different channel identifiers for one Stoa do not fail, do not warn and do not retry: each opens a channel nobody else is in, and the two never see one another's ops. The partition is silent and permanent, and no participant can observe it from inside. Every other refusal in this capability reports itself; this one cannot, so the property has to hold by construction rather than be checked.

A deterministic epoch SHALL NOT be introduced as a variant of this. An epoch that every peer must recompute identically is a rendezvous problem at every boundary where it changes, and it reintroduces the same silent partition at each one.

Re-deriving channel identity for a Stoa already joined SHALL yield the same values it yielded before.

#### Scenario: Deriving twice yields the same identity

- **WHEN** channel identity is derived twice for one Stoa address
- **THEN** both derivations yield the same channel identifier
- **AND** both yield the same content topic

#### Scenario: Identity does not vary with local state

- **WHEN** channel identity is derived for one Stoa address under differing local state — a different peer identity, a different count of prior opens, a different clock reading
- **THEN** every derivation yields the same channel identifier and content topic

#### Scenario: Reopening a channel does not change its identity

- **WHEN** a channel for a Stoa is closed and its identity is derived again
- **THEN** the channel identifier is the one derived before the close
- **AND** it carries no epoch, counter or generation distinguishing the second derivation from the first

#### Scenario: The derivation is pinned against silent change

- **WHEN** channel identity is derived from a fixed Stoa address
- **THEN** the channel identifier and content topic equal values derived independently of this implementation
- **AND** a change to the derivation fails this rather than passing quietly

### Requirement: The transport's sender identifier is never an identity

A channel participant SHALL supply a sender identifier when it opens a channel, and every receiving peer SHALL be handed that identifier alongside each message.

**That identifier SHALL NOT be read as an author identity, and SHALL NOT reach any authorisation decision.** It is an application-chosen string the transport uses to let a participant recognise its own traffic; the transport neither issues it, verifies it, nor prevents two participants from claiming the same one. Authorship comes from the op's own signature and the author key inside it, and from nothing else.

Specifically, a receiving peer SHALL NOT use the sender identifier to decide who authored an op, to decide whether an op's author may moderate, to decide whether an op may revise another, to accept an op whose signature does not verify, or to prefer one op over another.

The sender identifier SHALL NOT be stored as part of an op, and SHALL NOT participate in an op's identifier. An op received twice under two different sender identifiers SHALL be one op.

#### Scenario: The sender identifier does not establish authorship

- **WHEN** an op authored and signed by one key arrives under a sender identifier belonging to a different participant
- **THEN** the op's author is the one its signature and author key establish
- **AND** the sender identifier does not change that answer

#### Scenario: A forged sender identifier grants nothing

- **WHEN** an op arrives whose sender identifier matches a Stoa moderator's, and whose signature does not verify under the author it claims
- **THEN** the op is refused as failing verification
- **AND** the matching sender identifier does not admit it

#### Scenario: One op under two sender identifiers is one op

- **WHEN** the same signed op arrives twice under two different sender identifiers
- **THEN** both yield the same op identifier
- **AND** the peer holds one op

#### Scenario: The sender identifier is not part of what is stored

- **WHEN** an op received over a channel is read back from storage
- **THEN** nothing recovered from it carries the sender identifier it arrived under

### Requirement: The arrival timestamp is a local clock reading and orders nothing

A receiving peer SHALL be handed a timestamp alongside each inbound message. **That timestamp SHALL NOT be read as a property of the message.** It is the receiving peer's own clock read at the moment its handler ran, so two peers receiving one message record two different values for it, and a peer receiving two messages records values reflecting its own receive sequence rather than anything either sender did.

A receiving peer SHALL NOT derive any order from it: not an order between two ops, not a recency, not a tiebreak within an order, and not a fallback used when other ordering metadata is absent. It SHALL NOT be recorded as an op's ordering metadata, and SHALL NOT be substituted for an ordering value the transport did not supply.

**This is not a preference for a wire value over a local one. There is no wire timestamp on this event.** A peer recording this value as though it were one would produce an order that is stable, total, and different on every peer, which is indistinguishable from a real order until two peers are compared.

#### Scenario: The arrival timestamp is not recorded as ordering metadata

- **WHEN** an op arrives and its arrival metadata is recorded
- **THEN** the recorded metadata reports no Lamport timestamp
- **AND** the timestamp supplied with the message appears nowhere in it

#### Scenario: The timestamp handed in does not change what is recorded

- **WHEN** one op is received twice, accompanied by two different timestamps — including a far-past and a far-future value
- **THEN** the recorded arrival metadata is the same in both cases
- **AND** it is the same as for an op received with a zero timestamp

#### Scenario: What is recorded does not vary with receive sequence

- **WHEN** two ops are received on one channel in one sequence, and the same two are received in the reverse sequence
- **THEN** each op's recorded arrival metadata is the same in both cases
- **AND** nothing recorded distinguishes which of the two was received first

### Requirement: An arrival over this transport carries no ordering metadata

A peer receiving an op over this transport SHALL record the arrival as carrying no Lamport timestamp and no transport message identifier, because this transport supplies neither. It SHALL NOT fabricate either value, and SHALL NOT record in their place a value derived from the message's arrival timestamp, from the sender identifier, or from a counter local to this peer.

The prohibition is on *recording a value as though the transport supplied it*. It is not a prohibition on a Stoa's ops carrying an ordering value of their own: such a value would travel inside a signed op and reach a peer through the op decoder, not through this event, and this capability neither provides nor forbids one.

No causal history, message identifier list, or list of messages the sender held reaches this layer either, so a receiving peer SHALL NOT be contracted to detect a gap from what the transport tells it. Detecting that an op names a parent the peer does not hold is a property of the ops themselves and is outside this capability.

The consequence — that such ops fall into a defined degraded order, below every op the transport did order, identical on every peer, and reported as degraded — is `op-ordering`'s requirement "An op the transport did not order sorts below every op it did", and is not restated here. What this capability contracts is only that this transport is a source of exactly those arrivals.

#### Scenario: Every arrival over this transport is recorded as unordered

- **WHEN** any op is received over a reliable channel and its arrival metadata is recorded
- **THEN** the metadata reports the op as not ordered by the transport

#### Scenario: No ordering value is fabricated from what did arrive

- **WHEN** an op arrives with a sender identifier and an arrival timestamp
- **THEN** neither appears in the recorded arrival metadata
- **AND** the recorded metadata is indistinguishable from one recorded for an op that arrived with neither

### Requirement: A locally-authored op is stored before it is published

An op the local peer authored SHALL be appended to the peer's own op log before its bytes are handed to the transport.

The order matters and is not an implementation detail. Publishing first and storing second loses the op in every case where the store fails after the send succeeded: the op is on the network, other peers hold it, and its author does not — so the author's own view of the forum is missing a post it published, and the peer has no record with which to notice.

A published op SHALL be recorded as arriving unordered, exactly as a received one is. A peer SHALL NOT record ordering metadata for its own op that it could not record for the same op received from a peer, and SHALL NOT give its own ops a position other peers cannot reproduce.

A failure to hand the bytes to the transport SHALL NOT remove the op from the log or alter it. The op is signed and stored; whether it reached the network is a separate fact, and discarding a stored op on a send failure would make an op's existence depend on network conditions at one instant.

Publishing on a Stoa whose channel this peer has no open channel for SHALL be reported as a failure to publish, distinguishably from a transport failure on an open channel, and SHALL NOT open a channel as a side effect. When a channel is opened, and for which Stoas, is the Stoa-lifecycle capability's; publishing SHALL NOT be the act that decides it. The op SHALL still be stored, on the same reasoning: the author authored it.

#### Scenario: A published op is in the local log

- **WHEN** a locally-authored op is published
- **THEN** it is readable from the local op log
- **AND** it is readable whether or not the send has been confirmed

#### Scenario: A send failure does not lose the op

- **WHEN** publishing an op fails at the transport
- **THEN** the op is still in the local log
- **AND** it is byte-identical to the op that was stored

#### Scenario: A peer's own op carries no ordering metadata of its own

- **WHEN** a locally-authored op is stored on publication
- **THEN** its recorded arrival reports no Lamport timestamp
- **AND** the recorded arrival is indistinguishable from that of an op received from another peer

#### Scenario: The bytes published are the bytes stored

- **WHEN** an op is published
- **THEN** the payload handed to the transport is the op's wire form as stored
- **AND** nothing is prepended, appended or re-encoded around it

#### Scenario: Publishing without an open channel fails and opens nothing

- **WHEN** an op naming a Stoa with no open channel is published
- **THEN** publishing reports a failure distinguishable from a transport failure on an open channel
- **AND** no channel is opened
- **AND** the op is in the local log

### Requirement: What the channel carries is an op's wire form and nothing else

The payload published on a channel SHALL be exactly one signed op's wire form. No envelope, header, framing or metadata of this capability's own SHALL be wrapped around it.

An envelope would be attacker-controlled bytes outside the signature, which is the one shape this system has no defence for: anything a peer reads from outside the signed preimage is something any relay may rewrite. Everything a receiving peer needs in order to act on an op — the Stoa, the author key, the kind, the target — is already inside the signed bytes.

A payload carrying more than one op, or carrying an op followed by anything else, SHALL be refused rather than partially read. That refusal is the op decoder's — `op-format` already contracts that a wire form followed by an extra byte is rejected — and what this requirement adds is that the whole payload is handed to that decoder, so there is no region of a payload the decoder never sees and no framing that would make trailing bytes legitimate.

#### Scenario: A payload is one op's wire form

- **WHEN** an op is published and the payload is examined
- **THEN** the payload equals the op's wire form exactly

#### Scenario: The whole payload is what is decoded

- **WHEN** a payload carrying a valid op's wire form followed by any extra byte arrives
- **THEN** it is refused
- **AND** the op it begins with is not stored, so no prefix of the payload was decoded in isolation

### Requirement: Every inbound payload is validated before it reaches storage

A payload arriving on a channel SHALL be validated before it is appended to the op log, before it reaches any resolver, and before any property of it is used to look anything up.

Every byte arriving on a channel is attacker-controlled. A channel has no membership: any peer that computes a Stoa's channel identity may send on it, and the transport neither knows nor asserts who a participant is. Forged authorship, malformed bytes, oversized payloads and ops naming a Stoa the sender has no business touching are the ordinary content of this boundary rather than exceptional cases.

Each of the following SHALL be refused, and each SHALL be reported distinguishably from the others:

- a payload arriving on a channel identifier this peer has no open channel for;
- a payload the op decoder does not accept;
- a payload carrying an op whose signature does not verify, or whose presented key does not bind to the author it claims;
- a payload carrying an op naming a Stoa other than the Stoa whose channel it arrived on;
- a payload larger than a single message may carry.

A refusal SHALL NOT partially apply: a refused payload SHALL leave the op log, the peer's view of the Stoa, and its channel state as they were.

**A boundary reporting only "invalid" sends the reader looking in the wrong place.** These five failures have five different causes and five different responses — a build that is behind, a corrupt or hostile payload, a forgery, a misdirected or replayed op, and a peer sending more than the network permits — and a peer that cannot tell them apart cannot report which of them is happening to its user or to a log.

#### Scenario: A payload on an unknown channel is refused

- **WHEN** a payload arrives bearing a channel identifier this peer has no open channel for
- **THEN** it is refused, reported as an unknown channel
- **AND** nothing is stored

#### Scenario: A payload that does not decode is refused

- **WHEN** a payload that the op decoder does not accept arrives on an open channel
- **THEN** it is refused, reported distinguishably from an unknown channel
- **AND** no partially-populated op is stored

#### Scenario: An op whose signature does not verify is refused

- **WHEN** a payload decodes to an op whose signature does not verify under the author it claims
- **THEN** it is refused, reported distinguishably from a payload that did not decode
- **AND** the op is not stored

#### Scenario: An op whose key does not bind to its claimed author is refused

- **WHEN** a payload decodes to an op carrying a valid signature under a key whose address is not the author the op claims
- **THEN** it is refused
- **AND** the refusal is distinguishable from a malformed payload

#### Scenario: Refusal leaves nothing behind

- **WHEN** any refused payload is processed
- **THEN** the op log holds no entry for it
- **AND** the channel remains open and continues to accept subsequent payloads

#### Scenario: A valid op is stored

- **WHEN** a payload decodes to an op that verifies and names the Stoa whose channel it arrived on
- **THEN** it is appended to the op log
- **AND** its recorded arrival reports it as not ordered by the transport

### Requirement: An op is refused unless it names the Stoa whose channel it arrived on

An inbound op SHALL be refused unless the Stoa named inside its signed bytes is the Stoa whose channel the payload arrived on. The refusal SHALL be reported distinguishably from a payload that failed to decode and from one whose signature failed.

An op's Stoa is inside its signature, so an op cannot be *rewritten* to name another Stoa without breaking it — but it can be **copied unchanged onto another Stoa's channel**, where it is authentic, verifies, and is a post its author never addressed there. Only comparing the op's own Stoa against the channel's refuses it.

**This is also the whole answer to what happens when an op arrives for a Stoa the peer has not joined.** A channel is opened for a Stoa the peer holds, so there is no channel on which an op for an unjoined Stoa can arrive as a legitimate message: such an op arrives only as a payload whose named Stoa differs from its channel's, and this requirement refuses it. A peer SHALL NOT open a channel, derive a channel identity, join a Stoa, store a genesis record, or store the op in response to receiving one. Whether the peer holds a Stoa at all, and what holding it means, belongs to the Stoa-lifecycle capability; this capability contracts only that a received op never causes it.

#### Scenario: An op naming another Stoa is refused

- **WHEN** a validly signed op naming one Stoa arrives on the channel of a different Stoa
- **THEN** it is refused, reported as a Stoa mismatch
- **AND** the refusal is distinguishable from a decode failure and from a signature failure

#### Scenario: A cross-Stoa copy does not join the peer to anything

- **WHEN** an op naming a Stoa this peer does not hold arrives on an open channel
- **THEN** the op is not stored
- **AND** no channel is opened for the Stoa the op names
- **AND** the peer does not hold that Stoa afterwards

#### Scenario: An authentic op is still refused on the wrong channel

- **WHEN** the op refused for naming another Stoa is examined
- **THEN** its signature verifies under its author
- **AND** the refusal is the Stoa comparison rather than a failure of authenticity

### Requirement: An oversized payload is refused, and the limit is the transport's

A payload larger than the maximum a single message may carry SHALL be refused, and the refusal SHALL be reported distinguishably from a decode failure so that "no peer could legitimately have sent this" is tellable from "this op is corrupt".

That maximum SHALL be **150 KiB**, the network-wide validation limit applied to one message, which cannot be raised unilaterally. The value is named here because the requirement's justification depends on it: a limit that had silently drifted upward would still refuse an absurd payload and still satisfy a scenario probing only absurd sizes.

The check SHALL apply to the payload as received, before it is decoded. `op-format`'s cap bounds one field of an op and explicitly declines to bound a decoded op's total size, on the grounds that a decoder handed a byte slice cannot see the frame the bytes arrived in. **This capability is that frame**, and this requirement is the total-size bound `op-format` named as belonging here.

#### Scenario: A payload over the limit is refused

- **WHEN** a payload larger than the message limit arrives
- **THEN** it is refused, reported as over-long
- **AND** the refusal is distinguishable from a decode failure

#### Scenario: A payload at the limit is not refused for its size

- **WHEN** a payload of exactly the maximum permitted size arrives
- **THEN** it is not refused on account of its size

#### Scenario: The limit's value is pinned against silent drift

- **WHEN** the configured maximum is compared against the transport's stated message limit
- **THEN** they are equal
- **AND** a change to either fails this rather than passing quietly

### Requirement: Receiving a payload never aborts the process

Processing an inbound payload SHALL NOT panic, for any byte string of any length, whatever channel identifier, sender identifier or timestamp accompanies it.

A panic on this path aborts the module process. Every later call reports the module as not loaded and nothing restarts it, so a panic reachable from a payload any peer may send is a remotely triggerable denial of service against the peer that received it — not a robustness nicety. The bytes are chosen by whoever sent them.

#### Scenario: Arbitrary bytes are refused without a panic

- **WHEN** arbitrary byte strings, including empty payloads, single bytes, single-byte mutations of a valid op, and payloads of unrelated lengths, arrive on an open channel
- **THEN** each is refused or stored without panicking

#### Scenario: A hostile channel or sender identifier does not panic

- **WHEN** a payload arrives bearing an empty, maximal, or non-textual channel identifier or sender identifier
- **THEN** processing returns an outcome without panicking

#### Scenario: The peer keeps receiving after a refusal

- **WHEN** a refused payload is followed by a valid one on the same channel
- **THEN** the valid op is stored

### Requirement: The delivery node is shared and is never stopped by this peer

The delivery node SHALL be created once per context, and this capability SHALL NOT stop it.

The node is a separate, shared process serving every module in one context. Stopping it takes delivery away from every other module using it, which is not this application's decision to make, and creating a second one is not available. Neither leaving a Stoa nor shutting the application down SHALL stop the node.

Consequently, releasing what a Stoa's channel holds SHALL be done by closing the channel rather than by stopping the node.

#### Scenario: Shutdown does not stop the node

- **WHEN** the application shuts down
- **THEN** the node is not stopped
- **AND** the channels this peer opened are closed

#### Scenario: Leaving a Stoa does not stop the node

- **WHEN** a user leaves a Stoa
- **THEN** the node is not stopped

#### Scenario: The node is created once

- **WHEN** several Stoas' channels are opened
- **THEN** no additional node is created for any of them

### Requirement: A channel is closed when a user leaves a Stoa and on shutdown

A Stoa's channel SHALL be closed when the user leaves that Stoa, and every open channel SHALL be closed when the application shuts down.

Both cases are one situation: a channel must not outlive the application that opened it. A channel left open keeps the shared node working on behalf of a Stoa belonging to an application nobody has open — holding filter subscriptions and the remote peer slots serving them, or running the topic's handler chain and synchronisation loops locally.

**Closing SHALL be treated as best-effort and SHALL NOT be reported as a guarantee.** The underlying close undertakes to stop the channel's own loops; the release of the content topic behind it is reference-counted across channels on that topic and takes effect only when the last one goes, and its failures are not surfaced to the caller. A peer therefore cannot observe that a release reached the network, and SHALL NOT claim it did.

A channel closed SHALL be reopenable under the same identity, because a user may leave a Stoa and rejoin it. Reopening SHALL NOT require a restart and SHALL NOT change the channel identifier.

Closing a channel SHALL NOT discard, alter or hide the ops already stored from it. The ops are the peer's, and leaving a Stoa is a statement about what it listens to rather than about what it has seen.

#### Scenario: Leaving a Stoa closes its channel

- **WHEN** a user leaves a Stoa whose channel is open
- **THEN** that channel is closed
- **AND** no other Stoa's channel is closed

#### Scenario: Shutdown closes every open channel

- **WHEN** the application shuts down with channels open for several Stoas
- **THEN** each is closed

#### Scenario: A Stoa can be rejoined without a restart

- **WHEN** a user leaves a Stoa and rejoins it in the same session
- **THEN** a channel is opened under the same channel identifier as before
- **AND** no restart is required

#### Scenario: Closing a channel keeps the ops received on it

- **WHEN** a Stoa's channel is closed
- **THEN** the ops received on it are still readable from the op log

### Requirement: A peer's own published op is not received back as an arrival

A peer SHALL hold its own published op through having stored it on publication, and SHALL NOT depend on receiving it back over the channel.

The receive event does not fire for a participant's own messages; a peer's own send is reported to it through a separate send outcome. A peer relying on the receive path for its own ops would never store them, and a peer relying on the sender identifier to filter out its own arrivals would be filtering something that never arrives — which is why the sender identifier is not that filter here.

An op the peer already holds arriving on the channel SHALL leave the peer holding one op. Deduplication is by op identifier and is `op-log`'s; this requirement contracts only that a second arrival of an op the peer authored is not a second op.

#### Scenario: A published op does not depend on being received back

- **WHEN** a peer publishes an op and no inbound arrival for it is ever delivered
- **THEN** the peer holds the op

#### Scenario: An op the peer already holds arriving again is one op

- **WHEN** an op already in the peer's log arrives on the channel
- **THEN** the peer holds one entry for it
- **AND** the recorded arrival is the one recorded first

### Requirement: This capability decides nothing about an op beyond admitting it

Admitting an op to the op log SHALL NOT be read as a statement that the op is permitted, that its author may moderate, that a revision it declares takes effect, or that a Stoa's posting policy admits its author. Validation at this boundary answers whether the bytes are a well-formed, authentic op addressed to this channel's Stoa, and nothing further.

Authority is decided on read, against state a peer may not have held when the op arrived. A boundary that also decided authority would decide it once, from whatever the peer knew at that instant, and would then present the answer as a property of the stored op.

Conversely, this boundary SHALL NOT admit an op that fails authenticity on the grounds that authority is checked later. The two checks answer different questions against different inputs, and neither substitutes for the other.

#### Scenario: An authentic moderation op from a non-moderator is admitted

- **WHEN** an authentically signed moderation op whose author is not a moderator of the Stoa arrives on that Stoa's channel
- **THEN** it is stored
- **AND** storing it is not a statement that the moderation binds

#### Scenario: An unauthentic op is not admitted on the promise of a later check

- **WHEN** an op whose signature does not verify arrives
- **THEN** it is refused at this boundary
- **AND** it is not stored for a reader to judge later
