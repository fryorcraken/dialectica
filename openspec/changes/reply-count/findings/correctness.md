# Correctness review — `reply-count`

Scope: correctness only (one of six reviewers on this change; other dimensions
are covered elsewhere). Reviewed the three-dot diff `origin/main...HEAD` on
`piece/100-reply-count`, cross-checked against
`openspec/changes/reply-count/specs/feed-read/spec.md` and `design.md`.

## Method

- Read `feed.rs`'s new `visible_replies_by_thread` and the `list_threads`
  integration, `wire.rs`'s `feed_page_json` change, and the new/changed tests in
  `feed.rs`, `wire.rs` and `tests/end_to_end.rs`.
- Verified the two properties the design leans on directly in the surrounding
  code rather than taking the doc comment's word for it: `OpLog::iter_stoa`
  returns entries in `cmp_ops` order (`log/mod.rs`'s trait doc and
  `MemoryOpLog::sorted`-backed impl), and `thread::thread_of` verifies each op,
  checks `Entry::id()` against the walked id, and terminates on a per-walk
  `visited` set over any parent references a peer chose (`thread.rs:426-486`).
- Ran the full `dialectica`/`dialectica-core` suite as a baseline (1046 + 30
  tests, all green), then applied and reverted four targeted mutations to
  `feed.rs`, each followed by `cargo test -p dialectica-core feed::` and, for
  one, the full suite.
- Attempted `cargo mutants --file dialectica-core/src/feed.rs`; it did not
  finish within the time budget this task allows (it was still building/running
  past several minutes with no output), so it was abandoned per instructions
  rather than left to poll. The four hand-picked mutations below substitute for
  it and target exactly the properties the spec and design.md call out as
  load-bearing.

## Mutations tried

1. **"Latest wins" instead of "first wins."** Changed
   `and_modify` to also overwrite `replies.latest = id.to_hex()` on every later
   reply met, not just the count (`feed.rs` `visible_replies_by_thread`, the
   `by_thread.entry(root)...or_insert_with` block). This is exactly the
   mutation `design.md` Decision 2 says was *predicted* but "refused in the
   implementing session" and never actually measured, leaving the claim as an
   assertion rather than a checked fact.
   **Result:** 6 of 47 `feed::` tests failed (`a_hidden_reply_is_neither_counted_nor_latest`,
   `a_hidden_threads_row_counts_its_visible_replies`,
   `a_reply_carrying_a_far_future_asserted_time_is_not_thereby_latest`,
   `a_revision_does_not_move_a_reply_or_change_the_id_reported`,
   `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`,
   `the_latest_reply_is_the_one_the_ordering_rule_places_first`). This
   independently confirms design.md's prediction — the suite does catch it.
   Restored.
2. **Root posts (`parent: None`) counted as replies of their own thread.**
   Widened the fold's kind guard from `OpKind::Post { parent: Some(_), .. }` to
   any `OpKind::Post { .. }`.
   **Result:** 18 of 47 `feed::` tests failed. Restored.
3. **Hidden-reply check removed** (`moderation::resolve(...).is_hidden()` result
   discarded instead of `continue`-ing).
   **Result:** 6 of 47 `feed::` tests failed
   (`a_hidden_reply_is_neither_counted_nor_latest`,
   `a_reply_beneath_a_hidden_reply_is_counted`, `a_restored_reply_is_counted_again`,
   `a_thread_whose_only_reply_is_hidden_reports_zero_and_no_latest`,
   `the_count_agrees_with_the_thread_read`,
   `the_include_hidden_flag_changes_no_rows_reply_fields`). Restored.
4. **Removed the fold's own `entry.op.verify()` guard**, leaving only
   `thread_of`'s internal re-verification of the same op.
   **Result:** all 47 `feed::` tests still passed. This matches design.md
   Decision 7's own claim ("Removing the fold's check turns no test red,
   because `thread_of` refuses the forgery anyway") almost verbatim — the
   redundancy is honestly documented as defense-in-depth rather than
   load-bearing, so this is not a gap, just a confirmed non-finding. Restored.

All four mutations were reverted; `git status --short` and `git diff --stat`
show a clean tree, and a final full-suite run (`cargo test -p dialectica -p
dialectica-core`) passed (1046 + 30 tests) before this file was written.

No mutation attempt was refused by the permission classifier during this
review.

## Findings

No correctness defects found. Everything checked below is a clean-bill
observation, not a box to tick.

- **Membership and moderation are delegated, not re-implemented.**
  `visible_replies_by_thread` calls `thread::thread_of` for parent-chain
  membership and `moderation::resolve` for hidden state, exactly as
  `design.md` Decision 1 describes, rather than re-deriving either rule. This
  is the mechanism the spec's forgery/cross-Stoa/cycle/off-parent scenarios
  depend on, and it held under mutation (see #2 and #3 above).
- **"First met is latest" is correct given `iter_stoa`'s ordering contract.**
  `OpLog::iter_stoa`'s doc (`log/mod.rs:351`) and `MemoryOpLog`'s
  implementation both guarantee `cmp_ops` order, so `or_insert_with` recording
  only the first reply met per thread is sound, not an accidental artifact of
  test data. Confirmed both by reading the ordering contract and by mutation
  #1.
- **Cycle and dangling-parent termination is inherited correctly.**
  `thread_of`'s `visited` set is per-walk and its "compare `Entry::id()` to the
  id it was looked up under" check is exactly what makes the `WrongKeyLog`
  fixture's forged-key cycle test meaningful (a real `MemoryOpLog` cannot
  produce a cycle, since op ids are content hashes over the parent field).
  `visible_replies_by_thread` inherits this termination property by calling
  `thread_of` rather than walking chains itself, so it cannot hang or panic on
  adversarial parent graphs it did not itself have to be tested against
  directly.
- **`replyCount`/`latestReply` wire emission matches the spec's absent-vs-null-vs-zero
  rules.** `replyCount` is built unconditionally into the `json!` object
  (present and zero for a threadless row); `latestReply` is inserted only via
  `if let Some(latest) = row.latest_reply()`, so it is omitted rather than sent
  as `null` or `""`, matching spec lines 127 and 162-163. The `Option<Replies>`
  representation on `FeedRow` (with a `NonZeroUsize` count) makes "count of
  zero with a latest reply present" and "count of N>0 with no latest reply"
  unrepresentable by construction, which is stronger than a check at the wire
  boundary.
- **Design.md's "not measured" admission is worth flagging to whoever tracks
  design-review status, even though it is not a correctness defect.** Decision
  2 states a mutation prediction was never run because the implementing
  session's mutation attempt was refused by the permission classifier, and
  that "the tester's run is what establishes it." My mutation #1 above is that
  measurement (6/47 failures) and it confirms the prediction. This review's
  scope is correctness, not design-doc bookkeeping, so I am not filing this as
  a correctness defect — flagging it here only so the design reviewer (or the
  runner) knows the gap has now actually been closed and can update or drop
  that caveat rather than re-attempt the same mutation.

## What I did not check

**Update:** the `cargo mutants --file dialectica-core/src/feed.rs` run above was
abandoned as not finished within budget, but it completed shortly afterward in
the background: **19 mutants tested in 8m: 17 caught, 2 unviable, 0 missed.**
Zero survivors is consistent with the four hand-picked mutations above finding
no gap — `cargo mutants` found nothing beyond what manual testing already
covered on this file. "Unviable" means those two mutants failed to compile, not
that they passed undetected.

I did not attempt mutations in `wire.rs` beyond reading it, since the
`json!`-macro construction there is straightforward enough that manual reading
was conclusive (an object literal cannot fail `.as_object_mut()`, so the `if
let` guard is unreachable-dead defensive code, not a bug).
