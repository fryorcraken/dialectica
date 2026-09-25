# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
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

No production code changes (`design.md` D1). Every task is a test, or a
refactor that makes room for one.

## 1. Make room: a keep test can send any `index` text

- [x] 1.1 Move the keep request building into `keep_with_raw_index` and have
  `keep_through_the_wire` delegate to it. Verified by the full suite staying
  green on that commit alone, which changes no behaviour.

## 2. A malformed `index` is refused by name (`identity-onboarding`)

- [x] 2.1 *Each malformed kind of `index` is refused by name*:
  `each_malformed_kind_of_index_is_refused_by_name` sends all eight kinds from
  the scenario. It asserts the error shape, a message containing `index`, and
  no keep field. Verified red with `{field}` dropped from `parse_index`'s
  wrong-type message.
- [x] 2.2 *A malformed `index` stores nothing*: `a_malformed_index_stores_nothing`
  runs on a fresh peer (no keystore appears, no path recorded) and on a peer
  already holding a master key (bytes unchanged, no path recorded). It ends with
  a well-formed keep on the same fixture, which does store.
- [x] 2.3 *A well-formed `index` naming no candidate is a not-kept reply*:
  `a_selection_outside_the_set_is_refused_and_stores_nothing` now also asserts
  a non-empty `reason` and no `error` for `SLATE_SIZE` and beyond.
- [x] 2.4 `malformed_pagination_fields_are_refused_by_name` still passes,
  unchanged. Verified by the full suite.

## 3. Positions at the wire (`thread-read`, no spec change)

- [x] 3.1 Add the `a_thread_log_with_shared_authors` fixture (five items, two
  pairs sharing an author) and a page-walking reader. The uniqueness test
  asserts the fixture really does have two items by one author.
- [x] 3.2 *Two items of one thread never share a position*:
  `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`.
  Verified red under a constant position, `item.author`, and a per-page
  `enumerate` index in `thread_page_json`.
- [x] 3.3 *A position … does not restart per page*:
  `a_position_is_the_same_whatever_page_size_the_read_used`. Verified red under
  the per-page index. It is green under the constant and the per-author value
  by design, as noted in the test and in `design.md` D2.

## 4. Gates

- [x] 4.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` green.
- [x] 4.2 `nix build ./dialectica#lgx` green.
