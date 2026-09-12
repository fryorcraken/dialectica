## 1. The membership store

- [x] 1.1 Add `dialectica-core/src/membership.rs` with `MembershipError`,
      `MEMBERSHIP_LAYOUT_VERSION` and a `Membership` row type; verify
      `cargo build -p dialectica-core --all-targets` compiles and
      `the_membership_layout_version_is_pinned_to_a_known_answer` passes against
      a hardcoded `1`.
- [x] 1.2 Implement `MembershipStore::open` / `in_memory` with its own
      `PRAGMA user_version`, refusing an unknown layout in both directions;
      verify tests for a fresh store, a higher version and a negative version
      each pass, with the found and expected numbers hardcoded on both sides.
- [x] 1.3 Implement `join(address, genesis)` verifying the record against the
      address before any statement runs, with `INSERT OR IGNORE`; verify a
      matching record is recorded, a record differing in any field is refused,
      and a malformed record is refused — each asserting the store holds no new
      row afterwards.
- [x] 1.4 Implement `get`, `list(page, per_page)` and `len`, reading the record
      from its stored canonical bytes; verify a retained record read back
      recomputes to the address it is filed under, and that a corrupt
      `genesis_bytes` row is reported rather than skipped.
- [x] 1.5 Verify idempotence and non-destructiveness: a second `join` of the same
      address reports success, leaves exactly one row, and leaves the retained
      record byte-identical.

## 2. Persistence and the op-log boundary

- [x] 2.1 Verify membership survives a restart: create and join against a real
      file, drop the store, reopen, and assert both are listed and each still
      answers its founding title and policy.
      (`memberships_outlive_the_store_object_that_recorded_them`)
- [x] 2.2 Verify a refused join leaves nothing across a restart.
      (`a_refused_join_leaves_nothing_behind_a_restart`)
- [x] 2.3 Verify an op store predating membership stays readable — **satisfied by
      construction rather than by a test that could fail.** Nothing in this change
      edits `SqliteOpLog`: no schema, no version, no refusal. The membership store
      is a separate file, so there is no store this change can make unreadable and
      no code path a test could exercise to try. Recorded here rather than
      silently dropped; `design.md` has the argument and the rejected alternatives.
- [x] 2.4 Verify a membership is recordable into such a store — same answer, and
      the same reason: `MembershipStore::open` creates its own file whether or not
      an op store exists beside it, and `a_fresh_store_is_created_rather_than_refused`
      pins that the never-stamped case is created rather than refused.
- [x] 2.5 Verify ops never create membership: append many ops addressed to an
      unjoined Stoa and assert the listing is unchanged and an existing
      membership's retained record is byte-identical.
      (`wire::an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership`)
- [x] 2.6 Verify membership is not lost for want of ops.
      (`membership_is_not_lost_because_a_stoa_has_no_ops`,
      `wire::an_empty_op_log_does_not_empty_the_listing`)

## 3. The wire handlers

- [x] 3.1 Add `create_stoa` to `wire.rs`, taking a title and a key lookup
      closure, refusing an over-cap title before recording anything.
      (`creation_returns_the_address_of_the_record_it_built`,
      `the_creator_is_the_callers_own_key_and_no_creator_is_accepted_from_the_request`)
- [x] 3.2 Verify creation without a usable key fails and records no Stoa, across
      four keystore states.
      (`creation_without_a_usable_key_fails_and_records_nothing`)
      **"Mints no key" is satisfied by construction, not by that test**, and the
      checkbox used to claim otherwise. The test hands `create_stoa` a closure
      returning `Err`, so no `Keystore` is in scope and nothing it asserts could
      distinguish a handler that called `Keystore::generate()` and then returned
      the error anyway. What actually holds the requirement is that **there is no
      path from this handler to `Keystore::generate()`** — the handler takes a
      `PublicKey` through a closure and has no constructor reachable from it. The
      test's own comment was honest about this; the checkbox was not.
- [x] 3.3 Verify the title bounds and that bidi/zero-width characters survive.
      (`an_over_long_title_creates_nothing`,
      `a_title_at_the_maximum_length_creates_a_stoa`,
      `an_empty_title_is_accepted_rather_than_refused`,
      `a_title_carrying_control_or_bidirectional_characters_is_not_rejected_for_that_reason`)
- [x] 3.4 Verify same-creator-same-title twice is one Stoa, and two titles two.
      (`the_same_creator_and_title_reach_the_same_stoa`, `two_titles_are_two_stoas`)
- [x] 3.5 Add `join_stoa` taking `{stoa, genesis}`.
      (`a_matching_record_joins_and_the_reply_carries_the_address_and_founding_title`,
      `a_record_that_does_not_match_the_address_is_refused_and_joins_neither_stoa`,
      `a_malformed_record_is_refused_without_a_membership`)
- [x] 3.6 Add `list_stoas` with the pagination envelope.
      (`the_listing_envelope_is_the_ecosystems_pagination_shape`,
      `a_peer_in_no_stoa_lists_nothing_and_reports_no_failure`,
      `every_stoa_is_reachable_by_paging_and_appears_once`)
- [x] 3.7 Verify every handler is guarded and answers one failure shape across
      every hostile request shape the spec enumerates.
      (`a_hostile_request_is_an_error_rather_than_an_abort_and_carries_no_result`,
      `a_failed_call_records_nothing_and_disturbs_no_retained_record`)
- [x] 3.8 Verify the reported title is named as founding and no bare `title` key
      is present. (`a_listed_title_is_named_as_founding_and_never_as_a_bare_title`)

## 4. The module surface

- [x] 4.1 Add `create_stoa`, `join_stoa` and `list_stoas` to the
      `DialecticaModule` trait with doc comments stating request and reply shapes.
- [x] 4.2 Forward each from the adapter, deriving the membership and keystore
      paths from the host's persistence path. The repeated
      "has the host told us where storage is" guard was extracted to
      `Dialectica::storage_dir` **first**, as its own behaviour-preserving step:
      it had two inline copies and this change would have made it five.
- [x] 4.3 Re-export the handlers, plus `with_membership_store` and
      `membership_path_in`, from `dialectica-core/src/lib.rs`.
- [x] 4.4 Add `Keystore::identity_key` / `identity_public_key` / `identity_address`
      — the root secret used directly, per PLAN §5.2's MVP one-identity rule — and
      name it as the creator in `create_stoa` and as the identity in
      `get_capabilities`, so the creator of a Stoa can moderate it. Pinned by a
      known-answer test, since the value is address-determining.
      (`the_creator_of_a_stoa_this_keystore_made_can_moderate_it`,
      `the_identity_key_is_pinned_to_a_known_answer`,
      `the_identity_key_is_the_root_used_directly_and_not_a_derived_one`,
      `the_identity_key_is_stable_and_differs_between_keystores`,
      `no_stoa_address_a_caller_can_name_reaches_the_identity_key`)

      **Replaces the original 4.4**, which added `Keystore::creator_public_key`
      deriving from a synthetic all-zero domain. That key was not the key the peer
      posts with, so `Moderators::of(genesis).contains(posting_key)` was false for
      a Stoa's own creator. `CREATOR_KEY_DOMAIN` and its three now-superseded
      tests are deleted; `design.md` carries the alternatives and the security
      argument.

## 4a. Fixes from review

- [x] 4a.1 Fix `MembershipStore::list` answering empty for a valid page 0 at an
      over-large `per_page`: the `i64::try_from` guard tested the incremented
      limit. `per_page` now saturates; only `page` refuses.
      (`a_per_page_at_the_conversion_boundary_still_lists_the_whole_store`)
- [x] 4a.2 Fix `per_page == 0` reporting a page both empty and not-the-last, which
      made paging to exhaustion never terminate. Reshaped the remaining boundary
      to one `split_off` so the page and `has_more` cannot disagree.
      (`a_per_page_of_zero_terminates_rather_than_paging_forever`)
- [x] 4a.3 Rename `MembershipError::UndecodableRecord` to `UnencodableRecord` and
      fix its message: its only construction site is the **encode** side, so a
      caller was told its record "could not be read" when nothing was read.
      (`an_encode_failure_does_not_report_itself_as_a_failure_to_read`)
- [x] 4a.4 Add the missing `false` case for `MembershipStore::is_empty`, which
      survived `cargo mutants` replaced by `Ok(true)`.
      (`an_empty_store_and_a_populated_one_disagree_about_being_empty`)
- [x] 4a.5 Correct three places crediting **version independence** with the
      boundary between the op store and the membership store. Both layout
      constants are `1`, so the version check separates nothing; `check_layout`
      naming columns is what refuses an op-log file. The recovery asymmetry claim
      is kept — it is a property of the separate file and survives both constants
      being `1`. (`membership.rs`'s constant doc, the independence test's comment,
      and `design.md`.)
- [x] 4a.6 Record in `design.md` what the code held alone: the creator key's
      rejected alternatives, the `"open"` wire token as the fifth `NO SPEC`
      subject, the single witness for `INSERT OR IGNORE` over `OR REPLACE` and
      that the wire hides the distinction, and that two files foreclose a
      transaction spanning membership and ops.
- [x] 4a.7 Correct `docs/PLAN.md` §9.1's `joinStoa({address})` — a hash verifies a
      record but cannot reconstruct one — and strike its duplicated verification
      reasoning down to a line saying the three calls exist. `design.md` argues
      the departure.

## 5. The UI brief

- [x] 5.1 Fix `docs/UI-BRIEF.md` where this change makes it wrong: the Stoa list
      no longer claims to show a current title, the release summary no longer says
      a Stoa is joined by pasting an address alone, obligation 2b extends to the
      list screen, and a *Creating a Stoa* section states the same-title-is-one-Stoa
      behaviour a designer would otherwise design a collision dialog for.

## 6. Gates

- [x] 6.1 `cargo test -p dialectica -p dialectica-core` passes. The count is what
      the command reports; do not write it here, per CLAUDE.md — a number in a
      checklist cannot fail loudly when it drifts.
- [x] 6.2 `cargo fmt --check` (no `-p`) exits 0. **`dialectica-core` carries
      pre-existing format drift that this gate structurally cannot see** — the
      known CI gap, since `cargo fmt` does not follow path dependencies. The two
      files this change adds to heavily (`membership.rs`, `wire.rs`) were
      formatted; `keystore.rs` and `identity.rs` were **not**, because their drift
      is entirely pre-existing and reformatting them would bury a 5-line and a
      135-line addition under dozens of unrelated reflows. Verified with
      `rustfmt --check` per file that every line this change adds is clean.
- [x] 6.3 `cargo clippy -p dialectica-core --all-targets -- -D warnings` is clean.
- [x] 6.4 `cargo build -p dialectica-core --all-targets` succeeds.
- [x] 6.5 `openspec validate stoa-lifecycle --strict` passes.
- [x] 6.6 The `logos-rust-sdk-src` symlink is removed before committing.
