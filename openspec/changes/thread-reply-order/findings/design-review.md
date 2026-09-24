# Design review — thread-reply-order

`design.md` exists and its Decisions section was checked against the code
(`git diff origin/main...HEAD`, three dots), against the amended
`thread-read` spec, and against `gh issue view 147 --repo fryorcraken/dialectica`
plus the owner's 2026-09-24 decision recorded in `proposal.md`.

## Summary

No unticked findings. The Decisions section is in good shape:

- **Decision 1** (walk backwards, do not sort) matches `read_thread`
  (`dialectica/rust-lib/dialectica-core/src/thread.rs:698`,
  `.into_iter().rev()`), and every one of its three named alternatives is a
  real alternative with a real reason it was rejected. The mutation-evidence
  claim ("swapping the backwards walk for `sort_by_key(|e| (counter, id))`
  turns exactly three tests red") was checked by hand-tracing each named test's
  fixture against that mutant's comparator rather than by trusting the prose:
  `replies_with_equal_counters_are_reversed_rather_than_re_sorted` fails
  because an ascending-id tiebreak puts the lower id first where the spec
  wants the higher; `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`
  fails because a natural ascending comparator leaves counter-less entries in
  ascending id (same as `cmp_ops`'s own tiebreak, so no reversal), where the
  spec wants descending; `the_root_is_the_first_item_whatever_the_logs_order_puts_first`
  uses `a_reply` (counter-less) fixtures throughout, so the same ascending-id
  argument applies and the reversal the test asserts does not appear under the
  mutant. The two tests the design says stay green
  (`the_lowest_counter_leads_the_replies_and_the_highest_ends_them`,
  `a_reply_orders_after_the_reply_it_answers`) use fixtures with distinct
  counters and no ties, where ascending-by-counter and reverse-of-descending
  agree — consistent with the claim. I did not execute the mutant (see note
  below); this is a traced-by-hand check, not a run one.
- **Decision 2** (the rule stays as it is) is accurate: `git grep -n
  "cmp_ops"` across `revision.rs`, `moderation.rs`, `feed.rs` confirms each of
  the three readers named (`revision::current_version`, `moderation::resolve`,
  the feed's `latestReply`) does depend on `cmp_ops`'s own direction, so
  changing it would move all three.
- **Decision 3**'s rebuilt fixture (`the_sequence_follows_the_counters_and_not_the_asserted_times`)
  is in the diff exactly as described: three distinct sequences (by counter,
  ascending time, descending time), asserted apart with `assert_ne!` before
  the real assertion runs.
- **Decision 4** (`latestReply` unaffected) — `the_count_agrees_with_the_thread_read`
  in `feed.rs` compares membership, not position, as claimed; unaffected by
  this change.
- The Risks section's two edge cases (no counter; an over-bound parent) are
  each pinned by name:
  `a_reply_carrying_a_lower_counter_than_the_reply_it_answers_comes_before_it`
  and the `a_thread_read_over_a_store_on_disk_returns_the_root_and_its_replies`
  end-to-end update.
- Non-Goals' claim that the pre-existing `cargo fmt` drift in `identity.rs`
  and `wire.rs` is unrelated and already on `main` was checked directly
  (`cargo fmt --check -p dialectica-core -p dialectica`): both files show
  drift, `thread.rs` shows none.
- Nothing in the diff reads as a decision made in code but left unrecorded —
  every constant, tiebreak direction and boundary case traces to a Decision or
  a Risk entry, largely because the module's own doc comments repeatedly point
  at `design.md` by name.
- The code does not contradict the GitHub issue: the issue leaves the choice
  between "reverse" and "amend the scenario" open, `proposal.md` records the
  owner's 2026-09-24 choice of "reverse," and the code implements that. The
  issue's "Not affected" section is reproduced as Decision 4 rather than left
  to rot in the issue alone — the reasoning migrated to `design.md` as it
  should.
- Both gates are green in this worktree: `cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` (1146 + 30
  passed, 0 failed) and `nix build ./dialectica#lgx`.

## Note on the mutation-evidence check

I attempted to actually run Decision 1's named mutant (replacing the
backwards walk with an ascending `sort_by` over `(counter, id)`) to confirm
the "exactly three tests turn red" claim by execution rather than by hand
tracing. The edit was blocked by the harness's auto-mode classifier
("Modify Shared Resources") before it compiled, and I did not attempt to work
around that block — I reverted the partial edit (`git checkout --
dialectica/rust-lib/dialectica-core/src/thread.rs`) immediately and the tree
is confirmed back to the committed diff (`git diff origin/main...HEAD --stat`
matches, `git status --porcelain` is empty). The hand-traced check above
stands in its place and reaches the same conclusion the design claims, but a
reviewer with permission to mutate source in this worktree could confirm it
by execution.

No `- [ ]` items are recorded because no defect was found. If a later reviewer
disagrees with the hand-traced mutation analysis above, that would be the one
thing worth re-opening.
