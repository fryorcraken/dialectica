# Correctness review — `genesis-in-replies`

Scope: **correctness only**, per dispatch. Verified against PR #130 /
`piece/genesis-in-replies` (commits `46708ba`..`ce8ab44` on top of `2549468`),
checked out locally as `review-genesis-in-replies` from `origin`'s
`piece/genesis-in-replies`. `dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core` run directly (1022 lib tests + 30 end-to-end tests, all green);
QML run through `dialectica-ui/tests/run-qml-tests.sh tst_stoa_screens.qml` (81
passed).

## What was verified and confirmed correct

1. **The diagnosis holds.** `stoa.rs::decode_of_encode_is_the_identity` (line
   467) exists, round-trips several titles including empty string, unicode and
   emoji, and passes. The bug really is an absent field, not a codec defect.

2. **The reply carries the *right* record**, not merely a well-formed one. Every
   new Rust test decodes the `genesis` field and asserts
   `genesis.address() == reply["stoa"]` (the relation), via the shared
   `genesis_of` helper — not a pinned literal. Confirmed by reading
   `wire.rs:12760-12780`.

3. **The error path is sound.** `membership_page_json`'s rewrite from `.map()`
   to a `for` loop is necessary and correct: a `map` closure cannot return from
   the enclosing function, so it could only have swallowed a `canonical_bytes`
   error or panicked. The loop's `return error_json(...)` discards any
   already-pushed items and returns the one failure shape — no path was found
   that can emit an item, or a reply, with an empty `genesis` field.
   `canonical_bytes()`'s only failure mode is `TitleTooLong`, and by the time a
   record reaches either `stoa_reply` or `membership_page_json` its title has
   already been encoded once (to compute the address / to be stored), so the
   failure path exists for defense rather than being reachable today — correctly
   handled either way. `cargo mutants` scoped to `stoa_reply` and
   `membership_page_json` (`-F "stoa_reply|membership_page_json"`) found 4
   mutants, all 4 caught by the existing suite.

4. **The QML `var` reassignment claim is true and the code does what the
   comment says.** `rememberGenesis` (DStoaListScreen.qml:79-91) builds a new
   object (`var next = screen.genesisByStoa; next[stoa] = genesis;
   screen.genesisByStoa = next`) and fires `genesisByStoaChanged()` explicitly.
   This is consistent with QML's known behaviour: a `property var` holding a
   JS object does not emit a property-changed signal on an in-place key
   mutation, only on reassignment of the property itself — bindings such as
   `canShare(row.rowStoa)` that read `genesisByStoa` would not re-evaluate
   without this reassign-and-signal pattern. Not independently re-derived from
   Qt source in this pass, but the code's own behaviour (verified by test,
   below) is consistent with the claim.

5. **Three Rust tests proved to fail first**, independently re-run against
   commit `46708ba` (tests present, implementation absent):
   - `a_creation_reports_the_record_its_address_is_the_hash_of` — panics:
     *"the reply must carry a `genesis` record…"*
   - `a_listed_stoa_carries_the_record_its_address_is_the_hash_of` — same panic
     against the listing shape
   - `a_listed_record_round_trips_through_the_join_the_view_performs` — panics:
     `Option::unwrap() on a None value` (the `row["genesis"]` lookup)

   All three pass at `HEAD`. A fourth Rust test
   (`a_stoa_is_openable_from_the_listing_after_a_restart`) was added later, in
   `57a6f14`, *after* the fix landed rather than before it — it was never
   proven red by the author's own TDD discipline, though the PR description
   only claims "three Rust tests" were proven to fail first, which is accurate.
   The restart test itself is sound: `WireTempDir` (wire.rs:13315) is a real
   `std::fs` temp directory, and `MembershipStore::open` is called twice on the
   same path — a genuine reopen, not an in-memory fixture reused across two
   variable bindings.

6. **The seeding trap was avoided.** The four new QML specs
   (`tst_stoa_screens.qml:565-652`) all go through `makeList()` /
   `Core.bridge` replies and never write `screen.genesisByStoa` directly.
   Direct seeding (`screen.genesisByStoa = held`) does appear in this file at
   lines 669, 718, 1709, 1849 and 1968 — but all five are in *pre-existing*
   tests (`test_a_share_is_offered_only_where_the_view_holds_the_record` and
   others), exactly the tests the proposal says already existed and could not
   have caught this defect, because they assume the very thing that was
   missing.

7. Re-ran the three positive QML specs against a locally reverted
   implementation (removed the two `rememberGenesis` call sites in `reload()`
   and `create()`, leaving the tests untouched) and confirmed exactly 3 of the
   4 new specs go red:
   - `test_a_created_stoa_is_openable_from_the_creation_reply_alone` — FAIL
   - `test_a_listed_stoa_is_openable_from_the_reply_alone` — FAIL
   - `test_a_record_survives_paging_away_from_the_stoa_that_carried_it` — FAIL
   - `test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one` — still
     PASSED (see finding below — this one does not hold in both directions).

## Findings

- [ ] **`spec-writer`** — `openspec/changes/genesis-in-replies/tasks.md` —
      no stage block. This `tasks.md` was written by the `spec-writer` commit
      (`7b0d5e6`) without the roster block README.md §"Two files carry the
      state of a change" describes (`spec-writer.md`'s roster, one row per
      stage including one `code-reviewer` row per dimension). There is no
      `.openspec.yaml` with `skip_specs: true` either, so this is not a
      declared skip — the block appears to have simply never been written.
      **Scenario:** a `closer` reading `tasks.md` for "which reviewers ran"
      finds nothing to check, which is exactly the failure the stage block
      exists to make visible (README.md: "a piece here once reached the edge
      of merge with zero reviewers... neither visible until someone asked").
      **Severity:** process/bookkeeping, not a code defect — filed here rather
      than silently worked around, since inventing a stage block unilaterally
      risks colliding with any sibling reviewer editing the same file
      concurrently (see `reviewers-share-tasks-md` in project memory). Not
      ticking a nonexistent row is preferable to fabricating one; the `closer`
      or runner should reconcile this before merge.

- [ ] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:626-651`
      (`test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one`) and
      `dialectica-ui/src/qml/DStoaListScreen.qml:72-91` (`rememberGenesis`) —
      the "guard direction" of the fourth QML spec is vacuous for the
      empty-string case; it only actually tests the missing-field case.
      **Scenario:** `canShare(stoa)` is defined as `genesisFor(stoa) !== ""`,
      and `genesisFor` returns `""` both when the map holds no entry for
      `stoa` *and* when the map holds an explicit empty string for it — the
      two states are indistinguishable through the only accessor the test
      reads. I removed the `genesis === ""` half of `rememberGenesis`'s guard
      (kept the `typeof genesis !== "string"` check, which still rejects the
      "No field" row since its `item.genesis` is `undefined`) and re-ran
      `tst_stoa_screens.qml`: **81 of 81 still pass, 0 failed** — the mutation
      that the docstring at DStoaListScreen.qml:76 explicitly warns against
      ("Writing `""` in would make `canShare` true... and would send that same
      `""` to `read_feed`") is not caught by this test, or by any other test in
      the suite (no test inspects `genesisByStoa`'s contents directly via
      `Object.keys` or similar). The task list's claim at `tasks.md` 4.5 —
      "4.4 holds in both directions by design; it pins that nothing writes
      `\"\"`" — is true only for the *missing-field* direction, not the
      *explicit-empty-string* direction, which is the direction the
      surrounding comments and docstring treat as the more important of the
      two (it is the one that would silently reproduce the owner's original
      bug through a visible, clickable share button). **Measured:** 81/81 pass
      under this mutation, where the docstring's own stated purpose implies it
      should not.

## Areas reviewed and clean

- `wire.rs` `create_stoa`, `join_stoa`, `list_stoas`, `stoa_reply`,
  `membership_page_json`: encode-failure handling, hex round-trip via
  `Core.qml`'s pass-through (`root.call` does no field filtering, confirmed by
  reading `Core.qml:182-206`), and the address-is-hash-of-record relation are
  all correct.
- `cargo mutants` on the two touched functions: 4/4 mutants caught, no gap.
- The restart test's use of a real file and genuine reopen: confirmed.
- The absence of direct-seeding in the four new QML tests: confirmed by line
  ranges.
