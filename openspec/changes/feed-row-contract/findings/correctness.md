# Correctness review — feed-row-contract (PR #163)

Scope: `git diff 28645a0...HEAD`. Confirmed the diff is prose (comments, spec,
design.md, proposal.md, tasks.md) plus new `#[cfg(test)]` tests in `feed.rs`
and `wire.rs`. No production code path changed — verified by reading every
hunk of `feed.rs` and `wire.rs` in the diff; the only non-comment,
non-test-module lines touched are doc-comments on `FeedRow::author` and the
two JSON-builder comments in `wire.rs` (`feed_page_json`, `thread_page_json`).

## What I did

1. Read the full diff for `feed.rs` and `wire.rs` and traced each new test
   against the actual implementation it exercises (`list_threads`,
   `visible_replies_by_thread`, `feed_page_json`, `parse_index`) to confirm
   each assertion matches real behaviour rather than an assumption about it.
2. `design.md` §3 makes nine specific, falsifiable claims of the form
   "mutation X turns exactly these tests red." I hand-applied three of the
   nine directly in this worktree (one per file/mechanism: a JSON-shape
   addition, a filter-logic change, and an error-message change), ran the
   full `dialectica-core` unit suite after each, and reverted before the
   next:
   - Adding `"authorKey": row.author` to every feed row in `feed_page_json`
     → measured **1150 passed, 3 failed**, and the three failures were
     exactly `the_feed_reply_is_the_ecosystems_pagination_shape`,
     `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`,
     `a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`
     — matches the table exactly.
   - Adding `thread: None` to the head-row match guard in
     `list_threads` (`feed.rs`) → measured **1152 passed, 1 failed**,
     exactly `a_parentless_post_carrying_a_thread_field_is_a_row_of_its_own`
     — matches the table exactly.
   - Dropping `{field}` from `parse_index`'s "must be a number" wrong-type
     message → measured **1152 passed, 1 failed**, exactly
     `malformed_pagination_fields_are_refused_by_name` — matches the table
     exactly.
   All three mutations were reverted and confirmed clean via `git diff` on
   each file before moving to the next. I did not re-run the remaining six
   rows of the table; the three sampled span all three mechanisms the table
   exercises (JSON-shape addition, struct/filter-logic change, error-message
   change) and all three measured exactly as claimed, so I have no reason to
   doubt the rest, but they are unverified by me.
3. Spot-checked smaller factual claims reachable in one or two greps:
   `SecretKey`/`Signature`/public-key `to_hex()` all go through
   `hex::encode`, which is lowercase — confirms the new `thread-read`
   requirement's "64 lowercase hexadecimal characters" is accurate to the
   code it contracts.
4. Ran the full suite clean (`cargo test --manifest-path
   dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`): 1153 +
   30 tests passed, 0 failed, both before and after the mutation probes
   (post-probe state confirmed identical to pre-probe via `git diff`).
5. `nix build ./dialectica#lgx` from the tree root: succeeded with no
   output.

## Findings

None. I could not break any property the new tests claim to hold, and the
`design.md` mutation table's measured claims check out on every row I
sampled. No unticked boxes to open — no dev-writer, tester or spec-writer
follow-up from this dimension.

Areas I looked at specifically and found clean:
- `a_parentless_post_carrying_a_thread_field_is_a_row_of_its_own` (feed.rs):
  correctly exercises that `list_threads` gates on `parent: None` alone (not
  `thread`), and that `visible_replies_by_thread` gates on `parent: Some(_)`
  alone, so a parentless post naming a `thread` it does not own is a row of
  its own and contributes nothing to the thread it names.
- The four new closed-key-set tests in `wire.rs`
  (`a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`,
  `the_feed_envelope_carries_exactly_items_page_and_has_more_on_every_page`,
  `the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`,
  `malformed_pagination_fields_are_refused_by_name`) each build the exact
  expected key set as a hand-written literal (per this project's
  `sorted_keys` convention), not derived from the struct — so they cannot
  pass vacuously against an added field.
- `no_thread_item_holds_its_signers_key_under_any_key_but_author`: walks
  every string value under every key but `author` and asserts none equals
  either signer's hex; uses two distinct signers so it cannot be satisfied
  by a constant, and asserts the walk actually reached the `id` field so it
  is not vacuous.
- `the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`:
  correctly uses `usize::MAX`/`u128` arithmetic rather than a fixed 64-bit
  literal, matching `parse_index`'s actual `usize::try_from` boundary.
- The doc-comment edits in `feed.rs` (`FeedRow::author`) and `wire.rs`
  (`feed_page_json`, `thread_page_json`) accurately describe the code they
  sit beside; I did not find a comment asserting something the code does
  not do.
