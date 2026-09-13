# Findings — design

Dimension: **design only** — do the recorded decisions match the code, and were
the decisions worth recording recorded. Correctness, security, readability,
architecture and spec-test are reviewed separately and are not re-reported here.

Reviewed `piece/drop-apparatus` at `4ab9144` against `origin/main` at `86893f9`.
`origin/main` has moved since the earlier reviews ran (PR #64 rewrote
`docs/UI-BRIEF.md` wholesale), which is relevant to two boxes below.

## What was re-run rather than read

Every figure in `design.md` §4 reproduces exactly. One probe, Qt 6.10.3, driven
through `run-qml-tests.sh` with a spec path:

| Claim in `design.md` §4 | Measured |
|---|---|
| `fillHeight` child gets 544 in a 600px-high card | `frame.h=600 filler.h=544` ✓ |
| two 40px rows: `implicitHeight` 156, packed y=0, y=60 | `156`, `r1.y=0 r2.y=60` ✓ |
| the same card at `height: 600` scatters to y=111, y=393 | `r1.y=111 r2.y=393` ✓ |
| trailing `Item { Layout.fillHeight: true }` gives 176 | `176`, rows y=0, y=60 ✓ |
| `Layout.fillHeight` on a real child gives 156 | `156`, rows y=0, y=60 ✓ |
| the 20px delta is `Theme.blockGap` exactly | `blockGap=20` ✓ |

`origin/main`'s `ScreenFrame` genuinely has no `implicitHeight` and its
`RowLayout` genuinely uses `anchors.fill: parent`, so §4's diagnosis of the
original defect is right. Measured in a harness shaped like `Main.qml`,
`origin/main` gives `frame.h=0 implicitH=0`; the branch gives `116`. That half of
the fix is real.

`has_more` is computed at `dialectica/rust-lib/dialectica-core/src/feed.rs:267` as
`end < rows.len()` over this peer's own slice, so the extent-claim reasoning
behind obligation 10 rests on the code and not on a paraphrase. The ordering
label is `"same order for everyone"` (`FeedScreen.qml:63`) and the relocated
sentence is byte-identical to the deleted `MarginNote`'s `body` — §3's "verbatim"
holds. `docs/OPENSPEC-ARCHIVE.md:30` confirms `design.md`/`proposal.md`/`tasks.md`
are moved rather than deleted, so the archival reasoning in both documents is
sound. The full suite is green.

**On obligation 10, which the dispatch asked me to judge: it is coherent, and it
is not the apparatus re-created.** The apparatus printed an explanation of the
*design* in a margin addressed to a designer; obligation 10's second half
requires a sentence addressed to the *reader*, qualifying a claim the interface
itself just made, rendered only in the state that makes it. `UI-BRIEF.md:793-794`
supplies the discriminating test in passing — *"the claim is made by a control,
so no structure can unmake it"* — and the explicit non-obligation at `:787-790`
forecloses the disclaimer failure mode. The brief argues its own case. Two
narrower problems with how it was recorded are boxed below.

## Findings

- [ ] **`dev-writer`** — `design.md` — the branch's most consequential decision,
      rendering obligation 10, is **not in `design.md` at all**
      **Measured:** `grep -n "obligation 10\|extent claim\|extent"` over
      `design.md` returns **nothing**. The decision — a new brief obligation, the
      first that requires a sentence rather than a structural discharge, with two
      alternatives explicitly rejected ("a locality line wherever a count
      appears", "the empty-screen-only behaviour is correct as it stands") —
      lives in `proposal.md` under *The obligation the move narrowed* and in
      `findings/correctness.md`.
      **Why that placement is wrong rather than merely unusual:** `design.md` is
      this change's Decisions record, and a design reviewer, a later archaeologist
      or the `dev-writer` who must still close the open `FeedScreen.qml` box all
      read `design.md` first. §4 runs to eight subsections on a layout binding;
      the obligation that changes what every future screen owes its reader gets
      none. The asymmetry will read to the next person as "the layout was the
      decision and the obligation was a detail", which is backwards.
      `proposal.md` does survive archive, so nothing is *lost* — but a decision
      recorded only in the proposal is a decision `design.md` claims did not
      happen.
      **What the entry needs**, judged against what a good Decisions entry
      contains: it has what was chosen, the constraint (paging asserts extent and
      `hasMore` is local), both alternatives and what ruled each out — all already
      written in `proposal.md`. What is missing everywhere is **what it costs**:
      obligation 10's second half is the first brief obligation dischargeable only
      by prose, in a change whose thesis is that obligations are things the
      interface *does*. That tension is real and defensible, and naming it is what
      stops the next author reading the obligation as licence for the disclaimer
      the same obligation forbids.

- [ ] **`dev-writer`** — `design.md:34` and `:66-70` — §2's `ON WHAT YOU HOLD`
      row still makes the claim the `spec-writer` has since retracted, and points
      at a file the closer deletes
      **The row (`:34`) reads:** the obligation survives "And in the **interface
      already**: `FeedScreen.qml`'s empty state says …". `:66-70` then says the
      narrowing "is **not** mine to make" and files it as "a `spec-writer` box in
      `findings/correctness.md`".
      **Both halves are now stale.** The `spec-writer` answered that box on
      `0073ed9` — the answer is `docs/UI-BRIEF.md` rendering obligation 10, and
      `findings/correctness.md:191-196` explicitly hands the `design.md` edit back
      to `dev-writer` ("**One documentation edit belongs with the code fix**,
      because it is your file and not mine"). So §2 still records an open
      question that is closed, and the row still overclaims exactly as
      `findings/architecture.md`'s third box said it did.
      **The pointer is the worse half.** `findings/` is deleted at archive —
      `docs/OPENSPEC-ARCHIVE.md:27-32` moves `proposal.md`, `design.md` and
      `tasks.md` and says nothing about `findings/`, and the closer's own
      `tasks.md` row is "findings all ticked, `findings/` deleted". So on merge
      `design.md:69` becomes a citation to a file that does not exist, guarding a
      claim the same paragraph admits is too strong. §2's `ON THE MARK` row was
      already narrowed this way in `8.5`; this row needs the same treatment,
      pointing at obligation 10 and `proposal.md` rather than at `findings/`.

- [ ] **`dev-writer`** — `docs/UI-BRIEF.md:510-519` — the shell contract's two
      middle bullets contradict each other, and the `fillHeight` promise is false
      in the configuration the same list mandates
      **The two bullets, five lines apart:**
      `:510` — "**A child with `Layout.fillHeight: true` gets real slack** — it
      grows to fill the card rather than collapsing to nothing."
      `:513` — "**Do not give a `ScreenFrame` an explicit `height`.** Let it size
      from its content."
      **An author who follows the second gets the collapse the first promises
      cannot happen.** Measured, same probe, Qt 6.10.3 — a `ScreenFrame` with a
      40px row and a bare `Rectangle { Layout.fillWidth: true; Layout.fillHeight:
      true }`:

      | Frame | `frame.h` | `filler.h` |
      |---|---|---|
      | `height: 600` (explicit — the forbidden form) | 600 | **544** |
      | no explicit height (the mandated form) | 116 | **0** |
      | inside a `Main.qml`-shaped Flickable + ColumnLayout | 116 | **0** |

      The third row is the one that matters: that is the only call shape in the
      tree. `Main.qml:51-57` assigns the frame `Layout.alignment` and
      `Layout.preferredWidth` and no height, so **in the app as shipped a
      `fillHeight` child of a `ScreenFrame` is still zero-height.** On
      `origin/main` the same harness measures `frame.h=0 implicitH=0 filler.h=0`,
      so the branch fixed the card's height and did not fix this.
      **Why this is a design finding and not a correctness one.** The 544
      measurement is real and I reproduced it; what is wrong is the **scope
      claimed for it**. `findings/correctness.md`'s first box was filed against a
      `ScreenFrame { width: 1000; height: 600 }` and closed against the same
      markup, and `design.md:162-165` records the fix as "A `fillHeight` child
      gets the real slack (544 again)" without the qualifier. The brief then turns
      that unqualified sentence into guidance addressed to the exact authors on
      #60, #62 and #63 whom §4 names as the reason the fix mattered — and it is
      wrong for them, because §4's own trade paragraph (`:178-189`) establishes
      that no caller gives a card an explicit height.
      **The two sentences are individually true and jointly misleading**, which is
      the shape `findings/readability.md` already caught once in this file's
      escape hatch: a reader takes the remedy that needs least judgement, and here
      the remedy is "declare `fillHeight` and trust the bullet". Whether the right
      answer is to qualify the bullet ("in a card given an explicit height"), to
      make `ScreenFrame` fill its parent, or to say plainly that a body cannot
      fill a content-sized card is a decision, not a wording fix — and it is not
      recorded anywhere, because nobody noticed the two bullets disagree.
      **Verified:** the numbers above are from one run; the `Main.qml`-shaped case
      was measured in a detached `origin/main` worktree and in this one.

- [ ] **`dev-writer`** — `docs/UI-BRIEF.md:501` — inserting the first-ever `###`
      before obligation 1 nests all ten rendering obligations inside the
      `ScreenFrame` subsection
      **Measured:** on `origin/main`, `## Non-negotiable rendering obligations`
      (`:522`) has **no subsections at all** — obligations 1-9 sit directly under
      it, bold-numbered rather than headed. This branch adds `### What
      `ScreenFrame` gives you, and the one thing it asks` at `:501`, and the next
      heading of any level is `## The vote control` at `:806`. So obligations 1
      through 10, at `:526-802`, are now inside that `###` by document structure.
      **Why it matters for a live document read by a non-coder.** The brief is
      designed against by an external designer and, per this change's own edit,
      by a QML implementer; both navigate by heading. A designer opening the
      outline sees ten rendering obligations filed under a QML component's layout
      contract, and the implementer-facing section — which is genuinely addressed
      to a different reader than the obligations are — now appears to govern them.
      **`design.md:232-238` records the placement as deliberate** ("placed
      immediately after the apparatus box … so a reader meets them together") and
      the intent is sound; what is unrecorded is that the chosen heading *level*
      swallowed the list. Either the section belongs after obligation 10, or it
      belongs at `##`, or the obligations need a `###` of their own — a decision,
      and a cheap one, but it was made by accident rather than taken.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/FeedScreen.qml:225` — the
      relocated ordering sentence lost its `copy.json` provenance comment, and no
      document says the drop was intended
      **Measured:** the deleted `MarginNote` carried `// copy.json
      \`feed.orderingNote\`` immediately above its `body`. The `Text` that
      replaces it carries a fourteen-line comment about *why* the sentence is
      there and **no bundle key**. Every other bundle-sourced string in the file
      keeps one — `states.failedTitle` (`:256`), `states.failedBody` (`:266`),
      `states.retry` (`:290`), `states.emptyTitle` (`:317`), `states.emptyBody`
      (`:327`), `compose.blockedTitle` (`:449`) — as do two in
      `SanitisedText.qml`. So the convention is uniform across the tree and this
      is the one place it lapses, which is the "rule followed at three call sites
      and missed at a fourth" shape CLAUDE.md warns about.
      **Why it is more than tidiness here.** `design.md` §7 is a hand-off built
      entirely on bundle-key traceability — it reasons about what
      `compose.apparatus` requires and hands the question to the composer piece's
      `spec-writer`. A change that argues from bundle keys in §7 while silently
      dropping one in §3 leaves the next person unable to tell which strings are
      still the bundle's. If the drop was deliberate — the text is no longer a
      *margin* note, so arguably `feed.orderingNote` no longer names it — that is
      a decision with a real alternative and belongs in §3 beside "verbatim, since
      its wording was already reviewed". Nothing in `design.md`, `proposal.md` or
      `tasks.md` mentions `copy.json` except §7's "there is no `copy.json` in the
      tree at all", which is about a different question.

- [ ] **`dev-writer`** — `tasks.md:82` — the stale `UI-BRIEF.md` citation that
      `findings/readability.md` fixed in `design.md` was left uncorrected in its
      twin, and `origin/main` has since moved again
      **Measured:** task 2.2 cites obligation 6 layer 2 as `UI-BRIEF.md:584-593`.
      `git grep -n "attack is purely social" origin/main -- docs/UI-BRIEF.md`
      now returns **641**. The readability review measured 622 at the time it ran
      and fixed `design.md:35` by naming the heading instead; `tasks.md` carries
      the identical citation and was not touched. `origin/main` moved again since
      (`86893f9`, PR #64, which rewrote this file), so the number is now wrong by
      a different amount than when it was filed.
      **This is the same defect in the same change, at the second of two
      locations** — the shape that motivates fixing a family rather than an
      instance. `tasks.md` survives archive alongside `design.md`, so both copies
      persist and a later reader has a 50% chance of hitting the broken one. The
      fix is the one already applied next door: quote the heading, drop the range.
      `tasks.md:69`'s `UI-BRIEF.md:86-88` is fine — I checked, constraint 1 is
      still at 88 on today's `origin/main`.

- [ ] **`dev-writer`** — `proposal.md:143-153` — the Impact section says "**No
      test changes**" and omits `dialectica-ui/tests/run-qml-tests.sh`, which this
      change modifies
      **Measured:** `git diff --stat origin/main...piece/drop-apparatus` lists
      `dialectica-ui/tests/run-qml-tests.sh | 21 ++`. Commit `4ab9144` adds a
      single-spec argument mode to the runner. `proposal.md`'s **Modified:** list
      names five files and not this one; `design.md` §6 says "No test changes";
      `tasks.md` mentions the script only as a gate that was run.
      **`CLAUDE.md` is missing from the same list**, and it is also in the diff —
      the "Where to look for what" row gained the *before writing a QML screen*
      trigger. That edit **is** argued, in `design.md:232-238` and in
      `findings/architecture.md`, so only the Impact list needs the line; I name
      it here rather than in its own box because the defect is one list, not two.
      **The change itself is well-argued** — its commit message explains that
      invoking `qmltestrunner` directly means hand-writing
      `QT_QPA_PLATFORM=offscreen`, a shape the permission checker cannot analyse,
      and that the script already exported the variable for that reason. That
      reasoning is exactly what a Decisions entry is for, and it currently exists
      only in `git log`, which archive does not index and a reader of the change
      folder will not consult. It is also a decision with a live alternative: a
      wrapper that takes a spec versus leaving callers to reach past the script,
      and the choice has a cost (the script now has two modes, and the glob branch
      and the argument branch can drift).
      **The stricter half of the box:** an Impact list that says "no test changes"
      while the diff touches the test runner is the kind of claim a reviewer uses
      to decide what not to look at. Whether the runner counts as "a test" is
      arguable; whether it counts as **modified** is not.

## Judgement where no box is needed

**§4 is in good shape and I tried hard to break it.** It was extended at least
twice and rewritten passages are where stale claims hide, so I re-ran every
figure rather than reading them — all six reproduce, including the two that
reject alternatives, which is where an unmeasured assertion would have been
cheapest. The "worse than this section first claimed" sentence flags its own
supersession instead of quietly overwriting. The trade it accepts is stated with
what it forecloses, which is the part of a Decisions entry that usually goes
missing.

**§7 and §8 are model hand-offs.** Both decline to act and both bound the risk
with measurements rather than with confidence: §7's three facts (nothing merged,
no implementation on any branch, the blocking test on no branch) and §8's
verification that `ON PUBLISHING` exists on neither `origin/main` nor
`piece/ui-composer`'s pushed tip. A hand-off that shows why declining is safe is
worth more than one that is merely polite.

**`docs/PLAN.md` is untouched and no reasoning needed migrating out of it.** I
checked directly rather than assuming: the apparatus, margin notes and the word
"marginal" in the design sense appear nowhere in `origin/main`'s PLAN.md, and it
carries no reasoning about counts, extent or feed locality that obligation 10
displaces. PLAN.md:3577's note that §11.1 "Rendering obligations, collected"
arrives with `vouching-state` confirms `proposal.md:58-61`'s claim that there is
no PLAN.md section for this obligation to have been lifted from. Nothing in this
change contradicts PLAN.md, and its §9.1 Stage A framing (`Main.qml:12-15`) still
describes what the branch ships.

**§5's `paperDeep` argument is the right call and is recorded as one.** Keeping an
unreferenced palette token while deleting `apparatusWidth` with its two dead
readers draws the line where a palette stays a designed set rather than a record
of current usage. It names what it costs — a token nothing reads — which is what
makes it a decision rather than an omission.
