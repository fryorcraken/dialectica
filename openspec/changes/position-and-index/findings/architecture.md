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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs`,
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
