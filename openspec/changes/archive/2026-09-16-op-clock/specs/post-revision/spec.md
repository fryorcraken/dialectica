## MODIFIED Requirements

### Requirement: The current version is the one the ordering rule places first

Among the versions of a post that are authentic and by the post's author, the current version SHALL be the one the system's ordering rule places first. The resolver SHALL NOT define an order of its own, and SHALL NOT compare counters, timestamps, arrival sequence or any other value itself.

The ordering rule is defined once for all ops, so a resolver comparing values itself would be a second implementation that could disagree with the first — and two orders that disagree produce no error, only two peers rendering one post differently.

This binds even though a resolver carrying a *correct* copy of the ordering rule would answer identically today. The contract is system-wide consistency across every reader of the log: a copy passes now and diverges from every other reader on the first change to the shared rule, silently, because two orders that disagree produce no error.

**Placing first is now the same as being latest in the forum's order, for versions carrying a counter.** The ordering rule leads with the highest Lamport counter the op carries in its own signed bytes, and an author's later revision carries a counter above their earlier one because the author had seen the earlier one when they wrote it. So among an author's own versions the rule yields the genuinely latest, and "the current version" means what a reader expects it to mean.

**It is not the same as being latest by any clock, and the distinction is worth keeping.** A Lamport counter is a causal order: it guarantees that a revision written after seeing an earlier one orders after it. It says nothing about wall-clock time, and a reader SHALL NOT be told the current version is the most recent by any measure of time. For one author revising their own post the two coincide in practice, because an author has necessarily seen their own earlier version — which is why this requirement can promise what it promises.

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

### Requirement: A post resolves over the ops the peer happens to hold

Resolving SHALL produce an answer from the ops the log holds, and SHALL NOT report an error, an incomplete result, or an unknown outcome on the grounds that other ops may exist elsewhere.

Two peers routinely hold different sets of ops — one was offline, one joined late, a message has not propagated. A peer holding fewer versions resolves over the ones it has, and that is the correct answer for that peer rather than a defect. Completeness is not a property any peer can establish about itself, so a resolver that required it could never answer at all.

The answer SHALL be a pure function of the ops held, so that two peers with the same set agree whatever sequence those ops arrived in.

**It is not required to advance monotonically as ops arrive, and the reason has changed rather than gone away.** Among versions carrying counters the answer does advance: a late-arriving revision either carries a higher counter and becomes current, or carries a lower one and does not displace what is already there. Among versions carrying no counter the degraded order applies and a version arriving later may become current, because op id carries no recency. A reader that assumed monotonicity held everywhere would be relying on a guarantee the degraded order does not make, and the mixed case — a peer holding versions of both kinds — is the one where the assumption is least visible.

#### Scenario: A post with no versions resolves to itself

- **WHEN** a post that has never been revised is resolved
- **THEN** the original post is current

#### Scenario: A peer holding fewer versions resolves over the ones it has

- **WHEN** one peer holds a post and two versions of it, and another holds the same post and only one of those versions
- **THEN** each resolves over the versions it holds
- **AND** neither reports an error

#### Scenario: A late arrival carrying a higher counter becomes current

- **WHEN** a version carrying a counter above the current version's arrives
- **THEN** it becomes current

#### Scenario: A late arrival carrying a lower counter does not displace the current version

- **WHEN** a version carrying a counter below the current version's arrives
- **THEN** the current version is unchanged

#### Scenario: A late arrival re-resolves rather than only advancing

- **WHEN** a version carrying no counter arrives after another carrying none that the degraded order places ahead of it
- **THEN** the newly arrived version becomes current
- **AND** the result still depends only on the set of ops held, not on the sequence they arrived in

**This scenario keeps its original name and its scope has narrowed to versions carrying no counter.** Among versions that carry counters the answer does only advance, which the two scenarios above pin. The non-monotone case survives for the ops that already exist, which is why the scenario survives with it.

#### Scenario: A version whose target the peer does not hold resolves to nothing

- **WHEN** a version is held but the post it names has never reached this peer
- **THEN** resolving that post reports the post's absence
- **AND** the version is not returned as the current version of a post the peer does not hold
