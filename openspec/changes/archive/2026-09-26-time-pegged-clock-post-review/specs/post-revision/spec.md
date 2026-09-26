## MODIFIED Requirements

### Requirement: The current version is the one the ordering rule places first

Among the versions of a post that are authentic and by the post's author, the current version SHALL be the one the system's ordering rule places first. The resolver SHALL NOT define an order of its own, and SHALL NOT compare counters, timestamps, arrival sequence or any other value itself.

The ordering rule is defined once for all ops, so a resolver comparing values itself would be a second implementation that could disagree with the first — and two orders that disagree produce no error, only two peers rendering one post differently.

This binds even though a resolver carrying a *correct* copy of the ordering rule would answer identically today. The contract is system-wide consistency across every reader of the log: a copy passes now and diverges from every other reader on the first change to the shared rule, silently, because two orders that disagree produce no error.

**Placing first is now the same as being latest in the forum's order, for versions carrying a counter.** The ordering rule leads with the highest Lamport counter the op carries in its own signed bytes, and an author's later revision carries a counter above their earlier one because the author had seen the earlier one when they wrote it. So among an author's own versions the rule yields the genuinely latest, and "the current version" means what a reader expects it to mean.

**It is not the same as being latest by any clock, and the distinction is worth keeping.** A Lamport counter here is a causal order. It guarantees that a revision written after seeing an earlier one orders after it. A reader SHALL NOT be told the current version is the most recent by any measure of time. For one author revising their own post the two coincide in practice, because an author has necessarily seen their own earlier version — which is why this requirement can promise what it promises. The exception is an author publishing from a second device that has not yet received a version signed ahead of the time. That version stays current over the second device's until the second device's time passes its counter, and `op-ordering`'s receive window caps that lead at one hour.

**A version carrying no counter is one encoded before the clock fields existed.** The ordering rule places every such version below every version carrying one, so a revision published after this change supersedes every version predating it, and versions predating it order among themselves by ascending op id exactly as they always have. That is a convergent arbitrary order rather than a temporal one, and for those versions this requirement guarantees convergence only — two peers holding the same ones agree on which is current, though neither can say which was written last.

#### Scenario: The highest Lamport timestamp is current when the transport supplies one

- **WHEN** a post has several versions by its author carrying different Lamport counters
- **THEN** the version with the highest counter is current

**This scenario keeps its original name and the counter it reads has moved.** It previously read a counter the transport supplied, which never arrives. It now reads the counter the op carries in its own signed bytes, which is present on every version published from this change onward — so the scenario tests a live path rather than an unreachable one.

#### Scenario: A revision published later supersedes an earlier one

- **WHEN** an author publishes a post, revises it, and revises it again
- **THEN** the last revision is current
- **AND** the result is the same on a peer that received the three in any sequence

#### Scenario: A tie is broken by the ordering rule, not by the resolver

- **WHEN** two versions by the post's author carry the same counter
- **THEN** the one the ordering rule places first is current
- **AND** the result is the same whichever sequence the versions were appended in

#### Scenario: The current version is the same on two peers holding the same ops

- **WHEN** two peers hold the same ops, appended in different sequences
- **THEN** both resolve the post to the same version

#### Scenario: A version the transport ordered leads one it did not

- **WHEN** one version of a post carries a counter and another carries none
- **THEN** the version carrying a counter is current, whatever its counter and whatever the other's op id

**This scenario keeps its original name and its condition has moved from the arrival to the op.** "Ordered" now means the op carries a counter of its own rather than that the transport supplied one.

#### Scenario: Currency is convergent but not temporal when the transport supplies nothing

- **WHEN** a post's versions all predate the clock fields and carry no counter
- **THEN** the version the ordering rule's degraded order places first is current
- **AND** that result is derived from the ops alone, so every peer holding them agrees
- **AND** it is not derived from when any version was written or received

#### Scenario: The resolver reads no wall-clock

- **WHEN** a post's versions carry wall-clocks that disagree with the order their counters give
- **THEN** the current version is the one the counters and op ids select
- **AND** the wall-clock values do not affect the result
