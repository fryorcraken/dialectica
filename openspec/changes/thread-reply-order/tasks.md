## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Regression test first

- [x] 1.1 Add `a_reply_orders_after_the_reply_it_answers` to `thread.rs`, using
      #147's fixture (root at counter 1, a reply at 2, a reply to that reply at
      3). Verify: it fails against the unfixed code, returning
      `[root, counter 3, counter 2]`.
- [x] 1.2 Add one test for each new scenario: lowest counter leads,
      equal counters are reversed, and counter-less replies come first (with
      a counter-less root and with a counted root). Verify: each fails against
      newest first.

## 2. Reverse the reply order

- [x] 2.1 Walk `iter_stoa`'s sequence backwards in `read_thread`. Leave the
      root pinned at index 0 and assign positions after that (`design.md`
      Decision 1). Verify: the 1.x tests pass.
- [x] 2.2 Correct `thread.rs`'s module and function documentation, which said
      the sequence was the rule's own. Verify: `git grep -n "cmp_ops. order"
      -- dialectica/rust-lib/dialectica-core/src/thread.rs` shows only lines
      that mention the reversal.

## 3. Existing tests that asserted the old direction

- [x] 3.1 `the_root_is_the_first_item_whatever_the_logs_order_puts_first`:
      the replies are now the reverse of the log's relative order.
- [x] 3.2 `the_sequence_follows_the_counters_and_not_the_asserted_times`:
      rebuild the fixture so the counter order differs from both time orders
      (`design.md` Decision 3). Verify: its `assert_ne!` guards hold and it
      fails against newest first.
- [x] 3.3 `end_to_end.rs`'s
      `a_thread_read_over_a_store_on_disk_returns_the_root_and_its_replies`:
      counter-less replies are now expected in descending op id.

## 4. Gates

- [x] 4.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
      is green.
- [x] 4.2 A mutant that re-sorts by ascending counter turns the three tests
      named in `design.md` Decision 1 red.
- [x] 4.3 `nix build .#lgx` succeeds.
