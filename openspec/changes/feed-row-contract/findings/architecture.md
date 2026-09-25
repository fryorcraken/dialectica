# Architecture review — feed-row-contract

Scope: architecture only, per dispatch. Reviewed everything on
`piece/91-feed-row-contract` since `28645a0` (`git diff 28645a0...HEAD`, three
dots): `dialectica-core/src/feed.rs`, `dialectica-core/src/wire.rs`, and the
change's `openspec/` documents (`proposal.md`, `design.md`, `tasks.md`, the two
spec deltas, and the direct edit to the live `openspec/specs/feed-read/spec.md`
Purpose).

## What I checked and how

- Confirmed by reading the diff hunks (not just the PR description) that
  `feed.rs` and `wire.rs` changes are doc-comment/prose edits plus new
  `#[cfg(test)]` code only — no line inside `feed_page_json`, `thread_page_json`,
  `list_threads`, `list_threads_inner`, `list_threads_from_request` or
  `parse_index` changed. The claim in the brief ("changes no production logic")
  holds.
- Traced the "editing the live Purpose directly" move against its cited
  precedent, commit `e4938c2` ("Spec: thread-read's Purpose names no reply
  direction (#147)"). Same pattern: a delta carries no Purpose, so the live
  Purpose is corrected in place to a wording that is true both before and after
  archive. The precedent is real, not fabricated, and the new `feed-read`
  Purpose text (five boundaries, including `post-revision` and
  `generated-names`) is consistent with both the *unchanged* live requirements
  (reply count / latest reply / order-stability) and the new ADDED requirements
  that will be promoted at archive (author, body/attachments, isHidden,
  pagination) — it doesn't overclaim or underclaim coverage either way.
- Confirmed `generated-names` and `post-revision` are real, existing capability
  names (`openspec/specs/generated-names/spec.md`, referenced elsewhere), not
  dangling references.
- Confirmed the `thread-read` addition (`author` JSON key naming) is the one
  the proposal itself flags as an open scope question with an explicit escape
  hatch ("the `thread-read` delta can be dropped without affecting anything
  else"). That's disclosed, not smuggled in, so I'm not opening a box for it.
- Ran the full gate set: `cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` (1153 + 30
  passed, 0 failed), `nix build ./dialectica#lgx` (succeeded), and `openspec
  validate feed-row-contract --strict` (valid). No dependency changes, no
  `ci.yml` changes, nothing moved that a layout-derived gate would miss.

## Finding

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7432` and
      `wire.rs:7532` — the new `sorted_keys` helper (added at `wire.rs:7569` by
      this change) is not used by the two pre-existing key-set tests it was
      visibly modeled on, leaving three parallel copies of the same
      "collect keys, sort, compare" logic in one file.
      **Scenario:** `the_feed_reply_is_the_ecosystems_pagination_shape`
      (`wire.rs:7416`) and `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`
      (`wire.rs:7489`) both still inline
      `let mut keys: Vec<&str> = row.as_object().unwrap().keys().map(|k| k.as_str()).collect(); keys.sort_unstable();`
      immediately before the same style of `assert_eq!(keys, [...])` that the
      new `sorted_keys()` helper exists to do once. This change adds three new
      tests that *do* use the helper
      (`a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`,
      the same test's body/attachment key-set checks, and
      `the_feed_envelope_carries_exactly_items_page_and_has_more_on_every_page`),
      so the file now carries both forms side by side for the identical
      operation. Per CLAUDE.md's "one function, one job" / "fourth
      slightly-different copy of a guard is a signal to reshape": this is
      exactly the moment the duplication was recognized (the helper's own doc
      comment explains *why* the set must be a literal, which is the same
      reasoning the two older tests already follow inline) — the extraction
      just wasn't carried back to the tests that motivated it. Not a
      correctness defect (both forms are correct and independently green) and
      not blocking; a future rename of the pattern (e.g. adding a "no
      duplicate key" check to the helper) would need to touch the two
      un-migrated call sites separately, and a fifth test could easily be
      added copying the older, un-helpered form instead of the new one.
      Severity: low / cleanup. This is a style preference about consistency,
      not a defect — flagging so `tester` can decide whether to fold the two
      old call sites into `sorted_keys()` while touching this file, not
      demanding it be done before merge.

      **Outcome: fixed.** Folded both call sites into `sorted_keys()`.
      `the_feed_reply_is_the_ecosystems_pagination_shape` now does
      `let keys = sorted_keys(row);` (the `!keys.contains(&"displayName")`
      check became `!keys.iter().any(|k| k == "displayName")`, since
      `sorted_keys` returns `Vec<String>` rather than `Vec<&str>`), and
      `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`'s
      `assert_eq!(keys, [...])` became `assert_eq!(sorted_keys(row), [...])`
      directly. This is a pure refactor of test code with no behaviour change
      to pin — both tests assert the identical key sets before and after, and
      both still pass (`cargo test -p dialectica-core
      the_feed_reply_is_the_ecosystems_pagination_shape
      a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`). No
      mutation applies: there is nothing here for a test to newly catch, only
      one fewer copy of logic that could drift.

## Clean

- Capability boundary choice (extend `feed-read` rather than add a new `feed`
  capability) is argued and holds up: `feed-read` already existed and owned
  exactly this read; splitting would have put one `listThreads` reply under
  two specs.
- The `feed.rs` vs `wire.rs` test split follows the project's own established
  convention (struct-level pin in `feed.rs`, wire-shape pin in `wire.rs`) and
  the new tests respect it — nothing crosses the line into the wrong file.
- No new dependency, no CI workflow change, no moved/renamed file for a gate to
  miss.
- The direct edit to the live `feed-read` Purpose is not a novel pattern
  invented for this change; it repeats an established, precedented mechanism
  and the new wording is accurate against both the current and the
  post-archive requirement set.
- `sorted_keys` and `strings_under` (the two new test helpers) don't shadow or
  duplicate any existing helper in the file.
