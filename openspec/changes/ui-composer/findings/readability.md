# Readability — `ui-composer`

Reviewed on `piece/ui-composer` at `7dccbdc`, in a worktree of its own. The
whole QML suite was run before and after every mutation
(`dialectica-ui/tests/run-qml-tests.sh`): **118 tests across 8 spec files, all
green on the unmutated tree.** Every mutation below was verified to land, and
every one was reverted; `git status --porcelain` is empty.

`qmllint -I dialectica-ui/src/qml` over the five changed QML files exits 0 with
no output, which is CI's own invocation (`ci.yml:535-569`, no `--bare`).
`qmlformat -n` parses `Composer.qml` cleanly, which is the gate at `ci.yml:452`.
I checked the `textFormat` counting gate (`ci.yml:262-274`) by hand on all four
changed view files rather than trusting it: `PublishOutcome` 5/5, `VoteControl`
3/3, `FeedScreen` 15/15, `Composer` 3 openings against 4 assignments — the
`TextEdit {` is not matched by `\bText \{` and carries its own assignment, so
that file passes with one to spare rather than exactly.

Correctness and security findings were read first; nothing below repeats one.

## Findings

- [ ] **`dev-writer`** — `PublishOutcome.qml:55-59` vs `35` — the headline and
      `isRefusal` read an unrecognised `outcome` in **opposite directions**, so
      an unknown value renders a refusal headline on top of both success
      sentences
      **Scenario:** the headline is a ternary whose `else` arm is the refusal
      wording, so anything that is not `"stored"` or `"existing"` prints *"Your
      post was not published."* `isRefusal` is `outcome === "refused"`, so the
      same unknown value is **not** a refusal — and the two `!isRefusal`
      elements below it both render. Measured against the real component with a
      scratch spec file (written, run, deleted; not part of this branch), for
      `outcome` set to `"deferred"`, `"Refused"`, `"REFUSED"`, `"refused "` and
      `"stored "`, every one renders the identical three lines:

          Your post was not published.
          It is in this machine's log.
          Whether any other peer has received it is not something this software can tell you yet.

      A screen that says the post was not published and that it is in this
      machine's log is claiming both things at once, which is precisely what
      "one variable cannot hold two values" was adopted to make unrepresentable.
      Worse in the same breath: `detail` was set to `"core said no"` on every
      one of those cases and **does not appear**, because its `Text` is gated on
      `isRefusal` — so an unknown outcome silently swallows core's message while
      announcing a failure.
      The file's own header comment (lines 4-19) enumerates four values and
      reads as a closed set. It is not one: three of the five elements treat the
      set as closed by testing for the two successes, and two treat it as closed
      by testing for the one refusal, and those are different partitions of the
      same space. **Severity: medium** as a defect today — `Composer.applyReply`
      only ever writes the four documented strings, so nothing reaches this on
      the current tree — but the component is a registered, publicly-propertied
      QML type and this is exactly the shape the design document claims it does
      not have.
      The legible fix is one line: make the qualifier and the denial key on the
      two values that earn them (`outcome === "stored" || outcome === "existing"`)
      rather than on the negation of a third, so both halves partition the space
      the same way and an unknown value renders the refusal wording alone.

- [ ] **`tester`** — `tst_composer_claims.qml:752-794` — `pinnedSentences()` has
      one row per **known** outcome and nothing pins what an unknown one renders,
      so the contradiction above is invisible to the suite
      **Scenario:** the three-row table is iterated by
      `test_the_views_own_words_are_exactly_these_and_no_others`, and
      `test_an_unsubmitted_composer_displays_no_outcome_at_all` covers `""`. No
      test anywhere sets `outcome` to a value outside those four. **Measured:** I
      added a fourth outcome to `Composer.applyReply` — a `wasNew === "deferred"`
      arm setting `outcome = "deferred"` and calling `published()` — and re-ran
      the whole suite: **118 of 118 passed**, including
      `test_no_two_outcomes_display_the_same_message`,
      `test_the_three_outcomes_are_mutually_distinguishable` and
      `test_each_outcome_implies_what_it_must_and_denies_what_it_must_not`. A
      developer adding a fourth outcome gets a green suite and a screen that
      contradicts itself. The test this wants is a totality case: an outcome the
      component does not know must render the refusal wording and nothing from
      either success. **Severity: medium.** It is the measuring-instrument
      question asked of the claims table rather than of a helper — the table
      cannot report a row it does not have.

- [ ] **`dev-writer`** — `FeedScreen.qml:663` — the guard on
      `capability.reason` is live for a reason its comment does not give, and
      the comment beside it describes the opposite path
      **Scenario:** the `!== undefined` test reads as defending the closed
      branch, and against that branch it is dead: lines 185-195 construct
      `reason` as a string on every closed path (`probe.error` is a string from
      `call()`, and the other arm is `typeof`-checked). What it actually
      defends is the **open** branch, where line 184 assigns `probe.value`
      wholesale and a probe answering `{"canPost":true}` with no `reason` leaves
      the property `undefined` — and QML evaluates this binding even while the
      closed `ColumnLayout` is invisible. **Measured:** replacing it with a bare
      `text: screen.capability.reason` and running `tst_vote_and_gate.qml` emits
      `FeedScreen.qml:663: Unable to assign [undefined] to QString` on fourteen
      tests, all of them **open**-gate cases driving the
      `'{"canPost":true,"identity":"aa"}'` fixture; 29 of 29 still pass, so
      nothing fails, but the runtime says plainly which branch the guard is for.
      A reader following the comments concludes this guard is redundant and
      deletes it. **Severity: low** — a comment defect rather than a behaviour
      one, and the finding is that the "why" a reader would ask for is the one
      thing not written down.

- [ ] **`dev-writer`** — `Composer.qml:17` — "a post must not have one" is
      stated as an invariant and nothing establishes it
      **Scenario:** the comment on `kind` reads *"A reply needs `parentOp`; a
      post must not have one"*, but `parentOp` is a plain settable property and
      `submit()` reads it only on the reply arm. `Composer { kind: "post";
      parentOp: "deadbeef…" }` is accepted and silently drops the parent — no
      warning, no refusal, no test. The sentence is the file's only statement of
      the rule and it reads as a constraint the component enforces.
      **Severity: low.** It cannot produce a wrong publish today (the `kind`
      ternary decides the method, so a stray `parentOp` reaches nothing), which
      is why this is a legibility finding rather than a correctness one: the
      comment should say what is true — that `parentOp` is read only when `kind`
      is `"reply"` and is otherwise ignored — or the component should make an
      ignored parent impossible.

## What is clean

**The honesty obligations read as obligations.** This is the thing the piece is
for and it is the strongest part of it. `PublishOutcome.qml:90-119` spends
thirty lines saying why the denial is its own element and not a clause — that
hanging a requirement off one arm of a conditional is how it goes missing from
the other — before the eight lines that do it, and a reader who moves that
sentence back into the ternary has been told exactly what they are undoing. The
same shape recurs at `FeedScreen.qml:672-686` for the missing-box statement and
at `733-742` for the apparatus list, and in all three the comment earns its
place by saying what the code cannot: not what this does, but which plausible
alternative was rejected and what it cost when it was tried. The denial *is*
inherited by an outcome nobody has written yet — I confirmed that directly: my
fourth outcome rendered it. The finding above is that the **headline** is not.

**The author-facing/reader-facing asymmetry is legible where a reader meets it.**
`Composer.qml:123-142` states the rule as a rule — the reader cannot consent to
what they are shown so peer text is sanitised, the author can and is the only
person who can still change the text, so theirs is not — and only then says why
the count stops at removals. The `NO SPEC` marker is on the gap rather than on
the mechanism, and the argument for it (a membership test drifts visibly, a
homoglyph judgement drifts silently and nothing computes the two counts over the
same string) is the sort of reasoning a command cannot answer. That the two
sanitiser character sets are identical is measured in
`test_the_invisible_set_matches_the_ranges_core_removes` rather than asserted.

**`showScore`'s intent is recorded in all three places a binder would look.**
`VoteControl.qml:14-31` gives the reasoning at the property; `design.md:168-191`
lists the three rejected alternatives including the one that reads as though it
works (`score: -1`, where `Math.max(0, -1)` is `0`); and `UI-BRIEF.md:802-819`
tells the external designer the same thing and adds that the reference mockup's
score of 12 is not a target. The `height: 0` beside `visible` at
`VoteControl.qml:56-57` is the kind of detail that usually goes unexplained and
is explained: an invisible Item in a `Column` still occupies its row, leaving a
gap that reads as a number that failed to load.

**The measuring instruments are guarded, and the guards were earned.** I asked
of each helper what it would report if it were broken.
`test_the_sweep_filter_drops_only_the_pinned_denial` pins `stripPinnedDenials`
at both bounds over literals *and* under both case foldings;
`test_the_sweep_corpus_keeps_everything_but_the_denial` then pins the corpus the
sweeps actually call, against a real component, which is the gap the correctness
reviewer found by mutating `sweepCorpus` to `""` and watching thirteen tests
pass. `test_the_apparatus_walker_actually_excludes_the_column` pins the walker
in both directions — it must trim the column's heading and must leave the gate's
heading and core's reason — which is the mirror of the over-exclusion that
actually happened when the first walker matched `ColumnLayout`'s `content`.

**The duplicated denial is the right call, not a defect, and the prompt's
framing of it is slightly off.** The sentence lives in **two** helpers, one per
spec file (`tst_composer.qml:337`, `tst_composer_claims.qml:747`), because
QtTest spec files are separate QML documents sharing no scope — a single helper
is not available. `test_the_pinned_denial_is_spelled_the_same_way_in_both_files`
closes that by pinning this file's copy against what the component renders,
while the other file pins its copy as a literal: two independent checks on one
sentence, so a drift in either copy or in the component fails something. Both
helpers carry a comment forbidding the update-to-match-the-component move and
distinguishing it from the legitimate case where the spec moved first.

## Not findings, recorded so nobody re-opens them

`Composer.qml:169-178` splitting `submit()` from `applyReply()` is one function
one job done right, and the comment says which job each has. The `NO SPEC` at
`209-218` states both sides of the clear-the-draft argument and names which is a
wrong state and which is a convenience, which is more than the spec asks for.
`Core.qml:98-111`'s note on the three publish wrappers correctly refuses to
interpret `wasNew` at the seam.

`otherKnownDenials()` (`tst_composer_claims.qml:211-213`) will become a dead
entry when `piece/drop-apparatus` deletes the `ON PUBLISHING` note — a
`split`/`join` on an absent string is a no-op, so nothing breaks, and the
helper's own comment already says it is listed this way *because* the sentence
is on its way out. That is a note for whoever lands `drop-apparatus`, not a box
against this piece.
