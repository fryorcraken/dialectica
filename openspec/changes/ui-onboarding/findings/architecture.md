# Findings — architecture

Reviewed: `dialectica-ui/src/qml/Core.qml`, `Main.qml`, `OnboardingScreen.qml`,
`FeedScreen.qml`, `docs/UI-BRIEF.md`, and the four sibling pieces in flight
(`piece/ui-stoa-list`, `piece/ui-composer`, `piece/drop-apparatus`,
`piece/generated-names`). Dimension covered: **architecture** only. Readability
is my other file; correctness and security were another instance's.

Baseline: **99 QML tests across 6 spec files, all passing**.

## Defects

- [x] **`dev-writer`** — `docs/UI-BRIEF.md:17`, `:139`, `:196-198` — this change
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
      **Deferred — box left OPEN**, to `docs/name-shape-sweep` (PR #64), which
      already contains the fix. Partially mitigated here; the argument follows.
      Your closing sentence is why I did not correct the count. I read
      `d3e7579` and it **already rewrites `UI-BRIEF.md` in the same commit** —
      provenance block, the register sentence, the truncation rule relaxed by
      one word since "of" reads no hash bytes, and the collision figures
      recomputed by hand: **0.006% at a thousand and 0.15% at five thousand**
      against the 0.003% / 0.07% now on disk, from S = 2^33 over 8,192
      adjectives × 1,024 nouns × 1,024 places. Writing my own replacements would
      mean either duplicating that or deriving probabilities a second time from
      vocabulary sizes I would have to read off the same commit — and then
      conflicting with a PR awaiting the owner.
      **Checked before deciding**, because the live-brief rule turns on whether
      *this change* made the file wrong: `git diff main...HEAD -- docs/UI-BRIEF.md`
      shows this piece added exactly one paragraph, the count-neutral "core
      serves no name" bullet. All three four-word sites predate it. So the rule
      is satisfied by the sweep rather than by me, and duplicating it is the
      more expensive mistake.
      **What I did fix** is the half that is mine: my paragraph tells a designer
      to leave room for a name, and it sat adjacent to a four-word claim sizing
      that room. `a2e508d` adds a pointer beside it — do not size the space from
      the count stated elsewhere in this file, naming the three stale sites and
      `d3e7579` as what supersedes them, and directing the reader to PLAN.md
      §5.2.1, which wins any disagreement. It self-invalidates: once the sweep
      lands, the pointer is visibly about sites that no longer say four.
      **The box stays open** because the three sites are still wrong on disk,
      and it should close when #64 merges rather than on my say-so.
      **Fixed — but not the fix you proposed, because the premise moved when I
      checked it.** Your deferral reasoning holds and I am not duplicating #64.
      Two of your facts I re-ran rather than read: `git diff origin/main...HEAD
      -- docs/UI-BRIEF.md` returns exactly the one paragraph block, so all three
      four-word sites do predate this piece; and `d3e7579` is a commit object in
      this worktree's store but `git branch -a --contains d3e7579` returns
      nothing, so it is reachable from no branch here — #64 is still open.
      **What that makes wrong is the mitigation, not the three sites.** On this
      branch `PLAN.md:947` reads *"the shape is settled below and is **four
      words**"*, `:1316` heads "why the name is four words", `:1376` concludes
      *"Therefore the name is four words"*, and `:1682` is a rendering
      obligation — *"**A four-word name needs room**"*. So the brief and PLAN.md
      **agree** at four here; there is no disagreement for PLAN.md to win. Your
      pointer's closing sentence — *"take the shape from PLAN.md §5.2.1, which
      wins any disagreement with this file"* — therefore routes a designer off a
      four-word claim and onto a stronger one, citing as authority a section
      that says the thing the pointer just warned them off. It also asserts a
      supersession that has not landed, on a hash a reader cannot resolve.
      `spec-test.md:398-401` records `PLAN.md:947` and `:1682` as stale
      *"already recorded as deferred… in `UI-BRIEF.md:195-202`"*, and
      `design-review.md:295-303` endorsed the pointer as self-invalidating.
      Both passed over the same thing: the pointer names PLAN.md as the escape
      hatch and PLAN.md is one of the sites it is escaping.
      **The rewrite** drops both defects and keeps your intent. It tells the
      designer not to size the space from a word count *at all* — naming this
      file and PLAN.md §5.2.1 together, and saying plainly that the two agree
      and are simply a count ahead of a decision, so neither is a refuge. It
      cites `git log docs/PLAN.md` instead of a hash, per CLAUDE.md's rule that
      a document should name the command rather than the number. And it replaces
      the warning with a constraint that survives the count moving: size the row
      from the longest name it can show without the address moving. That is the
      same argument the screen makes by asserting no number, which is why it is
      safe to design against before the sweep lands.
      **The three sites and the collision figures are untouched**, for your
      reason: the figures are hand-derived arithmetic over vocabulary sizes and
      re-deriving them here would either duplicate #64 or conflict with it.
      **Checked for orphaned citations** before rewriting, per the repo rule.
      Five files cite `d3e7579`. `proposal.md:94-98` and `design.md:334-341`
      cite the count's *history of moving* as the argument for asserting none —
      true whether or not #64 lands, and neither instructs a designer, so both
      stand. The two ticked boxes near those lines were discharged by edits to
      `proposal.md` and `design.md`, not by the pointer, so nothing is stranded.
      `design-review.md:295-303` is prose describing the pointer's old wording;
      it is now a description of a superseded version, and I have left another
      reviewer's text alone rather than editing it — flagging it here instead.
      **No test.** A brief is prose and no gate reads it; the assertable half —
      that the screen states no count — is already pinned by
      `test_the_uniqueness_note_states_the_obligation_without_a_word_count` and
      its eight-spelling sweep, and that test is unchanged and still passes.

- [x] **`dev-writer`** — `Core.qml:38-45` — the seam's own comment promises
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
      **Fixed** in `a2e508d`, taking the cheap fix. The note sits at the
      `ok: true` return, opens with "**`ok: true` means THE MODULE ANSWERED. It
      does not mean the thing you asked for happened**", lists all three shapes
      you cite with their wire lines, and names the consequence for each — an
      identity reported that was never stored, a composer opened for a user who
      cannot post, and silently, because nothing failed. `call()`'s contract
      comment got the matching half, since it was the half promising the
      property the seam does not have.
      It also says why this is **not** a defect in the normalisation, which the
      note needs or a reader arrives at the wrong fix: `{"error":…}` is the
      wire's one failure shape and `ok:false` reports exactly that; a refusal is
      a different thing from a failure and the contract is right to keep them
      apart. What the caller owes is the second branch.
      **I considered and rejected the structural fix** of renaming `ok`. It
      would be a breaking edit to every existing call site across four in-flight
      pieces to fix a problem that is one comment wide, and your own "what was
      clean" section is the argument against it: four pieces extended this seam
      additively without needing to widen it, which is evidence the shape is
      right. `ok` is also accurate for what it reports — the call reached the
      module and the module answered. The gap was that nothing said so where it
      mattered.
      **No test.** A comment is not assertable, and I would rather say that than
      tick a box implying otherwise. What is assertable — that this screen
      distinguishes the three keep outcomes — is already pinned by
      `test_a_refusal_is_neither_the_kept_state_nor_the_failed_state`, which
      fails 4 tests under the `kept !== true` mutation. This finding is about the
      *next* caller, and nothing in this piece's suite can reach them.

- [x] **`dev-writer`** — `Main.qml:26`, `:48`, `:77-121` — the launch branch and
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
      **Deferred — box left OPEN**, to the coordinator, who has taken the
      sequencing and has independently verified the collision (that branch's
      `Main.qml` carries neither `stoaAddress` nor `identityState`). I have not
      touched `piece/ui-stoa-list` and have changed nothing in this piece's
      `Main.qml` in response, deliberately: a unilateral accommodation of a
      branch that may itself move is a guess at a decision that is not mine, and
      it would make this piece's `Main` answer to a navigator that does not
      exist here.
      Recording what the reconstruction needs, since I am the one who knows this
      half and that knowledge should not leave with me:
      **The launch branch is per-Stoa, not per-app.** `whoAmI(stoa)` takes a
      Stoa, and the spec's requirement that the branch be re-asked rather than
      remembered means the question is asked at each point a Stoa becomes
      current — not once at startup. So after `ui-stoa-list`, `askWhoAmI()`
      belongs where a Stoa is chosen, and `identityState` becomes a property of
      the chosen-Stoa screen rather than of `Main`. The `"unknown"` state exists
      precisely for "not asked yet", so a navigator that has not chosen a Stoa
      is already representable.
      **What must not be lost in the reconstruction**, whichever lands second:
      the module answers on every ask and no flag is remembered
      (`test_the_second_answer_decides_the_branch_and_the_first_does_not`); both
      absent cases route to onboarding with the reason held unparsed
      (`test_an_unloadable_identity_also_shows_onboarding_with_its_own_reason`);
      and a failed report shows neither branch
      (`test_a_failed_report_shows_neither_branch`). Those three tests are the
      contract, and they will fail loudly if the reconstruction drops one — which
      is the good outcome, and the reason to run
      `dialectica-ui/tests/run-qml-tests.sh` after the merge rather than trusting
      that a conflict-free rebase preserved the behaviour.
      **Deferred — box stays OPEN, and I am not ticking it.** I verified your
      citations still land: `Main.qml:26` is `property string stoaAddress: ""`,
      `:48` is `identityState`, and `:77-121` is `askWhoAmI()` with the
      `stoaAddress === ""` guard at `:78`. The collision is real as described.
      **I changed nothing in `Main.qml`,** and the reason is your own: the
      resolution is a decision about which shape `Main` takes, the coordinator
      has taken that sequencing, and a unilateral accommodation would be a guess
      at a decision that is not mine — the merge is semantic rather than
      textual, so a wrong guess compiles and then fails an identity check on
      every cold start. This box closes when that decision is made, not on my
      say-so, and it correctly blocks the merge until then.
      **What I did do is move the durable half out of the tracker**, which is
      deleted at archive: `design.md` now carries *"The launch branch is
      per-Stoa, and what a reconciliation must not lose"* beside the existing
      three-copies-of-the-guard entry, which already recorded the collision but
      only its guard half. It states that the question is per-Stoa rather than
      per-app and why (`whoAmI(stoa)` takes a Stoa, and re-asking rather than
      remembering means it is asked wherever a Stoa becomes current), that
      `"unknown"` already represents "not asked yet" so a navigator needs no new
      state, and it names your three tests as the contract with the instruction
      to run the suite after the merge rather than trusting a clean rebase.
      **Verified the three tests exist** at `tst_launch_branch.qml:132`, `:155`
      and `:181` before citing them, rather than copying the names across.
      **Fixed** — the question that kept this box open has been answered by the
      owner, so the half that was undecidable is now recorded rather than
      deferred. Your citations still land: `Main.qml:26` is
      `property string stoaAddress: ""`, `:48` is `identityState`, and `:77-121`
      is `askWhoAmI()` with the `stoaAddress === ""` guard at `:78`.
      **The answer is that identity is per-peer for the MVP.** `whoAmI()` is
      asked once at launch and takes no Stoa, so this piece's launch-branch
      shape — one ask, at startup, for the whole session — is right, and the
      reconstruction note that said otherwise was wrong. The previous pass
      reasoned the scope off the method signature (*"`whoAmI(stoa)` takes a
      Stoa, so the question is per-Stoa"*), and the signature is ahead of the
      product: with one identity per user across every Stoa, the parameter
      cannot change the reply.
      **Recorded as an MVP waypoint, not as a settled design**, which is the
      part that matters. `design.md`'s section — now *"The launch branch asks
      once, because the MVP has one identity per peer"* — cites merged
      `docs/PLAN.md` §5.2 rather than re-deriving anything: the section's own
      title is *"Scope: one identity per Stoa, permanent"*, its subsection
      *"The MVP ships ONE identity per user, and this section is the
      destination"* states *"the MVP is a waypoint on the way there, not a
      change of mind"*, and §9.2 lists per-Stoa identity as out of the MVP. It
      also carries §5.2's reversal path — `derive_stoa_key(root, stoa_address)`
      is built and tested in `dialectica-core`'s `identity.rs`, one identity per
      user means *not calling it*, and switching it back on needs no
      wire-format change, no address change and no new primitive — and §5.2's
      *"the deferred work is the flows, not the crypto"*: a create-or-select
      identity step when creating or joining a Stoa, plus a keystore holding
      more than one identity. So a reader arriving at "identity is per-peer"
      meets the destination and the reversal path in the same paragraph.
      **On the sibling collision, which is the rest of your finding: nothing in
      `Main.qml` changed**, and the coordinator still owns which shape `Main`
      takes. What the answer removes is the *sequencing hazard you named* — you
      wrote that after `ui-stoa-list` the launch branch *"becomes a per-Stoa
      question asked at a point the navigator does not currently have"*. Under
      per-peer identity there is no such point to find: the ask stays at launch
      and a navigator needs no new state, because `"unknown"` already means "not
      asked yet". That is recorded as a consequence in `design.md` precisely so
      the reconciliation is not re-derived from the signature a second time.
      Your three contract tests are unchanged and still named there, with the
      instruction to run the suite after the merge rather than trusting a clean
      rebase.
      **On `whoAmI`'s signature — judged, and deliberately NOT narrowed here.**
      Under per-peer identity the `stoa` parameter cannot change the answer, so
      it is wider than the product needs. It is not a parameter deletion: core's
      `whoami_for` (`wire.rs`) reads `store.path_for(stoa)` from a per-Stoa
      `IdentityStore` and derives the reported key with
      `stoa_public_key_at_path(stoa, path)`, and its `NO_CHOICE_FOR_THIS_STOA`
      state is shared with `getCapabilities` *"so the two methods cannot
      describe one situation in two ways"*. Narrowing it means deciding what
      replaces the two-store split and touching a sibling method — a core API
      change, which CLAUDE.md says is *"a decision to make on purpose rather
      than a side effect"* of a view change. It is a separate piece, the
      reasoning is in `design.md` under *"`whoAmI` still takes a Stoa, and
      narrowing it is a separate piece"*, and the open question it leaves is
      recorded there rather than left with me.
      **`tst_launch_branch.qml`'s comment at the Stoa-carrying test was the one
      code-adjacent site repeating the superseded claim**, so it now says why
      the request must carry the Stoa under *both* scopes: today because core
      reads the value, and after the flows land because it selects the identity.
      The assertion is untouched and still passes — only its justification
      moved, since the old one would have read as a scope guarantee.
      **Checked for orphaned citations before rewriting**, per the repo rule.
      `spec-test.md:144` uses "identity is per-Stoa" to argue the severity of a
      wrong-Stoa request. That argument survives the waypoint — core still reads
      the store entry the parameter names — so nothing is stranded, and it is
      another reviewer's text, which I have left alone and am flagging here
      instead. `design-review.md:276` describes the old `design.md` wording and
      is now a description of a superseded version; also left alone.
      **`docs/UI-BRIEF.md` needed no change**, checked rather than assumed: §2
      (`:97-119`) already ships the waypoint framing — one identity per person
      in the first release, unlinkability *"suspended, not abandoned"*, and the
      create-or-select step named as what arrives with per-Stoa identity, with a
      warning that a join flow assuming one possible identity forever will need
      reopening. It agrees with PLAN.md §5.2 and with this change.
      **No new test.** What changed is prose and one comment; the behaviour this
      box turns on is already pinned by the three contract tests you named and by
      `test_the_identity_report_request_carries_the_stoa_it_asks_about`.
      `dialectica-ui/tests/run-qml-tests.sh` runs green across all six spec
      files.

- [x] **`dev-writer`** — `OnboardingScreen.qml:707-747` — the screen's three
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
      **Fixed** in `a2e508d`, and I agree this piece is the one to make the
      placement call — the other piece would be choosing where a requirement it
      has not read should live.
      The uniqueness statement is now a body `Text` gated on the same phases as
      the permanence warning, which was already duplicated out of the margin for
      exactly this reason; the asymmetry between the two was the actual defect
      and it is now gone. The margin note stays: same text, in the place a
      reader of the mockup expects it, and duplication is cheap because neither
      copy is computed.
      `test_the_uniqueness_obligation_survives_without_the_apparatus_column`
      requires **two** elements to carry the statement and reports 1 when the
      body copy is removed. Worth noting it is not redundant with the existing
      count test: under that same mutation
      `test_the_uniqueness_note_states_the_obligation_without_a_word_count`
      still passes, because the margin note satisfies it — which is precisely
      the gap you identified, measured.
      `design.md` records the general rule rather than only this instance:
      **apparatus may repeat an obligation, never carry it alone**, because the
      column is removable by a change that has no reason to read this spec.
      Your two survivors check out — the permanence note is duplicated at what
      is now line 504, and the mark note is explanatory only, so neither needs
      moving.

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
