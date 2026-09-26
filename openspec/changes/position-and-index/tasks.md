# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Re-review of the findings-round commits

The runner records this. It covers the commits after `19667a04`:
- `b55d164`: `thread-read` delta MUST→SHALL; live `identity-onboarding`
  Purpose gains a `feed-read` boundary paragraph; proposal note.
- `70bef05`: test-only fixture refactor in `wire.rs`.
- `9e550eb`: `design.md` D1, D2 and D5 rewritten as measured.
- `4e2b8c0`: rustfmt of one in-piece line.

- [x] re-review: correctness — `code-reviewer`. Test fixture refactored in `wire.rs`.
- [x] re-review: readability — `code-reviewer`. Spec keyword change, `design.md` prose, formatting.
- [x] re-review: architecture — `code-reviewer`. Live-spec Purpose edit, test helper structure.
- [x] re-review: spec-test — `spec-test-reviewer`. Fixture refactor under the position tests, `thread-read` delta edit.
- [x] re-review: design — `design-reviewer`. D1, D2 and D5 rewritten.
- [ ] ~~re-review: security — `code-reviewer`~~ Not re-run: no production or
  boundary code changed; the new commits are test fixtures and prose, and the
  five rows above cover every one of them.

## Re-review of the D6 commit

The design re-review raised one finding, answered by `536a03e`, which adds
`design.md` D6 and changes nothing else but its finding's outcome. Its ground
is design prose, so only the two dimensions that read design prose re-run.

- [x] re-review D6: design — `design-reviewer`. The finding was theirs; D6 is a Decisions entry.
- [x] re-review D6: readability — `code-reviewer`. New prose with checkable citations.
- [x] re-review the D6 wording fix: readability — `code-reviewer`. The D6
  readability finding ("Two things" against one) is answered in `design.md`;
  only the dimension that raised it re-reads the fix.
- [ ] ~~re-review D6: correctness, security, architecture, spec-test~~ Not
  re-run: `536a03e` touches no code, test or spec.

## Re-review after merging main

The first closer stopped because `main` had moved: #175, #172, #173 and #181
landed after the piece's base `8368b2f`, and #173 removed a paragraph from the
`thread-read` requirement this change MODIFIES. This round covers:
- `d0f56a14`: `git merge origin/main`, clean, with cargo test,
  `nix build ./dialectica#lgx` and `openspec validate` green on the merged tree.
- `7d587fd`: the `thread-read` delta rebased on the live text after #173.
- `696d35f`: `design.md` D2 and D5 and `proposal.md` quotes corrected against
  the merged tree.

- [ ] re-review after merge: architecture — `code-reviewer`. The delta must replace the live requirement with nothing lost at archive.
- [ ] re-review after merge: spec-test — `spec-test-reviewer`. The tests on the merged tree must still pin the reconciled delta.
- [ ] re-review after merge: design — `design-reviewer`. D2 and D5 rewritten, proposal note changed.
- [ ] re-review after merge: readability — `code-reviewer`. Quotes and citations in the rewritten prose.
- [ ] ~~re-review after merge: correctness, security~~ Not re-run: this
  piece's code and tests are unchanged since their last review; the code the
  merge brought in was reviewed in its own PRs; cargo test is green on the
  merged tree, and spec-test re-runs the piece's tests there.

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

## 3. Positions at the wire (`thread-read`)

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
- [x] 3.4 *The item at a place carries that place's position in every read*
  (the `tester`'s, per `design.md` D5): a wire test that hides a reply with
  another reply after it, then reads the thread across every page with
  `includeHidden` true and without. It asserts that the items at each place both
  reads fill carry the same position, and that the following reply carries a
  different position in each read. Verified red with the position set to the
  item's op id in `thread_page_json`, and under a constant position. The fixture
  gives each reply an explicit ascending `OpClock` counter rather than
  `clock: None`, because `arrival::cmp_ops` ties `clock: None` ops on ascending
  `OpId` — an unpredictable hash — and the scenario needs one specific reply
  adjacent to the hidden one; the first version of this fixture (append order,
  no explicit counters) failed against the correct implementation for exactly
  that reason.

## 4. Gates

- [x] 4.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` green.
- [x] 4.2 `nix build ./dialectica#lgx` green.
