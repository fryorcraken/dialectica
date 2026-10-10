# Findings — security (code-reviewer)

- [x] **none** — reviewed, no findings. Outcome: nothing to act on.

The diff adds no code with a body, so the untrusted-input surface is the one it
had. Checked:

- No new panic site, index, slice, `unwrap`/`expect` or arithmetic: every moved
  body is byte-identical (see `correctness.md`). `request`'s `saturating_add` and
  `settle`'s guarded decrement are as they were.
- No widening of what can reach the book or the queue. `pub(super)` reaches
  `delivery` and its descendants only. Had it been `pub(crate)`, any module could
  have built a `ChannelBook` outside the lock that orders it. The `opening` map
  and the `messages` deque are now private to their own files, so `delivery.rs`
  can no longer change the open-request count or the queue contents except
  through the methods, which is tighter than before the move.
- The `pub(super) open` and `pub(super) bound` fields are read by `delivery.rs`
  at three sites (`stoa_of`, `handoff`, `hand_over`) and written nowhere.
- Public API: the only public item that moved is `INBOUND_BOUND`, and its path is
  unchanged. No dependency, wire format or storage change.
