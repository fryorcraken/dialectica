## Stages

- [ ] ~~spec — `spec-writer`~~ — **does not apply.** A test-only piece adds no
      requirement, so it has no spec delta; `.openspec.yaml` sets
      `skip_specs: true` with the reason. Struck through rather than omitted,
      because "does not apply" and "nobody did this" are different states and the
      block exists to tell them apart.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

**On the two ticked review rows:** `findings/readability.md` and
`findings/architecture.md` exist on the branch, committed in `df91185`, each with
its own defect list. Those are the evidence. The correctness and security rows are
unticked because no such file exists — one instance was still running when this
block was written, so those rows are doing exactly the job an unticked row is for.

**On the `tester` row:** this piece's code *is* tests, written by `dev-writer`
alongside the target. That does not discharge the `tester` stage, which asks a
different question — do the tests pin what the spec requires, and can they fail —
and answers it from the spec rather than from the code. Ticking it here would be
the false statement the row exists to prevent.

## 1. The integration target

- [x] 1.1 Add `dialectica-core/tests/end_to_end.rs`, importing `dialectica_core`
      as an outside consumer and touching nothing private — verified by the file
      compiling with no `pub(crate)` or private-item reference, and by
      `cargo test -p dialectica-core --test end_to_end` reporting 24 passed.
- [x] 1.2 Make every store a file and every restart a dropped connection plus a
      reopened path — verified by the mutation that makes `SqliteOpLog::open`
      return `:memory:`, which kills 21 of 24 tests; the survivors are the two
      pure-value tests and one whose claim the mutation does not change.
- [x] 1.3 Cover the read path from the keystore file through the store file to the
      feed — verified by
      `a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it`,
      whose expected author address is re-derived from the keystore reopened from
      disk rather than read from the row.
- [x] 1.4 Reproduce the encoder/decoder asymmetry as it stands on `main` —
      verified by `an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever`
      asserting `Ok(Appended::Stored)` on append and `CorruptEntry` on every
      subsequent read, across a restart.

## 2. Reach the JSON boundary (architecture finding)

- [x] 2.1 Add a test crossing `wire::list_threads_from_request` on the happy path
      — verified failing when the wire row's `author` is emitted as `""`, against
      an address derived independently from the post author's key.
- [x] 2.2 Add a test proving a storage failure reaches the view as
      `{"error":...}` and never as an empty page — verified failing when the
      handler returns an empty page on a failed store open, **while the
      `feed::list_threads`-level test one layer down stays green**, which is the
      measurement that made extending the target the right call rather than
      renaming the file.
- [x] 2.3 State in the header what this target does not cover — the publish path,
      `list_stoas`, membership, transport, the view — verified by inspection
      against `grep -rn "list_stoas\|listStoas"` over `dialectica/`, which finds
      only a `docs/PLAN.md` line.

## 3. Correct the CI count gate (both findings, same lines)

- [x] 3.1 Replace the three-root list with one rglob excluding `target/` —
      verified by running both shapes against the tree: rglob 530, three roots
      530, cargo `ran` 530.
- [x] 3.2 Make the `examples/` claim true rather than delete it — verified by
      restoring the review's `examples/probe.rs` holding one `#[test]`: `declared`
      becomes 531 while cargo still runs 530 and reports the example's test in no
      `Running` line, so `ran != declared` fires. Under the old list both numbers
      stayed at 530 and the gate passed, which is the review's measurement
      reproduced.

## 4. Stop the comments claiming what they cannot (readability findings)

- [x] 4.1 Replace the individual field reads in the vote test with an exhaustive
      `FeedRow` destructure — verified by adding `pub score: i64` set to `7`, which
      now fails with `E0027: pattern does not mention field 'score'`. The same
      mutation left the previous form passing.
- [x] 4.2 Replace the fabricated ordinal citation with `op.rs`'s own argument —
      verified by reading `.claude/agents/README.md`'s list, whose second entry is
      the hash-mutation defect and which does not contain a cap test at all.
- [x] 4.3 Fix the `FIELD_CAP` header premise — verified against
      `op.rs::the_field_cap_is_pinned_to_a_known_answer` (`MAX_FIELD_LEN == 150 * 1024`)
      and `stoa.rs` (`MAX_TITLE_BYTES == 1024`), both hardcoded, which the original
      premise said did not exist.
- [x] 4.4 Drop the uncited "three bugs" count — verified by
      `grep -rn "cross-layer"` over `openspec/` and `docs/`, which returns nothing
      but the findings file itself. The argument now rests on the shape of the
      defect rather than a tally.
- [x] 4.5 Drop the count from the section heading that said "three boundaries"
      over four — verified by counting the `#[test]` attributes between that
      divider and the next.
- [x] 4.6 Date the mutation table and say what a later author owes it — verified
      by `git log --oneline -- <the test file>`, which names `9bb2bc1` as the
      commit the first group was measured at.

## 5. Two reshapes

- [x] 5.1 Give `TempDir` a filename parameter (`store_at`, `reopen_at`) so the
      over-cap test stops hand-rolling the restart primitive — verified by the
      over-cap test passing through the helpers, and by the `:memory:` mutation
      still killing it.
- [x] 5.2 Use the recorded 16-byte shared prefix instead of 31 — verified by
      mutating `iter_stoa` to compare `substr(stoa, 1, 8)`, which still leaks
      (`["right","left"]` vs `["left"]`), so the fixture is as strong as the one it
      replaced.

## 6. Split the two-claim test names

- [x] 6.1 Split `an_empty_store_answers_every_read_and_a_missing_file_is_a_created_one`
      — verified by the mutation making `open` refuse a missing path, which fails
      the creation test at its own behavioural assertion rather than at a shared
      fixture.
- [x] 6.2 Split `a_page_past_the_end_is_an_empty_page_and_the_pages_before_it_tile_the_feed`
      — verified by the paging-slice mutation, which now fails the tiling test
      alone and leaves the boundary test green, naming which claim went.
- [x] 6.3 Fix the defect the split exposed: the past-the-end half survived the
      `:memory:` mutation, because an empty page 99 is also what a store holding
      nothing answers. Verified by the added fixture guard, which fails under that
      mutation and raises its kill count from 20 to 21 of 24.

## 7. Gates

- [x] 7.1 `cargo test -p dialectica -p dialectica-core` — 530 passed, 0 failed.
- [x] 7.2 `rustfmt --check --config skip_children=true` clean on the test file;
      nothing pre-existing reformatted.
- [x] 7.3 `cargo clippy --all-targets -- -D warnings` clean. The six remaining
      warnings are the staged upstream SDK's and pre-date this change.
- [x] 7.4 No `ignore`-fenced doc block added — verified by `grep -n '```'` over the
      test file returning nothing, so no doc-test is registered and the count gate
      is unaffected.
