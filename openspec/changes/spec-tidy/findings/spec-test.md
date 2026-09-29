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

## Not reached — noted as limits, not findings

- **Mutation testing was attempted and blocked, not completed.** One `Edit`
  attempt (flipping `sqlite.rs`'s `author` column write to store `stoa` bytes,
  to independently reproduce the mutation `a98c434`'s commit message claims)
  and one read-only `Grep` on the same file were both refused by the harness's
  auto-mode classifier ("Modify Shared Resources") — the same failure mode
  recorded in this repo's git history for `wire.rs` on a prior piece. I did not
  retry through another route, per the standing instruction not to pursue a
  denied outcome by another tool. This review therefore rests on the
  self-reported mutation evidence in commits `a98c434` and `5f95e7e` (both
  ancestors of `HEAD`, both describing a specific proved-red mutation and its
  revert) rather than an independently reproduced one.
- **`identity-onboarding`'s two requirements promoted from `first-run-identity`**
  ("A peer with no master key can obtain one without naming a Stoa" and
  "Obtaining a master key never replaces one") were not checked against
  `wire.rs`'s `mint_master_key` tests before time ran out. The commit message
  for `a98c434` claims these are pinned by "wire.rs's mint_master_key tests";
  not independently verified here.
- **`thread-view`'s and `moderation-view`'s remaining requirements** (beyond
  the two `NO SPEC` markers and the folded route requirements above) were not
  individually walked against `tst_thread_nesting.qml` /
  `tst_thread_states.qml` / `tst_moderation_screen.qml` scenario-by-scenario.
  Relied on `a98c434`'s coverage survey plus the spot checks above.
- **`stoa-navigation-view`'s row-separator requirement** (from
  `ui-remaining-screens`, task 1.4) was not independently checked against
  `tst_stoa_screens.qml`.
- **`sqlite.rs`'s second new test**,
  `a_read_against_storage_broken_after_open_is_a_failure_not_an_empty_result`
  (op-log's "Every read is defined over the ops the peer happens to hold"),
  was not read in full; only its commit-message description was available.

None of the above surfaced a suspected defect — they are unchecked, not
flagged. Given the mutation-tooling block, I have one settled finding to
report: the mutation-testing obligation in part 2 of this review could not be
discharged independently this session.

- [ ] **runner** — mutation testing for this piece's spec-test review was
      blocked both times it was attempted (`Edit` and `Grep` on `sqlite.rs`,
      denied as "Modify Shared Resources"). The `the_stored_author_is_the_signer_regardless_of_moderator_status`
      mutation this review most wanted to re-run independently (swap the
      `author` column write for `stoa` bytes) rests entirely on `a98c434`'s
      self-reported result. If an independent mutation run is needed before
      merge, it needs a session whose permissions allow editing `sqlite.rs`,
      or the runner's own confirmation that the self-reported evidence is
      sufficient.

## Verdict

- [x] **none** — no test-vs-spec defect found in what this review reached
      (see limits above for what was not reached under the time budget).
