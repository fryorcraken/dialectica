# Design review: home-screen-key-states-followup

Scope, per the brief: the ten commits that landed after the six-reviewer round
that ran on the pre-rebase piece (`2480536^..aac415b`), how this piece
interacts with #153 and #154 as they now sit together on `main`, and the whole
merged feature (`0cbe1d4`) read against the archived `design.md`
(`openspec/changes/archive/2026-09-24-home-screen-key-states/`), issue #150
(read fresh via `gh issue view 150`), and the three promoted specs.

## What was checked

- Read every one of the ten post-review commits individually
  (`2480536`, `4b29627`, `9efae86`, `59dbd0c`, `6aa76d7`, `fca9199`, `18ec847`,
  `4296bc5`, `f9617cc`, `2602da0`, plus the closer's `aac415b`), comparing each
  commit's stated reasoning against the code and test diff it carries.
- Read `design.md` end to end (Decisions 1–15, Risks, Migration Plan including
  "Meeting `join-preview-getstoa` (#154)").
- Read issue #150 fresh via `gh issue view 150 --repo fryorcraken/dialectica`.
- Read `view-navigation`'s "Acquiring an identity is reached from the
  navigator" as it now stands on `main`, and the merge conflict resolution in
  `2602da0` for `DStoaListScreen.qml` and `tst_stoa_screens.qml`, against
  design.md's "Meeting `join-preview-getstoa`" section and Decision 15.
- Built and ran the full gate set from a fresh worktree (symlink restored):
  - `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
    — 1141 + 30 passed, 0 failed.
  - `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`
    — 136 passed, 0 failed.
  - `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_navigation.qml`
    — 24 passed, 0 failed.
  - `sh dialectica-ui/tests/run-qml-tests.sh` (full suite) — 25 spec files, 0
    failed (two pre-existing, unrelated `QWARN`s in `tst_vote_and_gate.qml`
    from `SanitisedText.qml`, not a binding failure the `check_bindings` gate
    flags).
  - `lgs basecamp build` — both `.lgx` and portable artefacts built cleanly.
  - `python3 dialectica-ui/tests/check_qml_names.py` — ok, 53 files / 25
    `qmldir` entries.
  - `sh dialectica-ui/tests/check_qml_members.sh` — ok, 26 files.

## Result

**No findings.** The ten post-review commits are, if anything, an unusually
disciplined example of the thing this review exists to check for: every
decision made after the six-reviewer round is written up in `design.md` before
the change was closed, not left in a commit message.

Specifically, checked against the review's four priorities:

**1. Do the decisions recorded after the first round match what the code
does?** Yes, at every point checked:

- Decision 8/9's rewritten reasoning ("nothing in this release writes the key
  while the screen is hidden except another Basecamp instance or a
  hand-edited keystore file") is applied consistently: `git grep` for the old
  "per-Stoa keep" reasoning across `DStoaListScreen.qml`, `tst_stoa_screens.qml`
  and `design.md` turns up exactly one remaining hit, and it is the
  correctly-worded past-tense reference ("the per-Stoa keep that used to [write
  it] is not mounted") in the code comment at `DStoaListScreen.qml:200`, not a
  stale copy of the old claim. This is the shape CLAUDE.md warns about — a
  rule applied at three sites and missed at a fourth — and here it was not
  missed.
- Decision 15's `view-navigation` reconciliation (`fca9199`) is reflected
  byte-for-byte in the live spec at `openspec/specs/view-navigation/spec.md`,
  and the two split tests it describes
  (`test_following_the_route_makes_no_call_of_its_own`,
  `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`)
  both exist in `tst_navigation.qml` and both pass.
- The `join-preview-getstoa` (#154) reconciliation in the Migration Plan
  ("a fixture that exercises a create refusal must answer `get_master_key`
  with a held key") is exactly what
  `test_every_blank_title_reaches_the_core_rather_than_being_refused_here`
  does (`tst_stoa_screens.qml:2302`) — it answers `get_master_key` with
  `spec.heldKeyReply(...)` before exercising the three blank-title variants.

**2. Did the #153/#154 reconciliations record every decision worth
recording?** Yes. Two decisions that could plausibly have stayed as
commit-message-only reasoning were both promoted into `design.md` before the
change closed:

- `6aa76d7`'s Decision 15 originally left the `view-navigation` /
  `stoa-navigation-view` contradiction as "reported to the spec-writer to
  settle" — a decision deferred, not yet made. `fca9199` settled it (splitting
  one scenario into two), and `4296bc5` closed the loop by rewriting Decision
  15's last paragraph to name the ruling and both tests, and by updating
  `Main.qml`'s comment to match. Nothing here was left as only a commit
  message.
- `2602da0`'s merge conflict resolution (folding #154's blank-title rule into
  the key-gated create requirement) is not itself a design decision — the
  decision was `f9617cc`'s, and it is recorded in both the proposal and
  `design.md`'s new "Meeting `join-preview-getstoa`" section, with mutation
  evidence for both affected tests.

**3. Does anything contradict issue #150 as read fresh?** No. The issue's
`hasMachineKey`-style one-flag ask is met by `machineKey` (Decision 6, which
explains in the issue's own terms why the *boolean* half of the precedent was
not kept — the third outcome cannot fold into either binary answer without
recreating the exact trap Decision 2 exists to avoid). The issue's "omitted
entirely, not disabled" requirement is met by the `Loader`-based absence
(Decision 7), independently mutation-tested. The bundle-component mapping
table (Decision 11) accounts for every component the issue names. Decision 15,
added after the issue was filed, is an interaction with #153 the issue
could not have anticipated, and it is argued rather than merely applied
(three explicit consequences given, each with its own reasoning).

**4. Was reasoning worth keeping moved into design.md, in both directions?**
Reasoning migrated correctly in the direction the review checks hardest:
`6aa76d7`'s open question became `fca9199`'s ruling became `4296bc5`'s
Decision 15 rewrite, all before the change archived. In the other direction —
a trap for a *built* subsystem that belongs in a trigger-specific doc rather
than only in the archived `design.md` — nothing in this piece's own build
surface (QML naming/member gates, `lgs basecamp build`) surfaced a new trap of
that kind; the existing entries in `CLAUDE.md`'s "Never name a QML type
something basecamp also registers" section already cover the naming
mechanism this piece's `Loader`s and `D`-prefix decisions rely on, and this
piece added no new `qmldir` registration for that section to describe
(Decision 11: "The new blocks are `Loader` `sourceComponent`s ... not new
files").

No decision found in this piece was made in a commit message only and left
there — the two candidates for that (Decision 15's spec-writer question, and
the #154 merge reconciliation) both landed in `design.md` before archive.

## Minor observation, not a finding

`design.md`'s Decision 15 credits `fca9199` as "the spec-writer settled it",
which is a plausible read of the commit's own message ("Point the identity
route's comment... at the spec ruling") but the commit itself carries no
distinguishing author metadata beyond the standard co-author trailer shared
across this piece's commits. This is not a defect — the spec text and the two
tests it produced are what matters, and both are verified correct — so no box
is raised for it.

---

**Branch:** `worktree-wf_fff9ace7-faa-6`
**Tree status:** clean before this commit; `git diff --stat 0cbe1d4` after
committing this file shows only
`openspec/changes/home-screen-key-states-followup/findings/design-review.md`.
No mutation was left in place — no mutation was made; every check in this
review was read-only (code reading, `gh issue view`, and running the existing
test/build/lint gates unmodified).
