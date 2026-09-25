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
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

### 1. Make room: the receiving peer's time reaches `receive`

- [x] 1.1 `transport::receive` takes `now_ms: u64`, unused, with every call site
      passing the test constant `NOW_MS`. A refactor commit with no behaviour
      change. Verified by the suite passing with no test changed but the
      argument added.

### 2. The publish rule and the clock (`arrival.rs`, `authoring.rs`)

- [x] 2.1 `next_counter(clock, now_ms)` is `max(now_ms, clock.saturating_add(1))`.
      Verified by `arrival.rs`'s publish tests: the current time for an empty
      Stoa, the time for a clock behind it, one above a clock at or ahead of it,
      and saturation at `u64::MAX` for three current times.
- [x] 2.2 `clock_from_counters` is the maximum, or zero. `ADVANCE_BOUND` and its
      tests are removed. Verified by
      `the_clock_is_the_highest_counter_however_far_apart_the_counters_are`.
- [x] 2.3 `authoring::publish` pegs the counter to `Authorship::asserted_ms`.
      Verified by `a_first_op_in_a_stoa_carries_the_current_time`,
      `a_clock_behind_the_current_time_publishes_at_the_time`,
      `publishing_after_receiving_advances_past_what_was_received` (a received
      op ahead of the time), and
      `a_publish_at_the_maximum_representable_clock_saturates`.
- [x] 2.4 Issue #162 item 1: a reply carries the greater counter and the rule
      places it first, on both peers, over the real receive path. Verified by
      `a_reply_carries_a_greater_counter_than_the_post_and_the_rule_places_it_first`,
      which searches for a body whose ids would order the post first.
- [x] 2.5 An op signed ahead of the time leads only until the time passes it.
      Verified by `an_op_signed_ahead_of_the_time_leads_only_until_the_time_passes_it`,
      searched the same way after a fixed body turned out to coincide.
- [x] 2.6 `SqliteOpLog::clock` still folds in Rust, not SQL `MAX`. Verified by
      `the_clock_override_agrees_with_the_trait_default_it_replaces` (now pinned
      to `u64::MAX`) and `the_clock_survives_a_restart_and_a_rebuild_in_another_sequence`.
      Both fail under a SQL-ranked read, measured.

### 3. The receive window (`arrival.rs`, `transport.rs`)

- [x] 3.1 `RECEIVE_WINDOW_MS = 3_600_000`, pinned by
      `the_receive_window_is_pinned_to_one_hour`.
- [x] 3.2 `exceeds_receive_window(counter, now_ms)` with a saturating subtraction.
      Verified at both edges and at the extremes by `arrival.rs`'s window tests.
      `extreme_values_do_not_abort_the_window_check` panics under the `+` form,
      measured.
- [x] 3.3 `InboundRefusal::AheadOfTime { counter, now_ms }`, judged after the Stoa
      check and before the append. Verified by the `transport.rs` window
      section: the edge admitted, one millisecond past refused with the clock
      unchanged, `u64::MAX` refused while the peer still publishes, the past
      admitted, no counter admitted at any time, a second arrival admitted as a
      first, and the message timestamp and the wall-clock field each shown not
      to move the decision.
- [x] 3.4 The window is judged last. Verified by
      `another_failure_is_reported_ahead_of_the_window`, which fails when the
      check moves before the signature check, measured.
- [x] 3.5 The six refusals render distinctly. `every_refusal_variant` gains
      `AheadOfTime`, and `a_refusal_for_the_window_is_distinguishable_from_every_other`
      checks it on a refusal `receive` returned.
- [x] 3.6 **Satisfied by construction, no test:** the window runs on no rebuild,
      replay or restore path. `OpLog::append` and `OpLog::clock` take no time,
      so no such path has a `now_ms` to give the predicate. `design.md`
      Decision 4.

### 4. Text that stated the old rule as a reason

- [x] 4.1 Doc comments citing `ADVANCE_BOUND`, or "causal, not temporal" as the
      reason a counter is not a time, rewritten in `op.rs`, `asserted_time.rs`,
      `thread.rs`, `feed.rs`, `log/mod.rs`, `revision.rs`, `moderation.rs` and
      `wire.rs`. Verified by two greps over `dialectica/rust-lib`:
      `git grep -n -e ADVANCE_BOUND` finds only sentences that describe it as
      replaced, and `git grep -n -i -e "temporal"` finds no sentence that calls
      the counter not temporal. The first pass ran only the first grep, which
      is how `moderation.rs` was missed (architecture finding).
- [x] 4.2 Decision 10's hour of lead is recorded beside the two readers with a
      comparison worth exploiting: `revision.rs` on `current_version`, and
      `moderation.rs` on `resolve` (a moderator signing up to an hour ahead).
      Doc-only; no test can see it.

### 5. Gates

- [x] 5.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
      green; `cargo clippy … --all-targets -- -D warnings` clean on both crates.
- [x] 5.2 `nix build ./dialectica#lgx` green, which compiles the
      `cfg(logos_scaffold)` adapter. The adapter needed no change: publish
      already passes `now_ms()` as `asserted_ms`, and nothing in the adapter
      calls `receive` yet.
