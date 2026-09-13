# Readability — `core-e2e`

Reviewed: `dialectica/rust-lib/dialectica-core/tests/end_to_end.rs` (1,483 lines,
20 tests) and the `.github/workflows/ci.yml` test-count gate. Dimension:
**readability only** — architecture is in `architecture.md`, correctness and
security are another instance's.

All measurements below were taken in a throwaway worktree
(`review/core-e2e/shape`) and reverted; the tree was clean when the findings were
committed.

## Defects

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:694-697` — the `examples/`
      paragraph argues the opposite of what the gate does
      The comment states: *"If an example ever grows a `#[test]`, this gate fails
      — correctly, because that test would never be run."* It does not fail. The
      gate compares `ran` (from cargo's `test result:` lines) with `declared`
      (from `roots`), and `examples/` is in **neither**. Adding a `#[test]` to an
      example moves neither number, so the gate stays green over a test that is
      never run — which is precisely the false green the whole step exists to
      prevent.
      **Scenario:** created `dialectica-core/examples/probe.rs` containing
      `fn main() {}` and one `#[test] fn a_test_inside_an_example()`, then ran the
      CI invocation verbatim (`cargo test --manifest-path … -p dialectica -p
      dialectica-core`). **Measured:** cargo reported the same five `test result:`
      lines as before — 0 + 475 + 20 + 0 + 0 = **495 ran** — and `declared` is
      still 495 (475 in `dialectica-core/src` + 0 in `rust-lib/src` + 20 in
      `dialectica-core/tests`). `ran == declared`, gate passes, the example's test
      never appeared in any `Running` line. A reader would wrongly conclude the
      gate covers this case and skip adding a check that would.
      **Also note:** no `examples/` directory has ever existed in this repo
      (`git log --all -- "dialectica/rust-lib/*/examples" "dialectica/rust-lib/examples"`
      is empty), so the paragraph explains a decision about a directory that is
      not there. Either delete the paragraph, or make the claim true by adding
      `examples/` to `roots` (cargo would then run nothing from it and the
      mismatch would fire) — but do not leave the sentence as it stands.
      **Severity: medium** (a false claim about what a green gate proves, which
      `.claude/agents/README.md` names as the worst kind).

- [ ] **`tester`** — `end_to_end.rs:1417-1420` — the vote test's comment claims a
      shape-pinning assertion the test does not make
      *"Pinned by the row's own shape: if a score is ever added, this comparison
      against a fully-specified row fails and someone has to decide what the view
      does with it."* There is no comparison against a fully-specified row. The
      three assertions are `assert!(!is_revised)`, `assert!(!is_hidden)` and
      `assert_eq!(attachments, vec![])` — each reads one field; `FeedRow` is not
      `#[non_exhaustive]`, and nothing destructures it.
      **Scenario:** added `pub score: i64` to `feed::FeedRow` (`feed.rs:141`) and
      set it to a non-default `score: 7` at the one construction site
      (`feed.rs:259`). **Measured:** `a_vote_is_stored_and_is_rendered_by_nothing`
      **passed unchanged** — 1 passed, 0 failed. Reverted.
      A reader would wrongly conclude that adding a score to the wire row is a
      loud change. It is silent. A `let FeedRow { thread, current_version,
      author, body, attachments, is_revised, is_hidden } = &page.items[0];`
      destructure would make the claim true; either write that, or delete the
      sentence. **Severity: medium** — this comment is the file's stated
      justification for the test existing at all.

- [ ] **`dev-writer`** — `end_to_end.rs:1216-1218` — a fabricated ordinal
      citation of the project's defect list
      *"A test using only `u32::MAX` would pass under that tightening — which is
      the second entry on this project's list of tests that could not fail for
      the reason they named."* The only such list is
      `.claude/agents/README.md:303`, and its three entries are (1) comparing
      `"ab"` with `"abc"`, (2) **mutating a byte and asserting a hash moved**,
      (3) `assert_eq!(bytes[0], VERSION_1)`. A `u32::MAX` cap test is not the
      second entry and is not on the list at all.
      **Measured:** `grep -rn "could not fail for the reason"` over the whole
      worktree returns exactly one list, at `README.md:303`; `grep -rn "u32::MAX"`
      over `.claude/agents/README.md` and `docs/PLAN.md` returns nothing.
      A reader checking the citation finds a different defect and loses confidence
      in every other `§`-style reference in the file. Cite the real concern —
      `op.rs`'s own `the_field_cap_is_pinned_to_a_known_answer` argues it at
      `op.rs:1632-1640` — or drop the ordinal. **Severity: medium**; this is the
      persuasive-fabricated-citation family the project has been bitten by twice.

- [ ] **`dev-writer`** — `end_to_end.rs:12-17` — the header's argument for
      hardcoding `FIELD_CAP` rests on a premise `op.rs` disproves
      *"a crate whose public surface was missing a method entirely would pass
      every one of them"* is fine; the next claim is not: *"a cap that silently
      drifted would still refuse an absurd input and still pass every test that
      only probes absurd inputs."* `op.rs:1631`
      `the_field_cap_is_pinned_to_a_known_answer` already asserts
      `MAX_FIELD_LEN == 150 * 1024`, and `op.rs:1673`/`1686` already pin the
      at-cap / one-over pair. `stoa.rs:893` does the same for
      `MAX_TITLE_BYTES == 1024`.
      **Measured:** `grep -n "assert_eq!(MAX_FIELD_LEN" ` → `op.rs:1650`;
      `grep -n "assert_eq!(MAX_TITLE_BYTES"` → `stoa.rs:893`.
      A reader would wrongly conclude this file is the only thing pinning either
      cap, and would not know that a drifted cap already has two other tests to
      argue with. The reason to hardcode here is still good (an integration test
      cannot see a private const), so say *that* rather than a claim about the
      unit suite's coverage. **Severity: low** — wrong premise, right conclusion.

- [ ] **`dev-writer`** — `end_to_end.rs:448` — the section header says "three
      boundaries" over a section holding four, and the fourth test says so
      `// ─── Empty versus unreadable, at three boundaries ───` is followed by
      four `#[test]`s (empty store, foreign layout version, mislabelled layout,
      not-a-database), and the last one's own comment opens *"The fourth way a
      read can fail"* (line 575). The count was already wrong in the commit that
      introduced it.
      **Measured:** four `#[test]` attributes between line 448 and the next
      section divider at line 638.
      A reader counting sections to decide where a new boundary test belongs gets
      a contradiction from the file itself. Drop the number — "Empty versus
      unreadable, at each boundary a read can fail at" carries the same meaning
      and cannot go stale. **Severity: low** (stylistic in effect, but it is the
      same failure shape as the numeric claims above).

- [ ] **`dev-writer`** — `end_to_end.rs:25-29` — "The three bugs that reached
      this session's review" is an uncited count
      *"The three bugs that reached this session's review were each
      **cross-layer**: encode versus decode, wire versus store, memory versus
      disk."* I could not find any record of the second or third.
      **Measured:** `grep -rn "cross-layer\|encode versus decode\|memory versus
      disk"` over `openspec/` and `docs/` returns nothing. Three archived
      `tasks.md` files carry severity-tagged findings
      (`moderation-resolver`, `keystore`, `revision-resolver`) but none is
      described in those terms.
      A reader takes the count as measured history and reasons from it — this is
      the file's stated third reason for existing. Either name the three findings
      by the archive path that records them, or write the sentence without a
      count ("the cross-layer defects this crate has produced are the seam
      between encode and decode…"). **Severity: low**, but it is precisely the
      shape `.claude/agents/README.md:294-300` was written about.

- [ ] **`tester`** — `end_to_end.rs:456` and `end_to_end.rs:1148` — two test
      names state two claims joined by `and`, so neither can fail on its own
      name
      `an_empty_store_answers_every_read_and_a_missing_file_is_a_created_one`
      asserts a creation behaviour *and* five read behaviours;
      `a_page_past_the_end_is_an_empty_page_and_the_pages_before_it_tile_the_feed`
      asserts a past-the-end behaviour *and* a tiling property. When one half
      breaks, the failure names a test whose other half is still true, and the
      reader has to read the body to learn which claim went.
      The file's own convention makes the point: the nearby single-claim names
      (`a_reply_is_not_a_thread_head_in_the_feed`,
      `a_hide_by_a_non_moderator_leaves_the_thread_visible`) each state one
      falsifiable thing, and `feed.rs` already splits the equivalent pair into
      `a_page_past_the_end_is_empty_rather_than_a_panic` (line 717) and
      `pages_partition_the_feed_with_no_gap_and_no_repeat` (line 667).
      **Severity: low / stylistic** — but it is the precedent question, and the
      matching unit tests are already split.

- [ ] **`dev-writer`** — `end_to_end.rs:47-96` — the eleven-row mutation table
      has nothing that can keep it true
      The table's content is accurate today — I re-ran the load-bearing row and
      it reproduces exactly (see below) — and that is what makes it dangerous: it
      reads as current, and the next piece to add a test here has no reason to
      re-run eleven mutations, no gate that notices, and no instruction in the
      file saying it must. Six of the rows name implementation symbols
      (`feed::list_threads`, `moderation::resolve`, `Moderators::contains`,
      `SqliteOpLog::append`, `SqliteOpLog::open`, `from_connection`) that a rename
      silently invalidates.
      **Measured:** I applied the note-3 mutation (`SqliteOpLog::open`
      ignoring `path` and calling `Connection::open_in_memory()`, `sqlite.rs:236`)
      and got **18 of 20 failed, 2 passed**, the two survivors being exactly
      `the_moderator_set_of_a_genesis_record_is_exactly_its_creator` and
      `an_over_cap_genesis_title_is_refused_before_it_can_name_a_stoa` — the
      table's claim verbatim. Reverted.
      What is missing is a sentence saying what a later author owes the table:
      either "re-run these before adding a test, or move the row to a dated
      'measured once, at commit <sha>' list". As it stands the honest reading is
      "measured at 9bb2bc1", and nothing in the file says so. **Severity: low**
      — a documentation-durability defect, not a wrong claim.

## Clean

The rest of the file reads well and I found nothing to raise about it.

- **Idiom and naming match the crate.** The long sentence-style names, the
  `// rival explanation excluded:` opening, the `§`-anchored justifications and
  the `FIXTURE GUARD` capitalisation are all the existing house style
  (`feed.rs:363-835`, `log/fixtures.rs`, `log/mod.rs:508-528`). Comment density
  is high but in line with the crate, and the comments overwhelmingly say *why
  this fixture and not the obvious one* rather than restating the code — which is
  the bar CLAUDE.md sets.
- **Nine of the other numeric claims check out.** `FIELD_CAP = 153_600` matches
  `op.rs:132` (`150 * 1024`); `TITLE_CAP = 1024` matches `stoa.rs:103`; the
  31-byte shared prefix is asserted rather than assumed (line 1104); the CI
  comment's "cargo ran 495 where these roots declared 475, the difference being
  that file's 20 tests exactly" is exactly right — I counted 475 `#[test]` in
  `dialectica-core/src`, 0 in `rust-lib/src`, 20 in `tests/end_to_end.rs`, and
  cargo reports 0 + 475 + 20 + 0 + 0 = 495.
- **The `TempDir` 0o700 comment earns its place.** `Keystore::create` really does
  refuse a group- or other-writable parent (`keystore.rs:1070-1071`,
  `mode & 0o022 != 0`), so the comment says something a reader could not get from
  the code in front of them.
- **The `EXPECTED-DEFECT:` panic messages (lines 1291-1298) are the right shape.**
  They name what should replace the assertion when the encoder is fixed, so the
  test fails loudly and legibly rather than mysteriously.
- **`rustfmt --check --config skip_children=true` is clean** on the file, and the
  full suite is 20/20 green.
