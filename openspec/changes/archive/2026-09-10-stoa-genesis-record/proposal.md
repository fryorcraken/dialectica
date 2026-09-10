# The Stoa genesis record, and its policy field

## Why

A Stoa **is** its genesis record — that is what makes creation permissionless,
since there is no registry to register with. `stoa_address(genesis_bytes)`
already hashes one, but it takes opaque `&[u8]` and **nothing defines what those
bytes are**. Two peers encoding the same Stoa differently would compute
different addresses for it, so "pasting an address is enough to verify what you
joined" (§4.8) is currently unenforceable: there is no canonical form to check
against.

The `policy` field is the part that gets more expensive by waiting. PLAN.md
§13 states the cost plainly: *"the genesis record wants a `policy` field even
while `open` is the only implemented value. Adding it in Phase 1 costs an enum
with one variant; adding it later means migrating every Stoa already created."*
A genesis record is immutable and address-determining, so a field added later
changes every existing Stoa's address — there is no in-place upgrade.

## What Changes

- A `Genesis` type carrying the Stoa's creator public key, epoch, and posting
  policy, plus its human-readable title.
- A **canonical byte encoding** with exactly one valid form per record,
  length-prefixed so no two distinct records can encode to the same bytes.
- A `Policy` enum with `Open` as its only variant today, reserving the
  discriminant space for invite / first-post-approval / token-threshold (§7.1).
- Decoding that **rejects** anything non-canonical, unknown, or truncated,
  rather than accepting it leniently.
- `stoa_address` gains a typed companion that takes a `Genesis` rather than
  loose bytes, so an address is always computed over a canonical encoding.

## Capabilities

**New Capabilities**

- `stoa-genesis` — what a Stoa's genesis record contains, how it encodes to
  bytes, and the verification properties an address computed from it carries.

**Modified Capabilities**

None. No existing spec describes Stoa creation.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/stoa.rs`. No existing file's
  behaviour changes; `identity.rs` gains no edits, and `stoa_address` keeps its
  byte-oriented signature for callers that already have canonical bytes.
- No wire-contract change. Nothing crosses the module boundary yet.
- Depends on PR #5 for the `dialectica-core` crate and `PublicKey`. This is a
  new file within that crate and does not edit any file that PR touches.
