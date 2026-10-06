## ADDED Requirements

### Requirement: A message a reliable channel delivers reaches the op log only through the inbound boundary

Every message delivery hands over as received on a reliable channel MUST be put through the boundary "Every inbound payload is validated before it reaches storage" contracts, with the channel identifier and payload it arrived with, and, unless it was parked, with the sender identifier it arrived with. An op received from the network MUST reach the op log by no other route.

A parked message is put through the boundary without a sender identifier, because none is kept for it, as "Parked messages are kept apart from the op log, survive a restart, and keep no sender identifier" requires. The boundary decides nothing from one, as "The transport's sender identifier is never an identity" requires.

No other delivery event is an arrival. A message delivered outside a reliable channel, and a report on this peer's own send, MUST NOT reach the op log.

**The receive window MUST be judged against this peer's own clock, read when the payload is judged** — for a parked message, when its review judges it — and never against the timestamp the delivery event carries.

Each refusal MUST be recorded in the module's log naming which refusal it was. **The log MUST NOT carry the payload, the sender identifier, or a channel identifier this peer has no channel open under**: whoever sent the message chose each of them, and a channel identifier this peer did not open may belong to another application sharing the node.

A delivered message whose fields cannot be read MUST be discarded and recorded in the module's log, however the reading fails, a panic in the code that reads them included. A refusal, a discarded message, or a failure to store MUST NOT stop the peer from processing the messages that follow.

**A message the op log cannot take is not decided again.** When the op log cannot be opened, or the append fails, the module's log MUST record it as a storage failure, and the message MUST NOT be held for another attempt: its op is in the log afterwards only if it arrives again.

**What happens to a message on a channel that is not open but is being opened is "A message on a channel being opened is parked, and nothing waits on an open".** Delivery can hand over a message on a channel before its answer to the creation reaches this peer. Deciding a message MUST NOT wait on any channel's open, whichever channel the message is on.

**A channel is being opened from the moment a create, a join or the module's startup asks for it, not from the moment delivery is asked, and it stays being opened until every such request for it has settled.** A request settles when delivery reports that it holds the channel — created, or already existing, as `stoa-membership`'s requirement "Creating or joining a Stoa opens its reliable channel" says — when delivery declines or fails the creation, when this peer gives up after asking delivery and receiving no answer, or when this peer gives up the request without asking delivery. An open this peer never goes on to ask delivery for is given up. An open that waits behind other requests this peer has made of delivery is being opened while it waits. Delivery can hand over a message on a channel before this peer has asked delivery for that channel at all: a module restarted while delivery kept running is handed messages on its Stoas' channels from the moment it subscribes, while its own requests for those channels still wait behind node creation and behind each other.

**The longest this peer waits for delivery to answer one channel creation MUST be longer than the time delivery allows itself to answer one.** The order is contracted here; the values are not. With it, this peer does not give up on a creation that delivery would still answer within its own time.

**The module's startup MUST count the channel of every Stoa it asks for as being opened before it checks any message delivery hands over**, whether that check is made on hand-over or when the message is taken from the waiting payloads.

**If this peer cannot subscribe to the messages delivery hands over on reliable channels, the module's log MUST record that this peer will not receive ops from other peers**, and node creation, channel creation and sends MUST still be requested as they would have been otherwise.

#### Scenario: An op another peer published is stored

- **WHEN** a message arrives on a channel this peer has open, carrying a valid op that names that channel's Stoa and whose counter is within the receive window of this peer's clock
- **THEN** the op is in the log
- **AND** its recorded arrival reports it as not ordered by the transport

#### Scenario: The window is judged by this peer's clock, not the event's timestamp

- **WHEN** a message carries a valid op whose counter is more than one hour ahead of this peer's clock when it is judged, and the event's timestamp is later still
- **THEN** the op is refused as ahead of this peer's time
- **AND** it is not stored

#### Scenario: An event timestamp far in the past does not refuse an op within the window

- **WHEN** a message carries a valid op whose counter is within the receive window of this peer's clock when it is judged, and the event's timestamp is decades earlier
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

#### Scenario: This peer's wait on a creation outlasts delivery's own

- **WHEN** the longest this peer waits for delivery to answer one channel creation is compared with the time delivery allows itself to answer one, written as a literal independently of the implementation
- **THEN** this peer's wait is the longer

#### Scenario: An open this peer gives up without asking delivery leaves nothing parked

- **WHEN** the storage that retains sender identifiers cannot be written, a Stoa's channel is to be opened for the first time, and, once the module's log has recorded that Stoa, a message arrives on that Stoa's channel identifier
- **THEN** channel creation is not requested for that Stoa
- **AND** the message is refused as arriving on an unknown channel
- **AND** nothing is parked on that channel afterwards

#### Scenario: A message on a channel not being opened does not wait on another channel's open

- **WHEN** one Stoa's channel open is requested and unanswered, and a message arrives on a channel identifier this peer has neither open nor being opened
- **THEN** that message is refused as arriving on an unknown channel before the open is answered

#### Scenario: A message on an open channel does not wait on a repeated open

- **WHEN** a Stoa's channel is open, its opening is requested again and not yet answered, and a message carrying a valid op for that Stoa arrives on it
- **THEN** the op is stored before the repeated open is answered
- **AND** no park is recorded for it

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

### Requirement: Inbound payloads waiting to be taken are bounded

The payloads this peer holds between delivery handing them over and the boundary taking them MUST be bounded by a fixed count. Taking a message from delivery MUST NOT wait on the boundary deciding any payload, the refusals below included, and MUST NOT wait on any channel's open.

Waiting payloads MUST be taken in the order they arrived.

**When the bound is reached, the newest waiting payload of the channel holding the most waiting payloads, counting the arriving payload with its own channel, MUST be discarded**, and every other payload MUST be kept, the arriving one included whenever it is not the one discarded. Newest means arrived latest. When several channels hold the most, the arriving payload's own channel MUST be the one chosen if it is among them; otherwise the one among them whose newest waiting payload arrived latest. So when every waiting payload is on the arriving payload's channel, the arriving payload is the one discarded. This is the rule "Parked messages are bounded per channel and in total" applies to its total bounds.

A discarded payload is not stored. Every discard MUST be counted from the module's start, and each MUST be recorded in the module's log with the running count. A discard's log record MUST be distinguishable from a refusal's and from a park's, and MUST NOT carry the payload or the sender identifier.

**A message on a channel identifier this peer has neither open nor being opened MUST NOT take a place among the waiting payloads.** It MUST be refused as arriving on an unknown channel when delivery hands it over, and recorded in the module's log as that refusal is recorded by "A message a reliable channel delivers reaches the op log only through the inbound boundary". It MUST NOT count towards the bound and MUST NOT be counted or logged as a discard. Whether a channel is open or being opened — "being opened" as "A message a reliable channel delivers reaches the op log only through the inbound boundary" defines it — is judged when delivery hands the message over.

**A payload larger than the message limit MUST NOT take a place among the waiting payloads either**, on a channel that is open or being opened. It MUST be refused as over-long when delivery hands it over, as "An oversized payload is refused, against a limit pinned at 150 KiB" contracts, and recorded in the module's log as that refusal is recorded. It MUST NOT count towards the bound and MUST NOT be counted or logged as a discard. A payload of exactly the limit is not refused for its size. The payload bytes held waiting are therefore bounded by the bound times 150 KiB, whatever the largest message the node carries. A message on a channel identifier neither open nor being opened is refused as arriving on an unknown channel, as above, whatever its size.

A message refused when delivery hands it over is not among the waiting payloads, and "taken in the order they arrived" does not order its refusal against what happens to them.

#### Scenario: The waiting payloads never exceed the bound

- **WHEN** more payloads than the bound arrive on a channel this peer has open while none is taken
- **THEN** the number waiting equals the bound

#### Scenario: A full queue on one channel keeps what it holds and discards the arrival

- **WHEN** every waiting payload is on one channel, the bound is reached, and one more payload arrives on that channel
- **THEN** the payloads taken next are the ones that were waiting, in the order they arrived
- **AND** the arriving payload is not stored

#### Scenario: A full queue discards the newest payload of the channel holding the most

- **WHEN** the boundary is held up deciding one payload, payloads arrive on one open channel until it holds more waiting payloads than any other and the bound is reached, and a message carrying a valid op then arrives on a second open channel holding fewer
- **THEN** the payload that arrived last on the first channel is recorded as discarded
- **AND** the valid op is stored once the boundary is released

#### Scenario: The arriving payload's channel loses a tie for the most

- **WHEN** the bound is reached, and a payload arrives on an open channel that, counting the arrival, holds as many waiting payloads as another channel holds, with no channel holding more
- **THEN** the arriving payload is the one discarded
- **AND** every payload that was already waiting is kept

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

### Requirement: A message on a channel being opened is parked, and nothing waits on an open

When the boundary takes a payload from the waiting payloads, its channel's state at that instant MUST decide what happens to it. The state is read once for each payload, and the payload is parked or judged on that one reading:

- **The channel is open**, whether or not a further request for it is also unsettled: the payload MUST be judged at once, as "Every inbound payload is validated before it reaches storage" contracts.
- **The channel is not open, and is being opened**, as "A message a reliable channel delivers reaches the op log only through the inbound boundary" defines it: the payload MUST be **parked**. It MUST NOT be judged, refused or stored then, and the boundary MUST go on to the next waiting payload without waiting for that open to settle.
- **The channel is neither open nor being opened**: the payload MUST be judged at once, and the boundary refuses it as arriving on an unknown channel.

Nothing other than the boundary taking a waiting payload parks a message. A message refused when delivery hands it over, as "Inbound payloads waiting to be taken are bounded" contracts, is never parked.

**Parking judges nothing.** A parked message MUST NOT be refused for any reason before it is reviewed, save a discard under "Parked messages are bounded per channel and in total". A payload that does not decode, or that carries an op that does not verify, names another Stoa, or is ahead of this peer's time, is parked like any other, and refused, if it is, by its review.

Each park MUST be recorded in the module's log as a park, distinguishably from a refusal, a discard and a store, and the record MUST NOT carry the payload or the sender identifier.

If the parked messages cannot be written, the payload MUST NOT be parked, the module's log MUST record a storage failure, and the payload MUST NOT be held for another attempt.

#### Scenario: A message on a channel being opened is parked, not judged

- **WHEN** this peer has requested a Stoa's channel and delivery has not answered, and a message carrying a valid op for that Stoa arrives on its channel identifier and is taken
- **THEN** the module's log records it as parked
- **AND** the op is not in the op log
- **AND** no refusal is recorded for it while delivery has still not answered

#### Scenario: An unanswered open holds up no other channel

- **WHEN** this peer has requested one Stoa's channel and delivery never answers, three messages arrive one after another on that channel identifier, and a message carrying a valid op then arrives on a channel this peer has open
- **THEN** the valid op is stored while delivery has still not answered the first open
- **AND** each of the three is recorded as parked
- **AND** none of the three is recorded as refused while delivery has still not answered

#### Scenario: A payload that does not decode is parked, and refused by its review

- **WHEN** a payload the op decoder does not accept arrives on a channel being opened and is taken, and delivery then reports the channel created
- **THEN** it is recorded as parked, with no refusal recorded for it before delivery's answer
- **AND** after delivery's answer it is refused as not decoding

#### Scenario: A message taken after its open settled held is judged, not parked

- **WHEN** the boundary is held up deciding one payload, a message carrying a valid op is then handed over on a channel being opened, delivery reports that channel created, and the boundary is then released
- **THEN** the op is stored
- **AND** no park is recorded for it

#### Scenario: A message taken after its open was declined is refused, not parked

- **WHEN** the boundary is held up deciding one payload, a message carrying a valid op is then handed over on a channel being opened, delivery answers that channel's creation with its error shape for a reason other than that the channel already exists, and the boundary is then released
- **THEN** the message is refused as arriving on an unknown channel
- **AND** no park is recorded for it

#### Scenario: A message that cannot be parked is logged and not retried

- **WHEN** the parked messages cannot be written, a message carrying a valid op is taken on a channel being opened, the parked messages can then be written again, and delivery then reports the channel created
- **THEN** the module's log records a storage failure
- **AND** the op is not in the op log afterwards
- **AND** nothing is parked on that channel afterwards

### Requirement: Parked messages are reviewed on three events and no others

A **review** of a channel decides the messages parked on it. Exactly three events MUST begin one, and nothing else does: not the passing of time, not the arrival or taking of a message, not a create, join or startup request for a channel, not a send, and not closing a channel.

1. **Delivery reports that it holds the channel**, in answer to any request for it: it created the channel, or the channel already exists. Each message parked on that channel MUST then be judged at the boundary as a message taken on an open channel is — against the channels open when it is judged and this peer's clock read then — and stored, refused for whichever reason the boundary gives, or found already held.
2. **The channel stops being opened without being open**: the last unsettled request for it settles in some way other than delivery reporting that it holds the channel, and the channel is not open. Each message parked on that channel MUST then be refused as arriving on an unknown channel, and recorded in the module's log as that refusal is recorded.
3. **The module's first startup in a module process**, once it has counted the channel of every Stoa it asks for as being opened. Each parked message whose channel is then neither open nor being opened MUST be refused as arriving on an unknown channel, and recorded as that refusal is recorded. A message parked on a channel startup counts as being opened MUST be left parked, for that channel's next event of the first or second kind. A later startup in the same module process begins no review.

A request that settles while another request for the same channel is still unsettled, in any way other than delivery reporting that it holds the channel, MUST leave that channel's parked messages parked.

**The receive window is judged at review.** A parked op's counter MUST be compared with this peer's clock read when its review judges it, and never with the clock when it arrived or when it was parked.

**Messages on one channel that take a place among the waiting payloads are decided in the order delivery handed them over, whether or not they were parked.** A message refused when delivery hands it over is not among them. A review MUST decide a channel's parked messages in the order they were handed over, and MUST decide all of them before any message on that channel taken after the event that began the review is judged. Deciding a channel's parked messages MUST NOT wait on any other channel's open. No order is promised between messages on different channels.

**If the parked messages cannot be read at a review**, the module's log MUST record a storage failure, and the messages that review could not read MUST stay parked, to be decided by the channel's next review. The order above is not promised between them and the messages on their channel judged in the meantime. A parked message whose op the op log cannot take at its review is a storage failure as "A message a reliable channel delivers reaches the op log only through the inbound boundary" says, and it MUST NOT be parked afterwards.

A module stopping part-way through a review is outside this requirement, which contracts reviews that run to their end.

#### Scenario: Messages parked on an unanswered open are stored once delivery reports the channel created

- **WHEN** three messages carrying distinct valid ops for a Stoa are parked on its channel while its open is unanswered, and delivery then reports the channel created
- **THEN** all three ops are stored
- **AND** nothing is parked on that channel afterwards

#### Scenario: A parked message is stored once delivery reports the channel already exists

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel while its open is unanswered, and delivery then answers the creation that the channel already exists
- **THEN** the op is stored

#### Scenario: Messages parked on an open delivery declines are refused as an unknown channel

- **WHEN** two messages carrying valid ops for a Stoa are parked on its channel while its only request is unanswered, and delivery then answers the creation with its error shape, for a reason other than that the channel already exists
- **THEN** each is refused as arriving on an unknown channel
- **AND** neither refusal is recorded before delivery's answer
- **AND** nothing is parked on that channel afterwards

#### Scenario: Messages parked on an open this peer gives up are refused once it is given up

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel, delivery never answers the creation, and this peer then gives up waiting for the answer
- **THEN** the message is refused as arriving on an unknown channel
- **AND** the refusal is recorded after this peer gave up, not before
- **AND** nothing is parked on that channel afterwards

#### Scenario: A declined request leaves parked messages parked while another request is unsettled

- **WHEN** a Stoa's channel is not open and has been requested twice, a message carrying a valid op for that Stoa is parked on it, delivery answers the first request with its error shape for a reason other than that the channel already exists, and then reports the second request's channel created
- **THEN** no refusal is recorded for the message after the first answer
- **AND** the op is stored after the second answer

#### Scenario: Nothing but the three events reviews a parked message

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel while its open is unanswered, and then, with delivery still not answering and before this peer gives up on any request for that channel, further messages arrive on that channel and on a channel this peer has open, the Stoa is joined again, and a post is published into another Stoa
- **THEN** no store and no refusal is recorded for the parked message
- **AND** it is still parked

#### Scenario: A parked op is judged against this peer's clock at its review

- **WHEN** a message carrying a valid op whose counter is more than one hour ahead of this peer's clock is parked, this peer's clock then advances until the counter is less than one hour ahead of it, and delivery then reports the channel created
- **THEN** the op is stored

#### Scenario: A parked op that has come to be ahead of this peer's time is refused at its review

- **WHEN** a message carrying a valid op whose counter is within the receive window of this peer's clock is parked, this peer's clock then moves back so that the counter is more than one hour ahead of it, and delivery then reports the channel created
- **THEN** the op is refused as ahead of this peer's time
- **AND** it is not stored

#### Scenario: Parked messages are decided before later messages on their channel, in hand-over order

- **WHEN** messages carrying valid ops A and then B for a Stoa are parked on its channel, delivery then reports the channel created, and a message carrying a valid op C for that Stoa is then taken on the channel
- **THEN** the module's log records A's store, then B's, then C's, in that order

#### Scenario: Startup refuses a parked message on a channel it does not open

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel, the module stops before that open settles, and the module starts again with the record of the Stoas the peer is in unreadable, so that no channel is requested
- **THEN** the message is refused as arriving on an unknown channel
- **AND** nothing is parked afterwards

#### Scenario: Startup leaves a parked message on a channel it opens for that open's answer

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel, the module stops before that open settles, and the module starts again while the peer is in that Stoa, with delivery not yet answering startup's creation of that channel
- **THEN** no store and no refusal is recorded for the message before delivery answers that creation

#### Scenario: Parked messages a review could not read are decided by the channel's next review

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel, the parked messages cannot be read when delivery reports the channel created, they can be read again afterwards, the Stoa is then joined again, and delivery answers the repeated creation that the channel already exists
- **THEN** the module's log records a storage failure at the first answer
- **AND** the op is stored after the second answer

### Requirement: Every message is decided exactly once, however close to a settle it is taken

Each message that takes a place among the waiting payloads MUST be decided exactly once: judged when it is taken; or parked, and then decided by exactly one review; or discarded under a bound, while waiting or while parked; or, when it cannot be parked, recorded as a storage failure as "A message on a channel being opened is parked, and nothing waits on an open" says.

**The taking of a message and an event that begins a review of its channel MUST be ordered, one before the other.** A message taken before that event and parked MUST be decided by the review that event begins, however short the time between its being taken and the event. A message taken after that event MUST be parked, judged or refused by the channel's state after the event, as "A message on a channel being opened is parked, and nothing waits on an open" says.

A message a review has decided MUST NOT be parked afterwards and MUST NOT be decided by a later review. A message discarded from the parked messages MUST NOT be decided by any review.

#### Scenario: Messages taken around a settle are each decided once

- **WHEN** messages carrying distinct valid ops for a Stoa are handed over on its channel one after another, starting while this peer's only request for that channel is unanswered and continuing after delivery reports the channel created
- **THEN** every one of those ops is stored
- **AND** the module's log records one store for each, and records none of them as already held
- **AND** nothing is parked on that channel afterwards

#### Scenario: A message parked just before its open settles is decided by that settle's review

- **WHEN** a message carrying a valid op for a Stoa is taken and parked on its channel, and delivery reports that channel created before any other message is taken
- **THEN** the op is stored
- **AND** nothing is parked on that channel afterwards

#### Scenario: A message a review decided is not decided again

- **WHEN** a message carrying a valid op is parked, delivery reports its channel created and the op is stored, the Stoa is then joined again and delivery answers that the channel already exists, and the module is then restarted and delivery answers startup's creation of that channel that it already exists
- **THEN** the module's log records that op's store once
- **AND** records no decision for that message after its store

### Requirement: Parked messages are bounded per channel and in total

Four fixed bounds MUST hold over the parked messages at every moment, what survived a restart included:

- the number of messages parked on one channel;
- the payload bytes parked on one channel;
- the number of messages parked in all;
- the payload bytes parked in all.

Each per-channel bound MUST be no greater than the matching total bound. Each byte bound MUST be at least 150 KiB, the message limit "An oversized payload is refused, against a limit pinned at 150 KiB" pins, so that a payload at the limit can be parked on a channel that holds nothing parked. The values are not contracted here.

**When parking a payload would put its own channel over its count bound or its byte bound, that payload MUST be discarded**, and nothing already parked is.

**Otherwise, while parking it would put the parked messages over a total bound, the newest parked message of the channel holding the most of what that bound counts MUST be discarded** — messages for the count bound, payload bytes for the byte bound — counting the payload being parked with its own channel. The count bound is restored first, then the byte bound. Newest means handed over latest. When several channels hold the most, the payload's own channel MUST be the one chosen if it is among them; otherwise the one among them whose newest parked message was handed over latest. When the message chosen is the payload being parked, it is discarded and nothing more is.

A discarded parked message is not stored, and MUST NOT be decided by any review. Every discard from the parked messages MUST be counted from the module's start in the same running count as discards from the waiting payloads, and recorded in the module's log with that count. Its record MUST be distinguishable from a discard from the waiting payloads, from a park and from a refusal, and MUST NOT carry the payload or the sender identifier.

#### Scenario: A channel at its count bound discards the arrival and keeps what it parked

- **WHEN** as many messages carrying valid ops as the per-channel count bound are parked on one channel, one more message carrying a valid op is then taken on it while its open is still unanswered, and delivery then reports the channel created
- **THEN** that last message is recorded as discarded from the parked messages
- **AND** every message parked before it is stored
- **AND** its op is not stored

#### Scenario: A channel at its byte bound discards the arrival

- **WHEN** fewer messages than the per-channel count bound are parked on one channel, one more payload, which on its own fits the per-channel byte bound, would put that channel's parked payload bytes over it, and that payload is then taken on the channel while its open is still unanswered
- **THEN** that payload is recorded as discarded from the parked messages
- **AND** every message parked before it is still parked

#### Scenario: Over a total bound, the channel holding the most gives up its newest

- **WHEN** the parked messages reach the total count bound with one channel holding more parked messages than any other, and a message is then taken on a different channel being opened, which, counting that message, holds fewer
- **THEN** the message handed over last of those parked on the channel holding the most is recorded as discarded from the parked messages
- **AND** the message taken is recorded as parked

#### Scenario: The arriving payload's channel loses a tie for the most parked

- **WHEN** the parked messages reach the total count bound, and a message is then taken on a channel being opened that, counting that message, holds as many parked messages as another channel holds, with no channel holding more, and is within its own channel's bounds
- **THEN** the message taken is recorded as discarded from the parked messages
- **AND** every message that was parked before it is still parked

#### Scenario: Messages that survived a restart count towards the bounds

- **WHEN** as many messages as the per-channel count bound are parked on one Stoa's channel, the module stops before that open settles and starts again while the peer is in that Stoa, and one more message is taken on that channel before startup's open of it settles
- **THEN** that message is recorded as discarded from the parked messages

#### Scenario: The bounds are ordered as required

- **WHEN** the four bounds are compared with each other and with 150 KiB written independently of them
- **THEN** each per-channel bound is no greater than the matching total bound
- **AND** each byte bound is at least 150 KiB

#### Scenario: A discard from the parked messages shares the running count and is logged apart

- **WHEN** one payload is discarded from the waiting payloads, and then one message is discarded from the parked messages
- **THEN** the running count recorded with the second discard is two
- **AND** the two discards' records are distinguishable from each other and from a refusal's

### Requirement: Parked messages are kept apart from the op log, survive a restart, and keep no sender identifier

Parked messages MUST be kept in storage that outlives the module process. A message parked when the module stops MUST still be parked when it starts again, until a review decides it or a bound discards it.

For each parked message, this peer MUST keep its payload, the channel identifier it arrived on, and its place in the order delivery handed messages over. It MUST NOT keep the sender identifier or the timestamp the delivery event carried.

**A parked message is not an op this peer holds.** Until a review stores it:

- no read of the op log, and no reply the module gives to any call, MUST carry it or anything decoded from it;
- it MUST NOT change any Stoa's Lamport clock, which `op-ordering`'s requirement "A peer's Lamport clock is a function of the ops it holds" makes a function of the ops held;
- a judgement of an arriving message MUST NOT report its op as already held on account of the parked message.

#### Scenario: A parked op is not readable as an op

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel
- **THEN** no read of the op log returns that op
- **AND** no read of that Stoa's posts through the module's surface returns it

#### Scenario: A parked op does not move its Stoa's clock

- **WHEN** a message carrying a valid op for a Stoa, whose counter is above every counter this peer holds for that Stoa, is parked on that Stoa's channel, and this peer's Lamport clock for that Stoa is then read while the open is still unanswered
- **THEN** the clock is the value it had before the message was parked

#### Scenario: A parked op is not counted as held

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel, and delivery then reports the channel created
- **THEN** the module's log records the op as stored
- **AND** does not record it as already held

#### Scenario: A message parked before a restart is stored once startup's open is answered

- **WHEN** a message carrying a valid op for a Stoa is parked on its channel, the module stops before that open settles, the module starts again while the peer is in that Stoa, and delivery answers startup's creation of that channel that it already exists
- **THEN** the op is stored

#### Scenario: Nothing parked carries the sender identifier or the event's timestamp

- **WHEN** a message carrying a distinctive sender identifier and a distinctive timestamp is parked, and what is kept for it is read back from storage
- **THEN** neither the sender identifier nor the timestamp appears in it

## REMOVED Requirements

### Requirement: Every payload the reliable channel delivers passes the inbound boundary

**Reason**: A message on a channel being opened no longer waits for that open; it is parked. Thirteen of this requirement's scenarios pin the wait — its fixed time per open, the ask starting that time again, the once-only extension, and the requests that do not lengthen it — and a MODIFIED block cannot drop a scenario, so the requirement is replaced rather than amended.

**Migration**: Replaced by "A message a reliable channel delivers reaches the op log only through the inbound boundary", added in this same change. Its text is this requirement's with these edits: a parked message is put through the boundary without a sender identifier; the receive window is read when the payload is judged, which for a parked message is at its review; the paragraphs on the wait, its fixed time, the ask restarting it, the request that does not lengthen it, the bound on how long it holds other channels, and an expired wait are replaced by one paragraph pointing at "A message on a channel being opened is parked, and nothing waits on an open" and forbidding any decision to wait on an open; "being opened" now says it lasts until every request for the channel has settled, and lists how a request settles; of the three-way order, only "this peer's wait on a creation outlasts delivery's own" is kept. Scenarios kept unchanged: every one not about the wait, save that "when it is processed" reads "when it is judged" in the two window scenarios, and "A message on an open channel does not wait on a repeated open" gains that no park is recorded. "A message's wait outlasts this peer's wait on a creation, which outlasts delivery's own" becomes "This peer's wait on a creation outlasts delivery's own". "An open this peer gives up without asking delivery does not hold a message up" becomes "An open this peer gives up without asking delivery leaves nothing parked". The other eleven wait scenarios are dropped; what they protected — an op racing its channel's creation is not lost, and one stuck open does not hold up other Stoas — is now held by the scenarios of the five parking requirements added in this change. Within this capability only this requirement, the queue requirement and the removed requirement on wait records cited the name, and all three are replaced or removed here.

### Requirement: Inbound payloads waiting for the boundary are bounded

**Reason**: A full queue no longer always discards the arriving payload; it discards the newest payload of the channel holding the most, the rule the parked messages' total bounds use. The scenario "A full queue keeps what it holds and discards the arrival" states the old rule in its name, and a MODIFIED block cannot drop or rename a scenario, so the requirement is replaced rather than amended.

**Migration**: Replaced by "Inbound payloads waiting to be taken are bounded", added in this same change. Its text is this requirement's with these edits: payloads are held until the boundary *takes* them, and are taken, rather than decided, in arrival order, since a taken payload may be parked; taking a message from delivery also MUST NOT wait on any channel's open; the discard rule is the newest payload of the channel holding the most, counting the arrival, with its tie-break; a discard's record is also distinguishable from a park's. "A full queue keeps what it holds and discards the arrival" becomes "A full queue on one channel keeps what it holds and discards the arrival", restricted to one channel, where the old and new rules agree. "A full queue discards the newest payload of the channel holding the most" and "The arriving payload's channel loses a tie for the most" are added. Every other scenario is kept, "while none is decided" reading "while none is taken".

### Requirement: Nothing of a message's wait on an open is kept once it is judged

**Reason**: No message waits on an open any more. A message on a channel being opened is parked and the boundary moves on, so there is no wait whose record could be kept.

**Migration**: What is kept for a message on a channel being opened is now "Parked messages are kept apart from the op log, survive a restart, and keep no sender identifier", bounded by "Parked messages are bounded per channel and in total", and released by "Parked messages are reviewed on three events and no others". Tests pinning the per-message wait records are removed with the waits.
