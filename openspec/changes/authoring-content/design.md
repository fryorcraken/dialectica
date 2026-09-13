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

Each handler reads every field it needs out of the request — `stoa` and `body`
for a post, plus `parent` for a reply, `target` and `direction` for a vote — and
only then calls into `authoring.rs`. The alternative — validate-as-you-go,
building the `Op` incrementally — was rejected because of the requirement that a
refused publish appends nothing and does not invoke delivery. Under incremental
construction that is a property of the order the statements happen to be in;
under parse-then-act it is a property of where the `authoring` call sits.

**What was planned here and not built**, because a reader will otherwise look for
it: three named request structs (`PostRequest`, `ReplyRequest`, `VoteRequest`),
so that holding a value of the type would be *evidence* every field had parsed.
What shipped is the same statement sequence written out in each of the three
handlers. The requirement still holds, but it holds three times over rather than
by construction.

### The reshape this leaves on the table, and why it is a precondition of a fourth operation

The **parse prologue** is copied three times (`parsed_object`,
`reject_forbidden_fields`, `required_stoa`, then the per-kind fields). What is
common is a **sequence**, which is what a shape can hold and a copy cannot — so
requirements this document calls structural are in fact each true of three copies
independently.

The evidence that this costs something is in the change's own history rather than
in principle: `tasks.md` §7's mutation 8 had to be re-run per handler, and one of
the three turned out to be caught only by a sink-call *count* — which would miss a
hoist that replaced the later call instead of adding to it. A per-handler mutation
is only writable because there are three independent places to write it.

**The tail's `deliver` call is no longer one of the three copies**, and it was not
this reshape that closed it. Settling the panicking-sink question (below) put the
handoff in one function, `delivered_and_published`, called from all three success
arms — so `deliver` is now named once and a handler has no op id to hoist. That
was motivated by a correctness defect rather than by this entry, which is why the
reshape below is described as the prologue's and not the tail's. What remains
copied three times in the tail is only the `match`'s shape itself: the `Err` arm's
`error_json(&refusal.to_string())`, which carries no sequence and no guard.

The shape still wanted: a request type whose single constructor runs the prologue,
so a fourth operation inherits `reject_forbidden_fields` by taking the type rather
than by its author remembering to copy three lines. "Is the guard called
everywhere?" then becomes a question the type system answers instead of one a test
loop answers — and `tasks.md` §9 already notes the guard being forgotten in one
handler is invisible without checking all three. CLAUDE.md's rule is that the
fourth slightly-different copy of a guard is the signal to reshape rather than to
add a fourth test; this change added the looping test, which is the right test and
is not a reshape.

**A second reason to want it, found by the security review.** The adapter runs a
full Argon2id keystore unlock (64 MiB, RFC 9106 option 2) and opens SQLite
*before* calling the handler — so before any field is validated. Every refusal in
this change therefore pays for a keystore decrypt first: a request carrying an
`author` field, which the spec requires be refused, still costs the derivation.
That inverts the "parse the whole request first, then act" principle this section
states.

Hoisting `reject_forbidden_fields` alone would not fix it — the missing-`body`,
wrong-typed-`body`, non-hex-`parent` and unrecognised-`direction` cases would still
pay. What fixes it is a validate-only entry point, which is the request type above:
parse and refuse without a key, then derive a key only for a request that survived.
So the reshape and the cost-amplification fix are one change.

**Not done here** because it changes no behaviour but does reshape three call sites
plus the adapter — and the adapter is the one file no gate in this repo compiles, so
restructuring it belongs in a commit that can be reviewed for that alone rather than
riding along with a security fix whose tests do run. It is a precondition of the
next operation rather than a cleanup after it: `OpKind` already has a `Moderate`
variant, so `publish_moderation` is the copy that would otherwise make this the
fourth.

### The three operations share one `publish` and differ only in what they build

`publish(log, key, op) -> Published` signs, appends, and reports
`Published { id, appended }`, where `appended` is `op-log`'s own `Appended` value
rather than a `bool` — see "passed through rather than recomputed" below, which
is the reason it is not flattened here. The per-kind work — deriving a thread,
checking a parent's Stoa, refusing an unknown direction — happens *before* it, in
`post`/`reply`/`vote`, each of which returns an `Op` or a `Refusal`.

This is the "one function, one job" rule with a specific payoff: the ordering
requirement (sign, append, then hand to delivery, and never defer the reply on
delivery) is asserted once against one function rather than three times against
three near-copies.

### Delivery is a sink the caller supplies, and its outcome is discarded

`publish` does not know what delivery is. The **handler** calls it, after
`publish` has returned, and ignores its answer:

```rust
match authoring::post(log, key, stoa, body) {   // appended by the Ok arm
    Ok(published) => delivered_and_published(&published, deliver),
    Err(refusal) => error_json(&refusal.to_string()),
}
```

`delivered_and_published` is the handoff and the reply in one place — it calls
`deliver(&published.id)`, whose `()` return there is nothing to discard, and then
`published_json`. It holds the panic guard described two entries below.

Three requirements land on that shape at once. "The append completes before
delivery is invoked" is that `authoring::post` has already returned before the `Ok`
arm is entered, and `delivered_and_published` calls the sink after that. "Delivery
is not invoked on a refusal" is that the sink is reachable only through the `Ok`
arm, so no refusal path can reach it. "A publish returns while delivery is still
outstanding" is the sink's `()` return type — there is no outcome to wait for, so
a call that waited on one cannot be written.

**The sink is `&mut dyn FnMut(&OpId)`, and the dead end that made it so.**
`impl FnOnce(&OpId)` was written first, because at-most-once is the honest bound.
It cannot coexist with pinning all three handlers as one function-pointer type: a
generic monomorphises per call site, so the three become three types with no
shared pointer, and coercion fails on a higher-ranked lifetime. That pin is
`the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`'s, and it
is worth its cost because the adapter lives behind `cfg(logos_scaffold)`, which no
`cargo test` sets — so a drifted signature would otherwise surface only in the
builder's build.

Note precisely what does *not* force it, because a first draft of this reasoning
got it wrong: the adapter itself is generic over the handler and would accept a
generic sink. Recovering `FnOnce` therefore costs one test's `Handler` type, not a
redesign.

The alternative — `publish` taking the delivery sink itself — was rejected
because it would put a network-shaped parameter on the function whose tests are
all about the log, and because a sink inside `publish` is a sink whose *return
value* something inside `publish` could later be tempted to read.

**`deliver` is also not invoked on a refusal, and that is structural rather than
guarded**: it is called only on the success arm of the `Result`, so there is no
refusal path that reaches it.

### A panicking delivery sink is caught at the handoff, and the publish still reports success

Two reviewers found the same live contradiction (`findings/design-review.md` F6,
`findings/spec-test.md` entry 1): `deliver` was called inside `guarded`, so a sink
that panics produced `{"error":"panic in publish_post: …"}` with no `opId` for an
op that **is** in the log. Measured at the time: 530 passed / 1 failed when the
reply was asserted. That is precisely what the requirement forbids — "a publish
SHALL NOT be reported as having failed on the strength of a delivery outcome" —
and the scenario's condition is "delivery **refuses or errors** on the handoff",
of which a panic is the most violent form.

**The decision, taken by the owner: catch the panic at the handoff and report the
publish as successful.** The op is in the log and the requirement says so;
delivery belongs to the transport. Implemented as `wire::delivered_and_published`,
wrapping only the sink call in its own `catch_unwind`, called from all three
handlers.

**The alternative of moving `deliver` outside `guarded` is rejected, and the
measurement is why.** PHASE0-FINDINGS §3 measured what an unguarded panic costs:
the module process aborts (`failed to initiate panic, error 5`, SIGABRT), the
caller waits out a 20-second timeout, and every later call reports
`MODULE_NOT_LOADED`. A dead module is a worse answer than an unreported delivery
failure, so the outer guard stays and the handoff gets its own — two nested
guards being the cheap way to keep a panic contained *and* the reply truthful.

**The other alternative — contracting the error reply as an infrastructure fault —
was ruled out by the delivery contract, which inverted the premise the question
rested on.** `delivery_module.lidl` carries `channelMessageSent`,
`channelMessageError` and `messagePropagated`: the outcome arrives
**asynchronously, after the publish call has returned**. A return value could not
carry it even if the API wanted it to. So the synchronous reply was never the
place to learn about delivery, and was a *worse* signal rather than a missing one
— a sink that accepts an op tells you the transport took it, which is
`channelMessageSent` and says nothing about whether any peer received it.
`messagePropagated` is the fact a user cares about. Publishing and delivering are
two events at two times, and this keeps the API from pretending they are one.

**The cost, and where it lands.** A caught panic reaches stderr and nothing else;
there is no field in the reply for it, by the argument above. Making an op that
reaches `channelMessageError`, or that never reaches `messagePropagated` within
some bound, visible is therefore **`op-transport`'s obligation** — recorded in
`docs/PLAN.md` §9.2. Without that, this decision converts a loud failure into a
silent one, and `docs/UI-BRIEF.md` will need the rendering obligation once that
capability specifies the bound.

Pinned by `a_panicking_delivery_sink_still_reports_the_op_as_published_on_all_three_handlers`,
which fails before the fix with exactly the error shape above. Its sibling
`a_publish_whose_delivery_panics_leaves_the_op_in_the_log` is kept rather than
replaced: one reads the reply and the other reads the log, and an implementation
that reported success while rolling the op back would satisfy the first alone.

### `Refusal` is a typed enum, not a `String`

Two requirements demand that specific refusals be told apart from one another:
"no such op is held" from "the op held is not a post", and a wrong-typed field
from a missing one. A `String` message can carry that distinction and cannot
*hold* it — a test asserting on substrings pins the wording rather than the
category, and the next reword breaks the test without breaking the behaviour.

So `authoring.rs` returns a `Refusal` enum whose `Display` is the wire message.
Tests assert on the variant; the message is asserted separately, once, where it
is pinned as a shape.

**What the distinguishability discloses, and why that is accepted here.**
`WrongStoa` carries the Stoa the op *actually* belongs to, so the three refusals
together answer, for any op id a caller can name: do you hold it, is it a post, and
which Stoa is it in. Today the caller is the local view, which can read the store
directly, so this crosses no trust boundary — and the distinction is a spec
requirement, since a propagation gap and a category mistake need opposite responses.

Recorded because the trigger is specific: **the moment a publish handler is reachable
by anything less privileged than the local view** — a remote RPC, a multi-user host —
`actual` becomes a cross-Stoa read for a caller that could not otherwise perform one.
The project already treats that as a real category rather than a hypothetical;
`log/sqlite.rs` refuses a prefix-matching restricted read on the ground that it is
"a cross-Stoa leak in a censorship-resistant forum". If that day comes, `actual` is
the field to drop first, and dropping it costs only the wording, not the
distinguishability.

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

**The non-post arm answers rather than panicking.** `thread_of` matches on an
`OpKind`, so it needs an arm for kinds that are not posts. It returns the op's own
id — treating the op as its own thread root, which is the same answer a root post
gets, so no caller sees a shape it has not already handled. The arm is unreachable
today: `reply` refuses a non-post parent before `thread_of` is called, and
`thread_of` has exactly one caller.

Three alternatives, and why not: a `panic!`/`unreachable!` is the one to avoid
outright, because a panic aborts the module process rather than failing one call.
An `Option<OpId>` return would push a `None` case onto the single caller that has
already excluded it. Taking the already-matched `Post` fields instead of `&Op`
would make the arm *unrepresentable* — the "make the mistake impossible" move this
project prefers — and is the better shape; it is not taken here only because the
guard that makes it safe already exists in `reply` and moving it would be a reshape
of the same family as the prologue/tail one above. Worth doing with those.

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

### A locally-published op arrives `unordered`, and pays for it in every ordering

`Arrival` offers `ordered(lamport, message_id)`, `unordered()` and `from_parts`,
so this is a choice rather than the only option. A published op takes
`Arrival::unordered()`, because it did not arrive: claiming a Lamport value for an
op this peer created would be self-asserted ordering, which is the thing
CLAUDE.md's SDS section says must come from inside a signed preimage a relay
cannot forge rather than from whoever happens to be writing the row.

**The cost, stated because it is not obvious and is permanent.** An op with no
Lamport value sorts *after* every transport-ordered op, in the degraded
ascending-op-id block. So a user's own just-published post takes no position
advantage in any ordering — it does not appear at the top of the feed it was
posted into. That is the honest consequence of not inventing an ordering value,
and it resolves the moment the op comes back through delivery with real transport
metadata. It is the same gap `createdAt` would close, declined here for the
reasons the proposal gives.

### A store failure is a distinguishable refusal, not an absent parent

`Refusal::Storage` exists, and a `From<OpLogError>` routes every `?` in the module
into it, so "the store is broken" never arrives at the caller wearing "the parent
has not propagated"'s clothes. The two call for opposite responses — fix the disk
versus wait — and `a_store_that_cannot_be_read_is_a_refusal_and_not_an_absent_parent`
pins that the underlying reason survives.

This is the write-path twin of `list_threads`'s rule that a storage failure is
never an empty page. **The spec does not require it** — no requirement in
`content-authoring` or `module-wire-contract` owns a store failure on the publish
path — so it is observable behaviour this change chose. Recorded here and routed to
the spec-writer rather than left as a code comment citing a PLAN section number a
spec reader does not have open.

### The adapter reads `stoa` twice, and the second parser is the authority

`Dialectica::publishing` parses the request far enough to get the Stoa, because the
signing key is per-Stoa and must exist before a handler can be called; the handler
then parses the whole request properly, including that same field. The alternative
— thread the parsed `Address` in from the adapter — was rejected because it would
put half the request's validation in `src/lib.rs`, the one file no `cargo test`
compiles.

The cost is that one field is interpreted by two parsers that could in principle
disagree, and the error a caller sees for a malformed `stoa` may come from either.
The handler's parse is the one that owns the contract; the adapter's is a
key-derivation lookup that happens to need the same bytes.

- **No `createdAt`, no nonce**, and this is the one place that reasoning lives.
  Three things ruled it out, and each would have to be answered rather than
  argued around:

  1. It is a `MODIFIED` to `op-format`'s "An op carries no ordering field and no
     per-peer state" — a requirement that forbids a wall-clock field in terms
     `relevance-ordering`'s age requirement and PLAN's ranking rules both rest on.
     Changing it reaches two capabilities this change does not own.
  2. **The clamp is the whole defence and is unspecified.** An unclamped
     author-asserted timestamp lets a far-future value pin a post to the top of a
     recency ordering permanently. Adding the field without specifying the clamp
     ships the attack.
  3. A change that adds an authoring API *and* re-versions the op format cannot be
     reviewed for either.

  The consequence — identical content publishes once — is therefore implemented as
  the spec contracts it rather than worked around, and is visible to the caller
  through `wasNew`. PLAN keeps the forward-looking half (that the question is open,
  that a nonce is the narrower alternative if only deduplication is wanted, and what
  would decide it); it does not keep a second copy of the three reasons above.
- **No score, count or tally on a vote reply.** The reply shape is the same
  `{"opId","wasNew"}` as the others. Nothing in this change reads a `Vote` op.
- **No revision, no moderation, no attachments.** Out of scope per the proposal.
  `attachments` is therefore always the empty list, which `op-format` contracts
  as a value rather than an absence.
