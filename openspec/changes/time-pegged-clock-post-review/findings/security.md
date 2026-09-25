# Security review — `time-pegged-clock-post-review`

Scope covered: **security only**, on both diffs named in the brief:

1. `git diff origin/main...HEAD` (three dots) — this piece's own diff: removing
   the reasoning #165 wrote into six live specs (`op-ordering`, `op-format`,
   `op-transport`, `feed-view`, `post-revision`, `thread-read`), plus four
   repointed doc comments in `dialectica-core`.
2. `git diff 2eada33 c1f1a8f` — the four #165 commits that merged without
   review (`authoring.rs`, `moderation.rs`, `revision.rs`, `transport.rs`, and
   the `op-ordering`/`op-transport` deltas under
   `openspec/changes/time-pegged-clock/`).

I read the owner's decision comment on issue #162 first. Its first line:

> **Decision (owner, 2026-09-25): peg the Lamport counter to wall-clock time,
> as SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`, lines 148-155
> and 184-192).**

No findings to report. Both diffs are clean from a security standpoint. What
follows is the verification work, so the next reviewer (or the runner) does
not have to redo it.

## Diff 1 — reasoning removed from six specs

The brief's one security question: does any requirement protecting against a
hostile peer lose a MUST, a MUST NOT, or a scenario along with its reasoning?

I diffed each of the six delta files
(`openspec/changes/time-pegged-clock-post-review/specs/*/spec.md`) against the
current canonical spec at `openspec/specs/*/spec.md` requirement by
requirement, paragraph by paragraph — not just via `git diff`, since every
delta file is a new file against `origin/main` (the six requirements it
restates already exist on `main` from the archived #165 change). Findings:

- **Every SHALL / SHALL NOT / MUST / MUST NOT sentence survives verbatim**,
  across all six specs, including the two the owner's decision named as
  requiring the reversal to be stated: the receive window ("An op whose
  counter is more than one hour ahead of this peer's time is refused on
  arrival") and the wall-clock ("The wall-clock decides nothing…"). What is
  removed in every case is explanatory prose — "why", "this replaces X",
  "this scenario keeps its name because…" — never the normative sentence
  itself.
- **Every scenario survives verbatim, word for word**, in all six files. I
  checked scenario counts and WHEN/THEN/AND bodies line by line for
  `op-ordering` (5 requirements touched), `op-format` (2), `op-transport` (2),
  `feed-view` (1), `post-revision` (1) and `thread-read` (2). None dropped,
  none reworded in a way that narrows or weakens what it asserts.
- **The two reversals the owner explicitly required the spec to "say so and
  why" about** — "no op is refused for a field value" and "the author's
  claimed time decides nothing" — do lose their explanatory paragraphs from
  the spec text (e.g. `op-ordering`'s "**This requirement used to stand for a
  wider principle…and that principle is withdrawn**" paragraph, and "**The
  cost is stated, not hidden**" for the receive window). I checked
  `design.md`'s Decision 7 and Decision 8: both reversals, and the "why", are
  restated there in full, cross-referenced by label (O8, O17, O18, F3). This
  matches `proposal.md`'s "Reasoning removed from the specs" section, which
  quotes all 18+4+5+1+1+2 = 31 removed passages verbatim against their source
  and maps every one to a `design.md` Decision or to the archived design. I
  spot-checked several of these quotes against the actual removed text and
  they match exactly.
- Nothing checked here is itself a security requirement that got weaker: the
  actual enforceable rule (the one-hour window, no lower bound, no overflow,
  the wall-clock never ordering, the saturation-not-overflow rule, the
  window's independence from the wall-clock field) is unchanged text, matched
  identically between `origin/main`'s canonical specs and this piece's deltas.
- The four repointed doc comments (`arrival.rs`, `op.rs` ×2, `transport.rs`)
  only change which document a comment cites (`op-ordering` → the archived
  `design.md`, Decision 11); no code or behaviour changed under them.

This is a case where "reasoning removed" is exactly what it claims to be:
every dropped sentence is either pure narrative ("this scenario keeps its
name because…") or is captured, cited and expanded in this change's own
`design.md`. I did not find a passage that reads as behavioural in a way a
test could distinguish, silently dropped without a place it went.

## Diff 2 — the four unreviewed #165 commits

This range closes two `NO SPEC` markers and fixes one stale doc comment:

- `authoring.rs`: adds
  `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`,
  a new test proving the counter and wall-clock fields diverge correctly when
  a peer's clock is above its current time (closing `op-ordering`'s "A
  counter taken from the clock leaves the wall-clock at the current time"
  scenario, added to the spec delta in the same range). Verified: the test
  computes both expectations independently of the code under test, and it
  passes (`cargo test`, below).
- `transport.rs`: rewords a `NO SPEC:` marker into a cited requirement
  ("A held op arriving again beyond the window is refused, and stays held");
  no code change, doc and spec only.
- `moderation.rs` / `revision.rs`: adds doc-comment text disclosing a genuine,
  accepted timing property — a moderator (or an author revising their own
  post) can sign a counter up to one hour ahead of its true time and thereby
  win a last-write-wins race (`moderation::resolve`, `revision::current_version`)
  against a correction the other side has not yet had time to receive. This is
  exactly the "worth checking" item the owner's decision comment named
  ("what the one-hour window allows against `revision::current_version`,
  `moderation::resolve` and the feed's `latestReply`"). It is disclosure of an
  already-accepted, bounded cost (the one-hour window), not a new
  vulnerability introduced by this range — the underlying `cmp_ops` behaviour
  is unchanged; only the doc comment now says so.
- `findings/architecture.md` and `findings/spec-test.md` in
  `openspec/changes/time-pegged-clock/`: ticks two prior open findings
  ("stale causal-not-temporal reasoning in `moderation.rs`" and the two
  `NO SPEC` markers) with matching fixes, and records that the fix also
  caught two further stale sites in `revision.rs` the first sweep missed.

No security regression found in this range. It only adds tests, spec text and
disclosure comments; no refusal, bound or check changed.

## What I checked and found clean

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`:
  **1180 passed in the unit/lib suite, 30 passed in `end_to_end.rs`, 0
  failed.**
- `nix build ./dialectica#lgx`: succeeded (exit 0, no output).
- No new dependency introduced by either diff.
- No panic-reachable path touched by either diff (no indexing, slicing,
  `unwrap`/`expect`, or arithmetic changed — only doc comments, spec prose and
  one new test).
