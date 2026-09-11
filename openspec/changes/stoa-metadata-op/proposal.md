# The Stoa metadata op: what a Stoa is called today

## Why

The `stoa-genesis` spec settled what a Stoa **is**: an immutable record whose
hash is the Stoa's address. It says so explicitly — the record's values are
"**founding** values, not current ones [...] Superseding a title or policy is a
separate capability — a moderator-signed metadata op — and is out of scope
here."

That capability does not exist. So today a Stoa's title is fixed forever at
creation, because the only place a title lives is inside the address preimage
and changing it mints a different Stoa. PLAN.md §5.7 names the missing half and
the relationship it completes:

> **The genesis record now carries a founding title and policy**, which makes
> the relationship concrete: genesis values are what the *address commits to*
> and can never change; the metadata op carries what the Stoa is called
> *today*. A reader prefers the latest valid op and falls back to the genesis
> values.
>
> The op itself is not built.

`op.rs` already defines the signed envelope, its canonical encoding and its
four kinds. What is missing is a kind for this. Adding one is a small,
well-bounded change to a format built to take it: the kind byte is already
inside every signature, so a new kind is separated from the existing four by
construction.

## What Changes

- A new `OpKind::StoaMetadata` variant carrying the Stoa's **display title** and
  **description**, encoded and decoded in `op.rs`'s existing canonical format.
- Its kind discriminant inside the signed preimage, exactly as the four existing
  kinds have it, so a signature over a metadata op cannot be replayed as any
  other kind and vice versa.
- Strict decoding: each malformation refused with a distinct error, bounds
  checked through the shared `cursor` module, and every field length checked
  against `MAX_FIELD_LEN` **before** allocating.
- **`policy` is deliberately NOT in this op.** See design.md — Decisions. The
  short form: the wire format reserves the space for free (a later `SetPolicy`
  kind costs an unused discriminant, not a version bump), and shipping a
  mutable-policy field today would mean shipping a scenario that cannot be
  tested and a fallback rule that reopens the hole `stoa.rs` closes.

## Capabilities

**New Capabilities**

- `stoa-metadata` — what a Stoa's mutable, moderator-owned metadata op carries,
  how it encodes, how it relates to the immutable genesis values, and what it
  deliberately does not carry.

**Modified Capabilities**

None. `stoa-genesis` already anticipated this capability by name and declared
superseding out of its own scope, so nothing there needs amending.

## Impact

- `dialectica/rust-lib/dialectica-core/src/op.rs` gains one `OpKind` variant,
  one encode arm, one decode arm and its tests. No existing kind's encoding
  changes, and no existing byte moves — the new kind takes the next free
  discriminant.
- No change to `stoa.rs`, `identity.rs` or `cursor.rs`.
- No wire-contract change at the module boundary. Nothing crosses it yet.
- **Out of scope, and must stay out:** resolution. Preferring the latest op over
  the genesis values needs an ordering, and PLAN.md §13 records that the
  delivery contract exposes neither a Lamport clock nor an SDS message id. This
  change defines the op; it does not order ops.
- **Out of scope:** the moderator-set check. `op.rs` verifies authenticity, not
  authority (§3.3), and two existing tests pin that split. A metadata op signed
  by a non-moderator is authentic and must still verify here.
