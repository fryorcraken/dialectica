# Spec/test review — `ui-composer`

Read: `openspec/changes/ui-composer/specs/composer-view/spec.md` and the eight
QML spec files under `dialectica-ui/tests/`. The implementation was not read
except for the lines mutated in part 2, per the role's one exception.

Baseline before any mutation: **124 passed, 0 failed** across 8 spec files
(`run-qml-tests.sh`).

## Mutations run, and what each measured

Seven mutations, each applied alone, the tree restored between each and
`git status` clean at the end. Six of the seven **survived**.

| # | Mutation | Result |
|---|---|---|
| A | `Composer.qml` — delete `root.clearDraft()` from the `wasNew === true` arm | **survived, 124/124** |
| B | `Composer.qml` — add `root.clearDraft()` to the `wasNew === false` arm | **survived, 124/124** |
| C | `FeedScreen.qml:659` — `onPublished: screen.reload()` → no-op | **survived, 124/124** |
| D | `Composer.qml` — invisible-character warning `visible:` → `false` | **survived** |
| E | `Composer.qml` — over-limit warning `visible:` → `false` | **survived** |
| F | `FeedScreen.qml:517` — `vote:` binding → constant `0` | **survived** |
| G | `PublishOutcome.qml:70` — total `state` reverted to `root.outcome` | **caught** (2 fail) |

## Findings

- [ ] **`tester`** — the requirement *"The draft is cleared when the op was newly
      stored, and kept otherwise"* (spec.md:336-372) has **no test at all**, in
      either direction
      **Scenario:** the requirement names the three-way asymmetry as "the decision
      rather than an inconsistency" and spends two scenarios on it, including
      *"The draft's fate differs across the three outcomes"*. No test anywhere
      asserts `draft === ""` after a `wasNew:true` publish, and none asserts the
      draft is retained after a `wasNew:false` one — `grep` for `.draft` across
      `tests/` returns only byte-count assertions and refusal-retention ones.
      **Measured (A):** deleted `root.clearDraft()` from the stored arm —
      **124 of 124 passed**. **Measured (B):** added `root.clearDraft()` to the
      `existing` arm, which is precisely what the requirement's rationale forbids
      ("clearing would take away exactly what they need") — **124 of 124
      passed**. Both halves of the asymmetry can be inverted with a green suite.
      **Severity: high** — this is a whole requirement with zero coverage, and
      the failure mode is silent data loss for the user.

- [ ] **`tester`** — the two pre-submission warnings are pinned by the
      **computation feeding the binding**, never by the rendered property
      **Scenario:** spec.md:211 requires *"the view **displays** a warning naming
      how many were found"* and spec.md:244-245 requires *"the view **reports**
      that it is too long"*. `test_invisible_characters_are_counted_and_do_not_block_submission`
      (tst_composer.qml:187) asserts `c.invisibleCount === 3` and `submittable`;
      `test_an_over_length_draft_blocks_submission_before_any_call`
      (tst_composer.qml:113) asserts `c.overLimit` and `c.submittable`. Neither
      reads the rendered text. **Measured (D, E):** set both warnings'
      `visible:` to `false` — every test still passed. A user pasting text with
      bidirectional overrides gets no warning, and a user over the cap gets a
      greyed-out button with no explanation, with the suite green. This is the
      sibling-piece defect (`visible:` mutated, 47 tests green) reproduced on
      this piece, and the tests' own header comments warn about it.
      **Severity: high.** The fix is one assertion per warning against
      `renderedText(c)`, which both files already have a helper for.

- [ ] **`tester`** — `test_a_published_vote_is_reflected_on_that_posts_control_only`
      (tst_vote_and_gate.qml:179) asserts the map, not the control its name
      promises
      **Scenario:** spec.md:669-672 says *"**THEN** the control for that post
      shows that direction as the viewer's own"*. The test body reads only
      `screen.ownVotes["v1"]` — the state feeding the control's binding.
      **Measured (F):** replaced the `VoteControl`'s `vote:` binding with the
      constant `0`, so no control ever shows a vote back — **all 31 tests in
      that file passed**, this one included. The requirement's stated purpose
      ("the one thing about a vote that is true and immediate: the user pressed
      a button and the interface remembers") is entirely unpinned at the control.
      The same gap covers the scenarios *"A refused vote leaves the control
      unchanged"* and *"A post with no recorded vote renders neutrally"*, both of
      which are also asserted only against `ownVotes`. **Severity: medium** —
      no false claim results, but the affordance silently stops working.

- [ ] **`tester`** — nothing pins that a successful publish triggers the re-read;
      the signal's **emitter** is tested and its **receiver** is not
      **Scenario:** spec.md:501 requires *"After a successful publish the view
      SHALL re-read from core."*
      `test_the_published_signal_fires_on_a_success_and_not_on_a_refusal`
      (tst_composer.qml:618) pins that `Composer` emits `published`.
      `test_a_publish_adds_no_row_the_view_composed` (tst_vote_and_gate.qml:877)
      calls `screen.reload()` **itself** and then asserts the row count, so it
      tests `reload()` rather than the wiring that invokes it.
      **Measured (C):** replaced `onPublished: screen.reload()` with a no-op —
      **124 of 124 passed**. A published post then never appears until the user
      reloads by hand, and the suite reports nothing. **Severity: medium.**

- [ ] **`spec-writer`** — spec.md:208-212 and spec.md:234 contradict each other
      for a draft that is both over-length and carries invisible characters
      **Scenario:** the scenario *"An author is warned about invisible characters
      before submitting"* has the unconditioned **WHEN** *"a draft contains
      characters the display sanitiser would remove"* and asserts **"the submit
      affordance remains available"**. The body-limit requirement says the view
      *"SHALL keep the submit affordance unavailable while it [exceeds the
      limit]"*. One draft satisfies both antecedents, and the two consequents are
      contradictory. `openspec validate --strict` checks heading structure only
      and passes this. The fix is to make the invisible-warning scenario's
      availability claim relative — the warning is not itself a gate — rather
      than absolute. **Severity: medium.** A weaker sibling of the same shape sits
      at spec.md:240 vs spec.md:427 (a core refusal for length routed into "a
      refusal like any other", whose scenario requires the submit affordance to
      remain available).

- [ ] **`spec-writer`** — the `NO SPEC` marker at `Composer.qml:231-240` is now
      stale and should be removed, because the spec has since answered it
      **Scenario:** the marker reads *"the spec says what happens to a draft on a
      REFUSAL (it survives) and says nothing about one on a success"*. The spec
      now carries the full requirement *"The draft is cleared when the op was
      newly stored, and kept otherwise"* (spec.md:336-372), including the
      argument the marker rehearses. Leaving it says a decision is unmade when it
      is made and contracted, which is how the next reader concludes it is free
      to change. The second marker at `Composer.qml:157` (the homoglyph count) is
      **still accurate** — spec.md:178-199 names that gap deliberately — and
      should stay. **Severity: low**, but it is the marker's whole job to be
      trustworthy.

## What was clean

**The claims table is the strongest part of this suite**, and it does what its
header says: `test_each_outcome_implies_what_it_must_and_denies_what_it_must_not`
and `test_the_views_own_words_are_exactly_these_and_no_others` pin *meaning* and
then pin *exact text plus an empty residue*, so a reword that loses a required
meaning fails and a sentence added later fails too. That residue check is what
the sibling `thread-read` piece lacked.

**The open `tester` box in `findings/readability.md:91` is genuinely closed** —
I did not take the dev-writer's word for it. **Measured (G):** reverted
`PublishOutcome.state` to a bare alias of `outcome`, and both
`test_an_outcome_the_component_does_not_know_renders_as_a_refusal` and
`test_an_unknown_outcome_is_indistinguishable_from_a_refusal` fail, with the
output reproducing the three-line contradiction the architecture review
described verbatim ("Your post was not published." above "It is in this
machine's log."). The totality is pinned by the tests, not merely present in the
code. Whoever owns that box can tick it.

**Helpers and their callers are pinned**, which was the specific worry.
`stripPinnedDenials` is pinned on both bounds over literals
(`test_the_sweep_filter_drops_only_the_pinned_denial`) **and** its caller
`sweepCorpus` is pinned against a real component on both bounds
(`test_the_sweep_corpus_keeps_everything_but_the_denial`) — the second of those
exists because mutating the caller to return `""` passed all thirteen tests
including the first, which the file records. `renderedTextOutsideApparatus` has
`test_the_apparatus_walker_actually_excludes_the_column` asserting it trims
something *and* keeps the gate body. `renderedText` and `nonPlainTextElements`
have no guard of their own, but both are held by presence assertions elsewhere
(`mustSayOneOf`, the pinned-sentence residue), so a helper returning empty fails
before it can disarm anything.

**The corpus hole is defensible.** `sweepCorpus` removes the pinned denial from
what the delivery sweeps search, which does carve a hole by construction — but
the removed sentence is pinned character-for-character by
`test_the_views_own_words_are_exactly_these_and_no_others`, whose residue check
fails on *any* change to the outcome block, so a delivery claim smuggled into
the denial cannot survive. The removal is of the sentence and not of its
surrounding region, and the test proves a claim planted beside the denial still
reaches the sweep. That holds.

The gate coverage is thorough in a way worth naming: `tst_gate_affordance.qml`
probes by writing into every candidate rather than recognising a property name,
counts read-only inputs separately from editable ones, and
`test_the_box_arrives_and_leaves_with_the_probe_answer` re-probes one screen
across three reloads — which is the only test that can see a cached probe
answer.

## What I could not check

QtTest drives properties and signals, never pixels. Nothing in this repo can
check that a warning is legible, that the three outcomes are visually distinct,
that a box is not covered by an opaque sibling, or that any of this fits on a
screen — the test files say so themselves and I confirmed no mechanism exists.
The delivery sweeps remain absence assertions over a fixed needle list: a
delivery claim phrased in words nobody listed passes, and the real defect
underneath (a maximal legal post silently refused by every receiving peer)
cannot be seen by any test here, because nothing in this repo has a second peer.

Two scenarios are worth a spec-writer's eye but I am not opening boxes for them,
since each is currently satisfied by construction: *"That statement survives
without the annotation column"* (spec.md:151-155) and *"The denial survives
without the annotation column"* (spec.md:313-317) both turn on "every region
given over to annotating the design", which is a judgement about layout intent
rather than a queryable property. The first is testable today only because the
column happens to render a literal `APPARATUS` heading the walker can match; the
second is met because `PublishOutcome` is driven standalone. Both become
untestable the moment that heading changes.
