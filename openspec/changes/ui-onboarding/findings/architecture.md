# Findings — architecture

Reviewed: `dialectica-ui/src/qml/Core.qml`, `Main.qml`, `OnboardingScreen.qml`,
`FeedScreen.qml`, `docs/UI-BRIEF.md`, and the four sibling pieces in flight
(`piece/ui-stoa-list`, `piece/ui-composer`, `piece/drop-apparatus`,
`piece/generated-names`). Dimension covered: **architecture** only. Readability
is my other file; correctness and security were another instance's.

Baseline: **99 QML tests across 6 spec files, all passing**.

## Defects

- [ ] **`dev-writer`** — `docs/UI-BRIEF.md:17`, `:139`, `:196-198` — this change
      removed the word count from the screen because the owner settled three,
      and left the live brief asserting **four** in three places
      **Scenario:** CLAUDE.md's table says the brief is *"a **live document**…
      designed against by an external designer who cannot read the code. **If a
      change makes it wrong, fix it in the same change**; a stale brief is worse
      than none."* This change edited `UI-BRIEF.md` (the onboarding bullet, to
      say core serves no name) and so had the file open. But `:17` still reads
      "**The generated name's vocabulary is settled as of §5.2.1**: four words",
      `:139` reads "**It is four words, and they all matter.**", and `:196-198`
      gives collision probabilities *derived from the count* — "about 0.003%" at
      a thousand and "about 0.07%" at five thousand — with an explicit
      "**The fourth word is what bought** that". The owner's decision at
      `d3e7579` on `docs/name-shape-sweep` reads: *"the generated name is three
      words in the shape adjective + noun + 'of' + place … The merged four-word
      design is superseded"* — the same commit `findings/correctness.md` cites
      as the reason the screen copy dropped its number.
      So the piece reached the right answer for the screen and the wrong one for
      the document a designer actually reads. A designer laying out this row is
      told the name is four words, with a probability figure justifying the
      fourth; the shipped screen asserts no count precisely because that is not
      settled. The two halves of the same change disagree.
      **Severity: medium.** Not a code defect — the screen is right. It is the
      exact failure the live-brief rule exists to prevent, and the numbers at
      `:196-198` are the dangerous part, because a probability quoted to two
      decimal places reads as computed rather than as inherited from a
      superseded shape. Note the probabilities are not merely stale prose: they
      are arithmetic over a vocabulary size, so correcting the count without
      recomputing them would leave a worse artefact than either.

- [ ] **`dev-writer`** — `Core.qml:38-45` — the seam's own comment promises
      exactly the property `ok:true` does not have, and the three callers that
      must not believe it are outside the file
      **Scenario:** `call()`'s contract comment reads: *"A caller that has to
      distinguish 'failed' from 'succeeded with nothing' is the caller that
      eventually renders a broken store as an empty feed."* Read at the seam,
      that says the normalisation has removed the need to distinguish. Three
      core methods return a **wire success carrying a refusal** —
      `{"kept":false,"reason":…}` (`wire.rs:709`),
      `{"hasIdentity":false,"reason":…}` (`wire.rs:985`),
      `{"canPost":false,"reason":…}` (`wire.rs:178`) — so for those three,
      `ok:true` means "the module answered", not "the thing happened", and a
      caller that stops at `ok` is the caller the comment warns about.
      The knowledge that this is so is recorded — but **all of it lives at the
      call sites and none of it at the branch**. `Core.qml:106-111` states it in
      the onboarding *section* comment, below `call()`; `OnboardingScreen.qml:12`
      states it; `design.md:19-22` states it. The `return { ok: true, value:
      reply }` at line 76 carries nothing, and that is the line a fourth caller
      reads. The existing evidence that this invites the mistake is in the tree:
      `FeedScreen.qml:78-80` assigns `probe.ok ? probe.value : …` straight into
      `capability` and then gates on `capability.canPost !== true` at line 418 —
      correct, but correct by the render site re-checking rather than by the
      seam having said anything.
      `piece/ui-stoa-list` adds `createStoa`, `joinStoa` and `listStoas` through
      the same unchanged `call()`, so the number of callers who must know this
      unwritten rule is growing.
      **Severity: medium.** No live defect — every current caller checks. This is
      a finding about where the knowledge lives: the trap is documented three
      times at the places that already avoid it and zero times at the place a new
      caller looks. The cheap fix is a sentence at the `ok:true` return naming the
      three refusal-carrying shapes; the structural fix, if `dev-writer` prefers
      it, is to stop calling the field `ok`.

- [ ] **`dev-writer`** — `Main.qml:26`, `:48`, `:77-121` — the launch branch and
      `piece/ui-stoa-list` have each rewritten `Main.qml` around an incompatible
      idea of what `Main` owns, and neither accommodates the other
      **Scenario:** this piece gives `Main` a `stoaAddress` property (line 26),
      an `identityState` machine over `"unknown"/"present"/"absent"/"failed"`
      (line 48), and `askWhoAmI()` guarding on `stoaAddress === ""` (line 78).
      `piece/ui-stoa-list` **deletes `stoaAddress`, `stoaTitle` and
      `stoaGenesis` from `Main` outright**, with a comment stating the removal is
      the point: *"leaving the properties in place would leave a SECOND source
      for the one value these screens exist to supply — a build shipping a
      hardcoded Stoa"*. In their place it puts `chosen`/`previewing` and a
      `screenShown` navigator over `"feed"/"join"/"list"`, and it carries **no
      `identityState` and no `who_am_i` call at all**.
      So the two pieces hold opposite positions on `Main`'s single job. This
      one says `Main` owns the identity decision for a Stoa supplied from
      outside; that one says `Main` owns Stoa navigation and no Stoa comes from
      outside. Merged in either order, one of the two branches is reconstructed
      by hand, and the reconstruction is not mechanical: `askWhoAmI()` needs a
      Stoa address and after `ui-stoa-list` there is no single one — identity is
      per-Stoa (`whoAmI(stoa)`), so the launch branch becomes a per-Stoa question
      asked at a point the navigator does not currently have.
      **Severity: medium**, and it is a sequencing finding rather than a fault in
      this piece's own shape. Flagged to `dev-writer` because the resolution is a
      decision about who owns the branch, not a merge conflict to resolve
      mechanically — and because whichever lands second will otherwise be
      reviewed as if the reconstruction were incidental. The correctness review
      did not see this; it reviewed the file in isolation.

- [ ] **`dev-writer`** — `OnboardingScreen.qml:707-747` — the screen's three
      `MarginNote` entries sit in an `apparatus` property that
      `piece/drop-apparatus` deletes, and the uniqueness obligation is spec'd
      only into that block
      **Scenario:** `piece/drop-apparatus` removes `ApparatusColumn.qml` and
      `MarginNote.qml` and rewrites `ScreenFrame.qml` to a single-column card
      with **no `apparatus` property** — its comment: *"Those notes were
      annotation explaining the design to a reader of the mockup, not
      interface"*. This screen ships three `MarginNote`s in `apparatus: [...]`.
      Two of the three survive the deletion harmlessly, because their text is
      already duplicated in the body: the permanence note is also the body Text
      at line 504, and the mark note is explanatory only.
      The **uniqueness note is not**. It exists nowhere else on the screen, and
      it is the only thing satisfying the spec requirement *"The screen states
      that a name is not unique and not an identifier"* — required, the spec
      says, *"even though no row shows a name"*. `test_the_uniqueness_note_states_
      the_obligation_without_a_word_count` finds it only because the tree walk
      reaches apparatus children (I measured this: the walk reaches all three
      labels and bodies). So dropping the apparatus column deletes a spec'd
      obligation and fails that test — which is the good outcome, since the test
      will catch it. Worth flagging now because the fix is a body placement
      decision this piece is better placed to make than the piece deleting the
      column.
      **Severity: low-medium.** The gate catches it, so this is about who makes
      the placement choice rather than about a silent loss.

## What was clean

**`Core.qml` is the right seam, and it generalises — that is measured rather
than asserted.** `piece/ui-stoa-list` adds three wrappers and `piece/ui-composer`
adds more, both through an **unchanged `call()`**. Four independent pieces
extended the seam additively and none needed to widen it, which is the strongest
available evidence that the boundary is in the right place. The reasons the file
gives for its own shape hold up: the error branch exists once because the wire
contract has one failure shape, and named wrappers make a renamed core method one
edit. My finding above is about what the seam *says*, not where it *is*.

**The launch/keep split is right, and the `NO SPEC` note records the right
thing.** `Main` asks `who_am_i` and routes; the screen owns the keep and emits
`identityKept` carrying nothing; `Main` responds by asking the module again. The
screen making no `who_am_i` call of its own is the correct half of that split —
a screen that asked would be a second party answering a question `Main` already
owns, and the two could disagree. The deliberate extra round trip buys the
property that no screen state is derived from an action having been invoked,
which is `design.md`'s stated reason and is worth the call.

**The recovery flag reaching the screen as a property set by `Main` is sound.**
It arrives on the `who_am_i` reply, which is `Main`'s call, so `Main` is the only
party that has it; passing it down as a property rather than having the screen
re-ask avoids a second call answering a question already answered. Three-valued
throughout (`undefined` = no claim), and `tst_launch_branch.qml` covers both the
present and omitted cases end to end. The one gap is that nothing asserts the
*rendered* consequence of the omitted case — that is a finding in my readability
file, not here.

**The structure generalises to the sibling screens where it matters.** The
one-string state machine is now the third instance of the same idea
(`FeedScreen.readState`, `Main.identityState`, `OnboardingScreen.phase`), and
`piece/ui-stoa-list` independently reached a fourth (`screenShown`) with an
explicit note rejecting `StackView` for the same reason — "a push/pop lifecycle
alongside that is a second source of truth that can disagree with it". Four
pieces converging on one pattern without coordination is the pattern being right,
not a coincidence. The screen also reuses `ScreenFrame`, `Identicon`,
`AddressLabel`, `FlatButton` and `Theme` without restyling or re-implementing
any, and `AddressLabel` remains the only place abbreviation exists.

**The copy's "assert no number at all" is the right shape for this fact, and
the sweep is the right test for it.** The count has moved three times; a pin on
the current value fails on reword rather than on misinformation, which resists
the fix instead of catching the defect. Stating the obligation — names are not
unique, are not identifiers, the address distinguishes — and sweeping eight count
spellings inverts that: it fails on the reintroduction of *any* number. The
obligation genuinely does not depend on the count, because uniqueness is
unavailable rather than merely unbuilt. I would keep this shape. The one thing it
does not do is stop the number reappearing in a *document*, which is how
`UI-BRIEF.md` came to be stale — my first finding.

**One function, one job holds.** `requestSlate()`, `enterFailed()`, `select()`,
`keepSelected()`, `isCandidate()` and `candidateAt()` each do one thing, and none
has an `And` or a vague verb in its name. `enterFailed()` in particular is the
single place `candidates` is cleared and the selection dropped, so "is it called
everywhere?" stays a question with an answer — every failure path routes through
it. `isCandidate()` is a pure predicate with no side effects and no ambient
reads, which is what lets the validation loop run before assignment.

**Complexity is in the data structure rather than the logic.** The
`isCandidate()`-at-the-boundary decision is the repo's own principle applied
correctly: rather than a fourth slightly-different guard at each comparison site,
the shape of `candidates` is constrained once so the sentinel is unaddressable,
and every comparison site inherits it. That is what let the second guard in
`keepSelected()` be deleted rather than tested — a reshape that removed a branch
instead of adding one.

**No new dependency.** Nothing added to the QML imports beyond `QtQuick` and
`QtQuick.Layouts`, both already used. No licence question arises.

**CI would pass.** `qmlformat` parses `OnboardingScreen.qml` at exit 0. The
`Text`/`textFormat` balance gate at `.github/workflows/ci.yml:262-274` counts
**15 `Text {` openings against 15 `textFormat:` assignments** in this file, so it
balances. No file was moved or renamed, so neither of the two gates that derive
expectations from the source layout is measuring a directory that no longer holds
tests; the spec-count gate at `:594-596` counts `tst_*.qml` on disk against what
the runner reports, and both are 6.

## What I could not check

Whether the row leaves room for a name to arrive above the address without the
layout moving — the promise `design.md` and the newly-added `UI-BRIEF.md`
paragraph both make. The suite asserts on properties and the object tree, never
on geometry, so no gate can see it.

I judged the four in-flight pieces from their branch heads, which may move before
they merge; the `Main.qml` and apparatus findings are about the branches as they
stand today.

I did not run the module end to end against real core.
