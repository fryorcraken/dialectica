## Purpose

Defines the orderings a reader may request over a Stoa's posts, what a vote contributes to one, and the bounds an ordering SHALL respect however the ops it reads were produced.

This capability governs the **shape and bounds** of the engagement ordering, not its arithmetic. The weight a moderator's upvote carries is a constant in the scorer, deliberately outside this contract: a score is a local projection rather than an op, so retuning it is a code change and pinning a number here would make it a contract change instead.

## ADDED Requirements

### Requirement: An ordering is derived from ops and published as nothing

An ordering SHALL be computed from the ops a peer holds and SHALL NOT be published, transmitted, or carried in any op. No op SHALL contain a score, a rank, or a position.

A score no peer can verify is a claim rather than a fact, and a published one would invite a peer to assert a ranking instead of deriving it. Deriving it locally is also what allows the arithmetic to change without a wire-format version bump, which is the property wanted for the part of the system that will be retuned repeatedly.

It follows that two peers holding different ops SHALL rank differently, and this is correct rather than a fault. Divergent op sets are the ordinary case, and no reconciliation SHALL be attempted between two peers' orderings.

#### Scenario: No op carries a score

- **WHEN** any op is encoded
- **THEN** its bytes contain no score, rank or position field

#### Scenario: Two peers holding different ops rank differently

- **WHEN** one peer holds a vote op that another does not, and both order the same posts
- **THEN** the two orderings may differ
- **AND** neither peer reports an error, and neither adjusts its ordering toward the other's

#### Scenario: An ordering is a function of the ops held

- **WHEN** two peers hold the same ops with the same recorded arrival metadata, appended in different sequences
- **THEN** both produce the same ordering

### Requirement: A hidden post is excluded from every ordering, never ranked down

A post whose moderation resolves as hidden SHALL be absent from an ordering's results. It SHALL NOT be assigned a reduced score, a penalty multiplier, or a position at the end of the results.

A moderation op is a binding judgement, and a percentage penalty does not bind: a sufficiently upvoted hidden post would outrank a visible one, so a reader would see exactly what a moderator removed. Exclusion is what makes the moderator's decision hold regardless of any score.

Exclusion SHALL apply before scoring rather than after, so that no ordering can produce a hidden post by any combination of votes.

#### Scenario: A hidden post is absent whatever its votes

- **WHEN** a post with more votes than any other in its Stoa resolves as hidden
- **THEN** it is absent from the ordering
- **AND** no result names it at any position

#### Scenario: An unhidden post returns to the ordering

- **WHEN** a hidden post is subsequently unhidden by a binding moderation
- **THEN** it appears in the ordering again
- **AND** its position is derived from its votes, not reduced by having been hidden

#### Scenario: Hiding is not a score adjustment

- **WHEN** one post is hidden and another with fewer votes is not
- **THEN** the visible post appears and the hidden one does not
- **AND** the outcome does not depend on the relative number of votes

### Requirement: Exclusion follows the moderation resolver, however it is stored

Whether a post is excluded SHALL be the answer the moderation resolver gives for that post. Where an ordering keeps that answer in stored form for retrieval, the stored value SHALL be produced by consulting the resolver, and SHALL NOT be derived by any second evaluation of moderation authority.

Two implementations of one rule produce no error when they disagree — only two readers rendering one Stoa differently, or one reader's stored view drifting from what the same reader would resolve on demand. The resolver already settles authenticity, authority, scope, ordering and the degraded-order preference; an ordering that re-derived any of those would be a second copy that passes today and diverges on the first change to the shared rule.

A stored answer SHALL be brought up to date whenever a moderation op is received, and SHALL be recomputed rather than toggled from the received op's own action, since a received moderation op is not necessarily the one that decides. It SHALL likewise be brought up to date for a Stoa whose genesis record arrives after ops it authorises, because no answer about that Stoa's targets was available before it.

Exclusion SHALL NOT be represented as an adjustment to a post's score. A sufficiently voted post must not re-enter an ordering by accumulating votes.

#### Scenario: A stored exclusion matches what the resolver reports

- **WHEN** a post's exclusion is retrieved from an ordering and separately resolved directly
- **THEN** the two agree

#### Scenario: An unauthorised moderation arriving does not change exclusion

- **WHEN** a moderation op that does not bind is received for a post already resolved as not hidden
- **THEN** the post remains present in the ordering

#### Scenario: A received hide that does not decide does not exclude

- **WHEN** a `hide` is received for a target whose binding moderation the resolver still reports as an `unhide`
- **THEN** the target remains present in the ordering

#### Scenario: A late genesis record settles a Stoa's exclusions

- **WHEN** moderation ops for a Stoa are received before its genesis record, and the genesis record arrives afterward
- **THEN** the ordering thereafter excludes exactly the posts the resolver reports as hidden

#### Scenario: Votes cannot return an excluded post

- **WHEN** an excluded post accumulates more votes in the raising direction than any present post has
- **THEN** it remains absent from the ordering

### Requirement: An engagement ordering counts distinct voting identities

Where an ordering counts votes, it SHALL count each voting identity at most once per target, taking that identity's current vote as decided by the system's ordering rule. It SHALL NOT accumulate repeated votes from one identity, and SHALL NOT define a recency rule of its own.

An identity that votes repeatedly on one target has expressed one opinion and changed it. Counting each op would let a single identity multiply its own weight by publishing, which is the cheapest lever available and needs no minted identities at all.

The ordering rule is defined once for all ops, so a scorer comparing timestamps or arrival sequence itself would be a second implementation that could disagree with the first — and two rules that disagree produce no error, only two peers ranking one post differently.

#### Scenario: An identity voting twice is counted once

- **WHEN** one identity publishes two upvotes on one target
- **THEN** the target's count reflects one vote from that identity

#### Scenario: A changed vote replaces rather than accumulates

- **WHEN** an identity upvotes a target and later downvotes it
- **THEN** the identity contributes a downvote and not both
- **AND** which vote is current is decided by the system's ordering rule

#### Scenario: A vote's currency is not decided by arrival sequence

- **WHEN** two peers hold one identity's two votes on a target, appended in different sequences
- **THEN** both count the same one

### Requirement: A vote counts only where it is authentic and in scope

A vote SHALL contribute to an ordering only if it verifies against the key it names and names the Stoa whose ordering is being computed. A vote that fails either check SHALL contribute nothing, in either direction.

The store holds whatever arrived, including forgeries, so validity is established when the ordering is computed rather than assumed from an op's presence in the log. A vote's author field is a claim the op carries; only verification turns it into a fact about who voted.

Scope is checked separately from authenticity because an authentic vote naming another Stoa is genuinely from its signer and is genuinely not a vote in this Stoa. A signature refuses a replay; only comparing the op's Stoa to the Stoa being ordered refuses a deliberate cross-Stoa vote.

#### Scenario: A forged vote contributes nothing

- **WHEN** a vote names one identity as its author but was signed with another key
- **THEN** it contributes nothing to the target's position

#### Scenario: A vote naming another Stoa contributes nothing

- **WHEN** a validly signed vote names a Stoa other than the one being ordered
- **THEN** it contributes nothing to the target's position

#### Scenario: A forged downvote cannot lower a position

- **WHEN** a target's only downvotes fail verification
- **THEN** the target's position is the same as if those ops were absent

### Requirement: A weight above the ordinary comes from the system or from the reader, never from the voter

An ordering MAY weight a vote above the ordinary weight only where the distinction is conferred either by state the whole Stoa derives identically, or by a declaration the reader computing the ordering made themselves. A distinction a voter can confer on themselves SHALL NOT increase a vote's weight, and no property an op asserts about its own author SHALL do so.

The prohibited case is the one that matters: an attacker who can create identities can create identities carrying any self-asserted property, so weighting by one decorates a quantity the attacker already controls rather than costing them anything.

The two permitted sources are safe for different reasons, and neither reason transfers to the other. A Stoa-derived credential — the moderator set obtained from the genesis record — is safe because the creator's key sits inside the address preimage and an address is what a Stoa is, so a minted identity is not a moderator of any existing Stoa. A reader-declared weight is safe because it is not a claim about the world at all: it changes only the ranking of the reader who declared it, so an attacker who obtains one has persuaded exactly one person and gained nothing they did not already have.

This requirement bounds what may be weighted. It does not require that anything is, and it fixes no weight.

#### Scenario: A vote by a moderator may weigh more than one by a non-moderator

- **WHEN** a Stoa's moderator and a non-moderator each upvote a different post, all else equal
- **THEN** the moderator's may place its post ahead

#### Scenario: A newly created identity's vote carries no elevated weight

- **WHEN** an identity with no history votes
- **THEN** its vote carries the ordinary weight
- **AND** no property the identity can assert about itself raises that weight

#### Scenario: A moderator of another Stoa is not credentialed here

- **WHEN** the moderator of one Stoa votes in a Stoa they do not moderate
- **THEN** their vote carries the ordinary weight

#### Scenario: A reader's own declaration may weight a vote

- **WHEN** a reader has declared that it weighs a given identity's opinion above the ordinary, and that identity votes
- **THEN** that vote may weigh more than the ordinary in that reader's ordering

#### Scenario: One reader's declaration does not change another reader's ordering

- **WHEN** one reader has declared such a weighting and another has not, and both order the same ops
- **THEN** the second reader's ordering is unchanged by the first's declaration
- **AND** neither reader is in error

#### Scenario: A declaration is not derived from any op

- **WHEN** an ordering is computed over a log containing ops that assert standing, endorsement or reputation for their authors
- **THEN** no such op raises any vote's weight

### Requirement: A weight may amplify promotion but never suppression

Where an ordering weights a vote above the ordinary weight, that weighting SHALL apply only to a vote that raises a target's position. A weighted vote that lowers a target's position SHALL carry the ordinary weight. This SHALL hold for every source of weight.

Amplified suppression is a moderation action without moderation's properties. A moderation op binds, names a deciding op, and has a specified inverse; an amplified downvote does none of these — it reduces a post's visibility by a proportion that a sufficiently voted post survives, and nothing identifies it as a moderator's decision or reverses it as one.

Offering a moderator an amplified but non-binding way to suppress something is worse than offering none, because it is the nearer tool and it does not do what it appears to do. Suppression is a binding judgement or it is nothing.

A reader-declared weight is no exception, and its own reason is separate: amplifying the suppressions of those a reader has chosen to weigh would remove from that reader's view whatever their chosen voters disliked, which builds a self-reinforcing filter rather than a ranking. The asymmetry keeps such a declaration a statement about whose recommendations to surface, never about whose objections to act on.

#### Scenario: A moderator's downvote weighs the same as anyone's

- **WHEN** a Stoa's moderator downvotes a target
- **THEN** the target's position falls by the same amount as it would for a non-moderator's downvote

#### Scenario: A moderator's upvote and downvote are not symmetric

- **WHEN** a moderator upvotes one target and downvotes another, both otherwise unvoted
- **THEN** the magnitude of the change to the upvoted target's score is not equal to the magnitude of the change to the downvoted one's

#### Scenario: A reader-declared weight does not amplify a downvote

- **WHEN** an identity a reader has declared a weighting for downvotes a target
- **THEN** the target's position falls by the same amount as it would for an undeclared identity's downvote

### Requirement: Negative engagement orders a post last and never removes it

A target's engagement contribution to its position SHALL be bounded below, such that no accumulation of votes in the lowering direction removes a post from an ordering, places it behind a hidden post, or causes it to be withheld from a reader.

Downvoting and upvoting are not symmetric in their consequences. A wrongly promoted post is the most examined object in a forum and announces the attack that promoted it; a wrongly buried post is invisible by construction, so nobody reviews it, the author cannot distinguish brigading from indifference, and the suppression leaves no record. Where the signal is uncertain, the ordering fails toward visible.

A threshold at which accumulated lowering votes withhold a post SHALL NOT exist. Such a threshold is moderation performed by whoever creates the most identities, and it produces the binding effect that only a signed moderation op is entitled to produce.

#### Scenario: A heavily downvoted post is still present

- **WHEN** a post accumulates more votes in the lowering direction than any other post has in either direction
- **THEN** it appears in the ordering
- **AND** it is not reported as hidden

#### Scenario: Further downvotes stop moving a floored post

- **WHEN** a post already at the lower bound receives additional lowering votes
- **THEN** its position relative to other visible posts does not fall further

#### Scenario: A downvoted post still outranks nothing that is excluded

- **WHEN** a post at the lower bound is ordered alongside a hidden post
- **THEN** the downvoted post appears and the hidden post does not

### Requirement: An ordering is total, so pagination neither repeats nor skips

An ordering SHALL place every two distinct posts in a defined order, including posts whose scores are equal. Ties SHALL be broken by a value derived from the ops themselves and identical on every peer.

Paginated reads return successive ranges of one ordering. Where two posts compare equal, their relative positions are free to differ between two evaluations of the same query, so a reader paging through results can be shown one post twice and never shown another — with no error at either end. Equal scores are the ordinary case rather than a corner: every post with no votes ties with every other.

The tiebreak SHALL NOT be arrival sequence, a local clock reading, or a row identifier assigned by the store, since each of those differs between peers and would make the ordering depend on something other than the ops held.

#### Scenario: Equally scored posts have a defined relative order

- **WHEN** two posts have equal scores
- **THEN** one is placed before the other
- **AND** the same one is placed first on every evaluation

#### Scenario: Paging over equally scored posts returns each once

- **WHEN** a reader retrieves successive pages of an ordering in which every post is equally scored
- **THEN** each post appears exactly once across the pages
- **AND** no post is omitted

#### Scenario: The tiebreak is derived from the ops

- **WHEN** two peers hold the same equally scored posts, appended in different sequences
- **THEN** both place them in the same relative order

### Requirement: Age influences an ordering only through a value no author supplies

Where an ordering accounts for a post's age, the value it reads SHALL NOT be one the post's author supplied or can influence. An ordering SHALL NOT read a timestamp carried inside an op.

An op carries no timestamp precisely because a wall clock is a field the adversary sets. An ordering that read one would reintroduce the field the op format refuses: a post claiming a future time receives an unbounded advantage, and the claim costs nothing to make.

Where no such value is available for a post, the ordering SHALL proceed with no age adjustment for it, and SHALL NOT substitute a default, treat the post as maximally old, or omit it.

#### Scenario: No ordering reads an author-supplied time

- **WHEN** an ordering is computed
- **THEN** no value inside any op is read as a time

#### Scenario: A post with no recorded age is ordered without an age adjustment

- **WHEN** a post has no recorded value from which its age can be derived
- **THEN** it is placed by its other inputs alone
- **AND** it is neither omitted nor treated as older than every other post

### Requirement: Computing an ordering never aborts the process

Computing an ordering SHALL NOT panic for any combination of ops the op decoder accepts, whatever their signatures, authors, kinds, Stoas, directions, or recorded arrival metadata. This SHALL include votes naming absent targets, votes naming votes, votes naming the Stoa's genesis, and counts large enough to overflow a fixed-width accumulator.

Every op read arrived from a peer and is attacker-controlled. A panic here aborts the module process, turning a hostile op into a denial of service against the peer that received it — and an ordering reads more ops at once than any resolver, so an accumulator that overflows on a sufficiently voted target is reachable by publishing votes, which requires no permission.

#### Scenario: An ordering over adversarial ops returns an answer

- **WHEN** an ordering is computed over a log holding forged, unsigned, cross-Stoa and wrong-kind ops naming its posts
- **THEN** an ordering is returned
- **AND** no operation panics

#### Scenario: A vote naming an absent target does not abort

- **WHEN** a vote names a target op the log does not hold
- **THEN** the ordering is computed without panicking
- **AND** no post is created for the absent target

#### Scenario: Accumulated votes do not overflow

- **WHEN** a target carries enough votes in one direction to exceed the range of the accumulator used to total them
- **THEN** the ordering is returned without panicking
