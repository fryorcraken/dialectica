# Design review — unauthorised-rules

Verified against `.claude/agents/README.md`, `CLAUDE.md`, `design.md` (D1-D6),
`proposal.md`, the code diff, `openspec/specs/composer-view/spec.md`,
`openspec/specs/generated-names/spec.md`, `openspec/specs/op-format/spec.md`,
and `docs/PLAN.md` §10 ("CI") on `origin/main`.

## Summary

This change is in good shape. Every Decisions entry (D1-D6) was checked
against the actual code and against the sources it cites, and every one
holds:

- **D1 (demote, not delete).** Confirmed at all three named sites.
  `DComposer.qml:28-34` keeps the measured defect (`parentOp: "deadbeef"`
  silently dropped, no warning/refusal/test) and drops only the
  comment-writing rule. `check_qml_members.sh:156-166` keeps the `-W 0`
  trade-off and its cost, rephrased as "weigh that cost before doing it"
  rather than "never... raise the `-W` ceiling". `tst_stoa_screens.qml`'s
  header keeps the mutation-testing finding (the per-Stoa-identity absence
  assertion caught by planting text in the joined panel) with no remaining
  general rule bound on future absence assertions.
- **D2 (FeedScreen.qml).** `FeedScreen.qml:342-349` keeps "one ordering ...
  nothing to select ... `reload()` does not read `ordering`" and no longer
  contains "worse than absent" or "reads as a working control." Verified the
  cited spec text is real: `openspec/specs/composer-view/spec.md:639,666`
  contains the inert-affordance requirement and the "general and SHALL NOT be
  read as being about any one field" sentence verbatim.
- **D3 (VoteControl.qml left alone).** Confirmed untouched, and
  `openspec/specs/composer-view/spec.md:581` ("The vote control displays no
  score") and the "worse than printing nothing" comment at
  `VoteControl.qml:20-21` are both still present, matching the ratification
  claim.
- **D4 (DIdentityChip.qml split).** Confirmed the file now states a
  GROUNDED half (citing `docs/IDENTICON.md` and `generated-names`, verified
  present at `openspec/specs/generated-names/spec.md:507`) and a NOT GROUNDED
  half (citing the dangling `SPEC.md` and `op-format`'s Stoa-scoped sentence,
  verified at `openspec/specs/op-format/spec.md:494-496`, including the
  spec's own note that the sentence was scoped "because it is the one a later
  reader would cite out of context as authority for an author address").
  Confirmed no `SPEC.md` file exists anywhere in this worktree.
- **D5 (the four "worse than no gate" sites, plus run-qml-tests.sh checked
  and left alone).** All four edited sites (`check_qml_members.sh:29-31`,
  `tst_check_qml_members.sh:210-214`, `check_qml_names.py:36-40`,
  `tst_check_qml_names.py:281-283`) now attribute the observation to
  `PLAN.md §10 (CI)` / Radicle's CI and use "copy the habit" phrasing.
  Verified against `docs/PLAN.md:3627` (section literally titled "## 10.
  CI") and `docs/PLAN.md:3667-3672` ("Anti-false-green... Radicle's CI is
  built around the observation that a green gate which cannot see the thing
  it claims to check is worse than no gate. Copy the habits..."). The
  `§10 ("Anti-false-green")` mis-citation D5 reports as its own mid-flight
  error is not present anywhere in the current tree (correctly fixed) and
  is recorded as this piece's own failure mode in `design.md` — see the
  "own failure mode" checkpoint below. `run-qml-tests.sh:19` still reads "...
  which PLAN.md §10 calls worse than no gate at all" — accurate, unedited,
  as D5 claims.
- **D6 (green/orange).** Confirmed `DTheme.qml:65-69` and
  `DStatusBar.qml:13-22` both now say "currently"/"as things stand"/"at
  present" rather than a standing prohibition, and both retain the reasoning
  for why the exclusivity carries meaning (the lamp's colour half of its
  signal) and the `markGreen`/`markSage` carve-out. This is a sound
  application of the demote-not-delete rule to a claim design.md itself says
  is contradicted by the archive
  (`2026-09-17-ui-shell-components/design.md`: nothing executable reserves
  green/orange, no gate exists). The self-invalidating framing ("a reader can
  check it, and it fails visibly if a fourth colour lands") is the right
  target for a CLAUDE.md-style present-fact claim, and the escalation path
  the writer flags (gate + spec requirement, if the owner meant a real
  constraint) is correctly left as a recommendation rather than acted on —
  this piece has no authority to add a gate on the owner's behalf.

## The structural finding is recorded, not just reported

Verified: `design.md`'s "The structural observation" section (lines 11-44)
states the "rhetorically complete without being evidentially complete"
finding explicitly, gives the three supporting measurements (typographic
identity between `VoteControl.qml:20` and the deleted `FeedScreen.qml`
sentence; the PLAN.md §10 attribution drift into "which this repo rates";
the `SPEC.md` chain), and states plainly why the fix is prose and not a
gate ("No check can read English for whether a sentence claims authority it
lacks"). This is the durable reasoning the review brief asked to confirm
landed — it did, in `design.md` rather than only in an ephemeral report.

## The demote-vs-delete test is stated, and self-applying

`proposal.md`'s "What is deliberately NOT changed" section states the test
directly: "whether a sentence constrains future work beyond its own site
without recorded authority — not whether it sounds opinionated." `design.md`
restates it per-decision. A later reader has an explicit rule to check new
edits against, not just eight worked examples to induct from.

## No findings requiring a box

Nothing in D1-D6 contradicts the code, no site was left with a stray
authority-claiming clause, and no genuinely load-bearing reasoning was
deleted rather than demoted. The one near-defensible worry — that
`design.md`'s "a missing source survives being cited" account
(`docs/IDENTICON.md:21` → `DIdentityChip.qml`) could read as claiming the
`SPEC.md` problem is isolated to that one chain, when a plain search shows
`SPEC.md` is cited live as authority in well over a dozen other files
(`DVouchStamp.qml`, `SanitisedText.qml`, `DStatusBar.qml`, `Core.qml`,
several `tst_*.qml` files, `docs/IDENTICON.md` itself) — does not rise to a
finding: `design.md` presents it as one of three illustrative "consequences
visible in this tree," not as an exhaustive audit, and neither `proposal.md`
nor `design.md` claims this piece fixes every `SPEC.md` citation — only the
ones that generalise into an unauthorised binding rule, which is the
pattern actually in scope. Recorded here as a note for whoever scopes the
next sweep, not as a blocking item.
