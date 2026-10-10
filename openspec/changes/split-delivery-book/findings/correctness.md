# Findings — correctness (code-reviewer)

- [x] **none** — reviewed, no findings. Outcome: nothing to act on.

What was checked, against this tree rather than the PR body:

- **The move is a move.** `git diff --color=always --color-moved=plain
  origin/main...HEAD` over `delivery.rs`, `delivery/book.rs` and
  `delivery/queue.rs`, with every line that was not coloured as moved listed.
  The unmoved removed lines are the declaration and signature lines that gained
  `pub(super)`, and the import lines. The unmoved added lines are those same
  lines, the two `//!` headers, the six reference definitions, the new `use` and
  `mod` lines, and the three `use` lines per new file. Nothing else: no body line
  changed.
- **Zero diff in the test files.** `git diff --stat origin/main...HEAD` over
  `delivery/tests.rs` and `delivery/tests/` prints nothing.
- **Tests.** `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core` on the piece's HEAD: every test binary reports
  `ok`, none failed.
- **Clippy.** The CI invocation (`--all-targets -- -D warnings`) is clean on the
  HEAD, and also on `2c2b11e7` alone (detached), so the claim that the book
  commit is green on its own holds. The only warnings are the staged SDK's.
- **The adapter.** `nix build ./dialectica#lgx --no-link` succeeds on the HEAD.
- **Visibility.** Every moved item is `pub(super)`, never `pub(crate)`; `opening`
  and `messages` stay private; `INBOUND_BOUND` is the only `pub` item, re-exported
  with `pub use` from a private module, so no second public path appears. The
  `[`crate::delivery::INBOUND_BOUND`]` link in `parked.rs` resolves (`cargo doc`
  names no warning there).
- **No source-text test is blind after the move.** The only `include_str!` in the
  delivery tests reads the adapter `lib.rs`, not `delivery.rs`. Nothing outside
  the three files names `ChannelBook`, `InboundQueue`, `Taken`, `Review` or
  `Offered`.

Not run: `cargo mutants`. The owner's defaults rule it out, and a move changes
no logic for it to measure. A mutation of the `pub use` line (to see whether
anything fails without the `pub`) was attempted and the edit was refused by the
harness, so the design's own account of that gap (`design.md`, Risks) is
unverified by me; it is a reading-the-line check either way.
