# Design review — time-pegged-clock-post-review

Scope: two diffs, as `proposal.md`'s "Review scope" describes.

- **Diff 1**: `git diff origin/main...HEAD` — this piece's own removal of #165's
  spec reasoning into `openspec/changes/time-pegged-clock-post-review/design.md`.
- **Diff 2**: `git diff 2eada33 c1f1a8f` — the four #165 commits that merged
  without review (`d6208fb`, `ae0c30d`, `c34ec46`, `c1f1a8f`), now folded into
  `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md` and the
  archived specs/code.

## Diff 1 — this piece's own design.md

I checked every label in Decision 1's table (O1–O18, F1–F4, T1–T5, V1, R1, H1–H2)
against the section it claims to land in, rather than trusting the table, and
against the archived design's Decision it cites where it points there instead of
repeating the argument. Every label lands where the table says, and every
citation of an archived Decision number resolves to a Decision that actually
argues the point (checked Decisions 1, 2, 3, 5, 7, 9, 10, 11 in the archived
`design.md` directly). The Goals claim — "every removed passage is either argued
below or covered by a citation" — holds; I did not find an unargued passage.

I also swept every doc-comment reference to `op-ordering` across
`dialectica-core/src` (`git grep -n "op-ordering"`) to check the Risks claim that
exactly four comments pointed at reasoning this piece removes. The other
references found (`moderation.rs`, `authoring.rs` test comments, `log/sqlite.rs`,
`transport.rs`'s module doc, `stoa_metadata.rs`) all cite spec requirement or
scenario **names** (behaviour, still in the spec) or a different, pre-existing
archived change (`op-ordering`'s own, not #165's) — none cite reasoning this
piece is removing. The four-comment sweep claim holds, and the four repointed
comments (`arrival.rs`, `op.rs` ×2, `transport.rs`) correctly cite Decision 11
(and Decision 1 for `transport.rs`), which does carry the cost they point at.

No findings against diff 1. The decisions are in good shape: every removed
passage is accounted for, every citation resolves, and the code-comment sweep
is accurate.

## Diff 2 — the four unreviewed #165 commits

- [ ] **`dev-writer`** — `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md:127-133`,
      Decision 3, "What pins it" — the claim understates the change's own test
      coverage, in material this change's own commit range produced.
      **Diff 2, commits `c34ec46` then `c1f1a8f`.** `c34ec46` wrote: "The second
      scenario, where the counter comes from the clock and the wall-clock stays
      at the time, is satisfied by the same line of `publish`: the field is
      signed as passed, whatever `next_counter` returned" — i.e. no dedicated
      test, satisfied by inspection of one line. The very next commit in the
      same unreviewed range, `c1f1a8f` ("Pin the missing time-pegged-clock
      scenario: clock above time leaves the wall-clock alone"), added
      `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
      in `authoring.rs` specifically to pin that second scenario, and the commit
      message records the mutation evidence: "mutated `publish` in
      `authoring.rs` to write the counter into the wall-clock field, watched this
      test fail (left: 1789732304001, right: 1789729304000) ... then restored
      the mutation." Decision 3 was never updated to name this test or carry its
      mutation evidence, so a reader of the archived design is told a guard the
      change added is instead an untested inference from one line. **Verified:**
      `git grep -n "leaves_the_wall_clock_at_the_current_time" dialectica/rust-lib/dialectica-core/src/authoring.rs`
      finds the test; `cargo test -p dialectica-core` (run for this review)
      passes it. Since the archive is not normally re-edited (this change's own
      Decision 1 treats it as history), whoever acts on this should decide
      between a corrective note in this change's own `design.md` or a rare
      archive correction — but the record as it stands is wrong about its own
      coverage.

- [ ] **`dev-writer`** — `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md:262-265`,
      Decision 10's closing sentence, against `revision.rs` and `moderation.rs`.
      **Diff 2, commit `ae0c30d`** ("Reword the counter's reasoning in
      moderation.rs and revision.rs"). Decision 10 claims: "The exposure is
      recorded in each reader's doc comments too, `revision.rs` on
      `current_version` and `moderation.rs` on `resolve`." The rule was applied
      fully at one site and only partially at the other. `moderation.rs`'s
      addition on `resolve` states the exact exposure Decision 10 describes: "a
      moderator can sign a `Hide` or `Unhide` up to an hour ahead and beat an
      opposite action that another moderator published within that hour without
      having received it." `revision.rs`'s addition, in the module doc above
      `current_version`, only says the counter is "that author's unverified
      claim about the time — an author may sign up to an hour ahead of a
      receiver's time" — the generic bound, not the specific exposure Decision
      10 states for this reader ("the lead is over the author's other versions,
      published from a device that had not yet received the ahead-signed one,
      for up to an hour. No third party can use it"). `current_version`'s own
      function doc (`revision.rs:270-287`, "What is dropped, and in what order")
      carries none of this either. A reader of `revision.rs` alone would not
      learn that the one-hour lead is self-only here, which is exactly the fact
      that makes this reader's exposure narrower than `moderation.rs`'s.
      **Verified:** `git grep -n "up to an hour" dialectica/rust-lib/dialectica-core/src/revision.rs dialectica/rust-lib/dialectica-core/src/moderation.rs`
      shows the asymmetry directly.
