# thread-read Specification

## Purpose
Defines reading one thread: what identifies it, which of the ops a peer holds belong to it, what order and what page they come back in, what a reader is shown of a hidden root and a hidden reply, and what distinguishes a thread this peer does not hold from one holding no replies.

Three boundaries are named rather than restated, because each is another capability's and two specs asserting one rule is how two copies drift.

- **`op-ordering` owns what orders two ops**, including the degraded order in force while the transport supplies no ordering metadata. This capability places posts in that order and defines none of its own.
- **`post-revision` owns which version of a post is current**, who may publish one, and what a reader renders of it. This capability reports that answer per post and does not restate how it is reached.
- **`moderation-resolution` owns whether a target is hidden** and which op decided it. This capability decides what a thread read *does* with that answer, which is a different question.

Publishing anything, edit history, attachment retrieval and any tally over votes are out of scope.

## Requirements

### Requirement: A thread is identified by its root post's op id, and that identifier never moves

A thread SHALL be named by the op id of the post that started it — the post in that Stoa whose parent is absent. A caller SHALL ask for a thread by that op id.

That identifier SHALL NOT change when the root post is revised. A revision is a distinct op with its own op id, and a reader therefore holds two facts about the root at once: the thread's identifier, which is the original post's op id and is stable; and the op id of the version being rendered, which differs the moment the post has been edited. Both SHALL be reported, as two fields, because they are two facts and a caller collapsing them would have to re-derive one.

Stability is what makes the identifier usable at all: it is the value a reply names, the value a view holds across a refresh, and the value a caller passes back to read the next page. An identifier that moved on every edit would make an open thread unreadable the moment its author fixed a typo.

#### Scenario: A thread is read by its root's op id

- **WHEN** a thread is read by the op id of a post whose parent is absent
- **THEN** the reply carries that post
- **AND** the thread identifier it reports is the op id that was asked for

#### Scenario: Revising the root does not move the thread's identifier

- **WHEN** the root post of a thread is revised by its author, and the thread is read
- **THEN** the thread is still readable by the original post's op id
- **AND** the root's reported thread identifier is that original op id

#### Scenario: The rendered version's identifier is reported separately from the thread's

- **WHEN** a thread whose root has been revised is read
- **THEN** the root's reported version identifier is the revision's op id
- **AND** it differs from the thread's identifier
- **AND** for a root that has never been revised the two are equal

#### Scenario: Reading by a revision's op id is not reading the thread

- **WHEN** a thread read names the op id of a revision of a root post rather than the root post's own
- **THEN** the reply does not return that thread
- **AND** the refusal is the one for an op the peer holds that is not a post, a revision being exactly that
- **AND** it is not the refusal for an op the peer does not hold, which would be false about an op it has

### Requirement: Membership is derived from the parent chain, and a post's own thread field is never trusted

A post SHALL belong to the thread whose root is reached by following its parent, and its parent's parent, until a post with no parent is reached. The `thread` value carried inside a post's own signed bytes SHALL NOT be used to decide which thread that post is returned under.

**This is the security property of this capability.** A post's `thread` field is its author's claim and nothing more. The publish path derives that value from the parent for ops this peer creates, but every op arriving from a peer carries whatever its author chose, and the log stores it unexamined because the log decides nothing. A reader that placed posts by the claimed field would let any peer inject a post into any thread it names — including a thread in which that peer's post has no parent at all — and the injected post would render among a conversation it was never part of, signed and verifying, with nothing to mark it as out of place.

A post whose claimed `thread` names one thread while its parent chain reaches another SHALL be returned under the thread its parent chain reaches, and SHALL NOT be returned under the thread it claims.

A post whose parent chain cannot be completed — because some post in it is not held by this peer — SHALL NOT be returned under any thread, whatever it claims. A reader cannot establish where such a post belongs, and placing it by the claim is the one answer this requirement forbids. This is the ordinary consequence of a peer holding an incomplete set of ops rather than a fault, and the post becomes placeable when the missing ops arrive.

A post whose parent chain reaches an op that is not a post SHALL likewise not be returned under any thread.

Following the chain SHALL terminate for every set of ops the log may hold, including a post naming itself as its parent and parent references forming a cycle of any length. A post in such a chain reaches no root and SHALL therefore be returned under no thread. Parent references are attacker-supplied, so a chain walk that could be made not to terminate would turn one malformed op into a denial of service against the peer that received it.

#### Scenario: A reply is returned under the thread its parent belongs to

- **WHEN** a reply names a root post as its parent and that thread is read
- **THEN** the reply is among the items returned

#### Scenario: A reply to a reply is returned under the same thread as its parent

- **WHEN** a reply names another reply as its parent, and the thread containing that other reply is read
- **THEN** the deeper reply is among the items returned
- **AND** it is not returned under a thread rooted at its immediate parent

#### Scenario: A post claiming a thread its parent does not belong to is placed by its parent

- **WHEN** the log holds two threads, and a validly signed reply whose parent is in the first thread carries a `thread` field naming the second
- **THEN** reading the first thread returns that reply
- **AND** reading the second thread does not return it

#### Scenario: A forged thread claim cannot inject a post into a thread

- **WHEN** a peer publishes an authentically signed post whose `thread` field names a thread it has no parent in, and that thread is read
- **THEN** the post is not among the items returned

#### Scenario: A post whose parent the peer does not hold is not placed by its claim

- **WHEN** the log holds a reply naming a parent op id no op in the log carries, and that reply's `thread` field names a thread the log does hold
- **THEN** reading that thread does not return the reply
- **AND** no error is reported for the thread read

#### Scenario: A post becomes placeable when its missing parent arrives

- **WHEN** a reply whose parent was absent is read as part of a thread, the parent op is then appended, and the thread is read again
- **THEN** the second read returns the reply
- **AND** the first read did not

#### Scenario: A chain reaching an op that is not a post places nothing

- **WHEN** the log holds a post whose parent op id names a vote
- **THEN** no thread read returns that post

#### Scenario: A cycle among parents does not prevent an answer

- **WHEN** the log holds posts whose parent references form a cycle, so that no chain among them reaches a post with no parent
- **THEN** a thread read over that log returns an answer rather than failing to terminate
- **AND** none of the posts in the cycle is returned under any thread

#### Scenario: A post naming itself as its parent is not a root

- **WHEN** the log holds a post whose parent op id is its own
- **THEN** a thread read naming that op id is refused rather than serving it as a root
- **AND** the post is returned under no other thread
- **AND** a thread read over that log terminates

#### Scenario: A genuine thread is unaffected by adversarial chains elsewhere

- **WHEN** a thread is read against a log that also holds a self-parenting post, a parent cycle, and posts claiming that thread without a parent in it
- **THEN** exactly the posts whose parent chains reach that thread's root are returned

### Requirement: Only authentic posts in the named Stoa are returned

A thread read SHALL verify each op's signature before any of its fields is used to place or render it, and SHALL NOT return an op that fails verification. It SHALL return only ops belonging to the Stoa named in the request.

The store holds whatever arrived, forgeries included, so authenticity is established when a thread is read rather than assumed from an op's presence in the log. An op's author field is a claim carried in the op; only verification turns that claim into a fact about who sent it. A thread read that skipped this would render a post attributed to whoever an attacker named.

Verification SHALL run before an op's parent is followed, so that a forged op cannot place a genuine one or be placed by one.

#### Scenario: A forged reply is not returned

- **WHEN** a thread contains a reply whose author field names one key and whose signature was made with another, and the thread is read
- **THEN** that reply is not among the items returned
- **AND** the genuine posts of the thread are

#### Scenario: A forged root is not readable as a thread

- **WHEN** a thread read names the op id of a post whose signature does not verify
- **THEN** the reply is the refusal for a thread this peer does not hold, an unverified op being no usable post
- **AND** it is not the not-a-post refusal, which would tell the caller its identifier named the wrong kind of thing when the kind was never established
- **AND** the message is the same one an op id the log holds nothing for produces, so nothing in it reports that bytes were found

#### Scenario: A forgery in the log does not displace genuine posts

- **WHEN** a thread is read against a log holding both a forged reply and genuine ones
- **THEN** exactly the genuine replies are returned
- **AND** the forged op is still in the log, so the result is not an artefact of it never having been stored

#### Scenario: Another Stoa's posts are never returned

- **WHEN** a thread read names one Stoa, and the log also holds posts in a second Stoa
- **THEN** no post belonging to the second Stoa is among the items returned

#### Scenario: A thread read names a Stoa the root does not belong to

- **WHEN** a thread read names a Stoa and a root op id belonging to a different Stoa
- **THEN** the reply is the refusal for a thread this peer does not hold in that Stoa
- **AND** it is not the not-a-post refusal, the op being a perfectly good post of another Stoa
- **AND** the message is the same one an op id the log holds nothing for produces, so it names neither the Stoa the op belongs to nor the fact that it was found
- **AND** no post of either Stoa is returned

### Requirement: An author is reported as both an address and a public key, and never as a name

Each item SHALL report its author's per-Stoa address **and** the public key that signed that op. Both SHALL be present on every item, and neither SHALL be omitted in favour of the other.

**Two fields are required because two different digests are rendered from them, and neither input can be recovered from the other.** A reader is shown a generated display name and a generated mark beside every post. The name is derived from the author's **public key**; the mark is derived from the author's **address**; and an address is a one-way hash of a record rather than a transformation of the key. So an item carrying only an address is one whose name cannot be computed by anything that receives it, and an item carrying only a key is one whose address a reader would have to trust rather than check.

The address SHALL NOT be dropped once the key is present. The address is the only unforgeable identifier of the two in a reader's hands: a name and a mark are both cheaply re-rolled to resemble someone else's, and only the address settles who published something. A surface that offered recognition aids without it would give a reader three ways to be reassured and no way to be sure.

**No item SHALL carry a display name.** A name is a pure function of the public key, so sending one from here would put a second, derivable identifier on the wire beside the material it is derived from — where the two could disagree, and a reader would have no way to tell which was wrong. Deriving names is the job of whatever renders them, and this contract supplies the input rather than the output. The same holds for the mark.

The author fields reported SHALL be those of the key that actually signed the op, which verification has already established, and SHALL NOT be taken from an unverified claim.

#### Scenario: Each item carries both an address and a key

- **WHEN** a thread's posts are returned
- **THEN** every item carries its author's address
- **AND** every item carries its author's public key

#### Scenario: The two fields identify the key that signed

- **WHEN** a thread containing posts by two different authors is read
- **THEN** each item's public key is the one whose signature verifies that op
- **AND** each item's address is the address derived from that same key
- **AND** the items by different authors differ in both fields

#### Scenario: The two fields are distinct values and neither substitutes for the other

- **WHEN** an item's address and public key are compared
- **THEN** they are different values
- **AND** the address is the one the identity rules derive from that key, so a caller can check the pairing rather than trust it

#### Scenario: No item carries a derived display name

- **WHEN** a thread is read and every field of an item is enumerated
- **THEN** the only fields describing the author are the address and the public key
- **AND** the item carries no third author-describing field, which is what a name or a mark would have to be

### Requirement: Each returned post renders its current version and says whether it was revised

Each item SHALL carry the body and the attachment references of that post's current version, SHALL report whether the post has been revised, and SHALL identify the parent post it replies to where it has one.

The body and attachment references SHALL be taken from the current version and from nothing else, so that a thread read shows what an author's latest version says rather than text the author replaced. Whether a post was revised SHALL be determined by which op the current version is, and SHALL NOT be derived by comparing content: an author may revise a post to text identical to the original, and a post never revised is not distinguishable from one revised back.

A root post's parent SHALL be reported as absent, which is what makes the root identifiable within a flat page without the caller comparing op ids. Every other item SHALL report a parent.

**A reported parent is an op id and not a promise that the parent is among the items.** A parent may be missing from the page because it falls on another page, and it may be missing from every page because it is hidden and the caller did not ask for hidden content. A caller SHALL therefore be able to render an item whose parent it does not hold in hand, and this contract does not require a returned item's parent to be returned with it.

#### Scenario: A revised reply renders the revision's body

- **WHEN** a reply has been revised by its author and its thread is read
- **THEN** that item's body is the revision's body
- **AND** the original's body is not returned in its place
- **AND** the item is reported as revised

#### Scenario: An unrevised post is reported as not revised

- **WHEN** a thread containing a post with no versions is read
- **THEN** that item is reported as not revised
- **AND** its version identifier equals its own op id

#### Scenario: A revision that clears a body renders as empty

- **WHEN** a reply is revised by its author with an empty body and its thread is read
- **THEN** that item's body is empty
- **AND** the original's body is not substituted for it

#### Scenario: A revision by someone other than the author does not change what is rendered

- **WHEN** a log holds a revision of a reply published by someone who is not that reply's author, and the thread is read
- **THEN** the item renders the body the reply's own author last published
- **AND** the item is not reported as revised on the strength of that op

#### Scenario: A reply names the post it replies to

- **WHEN** a thread containing a reply to another reply is read
- **THEN** that item reports its parent as the op id of the post it replies to

#### Scenario: The root reports no parent

- **WHEN** a thread is read
- **THEN** the root item reports no parent
- **AND** every other item reports one

#### Scenario: A reported parent need not be among the items

- **WHEN** a thread whose replies span more than one page is read one page at a time
- **THEN** an item whose parent falls on an earlier page still reports that parent's op id
- **AND** the read is not refused for the parent being off the page

### Requirement: An item's moderation state is three-valued and names the op that decided it

Each item SHALL report its moderation state as one of three values — that no binding moderation was found for it, that a moderation hid it, or that a moderation deliberately restored it — and SHALL NOT collapse them into a single flag. Where the state is one of the latter two, the item SHALL identify the moderation op that decided it. Where no binding moderation was found, no deciding op SHALL be named, and the field SHALL be omitted rather than sent holding no value.

A flag loses two things a reader needs. The first is the difference between a post nobody moderated and a post a moderator looked at and deliberately restored — an untouched post and a vindicated one, which read identically under a boolean and mean different things. The second is the identity of the decision: a reader shown that something was moderated and not shown by which op has been told a conclusion it cannot examine, and nothing that later wants to name a specific decision has anything to name.

Which moderations bind, and which of several decides, are settled elsewhere and SHALL NOT be re-decided here. This requirement governs only that the answer reaches the caller whole rather than flattened.

**This requirement binds this read alone, and SHALL NOT be read as describing what any other read reports.** The feed currently reports moderation as a boolean built from the same resolver, so an item from a feed and an item from a thread do not carry the same moderation shape, and a view handling both must today tell which call produced which. That divergence is a known gap in core rather than a licence: it is the wire convention that JSON shapes are source-independent, being unmet. Whichever change closes it SHALL bring the other read to this shape rather than bring this one to a flag, because a flag cannot express the restored state at all — a contract narrowing to fit the weaker of two shapes would lose the distinction this requirement exists for.

#### Scenario: An unmoderated post names no deciding op

- **WHEN** a thread none of whose posts has been moderated is read
- **THEN** every item reports the state for no binding moderation
- **AND** no item names a deciding moderation

#### Scenario: A hidden post names the op that hid it

- **WHEN** a thread containing a post a moderator hid is read with hidden content included
- **THEN** that item reports the hidden state
- **AND** it identifies the moderation op that decided it

#### Scenario: A restored post is distinguishable from one never moderated

- **WHEN** a moderator hides a post and then unhides it, and the thread is read
- **THEN** that item is not reported as hidden
- **AND** its state is distinguishable from that of a post nobody moderated
- **AND** it identifies the unhide as the op that decided it

#### Scenario: A forged moderation decides nothing and is named by nothing

- **WHEN** a thread is read against a log holding a moderation of one of its posts that fails authenticity or authority
- **THEN** that item reports the state for no binding moderation
- **AND** it does not name that op as having decided anything

### Requirement: A hidden root is returned and marked, never silently dropped

Where a thread's root post is hidden by a binding moderation, the thread read SHALL still return that thread and SHALL report the root as hidden. It SHALL NOT refuse the read and SHALL NOT return an empty page in place of the thread.

**By default the root's body and attachment references SHALL be withheld** — reported as absent rather than as an empty value, so that a caller cannot mistake withheld content for content an author cleared. Where the caller has asked for hidden content to be included, the root's body and attachment references SHALL be returned along with the mark that it is hidden.

This is where a thread read must differ from a feed, and the difference is forced rather than stylistic. A feed omits a hidden thread from a list of many and the reader still has a feed. A thread read that omitted its own subject would answer the caller with nothing, and nothing is exactly what a thread this peer does not hold also produces — so the two states would be indistinguishable, which is the confusion this project's read paths exist to avoid. Returning the thread marked as hidden reports the moderation as a moderation, rather than as an absence a reader would read as a broken link or a lost post.

Hiding a root SHALL NOT hide the thread's replies. A reply is a separate op with its own moderation state, and a moderation naming the root decides nothing about anything else.

#### Scenario: A thread whose root is hidden is still readable

- **WHEN** a thread whose root a moderator has hidden is read with no request to include hidden content
- **THEN** the reply is not a refusal
- **AND** the root is among the items returned
- **AND** it is marked as hidden

#### Scenario: The hidden root's body is withheld rather than emptied

- **WHEN** a thread whose root is hidden is read with no request to include hidden content
- **THEN** the root item reports no body
- **AND** this is distinguishable from a root whose author revised it to an empty body, which reports a body that is empty

#### Scenario: The hidden root's body is returned when hidden content is asked for

- **WHEN** a thread whose root is hidden is read with hidden content included
- **THEN** the root item carries its current version's body
- **AND** it is still marked as hidden

#### Scenario: Hiding the root does not hide its replies

- **WHEN** a moderator hides a thread's root and the thread is read with no request to include hidden content
- **THEN** every reply that is not itself hidden is returned
- **AND** each is marked as not hidden

#### Scenario: A hidden thread is distinguishable from an absent one

- **WHEN** a thread whose root is hidden is read, and a thread the peer holds no root for is read
- **THEN** the first returns a page marked hidden and the second is a refusal
- **AND** the two replies are not the same

#### Scenario: A moderation by a peer with no authority does not hide a root

- **WHEN** an authentically signed moderation of a thread's root is published by someone who is not a moderator of that Stoa, and the thread is read
- **THEN** the root is not marked as hidden
- **AND** its body is returned

### Requirement: A hidden reply is omitted by default and returned, marked, on request

A reply hidden by a binding moderation SHALL be omitted from the items by default. Where the caller has asked for hidden content to be included, it SHALL be returned and marked as hidden.

The flag that includes hidden content SHALL be optional, and its default SHALL be to exclude. Omitting it, and supplying it with no value, SHALL both produce the excluding answer, so that no caller reaches a wider view of a thread by naming the flag without setting it.

A hidden reply is omitted rather than replaced with a placeholder, because moderation is a filter rather than a penalty and a visible placeholder is a penalty with extra steps. The exception is the root, for the reason that requirement gives. A reader who asked to see hidden content is owed knowing which items those were, which is why the mark travels with them.

**Hiding a reply SHALL NOT remove the replies beneath it.** A moderation names one op, and a reply whose own parent is hidden is not itself moderated. Such a reply SHALL still be returned in the default view, still reporting the hidden post as its parent — so a caller can render it as answering something it cannot show, rather than losing a subtree to one moderation. Removing descendants would let one hide reach ops no moderator acted on, which is a wider effect than the moderation contract grants.

#### Scenario: A hidden reply is omitted by default

- **WHEN** a moderator hides one reply in a thread and the thread is read with no request to include hidden content
- **THEN** that reply is not among the items returned
- **AND** every other reply is

#### Scenario: A hidden reply is returned and marked when asked for

- **WHEN** the same thread is read with hidden content included
- **THEN** the hidden reply is among the items returned
- **AND** it is marked as hidden
- **AND** the replies that are not hidden are marked as not hidden

#### Scenario: The flag's absence and its null both exclude

- **WHEN** a thread containing a hidden reply is read with the flag omitted, and again with the flag supplied holding no value
- **THEN** both replies are identical
- **AND** neither returns the hidden reply

#### Scenario: A reply to a hidden reply is still returned

- **WHEN** a moderator hides a reply that has a reply of its own, and the thread is read with no request to include hidden content
- **THEN** the deeper reply is among the items returned
- **AND** it still reports the hidden reply as its parent

#### Scenario: A forged hide removes nothing

- **WHEN** a moderation op naming a reply carries a moderator's key in its author field but was signed with another key, and the thread is read
- **THEN** the reply is returned
- **AND** it is marked as not hidden

### Requirement: A thread the peer does not hold is refused, and an empty thread is served

A thread read SHALL be refused when the peer holds no post under the op id named, when the op it holds under that id is not a post, and when the post it holds under that id is a reply rather than a root. The refusal SHALL be the wire contract's error shape.

**"Holds" here means holds a usable post in the named Stoa**, and the scoping is stated because two cases would otherwise look like exceptions. An op whose signature does not verify, and an op belonging to a different Stoa, are each present in the store as bytes and are each unusable to this read: the first is not established to be anyone's post, and the second is not this Stoa's. Both SHALL therefore take the not-held refusal rather than the not-a-post one, since neither is a post this read may use and the caller's remedy is the same as for an op that never arrived. **This SHALL NOT be read as licence to report a genuine, in-Stoa op as not held** — a revision, a vote, a moderation op or a metadata op in the named Stoa is a usable op of the wrong kind, which is precisely the not-a-post case.

**A not-held refusal SHALL carry no further detail about what the store holds under that op id**, and the three cases it covers — never arrived, failed verification, belongs to another Stoa — SHALL NOT be distinguishable from one another. One refusal, one message, whichever of the three produced it.

The caller's remedy is identical in all three, which is what makes merging them honest rather than lossy: there is no thread here to read, and the id may become readable if the op arrives. Splitting them would hand a caller a distinction it has no action for, while disclosing what this peer holds: that *something* arrived under an id this read will not describe, or that an op named by an id the caller guessed is filed under some other Stoa. Neither is a fact a thread read owes anyone.

**This is deliberately unlike the publish path**, where a cross-Stoa refusal does name the Stoa the parent belongs to. The difference is which side supplied the mismatch. There, the caller named the Stoa in its own request and can correct it, so naming the other Stoa lets a view fix a request it composed. Here, the caller's Stoa is not in question — it asked for a thread in the Stoa it is reading — and the id simply names something elsewhere, which is a dead end rather than a correctable mistake. The two capabilities therefore answer differently on purpose, and the earlier draft of this requirement, which copied the publish path's disclosure across, was wrong to.

A thread whose root the peer holds and for which it holds no replies SHALL be served: a page carrying the root and nothing else, reporting no further page.

**These two states SHALL NOT be reported alike**, and the reason is the failure this repository has already met: an empty listing is indistinguishable from a subject nobody has posted in, so a read that answered an unknown thread with an empty page would render a peer that has never received a thread exactly as it renders a thread whose author wrote one post. A reader shown the second when the first is true concludes a post vanished.

The refusal for a root that is not held SHALL be distinguishable from the refusal for an op held that is not a post, and from the refusal for a post held that is a reply.

**The three refusals are distinguishable because a caller acts differently on each**, and that — rather than tidiness — is why they are three:

- **Not held.** The op may still arrive. A view retries later, and says so: the post has not reached this peer yet. This is the only one of the three that a caller should wait on, which is why it must never be reported for an op the peer has.
- **Held, not a post.** The caller named the wrong kind of thing, and waiting will never fix it. A view treats this as a defect in whatever produced the identifier — a stale link, a mis-parsed address — rather than as a propagation gap.
- **Held, a post, but a reply.** The caller is one level too deep, and there is a right answer nearby: the thread that reply belongs to. This is the only one of the three a view can act on **by making another call**.

A refusal that merged the first with either of the others would be the expensive mistake, because it is the one that sends a reader waiting for something that has already arrived or that can never arrive.

**Every op kind that is not a post takes the not-a-post refusal**, and the rule is stated over kinds rather than enumerated. A vote, a moderation op, a Stoa metadata op and a revision are each an op the peer holds that is not a post, so each is refused that way; a kind added later is refused that way too, without this requirement being revisited. **A revision is named explicitly because it is the one that invites the other answer**: revisions are bound up with posts, and "this is a version of a post, not a post" is a distinction a reader can talk themselves out of. It is still an op the peer holds, so reporting it as not held would be false.

A refusal SHALL NOT be produced on the grounds that other ops may exist elsewhere. A peer routinely holds an incomplete set, and a thread read answers over what it has.

#### Scenario: An unheld root is refused

- **WHEN** a thread read names an op id no op in the log carries
- **THEN** the reply carries an error
- **AND** it carries no items

#### Scenario: A thread with no replies is served, not refused

- **WHEN** a thread read names a root post the log holds and no reply to it exists
- **THEN** the reply carries no error
- **AND** the items are exactly that root post
- **AND** no further page is reported

#### Scenario: The empty thread and the absent thread are different replies

- **WHEN** a thread with no replies is read, and a thread whose root is not held is read
- **THEN** the first carries items and no error
- **AND** the second carries an error and no items

#### Scenario: An op that is not a post is refused distinguishably

- **WHEN** a thread read names a vote's op id
- **THEN** the reply carries an error saying the op named is not a post
- **AND** that message differs from the one for an op id the log does not hold

#### Scenario: Every non-post kind takes the same refusal

- **WHEN** a thread read names in turn the op id of a vote, a moderation op, a Stoa metadata op and a revision of a post, each held by the peer
- **THEN** each reply carries the not-a-post error
- **AND** none of them carries the error for an op the peer does not hold
- **AND** the outcome does not vary by kind, so a kind this scenario does not name is not left with an answer of its own

#### Scenario: A held revision is not reported as unheld

- **WHEN** a thread read names a revision's op id, and the same read is compared against one naming an op id the log holds nothing for
- **THEN** the two messages differ
- **AND** the revision's message does not state that the op is not held, which the log disproves

#### Scenario: A reply's op id is refused distinguishably

- **WHEN** a thread read names the op id of a post that has a parent
- **THEN** the reply carries an error
- **AND** that message differs from both the not-held refusal and the not-a-post refusal

#### Scenario: A store failure is an error and never an empty thread

- **WHEN** a thread is read against a store whose reads fail
- **THEN** the reply carries an error carrying the reason the store gave
- **AND** it is not an empty page, which would report a broken store as a quiet thread

### Requirement: The items are a flat sequence in the system's order, with the root first

The items SHALL be a flat sequence. Each SHALL name its parent, which is what lets a caller reconstruct the reply structure; the sequence itself SHALL NOT be nested, and the read SHALL NOT report a depth or an indentation level.

The root post SHALL be the first item of the first page. The replies SHALL follow in the order the system's ordering rule places them, and this capability SHALL NOT define an order of its own, SHALL NOT sort by arrival, and SHALL NOT compare any value itself. A second implementation of the ordering rule could disagree with the first, and two orders that disagree produce no error anywhere — each peer stays internally consistent while rendering one thread differently from its neighbour.

**What that order guarantees today is convergence and not recency.** While the transport supplies no ordering metadata, the rule falls back to ascending op id, which is a hash and carries no temporal meaning whatever. Two peers holding the same ops therefore return the same sequence; neither can say which reply was written first. Nothing in this capability SHALL be reported to a caller as chronological, and no field SHALL be named or described in terms of recency.

A flat sequence is specified rather than a tree because whether a thread view should paginate by reply order or by reply tree is an open question that running against real traffic decides, and a flat page carrying each item's parent does not foreclose either answer. A tree would need a page boundary chosen before anyone knows where one should fall.

**A view rendering the replies nested is served by this shape and is not in tension with it.** Nesting is a depth, a depth is a function of how many parents separate a post from its root, and every item carries the parent that answers it. A reader therefore computes the indentation it wants from what it was given. Reporting a depth from here would be a second answer to a question the parent field already settles, and the two could disagree on a partial set of ops — which is why the read reports parents and not depths.

#### Scenario: The root is the first item

- **WHEN** the first page of a thread with several replies is read
- **THEN** the first item is the root post

#### Scenario: The root is not repeated on a later page

- **WHEN** a thread with more replies than fit one page is read page by page
- **THEN** the root appears on the first page only
- **AND** it occupies one of that page's item slots rather than being carried beside them

#### Scenario: Two peers holding the same ops return the same sequence

- **WHEN** two peers hold the same ops with the same recorded arrival metadata, appended in opposite sequences, and each reads the thread
- **THEN** both return the same sequence of op ids

#### Scenario: The sequence is the ordering rule's and is not re-sorted here

- **WHEN** a thread's replies are read
- **THEN** their relative order is the one the ordering rule places them in
- **AND** it is not the sequence they were appended in, where the two differ

#### Scenario: A deep chain of replies comes back flat

- **WHEN** a thread holds a reply to a reply to a reply and is read
- **THEN** all of them appear as items of one sequence
- **AND** each names its own parent
- **AND** no item carries a nested list of its own replies

### Requirement: Pages tile the thread with no gap and no repeat

A thread read SHALL accept a page index and a page size, and SHALL answer with the requested page, the index it served, and whether a further page exists in this peer's copy.

**The root is an item of the sequence and occupies a place in it**, rather than being carried outside the pagination or repeated on every page. It is the first item of the first page and it is not returned again on any later one, so the pages partition the thread exactly — a root repeated per page would make the concatenation of the pages differ from the thread, and a root carried outside them would make the first page one item shorter than every other for no reason a caller could predict. A caller reading a later page therefore receives replies only, and already holds the root from the first.

Successive pages SHALL partition the thread's items in order: concatenating every page SHALL reproduce exactly the sequence a single unbounded read would return, with no item missing and none appearing twice. Hidden replies SHALL be excluded **before** the items are cut into pages, so that a page is full of items the caller can see rather than the survivors of a slice — filtering after cutting produces short pages with gaps, and a reader paging forward silently skips items.

The page size SHALL be bounded above by a cap this peer will build, and a caller asking for more SHALL be served the cap rather than refused: the reply is serialised into one string, so an unbounded size is a caller asking this peer to build an arbitrarily large one, and a caller asking for too much is not attacking anything. A page size of zero SHALL be served as the default size rather than as an empty page, because a permanently empty thread is the confusion the previous requirement exists to prevent.

A page index past the end SHALL be served as an empty page reporting the index asked for and no further page, and SHALL NOT be refused and SHALL NOT abort. Both values arrive from a caller, so an index large enough to overflow the arithmetic that locates it SHALL still produce an empty page.

Whether a further page exists SHALL be answered about this peer's own copy. No count of the thread's total size SHALL be reported: a count of what this peer holds is not a count of what exists, and the two are indistinguishable once rendered.

#### Scenario: Pages partition the thread exactly

- **WHEN** a thread of several items is read one page at a time at a size smaller than the thread
- **THEN** concatenating the pages in index order reproduces the single-read sequence
- **AND** no item appears twice and none is missing

#### Scenario: A further page is reported only while one exists

- **WHEN** a thread whose items are an exact multiple of the page size is read page by page
- **THEN** every page but the last reports a further page
- **AND** the last full page reports none

#### Scenario: Hidden replies are excluded before the page is cut

- **WHEN** a thread whose earliest replies in order are hidden is read at a page size smaller than the number of visible items
- **THEN** the first page is full of visible items
- **AND** it is not a shorter page containing the survivors of the first items in order

#### Scenario: A page past the end is empty rather than an error

- **WHEN** a thread is read at a page index beyond its last page
- **THEN** the reply carries no error
- **AND** it carries no items and reports the index asked for and no further page

#### Scenario: An enormous page index does not abort

- **WHEN** a thread is read at the largest page index the request can express
- **THEN** the reply is an empty page rather than a failure or a page from the middle of the thread

#### Scenario: An oversized page size is served at the cap

- **WHEN** a thread read asks for a page size far above the cap
- **THEN** the reply carries no error
- **AND** it carries no more items than the cap

#### Scenario: A page size of zero is served at the default

- **WHEN** a thread read asks for a page size of zero
- **THEN** the reply carries the items the default page size would have carried
- **AND** it is not an empty page

#### Scenario: No total is reported

- **WHEN** any page of a thread is read
- **THEN** the reply reports the index served and whether a further page exists
- **AND** it reports no count of the thread's items

### Requirement: Every string a thread read returns is sanitised, and the stored op is not

Every peer-supplied string a thread read returns for display — a post's body and each of its attachment references — SHALL be sanitised before it leaves this contract, and the reply SHALL report what sanitising found for each of them.

Sanitising SHALL be a rendering and never a rewrite of the record: the op in the log SHALL still hold the author's bytes exactly, unchanged and still verifying. The two are not in tension. An op's bytes are what its signature commits to and what every peer computes its op id from, so normalising one would break agreement between peers about what a post is; what a *reader* is handed for display is a different artefact, and it is the one an attacker reaches a person through.

Post bodies are the largest and least constrained attacker-supplied strings this capability returns, which makes them the case that matters most rather than an afterthought to titles. Attachment references go through the same path, because a reference is a peer-supplied string a view renders like any other; nothing here judges whether one resolves.

The report SHALL distinguish what was removed from what was marked, so that a caller rendering the result can tell text that was altered from text that was flagged and left in place. Marking SHALL NOT correct: a character that renders as another is reported rather than replaced, because replacing it would make a deceptive string into a plausible one.

#### Scenario: A hostile body is sanitised on the way out

- **WHEN** a thread containing a post whose body carries a bidirectional override and a character that renders as another is read
- **THEN** the returned body no longer carries the override
- **AND** the reply reports that one character was removed and one was marked

#### Scenario: The stored op keeps the author's bytes

- **WHEN** the same thread is read and the post op is then read back from the log
- **THEN** the op's body is the author's original bytes, unchanged
- **AND** the op still verifies

#### Scenario: Attachment references are sanitised too

- **WHEN** a thread containing a post whose attachment reference carries a bidirectional override is read
- **THEN** that reference is returned with the override removed
- **AND** the removal is reported

#### Scenario: A body needing no sanitising reports nothing removed or marked

- **WHEN** a thread containing a post of ordinary text is read
- **THEN** its body is returned unchanged
- **AND** the reply reports nothing removed and nothing marked

#### Scenario: Marking does not correct

- **WHEN** a body containing a character that renders as another is returned
- **THEN** that character is still present in the returned text
- **AND** it is reported as marked

### Requirement: A thread read answers in the wire contract's shapes and never aborts the process

A thread read SHALL take a JSON object and answer with a JSON object. Every refusal SHALL be the single error shape carrying a message, and a reply SHALL NOT carry both an error and items.

It SHALL NOT abort the process for any request and for any contents the log may hold — including ops that do not verify, posts naming absent parents, posts naming themselves as their own parent, parent references forming a cycle of any length, ops of every kind naming one post, and fields of every type in the request. Everything in the log arrived from a peer and is attacker-controlled, and an abort here takes the module down rather than failing the call, turning one malformed op into a denial of service against the peer that received it.

A field the reply would have no meaning for SHALL be omitted rather than sent holding no value, so that a reply is never partly a success.

#### Scenario: A request that is not an object is refused

- **WHEN** a thread read is called with a JSON array
- **THEN** the reply carries an error saying the request is not an object
- **AND** the message differs from the one for a request that is not valid JSON

#### Scenario: A missing field is refused distinguishably from a wrong-typed one

- **WHEN** a thread read is called omitting the thread it should read, and again with that field holding a number
- **THEN** both replies carry an error
- **AND** the second says the field is the wrong type rather than that it is missing

#### Scenario: An adversarial log is read without aborting

- **WHEN** a thread is read against a log holding forged posts, posts naming absent parents, a post naming itself as its parent, parent references forming a cycle, and ops of every other kind naming the root
- **THEN** the read returns a reply rather than aborting

#### Scenario: Hostile request shapes are answered rather than aborting

- **WHEN** thread reads are called with fields of every type, with absent fields, with maximal field lengths and with adversarially chosen text
- **THEN** each returns a JSON object rather than aborting the process

#### Scenario: A refusal carries no items

- **WHEN** a thread read is refused for any reason
- **THEN** the reply carries an error and no items
