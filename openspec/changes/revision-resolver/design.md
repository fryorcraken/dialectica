# Design: the revision resolver, and why revisions do not chain

## Decisions

### Decision: Every revision names the original post; a revision of a revision is not a version of it

**This was the change's open design question**, and PLAN.md §5.7 does not settle
it outright. It says "an edit is a new version of *that post*" — one named
subject rather than a chain — which points at the flat reading but does not
exclude the other, since a chain's head is also, loosely, a version of the post.
Both readings are implementable and they give different answers, so the choice
had to be made and recorded rather than absorbed.

**Chosen: flat.** Every `Revise` names the original post. An op naming a
`Revise` is not resolved as a version of the post that `Revise` revised, and
nothing walks a chain. Two arguments decided it, and the first is the one that
makes the choice not merely preferable but forced.

**1. A chain is not resolvable over a partial set, and §3.3 makes a partial set
the normal case.**

This is the whole argument in one sentence: under a chain, a peer's answer
depends on which *intermediate* ops it happened to receive, and the missing ones
are not observable as missing.

Concretely. An author publishes `v1` (the post), then `v2` naming `v1`, then
`v3` naming `v2`. A peer holding all three resolves to `v3`. A peer holding `v1`
and `v3` but not `v2` — an entirely ordinary state, since §3.3 says two peers
"routinely hold different sets of ops" and nothing delivers them in order — has
`v3` in hand and cannot use it: `v3` names `v2`, which it does not hold, so `v3`
is unreachable from `v1`. That peer resolves to `v1`, holding the current version
in its own log the whole time.

Under the flat rule the same peer resolves to `v3` directly, because `v3` names
`v1`. Every version stands alone, so the answer is a function of *which versions
you hold* rather than of *which paths through them are intact*. That is the
difference between degrading and breaking.

The same fact stated from the security side: under a chain, an adversary who can
suppress delivery of one small intermediate op can pin every peer that misses it
to an arbitrarily old version of a post, without forging anything and without
any peer being able to detect that it has been pinned. The flat rule gives that
adversary nothing — suppressing a version denies exactly that version.

**2. A chain forks, and the ordering rule has no answer for a fork.**

`cmp_ops` orders *ops*. It does not order *branches*, and it has no notion of
one. An author who publishes two revisions of `v2` — trivially done by two
devices, or by one device retrying after a timeout — creates two heads. Both
name `v2`; both are perfectly well-ordered against one another by `cmp_ops`; and
whichever wins, the other subtree is silently discarded. Resolving that properly
needs a merge rule over branch structure, which is precisely the machinery §5.7
exists to avoid: "no CRDT, no merge function, no last-writer-wins ambiguity".

Under the flat rule there are no branches to merge. Every version is a sibling,
`cmp_ops` orders siblings, and that is the entire algorithm.

**What the flat rule gives up, stated honestly.** Two things, and neither is
load-bearing:

- **Edit provenance.** A chain records that `v3` was derived from `v2`
  specifically. The flat model records only that both are versions of the post.
  §5.7 asks for "the UI can show that a post was edited" and for a moderator to
  act on "a version they can name" — both are satisfied by having the versions,
  and neither needs the derivation edges. If a future feature genuinely wants
  "what changed between consecutive versions", it wants a diff between two
  versions a reader already holds, not a parent pointer.
- **Nothing else.** In particular the flat rule does not lose ordering: the
  ordering rule was never going to use chain depth, because depth is not
  transmitted and an author could assert any value for it.

**What would reverse this.** A requirement that an edit be valid only against a
specific prior version — an optimistic-concurrency rule, "reject this edit if
the post has changed since I loaded it". That genuinely needs a parent pointer.
It is not in the plan, and adding it later is additive: a `Revise` would gain a
field, and the flat resolution would remain correct for every op that does not
carry one.

**The encoding already permits a `Revise` to name a `Revise`**, because
`OpKind::Revise`'s target is an `OpId` with no kind constraint, and the op format
is not this change's to alter. So the rule is enforced where it is decided — on
read, at the resolver — and pinned by
`a_revision_of_a_revision_is_not_a_version_of_the_post`, whose fixture makes the
two readings give *opposite* answers.

### Decision: Verify, then compare authors — and never the reverse

The two checks look like an unordered pair and are not. Reversing them is a
complete bypass of the property this change exists to establish.

An op's `author` field is a **claim** carried in attacker-supplied bytes.
`SignedOp::verify` is what converts it to a fact, by re-deriving the address from
the key that actually signed (`identity::verify_authored_op`) rather than
trusting the field. Compare authors first and the comparison is between one
attacker-supplied string and another: an attacker writes the victim's public key
into `author`, signs with their own key, and the authorship check passes on a
forgery.

This is not hypothetical. `a_forged_revision` builds exactly that op, and
`a_forged_revision_is_dropped` asserts both halves that make it meaningful: that
the op does not verify, and that its author field *does* match the victim's. A
resolver checking authorship alone accepts it.

The two checks are therefore one expression, in one function
(`is_valid_revision`), in a fixed order, rather than two conditions a caller
composes.

### Decision: The resolver defines no order, and takes the first entry

`OpLog::iter_target` already returns entries in `cmp_ops` order. The resolver is
a `find` over that sequence — no `sort`, no `max_by`, no reference to `lamport()`
anywhere in the module.

This is a correctness property, not an economy. A second implementation of §5.7's
ordering rule could disagree with the first, and two orders that disagree produce
**no error**: each peer stays internally consistent and renders the post
differently from its neighbour. `arrival.rs` exists to make that unreachable, and
a resolver comparing timestamps itself would reintroduce it one layer up.

### Decision: "First" is the current one, and that is NOT "the most recent"

Worth recording because the project has now made this error twice, in two
resolvers, from the same source.

`iter_target`'s documentation formerly said resolvers want "the ops about this
subject, most recent first". That is true **only on the transport-ordered
branch**. `cmp_ops` leads with the highest Lamport timestamp only where the
transport supplied one — and nothing supplies one today, so every arrival is
`Arrival::unordered()` and the comparison falls back to **ascending op id**. An
op id is a SHA-256 of the op's own bytes. It carries no recency whatever.

So the current version, under the order production actually runs, is a
**convergent arbitrary choice rather than a temporal one**.

That is not a disappointment to be apologised for — it is the property the system
needs, stated exactly:

> Two peers holding the same revisions agree on which is current, even though
> neither can say which was written last.

Convergence is what makes a forum render consistently; temporal accuracy is a
separate good that becomes available when upstream supplies its metadata, at
which point the identical `find` yields it with no code change. Writing the
resolver against *position in the ordering rule* rather than against *recency* is
what makes it correct under both regimes.

The consequence most likely to surprise a later reader, and therefore pinned by a
test pair:

- **`under_a_transport_order_the_answer_only_moves_forward`** — monotonic. A
  late-arriving older version does not displace the current one.
- **`under_the_degraded_order_a_late_arrival_can_change_the_answer`** — *not*
  monotonic. A version arriving later with a lower op id becomes current,
  because op id knows nothing of arrival sequence.

Both are correct. Monotonicity is a property of the *regime*, not of this
resolver, and a reader who assumed it held everywhere would be relying on a
guarantee the degraded order does not make. What holds in both regimes is that
the answer is a pure function of the ops held.

### Decision: An op that is not a post resolves to absence, not to a distinct error

`current_version` returns `Option`, and `None` covers both "the log does not hold
this op" and "it holds it, but it is a vote / a moderation / a revision".

Merging them is deliberate. The alternative — a three-state result distinguishing
absent from wrong-kind — widens the return type for a distinction no caller can
act on differently: there is a current version to render, or there is nothing to
render, and a caller holding a vote's op id and asking for its body has made a
category mistake for which there is no rendering either way.

The check is not optional. Without it, a `Revise` naming a vote would be resolved
as that vote's "current version", which is content substitution into a reader's
view of an op that has no content. `an_op_that_is_not_a_post_has_no_current_version`
constructs exactly that and asserts the revision really does name the target, so
the kind check is what rejects it rather than an empty `iter_target`.

**A reply is a post.** §4.1 puts `threadId` and `parentPostId` in the payload and
`op.rs` records that "there is no `Reply` kind", so the check is
`matches!(.., OpKind::Post { .. })` and not anything narrower. A guard written as
"a top-level post" would make every reply unrevisable —
`a_reply_is_a_post_and_has_versions` pins the other side.

### Decision: `CurrentVersion` carries both entries, and knows how to read a body

Returning only the current entry was the smaller API and is worse. §5.7's "history
is kept" exists so "the UI can show that a post was edited, and a moderator acting
on a post is acting on a version they can name" — showing *that* it was edited
needs the original's identity, and naming the version needs the version's. Neither
is derivable from the other, so a caller wanting both would re-read the log.

`body()` and `attachments()` are on the type rather than left to callers because
extracting a body means matching on the entry's kind, and `current` is a `Post` or
a `Revise` depending on whether the post was ever edited. That is exactly the
branch a caller should not have to get right: written at four call sites, one of
them eventually reads "match `Revise`, else empty" and silently renders every
unedited post blank. CLAUDE.md's "a guard is a job" applied to a projection.

**Neither falls back to `original` when the current version's field is empty**,
and this is the decision in the pair most likely to be "fixed" into a bug by a
later reader. `if current.is_empty() { original }` reads as defensiveness
against a missing field, and it is data resurrection: a version *replaces* a
post rather than patching one, so emptiness is a value the author chose. For
`body` that silently restores deleted text; for `attachments` — Logos Storage
CIDs, per §4.6 — it means a deleted image is still fetched and rendered.

This is recorded rather than left to the code because it survived the whole
suite. A blind spec-test review mutated `attachments()` into exactly that
fallback and **all 37 tests passed**, while the identical mutation on `body()`
died immediately — the reasoning had been written for one field and never
carried to the other. Both now have a paired test and a spec requirement.

The unreachable arm of those matches **answers rather than panics**. It cannot be
reached — `current` is either `original`, established as a `Post`, or an entry the
`find` matched as a `Revise` — but this module runs on attacker-supplied content,
and PHASE0-FINDINGS §3 measured that a panic aborts the module process. An empty
body is a rendering; a crash is a denial of service.

## What this change deliberately does not build

- **The moderation resolver.** §5.7 is explicit that a moderator's hide and an
  author's edit "are about different things and do not contend: an edit does not
  clear a hide, and a hide does not invalidate an edit." Two questions, two
  resolvers, built in parallel against the same log.
  `a_moderation_or_a_vote_on_the_post_is_not_a_version_of_it` pins the
  non-interference from this side: a moderation given the top of the order is not
  a version and does not displace one.
- **The materialised view and its query indexes.** §3.3 distinguishes the log
  (authority) from the view (a cache rebuildable by replay). This is the function
  such a view folds.
- **Any op-format change.** `an_op_carries_no_ordering_fields` passes untouched,
  and no field was added to `Revise` — see the reversal condition in the first
  decision.
- **Edit provenance or a diff between versions.** Named in the first decision as
  what the flat rule gives up, and as what a future feature would actually want
  instead.

## What the green gate structurally cannot see

Recorded because a passing suite here proves less than it appears to.

- **That the ordered branch is ever exercised in anger.** Production constructs
  `Arrival::unordered()` for everything, so the degraded order is the only order
  a running peer uses. Every test here that asserts a *temporal* answer uses a
  fixture the transport has never actually produced. Inherited from
  `op-ordering`, not fixable here.
- **That the authorship rule is enforced anywhere but on read.** A peer running
  modified code can publish revisions of anyone's post, and they will sit in
  every peer's log. What this change guarantees is that no *conforming reader*
  renders them. §6.1's ceiling applies: this changes what peers render, and
  cannot prevent publication.
- **That a real revision has ever reached this code.** No op has arrived from a
  network. Every fixture signs ops in-process, so the suite exercises the
  resolver against ops that are well-formed by construction — the decoder's
  hostile-input tests are what cover the other side, and they stop at `op.rs`.
- **Whether resolving one post at a time is viable.** Each call allocates a
  `Vec<&Entry>` of everything naming the post and sorts it. Rendering a thread of
  N posts is N such calls. At test sizes this is free; at the size of a Stoa with
  months of history it may not be, and the suite would not notice. This is the
  `Vec<&Entry>` question `op-log`'s design named, reaching its first real
  consumer.
- **That two peers actually converge.**
  `two_peers_holding_the_same_ops_resolve_to_the_same_version` builds two logs in
  one process from one set of ops. It proves the answer does not depend on
  insertion sequence; it cannot prove two *machines* agree, which needs the
  transport that does not yet supply the metadata.
