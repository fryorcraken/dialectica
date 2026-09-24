# Design review: join-preview-getstoa

Scope: `openspec/changes/join-preview-getstoa/design.md` against the code
(`dialectica/rust-lib/dialectica-core/src/`, `dialectica-ui/src/qml/`), against
`openspec/specs/`, and against GitHub issue #143 and its comments (re-read
fresh via `gh issue view 143 --repo fryorcraken/dialectica --comments`).

## Summary

No findings. This is one of the more carefully cross-referenced `design.md`s
seen in this repo: every one of its 14 Decisions traces to a specific,
locatable piece of code, and every "measured" or "what breaks without X" claim
names tests that exist under exactly those names and exercise exactly the
mechanism claimed. Spot checks below; all confirmed:

- **Decision 1** (frozen 30-code-point blank list, not a Unicode property or
  regex): `stoa::BLANK_CHARACTERS` (`stoa.rs:148`) and `Core.blankCodeUnits`
  (`Core.qml:452`) are byte-for-byte the same 30 code points, in the same
  order, both with the same "White_Space, then five zero-width" grouping the
  decision describes.
- **Decision 2** (`canonical_bytes` stays total; `encode`/`decode` share one
  `check_admitted` guard, not called by `id`/`sign`/`verify`): confirmed
  exactly at `op.rs:743-852`. `check_admitted` has exactly two callers
  (`encode` at `op.rs:822`, `decode` at `op.rs:947`).
- **Decision 3** (SQLite `append` refuses before the write via `to_bytes`;
  `MemoryOpLog` does not): confirmed at `log/sqlite.rs:731-737` vs
  `log/mod.rs:531-544`.
- **Decision 4** (resolver checks kind, then blankness, then `authorises`, in
  that order, for the stated reason): confirmed verbatim at
  `stoa_metadata.rs:155-188`, comments included.
- **Decision 5** (genesis encoder/decoder check length cap before blankness;
  `genesis_for` decodes then `Membership::verified` re-encodes, so the check
  runs twice on join/getStoa): confirmed at `stoa.rs:354-423` (cap before
  blank check, in both directions) and `wire.rs:1424-1456`
  (`Genesis::decode` then `Membership::verified`).
- **Decision 6** (retained blank-titled record reported via existing
  `CorruptEntry` path, nothing new written): confirmed by reading the diff of
  `membership.rs` itself — the only changes to non-test code are two doc
  comments; `MembershipStore::list`'s corrupt-entry handling is untouched, and
  the new test `a_retained_blank_titled_record_is_reported_neither_skipped_nor_migrated`
  writes the row directly via SQL to reach the state, exactly as decision 6
  says is now the only way to produce it.
- **Decisions 7-12** (view-side lookup/outcome pairing, `isGenesisFallback`
  panel selection, description sourcing, failure rendering, lookalike scope,
  `Core.stoaMetadataFrom` normalisation): all confirmed against
  `DJoinScreen.qml` and `Core.qml`, including the named regression tests
  (`test_a_lookup_answered_for_one_address_is_not_rendered_over_another`,
  `test_the_lookalike_warning_cannot_be_claimed_before_a_join_happens`, and
  others named in the decisions), all present under those exact names in
  `tst_stoa_screens.qml`.
- **Decisions 13-14** (creation forwards a blank title unchecked; a listed
  blank-titled row still renders with no substitute title): confirmed in
  `DStoaListScreen.qml:456-464` and its surrounding comment.

## Owner rulings (issue #143, re-read fresh)

Both rulings hold in the code and are not contradicted:

- **Lookalike stays keyed to founding titles, not current titles.**
  `DJoinScreen.qml`'s `lookalikes` property (~line 268) reads only
  `screen.foundingTitle`, never `currentTitle`, and
  `test_the_lookalike_warning_cannot_be_claimed_before_a_join_happens` pins a
  case where a current title equals a held Stoa's founding title and no
  lookalike is reported.
- **Blank title invalid everywhere, in the same way as `""`.** Both
  `GenesisError::BlankTitle` and `OpError::BlankTitle` cover the empty and
  whitespace/zero-width-only cases identically (`stoa::is_blank_title`, shared
  by the genesis codec, the op codec, and the resolver).
- **A store already holding a blank-titled record is neither migrated nor
  skipped.** Confirmed above (decision 6): the row is reported as
  `CorruptEntry` and left untouched.

## Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`:
  1125 + 30 passed, 0 failed.
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`:
  106 passed, 0 failed.
- `nix build ./dialectica#lgx --no-link`: succeeded (first attempt hit a
  transient `eval-cache` SQLite-busy error from a concurrent build elsewhere on
  the machine; the retry succeeded cleanly).

No mutations were made to tracked files; only the gitignored SDK symlink was
created for the build, as instructed.
