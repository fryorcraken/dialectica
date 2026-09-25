# Correctness review — time-pegged-clock (issue #162)

Dimension reviewed: **correctness only** (this instance was not asked to cover
security, readability or architecture; those are separate reviewer passes).

## Scope confirmed

Read `gh issue view 162 --json body,comments`. The owner's decision comment
(2026-09-25) is the scope. First line quoted in the hand-back.

## Method

- Read the full diff `git diff origin/main...HEAD` (three dots) across every
  changed source file: `arrival.rs`, `authoring.rs`, `transport.rs`, `op.rs`,
  `wire.rs`, `feed.rs`, `revision.rs`, `thread.rs`, `asserted_time.rs`,
  `log/mod.rs`, `log/sqlite.rs`, and every spec delta under
  `openspec/changes/time-pegged-clock/specs/`, plus `design.md`.
- Ran the full test suite (`cargo test -p dialectica -p dialectica-core`):
  1198 tests passed, 0 failed.
- Applied two targeted mutations to the new logic and confirmed the suite
  catches each, then restored the source exactly (verified clean
  `git status --short` before committing this file):
  1. `arrival::exceeds_receive_window`: changed the boundary comparison from
     `>` to `>=`. Caught by
     `arrival::tests::a_counter_exactly_one_hour_ahead_is_within_the_window`
     and `arrival::tests::extreme_values_do_not_abort_the_window_check`
     (2 failures).
  2. `transport::receive`: swapped the order of the `StoaMismatch` check and
     the `AheadOfTime` window check (window checked first). Caught by
     `transport::tests::another_failure_is_reported_ahead_of_the_window` (a
     tampered-Stoa op whose counter is also beyond the window came back
     `AheadOfTime` instead of `StoaMismatch`).
- Ran `nix build ./dialectica#lgx` (the owner's scaffold-gated build): succeeded.

## Findings

No correctness defects found. Nothing to tick.

The implementation matches the owner's decision comment precisely:

- `next_counter(clock, now_ms) = now_ms.max(clock.saturating_add(1))` is
  exactly SDS's `max(timeNowInMs, current_lamport_timestamp + 1)`, saturating
  rather than wrapping (verified: `next_counter(u64::MAX, 0) == u64::MAX`, no
  panic).
- `exceeds_receive_window(counter, now_ms) = counter.saturating_sub(now_ms) >
  RECEIVE_WINDOW_MS` is inclusive at exactly one hour and refuses one
  millisecond past it — matches "more than one hour ahead... discarded" and
  "no lower bound" (a counter at or below `now_ms` always saturates to 0 and
  is never refused, however old).
- `clock_from_counters` is a plain `max()`, a function of the set and not the
  arrival sequence — checked against the SQLite backend's `as i64` cast
  round-trip (`row as u64` after `i64` storage correctly reverses two's
  complement for a `u64::MAX` counter stored as `-1`; confirmed by reading the
  `clock()` implementation directly, not just its doc comment).
- The window is checked in `transport::receive` only, last among the six
  boundary checks (after size, decode, signature, Stoa match), and is the only
  function with `now_ms` in its signature — `OpLog::append` and `OpLog::clock`
  take no time, so rebuild/replay/restore paths cannot reach the window, which
  is what the tests
  `the_clock_survives_a_restart_and_a_rebuild_in_another_sequence` and
  `a_publish_at_the_maximum_representable_clock_saturates` rely on.
- Item 1 from the issue (the `op-ordering` self-contradiction) is fixed: the
  replacement scenario says "the ordering rule places the reply before the
  post", consistent with descending-counter-first, and is pinned by
  `authoring::tests::a_reply_carries_a_greater_counter_than_the_post_and_the_rule_places_it_first`
  over the real `receive` boundary on both peers.
- Item 3 (the `thread-read` position wording) now reads "the position in the
  sequence the read returns" rather than "the position the ordering rule gives
  it", matching what `thread.rs` actually returns (root pinned first, replies
  reversed).
- The "worth checking" item (whether an over-bound/ahead-of-time counter is
  "always first" for `revision::current_version`, `moderation::resolve`, and
  the feed's `latestReply`) is answered in `design.md` Decision 10, and the
  answer — bounded lead of at most one hour, no code changes needed since none
  of the three readers implement their own comparison — checks out against
  `revision.rs`'s and `moderation.rs`'s `resolve` functions, both of which
  still just take `iter_target`'s first entry after their own filter.

Everything I tried to break (boundary arithmetic, check ordering, the SQLite
cast round-trip, saturation at `u64::MAX`, the "no lower bound" claim, the
counter-less/VERSION_1 escape hatch) held.

## One observation, not a defect (not boxed)

`moderation.rs`'s module doc (around line 101-107) still reads "A Lamport
counter is causal rather than temporal" without the fuller explanation added
to `revision.rs`, `feed.rs`, `thread.rs` and `log/mod.rs` ("raised above every
counter the author held... an author may sign up to an hour ahead..."). The
statement in `moderation.rs` is still true, and `design.md` Decision 10 records
the module needs no code change ("No reader has code of its own for it"), so
this is a documentation-consistency gap rather than a behavioural one — better
suited to the readability or design reviewer's pass, noted here only so it
isn't lost.
