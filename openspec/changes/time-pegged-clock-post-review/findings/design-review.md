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

- [x] **`dev-writer`** — `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md:127-133`,
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
      **Outcome (`dev-writer`): fixed** in the commit that ticks this box, as a
      corrective note in this change's `design.md`. The archive stays
      unedited, per this change's Decision 1. The note is the new Decision 11,
      "Two corrections to the archived design", first paragraph. It names the
      test, says why the first scenario's test cannot see this, and carries the
      mutation evidence. I re-ran the mutation rather than quoting it: with
      `publish`'s `asserted_ms` set to `next_counter(clock, who.asserted_ms)`,
      `cargo test -p dialectica-core authoring::` failed this test with the
      same numbers (left 1789732304001, right 1789729304000), and I then
      restored the line. It also failed a second test,
      `the_second_authoring_carries_the_higher_counter`. That failure comes from
      its fixture-drift guard, because the changed wall-clock changes the op
      id. It is not an assertion about the wall-clock field, and Decision 11
      says so, so that nobody counts it as a second guard. No code changed for
      this finding.

- [x] **`dev-writer`** — `openspec/changes/archive/2026-09-25-time-pegged-clock/design.md:262-265`,
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
      **Outcome (`dev-writer`): fixed** in the commit that ticks this box.
      `current_version`'s function doc gains a section, "What signing an hour
      ahead buys here, and whom". It is modelled on `moderation.rs`'s paragraph
      on `resolve`. It says the lead is over the author's own other versions,
      from a device that had not received the ahead-signed one, for up to an
      hour. It gives the reason: the authorship check admits only the post's
      author's versions. It says no third party can use it, and links
      `resolve` as the wider case. It also gives the *before* case from
      archived Decision 10 and cites the `op-ordering` scenario that pins the
      behaviour. Archived Decision 10's last sentence is now true of both
      sites. This change's `design.md`, Decision 11, second paragraph, records
      that it was not true at #165's merge, and which change made it true.
      Doc comment only. No test can see it, and the behaviour it describes
      was already pinned by
      `an_op_signed_ahead_of_the_time_leads_only_until_the_time_passes_it`.

## Re-review of 439c192..HEAD

Scope per dispatch: the answers to the two findings above, landed as new
Decision 11 in `design.md` and the new "What signing an hour ahead buys here,
and whom" section on `revision::current_version`'s doc comment
(`revision.rs:285-303`), plus the four citation rewordings in `arrival.rs`,
`op.rs` (×2) and `transport.rs` that `architecture.md` asked for. I read the
owner's decision comment on issue #162
(https://github.com/fryorcraken/dialectica/issues/162#issuecomment-5826096926)
first; its first line is "Decision (owner, 2026-09-25): peg the Lamport
counter to wall-clock time, as SDS does". Nothing in `439c192..HEAD` touches
that decision's scope — this range is entirely the two prior findings' fixes
plus a doc-comment reword — so there is nothing here to check against it
beyond confirming the range does not reopen it, which it does not
(`git diff 439c192 HEAD --stat` touches no publish/receive-window code beyond
the doc comments already listed).

**Decision 11's mutation claim, checked against the code, not just read.** I
mutated `publish` in `authoring.rs` (`asserted_ms: who.asserted_ms` →
`asserted_ms: next_counter(clock, who.asserted_ms)`, the same change Decision
11 and the commit message for `c1f1a8f` describe) and ran
`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica-core
authoring::`. Both failures Decision 11 claims occurred, with the numbers it
quotes: `a_counter_taken_from_the_clock_leaves_the_wall_clock_at_the_current_time`
failed with `left: 1789732304001, right: 1789729304000`, matching Decision 11
exactly. `the_second_authoring_carries_the_higher_counter` also failed, but —
as both Decision 11 and the corrected fixture comment
(`authoring.rs:1826-1841`, from `394f786`) say — on its own fixture-drift
guard (`authoring.rs:885-892`, "the fixture has drifted..."), not on the
assertion the test's own body names, `newest first, by counter`
(`authoring.rs:895`). I restored the mutation afterwards;
`git status --short` is clean before this commit.

**Decision 11 and the `394f786` comment agree with each other and with the
code.** Neither describes a different mutation or a different failure mode
than what actually happens; the fixture-drift guard really is a guard for a
different property (which op id sorts higher, not the wall-clock's value), and
both documents say so rather than claiming it as a second pin on the scenario.

**The `revision::current_version` addition matches what the finding asked
for and what `moderation::resolve` already states.** It names the specific
exposure (self-only, up to an hour, over the author's own other versions from
a device that had not received the ahead-signed one), gives the reason
(the authorship check above admits only the post's author's versions), says no
third party can use it, cross-references `moderation::resolve` as the wider
case, and cites the `op-ordering` scenario that pins the underlying rule
(`An op signed ahead of the time leads only until the time passes it`, present
at `openspec/specs/op-ordering/spec.md:396`). Archived Decision 10's sentence
is now true of both sites, as Decision 11 claims.

**Citation reword (`arrival.rs`, `op.rs` ×2, `transport.rs`).** All four now
read "the archived `time-pegged-clock` change's `design.md`" rather than the
dated folder path, matching three pre-existing citations of that form
(`transport.rs` on `op-ordering`, `op.rs` on `stoa-metadata-op`, `wire.rs` on
`get-stoa`, all cited in `design.md`'s Risks section). This is a citation-form
change, not a decision; it carries no new claim to check against the archive
or the issue.

**No new findings.** `cargo test --manifest-path dialectica/rust-lib/Cargo.toml
-p dialectica -p dialectica-core` passes in full (1180 + 30 + 0 dialectica-core
tests, 0 dialectica tests, 0 doc-tests). `nix build ./dialectica#lgx` from the
tree root succeeds. `tasks.md` is not ticked by me, per dispatch.

## Re-review of e03230e..HEAD

Read the owner's decision comment on issue #162 first (per dispatch); its
first line is: "**Decision (owner, 2026-09-25): peg the Lamport counter to
wall-clock time, as SDS does (LIP-109, `logos-lips/docs/anoncomms/raw/sds.md`,
lines 148-155 and 184-192).**" It supersedes the issue body's "What this
needs" where the two disagree, and both are silent on this range's actual
content — the range is entirely the `dev-writer`'s and `spec-writer`'s
response to the three prior findings ticked above (architecture's Decision 1
finding, correctness's "permanently" finding, readability's two line-wrap
findings) plus the six regular-review doc-comment fixes to `moderation.rs`,
`op.rs` and `revision.rs`.

This range does two things Decision 11 (`design.md:353-420`) newly claims:
adds two forward-pointer notes to the archived
`2026-09-25-time-pegged-clock/design.md` (after Decision 3's "What pins it"
and after Decision 10's last paragraph), and renames the ambiguous "Before the
window" phrase to `ADVANCE_BOUND` in both `revision.rs:300` and
`moderation.rs:437`. I checked both against the code and against each other.

**The two archived notes say what Decision 11 says they say, and land where
Decision 11 says.** `git grep -n -F -e "time-pegged-clock-post-review" --
openspec/changes/archive/2026-09-25-time-pegged-clock` returns exactly two
lines, at `design.md:137` (Decision 3) and `design.md:275` (Decision 10) —
matching Decision 11's own "How to check them" and matching
`git diff origin/main...HEAD --stat -- openspec/changes/archive/`, which shows
only that one file, 8 insertions, nothing else touched. Both notes are headed
`*Added by a later change:*`, sit directly after the paragraph they correct,
and name Decision 11 by number, exactly as Decision 11 describes.

**The `ADVANCE_BOUND` rewording is identical at both sites and is not a new
claim.** `moderation.rs:437-440` and `revision.rs:300-304` now read the same
sentence ("Under `ADVANCE_BOUND`, which the receive window replaced in #165,
the lead had no end: an op signed at the maximum counter was stored and led
the order while the clock stayed below it, so [it] was ... permanently"), and
Decision 11 states they were made to agree. `git grep -n -F -e "permanently"
dialectica/rust-lib/dialectica-core/src` finds no other unqualified
"permanently" near a bound claim that this pass should have caught and did
not — the other ten hits are each about an unrelated store-corruption or
moderation-durability claim, not this one.

**Decision 1's withdrawal of the no-edit rule is accurate, not merely
asserted.** Decision 1 now says "no document in this repo forbids editing an
archived change." `git grep -n -i -F -e "not edited" -e "never edit" -e
"immutable" docs/OPENSPEC-ARCHIVE.md` returns nothing, so the claim holds
against the one doc that would state such a rule if it existed anywhere. The
scope note for this dispatch says the same rule "came from the runner's own
earlier brief, not from any repo document" — consistent with what `git grep`
finds (or rather does not find).

**`design.md`, `proposal.md` and `tasks.md` agree with each other about the
archive edit.** `design.md`'s Context ("Neither is extended with the
reasoning this change carries (Decision 1). The archived design gains only
two labelled forward pointers to corrections made here (Decision 11)"),
`proposal.md`'s Impact ("two labelled notes, one under each of archived
Decisions 3 and 10") and `tasks.md` 1.2 ("Its only edit is the two forward
pointers of 4.5") all describe the same two-note edit and nothing more. None
still claims the archive folder is unedited.

**No decision in this range contradicts the code.** `cargo test` (1180 + 30
passed, 0 failed) and `nix build ./dialectica#lgx` both succeeded on this
tree, and the diff is comments only, matching `proposal.md`'s Impact
statement ("Code: comments only. Wire API and test logic: none.").

No findings. The prior findings this range closes were already correctly
argued and fixed; nothing this range added is undocumented, misdescribed, or
in tension with the issue, the archived design, or the code.
