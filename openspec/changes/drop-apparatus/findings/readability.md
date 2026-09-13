# Findings — readability

Dimension: **readability only.** Architecture is in `findings/architecture.md`
(same reviewer, separate file); correctness and security were reviewed
separately and are not re-reported here.

Reviewed `piece/drop-apparatus` at `843b445` — `ScreenFrame.qml`, `FeedScreen.qml`,
`Theme.qml`, `qmldir`, `docs/UI-BRIEF.md`, `design.md`, `proposal.md`, `tasks.md`.

Every number below came from running something in a throwaway worktree. The probe
was a temporary `tst_zzprobe.qml` driven by `/usr/lib64/qt6/bin/qmltestrunner`
against `dialectica-ui/src/qml`; it is deleted and the tree is clean.

## What was re-measured rather than read

Every figure in `ScreenFrame.qml`'s comment block checks out, and I ran each
rather than trusting it:

| Claim in the comment | Measured |
|---|---|
| `fillHeight` child gets 544 in a 600px card | `frame.h=600 filler.h=544` ✓ |
| explicit height scatters two 40px rows to y=111, y=393 | `r1.y=111 r2.y=393` ✓ |
| implicit height packs them at y=0, y=60 | `r1.y=0 r2.y=60` ✓ |
| `Theme.blockGap` is the 20px the spacer would add | `blockGap: 20` ✓ |
| suite is 41 green | 13 + 12 + 7 + 9 = 41 passed, 0 failed ✓ |

The stale fragment the last pass found ("ALL FOUR edges are anchored") is gone —
`grep -rn "ALL FOUR\|all four\|four edges"` over `dialectica-ui/src/qml/` and
`openspec/changes/drop-apparatus/` returns nothing.

`design.md` §4's two rejected alternatives are both stated with their numbers, and
the trailing-spacer rejection is the one I could most easily have disagreed with;
measuring it confirmed the 20px inflation is real (`implicitHeight` 156 without a
spacer, 176 with one — exactly `blockGap`).

## Findings

- [x] **`dev-writer`** — `ScreenFrame.qml:82-85` — the escape hatch the comment
      prescribes re-introduces the 20px defect the same comment rejects, and says
      nothing about it
      **Scenario:** a screen author hits the scatter, reads line 82 ("**If you
      give a ScreenFrame an explicit height and its content scatters, this is
      why** — add `Layout.fillHeight` to the child that should absorb the slack,
      or a trailing `Item { Layout.fillHeight: true }` in that screen"), and takes
      the second option because it needs no judgement about which child should
      grow. Their card's `implicitHeight` silently grows by `Theme.blockGap`.
      **Measured**, same probe, same Qt 6.10.3: a `ScreenFrame` holding two 40px
      rows measures `implicitHeight=156`; the identical frame with a trailing
      `Item { Layout.fillHeight: true }` measures **176**. The 20px delta is
      `blockGap` exactly. Lines 77-80 of the same file reject the *global* spacer
      for precisely this — "adds a `spacing` gap to `body.implicitHeight`,
      inflating the card by 20px so the Flickable scrolls past the end of the
      content" — but lines 82-85 recommend the *per-screen* spacer with no such
      warning, and the mechanism is identical. Both suggested fixes do work for
      the scatter (`r1.y=0 r2.y=60` under both), so the reader gets a correct
      answer to the question they asked and an uncorrected regression on the one
      they did not.
      **Severity: medium.** This is a genuine defect in the guidance, not a
      stylistic preference: the file's whole claim to legibility is that it tells
      the next author what to do, and the sentence that does so contradicts the
      paragraph five lines above it. The fix is one clause — say the spacer costs
      `blockGap` on `implicitHeight` and that the `fillHeight`-on-a-real-child
      form does not.

      **Fixed** in `291c719`. I re-measured rather than taking the figures, and
      added the third row your probe did not need — the form the comment should
      recommend — so the comparison is between the two remedies rather than
      against the broken case. One run, two 40px rows, Qt 6.10.3:

      | Form | `implicitHeight` | rows at `height: 600` |
      |---|---|---|
      | no slack-absorbing child | 156 | y=111, y=393 |
      | trailing `Item { Layout.fillHeight: true }` | **176** | y=0, y=60 |
      | `Layout.fillHeight` on a real child | **156** | y=0, y=60 |

      Your 156/176 reproduces exactly, and the delta is `Theme.blockGap`. The
      third row is what makes this a defect rather than a trade: the
      `fillHeight`-on-a-real-child form fixes the scatter at **no** cost to
      `implicitHeight`, so the comment was not offering two options with
      different prices — it was offering the right answer and a strictly worse
      one, unlabelled.

      The comment now names `Layout.fillHeight` on a real child as *the* fix and
      says what the spacer costs and why (`spacing` gap entering
      `body.implicitHeight`). It no longer reads as a choice. Recorded in
      `design.md` §4 under *The two escape hatches are not equivalent*, with the
      table, so it survives the deletion of this file.

      The general lesson is written there too: a reader who hits a symptom takes
      whichever remedy needs least judgement, so offering two and warning about
      neither means the cheaper-looking one gets picked.

- [x] **`dev-writer`** — `openspec/changes/drop-apparatus/design.md:35` — the
      `ON THE MARK` row cites a `UI-BRIEF.md` line range that is right on no
      branch, under a heading that says which branch it is from
      **Scenario:** §2 states "Line numbers are `origin/main`'s" (`:30`) and then
      cites "**Brief obligation 6, layer 2**, `UI-BRIEF.md:584-593`". On `main`
      that passage ("The attack is purely social, and it is defeated by showing
      the address") is at **line 622** — verified by `git grep -n "attack is
      purely social" main -- docs/UI-BRIEF.md`. On this branch it is at 585-591,
      so the citation is near-right *on the branch* by coincidence and wrong on
      the basis the section declares. The sibling citations are fine: `:86-88` is
      line 87 on both branches, `:331-375` brackets the Feed section, which is at
      327 on both.
      **Why it matters beyond tidiness:** this is the row the last review already
      narrowed once, and its value now rests entirely on a reader being able to
      go and read brief obligation 6 for themselves — that is the whole argument
      for accepting the "never a proof" half as undischarged. A pointer that
      lands 38 lines away on the branch it names is the one kind of error this
      row cannot afford.
      **Severity: low**, and it is a documentation defect rather than a code one.
      Naming the heading instead of the line range would not rot.

      **Fixed** in `291c719`, by naming the heading as you suggest. Verified
      independently before changing anything: `git grep -n "attack is purely
      social" origin/main -- docs/UI-BRIEF.md` gives **622**, the same grep on
      this branch gives **590**, and the citation said 584-593. Wrong on the
      basis the section declares, near-right on the branch by coincidence —
      exactly as filed.

      The row now cites **"Brief obligation 6"** with its heading quoted
      verbatim — *"A generated name is never unique and never an identifier —
      the address is."* — and names **layer 2, the identicon**, within its
      four-layer list. I confirmed that heading string is present on the branch
      (`grep -n "A generated name is never unique"` → 585) and it is the text of
      the obligation rather than my paraphrase, so a reader can find it with a
      grep on any branch.

      I checked the two sibling citations you said were fine rather than
      assuming: `:86-88` is line **87** on `origin/main` (the sentence wraps,
      which is why a grep for the whole phrase misses it), and the Feed section
      heading is at **327**, inside the cited `:331-375` bracket. Both left as
      they are — your assessment holds.

- [x] **`dev-writer`** — `ScreenFrame.qml:4-12, 23-29, 36-85` — 74 of 102 lines
      are comment, and four passages narrate what the file used to be
      **Scenario:** `grep -c "^\s*//"` gives **74** against `grep -c ""` = **102**
      — 73% comment, wrapping 16 lines of QML. Four separate passages tell the
      reader about a shape that no longer exists: `:7-10` ("It used to be a
      two-column RowLayout…"), `:24-27` ("Previously the RowLayout filled the
      card…"), `:37-49` (the three-edge form that was tried), `:60` ("matching
      the two-column shell this replaced"), plus `:77-80` on the spacer that was
      tried. Every one of those is a true and interesting fact, and every one of
      them is in `git log` and in `design.md` §4, which archive **moves rather
      than deletes** (`docs/OPENSPEC-ARCHIVE.md:30`) — so nothing is lost by
      pruning them here.
      **Why this is a finding and not taste:** CLAUDE.md's own rule is "Do not
      append a changelog of what landed — that is the failure mode this section
      exists to prevent", and the standard it sets for a comment is that it
      "earn[s] its place by saying what a command cannot". `git log -p
      dialectica-ui/src/qml/ScreenFrame.qml` answers "what did this used to be";
      the two things it does *not* answer — why `body.height` is bound at all,
      and what to do when your card scatters — are currently the shortest parts
      of the block and are buried at lines 37-49 and 82-85 respectively, after
      three paragraphs of history. The next author reads top-down and meets the
      obsolete two-column shape before they meet the live contract.
      **Severity: low (stylistic, but against a written repo rule).** The ask is
      to cut the historical narration, keep the two operative paragraphs, and
      leave the archaeology in `design.md` §4 where a reader who wants it will
      look.

      **Fixed** in `291c719`, as asked: the historical narration is cut, the two
      operative paragraphs are kept, and `design.md` §4 gained a closing
      paragraph saying the comment shrank and what it kept, so the archaeology
      has a stated home rather than merely still existing in `git log`.

      Measured before and after with your own commands — `grep -c "^\s*//"`
      against `grep -c ""`:

      | | comment lines | total | QML body |
      |---|---|---|---|
      | before | 74 | 102 | 28 |
      | after | 53 | 81 | 28 |

      The 21 lines removed are all comment; the QML is byte-identical apart from
      the header. `grep -n "used to be\|Previously\|this replaced\|was tried\|
      two-column"` over the file now returns nothing — all four passages you
      listed are gone, plus the `:77-80` spacer paragraph, whose surviving
      content moved into the escape hatch where it is now operative advice
      rather than history.

      **One thing I did not do, and the reason.** Your ask implies the ordering
      problem too — the operative paragraphs were buried after three paragraphs
      of history. Cutting the history fixes that as a side effect, so I did not
      also reorder: the file now opens with what the card is, points at the
      brief section for the contract, and the two remaining blocks are the
      `implicitHeight` rationale and the `body.height` rationale, each attached
      to the line it explains. Moving them further would separate a comment from
      its code, which is the trade the other direction.

      Related, and the reason this box and `architecture.md`'s first box were
      answered in one commit: the header no longer tries to *be* the contract.
      It points at `docs/UI-BRIEF.md`'s *What `ScreenFrame` gives you* and asks
      that the two be kept in step. Some of the 21 lines went because the
      contract they were carrying now lives somewhere a screen author reaches
      without opening this file.

## Areas that were clean

**`FeedScreen.qml`'s relocated ordering sentence is the best-written part of this
change.** The `Text` at `:225-233` is preceded by a comment (`:211-224`) that
answers exactly the question a future reader will ask — "isn't this redundant
with the label that already says 'same order for everyone'?" — and answers it
with the distinction that makes it not redundant (the label is neutral, the
sentence is a denial, and a reader assumes newest-first unless told otherwise).
That comment is the reason the sentence will survive the next tidy-up pass, and
it is the model the rest of the change should have followed.

**The obligations a reader meets as decisions rather than results.** I checked
this specifically because it is what the dispatch asked. `design.md` §3 does not
merely say the ordering sentence moved; it states the rejected alternative
(leave it to the brief alone) and the reason the rejection is not arbitrary —
"a brief obligation with no interface text is precisely how the apparatus came to
ship in the first place". §2's `ON THE MARK` subsection likewise names the
misreading to prevent rather than just recording the outcome. A reader meets the
reasoning, not only the answer.

**`docs/UI-BRIEF.md`'s new box reads correctly for its audience.** It is placed
under *Non-negotiable rendering obligations*, which is the section a designer
opens when deciding what a screen owes, and it opens with the rule rather than
with the incident. Its closing example ("'the mark is never proof' is met by
printing the address beside every mark") is the one that makes the abstract rule
actionable. The matching Feed-section clause at `:370-376` is correctly placed
among the ordering rules rather than appended at the end.

**`Theme.qml`'s two corrected comments say what the token is for rather than
where it currently appears** — `paperDeep` is now "a deeper paper, for panels
inset in a card" instead of "apparatus column", which is the form that does not
go stale when the next panel moves. `accent` lost "apparatus rules" and kept its
two real roles.

**Not a finding, recorded so the absence is legible.** I checked whether
`design.md` §4's rewrite left a claim behind after being extended twice, since
that is where stale text hides. It did not: §4's original `implicitHeight`
paragraphs and the added `fillHeight` subsection do not contradict each other,
the "worse than this section first claimed" sentence explicitly flags its own
supersession rather than silently overwriting, and the two rejected alternatives
each carry the number that rejected them. `tasks.md` §8's six boxes each name a
measurement rather than an assertion.
