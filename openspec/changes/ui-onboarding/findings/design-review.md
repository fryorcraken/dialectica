# Design review — `ui-onboarding`

Checked `design.md`'s Decisions against the code, and the code against
`docs/PLAN.md` as it stands on `origin/main` (`468e716`).

**The recorded decisions are, with one exception, taken by the code, and the
prose survives verification.** I re-ran every figure and citation rather than
reading them: `wire.rs:656` and `wire.rs:3075` are where design.md says they
are and say what it says; the count history ("three, then four merged in #22,
now three again on a different basis — adjective + noun + 'of' + place, on
`docs/name-shape-sweep`") matches `d3e7579`'s own message verbatim; the suite
is 102 tests (13+12+7+12+49+9, run); and the rewritten guard justification —
"Deleting the surviving guard now fails 2 tests" — I verified by mutation, and
it fails exactly 2, `test_keeping_is_refused_by_the_view_while_nothing_is_selected`
and `test_the_keep_guard_refuses_at_the_sentinel_even_when_a_row_carries_it`.
That was the claim most likely to be stale, since it was rewritten after the
guard collapse, and it is true.

The one serious finding is the apparatus rule: the general rule is stated in
the right places, but the **test that enforces it does the opposite of what its
name promises**, and it is the only durable enforcement once `design.md` is
archived.

## The code contradicts a recorded decision

- [ ] **`tester`** — `tst_onboarding_states.qml:1085` — the test named
      `test_the_uniqueness_obligation_survives_without_the_apparatus_column`
      **fails in exactly the scenario it is named for**, so it blocks the
      removal rather than surviving it
      `design.md:204` cites this test as what makes the body copy
      undroppable: *"requires two elements to carry it, so the body copy cannot
      be dropped silently."* But the assertion is
      `compare(carriers, 2, …)` — **exactly two**, not at least one in the
      body. `piece/drop-apparatus` (#70) removes `ApparatusColumn` and
      `MarginNote`, which leaves one carrier, and this test then fails.
      **Verified by mutation:** I replaced only the `ON UNIQUENESS` margin
      note's body (leaving the body-copy `Text` at `OnboardingScreen.qml:534`
      untouched) — the exact shape `drop-apparatus` produces — and got
      `FAIL! … Actual (): 1  Expected (): 2`. The suite is otherwise green;
      restored, and `git status` is clean.
      This inverts the decision. `design.md:207-210` states the rule as
      *"apparatus may repeat an obligation, never carry it alone"* — repetition
      is permitted, not required. The test requires it. A `drop-apparatus`
      author meets a red `compare(carriers, 2)` whose message says the
      obligation *"must be stated in the body as well as the margin"*, and the
      cheapest green is to edit the `2`, which is a count-pin on a decoration
      that is being deleted — the same failure mode this change's own word-count
      decision (`design.md:244-252`) argues against, reproduced one file away.
      What the rule actually wants asserted is that the **body** carries it:
      that at least one carrier is outside `apparatus`. That assertion passes
      today, passes after the column goes, and fails only if the body copy is
      dropped — which is the defect. As written the test cannot tell the
      obligation being deleted from the decoration being deleted.

## Decisions taken but not recorded

- [x] **`dev-writer`** — `design.md` has no entry for the
      `stoaAddress === ""` guard, now the **third** copy of the same check
      `Main.qml:78` (`identityState = "failed"`),
      `OnboardingScreen.qml:98` (`enterFailed(...)`) and the pre-existing
      `FeedScreen.qml:72` all carry the identical string *"No Stoa address was
      given to this view."* Following `FeedScreen`'s precedent is defensible,
      but it is a choice with real alternatives that a reader would plausibly
      have made differently — one helper, or `Main` declining to instantiate
      children at all — and CLAUDE.md names the fourth slightly-different copy
      of a guard as the signal to reshape. Three is where that conversation
      should be recorded, not had again silently at four.
      There is a consequence worth stating with it: `Main.qml:26` keeps
      `stoaAddress: ""` as the honest default, so a **fresh launch with no Stoa
      supplied shows the `"failed"` screen**, not onboarding. That is arguably
      right — the view does not know whether an identity exists — but it is a
      product-visible branch decided by a guard `design.md` does not mention,
      and the entry should say why `"failed"` rather than `"absent"` and what
      it costs (a developer-run build shows a failure card before it shows
      anything else).
      **Fixed** in `56905f4` — `design.md`, "The empty-`stoaAddress` guard is
      now the third copy, and stays one more time". Verified your count before
      writing it: `grep` over `dialectica-ui/src/qml/` returns exactly the three
      sites you name, all carrying the identical string.
      The entry records both alternatives and why neither is taken **now**. A
      shared helper removes the string duplication and not the check — each
      caller still has to remember to call it, which is the half that goes
      wrong. `Main` declining to instantiate children is the right structural
      fix, and it belongs to the change that settles where a Stoa comes from:
      `piece/ui-stoa-list` removes `stoaAddress` from `Main` outright, so
      reshaping now would be reshaping around a design being replaced. The entry
      says explicitly that whoever reconciles the two should collapse them,
      which is where "no Stoa chosen yet" stops being an error string three
      files repeat and becomes a navigator state.
      Your consequence is recorded as you framed it, and I agree it is the part
      that was missing: a fresh launch with no Stoa shows the **failed** card
      rather than onboarding. The entry says why `"failed"` and not `"absent"` —
      the view has asked the module nothing, so `"absent"` would be a claim
      about the keystore that no reply supports, and onboarding would invite a
      slate for a Stoa that was never named — and states the cost plainly, that
      a developer build shows a failure card first.
      **No test.** This is a recording finding; the branch it describes is
      already pinned by `test_a_failed_report_shows_neither_branch` and, for the
      sibling screen, `test_a_missing_stoa_address_is_a_failure_rather_than_a_silent_empty`.

- [x] **`dev-writer`** — `design.md` does not record that
      `recoveryNeedsTheRecord` **reaches the kept card from `Main`'s who-am-I
      reply, not from the keep reply**, nor that this makes the backup-gap text
      near-unreachable in production
      `design.md:83` says only *"The same applies to `recoveryNeedsTheRecord` on
      the who-am-I reply"*, which records the three-valued treatment but not the
      wiring. In the code it is a property set from outside
      (`Main.qml:145` → `OnboardingScreen.qml:84`), read at
      `OnboardingScreen.qml:671` on the **kept** card. The value in scope there
      came from the who-am-I that reported `hasIdentity:false` — the one that
      caused onboarding to be shown. The keep reply carries no such field, and
      the `who_am_i` re-ask that `design.md:105-111` describes flips
      `identityState` to `"present"`, which hides the whole screen
      (`Main.qml:143`). So the text renders only if an `hasIdentity:false` reply
      carried `recoveryNeedsTheRecord:true` — which is why
      `tst_launch_branch.qml:234` has to construct exactly that reply, and why
      `keptScreenShowing()` (`tst_onboarding_states.qml:818`) sets the property
      by hand.
      This is a decision with a real alternative — reading the field from the
      keep reply, as `encrypted` is read — and the spec's scenario is
      placement-neutral (*"WHEN the identity report says recovery needs more
      than the master key"*), so nothing outside `design.md` records which
      source was chosen or why. A later reader wiring the composer will read the
      kept card and reasonably conclude the keep reply supplies it. Record the
      source, and record that the kept card's third line is a state the launch
      branch normally steps past.
      **Fixed** in `56905f4`, both halves, in the `encrypted` decision where the
      three-valued sentence you quote already was — so a reader meets the source
      question at the point the two fields are first mentioned together.
      **One correction to the framing, and it strengthens your point.** You call
      this "a decision with a real alternative — reading the field from the keep
      reply, as `encrypted` is read". I traced it before writing, and that
      alternative is not available: **`keep_identity`'s reply does not carry the
      field.** `recoveryNeedsTheRecord` appears once in `wire.rs`, at `:981`,
      inside `Whoami::to_json`; `Kept::to_json` emits
      `{kept,address,publicKey,path,encrypted}` and the reply's field set is
      closed by `identity-onboarding`. So the asymmetry is forced by the wire
      contract, and taking the alternative would mean widening a core reply —
      which makes it a stronger reason to record, not a weaker one, because a
      reader wiring the composer would otherwise go looking for a field that
      does not exist.
      Your unreachability analysis I verified and recorded as you wrote it: the
      value in scope on the kept card came from the `hasIdentity:false` reply,
      the re-ask flips the branch to `"present"`, and `Main.qml:143` hides the
      screen — so the line renders only where an `hasIdentity:false` reply
      carried `recoveryNeedsTheRecord:true`, which is exactly why both covering
      tests construct that reply by hand.
      The entry names that as **the launch branch's gap rather than this
      field's**: the screen is hidden before the user reads what it says, and
      the honest fix is for the kept state to be shown by whatever renders after
      onboarding. Recorded rather than fixed, because moving the kept card is a
      question about the post-onboarding screen, which this change does not own.
      Flagging that to the coordinator as a product gap the merge should not
      lose.

- [x] **`dev-writer`** — the general rule at `design.md:207-210` will be
      **archived out of existence**, and the place `piece/drop-apparatus` will
      actually look carries no note
      The rule *"apparatus may repeat an obligation, never carry it alone"* is
      stated twice: in `design.md`, which is archived when this change closes,
      and in an instance-specific comment at `OnboardingScreen.qml:513-531`
      that argues this screen's case without generalising. I grepped
      `ScreenFrame.qml`, `docs/UI-BRIEF.md` and `CLAUDE.md`: the rule appears in
      none of them. `ScreenFrame.qml` is the file that *owns* `property alias
      apparatus` (line 10) and is the file that change edits.
      This is the finding `design.md` itself predicts — *"the column is
      removable by a change that has no reason to read this spec"* — applied to
      the rule rather than to the copy. A one-line note beside
      `ScreenFrame.qml`'s `apparatus` alias saying that anything a spec requires
      a screen to state must also live in the body would survive the archive and
      be read by the person who needs it. Pair it with the test fix above: the
      note says the rule, the test enforces it in the direction that matters.
      **Fixed** in `56905f4`: the note is beside `ScreenFrame.qml`'s `apparatus`
      alias, which is the location you name and the right one — the file owns
      the column and is the file that change edits. It states the rule, says
      *why* the column is the wrong home for an obligation (it is annotation, so
      it is removable by a change that has no reason to read any screen's spec),
      and points at `OnboardingScreen` as the worked example.
      **It carries your correction, which I had got wrong.** The note and
      `design.md` now both say repetition is *permitted, not required*, and name
      this screen's "ON THE MARK" as a note that is legitimately margin-only.
      My earlier framing implied every note needed a body twin, which would have
      made the rule an argument for keeping the column.
      **On the paired test** — `tester`'s box, not ticked here, but the half
      that was mine is done. `design.md` no longer claims the test "requires two
      elements to carry it, so the body copy cannot be dropped silently"; that
      sentence described an assertion that inverts the rule, and citing it as
      enforcement was the thing that made the inversion look intended. In its
      place the entry records **what the enforcing test must assert**: at least
      one carrier outside `apparatus`, never an exact count — with your reason,
      that an exact count's cheapest green is to edit the number, which is a
      count-pin on decoration being deleted and the same failure mode this
      change's own word-count decision argues against. So whoever fixes the test
      finds the property written down rather than having to re-derive it.

## An entry that is thin

- [ ] **`spec-writer`** — `proposal.md:88-95` still records the **opposite**
      decision from `design.md` and the shipped code on the word count
      The proposal says the uniqueness copy *"is therefore used with 'three
      words' corrected"*, that *"the settled shape is four words"*, and that it
      is *"corrected rather than dropped because the rest of the sentence
      carries the obligation."* The code drops the count entirely
      (`OnboardingScreen.qml:534`, `:767`), `design.md:242-252` argues at length
      that dropping rather than correcting is *"the durable decision here"*, and
      `correctness.md:255` records the fix. So the change's own front document
      states the rejected alternative as the chosen one, and asserts "four" as
      settled when `d3e7579` supersedes it.
      This is precisely the shape the task brief warns about — a sibling piece
      was just found citing a `design.md` section for the opposite of what it
      now says. Here it is one document inside the same change. `proposal.md`
      survives into the archive alongside the spec, so whoever reads the change
      later gets both answers with nothing marking which won. Two lines: state
      that no count is asserted, and point at the design entry.

## What I could not check

- The *"1 visible marker where a normal slate shows 0"* figure
  (`design.md:127`) is a measurement of pre-fix behaviour. The helper it names
  is real (`visibleSelectedMarkers`, `tst_onboarding_states.qml:451`) and
  `test_no_row_reads_as_chosen_while_nothing_is_selected` pins 0 on both a
  normal and a hostile slate, so the mechanism is sound; reproducing the `1`
  would mean reverting `isCandidate()`, which I judged out of scope for a
  read-only review. Recorded as unverified rather than accepted.
- The `Main.qml` collision with `piece/ui-stoa-list` is already an open box in
  `findings/architecture.md:130`, deferred to the coordinator with the
  per-Stoa reconstruction recorded. I checked that half against this piece's
  code and agree it is recorded well enough to reconcile from: `whoAmI(stoa)`
  taking a Stoa is visible at `Core.qml`'s wrapper and at `Main.qml:84`, and
  the three contract tests are named. Not re-raised here.

## The two deferrals

Both are honestly recorded, and I endorse both.

**Renaming `ok` was rejected**, and the rejection is argued where it belongs
rather than only in `design.md`: `Core.qml`'s `call()` now carries the warning
at the `ok: true` return itself (*"`ok: true` means THE MODULE ANSWERED. It
does not mean the thing you asked for happened"*), naming all three
two-success-shape methods. I checked the third — `get_capabilities` really does
answer `{"canPost":false,"reason":…}` as a wire success (`wire.rs:175-178`), so
the comment does not overclaim its own scope. The comment is placed at the line
a new wrapper's author reads, which is the mitigation the deferral promises.
`architecture.md:123` is straight about the cost: *"No test. A comment is not
assertable, and I would rather say that than tick a box implying otherwise."*

**The `UI-BRIEF.md` word count was deferred to #64**, and the pointer left
behind is genuinely self-invalidating rather than a promise: `docs/UI-BRIEF.md`
now says in the file itself that the count is unsettled, names the superseding
commit by hash (`d3e7579`), names the three passages that are wrong, and says
*"Until it lands, take the shape from PLAN.md §5.2.1, which wins any
disagreement with this file."* A designer reading the brief is warned at the
point of use, which is the test CLAUDE.md sets for a live document. I confirmed
the three named passages are still four-word on this branch, so the pointer
describes the file's actual state.

## On PLAN.md

No contradiction. The change's PLAN.md edit adds Stage A′ and rewires the
dependency sentence, and the reasoning it acts on migrated correctly: the
"unplaced" strike-through is struck rather than deleted, per that section's
convention, and the substance moved into the spec it names. §5.2.1 is untouched
and the code takes no position on the word count, which is the only way to be
consistent with a section that is mid-supersession. The cross-Stoa
unlinkability copy decision (`design.md:233-240`) quotes §5.2.1's *"Cross-Stoa
unlinkability is suspended, not withdrawn"* accurately and the screen ships the
first half with nothing in the second's place, as both the spec and PLAN
require.
