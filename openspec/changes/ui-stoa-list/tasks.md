# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

## Implementation

## 1. The call path

- [x] 1.1 Add `createStoa`, `joinStoa` and `listStoas` to `Core.qml` as named
      wrappers, so each core method string is spelled in exactly one place.
      Verified by `test_each_core_method_is_named_once_and_reached_through_the_wrapper`,
      which drives all three through a recording fake bridge.

## 2. The shareable thing

- [x] 2.1 Add `StoaReference.qml`, a singleton owning both `shareText` and
      `parse` — one file, because a share whose output the paste field cannot
      read is a string its recipient can do nothing with. Verified by
      `test_what_a_share_produces_is_what_a_paste_accepts` (the round trip) and
      `test_a_share_carries_both_halves_and_the_address_in_full`.
- [x] 2.2 Strip the `stoa:` display prefix on the way in and never add it on the
      way out, so a prefix cannot reach the core as part of a hash. Verified by
      `test_a_display_prefix_is_stripped_before_anything_is_sent`.
- [x] 2.3 Refuse input carrying only one half, naming which half is missing —
      distinct from the core's verification refusal. Verified by
      `test_an_address_with_no_record_is_named_as_the_missing_half`.

## 3. The clipboard

- [x] 3.1 Add `ClipboardSink.qml`, a hidden `TextEdit` giving
      `AddressLabel.copyRequested()` its first receiver anywhere in the view.
      Verified by `test_a_share_string_reaches_the_clipboard_sink_verbatim`,
      which asserts what the sink was **asked** to copy. **What actually lands on
      the system clipboard is not verified and cannot be**: the runner is
      headless (`QT_QPA_PLATFORM=offscreen`) and there is no clipboard to read.

## 4. The Stoa list

- [x] 4.1 Add `StoaListScreen.qml` with three read states computed from one
      variable. Verified by `test_an_empty_membership_is_the_ok_state…`,
      `test_an_unreadable_membership_is_a_failure…` and
      `test_the_empty_state_and_the_failed_state_are_different_states`, the last
      of which compares rendered text and not only the state string.
- [x] 4.2 Treat a success carrying no `items` array as a failure rather than an
      empty membership. Verified by
      `test_a_success_without_an_items_array_is_a_named_failure`, covering both
      an absent field and a non-array one.
- [x] 4.3 Render every row with its address beside its title, through
      `AddressLabel` and with no second abbreviation. Verified by
      `test_a_row_carries_the_address_as_well_as_the_title` and
      `test_two_stoas_with_the_same_title_render_differently`.
- [x] 4.4 Render an empty founding title as a row with no substitute. Verified by
      `test_an_empty_founding_title_still_gets_a_row_with_its_address`.
- [x] 4.5 Keep every peer-supplied title on `Text.PlainText`. Verified by
      `test_a_title_containing_markup_is_rendered_as_literal_characters`, which
      asserts the element's **`textFormat`** — an earlier version asserted the
      `text` string and passed against a `StyledText` implementation.
- [x] 4.6 Render no per-row count and substitute nothing in that position.
      Verified by `test_no_row_renders_a_count_of_held_posts_or_anything_global`,
      which answers a thread listing too so a page length is available to be
      wrongly rendered.
- [x] 4.7 Offer a share only where the view holds the record. Verified by
      `test_a_share_is_offered_only_where_the_view_holds_the_record`, which
      asserts the positive and the negative from one fixture so a rename breaks
      it rather than satisfying it.

## 5. Creating

- [x] 5.1 Offer a title field and no key or identity field, always, whatever the
      keystore holds. Verified by
      `test_the_create_affordance_is_present_when_no_key_exists`.
- [x] 5.2 Pass an empty title through rather than refusing it here. Verified by
      `test_an_empty_title_reaches_the_core_rather_than_being_refused_here`,
      which asserts the call was made **and** what was sent.
- [x] 5.3 Render the returned address, and render a repeat creation as success.
      Verified by `test_the_created_address_is_rendered` and
      `test_the_same_title_twice_is_one_stoa_reported_twice`, the latter also
      asserting nothing was appended to the user's title.
- [x] 5.4 Render the core's refusal unreworded. Verified by
      `test_a_creation_refused_for_want_of_a_key_renders_the_cores_reason`.

## 6. The join preview

- [x] 6.1 Add `JoinScreen.qml` whose join state is one string, so its two
      failures cannot render through one branch.
- [x] 6.2 Make no core call on render, and the join call only on the user's
      action. Verified by `test_the_preview_makes_no_join_call_until_the_user_acts`,
      which asserts against the bridge's **call log** — the only place the
      difference between previewing and silently joining is visible.
- [x] 6.3 Render the address in full and label the founding title as founding.
      Verified by `test_the_preview_renders_the_whole_address_not_an_abbreviation`
      and `test_the_founding_title_is_labelled_as_founding`.
- [x] 6.4 Render no current title while nothing resolves one, and render one when
      supplied. Verified by `test_no_current_title_is_rendered_while_nothing_resolves_one`
      and `test_a_resolved_current_title_fills_that_position_when_one_exists`.
- [x] 6.5 State a hash match and name the unverified remainder, narrowing the
      bundle's `join.note`. Verified by
      `test_the_explanation_claims_a_hash_match_and_names_the_unverified_rest`,
      which also asserts the bundle's overreaching clause is absent.
- [x] 6.6 Report a join from the reply only, and render a repeat join as success.
      Verified by `test_a_failed_join_does_not_report_the_stoa_as_joined` and
      `test_joining_a_stoa_already_held_is_success_with_no_warning`.
- [x] 6.7 Show a same-title Stoa already held beside the preview, over titles and
      never over addresses. Verified by
      `test_a_same_title_stoa_already_held_is_shown_beside_the_preview` and
      `test_the_same_address_is_not_shown_beside_itself_as_a_second_stoa`.
- [x] 6.8 Claim no per-Stoa identity, membership or moderator status — dropping
      the bundle's "generates you an identity for it alone". Verified by
      `test_nothing_on_the_preview_promises_a_per_stoa_identity`.

## 7. Navigation

- [x] 7.1 Remove `Main.qml`'s developer-supplied Stoa properties and make the
      list the entry point. Verified by
      `test_the_view_supplies_no_stoa_of_its_own_before_one_is_chosen`, which
      asserts the properties are `undefined` rather than merely empty.
- [x] 7.2 Carry the address and, where held, the record from the list row to the
      feed, and invent neither. Verified by
      `test_the_feed_is_given_the_address_and_the_record_from_the_chosen_row` and
      `test_no_record_is_invented_for_a_row_the_view_has_none_for`.

## 8. Gates

- [x] 8.1 `qmlformat -n` parses every new file; `qmllint --unqualified disable`
      exits 0 over the whole `src/qml` directory.
- [x] 8.2 `dialectica-ui/tests/run-qml-tests.sh` runs 5 spec files, all green.
- [x] 8.3 Prove the new suite can fail: seven mutations of the implementation,
      six caught. The seventh — switching a title's `textFormat` to
      `Text.StyledText` — **passed**, which is what moved
      `test_a_title_containing_markup…` from asserting `text` to asserting
      `textFormat`. The mutation now fails the suite.
- [x] 8.4 Update `docs/UI-BRIEF.md` where this change makes it wrong: the share
      affordance's conditional availability on a list row, the shape of the
      shareable thing, and what the address does and does not prove.
