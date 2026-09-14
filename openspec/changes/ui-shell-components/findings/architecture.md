# Architecture findings — `ui-shell-components`

Dimension: **architecture** only. Correctness, security and readability are held
by other instances.

Reviewed at `d9fe78f` against `origin/main` (three-dot). The judgement standard
is CLAUDE.md's own principles — *complexity in the data structure*, *one
function one job*, *make the change easy then make the easy change* — and the
fact that this piece exists to be depended on by five or six screen pieces, so a
wrong seam is paid for five or six times.

## Findings

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DIdentityChip.qml:24` — the chip
      takes a raw `hasIdentity: bool` where this tree already has a
      `capability` data shape for exactly that question, so the screens will
      carry two parallel answers to "can this machine act"
      **Scenario:** `FeedScreen.qml:47` establishes
      `property var capability: ({ canPost: false, reason: "" })`, funnelled
      through the single `capabilityFrom()` at `FeedScreen.qml:82` — and its
      own comment (`FeedScreen.qml:64-67`) names this as CLAUDE.md's
      *complexity in the data structure* move: "a second consumer of
      `capability.reason` now inherits the invariant instead of rediscovering
      it." Every existing affordance gates on it: `visible:
      screen.capability.canPost === true` at lines 571, 717, and `interactive:`
      at 580. `DIdentityChip` is the sixth consumer and it does not inherit the
      invariant — it takes a bare bool, so each of the five or six screen
      pieces must independently write `hasIdentity: screen.capability.canPost
      === true` and independently get the `=== true` right. The `=== true` is
      not cosmetic: `FeedScreen.qml:69-76` records that it is what makes
      failing closed structural, because `"true"`, `1`, `null` and `undefined`
      are all not-`true`. A screen that writes `hasIdentity:
      screen.capability.canPost` (no `=== true`) renders the *identity present*
      arm for a capability of `"false"` or `{}` — the chip claims an identity
      the machine does not have, and every gate stays green. This is the fourth
      slightly-different guard, which CLAUDE.md names as the signal to reshape
      rather than to add a fourth call site. **Severity: medium** — it is a
      seam decision, cheap now and expensive across six screens. It does not
      have to become a `capability` property: stating the contract in the
      component ("bind `capability.canPost === true`, never the raw probe") and
      in the brief would also close it, but it must be closed somewhere the
      screen author will look.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DIdentityChip.qml:36,46-73` —
      `hasIdentity` and `identityAddress` are two independent properties with
      no relation enforced, so `hasIdentity: true` with an empty address
      renders a confident all-zero mark labelled `CURRENT IDENTITY`
      **Scenario:** instantiate `DIdentityChip { hasIdentity: true;
      generatedName: ""; identityAddress: "" }`. **Measured** in this worktree
      under Qt 6.10.3: it instantiates, `implicitWidth=177`, no warning, no
      binding error, `check_bindings` clean. `Identicon.qml:103-106` pads a
      short or malformed address to 64 hex characters, so the empty address
      draws a perfectly valid, deterministic, *stable* mark — the mark of the
      zero address — and `AddressLabel` beside it renders empty. The chip
      therefore asserts "this is who you are" while showing a recognisable mark
      that belongs to nobody. That is precisely the claim
      `Identicon.qml:114-120` and this file's own header comment
      (`DIdentityChip.qml:9-20`) exist to prevent: the mark is a recognition
      aid, "a reader who needs to know WHO this is reads the address" — and
      here the address is not there to read. Every screen reaches this state:
      it is what the chip holds between the screen appearing and core answering
      the identity call. CLAUDE.md's *complexity in the data structure*: two
      booleans-and-a-string that can disagree, where one value that cannot
      (an address, empty meaning no identity) would make the invariant hold by
      construction for all six consumers at once. **Severity: medium.**

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/FlatButton.qml:21-33,52-53` — the
      `kinds` table makes a missing *colour* field visible but a missing
      *padding* field silent, reproducing the invisible-button failure the
      reshape was written to eliminate
      **Scenario:** the file comment at lines 9-11 claims "a missing field is
      visible where the kind is defined, rather than surfacing as a chain that
      silently falls through to `secondary`". **Measured** by adding a sixth
      entry `"probe-incomplete": { fill: "", stroke: DTheme.ink, font:
      DTheme.body, padX: 32 }` — `textInk` and `padY` both absent — and
      instantiating it: `instantiated=true color=#00000000 borderW=1
      labelColor=#000000 implicitH=NaN implicitW=60.078125`. The missing
      `textInk` raises `Unable to assign [undefined] to QColor` at
      `FlatButton.qml:62`, which `run-qml-tests.sh`'s `check_bindings` does
      catch — that half of the claim holds. The missing `padY` raises
      **nothing**: `implicitHeight` becomes `NaN` silently, no warning, no
      binding-loop, and `check_bindings` reports clean on that line. A
      `NaN`-height button laid out in a `RowLayout` is a control nobody can
      see that still accepts clicks — the exact failure D5 traced in the old
      ternary chains and the exact one the fallback to `secondary` was chosen
      to prevent. The colour fields fail loudly because QML type-checks a
      `QColor` assignment; the numeric ones do not because `NaN` is a valid
      `real`. **Severity: medium** — the table is still the right shape, but
      the guarantee written on it is only half true, and a sixth kind is
      exactly the change this piece is inviting. The fix is either a required-
      field assertion where the table is read, or a test that derives its kind
      list from the table (see the next box, which makes that free).

- [ ] **`tester`** — `dialectica-ui/tests/tst_flat_button.qml:22-24` — the test
      sweeps a hand-maintained `declaredKinds` array rather than the table it
      is testing, so the reshape's main benefit (a new kind is one edit) is not
      realised
      **Scenario:** `readonly property var declaredKinds: ["primary",
      "secondary", "destructive", "destructive-outline", "secondary-micro"]` is
      a parallel copy of `FlatButton.qml`'s five keys. Four tests iterate it —
      `test_every_kind_is_either_filled_or_outlined`,
      `test_no_kind_renders_its_label_in_the_colour_behind_it`,
      `test_no_two_kinds_render_identically`,
      `test_every_kind_renders_its_label_as_plain_text`. A sixth kind added to
      the table and not to this array is covered by **nothing**, and the suite
      stays green: measured, the `probe-incomplete` entry above passed the
      whole file. That is the `hand-maintained sweep lists go stale silently`
      trap this repo has already paid for and named in CLAUDE.md, and it is
      load-bearing here because D5's stated reason for the table is "adding a
      sixth kind is then one entry, not four edits in four places that must
      agree" — the array is a fifth place that must agree. **Measured that the
      fix is available:** `Object.keys(button.kinds)` returns
      `primary,secondary,destructive,destructive-outline,secondary-micro` and
      picked up the probe's sixth key, `count=6`, so the list can be derived
      from the component instead of restated. Deriving it also gives the
      previous box its gate for free: `test_every_kind_is_either_filled_or_
      outlined` over a derived list would have caught the `NaN` height.
      **Severity: medium.**

- [ ] **`dev-writer`** — `docs/UI-BRIEF.md` (no section for these three
      components) — five or six screen pieces will be written against a brief
      that does not mention the components this piece exists to give them
      **Scenario:** the brief carries `## What ScreenFrame gives you, and the
      one thing it asks` (line 804) and `## The vote control — settled, and
      smaller than it was` (line 1152), both addressed to "whoever implements a
      screen in QML, not only for the designer". `DStatusBar`, `DIdentityChip`
      and `DVouchStamp` are the same class of thing — every screen's footer
      carries two of them — and the brief names none of them: `grep -n
      "StatusBar\|IdentityChip\|VouchStamp"` over `docs/UI-BRIEF.md` returns
      nothing. The brief itself says why this matters, at lines 834-837: *"the
      apparatus column shipped because an obligation lived in a document nobody
      implementing a screen had a reason to open. A contract that can only be
      found by someone who already knows to look for it is a contract the next
      screen will not meet."* The obligations that need a home there are
      concrete and are not deducible from the property names: that an
      unrecognised lamp state renders **degraded** rather than ok, so a screen
      must not pass core's string through unchecked and assume green means
      green; that `DIdentityChip` must be given a capability-derived
      `hasIdentity` and never a raw probe field (first box); and that the lamp
      order DELIVERY/STORAGE/ZONE is fixed and positional
      (`DStatusBar.qml:124-127`) so a screen must not reorder or omit one.
      CLAUDE.md's rule is that a change making the brief wrong fixes it in the
      same change; this change does not make the brief *wrong*, it makes it
      *incomplete in the way that section exists to prevent*, and the cost lands
      on every screen piece that follows. **Severity: medium** — this is the
      finding most likely to be paid for repeatedly, because it is the one the
      next five authors cannot discover by reading the code they are told to
      use.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/FlatButton.qml:21` — `readonly
      property var kinds` exposes a per-instance, mutable copy of the table on
      the public seam, which is neither shared nor read-only despite reading as
      both
      **Scenario:** `readonly` on a `var` freezes the reference, not the object.
      **Measured**, Qt 6.10.3: `a.kinds["primary"].fill = "#00ff00"` throws
      nothing and the entry reads back `#00ff00`; forcing re-evaluation
      (`a.kind = "secondary"; a.kind = "primary"`) then yields
      `colorAfterReeval=#00ff00` — a consumer can silently restyle a button
      through a property that says `readonly`. Separately, the object literal
      is re-created per instance rather than shared: across five instances,
      `sharedWithFirst=0 of 4`, and a mutation to one instance's table leaves a
      later instance's entry at the original `#26231d`. So every `FlatButton`
      allocates six objects (the table plus five entries) it never mutates,
      which is a real cost in the `secondary-micro` case the table was extended
      for — a `ListView` of moderated authors with an `UNMODERATE` per row
      (`ModerationScreen.qml:102,145`). **Severity: low, and partly stylistic** —
      nothing is broken today and no consumer mutates it. It is filed because
      this is a seam five or six pieces bind to, and `kinds` does not need to be
      on it at all: only `spec` is read by the bindings, and the table could be
      a file-scoped `QtObject`/singleton read-only and allocated once. If the
      answer is "leave it, the test enumerates it" (see the `tester` box, which
      wants `Object.keys(kinds)`), then say so in the file — an intentionally
      public table is a different thing from an incidentally public one.

## What was clean

**`DStatusBar`'s seam is right, and the `normalisedState` split is the best
decision in the piece.** Validation is one named function used by all three
lamps (`DStatusBar.qml:49-53`) rather than a ternary per lamp, so a fourth
caller inherits the rule — CLAUDE.md's *a guard is a job*, applied. The three
lamps are literal children in a fixed order with the reasoning for not making
them a model stated at lines 124-127, which is the right call: a model would
make order and count a caller's choice, and the component's meaning depends on
neither being. `lampState` over `state` (D1) is a genuine trap avoided with a
measurement behind it. The six-property surface (three states, three tooltip
strings) is what a screen needs and nothing more; who computes a lamp's state is
correctly left outside the component, which keeps the core/UI split intact — the
component reaches for nothing ambient.

**`DVouchStamp`'s hover threading is the right shape, and the question in the
brief rests on a premise this branch does not contain.** `revealed` is a fact
about the *post row* that the stamp genuinely cannot observe, so it must be
passed; a `HoverHandler` inside the stamp would reveal on the stamp's own
bounds, which at `opacity: 0` it does not have. The three-level chain is the
minimum. It is also not yet a chain: the brief describes threading "from the
feed row through `PostHeader` into the stamp", and **`PostHeader.qml` is
untouched by this branch** — it still renders the static `YOU VOUCHED` badge
(lines 48-56) and has neither `rowHovered` nor `canVouch`, both of which the
bundle's own `PostHeader.qml:16-17` carries. So there is no pass-through to
judge here; the concern belongs to whichever piece replaces that badge with the
stamp, and that piece inherits a clean two-property contract. No box, because
nothing on this branch is wrong — but the next author should know the badge and
the stamp are two renderings of one fact and only one of them should survive.

**The reshape is correctly scoped, and the honesty about it is the right call.**
D5 could have claimed a behaviour-preserving refactor and split it into two
commits per CLAUDE.md's *make the change easy* rule. It is not behaviour-
preserving — tracing the old chains showed an unknown kind rendered invisibly —
and landing it as one commit with that reasoning recorded is better than a
refactor commit making a false claim. The `secondary`-fallback direction is the
right one.

**`markMutedAlpha` belongs in `DTheme`, and the question in the brief resolves
cleanly.** It is consistent with what is already there: `markInFeed`,
`markMinWeave`, `markMinDraw` (lines 181-184) and `headChars`/`middleChars`/
`tailChars` are all single-component tokens in the same file. More importantly
D5b draws the line in the right place — the token sits *outside* the frozen
`mark*` ink block because it changes how visible a mark is and never which inks
are selected, so two peers on different values still agree who a mark depicts.
Putting it beside the frozen constants would have implied a determinism
obligation it does not carry. Same reasoning for the three status colours: the
bundle declares `DTheme` the only place green and orange are permitted, so a
per-component value would make that rule unenforceable.

**`muted` as a root opacity (D4) is the right layer.** It cannot reach a
selector because `onPaint` is not re-entered, so the determinism contract holds
by construction rather than by promise — the *data structure, not the logic*
principle applied to a drawing contract.

**Nothing here duplicates `ScreenFrame`.** `ScreenFrame` decides card padding,
width and border; these three decide nothing about a card and none of them
anchors to one. `DIdentityChip` and `DStatusBar` are footer contents a screen
places inside a frame. The one near-duplication in the tree is `PostHeader`'s
`YOU VOUCHED` badge against `DVouchStamp`, noted above and correctly out of
scope.

**No component reaches for ambient state.** Every one takes what it renders as
properties and emits a signal for what it cannot decide — the core/UI split is
respected throughout, which is the constraint that matters most here since a
view cannot fetch or read anything under basecamp's sandbox.

## Method

Worked in `.claude/worktrees/review-shell-architecture` on
`review/ui-shell-components/architecture`. Every "measured" claim above comes
from a probe spec run through `dialectica-ui/tests/run-qml-tests.sh` against
Qt 6.10.3, and from one deliberate mutation of `FlatButton.qml` (the sixth
incomplete kind). Both were reverted; the tree was verified clean before the
findings commit, and the worktree is removed.
