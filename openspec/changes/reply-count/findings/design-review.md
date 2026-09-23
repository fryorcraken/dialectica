# Design review: reply-count

Reviewed `design.md`'s Decisions against `feed.rs`, `wire.rs`, `thread.rs` and
`moderation.rs` on `piece/100-reply-count` (diffed `origin/main...HEAD`), and
against GitHub issue #100 (read fresh via `gh issue view 100`).

Overall the Decisions section is in good shape: each of the seven decisions is
implemented as recorded, each names what was chosen, the constraint, the
rejected alternatives and what it costs, and the code was read directly rather
than trusted — nothing found contradicts what is written down. Two smaller
items below are worth flipping before merge, and one is a plain observation.

- [ ] **`dev-writer`** — Decision 2's mutation evidence is honestly marked
      unmeasured, and still is after the tester's last commit. Decision 2 says
      replacing `or_insert_with` with an overwriting insert "should turn
      `the_latest_reply_is_the_one_the_ordering_rule_places_first`,
      `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest` and
      `a_hidden_reply_is_neither_counted_nor_latest` red. **This is a prediction
      and has not been measured.**" The branch's last commit
      (`228dd89`, "Add reply-count tests for cycles, cross-Stoa forgery,
      asserted time and divergent peer copies") added coverage for other cases
      but not this mutation. **Verified:** `git grep -n -F -e "mutation" --
      openspec/changes/reply-count/` finds only the one hedge in `design.md`;
      no findings file records the mutation having been run since. The
      guidance this project holds itself to is explicit that a guard's entry
      should carry "removing this turns exactly these tests red" as measured
      fact, not prediction — this is the shape of decision that gets
      re-litigated or silently broken by a future refactor because nobody
      checked whether the tests actually catch it. Low cost to close: swap
      `or_insert_with` for `.entry(root).or_insert(...)`-with-overwrite for a
      moment, run the three named tests, and record which went red (or didn't).

- [ ] **`dev-writer`** — Decision 5's `NO SPEC:` marker is accurate but its
      companion spec-side obligation is still open, and design.md doesn't say
      who owns closing it. Decision 5 states plainly "It is still a choice the
      spec should either make or refuse," and Risks repeats "The spec-writer is
      to decide whether it stands." That's a legitimate deferral, not a defect
      — but as written it reads as though the obligation discharges itself.
      Since `feed-read`'s spec (`specs/feed-read/spec.md`) is *this change's
      own* new capability, not a pre-existing one being depended on, the
      obligation to decide is this change's to either resolve or explicitly
      hand off in `tasks.md`/`proposal.md`'s "what #91 should pick up" section
      — and right now neither proposal.md's "What #91 should pick up" list nor
      tasks.md names it. Recommend adding one line to proposal.md's handoff
      list (or tasks.md) naming this open question explicitly, so it isn't
      only findable by reading design.md's Risks section.

No issue-contradiction findings: issue #100's scope ("a fold, per thread, over
its replies... counts non-hidden replies, finds the most recent non-hidden
reply... wired into `listThreads`'s existing row shape") is met without
narrowing or widening what the issue asked for, and the issue's own reasoning
(membership via parent chain, not the `thread` field; moderation-resolved
state; the `active`-ordering motivation) is reproduced in Decision 1 and
Decision 6 nearly verbatim, correctly migrated into design.md rather than
left only in the issue.

The "found while implementing" note (the thread read orders a reply before the
reply it answers) is out-of-scope reasoning correctly *not* acted on in this
change, and correctly recorded as a defect for a separate change rather than
silently worked around — this is the right call given `design.md`'s own
argument that `the_count_agrees_with_the_thread_read` checks membership, not
position, so the defect doesn't propagate into this change's own tests.

Branch: `worktree-agent-a278d76fffddca77d`
