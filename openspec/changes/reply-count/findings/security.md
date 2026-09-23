# Security review — `reply-count`

Scope: security dimension only, per dispatch. Diff reviewed with
`git diff origin/main...HEAD` (three dots) against
`dialectica/rust-lib/dialectica-core/src/feed.rs`, `wire.rs`,
`tests/end_to_end.rs`, and the `openspec/changes/reply-count/` spec/design/
proposal/tasks documents.

Worktree: confirmed before mutating — `pwd` resolved under
`.claude/worktrees/agent-afa93a099eff4a2e2`, and
`git rev-parse --abbrev-ref HEAD` returned `worktree-agent-afa93a099eff4a2e2`,
not a `piece/*` branch. Safe to mutate.

SDK symlink: `ln -sfn /nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src dialectica/logos-rust-sdk-src`
succeeded without refusal; `cargo test` and `cargo mutants` both ran against a
built tree.

## What this change touches, security-wise

`visible_replies_by_thread` folds over every op in the Stoa to compute, per
thread, a reply count and a "latest reply" op id. Both values feed the wire
via `replyCount`/`latestReply`. The two properties CLAUDE.md calls out —
untrusted peer input, and authenticated/authorised moderation — are exactly
what this fold has to get right: a reply's membership must come from the
verified parent chain (never the self-reported `thread` field), and a
reply's hidden state must come from `moderation::resolve` (which itself
checks `moderators.authorises`), never from an unauthenticated or unauthorised
signal.

## Mutations attempted

Per the brief, I re-attempted the two mutations the tester's classifier
refused, plus one more from `design.md`'s own "prediction, not measured"
admission. **None of the three was refused for me** — all went through as
ordinary `Edit` tool calls (not classifier-gated Bash), so I cannot corroborate
the tester's refusal from my own session; I can only report what happened when
each mutation was actually applied and run.

1. **Removed the `moderation::resolve(...).is_hidden()` check** in
   `visible_replies_by_thread` (`feed.rs`, around line 463). Ran
   `cargo test -p dialectica-core feed::` — **6 tests failed**:
   `a_hidden_reply_is_neither_counted_nor_latest`,
   `a_reply_beneath_a_hidden_reply_is_counted`,
   `a_restored_reply_is_counted_again`,
   `a_thread_whose_only_reply_is_hidden_reports_zero_and_no_latest`,
   `the_count_agrees_with_the_thread_read`,
   `the_include_hidden_flag_changes_no_rows_reply_fields`.
   Restored; `git diff HEAD -- feed.rs` empty afterward.

2. **Read the reply's own `thread` field instead of calling `thread_of`** for
   membership (replaced the `thread_of(log, &id)?` call with a match on
   `entry.op.op.kind`'s `thread` field). Ran the same test target — **6 tests
   failed**: `a_post_is_counted_by_its_parent_chain_and_never_by_its_thread_field`,
   `a_post_whose_parent_is_not_held_is_counted_once_the_parent_arrives`,
   `a_post_in_a_parent_cycle_is_counted_under_no_thread`,
   `a_reply_beneath_a_hidden_reply_is_counted`,
   `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`,
   `the_count_agrees_with_the_thread_read`. Restored; confirmed clean.

3. **Last-write-wins instead of first-met-wins** for `latest` (changed
   `.and_modify(...)` to also overwrite `replies.latest = id.to_hex()` on every
   later reply, not just the first). `design.md` decision 2 states this exact
   mutation "was refused in the implementing session" and that the claim it
   would be caught "is a prediction and has not been measured" — attributing
   the actual measurement to the tester's run. I ran it directly: **6 tests
   failed**: `a_hidden_reply_is_neither_counted_nor_latest`,
   `a_hidden_threads_row_counts_its_visible_replies`,
   `a_reply_carrying_a_far_future_asserted_time_is_not_thereby_latest`,
   `a_revision_does_not_move_a_reply_or_change_the_id_reported`,
   `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`,
   `the_latest_reply_is_the_one_the_ordering_rule_places_first`. Restored;
   confirmed clean.

All three mutations sit on the authenticity/moderation boundary this
dimension owns, and all three are caught by the existing suite when actually
run. I did not find a way to reproduce the classifier refusal the tester
reported — worth noting for whoever reconciles the two reports, but it isn't
itself a code defect.

`cargo mutants --file src/feed.rs` (both from the workspace manifest and the
package manifest) reported "0 mutants found" under the path filter — the
workspace layout (`dialectica/rust-lib/Cargo.toml` as a virtual workspace over
`dialectica-core`) didn't resolve the `--file` filter in the time budgeted for
this pass, so I relied on the three manual mutations above rather than a full
mutants run.

Full suite after final restoration: `cargo test --manifest-path
dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` — 1046 + 30
passed, 0 failed.

## Findings

- [ ] **`spec-writer` / `dev-writer`** — `openspec/changes/reply-count/design.md` decision 2 — the claim that the last-write-wins mutation "has not been measured" and rests only on the tester's session is itself now contradicted by direct measurement in this review.
      **Scenario:** Applying the mutation described (overwrite `replies.latest` on every reply met, not just the first) and running `cargo test -p dialectica-core feed::` fails 6 tests: `a_hidden_reply_is_neither_counted_nor_latest`, `a_hidden_threads_row_counts_its_visible_replies`, `a_reply_carrying_a_far_future_asserted_time_is_not_thereby_latest`, `a_revision_does_not_move_a_reply_or_change_the_id_reported`, `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`, `the_latest_reply_is_the_one_the_ordering_rule_places_first`. This is not a security defect in the shipped code — the fold is correct and the tests do catch the mutation — but `design.md` currently understates its own evidence, which matters here because CLAUDE.md's own rule is "write down only what a command cannot tell you," and this is now a claim a command (this review) has answered differently than the doc states. Low severity; a documentation-accuracy finding, not a vulnerability. Flagging for whoever owns `design.md` rather than ticking it as mine, since design-doc accuracy is the design-reviewer's lane — recording here only because I generated the measurement while probing the same code path this dimension covers.
      **Outcome (spec-writer): nothing to change on the spec side; box left open for `dev-writer`.** Neither `proposal.md` nor `specs/feed-read/spec.md` claims anything about this mutation. The understated claim is only in `design.md` Decision 2, which the spec-writer does not write. The box stays unticked so the `dev-writer` does not read the finding as closed.

No other findings. Specifically clean, on this dimension:

- **Membership boundary.** `visible_replies_by_thread` calls `thread_of` (never reads the reply's own `thread` field) and the head loop's existing `entry.op.verify()` gate is mirrored inside the fold (`if !entry.op.verify() { continue; }` before any field is read). Forged replies, replies claiming a thread via the `thread` field alone, cross-Stoa replies (folded only over `log.iter_stoa(stoa)`'s own entries), and parent cycles (via `thread_of`'s visited-set termination, reused rather than reimplemented) are all excluded by construction and pinned by dedicated tests that this review confirmed fail under the corresponding mutation.
- **Moderation boundary.** Hidden-state comes from `crate::moderation::resolve`, which internally requires `moderators.authorises(e)` — an unauthorised (non-moderator) hide is confirmed inert by `a_non_moderators_hide_removes_nothing_from_the_count`, and the `Moderators` set passed into `visible_replies_by_thread` is the same one `list_threads` receives from its caller (traced through to `wire.rs::list_threads_inner`, which pairs it with the genesis address before calling into `feed::list_threads` — that pairing check is pre-existing and untouched by this diff).
- **`include_hidden` cannot widen the fold.** The flag is deliberately not threaded into `visible_replies_by_thread` at all (confirmed by reading the call site) — it can only change which *rows* appear, never what a row counts, closing off a path where an attacker-controlled or caller-controlled flag might have let hidden replies inflate a count.
- **No panics reachable from peer bytes in the new code.** The one place `wire.rs` could have unwrapped (`feed_page_json`'s `latestReply` insertion) uses `if let (Some(map), Some(latest)) = (...)` rather than `.unwrap()`, matching the documented "a panic aborts the module process" concern. All `unwrap()`/`expect()` introduced by this diff are confined to test code (verified by grepping the diff for `unwrap(`/`expect(`/panic-prone indexing and checking each hit's location).
- **Store-failure handling.** A storage error encountered while folding (even for a reply whose thread isn't on the requested page) propagates as `Err`, never silently becomes a zero count or a missing row — confirmed by reading `a_store_failure_while_counting_is_an_error_and_not_a_zero` and `a_store_failure_on_a_reply_off_the_page_fails_the_page`, and this matches the spec's explicit requirement.
- **Wire shape.** `replyCount` is always present (never omitted, so it can't be misread as "not computed"); `latestReply` is omitted rather than sent as `null`, per the wire contract's stated rule for a field with no meaning. Confirmed present/absent behavior against `feed_page_json`'s code and the two key-set tests.

## Not tested here (out of scope for this dimension or out of budget)

- The DoS/cost profile of folding the *whole* Stoa on every feed read (`design.md` decision 5 discusses this as a known, accepted cost — it is a resource-exhaustion/availability question rather than an authenticity/authorization one, and better suited to a correctness or architecture pass if it needs revisiting).
- `cargo mutants` automated run — attempted, returned "0 mutants found" against `--file src/feed.rs` under both manifest paths tried; not pursued further given the time budget. The three manual mutations above substitute for it on the specific lines this dimension cares most about.
