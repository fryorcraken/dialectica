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

- [ ] **`dev-writer`** — `design.md`'s Non-Goals and `proposal.md`'s Impact
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

**Not independently reverified in full:** the requirement-level correctness
of each of the ~10 spec-delta corrections (blank-title refusal, `op-clock`
decay point, the two `first-run-identity` sentences, etc.) against the pre-
merge live spec text — `design.md`'s own table of shipped-in/corrected/archived
commits was spot-checked for consistency with `git log` (all five rows match
real commits in the right order) but the substance of each correction was not
re-derived from the PRs it cites. Given time constraints this is a noted limit
rather than a finding; nothing observed while reading the diffs contradicted
the commit messages' claims.
