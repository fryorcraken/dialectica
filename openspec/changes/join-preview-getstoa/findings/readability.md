# Readability review — `join-preview-getstoa`

Dimension covered: **readability only** (naming, comments claiming things the
code does not do, numbers in comments, functions doing two jobs, whether a
reader can follow the change). Correctness, security and architecture are
other reviewers' passes and are not covered here.

Scope read: full `git diff origin/main...HEAD` — all six spec deltas,
`design.md`, `proposal.md`, every changed file under
`dialectica/rust-lib/dialectica-core/src/` (`stoa.rs`, `op.rs`,
`stoa_metadata.rs`, `wire.rs`, `membership.rs`, `transport.rs`, `cursor.rs`,
`revision.rs`, `authoring.rs`, `log/{mod,sqlite,contract,fixtures}.rs`), and
`dialectica-ui/src/qml/{Core,DJoinScreen,DStoaListScreen}.qml` plus
`dialectica-ui/tests/tst_stoa_screens.qml`.

Every load-bearing claim I could cheaply check against the code, I ran rather
than read: the `DStoaListScreen.qml` comment that a listing carrying a
retained blank-titled record now fails as a whole rather than filtering the
row (checked against `membership.rs`'s
`a_retained_blank_titled_record_is_reported_neither_skipped_nor_migrated` and
`log/sqlite.rs`'s `an_op_with_no_encoding_is_refused_and_nothing_is_written`
— both true as stated); `Core.qml`'s `getStoa` calling a real `get_stoa`
handler in `wire.rs` (confirmed at `wire.rs:2221`); and `design.md`'s
per-decision "what breaks" test-name citations, which I cross-checked by name
against the actual tests added in `op.rs`, `stoa_metadata.rs` and
`transport.rs` — all present and matching their stated behaviour.

## Findings

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:2363` — a doc-comment line runs to 100 characters, wider than the ~85-character wrap the surrounding paragraph and the rest of the file use
      **Scenario:** the line `/// cap and for a blank one. It returns `{"error":"title: …"}` below, before the store is reached at` was widened when "and for a blank one" was inserted into the existing sentence, and the wrap was not redone — every neighbouring line in the same doc comment (2359–2380) sits at 70–90 characters. Cosmetic only: `cargo fmt` does not reflow doc comments, so this survives the fmt gate untouched and needs a human edit.
      **Severity:** trivial (style only, no reader is misled).

- [ ] **`tester`** — `dialectica-ui/tests/tst_stoa_screens.qml` — `test_an_empty_title_reaches_the_core_rather_than_being_refused_here` now iterates three blank titles, not one
      **Scenario:** the test body loops `var typed = ["", "   ", "​　"]` (empty, whitespace-only, zero-width-only), but the name still says "an empty title" singular. The claim the name makes ("reaches the core rather than being refused here") is still true of the body, so this is not a false claim — but a reader scanning test names for "what covers the whitespace-only case" would not find it under this one, since the name reads as the single-empty-string test it used to be before this change widened its scope.
      **Severity:** minor. Renaming to something like `test_every_blank_title_reaches_the_core_rather_than_being_refused_here` would remove the ambiguity; not blocking.

## Areas checked and found clean

- **`stoa.rs`, `op.rs`, `stoa_metadata.rs`**: the blank-title machinery
  (`BLANK_CHARACTERS`, `is_blank_title`, `check_admitted`) is documented with
  unusual care — every non-obvious ordering decision (blank-check-after-structure,
  guard-not-under-`verify`) has a comment explaining *why*, not just *what*, and
  each claims a specific test by name that I found present and matching.
- **`op.rs`'s `Op::canonical_bytes` vs `Op::encode` split**: cleanly named and
  documented (`canonical_bytes` is the total layout; `encode` is the fallible
  admission gate). No `And`/`handle`/`process`-style naming found anywhere in
  the new surface.
- **`Core.qml`**: `getStoa`, `isBlankTitle`, `stoaMetadataFrom` are each one
  job, named for what they do, and the doc comments' claims about the core
  holding the same list, and about `stoaMetadataFrom` being the one place a
  malformed reply is judged, both check out against the core side.
- **`DJoinScreen.qml`**: large but not doing two jobs — the `outcome`/`lookup`
  pairing-with-its-reference pattern is repeated once (deliberately, per its
  own comment) rather than varied, and every derived property
  (`foundingTitle`, `currentTitle`, `titleKnown`, `lookalikes`) has a comment
  stating what it is derived from and why assignment would be wrong. This is
  dense reading but each density is load-bearing rather than padding.
- **`DStoaListScreen.qml`**: both updated comments (creation passes a blank
  title through; a blank-titled row still renders) make specific claims about
  core behaviour, and both are accurate (verified against `membership.rs` and
  the wire tests).
- **`design.md`**: unusually well cross-referenced to test names; spot-checked
  a majority of the "what breaks" citations across decisions 2–14 and found no
  fabricated or stale test name.
- **Mechanical diffs** (`authoring.rs`, `cursor.rs`, `revision.rs`,
  `transport.rs`, `log/{contract,fixtures,mod,sqlite}.rs`): the
  `to_bytes()` → `to_bytes().unwrap()` churn from making `SignedOp::to_bytes`
  fallible is uniform and unremarkable; new comments introduced alongside it
  (e.g. `cursor.rs`'s updated "an empty variable-length field is legitimate"
  example, now citing a description/body instead of a title that no longer
  has an empty encoding) are corrected consistently everywhere I found the
  old wording, with no stale copy left behind.
- **Spec deltas** (`stoa-genesis`, `op-format`, `op-log`, `stoa-membership`,
  `stoa-metadata`, `stoa-navigation-view`): prose is clear, scenario names
  match their `WHEN`/`THEN` bodies, and the thirty-code-point list is spelled
  identically in the spec, `stoa.rs`, and `Core.qml` (I diffed the three
  listings against each other by eye; they agree in count and order).

## Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`: passed in full (readability pass does not depend on this, but it was run to confirm the tree builds cleanly before reading it).
- `cargo mutants -p dialectica-core --file dialectica-core/src/stoa.rs`: attempted, abandoned after exceeding the couple-minute budget on this fresh worktree's baseline build (per instructions to abandon a slow run rather than wait it out). No mutants result obtained; this is a gap left for whichever reviewer re-runs it with a warmed build cache, not a finding in itself.
- `nix build ./dialectica#lgx` and the QML spec runner were not run for this pass — out of scope for a readability-only review, and no readability finding depends on either gate's output.
