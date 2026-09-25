## MODIFIED Requirements

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

**A boundary reporting only "invalid" sends the reader looking in the wrong place.** These six failures have six different causes and six different responses: a build that is behind, a corrupt or hostile payload, a forgery, a misdirected or replayed op, a peer sending more than the network permits, and an op signed by a clock more than an hour ahead of this one or by an author choosing a counter to jump the order. A peer that cannot tell them apart cannot report which of them is happening to its user or to a log.

**The window is judged last, and only on an op that would otherwise be admitted.** A payload that fails any other check MUST be reported as that failure, even if its counter is also beyond the window. The window's rule and its reasoning belong to `op-ordering` and are not restated here. What this capability adds is that the window is checked at this boundary, before anything is stored, and is reported as a refusal of its own. A forgery reported as "too far ahead" would describe a forged op by a field its forger chose.

**An op this peer already holds is judged like any other arrival.** Validation precedes every lookup by a property of the op, as the opening of this requirement says, and finding whether an op is already held is a lookup by its op id. An arriving op whose counter is beyond the window MUST therefore be refused as ahead of this peer's time even when the peer already holds that op, and MUST NOT be reported as already held. The refusal leaves the held op as it was. This can happen only when this peer's current time has moved back after it admitted the op.

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

### Requirement: This capability decides nothing about an op beyond admitting it

Admitting an op to the op log SHALL NOT be read as a statement that the op is permitted, that its author may moderate, that a revision it declares takes effect, or that a Stoa's posting policy admits its author. Validation at this boundary answers one question and nothing further: are the bytes a well-formed, authentic op, addressed to this channel's Stoa, whose counter is not further ahead of this peer's current time than `op-ordering`'s receive window allows?

**The window is a judgement on a field's value, and it is not a judgement of authority.** It asks how an op's counter stands against this peer's time at the moment of arrival. Its reasoning, and the reversal of this system's earlier rule that no op is refused for a field value, belong to `op-ordering`.

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
