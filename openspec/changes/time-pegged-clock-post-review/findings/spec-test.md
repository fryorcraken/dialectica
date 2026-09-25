# spec-test review

Reviewed per role: spec and tests only (`#[cfg(test)]` modules), no implementation
read beyond the one mutated line in part 2. Read the owner's decision comment on
issue #162 (2026-09-25); first line: "**Decision (owner, 2026-09-25): peg the
Lamport counter to wall-clock time, as SDS does (LIP-109,
`logos-lips/docs/anoncomms/raw/sds.md`, lines 148-155 and 184-192).**"

## Diff 1 — this piece (`git diff origin/main...HEAD`, reasoning removal)

Checked every one of the six modified capabilities (`op-ordering`, `op-format`,
`op-transport`, `feed-view`, `post-revision`, `thread-read`) by diffing each
capability's current live spec (`openspec/specs/<cap>/spec.md` on `origin/main`)
against this change's delta, and cross-referencing every removed/kept passage
against the proposal's itemised inventory (O1–O18, F1–F4, T1–T5, V1, R1, H1–H2)
and its "Judgement calls" section.

Result: **clean**. Every requirement's normative SHALL/MUST text and every
`#### Scenario:` block (WHEN/THEN/AND) is carried over verbatim — only
in-between reasoning prose is removed, and it matches the proposal's inventory
passage for passage, including the exact "reworded to stand alone" sentences
(e.g. "The clamp MUST NOT cause any op to be refused.", "A refusal under this
requirement does not reach the op's author.", the word "still" dropped from
"The counter is a causality mechanism."). No requirement was removed wholesale
(`git grep` for "REMOVED Requirements" across this change's spec deltas: no
hits), consistent with the proposal's claim of no behaviour change. The two
scenarios added by #165's unreviewed tail ("One reading of the time signs the
counter and the wall-clock alike", "A counter taken from the clock leaves the
wall-clock at the current time") are carried forward into this piece's delta
unchanged, since they are behaviour, not reasoning. The three doc-comment-only
Rust changes in this diff (`arrival.rs`, `op.rs`, `transport.rs`) only repoint
citations from `op-ordering` prose to the archived `design.md`'s Decision 11 —
no code changed, consistent with "no behaviour change."

The three requirements in the live `op-ordering` spec that this delta does not
mention at all ("Ops are deduplicated by op id before they are ordered",
"Absent transport metadata is represented, never fabricated", "An op carrying
no counter sorts below every op that carries one") are untouched by #165 in the
first place, so their absence from this delta is correct OpenSpec delta
semantics (unlisted requirements survive archiving unchanged), not a removal.

No untestable scenario and no coverage gap found in this diff.

## Diff 2 — the four unreviewed #165 tail commits (`git diff 2eada33 c1f1a8f`)

- **"One reading of the time signs the counter and the wall-clock alike"** —
  pinned by the pre-existing test `one_reading_of_the_time_signs_both_clock_fields`
  (`authoring.rs`); only its comment changed (from a `NO SPEC:` marker to citing
  the now-existing scenario). Behaviour and test unchanged.
- **"A counter taken from the clock leaves the wall-clock at the current time"**
  — pinned by the new test
  `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
  (`authoring.rs`). **Mutated** `publish` (`authoring.rs:276`) so
  `asserted_ms: who.asserted_ms` became `asserted_ms: next_counter(clock,
  who.asserted_ms)` — writing the counter into the wall-clock field, exactly the
  failure mode the test's own comment names. Ran
  `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`:
  the test **failed** (`assertion left == right failed: the wall-clock stays at
  this peer's current time... left: 1789732304001 right: 1789729304000`).
  Mutation caught; restored immediately afterward (`git status --short` clean
  before commit). This was the one mutation run against this budget.
- **A held op arriving again beyond the window** — the scenario "A held op
  arriving again beyond the window is refused, and stays held" and its
  requirement prose ("An op this peer already holds is judged like any other
  arrival...") are new in `op-transport`'s delta in this tail, not a reword of
  existing scenario text (there is no prior scenario by a related name at
  `2eada33`). What *is* reworded is the pre-existing test's comment in
  `transport.rs` (`a_held_op_arriving_again_beyond_the_window_is_refused_rather_than_reported_as_held`),
  from a `NO SPEC:` marker to citing the new requirement — the test itself, and
  the code path it exercises, predate this tail and were already covered by the
  original #165 review round. It asserts both the `AheadOfTime` error and
  `log.len() == 1` ("the held op is still held"), so it can fail on either half.
  Not separately mutated, given the one-or-two-mutation budget and that its
  underlying code was already in scope for #165's first review pass.
- **The "worth checking" callback** (issue #162, owner's decision: "what the
  one-hour window allows against `revision::current_version`,
  `moderation::resolve` and the feed's `latestReply`") is answered for
  `moderation::resolve` and `revision::current_version` in this tail's doc
  comments (`moderation.rs`, `revision.rs`) and in `design.md`'s Decision 10,
  which explicitly says "No reader has code of its own for it," so no new test
  was added or is needed beyond the existing `op-ordering` scenarios that
  already cover `cmp_ops`. `latestReply` was answered earlier, in the original
  (already-reviewed) #165 design.md Decision, not in this tail — confirmed
  present at `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`
  line 259.
- Grepped the tail diff for `NO SPEC:` additions: none added; two markers
  *removed* (both cases above), each because the tail's spec addition now
  specifies what was previously a `NO SPEC:` choice. No unmarked new NO-SPEC-style
  gap found in the tail.

No untestable scenario and no coverage gap found in this diff either.

## Verification run before hand-back

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: **1180 passed, 30 passed (end_to_end), 0 failed.**
- `nix build ./dialectica#lgx` from the tree root: succeeded (no output, clean
  exit).

## Summary

No checkbox findings. Both diffs are clean against the spec-test review's
checks: every scenario and MUST/SHALL in diff 1 survives verbatim, matching the
proposal's own inventory; the tail's two new scenarios in diff 2 are each pinned
by a test that can fail, and the mutation run against the second confirms it.

## Re-review of 439c192..HEAD

Scope per the runner's brief: only what landed after the first review round
(`git diff 439c192 HEAD`), restricted to its spec and test parts. Read the
owner's decision comment on issue #162 again to confirm scope; first line
quoted above still applies unchanged — this diff makes no scope-affecting
change, it only carries out already-in-scope items from the first review round
(a spec-text clarification and a test-comment correction).

`git diff 439c192 HEAD --stat` touches eleven files. Of those, only two are
spec/test material:

- `openspec/changes/time-pegged-clock-post-review/specs/op-ordering/spec.md`
- `dialectica/rust-lib/dialectica-core/src/authoring.rs` (test-module comment
  only)

The rest (`arrival.rs`, `op.rs`, `revision.rs`, `transport.rs` doc comments;
`design.md`; `findings/architecture.md`, `findings/design-review.md`,
`findings/readability.md`; `tasks.md`) are doc-comment citation rewording, new
design reasoning, or other reviewers' findings/tracking files — out of this
role's scope, left unread beyond confirming none of them touch a test module
or a spec requirement.

**Spec change.** The only edit to `spec.md` is in the "A peer's Lamport clock
is a function of the ops it holds" requirement: the bolded single line

> **The clock is the highest counter held, with no exception for a counter far
> above the rest.** **The clock does not read the current time.**

is split into two plain paragraphs with the bold markup removed:

> The clock is the highest counter held, with no exception for a counter far
> above the rest.
>
> The clock does not read the current time.

Compared word-for-word: identical text, reflowed into two sentences and
de-emphasized. No requirement wording changed. Clean.

**Test change.** The corrected comment sits on
`a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
(`authoring.rs`). It replaces the first review round's overstated claim (that
a mutation writing the counter into the wall-clock field "would still pass
every other test in this file, and only fails here") with two narrower,
checkable claims: (1) this test fails directly, on its own `asserted_ms`
assertion; (2) `the_second_authoring_carries_the_higher_counter` also trips
under the same mutation, but on that test's own op-id fixture-drift guard, not
on the assertion its name promises.

**Measured, not just read**, since this is exactly the shape part 2 exists to
catch (a comment asserting what a mutation does or does not touch). Mutated
`publish` (`authoring.rs:275-276`) so
`asserted_ms: who.asserted_ms` became `asserted_ms: next_counter(clock,
who.asserted_ms)` — the wall-clock field now holds the counter, matching the
letter of "one reading signs both fields" while dropping which value each
field gets, exactly the mutation the comment names. Ran both named tests
(`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core
<name>`):

- `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
  **failed**, on the `asserted_ms` assertion at line 1883-1884
  (`assertion left == right failed: the wall-clock stays at this peer's
  current time, not at the value the clock term produced for the counter`,
  `left: 1789732304001 right: 1789729304000`) — exactly the assertion the
  comment says it fails on.
- `the_second_authoring_carries_the_higher_counter` **also failed**, but on
  its own precondition guard at line 885 (`the fixture has drifted: the
  second op's id no longer sorts ABOVE the first's...`) rather than on the
  "successive counters" assertion its name and body describe — exactly the
  distinction the comment draws.

Restored the mutation immediately afterward (`git diff --stat
dialectica/rust-lib/dialectica-core/src/authoring.rs` empty before commit).
The comment now says truly what the named test catches and what else fails
under the same mutation, and why. No mutation left in the tree.

No new checkbox findings. Both changes in this diff are clean: the spec split
carries no requirement-text change, and the test comment's claims are each
confirmed by measurement rather than assumed.

### Verification run before hand-back

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: all passed, 0 failed (tree clean of mutation at the time
  of this run).
- `nix build ./dialectica#lgx` from the tree root: succeeded.
