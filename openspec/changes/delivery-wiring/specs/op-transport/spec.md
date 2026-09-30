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

**Start SHALL be requested only after delivery has accepted this application's creation.** When delivery declines, fails or does not answer the creation, start SHALL NOT be requested. A node this application did not create is not this application's to start, any more than to stop.

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

- **WHEN** the module's startup runs twice in one module process, and delivery accepts the creation
- **THEN** node creation is requested once
- **AND** node start is requested once

#### Scenario: Node creation precedes every channel operation

- **WHEN** the module starts while the peer is in two Stoas
- **THEN** node creation is requested before any channel creation is requested

#### Scenario: A declined node creation does not stop the module

- **WHEN** delivery answers node creation with its error shape
- **THEN** the module's log carries delivery's reason
- **AND** channel creation is still requested for every Stoa the peer is in
- **AND** node start is not requested
- **AND** a call that reads the op log answers as it would have had the creation succeeded

## ADDED Requirements

### Requirement: A published op is handed to its Stoa's reliable channel

Every publish that succeeds at the module's surface — one that stored the op, and one that found the op already held — MUST hand that op's wire form to delivery as a send on the reliable channel of the op's Stoa, when this peer has that channel open.

The payload handed over MUST be exactly the op's wire form as stored, as "What the channel carries is an op's wire form and nothing else" requires. The channel MUST be named by the channel identifier "Channel identity is a pure function of the Stoa address" derives from the op's Stoa, and by no other value.

Once the module's startup has wired delivery, when this peer has no channel open for the op's Stoa at the time the send would be made, the peer MUST NOT send anything and MUST NOT open a channel, and the module's log MUST record the op id and the Stoa it was not sent for.

**A publish answered before the module's startup has wired delivery MUST send nothing, then or later.** The op MUST NOT be held for a send once delivery is wired, and the module's log MUST record the op id. A later publish of the same op is sent as any re-publish is.

**The reply MUST NOT wait on the handoff.** `content-authoring` forbids the reply to wait on anything delivery does with the op, and a send is a call into another process. A send that delivery declines, fails, or never answers MUST leave the op in the log unchanged, MUST NOT change the reply, and MUST be recorded in the module's log with the op id and delivery's reason where delivery gave one.

**Sends MUST be made in the order their publishes were answered.** A send for an op whose Stoa's channel an earlier call asked to open MUST be made after that open has been answered.

#### Scenario: An op published on an open channel is sent as its wire form

- **WHEN** a post is published into a Stoa whose channel this peer has open
- **THEN** delivery is asked to send on the channel identifier derived from that Stoa's address
- **AND** the payload equals the op's wire form as read back from the log by the op id the reply names

#### Scenario: Re-publishing an op already held sends it again

- **WHEN** an op the peer already holds is published again into a Stoa whose channel this peer has open, so that the reply reports it was not newly stored
- **THEN** delivery is asked to send it on its Stoa's channel

#### Scenario: A publish into a Stoa with no open channel sends nothing and opens nothing

- **WHEN** the module's startup has wired delivery, and a post is then published into a Stoa this peer has no channel open for
- **THEN** delivery is not asked to send anything
- **AND** delivery is not asked to create a channel
- **AND** the reply reports the op as published and names its op id
- **AND** the module's log records that op id and that Stoa

#### Scenario: A publish before delivery is wired is not sent when it is

- **WHEN** a post is published into a Stoa the peer is in, before the module's startup has wired delivery, and startup then runs
- **THEN** the module's log records the op id as not sent
- **AND** delivery is not asked to send anything, before or after startup
- **AND** channel creation for that Stoa is requested once, by startup

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

A delivered message whose fields cannot be read MUST be discarded and recorded in the module's log, however the reading fails, a panic in the code that reads them included. A refusal, a discarded message, or a failure to store MUST NOT stop the peer from processing the messages that follow.

**A message the op log cannot take is not decided again.** When the op log cannot be opened, or the append fails, the module's log MUST record it as a storage failure, and the message MUST NOT be held for another attempt: its op is in the log afterwards only if it arrives again.

**A message arriving on a channel that is not open, but is being opened, MUST be judged only once that open is settled** — reported held (created, or already existing, as `stoa-membership`'s requirement "Creating or joining a Stoa opens its reliable channel" says), declined, failed, given up by this peer after asking delivery and receiving no answer, or given up by this peer without delivery being asked — and against the channels open then, unless the wait below expires first. Delivery can hand over a message on a channel before its answer to the creation reaches this peer. A message on any other channel identifier, an open one included, MUST be judged without waiting on any open.

**That wait is bounded by a fixed time for each open, not for each message.** An open being waited on has a time, which ends that fixed time after it starts. The time is started by the first message to begin waiting on the open after the latest create, join or startup asking for its channel, and is started again when this peer asks delivery to create that channel, as "Asking delivery to create a channel starts the open's time again" below says; nothing else starts it. A message MUST be judged no later than the end of the open's time as it stood when the message began waiting, whether or not delivery ever answers the open, save for the one extension that paragraph allows. Once the open's time has ended with the open unanswered, the wait on it has expired: the message waiting then, and every message on that channel taken after it until a request or an ask starts the open's time again, MUST be judged without waiting on that open, against the channels open when it is judged. How long one start of an open's time can hold up the messages on every other channel is therefore at most that fixed time, however many messages arrive on its channel.

**Asking delivery to create a channel starts the open's time again.** When this peer asks delivery to create a channel that is being opened, the open's time MUST start again at that ask, whether or not a message is waiting on the open and whether or not its time had already ended. A message waiting on the open when the ask is made, whose end has not yet passed, MUST then be judged no later than that fixed time after the ask, in place of the end it began waiting with, and its wait expires then rather than earlier. **A message whose end has already passed when the ask is made is not extended, even where it has not yet been judged:** its wait has expired, and it MUST be judged without waiting on that open, as the paragraph above says. That extension is made once for a message: a later ask made while the same message still waits starts the open's time again for the messages taken after it, and MUST NOT move that message's end again. Only this peer asks delivery to create a channel, so no sender can start an open's time again or extend a message's wait.

**That fixed time MUST be longer than the longest this peer waits for delivery to answer one channel creation, and that longest MUST be longer than the time delivery allows itself to answer one.** The order is contracted here; the values are not. With it, a message waiting on an open when this peer asks delivery to create its channel, where its end has not yet passed and that is the first ask made while it waits, and a message that begins waiting on the open after that ask, is judged only after delivery has answered that creation or this peer has given up waiting for the answer; and this peer does not give up on a creation that delivery would still answer within its own time. **Not every message racing a creation is covered, and one that is not can be refused, and lost, before delivery answers:** a message whose wait expired before this peer asked delivery for its channel, whether or not it had been judged by the time of the ask, because its open waited behind other requests this peer had made of delivery for longer than the fixed time, together with every message on that channel taken after that expiry and before the ask; and a message whose wait an earlier ask had already extended, when a later ask for the same channel is made while it still waits.

**The following is a consequence of the requirements above and adds none.** A message waits on an open only while it is the one being judged, and waiting payloads are judged in the order they arrived, so each start of an open's time holds the messages on every other channel up at most once, for at most that fixed time, and the waits of opens left unanswered at the same time run one after another. A message whose wait an ask extends holds them up for less than twice that fixed time: less than it before the ask, since its wait would otherwise have expired, and no more than it after. No wait extends past the moment its own open settles. How long they can hold the other channels up in all is therefore no more than that fixed time for each start of an open's time, and never extends past the moment the last of those opens settles. Only this peer's own creates, joins, startup and asks of delivery start an open's time, so that total is set by what this peer asks for, and no sender can lengthen it by putting more messages on any channel.

**An expired wait changes nothing else about the open.** The channel is still being opened: delivery's answer, when it comes, settles the open as above, and a message delivery hands over on that channel identifier is not refused on hand-over for being on a channel neither open nor being opened. Only a later create, join or startup asking for that channel, or this peer asking delivery to create it, lets a message wait on it again, and that wait is bounded in the same way.

**A request for a channel starts a new wait for the messages that begin waiting after it, and does not lengthen a wait already under way.** A message already waiting when a create, join or startup asks for its channel again MUST be judged no later than the end it began waiting with, or the end the first ask made while it waits moved it to, however many such requests are made while it waits: a request moves no waiting message's end, and only an ask does, once, as above. The new wait applies only to messages on that channel that begin waiting after the request.

**A channel is being opened from the moment a create, a join or the module's startup asks for it, not from the moment delivery is asked.** An open that waits behind other requests this peer has made of delivery is being opened while it waits. It stays so until delivery's answer settles it, or until this peer gives up asking for it, and an open this peer never goes on to ask delivery for is given up. Delivery can hand over a message on a channel before this peer has asked delivery for that channel at all: a module restarted while delivery kept running is handed messages on its Stoas' channels from the moment it subscribes, while its own requests for those channels still wait behind node creation and behind each other.

**The module's startup MUST count the channel of every Stoa it asks for as being opened before it checks any message delivery hands over**, whether that check is made on hand-over or at the boundary.

**If this peer cannot subscribe to the messages delivery hands over on reliable channels, the module's log MUST record that this peer will not receive ops from other peers**, and node creation, channel creation and sends MUST still be requested as they would have been otherwise.

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

#### Scenario: A panic reading one message does not end reception

- **WHEN** reading one delivered message's fields panics, and a message carrying a valid op then arrives on a channel this peer has open
- **THEN** the module's log records the failure
- **AND** the valid op is stored

#### Scenario: A message the op log cannot take is logged and not retried

- **WHEN** a message carrying a valid op arrives on an open channel while the op log cannot be opened, and the op log can be opened again afterwards
- **THEN** the module's log records a storage failure
- **AND** the op is not in the op log afterwards

#### Scenario: A message arriving while its channel opens is stored once delivery reports the channel created

- **WHEN** this peer has requested a Stoa's channel, a message carrying a valid op for that Stoa arrives on it before delivery has answered, and delivery then reports the channel created
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A message arriving while its channel opens is refused once delivery declines the open

- **WHEN** this peer has requested a Stoa's channel, a message carrying a valid op for that Stoa arrives on it before delivery has answered, and delivery then answers the creation with its error shape, for a reason other than that the channel already exists
- **THEN** the message is refused as arriving on an unknown channel
- **AND** the refusal is made after delivery's answer, not before it

#### Scenario: A message waiting on an open delivery never answers is judged after a bounded wait

- **WHEN** this peer has requested a Stoa's channel, delivery never answers the creation, and a message carrying a valid op for that Stoa arrives on it
- **THEN** the message is refused as arriving on an unknown channel while delivery has still not answered
- **AND** a message carrying a valid op, arriving after it on a channel this peer has open, is then stored

#### Scenario: Many messages on one unanswered open hold other channels up for one wait, not one each

- **WHEN** this peer has requested a Stoa's channel, delivery never answers the creation, at least three messages arrive one after another on that channel identifier, this peer does not ask delivery to create that channel while any of them waits, and a message carrying a valid op then arrives on a channel this peer has open
- **THEN** each message on the unanswered channel is refused as arriving on an unknown channel while delivery has still not answered
- **AND** the valid op is stored before the fixed time has passed twice over, counted from when the first of those messages began waiting

#### Scenario: Each unanswered open's wait is its own

- **WHEN** this peer has requested two Stoas' channels, delivery answers neither creation, a message arrives on the first Stoa's channel identifier, then one on the second Stoa's, and then a message carrying a valid op on a channel this peer has open, and this peer asks delivery to create neither channel while either of those messages waits
- **THEN** each message on an unanswered channel is refused as arriving on an unknown channel while delivery has still not answered
- **AND** the message on the second Stoa's channel is refused no sooner than the fixed time after the message on the first was refused
- **AND** the valid op is stored before the fixed time has passed three times over, counted from when the first of those messages began waiting

#### Scenario: A message's wait outlasts this peer's wait on a creation, which outlasts delivery's own

- **WHEN** the fixed time a message may wait on an open, the longest this peer waits for delivery to answer one channel creation, and the time delivery allows itself to answer one are compared
- **THEN** the time delivery allows itself is the shortest of the three
- **AND** the fixed time a message may wait on an open is the longest

#### Scenario: An open whose wait has expired still opens its channel when delivery answers

- **WHEN** a message has waited on a Stoa's unanswered channel open until the fixed time passed, a message carrying a valid op for that Stoa is then handed over on its channel identifier while the boundary is held up deciding another payload, and delivery reports the channel created before the boundary is released
- **THEN** that op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A new request for a channel whose wait has expired lets a message wait again

- **WHEN** a message has waited on a Stoa's unanswered channel open until the fixed time passed, the peer then joins that Stoa again, a message carrying a valid op for that Stoa arrives on its channel identifier, and delivery then reports the channel created
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A message waiting when this peer asks delivery for its channel is judged after delivery answers

- **WHEN** a message carrying a valid op for a Stoa begins waiting on that Stoa's channel open before this peer has asked delivery to create the channel, this peer then asks delivery to create it before the message's end has passed, and delivery reports the channel created after the fixed time has passed since the message began waiting, but before it has passed since the ask
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: An earlier message on a queued open does not cost an op that arrives while delivery is asked

- **WHEN** a message begins waiting on a Stoa's channel open before this peer has asked delivery to create the channel, this peer then asks delivery to create it, a message carrying a valid op for that Stoa is then handed over on its channel identifier, and delivery reports the channel created after the fixed time has passed since the first message began waiting, but before it has passed since the ask
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: Asking delivery for a channel whose wait has expired lets a message wait again

- **WHEN** a message has waited on a Stoa's channel open until the fixed time passed, before this peer asked delivery to create the channel, this peer then asks delivery to create it, a message carrying a valid op for that Stoa arrives on its channel identifier, and delivery then reports the channel created
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A second ask while a message waits does not extend its wait again

- **WHEN** a message is waiting on a Stoa's unanswered channel open, and this peer asks delivery to create that channel twice while the message waits, with delivery answering neither ask
- **THEN** the message is refused as arriving on an unknown channel no later than the fixed time after the first of those asks

#### Scenario: An ask made once a message's end has passed does not make it wait again

- **WHEN** a message is waiting on a Stoa's unanswered channel open before this peer has asked delivery to create the channel, its end passes while this peer is held up before judging it, and this peer asks delivery to create that channel after that end and before the message is judged, with delivery answering neither
- **THEN** the message is refused as arriving on an unknown channel
- **AND** it is refused before the fixed time has passed since the ask

#### Scenario: Requests made while a message waits do not lengthen its wait

- **WHEN** a message is waiting on a Stoa's unanswered channel open, and that channel is asked for again and again for as long as the message waits, with none of those requests reported created and none of them yet asked of delivery
- **THEN** the message is refused as arriving on an unknown channel while those requests are still being made
- **AND** the first open is still unanswered when it is refused

#### Scenario: An open this peer gives up without asking delivery does not hold a message up

- **WHEN** the storage that retains sender identifiers cannot be written, a Stoa's channel is to be opened for the first time, and, once the module's log has recorded that Stoa, a message arrives on that Stoa's channel identifier
- **THEN** channel creation is not requested for that Stoa
- **AND** the message is refused as arriving on an unknown channel before the fixed time a message may wait on an open has passed

#### Scenario: A message on a channel not being opened does not wait on another channel's open

- **WHEN** one Stoa's channel open is requested and unanswered, and a message arrives on a channel identifier this peer has neither open nor being opened
- **THEN** that message is refused as arriving on an unknown channel before the open is answered

#### Scenario: A message on an open channel does not wait on a repeated open

- **WHEN** a Stoa's channel is open, its opening is requested again and not yet answered, and a message carrying a valid op for that Stoa arrives on it
- **THEN** the op is stored before the repeated open is answered

#### Scenario: A message on a channel whose open waits behind another is judged once that open settles

- **WHEN** this peer has asked delivery for one Stoa's channel and delivery has not answered, a second Stoa is then joined, a message carrying a valid op for the second Stoa arrives on its channel identifier before delivery has been asked to create that channel, and delivery then reports both channels created
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A restarted peer keeps what delivery hands over before startup asks for its channel

- **WHEN** the module starts while the peer is in a Stoa and delivery has not yet answered node creation, a message carrying a valid op for that Stoa is the first thing delivery hands over after this peer subscribes, and delivery then answers node creation and answers that channel's creation that the channel already exists
- **THEN** the op is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A peer that cannot subscribe still publishes

- **WHEN** subscribing to reliable-channel messages fails at startup, the peer is in a Stoa, and a post is then published into it
- **THEN** the module's log records that this peer will not receive ops from other peers
- **AND** channel creation is requested for the Stoa
- **AND** delivery is asked to send the post

#### Scenario: Only reliable-channel receipts reach the op log

- **WHEN** this application is examined for which delivery events can reach the op log
- **THEN** the only one is a message received on a reliable channel
- **AND** a report on this peer's own send is not among them

### Requirement: Inbound payloads waiting for the boundary are bounded

The payloads this peer holds between delivery handing them over and the boundary deciding them MUST be bounded by a fixed count. Taking a message from delivery MUST NOT wait on the boundary deciding any payload, the refusals below included.

Waiting payloads MUST be decided in the order they arrived. When the bound is reached, a payload arriving MUST be discarded, and the payloads already waiting MUST be kept.

A discarded payload is not stored. Every discard MUST be counted from the module's start, and each MUST be recorded in the module's log with the running count. A discard's log record MUST be distinguishable from a refusal's, and MUST NOT carry the payload or the sender identifier.

**A message on a channel identifier this peer has neither open nor being opened MUST NOT take a place among the waiting payloads.** It MUST be refused as arriving on an unknown channel when delivery hands it over, and recorded in the module's log as that refusal is recorded by "Every payload the reliable channel delivers passes the inbound boundary". It MUST NOT count towards the bound and MUST NOT be counted or logged as a discard. Whether a channel is open or being opened — "being opened" as "Every payload the reliable channel delivers passes the inbound boundary" defines it — is judged when delivery hands the message over.

**A payload larger than the message limit MUST NOT take a place among the waiting payloads either**, on a channel that is open or being opened. It MUST be refused as over-long when delivery hands it over, as "An oversized payload is refused, against a limit pinned at 150 KiB" contracts, and recorded in the module's log as that refusal is recorded. It MUST NOT count towards the bound and MUST NOT be counted or logged as a discard. A payload of exactly the limit is not refused for its size. The payload bytes held waiting are therefore bounded by the bound times 150 KiB, whatever the largest message the node carries. A message on a channel identifier neither open nor being opened is refused as arriving on an unknown channel, as above, whatever its size.

A message refused when delivery hands it over is not among the waiting payloads, and "decided in the order they arrived" does not order its refusal against their decisions.

#### Scenario: The waiting payloads never exceed the bound

- **WHEN** more payloads than the bound arrive on a channel this peer has open while none is decided
- **THEN** the number waiting equals the bound

#### Scenario: A full queue keeps what it holds and discards the arrival

- **WHEN** the bound is reached and one more payload arrives
- **THEN** the payloads decided next are the ones that were waiting, in the order they arrived
- **AND** the arriving payload is not stored

#### Scenario: Every discard is counted and logged apart from refusals

- **WHEN** three payloads are discarded
- **THEN** the running count recorded with the third discard is three
- **AND** each discard's log record is distinguishable from a refusal's

#### Scenario: Traffic on a channel this peer is not opening takes no place in the queue

- **WHEN** the boundary is held up deciding one payload, more messages than the bound then arrive on a channel identifier this peer has neither open nor being opened, and a message carrying a valid op then arrives on a channel this peer has open
- **THEN** each of the messages on the channel identifier this peer is not opening is recorded as refused as arriving on an unknown channel, before the boundary is released
- **AND** no discard is recorded
- **AND** the valid op is stored once the boundary is released

#### Scenario: An oversized payload on an open channel takes no place in the queue

- **WHEN** the boundary is held up deciding one payload, more payloads than the bound then arrive on a channel this peer has open, each one byte larger than the message limit, and a message carrying a valid op then arrives on that channel
- **THEN** each oversized payload is recorded as refused as over-long, before the boundary is released
- **AND** no discard is recorded
- **AND** the valid op is stored once the boundary is released

#### Scenario: A payload at the limit waits its turn

- **WHEN** the boundary is held up deciding one payload, and a payload of exactly the message limit then arrives on a channel this peer has open
- **THEN** no refusal is recorded for it before the boundary is released
- **AND** it is not refused as over-long after the boundary is released

### Requirement: Nothing of a message's wait on an open is kept once it is judged

Once a message that waited on an open, as "Every payload the reliable channel delivers passes the inbound boundary" describes, has been judged, this peer MUST hold no record of that message's wait. This holds whichever way the wait ended — the open settled, the open's time ended while the message waited, or the message was judged at once because that time had already ended — and whether or not the channel is still being opened afterwards.

**The following is a consequence of the requirements above and adds none.** A message waits on an open only while it is the one being judged, so this peer holds a record of at most one message's wait at any time, however many messages a sender puts on a channel being opened and however long that open goes unanswered.

#### Scenario: A message that waited its open's time out leaves no record of its wait

- **WHEN** a message has waited on a Stoa's unanswered channel open until the fixed time passed, and the channel is still being opened
- **THEN** this peer holds no record of that message's wait on that open

#### Scenario: Messages judged at once after an open's time has ended leave no record of their waits

- **WHEN** a message has waited on a Stoa's unanswered channel open until the fixed time passed, and three more messages then arrive on that channel identifier and are each refused as arriving on an unknown channel while the channel is still being opened
- **THEN** this peer holds no more records of messages' waits on that open than it held before those three arrived

#### Scenario: A message whose open settles held leaves no record while another request for the channel is pending

- **WHEN** a Stoa's channel has been requested twice, a message carrying a valid op for that Stoa waits on the open, and delivery reports the first request's channel created while the second request is still pending
- **THEN** the op is stored
- **AND** this peer holds no record of that message's wait on that open while the second request is still pending

### Requirement: The sender identifier this peer supplies is its own, stable, and says nothing about its author

When this peer opens a Stoa's channel, the sender identifier it supplies MUST:

- differ from the one any other installation supplies for that Stoa, including an installation holding the same identity;
- be the same every time this installation opens that Stoa's channel, across restarts;
- differ between two Stoas for one installation;
- be made from nothing that is a public key this peer holds or signs with, or that is computed from one.

**The last property is checked by reading the code that makes a sender identifier, not by comparing an identifier with a key.** Where no key reaches that code, such a comparison cannot fail, and would read as a measurement while measuring nothing. It is an obligation on the code, of the same kind as the prohibition on stopping the node in "The delivery node is shared and is never stopped by this peer". The first property's scenario is the observable evidence beside it: two installations holding one identity supply different identifiers, so the identifier is not a function of that identity.

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

#### Scenario: Nothing a sender identifier is made from is a key

- **WHEN** the code that makes a sender identifier is examined for what it takes as input
- **THEN** none of its inputs is a public key this peer holds or signs with
- **AND** none is a value computed from such a key

#### Scenario: A sender identifier that cannot be retained opens no channel

- **WHEN** the storage that retains sender identifiers cannot be written, and a Stoa's channel is to be opened for the first time
- **THEN** channel creation is not requested for that Stoa
- **AND** the module's log records the Stoa
