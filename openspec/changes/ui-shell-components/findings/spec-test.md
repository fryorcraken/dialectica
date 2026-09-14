# spec-test review — `ui-shell-components`

**I did not read the implementation** except for the lines I mutated in part 2,
and no further than those lines. Judgements below come from `SPEC.md`,
`copy.json`, `design.md`, `tasks.md` and the five test files.

No spec-writer ran for this piece, so the contract judged against is the design
bundle at `tmp/ui-bundle-new/handoff/` — `SPEC.md` for behaviour, `copy.json`
for strings — as `tasks.md` states.

## Findings

- [x] **`tester`** — `dialectica-ui/tests/tst_vouch_stamp.qml`, the whole file
      **Spec clause unpinned:** `SPEC.md:88` — *"It is not drawn at all while
      this machine has no identity."* This is one of the vouch stamp's three
      specified behaviours and no test asserts it. `DVouchStamp` has exactly two
      properties (`vouched`, `revealed`) and the string `hasIdentity` appears
      nowhere in either the component or its test — measured with `grep`.
      **Scenario:** a screen binds the stamp into a post row without gating on
      `hasIdentity`. A reader with no identity sees VOUCH prompts on hover for
      an action they cannot take, and the whole suite stays green: the four
      `(vouched, revealed)` cases in
      `test_the_visibility_rule_over_all_four_states` are complete over the two
      properties that exist, so nothing notices the third condition is missing.
      Either the stamp takes the gate (and a test pins it), or `design.md`
      records that the gate is the caller's and names which piece owes it — at
      present neither document mentions the clause. **Severity: medium** — a
      silent gap in a spec obligation, not a defect in what was built.

      **Fixed** in `648b462`, taking the first of the two options: **the stamp
      takes the gate.**

      The reason to put it in the component rather than the contract is this
      box's own scenario — "a screen binds the stamp into a post row without
      gating on `hasIdentity`". A stamp is placed by every post row in every
      feed, so a contract saying "gate this yourself" must be got right at each
      of those sites and is silent when it is not.

      It defaults **`false`**, so the forgetful caller this box describes gets
      no stamp rather than the prompts it names. Three tests pin it and all
      three failed first:

      - `test_no_identity_means_no_stamp_in_any_state` drives **all four**
        `(vouched, revealed)` combinations under `hasIdentity: false`, not only
        the hovered one — because the existing sweep is complete over the two
        properties that existed, so a gate applied to one arm only would pass
        three of four and be caught by nothing else. Proved: the mutation
        `vouched || (hasIdentity && revealed)` fails it on the
        `vouched=true revealed=false` case.
      - `test_an_identity_restores_the_ordinary_rule` is the converse, so
        `opacity: 0` cannot satisfy the gate.
      - `test_the_identity_gate_defaults_closed` builds through the factory
        rather than the file's helper, which supplies `hasIdentity: true` on
        purpose so the other tests assert what their names say.

      Recorded as `design.md` D9, and stated in `docs/UI-BRIEF.md` for the
      screen author who must still pass the property.

- [ ] **`spec-writer`** — `SPEC.md:133`, *"Green and orange appear nowhere else
      in the design"*
      **Untested SPEC obligation.** This is a whole-codebase invariant and no
      test or gate enforces it. Measured: `grep -rn "statusOk\|statusDegraded\|
      #4f6b3a\|#b5731f" dialectica-ui/src/qml/` returns only `DTheme.qml`'s
      declarations and `DStatusBar.qml`'s three uses, so the tree is correct
      **today**. Nothing keeps it correct.
      **Scenario:** a later screen piece writes `color: "#4f6b3a"` for a success
      badge. Every test in this piece and every gate passes; the one signal the
      interface reserves for "is this machine working" has been spent elsewhere,
      which is precisely what the `DTheme.qml:65-69` comment says must not
      happen. The comment is the only enforcement, and a comment is not a gate.
      A static sweep (the shape of `check_qml_names.py`) is the fitting answer,
      and it is a piece of its own rather than this one's work — but it needs
      recording somewhere that outlives this PR. **Severity: low** for this
      piece, **medium** for the suite.

      **`dev-writer` note — this box is addressed to `spec-writer` and is left
      OPEN.** It asks for a static sweep that is a piece of its own, and I have
      not written one, so ticking it would claim work nobody did.

      What this pass did do, recorded so the next reader does not re-measure it:
      the **prose** the box says is the only enforcement has been corrected
      where it was false. Both `DStatusBar.qml` and `DTheme.qml` claimed green
      and orange appear nowhere else "in the design", which `markGreen` and
      `markSage` disprove in the very file cited; both now scope the claim to
      the interface palette and say why the marks are outside the rule rather
      than exceptions to it. That makes the comment true, which a sweep would
      need it to be — it does not make the comment a gate, and this box's point
      stands unchanged.

- [ ] **`spec-writer`** — the six `copy.json status.tooltips` strings
      **No test checks them against the bundle.** `DStatusBar` takes
      `deliveryText`/`storageText`/`zoneText` as caller-supplied properties, and
      `tst_status_bar.qml:257` asserts only the negative — that an unset
      explanation invents nothing, which is right and worth having. But
      `SPEC.md:132` requires each lamp carry *"a tooltip that says what the
      state means for this machine (strings in copy.json)"*, and after this
      piece merges no document says who owes those six strings or that they must
      be verbatim.
      **Scenario:** the footer-wiring screen paraphrases `deliveryNoPeers` as
      "No peers" — losing *"you have joined nothing yet"*, which is the half
      that distinguishes an empty feed from an unreadable store, the exact
      distinction `SPEC.md:136-137` exists to force. Nothing fails.
      The delegation itself is a sound design call (`design.md` Non-Goals is
      explicit that state computation is a screen's problem); what is missing is
      the record that the *strings* travel with it. **Severity: low.**

      **`dev-writer` note — addressed to `spec-writer`, left OPEN, but the
      record this box asks for now exists.**

      `docs/UI-BRIEF.md`'s new *three shared components* section states it in
      the terms this box uses: the six `copy.json status.tooltips` strings
      travel with the screen that computes the states, and must be **verbatim**
      — with this box's own reason, that losing "you have joined nothing yet"
      from `deliveryNoPeers` costs the distinction between an empty feed and an
      unreadable store.

      Left open because the box asks a spec-writer whether that obligation
      belongs in a spec rather than a brief, and that is a question about the
      contract, not about this piece. The brief is where a screen author looks;
      whether it is also where the requirement should *live* is the judgement I
      am not making on their behalf.

## The three `NO SPEC:` markers — my read on each

Asked for explicitly, so answered in prose rather than as boxes; none of the
three is a defect and none blocks the merge.

**Unrecognised lamp state → `degraded` (`tst_status_bar.qml:41`). Should be
specified.** This is not an implementation detail — it is a claim the interface
makes about the machine, and it is the one place in this piece where the choice
has a safety direction. `SPEC.md`'s Tone section (*"never claiming more than the
software delivers"*) already implies it, so the spec is not silent so much as
unspecific, and a spec-writer can capture it in one sentence. The test is the
strongest in the piece: it sweeps thirteen unrecognised values and asserts both
the normalised string **and** the rendered colour.

**Unrecognised button `kind` → `secondary` (`tst_flat_button.qml:172`).
Implementation detail, but worth a line in the spec.** Which kind it falls back
to is genuinely arbitrary. The invariant underneath it is not: a button must be
visible. That half deserves specifying — `design.md` D5 records that the *old*
behaviour produced an invisible control that still accepted clicks, and
`test_every_kind_is_either_filled_or_outlined` pins the general rule over all
five kinds rather than just the fallback. Specify "no kind renders invisibly";
leave the choice of `secondary` to the code.

**`markMutedAlpha` = 0.45 (`tst_identicon_muted.qml:49`). Implementation detail,
correctly a theme token.** A number chosen for how it reads on this paper is a
theme question, and `design.md` D5b's argument for keeping it out of the
determinism contract is sound — it changes how visible a mark is, never which
shape or inks it selects. No spec change needed.

One wording correction for the design reviewer rather than a box: `design.md`
D4 says the test "asserts the *relation* … rather than only pinning the literal,
so the test still means something if the number is retuned". The relation does
survive a retune; **the test does not**. Measured — setting `markMutedAlpha` to
0.6 fails `test_a_muted_mark_recedes_without_disappearing` at line 68 on the
hardcoded `compare(dim.opacity, 0.45)`. Pinning the literal is the right call
for an unspecified value, so this is a sentence that overstates what was built,
not a test that should change.

**Taken, and the sentence changed rather than the test.** D4 now says the test
asserts the relation *as well as* pinning the literal, and states plainly that
the relation survives a retune while the test does not — with the reason
pinning the literal is still right: it makes a retune a deliberate edit to a
test rather than a silent drift.

## Mutations run

Eleven mutations, one at a time, each restored before the next. Baseline and
final state both **18 specs, 332 passes, 0 failures**; `git status --porcelain`
empty after the last restore, confirming the tree is as the author left it.

| # | Mutation | Result |
|---|---|---|
| A1 | `normalisedState`: restore the obvious ternary (unknown → `"ok"`) | **2 fail** — the unknown-state sweep and the case-sensitivity test |
| A2 | `lampColor`: swap `statusFailed` / `statusDegraded` | **4 fail** |
| A3 | `lampColor`: read `DTheme.statusDegradedTYPO` | **3 fail** — the repair holds |
| A4 | Swap the STORAGE and ZONE lamps' declaration order | **3 fail**, incl. the order test |
| B1 | `opacity: revealed ? 1 : 0` (drop the `vouched` disjunct) | **2 fail** |
| B2 | `opacity: vouched ? 1 : 0` (drop the `revealed` disjunct) | **2 fail** |
| B3 | `FlatButton.spec`: restore the invisible-button fallback | **1 fail** |
| B4 | `secondary-micro` given `secondary`'s font and padding | **2 fail** |
| C1 | Chip's mark and CURRENT IDENTITY forced `visible: true` | **2 fail** |
| C2 | Chip's name `Text` → `textFormat: Text.RichText` | **1 fail** — the repair holds |
| C3 | Re-add `isPerson` to `Identicon` | **1 fail** |
| D1 | `_inkA()` made dependent on `muted` (the rejected design D4-(1)) | **1 fail** |
| D2 | `markMutedAlpha` retuned 0.45 → 0.6 | **1 fail** (see note above) |

**No mutation survived.** Every test I mutated the named property of failed for
the reason its name gives.

Two of these were the author's own reported repairs, and both are confirmed
rather than taken on trust:

- **A3** — `design.md` reports the colour test previously survived a token
  rename because both sides read `undefined`. Under A3 it now fails, and the
  failing line is 93, the **hardcoded literal**, not the token comparison. That
  is exactly the mechanism claimed: the token-vs-token check still passes
  `undefined === undefined`, and the literal is what catches it. The three-way
  assertion (token, literal, relation) is doing real work, not decorating.
- **C2** — `design.md` reports the markup test previously survived a `RichText`
  mutation. Under C2 `test_every_text_the_chip_renders_is_plain_text` fails,
  while `test_the_name_is_rendered_verbatim` **passes** — confirming both that
  the rewrite works and that the old assertion could not have caught this. The
  author was right to keep the verbatim check as a separate, differently-named
  test rather than delete it; it pins a different property (no elision, no
  "cleaning") and is honest about not covering format.

## Coverage: bundle forward to tests

Walked `SPEC.md` clause by clause for the behaviour this piece builds.

**Covered, and pinned by a test that can fail:** the three lamps in
DELIVERY/STORAGE/ZONE order, asserted as a sequence rather than a set (A4 kills
it); each of ok/degraded/failed mapping to its own distinct colour, three ways
(A2, A3); the vouch stamp outlined-and-hover-revealed when un-vouched and
filled-rotated-persistent when vouched, as a disjunction whose both halves are
tested (B1, B2); the identity chip's two arms including the absence half (C1);
`copy.json` strings used verbatim.

**Verbatim check, done string by string against `copy.json`:** `VOUCH`,
`VOUCHED`, `CURRENT IDENTITY`, `Create an identity`, `Voting, posting and
replying need an identity.`, and both vouch tooltips — all exact, em-dash
included. No paraphrase found. The tests hardcode the expected strings rather
than reading them off the component, which is the right shape.

**Correctly out of scope, and said so:** footer layout, lamp-state computation,
`AddressLabel` abbreviation (`SPEC.md:31-34` requires it be implemented once in
`AddressLabel.qml`; the chip delegates, which conforms).

## What was clean

The author's two stated blind spots are stated accurately and neither test
claims coverage it lacks. `test_toggling_emits_a_signal_the_owner_interprets`
and `test_clicking_emits_the_signal` both carry comments saying they bypass the
`MouseArea`, and neither asserts anything about `onClicked` — the removed
"the stamp does not flip its own `vouched`" assertion would indeed have passed
either way, and removing it was the correct call. The host-collision disclaimer
in `tst_status_bar.qml:9-13` is likewise precise about what
`check_qml_names.py` covers and what the spec file does not.

Several tests exist specifically to close the "two explanations, one answer"
shape and do so genuinely, verified by mutation:
`test_the_three_named_states_are_returned_unchanged` (blocks a
`normalisedState` that returns `"degraded"` for everything),
`test_a_mark_is_fully_opaque_by_default` (blocks `muted: true` as the default),
`test_each_lamp_reads_its_own_state_property` (blocks all three dots wired to
one property), `test_a_state_change_repaints_its_lamp` and
`test_losing_an_identity_switches_the_chip_back` (block read-once-at-construction).
`test_muting_changes_no_selector_for_any_address` sweeps twelve addresses rather
than one, which is the right answer to a branch that could be keyed on a byte
value.

`tasks.md` 1.2's claim that `apparatusWidth` was already absent at the branch
point is true — `git log -S apparatusWidth` names `3901e99` (#70), which is this
branch's base. Recording it rather than silently dropping the task was right.
