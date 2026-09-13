# Findings — readability

Reviewed: `dialectica-ui/src/qml/OnboardingScreen.qml`, `Core.qml`, `Main.qml`,
`design.md`, `tasks.md`, and the two new test files. Dimension covered:
**readability** only. Architecture is my other file; correctness and security
were another instance's and I have not re-reported anything in
`findings/correctness.md`.

Baseline: **99 QML tests across 6 spec files, all passing**
(`dialectica-ui/tests/run-qml-tests.sh`, qmltestrunner 6.10.3) — 13 + 12 + 7 +
12 + 46 + 9. The claim of 99 in the task brief is accurate.

## Defects

- [ ] **`tester`** — `tst_onboarding_states.qml:748` —
      `test_the_two_encryption_replies_produce_different_text` asserts on the
      property feeding the binding, never on the rendered text its name promises
      **Scenario:** the test drives two screens to the kept phase and compares
      `encrypted.keptIdentity.encrypted` to `plain.keptIdentity.encrypted`. Both
      are reads of the value the fixture supplied; nothing reads the `Text` the
      spec's scenario is about ("the two replies produce different text, so the
      state is readable rather than implied"). Collapse the render site's
      conditional to the encrypted string alone —
      `OnboardingScreen.qml:622-624`, `text: reported === true ? A : B` becomes
      `text: "The master key on this machine is stored encrypted."` — and the
      screen now tells a user whose key is in the clear that it is encrypted.
      That is the exact claim the spec forbids: *"Where the reply says the key
      was **not** encrypted, the screen SHALL say so plainly"*, and PLAN's
      reasoning that protection reading as strong while absent is worse than
      visible plaintext.
      **Measured: 99 of 99 tests pass under this mutation.** The test whose name
      contains "produce different text" is among the 99 that pass.
      The fix is in reach and needs no new harness: the file already has
      `everyTextOn()`, and `visibleSelectedMarkers()` at line 393 already shows
      the pattern of walking for a *shown* string. Assert that the kept phase's
      visible text contains "stored encrypted" for the one reply and "in the
      clear" for the other, and that the two rendered strings differ.
      **Severity: high** as a test gap — no defect in the shipped render today,
      but this is the screen's one safety-relevant claim and nothing pins it.

- [ ] **`tester`** — `tst_onboarding_states.qml:796` —
      `test_an_omitted_recovery_field_produces_no_claim` never checks that the
      claim is absent from the screen; it only round-trips the property
      **Scenario:** the test sets `screen.recoveryNeedsTheRecord` to `true`,
      then to `undefined`, and asserts the property `!== false`. It never looks
      at the `Text` at `OnboardingScreen.qml:640-649` whose `visible:` binding is
      the thing that makes "no claim" true. Replace that binding —
      `visible: screen.recoveryNeedsTheRecord === true` becomes `visible: true` —
      and the backup-gap sentence ("Which key you chose is recorded only on this
      machine… a copy of the master key by itself is not enough to get back in")
      is shown to every kept user, including one whose module never reported the
      field. The spec: *"A reply omitting either field SHALL produce no claim
      about it."*
      **Measured: 99 of 99 tests pass under this mutation.**
      This is the recorded defect-family variant that cost a sibling piece 47
      green tests — a test asserting the computation feeding a binding rather
      than the rendered property. Two instances of it are in this one file, so
      it is worth fixing as a family rather than twice: the sweep helpers needed
      already exist in the file.
      **Severity: medium** — the false claim here is over-warning rather than
      under-warning, which is the safer direction, unlike the finding above.

- [x] **`dev-writer`** — `tasks.md:42` — the implementation checklist states the
      opposite of what shipped, on the one line the copy history makes most
      load-bearing
      **Scenario:** the line reads "Copy: the cross-Stoa unlinkability clause is
      dropped with no replacement claim, **and the uniqueness note says four
      words**", and it is ticked `[x]`. The uniqueness note says no count at all
      — that is the whole point of `1d91627` and of `design.md`'s "the count is
      removed rather than corrected", and `test_the_uniqueness_note_states_the_
      obligation_without_a_word_count` sweeps eight spellings to keep any number
      out. A reader reconciling the ticked task list against the code finds a
      ticked box asserting the number that was deliberately removed, on a fact
      that has already moved three times. The first half of the same line is
      true; only the clause after "and" is stale.
      **Severity: medium** (documentation defect, not code). It is the fourth
      recorded instance in this repo of a stale record about this exact count,
      and the one place a reader is most likely to trust it, since a ticked box
      reads as verified.
      **Fixed** in `a2e508d`. My miss: I removed the count from the screen and
      from `design.md` in `1d91627` and left the task line asserting it, which
      is the orphaned-citation failure this repo has a standing memory about —
      in the record rather than the doc this time.
      The line now says the note states its obligation with no word count at
      all, and why (the count has moved three times), pointing at `design.md`
      rather than restating the reasoning. No test covers a task list, so this
      is a documentation fix with no accompanying assertion — the code half is
      already pinned by
      `test_the_uniqueness_note_states_the_obligation_without_a_word_count`,
      which sweeps eight spellings and would fail if any number returned.

- [x] **`dev-writer`** — `tasks.md:51` — two test counts in a ticked box are
      both wrong, in a repo whose CLAUDE.md forbids writing down what a command
      can answer
      **Scenario:** the line reads "Tests: `tst_onboarding_states.qml` (36) and
      `tst_launch_branch.qml` (10)." Counted with `grep -c "function test_"`:
      **44** and **10**. The runner reports 46 and 12 (each adds
      `initTestCase`/`cleanupTestCase`). So the second number is right by one
      reading and the first is wrong by eight under every reading.
      CLAUDE.md's "Keeping this file true" table names "how many tests pass" as
      a thing to replace with the command rather than write down. The durable
      fix is to name the runner rather than correct the number to a figure that
      goes stale on the next test added.
      **Severity: low** (documentation defect). Recorded because this repo has a
      standing memory that it fabricates figures in comments, and because a
      count in a ticked box is the shape that gets copied into the next piece's
      task list.
      **Fixed** in `a2e508d`, by the durable route you name rather than by
      correcting the numbers: the line now names
      `dialectica-ui/tests/run-qml-tests.sh` and states no count, so it cannot
      go stale on the next test added.
      Your diagnosis of how the 36 arose is right — I wrote it from the runner's
      totals at one moment and did not re-derive it after adding tests in
      `1d91627`, which is exactly the failure mode the CLAUDE.md rule describes.
      I have also dropped the "five mutations" count from the same line for the
      same reason; the findings files record which mutation reaches which test,
      and that is the durable record.

- [x] **`dev-writer`** — `tst_onboarding_states.qml:362` — a shipped test
      comment quotes `design.md` saying something `design.md` no longer says
      **Scenario:** the comment opens *"The guard `design.md` calls "the guard
      that actually guards" was unprotected"*. That phrase was removed from
      `design.md` during the same fix — correctly, because it had become false
      once the two guards collapsed into one. `grep -rn "actually guards"` over
      the tree now returns two hits: this comment, and `findings/correctness.md`
      quoting the history. So the surviving prose the brief asked me to check is
      true in `design.md` (the rewritten passage at `design.md:149-157` accurately
      describes one guard and why the second was not a guard) — but this test
      comment is the dangling half of that rewrite, pointing a reader at a
      document that will not confirm it.
      This repo has a standing memory that correcting prose can orphan a
      citation elsewhere, and this is that, in the test file rather than the doc.
      The comment's substance is otherwise accurate and worth keeping; only the
      attribution is stale.
      **Severity: low.**
      **Fixed** in `a2e508d`, keeping the substance and changing only the
      attribution, as you recommend. The comment now states the two-guard
      history in its own words and cites `design.md`'s heading "Selection is a
      candidate's own index, and the sentinel is unaddressable" — verified to
      resolve, rather than a phrase I hoped was still there.
      `grep -rn "actually guards"` over `dialectica-ui/` and `openspec/` now
      returns only the two findings files quoting it as history, which is the
      correct place for a phrase that was removed.

## What was clean

**The phase machine is legible and each transition is easy to find.** Five
states are named with a one-line gloss each at `OnboardingScreen.qml:41-45`,
and every assignment to `phase` is in one of three functions — `requestSlate()`,
`enterFailed()`, `keepSelected()` — so "what can commit the irreversible
choice?" has a short answer: only `keepSelected()`, only after
`reply.value.kept === true`, and only after an address check. Which states are
terminal is legible from the code rather than needing a comment: `"kept"` has no
outgoing transition, `"failed"` and `"refused"` each have a visible control that
re-enters `requestSlate()`. Every `visible:` binding is a `phase === "…"`
comparison that cannot overlap, so a reader does not have to hold a truth table.

**`isCandidate()` and `nothingSelected` read correctly as a stranger.** The
"why validate before assignment rather than at the render site" question is
answered *in the code*, at `OnboardingScreen.qml:176-181`, and answered with the
consequence rather than the rule: a range check at the render site "leaves a
candidate on screen that the user can see and cannot choose, which is the same
dead end with a narrower blast radius". That is the shape of comment CLAUDE.md
asks for — why this and not the obvious alternative. `nothingSelected`'s own
comment (line 53-62) explains that naming it is not cosmetic, which pre-empts
the reader who would otherwise delete it as ceremony. I read both cold before
reading `design.md` and did not need `design.md` to follow either.

**The surviving guard prose is now true.** `keepSelected()`'s comment
(lines 218-232) states the collapse and its justification — with the sentinel
unaddressable, "is a candidate selected" and "does the selection name a
candidate" are the same question — and `candidateAt()`'s comment at line 289
states the same fact from the other side, which is where a reader arrives if
they follow the call. `design.md:149-157` agrees with both. The one stale
artefact of that rewrite is the test comment in the defect above.

**The unparsed-reason rationale is visible in the code, not only in
`design.md`.** `Main.qml:58-60` carries it at the property: *"Any `indexOf(...)`
over core's wording would be a second copy of core's error taxonomy, maintained
in the wrong module, and would silently reclassify the day core rewords a
message."* `OnboardingScreen.qml:106-108` carries the matching half for the
failure text. A reader who never opens `design.md` gets the reason at both sites.
Verified there is no classification in either file.

**The absence assertions have a genuinely strong corpus** — I expected the
recorded "absence assertion over a corpus where the text could never appear"
variant here and did not find it. I instrumented `spec.collectText()` against a
live screen: the walk reaches **35 Text items**, including every phase's copy
regardless of `visible` (the refused, kept, failed and recovery strings are all
present while `phase === "slate"`) and all three `apparatus` MarginNote labels
and bodies. So `test_no_copy_claims_the_identity_cannot_be_linked_elsewhere`
and `test_no_copy_on_this_screen_uses_the_word_username` are scanning the whole
screen's text in every state, not one state's. The two tests that guard against
a vacuous corpus (`verify(texts.length > 5, …)`) make that self-checking.

Note the flip side, which is the two defects above rather than a third: because
the walk ignores `visible`, it is the wrong instrument for a *presence*
assertion. Every presence test in the file happens to be about copy that is
unconditional or phase-gated in a way the test drives correctly, so none is
currently wrong — but the two rendered-state properties that needed a
visibility-aware assertion did not get one.

**Comments earn their place throughout.** I looked for the restating-the-code
kind and did not find one. The density is high but each block answers a "why"
a reader would actually ask: why `String()` is absent (line 249-252), why
`encrypted` is copied raw rather than coerced (line 74-78), why the count comes
from the reply (line 154-156), why there is no counter on refresh (line 92-96).
The two `NO SPEC` markers are a good convention — they mark where the
implementation chose something the spec left open, which is exactly where a
later reader would otherwise assume a requirement existed.

## What I could not check

The suite asserts on properties and the object tree, never on layout or pixels.
So nothing here verifies that the candidate row leaves room for a name to arrive
above the address without the layout moving — which `design.md:241-252` and the
`UI-BRIEF.md` paragraph this change added both promise, and which is now a
written obligation to an external designer. That promise is unpinned by any gate.

I did not run the module end to end against real core; every reply I used came
from the fake bridge, which is the same surface the shipped tests use.
