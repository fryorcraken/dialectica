# moderation-resolution Specification

## Purpose

Defines what makes a moderation op binding on a reader, how competing moderations of one target are ordered, and what a reader concludes about a target from the ops it happens to hold.

## Requirements

### Requirement: Moderation authority is decided on every read

A reader SHALL decide, for each moderation op it considers, whether that op's signer was authorised to moderate, and SHALL do so each time it resolves a target. It SHALL NOT treat an op as binding because the op was accepted when it was stored, because it was accepted on a previous read, or because the peer that relayed it vouched for it.

An authentic signature answers who signed, and nothing else. A moderation op signed by a peer with no authority is genuinely from that peer and is genuinely not a moderation, and those two facts are established by different checks against different inputs. A reader that ran only the first would let any peer forge a moderation — or forge the removal of one — which is the failure this requirement exists to prevent, and which is documented in shipped code elsewhere.

Deciding on read rather than on receipt is also what makes the answer correct as the peer's knowledge grows: authority depends on state the peer may not have held when the op arrived.

#### Scenario: A moderation by a peer with no authority does not bind

- **WHEN** a target is resolved against a log holding an authentically signed moderation op whose signer is not a moderator
- **THEN** the target is not reported as hidden
- **AND** the op is not named as the deciding moderation

#### Scenario: A forged moderation does not bind

- **WHEN** a log holds a moderation op that names a moderator as its author but whose signature was made by another key
- **THEN** the target is not reported as hidden

#### Scenario: Authority is not conferred by storage

- **WHEN** a log holds both an unauthorised and an authorised moderation of one target
- **THEN** the outcome is the one the authorised op states
- **AND** it is that op that is named as the deciding moderation

#### Scenario: Two peers holding the same ops reach the same answer

- **WHEN** two readers resolve one target from the same ops and the same moderator set
- **THEN** both report the same outcome
- **AND** neither consults any state outside those ops and that set

### Requirement: A Stoa's moderator set is derived from its genesis record

The moderator set a reader checks against SHALL be derived from the Stoa's genesis record. The record's creator SHALL be the sole moderator.

A reader SHALL NOT resolve moderation for a Stoa whose genesis record it does not hold, and SHALL NOT fall back to an empty moderator set, to the set of a different Stoa, or to treating every signer as authorised.

The genesis record is what a Stoa is: it is the address's preimage, and the creator key in it is what makes the creator the moderator. A reader lacking it cannot distinguish an authorised moderation from a forgery, and so has no basis on which to report either "hidden" or "not hidden" as a decision about that Stoa.

#### Scenario: The creator is a moderator

- **WHEN** a moderator set is derived from a genesis record
- **THEN** the record's creator is in it

#### Scenario: Nobody else is a moderator

- **WHEN** a moderator set is derived from a genesis record
- **THEN** no key other than the creator is in it
- **AND** in particular the author of an arbitrary post in that Stoa is not

#### Scenario: Resolution requires a moderator set to have been supplied

- **WHEN** a reader has no genesis record for a Stoa
- **THEN** it cannot obtain a moderator set for that Stoa
- **AND** it therefore cannot report that Stoa's targets as either hidden or not hidden

### Requirement: A moderation binds only within the Stoa its moderator governs

A moderation op SHALL be considered only when the Stoa it names is the Stoa whose moderator set is being applied. A moderation op naming a different Stoa SHALL be ignored, whether or not its signature verifies and whether or not its signer moderates some other Stoa.

An op's Stoa is inside its signed bytes, so an op cannot be lifted from one Stoa and replayed into another without breaking its signature. That protects against a *replay*, and it does not protect against a Stoa's moderator deliberately signing a moderation that names a Stoa they do not moderate: such an op is authentic, names a real moderator key, and is not a moderation of anything. Only comparing the op's Stoa to the governing set's Stoa refuses it.

The same comparison is what keeps a reader from applying one Stoa's moderator set to another Stoa's ops when a target op id is known in both.

#### Scenario: A moderator's op naming another Stoa is ignored

- **WHEN** a target is resolved with one Stoa's moderator set, against a log holding a validly signed moderation op from that moderator that names a different Stoa
- **THEN** the target is not reported as hidden

#### Scenario: A moderator of one Stoa cannot hide in another

- **WHEN** two Stoas exist and each creator signs a moderation naming their own Stoa
- **THEN** resolving with the first Stoa's moderator set applies only the first moderation
- **AND** resolving with the second's applies only the second

### Requirement: The most recent valid moderation decides, and a hide is reversible

Where a target has more than one binding moderation, the outcome SHALL be the one stated by whichever of them the ordering rule places first. A moderation op SHALL carry the action it performs, and the action `unhide` SHALL reverse a previous `hide`.

"First" is the ordering rule's position and nothing else. That rule leads with the highest Lamport counter the op carries in its own signed bytes; where an op carries none — which is exactly an op encoded under the version predating the clock fields — it falls back to **ascending op id**, which carries no recency at all.

**Where the leading binding moderation carries no counter, `hide` SHALL decide.** Where the leading binding moderation *does* carry a counter, the ordering rule's position SHALL stand unmodified, **even if other candidates carry none**.

The condition is deliberately about the leading candidate rather than about all of them, and the two differ whenever one target's binding moderations are a mix of ops that carry a counter and ops that do not — the normal state for a target moderated both before and after the clock fields arrived. A leading op carrying a counter occupies its position by a genuine last-write-wins comparison, which is the whole reason not to second-guess it; a candidate further down carrying none says nothing about that. The ordering rule already places every op carrying a counter ahead of every op that carries none, so a leading op with a counter means those ops won on their own terms.

**Reversibility now works, and this is the change that makes it work.** An `unhide` published after a `hide` carries a higher counter, so it leads, so it decides. The preference below is consequently **unreachable for any target whose moderations all carry counters**, which is every target moderated from this version onward. It is retained for the ops that already exist, and for those only.

**Why the preference was needed, and why its original justification no longer holds.** It was introduced because a moderation op was fully determined by its Stoa, author, target and action — no nonce, no timestamp, no free byte — so for one Stoa, one moderator and one target **exactly two ops could ever exist**, with two fixed op ids. Under ascending op id, whichever id was lower would win permanently, which made a bare `unhide` published against an unmoderated target a **pre-emptive veto**: publish it, discard the key, and if the pair hashed the wrong way the target could never be hidden by anyone. It was grindable, because the creator picks the Stoa title, the title fixes the address, and the address is inside both ids.

**That premise is now false and SHALL NOT be relied on anywhere.** A moderation op carries a counter and a wall-clock, both author-chosen, so a moderator can mint arbitrarily many distinct `hide` ops and arbitrarily many distinct `unhide` ops for one target. The consequences are worth separating, because one of them is a fix and one is a new fact to hold:

- **The pre-emptive veto is closed for ops carrying counters.** A later `hide` carries a higher counter and leads, whatever any earlier op's id. Grinding the op id buys nothing against an op that outranks it on the counter.
- **An unbounded number of moderation ops can name one target.** Nothing downstream may assume the candidate set is small or bounded. In particular, any implementation that enumerated the two possible ops, or that reasoned about "the hide" and "the unhide" as unique, is incorrect under this change.

Ops that are not binding SHALL NOT participate in this ordering at all: a more recent op that fails authenticity, authority or scope SHALL NOT displace an older binding one, and SHALL NOT be reported as the deciding moderation.

**Only moderations that already bind are eligible to decide, including under this preference.** A `hide` that fails authenticity, authority or scope SHALL NOT be preferred, SHALL NOT decide the outcome, and SHALL NOT be named as the deciding moderation.

This is stated separately because it is the point at which the preference could silently undo the requirement it sits beneath. A preference that searched every op naming the target, rather than only those already established as binding, would let any peer publish a forged or unauthorised `hide` of any target and have every conforming reader report it as hidden. **The minting freedom above makes that failure cheaper than it was**: an attacker can now produce unlimited distinct forged `hide` ops for one target rather than one, so a preference that searched unfiltered would find one with certainty rather than by chance.

#### Scenario: A later unhide reverses an earlier hide

- **WHEN** a moderator hides a target and later unhides it
- **THEN** the target is not reported as hidden
- **AND** the unhide is named as the deciding moderation

#### Scenario: A later hide reverses an earlier unhide

- **WHEN** a moderator unhides a target and later hides it
- **THEN** the target is reported as hidden

#### Scenario: An unhide reverses a hide whatever the two op ids are

- **WHEN** a moderator hides a target and later unhides it, and the unhide's op id is the higher of the two
- **THEN** the target is not reported as hidden
- **AND** the outcome is the same when the unhide's op id is the lower of the two

#### Scenario: A pre-emptive unhide does not veto a later hide

- **WHEN** a moderator publishes an unhide of an unmoderated target and later hides it, both ops carrying counters
- **THEN** the target is reported as hidden
- **AND** the outcome does not depend on which op's identifier sorts first

#### Scenario: An unauthorised later op does not displace an authorised earlier one

- **WHEN** a moderator hides a target and a non-moderator subsequently publishes an unhide of it carrying a higher counter
- **THEN** the target is still reported as hidden
- **AND** the moderator's hide is named as the deciding moderation

#### Scenario: Many minted moderations of one target resolve to the leading binding one

- **WHEN** a target carries many binding moderation ops differing only in their counters and wall-clocks
- **THEN** the one the ordering rule places first decides
- **AND** resolution reports an answer rather than failing on the number of candidates

#### Scenario: A forged hide does not win the degraded preference

- **WHEN** a target's moderations include a binding `unhide` and one or more `hide` ops that fail authenticity, authority or scope, and none carries a counter
- **THEN** the target is not reported as hidden
- **AND** the binding `unhide` is named as the deciding moderation

#### Scenario: A forged hide does not win against a binding unhide however many are minted

- **WHEN** a target's moderations include a binding unhide carrying a counter and many distinct hide ops that fail authenticity, authority or scope
- **THEN** the target is not reported as hidden
- **AND** the binding unhide is named as the deciding moderation

#### Scenario: A binding hide still wins over hides that do not bind

- **WHEN** a target's moderations include a binding `hide` alongside `hide` ops that do not bind, and none carries a counter
- **THEN** the target is reported as hidden
- **AND** the binding `hide` is named as the deciding moderation

#### Scenario: A transport-ordered leading op decides despite unordered candidates

- **WHEN** a target's leading binding moderation carries a counter and another binding moderation of it carries none
- **THEN** the leading op decides
- **AND** the preference for `hide` does not apply

**This scenario keeps its original name and its condition has moved from the arrival to the op.** "Ordered" now means the op carries a counter of its own rather than that the transport supplied one.

#### Scenario: A transport-ordered unhide still reverses a hide

- **WHEN** a moderator's hide and a later unhide of one target both carry counters
- **THEN** the target is not reported as hidden
- **AND** the preference for `hide` does not apply

#### Scenario: The hide preference does not apply where the leading op carries a counter

- **WHEN** a target's leading binding moderation is an unhide carrying a counter and another binding hide of it carries none
- **THEN** the unhide decides
- **AND** the target is not reported as hidden

#### Scenario: The hide preference still applies among ops carrying no counter

- **WHEN** a target's binding moderations all predate the clock fields and include a hide and an unhide
- **THEN** the target is reported as hidden
- **AND** the outcome does not depend on which op's identifier sorts first

#### Scenario: A moderator may reverse a moderation they did not place

- **WHEN** one moderator's hide is followed by an unhide from a different moderator of the same Stoa
- **THEN** the unhide takes effect

**Not verifiable in this change.** It needs two distinct moderators of one Stoa, and the moderator set is the creator alone until a mutable set exists. What is verified is the structural precondition — that authority is decided by membership in the moderator set and not by comparison with an earlier op's author. Anything relying on this scenario should treat it as specified intent, not as covered behaviour.

#### Scenario: Ordering does not depend on the sequence ops were received in

- **WHEN** two readers hold the same moderation ops, appended in different sequences
- **THEN** both report the same outcome and name the same deciding op

### Requirement: A target with no binding moderation is not hidden

A target for which the reader holds no binding moderation op SHALL be reported as not hidden, and no deciding moderation SHALL be named. This SHALL apply equally to a target the log holds no ops about at all, to a target the log does not itself hold, and to a target whose only moderation ops fail authenticity, authority or scope.

Two peers routinely hold different sets of ops, so absence of a moderation is the ordinary state of a target whose hide has not propagated yet, and it is not distinguishable from a target nobody moderated. Reporting it as not hidden is the answer for that peer over the ops it has; convergence closes the gap when the op arrives.

The converse — a peer missing a newer `unhide` continuing to report a target as hidden — is the same property seen from the other side, and is likewise correct rather than a defect.

#### Scenario: A target with no ops about it is not hidden

- **WHEN** a target no op names is resolved
- **THEN** it is reported as not hidden
- **AND** no error is reported

#### Scenario: A target the log does not hold is resolved from the ops about it

- **WHEN** a moderator's hide names a target op the log has never received
- **THEN** the target is reported as hidden
- **AND** the absence of the target op is not an error

#### Scenario: A peer missing the newest unhide still reports hidden

- **WHEN** one reader holds a hide and an unhide, and another holds only the hide
- **THEN** the first reports not hidden and the second reports hidden
- **AND** neither is in error

### Requirement: Only moderation ops decide moderation

A reader resolving moderation SHALL consider only ops whose kind is moderation. Revisions, votes and posts naming the target SHALL NOT affect the outcome.

A moderator's hide and an author's edit are about different things and do not contend: an edit does not clear a hide, and a hide does not invalidate an edit. Ops of other kinds reach the reader through the same target-restricted read, so ignoring them is a check the resolver performs rather than a property of what it is given.

#### Scenario: A revision does not clear a hide

- **WHEN** a moderator hides a target and the target's author subsequently publishes a revision of it
- **THEN** the target is still reported as hidden

#### Scenario: A vote does not moderate

- **WHEN** the only ops naming a target are votes
- **THEN** the target is reported as not hidden

### Requirement: The deciding moderation is named, not merely counted

Where a target is subject to a binding moderation, the reader SHALL be able to identify the op that decided it, including its author and its action.

A moderator acting on a post is acting on a judgement they can name, in the same way §5.7 gives an author's revisions a version a moderator can name. A bare boolean cannot support a moderator reversing a specific hide, a reader being shown who hid something, or an interface distinguishing "hidden by this Stoa's moderator" from any other reason content is absent.

#### Scenario: A hidden target names the op that hid it

- **WHEN** a target is reported as hidden
- **THEN** the moderation op that decided it is identified
- **AND** its author and action are available

#### Scenario: An unhidden target names the unhide that lifted it

- **WHEN** a hide is followed by a binding unhide
- **THEN** the target is reported as not hidden
- **AND** the unhide is identified as the deciding op, distinguishing it from a target nobody ever moderated

### Requirement: Resolution never aborts the process

Resolving a target SHALL NOT panic for any combination of ops the op decoder accepts, whatever their signatures, authors, kinds, Stoas or recorded arrival metadata.

Every op considered arrived from a peer and is attacker-controlled. A panic in this module aborts the module process, which turns a malformed or hostile op into a denial of service against the peer that received it.

#### Scenario: A log of adversarial ops resolves without a panic

- **WHEN** a target is resolved against a log holding forged, unsigned, mis-signed, cross-Stoa and wrong-kind ops naming it
- **THEN** resolution returns an answer
- **AND** no operation panics
