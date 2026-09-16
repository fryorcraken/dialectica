# The shell three screens sit on

## Context

A new UI design bundle (`tmp/ui-bundle-new/handoff/`, gitignored) is the
authority for the interface. It ships eleven QML files, of which the impl
already carries eight in some form. This piece takes the three the impl does
not have — `StatusBar`, `VouchStamp`, `IdentityChip` — plus the theme tokens
and two button kinds they and the moderation screen need.

**It builds no screen.** Screens are separate pieces that depend on this one,
so the components have to be complete and correct without a screen to exercise
them. That is the constraint shaping every decision below: a component whose
only proof of correctness is "the screen that uses it looks right" has no proof
here, because that screen does not exist yet.

Three facts about this tree that the design answers to:

- Basecamp registers its own QML types in the host's C++ registration, so a
  name it also uses wins and every property read off it is `undefined` at
  runtime with no error. `dialectica-ui/tests/check_qml_names.py` enforces a
  `D` prefix. Every new type here is `D`-prefixed.
- CI counts `Text {` openings against `textFormat:` assignments per file, and
  QML's default `Text.AutoText` sniffs its input and renders markup found in
  peer-supplied strings. Every `Text` gets an explicit `textFormat`.
- `check_qml_members.sh` runs qmllint with `-W 0`, so **any** qmllint warning
  in `src/qml` fails the build, not only a missing member.

## Goals / Non-Goals

**Goals**

- `DStatusBar`, `DVouchStamp`, `DIdentityChip` exist, are registered, and each
  is provably correct on its own without a screen.
- `DTheme` carries the three status colours, and no longer carries a token with
  no consumer.
- `FlatButton` answers the two kinds the moderation screen will pass it.
- `Identicon` answers `muted` without disturbing a drawing channel.

**Non-Goals**

- No screen. Not `ModerationScreen`, not `FeedScreen`'s footer wiring.
- `ScreenFrame`, `VoteControl` and `PostHeader` are untouched. The first is
  strictly stronger than the bundle's and pinned by
  `tst_screen_frame_geometry.qml`; the other two carry open design questions.
- No wiring of a status lamp to a real core probe. The component takes its
  state as a property; who computes that state is a screen's problem and a
  core-API question this piece does not answer.
- The sanitiser stays. `copy.json` dropped its three strings, but SPEC.md still
  requires sanitisation in prose and it is a standing security rule — the
  omission reads as an oversight in the bundle, not a withdrawal.

## Decisions

### D1. The lamp's state property is `lampState`, not `state`

The bundle writes `property string state: "ok"` on a `Rectangle`.
`QQuickItem` already has a `state` property — the name of the currently active
`State` in `states` — and redeclaring it is legal QML that silently shadows the
built-in.

**Measured, Qt 6.10.3.** A probe declared `property string state` on a
`Rectangle` alongside `states: State { name: "degraded"; PropertyChanges {…
marker: 99 } }`, then set `state: "degraded"`. The component instantiated, the
property held `"degraded"` — and `marker` read **0**, not 99. The state machine
never saw the assignment. No error, no warning, no binding loop.

So the bundle's spelling works today and disables `states`/`transitions` on
every lamp, invisibly, for whoever reaches for them next. `lampState` costs one
word and takes the trap off the table. The rejected alternative was keeping
`state` on the grounds that "we will never use QML states here" — that is a
fact about today with no expiry date, which is the shape this repo has
repeatedly paid for.

The three outer properties stay as the bundle names them
(`deliveryState`/`storageState`/`zoneState`): those are on a `RowLayout` root
and collide with nothing, and renaming them would break the bundle's own
vocabulary for no gain.

### D2. Lamp state is validated, not trusted

`lampState` is a free string. The bundle's ternary chain treats *anything* that
is neither `"failed"` nor `"degraded"` as ok — including `""`, `"OK"`, a typo,
and a value that reached the UI from core.

Reading a typo as **green** is the wrong direction to fail. A lamp exists to
say whether this machine is working; an unrecognised value means the UI does
not know, and claiming ok is a claim the software cannot back — which is the
one thing SPEC.md's Tone section forbids.

So an unrecognised state renders `degraded`, and the choice is
`DStatusBar.normalisedState()`, one function, used by all three lamps. It is a
function rather than four inline ternaries so a test can drive it directly and
so a fourth caller inherits the rule instead of re-deriving it.

`NO SPEC:` this is unspecified behaviour. The bundle enumerates three states
and says nothing about a fourth. `tst_status_bar.qml` marks it.

**This runs against a repo-wide discipline that points the other way, and the
tension is real rather than apparent.** `docs/PLAN.md:2770-2773`: *"An
unrecognised ordering is an error, never defaulted — the same discipline
`stoa.rs` applies to an unknown policy discriminant, and for the same reason: a
view asking for `top` and silently getting `new` has been told a falsehood no
test will catch."* D2 defaults where that refuses.

The rule does not transfer, for two reasons. **An API has an error channel and a
QML property does not** — `stoa.rs` can return `Err`, where a `color:` binding
must produce a colour and the only question is which. And PLAN's objection is to
a *silent substitution that overclaims*: `top` silently serving `new` tells the
caller it got what it asked for. `degraded` claims strictly less than the input
did, and is visually distinct from green, so the lamp withholds an assurance
rather than fabricating one.

**The cost is that `lampState: "okk"` renders orange and nothing catches the
typo**, which is exactly PLAN's closing clause applied here. That is accepted
rather than solved: the alternative is a gate over the literal set, which is a
piece of its own — and the failure it would catch is a lamp that looks wrong,
not one that lies.

### D2a. The lamp state properties default to `degraded`, not `ok`

The bundle defaults all three to `"ok"`, so a `DStatusBar` with nothing bound
renders **three green lamps**.

That is D2's claim in a stronger form than the one D2 refuses. An unrecognised
state degrades because green is a claim the software cannot back; a bar with no
state bound has *less* to back one with, not more. And it is not a hypothetical
state — it is what every screen holds between appearing and core answering the
call, so it is the first thing a user sees.

Recorded as its own decision because the original was a **partial application of
D2's own rule**: the guard was on the unknown-value path and not on the unbound
one, which is CLAUDE.md's signal to reshape rather than to add a fourth check.

The argument for keeping `"ok"` — that a screen always binds all three, so the
default is unreachable — is a fact about callers that do not exist yet, and it
is the shape this repo has repeatedly paid for.

`NO SPEC:` the bundle is silent on what an unbound lamp claims.
`tst_status_bar.qml::test_a_bar_nobody_has_bound_yet_claims_nothing` marks it.

### D3. `DIdentityChip` drops `isPerson`

The bundle passes `isPerson: true` to `Identicon`. The impl's `Identicon` has
no such property and deliberately: the comment above `_form()` records that the
angular/curved split halved the vocabulary any one address could reach and was
restating what position already says, so the distinction "is now carried by
POSITION ALONE".

(Cited by the phrase rather than a line range. The first draft said
`Identicon.qml:95-101`, which is the tail of the `inks` array — the text is at
114-120. A pinned range rots on any edit above it and cannot fail loudly, which
is the shape CLAUDE.md's self-invalidating rule warns about.)

Passing an undeclared property to a QML component is not an error — it is
silently dropped at instantiation — so re-adding it would have cost nothing
visible and quietly reintroduced a rejected design. Dropped.

The obligation `Identicon.qml` attaches to that decision is real and lands on
this piece: *"if a mark is ever rendered somewhere the context does not
disambiguate, that placement must label it."* `DIdentityChip` labels it —
`CURRENT IDENTITY` sits beside the mark, from `copy.json common.currentIdentity`
verbatim. The chip is therefore a conforming placement rather than an exception
to the rule.

### D4. `muted` is an opacity, applied at the root, outside `onPaint`

**The bundle's** `ModerationScreen.qml` wants a de-emphasised mark for an author
who is already moderated. It is not a file in this tree — it lives under the
gitignored `tmp/ui-bundle-new/handoff/`, and Non-Goals says this piece does not
build it, so an unqualified citation sends a reader looking for a file that does
not exist.

**And the bundle asks for a property it did not ship.** Its own `Identicon.qml`
has no `muted` at all, so its `ModerationScreen` passes `muted: true` to a
component that silently drops it. That is the second time the bundle's QML
carries a defect into this piece (D5a is the first, and D1 the shadowing), and
together they are the case for reading the bundle's QML as a sketch and its
`SPEC.md` as the contract.

Three ways to implement it:

1. desaturate the inks inside `onPaint`;
2. draw a paper-coloured scrim over the finished mark;
3. set `opacity` on the `Canvas` root.

(1) is rejected outright. The ink selectors are the mark's determinism
contract — `tst_identicon.qml` pins their indexing precisely because a change
there alters every identity in the system — and a `muted` branch inside
`_inkA()` would make what a mark looks like depend on the *context* it is drawn
in. Two peers rendering the same person in different lists would disagree.

(2) is more code than (3) and gets the compositing subtly wrong where the mark
overlaps a non-paper ground.

(3) is one line, touches no drawing channel, and cannot change *which* shape or
*which* inks are selected — which is exactly the part the contract covers.
`opacity: muted ? 0.45 : 1`. `onPaint` is not re-entered at all, so
`tst_identicon.qml`'s selectors are untouched by construction rather than by a
promise.

`NO SPEC:` the bundle gives no value for how muted "muted" is. 0.45 is chosen
to read as clearly recessive while leaving the contour visible, and the test
marks it as a choice. It asserts the *relation* — muted is strictly less opaque,
and every selector is unchanged — **as well as** pinning the literal.

**The relation survives a retune; the test does not.** An earlier draft of this
paragraph said the test "still means something if the number is retuned", which
overstates it: setting `markMutedAlpha` to 0.6 fails
`test_a_muted_mark_recedes_without_disappearing` on the hardcoded
`compare(dim.opacity, 0.45)`. Pinning the literal is the right call for a value
nothing specifies — it makes a retune a deliberate edit to a test rather than a
silent drift — but the sentence describing it should say so.

### D5. The two new button kinds are declared as data, not as two more ternaries

`FlatButton` chose colour, border and padding with three parallel ternary
chains keyed on `kind`. Adding two kinds would have made each chain five deep,
and the fourth slightly-different guard is this repo's stated signal to reshape.

So `kind` indexes one `readonly property var kinds` object holding each kind's
`fill`, `stroke`, `textInk` and padding pair, and the bindings read fields off
it. Adding a sixth kind is then one entry, not four edits in four places that
must agree — and a kind that is missing a field is visible in one place rather
than as a ternary that silently falls through to `secondary`.

**The table is public on purpose**, and that is now load-bearing: the test
derives its sweep list from `Object.keys(kinds)` rather than restating the five
names, so a sixth kind is covered the moment it is added. An earlier version
restated them, which is the `hand-maintained sweep lists go stale silently`
trap — and it defeated the stated benefit above, since the array was a fifth
place that had to agree.

**Two wrinkles on that seam are deferred rather than unnoticed**, both measured
by review: `readonly` on a `var` freezes the reference and not the object, so a
consumer can mutate an entry; and the literal is re-created per instance rather
than shared, so every button allocates six objects it never writes to. Neither
costs anything today — nothing in the tree mutates `kinds`, and the allocation
matters only in a long list. The fix is a singleton or file-scoped `QtObject`
that must *still* be reachable for the sweep, which makes it a design question
rather than a mechanical change, and it is worth answering when the
moderated-author list with a per-row `UNMODERATE` exists to feel it. Recorded
in `FlatButton.qml` beside the table as well as here.

**An unknown `kind` now resolves to `secondary`, and that is a behaviour
change** — recorded because the first draft of this note claimed the reshape
preserved behaviour, and tracing the old chains disproved it. Under the old
form `filled` was `kind !== "secondary"`, so an unknown kind was `filled`: no
border, no fill (neither colour branch matched), paper-coloured text. A typo'd
kind rendered as **paper text on nothing** — an invisible button that still
accepted clicks.

That is the worst available failure, so the fallback is deliberate rather than
incidental: a typo should look wrong, not look absent. Because it changes
behaviour, the reshape and the two new kinds land as one commit with this
reasoning rather than as a refactor commit claiming to change nothing.

`NO SPEC:` the bundle does not say what an unknown kind does; the test marks
it.

What the two new kinds are:

- `destructive-outline` — accent border and accent text on no fill. The
  moderation screen puts it beside a filled `destructive`, and two filled red
  buttons side by side would read as one decision offered twice; the outline
  says "also destructive, and the secondary of the two".
- `secondary-micro` — `secondary` at label type and tighter padding, for the
  per-row `UNMODERATE` in a list. A full-size secondary in a 19px list row
  would out-weigh the row it acts on.

  **Whoever builds that row inherits an obligation this piece does not
  discharge.** `docs/PLAN.md:3885-3890` records that moderation reversibility
  "exists in the format and is suspended in practice" — in the degraded order an
  `Unhide` loses to a `Hide` of the same target regardless of when it was
  published — and `:2940-2942` states the consequence: *"an `Unhide` affordance
  must not be offered as though it works. A UI that shows hide and unhide as a
  symmetric pair is asserting a symmetry the resolver does not currently have."*

  Strictly this piece contradicts nothing: a button *kind* is styling, and PLAN
  says the obligation fires "the moment a hide button exists". It is recorded
  here because the failure shape is an affordance that looks pre-approved
  because a previous piece shipped its styling — a screen author reaching for
  `secondary-micro` finds a ready-made `UNMODERATE` look with nothing attached
  saying the action it names does not bind.

### D5d. The kind lookup asks `hasOwnProperty`, not `!== undefined`

**Added after review; the first form had a hole the shape of `Object.prototype`.**

`kinds[kind] !== undefined` is a bracket lookup, and a bracket lookup walks the
prototype chain. `kind: "constructor"` resolves to `Object.prototype.constructor`
— a `Function`, not `undefined` — so the guard passed, `spec` became that
Function, and every field read off it was `undefined`.

**Measured, Qt 6.10.3.** All seven inherited member names (`constructor`,
`toString`, `valueOf`, `hasOwnProperty`, `__proto__`, `isPrototypeOf`,
`propertyIsEnumerable`) render `#ffffff` fill, black ink, `implicitWidth` and
`implicitHeight` both `NaN`, with four `Unable to assign [undefined]` warnings
per instance — against `secondary`'s `#00000000` fill at 60×35.

That is **the exact failure D5 exists to close, reached through a different
door**: an invisible control that still accepts clicks.

**`hasOwnProperty` asks the question that was always meant** — does this table
have this key of its own — where `!== undefined` was a proxy for it that is
wrong for seven strings. `Object.create(null)` would also work and is the purer
data-structure answer, but a QML object literal cannot be given a null
prototype without building it in an initialiser, which trades a clear one-liner
for a block that does the same job less legibly.

**Latent, not live.** Every `kind:` in the tree is a string literal today. It
becomes live the moment one is bound from a model field or from core — which is
precisely when nobody is looking at this line.

The old corpus was seven plain typo strings and could not see it. The prototype
names are now beside them, and the test fails on the pre-fix code at
`'constructor' does not fall back to secondary's fill`.

### D5c. `statusFailed` is the same value as `accent`, and they stay separate tokens

`DTheme` declares `statusFailed: "#a33a2b"` and `accent: "#a33a2b"`. The
alternative was a fourth distinct red for the failure lamp.

Rejected: a failed lamp and a destructive action are the same alarm at different
scales, and two nearly-identical reds on one screen read as a distinction the
design does not intend.

**They are two tokens because they are two ROLES**, not because the values
differ — retuning the destructive red must not silently retune the failure lamp.
That is the whole content of the decision, and it is why "just use `accent`" is
wrong even though it would render identically today.

**The cost is that two assertions pass for a reason unrelated to what they
check.** `tst_status_bar.qml`'s distinctness checks and `tst_flat_button.qml`'s
signature check both hold today partly because the values happen to be equal; if
the roles were ever collapsed into one token, neither would notice. Recorded
rather than fixed: the test that would catch it is one asserting two tokens are
*separately declared*, which is a statement about source rather than rendering
and is not what those tests are for.

### D5a. The lamp dot uses `implicitWidth`, departing from the bundle

The bundle's `Lamp` sets `width: 7; height: 7` on a `RowLayout` child. That is
undefined behaviour — qmllint reports it as `Quick.layout-positioning`, and
`check_qml_members.sh`'s `-W 0` turns it into a build failure. Found by running
the gate, not by reading.

Worth recording because it is the second defect the bundle's own QML carries
into this piece (the `state` shadowing is the first), and because the gate that
caught it is one whose comment warns it "fires wider than its name" — this is
that firing, and it was a real defect rather than a false positive.

**Three further departures from the bundle, recorded because D5a set the
standard and a reader comparing the two files will meet them:**

- **`DVouchStamp` renames the bundle's `id: text` to `id: stampText`.** The
  bundle's spelling shadows the `text` property in `implicitWidth:
  text.implicitWidth`, which is very likely the same class of defect as D1 —
  a legal spelling that silently resolves to something other than intended.
- **`textFormat: Text.PlainText` is added to five `Text` elements the bundle
  leaves at the default.** This is stated in Context as a fact about the tree's
  CI; it is also a divergence from the bundle, which is a different thing.
- **The bundle's `Identicon.qml` has no `muted`**, so its own
  `ModerationScreen.qml` passes a property that is silently dropped. Argued
  under D4.

### D5b. `markMutedAlpha` is a `DTheme` token, not a literal in `Identicon.qml`

The muted opacity sits in the theme because it is a theme question — how loud
"recessive" reads on this paper — and deliberately *not* beside the `mark*`
inks' frozen-constant warning, which it would otherwise look like a member of.
It is not part of the determinism contract: it changes how visible a mark is,
never which shape or inks it selects, so two peers on different versions of this
number still agree about who a mark depicts.

### D6. `apparatusWidth` goes, and two comments stop describing a deleted column

**Already done, by `#70`, before this branch existed.** The dispatch brief said
the impl "still declares it at roughly `DTheme.qml:161`"; it does not.
Measured — `grep -n apparatusWidth dialectica-ui/src/qml/DTheme.qml` returns
nothing, and `git log -S apparatusWidth -- dialectica-ui/src/qml/DTheme.qml`
names `3901e99` (#70), which is this branch's base commit. So the token and its
only consumer went together, which is the right shape; there was nothing left
for this piece to remove.

Recorded rather than dropped because a task list that quietly loses an item
reads afterwards as an item nobody did.

`paperDeep` and `accent` carried comments defining them by their role in that
column. Both now say what the bundle's own comments say — "recessed panel" and
"moderation" — so the token means what it is used for rather than what it was
once used for.

### D7. The seven `mark*` inks are not reverted to the bundle's three

The bundle's `Theme.qml` carries a three-ink mark palette. The impl carries
seven, derived in `docs/IDENTICON.md` against three measured constraints
(pair separation under simulated dichromacy, contrast against paper, chroma at
high lightness). Taking the bundle's three would be a regression with a
measurement behind the thing being regressed. Recorded because "the bundle is
the authority" would otherwise read as licence to overwrite them.

### D8. Every tooltip is a `DTip`, and the attached form is banned

**Added after review measured the defect this closes.**

`ToolTip.text: lamp.explanation` bound the one free string `DStatusBar` takes
into a `Text` whose `textFormat` is **`Text.StyledText` (2)**. Qt Quick Controls
sets that explicitly on the default content item, so this is not QML's
`AutoText` default and none of the reasoning about AutoText reaches it — a
tooltip is markup-rendering unconditionally.

**Measured, Qt 6.10.3**, against `"<b>OWNED</b> x"`:

| form | `contentItem.textFormat` | `contentWidth` |
|---|---|---|
| attached `ToolTip.text:` | 2 (StyledText) | 56.66 |
| `DTip` | 0 (PlainText) | 102.02 |

and a `Text.PlainText` element painting the same string measures 102.02. The
tags are **consumed** in the first and **drawn** in the second. Both forms
return `"<b>OWNED</b> x"` verbatim from `contentItem.text`, which is why a
string-based assertion cannot tell them apart — only `textFormat` can.

**It cannot be fixed in place.** `ToolTip.text` on an item routes through a
*shared* tooltip instance owned by the attached type, so there is no per-site
content item to give a `textFormat:` to. The format can only be pinned by
declaring the tooltip, which is what `DTip` does.

**The ban is total rather than case-by-case**, including `DVouchStamp`, whose
two strings are hardcoded literals and render no markup today. A tooltip binding
a literal is one edit from binding a name or a reason, and that edit has no
reason to think it touched rendering — which is the same argument the
`textFormat` gate's own comment already makes about `Text`. A rule with an
exemption for "this one is safe" is a rule no gate can state.

**Named `DTip`, not `DToolTip`.** `check_qml_names.py` derives its
stale-reference ban from `qmldir`: for every type declared `DX`, a bare `X` in
any `.qml` body is an error, because that is what a half-finished rename of *our*
type looks like. `DToolTip` therefore bans the word `ToolTip` tree-wide —
including in the file that must name the Qt Controls type it derives from, and
in the tests that must say what they assert about. The gate is right about our
own renames and has no way to know this `X` is upstream's. Measured: four errors
under `DToolTip`, none under `DTip`. Weakening the gate to distinguish the two
cases would cost more than the name does.

### D8a. Two guards reported clean over D8's defect, and both are repaired

Recorded separately because the component fix alone would have left the defect
able to return silently, which is the "a gate the defect satisfies" shape.

**The test walkers could not reach a tooltip.** `nonPlainTextElements` descended
`node.children`, and a `ToolTip` is a `Popup` — not an `Item`, so never a child.
Measured: it returned `[]` against a bar rendering
`"<b>OWNED</b> <img src=x>"`, so `test_every_text_the_bar_renders_is_plain_text`
passed over a bar rendering markup. "No markup rendered" and "the walker never
looked" were the same green, which is this suite's recorded defect family.

They now descend `data` and `contentItem`. Proved by mutation: with `DTip`'s
content item set to `StyledText` the status-bar walker reports **three**
findings where the old one reported none.

**And each new tooltip test asserts its COUNT before its formats.** A walker
narrowed back to `children` fails with "found 0" rather than passing quietly —
without that, repairing the walker and breaking it again would be
indistinguishable.

**The CI `textFormat` gate is structurally blind to it.** It counts `Text {`
openings against `textFormat:` assignments per file; an attached tooltip binding
is neither, so `DStatusBar.qml` passed at `opens=1 formats=2` — measured on the
defective file — while rendering markup through a second, uncounted element. A
new step bans attached tooltip bindings outright, measured firing on the
reintroduced defect (2 hits) and clean on the tree.

The new step strips `//` comments before matching, because `DTip.qml` and both
its callers discuss `ToolTip.text` in prose and a gate that fires on its own
documentation gets deleted.

### D9. `DVouchStamp` takes the `hasIdentity` gate; the caller does not

`SPEC.md:88` — *"It is not drawn at all while this machine has no identity."*
The component had only `vouched` and `revealed`, so the clause was unbuilt and
neither document said whose it was.

**The gate goes in the component.** A vouch stamp is placed by every post row in
every feed; a contract saying "gate this yourself" must be got right at each of
those sites and is silent when it is not. CLAUDE.md's *complexity in the data
structure*: one property makes the invariant hold for every consumer at once.

**It defaults `false`.** A caller that forgets the property gets no affordance,
rather than a VOUCH prompt on a machine that cannot vouch — the same direction
as the chip's `capability.canPost === true` discipline, where failing open is
what puts the obligation back on the caller.

Written `hasIdentity && (vouched || revealed)`, gating **both** disjuncts.
Proved by mutation: `vouched || (hasIdentity && revealed)` still draws a vouched
stamp on a machine with no identity — a claim about a decision that machine can
no longer make — and it passes three of the four states the old sweep drove.

**A related seam is NOT closed here**, and is named so it is not mistaken for an
oversight. `DIdentityChip` takes a raw `hasIdentity: bool` where `FeedScreen`
already funnels the same question through a `capability` object, so each screen
must independently write `capability.canPost === true` and independently get the
`=== true` right. That is a real finding; the fix is a seam change across the
chip and its consumers, and the chip's two arms are already mutually exclusive
and tested in both directions. `DIdentityChip.qml`'s own header now states the
binding rule — `hasIdentity: <capability>.canPost === true`, with the three
degenerate shapes named — where a screen author will meet it, which is the half
this piece can discharge without widening.

### D10. `DVouchStamp` duplicates a chip `PostHeader` already renders

`PostHeader.qml` renders a **`YOU VOUCHED`** chip — outlined in accent, from its
own `property bool vouched` — for exactly the fact `DVouchStamp` stamps. The
bundle's own `PostHeader.qml` has **no such chip**: it instantiates `VouchStamp`
in its place, with `onToggled: root.vouchToggled()`.

So the bundle resolves the duplication **by replacement**, this piece builds the
replacement, and the original stays standing. The result is a component with no
consumer beside a rival rendering of the same fact in a different visual
language.

**Two renderings of one fact, and only one should survive — the stamp.** It is
the bundle's answer, it carries the hover rule and the identity gate, and the
chip carries neither.

**The removal belongs to whichever piece rewires `PostHeader`'s attribution
row**, because that piece must thread `revealed` from the feed row through the
header into the stamp and pass `hasIdentity` — work with no meaning until a feed
row exists to thread from. Non-Goals said `PostHeader` was untouched because it
"carries open design questions", which is true and is not the specific reason;
the specific reason is that the bundle already answered this one and the answer
needs a screen to land in.

### D10a. `DVouchStamp.vouched` is a boolean, and that forecloses a PLAN distinction

`docs/PLAN.md:2240-2243` distinguishes **earned** weight from a **declared**
vouch, and states the rendering obligation directly: *"keep the two
distinguishable in the UI: one is something the reader chose and can revoke in a
click, the other is something they should be told has happened and be able to
undo."*

A single boolean cannot carry that, and the stamp's two tooltips both read as
the declared case.

**This is not a contradiction.** §7.3's vouching mechanism is unbuilt, this
piece builds no screen, and a boolean is the right shape for what exists today
— a tri-state whose third value nothing can produce would be a wider surface
with no way to test the widening.

It is recorded because a later screen piece **will** inherit this property and
have to widen it, and because the alternative was real: a tri-state, or a second
property beside `vouched`. Choosing the boolean defers the distinction rather
than deciding against it.

PLAN's other vouch obligations are met: the absence of any count or count-shaped
property is exactly `PLAN.md:2226-2227`, and `tst_vouch_stamp.qml` pins it by
rendering rather than by property name — so the next spelling, called `weight`
or `tally`, is caught too.

### D11. The DELIVERY lamp has no honest source, and what it defers to is unbuilt

Non-Goals defers wiring a lamp to a real probe, and reads as though the answer
is merely elsewhere. It is not. `docs/PLAN.md:3471-3480`: a successful publish
*"must not be rendered as sent, delivered or seen, and **no in-flight state is
to be designed because no call produces the signal one would wait on**"*, with
the three things still owed named at `:3463-3469` as **"Still not built"**.

So a screen author reaching for `deliveryState` finds a property with three
legal values and no call that can compute any of them.

**This piece contradicts nothing** — it wires nothing, and a component that
takes its state as a property is the right shape for a signal that does not
exist yet. The record needs to name PLAN's obligation rather than describe an
open question, because the gap is invisible from the component: `DStatusBar` is
otherwise complete, and nothing about it says that one of its three lamps cannot
currently be told the truth.

The same is true in a weaker form for `DIdentityChip.generatedName`.
`generated-names/spec.md:74-79` records that the derivation is *"reachable by no
caller until an entry point exists"* — the QML sandbox holds no wordlists and
cannot derive a name for itself — and tracks it as issue #81. The chip takes the
name as a string, following `PostHeader`'s precedent; the existing consumer
passes `generatedName: ""`, and will until #81 lands.

## What the tests here cannot see

Stated explicitly, because a green suite mistaken for a broader guarantee is a
failure this repo has already paid for.

- **A host type-name collision.** Under `qmltestrunner` basecamp is absent, so
  a `DStatusBar` assertion has no competitor to lose to.
  `check_qml_names.py` covers it; no test in this piece claims to.
- **That a `MouseArea`'s `onClicked` is wired.** Both signal tests emit the
  signal directly, which bypasses the handler. Driving a real press needs a
  window, and a component built with `createObject(null, …)` has none. The
  wiring is one line in each file and is covered by reading it — **in three
  files now** (`DVouchStamp`, `FlatButton`, and `DIdentityChip`'s forward to
  `createRequested()`). The count is the thing worth tracking, because it is
  what decides whether a windowed harness becomes worth building. This was going
  to be asserted anyway — "the stamp does not flip its own `vouched`" — and the
  assertion would have passed whether or not the handler wrote the property,
  which is the "two explanations, one answer" shape. It was removed rather than
  kept as a green nobody earned.
- **That a `Text` renders markup.** `Text.text` returns its *source* string
  whatever `textFormat` is, so checking that a markup-shaped string comes back
  verbatim proves nothing about formatting. **Measured here**: the first version
  of `test_a_name_containing_markup_is_not_rendered_as_markup` survived a
  `textFormat: Text.RichText` mutation with 12 passed, 0 failed. Every such
  assertion in this piece now reads `textFormat !== 0` instead, and the
  string-verbatim check is kept beside it as a separate, differently-named test
  so neither is mistaken for the other.

## Risks / Trade-offs

- **These components have no screen to prove them in.** → Each test
  instantiates the component directly and asserts on its own properties and on
  its rendered children, so correctness does not wait on a screen. The residual
  risk is integration shape — whether a screen can actually lay these out —
  and it is genuinely deferred to the screen pieces.
- **`opacity` on a `Canvas` composites the whole mark, outline included.** →
  Accepted: the outline losing weight with the fill is what "recessive" should
  look like. The alternative preserves the contour at full strength and makes a
  muted mark *more* conspicuous in outline than an unmuted one.
- **No test here can catch a host type-name collision.** → Stated rather than
  mitigated. Under `qmltestrunner` the host is absent, so there is no competitor
  to lose to; `check_qml_names.py` is what covers it, and it is run as part of
  this piece's gates. No test in this piece claims otherwise.
- **`-W 0` means an unrelated qmllint category can redden this piece.** →
  Gates run locally before the PR opens, so it surfaces here rather than in CI.

## Open Questions

None that change what is built. Who computes a lamp's state — a core probe, a
UI heuristic, or a mix — is a screen-and-core question, and the component is
written so that answer can arrive later without touching it.
