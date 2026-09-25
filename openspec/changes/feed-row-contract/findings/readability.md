# Readability review — feed-row-contract

Scope: readability only, per dispatch. Covers the full diff since `28645a0`
(`git diff 28645a0...HEAD`): `feed.rs`, `wire.rs` (doc comments + new tests),
`design.md`, `proposal.md`, `tasks.md`, the two spec deltas, and the live
`feed-read` spec's corrected Purpose.

## Findings

- [ ] **`spec-writer`** — `openspec/changes/feed-row-contract/proposal.md:62,71,112,116`
      — two of the change's own line-number citations resolve to unrelated code
      in the final tree, not to the thing they claim to point at.
      **Scenario:** proposal.md states "`feed.rs:657` is now `feed.rs:863`" and
      that the second marker "is still present twice, at `wire.rs:2151`
      (`thread_page_json`) and at `wire.rs:8549`
      (`the_wire_reports_the_author_as_one_key_and_no_name`)". In the tree this
      change ships, `feed.rs:863` is `.unwrap();` inside an unrelated foreign-Stoa
      test, and `wire.rs:8549` is `decided.sort();` inside an unrelated
      moderation-shape test — neither line is inside, or even near, the function
      named beside it. `a_row_carries_the_public_key_and_no_derived_display_name`
      is actually at `feed.rs:886`, and `the_wire_reports_the_author_as_one_key_and_no_name`
      is at `wire.rs:8807`. I confirmed the citations were computed against the
      tree at commit `f06d763` (spec-writer's own commit, before the dev-writer's
      and tester's edits added ~45 lines to `feed.rs` and ~270 lines to `wire.rs`
      ahead of those points) and were never recomputed after. A reader who
      follows either citation today to verify the claim it makes lands in
      unconnected code. `wire.rs:2151` (also cited) still lands inside the right
      `json!({...})` block and is fine; only the two above are broken.
      **Severity:** low — these are prose citations in a proposal document, not
      code or spec text, and the claims themselves are otherwise correct
      (verified independently against the current tree). But `design.md`
      elsewhere is careful to say a citation is "from the tree at #88" when it
      is stale by construction; these two read as current-tree facts and are
      not marked as historical, so they mislead rather than merely age.
      **Measured:** `grep -n "fn a_row_carries_the_public_key_and_no_derived_display_name" dialectica/rust-lib/dialectica-core/src/feed.rs` → line 886; `grep -n "fn the_wire_reports_the_author_as_one_key_and_no_name" dialectica/rust-lib/dialectica-core/src/wire.rs` → line 8807; both diverge from the 863/8549 proposal.md cites.

## Clean

- `feed.rs`'s and `wire.rs`'s doc-comment rewrites (the `FeedRow::author` field
  comment, the `feed_page_json`/`thread_page_json` inline comments, and the
  resolved `NO SPEC:` markers) read clearly, name the resolving requirement by
  its exact quoted text, and correctly distinguish "this is #80's history" from
  "this is what `feed-row-contract` adds" — a reader does not have to guess
  which change is responsible for which sentence.
- The new tests in `wire.rs` (`sorted_keys`, `strings_under`,
  `a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`,
  `the_feed_envelope_carries_exactly_items_page_and_has_more_on_every_page`,
  `the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`,
  `malformed_pagination_fields_are_refused_by_name`,
  `no_thread_item_holds_its_signers_key_under_any_key_but_author`) and the new
  test in `feed.rs` follow the codebase's existing long-descriptive-name
  convention consistently, and each has a comment that states which spec
  requirement it pins and what mutation it is a guard against — none of them
  needed a second reading to understand what they check or why.
- `design.md`'s mutation table (Decision 3) and its surrounding prose are easy
  to follow: the table's claim ("the right-hand column is the complete red
  set") is checkable and phrased as a measurement rather than an assertion.
- `proposal.md` and `design.md`'s division of labour — "why" and "what changed"
  in proposal.md, "how it was verified" in design.md — is followed
  consistently; nothing needed cross-referencing to understand.
- The two spec deltas (`specs/feed-read/spec.md`, `specs/thread-read/spec.md`)
  and the live `feed-read` Purpose correction read as clear, testable
  requirements with scenarios that match their prose; no ambiguous pronoun or
  unstated antecedent found.
- `tasks.md` is consistent with what the diff actually contains; no task claims
  work that isn't there.

## Gates run in this tree

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` — passed (1153 + 30 = 1183 tests, 0 failed).
- `nix build ./dialectica#lgx` — succeeded.

Dimension covered: **readability only**, as instructed. No mutation testing
was performed (out of lane for this dimension) and no source files were
modified in this tree beyond this findings file.
