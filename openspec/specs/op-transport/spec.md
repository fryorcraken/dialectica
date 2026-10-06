# op-transport Specification

## Purpose
Defines how a Stoa's signed ops move between peers: how the channel every peer in a Stoa must find is identified, what publishing a locally-authored op means, what an inbound payload must survive before it reaches storage, and what the transport tells a receiving peer that the peer must not believe.

The boundary with three neighbouring capabilities is drawn deliberately, and the substance is not restated here.

`op-format` owns what an op is, how it encodes, and what its decoder refuses. This capability requires that an inbound payload be put through that decoder before anything else looks at it, and does not repeat the decoder's own refusal list.

`identity` owns authenticity — whether an op verifies under the public key it carries as its author, since the key is the author's identity and there is no separate identifier for it to be bound to. This capability requires that check on the receive path and names nothing about how it works.

`op-ordering` owns what orders two ops, including the degraded order that applies when the transport supplied no ordering metadata. This capability contracts only what a receiving peer records about an arrival, and that this transport supplies nothing to record.

`op-log` owns what happens once an op is appended — that the log stores it, decides nothing about it, holds one entry per op identifier, and does not verify a signature on append. **That is not in tension with this capability's refusals, and the boundary is worth stating because it reads as though it were.** `op-log` contracts what the log does with an op it is handed; this capability contracts which inbound payloads become an op that is handed to it. A log that refused on append would be deciding validity at a point where the peer's knowledge is incomplete, which is why it does not; a transport boundary that appended whatever arrived would be storing forgeries on behalf of any peer that sent them, which is why it does not. Nothing here requires the log to check anything, and nothing in `op-log` licenses this boundary to skip a check.

**What is outside this capability**, named rather than covered because most of it needs a running delivery node and two live peers to observe, and a requirement nothing can check is worse than an acknowledged gap:

- **That two peers deriving one Stoa's channel identity actually meet.** The derivation being a pure function of the Stoa address is contracted below, held by what it accepts and pinned against an independently derived value; that two processes on a network consequently exchange ops is a property of the node. **Comparing one peer's derivation against another peer's** is also outside reach, which is why the requirement below says how it is held instead of asking for a comparison no peer can make.
- **That a published op reaches another peer** — retransmission of an unacknowledged message, recovery of messages missed while offline, and how far back a repairing peer reaches. That is the transport's own reliability, and none of it is observable from this capability's surface. **Excluding the observation is not excluding the obligation**: the requirement "A successful publish is a statement about the local log and nothing more" contracts what a publish may claim, and names the three things still owed for surfacing an op that never propagated — a bound, and what a peer records for an op in flight and for one that never propagated. Those are owed here and are not met by this change.
- **That closing a channel released anything on the network.** The release is reference-counted across channels on a content topic and its failures are not surfaced to the caller, so a peer cannot observe the outcome. What is contracted below is that a peer does not claim otherwise.
- **Whether a hashed content topic buys the anonymity intended.** That a Stoa's title does not appear in a topic is checkable; the size of the set a hashed topic hides a peer within is a property of the deployed network.
- **That the message-size limit contracted below equals what the network validates against.** The limit arrives from no transport interface — nothing in the delivery contract states one — so a peer has no second value to compare its own against. The requirement below pins the constant so a local edit fails loudly; whether the network has since moved is observable only against a live node, and a peer over-sending or needlessly refusing because of a drifted network limit is a failure this capability cannot detect.
- **The node's configuration** — which network, which mode, which entry nodes — beyond the lifecycle contracted below.
- **Observing that the shared delivery node was left running.** The node is never reached from this capability's own surface, so the requirement "The delivery node is shared and is never stopped by this peer" is contracted below as a prohibition on what this application may call, and is discharged by reading it. What it would take to observe the failure is a second module in the same context losing its delivery, which is a property of a deployed context. This is named here rather than left to read as covered.
- **The lifecycle handlers that must drive a close.** This application has no shutdown handler and no leave-Stoa handler, so the two events the requirement "A channel is closed when a user leaves a Stoa and on shutdown" names have no site at which they are observed. **Excluding the site is not excluding the obligation**: that requirement states the obligation and contracts the two close operations a handler will need, and the requirement "The delivery node is shared and is never stopped by this peer" names the handlers as a fourth owed thing beside the three owed for a delivery outcome. Supplying them is not this change's work, and no scenario below is phrased over either event, because one phrased over an event with no site would pass by never firing.
- **Attachments and any content addressed outside an op.** A payload here is one op's wire form.
- **What a channel is opened for.** Which Stoas a peer holds, what joining one means, and where a Stoa's genesis record is stored belong to the Stoa-lifecycle capability. This capability contracts only that an inbound op never causes a peer to hold a Stoa it did not already hold.

## Requirements

### Requirement: A Stoa's ops travel on one reliable channel per Stoa

A Stoa's ops SHALL travel on exactly one reliable channel, and that channel SHALL carry nothing but signed ops of that Stoa.

The channel SHALL be identified by a channel identifier and a content topic, both derived from the Stoa's address. The channel identifier names the conversation every peer in the Stoa must join; the content topic names what the underlying network filters on.

A thread identifier, a parent op identifier, an author, or any other property of an individual op SHALL NOT appear in either the channel identifier or the content topic. Every such value is already inside the signed op.

The content topic SHALL NOT contain a Stoa's human-readable title or any other human-readable name. A content topic is disclosed to peers that serve filtering, storage and forwarding, so a readable name in one links a network address to an interest.

**The content topic and the channel identifier SHALL each begin with the literal prefix `/dialectica/1/`, and that prefix SHALL NOT be changed as though it were a naming choice.** The network's autosharding places a topic by hashing only the application and version segments of its name and ignores the rest, so this prefix — and nothing further along the string — is what puts every dialectica Stoa on one shard. Changing it, shortening it, or bumping the version segment moves every Stoa that adopts the change to a different shard from every Stoa that has not, which is the same silent partition the next requirement exists to prevent, arrived at from the other direction. A change to this prefix is a network migration and SHALL be treated as one.

**The content topic SHALL also be a name the network's content-topic rule accepts, and that rule SHALL read it with `dialectica` as its application and `1` as its version.** The content topic SHALL take that rule's four-part form: a leading `/`, then exactly four non-empty parts separated by `/`, read as `/<application>/<version>/<name>/<encoding>`. It SHALL NOT take the rule's generation-bearing form, in which a numeric generation precedes the application. Beginning with the literal prefix does not meet this on its own: a name that begins with `/dialectica/1/` and has any other number of parts is refused by that rule. A name that begins with the prefix and is accepted is read with `dialectica` as its application and `1` as its version, so that half of the rule restates the prefix rather than adding to it.

This rule binds the content topic alone. Nothing here constrains the channel identifier's shape beyond the prefix.

**What is checkable here is the content topic against that rule, and not a deployed node's acceptance of it.** That a running node still applies the rule is observable only against a live node, and is outside this capability in the way the message-size limit's agreement with the network is.

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

#### Scenario: Both names keep the prefix autosharding reads

- **WHEN** channel identity is derived for any Stoa
- **THEN** the content topic begins with the literal `/dialectica/1/`
- **AND** the channel identifier begins with the same literal
- **AND** the check is against that literal rather than against whatever the implementation currently produces

#### Scenario: The network's content-topic rule reads the content topic as dialectica version 1

- **WHEN** channel identity is derived for any Stoa, and its content topic is put through the network's content-topic rule
- **THEN** the rule accepts it in the four-part form, with no generation
- **AND** the rule reads `dialectica` as its application and `1` as its version
- **AND** the same rule refuses a name that begins with `/dialectica/1/` and has five parts
- **AND** the check applies that rule as written independently of the derivation, rather than comparing against whatever the implementation currently produces

### Requirement: Channel identity is a pure function of the Stoa address

The channel identifier and the content topic SHALL each be a pure function of the Stoa's address. Nothing else SHALL participate: not a session counter, not an epoch, not a local sequence number, not a device identifier, not a wall-clock reading, not the number of times this peer has opened the channel, and not the peer's own identity.

**This requirement exists because its violation produces no error.** Two peers computing different channel identifiers for one Stoa do not fail, do not warn and do not retry: each opens a channel nobody else is in, and the two never see one another's ops. The partition is silent and permanent, and no participant can observe it from inside. Every other refusal in this capability reports itself; this one cannot, so it is held before a peer runs rather than reported after — by what the derivation accepts, and by a pin against a value nothing in this implementation produced. Both are stated below.

A deterministic epoch SHALL NOT be introduced as a variant of this. An epoch that every peer must recompute identically is a rendezvous problem at every boundary where it changes, and it reintroduces the same silent partition at each one.

Re-deriving channel identity for a Stoa already joined SHALL yield the same values it yielded before.

**How this requirement is held, since the peer-to-peer half of it is not observable from one peer.** The values that differ *between* peers — a peer's own identity, a device identifier, a process or installation identifier — cannot be varied by a peer inspecting its own derivation: a peer has one of each, and a derivation consulting one would agree with itself every time it was asked. The requirement is therefore discharged in two parts, and both are obligations:

- **By construction.** The derivation SHALL take the Stoa's address as its only input, so that there is no parameter through which a per-peer or per-session value could enter. A derivation reaching for such a value outside its inputs violates this requirement whether or not any peer can observe the result.
- **By a pinned known answer.** The derivation's output for a fixed Stoa address SHALL equal a value derived independently of this implementation, so that any change to what participates — including one adding a per-peer input — fails loudly at the pin rather than passing every self-comparing check. That pin is the scenario "The derivation is pinned against silent change" below, and it is what makes a construction change visible.

The part a peer *can* vary, and SHALL find makes no difference, is its own history: how many times it has opened or closed the channel, how many ops it holds, and how much time has passed between two derivations.

#### Scenario: Deriving twice yields the same identity

- **WHEN** channel identity is derived twice for one Stoa address
- **THEN** both derivations yield the same channel identifier
- **AND** both yield the same content topic

#### Scenario: Identity does not vary with the peer's history

- **WHEN** channel identity is derived for one Stoa address, then derived again after that peer's history has changed around it — channels opened and closed, ops stored, and time passed between the two derivations
- **THEN** both derivations yield the same channel identifier and content topic

#### Scenario: The derivation takes the Stoa address and nothing else

- **WHEN** the derivation is examined for what it accepts
- **THEN** the Stoa's address is its only input
- **AND** there is no parameter through which a peer identity, a device or process identifier, a session value or a clock reading could reach it

#### Scenario: Reopening a channel does not change its identity

- **WHEN** a channel for a Stoa is closed and its identity is derived again
- **THEN** the channel identifier is the one derived before the close
- **AND** it carries no epoch, counter or generation distinguishing the second derivation from the first

#### Scenario: The derivation is pinned against silent change

- **WHEN** channel identity is derived from a fixed Stoa address
- **THEN** the channel identifier and content topic equal values derived independently of this implementation
- **AND** a change to the derivation fails this rather than passing quietly
- **AND** a derivation that had come to consult a value differing between peers fails it too, since such a value cannot equal the independently derived one

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

### Requirement: A successful publish is a statement about the local log and nothing more

A publish reported as successful SHALL mean that the op is in the local op log. That is the whole of what it means: **it SHALL NOT be read as a statement that the transport accepted the bytes, and SHALL NOT be read as a statement that any peer received the op.** No field of what a publish reports SHALL carry, imply or be documented as carrying a delivery outcome.

Publishing and delivering are two events at two times. The transport's own outcomes — that it accepted a message, that a message propagated, that sending it errored — arrive after the call has returned and are keyed by a handle the send produced, so a publish's reply is not merely silent about delivery; it is structurally incapable of carrying it. A reply shaped as though it could would be a worse signal than no signal, because "the transport accepted this" reads as "this arrived" while meaning only the former.

**A publish SHALL NOT be reported as having failed on the strength of a delivery outcome, and SHALL NOT be reported as having failed because the handoff to the transport failed.** An op in the log is published; whether its bytes reached the transport, and whether the transport then reached the network, are both separate facts, and reporting a failure for either would tell a caller to retry or discard something that already exists.

The two failures differ in when they are knowable and in nothing else that matters here. A refused handoff is known before the call returns; a delivery outcome arrives after the call is over. **Neither may be reported as a failed publish**, because the op is in the log in both cases and the previous requirement makes that the thing a failure would be lying about. The earlier requirement's distinguishable "transport failure on an open channel" is a distinction *between refusals of a publish that never stored anything* — there is no open channel, so nothing was published — and is not a licence to report a stored op's handoff failure as a failed publish.

**Where the handoff rule binds is not this capability's own surface.** This capability's publish path hands out the bytes to send rather than sending them, so no handoff failure is reachable from it and there is no branch here that could undo the append — the rule is structural rather than checked, which is what the last scenario below records. Reporting it correctly falls to whichever caller performs the send, and the content-authoring capability contracts that reply. What this requirement adds is that nothing here supplies a route by which such a failure could remove a stored op or be dressed as a failed publish.

**The delivery outcome is an obligation, not an absence, and it is this capability's obligation to specify.** An op the transport reported an error for, or that never propagated within some bound, has to become visible to the user, or a publish that claims only the local log converts a loud failure into a silent one. **Three things are consequently owed and are not supplied by anything below: the bound, what a peer records for an op in flight, and what it records for one that never propagated.**

**None of them is discharged by this change**, and that is a scope statement rather than an impossibility. Meeting the obligation needs state outliving a publish call — an association from the transport's send handle to the op it sent — and a clock to bound the wait, so it is a component rather than a branch on the path contracted above; a clock is also something this capability has deliberately never held. What is ruled out here is only that a publish's reply could carry the answer, for the reason given above. Naming the three owed things is what makes this a tracked gap rather than a silence, and what stops the reply contracted above being mistaken for having met it: **a view rendering a successful publish as delivered is relying on a guarantee no requirement here provides**, and the rendering obligation for an op in flight lands on whatever states it.

#### Scenario: A send the transport accepted is not a delivery

- **WHEN** an op is published on an open channel and the handoff to the transport succeeds
- **THEN** what is reported identifies the op, names the channel it belongs on, and carries the bytes to send
- **AND** no field of it reports whether the transport accepted the bytes or whether a peer received the op

#### Scenario: A publish reports success on the strength of the log alone

- **WHEN** an op is published and no delivery outcome for it ever arrives
- **THEN** the publish is reported as successful
- **AND** the op is readable from the local op log

#### Scenario: There is no route by which a handoff failure could unpublish the op

- **WHEN** what this capability's publish path can do after the op is appended is examined
- **THEN** it hands out the bytes to send rather than sending them
- **AND** there is no branch that could remove or alter the appended op on a send failure

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
- a payload carrying an op whose signature does not verify under the public key that op carries;
- a payload carrying an op naming a Stoa other than the Stoa whose channel it arrived on;
- a payload larger than a single message may carry;
- a payload carrying an op whose counter is further ahead of this peer's current time than `op-ordering`'s receive window allows.

A refusal SHALL NOT partially apply: a refused payload SHALL leave the op log, the peer's view of the Stoa, and its channel state as they were.

**The window is judged last, and only on an op that would otherwise be admitted.** A payload that fails any other check MUST be reported as that failure, even if its counter is also beyond the window. The window's rule belongs to `op-ordering` and is not restated here. What this capability adds is that the window is checked at this boundary, before anything is stored, and is reported as a refusal of its own.

**An op this peer already holds is judged like any other arrival.** An arriving op whose counter is beyond the window MUST be refused as ahead of this peer's time even when the peer already holds that op, and MUST NOT be reported as already held. The refusal leaves the held op as it was.

**Forgery is one refusal here rather than two, and that is a narrowing.** This list previously separated a bad signature from a key that did not bind to a separately-claimed author. An op names its author by carrying that author's public key and by nothing else, so there is no second identifier a key could fail to bind to: substituting the author substitutes the key, and the signature then fails under it. The forged-authorship case is therefore wholly caught by the signature check, and a peer that reported the two apart would be reporting a distinction its inputs cannot make.

#### Scenario: A payload on an unknown channel is refused

- **WHEN** a payload arrives bearing a channel identifier this peer has no open channel for
- **THEN** it is refused, reported as an unknown channel
- **AND** nothing is stored

#### Scenario: A payload that does not decode is refused

- **WHEN** a payload that the op decoder does not accept arrives on an open channel
- **THEN** it is refused, reported distinguishably from an unknown channel
- **AND** no partially-populated op is stored

#### Scenario: An op whose signature does not verify is refused

- **WHEN** a payload decodes to an op whose signature does not verify under the public key that op carries
- **THEN** it is refused, reported distinguishably from a payload that did not decode
- **AND** the op is not stored

#### Scenario: An op whose key does not bind to its claimed author is refused

- **WHEN** a payload decodes to an op whose carried public key has been replaced with a different well-formed key, leaving the signature as it was
- **THEN** it is refused
- **AND** the refusal is distinguishable from a malformed payload
- **AND** the same signature still verifies under the original key, so the refusal is the signature not matching the carried key rather than a decode failure

  The scenario keeps its name because it is the case this delta alters, and what it
  checks is unchanged in substance: an op presenting a key other than the one that
  signed it is refused. What changes is the mechanism — substituting the author *is*
  substituting the key, so the signature check catches it and there is no separate
  binding step left to fail.

#### Scenario: An op too far ahead of this peer's time is refused distinguishably

- **WHEN** a payload decodes to an op that verifies, names the Stoa whose channel it arrived on, and carries a counter beyond `op-ordering`'s receive window of this peer's current time
- **THEN** it is refused, reported as ahead of this peer's time
- **AND** the refusal is distinguishable from each of the other five
- **AND** the op is not stored

#### Scenario: Another failure is reported ahead of the window

- **WHEN** a payload decodes to an op whose counter is beyond the receive window and whose signature does not verify, and another such op that verifies but names a different Stoa from the channel it arrived on
- **THEN** the first is reported as failing verification
- **AND** the second is reported as a Stoa mismatch
- **AND** neither is reported as ahead of this peer's time

#### Scenario: A held op arriving again beyond the window is refused, and stays held

- **WHEN** a peer admits an op, its current time then moves back so that the op's counter is more than one hour ahead of it, and the same op arrives again
- **THEN** the second arrival is refused, reported as ahead of this peer's time
- **AND** it is not reported as already held
- **AND** the peer still holds exactly one entry for the op, the one it admitted

#### Scenario: Refusal leaves nothing behind

- **WHEN** any refused payload is processed
- **THEN** the op log holds no entry for it
- **AND** the channel remains open and continues to accept subsequent payloads

#### Scenario: A valid op is stored

- **WHEN** a payload decodes to an op that verifies, names the Stoa whose channel it arrived on, and carries either no counter or a counter within `op-ordering`'s receive window of this peer's current time
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

### Requirement: An oversized payload is refused, against a limit pinned at 150 KiB

A payload larger than the maximum a single message may carry SHALL be refused, and the refusal SHALL be reported distinguishably from a decode failure so that "no peer could legitimately have sent this" is tellable from "this op is corrupt".

That maximum SHALL be **150 KiB**. The value is named here because the requirement's justification depends on it: a limit that had silently drifted upward would still refuse an absurd payload and still satisfy a scenario probing only absurd sizes. 150 KiB is the figure this system was designed against as a network-wide validation limit applied to one message, which cannot be raised unilaterally.

**What is checkable here is the pin, and not agreement with the network.** No limit reaches this capability from the transport — the delivery contract states none — so the value is a constant this system chose, and the only property a peer can establish locally is that the constant is still the one that was chosen. That a raised or lowered network limit would make this constant wrong is real and is out of scope below; a scenario claiming the two are compared would be comparing the constant against itself.

The check SHALL apply to the payload as received, before it is decoded. `op-format`'s cap bounds one field of an op and explicitly declines to bound a decoded op's total size, on the grounds that a decoder handed a byte slice cannot see the frame the bytes arrived in. **This capability is that frame**, and this requirement is the total-size bound `op-format` named as belonging here.

#### Scenario: A payload over the limit is refused

- **WHEN** a payload larger than the message limit arrives
- **THEN** it is refused, reported as over-long
- **AND** the refusal is distinguishable from a decode failure

#### Scenario: A payload at the limit is not refused for its size

- **WHEN** a payload of exactly the maximum permitted size arrives
- **THEN** it is not refused on account of its size

#### Scenario: The limit's value is pinned against silent drift

- **WHEN** the configured maximum is compared against a hardcoded 150 KiB written independently of it
- **THEN** they are equal
- **AND** a local edit to the configured maximum fails this rather than passing quietly

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

### Requirement: A channel is closed when a user leaves a Stoa and on shutdown

A Stoa's channel SHALL be closed when the user leaves that Stoa, and every open channel SHALL be closed when the application shuts down. **That is an obligation this change does not discharge, and it is stated as an obligation rather than narrowed to what exists**: the two operations it needs are supplied and contracted below, and the handler that must invoke them is the fourth owed thing the previous requirement names. A change that writes a shutdown handler which closes nothing breaks this requirement, which is why it is not rewritten into a statement about the operations alone.

Both cases are one situation: a channel must not outlive the application that opened it. A channel left open keeps the shared node working on behalf of a Stoa belonging to an application nobody has open — holding filter subscriptions and the remote peer slots serving them, or running the topic's handler chain and synchronisation loops locally.

**Closing SHALL be treated as best-effort and SHALL NOT be reported as a guarantee.** The underlying close undertakes to stop the channel's own loops; the release of the content topic behind it is reference-counted across channels on that topic and takes effect only when the last one goes, and its failures are not surfaced to the caller. A peer therefore cannot observe that a release reached the network, and SHALL NOT claim it did.

A channel closed SHALL be reopenable under the same identity, because a user may leave a Stoa and rejoin it. Reopening SHALL NOT require a restart and SHALL NOT change the channel identifier.

Closing a channel SHALL NOT discard, alter or hide the ops already stored from it. The ops are the peer's, and leaving a Stoa is a statement about what it listens to rather than about what it has seen.

**What is checkable here is the record of which channels are open, and closing one Stoa's channel and closing every open channel are the two operations over it.** The events that must drive them — a user leaving a Stoa, and the application shutting down — have no site in this application, which is the gap the previous requirement names as owed. So the scenarios below are stated over the two operations rather than over the two events: they pin that closing one channel closes that one and no other, and that closing every channel yields the identifier of each so that a caller can act on it, which is what the absent handler will need. **What no scenario here claims is that any handler invokes either operation**, because none does, and a scenario phrased over the event would pass by never firing.

#### Scenario: Closing one Stoa's channel closes that one and no other

- **WHEN** several Stoas' channels are open and one Stoa's channel is closed
- **THEN** that channel is no longer open
- **AND** every other Stoa's channel is still open

#### Scenario: Closing every open channel yields each channel's identifier

- **WHEN** channels are open for several Stoas and every open channel is closed
- **THEN** no channel is open
- **AND** what is reported names each closed channel's identifier, so that a caller can act on each at the transport

#### Scenario: A channel closed can be reopened under the same identifier

- **WHEN** a Stoa's channel is closed and then opened again in the same session, deriving its identity afresh as a rejoin would
- **THEN** it is open under the same channel identifier as before
- **AND** nothing about the reopening requires a restart or distinguishes it from the first open

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

Admitting an op to the op log SHALL NOT be read as a statement that the op is permitted, that its author may moderate, that a revision it declares takes effect, or that a Stoa's posting policy admits its author. Validation at this boundary answers one question and nothing further: are the bytes a well-formed, authentic op, addressed to this channel's Stoa, whose counter is not further ahead of this peer's current time than `op-ordering`'s receive window allows?

**The window is a judgement on a field's value, and it is not a judgement of authority.** It asks how an op's counter stands against this peer's time at the moment of arrival.

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

#### Scenario: Among other channels tied for the most waiting, the one whose newest arrived latest gives up its newest

- **WHEN** the boundary is held up deciding one payload, messages carrying distinct valid ops arrive on two open channels until the bound is reached with each of the two holding as many waiting payloads as the other, the first and the last of them arriving on the first channel, and a message carrying a valid op then arrives on a third open channel
- **THEN** the payload that arrived last on the first channel is recorded as discarded
- **AND** every other of those ops, the third channel's included, is stored once the boundary is released

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

Each park MUST be recorded in the module's log as a park, distinguishably from a refusal, a discard and a store, and the record MUST NOT carry the payload or the sender identifier. A parked message's channel is not open when the message is taken, so "A message a reliable channel delivers reaches the op log only through the inbound boundary" keeps that channel's identifier out of the record as well.

If the parked messages cannot be written, the payload MUST NOT be parked, the module's log MUST record a storage failure, and the payload MUST NOT be held for another attempt.

#### Scenario: A message on a channel being opened is parked, not judged

- **WHEN** this peer has requested a Stoa's channel and delivery has not answered, and a message carrying a valid op for that Stoa arrives on its channel identifier and is taken
- **THEN** the module's log records it as parked
- **AND** the op is not in the op log
- **AND** no refusal is recorded for it while delivery has still not answered

#### Scenario: A park is logged without text the sender chose

- **WHEN** a message carrying a distinctive sender identifier and payload is taken on a channel being opened
- **THEN** the module's log records it as parked
- **AND** the park's record contains neither that channel's identifier, nor that sender identifier, nor the payload's bytes

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

**Otherwise, while parking it would put the parked messages over a total bound, the newest parked message of the channel holding the most of what that bound counts is chosen** — messages for the count bound, payload bytes for the byte bound — counting the payload being parked with its own channel, and leaving out every message already chosen. Messages are chosen for the count bound first, until it would hold, and then for the byte bound, until it would hold. Newest means handed over latest. When several channels hold the most, the payload's own channel MUST be the one chosen if it is among them; otherwise the one among them whose newest parked message was handed over latest.

**If the payload being parked is chosen, for either total bound, that payload MUST be discarded and every message already parked MUST be kept**, a message chosen before it included. Otherwise every message chosen MUST be discarded, and the payload MUST be parked.

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

#### Scenario: Among other channels tied for the most parked, the one whose newest was handed over latest gives up its newest

- **WHEN** the parked messages reach the total count bound with two channels holding as many parked messages as each other and more than any other, the first and the last of those two channels' messages to be handed over being on the first channel, and a message is then taken on a third channel being opened, which, counting that message, holds fewer, is within its own channel's bounds, and keeps the parked payload bytes within their total bound
- **THEN** the message handed over last of those parked on the first channel is recorded as discarded from the parked messages
- **AND** every message parked on the second channel is still parked
- **AND** the message taken is recorded as parked

#### Scenario: Over both total bounds at once, the count bound is restored first

- **WHEN** the parked messages reach the total count bound, with one channel holding more parked messages than any other and a second channel holding more payload bytes than any other, and a message is then taken on a third channel being opened, which is within its own channel's bounds, holds, counting that message, fewer parked messages than the first channel and fewer payload bytes than the second, and whose payload puts the parked payload bytes over their total bound by more than the payload bytes of the first channel's newest parked message and by no more than those of the second channel's newest parked message
- **THEN** the message handed over last of those parked on the first channel is recorded as discarded from the parked messages
- **AND** the message handed over last of those parked on the second channel is recorded as discarded from the parked messages
- **AND** the message taken is recorded as parked

#### Scenario: A payload discarded for the byte total costs no message already parked

- **WHEN** the parked messages reach the total count bound, with one channel holding more parked messages than any other, and a message is then taken on a second channel being opened, which is within its own channel's bounds, holds, counting that message, fewer parked messages than the first channel and more payload bytes than any other channel holds, and whose payload puts the parked payload bytes over their total bound even once the first channel's newest parked message is left out
- **THEN** the message taken is recorded as discarded from the parked messages
- **AND** every message that was parked before it is still parked
- **AND** no other discard is recorded

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
