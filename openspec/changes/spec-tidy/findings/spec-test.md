# spec-test review — spec-tidy

Scope: the five capabilities this piece promoted or changed through archives
(`op-log`, `identity-onboarding`, `thread-view`, `stoa-navigation-view`,
`moderation-view`) and this change's own deltas under
`openspec/changes/spec-tidy/specs/`. Per the runner's correction, the
`identity-onboarding`/`thread-read` malformed-`index` material is #166's
(`position-and-index`), arrived via merge, and is out of this piece's scope —
not reviewed here.

## What was checked and held up

- **op-log's promoted `sqlite-projection` requirement** ("A persistent log
  stores the inputs a ranking is computed from, never a ranking"): the
  `author`-column half is pinned by
  `the_stored_author_is_the_signer_regardless_of_moderator_status` in
  `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs`. Read in full: it
  signs with two distinct keys, asserts the stored bytes equal each signer's
  own public key (hardcoded per-signer, not merely "differ"), and the fixture
  gives the non-creator author no standing — so it can't pass by the write
  accidentally storing a constant or the wrong field. Not tautological.
- **op-log's "Every read is defined over the ops the peer happens to hold"**,
  the storage-failure half: `a_read_against_storage_broken_after_open_is_a_
  failure_not_an_empty_result` read in full. It first asserts the fixture is
  genuinely non-empty (`len() == 1`), breaks storage through the log's own
  already-open connection (`DROP TABLE ops`, bypassing `open`'s guard
  entirely, which is necessary since `check_layout` would otherwise catch a
  reopen first), then asserts `iter()` returns `Err(OpLogError::Storage(_))`
  rather than `Ok(vec![])` or a panic. This cannot pass on an implementation
  that swallows the error into an empty result, matching the scenario "A
  storage failure is reported, not confused with emptiness" exactly.
- **Both reworded `NO SPEC:` markers** cite requirements that say what they
  claim, checked verbatim against the live specs:
  - `tst_moderation_screen.qml:312` cites `moderation-view`'s "The screen does
    not state or imply that the user moderates the Stoa" (scenario "No
    moderator standing is claimed for the reader") — heading and scenario
    both exist verbatim in `openspec/specs/moderation-view/spec.md`.
  - `tst_stoa_screens.qml:2879` cites `stoa-navigation-view`'s "Every number
    rendered is one this peer can actually answer" (scenario "A row's
    placeholder does not assert emptiness") — both exist verbatim in
    `openspec/specs/stoa-navigation-view/spec.md`, and the emptiness-ban
    paragraph the moderation-screen correction restored is present.
- **`view-navigation`'s folded requirements.** Read the delta
  (`openspec/changes/spec-tidy/specs/view-navigation/spec.md`) against the two
  `REMOVED` blocks in `thread-view`'s and `moderation-view`'s deltas. Every
  scenario named by each `REMOVED` block's Migration is present in the
  `view-navigation` delta with matching content (one is folded as a new `AND`
  clause on an existing scenario, one is merged as a sentence rather than kept
  as its own scenario — both as the proposal describes). Found tests for each:
  `test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op`,
  `test_the_feed_is_reached_again_from_the_thread`,
  `test_a_thread_that_could_not_be_read_still_offers_the_way_back` and
  `test_the_way_out_survives_a_refused_read` (`tst_thread_reply.qml:265`),
  `no thread read is made before a thread has been chosen`
  (`tst_thread_states.qml:434`), and
  `test_the_screen_holds_no_usable_default_for_what_it_renders`
  (`tst_thread_reply.qml:292`) for the thread-side scenarios; and
  `test_the_feed_offers_a_route_into_moderation`,
  `test_the_moderation_screen_can_be_left_after_pressing_its_controls`,
  `test_cancel_leaves_the_moderation_screen` for the moderation-route ones.
- **The four prose-only deltas** (`moderation-resolution`, `stoa-genesis`,
  `generated-names`, `feed-view`): read each and confirmed no scenario content
  changed, only citations/wording, so no new test is owed. Checked every
  citation resolves: `post-revision`'s "A post is never edited in place" and
  `generated-names`' own "Core exposes the derivation to a caller" both exist
  verbatim as `### Requirement:` headings; `stoa-navigation-view`'s "Joining
  shows what is being joined, and joins nothing until the user acts" exists
  verbatim; the archived `op-clock` `design.md`'s "What this deliberately does
  not do" section exists at the cited path. `feed-view`'s corrected clause ("it
  has nothing to bind while the field is absent") reads as the proposal
  describes, not as the self-contradictory original.
- **`test_no_item_is_added_by_a_publish`** (`tst_thread_reply.qml:143`, the
  test named in the brief): reads real, not tautological — it hardcodes the
  expected count (1) both before and after publish, separately asserts the
  re-read happened (`reads >= 2`), so it cannot pass on a screen that simply
  never reads again.
- **`identity-onboarding`'s two requirements promoted from `first-run-identity`**
  ("A peer with no master key can obtain one without naming a Stoa" and
  "Obtaining a master key never replaces one"), checked scenario by scenario
  against `wire.rs`'s mint tests:
  - "A peer with no master key obtains one" →
    `a_first_run_mint_writes_a_master_key_and_reports_it_as_new` — asserts
    against the key derived independently from the file on disk, not an echo
    of the reply.
  - "The key obtained is the key a Stoa creation uses" →
    `a_minted_key_is_the_one_a_stoa_creation_names_as_creator` — an
    end-to-end relation (no hardcoded literal, since the root is generated).
  - "Creation is still refused before a key exists" →
    `creation_still_fails_before_a_mint`, explicitly paired with the test
    above so the two-explanations trap can't hide a `create_stoa` that
    succeeds unconditionally.
  - "No per-Stoa choice is recorded" → `a_mint_records_no_per_stoa_choice`.
  - "Protection at rest is reported" → `a_mint_with_no_passphrase_reports_
    the_key_as_unencrypted` / `a_mint_under_a_passphrase_reports_the_key_as_
    encrypted` (both directions, not just the constant one).
  - "A second request replaces nothing" → `a_mint_over_an_existing_keystore_
    replaces_nothing_and_reports_it_as_not_new` — compares the keystore
    file's raw bytes, not just the reported key, specifically to catch a
    same-root re-encrypt under a fresh salt; its own comment documents a
    proved-red mutation (deleting the `exists()` guard).
  - "A second request is a success, not a refusal" → same test,
    `wasNew: false` and no error.
  - "An existing key's protection is reported from the key itself" →
    `an_existing_keystores_protection_is_read_off_the_file_and_not_off_the_
    argument` — deliberately chose the direction that can actually
    distinguish "read from file" from "read from argument" (the reverse
    direction would be vacuously true, per its own comment), and documents a
    proved-red mutation.
  All scenarios covered, no tautological test found.
- **`stoa-navigation-view`'s row-separator requirement** (from
  `ui-remaining-screens`, promoted with no correction): all three scenarios
  covered in `tst_stoa_screens.qml` — `test_every_rendered_row_is_separated_
  from_the_next` (3 rows, count-against-count rather than existence, so it
  can't be satisfied by a single separator for the whole list), `test_one_
  row_draws_exactly_one_boundary` and `test_the_row_count_and_the_separator_
  count_move_together` (the 1-row case specifically catches a separator
  dropped on the last row, and the 1-vs-4 pairing catches a separator hoisted
  out of the delegate into a fixed count), and `test_an_empty_list_draws_no_
  row_boundary` (catches a separator hoisted to the list container, which
  would render even with no rows). Well-reasoned test design; no gap. The
  archive commit (`2fa2b11`) also records that a prior spec-test review
  already reproduced both mutations tasks.md claims for this requirement.

## New findings from the completed coverage walk

- [ ] **`tester`** — `thread-view`'s "The reply composer is wired to the
      publish call and names the parent it is under" has a scenario "A post
      carrying no identifier offers no working reply affordance" (`WHEN` the
      screen is given an item carrying no op id and its reply affordance is
      acted on, `THEN` no publish call is made and no request is sent
      omitting the field that would have named the parent). No test in
      `tst_thread_reply.qml` gives the root item no `id` — every fixture
      (`makeScreen`, `rootItem()`) sets one. This screen offers exactly one
      composer, on the root, so the gap is concrete: a regression that made
      the reply call fire with `parent: undefined` for a rootless item would
      pass every existing test. Severity: moderate — this is the guard the
      requirement's own reasoning calls out ("no value derived from a
      missing field reach a core call").
- [ ] **`tester`** — `thread-view`'s "Every string rendered from an item is
      rendered as the read supplied it" (3 scenarios: a body rendered as
      returned, a sanitiser report renderable, a marked character not
      corrected) has no test in `tst_thread_states.qml`, `tst_thread_reply.
      qml` or `tst_thread_nesting.qml` — none of the three files mentions
      "sanitis" at all. `tst_sanitised_text.qml` proves a shared component
      can render a sanitiser report correctly, but that is a component test,
      not an integration one: it does not show the thread screen actually
      passes a thread item's sanitiser-report fields through to that
      component. This is the "test at a layer that cannot observe the
      behaviour" shape — a wiring defect between the item model and the
      render path would not be caught by either file alone. Severity:
      moderate.
- [ ] **`tester`** — `thread-view`'s "A revised post is marked as revised…"
      scenario "The marker claims nothing about the earlier version", and
      "The earlier-versions affordance is inert…" scenario "No earlier
      version text is rendered", have no explicit test. `test_a_revised_item_
      is_marked` / `test_an_unrevised_item_carries_no_marker` /
      `test_the_marker_follows_isRevised_not_the_identifiers` only assert a
      boolean `edited` flag, never that no text claiming to be prior content
      is rendered; `test_the_earlier_versions_control_reaches_no_core_call`
      proves no call is made but does not check what is rendered. Lower
      severity than the two findings above, because there is currently no
      data channel for earlier-version text to travel through at all (no
      call exists, per the sibling requirement), so a violation would need a
      hardcoded string in the view rather than a data-flow bug — but the
      normative "SHALL NOT" is still unpinned.
- [ ] **`tester`** — `thread-view`'s "No score, tally or vote count is
      rendered on the thread screen" scenario "No ordering is offered as
      vote-based" has no test (checked `tst_thread_reply.qml`,
      `tst_thread_states.qml`, `tst_thread_nesting.qml` for
      "vote"/"reorder"/"order" — no hit beyond the score-field check).
      Likely vacuous today since no such control exists in this MVP, which is
      why this is the lowest severity of the four: flagging it so it is not
      forgotten once ordering controls are built, not because a defect is
      suspected now.

## Process finding — resolved by the runner

- [x] **runner** — mutation testing for this piece's spec-test review was
      blocked both times it was attempted (`Edit` and `Grep` on `sqlite.rs`,
      denied as "Modify Shared Resources"). **Runner's decision: sufficient.**
      The tester independently ran the equivalent mutation in its own
      session (swapping the `author`-column write for `stoa` bytes,
      predicting and observing mismatched 32-byte arrays, then restoring),
      recorded in the tester's hand-back and `tasks.md`'s tests row. That is
      a second independent measurement of the same mutation my blocked
      attempt would have been a third of.

## Verdict

Four new coverage-gap findings above (all `tester`'s, none blocking on their
own reading of severity — three moderate/low, one low/vacuous), plus the
resolved process finding. Everything else walked in this review — all five
capabilities' promoted requirements, the two `NO SPEC` markers, the folded
`view-navigation` requirements, the four prose-only deltas, and the two named
`sqlite.rs`/`tst_thread_reply.qml` tests — held up with no test-vs-spec
defect.
