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

## Re-review of the findings-round commits

Dimension: **readability only**, per `tasks.md`'s "Re-review of the
findings-round commits" row for `code-reviewer`/readability. I read issue #166
(`gh issue view 166 --json body,comments`) again, including the owner's
decision comment of 2026-09-25
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024),
which settles item 2: a malformed `index` is refused with an error that names
`index`, and leaving the message unspecified is ruled out.

Scope was `git diff 19667a04...HEAD` (three dots), restricted to the four
commits `tasks.md` names for this row: `b55d164` (`thread-read` delta's
`MUST`→`SHALL`, and the new `feed-read` boundary paragraph added directly to
the live `openspec/specs/identity-onboarding/spec.md` Purpose), `9e550eb`
(`design.md` D1, D2 and D5 rewritten as measured), `70bef05` (the `wire.rs`
fixture refactor extracting `a_thread_post_with_clock`), and `4e2b8c0`
(rustfmt of the one drifted call). I also read `tasks.md`'s re-review section
itself.

**My own earlier finding.** The one finding I raised in the first round (the
two stray `MUST`s in `thread-read`'s place-rule paragraph) was answered in
`b55d164`: both are now `SHALL`, confirmed by `git diff 19667a04...HEAD --
openspec/changes/position-and-index/specs/thread-read/spec.md` — the diff
touches exactly those two words, nothing else in the paragraph moved. The
requirement now uses one modal throughout, matching the rest of the file's
per-requirement convention. Correctly resolved.

**Claims checked by running rather than reading, this round:**

- The new `identity-onboarding` Purpose paragraph names *A malformed `index`
  in a keep request is refused with a message naming `index`* — that heading
  exists verbatim in `openspec/changes/position-and-index/specs/identity-onboarding/spec.md:3`
  (`git grep -n`). It also claims `feed-read` "requires its message to name the
  field" for `page`/`perPage` — confirmed at `openspec/specs/feed-read/spec.md:555`
  ("A `page` or `perPage` that is … MUST be refused with the error shape. The
  message MUST name the field.").
- `proposal.md`'s claim that this edit had to go directly against the live
  spec because "a delta cannot change a Purpose": the change's own delta,
  `openspec/changes/position-and-index/specs/identity-onboarding/spec.md`,
  opens directly on `## ADDED Requirements` with no `## Purpose` section at
  all, consistent with that constraint. `openspec validate --strict
  position-and-index` passes with the boundary paragraph in place.
- `design.md` D1's rewritten claim that dropping `{field}` from `parse_index`'s
  non-integer arm (`"{field} must be a non-negative integer …"`, `wire.rs:1826`)
  turns both `each_malformed_kind_of_index_is_refused_by_name` and
  `malformed_pagination_fields_are_refused_by_name` red "first on `-1`": `-1`
  is indeed the first entry of `MALFORMED_INDEXES` (`wire.rs:5076`), and `-1`,
  `1.5`, `0.0` and `1e2` all reach that arm via `as_u64() == None` while
  `"two"`, `[]` and `true` reach the wrong-type arm instead — read from
  `parse_index` directly. The claim that this also turns
  `the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`
  red checks out too: that test's "one larger" value is `usize::MAX + 1` =
  2^64, which the *first* round's readability review already confirmed lands
  on `as_u64() == None` (not the unreachable `try_from` arm) — the same arm.
  All three test names cited exist (`git grep -n`).
- Cross-checked `4e2b8c0f`'s claim that the remaining `cargo fmt --check`
  drift (`identity.rs`, two more `wire.rs` hunks) "predates this piece and is
  left alone": ran `cargo fmt --manifest-path dialectica/rust-lib/Cargo.toml
  --check -p dialectica -p dialectica-core` myself and got exactly those four
  hunks (two in `identity.rs`, two more in `wire.rs` beyond the one this piece
  fixed). `identity.rs` has zero diff against `origin/main` anywhere in this
  piece, and `git blame` on the pre-existing `wire.rs` drift lines attributes
  them to `93f1ac3f` (#153), dated before this piece's first commit. Claim
  holds.
- `design.md` D5's new "fixture's replies carry explicit ascending clocks"
  paragraph claims `a_thread_post_with_clock` is "the one constructor for
  fixture posts". `grep -n "a_thread_post("` shows every thread fixture in
  `wire.rs` (including `a_thread_log_with_shared_authors` and the
  hidden-reply fixture) funnels through `a_thread_post`, which now delegates
  to `a_thread_post_with_clock`. Other `OpKind::Post {` literals do exist
  elsewhere in the file, but they belong to unrelated fixtures (feed reads,
  `get_stoa`, etc.), so the claim is correctly scoped, not overbroad.
- The `1889eaf` and `70bef05` commit messages, cited by `design.md` as where
  "both arms were measured by mutation" and where "every mutation `design.md`
  names was re-run" respectively, do contain exactly those claims in their own
  text (`git log -1 --format=%B`), matching what `design.md` now says.

**One limitation to report.** I attempted to independently re-run the `{field}`-
drop mutation on `parse_index`'s non-integer arm myself (the highest-value
check available for a claim this central), after confirming I am standing in
my own worktree (`pwd` → `.claude/worktrees/agent-a2f9b82c4d36fa469`,
`git rev-parse --abbrev-ref HEAD` → `worktree-agent-a2f9b82c4d36fa469`, not
`piece/166-position-and-index` and not the repo root). Two `Edit` attempts on
`dialectica/rust-lib/dialectica-core/src/wire.rs` — including a minimal
one-line change — were both denied by the harness's own auto-mode classifier
("Modify Shared Resources"), independent of the edit's content. Per that
denial's own instructions I did not pursue a workaround. I therefore rely on
the self-reported mutation evidence in the `1889eaf` and `70bef05` commit
messages plus the static checks above, rather than a mutation I ran myself.
`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core` does pass green (1185 tests) on the unmutated tree, confirmed
this session.

**No new readability findings.** The prose added in this round (the `identity-
onboarding` boundary paragraph, the `design.md` D1/D2/D5 rewrites, the doc
comments on `a_thread_post_with_clock`) is accurate against the code, mirrors
the established "decline to restate, say so in the Purpose" convention
`docs/OPENSPEC-ARCHIVE.md` documents for `op-ordering`/`op-format`, and every
checkable claim in it held up. The `wire.rs` refactor (`70bef05`) is a clean
extraction with a doc comment that earns its place (explains *why* the clock
became a parameter, not what the code already shows), and the `4e2b8c0f`
formatting commit is exactly what it claims to be. `tasks.md`'s re-review
section itself is clear and accurately scoped.
