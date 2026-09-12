## Purpose

Defines what a caller supplies to publish a post, a reply or a vote; what must be
true of the op that results; what the peer holds afterwards; and what is refused.

Three boundaries are named rather than restated, because each is another
capability's and two specs asserting one rule is how two copies drift.

- **`op-format` owns the op itself** — its kinds, its canonical bytes, what a
  signature commits to, how an op id is derived, and what a decoder does with
  hostile input. Its requirement "An op carries no ordering field and no per-peer
  state" is why an op id is a function of content alone, which this capability
  depends on and does not restate.
- **`op-log` owns the store** — that it records what arrived and decides nothing,
  that an op is stored once identified by its op id, and that re-appending leaves
  one entry. This capability contracts what publishing *asks* of the store, not
  what the store does with it.
- **`posting-capability` owns the probe** a caller asks before rendering a compose
  affordance. A successful probe is not a promise that a later publish succeeds,
  and this capability does not require a publish to re-derive the probe's reasons.

Reading is out of scope. So are revising a post, moderating, and attachments.

## ADDED Requirements

### Requirement: Publishing signs, appends, and hands off — in that order

Each publish operation SHALL sign an op with the caller's identity for the named
Stoa, append it to the local op log, and hand it to delivery. The append SHALL
complete before delivery is invoked, and the reply SHALL NOT be deferred until
delivery reports an outcome.

A reply therefore states that the op exists locally. It states nothing about
whether any peer received it.

Where delivery declines the handoff, the op SHALL remain in the log and the
reply SHALL report the op as published. A publish SHALL NOT be reported as having
failed on the strength of a delivery outcome, and SHALL NOT remove the op from
the log.

#### Scenario: The op is in the log when the reply is returned

- **WHEN** a publish returns successfully
- **THEN** the op it names is readable from the log by that op id

#### Scenario: A publish returns while delivery is still outstanding

- **WHEN** a publish is called against a delivery that accepts the handoff and
  reports no outcome
- **THEN** the publish returns successfully naming the op id
- **AND** the result is the same as when delivery reports an outcome promptly

#### Scenario: A declined handoff leaves the op published

- **WHEN** delivery refuses or errors on the handoff
- **THEN** the reply reports the op as published and names its op id
- **AND** the op is readable from the log

#### Scenario: A publish that is refused appends nothing

- **WHEN** a publish is refused for any reason
- **THEN** the log holds no op it would have created
- **AND** delivery was not invoked

### Requirement: The reply names the op that was published

Each publish operation SHALL answer with the op id of the op it published. It
SHALL NOT answer with a bare acknowledgement.

A caller that cannot name what it just created cannot scroll to it, render it, or
name it in a later call.

#### Scenario: The reply's op id is the published op's own

- **WHEN** a post is published
- **THEN** the op id in the reply is the op id of the op now in the log
- **AND** reading the log by that op id returns that op

#### Scenario: The reply carries no acknowledgement in place of an op id

- **WHEN** any publish succeeds
- **THEN** the reply carries an op id

### Requirement: The author is derived from the Stoa, never supplied

No publish operation SHALL accept an author, an identity, a key, or an address as
a parameter. The identity that signs SHALL be derived from the Stoa named in the
request.

A method taking an author is a method that can be asked to sign as someone it is
not.

A request carrying a field that names an author SHALL be refused rather than
ignored, so that a caller which believes it is choosing an identity is told it is
not.

#### Scenario: The published op's author is the derived identity

- **WHEN** a post is published into a Stoa
- **THEN** the op's author is the identity derived for that Stoa
- **AND** the op verifies against that identity's key

#### Scenario: A request naming an author is refused

- **WHEN** a publish request carries a field naming an author, an identity or a
  key
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: The signing identity is the one the probe reports

- **WHEN** the posting-capability probe reports an identity for a Stoa and a post
  is then published into that Stoa
- **THEN** the published op's author is the identity the probe reported

### Requirement: A publish requires a usable identity and says so when there is none

A publish SHALL be refused when no identity is available to sign with, and the
refusal SHALL be the wire contract's error shape carrying a reason a reader can
act on.

A publish SHALL NOT create an identity, a keystore, or any key material as a side
effect of being called.

#### Scenario: No identity is a refusal, not a silent no-op

- **WHEN** a publish is attempted with no usable identity
- **THEN** the reply carries an error naming what is missing
- **AND** no op is appended

#### Scenario: A refused publish creates no key material

- **WHEN** a publish is refused because no identity is available
- **THEN** no keystore and no key exists that did not exist before

### Requirement: A post names a Stoa and carries a body

Publishing a post SHALL require the Stoa it belongs to and the body to publish,
and SHALL produce an op of the post kind naming no parent.

An **empty body SHALL be accepted**, because `op-format` contracts an empty
variable-length field as a value rather than an absence, and refusing it here
would make the publish path disagree with the format about what an op may
contain.

A request omitting the Stoa, or omitting the body, SHALL be refused. A request
whose Stoa or body is present but of the wrong type SHALL be refused
distinguishably from one omitting it.

#### Scenario: A post is published and is a thread root

- **WHEN** a post is published with a Stoa and a body
- **THEN** the op is of the post kind
- **AND** it names no parent
- **AND** its body is the body supplied, unchanged

#### Scenario: An empty body is published

- **WHEN** a post is published with an empty body
- **THEN** the publish succeeds and the op's body is empty

#### Scenario: A post omitting its Stoa is refused

- **WHEN** a post request carries no Stoa
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: A post omitting its body is refused

- **WHEN** a post request carries no body
- **THEN** the reply carries an error

#### Scenario: A wrong-typed field is distinguishable from a missing one

- **WHEN** a post request carries a body that is not a string
- **THEN** the reply's message says the field is the wrong type rather than that
  it is missing

### Requirement: A reply names its parent, and the thread is derived

Publishing a reply SHALL require the Stoa, the parent op id, and the body. It
SHALL NOT accept a thread as a parameter. The thread the reply belongs to SHALL
be derived from the parent.

A thread supplied alongside a parent can disagree with it, and a reply filed
under a thread its parent does not belong to is a reply no reader will find under
either. Deriving the thread makes that state unrepresentable rather than checked
at each call site.

A request carrying a field that names a thread SHALL be refused rather than
ignored.

The op produced SHALL be of the post kind naming the parent supplied, since
`op-format` contracts a reply as a post that names a parent rather than as a kind
of its own.

#### Scenario: A reply names the parent it was given

- **WHEN** a reply is published naming a parent
- **THEN** the op is of the post kind
- **AND** the parent it names is the op id supplied

#### Scenario: A request supplying a thread is refused

- **WHEN** a reply request carries a field naming a thread
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: Two replies to one parent are derived into one thread

- **WHEN** two replies are published naming the same parent, in requests that
  differ in every other accepted field
- **THEN** both are derived into the same thread
- **AND** that thread is the one the parent belongs to

#### Scenario: A reply omitting its parent is refused

- **WHEN** a reply request carries no parent
- **THEN** the reply carries an error

#### Scenario: A reply to a reply is derived into the same thread as its parent

- **WHEN** a reply is published naming another reply as its parent
- **THEN** it is derived into the thread that parent belongs to

### Requirement: A reply to a parent the peer does not hold is refused

Publishing a reply SHALL be refused when the parent op id names no op in the
peer's log, or names an op the peer holds that is not a post.

This is the cost of deriving the thread rather than accepting one, and it is
contracted rather than hidden: with no parent to read, there is no thread to
derive, and publishing a reply into a guessed thread is the state deriving was
chosen to prevent.

The refusal SHALL distinguish "no such op is held" from "the op held is not a
post", so that a caller can tell a propagation gap from a category mistake.

#### Scenario: A reply to an absent parent is refused

- **WHEN** a reply names a parent op id the log holds no op for
- **THEN** the reply carries an error saying the parent is not held
- **AND** no op is appended

#### Scenario: A reply to an op that is not a post is refused

- **WHEN** a reply names a vote's op id as its parent
- **THEN** the reply carries an error saying the target is not a post
- **AND** the error is distinguishable from the parent being absent

#### Scenario: A reply becomes publishable when its parent arrives

- **WHEN** a reply to an absent parent is refused, that parent op is then
  appended to the log, and the same reply is published again
- **THEN** the second publish succeeds

### Requirement: A reply and its parent belong to one Stoa

Publishing a reply SHALL be refused when the Stoa named in the request is not the
Stoa the parent op belongs to.

A reply is signed over the Stoa it names, so a reply naming one Stoa and a parent
in another is an op whose thread is in a Stoa its signature does not cover.

#### Scenario: A cross-Stoa reply is refused

- **WHEN** a reply names one Stoa and a parent op belonging to another
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: A reply within one Stoa is published

- **WHEN** a reply names the same Stoa its parent belongs to
- **THEN** the publish succeeds

### Requirement: A vote names a target and a direction, and both directions publish

Publishing a vote SHALL require the Stoa, the target op id, and the direction,
and SHALL produce an op of the vote kind carrying that target and that direction.
Both directions — raising and lowering — SHALL be publishable.

A request omitting the direction SHALL be refused. A direction the operation does
not recognise SHALL be refused and SHALL NOT be mapped onto a recognised one.

#### Scenario: A raising vote is published

- **WHEN** a vote is published in the raising direction
- **THEN** the op is of the vote kind naming that target
- **AND** its direction is the raising one

#### Scenario: A lowering vote is published

- **WHEN** a vote is published in the lowering direction
- **THEN** the op is of the vote kind naming that target
- **AND** its direction is the lowering one
- **AND** its op id differs from the raising vote's on the same target

#### Scenario: An unrecognised direction is refused, not defaulted

- **WHEN** a vote request names a direction the operation does not recognise
- **THEN** the reply carries an error naming the direction supplied
- **AND** no op is appended
- **AND** no vote is published in either direction

#### Scenario: A vote omitting its direction is refused

- **WHEN** a vote request carries no direction
- **THEN** the reply carries an error

#### Scenario: A vote omitting its target is refused

- **WHEN** a vote request carries no target
- **THEN** the reply carries an error

### Requirement: A published vote is stored and readable, and no ordering consumes it

A published vote SHALL be appended to the log and SHALL be readable from it, by
its own op id and by a read restricted to its target.

**No ordering, count, tally or score in the current contract reads it.** A
caller SHALL NOT be told that publishing a vote changed any position, any count
or any reader's view, and this capability SHALL NOT report one. The reply to a
published vote SHALL carry its op id and nothing that describes an effect.

This is the honest bound on what a vote does today, stated because the alternative
is a caller inferring an effect from the absence of a statement. A published vote
accumulates history that a later scorer reads; whether a scorer exists is not a
property of publishing one.

#### Scenario: A published vote is readable by its op id

- **WHEN** a vote is published
- **THEN** the op is readable from the log by the op id the reply named

#### Scenario: A published vote is readable by its target

- **WHEN** a vote is published naming a target
- **THEN** a read of the log restricted to that target returns the vote

#### Scenario: The reply describes no effect

- **WHEN** a vote is published
- **THEN** the reply carries the op id
- **AND** it carries no score, count, tally, rank or position

#### Scenario: A voted target reads exactly as it did before

- **WHEN** a post is published, read back, then voted on in either direction, and
  read back again
- **THEN** the post op read back the second time is byte-identical to the first
- **AND** nothing this capability reports about the post differs between the two
  reads

### Requirement: Publishing a vote is permitted on any op the peer holds

Publishing a vote SHALL be refused when the target op id names no op the peer
holds. It SHALL NOT be refused on the grounds of the target's kind, the target's
author, or the voter having voted on that target before.

A vote naming an op nobody holds is a vote on nothing, so the target must be
present. Beyond that, whether a vote *counts* is decided by whoever counts votes
and not by whoever publishes one — the publish path refusing a vote on a kind a
future scorer would ignore would be that decision made twice, in two places that
can disagree.

#### Scenario: A vote on an absent target is refused

- **WHEN** a vote names a target op id the log holds no op for
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: Voting twice on one target publishes two ops

- **WHEN** one identity publishes a raising vote and then a lowering vote on one
  target
- **THEN** both ops are in the log
- **AND** neither publish was refused on the grounds of the other

#### Scenario: An identity may vote on its own post

- **WHEN** a vote names a target op the voting identity authored
- **THEN** the publish succeeds

### Requirement: A vote and its target belong to one Stoa

Publishing a vote SHALL be refused when the Stoa named in the request is not the
Stoa the target op belongs to.

#### Scenario: A cross-Stoa vote is refused

- **WHEN** a vote names one Stoa and a target op belonging to another
- **THEN** the reply carries an error
- **AND** no op is appended

### Requirement: Publishing the same content twice publishes one op

An op id is a function of the op's own bytes, and those bytes carry no timestamp
and no value the publisher varies between calls. It follows that one identity
publishing the same content into the same Stoa twice produces **one op**: the
second publish SHALL succeed, SHALL report the same op id as the first, and the
log SHALL hold one op.

**This is contracted rather than left to be discovered, because it is correct for
one case and wrong for another.** A double-submitted form is deduplicated, which
is the behaviour wanted. A person deliberately posting the same short reply twice
publishes once, which is not. No field distinguishes the two, and adding one is a
change to what an op contains — `op-format` owns that, and its requirement "An op
carries no ordering field and no per-peer state" is what currently forbids the
obvious candidate.

Because both outcomes reach a caller as a success carrying one op id, the publish
SHALL surface to its caller the newly-stored-or-already-present answer the append
gives it. `op-log`'s "The caller is told whether the op was new" is what makes
that answer available; what is required here is that a publish **passes it on**
rather than discarding it, since a wire caller has no other way to tell a
deduplicated publish from a first one.

The reply SHALL NOT report a refusal for a repeated publish. Refusing would make
a retried submission an error, which is the case the behaviour is right for.

#### Scenario: The same content published twice yields one op

- **WHEN** one identity publishes a post with the same Stoa and the same body
  twice
- **THEN** both publishes succeed
- **AND** both replies name the same op id
- **AND** the log holds one op for it

#### Scenario: The caller is told the second publish was not new

- **WHEN** a publish stores an op the log already held
- **THEN** the reply states the op was already present
- **AND** the first publish's reply stated it was newly stored

#### Scenario: A repeated publish does not disturb the stored op

- **WHEN** the same content is published twice
- **THEN** the op read back is the one the first publish stored

#### Scenario: Content differing in any way publishes a second op

- **WHEN** one identity publishes two posts into one Stoa whose bodies differ by
  a single character
- **THEN** the two replies name different op ids
- **AND** the log holds both

#### Scenario: The same body in two Stoas is two ops

- **WHEN** one identity publishes the same body into two different Stoas
- **THEN** the two replies name different op ids

#### Scenario: The same body from two identities is two ops

- **WHEN** two identities publish the same body into one Stoa
- **THEN** the two replies name different op ids

#### Scenario: Two replies with the same body to different parents are two ops

- **WHEN** one identity publishes the same reply body to two different parents
- **THEN** the two replies name different op ids

### Requirement: A publish checks only what it can decide, and never in place of a reader

The checks a publish performs SHALL be understood as a third thing, alongside
`op-format`'s verification of authenticity and the read-time authority checks the
resolving capabilities perform. A publish check SHALL NOT be relied upon by any
reader, and SHALL NOT be presented as making a read-time check unnecessary.

**This does not replace or weaken any read-time check, and the direction of the
mistake matters.** An outbound check constrains only the ops *this* peer creates.
Every op arriving from a peer arrived without passing it, so a reader that treated
an outbound check as having established anything about an inbound op would have
been given no protection at all — which is the failure `op-format`'s
"Verification answers authenticity, not authority" records as measured in a
comparable project, where authority is checked on the send path and never on read.

Accordingly, no requirement in this capability SHALL be read as asserting a
property of any op the peer received rather than created.

#### Scenario: An op that a publish would refuse is still stored when it arrives

- **WHEN** an op a publish would have refused — a reply naming an absent parent,
  or a reply whose Stoa differs from its parent's — arrives from a peer and is
  appended
- **THEN** the append succeeds and the op is readable
- **AND** the log does not distinguish it from an op this peer published itself

#### Scenario: A published op is verified on read like any other

- **WHEN** an op this peer published is read back and verified
- **THEN** verification is performed on the op's own bytes and signature
- **AND** the result does not depend on this peer having been the publisher

### Requirement: A publish answers in the wire contract's shapes and never aborts

Every publish SHALL answer with a JSON object. Every refusal SHALL be the error
shape carrying a message, and SHALL NOT carry an op id alongside an error.

A publish SHALL NOT panic for any request, including a request that is not valid
JSON, a request whose fields hold values of any type, and a request whose text
fields hold arbitrary valid UTF-8.

Text a caller supplies SHALL reach the op unchanged. A publish SHALL NOT
normalise, trim, case-fold or otherwise transform a body it is given, because
`op-format` contracts an accepted encoding as re-encoding to itself and a
transformation at this boundary would mean the op published is not the content the
caller supplied.

#### Scenario: A refusal carries no op id

- **WHEN** a publish is refused
- **THEN** the reply carries an error and no op id

#### Scenario: An unparseable request is the error shape

- **WHEN** a publish is called with a request that is not valid JSON
- **THEN** the reply carries an error
- **AND** no op is appended

#### Scenario: Hostile input is never a panic

- **WHEN** publish requests carrying arbitrary field types, absent fields,
  maximal field lengths and adversarially chosen text are called
- **THEN** each returns a JSON object rather than panicking

#### Scenario: A body is published exactly as supplied

- **WHEN** a post is published with a body containing multi-byte sequences,
  bidirectional controls and zero-width characters
- **THEN** the op's body equals the body supplied, byte for byte
- **AND** two bodies differing only by Unicode normalisation form publish as two
  ops with different op ids
