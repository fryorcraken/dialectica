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

- [x] **`tester`** — `thread-view`'s "The reply composer is wired to the
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

      **fixed**, with a wrinkle the finding's literal framing did not
      anticipate. `DThreadScreen.qml`'s composer binds `parentOp:
      screen.threadId` — the screen's own navigation-supplied property,
      which the file's header comment names "The ROOT POST's op id" — and
      never reads the *read's* item shape at all. So the concrete rendering
      of "the view holds no op id for [the root]" for this screen is
      `threadId === ""`, not `items[0].id` being absent; a literal test that
      set the root item's `id` to missing while leaving `threadId: "root1"`
      still produces a well-formed `publish_reply` call (`parent: "root1"`),
      because that value never touches the item's own field. Added two
      tests in `tst_thread_reply.qml`, covering both readings so neither gap
      is left open:

      - `test_no_reply_affordance_is_reachable_with_no_root_identifier`
        (`threadId: ""`, `stoaAddress` set): the literal scenario, translated
        into this screen's own definition of "the op id it holds for the
        root". **Predicted / observed:** removing the `screen.threadId ===
        ""` guard from `reload()` together with the `&&
        screen.threadId !== ""` clause on `replyComposerOpen`'s `visible`
        turned it red (`'the open-gate composer group is not rendered...'
        returned FALSE`) — predicted and observed match exactly. Restored
        both lines; `git diff --stat` on `DThreadScreen.qml` is empty.
      - `test_a_root_items_own_missing_id_does_not_reach_the_reply_parent`
        (root item from `read_thread` carries no `id`, `threadId: "root1"`):
        pins the actual regression risk the finding named — a future change
        deriving `parentOp` from the item's own `id` instead of `threadId`.
        **Predicted / observed:** rebinding `parentOp` to
        `screen.items.length > 0 ? screen.itemId(screen.items[0]) :
        screen.threadId` turned it red exactly as predicted (`Actual (): ` /
        `Expected (): root1`), since `itemId` returns `""` for a missing
        `id`. Restored; `git diff --stat` empty.

      Both pass against the untouched implementation (16 passed, 0 failed,
      `dialectica-ui/tests/tst_thread_reply.qml`).
- [x] **`tester`** — `thread-view`'s "Every string rendered from an item is
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

      **fixed** — added
      `test_the_screen_threads_the_items_sanitiser_report_through_to_the_render`
      to `tst_thread_reply.qml`: a root item with `body: {text: "...",
      removed: 2, marked: 3}`, rendered by the real `DThreadScreen`, and
      asserts the `SanitisedText` instance it renders through carries
      `removedCount === 2`, `markedCount === 3` and the body text unaltered.
      Located it by a recursive walk over `toString()` for the type name
      rather than `objectName` — `SanitisedText` carries none in
      `DThreadScreen.qml`, and adding one would be an implementation change
      made only to serve a test.

      Passes against the untouched implementation (17 passed, 0 failed).
      **Predicted / observed:** mutating the `value:` binding at
      `DThreadScreen.qml`'s post-body `SanitisedText` from
      `post.modelData.body` to `{ text: post.modelData.body.text, removed:
      0, marked: 0 }` (the exact "body.text alone, dropping removed/marked"
      defect the finding names) turned it red exactly as predicted — `the
      item's own removed count reaches the shared component / Actual (): 0 /
      Expected (): 2` — with every other test in the file still green.
      Restored; `git diff --stat` on `DThreadScreen.qml` is empty.
- [x] **`tester`** — `thread-view`'s "A revised post is marked as revised…"
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

      **fixed** — added
      `test_the_marker_and_the_inert_row_state_nothing_about_earlier_content`
      to `tst_thread_reply.qml`, covering both scenarios with hardcoded,
      exact-match assertions (per the finding's own point that only a
      hardcoded addition could violate either SHALL NOT):
      - Counts exact occurrences of the literal string `"edited"` across the
        whole screen and requires exactly 1 — not "does some Text say
        edited", which a `"edited (from rev3)"` mutation would still satisfy.
      - Collects every rendered string inside the affordance row
        (`earlierVersionsInert`'s parent) and requires the set to be
        exactly `{"read the earlier versions", "NOT YET AVAILABLE"}`.

      Passes against the untouched implementation (18 passed, 0 failed).
      **Predicted / observed, both mutations:**
      - Changed `PostHeader.qml`'s marker text from `"edited"` to `"edited
        (from an earlier version)"` — predicted the exact-count assertion
        would go to 0; observed `Actual (): 0 / Expected (): 1`. Restored.
      - Added a third `Text { text: "the previous text is no longer shown"
        }` sibling inside `DThreadScreen.qml`'s affordance row — predicted
        the row's text count would go to 3; observed `Actual (): 3 /
        Expected (): 2`. Restored; `git diff --stat` on both files is empty.
- [x] **`tester`** — `thread-view`'s "No score, tally or vote count is
      rendered on the thread screen" scenario "No ordering is offered as
      vote-based" has no test (checked `tst_thread_reply.qml`,
      `tst_thread_states.qml`, `tst_thread_nesting.qml` for
      "vote"/"reorder"/"order" — no hit beyond the score-field check).
      Likely vacuous today since no such control exists in this MVP, which is
      why this is the lowest severity of the four: flagging it so it is not
      forgotten once ordering controls are built, not because a defect is
      suspected now.

      **deferred** — confirmed by grepping `DThreadScreen.qml` and
      `VoteControl.qml` for `reorder`/`sortBy`/`orderBy` and any second
      `VoteControl`-like affordance: the only control on the thread screen is
      the single, non-interactive `VoteControl` per row (`showScore: false,
      interactive: false`), and there is no ordering mechanism of ANY kind —
      vote-based or otherwise — anywhere in this screen. A test for "no
      control orders items by votes" would necessarily pass today regardless
      of whether a guard exists, because there is nothing to guard: it cannot
      fail for the reason it would name, since the feature space it is
      checking is empty. Writing it now would be exactly the "reports safety
      that was never checked" case the brief warns against. Deferred to
      whichever future change first adds an ordering or re-ordering control to
      the thread screen — that change should add this test alongside the
      control, at the point where "no ordering is vote-based" becomes a real
      distinction to draw. No test added.

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

- [x] **re-review round 1 `b4378cf5..218acde3`: no findings** — read the range
      diff and full text of `dialectica-ui/tests/tst_thread_reply.qml` (the four
      new/changed tests answering the reply-composer, sanitiser-threading,
      revised-marker and vote-ordering findings above), the comment-only hunk
      of `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs`, and
      `thread-view`'s "The reply composer is wired…", "Every string rendered
      from an item…", "A revised post is marked as revised…",
      "The earlier-versions affordance is inert…" and "No score, tally or vote
      count…" requirements in `openspec/specs/thread-view/spec.md`. All four
      new tests assert against something the untouched implementation did not
      merely echo back (the composer visibility/no-publish guard on
      `threadId === ""`; a regression guard that the composer's `parent` is
      never derived from the item's own `id` field; the `SanitisedText`
      instance's `removedCount`/`markedCount`/text asserted through the real
      screen rather than the shared component alone; exact-count string
      collection for `"edited"` and the inert row's two static strings) — each
      is corroborated in this file's existing predicted/observed mutation
      records (lines 156–170, 199–207, 236–243), which name the exact line
      changed, the exact failure observed, and the restore. I independently
      attempted one further mutation (removing the `&& screen.threadId !== ""`
      clause from `DThreadScreen.qml:672`'s `replyComposerOpen.visible`
      binding, the same line the tester's own predicted/observed entry names)
      to cross-check `test_no_reply_affordance_is_reachable_with_no_root_
      identifier`; the edit was refused by the environment's permission
      classifier ("Modify Shared Resources") before any test ran, the same
      block the runner already recorded and accepted for `sqlite.rs` in this
      file's process finding above. No mutation of mine is in the tree. Given
      that prior precedent and the tester's own specific, restorable
      predicted/observed entries, I did not press further within this round's
      budget. The composer tests' translation of "an item carrying no op id"
      into this screen's own `threadId === ""` (test 1) plus a direct guard
      against deriving `parent` from the item's own field instead (test 2) is
      a disclosed, reasoned choice given this screen exposes exactly one
      reply affordance, on the root, whose id the view tracks as `threadId`
      rather than as a field on the read item — not a gap. Deferring "No
      ordering is offered as vote-based" with no test is an acceptable answer:
      confirmed by reading `DThreadScreen.qml`/`VoteControl.qml` are not part
      of this round's diff and no ordering control of any kind exists on this
      screen today, a test asserting its absence could not be shown to fail
      by any mutation (there is no line to remove that would make it pass
      falsely), which is exactly the vacuous-safety shape this role's
      guidance warns against; deferring to the change that first adds such a
      control is the correct point to add it. Clean.
