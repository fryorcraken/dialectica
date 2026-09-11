# The op log: the append-only store every piece of forum state is derived from

## Why

PLAN.md §3.3 makes the op log the authority: "the forum's whole state is a
function of the ops a peer has seen [...] Ops are the authority; the view is a
cache that can be rebuilt by replay." Everything downstream — which version of a
post is current (§5.7), whether a target is hidden (§6), what a thread looks like
— is a fold over this log. Nothing can be built until it exists.

`op.rs` supplies what is stored and `arrival.rs` supplies what orders it. Neither
supplies the thing that holds them together: a peer has nowhere to put an op it
received, no way to notice it has seen that op before, and no way to ask for its
ops in the order §5.7 defines. This change is that container.

**The two resolvers are blocked on this and not on each other.** §13's
op-ordering entry records that "the op log and the resolvers are unblocked: they
have a defined thing to key on" — the ordering primitive cleared their shared
dependency, and this change supplies the store they both read. A revision
resolver asks "which of these ops, all revising one target, is current?" and a
moderation resolver asks "what is the latest moderation op on this target, and
was its signer a moderator?". Both are folds over an ordered subsequence of the
log. Landing the log separately is what lets them be built in parallel, by
different changes, against a read API that already exists.

## What Changes

- An `OpLog` **trait**: the `Store` seam PLAN.md §9 Phase 1 names by that word.
  Append an op with its `Arrival`; look one up by `OpId`; iterate in `cmp_ops`
  order; count. The trait is the contract the resolvers are written against, so
  that swapping the SQLite implementation underneath them is a change to
  one file rather than to every reader.
- A `MemoryOpLog`: the in-memory implementation, which is also the fake §9 Phase 1
  requires ("tested against fakes with no node running"). It is not a test double
  living in `#[cfg(test)]` — a peer with no persistence configured is a real
  configuration, and the resolvers' tests are the same code path a real peer runs.
- **Dedup by `OpId`, and the first arrival's metadata wins.** §3.1 calls ops
  "idempotent by `opId`". Appending the same op twice yields one entry, and the
  caller is told which of the two happened rather than having to infer it.
- **Nothing is verified, filtered, or rejected on append.** §3.3: "Verification
  therefore happens on **read** [...] The store may hold junk; the reader never
  trusts it." A log that dropped an unverifiable op on the way in would hide a
  forgery instead of recording it, and would discard ops that a later
  moderator-set change might make meaningful.
- A **read API shaped for a fold**: ops in order, ops in order filtered to one
  Stoa, and ops in order filtered to those naming a given target. The third is
  the shape both resolvers need, and it is one method rather than two because
  "the ops about this target" is one question.
- **Replay**: iterating the log in order is replay. There is no separate verb,
  because a materialised view rebuilt by replay is a later change and a `replay()`
  that only calls `iter()` would be a second name for one job.

Not changed: the op format, `arrival.rs`, or any transport wiring. Ops enter the
log as `Arrival::unordered()` today because nothing can supply a Lamport value
(see the `op-ordering` change); when the upstream fields land, an ordered
`Arrival` flows through the same `append` with no schema change and no migration.

## Capabilities

**New Capabilities**

- `op-log` — what a peer stores, what it refuses to decide while storing, and what
  a reader may ask of it over a set of ops that is legitimately incomplete.

**Modified Capabilities**

None. `op-ordering` and `stoa-genesis` are untouched.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/log.rs`.
- `dialectica-core/src/lib.rs` gains one `pub mod` line.
- **No new dependency.** `rusqlite` was considered and declined for this change;
  `design.md` records the argument and what would reverse it.
- No wire-contract change, no change to the module's JSON API. The log is internal
  state; what a view sees is the later materialised-view change.
- Does not build the resolvers, the materialised view, or any query index. The
  read API is shaped so each can be, and `design.md` says concretely how.
