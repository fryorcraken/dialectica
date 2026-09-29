## MODIFIED Requirements

### Requirement: A locally-authored op is stored before it is published

An op the local peer authored SHALL be appended to the peer's own op log before its bytes are handed to the transport.

The order matters and is not an implementation detail. Publishing first and storing second loses the op in every case where the store fails after the send succeeded: the op is on the network, other peers hold it, and its author does not — so the author's own view of the forum is missing a post it published, and the peer has no record with which to notice.

A published op SHALL be recorded as arriving unordered, exactly as a received one is. A peer SHALL NOT record ordering metadata for its own op that it could not record for the same op received from a peer, and SHALL NOT give its own ops a position other peers cannot reproduce.

A failure to hand the bytes to the transport SHALL NOT remove the op from the log or alter it. The op is signed and stored; whether it reached the network is a separate fact, and discarding a stored op on a send failure would make an op's existence depend on network conditions at one instant.

Publishing on a Stoa whose channel this peer has no open channel for SHALL be reported as a failure to publish, distinguishably from a transport failure on an open channel, and SHALL NOT open a channel as a side effect. When a channel is opened, and for which Stoas, is the Stoa-lifecycle capability's; publishing SHALL NOT be the act that decides it. The op SHALL still be stored, on the same reasoning: the author authored it.

**That report is this capability's publish path answering the code that performs the send, and it is not the reply a caller of the module receives.** At the module's surface, a publish into a Stoa with no open channel SHALL answer as `content-authoring`'s requirement "Publishing signs, appends, and hands off — in that order" requires where the handoff does not happen: the op is reported as published, because it is in the log, and the reply carries no delivery outcome. What is sent and what is logged instead is the requirement "A published op is handed to its Stoa's reliable channel".

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

#### Scenario: The module's reply to a publish with no open channel is a published op

- **WHEN** a post is published through the module's surface into a Stoa this peer has no open channel for
- **THEN** the reply reports the op as published and names its op id
- **AND** the reply is not the error shape

### Requirement: The delivery node is shared and is never stopped by this peer

The delivery node is the delivery module's. This application SHALL request its creation and its start at most once per module process, SHALL NOT create a node of its own, and SHALL NOT stop it.

The node is a separate, shared process serving every module in one context. Stopping it takes delivery away from every other module using it, which is not this application's decision to make, and creating a second one is not available. Neither leaving a Stoa nor shutting the application down SHALL stop the node.

Consequently, releasing what a Stoa's channel holds SHALL be done by closing the channel rather than by stopping the node.

**Creation SHALL be requested with the reliable-channel layer mounted, and the configuration SHALL name that layer** rather than rely on it being delivery's default. Every Stoa's ops travel on a reliable channel, and a node without that layer refuses every channel operation.

**Creation SHALL be requested before any channel operation this application requests.** A creation that delivery declines or that fails SHALL NOT stop the module from serving: it SHALL be written to the module's log with delivery's reason; every call that does not need delivery SHALL answer as it would have otherwise; and channel operations SHALL still be requested, because a node that another module in the same context created serves this application's channels too.

**This requirement binds the adapter that makes delivery calls, which is not the code the requirements above contract.** No part of this capability's own surface starts, stops or counts nodes. The obligation is stated here, where the reasoning for it lives, and discharged in the adapter.

**A peer cannot observe compliance with the prohibition, and no scenario below claims it can.** Observing it would take a second module in the same context noticing that its delivery had gone, which is a property of a deployed context rather than of this capability. The obligation is therefore that a stop call **SHALL NOT appear** in this application at all: a prohibition on the code rather than on an outcome, checked by reading the application rather than by exercising it. A change that adds one satisfies every other requirement in this capability and breaks this one, and nothing in this capability's surface will say so.

**The prohibition covers the whole application, not one handler.** This application's one lifecycle handler runs when the module is ready, and that is where the node's creation is requested. It has no shutdown handler and no leave-Stoa handler. A requirement phrased as a property of those two handlers would be satisfied by their absence, and would go on reading as satisfied after one was written that violated it. Phrased over the application, the prohibition can be checked now — no site in it stops a node — and stays checkable when those handlers land. **A fourth thing is consequently owed alongside the three named by "A successful publish is a statement about the local log and nothing more": the shutdown and leave-Stoa handlers, which this change does not supply.** It is a separate gap from those three. They concern a delivery outcome. This one is that nowhere is shutdown or leaving a Stoa observed at all.

Whether a handler, once written, closes this peer's channels is contracted by "A channel is closed when a user leaves a Stoa and on shutdown" and SHALL NOT be restated here. This requirement is about the node.

#### Scenario: No site in this application stops or creates a delivery node

- **WHEN** this application is examined for what it does with a delivery node
- **THEN** no site in it stops a node
- **AND** no site in it creates a node of its own
- **AND** exactly one site in it asks the delivery module to create and start the node
- **AND** the examination is over the whole application rather than over a lifecycle handler, because there is no lifecycle handler for shutdown or for leaving a Stoa

#### Scenario: The node's configuration names the reliable-channel layer

- **WHEN** the configuration this application hands to node creation is examined
- **THEN** it names the reliable-channel entry layer explicitly

#### Scenario: Node creation is requested once however often startup runs

- **WHEN** the module's startup runs twice in one module process
- **THEN** node creation is requested once
- **AND** node start is requested once

#### Scenario: Node creation precedes every channel operation

- **WHEN** the module starts while the peer is in two Stoas
- **THEN** node creation is requested before any channel creation is requested

#### Scenario: A declined node creation does not stop the module

- **WHEN** delivery answers node creation with its error shape
- **THEN** the module's log carries delivery's reason
- **AND** channel creation is still requested for every Stoa the peer is in
- **AND** a call that reads the op log answers as it would have had the creation succeeded

## ADDED Requirements

### Requirement: A published op is handed to its Stoa's reliable channel

Every publish that succeeds at the module's surface — one that stored the op, and one that found the op already held — MUST hand that op's wire form to delivery as a send on the reliable channel of the op's Stoa, when this peer has that channel open.

The payload handed over MUST be exactly the op's wire form as stored, as "What the channel carries is an op's wire form and nothing else" requires. The channel MUST be named by the channel identifier "Channel identity is a pure function of the Stoa address" derives from the op's Stoa, and by no other value.

When this peer has no channel open for the op's Stoa at the time the send would be made, the peer MUST NOT send anything and MUST NOT open a channel, and the module's log MUST record the op id and the Stoa it was not sent for.

**The reply MUST NOT wait on the handoff.** `content-authoring` forbids the reply to wait on anything delivery does with the op, and a send is a call into another process. A send that delivery declines, fails, or never answers MUST leave the op in the log unchanged, MUST NOT change the reply, and MUST be recorded in the module's log with the op id and delivery's reason where delivery gave one.

**Sends MUST be made in the order their publishes were answered.** A send for an op whose Stoa's channel an earlier call asked to open MUST be made after that open has been answered.

#### Scenario: An op published on an open channel is sent as its wire form

- **WHEN** a post is published into a Stoa whose channel this peer has open
- **THEN** delivery is asked to send on the channel identifier derived from that Stoa's address
- **AND** the payload equals the op's wire form as read back from the log by the op id the reply names

#### Scenario: Re-publishing an op already held sends it again

- **WHEN** an op the peer already holds is published again, so that the reply reports it was not newly stored
- **THEN** delivery is asked to send it on its Stoa's channel

#### Scenario: A publish into a Stoa with no open channel sends nothing and opens nothing

- **WHEN** a post is published into a Stoa this peer has no channel open for
- **THEN** delivery is not asked to send anything
- **AND** delivery is not asked to create a channel
- **AND** the reply reports the op as published and names its op id
- **AND** the module's log records that op id and that Stoa

#### Scenario: A send delivery declines leaves the op published

- **WHEN** delivery answers a send with its error shape
- **THEN** the reply to the publish reports the op as published and names its op id
- **AND** the op is readable from the log, unchanged
- **AND** the module's log records the op id and delivery's reason

#### Scenario: An unresponsive delivery does not delay the reply

- **WHEN** delivery never answers a send
- **THEN** the publish's reply is returned without waiting for it
- **AND** the reply reports the op as published

#### Scenario: Sends follow the order of their publishes

- **WHEN** a post and then a reply to it are published into a Stoa whose channel this peer has open
- **THEN** the post's send is requested before the reply's

#### Scenario: A publish after a join is sent on the channel the join opened

- **WHEN** a peer joins a Stoa, delivery reports the channel created, and the peer then publishes a post into that Stoa
- **THEN** channel creation is requested before the send
- **AND** the send is requested on the channel identifier that was created

### Requirement: Every payload the reliable channel delivers passes the inbound boundary

Every message delivery hands over as received on a reliable channel MUST be put through the boundary "Every inbound payload is validated before it reaches storage" contracts, with the channel identifier, sender identifier and payload it arrived with. An op received from the network MUST reach the op log by no other route.

No other delivery event is an arrival. A message delivered outside a reliable channel, and a report on this peer's own send, MUST NOT reach the op log.

**The receive window MUST be judged against this peer's own clock, read when the payload is processed**, and never against the timestamp the delivery event carries.

Each refusal MUST be recorded in the module's log naming which refusal it was. **The log MUST NOT carry the payload, the sender identifier, or a channel identifier this peer has no channel open under**: whoever sent the message chose each of them, and a channel identifier this peer did not open may belong to another application sharing the node.

A delivered message whose fields cannot be read MUST be discarded and recorded in the module's log. A refusal, a discarded message, or a failure to store MUST NOT stop the peer from processing the messages that follow.

#### Scenario: An op another peer published is stored

- **WHEN** a message arrives on a channel this peer has open, carrying a valid op that names that channel's Stoa and whose counter is within the receive window of this peer's clock
- **THEN** the op is in the log
- **AND** its recorded arrival reports it as not ordered by the transport

#### Scenario: The window is judged by this peer's clock, not the event's timestamp

- **WHEN** a message carries a valid op whose counter is more than one hour ahead of this peer's clock when it is processed, and the event's timestamp is later still
- **THEN** the op is refused as ahead of this peer's time
- **AND** it is not stored

#### Scenario: An event timestamp far in the past does not refuse an op within the window

- **WHEN** a message carries a valid op whose counter is within the receive window of this peer's clock when it is processed, and the event's timestamp is decades earlier
- **THEN** the op is stored

#### Scenario: A refusal is logged by kind, without text the sender chose

- **WHEN** a message arrives on a channel identifier this peer has not opened, carrying a distinctive sender identifier and payload
- **THEN** the module's log records the refusal as an unknown channel
- **AND** the log contains neither that channel identifier, nor that sender identifier, nor the payload's bytes

#### Scenario: The peer keeps processing after an unreadable message

- **WHEN** a message whose fields cannot be read is followed by one carrying a valid op on an open channel
- **THEN** the valid op is stored

#### Scenario: Only reliable-channel receipts reach the op log

- **WHEN** this application is examined for which delivery events can reach the op log
- **THEN** the only one is a message received on a reliable channel
- **AND** a report on this peer's own send is not among them

### Requirement: Inbound payloads waiting for the boundary are bounded

The payloads this peer holds between delivery handing them over and the boundary deciding them MUST be bounded by a fixed count. Taking a message from delivery MUST NOT wait on the boundary.

Waiting payloads MUST be decided in the order they arrived. When the bound is reached, a payload arriving MUST be discarded, and the payloads already waiting MUST be kept.

A discarded payload is not stored. Every discard MUST be counted from the module's start, and each MUST be recorded in the module's log with the running count. A discard's log record MUST be distinguishable from a refusal's, and MUST NOT carry the payload or the sender identifier.

#### Scenario: The waiting payloads never exceed the bound

- **WHEN** more payloads arrive than the bound while none is decided
- **THEN** the number waiting equals the bound

#### Scenario: A full queue keeps what it holds and discards the arrival

- **WHEN** the bound is reached and one more payload arrives
- **THEN** the payloads decided next are the ones that were waiting, in the order they arrived
- **AND** the arriving payload is not stored

#### Scenario: Every discard is counted and logged apart from refusals

- **WHEN** three payloads are discarded
- **THEN** the running count recorded with the third discard is three
- **AND** each discard's log record is distinguishable from a refusal's

### Requirement: The sender identifier this peer supplies is its own, stable, and says nothing about its author

When this peer opens a Stoa's channel, the sender identifier it supplies MUST:

- differ from the one any other installation supplies for that Stoa, including an installation holding the same identity;
- be the same every time this installation opens that Stoa's channel, across restarts;
- differ between two Stoas for one installation;
- be neither a public key this peer holds or signs with, nor computable from one.

If the sender identifier cannot be retained so that the next start supplies the same one, the channel MUST NOT be opened, and the module's log MUST record the Stoa.

Every participant in a channel sees the sender identifier. What a receiving peer may do with one is "The transport's sender identifier is never an identity", and this requirement does not restate it.

#### Scenario: Two installations holding one identity supply different sender identifiers

- **WHEN** two installations holding the same keystore each open the same Stoa's channel
- **THEN** the two sender identifiers differ

#### Scenario: A restart supplies the same sender identifier

- **WHEN** an installation opens a Stoa's channel, is restarted, and opens it again
- **THEN** both opens supply the same sender identifier

#### Scenario: Two Stoas get two sender identifiers

- **WHEN** one installation opens the channels of two different Stoas
- **THEN** the two sender identifiers differ

#### Scenario: The sender identifier is not the author's key

- **WHEN** a sender identifier is compared with the public key the installation signs with
- **THEN** it is not that key, in any encoding this application uses for a key

#### Scenario: A sender identifier that cannot be retained opens no channel

- **WHEN** the storage that retains sender identifiers cannot be written, and a Stoa's channel is to be opened for the first time
- **THEN** channel creation is not requested for that Stoa
- **AND** the module's log records the Stoa
