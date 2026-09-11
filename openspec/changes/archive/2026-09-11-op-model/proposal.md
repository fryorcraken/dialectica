# The op: the signed envelope everything else is built from

## Why

PLAN.md §3.3 makes the op the centre of the design: *"An op is a signed
operation, and it is the unit everything else is built from. [...] Nothing else
crosses the wire, and the forum's whole state is a function of the ops a peer
has seen."* Every post, revision, moderation action and vote is one.

`identity.rs` already had both halves of the cryptography — `sign_op_bytes` to
make a signature and `verify_authored_op` to check one — but **both take opaque
`&[u8]`, and nothing defined what those bytes were.** The same gap the
`stoa-genesis` change closed for a Stoa's address, one level up: without a
canonical encoding there is no agreement about what was signed, so two peers
cannot compute the same op id for the same op, and nothing constrains what a
signature covers.

The gap is not merely an omission. `identity.rs` records a live correctness debt
it could not discharge itself:

> **What it does NOT buy, because there is one prefix for all ops:** separation
> between op *kinds*. [...] that separation has to come from the canonical bytes
> being unambiguously typed, which is the serialiser's job and the serialiser
> does not exist yet.

One `OP_SIGNING_PREFIX` serves every op, so a signature commits to "some
dialectica op" and not to which one — a signature over a vote is, at the
cryptographic layer, a valid signature over any other op with the same bytes.
An encoding that leads with the kind discriminant makes the separation
structural rather than conventional, and that is a security fix, not a
formatting choice.

**This change is retroactive.** The code landed on this branch in two commits
(`Make room: the read head becomes a shared primitive` and `Define the op, and
put its kind inside the signature`) without a proposal, spec, design or tasks.
The op format is the most spec-worthy artifact in the repository — nothing else
crosses the wire — so the one change that most needs a behaviour contract was
the one change that did not have one. This supplies it. See `design.md` for what
writing the spec after the code cost and how that was managed.

## What Changes

Documents only. No behaviour changes; the gates were green before this change
and are green after it, over the same code.

- A `op-format` capability spec stating the op's behaviour contract: what a
  caller may rely on, and what a hostile peer cannot do.
- A `design.md` carrying the reasoning that `op.rs`'s module documentation
  already held — why the kind is inside the signed preimage, and why an op
  deliberately carries no Lamport timestamp, no `senderId`, no `channelId`, no
  wall clock and no sequence number.
- A `tasks.md` recording what was actually done, in the order it was done.

The code being specified, for reference:

- `dialectica-core/src/op.rs` — `Op`, `OpKind`, `SignedOp`, `OpId`, `OpError`,
  the canonical encoding, strict decoding, signing and verification.
- `dialectica-core/src/cursor.rs` — the bounds-checked read head, lifted out of
  `stoa.rs` in a separate refactor commit so the op decoder could share it
  rather than copy it.

## Capabilities

**New Capabilities**

- `op-format` — what an op contains, how it encodes to bytes, what a signature
  over it commits to, and what a decoder does with hostile input.

**Modified Capabilities**

None. `stoa-genesis` is untouched — see `design.md`'s "Why the shared encoding
rules were not extracted into a general capability" for why that was considered
and declined.

## Impact

- `dialectica-core/src/op.rs` and `dialectica-core/src/cursor.rs`, both already
  on this branch. `stoa.rs` changed only in the refactor commit, to use the
  shared cursor instead of its own private one — no behaviour change.
- No wire-contract change at the module boundary. Nothing in `wire.rs` exposes
  an op yet; this is the format, not the API that carries it.
- PLAN.md §13 and §5.7 were updated by the code commit: three open questions
  (are votes an op, is a hide reversible, when the `policy` field lands) are
  struck through as answered, and §13 records that §5.7's ordering rule still
  has no input at the delivery contract we have. That gap blocks the op log and
  the resolvers, **not** the op format, which is why the format landed without
  it.
- **Blocks nothing; unblocks the store.** An op log needs a decoder and an id,
  and both now exist.
