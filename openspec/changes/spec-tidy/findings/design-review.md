## Design review

Scope reviewed: `git diff d57edaf..HEAD` (the piece's own commits), `git show
--remerge-diff cb46319` (the merge of `origin/main` #180), `proposal.md`,
`design.md`, `tasks.md`, `gh pr view 189`, `gh issue view 186`.

The owner's rulings checked against the recorded Decisions: "tidy up the specs
and as needed", `relevance-votes` "(2) ignore", the `view-navigation` route
move ("3. ok yes sounds good"), the "this change" sweep tracked as its own
issue (#186), `skip_specs` (rejected in favour of real deltas), and signing
("resign all commits when possible", then "rewrite regardless"). All six check
out: `relevance-votes` is untouched, every commit from `1f5df8f3` onward is `G`
(gpg-verified) except the pre-existing base `d57edafe`, no `skip_specs` marker
was added, and the `view-navigation` fold matches what the owner approved.
`gh issue view 186` confirms the "this change" sweep is tracked there and
out of scope here, matching Decision 5. The PR body (`gh pr view 189`) has no
closing keyword — it says "Not in this PR" and links #186 without "Closes"/
"Fixes"/"Resolves".

The merge resolution in `cb46319` (both master-key requirements first, then
#180's malformed-index requirement) is mechanical, changes no text on either
side, and matches its own commit message. No finding there.

One gap, below.

- [x] **`dev-writer`** — `design.md`'s Non-Goals and `proposal.md`'s Impact
      section both understate this piece's own footprint. Non-Goals says "Any
      code change beyond comments. The only non-spec edits are four comments
      that this piece's own moves made stale," and `proposal.md`'s Impact
      section says "four comments change... No behaviour, test assertion or
      wire contract changes." Two later commits on this branch, `a98c4345`
      ("Tests: pin the reserved author column, resolve two NO SPEC markers")
      and `5f95e7e9` ("Close two more gaps..."), add ~150 lines across
      `dialectica-ui/tests/tst_thread_reply.qml` and
      `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs` — three new
      `#[test]` functions and one new QML test. These are legitimate: they are
      the `tester` stage's work (ticked in `tasks.md`), pin requirements the
      archives promoted, and add no production-code behaviour (verified by
      reading the `sqlite.rs` diff directly — every added line is inside
      `mod tests`). But "no test assertion changes" is now false on its face,
      and "the only non-spec edits are four comments" was true when `design.md`
      was written (`fb94e716`, before the test commits) and is not true of the
      branch as it stands. A reader of `design.md` alone would not learn that
      new coverage was added, why, or that it's test-only. Recommend a short
      addition to Non-Goals/Impact (or a new Decision) naming the two test
      commits and stating the "no production code changed" fact explicitly,
      the way the commit messages already argue it — the review currently
      has to reconstruct that from `git log`, not from `design.md`.
      **Fixed** in the commit that ticks this box. `design.md`'s Non-Goals now
      reads "Any production-code change" and inventories what the piece does
      change outside the specs: the four comments; the tests from `a98c434`
      and `5f95e7e`, each named with its file and why the stage added them;
      the two reworded `NO SPEC:` comments; and the tests still pending from
      the five open `tester` findings, pointed at by file rather than listed.
      Its Context paragraph, `proposal.md`'s Why ("touches no code") and
      Impact, and `tasks.md`'s Implementation preamble ("spec text only") are
      corrected to match. One count correction: the diff adds **two**
      `#[test]` functions to `sqlite.rs` plus a non-test helper
      (`stored_author`), not three — `git diff d57edaf...HEAD --
      dialectica/rust-lib/dialectica-core/src/log/sqlite.rs` shows it. The
      "every added line is inside `mod tests`" claim holds. No test can show
      a prose inventory is right; `git diff d57edaf...HEAD --stat --
      dialectica dialectica-ui` is the check against it.

**Not independently reverified in full:** the requirement-level correctness
of each of the ~10 spec-delta corrections (blank-title refusal, `op-clock`
decay point, the two `first-run-identity` sentences, etc.) against the pre-
merge live spec text — `design.md`'s own table of shipped-in/corrected/archived
commits was spot-checked for consistency with `git log` (all five rows match
real commits in the right order) but the substance of each correction was not
re-derived from the PRs it cites. Given time constraints this is a noted limit
rather than a finding; nothing observed while reading the diffs contradicted
the commit messages' claims.

## Re-review round 1 `b4378cf5..218acde3`

`153c4f5c` answers the finding above: `design.md`'s Non-Goals and
`proposal.md`'s Impact were rewritten to name the two `sqlite.rs` tests, the
one `tst_thread_reply.qml` test and the two reworded `NO SPEC:` comments, and
to point at "the five open `tester` findings" as the tests still pending. That
fix was accurate for the state at `153c4f5c`.

`218acde3` then closed all five of those findings — four fixed (three new
`tst_thread_reply.qml` tests plus one comment reword in `readability.md`'s
target, plus the fourth spec-test gap) and one deferred with no test — but
touched none of `design.md`, `proposal.md` or `tasks.md`. The inventory is
stale again, in the same place it was stale before:

- [x] **`dev-writer`** — `design.md`'s Non-Goals (and `proposal.md`'s Impact,
      which mirrors it) undercounts the tests this piece added and
      misdescribes their status. Both say "one QML test" in
      `tst_thread_reply.qml`; `218acde3` added four more there
      (`test_no_reply_affordance_is_reachable_with_no_root_identifier`,
      `test_a_root_items_own_missing_id_does_not_reach_the_reply_parent`,
      `test_the_screen_threads_the_items_sanitiser_report_through_to_the_render`,
      `test_the_marker_and_the_inert_row_state_nothing_about_earlier_content`
      — confirmed by `git diff 153c4f5c 218acde3 --
      dialectica-ui/tests/tst_thread_reply.qml`, all four inside `mod
      tests`-equivalent QML test functions, no production QML touched). Both
      documents also still say "Tests still pending from the five open
      `tester` findings... until those boxes are closed, this inventory is
      incomplete" — but `218acde3`'s own commit message reports all five
      closed (four fixed, one deferred). The inventory needs a second pass
      naming the four new tests and stating plainly that nothing is pending
      any more.

      Separately, the one deferred finding — "No ordering is offered as
      vote-based" gets no test, because no ordering control of any kind
      exists on the thread screen today, so a test for it cannot fail for the
      reason it would name — is exactly the kind of reasoning `design.md`'s
      Non-Goals should carry forward rather than leave only in
      `findings/spec-test.md`: it is a decision not to add a test, argued
      from a real alternative (write a vacuous test now vs. defer), and it
      names its own trigger (whichever future change adds an ordering
      control). Once findings files are read only by mutation-hunting
      reviewers rather than by every future reader of this change, that
      "why no test" reasoning is otherwise stranded. Recommend folding one
      sentence into Non-Goals: the deferral, its reason, and its trigger,
      the way the four-comments and tests-added bullets already do for the
      rest of this inventory.

      **Fixed** in the commit that ticks this box. Counts re-derived from
      `git diff origin/main...HEAD -- dialectica-ui/tests/tst_thread_reply.qml`
      and `git log -S "function test_" origin/main..HEAD` on that file: the
      piece adds **five** test functions there, one from `5f95e7e` and the
      four this finding names from `218acde`, plus three non-test helpers
      (`findByTypeName`, `bodyTextOf`, `collectTexts`). `design.md`'s
      Non-Goals now names all five with their commits, records `218acde`'s
      comment-only reword in `sqlite.rs`, states that no test is pending, and
      carries the "No ordering is offered as vote-based" deferral: its reason
      (no ordering control of any kind on the thread screen, so the test could
      not fail for the reason it names; confirmed by `git grep` on
      `DThreadScreen.qml`, whose only per-row control is `VoteControl` with
      `showScore: false, interactive: false`), the rejected alternative, and
      its trigger. `proposal.md`'s Impact now says five QML tests and "No
      tests are pending". In `tasks.md`, the Implementation preamble now
      points at the deferral. Group 4.6 said "This piece touches no QML",
      which is false because the piece edits comments in two QML sources and
      adds QML tests, so it now says what a green suite shows. The Stages
      block is untouched. No test can show a prose inventory is right;
      `git diff origin/main...HEAD --stat -- dialectica dialectica-ui` is the
      check against it.

## Re-review round 2 `cdc87c5c..55f73e39`

- [x] **re-review round 2 `cdc87c5c..55f73e39`: no findings** — read `git diff
      cdc87c5c..55f73e39` and independently re-derived every count and claim
      it makes against the tree. `tst_thread_reply.qml` has exactly five test
      functions (`git diff origin/main...HEAD -- dialectica-ui/tests/tst_thread_reply.qml`
      confirms the five names and the three helpers `findByTypeName`,
      `bodyTextOf`, `collectTexts`). `sqlite.rs` has exactly two `#[test]`
      functions plus the `stored_author` helper, both hunks inside `mod
      tests`. `218acde`'s reword of the stored-author test's comment is
      comment-only, and the new quoted text ("the column is reserved:
      'nothing writes this and no read consults it'...") now matches the real
      `score_epoch` comment at `sqlite.rs:1468-1470` word for word — the
      earlier fabricated citation this round's commit message says it fixed
      is in fact fixed. `FeedScreen.qml` and `Main.qml` diffs are
      comment-only, citing `view-navigation` requirements that exist. The
      deferred-decision paragraph's factual claim — `DThreadScreen.qml`'s
      only per-row control is `VoteControl { showScore: false; interactive:
      false }` — is exactly what the file shows. `proposal.md` and
      `tasks.md` mirror `design.md` and "No tests are pending" is true: all
      five `tester` findings from round 1 are closed. Clean.
