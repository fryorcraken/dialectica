# The revision resolver: which version of a post is current

## Why

PLAN.md §5.7 is the whole conflict rule for post content: "A post is never edited
in place. An edit is a new version of that post, published and signed by the same
author." It states three properties — authorship decides validity, Lamport order
decides currency, history is kept — and nothing implements any of them.

Every piece of the machinery now exists and none of it answers the question. The
op log holds `Revise` ops and orders them (see the `op-log` spec); `op.rs` can
say whether a `Revise` is authentic; `cmp_ops` can say which of two arrivals is
more recent. What is missing is the one thing that needs all three at once:
**given a post, which version of it does a reader render?**

That gap is not merely unfinished work, it is a **security hole with a green
suite**. `op.rs` records the shape of it in as many words — its test
`a_revision_by_a_different_author_is_authentic_and_still_not_valid` asserts that
a stranger's revision of someone else's post *verifies*, because authenticity is
not authority and the check needs the target op, which `op.rs` does not have. So
today a `Revise` from any peer on the network is indistinguishable, to every
piece of code that exists, from one by the post's author. Until something holds
the log and the target op together and compares the two authors, "only the author
may revise" is a sentence in a document rather than a property of the system.

This change is that something.

## What Changes

- A **`revision` module** with one entry point: given a log and a post's op id,
  return the version of that post a reader should render, together with the
  original it derives from.
- **The authorship check**, which is this change's reason to exist. A `Revise`
  whose author is not the original post's author is dropped, whatever its
  signature, whatever its Lamport timestamp, and whatever else it says.
- **Verification on read** (§3.3), because the log deliberately stores junk. A
  `Revise` that does not verify is dropped before its author is even compared —
  an unverified author field is a claim, not a fact, and comparing claims to
  facts is how a forgery passes an authorship check.
- **Currency by the log's own order.** The resolver performs no comparison of its
  own: `iter_target` already returns entries in `cmp_ops` order, so the current
  version is a `find` over that sequence rather than a second implementation of
  §5.7's ordering rule.

  **And taking the first entry is not taking the most recent.** `cmp_ops` leads
  with the highest Lamport timestamp only on the transport-ordered branch, which
  production never reaches — no Lamport value arrives, so every op falls to
  ascending op id, which is a hash and carries no recency. The property this
  change actually delivers today is therefore **convergence, not recency**: two
  peers holding the same revisions agree on which is current, though neither can
  say which was written last. That is what a forum needs to render consistently,
  and the same `find` yields the temporal answer unchanged when upstream supplies
  the metadata.
- **A defined answer over a partial set.** A peer holding fewer revisions
  resolves over the ones it has, and a peer holding no valid revision resolves to
  the original. Neither is an error; §3.3 makes different op sets the normal case.
- **History is kept**, which falls out of reading rather than being arranged: the
  resolver is a read over the log and removes nothing, so every superseded version
  stays exactly where it was and remains nameable by its own op id.

Not changed: the op format (`an_op_carries_no_ordering_fields` still passes
untouched), `arrival.rs`, `log.rs`, or any transport wiring.

## Capabilities

**New Capabilities**

- `post-revision` — which version of a post is current, who may publish one, and
  what a reader resolves to over a set of ops that is legitimately incomplete.

**Modified Capabilities**

None. `op-log`, `op-ordering` and `stoa-genesis` are untouched — this change is a
reader built on top of all three.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/revision.rs`.
- `dialectica-core/src/lib.rs` gains one `pub mod` line. The moderation resolver
  is adding a line to the same file in parallel; that is a one-line rebase and is
  expected.
- **No new dependency.**
- No wire-contract change and no change to the module's JSON API. What a view sees
  is the later materialised-view change; this is the function that change folds.
- Does not build the moderation resolver. §5.7 is explicit that a moderator's hide
  and an author's edit "are about different things and do not contend", so they
  are two resolvers rather than one, and the other is being built in parallel
  against the same log.
