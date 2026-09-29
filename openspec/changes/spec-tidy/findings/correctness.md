# Correctness review — spec-tidy

Scope: `git diff origin/main...HEAD` (`d57edaf..HEAD`), per the runner's
correction — this excludes #180's `position-and-index` content (`wire.rs`,
its `sqlite.rs` position tests, its archive folder) that came in through the
merge `cb46319d`. The merge resolution itself is in scope and was reviewed.

## What was checked

- The merge resolution in `identity-onboarding` (`git show --remerge-diff
  cb46319d`): the raw conflict markers show both sides' appended requirements
  kept in full, spec-tidy's two master-key requirements first and #180's
  malformed-`index` requirement after. Confirmed the live
  `openspec/specs/identity-onboarding/spec.md` has all three headings once
  each, with no duplication or loss.
- Each of the five archives (`sqlite-projection`, `first-run-identity`,
  `ui-thread-view`, `ui-remaining-screens`, `moderation-screen`), and every
  delta correction that precedes one, diffed against the live spec produced
  and against the shipped code:
  - `sqlite-projection`'s rebase (`1f5df8f3`) genuinely would have dropped the
    blank-title encoding refusal and promoted a decay point from transport
    metadata if archived unrebased — confirmed the refusal paragraph and its
    two scenarios exist in the live spec pre-rebase and the rebase's
    `MODIFIED` block carries them forward, and confirmed the blank-title
    refusal test (`log/sqlite.rs:2255` area) and the decay-point-as-counter
    behaviour still exist in shipped code.
  - `first-run-identity`'s correction (`b1d89662`): both falsified sentences
    checked against current fact — "Whether this peer holds a master key is
    reportable without creating one" does exist as a live requirement, and
    `mint_master_key` in `wire.rs` implements exactly the corrected text
    (non-destructive check-then-mint, protection reported from the stored
    file rather than the caller's unlock). The archived delta
    (`openspec/changes/archive/2026-09-28-first-run-identity/`) is
    byte-for-byte identical to the promoted live requirements.
  - `ui-thread-view`'s correction (`584359bf`) and the `view-navigation` fold
    (`06bfd483`, `ff55cf77`): traced every scenario of both `REMOVED`
    requirements (`thread-view`'s route requirement, `moderation-view`'s route
    requirement) against the `view-navigation` `MODIFIED`/`ADDED` blocks that
    absorb them. Nothing is lost — every paragraph and scenario is either
    folded in with a citation, or was already present in `view-navigation`
    (confirmed by reading the pre-fold live spec directly, e.g. "Acting on a
    feed row opens its thread" and "The feed is reached again from the
    thread" already existed before this piece touched it). The one dropped
    scenario, "The feed is reachable again from the thread", is redundant
    with the pre-existing "The feed is reached again from the thread" — the
    `REMOVED` block's Migration note says so and it checks out.
  - `moderation-screen`'s four corrections (`8e508d8a`): confirmed the
    emptiness-ban paragraph ("A phrase asserting that nothing has been
    received for a Stoa MUST NOT be rendered...") existed in the live
    `stoa-navigation-view` spec at the branch's start, under the same
    requirement heading the delta's uncorrected `MODIFIED` block would have
    replaced — so the drop this correction fixes was real, not hypothetical.
    Confirmed `rowSeparator` exists in `DStoaListScreen.qml` for the sibling
    `ui-remaining-screens` addition, and that the moderation-screen archive
    added no scenario the corrected delta didn't already state.
  - `generated-names` (`92b31622`): confirmed issue #81 is closed
    (`gh issue view 81`), and that `rust-lib/src/lib.rs` genuinely has a
    `display_name` trait method (line 423) whose impl (line 1098) calls
    `core::wire::display_name` — the entry point the correction says exists.
  - `feed-view` (`1bcf2bf3`) and `moderation-resolution`/`stoa-genesis`
    (`49b98ca1`): confirmed every new citation resolves to a live requirement
    heading that exists verbatim (`post-revision`'s "A post is never edited
    in place", `stoa-navigation-view`'s "Joining shows what is being joined,
    and joins nothing until the user acts", the archived `op-clock` design's
    section).
- The four comment-only edits (`0d345d1b`): `git grep -n "unarchived" --
  dialectica-ui` finds nothing, and `node
  dialectica-ui/tests/validate-ui-specs.mjs` accepts all six e2e yaml specs
  including the two whose headers changed.
- `openspec validate spec-tidy --strict` → valid. `openspec validate --specs
  --strict` → 26 passed, 0 failed.
- The tester's new tests, run for real (after recreating this worktree's
  missing `logos-rust-sdk-src` symlink, which `git worktree add` drops):
  - `the_stored_author_is_the_signer_regardless_of_moderator_status` and
    `a_read_against_storage_broken_after_open_is_a_failure_not_an_empty_result`
    in `sqlite.rs`: both pass. Read `ordered_read`'s
    `self.conn.prepare(&sql).map_err(storage)?` and confirmed dropping the
    `ops` table makes `prepare` fail exactly the way the test exercises. The
    write path for the `author` column is `entry.op.op.author.to_bytes()`
    (line 753) — reading it confirms the test's assertion is checking the
    right column against the right value. Could not independently re-run the
    claimed mutation (swap `author` for `stoa` bytes on write): the harness's
    auto-mode classifier denied the `Edit` on `sqlite.rs` ("Modify Shared
    Resources"), the same denial other reviewers on this repo have hit on
    shared files. Recorded as unverified; the reasoning from reading the code
    is sound and the commit's self-reported proof is consistent with it.
  - `test_no_item_is_added_by_a_publish` in `tst_thread_reply.qml`: ran via
    `sh dialectica-ui/tests/run-qml-tests.sh
    dialectica-ui/tests/tst_thread_reply.qml`, passes (14/14). Read
    `DThreadScreen.qml`'s `reload()` (plain `screen.items = reply.value.items`
    replace, no append) and `onPublished: screen.reload()` — confirms the
    mechanism the test's comment describes is the actual mechanism, so a
    reintroduced optimistic-insert would be caught.
  - Ran the full suites the tester's commits claim: `cargo test -p
    dialectica-core` → 1187 + 30 + 3 passed, 0 failed, matching exactly. Full
    QML suite (27 spec files) → 0 failed across every file, matching exactly.

## Findings

- [x] **none** — every archive and correction checked out against the live
      spec, the shipped code, or both; the merge resolution loses nothing;
      the new tests pass for the reasons their comments give; both full test
      suites are green at the exact counts the piece's commits claim.
