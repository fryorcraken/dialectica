# post-revision Specification

## Purpose

Defines which version of a post a reader renders: who may publish a new version, which of several versions is current, and what a reader resolves to when the ops it holds are incomplete, forged, or name something that is not a post.

## Requirements

### Requirement: A post is never edited in place

An edit to a post SHALL be a new op naming the post it supersedes, published and signed like any other op. No op SHALL modify a stored op, and resolving a post's current version SHALL NOT remove, rewrite or hide any op.

This is what makes post content require no merge function at all. Two versions of a post do not conflict; they are ordered. An order plus "only the author may revise" is a complete answer, so there is no CRDT, no merge, and no last-writer-wins ambiguity to resolve.

Keeping every superseded version is a requirement rather than a side effect: a reader can show that a post was edited, and a moderator acting on a post is acting on a version they can name by its own op id.

#### Scenario: A superseded version remains in the log

- **WHEN** a post has been revised and its current version resolved
- **THEN** the original post and every superseded revision are still readable from the log by their own op ids
- **AND** the number of ops the log holds is unchanged by resolving

#### Scenario: Resolving does not alter any op

- **WHEN** a post's current version is resolved
- **THEN** every op in the log reads back byte-identical, with its signature intact

### Requirement: Only the original post's author may publish a version of it

A version SHALL be treated as a version of a post only if its author is the author of the original post. A version whose author differs SHALL be dropped, regardless of whether its own signature is valid and regardless of its position in the order.

This is the load-bearing security property of the whole revision design. Without it any peer on the network can rewrite any other peer's post, since publishing an op requires no permission and a peer's own signature over their own op is always valid.

Authorship is compared against the original post, not against the version's own claims. A version carries an author field the version's own signature attests to, which establishes only that the named author really sent it — never that the named author owns the post being revised.

Because a version is valid only against the post's author, there is no case in which two authors contend for one post, and therefore no rule needed to decide between them.

#### Scenario: A version by a different author is dropped

- **WHEN** a version of a post is published by someone other than the post's author, correctly signed by that someone
- **THEN** it is not the post's current version
- **AND** the post resolves as though that version did not exist

#### Scenario: A different author's version is dropped even when it is the most recent

- **WHEN** the most recent version of a post by the ordering rule is by an author other than the post's author, and an older version by the post's own author exists
- **THEN** the post's own author's older version is current

#### Scenario: A different author's version is dropped when it is the only one

- **WHEN** the only version of a post is by an author other than the post's author
- **THEN** the original post is current

### Requirement: A version is verified before it is trusted

A version SHALL be verified to be authentic before its author is considered, and SHALL be dropped if it is not. The store holds whatever arrived, including forgeries, so validity SHALL be established when the post is read rather than assumed from the op's presence in the log.

An op's author field is a claim carried in the op; only verification turns that claim into a fact about who sent it. A version whose signature does not verify SHALL therefore be dropped whatever its author field says, so that writing a victim's key into that field gains an attacker nothing.

#### Scenario: A forged version is dropped

- **WHEN** a version carries the post author's name but a signature made with another key
- **THEN** it is not the post's current version
- **AND** the post resolves as though that version did not exist

#### Scenario: A version with an unparseable signature is dropped

- **WHEN** a version's signature is not a valid signature over its bytes
- **THEN** it is dropped, and no version is treated as current on the strength of its author field alone

#### Scenario: A forgery in the log does not displace a genuine version

- **WHEN** the log holds both a forged version that would be most recent and a genuine older version by the post's author
- **THEN** the genuine version is current

### Requirement: The current version is the one the ordering rule places first

Among the versions of a post that are authentic and by the post's author, the current version SHALL be the one the system's ordering rule places first. The resolver SHALL NOT define an order of its own, and SHALL NOT compare timestamps, arrival sequence or any other value itself.

The ordering rule is defined once for all ops, so a resolver comparing values itself would be a second implementation that could disagree with the first — and two orders that disagree produce no error, only two peers rendering one post differently.

This binds even though a resolver carrying a *correct* copy of the ordering rule would answer identically today. The contract is system-wide consistency across every reader of the log: a copy passes now and diverges from every other reader on the first change to the shared rule, silently, because two orders that disagree produce no error.

**Placing first is not the same as being most recent, and the difference is live rather than theoretical.** The ordering rule leads with the highest Lamport timestamp only where the transport supplied one. Where it did not — which is every op today, since no Lamport value reaches the system — the rule falls back to ascending op id, and an op id is a hash of the op's own bytes carrying no recency whatever.

So under the order in force today, the current version is a **convergent arbitrary choice rather than a temporal one**, and that is the property this requirement actually guarantees: two peers holding the same versions agree on which is current, even though neither can say which was written last. Convergence is what a forum needs to render consistently; temporal accuracy becomes available when the transport supplies its ordering metadata, at which point the same rule yields it with no change to this behaviour.

#### Scenario: The highest Lamport timestamp is current when the transport supplies one

- **WHEN** a post has several versions by its author at different Lamport timestamps
- **THEN** the version with the highest Lamport timestamp is current

#### Scenario: A tie is broken by the ordering rule, not by the resolver

- **WHEN** two versions by the post's author share a Lamport timestamp
- **THEN** the one the ordering rule places first is current
- **AND** the result is the same whichever sequence the versions were appended in

#### Scenario: The current version is the same on two peers holding the same ops

- **WHEN** two peers hold the same ops with the same recorded arrival metadata, appended in different sequences
- **THEN** both resolve the post to the same version

#### Scenario: Currency is convergent but not temporal when the transport supplies nothing

- **WHEN** a post's versions all arrived with no Lamport timestamp
- **THEN** the version the ordering rule's degraded order places first is current
- **AND** that result is derived from the ops alone, so every peer holding them agrees
- **AND** it is not derived from when any version was written or received

#### Scenario: A version the transport ordered leads one it did not

- **WHEN** one version of a post carries a Lamport timestamp and another carries none
- **THEN** the version the transport ordered is current, whatever its timestamp and whatever the other's op id

### Requirement: A reader can tell whether a post has been revised

Resolving a post SHALL report whether the version to render is the post as first published or a later version of it.

A reader shows an edited post differently from an unedited one, and cannot derive the distinction from the content alone: an author may revise a post to text identical to the original, and a post that was never revised is not distinguishable from one revised back. The answer SHALL therefore be determined by which op the current version is, not by comparing content.

A version that is dropped — by authorship, by verification, or by kind — SHALL NOT cause a post to be reported as revised. Reporting an edit that a reader then cannot see would be worse than reporting none, because it tells the reader their view is stale when it is correct.

#### Scenario: An unrevised post is reported as not revised

- **WHEN** a post with no versions is resolved
- **THEN** it is reported as not revised

#### Scenario: A revised post is reported as revised

- **WHEN** a post with a valid version by its author is resolved
- **THEN** it is reported as revised

#### Scenario: A version restoring the original content still counts as a revision

- **WHEN** a post is revised by its author to content identical to the original
- **THEN** it is reported as revised
- **AND** the report is not derived from comparing content

#### Scenario: A dropped version does not make a post look revised

- **WHEN** the only version of a post is one that is dropped, whether by a mismatched author, a failed verification, or a kind that is not a version
- **THEN** the post is reported as not revised

### Requirement: The current version's content wholly replaces the original's

The body and the attachments a reader renders SHALL be taken from the current version and from nothing else. An **empty** body or an **empty** attachment list in the current version SHALL be rendered as empty, and SHALL NOT be treated as an absent field for which the original's value is substituted.

A version replaces a post; it does not patch one. Emptiness is a value an author chose, not a gap to be filled — clearing a post's text and removing a post's images are both ordinary edits, and a resolver that fell back to the original when a field came back empty would silently restore content the author deleted, with no error and no way for the author to tell.

The attachment case is the sharper of the two, because attachments are storage addresses rather than display text: resurrecting one means a deleted image is still being fetched and rendered. A fallback of this kind is the defensive-looking mistake — it reads as robustness against a missing field and is data resurrection — so it is specified rather than left to judgement.

#### Scenario: A version that clears the body renders as empty

- **WHEN** a post is revised by its author with an empty body
- **THEN** the rendered body is empty
- **AND** the original's body is not substituted for it

#### Scenario: A version that removes every attachment renders with none

- **WHEN** a post carrying an attachment is revised by its author with an empty attachment list
- **THEN** the rendered attachment list is empty
- **AND** the original's attachments are not substituted for it
- **AND** the revision is still reported as the current version

#### Scenario: A version that changes the attachments replaces them entirely

- **WHEN** a post carrying one attachment is revised by its author with different attachments
- **THEN** exactly the revision's attachments are rendered
- **AND** the original's attachment is not among them

### Requirement: Every version names the original post, and a version of a version is not one

A version SHALL name the original post as the op it supersedes. An op naming a *version* rather than the original post SHALL NOT be resolved as a version of that post.

The rule is stated as an edit being a new version of *the post*, which is one named subject rather than a chain. Two consequences make the flat form the correct reading rather than merely a permitted one.

A chain is not resolvable over a partial set. A peer routinely holds some ops and not others. Under the flat rule, a peer missing a version simply resolves over the versions it has, and every peer holding the same set reaches the same answer. Under a chain, a peer missing one *link* cannot reach the versions beyond it at all — so two peers holding the same set of revisions would resolve differently depending on which intermediate links each happened to receive, and the missing link is not observable as missing. Every version naming the original is what makes each version stand alone, so the answer depends on the set of ops held and nothing else.

A chain also has no defined answer when it forks. An author who publishes two versions of one version has created two branches, and nothing in the ordering rule chooses between branches rather than between ops — it orders ops, and the two heads are ordered against each other perfectly well while their chains say different things about what came before.

#### Scenario: A version of a version is not a version of the post

- **WHEN** an author publishes a version of their post and then an op naming that version as its subject
- **THEN** the post's current version is the version naming the post
- **AND** the op naming the version is not the post's current version

#### Scenario: Every version competes against every other directly

- **WHEN** an author publishes several versions of one post, each naming the original post
- **THEN** all of them are candidates for current, ordered against one another by the ordering rule

### Requirement: A post resolves over the ops the peer happens to hold

Resolving SHALL produce an answer from the ops the log holds, and SHALL NOT report an error, an incomplete result, or an unknown outcome on the grounds that other ops may exist elsewhere.

Two peers routinely hold different sets of ops — one was offline, one joined late, a message has not propagated. A peer holding fewer versions resolves over the ones it has, and that is the correct answer for that peer rather than a defect. Completeness is not a property any peer can establish about itself, so a resolver that required it could never answer at all.

The answer SHALL be a pure function of the ops held, so that two peers with the same set agree whatever sequence those ops arrived in. It is **not** required to advance monotonically as ops arrive: under the degraded order a version arriving later may become current, because op id carries no recency. That is a property of the order in force rather than of this resolver, and a reader that assumed monotonicity held everywhere would be relying on a guarantee the degraded order does not make.

#### Scenario: A post with no versions resolves to itself

- **WHEN** a post that has never been revised is resolved
- **THEN** the original post is current

#### Scenario: A peer holding fewer versions resolves over the ones it has

- **WHEN** one peer holds a post and two versions of it, and another holds the same post and only one of those versions
- **THEN** each resolves over the versions it holds
- **AND** neither reports an error

#### Scenario: A late arrival re-resolves rather than only advancing

- **WHEN** a version arrives after another that the degraded order places ahead of it
- **THEN** the newly arrived version becomes current
- **AND** the result still depends only on the set of ops held, not on the sequence they arrived in

#### Scenario: A version whose target the peer does not hold resolves to nothing

- **WHEN** a version is held but the post it names has never reached this peer
- **THEN** resolving that post reports the post's absence
- **AND** the version is not returned as the current version of a post the peer does not hold

### Requirement: Only a post has versions

Resolving SHALL report the absence of a post when the named op is not a post, and SHALL NOT return a version of it. An op that is not a post SHALL NOT be treated as having a current version.

The rule defines a version of a *post*, and only posts carry the body a version replaces. Resolving a vote or a moderation op as though it had versions would let an op naming one of those substitute content into a reader's view of something that has no content to substitute.

Reporting absence rather than a distinct failure keeps the resolver's outcomes to two, which is what the caller can act on: there is a current version to render, or there is nothing to render. A caller that holds a vote's op id and asks for its current version has made a category mistake, and there is no rendering for it either way.

#### Scenario: A vote has no current version

- **WHEN** a vote's op id is resolved as though it were a post
- **THEN** the result reports absence
- **AND** no version naming that vote is returned

#### Scenario: A moderation op has no current version

- **WHEN** a moderation op's op id is resolved as though it were a post
- **THEN** the result reports absence

#### Scenario: A version is not itself a post with versions

- **WHEN** a version's own op id is resolved as though it were a post
- **THEN** the result reports absence

#### Scenario: An op id the log does not hold reports absence

- **WHEN** an op id no op in the log carries is resolved
- **THEN** the result reports absence
- **AND** no error is reported

### Requirement: Resolving a post never aborts the process

Resolving SHALL NOT panic for any contents the log may hold, including ops that do not verify, ops naming absent targets, ops naming themselves, and ops of every kind naming one target at once.

Everything in the log arrived from a peer and is attacker-controlled. A panic here aborts the module process, which turns a malformed or malicious op into a denial of service against the peer that received it.

#### Scenario: A log of adversarial ops resolves without a panic

- **WHEN** a post is resolved against a log holding forged versions, versions by strangers, versions naming absent targets, and ops of every other kind naming the post
- **THEN** resolving returns an answer without panicking

#### Scenario: An op naming itself as its subject does not loop

- **WHEN** the log holds an op naming its own op id as the op it acts upon
- **THEN** resolving terminates
