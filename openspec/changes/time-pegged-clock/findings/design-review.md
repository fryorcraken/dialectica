# Design review — time-pegged-clock

Read against `openspec/changes/time-pegged-clock/design.md`, issue #162
(`gh issue view 162 --json body,comments`), and the code on
`git diff origin/main...HEAD`.

## Summary

`design.md` is in good shape. No finding rises to blocking. Every recorded
decision was checked against the code it claims to describe, and each one
holds:

- **Decision 1** (peg to time, three superseded alternatives) matches the
  owner's issue comment verbatim in scope and reasoning, and the rejected
  options (write down current behaviour / demote over-bound ops / refuse by
  arrival order) are the same three the comment says were superseded.
- **Decision 2** (time reaches `core` as a parameter, not a field) —
  confirmed: `transport::receive(message, channels, log, now_ms: u64)`
  (`transport.rs:500`) takes `now_ms` as an argument, and `InboundMessage`
  carries only the delivery `timestamp`, never a second time field.
- **Decision 4** (window checked only at `transport::receive`, never on
  rebuild/replay/restore) — confirmed structurally: `OpLog::append` (`log/mod.rs:349`,
  `log/sqlite.rs:716`) and `OpLog::clock` (`log/mod.rs:441`, `log/sqlite.rs:859`)
  take no time parameter, so `exceeds_receive_window` (`arrival.rs:237`) has
  only one caller, `transport::receive` (`arrival.rs` doc, confirmed by
  `git grep`).
- **Decision 6** (`saturating_sub` to avoid overflow) and **Decision 7** (clock
  is a `max`, SQLite override still folds in Rust because of the `as i64`
  cast) — both confirmed against `arrival.rs` and `log/sqlite.rs`, including
  the cited tests (`the_clock_survives_a_restart_and_a_rebuild_in_another_sequence`,
  `the_clock_override_agrees_with_the_trait_default_it_replaces`), which exist
  and pass.
- **Decision 9** (window checked last, its own refusal variant) — confirmed:
  `transport.rs:540-544` checks `AheadOfTime` after Stoa-match and before
  append, and `another_failure_is_reported_ahead_of_the_window` exists.
- **Decision 10** (three readers of the ordering rule's first entry) — checked
  against `revision.rs`, `moderation.rs`, `feed.rs`; the analysis matches what
  the code does (only a comment changed in `revision.rs`, consistent with the
  claim that no reader has code of its own for this).
- **Decision 11** (receiver whose own clock is wrong, both directions) is the
  most load-bearing of the three topics the owner named for this review, and
  it is worked through quantitatively for both a slow and a fast receiver,
  including the "no ratchet" argument for why the reference is the time and
  not the clock. Nothing in the code contradicts it.

`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
passes in full (1168 + 30 tests), including every test design.md cites as
mutation evidence for its guard decisions (checked by name with `git grep`).
`nix build ./dialectica#lgx` succeeds.

I read issue #162's comments (not just the body) and the owner's decision
comment of 2026-09-25 governed this review's scope, as instructed. No
departure from that comment was found in `design.md` or the code.

No checkboxes are raised. If a future reader wants one place to start
re-verifying this piece, Decision 11's "no ratchet" claim and Decision 4's
"nothing can move the check into `append` without giving it a time" are the
two load-bearing arguments — both are checked above and both hold.
