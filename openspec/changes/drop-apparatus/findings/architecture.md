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

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/ScreenFrame.qml` — the shell's
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

- [ ] **`dev-writer`** — `docs/UI-BRIEF.md:468-485` — the general rule is
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
