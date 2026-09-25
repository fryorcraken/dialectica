# spec-test review — time-pegged-clock

Scope confirmed: read issue #162 with `gh issue view 162 --json body,comments`.
The owner's decision comment
(https://github.com/fryorcraken/dialectica/issues/162#issuecomment-5826096926)
is the scope of this piece; its first line is:

> **Decision (owner, 2026-09-25): peg the Lamport counter to wall-clock time, as
> SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`, lines 148-155 and
> 184-192).**

## Findings

- [ ] **`spec-writer`** — `authoring.rs:1712`'s `NO SPEC:` marker: the spec
      requires the counter to take "the peer's current Unix time in
      milliseconds" and the wall-clock field to carry "the author's assertion",
      but never says whether a publish must read the clock once for both or may
      read it twice (e.g. an implementation that read the wall-clock a moment
      after the counter, under load). The code pegs both fields to one reading
      (`design.md` Decision 3, per the proposal's own dev-writer note), and
      `one_reading_of_the_time_signs_both_clock_fields` pins that choice.
      **Scenario:** a future implementation samples the clock twice; nothing in
      `op-ordering` or `op-format` says that is wrong, even though it would let
      the wall-clock field (still meant to be "the author's assertion" of the
      same moment the counter reflects) silently drift from the counter's basis
      by however long the two reads are apart. Worth a decision on whether the
      spec should require one reading, or is content to leave it to the
      implementation as it does today.

- [ ] **`spec-writer`** — `transport.rs:2600`'s `NO SPEC:` marker: the spec says
      nothing about an op this peer **already holds** arriving again once this
      peer's own clock has been set backward past the op's counter minus the
      window. The code refuses it as `AheadOfTime` (the window is judged before
      the log is consulted for an `AlreadyPresent` report), pinned by
      `a_held_op_arriving_again_beyond_the_window_is_refused_rather_than_reported_as_held`.
      **Scenario:** a peer's system clock is corrected backward by more than an
      hour after it has admitted an op; every later re-arrival of that same op
      is now reported as a fresh refusal rather than "already held", which is a
      different observable outcome (and a different log message) from every
      other already-held op. The spec's "A refusal leaves nothing behind that
      affects a later arrival" and "the same op arriving again... admitted
      exactly as an op arriving for the first time would be" clauses are silent
      on what "the same op, but already held" should report. Worth a decision.

## What was checked and found clean

- **Coverage.** Walked every ADDED/MODIFIED scenario in
  `op-ordering`, `op-transport`, `op-format`, `thread-read`, `feed-view` and
  `post-revision` against the `#[cfg(test)]` modules in `arrival.rs`,
  `authoring.rs`, `transport.rs`, `thread.rs`, `revision.rs`, `feed.rs`,
  `log/sqlite.rs` and `wire.rs` (via `git diff origin/main...HEAD`, three
  dots). Every scenario has a test, at the right layer — `feed-view` and
  `post-revision`'s deltas are pure rationale corrections with unchanged
  scenario text, so no new test was owed there, and `feed-view`'s label text is
  a UI-layer concern this diff never touches (confirmed: no `dialectica-ui`
  file appears in the diff), so there is no core/UI layer mismatch to flag.
- **thread-read's "AND this holds when the first reply's counter was ahead of
  the second peer's current time" clause** is not exercised by one single
  end-to-end test, but is pinned by composition: `authoring.rs`'s
  `a_reply_to_an_op_ahead_of_the_time_still_carries_the_greater_counter` proves
  the counter comes out greater in exactly that case, and `thread.rs`'s
  `a_reply_orders_after_the_reply_it_answers` proves any greater counter is
  placed after in the returned sequence, for a reason `thread.rs` states
  explicitly: it takes counters as given and "the read cannot tell the two
  paths apart, and is not supposed to." This is deliberate modularity, not an
  oversight, so no checkbox for it.
- **Self-consistency.** The contradiction issue #162 opened with — "A reply
  orders after the post it replies to" placing the post first under a
  descending-counter rule — is fixed: the new scenario "A reply carries a
  greater counter than the post it replies to, and the rule places it first"
  states the rule's own placement correctly, and `thread-read` correctly
  reverses it. No other internal contradiction found on a full read of all six
  delta files.
- **Scope staleness.** Checked the spec against the decision comment, which
  supersedes the issue body's "What this needs". The spec implements exactly
  the decision's publish/receive/no-lower-bound/re-delivery rules, keeps issue
  items 1 and 3 in scope, and does not touch the op format, consistent with
  "Out of scope, per the owner's decision: changing the op format... migrating
  stored ops."

## Mutation testing (part 2)

Two mutations, both restored; `git status --short` shows nothing outstanding
beyond this findings file.

1. **`exceeds_receive_window`, `dialectica/rust-lib/dialectica-core/src/arrival.rs`**
   — changed `>` to `>=` in `counter.saturating_sub(now_ms) > RECEIVE_WINDOW_MS`.
   This is the security-critical boundary of the receive window (op-ordering's
   defence against a forged-counter censorship-vector reversal), and its exact
   edge is spec-mandated ("MUST NOT refuse an op whose counter exceeds the
   peer's current time by the window exactly"). **Caught**: both
   `arrival::tests::a_counter_exactly_one_hour_ahead_is_within_the_window` and
   `transport::tests::an_op_at_the_edge_of_the_window_is_admitted` failed
   (`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core an_op_at_the_edge_of_the_window_is_admitted`
   and the equivalent for the arrival test). Restored; `git diff` on the file
   is empty.
2. **Check ordering in `transport::receive`,
   `dialectica/rust-lib/dialectica-core/src/transport.rs`** — moved the receive
   window's judgement to run right after decoding, before the signature check,
   testing the tasks.md claim that `another_failure_is_reported_ahead_of_the_window`
   "fails when the check moves before the signature check". **Caught**: the
   test failed with `left: AheadOfTime { .. }` where `right: FailsVerification`
   was expected
   (`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core another_failure_is_reported_ahead_of_the_window`).
   Restored; `git diff` on the file is empty.

Neither mutation survived. Both are reported as a measurement, not a defect:
the tests that guard the window's edge and its position in the check order do
in fact fail when those properties break.

## Test run

Full suite green after restoring both mutations:
`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
— 1168 passed in `dialectica-core`, 30 passed in the `end_to_end` integration
test, 0 failed.
