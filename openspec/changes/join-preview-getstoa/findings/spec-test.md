# spec-test review — `join-preview-getstoa`

No findings. Every scenario I could find across the six spec deltas is pinned by
a test that can fail, no `NO SPEC:` markers were introduced unmarked, and the
two REMOVED/ADDED requirement pairs (`stoa-membership`, `stoa-metadata`) are
within-capability replacements whose migration notes match their ADDED text —
there is no cross-capability requirement move to lose text in transit. This
file records what was checked and how, per the role's mandate to write a file
even when nothing is wrong.

## 1. Scenario coverage, walked spec by spec

- **`stoa-genesis`** ("A blank title is not a valid title", MODIFIED "A tampered
  or truncated record is rejected"): every scenario has a matching test in
  `stoa.rs` — empty title, each of the thirty alone, the mixed blank title, the
  one-byte title, one visible letter among blank characters, the two
  not-blank-but-invisible characters (U+200E, U+180E), the trailing-bytes
  ordering, and distinguishability from the other four refusals.
- **`op-format`** (MODIFIED "Valid text is never normalised...", MODIFIED "A
  Stoa metadata op carries display fields and no policy"): matching tests in
  `op.rs` for the same set, plus the description-is-exempt scenarios and the
  encode/decode symmetry (`Op::encode`/`Op::check_admitted`).
- **`stoa-membership`** (REMOVED/ADDED "A title the genesis record cannot carry
  ... is refused before a Stoa exists", MODIFIED join/retention requirements):
  covered in `wire.rs` (`a_blank_title_creates_nothing_and_says_it_is_blank`,
  `a_title_of_one_letter_among_blank_characters_creates_a_stoa_unaltered`,
  `a_blank_titled_record_is_refused_by_a_join_and_records_nothing`) and
  `membership.rs` (`a_blank_titled_record_cannot_be_joined`,
  `a_retained_blank_titled_record_is_reported_neither_skipped_nor_migrated`,
  the last exercising both the empty and mixed-blank raw-bytes cases against
  the owner's no-migration ruling).
- **`stoa-metadata`** (REMOVED/ADDED "Current metadata resolves...", MODIFIED
  reply/refusal requirements): the fourth binding condition, its ordering
  before `authorises`, the non-displacement case, and the one-visible-letter
  case are in `stoa_metadata.rs`. The wire-level fallback-for-a-nonblank-op and
  refusal-for-a-stored-pre-refusal-op scenarios are in `wire.rs`
  (`a_creator_signed_blank_titled_metadata_op_is_answered_as_a_fallback`,
  `a_blank_titled_op_stored_before_the_refusal_is_an_error_not_a_fallback`).
- **`op-log`** (MODIFIED "The log records what arrived...", ADDED "A stored
  entry that does not decode fails every read..."): the append-refusal and
  values-log-stores-it scenarios are split correctly across `sqlite.rs`
  (`an_op_with_no_encoding_is_refused_and_nothing_is_written`) and `log/mod.rs`
  (`a_blank_titled_op_is_stored_and_read_back_unchanged`, explicitly noting why
  it must live where `MemoryOpLog` alone can exhibit it). The
  stored-before-refusal read failure, including the Stoa-restricted and
  unrestricted read paths and the survivor-not-silently-returned case, is in
  `sqlite.rs` (`a_creator_signed_blank_titled_op_stored_before_the_refusal_fails_every_read`,
  `a_read_fails_rather_than_returning_the_surviving_op_beside_a_corrupt_one`).
- **`stoa-navigation-view`** (both ADDED requirements plus the five MODIFIED
  ones): every scenario I could match against `tst_stoa_screens.qml` is there,
  including the reference-keying tests, the description-only-from-non-fallback
  tests, the malformed-shape and blank-title failure tests (with the
  no-wider-no-narrower boundary test against U+200E/U+180E), the
  join-reply-takes-precedence-over-fallback and
  fallback-stays-when-join-is-blank precedence tests, and the
  blank-title-matches-nothing lookalike test. These are QML component tests
  driving a fake `callModule` bridge — the right layer for a requirement about
  what the view calls and renders, since `getStoa`/`join_stoa` are core-boundary
  calls the fake stands in for correctly (per `dialectica-test-defect-family`
  and `copy-the-working-example` memory notes, I checked the fake answers
  **per request** rather than a fixed string, which the two-reference tests
  need and get).

I found no scenario left without a test, and no scenario that is untestable as
written.

## 2. Mutation testing

Two mutations, both restored before this commit; `git status` shows only this
findings file.

- **`dialectica-ui/src/qml/Core.qml`, `isBlankTitle`**: widened the blank test
  to also treat U+200E as blank (`&& s.charCodeAt(i) !== 0x200E` removed from
  the "not blank" branch — i.e. the function stopped rejecting U+200E as a
  non-blank character). Ran `sh dialectica-ui/tests/run-qml-tests.sh
  dialectica-ui/tests/tst_stoa_screens.qml`: **caught** —
  `test_the_views_blank_list_is_the_thirty_the_core_lists` failed ("not blank:
  0", actual 1 vs expected 0). Restored; full suite back to 106/106.
- **`dialectica/rust-lib/dialectica-core/src/stoa.rs`, `Genesis::decode`**:
  moved the blank-title check to before `cursor.finish()` (reversing the
  "blank is the last refusal" ordering). Ran `cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica-core
  a_blank_title_followed_by_trailing_bytes_reports_the_trailing_bytes`:
  **caught** — `stoa::tests::a_blank_title_followed_by_trailing_bytes_reports_the_trailing_bytes`
  failed (`left: Err(BlankTitle), right: Err(TrailingBytes)`). Restored; full
  `cargo test -p dialectica -p dialectica-core` back to 1125+30 passed, 0
  failed, and `git status` clean.

Note: an initial attempt to mutate `stoa_metadata.rs`'s `binding_metadata` (the
resolver's fourth condition — design.md's own predicted highest-value mutation)
was blocked by the harness's auto-mode security classifier ("Security Weaken")
on both the direct edit and a retry, and I did not attempt a workaround per the
tool's own instruction not to route around a denial's intent. I substituted the
two mutations above from the same budget instead. This is worth flagging to
whoever owns agent tooling: the classifier blocked a sanctioned,
restore-before-commit mutation on a role that structurally requires editing
code to do its highest-value check.

## 3. `NO SPEC:` markers

`git diff origin/main...HEAD` on `dialectica/rust-lib/dialectica-core/src`,
`dialectica-ui/src`, `dialectica-ui/tests` and
`dialectica/rust-lib/dialectica-core/tests` adds no new `// NO SPEC:` markers.
Two markers referenced by `design.md` §5
(`a_blank_title_followed_by_trailing_bytes_reports_the_trailing_bytes` in
`stoa.rs` and in `op.rs`) were removed, matching design.md's claim that the
spec now covers what they used to mark as an unspecified choice. Confirmed
neither file carries a `NO SPEC` string any more.

I did not find any unmarked behaviour that reads as a silent dev decision.

## 4. Requirements moved between capabilities

Both REMOVED/ADDED pairs in this change (`stoa-membership`'s title-refusal
requirement, `stoa-metadata`'s resolution requirement) are **within the same
capability** — replaced because a MODIFIED block cannot drop a scenario, not
relocated to a more general capability. There is no cross-capability move here,
so the "did the text survive verbatim in its new home" check does not apply.
I did check that each migration note's prose description of the edits matches
the ADDED requirement's actual text, and it does in both cases (verified by
reading both full texts side by side, above).

I also checked the five MODIFIED requirements in `stoa-navigation-view` that
claim a narrow, specific change (e.g. "only its citation ... changes") against
`openspec/specs/stoa-navigation-view/spec.md`'s live text, for
"No current title is rendered until one has been resolved": confirmed the only
diff is the cited requirement name, exactly as claimed.

## 5. Spec soundness

- **Self-consistency**: read all six delta files in full rather than relying on
  `openspec validate --strict`. No internal contradiction found.
- **Testability**: no scenario asserts something untestable as written.
- **Staleness against issue #143**: read fresh via `gh issue view 143
  --repo fryorcraken/dialectica --comments`. Both owner rulings quoted in the
  task brief (blank-title invalidity in every respect; no migration of a store
  already holding one; the pre-existing lookalike-stays-on-founding-titles
  scope note from #98/PR #144) match the proposal's "Out of scope" and "A blank
  Stoa title is invalid" sections verbatim in substance. I found no requirement
  in the six deltas that is now out of scope relative to the issue.

## Summary

No unticked findings for `spec-writer`, `dev-writer` or `tester`. The one
non-finding worth surfacing (§2's classifier block) is reported above rather
than as a checkbox, since it is not something any of those three roles can act
on.
