# Design review: reply-count

Reviewed `design.md`'s Decisions against `feed.rs`, `wire.rs`, `thread.rs` and
`moderation.rs` on `piece/100-reply-count` (diffed `origin/main...HEAD`), and
against GitHub issue #100 (read fresh via `gh issue view 100`).

Overall the Decisions section is in good shape: each of the seven decisions is
implemented as recorded, each names what was chosen, the constraint, the
rejected alternatives and what it costs, and the code was read directly rather
than trusted — nothing found contradicts what is written down. Two smaller
items below are worth flipping before merge, and one is a plain observation.

- [x] **`dev-writer`** — Decision 2's mutation evidence is honestly marked
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
      **Fixed** in the commit "Record the measured first-met-wins mutation in
      design.md Decision 2". The mutation measured is `and_modify` also
      overwriting `latest`, not an overwriting `insert`. An overwriting insert
      would also reset the count to one, so it would test the count as well as
      the latest-reply rule. Run over the whole suite, it turned exactly six
      tests red: the three predicted and three more. Decision 2 lists all six
      as measured. The architecture, correctness and security reviews each got
      the same six independently.

- [x] **`dev-writer`** — Decision 5's `NO SPEC:` marker is accurate but its
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
      **Fixed**, which closes the question rather than handing it off. The
      spec-writer contracted the behaviour in `4eaa19c`. `feed-read`'s
      requirement *"Computing the reply fields fails as an error and never
      aborts"* now requires the read to fail whichever reply in the Stoa the
      failure is met on, and `proposal.md`'s What Changes lists it. The commit
      "Align the off-page store-failure marker and Decision 5 with feed-read"
      does the rest. It removes the `NO SPEC:` marker from
      `a_store_failure_on_a_reply_off_the_page_fails_the_page` and replaces it
      with a pointer to the requirement. It rewrites Decision 5 and the Risks
      entry to say the behaviour is required rather than open, and records why
      page-scoping was refused: a failure inside `thread_of` is met before the
      reply's thread is known. Task 2.2 no longer calls the scope unspecified.
      No new handoff is needed. The spec's new scenario for a hidden thread's
      reply has no test yet, and that is the `tester`'s.

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
