# Drop the apparatus column from the shipped view

## Why

The design bundle's right-hand column headed `APPARATUS` carries italic marginal
notes — `ON THIS ORDERING`, `ON WHAT YOU HOLD`, `ON THE MARK` — which explain **to
a reader of the design** why the screen behaves as it does. **The owner has
confirmed it is annotation about the design, not part of it.**

It reached the shipped QML anyway. On `origin/main` (`468e716`):

- `ApparatusColumn.qml` renders a `Theme.paperDeep` panel with a literal
  `"APPARATUS"` heading;
- `MarginNote.qml` renders one entry in it;
- `ScreenFrame.qml` — the shell **every** screen is built on — is a two-column
  `RowLayout` whose right column is that panel, unconditionally;
- `FeedScreen.qml` ends with three `MarginNote`s filling it.

So a real user of the app sees a 244px column of commentary addressed to a
designer. That is the mistake.

**The obligations those notes carry are real and are not being dropped.** Each
note restates something the interface genuinely owes the reader. They survive
because `docs/UI-BRIEF.md` already states all three, in prose, where the brief
states obligations — verified line by line in `design.md` §2 rather than
assumed. Only the marginal-column *presentation* goes.

## What Changes

- **Delete `ApparatusColumn.qml` and `MarginNote.qml`**, and their two `qmldir`
  registrations. Nothing else instantiates either.
- **Reshape `ScreenFrame.qml`** from a two-column `RowLayout` into a single
  content column. The alternative — keeping the layout and leaving the right
  column empty — is the shape this change exists to remove, and it would leave
  244px of dead width on every screen.
- **Remove `FeedScreen.qml`'s `apparatus: [...]` block**, the only assignment to
  the property `ScreenFrame` exposed.
- **Remove `Theme.apparatusWidth`** and the two theme comments that describe inks
  by their role in the column. `Theme.paperDeep` itself stays: it is a surface
  token, and a screen may want a deeper paper for an inset panel; only the
  comment claiming it is *the apparatus column* is wrong once the column is gone.
- **State the `ON WHAT YOU HOLD` obligation in `docs/UI-BRIEF.md` as rendering
  obligation 10**, because review found it was the one apparatus obligation the
  move did not preserve intact. The brief was **incomplete rather than wrong**;
  the code is wrong against the completed brief, and fixing the code is a
  `dev-writer` box rather than this piece's. See "The obligation the move
  narrowed" below.

## The obligation the move narrowed

Two reviews filed the same gap: the `ON WHAT YOU HOLD` note's obligation now
renders **only on the empty screen**, because both carriers — the sentence "This
is a fact about your copy, not about the Stoa." and the `"STORE READ OK · N POSTS
HELD"` line — sit inside the `Rectangle` gated on `rows.length === 0`. Whether a
non-empty feed owes a locality line is a requirement question, and this is the
answer.

**What the source documents actually say.** `docs/PLAN.md` has no section on
counts or their locality — its §11.1 "Rendering obligations, collected" does not
exist in the file yet and arrives with `vouching-state`, which `git log` will
show. What PLAN.md supplies is the architecture the obligation follows from:
eventual consistency among active participants, and the standing shape it states
twice — *"core's honest answer is incomplete without something the view says"*.
The written rule is `docs/UI-BRIEF.md` constraint 1: **"a count of *anything*
global — members, total posts — is unknowable. Do not show one."**

**That rule is a prohibition, and the feed already obeys it.** Measured rather
than assumed: the non-empty feed renders **no number at all**. The only count on
the screen is `"STORE READ OK · N POSTS HELD"`, which renders only when the list
is empty and so only ever reads `0`. There is no page number, no "showing 30 of",
no reply count. So the finding's framing — that thirty posts appear with numbers
a reader may read as complete — does not hold as stated, and a requirement
demanding a locality line "wherever a count is shown" would have no subject.

**The live half is the pagination control, which is an extent claim without being
a number.** `hasMore` is computed by `feed::list_threads` from this peer's log
alone, so "Next" means *this machine holds another page*. A reader meeting thirty
posts and a "Next" button reads it as "this Stoa has more" — the global claim the
architecture forbids — and nothing on that screen corrects it. The apparatus note
said "Every number here counts what this machine has received", and the word
*number* is what let the obligation slip: the assertion moved from a numeral to a
button and stopped being covered.

**Decision: the requirement is triggered by the extent claim, not by the
screen.** `docs/UI-BRIEF.md` gains rendering obligation 10, in two halves — never
render a quantity the core cannot know, and where the interface *does* assert
extent, that assertion must be readable as local. Both are checkable by looking
at what a screen renders in a given state.

**Rejected: "the locality claim must appear once per screen."** That is a
disclaimer rather than an obligation, and it is the exact mistake this change
exists to undo — a screen with no count and no paging would be made to print a
sentence at the reader that corrects nothing. An obligation is a thing the
interface must **do**.

**Rejected: "the empty-screen-only behaviour is correct as it stands."** It is
correct about counts and silent about paging, and the paging case is the one
where a reader is actually misled. Accepting it would leave the interface relying
on the reader not to make the ordinary assumption, which §3 of `design.md`
already established is not a thing an interface may rely on — the same test,
applied to the other obligation.

**So the brief was incomplete and the code is wrong against it.** The brief never
covered the non-numeric extent claim, which is why no gate and no reviewer caught
the gap before the apparatus column was removed from around it. Completing the
brief is this change's job and is done here. **Changing `FeedScreen.qml` is not**:
that is a `dev-writer` box, filed in `findings/correctness.md`, and the shape it
should take is the one this change already demonstrated for `ON THIS ORDERING` —
a body-level sentence outside the state branches, discharging a named obligation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true` with the measurement behind it.

**No merged spec requires apparatus text.** Re-measured after merging `main`,
because the two specs this paragraph once reported as non-existent —
`composer-view` and `stoa-navigation-view` — **have since merged**, and a stale
"it does not exist" is exactly the claim that decays into a wrong one. Run
`openspec list --specs` for the current inventory.

Two `apparatus` hits across `openspec/specs/` today, neither requiring apparatus
text:

- `module-wire-contract/spec.md` uses the word in its Phase 0 sense — "the probe
  is apparatus rather than forum surface".
- `composer-view/spec.md` names `compose.apparatus` as a **bundle wording
  source**, while requiring the statement itself *"in the closed gate's own
  body"* and stating explicitly that the requirement is not on occupying any
  region of the screen. `design.md` §7 quotes it in full.

There is still no `copy.json` anywhere in the tree, so no `*.apparatus` key is
required verbatim by anything — and `composer-view` now says in the spec why such
a requirement would be unenforceable.

**Rendering obligation 10 goes in `docs/UI-BRIEF.md`, not into a capability, and
that placement is a decision rather than a default.** Every capability
`openspec list --specs` reports contracts **core** behaviour; there is no
view-facing capability in the tree, and `list_threads` — the call whose `hasMore`
this obligation is about — is itself built and uncontracted. Minting a capability
to hold one QML rendering rule would introduce a near-duplicate of the surface
`docs/UI-BRIEF.md` already owns, which CLAUDE.md names as the contract for what
the UI must show, hide or refuse to claim. The brief is where a screen author and
a designer both look; a spec file neither opens is how the apparatus shipped in
the first place.

## Impact

- **Deleted:** `dialectica-ui/src/qml/ApparatusColumn.qml`,
  `dialectica-ui/src/qml/MarginNote.qml`.
- **Modified:** `dialectica-ui/src/qml/ScreenFrame.qml`,
  `dialectica-ui/src/qml/FeedScreen.qml`, `dialectica-ui/src/qml/qmldir`,
  `dialectica-ui/src/qml/DTheme.qml`, `docs/UI-BRIEF.md`, and `CLAUDE.md` (the
  *Where to look for what* row now sends a screen author to the brief **before
  writing a screen**, argued in `design.md` §4).
- **Modified by the merge with `main`, and argued in `design.md` §9:**
  `dialectica-ui/src/qml/DJoinScreen.qml`,
  `dialectica-ui/src/qml/DStoaListScreen.qml` and
  `dialectica-ui/src/qml/DOnboardingScreen.qml` lose their `apparatus` blocks —
  eleven margin notes in total, deleted on the owner's decision rather than
  migrated. `dialectica-ui/tests/tst_vote_and_gate.qml` and
  `dialectica-ui/tests/tst_onboarding_states.qml` lose the assertions that
  reached into `ScreenFrame.apparatus`.
- **Added:** `dialectica-ui/tests/tst_feed_extent_claim.qml`, pinning obligation
  10's discharge.
- **The test runner's single-spec mode is no longer this change's to add.**
  This piece added one; `main` independently grew the same feature and did it
  better, adding `check_bindings` (which fails a spec whose bindings evaluate to
  `undefined` — a QWARN `qmltestrunner` otherwise reports while exiting 0) and an
  `--import` option. Merging took `main`'s file **wholesale**, verified
  byte-identical to `origin/main`'s blob by `git hash-object`, so
  `dialectica-ui/tests/run-qml-tests.sh` is untouched by this change.

  Nothing was lost in doing so: `main`'s version keeps the same
  `run-qml-tests.sh <file.qml>` interface and the same reasoning for it — that
  invoking `qmltestrunner` directly means hand-writing `QT_QPA_PLATFORM=offscreen`,
  a shape the permission checker cannot analyse, costing an approval click per
  call, and losing the runner discovery that distinguishes a real failure from a
  Qt5 binary exiting 1 with no output.
- **No test asserted the *feed's* apparatus content**, which is worth recording:
  that column shipped and no gate could see it — and, before this change,
  **no gate could see it come back** either.

  **That was measured rather than argued, and the measurement is the reason the
  answer is not "accept it".** Both components were reconstructed as a future
  author would write them today — `D`-prefixed, reading `DTheme`, a
  `textFormat` on every `Text` — registered in `qmldir`, and instantiated in
  `FeedScreen` so the literal heading `APPARATUS` rendered in the shipped view.
  Against that tree: `check_qml_names.py` **ok**, `check_qml_members.sh` **ok**,
  the layout-import and `textFormat` gates green, and `run-qml-tests.sh` green
  across every spec file with zero failures — byte-for-byte the same result as
  the clean tree. The suite is **indifferent** to whether the annotation is
  there.

  **So the requirement is stated where it can be checked**, in
  `docs/UI-BRIEF.md`'s box under *Non-negotiable rendering obligations*: no
  screen may render a region whose heading announces it as commentary on the
  design, the discriminator being *who is addressed*.

  **And the instrument is named, because the obvious one does not work.** A
  check that a component called `ApparatusColumn` is absent is defeated by a
  rename — the probe above was called `DApparatusColumn` and passed every
  name-based gate in the repo. The assertion has to be over **what a screen
  renders**: no screen's rendered text contains a heading that presents a region
  as annotation. That is the same instrument `tst_feed_copy.qml` already uses
  for the feed's locality sentence — a sweep over the rendered `Text` of a
  driven screen — so the machinery exists and the spec files that would carry it
  are already in the suite.

  **One existing assertion looks like this gate and is not**, which is worth
  recording so the next reader does not mistake it for coverage.
  `tst_stoa_screens.qml`'s
  `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus` asserts
  `apparatusText(screen) === ""` and its comment says a failure means
  "annotation has returned to the shipped view". It reads
  `ScreenFrame.apparatus`, the alias this change deleted — so it catches a
  return *through that property* and nothing else. The probe above restored the
  column without touching the alias and that test passed. It is correct about
  its own corpus and should stay; it is not the gate this box asks for.

  **Writing the test is a `tester` box rather than this role's**, and it is left
  open deliberately: the `tester` has finished and its work is pushed, and a
  spec-writer editing the suite behind it is the overlap this flow exists to
  prevent.

  **Three assertions elsewhere did have to change**, and this list no longer
  claims otherwise. The screens `main` added were tested, and two of those tests
  read `ScreenFrame.apparatus` directly. Each edit is argued in `design.md` §9 —
  one obsolete guard deleted, one count-pin replaced by the property it stood
  in for, one dereference guarded and then **proved still able to fail** by
  mutation.
- **No core change.** This is view-only.
- **`docs/UI-BRIEF.md` needed less correcting than expected on the apparatus
  itself**, and the reason is recorded in `design.md` §1: the brief never
  presented the apparatus as something the interface renders. The word does not
  appear in it. What this change adds there is one short paragraph making that
  silence explicit, so the next reader of the bundle does not repeat the mistake.
- **The brief did need completing on one obligation**, which review found and this
  change answers: rendering **obligation 10** (every quantity is local, and "more"
  is a quantity) plus the matching clause in the Feed section. That gap is why the
  `ON WHAT YOU HOLD` move narrowed without anyone noticing — see "The obligation
  the move narrowed".
- **One `dev-writer` box remains open by design**, in `findings/correctness.md`:
  `FeedScreen.qml` does not yet meet obligation 10, because its locality sentence
  renders only in the empty state while the paging control renders only outside
  it. The requirement is settled here; the layout is not this role's to pick.
