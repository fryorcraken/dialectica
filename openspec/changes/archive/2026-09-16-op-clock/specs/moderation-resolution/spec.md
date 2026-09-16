## MODIFIED Requirements

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
