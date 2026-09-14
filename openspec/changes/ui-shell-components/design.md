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

### D3. `DIdentityChip` drops `isPerson`

The bundle passes `isPerson: true` to `Identicon`. The impl's `Identicon` has
no such property and deliberately: `Identicon.qml:95-101` records that the
angular/curved split halved the vocabulary any one address could reach and was
restating what position already says, so the distinction "is now carried by
POSITION ALONE".

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

`ModerationScreen.qml:94` wants a de-emphasised mark for an author who is
already moderated. Three ways to do that:

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
to read as clearly recessive while leaving the contour visible; the test marks
it as a choice, and asserts the *relation* (muted is strictly less opaque, and
selectors are unchanged) rather than only pinning the literal, so the test still
means something if the number is retuned.

### D5. The two new button kinds are declared as data, not as two more ternaries

`FlatButton` chose colour, border and padding with three parallel ternary
chains keyed on `kind`. Adding two kinds would have made each chain five deep,
and the fourth slightly-different guard is this repo's stated signal to reshape.

So `kind` indexes one `readonly property var kinds` object holding each kind's
`fill`, `stroke`, `textInk` and padding pair, and the bindings read fields off
it. Adding a sixth kind is then one entry, not four edits in four places that
must agree — and a kind that is missing a field is visible in one place rather
than as a ternary that silently falls through to `secondary`.

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

### D5a. The lamp dot uses `implicitWidth`, departing from the bundle

The bundle's `Lamp` sets `width: 7; height: 7` on a `RowLayout` child. That is
undefined behaviour — qmllint reports it as `Quick.layout-positioning`, and
`check_qml_members.sh`'s `-W 0` turns it into a build failure. Found by running
the gate, not by reading.

Worth recording because it is the second defect the bundle's own QML carries
into this piece (the `state` shadowing is the first), and because the gate that
caught it is one whose comment warns it "fires wider than its name" — this is
that firing, and it was a real defect rather than a false positive.

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

## What the tests here cannot see

Stated explicitly, because a green suite mistaken for a broader guarantee is a
failure this repo has already paid for.

- **A host type-name collision.** Under `qmltestrunner` basecamp is absent, so
  a `DStatusBar` assertion has no competitor to lose to.
  `check_qml_names.py` covers it; no test in this piece claims to.
- **That a `MouseArea`'s `onClicked` is wired.** Both signal tests emit the
  signal directly, which bypasses the handler. Driving a real press needs a
  window, and a component built with `createObject(null, …)` has none. The
  wiring is one line in each file and is covered by reading it. This was going
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
