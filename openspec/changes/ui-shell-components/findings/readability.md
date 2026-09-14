# Readability review — `ui-shell-components`

Dimension: **readability only**. Correctness, security and architecture are held
by other instances; an unticked row for those still means nobody has done them.

Reviewed at `d9fe78f`: the three new components, the `DTheme`/`FlatButton`/
`Identicon` changes, `qmldir`, five test files, `design.md` and `tasks.md`.

## Findings

- [x] **`dev-writer`** — `openspec/changes/ui-shell-components/design.md:120` —
      D4 cites `ModerationScreen.qml:94` as if it were a file in this tree
      **Scenario:** D4 opens "`ModerationScreen.qml:94` wants a de-emphasised mark".
      A reader takes that as the consumer justifying `muted` and goes looking:
      `git ls-files dialectica-ui/src/qml/` lists 22 files and no
      `ModerationScreen.qml`. The file is the design bundle's, under the
      gitignored `tmp/ui-bundle-new/handoff/` — which `design.md:5` establishes
      at the top but this citation does not restate, and the two are 115 lines
      apart. The same paragraph's Non-Goals (`design.md:42`) say "Not
      `ModerationScreen`", so the reader now has one line saying the screen does
      not exist here and another citing a line number in it. Say "the bundle's
      `ModerationScreen.qml`" and the ambiguity is gone.
      **Severity:** low — defect, not preference. Costs a reader one failed
      lookup, and the failed lookup is the kind that reads as a missing file
      rather than as a bundle reference.

      **Fixed.** D4 now opens "**The bundle's** `ModerationScreen.qml`" and says
      in the same sentence that it is not a file in this tree and that Non-Goals
      excludes building it, so the two lines the finding notes as contradictory
      now sit together.

      The line number is dropped rather than qualified: the bundle is
      gitignored, so a line reference into it is one no reader of this repo can
      check. Also recorded there, since the lookup was going to be attempted
      anyway: the bundle's own `Identicon.qml` has **no `muted` property at
      all**, so that `ModerationScreen` passes one that is silently dropped.

- [x] **`dev-writer`** — `openspec/changes/ui-shell-components/design.md:102` —
      D3's citation `Identicon.qml:95-101` points at the wrong lines
      **Scenario:** D3 says "`Identicon.qml:95-101` records that the
      angular/curved split halved the vocabulary". Lines 95-101 of
      `Identicon.qml` are the tail of the `inks` array and the `_body`
      address-padding comment — nothing about contour. The text quoted
      ("carried by POSITION ALONE", and the placement obligation D3 leans on) is
      at **lines 114-120**. A reader who follows the citation to check the
      obligation is real finds an unrelated paragraph, and D3's entire argument
      for why `CURRENT IDENTITY` is load-bearing rests on that obligation.
      **Measured:** `grep -n "POSITION ALONE" dialectica-ui/src/qml/Identicon.qml`
      returns 118; the `inks` array closes at 96.
      **Severity:** low — a pinned line range is the shape CLAUDE.md's
      "self-invalidating" rule warns about: it rots on any edit above it and
      cannot fail loudly. Cite the function (`_form()`) or the phrase instead.

      **Fixed**, taking the suggestion: D3 now cites "the comment above
      `_form()`" and quotes the phrase, with no line range. The old range and
      why it was wrong are kept in a parenthesis, so the next reader learns the
      rule rather than just inheriting the corrected text.

- [x] **`tester`** — `dialectica-ui/src/qml/FlatButton.qml:29` — the `font`
      field of a kind is described in a comment and pinned by no test
      **Scenario:** `secondary-micro`'s comment says it is "`secondary` at label
      type with the padding pulled in", and the table's `font: DTheme.label` is
      the half that delivers "at label type". Changing it to `DTheme.body` leaves
      the whole suite green, so the comment states a property of the code that
      nothing holds the code to. A reader who trusts the comment and a reader who
      trusts the tests learn different things about the same line.
      `test_secondary_micro_is_a_smaller_secondary` looks like the test that
      covers this, and does not: it asserts `implicitHeight`/`implicitWidth`,
      both of which still shrink from `padX`/`padY` alone under the mutation.
      **Measured:** with `secondary-micro`'s `font` changed to `DTheme.body`,
      `run-qml-tests.sh` reports `tst_flat_button.qml` **12 passed, 0 failed**,
      and all 18 specs green. Mutation applied and restored in a reviewer
      worktree; the piece branch was never touched.
      **Severity:** low-to-medium — defect. `font` is the only field of the five
      in each table entry with no assertion, so
      `test_every_kind_is_either_filled_or_outlined`'s stated promise that "a
      sixth entry added to the table with a field missing fails here" is true of
      four fields and false of the fifth.

      **Fixed** in `648b462`, and the mutation this box measured surviving now
      fails. `test_secondary_micro_is_a_smaller_secondary` asserts the label's
      `font.pixelSize` against `DTheme.label.pixelSize` **and** as a relation to
      `secondary`'s — the relation as well as the token, because a token
      comparison alone passes when both sides read `undefined`, which is the
      mechanism that let a renamed colour token through elsewhere in this piece.

      Re-measured with `secondary-micro`'s `font` changed to `DTheme.body`:
      `FAIL secondary-micro is not set at label type / Actual 15 / Expected 9`.

      The stated promise is now true of all six fields, by a different route
      than this box assumed: `test_every_kind_declares_every_field` sweeps every
      required key over a kind list derived from the table itself. That was
      filed separately by the architecture reviewer and the two fixes met.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DStatusBar.qml:14` — "the ONLY
      green and orange in the design" is false as written; `DTheme.qml:65` is
      the qualified version
      **Scenario:** `DStatusBar.qml:14` says "The lamp colours are the ONLY green
      and orange in the design — see DTheme." A reader follows the pointer, reads
      the tokens, and finds `markGreen` (`#42744f`) and `markSage` (`#aeab84`)
      thirty lines below. `DTheme.qml:67` gets it right — it scopes the claim to
      "the one signal **the interface** reserves" — but its own section header at
      line 65 is the unqualified "the ONLY place green and orange are allowed",
      so the reader who checks finds the claim contradicted in the very file
      cited to support it. The rule this expresses is real and worth keeping; it
      is the mark palette that is out of scope, exactly as `DTheme.qml:81`
      ("they are NOT the interface palette") already says.
      **Severity:** low — defect in the prose, not the code. It matters because
      an absolute claim a reader disproves on first check stops being consulted,
      and this one is the reason a future author will not reach for green.

      **Fixed** in `4ab8c84`, in **both** places rather than only the one the
      box names. `DStatusBar.qml` now says "the only green and orange THE
      INTERFACE uses"; `DTheme.qml`'s section header, which the box correctly
      identifies as the unqualified half of the file it sends the reader to, is
      now "the only INTERFACE green and orange".

      Both carry the reason the qualifier is load-bearing rather than pedantic —
      that the marks are outside the rule rather than exceptions to it, because
      an identicon ink is selected by an address and never signals a state — so
      a reader who checks finds the claim and its scope together.

## What was clean

The comment prose in this piece is, on the whole, exactly what CLAUDE.md asks
for, and most of it will not rot.

**The `lampState` rationale is discoverable where it is needed, and check 3
comes back negative.** The concern was that the trap is invisible — no error, no
warning, a `PropertyChanges` that silently does nothing — so one rename undoes
it. It does not, for three independent reasons that each stand alone: the
19-line comment at `DStatusBar.qml:67-80` sits immediately above the declaration,
which is the point a renamer edits; `design.md` D1 records the probe and
explicitly names and rejects the tempting counter-argument ("we will never use
QML states here — a fact about today with no expiry date"); and
`tst_status_bar.qml:209`
(`test_a_lamp_does_not_shadow_the_item_state_machine`) **fails** on the rename,
asserting the built-in `state` is still `""`. Comment, rationale and an executable
guard, in the three places a person would look. The comment's one pinned detail —
"Qt 6.10.3" — is the self-invalidating kind: it names the version the measurement
was taken on rather than asserting a fact with no date.

**Naming.** `lampState` says what it is and why it is not `state`. `markMutedAlpha`
reads correctly as theme-scoped (`mark*`), an alpha, and about muting;
`DTheme.qml:186-192` explains why it is a token and not a literal without
restating the code. `destructive-outline` and `secondary-micro` each name a
visual kind rather than a use site, which is right — a kind named
`moderation-cancel` would have been wrong the moment a second screen used it —
and each carries a comment giving the *reason* the kind exists (two filled reds
read as one decision offered twice; a full-size secondary out-weighs a 19px row)
rather than describing its colours. The `D` prefix is on all three new types and
all three are in `qmldir`.

**Test names and failure messages.** The names are sentences that state the
invariant, not the mechanics — `test_a_vouch_must_not_need_a_hover_to_be_found`'s
sibling messages, `test_the_mark_is_labelled_because_position_alone_cannot_say_whose_it_is`,
`test_an_unrecognised_kind_falls_back_to_secondary`. Failure messages interpolate
the offending value and say what the failure *means* rather than what compared
unequal: "renders with neither fill nor border — an invisible control that still
accepts clicks", "the stamp renders a digit — a vouch is never counted: …", "the
item's built-in `state` was written to — a lamp property is shadowing
QQuickItem.state". A reader who gets one of these in CI can act on it without
opening the file.

**The `FlatButton` table.** The comment at lines 5-10 says what the shape change
bought (a kind is one object; a missing field is visible where the kind is
defined) rather than describing the table, and the behaviour-change note at
lines 27-38 traces the *old* code's failure rather than asserting it — which is
what makes it checkable. The `fill: ""` / `stroke: ""` sentinel is documented at
the one place a reader needs it.

**Where mutations are recorded** — check 4 — is defensible rather than a gap.
`tasks.md` 5.4 says plainly that the mutations are named in the PR body, and the
PR body names them; `tasks.md` 2.1 additionally names its mutation inline
("restoring the old fallback spec (mutation B3)"). The two mutations that
mattered most — the assertions that *could not fail* for the reason they named —
are recorded in `design.md`'s "What the tests here cannot see" **and** in the
test files themselves, which is where a reader tempted to rewrite them will be.
Not raised as a box: nothing is misleading, and a reader is not sent anywhere
that fails them.

**`design.md` and `tasks.md` read cold** — check 5. The struck spec row at
`tasks.md:3-10` gives the reason (no spec-writer ran; the bundle is the
contract), says what would have been in a delta (nothing — no requirement is
added), and points at the three `NO SPEC:` markers as the follow-up, so the
absence is legible as a decision rather than an omission. `tasks.md` 1.2 and
`design.md` D6 handle the brief's wrong `apparatusWidth` claim the right way
round: they record what was measured, name the commit, and say why the box is
ticked with no code of the author's involved. The two citation defects above are
the only places either document sends a reader somewhere unhelpful.

## Tree state

The `font` mutation was applied and restored in the reviewer worktree
`.claude/worktrees/review-shell-readability`; `git status --short` there reports
clean, and the worktree was removed after this file was committed. No mutation
was ever applied to `piece/ui-shell-components`.
