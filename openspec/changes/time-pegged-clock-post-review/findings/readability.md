# Readability review — dimension: readability only

Read the owner's decision comment on #162
(https://github.com/fryorcraken/dialectica/issues/162#issuecomment-5826096926):
first line is "**Decision (owner, 2026-09-25): peg the Lamport counter to
wall-clock time, as SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`,
lines 148-155 and 184-192).**"

Reviewed both diffs per `proposal.md`, "Review scope":
1. This change: `git diff origin/main...HEAD` (three dots).
2. The #165 unreviewed tail: `git diff 2eada33 c1f1a8f`.

## Findings

- [x] **`spec-writer`** — `openspec/changes/time-pegged-clock-post-review/specs/op-ordering/spec.md:71`
      (diff 1, this change's own delta) — the clock requirement's reasoning
      paragraph was reduced to two bolded sentence fragments jammed onto one
      line with no connecting prose between them:
      `**The clock is the highest counter held, with no exception for a
      counter far above the rest.** **The clock does not read the current
      time.**`
      **Scenario:** before this change, the same spot read: "**The clock is
      the highest counter held, with no exception for a counter far above the
      rest.** This clock previously excluded a counter more than a fixed bound
      above the rest, so that one absurd counter could not pull it to the
      ceiling. That exclusion is gone. The receive window, [...], keeps such a
      counter out of the store altogether, so every counter the clock reads is
      one the peer admitted. **The clock does not read the current time.** The
      time enters when an op is signed, through [...]. It does not enter the
      clock, so the clock remains a function of the ops held and nothing
      else." Removing O3/O4's reasoning (correctly, per `proposal.md`) left
      two topic sentences with nothing under either — the only place in all
      six spec deltas where two bold leads sit back to back with no supporting
      sentence for either (confirmed by `git grep -F -e "** **"` across
      `specs/`, one hit total). Every other paragraph in these deltas keeps at
      least one plain sentence after its bold lead. A reader hits two
      assertions in a row with no "why" and no distinguishing separation
      (not even a line break) — it reads as leftover debris from the cut
      rather than a considered requirement clause. Fix: either put each
      sentence in its own paragraph (they cover two different sub-points: "no
      exception" and "does not read the time") or restore one connecting
      clause for each.
      **Severity:** minor/cosmetic — no normative content is lost (both
      claims are otherwise supported: "highest counter held" restates the
      requirement's own opening MUST, and "does not read the current time" is
      implied by the "MUST depend on those ops and on nothing else" sentence
      two paragraphs up) — but it is a genuine readability defect, not a
      stylistic preference, since it's the one spot where the mechanical
      removal left an ungrammatical-reading residue.
      **Outcome (`spec-writer`): fixed.** I took the reviewer's first option.
      Each sentence now stands as its own plain paragraph, with the text
      unchanged word for word. The bold is dropped, because a bold lead with
      nothing under it is what made the pair read as debris. Restoring a
      connecting clause was ruled out: the clauses that used to sit there are
      O3 and O4, which are reasoning the owner's "follow the readme" ruling
      moves to `design.md`. Adding them back would undo this change. The
      edit adds no MUST, MUST NOT, SHALL or SHALL NOT, and removes none. No
      scenario changes. `proposal.md`'s "Judgement calls" already quotes both
      sentences, unbolded, as behaviour kept, so it needs no edit. There is
      no test to add, since no behaviour changed.

## What was checked and found clean

- **All six spec deltas** (`op-ordering`, `op-format`, `op-transport`,
  `feed-view`, `post-revision`, `thread-read`) were diffed sentence-by-sentence
  against the archived `2026-09-25-time-pegged-clock` spec text (#165's
  version) to check every removal proposal.md labelled O1–O18, F1–F4, T1–T5,
  V1, R1, H1–H2. Every requirement reads as a complete, grammatical statement
  once its reasoning is removed, with the single exception above. No dangling
  pronoun, no orphaned "This is inherent to..." referring to a deleted
  antecedent, no scenario left describing a rule that no longer exists.
- **The four repointed doc comments** (`arrival.rs` on `RECEIVE_WINDOW_MS`,
  `op.rs` twice, `transport.rs` on `receive`) all send the reader to
  `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md`, Decision
  11 (and Decision 1 for `transport.rs`). Verified both decisions exist under
  those exact headings (`### 11. A receiver whose own clock is wrong` and
  `### 1. Why the counter is pegged to the time...`) and that each argues
  exactly the cost the comment claims (Decision 11 works through a slow and a
  fast receiver's clock in the detail the old "op-ordering states the cost"
  pointer promised; Decision 1 gives the rejected alternatives that
  `transport.rs`'s "is not a judgement of authority" line draws on). Both
  links answer the reader's question.
- **`design.md`'s own citation table** (Decision 1, "Where each label went")
  was spot-checked against the archived design's actual decision numbers
  (11 exists, its content matches; Decision 9's "both numbers" claim in this
  piece's Decision 9 matches the archived text verbatim in substance). No
  citation pointed at a decision number that doesn't exist or doesn't argue
  the claim.
- **The #165 unreviewed-tail diff** (`2eada33..c1f1a8f`): the doc-comment and
  test-comment rewrites in `authoring.rs`, `moderation.rs`, `revision.rs`,
  `transport.rs`, and the spec deltas they carry in `op-ordering`/
  `op-transport`, all read clearly. The new test
  `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
  in `authoring.rs` has an unusually thorough comment explaining exactly which
  mutation it catches that the sibling test doesn't — good, not a finding.
- **`proposal.md` and `design.md`** (both new, prose-only): read in full.
  Internally consistent — the "Reasoning removed from the specs" catalogue in
  `proposal.md` and the "Where each label went" table plus the Decisions in
  `design.md` account for every label with no gaps or duplicates found.

## Build/test results

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`: 1180 passed (dialectica-core unit tests) + 30 passed (end_to_end), 0 failed.
- `nix build ./dialectica#lgx`: succeeded (no output, exit 0).

No mutations were made to the tree during this review (read-only + test/build runs).
