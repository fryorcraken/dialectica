# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

## Implementation

## 1. Membership, which is the security property

- [x] 1.1 Write the attack test first — an authentically signed post whose
      `thread` names a thread its parent chain does not reach — and watch it fail
      against a claim-trusting implementation. Verified: nine tests fail under
      the mutation, headed by
      `thread::tests::a_forged_thread_claim_cannot_inject_a_post_into_a_thread`,
      which returns `[root, injected]` where `[root]` is required.
- [x] 1.2 Implement `thread::thread_of` as a parent-chain walk that never reads
      the `thread` field, verifying each op before following its parent. Verified
      by `a_post_claiming_one_thread_is_placed_by_its_parent_in_another` (both
      directions) and `a_forged_op_in_the_chain_cannot_place_a_genuine_post`.
- [x] 1.3 Terminate the walk over adversarial parents with a per-walk visited
      set. Verified by `a_cycle_among_parents_terminates_and_places_nothing`
      (lengths two, three and four),
      `a_post_naming_itself_as_its_parent_terminates_and_places_nothing` and
      `a_chain_that_runs_into_a_cycle_terminates_and_places_nothing` — each
      **hangs** rather than failing without the set, measured with a timeout.
- [x] 1.4 Return a post whose chain cannot be completed under no thread, with no
      error for the read. Verified by
      `a_post_whose_parent_is_absent_is_placed_under_no_thread`,
      `a_post_becomes_placeable_when_its_missing_parent_arrives` (both halves)
      and `a_chain_reaching_an_op_that_is_not_a_post_places_nothing`.

## 2. The page a caller receives

- [x] 2.1 Build `thread::read_thread`: refuse a non-root three ways, then place,
      resolve, filter and page. Verified by
      `the_three_refusals_are_three_different_messages` and
      `a_thread_with_no_replies_is_served_and_not_refused`.
- [x] 2.2 Report the author as an address **and** a public key, and no name.
      Verified by `every_item_carries_both_an_address_and_the_key_that_signed`,
      which re-derives the address from the returned key rather than trusting the
      pairing.
- [x] 2.3 Render the current version and report revision by op id rather than by
      content. Verified by
      `a_revised_reply_renders_the_revisions_body_and_is_marked_revised`,
      `an_unrevised_post_reports_the_two_ids_as_equal_and_is_not_revised` and
      `a_revision_by_someone_other_than_the_author_changes_nothing`.
- [x] 2.4 Carry the resolver's three-valued moderation state and its deciding op.
      Verified by `an_unmoderated_post_names_no_deciding_op`,
      `a_hidden_post_names_the_op_that_hid_it` and
      `a_restored_post_is_distinguishable_from_one_nobody_moderated`.
- [x] 2.5 Keep a hidden root, marked, with body and attachments **withheld**
      (`None`) rather than emptied; omit a hidden reply. Verified by
      `a_hidden_root_is_returned_marked_with_its_body_withheld`,
      `a_revision_that_clears_a_body_renders_as_empty_and_not_as_withheld`,
      `a_hidden_reply_is_omitted_by_default_and_returned_marked_on_request` and
      `a_reply_to_a_hidden_reply_is_still_returned_and_still_names_it`.
- [x] 2.6 Put the root first, flat, each item naming its parent, and write no
      comparison. Verified by
      `the_root_is_the_first_item_whatever_the_logs_order_puts_first` (which also
      asserts the replies keep the log's relative order),
      `two_peers_holding_the_same_ops_return_the_same_sequence` and
      `a_deep_chain_comes_back_flat`.
- [x] 2.7 Page with the root occupying a slot, hidden replies excluded before the
      cut. Verified by `pages_partition_the_thread_with_no_gap_and_no_repeat`,
      `the_root_occupies_a_slot_and_is_not_repeated_on_a_later_page`,
      `has_more_is_true_exactly_while_a_further_page_exists`,
      `hidden_replies_are_excluded_before_the_page_is_cut`,
      `a_page_past_the_end_is_empty_rather_than_an_error` and
      `an_enormous_page_index_does_not_overflow`.
- [x] 2.8 Sanitise every string on the way out and leave the stored op untouched.
      Verified by `a_hostile_body_is_sanitised_and_the_stored_op_is_not` (which
      also re-verifies the stored op), `attachments_are_sanitised_too` and
      `an_ordinary_body_reports_nothing_removed_and_nothing_marked`.

## 3. The wire surface

- [x] 3.1 Add `wire::read_thread` and `wire::read_thread_from_request`, sharing
      `read_thread_inner` so the request is parsed exactly once. Verified by
      `the_thread_reply_is_the_ecosystems_pagination_shape` and
      `a_thread_request_carrying_its_genesis_record_reads_the_thread`.
- [x] 3.2 Omit — never null — `parent` on the root, `decidedBy` when unmoderated,
      and `body`/`attachments` when withheld. Verified by
      `the_wire_reports_moderation_as_three_values_and_omits_the_deciding_op` and
      `a_withheld_body_is_absent_and_a_cleared_one_is_an_empty_string`.
- [x] 3.3 Refuse a missing field distinguishably from a wrong-typed one, and read
      `includeHidden`'s absence and its `null` as the restrictive default.
      Verified by
      `a_thread_read_refuses_a_missing_field_distinguishably_from_a_wrong_typed_one`
      and `the_include_hidden_flags_absence_and_its_null_both_exclude` (which
      also asserts the flag genuinely widens the answer when set).
- [x] 3.4 Verify the genesis record against the Stoa before a moderator set is
      built. Verified by
      `a_thread_genesis_record_that_does_not_hash_to_the_stoa_is_refused`.
- [x] 3.5 Never answer a store failure with an empty page, and never abort.
      Verified by `a_thread_read_against_a_broken_store_is_an_error_and_not_an_empty_page`
      and `a_panicking_store_reaches_the_caller_as_the_error_shape`.
- [x] 3.6 Add both handlers to `every_request_taking_method` and give each a
      served-request fixture, so every envelope sweep covers them. Verified by
      the existing sweeps turning over the two new names.
- [x] 3.7 Forward `readThread` from the module adapter. **Not verified by any
      test that runs**: `dialectica/rust-lib/src/lib.rs` is behind
      `cfg(logos_scaffold)` and `cargo test` does not compile it. Only CI's Build
      LGX job sees this file.

## 4. The gaps the dev's suite left, closed by the tester

- [x] 4.1 Sweep the not-a-post refusal over **kinds** rather than over the vote
      alone. `thread::every_non_post_kind_takes_the_same_refusal` and
      `wire::every_non_post_kind_takes_one_refusal_on_the_wire_and_never_the_unheld_one`
      run a vote, a moderation op, a Stoa metadata op and a revision through
      `read_thread`, and
      `thread::a_chain_reaching_any_op_that_is_not_a_post_places_nothing`
      (which replaces the vote-only
      `a_chain_reaching_an_op_that_is_not_a_post_places_nothing`) runs all four
      through the chain walk. Proved by mutating the `_ => NotAPost` arm to
      `Revise => IsAReply`: the two sweeps fail, the dev's vote-only
      distinguishability tests both pass.
- [x] 4.2 Keep the hand-written kind table from going stale.
      `every_op_kind_is_either_a_post_or_in_the_non_post_table` matches
      exhaustively with no wildcard arm, so a sixth `OpKind` variant is a
      compile error here rather than a silently narrower sweep, and asserts the
      four entries are four distinct discriminants.
- [x] 4.3 Pin that a **held** op is never reported as unheld.
      `thread::no_held_non_post_is_ever_reported_as_unheld` compares each
      refusal against `NotAThread::NotHeld(<the same id>).to_string()` — the
      message the implementation would have produced had it been wrong — so it
      cannot pass by a joint reword nor by two ids making two strings. Proved by
      mutating that arm to `NotHeld`: it fails, as does the wire sweep.
- [x] 4.4 Assert the three refusals by the **next action each implies** rather
      than by three distinct strings.
      `the_three_refusals_each_name_the_next_action_they_imply` requires the
      wait-for-propagation phrasing of exactly one refusal and forbids it of the
      other two, and likewise for the category error and the read-this-instead
      pointer. Proved twice: giving `NotAPost` the "may not have arrived yet"
      wording, and stripping `IsAReply`'s pointer — each fails this test alone
      while both dev distinguishability tests stay green.
- [x] 4.5 Replace the branch-on-what-happened root-ordering fixture.
      `a_thread_whose_root_does_not_sort_first` **searches** for a body whose
      root does not lead `iter_stoa`'s order (the pattern `moderation.rs` uses),
      and the test now asserts the arrangement rather than reporting it. Proved
      by turning `items.insert(0, item)` into `items.push(item)`.

## 5. Documents

- [x] 4.1 Write `design.md` alongside the code, recording the chain-walk decision
      and what was rejected.
- [x] 4.2 Correct `docs/UI-BRIEF.md`'s vote-count passage, which said "there is
      no thread read at all". Its conclusion — that no call returns a vote count
      — is untouched and still correct.
