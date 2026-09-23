# Architecture review — `reply-count`

Dimension covered: **architecture only** (correctness, security and
readability are separate reviewers' rows).

Scope reviewed: `git diff origin/main...HEAD` for
`dialectica/rust-lib/dialectica-core/src/feed.rs`,
`dialectica/rust-lib/dialectica-core/src/wire.rs`,
`dialectica/rust-lib/dialectica-core/tests/end_to_end.rs`, and
`openspec/changes/reply-count/{proposal,design}.md` and `specs/feed-read/spec.md`.

`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core` is green: 1046 + 30 tests pass. `cargo clippy` on both
packages with `-D warnings` is clean (the only warnings printed are in the
vendored, gitignored `logos-rust-sdk-src`, not part of this piece).

## What's clean

- **The fold is a genuinely separate concern, not a bolt-on.**
  `visible_replies_by_thread` is called once, before the head loop, over the
  same `entries` the head loop already holds (`log.iter_stoa` is called once
  and shared via a slice — no second store read). The head loop's only change
  is `replies: replies.remove(&id)`. This is a clean instance of "make the
  change easy, then make the easy change": the design doc records two
  rejected alternatives (a thread-read per row; a projection updated on
  append) with real, stated reasons (cost class; moderation resolved-on-read
  semantics), rather than picking the fold shape by default.
- **The type-level guard is honored by every consumer.** `Replies` makes a
  count of zero alongside a `latest` unrepresentable
  (`count: NonZeroUsize`), and I confirmed by grep that `wire.rs` never
  reaches into `FeedRow.replies` directly — it only calls the `reply_count()`
  / `latest_reply()` accessors. A field this well-encapsulated can't be
  bypassed by a future call site the way two independent fields could be.
- **The ordering rule is honored by construction, not by a second
  comparison.** `visible_replies_by_thread` takes the first visible reply
  the fold meets (`or_insert_with`) rather than comparing counters itself.
  `feed.rs`'s own header states the reason: `feed.rs`, `thread.rs`,
  `revision.rs` and `moderation.rs` all avoid writing a second ordering rule,
  because two orders that disagree produce no error anywhere. I mutated this
  (see Measured, below) and the tests catch a violation, so this isn't just
  an assertion in a doc comment.
- **The scope is honestly bounded.** `proposal.md` and `design.md` both
  explicitly defer the `active` ordering and hand the "count reads as a
  total" obligation to whichever change first renders it
  (`feed-view`), rather than solving problems this change doesn't need to
  solve. `moderation::resolve` (checked directly) is an O(bindings-on-target)
  lookup via `iter_target`, not a chain walk, so it does not compound the
  quadratic-in-depth cost the design doc flags for `thread_of` — the design's
  cost analysis is accurate, not just asserted.
- **The failure-mode extension matches the existing pattern rather than
  inventing a new one.** A store failure met while folding *any* reply in the
  Stoa (even one under an off-page thread) fails the whole read. This is the
  same shape the pre-existing head loop already had (a head failure anywhere
  fails the page), extended to replies, and is flagged with an explicit
  `NO SPEC:` marker in both the test and `design.md`'s Risks section rather
  than being silently absorbed as new unstated behaviour.

## Findings

- [x] **`dev-writer`/`design-reviewer`** — `openspec/changes/reply-count/design.md` Decision 2, and `dialectica/rust-lib/dialectica-core/src/feed.rs:467-472`
      **Observation, not a defect in the shipped code:** design.md states outright that the claim "replacing `or_insert_with` with an overwrite would make the last reply met win, turning three named tests red" **"is a prediction and has not been measured"** — the implementing session's mutation was refused. I ran that exact mutation (changed `and_modify` to also overwrite `replies.latest = id.to_hex()` on every reply, restoring it immediately after) and it turns **6** tests red, not the 3 design.md names: `a_hidden_reply_is_neither_counted_nor_latest`, `a_hidden_threads_row_counts_its_visible_replies`, `a_reply_carrying_a_far_future_asserted_time_is_not_thereby_latest`, `a_revision_does_not_move_a_reply_or_change_the_id_reported`, `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`, and `the_latest_reply_is_the_one_the_ordering_rule_places_first` (41 passed, 6 failed under this mutation, of the `feed::tests` module). This is good news for the code — the design's central "first-met-wins, no comparison" property is well covered by tests, better covered than the doc claims — but `design.md` Decision 2's "What breaks without it" paragraph is now a stale, incomplete list once this review lands, and it says plainly that the claim was unmeasured. Either the design-reviewer or a follow-up should update that paragraph with the measured list (or a pointer to this finding) so the doc stops describing a prediction as still open. Severity: low — documentation staleness, not a code defect. **Mutation was restored**; `git status --short` and `git diff --stat` on `feed.rs` are both empty.
      **Fixed** in the commit "Record the measured first-met-wins mutation in design.md Decision 2". I re-ran the mutation myself before writing it down, over the whole `-p dialectica -p dialectica-core` suite rather than `feed::` alone. It turned exactly the six tests named here red (1040 + 30 passed, 6 failed), and none in `wire.rs` or `end_to_end.rs`. Decision 2's "What breaks without it" now lists those six as measured, and the "prediction" hedge is gone. The code is unchanged, and the tree was clean after restoring.

## Not evaluated further / abandoned per instructions

- `cargo mutants --file dialectica-core/src/feed.rs` was started scoped to
  this file (`-p dialectica-core --timeout 60`) but produced no output after
  more than 5 minutes (past the "abandon if it runs past a couple of
  minutes" guidance) and was killed. It appears to have been rebuilding from
  a cold `mutants.out` baseline rather than reusing the incremental build
  already primed by `cargo test`/`cargo clippy` in this tree. I did not
  retry it. The one high-value mutation on this file's changed logic (the
  first-met-wins fold, above) was instead checked by hand and is reported
  above with a measured result.

## Dependencies

No new dependency was introduced by this change (only `std::collections::HashMap`,
`std::num::NonZeroUsize`, and existing in-crate modules `crate::thread::thread_of`
and `crate::moderation::resolve` are newly imported into `feed.rs`).

## CI gates

No file was moved or renamed by this change, and no test directory's location
changed, so I see no risk to a CI gate that derives expectations from source
layout (`.github/workflows/ci.yml`'s directory-based gates, per CLAUDE.md's
own note about `dialectica-core`'s `fmt` gap, are unaffected — this change
doesn't touch that gap either way).
