# Findings — architecture (code-reviewer)

- [x] **none** — reviewed, no findings. Outcome: nothing to act on.

Judged against the owner's eight decisions in `proposal.md`, which are settled
and were not reopened:

- The split follows the line the decisions draw: `ChannelBook`, `Taken` and
  `Review` in `book.rs`; `InboundQueue`, `Offered` and `INBOUND_BOUND` in
  `queue.rs`; `Arriving`, `Channels`, `Shared`, `Next`, `HandedOver` and
  `refused_on_hand_over` stay, and nothing else left `delivery.rs`.
- No accessor was added (decision 2); the three field reaches are `pub(super)`
  fields. No new CI gate (decision 7). Neither new file contains a test.
- The dependency direction is parent-to-child for everything except `Arriving`,
  which `queue.rs` reaches upward with `use super::Arriving` and whose private
  `channel_id` field it reads. That is decision 4, and it compiles because a
  child sees its ancestors' private items; recorded here so it is not mistaken
  for an oversight.
- Doc links: the "three seams" section in `delivery.rs` still resolves its
  links to `ChannelBook::on_take`, `settle`, `startup_review`, `Review` and
  `InboundQueue` through the `use` lines (`cargo doc --document-private-items`
  reports them as private-item links, the same class as before, none as
  unresolved). No warning names `book.rs`; the one naming `queue.rs` is
  `INBOUND_BOUND`'s link to `refused_on_hand_over`, a private item reached from a
  public doc, which is how it stood in `delivery.rs`.
- Each commit is a coherent step: `2c2b11e7` touches `delivery.rs` and `book.rs`
  only, `86a0c180` touches `delivery.rs` and `queue.rs` only, and the first is
  clippy-clean on its own.
- CI gates: the Rust `Tests` step's count of `#[test]` under
  `dialectica/rust-lib/` is unchanged (no test moved), and no workflow or script
  names a path under `delivery/` other than through the test files.
