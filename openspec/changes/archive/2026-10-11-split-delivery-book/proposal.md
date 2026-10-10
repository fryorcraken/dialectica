# Proposal — split the channel book and the inbound queue out of `delivery.rs`

Scoped by GitHub issue #205.

## Why

`dialectica/rust-lib/dialectica-core/src/delivery.rs` holds many jobs in one
file: the delivery seam and reply parsing, the node config, the journal, the
channel state machine, the lock that orders it, the bounded inbound queue, the
worker, the listener, the processor and the lifecycle. Two of those jobs are
pure data and rules with no I/O and no threads — the channel book and the
inbound queue — and they are where the parking decisions live (`on_take`,
`settle`, `startup_review`, `offer`). Read in a file of their own, they sit
beside the tests in `delivery/tests/parking.rs` that drive them, and the rest
of `delivery.rs` gets shorter by exactly what is pure.

The architecture review of the parking change (PR #202) raised this as a
preference and it was deferred to its own piece rather than bundled, so that
the move can be reviewed as a move.

## What Changes

A no-behaviour code move. Nothing a caller, a peer or a test can observe
changes.

- **New `delivery/book.rs`** takes `ChannelBook`, `Taken` and `Review`, with
  their `impl` blocks and doc comments.
- **New `delivery/queue.rs`** takes `InboundQueue`, `Offered` and
  `INBOUND_BOUND`, with their `impl` blocks and doc comments.
- **`delivery.rs` declares both modules and re-imports what moved**, so the
  tests' `use super::*` still sees every moved name.
- **`INBOUND_BOUND` is re-exported with `pub use`**, so the public path
  `dialectica_core::delivery::INBOUND_BOUND` is unchanged.

### Owner decisions

Settled by the owner in a design interview on 2026-10-11. They are recorded
here because they are in no other file, and decisions 1 and 3 amend the
issue's text. They are the owner's: implement and review against them, do not
revisit them.

1. **Acceptance bar.** The issue's "`git diff --stat` shows only moved lines"
   cannot be met literally: the moved items are private today, and a child
   module needs visibility changes. The bar is **moved lines plus a closed list
   of permitted residue, stated in the PR body**:
   - the `mod` declarations and `use` lines, on both sides;
   - `pub(super)` on moved items;
   - doc-link path fixes;
   - a short module header per new file.

   **Zero diff in `delivery/tests.rs` and under `delivery/tests/`.** Review
   reads `git diff --color-moved`.
2. **Field reaches.** `delivery.rs` reads `book.open` twice and `waiting.bound`
   once. Those become `pub(super)` fields. **No accessor methods**: an accessor
   is new code, and this piece adds none.
3. **Two files, not one.** This amends the issue's title, which names only
   `delivery/book.rs`. `delivery/book.rs` takes `ChannelBook`, `Taken` and
   `Review`; `delivery/queue.rs` takes `InboundQueue`, `Offered` and
   `INBOUND_BOUND`.
4. **`Arriving` stays in `delivery.rs`**; the queue imports it from the parent.
   `INBOUND_BOUND` moves with the queue and is re-exported with `pub use`.
5. **Where the move stops.** `Channels`, `Shared`, `Next`, `HandedOver` and
   `refused_on_hand_over` stay in `delivery.rs`. No other job — the journal and
   `Note`, reply parsing, the worker, the processor — leaves the file in this
   piece, and no follow-up issue is filed for any of them.
6. **The pure unit tests do not move.** The test files have zero diff.
7. **Docs.** Each new file gets a three-to-five-line `//!` header saying what
   is in it and that the lock is `Channels`'s. The "three seams" section of
   `delivery.rs`'s module header stays where it is. Outbound intra-doc links
   from the moved docs are fixed with `super::` paths, and checked once locally
   with `cargo doc --document-private-items`. **No new CI gate.**
8. **Roster.** One `dev-writer`, one `code-reviewer` instance covering all four
   dimensions, then the `closer`. No `tester`, no `spec-test-reviewer`, no
   `design-reviewer`: there is no behaviour to pin and the tests have zero
   diff. `tasks.md`'s stage block carries this.

### Owner defaults for the implementation

- **Two commits, one per new file**, each leaving every gate green on its own.
- **The gates** are the `cargo test` command in `README.md`, clippy, and
  `nix build ./dialectica#lgx`.
- **No `cargo mutants` run**: no logic changes, so there is nothing for it to
  measure.
- **The PR body says `Closes #205`**, and contains no other closing keyword
  next to an issue number.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` declares `skip_specs: true` alongside its `schema:` key:
the parking, hand-over and bound behaviour this code implements is contracted
in `op-transport`, and none of it changes — the same items, with the same
bodies, are compiled into the same crate from different files.

## Impact

- **Code:** `dialectica/rust-lib/dialectica-core/src/delivery.rs`, and the two
  new files `delivery/book.rs` and `delivery/queue.rs` beside the existing
  `delivery/tests.rs`.
- **Public API:** unchanged. `dialectica_core::delivery::INBOUND_BOUND` is the
  one public item that moves, and its path is kept by the re-export. The moved
  types are private to `delivery` today and are `pub(super)` afterwards, which
  reaches no further than the `delivery` module.
- **Tests:** unchanged, by decision 6.
- **What the gates cannot see here:** the public path
  `dialectica_core::delivery::INBOUND_BOUND` has no code user outside the
  `delivery` module — `git grep -F INBOUND_BOUND -- dialectica/rust-lib` finds
  it only in `delivery.rs`, in `delivery/tests.rs` and in one doc link in
  `parked.rs` — so a dropped or narrowed re-export compiles and passes every
  test; it is checked by reading the `pub use` line. `cargo test` never
  compiles the adapter behind `cfg(logos_scaffold)`, which is why
  `nix build ./dialectica#lgx` is among the gates. No gate checks intra-doc
  links: decision 7's one-off `cargo doc` run is the only check on them, the
  `parked.rs` link included.
- **Dependencies, wire format, storage:** none.
