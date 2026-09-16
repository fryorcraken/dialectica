<!--
Only the inbound-validation requirement changes. Every other address in this
capability is a Stoa address — channel identity derivation, and the
Stoa-mismatch refusal — and none of those requirements is in this delta.

`The transport's sender identifier is never an identity` is deliberately NOT
here: it already says authorship comes from "the op's own signature and the
author key inside it, and from nothing else", which is the post-sweep shape
stated before the sweep. It survives unchanged.
-->

## MODIFIED Requirements

### Requirement: Every inbound payload is validated before it reaches storage

A payload arriving on a channel SHALL be validated before it is appended to the op log, before it reaches any resolver, and before any property of it is used to look anything up.

Every byte arriving on a channel is attacker-controlled. A channel has no membership: any peer that computes a Stoa's channel identity may send on it, and the transport neither knows nor asserts who a participant is. Forged authorship, malformed bytes, oversized payloads and ops naming a Stoa the sender has no business touching are the ordinary content of this boundary rather than exceptional cases.

Each of the following SHALL be refused, and each SHALL be reported distinguishably from the others:

- a payload arriving on a channel identifier this peer has no open channel for;
- a payload the op decoder does not accept;
- a payload carrying an op whose signature does not verify under the public key that op carries;
- a payload carrying an op naming a Stoa other than the Stoa whose channel it arrived on;
- a payload larger than a single message may carry.

A refusal SHALL NOT partially apply: a refused payload SHALL leave the op log, the peer's view of the Stoa, and its channel state as they were.

**A boundary reporting only "invalid" sends the reader looking in the wrong place.** These five failures have five different causes and five different responses — a build that is behind, a corrupt or hostile payload, a forgery, a misdirected or replayed op, and a peer sending more than the network permits — and a peer that cannot tell them apart cannot report which of them is happening to its user or to a log.

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

#### Scenario: Refusal leaves nothing behind

- **WHEN** any refused payload is processed
- **THEN** the op log holds no entry for it
- **AND** the channel remains open and continues to accept subsequent payloads

#### Scenario: A valid op is stored

- **WHEN** a payload decodes to an op that verifies and names the Stoa whose channel it arrived on
- **THEN** it is appended to the op log
- **AND** its recorded arrival reports it as not ordered by the transport
