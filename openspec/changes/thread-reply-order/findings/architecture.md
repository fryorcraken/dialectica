# Architecture review — thread-reply-order (#147 / PR #159)

Scope: architecture only, per dispatch. Correctness, security and readability
are separate reviewer instances.

## What I checked

- `git diff origin/main...HEAD` in full (`thread.rs` prod + tests,
  `end_to_end.rs`, the `openspec/` proposal/design/spec/tasks files).
- Whether the fix keeps `thread.rs`'s stated invariant ("no `sort`, no `cmp`,
  no `max_by`") and whether the boundary between `op-ordering` (the rule) and
  `thread-read` (this reversal) is still clean.
- Whether other readers of `iter_stoa` (`feed.rs`, `stoa_metadata.rs`) needed
  the same treatment and correctly did not get it.
- Gates: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core` — 1146 + 30 passed, 0 failed. `nix build
  ./dialectica#lgx` — succeeded (exit 0). `cargo mutants --file thread.rs`
  was started but did not finish inside a reasonable window; abandoned per
  the "couple of minutes" guidance rather than run further. `design.md`
  Decision 1 already documents a manual mutation (ascending re-sort) and
  which three tests it turns red, which substitutes for it here.

## Overall

The change is a one-line direction change (`.into_iter().rev()`) plus tests
and docs, and it is unusually well-contained for what it touches. Specifically:

- **The complexity is in the walk direction, not in a new branch.** This is
  the CLAUDE.md principle ("make the change easy, then make the easy change";
  "complexity in the data structure, not the logic") applied about as
  cleanly as it can be: no new `if`, no new comparator, the existing
  `is_root` / membership / moderation per-entry logic in `read_thread` is
  untouched.
- **The capability boundary holds.** `design.md` Decision 2 considers and
  rejects changing `cmp_ops` itself, which would have fixed this reader by
  breaking `revision::current_version`, `moderation::resolve` and the feed's
  `latestReply` — all three depend on the rule's "places first" meaning
  latest-wins/highest-counter. Checked directly: `iter_stoa` is called
  un-reversed in `feed.rs` and `stoa_metadata.rs`, and only `thread.rs`
  reverses it. No other reader needed the same change and none got it —
  correctly.
- **No second ordering rule was introduced.** Decision 1's alternatives
  (ascending re-sort, `sort_by(|a,b| cmp_ops(b,a))`) are both rejected with
  concrete reasons (asymmetric tiebreaks; reintroducing a comparator into a
  module whose header claims to have none), and the chosen approach avoids
  both. Verified against the code: `thread.rs` contains no `sort`/`cmp`/
  `max_by` outside the doc comments describing their absence.
- **Position assignment stays correct under the new walk.** `Placed::at` is
  applied after the full walk (root insert included), over the whole thread
  before paging — so an item's position is stable across page-size changes
  and doesn't depend on when in the walk the root happened to be found. This
  was pre-existing structure and the diff doesn't disturb it.
- No new dependency, no Cargo.toml change, no CI-relevant file moved.

I have one non-blocking observation, not a defect in the shipped behaviour.

- [ ] **`dev-writer`/`tester`** — `dialectica/rust-lib/dialectica-core/src/thread.rs:2952` (`a_root_at`, added by this change)
      **Finding:** the test module now has three separate hand-rolled
      `Op { .. }.sign(&key)` constructors that each build a `Post` op from
      scratch: `a_post_in` (pre-existing, no clock), `a_reply_at`
      (pre-existing, always a reply, always `Some(clock)`), and `a_root_at`
      (new in this change, always a root, always `Some(clock)`). `a_root_at`
      duplicates `a_post_in`'s stoa/author/kind construction rather than
      extending `a_post_in` (or a shared low-level builder) with an optional
      clock parameter. This is CLAUDE.md's "fourth slightly-different copy of
      a guard is a signal to reshape" pattern, one instance short of "fourth"
      — three near-identical constructors now exist for what is really one
      operation (build a signed `Post`) varying only in `parent`/`thread`
      and `clock`.
      **Severity:** low, and arguably matches existing precedent —
      `a_reply_at` already duplicated `a_post_in` before this change, so
      `a_root_at` is consistent with the file's existing (already slightly
      duplicative) fixture style rather than a new pattern this change
      invented. Test-only code, no production-path impact. Flagging so a
      fifth variant doesn't get added without someone noticing there's now a
      reshape opportunity (e.g. a single `a_post_with(parent: Option<&SignedOp>,
      clock: Option<(u64, u64)>, ...)` builder). Not a blocker.

## Areas I looked at and found clean

- `read_thread`'s decomposition (verify → kind filter → membership via
  `thread_of` → `resolve_item` → position → page) is unchanged by this diff
  and still keeps each job separate; the reversal didn't get folded into any
  of those steps as an extra condition.
- `resolve_item` remains order-agnostic — it resolves one entry and knows
  nothing about where it lands in the sequence, which is what makes the
  reversal safe to implement as a one-line walk-direction change instead of
  a change threaded through the per-item logic.
- The four new production-doc-comment sites (module header, `read_thread`'s
  doc list, the inline "THE REPLY ORDER IS DECIDED HERE" block, and the
  Risks section reflected in `design.md`) all state the same rule
  consistently — reversed-not-resorted, root pinned first, no comparison
  written — so there's no drift between what the code says at different
  sites.
