# Design-record review — `ui-shell-components`

Reviewed `design.md` against the code, against the design bundle at
`tmp/ui-bundle-new/handoff/`, and against `docs/PLAN.md` **as it stands on
`origin/main`** (`6eec84f`).

**The recorded decisions are in unusually good shape, and three of them were
verified by execution rather than by reading.** D1's shadowing measurement, D5a's
qmllint claim and D6's "already done by #70" correction all reproduce exactly as
written — see the notes under each finding below where they bear on one. D2's
fail-toward-degraded, D4's opacity-not-ink argument and D5's traced behaviour
change are each stated with the constraint, the alternatives and the cost, which
is the full shape. D5's honesty about its own first draft being wrong — "the
first draft of this note claimed the reshape preserved behaviour, and tracing the
old chains disproved it" — is the thing this role exists to see more of.

The findings below are gaps rather than contradictions. **The first is a blocker
for the closer**; the rest are decisions taken in the code that the record does
not carry.

---

- [x] **`dev-writer`** — `openspec/changes/ui-shell-components/` has **no
      `.openspec.yaml` and no `proposal.md`**, so the no-spec decision is
      recorded only in prose the tooling cannot read, and the change **fails
      `openspec validate --strict` today**. **Verified by running it** from the
      piece worktree:

      ```
      ✗ [ERROR] file: Change must have at least one delta. No deltas found.
        ... If this change intentionally modifies no specs (pure refactor,
        tooling, docs), set "skip_specs: true" in the change's .openspec.yaml
      ```

      `tasks.md:3-10` argues the no-delta case well, and I agree with the
      judgement — this piece adds no core behaviour and no requirement. But the
      argument is in a struck checkbox, and `tasks.md:20` hands the closer
      `openspec validate --strict` as a gate that cannot pass. The precedent is
      `2026-09-14-drop-apparatus`, which made the same call and carried both
      files: its `.openspec.yaml` is worth copying verbatim in shape, including
      the comment explaining that **`schema: spec-driven` is required or
      `skip_specs` is silently not honoured** — `docs/OPENSPEC-ARCHIVE.md` and
      that file both record this, and without the schema line the validator
      "reports the marker as ignored and then fails for having no deltas, which
      reads as two problems rather than one". Every one of the 24 archived
      changes carries a `proposal.md`; this is the first that does not.

      **Fixed.** Both files added, with the `schema: spec-driven` line and the
      comment explaining why it is required. Verified by running the gate the
      box names:

      ```
      $ openspec validate ui-shell-components --strict
      Change 'ui-shell-components' is valid
      ℹ [INFO] skip_specs is set in .openspec.yaml: change declares no
        spec-level behavior changes, zero deltas accepted
      ```

      The INFO line is the confirmation that matters — it says `skip_specs` was
      *honoured*, which is exactly what the missing `schema:` line would have
      broken silently.

      The no-delta argument is re-measured rather than copied from the
      precedent: `grep -rniIl` over `openspec/specs/` for the three component
      names returns **nothing**, and the two files matching
      `tooltip|textFormat|PlainText` match on the cryptographic sense of
      "plaintext" (`keystore/spec.md:33,309`,
      `view-identity-onboarding/spec.md:562`). So there is no requirement to
      modify and none to delete.

      `.openspec.yaml` also names what `skip_specs` does **not** say: four
      behaviours here are chosen rather than specified, each marked `NO SPEC:`
      beside the test that pins it. (Four, not three — the lamp default became
      the fourth; see the box below.)

- [x] **`dev-writer`** — `design.md` Decisions is **silent on
      `statusFailed` being the same value as `accent`**, which is a decision with
      a real alternative and a paragraph of justification already written for it.
      `DTheme.qml:66-72` argues it at length — "A failed lamp and a destructive
      action are the same alarm at different scales... They are separate tokens
      because they are separate ROLES — retuning the destructive red must not
      silently retune the failure lamp — not because the values differ today."
      That is a complete Decisions entry sitting in a source comment. The
      alternative (a fourth distinct red) is real and was evidently considered,
      and the cost is real too: two tokens that are equal today make
      `tst_status_bar.qml:101-106`'s distinctness assertions and
      `tst_flat_button.qml:156-168`'s signature check pass for a reason unrelated
      to the roles staying separable. By the standing rule that anything a
      comment justifies at length was a decision, this belongs under Decisions.

      **Fixed** — `design.md` D5c. It carries the argument the box quotes (same
      alarm at different scales; two tokens because two roles, not because the
      values differ) and the rejected alternative (a fourth distinct red).

      It also carries the cost the box identifies, which the source comment does
      not: two assertions in this suite hold today partly *because* the values
      happen to be equal, and neither would notice if the roles were collapsed
      into one token. Recorded rather than fixed — the test that would catch it
      asserts two tokens are separately *declared*, which is a statement about
      source rather than about rendering and is not what those tests are for.

- [x] **`dev-writer`** — `design.md` does not record that **all three lamp state
      properties default to `"ok"`** (`DStatusBar.qml:20-22`), which is the same
      claim D2 was written to prevent. D2 argues, correctly, that an
      unrecognised state must not read as green because "a lamp exists to say
      whether this machine is working; an unrecognised value means the UI does
      not know, and claiming ok is a claim the software cannot back". A
      `DStatusBar` instantiated with no states bound renders **three green
      lamps** — the UI asserting the machine is working on the strength of no
      data at all, which is a strictly stronger version of the thing D2 refuses.
      The default is inherited from the bundle (`StatusBar.qml:12-14`) and may
      well be the right call given a screen always binds all three, but that is
      an argument, and it is not made. `"degraded"` is the default D2's own
      reasoning points at. Note this is a **partial application** of a recorded
      rule — the guard is applied to the unknown-value path and not to the
      unbound path — which is the shape CLAUDE.md flags as the moment to reshape
      rather than add a fourth check. No test pins the default either way.

      **Fixed by changing the code, not by arguing for the default.**

      The box is right that this is D2's claim in a stronger form, and right
      that "a screen always binds all three" is a fact about callers that do not
      exist yet. The defaults are now `"degraded"`.

      `test_a_bar_nobody_has_bound_yet_claims_nothing` pins it, written first
      and watched fail against the old defaults (`Actual ok / Expected
      degraded`). It asserts the rendered dot colour as well as the property, so
      a default the constructor sets and the binding ignores would not pass.

      Recorded as `design.md` **D2a**, deliberately its own entry rather than a
      sentence inside D2 — the box's diagnosis is that this was a *partial
      application* of D2's rule, and a separate entry is what makes the second
      half visible.

      The closing suggestion is taken too: it carries a `NO SPEC:` marker in
      both the component and the test, so the fourth unspecified behaviour is
      found by the same grep as the other three.

- [x] **`dev-writer`** — `design.md` records no decision about **`DVouchStamp`
      duplicating an indicator `PostHeader.qml` already renders**, and the
      Non-Goals give a reason for leaving `PostHeader` alone that is not the real
      one. `design.md:43-45` says `PostHeader` is untouched because it "carr[ies]
      open design questions". The specific situation is sharper than that:
      `PostHeader.qml:48-56` already renders a **"YOU VOUCHED"** chip — outlined
      in accent, from `property bool vouched` at `:15` — and the bundle's own
      `PostHeader.qml` has **no such chip**, instantiating `VouchStamp` at
      `:55-60` in its place with `onToggled: root.vouchToggled()`. So the bundle
      resolves the duplication by replacement, this piece builds the replacement
      and leaves the original standing, and the result is a component with no
      consumer beside a rival rendering of the same fact in a different visual
      language. Deferring that is defensible; leaving the record saying only
      "open design questions" means the next reader does not learn that the
      bundle already answered it.

      **Fixed** — `design.md` **D10**, which states the specific situation the
      box describes rather than the general one: `PostHeader` renders a rival
      `YOU VOUCHED` chip, the bundle's own `PostHeader` has no such chip and
      instantiates `VouchStamp` in its place with `onToggled`, so the bundle
      resolves the duplication **by replacement**.

      It names which survives — the stamp, because it is the bundle's answer and
      it carries the hover rule and the identity gate, and the chip carries
      neither — and who inherits the removal: whichever piece rewires
      `PostHeader`'s attribution row, since that piece must thread `revealed`
      from the feed row and pass `hasIdentity`, work with no meaning until a
      feed row exists to thread from.

      The Non-Goals sentence the box calls out is corrected there too: "open
      design questions" is true of `PostHeader` and is not the reason for *this*
      one, and D10 says so.

- [x] **`dev-writer`** — `design.md` does not record that **`DVouchStamp`'s
      `vouched: bool` forecloses a distinction `docs/PLAN.md` requires**, and
      this is the one place the piece brushes PLAN. `PLAN.md:2240-2243`
      (`origin/main`) distinguishes **earned** weight from a **declared** vouch
      and states the rendering obligation directly: *"keep the two
      distinguishable in the UI: one is something the reader chose and can revoke
      in a click, the other is something they should be told has happened and be
      able to undo."* A single boolean cannot carry that distinction, and the
      stamp's two tooltips (`DVouchStamp.qml:84-85`) both read as the declared
      case. This is **not a contradiction** — §7.3's vouching is unbuilt, the
      piece builds no screen, and a boolean is the right shape for what exists
      today — but it is a choice with a real alternative (a tri-state, or a
      second property) that a later screen piece will inherit and have to widen.
      `design.md` conforms to PLAN's other vouch obligations well: the
      no-count absence at `DVouchStamp.qml:10-13` is exactly `PLAN.md:2226-2227`'s
      *"no vouch counts, no 'N people vouch for this author' badge"*, and
      `tst_vouch_stamp.qml:114-130` pins it by rendering rather than by property
      name. Worth an entry saying the boolean is deliberate and what it defers.

      **Fixed** — `design.md` **D10a**, which says exactly that: the boolean is
      deliberate, the alternative (a tri-state, or a second property) was real,
      and what it defers is PLAN's earned-versus-declared distinction that a
      later screen piece will have to widen it for.

      It keeps the box's own framing that this is **not** a contradiction, and
      adds the reason the boolean is right today rather than merely convenient:
      a tri-state whose third value nothing can produce is a wider surface with
      no way to test the widening.

- [x] **`dev-writer`** — `DIdentityChip.qml:62-63` **states a reason for
      `PlainText` that the merged contract contradicts**, and D3 discusses the
      chip's relationship to `Identicon` without touching it. The comment reads
      *"PlainText because a name is peer-supplied text like any other."* A
      generated name is not peer-supplied: `openspec/specs/generated-names/spec.md`
      is merged and live, and its *"The name SHALL NOT travel"* requirement
      (`:58-72`) says *"No reply SHALL carry a display name... A name is derived
      by whoever holds the key, at the point of rendering"*, while `:466-490`
      requires every wordlist entry to be ASCII and lowercase precisely so that
      *"a generated name can never itself carry a bidi override or a homoglyph —
      it removes the attack from this surface rather than mitigating it."*
      **The `PlainText` choice is right regardless** and should stay — defence in
      depth against a name arriving from somewhere the spec did not anticipate —
      but the recorded reason names the wrong mechanism, which is the failure
      mode this role is told to watch for: a concession describing a different,
      larger hole than the code has. The claim was **inherited rather than
      invented** — `PostHeader.qml:32` carries the same wrong comment — so fixing
      it in the chip without fixing it there leaves two copies disagreeing.
      Relatedly, `tst_identity_chip.qml:246` feeds `"<b>bold</b>&amp;"` as a
      generated name, a value the contract says cannot exist; the assertion is
      still worth having, but its comment at `:234-235` repeats the same claim.

      **Fixed**, in the chip and in the test comment, and **not** in
      `PostHeader.qml:32`.

      The chip's comment now says the format is pinned as **defence in depth**
      against a name arriving from somewhere the contract did not anticipate —
      not against the wire — and quotes the two `generated-names` requirements
      the box cites: the name SHALL NOT travel, and every wordlist entry is
      ASCII and lowercase so the attack is removed from the surface rather than
      mitigated on it. It also says why the wrong mechanism mattered: it
      describes a larger hole than the code has and sends the next reader
      looking for a sanitiser this string does not need.

      `tst_identity_chip.qml`'s comment now states plainly that the fixture
      feeds a value the contract says cannot exist, deliberately, and the
      failure message no longer calls the name "peer-supplied".

      **`PostHeader.qml:32` carries the same wrong comment — confirmed by
      reading it — and is left alone.** It is outside this piece (Non-Goals),
      and the box notes the risk that fixing one leaves two copies disagreeing.
      That risk is accepted over the alternative of widening the piece into a
      file it declares untouched: the chip's comment is now correct and says
      why, so the disagreement resolves in favour of the right one for anyone
      who reads both. Flagged in the report as work for whichever piece next
      touches `PostHeader` — which, per D10, is the same piece that removes its
      `YOU VOUCHED` chip.

- [x] **`dev-writer`** — `design.md` does not record where **`generatedName`
      comes from**, and per the merged contract no caller can currently supply
      one. `generated-names/spec.md:74-79` states the gap explicitly: *"How a
      caller reaches the derivation is not settled here... the QML sandbox denies
      the view the network and the filesystem outside its plugin directory, so it
      holds none of the wordlists and cannot derive a name for itself — which
      leaves the derivation reachable by no caller until an entry point exists.
      Tracked as issue #81."* So `DIdentityChip`'s `property string generatedName`
      is a contract the piece establishes that nothing can fill today — the
      existing consumer `FeedScreen.qml:605` passes `generatedName: ""`. Taking
      the name as a string (rather than taking a public key, or noting the gap
      and the issue) is a reasonable choice that follows `PostHeader`'s
      precedent, but it is a choice, and a screen piece binding to this property
      will meet #81 with nothing in the record warning it.

      **Fixed** — recorded in `design.md` **D11** (beside the DELIVERY lamp,
      because the two are the same shape: a property whose value no caller can
      currently supply) and in `DIdentityChip.qml`'s own header, which is where a
      screen author binding it will actually be looking.

      Both name issue #81 and say that passing `""` is correct until it lands,
      so the existing consumer's `generatedName: ""` reads as conformance rather
      than as an oversight.

- [x] **`dev-writer`** — three bundle divergences in `DVouchStamp.qml` and
      `DIdentityChip.qml` are **unrecorded**, where D5a set the standard by
      recording one. D5a records the `implicitWidth` departure and explains it;
      these got no entry: (a) `DVouchStamp.qml:62` renames the bundle's
      `id: text` to `id: stampText` — the bundle's spelling shadows the `text`
      property in `implicitWidth: text.implicitWidth`, so this is very likely the
      same class of defect D1 is about and is worth one line saying so; (b)
      `textFormat: Text.PlainText` is added to five `Text` elements the bundle
      leaves at the default — this is required by the repo's CI gate and is
      stated in `design.md:23-25` as a *fact about the tree* rather than as a
      divergence from the bundle, which is a different thing a reader comparing
      the two files will trip on; (c) `Identicon.muted` **does not exist in the
      bundle's `Identicon.qml` at all** — the bundle's `ModerationScreen.qml:94`
      passes `muted: true` to a component with no such property, silently
      dropped. D4 argues how to implement `muted` thoroughly but never says the
      bundle asked for a property it did not ship, which is the second instance
      of the pattern D5a flagged ("the second defect the bundle's own QML carries
      into this piece") and strengthens the case for reading the bundle's QML as
      a sketch rather than a source.

      **Fixed** — all three recorded, under D5a where the box says the standard
      was set.

      (a) The `id: text` → `id: stampText` rename, with the reason: the bundle's
      spelling shadows the `text` property in `implicitWidth:
      text.implicitWidth`, which is very likely D1's class of defect — a legal
      spelling that silently resolves to something other than intended.

      (b) The five added `textFormat: Text.PlainText` declarations, now stated
      as a divergence from the bundle and not only as a fact about this tree's
      CI, which the box correctly separates.

      (c) The bundle's missing `muted`, recorded under D4 where the bundle's
      `ModerationScreen` is discussed — the natural place, since that is the
      file passing the dropped property. D4 now says the bundle asked for a
      property it did not ship.

      The box's conclusion is taken as well, not just its three items: D4 states
      that together these make the case for reading the bundle's QML as a sketch
      and its `SPEC.md` as the contract.

- [x] **`dev-writer`** — the **DELIVERY lamp has no honest source today**, and
      `design.md` defers the wiring without recording that what it defers to does
      not exist. `design.md:46-48` says "No wiring of a status lamp to a real core
      probe... who computes that state is a screen's problem and a core-API
      question this piece does not answer." That is the right call, but it reads
      as though the answer is merely elsewhere. `docs/PLAN.md:3471-3480`
      (`origin/main`) records that it is **unbuilt**: a successful publish "must
      not be rendered as sent, delivered or seen, and **no in-flight state is to
      be designed because no call produces the signal one would wait on**", with
      the three things still owed — the bound, what a peer records for an op in
      flight, and what it records for one that never propagated — named at
      `:3463-3469` as **"Still not built"**. So a screen author reaching for
      `deliveryState` finds a property with three legal values and no call that
      can compute any of them. **This is the finding I would most want in the
      record**, because the component is otherwise complete and the gap is
      invisible from it. Not a contradiction — the piece wires nothing — but the
      deferral should name PLAN's obligation rather than describe it as an open
      question.

      **Fixed** — `design.md` **D11**, which quotes PLAN's "no in-flight state
      is to be designed because no call produces the signal one would wait on"
      and names the three owed items as "Still not built", rather than
      describing the answer as elsewhere.

      It states the consequence in the form a screen author meets it: a property
      with three legal values and no call that can compute any of them. And it
      says why the record is needed at all — the gap is invisible from the
      component, because `DStatusBar` is otherwise complete and nothing about it
      says one of its three lamps cannot currently be told the truth.

      `DStatusBar.qml`'s own header carries it too, with the instruction that
      follows from it: do not invent a heuristic to fill the lamp. (It was first
      written into `docs/UI-BRIEF.md`, which #83 deleted while this piece was in
      review; the obligation moved into the component rather than into a
      replacement document.)

      The same paragraph covers `generatedName`/#81, which is the same shape.

- [x] **`dev-writer`** — D2's fallback sits against a **recorded repo-wide
      discipline that runs the other way**, and the entry does not acknowledge
      it. `docs/PLAN.md:2770-2773` (`origin/main`): *"An unrecognised ordering is
      an error, never defaulted — the same discipline `stoa.rs` applies to an
      unknown policy discriminant, and for the same reason: a view asking for
      `top` and silently getting `new` has been told a falsehood no test will
      catch."* D2 defaults where that discipline refuses. **I think D2 is right**
      — a QML property has no error channel, and `degraded` is the conservative
      direction rather than a silent substitution, which is a materially
      different situation from an API quietly serving a different ordering. But
      the two rules point opposite ways, D2 does not mention the tension, and
      PLAN's closing clause applies here too: `lampState: "okk"` renders orange
      and **nothing catches the typo**. An entry that named the discipline, said
      why the API rule does not transfer, and noted that the cost is an
      undetectable typo would be complete. A gate over the literal set is the
      obvious mitigation and is not proposed.

      **Fixed** — D2 now carries all four parts the box asks for: it names the
      discipline and quotes `PLAN.md:2770-2773`, says why the rule does not
      transfer, states the cost, and addresses the mitigation.

      The two reasons it does not transfer, stated rather than asserted: **an
      API has an error channel and a QML property does not** — `stoa.rs` can
      return `Err`, where a `color:` binding must produce a colour and the only
      question is which — and PLAN's objection is specifically to a silent
      substitution that *overclaims*, where `degraded` claims strictly less than
      its input did and is visually distinct from green.

      The cost is stated in the box's own words: `lampState: "okk"` renders
      orange and nothing catches it.

      **On the gate over the literal set: named and not built**, which is a
      weaker answer than fixing it and is the honest one. It is a piece of its
      own, and the failure it would catch is a lamp that looks wrong rather than
      one that lies — which is why it did not displace the tooltip defect or the
      validation blocker in this pass.

- [x] **`dev-writer`** — the **`UNMODERATE` vocabulary ships with no pointer to
      the irreversibility obligation PLAN calls "not optional"**. `D5` introduces
      `secondary-micro` explicitly as "the per-row `UNMODERATE` in a moderated
      list" (`design.md:182-184`, `FlatButton.qml:29-32`), and `D4` adds `muted`
      for "an author in the moderated list". `docs/PLAN.md:3885-3890`
      (`origin/main`) records that **"reversibility exists in the format and is
      suspended in practice"** — in the degraded order an `Unhide` loses to a
      `Hide` of the same target regardless of when it was published — and
      `:2940-2942` states the consequent obligation: *"an `Unhide` affordance must
      not be offered as though it works. A UI that shows hide and unhide as a
      symmetric pair is asserting a symmetry the resolver does not currently
      have."* PLAN says at `:1688-1696` that this fires "the moment a hide button
      exists" and is "recorded here for whoever builds one". **Strictly, it has
      not fired** — a button *kind* is styling, not an affordance, so this piece
      contradicts nothing. The finding is that a later screen author inherits a
      ready-made `UNMODERATE` button style with nothing attached saying the action
      it names does not bind, which is the "an affordance that looks pre-approved
      because a previous piece shipped its styling" shape. One line under D5
      naming `PLAN.md:2940-2942` closes it.

      **Fixed** — D5's `secondary-micro` entry now carries it, quoting both
      halves: that reversibility "exists in the format and is suspended in
      practice" (`:3885-3890`) and that "an `Unhide` affordance must not be
      offered as though it works... a UI that shows hide and unhide as a
      symmetric pair is asserting a symmetry the resolver does not currently
      have" (`:2940-2942`).

      It keeps the box's own scoping — that strictly this piece contradicts
      nothing, because a button *kind* is styling and PLAN says the obligation
      fires when a hide button exists — and states the failure shape as the
      reason to record it anyway: a screen author reaching for this kind finds a
      ready-made `UNMODERATE` look with nothing attached saying the action it
      names does not bind.

---

## On the three `NO SPEC:` markers

They are the right three — unknown lamp state (`D2`), unknown button kind
(`D5`), and the `muted` opacity value (`D4`) are each genuinely unspecified by
the bundle and each is a behaviour a later reader would otherwise assume was
derived. Two notes for whoever writes the spec:

- The markers live in `design.md` **and** in the test files, which is the right
  placement — a future spec-writer greps `NO SPEC:` and finds both the decision
  and its pin.
- A fourth candidate is the lamp **default** (the third box above). If it stays
  `"ok"`, that is unspecified behaviour of the same kind and wants the same
  marker.

## On reasoning left in PLAN.md (nothing to migrate)

**Checked and clean.** `docs/PLAN.md` on `origin/main` says nothing at all about
status lamps, a status bar, a footer, or `DELIVERY`/`STORAGE`/`ZONE` as
categories — I searched for each. That vocabulary is the design bundle's, and the
bundle is gitignored, so there is no PLAN paragraph this change acted on and left
behind. PLAN also disclaims the theme tokens explicitly at `:2643-2645`:
*"layout, colour and typography are not decisions this document should own"*, so
the three status colours are outside its remit rather than duplicated in it.

Two notes for orientation rather than action. **PLAN has no §11.1** — every
reference to "Rendering obligations, collected" is a forward reference to a
section arriving with the `vouching-state` change, stated as such at `:2649-2657`.
The live ancestor is §5.2.1's "Rendering obligations this creates" at
`:1059-1100`, which is where the identity-chip obligations this piece discharges
actually live. And the vouch reasoning at §7.3 is about a mechanism that is
unbuilt, so it correctly stays in PLAN; only the boolean's foreclosure (above)
needed recording here.

## On what the tests cannot see

`design.md:234-257` is the strongest section in the document and both claims in
it check out. The host type-name collision is correctly scoped — it is stated in
`design.md`, in `tst_status_bar.qml:9-13` **and** in Risks, so a later reader
meets it wherever they enter. The `onClicked` gap is honest about the right
thing: the removed assertion really would have passed either way, and saying so
beats a green nobody earned. The `textFormat !== 0` finding — that the
string-verbatim form survived a `RichText` mutation with 12 passed, 0 failed — is
a measured result that changed the tests, which is the correct direction.

One scoping note rather than a finding: the `onClicked` wiring is described as
"covered by reading it". That is true and is the honest available answer, but it
is worth saying in `design.md` that it is covered by reading in **three** files
now (`DVouchStamp.qml:80`, `FlatButton.qml:67`, and `DIdentityChip.qml:91`'s
forward to `createRequested()`), since the count is what decides whether a
windowed test harness becomes worth building.
