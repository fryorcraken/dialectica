# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
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

## 9. Making the absence assertions honest about the apparatus

The apparatus column is annotation, not interface, and a separate piece removes
it. Re-reading the two whole-screen absence assertions for that change turned up
a defect that was never about the apparatus at all.

- [x] 9.1 Measure rather than estimate what those assertions scan: **747 of 1371
      characters** on a rendered join screen are apparatus. Established with a
      throwaway probe under `tmp/`, since `console.log` is swallowed by the
      runner and a deliberately-failing `compare` is what surfaces a string.
- [x] 9.2 Add `bodyText(screen)`, which subtracts the apparatus rather than
      walking a named body child — `ScreenFrame` exposes its body as a
      default-property alias with no `objectName`, so a structural lookup would
      break silently where subtraction breaks loudly. Both absence assertions now
      run over it.
- [x] 9.3 **Fix the real defect found by mutation.**
      `test_nothing_on_the_preview_promises_a_per_stoa_identity` scanned a fresh
      preview, whose body says nothing whatever about what joining does — the
      only sentence on the subject was in the apparatus. Planting "generates you
      an identity for it alone" in the joined panel left it **passing**. Proved
      by running the original assertion verbatim against the planted claim. It
      now drives the screen to the joined state, and fails on that mutation.
- [x] 9.4 Assert each absence's corpus before asserting the absence, so a test
      cannot go vacuous: `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus`
      pins that the body carries on-topic text of its own and that `bodyText` is
      genuinely subtracting.
- [x] 9.5 Verify the whole suite against a hidden apparatus column: **54 tests,
      all green**, so nothing here depends on annotation and the removal piece
      cannot silently narrow this file.
- [x] 9.6 A word-boundary regex, not a substring, for the repeat-join wording
      check — a bare `indexOf("again")` fires on "not checked **again**st any
      registry", which is legitimate copy. Caught by the assertion failing on its
      first run.

## 10. The tester's pass

Six tests added, each proved to fail by a mutation that the 47 tests already
here left green. **Three of the six close a defect of the same family as the
dev-writer's seventh mutation** — a test asserting the SOURCE where the
requirement is about the RENDER — and three close a test that pinned a literal
or a difference where the requirement is about meaning.

- [x] 10.1 `test_no_share_button_is_on_screen_for_a_row_whose_record_is_not_held`.
      The existing test asserts `canShare()`, which is the computation and not
      the screen. Mutation: `visible: screen.canShare(row.rowStoa)` →
      `visible: true`. All 47 prior tests passed with a share offered for a Stoa
      the view holds no record for — the third failure this test file's own
      header names as what it exists to catch. Observed: 2 buttons, expected 1.
- [x] 10.2 `test_the_joined_outcome_is_reported_on_screen_and_not_only_in_a_property`.
      Mutation: the joined panel's `visible:` → `false`. All 47 passed while
      nothing on screen reported the join; `joinState` held `"joined"` and the
      user was told nothing. Observed: 0 panels, expected 1.
- [x] 10.3 `test_no_feed_is_on_screen_before_a_stoa_has_been_chosen`. The existing
      test asserts `screenShown === "list"`, a derived string. Mutation:
      `FeedScreen`'s `visible:` → `true`. All 47 passed with a feed rendering
      for the empty address at startup. Observed: 1 feed, expected 0.
- [x] 10.4 `test_no_digit_is_rendered_that_the_reply_did_not_supply`. The
      strengthening of 4.6, which the dev-writer named as their least confident
      test and correctly said could not catch a count rendered as a bare `31` in
      a row's margin. Stated as a relation instead of a phrase blocklist: every
      digit-run on screen must appear in the address or the title the fixture
      supplied, and the fixture carries digits so the assertion is not vacuous.
      Mutation: a `Text { text: "31" }` in the row's margin. The new test fails
      naming `31`; **4.6's test passes against the same mutation**, in the same
      run. The residue it still cannot see — a count spelled in words, or in
      non-digit glyphs — is named in the test rather than left to be assumed.
- [x] 10.5 `test_a_malformed_paste_and_an_unverified_record_say_different_things_to_do`.
      The existing test asserts the two refusals are DIFFERENT, which is the
      defect the sibling `thread-read` piece shipped: misinforming strings are
      still distinct strings. This asserts what each must and must not imply —
      a malformed paste names the input and must not report a verification
      outcome; a verification refusal carries the core's words and offers no
      retry. Mutation: reworded the malformed-paste reason to "This reference
      does not hash to its address." The existing distinctness test **passed**;
      this one failed.
- [x] 10.6 `test_the_address_note_cannot_be_simplified_into_an_unqualified_verified`.
      6.5's test pins three substrings, so it fails on a reword and passes on a
      misinformation. This asserts the three things the copy cannot lose — the
      scope of the proof, the named remainder, the explicit bound on what was
      consulted — plus that no unqualified claim of verification stands anywhere
      on the screen. Mutation: a note carrying all three pinned substrings and
      ending "This is a verified record and a verified Stoa." **6.5's test
      passed**; this one failed. A `text: "Verified."` simplification fails both.

- [x] 10.7 Implementation restored after every mutation, proved with
      `git diff --stat -- dialectica-ui/src` returning empty rather than from
      memory.

### What remains unverifiable, stated rather than asserted around

- **The clipboard.** Unchanged from 3.1 and honestly drawn: under
  `QT_QPA_PLATFORM=offscreen` there is no system clipboard, `TextEdit.copy()`
  writes nowhere readable, and `ClipboardSink.lastCopied` records only what the
  sink was ASKED to copy. No test claims otherwise.
- **Anything visual.** QML tests see properties and signals, never pixels. That
  the failed and empty read states are legible as different screens, that the
  lookalike panel reads as two Stoas rather than one, and that the address is
  large enough to compare by eye are unverified by anything here.

## 11. Four scenarios the spec-test review found unpinned

Numbered from this file's current maximum rather than from what I last wrote —
the rule the `## 9` collision produced, and the reason `validate --strict` had
to be repaired once already.

The spec-test reviewer ran five mutations and **four survived**, each confirmed
rendered before being called a survivor. All four are the two defect families
this change has now hit repeatedly: three are an absence asserted over a corpus
that could not contain the candidate, and one is a literal blocked where the
requirement is about what a caption claims.

- [x] 11.1 R13's scenario names **the list and the creation outcome**; the only
      test on the subject scanned the join screen. Fixed by
      `test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity`,
      which drives the list into the created state and **asserts its corpus
      before any absence** — the clause that stops it going vacuous the way its
      sibling did. Mutation: the reviewer's caption claiming both moderation and
      a per-Stoa identity. Caught, where all 100 previously passed.
- [x] 11.2 R11's "it offers no field for a creator key" was unpinned — the only
      create test asserts the button exists. Fixed by
      `test_the_create_affordance_offers_exactly_one_field_and_it_is_not_a_key`,
      counting editable inputs (hardcoded 2, since `<= 2` passes on zero) via a
      new `editableInputs` helper keyed on input behaviour rather than on a name
      or a placeholder, plus a caption assertion as a second net. Both nets
      proved independently.
- [x] 11.3 R1's "A second abbreviation MUST NOT be written" was knowingly
      unpinned — the row test says so in its own comment. Fixed by
      `test_a_row_abbreviates_through_the_one_component_and_keeps_a_middle_group`:
      three groups (the requirement itself, which a correct reimplementation
      would satisfy) **and** `full !== undefined` (the no-second-implementation
      clause). Two mutations, one for each clause.
- [x] 11.4 R6 was pinned by two literal strings. Fixed by adding two
      conjunctions to `test_no_current_title_is_rendered_while_nothing_resolves_one`
      — a present-tense naming word with a title word, and a moderator with a
      naming word, each in either order. Mutation: the reviewer's extended
      caption, which carries neither blocked literal.
- [x] 11.5 Implementation restored after all five mutations, proved with
      `git diff --stat -- dialectica-ui/src` returning empty rather than from
      memory.

### What this pass did not close

The reviewer's fifth mutation was **caught**, so it opened no box. Their two
`spec-writer` boxes on this file — the undeclared `stoa:` prefix behaviour and
the missing route out of terminal states — are not mine and are left open.

The residue named at 10.4 stands unchanged: a count spelled in words or in
non-digit glyphs is still invisible to
`test_no_digit_is_rendered_that_the_reply_did_not_supply`. The reviewer
independently reached the same conclusion and declined to mutate it, on the
grounds that the test already declares the limit — which is the right call, and
is what an honestly stated gap is for.

## 12. The spec-writer's second pass, and what it leaves uncovered

Closing the eight `spec-writer` findings added three requirements and extended
two. **No code and no test was touched** — every `dev-writer` and `tester` box is
closed and both agents have left the tree, so a spec moving under them is the
failure the one-writer rule exists to prevent.

- [x] 12.1 The `stoa:` display prefix contracted under R9, where the defect it
      caused belongs: a surviving prefix manufactures a verification accusation
      out of a clipboard artefact. Stripping is required to be unbounded rather
      than a fixed number of passes.
- [x] 12.2 New requirement fixing the reference encoding as a compatibility
      surface, including that it must be self-describing so a truncated paste
      fails as malformed rather than as a verification mismatch.
- [x] 12.3 New requirement "Every state a user can enter has a specified way
      out", covering `preview → joined` and `create → created`.
- [x] 12.4 R10's summary sentence rewritten so it stops forbidding the address
      comparison its own second scenario requires.
- [x] 12.5 The share requirement extended with the immediately-unshareable
      created Stoa, and the character-class question recorded as deliberately
      open with the bound that keeps it safe.
- [x] 12.6 PLAN.md: two passages struck to strikethrough-plus-pointer, and the
      join-confirmation entries redirected off the never-landed §11.1.

### What the new scenarios cover, and the four that are genuinely unpinned

**Most of the new scenarios were already written against behaviour the suite
tests** — they were added because no *requirement* named it, which was the whole
of the finding in each case. Checked against a full run (103 passing, and the run
corrected one claim I had drafted from memory):

Already covered, so these are a citation rather than new coverage:

| Scenario | Existing test |
|---|---|
| a display prefix never reaches the core | `test_a_display_prefix_is_stripped_before_anything_is_sent` |
| a repeated prefix is stripped rather than forwarded | `test_a_repeated_display_prefix_is_stripped_rather_than_forwarded` |
| a share round-trips through a paste | `test_what_a_share_produces_is_what_a_paste_accepts` |
| the encoding is a JSON object of two string halves | `test_the_reference_format_is_a_json_object_with_two_hex_fields` |
| a half of the wrong type is refused | `test_a_half_that_is_not_a_string_is_refused_rather_than_coerced` |
| a truncated reference fails as malformed | `test_text_that_is_not_a_stoa_reference_is_refused_before_any_call` |
| a Stoa joined in this session can be shared | `test_a_share_is_offered_only_where_the_view_holds_the_record` |
| a row without a record offers no share | `test_no_share_button_is_on_screen_for_a_row_whose_record_is_not_held` |

**Genuinely unpinned, for the `tester`** — each is satisfied by the shipped tree
(checked against the code before being written, and two were rewritten when the
code disproved my first draft), but none has a test, so each would survive its
behaviour being deleted:

| Requirement | Scenario |
|---|---|
| R9 (malformed vs unjoinable) | a prefix is not added to what is shared |
| Way out | the preview can be left without joining |
| Way out | the return is still available after a join succeeds |
| Way out | a created Stoa reaches the list without a restart |

The last three are the ones the spec-test reviewer measured as "relied on by the
tests and required by no scenario" — `join.cancelled()` is called at line 897 as
setup only, and no test asserts the post-creation `reload()`. They now have
requirements; they still need tests.

**The `list → feed` return is deferred and is not in the table**, because it is
the one transition the code cannot satisfy: `FeedScreen` declares no signal and
nothing clears `Main.qml`'s `chosen`. The requirement names it as deliberately
outside itself so the gap reads as deferred rather than as an oversight. Closing
it needs a `dev-writer`, and the architecture reviewer's box for it stays open.

> **Superseded by §13.1.** The `dev-writer` pass below added `FeedScreen.closed()`
> and the affordance that emits it, so the deferral above is discharged and the
> architecture box is closed. Left in place rather than rewritten, because the
> reasoning for deferring it is still the reason the requirement is worded as an
> outcome rather than a mechanism.

## 13. The way out, and two shapes that only held by agreement

Numbered from the file's current maximum, per §11's rule.

- [x] 13.1 `FeedScreen` gains a `closed()` signal and an unconditional
      "All Stoas" affordance; `Main.qml` clears `chosen` on it. Verified by
      `test_a_user_who_opened_a_stoa_can_return_to_the_list`, confirmed failing
      `Actual: 0 / Expected: 1` beforehand. **The test finds the control among
      the VISIBLE elements and clicks it**, rather than emitting the signal — a
      signal with no rendered control is a route only a test can take, and would
      leave the user exactly as stranded. `grep -n "signal" FeedScreen.qml`
      returned nothing before this, confirming the reviewer's measurement
      independently.
- [x] 13.2 The return costs the user nothing: `genesisByStoa` lives on the list,
      so a second open is as complete as the first. Verified by
      `test_reopening_a_stoa_after_returning_still_carries_its_record`.
- [x] 13.3 `Main.qml` gains `preview()`, `open()` and `closeFeed()`, each
      clearing what the others own, so `(chosen ≠ null, previewing ≠ null)` —
      which has no rendering and which the ordered ternary resolved by accident
      — cannot be constructed. Verified by
      `test_a_preview_requested_while_a_feed_is_open_is_not_swallowed`, which
      asserts both directions.
- [x] 13.4 `rows` becomes `lastListing` plus a derived `visibleRows` carrying the
      read-state guard, removing both call-site copies of it (the `Repeater`
      model and `Main.qml`'s `heldStoas`). Verified by
      `test_a_failed_reload_makes_the_stale_listing_unreadable_not_merely_unrendered`,
      which pins that `visibleRows` is empty **and** `lastListing` still holds
      the previous page — so a later change cannot "fix" this by destroying a
      good listing, which is the trap the original comment avoided.
- [x] 13.5 Three wrong symbol names corrected — `Core.qml`'s pointer to a
      nonexistent `JoinScreen.parseReference`, and `design.md` D1's
      `parseReference`/`shareTextFor`. Re-measured afterwards: zero hits in any
      implementation, design or spec text.
- [x] 13.6 The share degradation restated correctly in `Main.qml`, `design.md`
      and `docs/UI-BRIEF.md`. All three said "lost on restart", which is true of
      a JOINED Stoa and **false of a created one** — `create_stoa` returns no
      genesis record, so a created Stoa is unshareable immediately. UI-BRIEF was
      the worst of the three: it said "created or joined in the current session",
      which affirmatively misdescribes creation to a designer who cannot read the
      code.

## 14. Claims in the test file that no command can check

Three readability boxes, all addressed to `tester` and all one species: a
sentence in a test comment asserting a fact that goes stale without failing.
`CLAUDE.md`'s "do not write down anything a command can answer" is written for
prose and applies here unchanged — the suite is the command.

- [x] 14.1 The apparatus measurement was stated twice, 157 lines apart, in
      different words. Kept at the `bodyText` helper where it justifies the
      subtraction; the header now points there. A duplicated constant is one
      somebody updates in one place, and the column it measures is scheduled for
      removal by another piece.
- [x] 14.2 "All 47 prior tests passed", three times, plus "95 of 95". Restated
      as the relation — "left the whole suite green with the defect present" —
      with the number left to the runner. **Worse than the box reported:** the
      file now defines 64 `test_*` functions, so the figure was stale by
      seventeen, and the reviewer's own "47" had itself gone stale between the
      review and this fix. The claim a reader needs — that the mutation was
      invisible to everything else — survives and does not rot.
- [x] 14.3 `test_a_record_that_does_not_verify_is_a_failure_distinct_from_a_bad_paste`
      **removed**, not labelled. The box offered a label; a label does not stop
      a weak test being the template for the next one, it only warns whoever
      copies it. Proved lossless before deleting, with two mutations:
      swallowing the core's refusal now fails **two** tests where the removed one
      made one assertion, and making the two refusals identical is caught on a
      meaning check rather than on the inequality the old test asserted. A
      comment at the section head records what was removed and where each of its
      assertions now lives.
- [x] 14.4 Implementation restored after both mutations, proved with
      `git diff --stat -- dialectica-ui/src` returning empty.

This section removes exactly one test and adds none. Stated as a delta rather
than as a before-and-after count, which is 14.2's own lesson applied to the
sentence describing it — `grep -c "function test_"` and the suite are the two
commands that answer the rest, and neither goes stale.
