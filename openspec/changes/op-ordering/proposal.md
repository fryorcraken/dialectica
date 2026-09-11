# How ops are ordered, and the transport metadata that orders them

## Why

PLAN.md §13's last open question states that §5.7's ordering rule "has no input at
the contract we have", and that this "blocks the op log and the
revision/moderation resolvers". §5.7 orders an author's revisions by "the highest
Lamport timestamp [...] ties broken by ascending message id — the same rule SDS
already applies (§4.4), so nothing new is invented". Moderation ops on one target
are ordered the same way.

**The investigation that settles it found the metadata is real, is what SDS
orders by, and does not reach us.** SDS maintains a per-channel Lamport clock and
a per-message id, and its own conflict rule is exactly §5.7's. But the Reliable
Channel API — the layer the delivery module consumes, one below the LIDL contract
— defines its received-message event as carrying a single field: the reassembled
payload. Nothing below it is dialectica's to change. The evidence and the exact
upstream gap are in `design.md`.

So this change does **not** design an ordering algorithm. §5.7's rule is already
the right rule and already SDS's rule; what is missing is its input. What this
change settles is the narrower, answerable question: **what a peer records
alongside an op when the op arrives, so that the resolvers §13 says are blocked
have a defined thing to read** — and what those resolvers must do while the
transport does not supply it.

The reason to land the type now rather than with the store is that it is the
store's key. A store built against "whatever the transport happened to hand us"
would have the ordering decision baked into its schema by accident, which is the
outcome §13 asked to avoid by saying "deciding by accident is not" acceptable
elsewhere in the same list.

## What Changes

- An `Arrival` type: the transport metadata a receiving peer records **alongside**
  an op, never inside it. It carries an optional Lamport timestamp and an optional
  SDS message id, because the contract we have supplies neither, and a peer must
  be able to record what it actually received rather than a fabricated value.
- A total order over `(Arrival, OpId)` implementing §5.7's rule — highest Lamport
  first, ties by ascending message id — with a defined, documented answer for the
  degraded case where the transport supplied no Lamport value.
- The **degraded case is explicit and named**, not a silent fallback: ops whose
  Lamport value is absent are ordered among themselves by op id, and always sort
  below any op that carries one. A peer can tell the two apart and say so.
- `op.rs`'s module documentation is corrected. It currently says the gap "is
  Phase 2's to close"; the investigation shows it is upstream's to close, across
  two layers, and names both.

Not changed: the op format. An op still carries no ordering fields, and
`an_op_carries_no_ordering_fields` still pins that. This change is the other half
of the sentence that test's comment already contains.

## Capabilities

**New Capabilities**

- `op-ordering` — what orders two ops, what a peer records to be able to answer
  that, and what it does when the transport does not tell it.

**Modified Capabilities**

None. `stoa-genesis` is untouched.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/arrival.rs`.
- `dialectica-core/src/lib.rs` gains one `pub mod` line.
- `op.rs`: doc comment only. No behaviour, no encoding, no test expectation moves.
- **No wire-contract change, and deliberately no upstream filing.** `design.md`
  records precisely what `delivery_module.lidl` and the Reliable Channel API
  would each have to add; filing it is the project owner's.
- Does not build the store. The store is a later change, and this type is what it
  will key on.
