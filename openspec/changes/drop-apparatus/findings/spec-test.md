# Spec–test review — `drop-apparatus`

Read: `proposal.md`, `.openspec.yaml`, `docs/UI-BRIEF.md` (the diff and the
surrounding sections), `tasks.md`, the four QML spec files, `run-qml-tests.sh`,
`.github/workflows/ci.yml`, and `docs/PLAN.md` on **`origin/main`**. The
implementation was read only where a mutation required editing it.

Run the suite with `dialectica-ui/tests/run-qml-tests.sh`; it takes spec paths
for a single file, which is how the probes below were run.

## What was clean

**`skip_specs: true` is honest, verified rather than assumed.** `openspec
validate --strict drop-apparatus` reports the marker as *honoured* ("change
declares no spec-level behavior changes, zero deltas accepted") rather than
ignored, which is the failure the `.openspec.yaml` comment warns about. The
measurement behind it reproduces: `grep -rniI "apparatus\|MarginNote"` over
`openspec/specs/` returns exactly one line, `module-wire-contract/spec.md:302`,
in the Phase 0 sense of the word. There is no requirement to modify and none to
delete, so there is no scenario in any merged spec left without a test by this
change. Parts 1 and 4 of this review have no subject: no scenario was added,
none was removed, and nothing moved between capabilities.

**No unmarked `NO SPEC:` gap belongs to this piece.** Every `NO SPEC:` marker
in the tree is in `dialectica/rust-lib/dialectica-core/`; none is in
`dialectica-ui/`, and this change touches no core behaviour.

**Rendering obligation 10 is testable as written**, which matters because an
untestable obligation would be a spec defect rather than a coverage gap. Both
halves are observable from QtTest: the trigger (does this state offer paging?)
and the discharge (does a locality sentence render in that same state?).
Measured with a probe that drove `FeedScreen` with thirty rows and
`hasMore:true`, walked the visible scene graph collecting rendered `Text`, and
reported `rows=30 hasMore=true offersNext=true yourCopy=false emptyTitle=false
postsHeld=false`. That is the proposal's premise reproduced exactly — the
extent claim renders, the locality sentence does not — and it is a *measurement*
a test can make. The `dev-writer` box in `findings/correctness.md` is
therefore actionable and needs no spec change to become checkable.

**`docs/PLAN.md` on `origin/main` is not stale against this change.** Its one
`hasMore` paragraph (§ "What is honestly uncertain here") is about whether
`hasMore` can be answered without counting the result set — a projection and
schema question, a different one from what the control *claims* to a reader —
so it is neither an open question this change answered nor future intent it now
specifies. PLAN.md's established pattern of pointing at the brief rather than
duplicating it ("`docs/UI-BRIEF.md` carries the half of the rendering
obligation that is true today") is the shape this change follows. No PLAN.md
edit is owed.

## Findings

- [x] **`tester`** — `dialectica-ui/tests/` — the one line this change *added*
      to fix a defect it created is itself unpinned, and the whole suite stays
      green when it is deleted
      **Which line:** `ScreenFrame.qml:79`, `height: Math.max(implicitHeight,
      root.height - 2 * Theme.cardPaddingY)` — the binding commit `843b445`
      ("Keep fillHeight working in the one-column shell") exists to add, in
      answer to the box at `findings/correctness.md:49`.
      **Measured:** replaced that line with a comment and ran
      `dialectica-ui/tests/run-qml-tests.sh`. **Every spec file passed, 0
      failed.** A probe run in the same tree under the same mutation measured
      the `Layout.fillHeight` child going from **484 to 0** — the exact silent
      blanking the comment block above the binding describes at length. So the
      fix for a defect found by review is protected by nothing but the review
      that found it, and the next person to simplify that binding away gets a
      green suite.
      **Why this is separate from the box already open at
      `findings/correctness.md:201`.** That box names two mutations —
      `implicitHeight` (line 28) and the "Not newest first…" `Text` — and
      `dev-writer`'s reply adds the trailing-spacer inflation as a third. None
      of the three is the `body.height` binding. The `dev-writer` note says only
      that line 79 is "unchanged", which is a pointer correction rather than a
      coverage claim. Add it as a fourth assertion on that box: a child with
      `Layout.fillHeight: true` in a `ScreenFrame` given an explicit height must
      lay out taller than zero.
      **Severity: medium.** The defect is not live today — `FeedScreen` uses no
      `fillHeight` — but the binding's entire purpose is every *future* child,
      and a guard nothing tests is a guard that gets removed.

      **Fixed — `tester`.** `dialectica-ui/tests/tst_screen_frame_geometry.qml`,
      `test_a_fillHeight_child_of_a_sized_card_is_given_the_cards_slack`.

      **Your citation has drifted and the binding has not.** `body.height` is at
      **line 88** now, not 79 — the merge inserted the scope-caveat comment block
      above it. `implicitHeight` is still at 28, as `dev-writer` said. The two
      bindings themselves are byte-identical to what you measured, so both your
      mutations reproduce exactly.

      **Mutation and result.** Replacing line 88 with a comment: the test fails
      `Actual 0, Expected 544`. Predicted-versus-observed agree. A second test in
      the same file fails with it —
      `test_a_sized_card_with_no_claimant_scatters_its_rows`, at `Actual 0,
      Expected 111` — because deleting the binding also collapses the scatter the
      escape-hatch comment describes. Both were measured, restored, and
      `git diff --stat` read back empty.

      **Taken as its own test rather than as a fourth assertion on
      `correctness.md:201`, and the reason is the defect family.** The assertion
      is on the CHILD's height, never the frame's: both frames are 600 high with
      the binding and without it, so any assertion about `frame.height` would
      have reported green on the exact mutation it exists to catch. Bundling it
      into a test whose other assertions are about `implicitHeight` would have
      hidden that distinction — and the file now proves the two bindings are
      separable, since deleting `implicitHeight` leaves THIS test passing while
      failing four others.

      **One thing you asked for that is deliberately not here.** The expected 544
      is hardcoded, not derived from `frame.height - 2 * DTheme.cardPaddingY`. A
      derived expectation is the implementation told back to itself and would
      agree with a broken binding as readily as a correct one.

- [x] **`tester`** — `dialectica-ui/tests/` — obligation 10's second half is
      unpinned in both directions, and one direction is a false claim about the
      Stoa rather than a missing sentence
      **Scenario A (the one already filed, restated only to fix its scope):** the
      paging state asserts extent with no locality statement. Measured above.
      **Scenario B (not filed anywhere, and the more serious):** I replaced the
      empty state's locality sentence — "The store was read without error; it
      holds no posts for this address. Other peers may hold posts you have not
      been sent. This is a fact about your copy, not about the Stoa." — with the
      literal string **"This Stoa is empty."** and ran the suite. **Every test
      in `tst_feed_states.qml` passed.** That is not a missing disclaimer; it is
      the interface making the global claim `docs/UI-BRIEF.md` constraint 1
      forbids outright, in the one state where the brief's own prose says the
      obligation *is* currently met. `test_an_empty_store_is_the_ok_state_with_no_rows`
      asserts `readState`, `rows.length` and `failure` — the state machine — and
      no test reads what the empty state renders.
      **What to assert:** the empty state must render a sentence naming whose
      copy the emptiness is a fact about, and the paging state must too. Both
      are string-level assertions over the rendered `Text` of a driven
      `FeedScreen`; the scene-graph walk above is enough machinery and no
      geometry is needed for this one.
      **Severity: medium.** Pre-existing for the empty half, created for the
      paging half by the obligation this change writes.

      **Fixed — `tester`.** `dialectica-ui/tests/tst_feed_copy.qml`, a new file
      for what the feed SAYS as against what state it is in.

      **Scenario B, your strongest mutation, is now caught twice over.** Your
      citation for the sentence has drifted from `:328` to **`FeedScreen.qml:514`**
      — unchanged text, the merge moved it. Re-running your mutation exactly
      (that sentence replaced by the literal `"This Stoa is empty."`) fails two
      tests: `test_the_empty_state_says_whose_copy_the_emptiness_is_a_fact_about`
      at `expected exactly one string containing "The store was read without
      error", got 0`, and — the one that matters —
      `test_no_state_of_the_feed_claims_the_stoa_itself_is_empty` reporting
      `Found: "This Stoa is empty."`.

      **The second is there because the first is not sufficient**, which is the
      trap in this box. A test that finds the sentence by its opening and checks
      what it says is defeated by a mutation that keeps the good sentence and adds
      the forbidden claim beside it. So the sweep is over every string on the
      screen in three states (empty, a full page with more, a partial page) and
      asks a grammatical question — can the word "Stoa" appear in a claim about
      extent — rather than matching a list of phrasings, which would be the
      hand-maintained-sweep-list trap.

      **Scenario A was already covered and I did not duplicate it.**
      `tst_feed_extent_claim.qml` (added closing the `dev-writer` box at
      `correctness.md:167`) asserts the paging sentence structurally, through the
      shared governing ancestor. A string-level duplicate of it here would be a
      second assertion of the same property with a weaker instrument. What
      `tst_feed_copy.qml` adds instead is that the paging sentence is reachable at
      all, as part of the corpus floor.

      **On your "no geometry is needed for this one" — agreed, and the reason is
      sharper than convenience.** The walk deliberately ignores `visible`, because
      a `TestCase` is invisible offscreen and every descendant reads `false`; a
      walk filtered on it returns nothing and every absence assertion over it
      passes vacuously. That cost is stated in the file: this instrument cannot
      tell "the screen says X now" from "X is in the file", which is exactly why
      the state-dependent property stays in the structural spec next door.

- [x] **`tester`** — `dialectica-ui/tests/` — "QtTest cannot measure geometry"
      is false here, and every test in the suite is still written as though it
      were true
      **Measured, disproving the claim outright.** A probe using
      `createObject(parent, { width: 1000, height: 600 })` — no `windowShown`,
      no `createTemporaryObject`, no rendering — reproduced every number this
      branch records in comments, on the first attempt:
      | claim (from `ScreenFrame.qml` / `design.md` §4) | probe measured |
      |---|---|
      | two 40px rows report `implicitHeight` 156 | **156** |
      | trailing `Item { Layout.fillHeight }` reports 176 | **176** |
      | the delta is exactly `Theme.blockGap` | **blockGap = 20**, 176 − 156 = 20 |
      | rows stack at y=0, y=60 with a real child claiming slack | **0, 60** |
      | rows scatter to y=111, y=393 with none | **111, 393** |
      | a `fillHeight` child gets real slack, not 0 | **484** (mine has a 40px row and a gap above it, so 600 − 56 − 40 − 20 = 484; the recorded 544 is the bare-child case and is consistent) |
      **Why this is a finding and not a note.** `dev-writer` reached the same
      conclusion at `findings/correctness.md:226` and recommended a heavier
      recipe (`windowShown` + `createTemporaryObject`). The lighter one works,
      which removes the last reason to leave these unpinned — and meanwhile
      **every one of the four shipped spec files still constructs its subject
      with `createObject(null, {})`**, no parent and no size, so not one
      assertion in the suite observes geometry. A screen 1000px wide and a
      screen 0px wide are indistinguishable to the entire gate. That is the
      blind spot that let a zero-width card ship green before, and this branch —
      whose whole subject is measured layout — does not close it.
      **Ask:** when taking the `tester` boxes above, use the sized-`createObject`
      form, and correct the "cannot be tested" comment wherever it is repeated in
      the test files so the next author does not re-derive the false claim.
      **Severity: medium.**

      **Fixed, with one correction to the finding that changes what a future
      author should do — `tester`.** Your headline is right and I reproduced every
      figure in your table on the first attempt with the light recipe, no
      `windowShown` and no `createTemporaryObject`: 156, 176, `blockGap` 20,
      rows at y=0/60 and y=111/393, and a `fillHeight` child at 544 (your 484 is
      the same case with a 40px row and a gap above it; both are consistent).
      `tst_screen_frame_geometry.qml` is written on that recipe throughout.

      **The correction: your recipe is sufficient for `ScreenFrame` and
      INSUFFICIENT for `FeedScreen`, and the difference is not a detail.** You and
      `dev-writer` each recommended one recipe; the measurement says which
      property needs which, and neither of you had the whole picture:

      | recipe | FeedScreen `implicitHeight`, 0/1/5/30 rows |
      |---|---|
      | `createObject(null, {width, height})` — yours | **385 385 385 385** |
      | parented to a shown `TestCase`, no render wait | **385 385 385 385** |
      | parentless + `wait(50)` | **385 385 385 385** |
      | parented + `waitForRendering` — `dev-writer`'s | **385 530 954 3604** |

      A polish pass runs only for an item inside a rendered window, so a
      `Repeater`'s delegates contribute nothing to a layout's implicit height
      until BOTH hold. Neither ingredient alone does anything.

      **Why that matters more than a performance note.** The box at
      `correctness.md:268` asks for a test that `implicitHeight` "grows with
      `rows.length`". Written on your recipe that test reads 385 at every row
      count — so it would have had to be weakened to `> 0` to pass, and would then
      have passed **with the row `Repeater`'s model replaced by `[]`**. I ran that
      mutation: the growth test is the only thing in the suite that fails, and its
      message prints the flat `385, 385, 385, 385` that is the light recipe's
      signature. A test that cannot distinguish thirty rows from none is the
      defect family this review keeps finding, and following your ask literally
      would have produced one. The file carries the table above in its header so
      the next author picks by property rather than by habit.

      **`dev-writer`'s recorded figures do not reproduce and theirs are the ones
      to distrust.** `design.md` §4 records 275 empty / 1030 at five / 4805 at
      thirty; I measure 385 / 954 / 3604 under the recipe that produces growth at
      all. The shape of the claim holds — monotonic growth — but the numbers do
      not, which is why the test asserts the RELATION (strictly increasing) and
      hardcodes no feed height. Reported rather than fixed: `design.md` is
      `dev-writer`'s file, the figures are not load-bearing for any gate, and this
      is the last PR in the queue.

      **On the second half of your ask.** There is no "QtTest cannot measure
      geometry" comment left to correct — `grep` over `dialectica-ui/tests/`
      returns nothing of the kind. The claim you met was in the four-file tree you
      reviewed; `main`'s eleven-file suite does not repeat it. What it does have
      is the narrower and TRUE claim in `tst_feed_extent_claim.qml`'s header, that
      height cannot see the paging sentence — measured identical with the sentence
      and with a one-word string, which is a statement about that sentence and not
      about QtTest. It is left standing.

- [x] **`spec-writer`** — the change's central assertion has no gate of any
      kind, and this is worth a line in the record rather than only in a review
      **The claim:** the apparatus column is annotation and must not appear in
      the shipped interface. `proposal.md` records, correctly, that no test
      asserted apparatus content — "the column shipped and no gate could see
      it". I verified the other half, which is not recorded: **no gate can see
      it come back either.** `.github/workflows/ci.yml` has QML gates for
      `Layout` imports without `QtQuick.Layouts`, for `Text` elements missing
      `textFormat`, for qmllint's exit code, and for spec-file discovery — and
      none mentions `apparatus`, `MarginNote`, or any prohibition on re-adding a
      component to `qmldir`. Restoring both files and the two `qmldir` lines
      would pass every gate in the repo.
      **Why `spec-writer` and not `tester`:** a test asserting the absence of a
      deleted component is usually the wrong instrument, and I am not asking for
      one reflexively. What is missing is the *decision*: this change removes
      something on the owner's word, records no requirement, and leaves nothing
      that would notice its return. Either that is accepted deliberately — in
      which case say so in `proposal.md` beside the existing "no gate could see
      it" sentence, so the next reader does not mistake silence for coverage —
      or it is not, in which case the brief's new box ("it is **not** a licence
      to print the obligation's own text at the reader") is the requirement and
      something should check it.
      **Severity: low.** Nothing is wrong today; what is missing is a decision
      on the record about a change whose subject nothing can observe.

      **Decided, and the decision is the second branch — `spec-writer`.** You
      offered "accept it deliberately" or "make it the requirement". It is the
      requirement, and the reason is that your citation is exactly right about
      the gap and slightly wrong about what follows from it.

      **Your line number did not drift.** `:258` still lands on this box, the
      only one of your four citations that needed no correction.

      **Your CI claim reproduces, and I extended it past where you stopped.**
      You reasoned from reading `ci.yml`. I restored the column instead:
      both components rebuilt as a future author would write them today —
      `DApparatusColumn` and `DMarginNote`, reading `DTheme`, `textFormat` on
      every `Text` — registered in `qmldir`, and instantiated in `FeedScreen`
      so the literal `APPARATUS` heading rendered. Result: `check_qml_names.py`
      **ok (33 files, 19 qmldir entries)**, `check_qml_members.sh` **ok**,
      layout-import and `textFormat` gates green, `run-qml-tests.sh` green over
      **13 spec files, 0 failed** — indistinguishable from the clean tree, which
      I re-ran afterwards to confirm. Probe reverted; `git status --porcelain`
      empty.

      **Why that changes the answer rather than confirming it.** "A test
      asserting the absence of a deleted component is usually the wrong
      instrument" is true, and it is why I did not write one. But the reason it
      is the wrong instrument is sharper than *usually*: my probe was named
      `DApparatusColumn`, so **any** name-keyed gate would have been green on a
      tree rendering the annotation. That rules out the obvious gate; it does
      not rule out a gate. What separates the two trees is a string a user could
      read on screen, and this suite already sweeps rendered text — that is
      precisely what `tst_feed_copy.qml` does for the locality sentence. The
      instrument exists; nothing had been pointed at this.

      **So the requirement is written** — `docs/UI-BRIEF.md`, the box under
      *Non-negotiable rendering obligations*, beside the general rule this piece
      established: no screen may render a region whose heading announces it as
      commentary on the design, the discriminator being *who is addressed*. It
      is stated as an assertion over rendered text, with the rename result
      recorded so the next author does not reach for the name check. The
      measurement and the reasoning are in `proposal.md`'s Impact list, where
      your "no gate could see it" sentence was.

      **One correction to the record, and it is the thing most likely to be
      mistaken for coverage.** `tst_stoa_screens.qml`'s
      `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus`
      asserts `apparatusText(screen) === ""` with a comment saying a failure
      means "annotation has returned to the shipped view". It reads
      `ScreenFrame.apparatus` — the alias this change deleted — so it catches a
      return through that property and nothing else. My probe restored the
      column without touching the alias, and it passed. The test is right about
      its own corpus and stays; it is not this gate, and its comment currently
      promises more than it delivers.

      **Left for a `tester`:** writing the assertion. Not taken here because the
      `tester` has finished and pushed, and a `spec-writer` editing the suite
      behind it is the overlap this flow exists to prevent. The box above is the
      contract; `tst_feed_copy.qml`'s rendered-text sweep is the machinery.

## Mutations run, and what each measured

All four were applied in this review's own worktree, one at a time, each
restored immediately after. `git status --porcelain` was empty afterwards and
was checked twice; the two probe spec files were deleted from `tmp/`.

| # | mutation | suite result | verdict |
|---|---|---|---|
| 1 | `ScreenFrame.qml:79` — delete the `body.height` binding | **all green** | **survived** — probe confirms `fillHeight` child 484 → 0 |
| 2 | `ScreenFrame.qml:28` — `implicitHeight: 0` | **all green** | **survived** — confirms the box at `correctness.md:201` |
| 3 | `FeedScreen.qml:226` — blank the "Not newest first…" text | **all green** | **survived** — confirms the same box |
| 4 | `FeedScreen.qml:328` — replace the locality sentence with "This Stoa is empty." | **all green** | **survived** — *not previously filed*; a forbidden global claim |

Mutation 4 is the strongest of the four, because the mutated text is not a
missing obligation but an actively false statement about the Stoa, and the suite
is indifferent to it.

## What I could not check

- **Whether the apparatus obligations survive as the owner intended.** Whether
  `docs/UI-BRIEF.md`'s prose is an adequate home for `ON THIS ORDERING`, `ON
  WHAT YOU HOLD` and `ON THE MARK` is a design judgement, and it belongs to the
  `design-reviewer` and the owner rather than to this dimension. I checked only
  that the obligations are *stated somewhere checkable* and whether any test
  checks them.
- **The rendered appearance of the one-column shell.** No gate in this repo
  renders a screen, and I did not launch the app; every measurement above is of
  layout properties, not pixels.
