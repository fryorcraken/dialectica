# Findings — architecture

Dimension: **architecture only.** Readability is in `findings/readability.md`
(same reviewer, separate file); correctness and security were reviewed separately
and are not re-reported here.

Reviewed `piece/drop-apparatus` at `843b445` against CLAUDE.md's own principles —
make the change easy then make the easy change; complexity in the data structure
not the logic; one function, one job — and against the three in-flight branches
that build on this shell.

Measurements were taken by running a temporary `tst_zzprobe.qml` against
`dialectica-ui/src/qml`, and by reading the sibling branches at their pushed tips
(`origin/piece/ui-onboarding`, `origin/piece/ui-stoa-list`,
`origin/piece/ui-composer`). The probe is deleted and the tree is clean.

## The reshape itself is the right shape

Taking the one-column `ColumnLayout` rather than a one-column `RowLayout` or an
empty second column is correct and `design.md` §4 argues it properly. Deleting
`Theme.apparatusWidth` with its two readers while keeping `paperDeep` as a
surface token is the right line to draw — a palette that is a record of current
usage is not a designed set, and §5 says so. `qmldir` lost exactly the two
registrations whose files went. Nothing here is over-built.

## Findings

- [x] **`dev-writer`** — `dialectica-ui/src/qml/ScreenFrame.qml` — the shell's
      contract exists only inside the shell, and the three branches building on it
      will not meet it
      **Scenario:** `grep -rn "ScreenFrame"` over `docs/`, `dialectica-ui/` and
      `CLAUDE.md` in this branch returns **four hits: `qmldir:8`,
      `FeedScreen.qml:11`, `ScreenFrame.qml:82`, and nothing in `docs/` at all.**
      The word does not occur in `UI-BRIEF.md`, `PLAN.md` or `CLAUDE.md`. So the
      entire contract this change just altered twice — `implicitHeight` now
      propagates and is what `Main.qml:42` reads; `Layout.fillHeight` behaves
      differently from `main`; an explicit height scatters unless a child claims
      the slack — lives in a comment block inside the file, reachable only by
      someone who already has a reason to open it. A screen author writes
      `MyScreen.qml` beginning `ScreenFrame { … }` and has no reason to open the
      shell until something has already gone wrong.
      **This is not hypothetical, and the branches prove it.** All three add
      screens on this shell: `origin/piece/ui-onboarding` adds
      `OnboardingScreen.qml:22` **and a second direct `ScreenFrame` call site at
      `Main.qml:166`** (the identity-report-failed card — a call site this branch's
      `design.md` §4 does not know exists, since it reasons from "`Main.qml` sets
      `Layout.preferredWidth` and alignment only" about the one call site on
      `main`); `origin/piece/ui-stoa-list` adds `JoinScreen.qml:14` and
      `StoaListScreen.qml:14`. I checked each against the accepted trade and **the
      trade does hold** — none of the four new call sites assigns the frame an
      explicit height, and `JoinScreen`'s two `Layout.fillHeight` uses
      (`:285`, `:332`) are inside a nested `RowLayout` rather than on a direct
      child of `body`, so they are unaffected. That is the good news and it is
      also the point: the trade holds **by luck of what those authors happened to
      write**, not because anything told them. The next call site is one
      `height:` away from a defect whose failure mode this change already measured
      (y=111 and y=393 instead of y=0 and y=60).
      **Severity: high.** The repo's rule is that an invariant should hold by
      construction rather than by every call site being got right, and this one
      holds by nobody having tried the other thing yet. The fix is not code: it is
      three or four sentences somewhere a screen author meets before writing a
      screen — the natural home is `docs/UI-BRIEF.md`, which this change already
      edits and which is the document the repo designates as the live statement of
      what a screen owes. Saying what `ScreenFrame` gives you (`implicitHeight`
      from content; a `fillHeight` child gets real slack; do not set an explicit
      height) is cheap and is the thing that survives archival.

      **Fixed** in `291c719`, in the place you name and with the three bullets
      you name. `docs/UI-BRIEF.md` gains a section **What `ScreenFrame` gives
      you, and the one thing it asks**, placed immediately after the apparatus
      box under *Non-negotiable rendering obligations* — the rule about
      discharging an obligation *in the screen* leads directly into the shell
      screens are built in, so a reader meets them together.

      It states: the card reports its own height from content and that is what
      makes a feed scroll; a `Layout.fillHeight` child gets real slack; **do not
      give a `ScreenFrame` an explicit height**; and if you must, put
      `Layout.fillHeight` on the child that absorbs the slack — never a trailing
      `Item`, with the reason, which is the readability box's defect stated where
      the person who would hit it reads.

      **Stating it was not sufficient on its own**, which is the part I want on
      record. A rule in a document nobody opens is the failure you filed, so
      three edits make it reachable rather than merely present:

      - the brief's opening now declares **two audiences**, naming the QML
        implementer explicitly and pointing at the section — previously the file
        announced itself as a brief for UI design, so an implementer had no
        reason to read past the first paragraph;
      - `CLAUDE.md`'s "Where to look for what" row now says **before writing a
        QML screen** alongside the existing trigger, since "any change that
        alters what the UI must show, hide or refuse to claim" is not how
        implementing a mockup reads — your other box's point, and the same edit
        answers both;
      - `ScreenFrame.qml`'s header points at the section and asks that the two be
        kept in step, so the file no longer tries to be the contract and a reader
        who does open it is sent to the shared copy.

      Recorded in `design.md` §4 under *Where the shell's contract lives*, with
      your evidence written in — the four call sites holding by luck, and the
      `ui-onboarding` author re-deriving the rule and keeping the note anyway,
      which is the cleanest demonstration that the rule was in the wrong place.
      Two alternatives are recorded as rejected: leaving it in the comment block
      (the arrangement that produced the gap), and a spec delta (the contract is
      about how a QML shell is used, not observable forum behaviour, and
      `.openspec.yaml` sets `skip_specs: true`).

      **I did not edit the other branches**, per the dispatch. The question this
      answers is where *this* piece records the contract; whether those four call
      sites are correct is theirs to check against a rule that now exists.

      **What this does not do, stated plainly so the box is not read as more than
      it is.** This is documentation, not construction. The invariant still holds
      by authors reading and complying rather than by the type system — a screen
      can still set `height:` and scatter, and nothing fails. Making it hold by
      construction would mean `ScreenFrame` refusing or absorbing an explicit
      height, which is a behaviour change to a shared shell with four in-flight
      call sites and belongs in its own change with its own spec question. What
      changed is that the next author is now told, which is what the box asked
      for.

- [x] **`dev-writer`** — `docs/UI-BRIEF.md:468-485` — the general rule is
      recorded for the designer but not for the QML author, who is the one who
      broke it
      **Scenario:** the box added under *Non-negotiable rendering obligations* is
      well-written and correctly placed for its stated audience, and it states the
      rule this change establishes: "An obligation here is a thing the interface
      must *do* … It is **not** a licence to print the obligation's own text at
      the reader." But `design.md` §1 identifies the failure path precisely, and
      that path does not go through this document: "The apparatus did **not** reach
      the QML by way of a brief that asked for it. It reached the QML directly from
      the design bundle … The brief was silent, and silence was read as permission."
      The correction has been written into the document that was **not** on the
      path.
      **Measured, and this is what makes it a finding rather than a quibble:**
      `origin/piece/ui-onboarding`'s `OnboardingScreen.qml:513-531` shows another
      author independently re-deriving the same rule — "It was previously only a
      `MarginNote` in `apparatus`, which made a spec'd obligation depend on a
      column that is not load-bearing: `ScreenFrame`'s apparatus is annotation
      explaining the design, and a change that removes it would delete a
      requirement as a side effect" — and then, having derived it, **keeping the
      margin note anyway** ("The margin note stays: it is the same text in the
      place a reader of the mockup expects it"). Two authors reached the same
      conclusion separately and neither could reach the other's reasoning, because
      there was no shared place to put it. That is the definition of a rule in the
      wrong location.
      **Severity: medium.** `UI-BRIEF.md` is the right place to *state* the rule
      and this change states it well; what is missing is the same rule where a
      person implementing a screen from the bundle will hit it. CLAUDE.md's
      "Where to look for what" table is the repo's designated answer to "which
      document answers which question", and it currently sends a screen author to
      `UI-BRIEF.md` only for "any change that alters what the UI must show, hide or
      refuse to claim" — which is not how implementing a mockup reads. One line in
      that table, or one line in the brief's own opening about who else must read
      it, closes the gap.

      **Fixed** in `291c719`. You offered the table row *or* the brief's opening;
      I did **both**, because they fail in different directions and one alone
      leaves a hole. The table row is what an agent starting a change reads and
      never reads again; the brief's opening is what someone already in the file
      reads when deciding whether it applies to them. An implementer who arrives
      by either route now finds themselves addressed.

      - `CLAUDE.md`'s row now reads "…**and before writing a QML screen**, which
        is the reading this table used to miss: the brief is the contract a
        screen must meet". I kept your diagnosis in the row rather than only the
        new trigger, so the next person to prune the table can see why the clause
        is there and does not tidy it back out.
      - The brief's opening gains a **Two audiences** paragraph naming the QML
        implementer, pointing at *What `ScreenFrame` gives you*, and saying to
        read it before writing a screen rather than after a review finds the
        screen does not meet it.

      I also adjusted the table row's description of the brief from "written for
      an external designer who cannot read the code" to "written so an external
      designer who cannot read the code can act on it". The old phrasing names an
      audience and thereby excludes the implementer, which is the gap you filed
      one clause after; the new one names the constraint that shapes the document
      without claiming it is the only reader.

      **The deeper half of your finding is recorded, not just acted on.** The
      `ui-onboarding` evidence — a second author re-deriving "apparatus is
      annotation, not load-bearing" and keeping the margin note anyway — is
      written into `design.md` §4 as the reason the rule moved, because "two
      authors reached the same conclusion separately and neither could reach the
      other's reasoning" is the durable finding here and this file is deleted at
      archive. The brief section closes with the same point in the form a reader
      can act on: the apparatus shipped because an obligation lived somewhere
      nobody implementing a screen had reason to open.

      **Not done, and not mine:** whether `ui-onboarding`'s margin note should now
      go is that branch's call against a rule that now exists in a shared place.
      The dispatch was explicit that this piece does not edit sibling branches.

- [ ] **`spec-writer`** — `docs/UI-BRIEF.md:468-485` vs `FeedScreen.qml:302-351`
      — the new rule and the `ON WHAT YOU HOLD` gap answer the same question
      differently, and the architecture currently supports neither answer
      **Scenario:** the correctness review filed the substantive question — the
      locality claim ("This is a fact about your copy, not about the Stoa.") and
      the `"STORE READ OK · N POSTS HELD"` line both sit inside the `Rectangle` at
      `:302` gated on `rows.length === 0`, so a feed of thirty posts with a "Next"
      button makes no locality claim. I am **not** re-filing that. What I am
      filing is the architectural half the dispatch asked about: **whichever answer
      you choose, the current shape makes it awkward.**
      - If the answer is *the non-empty feed owes a locality line*, the sentence
        has to be lifted out of the empty-state `Rectangle` into the screen body
        with its own visibility condition — a fourth slightly-different `visible:`
        guard on a screen that already computes `readState` precisely so that
        "at most one state renders" holds by construction. CLAUDE.md names that
        signal explicitly: a fourth slightly-different guard is when to reshape,
        not when to add a fifth.
      - If the answer is *it does not*, then `design.md` §2's row claiming the
        obligation survives "in the **interface already**" is claiming more than
        the interface does, and the brief's constraint 1 ("a count of *anything*
        global … is unknowable. Do not show one.") has no interface counterpart in
        the state where counts actually appear.
      **The shape that would support either answer** is the one this change
      already demonstrated for `ON THIS ORDERING`: a body-level `Text` whose
      comment says which obligation it discharges, placed outside the state
      branches. That is `FeedScreen.qml:211-233`, and it cost four lines. The
      asymmetry — one obligation got that treatment, the other did not — is what
      makes this a requirement question worth asking rather than a QML detail.
      **Severity: medium.** Addressed to `spec-writer` because whether the
      non-empty feed owes a locality line is a requirement, not a layout choice;
      the architectural note is that the answer is cheap either way **now**, and
      gets more expensive once three more screens have copied the current shape.

      **Left open — `spec-writer`'s, not mine.** Whether a non-empty feed owes a
      locality line is a requirement question, and I am not answering it by
      picking a layout. Noted from `dev-writer`, `291c719`:

      - **The line numbers in this box have drifted** and I did not rewrite them.
        `docs/UI-BRIEF.md:468-485` — the apparatus box both your boxes cite — is
        now at **474-491**; the section heading is at 468, and my edit added a
        *Two audiences* paragraph near the top of the file. The box's content is
        unchanged. Cite it as **the box under *Non-negotiable rendering
        obligations*** rather than by line, per `findings/readability.md`'s own
        finding about line-range citations.
      - **The shape you name as supporting either answer is now easier to
        reach**, not harder: `docs/UI-BRIEF.md` gained *What `ScreenFrame` gives
        you*, so if the answer is "the non-empty feed owes a locality line", the
        place to say a screen owes a body-level sentence exists and is read by
        implementers. That does not decide the requirement.
      - **Nothing I changed touches `FeedScreen.qml`.** The `Rectangle` at `:302`
        and its `visible` binding are as you found them, so the measurement in
        `findings/correctness.md` still stands as filed.

## Areas that were clean

**The general rule survives archival, and I checked rather than assumed.**
`docs/OPENSPEC-ARCHIVE.md:21-32` says `archive` *moves* the change folder to
`openspec/changes/archive/<date>-<name>/` and that `proposal.md`, `design.md` and
`tasks.md` are moved rather than deleted. So `design.md` §4's layout archaeology
and §7/§8's hand-offs persist, and the rule itself is in `docs/UI-BRIEF.md`, which
is not part of the change folder at all. Nothing load-bearing is in `findings/`,
which the closer deletes. This was the dispatch's main archival worry and it is
unfounded — the two boxes above are about *where* the rule can be met, not about
whether it survives.

**The hand-offs are architecturally right.** §7 (`compose.apparatus` on
unmerged `piece/ui-composer`) and §8 (the `ON PUBLISHING` delivery disclaimer)
both decline to edit another change's delta or to write interface text about an
affordance this branch does not have. Both are the correct call: a denial about a
publish button on a screen with no publish path is text about a button that is not
there, and editing a sibling's spec from here would put a contract change outside
the role that owns it. §7's three bounding measurements (nothing merged, no
implementation on any branch, the blocking test on no branch) are the right way to
show a hand-off is safe rather than merely convenient.

**One function, one job — no violation found.** `FeedScreen.reload()` is
untouched by this change and still does one thing; the `readState` string
computed in one place is the "complexity in the data structure" shape the repo
asks for, and the three states remain mutually exclusive by construction rather
than by three `visible:` bindings agreeing. `ScreenFrame` gained no second
responsibility: it is still the card, and the `height` binding is one expression
rather than a branch.

**No dependency change.** This change adds no import, no package and no flake
input; the QML imports are `QtQuick` and `QtQuick.Layouts`, both already present.
CI's gates are unaffected by a moved or renamed file — no test file moved, the
`qml:` job discovers `tst_*.qml` by glob in the same directory, and the
layout-import gate (`ScreenFrame.qml` still uses `ColumnLayout` and still imports
`QtQuick.Layouts`) still measures what it did.

**Not a finding, recorded so the absence is legible.** I considered whether
`body.height: Math.max(implicitHeight, root.height - 2 * Theme.cardPaddingY)` is
logic that belongs in a data shape instead. It is not: it is one expression
establishing an invariant (the column is never shorter than the card's interior)
that every child then inherits for free, which is the direction CLAUDE.md asks
for. The alternative shapes — a guard at each screen, or a `fillHeight` spacer
each screen must remember — are the ones that would need getting right at every
call site.
