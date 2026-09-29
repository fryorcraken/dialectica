# Architecture review — position-and-index (#166 / PR #180)

Dimension covered: **architecture only** (capability ownership, whether two
capabilities now assert one rule, test fixture/helper shape, fit with the
codebase's structure). Read issue #166 and the owner's decision comment of
2026-09-25 (https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024),
which settles item 2: a malformed `index` is refused with a message naming
`index`, and leaving the message unspecified is ruled out. Reviewed
`git diff origin/main...HEAD` (three dots): `proposal.md`, `design.md`,
`tasks.md`, both spec deltas, and the test additions in `wire.rs`. Confirmed
the diff touches only `wire.rs` (inside `mod tests`) and the `openspec/`
files — no production code changed, matching the "No production code changes"
claim in `design.md`/`tasks.md`.

## Findings

- [x] **`spec-writer`** — `openspec/specs/identity-onboarding/spec.md`
      (Purpose section) — the new ADDED requirement shares `parse_index` with
      `feed-read`'s `page`/`perPage` refusal, and `proposal.md` ("Modified
      Capabilities") and `design.md` (D1) both reason carefully about why this
      is *not* a restatement of `feed-read`'s rule — but that reasoning is
      only in the change's own `proposal.md`/`design.md`, never in the live
      spec. `identity-onboarding`'s Purpose already has a "Boundary with other
      capabilities" paragraph (naming `identity`, `keystore`,
      `posting-capability`) but it does not name `feed-read`, even though this
      piece adds the first requirement that shares implementation with it.
      Contrast `thread-read`'s Purpose, which lists `op-ordering`,
      `post-revision` and `moderation-resolution` with one line each on what
      is deliberately not restated — and contrast `op-ordering`'s own
      Purpose, which `docs/OPENSPEC-ARCHIVE.md` names as the example that
      "declined to restate [`op-format`'s requirement], and said so in its
      Purpose". This change does the reasoning but not the recording: once
      `archive` moves this change's `proposal.md`/`design.md` into
      `openspec/changes/archive/`, a future reader of the live
      `identity-onboarding` spec alone has no signal that the omission of a
      `feed-read` boundary line is deliberate rather than an oversight — the
      exact failure mode `docs/OPENSPEC-ARCHIVE.md` "Two capabilities
      asserting one rule" section warns produces silent drift (its
      counter-example is `spec-backfill`, which didn't say so and produced a
      duplicate). **Scenario:** a later change touches `feed-read`'s
      pagination-refusal wording, sees no boundary note pointing at
      `identity-onboarding`, and does not think to check whether `index`'s
      refusal (same parser) should move in step. Fix: add one sentence to
      `identity-onboarding`'s Purpose boundary paragraph naming `feed-read`
      and the shared parser, the way `op-ordering` does for `op-format`.
      Severity: moderate — not a behavioural defect, but a discoverability
      gap in exactly the place `docs/OPENSPEC-ARCHIVE.md` calls out as
      already having bitten this repo once.

      **Outcome (`spec-writer`): fixed.** `identity-onboarding`'s Purpose gains a
      paragraph after its boundary paragraph. It names `feed-read` as the owner
      of the `page`/`perPage` refusal, cites the new requirement by name, says
      the two apply to the same kinds of malformed value, that neither restates
      the other and each covers only its own fields, and that a change to what
      `feed-read` counts as malformed has to be checked against the `index`
      requirement. It is a separate paragraph rather than a fourth name in the
      existing list, because that list names capabilities this one relies on,
      and `feed-read` is a parallel. The parser goes unnamed because it is
      implementation, and D1 in `design.md` already records it. The edit is made
      directly to the live spec, because a delta cannot change a Purpose. That
      follows #160's precedent (`d8a56272`), and `proposal.md` now records it
      under Modified Capabilities. Archive merges only the ADDED requirement, so
      the edit survives it.

      Only one direction is fixed. The scenario starts with a reader in
      `feed-read`, and `feed-read`'s Purpose gains no reverse pointer. Its five
      boundaries name the capabilities whose rules it uses. Listing every other
      capability that refuses a field by name would be an open-ended list that
      goes stale when a sixth one appears. The dependency points from `index`'s
      list of kinds to `feed-read`'s, so the note belongs on the dependent side,
      as `op-ordering`'s does for `op-format`. No test covers this, because the
      Purpose states no behaviour.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs`,
      the `under_root_at` closure inside
      `a_thread_log_with_a_hidden_reply_and_a_reply_after_it` — duplicates
      the existing `a_thread_post` helper's `Op` literal (same `stoa`,
      `author`, `kind: OpKind::Post { .. }` shape) instead of extending or
      delegating to it, differing only in that it sets an explicit `clock`
      where `a_thread_post` hardcodes `clock: None`. This piece's own D4
      decision (`design.md`) shows the delegation pattern was known and used
      elsewhere in this same change (`keep_through_the_wire` now delegates to
      `keep_with_raw_index`), so the same move was available here: add an
      optional-clock parameter to `a_thread_post` (or a
      `a_thread_post_with_clock` that `a_thread_post` calls through) rather
      than a second, hand-written copy of the `Op` struct literal.
      **Scenario:** `Op` or `OpKind::Post` gains a field later (e.g. an
      attachment kind or a revision marker); the two literals now have to be
      updated in step by hand, in a file where every other `OpKind::Moderate`
      construction (all 8 occurrences found) is already independently
      inlined per test — this would be the second such pair rather than the
      first. This is not a defect in behaviour (the doc comment on the new
      function already explains, correctly, why the *fixture-level*
      `a_thread_log_with_shared_authors` can't be reused — a different,
      correct point) — it is a stylistic/maintenance observation, not a
      blocking one. Low severity.

      **Outcome (`dev-writer`): fixed** in the commit that ticks this box. The
      `Op` literal moved into a new `a_thread_post_with_clock`, which takes the
      clock as a parameter. `a_thread_post` now delegates to it with `None`, and
      the hidden-reply fixture's `under_root_at` calls it with its ascending
      counter. The `Op` literal for a fixture post now exists in one place. The
      change is test-only and changes no behaviour: `a_thread_post` builds the
      same op it did before, and signing is deterministic, so every fixture op
      id is unchanged. The full suite stays green.
      No test fails without this change, because it adds no behaviour. What
      it has to show is that the fixtures still discriminate. Every mutation
      `design.md` names was re-run against `thread_page_json` and `parse_index`
      and then restored:
      - constant position (`"0"`): the uniqueness and place tests go red, and
        the page-size test stays green.
      - `item.author`: the uniqueness and place tests go red, and the page-size
        test stays green.
      - op id (`item.id`): only the place test goes red.
      - per-page index: the uniqueness and page-size tests go red, and the place
        test stays green.
      - `{field}` dropped from the wrong-type arm: the index and pagination
        by-name tests go red. Dropped from the non-integer arm: those two go red
        along with the largest-page test.

      Every test stays green or goes red exactly as D2 and D5 predict. The place
      test's failure under `item.author` and the op id is at place 1, where
      `to_hide` and `after` disagree. That confirms the refactored fixture still
      places `to_hide` immediately before `after`. The eight `OpKind::Moderate`
      literals the finding counts are left as they are. They predate this
      piece, and reshaping them would be a refactor this change does not need.

## What was clean

- **Capability ownership.** `identity-onboarding` is confirmed (by reading
  `keep_identity` in `wire.rs`) to be the capability that owns
  `keep_identity`, matching the owner's decision comment and the proposal's
  placement. The new requirement cross-references rather than restates
  `identity-onboarding`'s existing *Every entry point refuses malformed input
  rather than guessing* and *A selection outside the current set is refused*
  — the same non-duplicating pattern `docs/OPENSPEC-ARCHIVE.md` praises for
  `op-ordering`.
- **The `thread-read` MODIFIED delta is exactly what `proposal.md`/`design.md`
  claim.** Diffed the delta text against the current live requirement
  byte-for-byte (`diff` of the two blocks): exactly one sentence edited (the
  uniqueness clause), one paragraph added (the place rule), one scenario
  added, everything else — including every other scenario — copied
  unchanged. No silent narrowing or loss of an existing scenario, which is
  the specific "MODIFIED requirement replaces the WHOLE block" trap
  `docs/OPENSPEC-ARCHIVE.md` calls out.
- **No cross-capability contradiction.** `op-ordering`'s own requirement
  (line 250 of its spec) already explicitly defers "what the ordering-position
  field promises" to `thread-read`, so this change's tightening of
  `thread-read`'s position rule (place-determined, not item-determined) is
  filling a gap `op-ordering` already assigned to it, not colliding with it.
  `feed-read`'s uses of "position" are a distinct field on feed rows, not the
  same value.
- **Test fixture/helper shape, otherwise.** `MALFORMED_INDEXES` is a single
  table both `each_malformed_kind_of_index_is_refused_by_name` and
  `a_malformed_index_stores_nothing` iterate, avoiding the "hand-maintained
  sweep list" drift risk. `keep_with_raw_index`/`keep_through_the_wire` is a
  clean, isolated test-only refactor (task 1.1), independently green per
  `tasks.md`. `thread_positions_at`'s page-walking-until-`hasMore`-false shape
  matches the existing (if independently-duplicated-per-capability)
  convention already used elsewhere in this file for other capabilities' own
  pagination tests — not a new inconsistency this piece introduces.
- **No changes under `.claude/`.** Nothing in this piece touches `.claude/`;
  no action needed there.

Findings: 1 for `spec-writer`, 1 for `dev-writer`. Both are non-blocking
(discoverability / minor duplication), not correctness or security defects —
those dimensions are other reviewers' rows.

## Re-review of the findings-round commits

Dimension: **architecture only**, as before. Read issue #166 and the owner's
decision comment of 2026-09-25
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024),
which settles item 2 and rules out leaving the message unspecified. Scoped to
`git diff 19667a04...HEAD`, against the three items named for this dimension in
`tasks.md`'s "Re-review of the findings-round commits" section: the live-spec
edit in `b55d164`, the test-helper restructuring in `70bef05`, and how the two
findings above were answered. `9e550eb` (design.md) and `4e2b8c0` (rustfmt) are
not architecture's rows in that section and are not re-reviewed here beyond
confirming no production code changed.

### 1. The live-spec edit (`identity-onboarding` Purpose)

Checked directly rather than by reasoning about `openspec`'s documented
behaviour: ran a real `openspec archive position-and-index -y` in this
worktree, inspected the result, then discarded it
(`git checkout -- openspec/specs/... openspec/changes/position-and-index/`,
`rm -rf openspec/changes/archive/2026-09-26-position-and-index`) and confirmed
`git status` was clean again before continuing.

- **Survives archive intact.** The new Purpose paragraph — "A keep request's
  `index` is refused by name in parallel with `feed-read`, not by it..." — came
  through byte-for-byte unchanged. Archive only appends the delta's `## ADDED
  Requirements` into the target's `## Requirements`; it never touches `##
  Purpose`, so a Purpose edit made directly (the only way to edit one, since a
  delta can't) cannot be disturbed by the merge step. The requirement title the
  paragraph cites in italics — "A malformed `index` in a keep request is
  refused with a message naming `index`" — matches the delta's actual heading
  character-for-character, so after archive the citation names a requirement
  that is now really there, in the same file, not a hopeful forward reference.
- **Citing the ADDED requirement by name before it's promoted is not a
  problem.** `openspec validate --strict position-and-index` passes clean on
  the current, unarchived tree — the tool does not parse or check prose
  citations of requirement titles, so the forward reference (the cited title
  exists nowhere yet in this file's own `## Requirements`) is invisible to
  validation. And per `docs/OPENSPEC-ARCHIVE.md` ("Archiving before the merge
  rather than after is deliberate"), the closer runs `archive` in the same PR,
  before CI's final gate and before the merge — so the forward-reference state
  is never what lands on `main` or what CI's last run sees; it exists only
  during review of an in-flight branch, which is the state every other
  ADDED-but-not-yet-archived requirement in this repo is already in.
- **The decision not to add a reciprocal note to `feed-read`'s Purpose holds
  up.** Read `feed-read`'s Purpose: its "Five boundaries are named rather than
  restated" list names capabilities `feed-read` itself relies on for a rule
  (`thread-read`, `op-ordering`, `moderation-resolution`, `post-revision`,
  `generated-names`) — dependencies, not dependents. `identity-onboarding`
  depends on nothing `feed-read` owns; the two only share an implementation
  detail, `parse_index` (D1 in `design.md`), which neither capability's spec
  is about. Adding a line to `feed-read` naming `identity-onboarding` would
  break that list's own pattern (capabilities it relies on) rather than extend
  it. This matches `op-ordering`'s asymmetric precedent that
  `docs/OPENSPEC-ARCHIVE.md` cites approvingly: the dependent names the
  dependency-like relationship, not the other way round.

No finding. This is confirmed clean, not merely re-asserted.

### 2. The test-helper restructuring (`a_thread_post_with_clock`, `70bef05`)

Read the current code (`wire.rs:8556-8592` for the two functions,
`wire.rs:9351-9367` for the call site). `a_thread_post` now delegates to
`a_thread_post_with_clock(author_seed, thread, parent, body, None)`; the
hidden-reply fixture's `under_root_at` closure calls the same shared function
with its own per-reply clock instead of hand-writing a second `Op` literal.
This is exactly the shape my first-round finding asked for, and the doc
comment's claim — "The one place a fixture post's `Op` literal is written, so
a field added to `Op` or `OpKind::Post` is added here once" — is true of the
code as it stands.

Checked that the refactor didn't leave a stray duplicate or overreach its
scope: `grep -n "OpKind::Post {"` in `wire.rs` still finds roughly a dozen other
literal constructions, all pre-existing, all outside this fixture family (they
build ops for other tests' own fixtures, e.g. `OpKind::Moderate`). None of them
duplicates `a_thread_post`'s shape, so there's nothing left over for this
change to have folded in — the refactor is scoped to the two call sites that
actually duplicated one another, not stretched to a repo-wide sweep.

Ran the full suite after staging the SDK symlink
(`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core`): 1185 passed in `dialectica-core`, 30 in `dialectica`, 0
failed — consistent with the claim that the refactor changes no behaviour
(signing is deterministic, so every fixture op id is unchanged).

No finding.

### 3. How the two first-round findings were answered

Checked both "Outcome" write-ups above against the current diff and file
contents, not taken on trust:

- **`spec-writer`'s outcome** (lines 50–63): the paragraph's placement (a
  separate paragraph after "Boundary with other capabilities" rather than a
  fourth bullet in that list), its wording, and its survival through archive
  all match what the outcome claims — independently re-derived above rather
  than re-read. The separate-paragraph placement is the right call: the
  existing boundary list names capabilities this one restates nothing from,
  while the new paragraph names a capability this one runs a rule in parallel
  with — a different relationship, and conflating the two into one list would
  have obscured which was which.
- **`dev-writer`'s outcome** (lines 98–126): the structural claim — "the one
  place a fixture post's `Op` literal is written" — is true of the code (§2
  above). I did not re-run the five position mutations myself; that mutation
  proof is `correctness`/`spec-test`'s re-review row, not architecture's, and
  re-running it here would duplicate rather than add to their check.

No new architecture findings from this re-review round. Both boxes ticked in
the first round remain correctly resolved.

## Re-review after merging main

Dimension: **architecture only**. Read issue #166 fresh
(`gh issue view 166 --json body,comments`) and confirmed the owner's decision
comment of 2026-09-25
(https://github.com/fryorcraken/dialectica/issues/166#issuecomment-5832956024):
malformed input is rejected and the refusal names `index`; leaving the message
unspecified was explicitly ruled out. Read `docs/OPENSPEC-ARCHIVE.md` before
mutating anything.

Scope per `tasks.md`'s "Re-review after merging main": the merge `d0f56a14`,
the `thread-read` delta rebase `7d587fd`, and the design/proposal citation fix
`696d35f`. The question is whether archiving now loses nothing and restores
nothing.

**Safety check before mutating**, as required: `pwd` reported
`.../.claude/worktrees/agent-ad91ddc5d528c9212`, and `git rev-parse
--abbrev-ref HEAD` reported `worktree-agent-ad91ddc5d528c9212` — the tree the
harness created for this dispatch, not `piece/166-position-and-index` and not
the repository root. Confirmed clean (`git status`) before archiving.

### What was checked, and how

1. **What else moved in main.** `git log --oneline 8368b2f..d0f56a14^2` shows
   four PRs landed after the piece's base: #181, #173, #172, #175. Of these,
   only `2b53e6eb` (#173) touched a spec file this change asserts a rule in
   (`openspec/specs/thread-read/spec.md`); it also touched
   `openspec/specs/feed-view`, `op-format`, `op-ordering`, `op-transport` and
   `post-revision`, none of which this change carries a delta for. #181
   touched `feed-view`/`stoa-navigation-view` (UI/e2e), #172 touched
   `view-navigation` and CI/test tooling, #175 touched CI/docs only — none
   overlaps `thread-read` or `identity-onboarding`.
2. **#173's code-level touch.** #173's Rust diff (`arrival.rs`, `authoring.rs`,
   `moderation.rs`, `op.rs`, `revision.rs`, `transport.rs`) is doc-comment
   rewording only, confirmed by #173's own commit message ("Impact said 'Code,
   wire API, tests: none' ... every changed line a comment") and by reading the
   `arrival.rs` hunk directly: it repoints a doc comment from `op-ordering` to
   the archived `time-pegged-clock` design, no logic changed. `arrival::cmp_ops`
   — which D5's fixture comment relies on for ascending-clock ordering — is
   untouched. So the merge changes no behaviour this piece's tests or design
   reasoning depend on.
3. **The `thread-read` delta against the live text after #173, measured, not
   read.** Ran `openspec archive position-and-index -y` in this worktree (a
   real archive, not a inference from the CLI's documented behaviour).
   `git diff` on the resulting `openspec/specs/thread-read/spec.md` shows
   exactly three hunks, matching `tasks.md`'s claim and nothing more: the
   uniqueness sentence narrowed from "for two distinct items of one thread" to
   "for two distinct items returned by one read of a thread, taken across all
   of that read's pages"; one new paragraph ("The position SHALL be determined
   by the place alone..."); and one new scenario ("The item at a place carries
   that place's position in every read") inserted between the existing
   "Two items of one thread never share a position" and "The position is not
   surfaced as a quantity" scenarios. Every other sentence and every other
   scenario in the requirement — including the ones #173 left alone — came
   through byte-for-byte. This is the specific "MODIFIED requirement replaces
   the WHOLE block" trap `docs/OPENSPEC-ARCHIVE.md` warns about, and it does
   not fire here.
4. **The `identity-onboarding` ADDED requirement against whatever main now
   holds.** The same archive run's diff on
   `openspec/specs/identity-onboarding/spec.md` shows the requirement appended
   verbatim at the end of the file, nothing else touched. No merged PR added a
   competing `index`-refusal requirement under a different heading (checked
   via `grep -n "^### Requirement" openspec/specs/identity-onboarding/spec.md`
   before archiving — no such heading existed), so there is no duplicate to
   reconcile. The live Purpose's forward-referencing "boundary with
   `feed-read`" paragraph (from the earlier `b55d164` round, D6) is unaffected
   by the merge and survived this archive run intact, matching the prior
   re-review's finding.
5. **Discarded the archive completely**, per the brief: `git checkout --
   openspec/` followed by `rm -rf
   openspec/changes/archive/2026-09-26-position-and-index`. `git status`
   confirmed a clean tree afterward.

### Result

No architecture findings from this round. Archiving now loses nothing from
the live `thread-read` requirement as #173 left it, and restores nothing that
#173 removed on purpose (the "alternatives all invite arithmetic that means
nothing" paragraph stays out, moved to #173's archived design as that piece
intended). No other merged PR (#175, #172, #181) touches a capability this
change asserts a rule in.
