# Authoring content: posting, replying and voting through the wire API

## Why

The MVP scope the owner recorded puts posting, replying and voting in it, and
`wire.rs` today exposes a version string, a ping, a panic probe, a capability
probe and a delivery bridge — nothing that reaches an op, a log or a resolver.
The `posting-capability` spec already contracts the probe a view asks *before*
showing a compose box. Nothing contracts what happens when the user presses
submit.

Everything the publish path needs already exists as a contract. `op-format` owns
the op shapes, the canonical bytes, the signing and what a decoder does with
hostile input. `op-log` owns what a peer stores and what the store refuses to
decide. `op-ordering` owns what orders two ops. `post-revision` owns which
version of a post a reader renders. `module-wire-contract` owns JSON in, JSON
out and the single failure shape.

What is missing is the **operations**: what a caller supplies to publish
something, what must be true of the op that results, what the peer holds
afterwards, and what is refused. That is one capability, and it is the first
half of the core API that the plan calls the deliverable.

## What Changes

- Three publish operations — a post, a reply, a vote — each taking JSON,
  returning the op id of what was published, and reporting every failure as the
  single error shape.
- **A reply names its parent and nothing else.** The thread is derived from the
  parent, so a reply filed under the wrong thread is unrepresentable rather than
  checked. This is the plan's "complexity in the data structure, not the logic"
  rule applied, and it is a deliberate departure from the plan's illustrative
  `createReply({stoa, thread, parent, ...})` shape. It buys a real refusal: a
  reply to a parent this peer does not hold cannot be published, because there
  is nothing to derive the thread from. That refusal is contracted rather than
  hidden.
- **The identity is never a parameter.** It falls out of the Stoa, so no
  operation can be asked to sign as someone it is not.
- **Publishing is "append and publish", not "send".** The caller is told the op
  exists locally; delivery's outcome arrives later and a call that waited on it
  would be a call that can hang.
- **Two identical posts are one op, and the spec says so.** An op id is the hash
  of bytes carrying neither a timestamp nor a nonce, so one author posting the
  same body into the same Stoa twice publishes one op and the second call reports
  the op id of the first. This is contracted as observable behaviour of the
  publish path — see Capabilities for why it is not fixed here.
- **A vote is published, stored and readable, and no merged read path ranks by
  it.** Contracted exactly that narrowly.

## Capabilities

### New Capabilities

- `content-authoring` — the publish path for a post, a reply and a vote: what a
  caller supplies, what must hold of the op produced, what the peer holds after,
  and every refusal. One capability rather than three, because the three share
  every property that is decided here — the identity is derived not supplied,
  the reply is the op id, the append precedes the handoff to delivery, and the
  failure shape is the wire contract's.

`openspec list --specs` was checked before naming it. The near-duplicate to
avoid was `posting-capability`, which is a **probe** — it answers whether
posting is possible and publishes nothing. Extending it would have put the
question and the act under one name, which is how the answer to "can I?" starts
being taken as the record of "I did". `content-authoring` is the act.

### Modified Capabilities

None. This is the part of the change most worth defending, because three
capabilities were each considered and declined.

- **`op-format` is not amended, and the duplication-collision question is the
  reason to say so explicitly.** The publish path builds ops of kinds
  `op-format` already contracts, and the reply-names-a-parent field is already
  inside its signed bytes. Nothing here changes an op's shape.

  A **`createdAt` field was considered and declined for this change**, and it is
  the one declination that leaves a known defect in place rather than merely
  keeping a boundary. It would fix two symptoms at once — identical content
  colliding into one op, and both accepted feed orderings degrading to ascending
  op id because no op carries a value that orders anything. It is declined here
  for three reasons, none of them that the problem is small:

  1. It is a `MODIFIED` to `op-format`'s "An op carries no ordering field and no
     per-peer state", a requirement that **forbids a wall-clock timestamp** in
     terms load-bearing elsewhere — the relevance capability's age requirement
     and the plan's decay rule both rest on it. Overturning it is a decision
     about the wire format, argued in that capability's own change.
  2. An author-asserted timestamp is unusable until it is **clamped**, and the
     clamp is unspecified. The nearest comparable project reads an unclamped
     author timestamp and a post can pin itself to the top permanently. A field
     a malicious peer sets freely is not an ordering until it has a bound, and
     specifying the bound is not a rider on a publish path.
  3. A change that both adds an authoring API and re-versions the op format
     cannot be reviewed for either.

  **What this change does instead is make the behaviour visible rather than
  leaving it to be discovered**: a requirement here states that identical content
  from one author is one op, that the second publish reports the first op's id
  rather than failing, and that a caller can tell the two apart. The next reader
  meets it as a contracted decision. A `createdAt` change, when it comes, will
  need to modify that requirement, which is the correct place for the pressure
  to land.

- **`op-log` is not amended.** It already contracts that the log stores what
  arrived and decides nothing, that an op is stored once identified by its op id,
  and that re-appending leaves one entry and does not disturb the stored op. The
  publish path is a caller of that contract, and the duplicate-post behaviour
  above is that contract seen from the write side — not a second statement of it.

- **`posting-capability` is not amended.** Its probe answers before the compose
  box renders; this capability governs submission. The one thing worth stating is
  the boundary, and it is stated in the spec's Purpose rather than duplicated as
  a requirement: a successful probe is not a guarantee that a later publish
  succeeds, and this capability does not require the publish path to re-derive
  the probe's reasons.

- **`module-wire-contract` is not amended, and one nearby fix is deliberately
  left alone.** Valid JSON that is not an object currently reaches a field lookup
  where an absent field and a wrong-shaped request are indistinguishable. That is
  a defect across the whole wire surface and **a separate change owns it.** No
  requirement here contradicts it: this capability's refusals are stated in terms
  of what a request must carry, never in terms of what a non-object request does.

## Impact

- `wire.rs` gains three methods. This is a widening of the deliverable and the
  first publish path across the module boundary.
- **Out of scope, and must stay out:**
  - **Revising a post.** `post-revision` contracts which version is current and
    the plan names `revisePost`, but a revision's refusals turn on authorship of
    a target op, which is a different question from either of the three here.
    Deferred so this change carries one argument.
  - **Moderating.** Out of the MVP by the owner's scope decision.
  - **Attachments.** Out of the MVP; a post in the MVP is text. `op-format`
    already contracts that attachment references decode and that an empty list
    is a value, so nothing is lost by not exercising them here.
  - **Reading.** The feed and thread reads are their own capability, and the
    ordering question they carry is not settled by publishing.
  - **Ranking a vote.** The in-flight `relevance-ordering` capability owns what a
    vote contributes to an ordering. This change publishes votes and asserts
    nothing about their effect.
- **A note for whoever writes the read side:** this change's honest bound on
  votes — published, stored, readable, ranked by nothing merged — stops being
  true the moment `relevance-ordering` is archived. That requirement is the one
  to revisit then.
