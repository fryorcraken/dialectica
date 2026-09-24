# Correctness review — `join-preview-getstoa`

Dimension covered: **correctness only** (does the code do what the spec says,
on every path including malformed and adversarial input; can any panic be
reached; is any guard missing or wrong). Security, readability and
architecture are other reviewers' rows.

## What was checked

- `dialectica/rust-lib/dialectica-core/src/stoa.rs` — `BLANK_CHARACTERS`,
  `is_blank_title`, `Genesis::canonical_bytes`/`decode` blank-title refusal
  and its ordering against `TrailingBytes`/`TitleTooLong`/`LengthMismatch`.
- `dialectica/rust-lib/dialectica-core/src/op.rs` — `Op::check_admitted`,
  `Op::encode`/`decode`, the `canonical_bytes`/`encode` split, and that the
  guard is scoped to `StoaMetadata` only.
- `dialectica/rust-lib/dialectica-core/src/stoa_metadata.rs` — the resolver's
  four binding conditions (kind, blank title, authority, ordering), the
  fallback path, and the failure-vs-fallback distinction on an unreadable log.
- `dialectica/rust-lib/dialectica-core/src/membership.rs` — `Membership::verified`,
  `MembershipStore::join`/`get`/`list`, and the retained-corrupt-record path
  (`decode_row`, `CorruptEntry`, not skipped/migrated).
- `dialectica/rust-lib/dialectica-core/src/wire.rs` — `genesis_for`,
  `get_stoa`, `create_stoa`, `join_stoa`, `list_stoas`, and the
  allocate-before-decode hex-length bound.
- `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs` — `append`'s
  encode-before-write refusal (`Unencodable`), `ordered_read`/`decode_entry`'s
  whole-read failure on a corrupt entry (not skip-and-continue).
- `dialectica/rust-lib/dialectica-core/src/transport.rs` — `publish`'s
  encode-before-append ordering.
- `dialectica-ui/src/qml/Core.qml` — `stoaMetadataFrom`, `isBlankTitle`
  (UTF-16 code-unit walk), `getStoa` wrapper.
- `dialectica-ui/src/qml/DJoinScreen.qml` — `currentLookup`/`currentOutcome`
  reference-keyed filtering, `foundingTitle`/`currentTitle`/`currentDescription`
  derivation, `lookalikes`, the failure/fallback panels.
- `dialectica-ui/src/qml/DStoaListScreen.qml` — comment-only diff, verified
  against no behaviour change.
- Cross-checked every changed requirement in the six spec deltas
  (`op-log`, `stoa-genesis`, `op-format`, `stoa-membership`, `stoa-metadata`,
  `stoa-navigation-view`) against the implementing code.
- Owner rulings from the issue (lookalike stays keyed on founding titles;
  blank title invalid everywhere; a store already holding a blank-titled
  record is neither migrated nor skipped) — verified each is honoured (see
  `lookalikes` reading only `foundingTitle`;
  `a_retained_blank_titled_record_is_reported_neither_skipped_nor_migrated`;
  `op-log`'s "A stored entry that does not decode fails every read").

## Mutation probes run (and reverted)

All of the following were applied by hand in this worktree, run against the
suite, confirmed caught, then reverted. `git status` is clean; nothing here
reached a commit.

1. Deleted the blank-title check in `stoa_metadata::binding_metadata`
   (`stoa_metadata.rs:176-178`). **Caught**: 2 of 1125 core tests failed
   (`a_metadata_op_carrying_a_blank_title_does_not_bind`,
   `a_later_blank_titled_op_does_not_displace_a_binding_one`), matching
   `design.md` decision 4's own measurement.
2. Swapped `check_admitted`'s field from `title` to `description`
   (`op.rs:847`), a field-swap bug that would let a blank title through while
   refusing a blank description. **Caught**: 6 of 1125 core tests failed.
3. Replaced `!Core.isBlankTitle(joined)` with `joined !== ""` in
   `DJoinScreen.qml:157` (founding-title-from-join blank check), leaving the
   empty-string case caught but not whitespace/zero-width titles. **Caught**:
   2 of 106 QML specs failed
   (`test_a_blank_founding_title_from_a_join_is_not_a_founding_title`,
   `test_a_fallback_replys_title_stays_when_the_join_replys_is_blank`).
4. Dropped the `genesis` half of `currentLookup`'s reference-match guard
   (`DJoinScreen.qml:77-81`), leaving only the address compared.
   **Caught**: 1 of 106 QML specs failed
   (`test_a_lookup_answered_for_one_address_is_not_rendered_over_another`),
   exactly the single test `design.md` decision 7 names.

Every probed mutation was caught by an existing test, none silently. `cargo
mutants` itself could not be run to completion within the time budget (each
invocation of the full crate/file scope on `stoa_metadata.rs` exceeded the
few-minutes budget this review works to; a `--list` pass succeeded and named
12 candidate mutants in that file, four of which were hand-verified above).

## Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: 1125 + 30 passed, 0 failed.
- `nix build ./dialectica#lgx --no-link` from the repo root: succeeded (no
  error output).
- `sh dialectica-ui/tests/run-qml-tests.sh
  dialectica-ui/tests/tst_stoa_screens.qml`: 106 passed, 0 failed (both before
  mutating and after restoring).

## Findings

No correctness defects found. The blank-title rule is implemented as one
predicate (`stoa::is_blank_title`) shared by the genesis codec, the op codec's
`check_admitted`, and the metadata resolver, with a mirrored UTF-16
code-unit list in the view — matching `design.md` decision 1's stated intent,
and each side's test suite pins every one of the thirty code points plus the
two named non-members (U+180E, and the bidi controls).

The ordering claims in `design.md` (blank-title refusal reported *after*
structural faults, so a title also carrying trailing bytes reports
`TrailingBytes`; `check_admitted` run after `cursor.finish()` in both codecs)
match the code exactly and are pinned by
`a_blank_title_followed_by_trailing_bytes_reports_the_trailing_bytes` in both
`stoa.rs` and `op.rs`.

The encode/decode split for `Op` (`canonical_bytes` total, `encode`/`decode`
fallible through the shared `check_admitted` guard, never called from `id`,
`sign` or `verify`) is exactly what lets the resolver's fourth binding
condition — refusing a blank-titled op "a reader nonetheless holds" — be
independently testable, and the resolver test suite exercises it directly
(`a_metadata_op_carrying_a_blank_title_does_not_bind` builds, signs and stores
such an op by hand, bypassing `encode` entirely).

The view's reference-keyed `currentLookup`/`currentOutcome` pattern correctly
prevents a title answered for one `(stoa, genesis)` pair from being rendered
over a different pair on screen, which is the specific impersonation
`stoa-navigation-view`'s "A lookup's answer is rendered only for the
reference it was made for" exists to close.

No reachable panic was found on the changed paths: no new `unwrap`/`expect`
outside `#[cfg(test)]`, no new indexing or arithmetic that can overflow on
peer-controlled input, and the adversarial-input tests already in
`stoa_metadata.rs` (`an_adversarial_log_resolves_without_panicking`, mixing
forged ops, `u64::MAX` counters, and `MAX_FIELD_LEN`-length bidi-override
titles) and `stoa.rs` (`truncation_at_any_point_is_refused`, walking every
byte-length prefix of a valid record) exercise the boundary and length-prefix
paths a hostile peer would probe.

I found nothing to hand to `dev-writer`, `spec-writer` or `tester` on this
dimension.
