# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

## 1. The row's shape

- [x] 1.1 Add `feed::Replies` (a `NonZeroUsize` count and the latest reply's hex
      op id), `FeedRow::replies: Option<Replies>`, and the `reply_count()` /
      `latest_reply()` accessors (`design.md` Decision 3). Verify: the crate
      compiles, and the three places that pin the row's shape name the new field:
      the destructures in `feed.rs` and `tests/end_to_end.rs`, and the key set in
      `wire.rs`.

## 2. The fold

- [x] 2.1 Add `feed::visible_replies_by_thread`: one pass over the Stoa's
      entries, membership from `thread::thread_of`, hidden replies dropped by
      `moderation::resolve`, and the first reply met kept as latest (Decisions 1,
      2 and 5). Call it from `list_threads` before the head loop, without the
      include-hidden flag. Verify: every `feed::tests` test under "The reply
      count and the latest reply" passes: zero and absent, every depth, the
      ordering rule's first, hidden (with the no-hide counterfactual),
      only-reply-hidden, beneath-a-hidden, restored, non-moderator hide,
      forged, the thread-field claim, a parent that is not held, revision,
      include-hidden invariance, a hidden thread's row, row order unchanged,
      agreement with the thread read, and a store failure while counting.
- [x] 2.2 Pin the failure scope: a store failure on a reply whose thread is on
      another page fails the read. `feed-read` contracts this (commit `4eaa19c`),
      so the test no longer carries a `NO SPEC:` marker. Verify:
      `a_store_failure_on_a_reply_off_the_page_fails_the_page` passes.

## 3. The wire

- [x] 3.1 `feed_page_json` sends `replyCount` on every row and inserts
      `latestReply` only where there is one, never `null` (Decision 4). Verify:
      `the_feed_reply_is_the_ecosystems_pagination_shape` pins the key set
      without `latestReply`, and
      `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new` pins it
      with `latestReply`.

## 4. The record

- [x] 4.1 Rewrite `feed.rs`'s header paragraph that argued against a reply
      count, and record the withdrawal in `design.md` Decision 6. Verify: the
      header no longer says the row has no reply count.
- [x] 4.2 Run `thread-read`'s scenario "A reply orders after the reply it
      answers" against the code, and record the result without fixing it.
      Verify: `design.md` Risks, "Found while implementing".

## 5. Gates

- [x] 5.1 Verify with `cargo test --manifest-path dialectica/rust-lib/Cargo.toml
      -p dialectica -p dialectica-core` (green), clippy with `-D warnings` over
      both packages (clean), and
      `cargo fmt --manifest-path dialectica/rust-lib/dialectica-core/Cargo.toml
      --check` (clean). The core crate's manifest is named because the workspace
      `fmt` does not reach it.
