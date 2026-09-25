# Security review — time-pegged-clock (issue #162)

Scope: the **security** dimension only. Diff read via
`git diff origin/main...HEAD` (three dots). Owner's decision comment on #162
(2026-09-25) read and treated as the scope; quoted in the hand-back.

## What was checked

- `arrival.rs`: `next_counter`, `clock_from_counters`, `exceeds_receive_window`
  and the removal of `ADVANCE_BOUND`, for overflow, wraparound, and whether the
  window can be bypassed or misjudged.
- `transport.rs`: `receive`'s six-check ordering (build/size → decode → verify
  → Stoa match → window → append), whether `now_ms` can be influenced by
  attacker-controlled input (message `timestamp`, the op's own `asserted_ms`),
  and whether a refusal ever partially mutates the log.
- `authoring.rs`: where `Authorship::asserted_ms` is populated (only
  `dialectica/rust-lib/src/lib.rs`'s `now_ms()`, never parsed from a request),
  and whether the counter peg gives a local caller a way to forge ordering
  through anything other than their own already-trusted signing key.
- `log/sqlite.rs`: the `as i64`/`as u64` round trip for counters ≥ 2^63, and
  that `clock()` folds in Rust rather than trusting SQL's signed `MAX`.
- Whether `transport::receive` is wired to any production call site yet (it is
  not — delivery is unwired, confirmed by grep and by `design.md`'s own
  Non-Goals; the window's remote-attacker surface is therefore not reachable
  from the network in this piece, only exercised by direct unit tests).
- The design.md Risk entries against the code, specifically the two scenarios
  issue #162 raised: (a) a chain of honest replies to a near-edge op cascading
  toward the window boundary on other peers, and (b) whether an over-bound
  counter is "always first" for `revision::current_version`,
  `moderation::resolve`, and the feed's `latestReply`. Both are worked through
  in Decisions 10–11 with the same conclusions I derived independently before
  reading them; nothing here contradicts the code.

## Mutation testing (all reverted; tree confirmed clean before commit)

Three mutations, each reverted immediately after observing the failure,
confirmed via `git status --short` / `git diff --stat` showing no diff before
committing this file:

1. Moved the `AheadOfTime` window check to run immediately after decode,
   ahead of signature verification and the Stoa-match check, in
   `transport.rs`. Caught: `another_failure_is_reported_ahead_of_the_window`
   failed (a tampered, unverifiable op was reported `AheadOfTime` instead of
   `FailsVerification`). This is the property CLAUDE.md's "never trust an
   inbound message" and "each refusal reported distinguishably" both name —
   confirmed enforced, not merely asserted in a comment.
2. Changed `exceeds_receive_window`'s `>` to `>=` (off-by-one at exactly one
   hour) in `arrival.rs`. Caught: 2 of 29 `arrival::tests` failed
   (`a_counter_exactly_one_hour_ahead_is_within_the_window`,
   `extreme_values_do_not_abort_the_window_check`).
3. Changed the field the window predicate is fed in `transport.rs` from
   `c.counter` to `c.asserted_ms` (i.e., judging admission on the unverified,
   author-chosen wall-clock field instead of the counter). Caught: 8 of 75
   `transport::tests` failed, including
   `the_decision_does_not_read_the_wall_clock_field` and
   `the_decision_does_not_read_the_timestamp_handed_in_with_a_message`. This is
   the mutation that would matter most here — reading the wall-clock would
   let an author's freely-chosen field decide admission, exactly the
   censorship-vector CLAUDE.md's security posture rules out — and it is
   caught decisively.

All three confirm the suite guards the properties the code's own comments
claim, rather than merely documenting them.

`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core` passes clean at HEAD (1168 passed in `dialectica-core`, 30 in
`end_to_end`, 0 in `dialectica`'s own harness since `lib.rs` is untestable
outside the builder by design). `nix build ./dialectica#lgx` succeeds.

## Findings

None. I found no reachable panic, no bypassable check, no place where
attacker-controlled or self-forgeable input reaches an index, length,
allocation, or ordering decision undefended, and no gap between what the code
does and what CLAUDE.md's two standing security rules ("never trust an inbound
message", "moderation must be authenticated") require. Specifically:

- `exceeds_receive_window` uses `saturating_sub`, never `+`, so no input
  (including `u64::MAX` at either argument) can panic it — matches
  design.md Decision 6 and is exercised at the extremes by
  `extreme_values_do_not_abort_the_window_check`.
- `next_counter` uses `saturating_add`, never wraps.
- `clock_from_counters` is `Iterator::max`, cannot panic on any input including
  an empty one (`unwrap_or(0)`).
- The window is judged **last**, after signature verification and the Stoa
  match, so a forged or misdirected op is never given the more specific
  `AheadOfTime` diagnosis in its place (confirmed by mutation, above).
- The window reads the counter alone, never the freely-forgeable wall-clock
  field, and never the transport's own `timestamp` (confirmed by mutation and
  by the two `the_decision_does_not_read_*` tests already in the suite).
- `now_ms` reaching `receive` is a parameter the (not-yet-written) adapter
  supplies from the host's own clock — nothing on the wire can set it — and
  `Authorship::asserted_ms` is likewise only ever populated from the adapter's
  `now_ms()`, never parsed from a request in `wire.rs`.
- The two scenarios issue #162 flagged as open ("worth checking") are
  addressed in design.md Decision 10 with a per-reader account
  (`revision::current_version` is exposed only to the author's own other
  devices; `moderation::resolve` requires authority to reach the comparison at
  all; the feed's `latestReply` lead is bounded to the one-hour window and
  requires the other peer not already hold the op) — I re-derived the same
  bounds independently before reading that section and found nothing it
  understates.
- The known-and-accepted cascade (an honest reply to a near-edge op can itself
  be judged `AheadOfTime` by a third peer whose clock lags) is in design.md's
  Risks, bounded there to "the milliseconds by which C is behind B, plus one
  per reply in the chain" — I checked that bound against the code (each
  `next_counter` step adds at most 1 relative to a held clock) and it holds.
  This is a deliberate, owner-accepted trade-off per the decision comment
  ("It deliberately reverses two recorded rules... The spec change must say so
  and why"), not an unrecorded gap, so it is not raised here as a defect.
- No new dependency was added by this piece.
