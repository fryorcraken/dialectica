# Readability review — spec-tidy

Scope actually reviewed: `git diff d57edafe..HEAD`, not `git diff 5ec1295...HEAD`
as given. `5ec1295` (#174, merged 2026-09-27) is an ancestor of `d57edafe`
(#180, merged 2026-09-29), so the three-dot diff against it also carries the
whole `position-and-index` piece that landed in #180 before `piece/spec-tidy`
branched — `wire.rs`, the `sqlite.rs` position tests, and the
`archive/2026-09-29-position-and-index` folder. None of that is spec-tidy's;
`git diff d57edafe..HEAD` is exactly the 53 files the `git merge --ff-only`
onto `piece/spec-tidy` reported. Reviewed against that scope: the five
promoted specs and their corrections, this change's seven deltas and the three
Purpose edits, `proposal.md`/`design.md`/`tasks.md`, the four code/yaml
comments, and the tester's two follow-up commits (`a98c4345`, `5f95e7e9`) that
touch `sqlite.rs` and the QML test files.

Checked and clean, with the command that confirmed each:
- Every cross-reference from the deltas, the four code/yaml comments, and the
  two `NO SPEC` resolutions into a live requirement or scenario heading
  resolves, character-for-character where quoted (spot-checked with
  `git grep -c "^### Requirement: …$"` against each cited heading, and by
  reading the target section for prose citations).
- All SHAs in `proposal.md`'s "Unarchived changes" and `design.md`'s Decision-1
  table are ancestors of HEAD (`git merge-base --is-ancestor <sha> HEAD`, all
  15 checked).
- All PR/issue numbers cited (#20, #122, #124, #127, #128, #80, #81, #174, #180,
  #186) match their actual state and title (`gh pr view`/`gh issue view`).
- `openspec validate spec-tidy --strict` → valid; `openspec validate --specs
  --strict` → 26 passed, 0 failed; `node
  dialectica-ui/tests/validate-ui-specs.mjs` → 6/6 ok;
  `sh dialectica-ui/tests/run-qml-tests.sh` → 27 spec files, 0 `FAIL`;
  `git grep -n "unarchived" -- dialectica-ui` → empty. All match what
  `tasks.md` claims.
- No duplication survives the `view-navigation` move: the two `REMOVED` blocks
  in `thread-view` and `moderation-view` carry only a `Migration` note, no
  restated requirement text, and the corresponding content lives exactly once,
  in `view-navigation`'s delta.
- No stray "this change" was found in the MODIFIED/ADDED prose of any of this
  piece's own deltas (one instance in the `thread-view` delta's `REMOVED`
  Migration note refers to `spec-tidy` itself in a note that is archived, not
  merged into a live spec, so it is not an instance of the #186 problem).
- `cargo test` for the Rust side could not be run in this worktree — the
  gitignored `logos-rust-sdk-src` symlink `git worktree add` drops is absent —
  so the tester's commit-message test counts are unverified rather than
  confirmed false.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs:1592-1597`
      — a quoted citation in `the_stored_author_is_the_signer_regardless_of_moderator_status`'s
      comment doesn't match the text it's attributed to, and names the wrong
      column.
      **Scenario:** the comment says: "This is the column
      `an_op_with_no_counter_and_one_at_the_maximal_counter…`'s comment warns
      about for `score_epoch`: 'reserved' and 'nothing queries it yet' is a
      comment that excuses a column from every behavioural test…". The
      `score_epoch` test's actual comment (same file, lines 1468-1469) reads
      "the column is reserved: 'nothing writes this and no read consults it' is
      a comment that excuses a column…" — no occurrence of "nothing queries it
      yet" anywhere near it. That exact phrase does exist in the file, but on
      an unrelated reserved *index* for a future vote tally (line 557: "RESERVED,
      and nothing queries it yet. `(target, author)` is the shape a vote tally
      needs…"), not on `score_epoch` at all. The new comment presents a
      paraphrase in quotation marks as if verbatim, and attributes it to the
      wrong comment. A reader who follows the citation to check it (as this
      review did) finds neither the wording nor the location match — the same
      failure mode this repo's own memory notes call out ("persuasive citations
      get fabricated", "run the claim, don't read it").
      **Severity:** low — comment-only, no behaviour or test-correctness
      impact, and the argument the comment is making (this column was as
      untested as `score_epoch` was) is still true; only the quotation is
      wrong. Same issue is repeated near-verbatim in commit `a98c4345`'s message
      ("`score_epoch`'s own comment warns about ('reserved, and nothing queries
      it yet' excuses a column from every behavioural test…")), which doesn't
      survive the squash but is worth knowing the source of the error was
      already in the commit message before it was copied into code.

No other readability defect was found in this piece's own material.

**fixed** — reworded the comment in
`the_stored_author_is_the_signer_regardless_of_moderator_status` to quote
`score_epoch`'s actual comment ("the column is reserved: 'nothing writes this
and no read consults it' is a comment that excuses a column from every
behavioural test", lines 1468-1469) instead of the fabricated paraphrase
attributed to it. The misattributed phrase "nothing queries it yet" belongs to
the unrelated `ops_by_target`-adjacent reserved index comment at line 557 and
has been dropped from this citation. Comment-only change; no test or
implementation behaviour affected.
