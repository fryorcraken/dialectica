# spec-test review — `core-e2e`

Reviewed at `f007bcd`, in worktree `.claude/worktrees/rev-e2e-spectest` on
`review/core-e2e/spec-test`. Baseline confirmed: **531 passed** (506 in-crate +
25 in `tests/end_to_end.rs`), matching the dispatch and `tasks.md` §8.1.

This piece has no spec delta (`skip_specs: true`), so the contract reviewed
against is `openspec/specs/` as promoted on `origin/main` — and the piece changes
neither `openspec/specs/` nor `docs/PLAN.md` (`git diff origin/main --` over both
is empty). Sections 4 (requirements moved between capabilities), 5 (spec
soundness) and 6 (PLAN.md shedding) of the reviewer brief are therefore moot for
this piece: there is no moved requirement, no new scenario and no PLAN.md edit to
assess. What remains is coverage, falsifiability, and whether the header's claims
about its own scope are true.

## Mutations run

Five, all applied to implementation source, run, and reverted;
`git status --porcelain` over the worktree is empty, so every restore landed.
Three of the five are re-runs of rows in the file's own mutation table, which the
dispatch asked to be verified.

| Mutation | Table's claim | Observed here |
|---|---|---|
| `Moderators::contains` returns `true` unconditionally | 2 tests, hide test on the RESOLUTION | **2 of 25** — `the_moderator_set_of_a_genesis_record…` at "nobody else is", and `a_hide_by_a_non_moderator…` at `end_to_end.rs:1172` with `Ok(Hidden(Entry{..action: Hide..}))` vs `Ok(Unmoderated)`. **Table accurate; note 5's remedy verified.** |
| `SqliteOpLog::open` ignores its path and opens `:memory:` | 21 of 24 | **21 of 25.** Survivors are exactly the four the table's note 4 predicts: `an_over_cap_genesis_title…`, `the_moderator_set_of_a_genesis_record…`, `two_temp_dirs_with_the_same_tag…` (all pure-value) and `an_empty_store_answers_every_read…` (correctly unaffected). `a_page_past_the_end…` now dies at its added fixture guard (`end_to_end.rs:1537`, 0 vs 3), confirming note 4's fix. **Table accurate.** |
| `FeedRow.is_hidden` hardcoded `false` (not in the table) | — | 1 of 25, `a_hide_by_the_moderator…` at `end_to_end.rs:1104` "the hidden one is marked hidden" — the §9.1 obligation, on its own assertion. Clean. |
| `wire.rs` feed reply emits `"page": 0, "hasMore": false` as constants | — | **SURVIVED all 25 e2e tests.** See the first finding. |

The `wire.rs` `page`/`hasMore` mutation is caught by exactly one test in the whole
repository — `wire::tests::the_index_refusal_says_what_it_actually_refuses`, at
`wire.rs:2489`, against a `MemoryOpLog`. Notably
`wire::tests::the_feed_reply_is_the_ecosystems_pagination_shape` also survived it.

## Findings

- [ ] **`tester`** — `end_to_end.rs:1898-1955`,
      `a_request_naming_a_stoa_on_disk_comes_back_as_the_feed_in_json` — the two
      pagination-envelope assertions cannot fail, because the fixture's expected
      values are the constants a broken handler would emit.
      The test asserts `v["page"] == 0` and `v["hasMore"] == false` over a fixture
      of **one** post read at **page 0**. Both are the default any handler
      producing a fixed envelope would write, so the test agrees with itself —
      this project's own defect family (two explanations, one answer) at the JSON
      boundary the section heading calls "the outermost boundary this crate has".
      **Scenario:** a handler that echoed nothing — `"page": 0, "hasMore": false`
      hardcoded — hands a view `page: 0` for a request that asked for page 7, and
      `hasMore: false` for a feed with 400 unread threads, so the view's infinite
      scroll stops after one page and its page counter never advances.
      **Measured:** replaced `"page": page.page, "hasMore": page.has_more` in
      `wire.rs:624-625` with the literals `0` and `false`. **All 25 e2e tests
      passed.** In-crate, only `wire::tests::the_index_refusal_says_what_it_actually_refuses`
      failed (`Number(0)` vs `100`) — and that test uses a `MemoryOpLog`, so the
      seam this file exists to own (JSON against a real file) has no coverage of
      the envelope at all. The file's own header argues this seam is why stopping
      at `feed::list_threads` was a gap; the same argument applies one field over.
      A second wire test over a two-page store on disk, asserting `page == 1` and
      `hasMore == true`, kills it.

- [ ] **`tester`** — `end_to_end.rs:865` and `end_to_end.rs:1962` — both cite
      "§11.1 obligation 5" and quote it verbatim, and **§11.1 does not exist**.
      `docs/PLAN.md:3196` says so in terms: *"§11.1 'Rendering obligations,
      collected' arrives with the `vouching-state` change… If §11.1 is absent when
      you read this, that change has not merged yet."* The quoted sentence — "an
      empty feed is indistinguishable from a Stoa nobody has posted in" — appears
      nowhere under `docs/`. The real source is `docs/UI-BRIEF.md:397`, "A storage
      failure must never render as an empty feed."
      This is the repo's recorded "persuasive citations get fabricated" family: a
      quotation in quote marks reads as evidence somebody checked. It also breaks
      `.claude/agents/README.md`'s rule that a requirement must never cite a PLAN
      section number, since the number is unstable and the reader does not have
      the file open.
      **Scenario:** a reader deciding whether to relax
      `a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed`
      greps `docs/` for the obligation, finds nothing, and concludes the test
      guards nothing — when the obligation is real and is in `UI-BRIEF.md`.
      **Measured:** `grep -rn "indistinguishable from a Stoa\|nobody has posted in"`
      over `docs/` returns nothing; `grep -n "11.1"` over `docs/PLAN.md` returns
      only references to a section that has not landed. Cite `UI-BRIEF.md`'s
      sentence, or the live `module-wire-contract` requirement "Failure is always
      the error shape, and never a partial success", which says the same thing and
      is promoted.

- [ ] **`tester`** — `end_to_end.rs:1131` — `(§6)` is a bare PLAN section
      citation with no such heading, and the live spec already carries the rule.
      `docs/PLAN.md` numbers sections `6.1`, `6.2`…; there is no `§6` to read.
      Meanwhile `openspec/specs/moderation-resolution/spec.md:40` promotes exactly
      this requirement — "A Stoa's moderator set is derived from its genesis
      record". A test asserting promoted behaviour should cite the promoted
      requirement, not a PLAN number the archive flow is designed to shed.
      **Scenario:** the `vouching-state` change adds moderators beyond the
      creator; whoever updates this assertion looks for §6 to find out what the
      rule was, and has to guess which of §6.1–§6.n it meant.

- [ ] **`tester`** — `end_to_end.rs:10-25`, the "Does NOT cover" list — it omits
      `posting-capability`, which is promoted, reachable, and cross-boundary.
      The list names the publish path, `list_stoas`, membership, transport and the
      view. It does not name the capability probe, whose public entry points
      (`wire::get_capabilities`, `wire::capability_for`) sit in the same module as
      the `list_threads_from_request` this piece added, and which
      `openspec/specs/posting-capability/spec.md` requires to distinguish six
      **file-level** states — "the keystore's permissions are too open", "the
      keystore's directory is writable by others", "the keystore is unreadable or
      malformed" (spec.md:53). Those are file properties, so they are exactly what
      an in-memory per-change suite cannot pin and what this target is for; the
      file already creates keystores on disk and already sets `0o700`, so the
      fixture cost is near zero.
      This is the case the dispatch says was found here once already —
      `wire::list_threads_from_request` was public on `main` while the file stopped
      one layer below it and said nothing about the gap. The remedy then was to
      reach it. The minimum remedy now is an honest line in the list.
      **Measured:** `grep -rn "get_capabilities\|capability_for" end_to_end.rs`
      returns nothing; `grep -n "^pub fn" wire.rs` shows both as public alongside
      `list_threads_from_request`. `grep -rn "list_stoas\|listStoas"` over
      `dialectica/` finds no implementation, so **that** half of the list is
      substantively honest (see the next box for its citation).

- [ ] **`tester`** — `end_to_end.rs:17-20` — the reproduction instruction does not
      reproduce what it claims, and is self-referential as scoped.
      The comment says: *"`grep -rn "list_stoas\|listStoas"` over `dialectica/`
      finds only a line in `docs/PLAN.md`'s JSON contract."* But `docs/PLAN.md` is
      **not under `dialectica/`**, so that grep cannot reach it. Run exactly as
      written, it returns two lines — lines 17 and 18 of this very comment, and
      nothing else. The PLAN.md line (`docs/PLAN.md:3490`) is found only by a grep
      over the repository root.
      The underlying claim is true and I verified it independently: there is no
      `list_stoas` implementation, declaration or test anywhere. So this is a
      citation defect rather than a coverage one — but it is the same family as the
      §11.1 box above and as the repo's `persuasive-citations-get-fabricated.md`
      note: an instruction that reads as verified, that nobody reran, and that
      returns a result confirming itself.
      **Scenario:** a later author adding `list_stoas` reruns the grep to check
      whether the comment is stale, sees two hits, and cannot tell the comment's
      own text from evidence about the tree.
      **Measured:** `grep -rn "list_stoas\|listStoas"
      /home/…/rev-e2e-spectest/dialectica/` → exactly the two comment lines.
      Widening the scope to the repo root, or naming `docs/PLAN.md:3490` directly,
      fixes it.

- [ ] **`spec-writer`** — `openspec/specs/` has **no promoted requirement for the
      feed at all**, and 19 of these 25 tests assert against `feed::list_threads`.
      `grep -rln "feed\|thread head\|list_threads"` over `openspec/specs/` returns
      nothing. There is no promoted requirement that a feed lists thread heads and
      not replies, that a reply is excluded, that pages tile with no gap or repeat,
      that a page past the end is empty rather than a panic, that a forged post is
      refused by the reader, that a hidden thread is excluded by default and
      flagged under `include_hidden`, or that a body is sanitised on the way out.
      Nor is any of it a delta in `openspec/changes/` — `relevance-votes` and
      `sqlite-projection` mention the feed only in `design.md`/`proposal.md`/
      `tasks.md`, never in a `specs/` delta.
      So the largest behaviour block this suite pins is unspecified. The tests
      carry one `NO SPEC:` marker (`end_to_end.rs:1714`, on the vote) and it is
      correct as far as it goes, but it marks the narrowest instance of a much
      wider gap: the marker says "no requirement says what a vote does to a feed"
      while the unstated premise is that no requirement says what a feed does at
      all.
      **Scenario:** someone changes `feed::list_threads` to include replies as
      their own rows. `openspec validate --strict` passes, no requirement is
      contradicted, and the only thing that objects is a test in a file whose
      header says its tests are about boundaries rather than capabilities — so the
      reviewer has no contract to weigh the change against, which is the failure a
      spec exists to prevent.

- [ ] **`spec-writer`** — `openspec/specs/op-log/spec.md` specifies no
      file-backed behaviour, so five e2e tests pin store-lifecycle behaviour no
      promoted requirement describes.
      `grep -rn "LAYOUT_VERSION\|layout version\|UnknownLayoutVersion\|LayoutDoesNotMatch\|sqlite"`
      over `openspec/specs/` returns **nothing**. `op-log`'s requirements are all
      about what a log holds and returns, never about a file, a layout version, or
      opening a path. Yet these tests pin:
      `a_missing_store_file_is_created_rather_than_refused` (a missing path is
      created, not refused, and created empty),
      `a_store_written_by_another_layout_version_is_refused_naming_both_numbers`
      (and that the refusal names both numbers),
      `a_store_stamping_our_layout_without_our_tables_is_refused_as_mislabelled`
      (and that this is a *different variant* from the previous one),
      `a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed`,
      and `a_stored_op_reads_back_byte_identical_across_a_restart`'s restart half
      (`op-log` requires byte-identity on read-back, but says nothing about
      surviving a reopen).
      These are unmarked spec gaps — behaviour a test pins that no scenario
      describes — and per the reviewer brief that is the same gap as a `NO SPEC:`
      marker with less attention on it, not more. The three-way distinction
      between `UnknownLayoutVersion`, `LayoutDoesNotMatchItsVersion` and `Storage`
      is a real contract a caller branches on, and it exists only in test code.
      **Scenario:** a `sqlite-projection` change collapses the mislabelled case
      into `Storage` on the reasonable grounds that both mean "this file is not
      usable". Nothing in `openspec/specs/` says otherwise; two e2e tests fail and
      the fixer has no requirement to decide whether they were right.

## Clean, in prose

**Falsifiability, generally.** I read all 25 tests against the one invariant — a
test must assert against something the implementation did not produce — and the
file is unusually disciplined about it. The keystore test re-derives the expected
author address from a keystore **reopened from disk** rather than reading the feed
row; the byte-identity test holds `to_bytes()` taken before the store existed;
the shared-prefix test constructs its collision from literals rather than hunting
for one; the caps are hardcoded with an explicit refusal to import the constants;
the forged-hide fixture *arranges* its ordering with Lamport values and then
asserts the ordering it depends on. The vote test's exhaustive `FeedRow`
destructure is the right instrument — it converts a field addition into `E0027`
rather than a silent pass, which is louder than an assertion and cannot be
skipped.

**The mutation table is accurate where I re-ran it.** Three rows verified exactly,
including both halves of note 5's claim (the guard now sits below the assertions,
and the same mutation now fails on the resolution rather than the guard) and note
4's (the added fixture guard is what raises the `:memory:` kill count, and it is
the assertion that fires). The table's framing — dated history, not a current
claim, with a named obligation on whoever renames a symbol it cites — is the right
shape for something no gate re-runs.

**The defect-family sweep found no further instance.** I asked of each fixture
whether the property under test could be removed and the data still give the
expected answer. Every candidate I found is already excluded deliberately and the
comment says which rival explanation it rules out: two threads in the hide test so
"hidden" differs from "empty"; `iter_stoa` asserted at 2 while the feed is asserted
at 1, so a log that dropped the forgery on append fails; the reply asserted present
and resolvable so "missing" cannot mean "rejected"; the second Lamport value higher
than the first so last-wins and richer-wins both show; five posts named rather than
counted. `an_empty_store_answers_every_read…` surviving the `:memory:` mutation is
correct and note 4's argument for that is sound — an in-memory store genuinely does
answer every read empty, which is the behaviour asserted.

**`arbitrary_bytes`-style "the call returned" tests.** There are none of the weak
form here. The closest is
`a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed`, which
accepts `Err(_)` from either `open` or the first read. That is the **best available**
signal rather than a weak one: the comment correctly notes that SQLite may surface
a bad header at either point, and the assertion refuses the one outcome that would
be wrong (an `Ok` page). Its JSON-layer sibling is strictly stronger, pinning
`error` present, `items` absent and the message non-empty. The over-cap test's
`match … { Err(CorruptEntry(_)) => {} }` arms are likewise specific variants, not
"it returned".

**Header claims I checked and confirmed.** `list_stoas` and membership genuinely do
not exist (`grep` over `dialectica/` finds only `docs/PLAN.md:3490`). §2.5, §3.3,
§4.4, §4.8, §5.2 and §9.1 all resolve to real PLAN.md headings, and §9.1's quoted
sentence is verbatim at `docs/PLAN.md:3349`. The "no count in a section heading"
rule is observed. `tasks.md`'s stage block is honest: `tester` and the two review
rows are unticked with a stated reason, and the note explaining why a test-only
piece still owes a `tester` stage is correct.

**The documented defect.** `an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever`
asserts the current asymmetry rather than the correct behaviour, and says so in
both the header and the panic message, with the search terms a future fixer needs.
Pinning a known defect as current behaviour with a loud flip on fix is the right
call for a test-only piece. Its "this test is currently the only record that the
defect exists" paragraph is the correct warning to leave, and filing it belongs to
whoever owns the fix rather than to this review.
