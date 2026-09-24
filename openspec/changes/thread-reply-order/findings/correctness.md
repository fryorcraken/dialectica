# Correctness review — thread-reply-order (#147, PR #159)

Scope: correctness only, per dispatch. Diffed `origin/main...HEAD`; touched files
are `dialectica/rust-lib/dialectica-core/src/thread.rs`,
`dialectica/rust-lib/dialectica-core/tests/end_to_end.rs`,
`openspec/specs/thread-read/spec.md`, and the change's own openspec files.

## What I did

- Read the full diff of `thread.rs`, `end_to_end.rs`, and
  `openspec/specs/thread-read/spec.md` against `origin/main`, and the new
  `specs/thread-read/spec.md` and `design.md` under the change.
- Worked out `cmp_ops`'s (`arrival.rs`, untouched by this diff) actual ordering
  by hand — for equal counters it breaks ties by **ascending** `OpId`, and for
  two counter-less ops it also falls back to ascending `OpId`, with every
  counted op sorting ahead of every counter-less one. Reversing that whole
  sequence therefore has to produce: counter-less replies first (descending id
  among themselves), then counted replies ascending by counter (descending id
  on a tie). That is exactly what `read_thread`'s new backward walk
  (`log.iter_stoa(stoa)?.into_iter().rev()`, line 698) produces, and exactly
  what the new tests assert — verified against the code line by line rather
  than taken from the module's own comments.
- Ran the full suite: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml
  -p dialectica -p dialectica-core` — 1146 + 30 passed, 0 failed (plus 0
  doc-tests either package declares). Needed to stage
  `dialectica/logos-rust-sdk-src` first via `nix build
  "github:logos-co/logos-module-builder/9f420c2901e35a16ba8fc77383e796480000a1d2#rust-sdk-src"
  -o dialectica/logos-rust-sdk-src`, using the rev read out of
  `dialectica/flake.lock`'s root → `logos-module-builder` node, per the CI
  job's own method.
- Mutation-tested the core change directly (each mutation applied with `Edit`,
  verified, then reverted with `Edit`; `git diff -- .../thread.rs` confirmed
  byte-identical restoration before commit):
  - Reverting the fix (`.into_iter()` with no `.rev()`, i.e. the pre-#147
    behaviour) turns 7 tests red: `a_reply_orders_after_the_reply_it_answers`,
    `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`,
    `a_reply_carrying_a_lower_counter_than_the_reply_it_answers_comes_before_it`,
    `replies_with_equal_counters_are_reversed_rather_than_re_sorted`,
    `the_lowest_counter_leads_the_replies_and_the_highest_ends_them`,
    `the_root_is_the_first_item_whatever_the_logs_order_puts_first`,
    `the_sequence_follows_the_counters_and_not_the_asserted_times`. Strong,
    redundant coverage of the one-line behaviour change.
  - Reproduced `design.md` Decision 1's specific "measured" claim myself
    (replacing the backward walk with
    `sort_by_key(|e| (e.op.op.clock.map(|c| c.counter), e.id()))`, the ascending
    re-sort with the ascending-id tiebreak the design doc says is the wrong
    obvious fix): this turns **exactly** the three named tests red
    (`replies_with_equal_counters_are_reversed_rather_than_re_sorted`,
    `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one`,
    `the_root_is_the_first_item_whatever_the_logs_order_puts_first`) and leaves
    `a_reply_orders_after_the_reply_it_answers` and
    `the_lowest_counter_leads_the_replies_and_the_highest_ends_them` green, as
    claimed. The design doc's claim checks out exactly, not approximately.
  - Spot-checked that `ADVANCE_BOUND` (cited in the module doc and in
    `design.md`'s Risks as the reason an answer's counter can come out lower
    than its parent's) is a real, existing constant in `arrival.rs` and not an
    invented name.
- Read every scenario in `openspec/changes/thread-reply-order/specs/thread-read/spec.md`
  against its corresponding test in `thread.rs`, including the "root carrying
  no counter" vs. "root carrying a counter" pair inside
  `a_reply_carrying_no_counter_precedes_the_replies_that_carry_one` — both
  loop iterations are exercised (confirmed via the mutation run above, which
  failed on the first iteration and would have failed on the second had the
  first passed).
- Kicked off `nix build ./dialectica#lgx` as the build gate; see below for the
  result once it completed.

## Findings

No correctness defects found. Nothing to tick.

The one-line behavioural change (`.rev()` on an already-sorted, already
peer-convergent sequence) is mathematically sound against `cmp_ops`'s actual
tie-break rules, matches every scenario the amended spec states (including the
two edge cases — equal counters, and a reply's counter coming out lower than
the reply it answers when the answered reply's counter exceeded
`ADVANCE_BOUND` — that a shallower fix would have missed), and is backed by
tests that fail concretely and specifically under both the reverted fix and
the "obvious but wrong" ascending re-sort alternative the design doc
considered and rejected. The arithmetic in the surrounding pagination
(`start`/`end`/`has_more`, using `saturating_mul`/`saturating_add`/`min`) is
untouched by this diff and was not re-derived here.

`nix build ./dialectica#lgx` completed with exit code 0: the build gate
passes.
