# ui-stoa-list — design review

Checked `design.md`'s Decisions against the shipped QML, and `design.md` against
`docs/PLAN.md` as it stands on **`origin/main`**. Run, not read: every claim
below that says **Measured** was produced by driving the real components under
`qmltestrunner -platform offscreen` against `dialectica-ui/src/qml`, and the
probe files were deleted afterwards.

The suite is green as it stands — 65 in `tst_stoa_screens.qml`, 106 across the
five spec files, `bash dialectica-ui/tests/run-qml-tests.sh`. That is the
problem with the first two findings: both describe a screen the tests never
drive into the state a user reaches.

## The headline

D4 and D8 are each internally coherent and each correct about the defect it was
written for. **Together they switch off the impersonation defence.** D8 made
`foundingTitle` a derivation of `currentOutcome`, which is `null` until the core
answers a join; D4's `lookalikes` gates on `foundingTitle !== ""`. So the panel
whose whole purpose is to warn *before* the user commits can only render *after*
they have committed. Neither document notices, and the architecture review
records the opposite conclusion in as many words.

This is the failure mode the brief warned me to look for, arriving by a route
neither of the two reviews that touched it could see from where they stood:
the correctness reviewer verified the carry-over was fixed, the architecture
reviewer verified the guard could no longer be fed its own output, and nobody
asked whether the guard fires at all.

## Findings

- [x] **`dev-writer`** — the lookalike panel cannot render at preview time, so
      the impersonation defence never runs before the user acts.
      `JoinScreen.qml:141-153` computes `lookalikes` and returns `[]` immediately
      when `screen.foundingTitle === ""`. `foundingTitle` (`JoinScreen.qml:74-75`)
      is `readonly` and derived from `currentOutcome`, which is `null`
      (`JoinScreen.qml:57-61`) until `join()` has stored an outcome for the
      reference on screen. Nothing else writes an outcome. So at preview time —
      the only moment the warning is useful — `foundingTitle` is `""` and the
      panel is unconditionally absent.
      **Measured**, driving the real `Main.qml` through the real paste route with
      a held Stoa titled `Nym Research` in the listing and an attacker's
      reference presenting the same title pasted into the field:
      `foundingTitle=<>`, `lookalikes=0`, `lookalikePanel` visible count `0`,
      `heldStoas=1`. Driving the same screen through a successful join instead:
      before `join()` `lookalikes=0`, after `join()` `lookalikes=1`. The warning
      arrives one action too late to be a warning.
      **Why the tests do not see it:**
      `test_a_same_title_stoa_already_held_is_shown_beside_the_preview`
      (`tst_stoa_screens.qml:1257`) passes `foundingTitle: "Nym Research"` into
      `joinComponent.createObject(null, props)`. QML accepts that as the
      *initial value* of a readonly property — **measured**: after that call
      `foundingTitle` is `Nym Research` while `outcome` is `null`, a pair the
      shipped screen cannot produce. The test drives a state that exists only in
      the test.
      **Severity: high**, and this is the one finding that blocks. The spec
      requirement "A Stoa already held whose title matches is shown as a distinct
      Stoa, not as a duplicate" (`spec.md:550`) is not met by the shipped
      screen, and `design.md` D4 describes the comparison as working.
      **Note it is not a regression from D8** — `git show 8de7571` has the same
      structure, `foundingTitle` written only inside `join()`. D8 made the
      impossibility permanent rather than introducing it, which is why the fix
      cannot be "revert D8". The real question is whether the view can obtain a
      founding title before joining at all; see the next box.

      **Fixed in part, and the unfixable part is now recorded rather than
      silent.** Reproduced first, driving the real `Main.qml` paste route exactly
      as you did — `foundingTitle=<> lookalikes=0 panel=0 heldStoas=1` before,
      `<Nym Research> / 1 / 1` after `join()`. Your measurement stands in every
      digit.

      **The view cannot obtain a founding title before joining, and I did not
      invent a core change to make it.** `join_stoa` is the only method that
      answers a `foundingTitle` for a `(stoa, genesis)` pair; `list_stoas` and
      `create_stoa` answer only about Stoas already held or just made. The title
      *is* in the pasted record (`wire.rs:1526`, `stoa_reply` reads
      `genesis.title`), so this is an API gap rather than a data one — and
      closing it in QML means a second implementation of core's genesis encoding,
      which is the thing the split exists to prevent. That is now **constraint 4**
      at the top of `design.md`, with the same weight as the other three.

      So the resolution is the one you named as possibly acceptable, made
      explicit rather than left to the empty panel: **the check is late, not
      missing, and the screen says which.** `D11` records it, including why the
      three tempting fixes are each worse — relaxing D4's guard matches every
      untitled Stoa, carrying a title over is the impersonation D8 exists to
      stop, and decoding in QML is constraint 4.

      Two regression tests through the **real paste route**, both watched failing
      first (`test_a_preview_before_a_join_carries_no_founding_title_and_says_so`,
      `test_the_lookalike_warning_cannot_be_claimed_before_a_join_happens`), and
      both then mutation-tested by reverting the two `visible:` bindings. **The
      first pass of that mutation caught a defect in my own test**: a whole-body
      scan stayed green with the note hidden, because the apparatus column's `ON
      THE TITLE` note already carries "title" and "matched against nothing" — a
      corpus with a decoy in it, the family this file's header names. Both
      assertions now read one named element's text instead.

      The two props-route tests are left in place — "when a title exists it is
      labelled founding" is still the requirement — but each now carries a
      comment saying it drives a state `Main.qml` cannot construct, and names the
      real-route test that measures the timing.

- [x] **`dev-writer`** — the join preview renders an empty `FOUNDING TITLE —
      FIXED FOREVER` panel, and `design.md`, the spec and `docs/UI-BRIEF.md` all
      say otherwise.
      `JoinScreen.qml:305-316` binds `foundingTitleText` to `screen.foundingTitle`,
      which is `""` before a join for the reason above. The label above it is
      unconditional, so a real preview shows the caption with nothing under it.
      **Measured** through `Main.qml`'s paste route: `foundingTitleText=<>`,
      and the whole rendered body is the eyebrow, the full address, and the
      address note — no title anywhere.
      The spec requires "The preview MUST render the founding title, and MUST
      label it as the **founding** title" (`spec.md:324`) with the scenario "The
      founding title is labelled as founding" (`spec.md:349`). `tasks.md` 6.3 is
      ticked citing `test_the_founding_title_is_labelled_as_founding`
      (`tst_stoa_screens.qml:935`), which injects `foundingTitle` through
      `createObject` props exactly as above.
      **The constraint is real and is the thing to record.** `join_stoa` returns
      `foundingTitle` (`wire.rs:2016`, and the core test at `wire.rs:8858`
      asserts the reply carries it), but there is no view-side call that reads a
      title out of a genesis record, and writing a genesis decoder in QML would
      be a second implementation of core's encoding. So the honest position is
      that this build **cannot** show a founding title before joining. That is a
      constraint of the same family as the three at the top of `design.md`, and
      it belongs there with the same weight — it is the reason two spec
      requirements and a UI-BRIEF paragraph are currently describing a screen
      that does not exist. Decide it, record it, and make the spec, the brief and
      the empty panel agree; the panel being reserved-and-empty is defensible,
      quietly rendering a caption over nothing is not.

      **Fixed** — the caption no longer stands over nothing, and the constraint is
      recorded in all three places.

      The founding panel is now conditional on the title existing, and a
      `titleUnknownNote` renders in its place saying, in words, that the title is
      inside the record and only joining asks for it, and that the same-title
      comparison has therefore not been made. Both bind to one derived
      `titleKnown`, so they cannot both show or both hide.

      **I went further than "reserved and empty" deliberately, and the reason is
      one you could not check without a screenshot but which does not need one.**
      An empty founding title is a *legal* value — the genesis record has no
      minimum length, and `test_an_empty_founding_title_still_gets_a_row_with_its_address`
      pins that the list renders such a row. So a caption over blank space is not
      merely unclear, it is a **specific false claim**: it says this Stoa's
      founding title is blank, on the one screen where the reader is deciding
      whether to trust an address, and the reader has no way to tell that from
      "unknown here". Reserved-and-empty would be defensible for a value that
      cannot legally be empty; this one can.

      `design.md` constraint 4 and D11 carry the reasoning; `docs/PLAN.md` now
      strikes the title half of the obligation rather than claiming it discharged
      (next box but one); `docs/UI-BRIEF.md` 340-345 is rewritten (box after
      that).

- [ ] **`spec-writer`** — two spec requirements describe a preview this build
      cannot render, and both are ticked.
      "Joining shows what is being joined, and joins nothing until the user acts"
      (`spec.md:306`) requires the founding title rendered and labelled;
      "A Stoa already held whose title matches is shown as a distinct Stoa"
      (`spec.md:550`) requires the lookalike beside the preview. Neither is
      satisfiable while the only source of a founding title is a join reply.
      Either the requirements gain the conditional half the share requirement
      already has — `spec.md:186` is the model: "Where the record is not
      available for a Stoa, no share is offered for it; the affordance's absence
      is the honest rendering, and it is not an error state" — or they name the
      core change that makes them satisfiable. Leaving them as written means the
      next reader believes the defence is contracted and running.

      **Left open — not addressed to me, and I have not touched `spec.md`.** But
      the code has moved under this box, so whoever ticks it should read the two
      boxes above first. The screen now renders the address, states in words that
      no title is available before joining, and states that the same-title
      comparison has not been made; after a join both the title and the lookalike
      appear. So the shipped behaviour is no longer "describes a preview this
      build cannot render" — it is a preview that renders an honest partial and
      names what is missing.
      Of your two options, the **first** is the one the code now matches:
      `spec.md:186`'s shape, where the absence is the honest rendering and is not
      an error state. The second — naming the core change — is recorded in
      `design.md` D11 and in `docs/PLAN.md` as `getStoa`'s remaining job, but **no
      core change is made in this piece**. `spec.md:324` and `spec.md:550` still
      read unconditionally and I have left them that way.

- [x] **`dev-writer`** — the branch's `docs/PLAN.md:3567-3572` marks the
      join-confirmation obligation as discharged while half of it is not.
      `docs/PLAN.md` on `origin/main`, line 3558, reads: acting on an in-post
      address "shows what is being joined — **the Stoa's title and address** —
      **before** joining". The branch strikes the "not built" sentence around it
      and writes "**The obligation is now contracted** by `stoa-navigation-view`'s
      'Joining shows what is being joined…'". The title half is not built, as the
      two boxes above establish, and the sentence naming it (branch PLAN.md:3565)
      is left **un-struck** and is now false of the shipped screen. Contradicting
      PLAN.md is legitimate and this repo says so; doing it in passing is not.
      Either strike the title half with the reason, or narrow the "now
      contracted" claim to the address and the no-auto-join half.

      **Fixed — both, because they turned out to be one edit.** The sentence now
      reads "shows what is being joined — ~~the Stoa's title and address~~ **the
      Stoa's address; see below for why not its title** — **before** joining",
      and the "now contracted" claim is narrowed to "the address half and the
      no-auto-join half".
      Two paragraphs follow, and they are the part worth reviewing rather than
      the strike: one says the title half cannot be built on this API and why
      (`join_stoa` is the only call that answers a title; reading the record in
      the view is a second implementation of core's encoding), and one names what
      it costs — **the same-title warning becomes a record of what happened
      rather than a warning about what is about to**, which is a real weakening
      of the property that section describes and should not be buried in a
      strike-through. Both name `getStoa` as what closes it.
      I also narrowed the paragraph at PLAN.md:3597-3604, which you did not flag:
      its third contracted item is the lookalike, and it now says that item runs
      after a join rather than before one. Without that, fixing one paragraph
      would have left the next one making the same claim.

- [x] **`dev-writer`** — `docs/UI-BRIEF.md` is now wrong about the join
      confirmation, and the brief is designed against.
      Lines 340-344 tell the designer "Nothing resolves the moderator-signed
      metadata op, so **the founding title is the only title there is**. A panel
      captioned 'current title' filled with the founding value asserts that
      nobody has renamed the Stoa… Reserve the position; do not fill it." A
      designer reads that as: one title panel, filled with the founding value;
      one position, reserved. The shipped preview has **both** positions empty
      before a join. CLAUDE.md's rule is that a change invalidating the brief
      fixes it in the same change. Say plainly that on this build the preview
      carries no title until a join has succeeded, so the screen is designed for
      the state that actually renders.

      **Fixed**, and expanded past what you asked because the shortest honest
      version was misleading. The bullet now separates the two empty positions
      and gives each its own reason — the current title has none because nothing
      resolves the metadata op, the founding one has none because only joining
      asks for it — and says explicitly that a founding caption over blank space
      tells the reader the title **is** blank, which is a legal value the list
      renders, so the reader cannot distinguish "empty" from "unknown".
      It also says what to design *instead* (the absence needs a voice, not a
      blank panel), and adds a second bullet on the same-title warning arriving
      after the join, ending with the instruction that matters most to a
      designer: do not let an absent warning read as a clean result.
      Both are marked as constraints of the current API rather than permanent,
      naming `getStoa`, so the paragraph shrinks visibly when that lands rather
      than going quietly wrong.
      I checked the brief's other founding-title passages (lines 247, 259-265,
      268, 280, 284, 583) — those are about the list and the general
      title-is-not-an-identifier rule, and all remain true.

- [x] **`dev-writer`** — `design.md:23` states as verified evidence something a
      grep disproves. "a grep of `dialectica/` finds no `stoa:` literal
      anywhere" — `dialectica/rust-lib/src/lib.rs:517` is
      `core::error_json(&format!("stoa: {e}"))`. The **conclusion** survives
      untouched: that is an error-message prefix, not a wire format, and nothing
      in the core writes or accepts a `stoa:` display prefix on an address. But
      the sentence is offered as a command's answer and the command gives a
      different one, which is the shape this repo's memory calls a persuasive
      citation. Reword to what is actually true — no core path produces or
      accepts the prefix on an address — or name the grep that shows it.

- [x] **`dev-writer`** — `design.md:256` cites a distance that is off by
      seventy. D10 says the `readState === "ok" ? rows : []` guard sat "on the
      `Repeater` model 213 lines from the state making it necessary". In the
      pre-fix file (`git show a9888f8:dialectica-ui/src/qml/StoaListScreen.qml`)
      `readState` is line 21 and the guarded `model:` is line 304 — 283 lines,
      not 213. The argument does not depend on the figure, which is why it is one
      box and not a serious one; but this change has already had a stale count
      removed once, and a number in a design document is a claim. Prefer "the far
      end of the file" over a digit nobody re-derives.

      **Fixed** — and I took the wording you offered rather than correcting 213
      to 283. D10 now reads "at the far end of the file from the state making it
      necessary", with a parenthetical naming the two commands that re-derive it
      (`git show a9888f8:dialectica-ui/src/qml/StoaListScreen.qml`, grep
      `readState`) and saying why the digit is gone rather than corrected. Your
      figure is right — `readState` is line 21 and the guarded `model:` is line
      304 in the pre-fix file — which is precisely why replacing it with another
      digit would only reset the clock on the same rot.

- [x] **`dev-writer`** — `design.md:330` says "Four choices below are observable
      behaviour the spec is silent on" and there is a fifth marked in the code.
      `StoaListScreen.qml:46` carries `// NO SPEC: the spec names no page size for
      this listing. 25 was chosen to fill a card without a scroll on the mockup's
      1000px width`. That is exactly the shape of the four already listed, and it
      is the one with a user-visible consequence the others lack — it decides how
      many Stoas a user sees before paging. Add it to the section, or drop the
      count and let the list be a list.

      **Fixed — both halves, because either alone leaves the same trap.** The
      page size is now the fifth bullet, carrying your reason for why it is the
      one that matters: the other four are all failure-handling, and this is the
      only one a user meets in normal use. And the count is gone — the opening
      sentence now names `grep -rn "NO SPEC:" dialectica-ui/` as the list rather
      than asserting a number, so a sixth marker cannot silently outrun the prose
      the way the fifth did.
      Adding the bullet without dropping the count would have been the trap: it
      makes the sentence true today and restores exactly the condition that made
      it false. That is this repo's hand-maintained-sweep-list failure, which
      goes stale with every gate green.

## Checked and sound

Everything below I tried to break and could not.

- **D1.** `StoaReference.shareText` / `parse` are one file and one decision, the
  JSON round trip holds, and the three rejected alternatives are each given a
  reason that is a property rather than a preference. The `stripPrefix` loop is
  a real loop (`StoaReference.qml:62-67`), not a second `if`, and D1's account
  of why a fixed number of passes was the defect is accurate to the code.
- **D2.** The Qt-module argument checks out: `.github/workflows/ci.yml:442-444`
  installs `qml6-module-qtquick`, `-controls`, `-layouts`, `-templates` and
  `-window`, and `Qt.labs.platform` is not among them. `ClipboardSink` is
  described as "a zero-size, non-visible `TextEdit`" where it is an `Item`
  wrapping one — accurate enough that I am not spending a box on it. The
  untestable half is named in the component itself, not merely asserted around.
- **D3.** The two state strings match the code, and the reason given — that no
  combination of flags can put two states on screen at once — is the right
  argument for the shape. `"unread"` is a fourth list state that nothing renders
  (`Component.onCompleted` calls `reload()` immediately); harmless, and I
  mention it only so the next reader does not go looking for its branch.
- **D5 and D9.** `Main.qml` holds `chosen` and `previewing`, the setters clear
  each other, and `(chosen ≠ null, previewing ≠ null)` is genuinely
  unconstructible through `preview()` / `open()`. The `StackView` rejection is
  argued from a property — a push/pop lifecycle is a second source of truth —
  rather than from size. `onCancelled: root.previewing = null` (`Main.qml:161`)
  is a bare assignment rather than a function, which reads as an exception to
  D9's own rule; it only ever clears, so it cannot construct the bad state, and
  I am satisfied it is safe rather than an oversight left unrecorded.
- **D6.** Three wrappers, `perPage` taken from the caller, each core method
  spelled once. Verified by reading `Core.qml` end to end.
- **D7.** The corpus measurement is the one figure in the document that survives
  re-running it: apparatus **747** characters, as written. The whole is now
  **1358** rather than the recorded 1371 — copy shifted under it — and "more than
  half is margin note" still holds, so this is a note and not a box.
- **D8** as a fix for what it was written for. The outcome genuinely cannot
  outlive its reference: `currentOutcome` compares both halves of the pair, and
  `join()` captures `stoa`/`genesis` once at the top and never re-reads the
  binding. The recorded concession about re-previewing the identical reference
  is correct and is the right invariant.

## On `Main.qml` and `piece/ui-onboarding` (#60)

You asked whether D5's reasoning survives the other branch's position. **The
reasoning survives; the two designs do not compose, and neither document owns
the question.**

D5's argument is that a defaulted Stoa property on the top-level view is a
second source for the one value these screens exist to supply, so a build can
ship a hardcoded Stoa. That is sound and independent of onboarding — nothing
#60 does weakens it, and it would still be the right call if #60 landed first.

The collision is sharper than "one branch deletes what the other reads".
`piece/ui-onboarding`'s `Main.qml` opens with `Component.onCompleted:
root.askWhoAmI()`, and `askWhoAmI()` begins:

```
if (root.stoaAddress === "") {
    root.identityState = "failed"
    root.identityFailure = "No Stoa address was given to this view."
    return
}
```

It then calls `Core.whoAmI(root.stoaAddress)`. So on that branch identity is
asked **per Stoa address**, at launch, and the whole view is gated on the
answer — `OnboardingScreen`, `FeedScreen` and the failure frame are the three
branches of `identityState`. This branch's entry point is a Stoa **list** with
no Stoa chosen, by design. Merge them naively and the launch path asks
`whoAmI("")` and lands every user on "Whether you have an identity here could
not be determined", permanently, because there is no longer anywhere for a
launch-time address to come from.

The question that needs an answer in one of the two `design.md`s, and is in
neither: **is identity scoped to a Stoa or to the peer?** If per-Stoa, the
onboarding gate belongs after a Stoa is chosen — between the list and the feed,
or on the join preview — and `Main.qml` gains a third navigator state rather
than a fourth top-level property. If per-peer, `whoAmI` should not be taking an
address and the gate is genuinely at launch, in which case D5's removal is fine
and it is #60's call that needs rework. Nothing in this branch's documents
records which, because from inside this piece the question does not arise.

That is yours to resolve and I have touched neither branch.

## What I could not check

- **Anything visual.** Whether the empty founding-title panel reads as reserved
  or as broken is exactly the judgement a screenshot answers and I have none.
  It matters more than usual here, because the second finding's acceptable
  resolution may be "leave it empty and say so".
- **The core's replies.** Every reply in every probe was a fake bridge. I read
  `wire.rs` for the `create_stoa`, `join_stoa` and `list_stoas` shapes rather
  than running the core.
- **`QT_QPA_PLATFORM=offscreen` height.** I confirmed QtTest reaches `height`
  and `implicitHeight` as properties, and got `0` for both on a `Main.qml`
  mounted with an explicit 1000×900 — the `ColumnLayout` children are sized by
  the layout, which needs a real window pass. So "QtTest can measure height" is
  true of the API and I could not make it produce a non-zero number here. I did
  not pursue it, since no finding of mine turns on a height.
- **`piece/ui-onboarding`'s tests.** I read its `Main.qml` and nothing else on
  that branch.
