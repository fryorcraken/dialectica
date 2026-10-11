# Tasks — split the channel book and the inbound queue out of `delivery.rs` (#205)

## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec delta: a no-behaviour code move, `skip_specs: true` in `.openspec.yaml` says why; `proposal.md` is written
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — owner decision 2026-10-11: no-behaviour move, tests have zero diff
- [x] review: correctness, security, readability, architecture — `code-reviewer`
- [ ] ~~review: spec-test — `spec-test-reviewer`~~ — owner decision 2026-10-11: no-behaviour move, tests have zero diff
- [ ] ~~review: design — `design-reviewer`~~ — owner decision 2026-10-11: no-behaviour move, tests have zero diff
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Before moving anything

- [x] 1.1 Read every use of the moved types in `delivery/tests.rs` and `delivery/tests/parking.rs`, and list what they reach that `delivery.rs` does not. Verified by `design.md`, Decision 11: `InboundQueue::len`, and `HashMap` through the glob import.
- [x] 1.2 Run `cargo doc --no-deps --document-private-items -p dialectica-core` on the unmoved tree, as the baseline the moved docs' links are compared against. Verified by the run: `delivery.rs` has private-item link warnings and no unresolved link.

## 2. The channel book, in its own commit

- [x] 2.1 Move `ChannelBook`, `Taken` and `Review`, with their `impl` block and doc comments, into `delivery/book.rs` under a five-line `//!` header; `delivery.rs` declares `mod book` and re-imports the three. Verified by `git diff --color-moved` showing the bodies as moved.
- [x] 2.2 `pub(super)` on the three types, on the five methods and on the `open` field; `opening` stays private. Verified by the crate compiling with the test files untouched.
- [x] 2.3 Fix the three outbound doc links (`channel_answer`, `Opening`, `Action::Open`) and `Review`'s one (`Processor::review`) with `super::` reference definitions. Verified in 4.2.
- [x] 2.4 Keep `HashMap` in `delivery.rs` as a `#[cfg(test)]` import. Verified by removing it: `delivery/tests.rs` fails to compile at `declined_channels: HashMap<…>`.
- [x] 2.5 Clippy, the test suite and `nix build ./dialectica#lgx` green on this commit alone. Verified by running each against it.

## 3. The inbound queue, in its own commit

- [x] 3.1 Move `InboundQueue`, `Offered` and `INBOUND_BOUND`, with their `impl` block and doc comments, into `delivery/queue.rs` under a five-line `//!` header; `Arriving` stays and the queue imports it from the parent. Verified by `git diff --color-moved` showing the bodies as moved.
- [x] 3.2 `delivery.rs` declares `mod queue`, re-imports `InboundQueue` and `Offered`, and re-exports the constant with `pub use queue::INBOUND_BOUND`. Verified by reading the line: no gate fails without the `pub` (`design.md`, Risks).
- [x] 3.3 `pub(super)` on the two types, on `with_bound`, `offer`, `pop` and the test-only `len`, and on the `bound` field; `messages` stays private. Verified by the crate compiling with the test files untouched.
- [x] 3.4 Fix the two outbound doc links (`refused_on_hand_over`, `Channels`) with `super::` reference definitions. Verified in 4.2.
- [x] 3.5 Clippy, the test suite and `nix build ./dialectica#lgx` green on this commit alone. Verified by running each against it.

## 4. The whole move

- [x] 4.1 Zero diff in the test files. Verified by `git diff --stat` over the two code commits listing `delivery.rs`, `delivery/book.rs` and `delivery/queue.rs` and nothing else.
- [x] 4.2 `cargo doc --no-deps --document-private-items -p dialectica-core` after the move. Verified by the run: no warning names `book.rs`, the one naming `queue.rs` is the private-item link that was reported at `delivery.rs` in 1.2, and `delivery.rs` still has no unresolved link.
- [x] 4.3 Record the owner's eight decisions and the implementation's four in `design.md`. Verified by the file.

## Workflow follow-up

- Review, by one `code-reviewer` instance covering all four dimensions, reading `git diff --color-moved`.
- Archive the change once review is done; the struck stage rows are expected to be reported as incomplete.
