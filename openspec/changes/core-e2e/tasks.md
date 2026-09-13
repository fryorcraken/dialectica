## Stages

- [ ] ~~spec — `spec-writer`~~ — **does not apply.** A test-only piece adds no
      requirement, so it has no spec delta; `.openspec.yaml` sets
      `skip_specs: true` with the reason. Struck through rather than omitted,
      because "does not apply" and "nobody did this" are different states and the
      block exists to tell them apart.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — **deliberately not done, and the runner accepts it.**
      See the note below: this piece's code *is* tests, and a `tester` stage asks a
      question it cannot answer here. Struck rather than ticked, so the row stays
      honest, and struck rather than left bare, so a reader can tell a decision
      from an oversight.
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

**On the four ticked review rows:** `findings/readability.md` and
`findings/architecture.md` were committed in `df91185`; `findings/correctness.md`
and `findings/security.md` followed from the instance that was still running when
this block was first written. Each carries its own defect list, and those files are
the evidence. `spec-test` and `design` have since been ticked too, each carrying
its own findings file, so every review row is now answered by a file rather than by
this note.

The correctness and security reviewer measured against `8bfe77d` rather than the
`9bb2bc1` its dispatch named, and says so in both files: two of the findings it had
measured at `9bb2bc1` were already closed by `5a1719b`, and are recorded as closed
rather than as boxes. Its `examples/` measurement was reached independently of
§3.2's and agrees with it.

**On the `tester` row:** this piece's code *is* tests, written by `dev-writer`
alongside the target. That does not discharge the `tester` stage, which asks a
different question — do the tests pin what the spec requires, and can they fail —
and answers it from the spec rather than from the code. Ticking it here would be
the false statement the row exists to prevent.

**The runner's ruling, 2026-09-13: struck through, not ticked, and the piece
merges anyway.** The argument above is right and is honoured rather than
overruled. Two things make the absence acceptable here and would not elsewhere:

- **The question has no spec to ask it from.** A `tester` works from the
  contract, and this piece declares `skip_specs: true` because it adds no
  requirement. The one spec gap it *did* surface — the feed read having no
  promoted contract at all — is recorded in `docs/PLAN.md` §9.1 as a named debt
  with an owner, which is the honest place for it.
- **The `spec-test` reviewer asked the tester's question from the other side and
  it failed loudly.** It hardcoded the feed reply's `page` and `hasMore` and
  watched all 25 tests pass, which is exactly the "can these fail?" measurement
  the stage exists to force. That defect is closed (§9.1 above). A stage answered
  by a different role is still answered; a stage answered by nobody is the thing
  the block catches.

Recorded here rather than left as a ticked row contradicting its own note,
because the contradiction was the real defect: two agents each declined to resolve
it and left it for the runner, correctly.

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

## 7. The correctness and security findings

Four boxes, all addressed to `tester`, all closed. The measurements live in
`findings/correctness.md` and `findings/security.md` beside the reviewer's own
text; this section records only what was done.

- [x] 7.1 Move `a_hide_by_a_non_moderator_leaves_the_thread_visible`'s fixture
      guard below the resolution and feed assertions — verified by re-running the
      `Moderators::contains → true` mutation before and after: the hide test dies
      at the guard beforehand and on `Hidden(..)` vs `Unmoderated` afterwards.
- [x] 7.2 Check whether the finding generalises to the other same-shaped guard. It
      does not: the `iter_target` guard in `a_forged_hide_does_not_displace…` is
      correctly placed, measured independently by the authority-after-taking
      mutation, which fails that test at its own `resolve` assertion. One line, not
      a pattern — recorded in the test file's note 5.
- [x] 7.3 Replace both false claims in the `TempDir` doc — verified: `create` was
      read to check only `path.exists()`, `check_directory_mode` was traced to
      `read_checked` (i.e. `open`/`is_encrypted`) only, and the `0o777` probe failed
      both keystore tests at their `open` call sites with
      `DirectoryWritableByOthers { mode: 511 }`. The "a failure leaves a directory"
      half was disproved by `Drop::drop`'s unconditional `remove_dir_all` plus the
      absence of any `panic = "abort"` profile.
- [x] 7.4 Randomise the temp directory name with 8 bytes from `getrandom`, matching
      `keystore.rs:1319-1322`, and drop the unconditional `remove_dir_all` that the
      predictable name made necessary. No new dependency: `getrandom` and `hex` are
      ordinary `[dependencies]` of this crate. Closes both security boxes, which the
      reviewer wrote as two because one edit discharges both.
- [x] 7.5 Pin the randomness with
      `two_temp_dirs_with_the_same_tag_get_different_unguessable_names`, since no
      other test in the suite would notice if it stopped arriving — verified by two
      mutations, the reverted pid-based name (fails the `assert_ne!`) and a 2-byte
      suffix (fails the length assertion at 21 vs 33).

## 8. Gates

- [x] 8.1 `cargo test -p dialectica -p dialectica-core` — **531 passed**, 0 failed
      (506 in-crate + 25 in `tests/end_to_end.rs`). The baseline was 530; the one
      added test is 7.5's.
- [x] 8.2 `rustfmt --check --config skip_children=true` clean on the test file;
      nothing pre-existing reformatted. `moderation.rs` was mutated and restored,
      and `git diff` shows it byte-identical — its own `rustfmt --check` reports
      pre-existing differences in `#[cfg(test)]` code this change did not touch,
      left alone deliberately.
- [x] 8.3 `cargo clippy --all-targets -- -D warnings` clean. The six remaining
      warnings are the staged upstream SDK's and pre-date this change.
- [x] 8.4 No `ignore`-fenced doc block added — verified by `grep -n '```'` over the
      test file returning nothing, so no doc-test is registered and the count gate
      is unaffected. The count gate itself needs no edit: it counts `#[test]`
      occurrences under an rglob, so 7.5's test is counted and run.

## 9. The spec-test findings

Seven boxes in `findings/spec-test.md`; **five addressed to `tester` and two to
`spec-writer`, and all seven are now closed**. The measurements are in that file
beside the reviewer's text.

- [x] 9.1 Close the surviving envelope mutation, which is why this piece was
      extended to the JSON seam in the first place. The review hardcoded
      `"page": 0, "hasMore": false` in the feed reply and **all 25 tests passed**,
      because the only wire happy-path test reads page 0 of a one-post store — so
      the two values it asserts are exactly what a broken handler emits.
      `the_json_envelope_reports_the_page_that_was_asked_for_and_whether_more_follows`
      uses five posts at `perPage: 2`: three pages, a non-zero index, `hasMore`
      true on one read and false on another, plus a tiling assertion. Verified by
      re-applying the same mutation twice — both literals, then `page` alone — so
      each half is shown to discriminate on its own. Recorded as the test file's
      note 6, including the prediction that missed.
- [x] 9.2 Replace the fabricated `§11.1 obligation 5` citation in two test
      comments, citing `docs/UI-BRIEF.md` by **heading rather than by number**.
      The obligation is real at `docs/UI-BRIEF.md:429` and `docs/PLAN.md` states in
      about a dozen places that §11.1 arrives with `vouching-state` and is absent
      until then. The ordinal is dropped because UI-BRIEF restarts its numbering
      per section and contains a `2b`, so "obligation 5" locates no more there than
      "§11.1" did in PLAN.md — a correction I owe the concurrent `dev-writer`,
      which reached it independently.
      Also names the promoted requirement the assertions actually check,
      `module-wire-contract`'s "Failure is always the error shape, and never a
      partial success" (`spec.md:229`). **`wire.rs:469` still carries the same
      citation** and is `dev-writer`'s to fix — recorded in the findings file.
- [x] 9.3 Replace the bare `(§6)` in the moderator-set assertion with
      `moderation-resolution`'s "A Stoa's moderator set is derived from its genesis
      record", verified at `openspec/specs/moderation-resolution/spec.md:40`.
- [x] 9.4 Name `posting-capability` in the "Does NOT cover" list, as a GAP rather
      than an absence: both public entry points named, `spec.md:53`'s six reasons
      cited, and the near-zero fixture cost stated. **Not closed by a test**, and
      the entry says why — three of the six states are reached by making a file
      hostile, and which of them an integration test may construct is a contract
      question this pass leaves to `spec-writer`.
- [x] 9.5 Fix the self-referential grep instruction: it now names both
      `dialectica/` and `docs/`, says what each returns, and states that the
      single-root form "returns only itself, which confirms nothing". Both greps
      re-run here.
- [x] 9.6 The two `spec-writer` boxes: **both upheld, both deferred to an owner,
      with the deferral recorded in `docs/PLAN.md` §9.1** under a new subsection
      "The feed read is built and still has no contract — a named debt, not an
      oversight". No spec text is added, so `skip_specs: true` stays honest.
      The feed gap is real and re-measured (`grep -rli` for both "feed" and
      "list_threads" over `openspec/specs/` returns nothing across all 12 promoted
      capabilities). It is not written here because the feed merged in `0538c0d`
      (PR #23) — a different piece with its own reviewers — and because PLAN.md
      §9.1 already reserves the feed-vs-thread capability split for "whoever writes
      it, against the projection that actually exists". `0538c0d` predates OpenSpec
      adoption, so no delta was skipped.
      The store-lifecycle box's premise is **narrower than it reads**: it greps
      promoted specs only, and four of its five behaviours are already contracted
      in `sqlite-projection`'s in-flight `op-log` delta (that change is at 46/47
      tasks). Only the missing-file-is-created half is unspecified even there, and
      it belongs to that change. Writing an `op-log` delta here would give one
      capability two concurrent deltas from two changes.
- [x] 9.7 Answer the contract question box 9.4 routed to `spec-writer` — may an
      integration test construct a hostile keystore? **Yes, all three states, and
      no spec change is needed to warrant it.** `keystore` specifies each as a
      scenario whose WHEN clause is that construction (`spec.md:128`, `:153`, and
      `posting-capability/spec.md:78`), and `tasks.md` 7.3's `0o777` probe already
      demonstrated it in this file. Appended to box 9.4 in the findings file
      without editing the reviewer's or `tester`'s text.
      **One real gap found while answering:** `posting-capability`'s six-reason
      requirement has a scenario for five reasons; "the keystore's directory is
      writable by others" has none (`grep -n "directory"` over that file returns
      only line 53, the requirement text). The behaviour is contracted in
      `keystore`; what is unpinned is that the *probe* reports it as its own
      reason. One scenario on an existing promoted requirement closes it — recorded
      in PLAN.md §9.1 beside the other two, not added here.

## 10. Gates, re-measured after section 9

The numbers in §8 were measured at `f007bcd`, before `5323b57` merged
`origin/main`. They are left as the dated record they are; these are current.

- [x] 10.1 `cargo test -p dialectica -p dialectica-core` — **593 passed**, 0
      failed (567 in-crate + 26 in `tests/end_to_end.rs`). The baseline in this
      worktree before this pass was **592**; the one added test is 9.1's. The `-p`
      flags are load-bearing: without them cargo tests almost nothing and still
      reports `ok`.
- [x] 10.2 `rustfmt --check --edition 2021 --config skip_children=true` clean on
      `tests/end_to_end.rs`; nothing pre-existing reformatted. `wire.rs` was
      mutated twice and restored, and `git diff` over `dialectica-core/src/` is
      **empty**, so neither mutation shipped.
- [x] 10.3 `cargo clippy --all-targets -- -D warnings` clean on both crates. The
      six remaining warnings are the staged upstream SDK's and pre-date this
      change.
- [x] 10.4 No `ignore`-fenced doc block added; the run reports **0 doc-tests**, so
      CI's `ran == declared` gate is undisturbed.
