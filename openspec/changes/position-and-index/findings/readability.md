# Readability review — position-and-index

Dimension reviewed: **readability only** (names, test/doc comments that must stay
true, and prose in spec/proposal/design that a later reader relies on). I read
issue #166 (`gh issue view 166 --json body,comments`) including the owner's
decision comment of 2026-09-25
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024),
which settles item 2: a malformed `index` is refused with an error naming
`index`, and leaving the message unspecified is ruled out. The spec delta in
this change (`identity-onboarding`'s ADDED requirement) implements exactly that
ruling.

Reviewed via `git diff origin/main...HEAD` (three dots): `proposal.md`,
`design.md`, `tasks.md`, both spec deltas, and the test code added to
`dialectica/rust-lib/dialectica-core/src/wire.rs`. Every checkable claim in a
comment or doc I could verify (parser behaviour, `cmp_ops` ordering, which
tests iterate `MALFORMED_INDEXES`, the standalone refactor commit, `clock: None`
tie-break, compiler warnings) checked out true against the code, so this list
holds only the one finding below plus a clean bill for everything else.

## Findings

- [x] **`spec-writer`** — `openspec/changes/position-and-index/specs/thread-read/spec.md:13`
      (requirement *An item carries its ordering position and the author's
      asserted time, as two separate fields*) — the one paragraph this piece adds
      to that requirement uses `MUST` twice ("The position MUST be determined by
      the place alone...", "...those two items MUST carry the same position"),
      where every other normative sentence in this requirement — all seven of
      them, before and after this piece's edit — uses `SHALL`. Confirmed against
      `git show origin/main:openspec/specs/thread-read/spec.md` lines 704–722:
      zero `MUST` in the base text of this requirement, seven `SHALL`. The rest
      of the file does use `MUST` elsewhere (e.g. the ordering requirement at
      base lines 482–486, the author-key requirement at 783–785), but each of
      those requirements is internally consistent — one modal per requirement —
      and this is the one place in the file where a single requirement mixes
      the two. No distinction between `SHALL` and `MUST` is documented anywhere
      in `openspec/` or `docs/`, so a later reader has no way to tell whether the
      switch is meaningful or a slip. **Scenario:** a reader diffing this
      requirement against the rest of the file, or against `identity-onboarding`'s
      new requirement (which correctly stays all-`MUST`, matching that file's own
      established mixed-but-per-requirement-consistent convention), would
      reasonably ask whether the place-rule paragraph is meant to bind more
      strictly than the rest of the requirement it sits in. **Severity:** low —
      cosmetic, does not change any test's pass/fail behaviour and both keywords
      are RFC-2119-style synonyms throughout this codebase. Fix is a two-word
      s/MUST/SHALL/ in that one paragraph, or a decision that mixing is fine (in
      which case it's worth saying so once, rather than leaving it to be
      re-derived).

      **Outcome (`spec-writer`): fixed.** Both `MUST`s in the place-rule
      paragraph are now `SHALL`, so the requirement uses one modal throughout,
      matching the per-requirement convention the reviewer measured. The
      alternative, rewriting the requirement's seven `SHALL`s to `MUST`, was not
      taken: those sentences are carried verbatim from the live spec, and
      `proposal.md` says the rest of the block is copied unchanged, which two
      reviewers checked byte for byte. The meaning is unchanged, so no test
      moves and none can fail on it. `git grep -F` finds no other quote of either
      sentence to update, apart from this finding.

## What I checked and found clean

- **Names.** `keep_with_raw_index`, `MALFORMED_INDEXES`,
  `thread_positions_at`, `a_thread_log_with_shared_authors`,
  `a_thread_log_with_a_hidden_reply_and_a_reply_after_it`, and
  `the_item_at_a_place_carries_that_places_position_in_every_read` (the missing
  apostrophe in "places" is the codebase's existing convention for possessives
  in `snake_case` test names — Rust identifiers can't carry one, and other
  pre-existing tests in this file do the same, e.g.
  `no_thread_item_holds_its_signers_key_under_any_key_but_author`) are accurate
  and specific. No vague `handle`/`process`/`_and_` names introduced.
- **Checkable claims, run rather than read:**
  - `parse_index`'s reachable arm for `18446744073709551616`: confirmed by
    reading `parse_index` (wire.rs:1780–1833) that `as_u64()` returns `None` for
    a value that overflows `u64`, landing on the "must be a non-negative
    integer written without a decimal point or exponent" arm, never `try_from`
    — exactly what `design.md`'s Risks section and the `MALFORMED_INDEXES` doc
    comment claim.
  - `arrival::cmp_ops`'s tie-break: confirmed both the `(Some, Some)` descending-
    counter arm and the `(None, None)` ascending-`OpId` arm match the claims in
    `tasks.md` task 3.4 and the doc comment on
    `a_thread_log_with_a_hidden_reply_and_a_reply_after_it` (ascending explicit
    counters land in append order once `read_thread` reverses the sequence).
  - `a_thread_post`/`a_thread_root` (used by `a_thread_log_with_shared_authors`)
    do set `clock: None`, confirming the claim that fixture "only asserts SET
    properties" because it can't rely on `OpId`-hash tie-break for adjacency.
  - The claimed standalone refactor commit exists exactly as described:
    `fc9c2edb "Let a keep test send its index as raw JSON text"`, 13
    insertions, no test added — matches `design.md` D4 and `tasks.md` 1.1
    exactly.
  - `MALFORMED_INDEXES` (8 entries) is iterated by both
    `each_malformed_kind_of_index_is_refused_by_name` and
    `a_malformed_index_stores_nothing`, matching D4's claim that "the two
    cannot disagree about what 'malformed' means."
  - `cargo test -p dialectica -p dialectica-core` on the `position`- and
    `index`-named tests: all pass, and `dialectica`/`dialectica-core` compile
    with no new warnings (the only warnings are pre-existing, from the vendored
    `logos-rust-sdk-src`).
- **Spec prose.** Compared the `thread-read` delta against
  `origin/main:openspec/specs/thread-read/spec.md` line by line for the two
  edits `proposal.md` claims: the uniqueness sentence's narrowing to "one read
  of a thread, taken across all of that read's pages" and the new "place alone"
  paragraph are exactly as described, no more and no less changed. The
  `identity-onboarding` ADDED requirement's five malformed-kind bullets
  correctly generalise the test table's eight concrete values (the "not a
  number" bullet covers the string/array/boolean cases as one bucket) — no
  mismatch between spec and test enumeration.
- **`tasks.md` and `design.md` cross-references** (test names, line-level
  claims like "task 3.4, which the tester writes") all resolve to what's
  actually in `wire.rs`.

No other readability defects found. I did not review correctness, security or
architecture — those are separate reviewer dimensions per this piece's
`tasks.md`.
