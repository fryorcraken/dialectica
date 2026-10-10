# Design — split the channel book and the inbound queue out of `delivery.rs`

## Context

See `proposal.md` — Why. `dialectica-core/src/delivery.rs` held many jobs in
one file. Two of them are pure: `ChannelBook` with its two answers `Taken` and
`Review`, and `InboundQueue` with `Offered` and `INBOUND_BOUND`. Neither does
I/O or starts a thread, and neither takes a lock — the lock is `Channels`'s.
They are also where the parking decisions live (`on_take`, `settle`,
`startup_review`, `offer`), which is what makes them worth reading on their
own.

The constraint that shapes everything below: `delivery/tests.rs` and
`delivery/tests/parking.rs` open with `use super::*`, and must not change. A
glob import of a module brings in that module's own private `use` imports as
well as its items, so whatever `delivery.rs` imports, the tests see.

## Goals / Non-Goals

**Goals:**

- The two pure jobs each in a file of their own, with identical bodies.
- A diff that reads as a move under `git diff --color-moved`, plus a closed
  list of residue.
- Zero diff in the test files.

**Non-Goals:**

- Any change to what the code does, to the public API, or to a test.
- Moving any other job out of `delivery.rs` (Decision 5).
- Tidying what the move leaves looking slightly off — see "Risks".

## Decisions

Decisions 1 to 8 are the owner's, settled in a design interview on 2026-10-11
and listed in `proposal.md`. They are recorded here with their reasoning
because the proposal carries them as rulings. Decisions 9 to 12 are the
implementation's own.

### 1. The acceptance bar is moved lines plus a closed list of residue

The issue asked that `git diff --stat` show only moved lines. That cannot be
met literally: the moved items were private to `delivery`, and an item in a
child module is invisible to its parent until it says otherwise. So the bar is
moved lines plus four kinds of residue, and nothing else:

- the `mod` declarations and `use` lines, on both sides;
- `pub(super)` on moved items;
- doc-link path fixes;
- a short module header per new file.

The list is closed so that review has something to check a hunk against: a
changed line that is none of the four is a finding. Review reads
`git diff --color-moved`, which marks a moved block apart from a changed one;
`--stat` cannot tell them apart.

### 2. Field reaches become `pub(super)` fields, with no accessors

`delivery.rs` reads `book.open` (in `Channels::stoa_of` and
`Channels::handoff`) and `waiting.bound` (in `Channels::hand_over`). Both
fields are now `pub(super)`.

Considered: accessor methods (`ChannelBook::open()`, `InboundQueue::bound()`),
which would keep the fields private. Ruled out because an accessor is new code
with a body, and this piece adds none — it would be the one hunk in the diff
that is neither a move nor residue, and the reason to review a move as a move
is that there is nothing else in it.

`ChannelBook::opening` and `InboundQueue::messages` stay private: nothing
outside their own file reads either, the tests included.

### 3. Two files, not one

The issue's title names only `delivery/book.rs`. The book and the queue are two
jobs that share nothing but a lock neither of them owns: the book answers
"what state is this channel in", the queue answers "which payload does a full
queue lose". One file holding both would repeat, one level down, the shape this
piece exists to undo. So `delivery/book.rs` takes `ChannelBook`, `Taken` and
`Review`, and `delivery/queue.rs` takes `InboundQueue`, `Offered` and
`INBOUND_BOUND`.

### 4. `Arriving` stays in `delivery.rs`; `INBOUND_BOUND` moves and is re-exported

`Arriving` is the event as the listener received it. The listener, the
processor, the adapter (which builds one per event) and the queue all use it,
so it belongs to the module they share rather than to the queue, which merely
holds some. `queue.rs` imports it with `use super::Arriving`.

`INBOUND_BOUND` is the queue's bound and its doc comment is the argument for
the number, so it moves with the queue. It is `pub`, and
`dialectica_core::delivery::INBOUND_BOUND` is a path `parked.rs` links to, so
`delivery.rs` re-exports it with `pub use queue::INBOUND_BOUND`. The modules
themselves are private (`mod book; mod queue;`), so the re-export is the only
path to the constant and no second public path appears.

### 5. Where the move stops

`Channels`, `Shared`, `Next`, `HandedOver` and `refused_on_hand_over` stay in
`delivery.rs`. Each of them is about the lock or about hand-over: `Channels`
owns the mutex and the condvar, `Shared` is what the mutex guards, `Next` and
`HandedOver` are what the two locked operations return, and
`refused_on_hand_over` reads the book and the message limit together. None is
pure data in the sense the two moved jobs are.

No other job — the journal and `Note`, reply parsing, the worker, the
processor — leaves the file in this piece, and no follow-up issue is filed for
any of them. The issue asked for the two that are "pure data and rules, with no
I/O and no threads", and the owner ruled that the move stops there.

### 6. The pure unit tests do not move

The book's and the queue's unit tests stay where they are, in
`delivery/tests.rs` and `delivery/tests/parking.rs`. The issue's bar is that
"every test passes unchanged", and zero diff under `delivery/tests.rs` and
`delivery/tests/` is what lets that be checked with one `git diff --stat`
rather than by reading a second move.

### 7. Docs: a header per file, the "three seams" section stays, links fixed with `super::`

Each new file opens with a five-line `//!` header saying what is in it and that
the lock is `Channels`'s. The "three seams parking turns on" section of
`delivery.rs`'s module header stays where it is: it is about how the book, the
lock and the store fit together, which no one of the three files can say alone.
Its links to `ChannelBook::on_take` and the rest still resolve, through
`delivery.rs`'s `use book::…` imports.

Outbound links from the moved docs are fixed with `super::` paths and checked
once, locally, with `cargo doc --document-private-items`. No CI gate is added
for it; that is the owner's ruling. Worth knowing beside it: that run reports
unresolved doc links in other modules of `dialectica-core` today, so such a
gate would be red on arrival for reasons outside this piece.

### 8. Roster: one `dev-writer`, one `code-reviewer` covering all four dimensions, the `closer`

No `tester`, no `spec-test-reviewer`, no `design-reviewer`: there is no
behaviour to pin and the tests have zero diff. `tasks.md`'s stage block carries
the struck rows.

### 9. Doc links are fixed with reference definitions, so the moved lines stay byte-identical

Six links in the moved docs pointed at items that stay in `delivery.rs`:
`channel_answer`, `Opening` and `Action::Open` from `ChannelBook`'s doc,
`Processor::review` from `Review`'s, `refused_on_hand_over` from
`INBOUND_BOUND`'s, and `Channels` from `InboundQueue`'s (twice, one
definition).

Each is fixed by appending a link reference definition to the doc comment that
uses it:

```rust
/// [`Opening`]: super::Opening
```

Considered: rewriting each link in place, as ``[`Opening`](super::Opening)`` or
``[`super::Opening`]``. Ruled out because either edits a line inside a moved
paragraph, and lengthens it — where that passes the wrap column the paragraph
re-flows, and a block `--color-moved` would show as moved shows as changed.
With definitions, every moved doc line is byte-identical and the fix is added
lines only.

Considered: importing the names into the child (`use super::Opening`) so the
bare links resolve. Ruled out: the imports would be used by docs alone, and
`unused_imports` fails clippy under `-D warnings`.

### 10. Every moved item is `pub(super)`, never wider

`pub(super)` from `delivery::book` reaches `delivery` and its descendants —
which includes `delivery::tests` and `delivery::tests::parking` — and stops
there. `pub(crate)` would have compiled and would have let any module in the
crate build a `ChannelBook` outside the lock that orders it.

### 11. What the tests reach that `delivery.rs` does not

Checked by reading every use of the moved types in both test files, because the
tests cannot change and anything they reach needs visibility too. Two things:

- **`InboundQueue::len`**, already `#[cfg(test)]`, called at three sites in
  `tests.rs` and nowhere in `delivery.rs`. It is `pub(super)` and keeps its
  `#[cfg(test)]`.
- **`HashMap`, through the glob.** `tests.rs` uses `HashMap` in its fake
  delivery and never imports it: it arrived through `use super::*` from
  `delivery.rs`'s own `use std::collections::{HashMap, …}`. `ChannelBook` was
  the only user in `delivery.rs`, so after the move the import is unused
  outside a test build and clippy rejects it. It is kept as
  `#[cfg(test)] use std::collections::HashMap;`, the same shape as the
  `#[cfg(test)] use crate::parked::ParkBounds;` already beside it.
  **Removing that line turns the whole `delivery::tests` module red** — it
  fails to compile at `tests.rs`'s `declined_channels: HashMap<…>`.

Everything else the tests reach — `ChannelBook::default`, `request`, `settle`,
`on_take`, `is_known`, `startup_review`, the `open` field,
`InboundQueue::with_bound`, `offer`, `pop`, and the variants of `Taken`,
`Review` and `Offered` — `delivery.rs` reaches too.

### 12. Two commits, one per new file

Each commit leaves clippy, the test suite and `nix build ./dialectica#lgx`
green on its own, so either can be read, or reverted, without the other.

## Risks / Trade-offs

- **A dropped or narrowed `pub use queue::INBOUND_BOUND` compiles and passes
  every test.** Nothing outside `delivery` uses the public path in code; the one
  outside reference is a doc link in `parked.rs`, and `delivery.rs` itself uses
  the name, so even a private `use` raises no warning. → Checked by reading the
  line; no gate was found that fails without the `pub`.
- **No gate checks intra-doc links.** → `cargo doc --no-deps
  --document-private-items -p dialectica-core` was run before and after the
  move. After it, no warning names `book.rs`; the one naming `queue.rs` is the
  private-item link from `INBOUND_BOUND` to `refused_on_hand_over`, which was
  reported at `delivery.rs` before the move; and `delivery.rs` has no
  unresolved link in either run.
- **`cargo fmt --check` cannot see these files.** CI's formatting step does not
  follow the path dependency into `dialectica-core`. → The moved bodies are
  unchanged, and the added lines were kept within the width the surrounding
  code uses.
- **The section comment `// ─── Inbound: the bounded queue ───` now heads
  `Arriving` and `parked_inbound`, with the queue itself gone.** Left as it is:
  rewording it is none of the four kinds of residue.
- **`cargo test` never compiles the adapter**, which names
  `dialectica_core::delivery` items behind `cfg(logos_scaffold)`. → That is why
  `nix build ./dialectica#lgx` is run on each commit.
