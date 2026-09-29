## ADDED Requirements

### Requirement: Creating or joining a Stoa opens its reliable channel

A create or a join that succeeds after the module's startup has wired delivery MUST request that the Stoa's reliable channel be opened. One that succeeds before then MUST NOT request it at that moment; the Stoa is then among those the peer is in, and its channel is requested by startup as the next requirement says, once. The channel identifier and content topic MUST be those `op-transport` derives from the Stoa's address, and the sender identifier MUST be the one `op-transport`'s requirement "The sender identifier this peer supplies is its own, stable, and says nothing about its author" requires. A repeated create or join requests the open again; opening a channel that is already open changes nothing.

The request MUST be made after the membership is recorded. A create or join that is refused MUST NOT request any channel. Requesting the channel is not part of verifying the record, which "Verification consults nothing but the two inputs" governs.

**The reply MUST NOT depend on the channel.** It MUST NOT wait for the channel to open. It MUST NOT report whether the channel opened. It MUST NOT be a failure because the channel did not open: membership is what the user chose, and the network's state at one instant does not change that.

A channel is open only once delivery reports that it created the channel. If delivery answers the creation with its error shape, fails, or does not answer, a channel that was not already open is not open. The module's log MUST record the Stoa and delivery's reason, where delivery gave one. Such a Stoa's channel is requested again at the next module start, or when the Stoa is next created or joined.

**A repeated request that delivery declines, fails or does not answer MUST leave a channel that is already open open.** Delivery created it once, and nothing this change supplies closes it.

#### Scenario: Joining requests the Stoa's channel

- **WHEN** a join succeeds on a peer whose startup has wired delivery
- **THEN** channel creation is requested with the channel identifier and content topic derived from that Stoa's address
- **AND** the request is made after the Stoa appears among the ones the peer is in

#### Scenario: Creating requests the Stoa's channel

- **WHEN** a create succeeds on a peer whose startup has wired delivery
- **THEN** channel creation is requested with the channel identifier and content topic derived from the created Stoa's address

#### Scenario: A refused join requests no channel

- **WHEN** a join is refused because the record does not match the address
- **THEN** no channel creation is requested for the address supplied
- **AND** none is requested for the address the supplied record would name

#### Scenario: A channel delivery declines does not fail the join

- **WHEN** a join succeeds for a Stoa whose channel is not open, and delivery answers the channel creation with its error shape
- **THEN** the join's reply is the one it would have been had the channel opened
- **AND** the peer is in the Stoa
- **AND** the module's log records the Stoa and delivery's reason
- **AND** a message arriving afterwards on that Stoa's channel identifier is refused as arriving on an unknown channel

#### Scenario: An unresponsive delivery does not delay the join

- **WHEN** delivery never answers a channel creation that a join requested
- **THEN** the join's reply is returned without waiting for it

#### Scenario: A repeated join requests the channel again

- **WHEN** a peer whose startup has wired delivery joins a Stoa it is already in
- **THEN** channel creation is requested for that Stoa again

#### Scenario: A declined repeat request leaves an open channel open

- **WHEN** a Stoa's channel is open, the peer joins that Stoa again, and delivery answers the repeated channel creation with its error shape
- **THEN** a message carrying a valid op for that Stoa, arriving afterwards on its channel identifier, is stored
- **AND** it is not refused as arriving on an unknown channel

#### Scenario: A join before delivery is wired has its channel requested once, by startup

- **WHEN** a peer joins a Stoa before the module's startup has wired delivery, and startup then runs
- **THEN** no channel creation is requested before startup
- **AND** channel creation is requested for that Stoa exactly once

### Requirement: Every Stoa the peer is in has its channel opened when the module starts

When the module starts, it MUST request the channel of every Stoa the peer is in, as the requirement above does for one Stoa. It MUST NOT request a channel for any other Stoa, and that includes a Stoa this peer knows of only because it holds ops naming it.

If the record of the Stoas the peer is in cannot be read, no channel is requested, the module's log MUST record the failure, and the module MUST keep answering calls.

**Startup running again in the same module process MUST NOT request any channel.** The first run requested every Stoa the peer was in, and every create or join since has requested its own.

#### Scenario: A restarted peer requests every Stoa's channel

- **WHEN** a peer that created one Stoa and joined another is restarted
- **THEN** channel creation is requested for each of the two
- **AND** for no other Stoa

#### Scenario: A Stoa known only from ops gets no channel

- **WHEN** the op log holds ops naming a Stoa the peer is not in, and the module starts
- **THEN** no channel creation is requested for that Stoa

#### Scenario: An unreadable membership record opens nothing and stops nothing

- **WHEN** the module starts and the record of the Stoas the peer is in cannot be read
- **THEN** no channel creation is requested
- **AND** the module's log records the failure
- **AND** a later call is answered rather than aborting the module

#### Scenario: A second startup in one process requests no channel

- **WHEN** a peer in one Stoa runs the module's startup twice in one module process
- **THEN** channel creation is requested for that Stoa once
