# spec-test review: reply-count

Scope: `openspec/changes/reply-count/specs/feed-read/spec.md` against
`dialectica/rust-lib/dialectica-core/src/feed.rs` (`mod tests`, "The reply count
and the latest reply" section), `dialectica-core/src/wire.rs`'s feed-row tests,
and `dialectica-core/tests/end_to_end.rs`. Read per the role's restriction: spec
and tests only, except for the two mutations below, which touched only the lines
mutated.

The SDK symlink (`dialectica/logos-rust-sdk-src`) was created successfully; the
crate built and `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
dialectica -p dialectica-core` ran throughout.

## 1. Scenario coverage

Both requirements' scenarios are covered well, with two exceptions:

- [x] **`tester`** — spec.md's requirement *"A feed row reports how many of its
      thread's replies are visible"*, scenario **"Ops that are not posts are not
      counted"** (a thread's only reply revised, voted on, and unhidden by a
      moderator without ever having been hidden — count must still be one) has no
      corresponding fixture.
      **Measured:** `git grep -n -F -e "OpKind::Vote" -- dialectica/rust-lib/dialectica-core/src/feed.rs`
      returns only the header comment ("Nothing reads `Vote` ops, so no row
      carries a score") — no test in the reply-count section constructs a `Vote`
      op. `git grep -n -F -e "voted on"` over the crate's `src/` finds nothing
      relevant either. The closest tests each isolate one non-post kind against a
      *different* reply (`a_revision_does_not_move_a_reply_or_change_the_id_reported`
      revises; `a_restored_reply_is_counted_again` hides-then-unhides with a prior
      hide present), but none combines a revise, a vote, and an **unhide with no
      prior hide** against a single reply, which is exactly the "looks right by
      accident" shape the scenario is written to catch — e.g. a fold that treated
      any `Moderate` op naming the reply (regardless of action) as evidence of a
      hide-then-something-else state, or that miscounted an unhide with no
      matching hide, would not be caught by any current test.
      A related fixture exists in `end_to_end.rs` (`assert_eq!(*replies, None, "a
      vote is not counted as a reply")`), but it votes on the *root*, not a reply,
      and asserts no revise/unhide combination — it does not substitute for this
      scenario.
      Severity: moderate — a real, specific combinatorial case the spec calls out
      by name is untested.
      **Outcome (tester): fixed.** Added
      `a_reply_revised_voted_on_and_unhidden_with_no_prior_hide_is_counted_once`
      in `dialectica-core/src/feed.rs`: one reply carrying a `Revise`, a `Vote`
      and an `Unhide` with no prior `Hide` anywhere in the log, asserting a count
      of one and that reply as latest. **Predicted-vs-observed:** predicted this
      would catch a fold that treats any `Moderate` op naming a reply as evidence
      of a hide, regardless of action; measured directly by replacing the
      `moderation::resolve` call in `visible_replies_by_thread` with a local
      check that any `Moderate` op names the reply — the test failed
      (`left: 0, right: 1`), matching the prediction exactly. Mutation restored;
      `git diff --stat` shows only additions to `feed.rs`.

- [x] **`tester`** — spec.md's requirement *"Computing the reply fields fails as
      an error and never aborts"*, scenario **"An adversarial log is read without
      aborting"** (forged posts, posts naming parents not held, a post naming
      itself as its parent, a parent cycle, and ops of every other kind naming a
      reply, all in **one** log, read **once**, asserting the read terminates and
      every row reports a reply count) has no combined fixture in `feed.rs`.
      **Measured:** `git grep -n -F -e "naming itself"` over
      `dialectica-core/src` finds only `thread.rs` (which is `thread-read`'s own
      suite, testing `thread_of`'s termination — a different capability's
      requirement, per this spec's own "boundaries named rather than restated").
      `feed.rs` has one cycle test,
      `a_post_in_a_parent_cycle_is_counted_under_no_thread` (a 3-op ring), which
      covers the cycle sub-case in isolation but not a self-referencing parent,
      and not all five adversarial shapes combined in one log as the scenario
      specifies. Each individual shape *does* have its own isolated test
      elsewhere in `feed.rs` (forged reply, unheld parent, cross-Stoa post), so
      this is narrower than "the property is unchecked" — but the scenario is
      explicitly about the **combination not causing non-termination or a
      panic**, which is a property isolated fixtures cannot establish (an
      interaction between two adversarial shapes, e.g. a cycle where one member
      also fails verification, is untested).
      Severity: low-moderate — the individual mechanisms are well covered
      elsewhere; the combined-log termination property named by the scenario is
      not.
      **Outcome (tester): fixed.** Added `an_adversarial_log_is_read_without_aborting`
      in `dialectica-core/src/feed.rs`: one `WrongKeyLog` holding a
      self-referencing post, a two-op parent cycle, a post whose parent is never
      held, a forged post, and a genuine reply named by a `Revise`, a `Vote` and
      a `Moderate` op, read once, asserting `list_threads` returns `Ok` with a
      non-empty page and every row reports a `reply_count()`. **Predicted-vs-observed:**
      predicted this would catch a `feed.rs`-local re-implementation of the
      parent walk that omits a visited set (the exact trap the existing cycle
      test's own comment names — "a walk with no visited set would spin
      forever"); measured directly by replacing the fold's `thread_of(log, &id)?`
      call with a naive unguarded walk starting from each entry's own `parent`
      field. Run under a 15-second timeout, the test hung (`exit 124`) rather
      than failing an assertion — a non-termination catch, which is what this
      scenario is actually about, distinct from a `left != right` failure.
      Restored; re-ran clean at 0.27s. `git diff --stat` shows only additions to
      `feed.rs`.
      A third scenario the spec gained since this finding was written —
      *"A store failure on a reply of a thread whose row is not returned fails
      the read"* (commit `4eaa19c`) — also had no test; added
      `a_store_failure_on_a_reply_of_a_hidden_thread_fails_the_read`, a hidden
      root with a reply whose store read fails, with a healthy-store control
      confirming the healthy read returns only the visible thread's row.
      Predicted to catch a fold that swallows a store failure met while placing
      a reply instead of propagating it (the shape design.md Decision 5
      forbids); measured by making `thread_of`'s error branch map to `None`
      instead of `?`-propagating — the test failed with the swallowed-error page
      printed in the panic message, exactly as predicted. Restored.

Everything else scans clean: zero-reply rows, every-depth counting, the ordering
rule's first-met reply (both insertion directions), hidden-reply exclusion (with
paired before/after fixtures per requirement, exactly as spec.md's "the same log
without the hide" phrasing asks for), reply-beneath-hidden, non-moderator hide,
restore, forgery (both for counting and for latest), thread-field-vs-parent-chain
(`a_post_is_counted_by_its_parent_chain_and_never_by_its_thread_field`), the
not-yet-arrived-parent case (both halves: absent, then present after append),
revision (both the earlier-reply-revised-doesn't-move-it half and the
latest-reply's-own-id-not-its-revision's half), the include-hidden invariance
(row-level, and the hidden-root-with-visible-replies case), row order/paging
unaffected by replies, the two failure scenarios (store failure while counting,
and the off-page `NO SPEC` variant), the asserted-time-is-not-order scenario, the
cross-Stoa reply scenario, the two-peers-partial-view scenario, and the
agree-with-thread-read scenario. The wire-level closed-key-set assertions
(`the_feed_reply_is_the_ecosystems_pagination_shape` and
`a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new` in
`wire.rs`) correctly pin both the omitted-when-absent and
present-and-nothing-else-new cases for `latestReply`, and `replyCount`'s
always-present/zero-not-omitted rule.

The layer is right throughout: every reply-count/latest-reply test runs at
`dialectica-core`'s Rust layer (`feed::list_threads` directly, or through
`wire::list_threads`'s JSON), which is the only layer that can see the
cross-process fold this spec contracts — there is no QML-layer test claiming to
cover this capability, correctly, since nothing in `feed-view` renders either
field yet (per `proposal.md`'s "No view change").

## 2. Mutation testing

The tester's report (relayed by the runner, not read from a file) said two
mutations were refused by the permission classifier and so were never proven to
fail: **hidden-reply exclusion**, and **membership by parent chain rather than
by the post's own `thread` field**. `design.md` Decision 2 independently records
the same gap for a different mutation ("last-reply-wins" instead of
"first-met-wins"): *"The mutation was refused in the implementing session, and
the tester's run is what establishes it."*

Both of the tester's flagged mutations were attempted here, using the `Edit`
tool directly on `dialectica-core/src/feed.rs` (no shell command was needed to
make either edit) followed by `cargo test --manifest-path
dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core --lib feed::`.
**Neither mutation was refused in this environment.** Both ran to completion and
both were caught:

- **Membership by `thread` field instead of parent chain** — in
  `visible_replies_by_thread`, replaced `thread_of(log, &id)?` with a match
  reading `entry.op.op.kind`'s `Post { thread: Some(t), .. }` directly. Killed
  by 6 tests: `a_post_is_counted_by_its_parent_chain_and_never_by_its_thread_field`,
  `a_reply_beneath_a_hidden_reply_is_counted`,
  `a_post_whose_parent_is_not_held_is_counted_once_the_parent_arrives`,
  `a_post_in_a_parent_cycle_is_counted_under_no_thread`,
  `replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`,
  `the_count_agrees_with_the_thread_read`.
- **Hidden-reply exclusion disabled** — the `is_hidden()` guard was short-circuited
  to `false && …`, so no reply is ever excluded as hidden. Killed by 6 tests:
  `a_hidden_reply_is_neither_counted_nor_latest`,
  `a_reply_beneath_a_hidden_reply_is_counted`,
  `a_restored_reply_is_counted_again`,
  `a_thread_whose_only_reply_is_hidden_reports_zero_and_no_latest`,
  `the_include_hidden_flag_changes_no_rows_reply_fields`,
  `the_count_agrees_with_the_thread_read`.

Both mutations were restored immediately after their run (`git diff
dialectica/rust-lib/dialectica-core/src/feed.rs` is empty as of this commit;
`cargo test … feed::` was re-run green — 47 passed — after each restore). **No
mutation is left in the tree.**

This resolves the tester's open question for these two properties: both are
real, both are guarded, and the guarding tests can fail. It does not resolve
`design.md` Decision 2's separate "last-reply-wins" mutation, which nobody has
run — that mutation was not in my two-mutation budget and is a legitimate
follow-up for whoever next has budget to spend, though the three tests
Decision 2 names as the predicted killers
(`the_latest_reply_is_the_one_the_ordering_rule_places_first`,
`replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest`,
`a_hidden_reply_is_neither_counted_nor_latest`) look, by inspection of their
fixtures (higher-counter reply appended first and checked under both insertion
orders), like they would in fact catch an overwrite-on-insert.

## 3. NO SPEC markers

One `NO SPEC:` marker was introduced by this change (`feed.rs:2001`, on
`a_store_failure_on_a_reply_off_the_page_fails_the_page`):

> `feed-read` requires a store failure met while computing a row's reply fields
> to fail the read, and says nothing about a failure met on a reply whose thread
> is NOT on the requested page. The fold runs over the whole Stoa, so this fails
> the page too; `design.md` records why.

This is a genuine, well-documented spec gap, not a defect — `design.md` Decision
5 and its Risks section reason through the alternative (fold only on-page
threads) and reject it on cost grounds, explicitly leaving the choice for the
spec-writer: *"It is still a choice the spec should either make or refuse."*

- [x] **`spec-writer`** — decide and record whether a store failure on a reply
      whose thread is off the requested page should fail the whole read (current
      behaviour, matching the head loop's existing behaviour for off-page heads)
      or should be scoped to the page somehow. Either answer is defensible;
      leaving it silent means a future reader of the spec alone cannot tell this
      was a deliberate choice rather than an oversight.
      **Outcome (spec-writer): fixed — the current behaviour is now contracted.**
      `specs/feed-read/spec.md`'s requirement *"Computing the reply fields fails
      as an error and never aborts"* now requires the read to fail whichever
      reply in the Stoa the failure is met on: a reply on the page, one on
      another page, or one whose thread's row the read does not return (hidden
      root, hidden content excluded). It forbids skipping a failure because its
      reply belongs to no returned row. Two scenarios were added, each with a
      healthy-store control: *"A store failure on a reply of a thread on
      another page fails the read"* and *"A store failure on a reply of a thread
      whose row is not returned fails the read"*. `proposal.md`'s What Changes
      lists it too. Why page-scoping was refused, for the `dev-writer` to add
      to Decision 5: the failing read in this marker's own fixture is the `get`
      on the reply itself, the first step of `thread_of`. So when a failure is
      met while placing a reply, that reply's thread is not yet known, and
      nothing can show it is off the page. Scoping would silently undercount
      whichever on-page thread the reply really belongs to. The `NO SPEC:`
      marker at `feed.rs:2001` should now be removed.

(`feed.rs:863`'s `NO SPEC:` marker, on the `author` field's shape, predates this
change — introduced by #88's "Delete the author address" — and is out of scope.)

## 4. Requirements moved between capabilities

Not applicable. `proposal.md`'s "Modified Capabilities: None" is accurate:
`git diff main --stat -- openspec/specs/` shows no change to any existing spec
file. `feed-read` is a wholly new, additive capability; no `REMOVED`/`ADDED`
pair exists to check.

## 5. Spec soundness

- **Self-consistency:** read `feed-read/spec.md` in full. No internal
  contradiction found. The three boundary-ownership statements in the Purpose
  section (membership to `thread-read`, ordering to `op-ordering`, hiding to
  `moderation-resolution`) are honoured throughout the requirements — none of
  them restates a rule it disclaims owning.
- **Testability:** every scenario is phrased as an observable outcome
  (a count, a returned/omitted field, an error, row order) checkable by a unit
  test at the core layer, and every scenario found a test except the two named
  in §1. The one normative clause with no matching scenario — "[the count] MUST
  NOT be documented or described as the thread's total" — is a documentation
  requirement rather than a behavioural one; it is not written as a scenario and
  so is not a coverage gap, and it is satisfied in `feed.rs`'s header prose (not
  a test-review concern).
- **Staleness against issue #100** (`gh issue view 100 --repo
  fryorcraken/dialectica`, read fresh): the spec matches the issue's substance
  fully — the parent-chain membership rule, the moderation-resolved-state
  requirement, the explicit "count that includes hidden replies looks entirely
  plausible and is wrong" concern, and the `active`-ordering motivation are all
  present in `proposal.md`/`spec.md`. One divergence: the issue suggests
  "whoever writes the `feed` capability spec (#91) should include these two
  fields," implying the fields land in an existing/future single feed
  capability, and cites `feed-view/spec.md` as "the existing feed-row rendering
  contract this field will be added alongside." The change instead creates a
  **new** `feed-read` capability, distinct from `feed-view`. This is not
  staleness or a defect: `proposal.md`'s "Capabilities" section reasons through
  exactly this choice (`feed-view` is rendering, not core computation; a bare
  `feed` capability would be the only one dropping the `-read` suffix its
  sibling `thread-read` carries) and explicitly hands `feed-read` — not a second
  capability — as what #91 should extend. Recording it here only because the
  issue text alone would read as prescribing the other layout, and a future
  reader comparing issue to spec should not conclude anything was missed.

## Summary

4 findings: 2 for `tester` (missing scenario coverage — the "ops that are not
posts" combination and the "adversarial log" combined-abort scenario), 1 for
`spec-writer` (the off-page store-failure `NO SPEC` gap, asking for a decision),
and 1 mutation-testing result reported under §2 (informational — resolves the
tester's open question, no box needed since nothing failed to be caught).
Areas checked clean and not itemised as boxes: the bulk of scenario coverage
(§1), capability-move checks (§4, not applicable), and spec self-consistency and
issue staleness (§5).
