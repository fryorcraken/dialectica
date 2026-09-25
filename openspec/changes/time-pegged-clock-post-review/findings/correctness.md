# Correctness review

Dimension: **correctness only** (per dispatch prompt). Read the owner's decision
comment on #162 first; its first line:

> **Decision (owner, 2026-09-25): peg the Lamport counter to wall-clock time, as
> SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`, lines 148-155 and
> 184-192).**

Both diffs reviewed:

1. `git diff origin/main...HEAD` (three dots) — this piece's own diff.
2. `git diff 2eada33 c1f1a8f` — the four #165 commits that merged without review.

## Findings

No correctness defects found in either diff. No checkboxes below; see "What was
checked" for what was verified and how.

## What was checked

### Diff 1 (this change)

- **The only code changes are four doc-comment repoints**
  (`arrival.rs` on `RECEIVE_WINDOW_MS`, `op.rs` twice, `transport.rs` on
  `receive`), from "`op-ordering` states the cost" to the archived
  `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`. Confirmed
  by reading the diff directly: no logic, no `cfg`, no test lines changed.
  `git grep -n -F -e "states the cost" -- dialectica/rust-lib/dialectica-core/src`
  finds nothing left over, and the sweep's claimed count ("four doc comments")
  matches the diff stat exactly (arrival.rs one hunk, op.rs two hunks,
  transport.rs one hunk).
- **The new citations resolve.** `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`
  has a `### 11. A receiver whose own clock is wrong` section (not a differently
  numbered one — checked by listing every `### ` heading in the file), and its
  content (slow-clock / fast-clock analysis, "no ratchet") is exactly the cost
  the repointed comments claim it works through. `transport.rs`'s comment also
  cites Decision 1, which is present and relevant (the four rejected
  alternatives, including why demoting or arrival-order refusal were rejected).
- **Behaviour-preservation claims spot-checked against the live spec.** The
  reply-scenario fix (issue #162 item 1) and the `thread-read` position-wording
  fix (item 3) were already merged live by #165; this piece's delta
  (`specs/op-ordering/spec.md`, `specs/thread-read/spec.md`) keeps the
  WHEN/THEN/AND clauses of both **byte-for-byte** identical to
  `openspec/specs/`, removing only the surrounding prose. Diffed both by hand
  (`git grep` for the scenario titles and requirement text in both the live
  spec and the new delta) — no behavioural clause was dropped or reworded.
- **`cargo test` and `nix build ./dialectica#lgx` both green** (see below).

### Diff 2 (the four #165 commits)

- **The flagged claim** — the `moderation.rs` `resolve` doc's new paragraph
  ("a moderator can sign a `Hide` or `Unhide` up to an hour ahead and beat an
  opposite action... without having received it") — was checked against the
  actual mechanics rather than taken on trust, since `tasks.md` 4.2 in the
  now-archived `time-pegged-clock` change explicitly records it as untested
  ("Doc-only; no test can see it"):
  - `resolve`'s `deciding_index` takes the leading candidate (index 0) whenever
    it carries a counter — a pure last-write-wins-by-counter comparison, with
    no reference to arrival order. This is independently pinned by two
    existing tests, `a_later_unhide_reverses_an_earlier_hide` and
    `an_unhide_reverses_a_hide_whatever_the_two_op_ids_are` (both pre-existing,
    not touched by this diff), which show the higher counter always wins
    regardless of append order.
  - `clock_from_counters` is `counters.into_iter().max()` — the per-Stoa clock
    is the maximum of every counter admitted, with the window (not this
    function) as the only gate on what gets admitted.
  - `transport::receive`'s window check (`exceeds_receive_window`) runs against
    any op carrying a counter, of any `OpKind` including `Moderate` — not
    exempted for moderation ops.
  - Composing these three: a moderator can sign a counter up to
    `RECEIVE_WINDOW_MS` (one hour) ahead of a receiver's clock and have it
    admitted and treated as leading, and a second moderator's honest,
    real-time-countered correction that has not yet folded in the first
    op loses the comparison until it does. The claim is a correct corollary of
    already-tested mechanics, even though no single test exercises the
    end-to-end "hour-ahead moderator race" scenario. That gap is a testing
    concern (already tracked in the cited `tasks.md` line), not a correctness
    defect in the doc.
- **The two `NO SPEC` resolutions checked against the code:**
  - `authoring.rs::publish` signs `asserted_ms: who.asserted_ms` unconditionally
    (never the counter), confirming "one reading of the time signs both clock
    fields" and that the new test
    `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
    genuinely distinguishes the two fields. Reproduced the commit's own
    mutation-testing claim independently: edited `publish` to write the
    counter into `asserted_ms`, ran
    `cargo test -p dialectica-core a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`,
    watched it fail (`left: 1789732304001, right: 1789729304000`), then
    restored the file (`git status --short` confirms the tree is clean again).
  - `transport::receive` checks `exceeds_receive_window` before `log.append`
    (the only place `AlreadyPresent` can be reported), confirming a held op
    arriving again beyond the window is refused as `AheadOfTime` and never as
    `AlreadyPresent`, matching the new `op-transport` scenario and its
    pre-existing test.
- **`revision.rs`'s reworded doc** (`current_version`/`CurrentVersion::current`)
  makes the same "unverified claim about the time" correction as
  `moderation.rs`, and rests on the same `cmp_ops`/leading-entry mechanics
  already checked above.

### Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`:
  **1180 passed** (lib) + **30 passed** (`end_to_end.rs`), 0 failed, run twice
  (once with the temporary mutation above in place — 1 failed as expected —
  and once clean, both reported here).
- `nix build ./dialectica#lgx` from the tree root: succeeded (no output, exit
  0).

## Not this dimension

Test coverage gaps (e.g. no end-to-end test for the moderator-race scenario),
readability of the new doc comments, and architecture/design-record alignment
are covered by the other reviewer instances for this piece.

## Re-review of 439c192..HEAD

Re-review scope, per dispatch: PR #173, re-reviewing only `git diff 439c192
HEAD` — the round-1 findings fixes plus the new prose they added (a new doc
section on `revision.rs`'s `current_version`, a new Decision 11 in
`design.md`, and a corrected test comment in `authoring.rs`). Read the owner's
#162 decision comment again; its first line, unchanged from the section
above:

> **Decision (owner, 2026-09-25): peg the Lamport counter to wall-clock time,
> as SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`, lines 148-155
> and 184-192).**

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/revision.rs:300-302`
      — the new "What signing an hour ahead buys here, and whom" section on
      `current_version` claims the ahead-signed version's lead is
      **permanent**, contradicting the "for up to an hour" bound stated three
      sentences earlier in the same paragraph, and contradicting the very
      scenario it cites two sentences later.
      **Text:** "A revision published after receiving the ahead-signed one
      carries the greater counter and becomes current. Before the window, an
      author who signed the maximum counter fixed that version as current
      **permanently**, and their own later revisions **could never displace
      it**." The next sentence cites `op-ordering`'s scenario "An op signed
      ahead of the time leads only until the time passes it" — which is the
      opposite claim.
      **Scenario:** `openspec/specs/op-ordering/spec.md:396-400` states the
      mechanism directly: a second device that never received the
      ahead-signed op, once **its own current time has passed that counter**,
      publishes an op that "carries the greater counter" and the ordering
      rule "places [it] first" — with no dependence on ever receiving the
      earlier op. Confirmed against the actual functions: `next_counter`
      (`arrival.rs:297`) is `now_ms.max(clock.saturating_add(1))`, so once a
      device's own `now_ms` naturally exceeds the ahead-signed counter (which
      happens after at most one real hour, since the counter cannot be more
      than `RECEIVE_WINDOW_MS` ahead of *some* peer's clock when signed),
      **any** subsequent revision from **that same author**, even one signed
      from the very device that made the ahead-signed op, gets a strictly
      higher counter (`clock_from_counters` — `arrival.rs:275` — folds the
      ahead-signed op straight into that device's own clock, so its very next
      publish is `ahead_counter + 1`). Displacement is not merely possible; it
      is what a normal subsequent publish, by anyone including the same
      author, produces. "Fixed... permanently" and "could never displace it"
      are both false of the code and false of the paragraph's own opening
      sentence.
      **Severity:** correctness/documentation — no runtime code changed, but
      this is new prose this round added specifically to make an archived
      design claim true (`tasks.md` 4.3), and it overshoots into asserting a
      permanence the cited requirement explicitly rules out. A maintainer
      reading only this docstring would conclude the exposure is unbounded,
      which is wrong and is the opposite error from underselling it.
      **For context, not a second checkbox:** the same "won such a dispute
      permanently" construction pre-exists, unchanged by this diff, in
      `moderation.rs:437-438` (`resolve`'s doc). That file was not touched
      between `439c192` and `HEAD` so it is outside this re-review's scope,
      but a fix to the wording here should probably use the same corrected
      phrasing there too, since `design.md`'s new Decision 11 explicitly
      frames this paragraph as making the two sites agree.

Nothing else in the `439c192..HEAD` diff drew a correctness finding:

- The corrected test comment in `authoring.rs:1830-1843` was checked by
  reproducing its claim rather than reading it: mutated `publish`
  (`authoring.rs:276`) to sign `asserted_ms: next_counter(clock,
  who.asserted_ms)` instead of `who.asserted_ms`, then ran both named tests.
  `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
  failed exactly as the comment says, on its own `asserted_ms` assertion
  (`left: 1789732304001, right: 1789729304000`).
  `the_second_authoring_carries_the_higher_counter` failed too, but exactly
  where the comment says — on its fixture-drift guard at `authoring.rs:885`
  ("the fixture has drifted..."), not on the "newest first, by counter"
  assertion the test is named for. Both outcomes match the comment
  word-for-word. Mutation reverted; `git status --short` confirmed clean
  before moving on.
- The four citation-format rewordings in `arrival.rs`, `op.rs` (twice) and
  `transport.rs` (from a full archive path to `` the archived `time-pegged-clock`
  change's `design.md` ``) change no claim, only how the same target is named.
  `git ls-files openspec/changes/archive` confirms
  `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md` is still
  the folder the new wording resolves to.
- The `design.md` Decision 11 additions were checked against what they
  describe: the `c1f1a8f` test claim (re-measured above, matches) and the
  `revision.rs` paragraph (flagged above) are both accurately described as
  being added; `design.md` itself asserts nothing false independent of the
  `revision.rs` text it points at.
- The `op-ordering` spec.md two-sentence split (bold sentence into two
  paragraphs) changes no WHEN/THEN/AND clause and drops no requirement —
  confirmed by reading the full requirement block; this is a readability-only
  edit and out of this dimension.

### Gates run (this re-review)

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: **1180 passed** (lib) + **30 passed** (`end_to_end.rs`), 0
  failed, on the clean tree (after reverting the temporary mutation above).
- `nix build ./dialectica#lgx` from the tree root: succeeded (no output, exit
  0).
