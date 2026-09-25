# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

This change specifies current behaviour. No task changes what a feed read or a
thread read returns. If a test written to the spec fails, report it to the
`spec-writer` and do not fix it here (`design.md`, Non-Goals).

### 1. Check the spec against the code

- [x] 1.1 Read each `feed-read` and `thread-read` requirement against
      `feed.rs::list_threads`, `wire.rs::list_threads_inner`,
      `list_threads_from_request`, `parse_index`, `feed_page_json` and
      `thread_page_json`. Verified: no disagreement was found, and the suite
      below passes with no production code changed.

### 2. Closed sets, each asserted exactly against a hand-written list (`wire.rs`)

- [x] 2.1 A row that is revised, hidden, carries an attachment and has a
      voted-on reply, read with `includeHidden:true`: the row's nine keys, the
      envelope's three, and `{marked, removed, text}` for `body` and for the
      attachment. The fixture's states are asserted before the key sets.
      Verified by
      `a_feed_row_revised_hidden_with_an_attachment_and_a_voted_reply_carries_no_further_key`,
      which the mutations in `design.md` Decision 3 turn red.
- [x] 2.2 The envelope is exactly `{hasMore, items, page}` on a page with a
      successor, on the last page and past the end. Verified by
      `the_feed_envelope_carries_exactly_items_page_and_has_more_on_every_page`,
      which an added `total` turns red.
- [x] 2.3 The plain row with and without `latestReply` is already covered by
      `the_feed_reply_is_the_ecosystems_pagination_shape` and
      `a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`.
      Verified: an added `authorKey` turns both red.

### 3. Behaviours the spec states that no test pinned

- [x] 3.1 A parentless post carrying a `thread` field is a row, and the thread
      it names does not count it. Verified by `feed.rs`'s
      `a_parentless_post_carrying_a_thread_field_is_a_row_of_its_own`, which a
      row filter demanding `thread: None` turns red.
- [x] 3.2 The page index `usize::MAX` is an empty page reporting that index,
      and one larger is refused with a message naming `page`. Verified by
      `the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`.
- [x] 3.3 `page` and `perPage` of `-1`, `1.5`, `1e2` and `"many"` are each
      refused, and the message names the field. Verified by
      `malformed_pagination_fields_are_refused_by_name`, which a message
      without the field name turns red.
- [x] 3.4 `thread-read`: no item key but `author` holds the signer's hex, at
      any depth. Verified by
      `no_thread_item_holds_its_signers_key_under_any_key_but_author`, which
      copying the key into `position` turns red.

### 4. Prose and markers the spec now answers

- [x] 4.1 `FeedRow::author`'s doc comment cites `feed-read` in place of "no
      capability owns this reply's shape". Verified by
      `git grep -n "No capability owns" -- dialectica/rust-lib`, which finds
      nothing.
- [x] 4.2 Remove the `NO SPEC:` marker in
      `a_row_carries_the_public_key_and_no_derived_display_name`, and replace
      its reference to the nonexistent
      `the_feed_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
      with the three `wire.rs` key-set tests. Verified by `git grep`, which
      finds neither.
- [x] 4.3 Remove the two thread-read markers on `author` (`thread_page_json`,
      and `the_wire_reports_the_author_as_one_key_and_no_name`). Each now cites
      *An item's author key travels under the JSON key `author`*. The markers
      on `position`, `assertedTime` and the moderation spellings stay.
      Verified by `git grep -n "NO SPEC:" -- dialectica/rust-lib`.

### 5. Gates

- [x] 5.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
      passes.
- [x] 5.2 `nix build ./dialectica#lgx` succeeds from the tree root.
