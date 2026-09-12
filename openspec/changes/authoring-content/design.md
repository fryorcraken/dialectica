# Design: the publish path

## Context

`content-authoring` contracts three publish operations — post, reply, vote — as
the first write path across the module boundary. Everything they compose already
exists: `op.rs` owns the op shapes, the canonical bytes, the signing and the op
id; `log/` owns the store and its newly-stored-or-already-present answer;
`keystore.rs` owns the per-Stoa key; `wire.rs` owns JSON in, JSON out and the one
failure shape.

So this change is mostly **plumbing under a validation gate**, and the design
questions are about where the gate lives, what it is allowed to know, and how the
three operations avoid becoming three copies of one another.

## Where the code goes

A new `authoring.rs` in `dialectica-core`, plus three handlers in `wire.rs` and
three methods on the module trait.

The split between them is the one `feed.rs`/`wire.rs` already established, and it
is not cosmetic: `wire.rs` owns **parsing** — JSON in, the error shape out — and
`authoring.rs` owns **deciding**, over already-typed values. That is what lets a
test assert on "a reply to a parent in another Stoa is refused" without also
asserting on how a Stoa address is spelled in JSON.

`authoring.rs` is handed a `&mut L: OpLog` and a `&SecretKey`. It does not open a
store, read the environment, or find a key. Two reasons, and the second is the
one that would otherwise bite: `dialectica-core` structurally cannot reach the
host's persistence path (`lib.rs`'s whole reason for existing), and a publish that
went looking for a key would be doing discovery at a moment its caller does not
control — which is also how "a publish creates no key material as a side effect"
stops being a property anyone has to remember.

## Decisions

### The request is parsed into a typed intent before anything is signed

Each handler parses its request into one of three small structs (`PostRequest`,
`ReplyRequest`, `VoteRequest`) and only then calls into `authoring.rs`. The
alternative — validate-as-you-go, building the `Op` incrementally — was rejected
because of the requirement that a refused publish appends nothing and does not
invoke delivery. Under incremental construction that is a property of the order
the statements happen to be in; under parse-then-act it is structural, because
there is nothing to append until every field has been read.

### The three operations share one `publish` and differ only in what they build

`publish(log, key, op) -> Published` signs, appends, and reports
`Published { id, was_new }`. The per-kind work — deriving a thread, checking a
parent's Stoa, refusing an unknown direction — happens *before* it, in
`post`/`reply`/`vote`, each of which returns an `Op` or a `Refusal`.

This is the "one function, one job" rule with a specific payoff: the ordering
requirement (sign, append, then hand to delivery, and never defer the reply on
delivery) is asserted once against one function rather than three times against
three near-copies.

### Delivery is a sink the caller supplies, and its outcome is discarded

`publish` does not know what delivery is. The **handler** calls it, after
`publish` has returned, and ignores its answer:

```rust
let published = authoring::post(log, key, req)?;   // appended by here
let _ = deliver(&published.id);                     // outcome discarded
reply_json(&published)
```

Three requirements land on that shape at once. "The append completes before
delivery is invoked" is the statement order. "A declined handoff leaves the op
published" is the `let _`. "A publish returns while delivery is still
outstanding" is that `deliver` is a `FnOnce(&OpId)` returning `()` — there is no
outcome to wait for, so a call that waited on one cannot be written.

The alternative — `publish` taking the delivery sink itself — was rejected
because it would put a network-shaped parameter on the function whose tests are
all about the log, and because a sink inside `publish` is a sink whose *return
value* something inside `publish` could later be tempted to read.

**`deliver` is also not invoked on a refusal, and that is structural rather than
guarded**: it is called only on the success arm of the `Result`, so there is no
refusal path that reaches it.

### `Refusal` is a typed enum, not a `String`

Two requirements demand that specific refusals be told apart from one another:
"no such op is held" from "the op held is not a post", and a wrong-typed field
from a missing one. A `String` message can carry that distinction and cannot
*hold* it — a test asserting on substrings pins the wording rather than the
category, and the next reword breaks the test without breaking the behaviour.

So `authoring.rs` returns a `Refusal` enum whose `Display` is the wire message.
Tests assert on the variant; the message is asserted separately, once, where it
is pinned as a shape.

### A reply's thread is derived as `parent.thread.unwrap_or(parent_id)`

The spec requires the thread to be derived from the parent and says the
derivation mechanism belongs to the read-side capability, which does not exist.
It is derivable today, from what `op-format` already contracts:

- a thread-opening post carries `thread: None`, because its own id is the hash of
  the bytes being signed and so is not knowable at signing time;
- a reply carries `thread: Some(t)`.

So the thread a parent belongs to is its own `thread` field when it has one, and
its own op id when it does not. One expression, no new state, and it satisfies
both of the spec's scenarios — a reply to a root lands in the root's thread, and a
reply to a reply lands in the thread its parent names.

**What this trusts, and why trusting it is correct here.** The parent's `thread`
field is inside the parent's signed bytes, so it is the parent's author's claim
and not a relay's. It may still be a *wrong* claim — an inbound op can name a
thread its own parent does not belong to, and the log stores it, because the log
decides nothing. This derivation propagates that claim rather than auditing it,
and that is the right division: auditing a thread graph is a read-side question
about which ops render where, and answering it here would be a publish-path check
a reader might then rely on — exactly the direction the spec's
"publish-path checks are a third thing" requirement forbids.

`op.rs`'s doc comment on `thread` says "the store fills the thread in as its own
id on ingest". **That is not what the store does, and it cannot be**: the op is
signed, so a store rewriting a field would invalidate the signature. The
comment is corrected in this change, because a reader who believed it would
expect `thread` to be non-`None` on every stored root and derive the thread
wrongly.

### Publishing does not verify the Stoa's genesis record

`list_threads` takes a genesis record because it needs a moderator set. No
requirement here needs one: a Stoa address is an opaque 32-byte key to both the
log and the signing derivation, and none of the three operations asks a question
whose answer depends on who founded the Stoa. Widening the request to carry a
genesis record would be widening the deliverable for nothing.

The consequence is honest and worth stating: **this change does not enforce a
Stoa's posting policy.** `Policy` has one variant (`Open`), so there is nothing
to enforce yet, and a policy check is a read-time authority question in the same
family as moderation — it belongs where `moderation.rs` already is.

### The reply carries `wasNew`, and it is passed through rather than recomputed

`op-log`'s `Appended` already answers newly-stored-or-already-present, and the
spec requires the publish to pass that answer on rather than discarding it. So
`Published` carries the `Appended` value straight through, and the wire shape is
`{"opId":"…","wasNew":bool}`.

The alternative that looks equivalent — `get` before `append` — was rejected: it
is two store round trips where one suffices, and it computes an answer the store
is about to compute anyway, so the two can disagree.

### The author field is refused, not ignored

The spec requires a request naming an author to be **refused**. Ignoring an
unknown field is otherwise this project's convention (`list_threads` ignores an
`order` field, deliberately). The difference is what the caller believes: a
caller passing `order` believes it is selecting between two orderings that exist;
a caller passing `author` believes it is choosing who signs, and it is not. The
refusal is the only way to tell them.

Implemented as one reusable guard over a list of field names rather than three
copies, and the list is the union across all three operations — a `thread` field
is refused on a post request too, because a caller who sent one has the same
wrong model whichever operation they sent it to.

**NO SPEC:** the spec requires a `thread` field to be refused on a *reply*; it
says nothing about one on a post or a vote. Refusing everywhere is chosen, and
marked in the test.

## What is deliberately not here

- **No `createdAt`, no nonce.** The proposal argues this at length. The
  consequence — identical content publishes once — is implemented as the spec
  contracts it and is visible to the caller through `wasNew`.
- **No score, count or tally on a vote reply.** The reply shape is the same
  `{"opId","wasNew"}` as the others. Nothing in this change reads a `Vote` op.
- **No revision, no moderation, no attachments.** Out of scope per the proposal.
  `attachments` is therefore always the empty list, which `op-format` contracts
  as a value rather than an absence.
