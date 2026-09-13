# Readability — `core-e2e`

Reviewed: `dialectica/rust-lib/dialectica-core/tests/end_to_end.rs` (1,483 lines,
20 tests) and the `.github/workflows/ci.yml` test-count gate. Dimension:
**readability only** — architecture is in `architecture.md`, correctness and
security are another instance's.

All measurements below were taken in a throwaway worktree
(`review/core-e2e/shape`) and reverted; the tree was clean when the findings were
committed.

## Defects

- [x] **`dev-writer`** — `.github/workflows/ci.yml:694-697` — the `examples/`
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

      **Fixed** in `5a1719b`, taking the second of your two options — `examples/`
      is now in scope, so the claim is true rather than deleted. The three-root
      list became one `rglob` over `dialectica/rust-lib` with
      `if "target" in p.parts: continue`; this is also the architecture entry on
      the same lines, and one change answers both.

      **Reproduced your measurement first, then re-ran it against the fix.** With
      your `examples/probe.rs` restored on the merged tree: the old three-root
      shape counts 530 and cargo runs 530 — green, exactly as you measured. The
      new rglob counts **531** while cargo still runs 530 and reports
      `a_test_inside_an_example` in no `Running` line, so `ran != declared` fires.
      (530 rather than your 495 because `origin/main` was merged in between; #54
      landed 31 tests. Your arithmetic was right for the tree you measured.)

      Your two rebuttals to the comment's objections were the deciding argument
      and both check out: `target/` is a one-line exclusion, and there is exactly
      one `target/` in the tree. The comment now quotes **no number at all**,
      because both move with every test added — the self-invalidating form
      CLAUDE.md asks for.

- [x] **`tester`** — `end_to_end.rs:1417-1420` — the vote test's comment claims a
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

      **Fixed** in `1342aa9` — wrote the destructure you suggested, rather than
      deleting the sentence, because the claim is worth having true.

      **Your mutation re-run against it:** `pub score: i64` on `feed::FeedRow` set
      to `7` at the construction site now fails with
      `error[E0027]: pattern does not mention field 'score'`. That is better than
      the failing assertion the comment promised — it fails to **compile**, so it
      cannot be skipped, filtered out, or left to a `--no-fail-fast` run nobody
      reads. The comment now says that is what it does, and credits the
      measurement that showed the previous form passing.

      Two things came out of writing it. The outer `author` key had to be renamed
      `poster`, because the destructured `FeedRow.author` field shadowed it — and
      the attribution assertion is precisely that the two differ, so a shadow
      there would have been the test agreeing with itself. And the row's `author`
      is now asserted equal to the poster's derived address **and** `assert_ne!`
      against the voter's, which is the attribution a vote-rendering bug would
      actually produce.

- [x] **`dev-writer`** — `end_to_end.rs:1216-1218` — a fabricated ordinal
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

      **Fixed** in `1342aa9`, taking your first option: the ordinal is gone and
      the comment now cites `op.rs`'s own argument, restating its substance (an
      absurd value is ~28,000x the cap, so it proves *a* cap exists and nothing
      about *where*) rather than pointing at a line number.

      **Confirmed your reading of the list**, and it is worse than a wrong
      ordinal — the cap case is not on the list at all. One point in your favour
      that strengthens the finding: the list has moved to `README.md:308` on the
      merged tree, so even a *correct* line-number citation would now be wrong.
      That is why the replacement cites by symbol name and quotes the argument
      instead.

- [x] **`dev-writer`** — `end_to_end.rs:12-17` — the header's argument for
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

      **Fixed** in `1342aa9`. The header now gives reach as the whole reason — an
      integration test cannot see a private `const` — and then says explicitly
      that this is *not* because nothing else pins the caps, naming both
      `op.rs::the_field_cap_is_pinned_to_a_known_answer` and `stoa.rs`'s
      equivalent, so a reader learns a drifted cap has two other tests to argue
      with before it reaches this one.

      Your "wrong premise, right conclusion" framing is exactly the distinction,
      and it is recorded in `design.md` as its own decision — the answer to
      "should a test import a constant?" is about **reach**, not strength, and the
      next person to ask deserves the right reason rather than the right verdict.

      Cited by symbol rather than line: your own two references to this test
      (`op.rs:1631` in the prose, `op.rs:1650` in the measurement) point at the
      test and its assertion respectively — both correct, and between them a good
      argument for keeping line numbers out of comments.

- [x] **`dev-writer`** — `end_to_end.rs:448` — the section header says "three
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

      **Fixed** in `1342aa9`, taking your suggested wording verbatim — the heading
      is now "Empty versus unreadable, at each boundary a read can fail at".

      Generalised rather than fixed in one place, because you are right that it is
      the same failure shape: the header now carries a sectioning rule whose third
      clause is that **a heading states the boundary and never a count**, so the
      next section added cannot reintroduce this. That rule also answers the
      architecture entry about sections having no home for the pieces in flight.

- [x] **`dev-writer`** — `end_to_end.rs:25-29` — "The three bugs that reached
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

      **Fixed** in `1342aa9`, taking your second option — the count is gone and the
      sentence now argues from the shape of the defect, which is what it needed to
      do all along. Your first option was not available: I re-ran your grep and got
      the same nothing, so there is no archive path to name. Rather than leave that
      implicit, the paragraph now **says** no count is given and why, and names the
      over-cap asymmetry as the one worked example the file can actually point to —
      which it documents itself.

      A count removed quietly would have read as a stylistic edit; saying "no count,
      because none is recorded anywhere countable" is the thing that stops someone
      re-adding one.

- [x] **`tester`** — `end_to_end.rs:456` and `end_to_end.rs:1148` — two test
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

      **Fixed** in `1342aa9`, both split, following `feed.rs`'s precedent as you
      argued. The suite is 24 tests rather than 20 as a result.

      **This was not stylistic, and your own reasoning is why.** You wrote that a
      failure names a test whose other half is still true — and splitting the
      paging pair produced exactly that case, in the opposite direction. Under this
      file's `:memory:` mutation the new
      `a_page_past_the_end_is_an_empty_page_rather_than_a_panic_or_a_wrapped_first_page`
      **survived**: "page 99 is empty" is also what a store holding nothing
      answers, so the test was passing on a fixture that had persisted no rows.
      That defect was invisible while the claim was bundled with the tiling claim
      that did fail — the bundle was hiding it, not merely obscuring which half
      went.

      A fixture guard asserting page 0 holds three rows fixes it, and the mutation
      now kills **21 of 24** rather than 20. Measured both ways.

      The empty-store split earns its keep more modestly: under the mutation making
      `open` refuse a missing path, the creation half dies at its own assertion
      (`a missing store is created, not refused`) while the other 17 deaths are at
      the `dir.store()` fixture helper. Which is the "failure names the right
      claim" property you were asking for.

      Recorded in `design.md`, including the case that correctly survives —
      `an_empty_store_answers_every_read…` is unaffected by the `:memory:`
      mutation because an in-memory store genuinely answers every read empty, and a
      test that survives a mutation which does not change what it claims is passing
      rather than weak. Telling those two apart is the reason to run the mutation
      instead of counting survivors.

- [x] **`dev-writer`** — `end_to_end.rs:47-96` — the eleven-row mutation table
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

      **Fixed** in `1342aa9`, taking your second option. `9bb2bc1` is confirmed as
      the commit — `git log --oneline -- <the test file>` returns it and nothing
      else — and the table is now grouped by when each group was measured, with a
      second group of seven rows for the mutations run while addressing these
      findings.

      **Rejected the half of your suggestion that says re-run before adding a
      test**, and the argument is in the file: a later author owes this table
      **almost nothing**. Re-running eleven mutations to add one test is a tax
      nobody would pay, so a file demanding it would get a stale table *and* a
      resented one. What the file asks instead is the same discipline for the new
      test — mutate, predict, watch, add a row — plus one hard obligation: if you
      rename or move something the table names, fix the row in that commit or
      delete it. A row pointing at a symbol that no longer exists is worse than no
      row, because it reads as though somebody checked.

      Your "measured once" framing is what the file now says in as many words, and
      the six symbol names you listed as rename-fragile are quoted in the warning.

      **Your note-3 re-measurement was load-bearing beyond this box.** Re-running
      that mutation at 24 tests turned up a test passing for the wrong reason —
      see the split-names entry above. A table that nothing keeps true is still
      the most useful thing in the file, which is an argument for dating it rather
      than deleting it.

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
