# Architecture — `ui-composer`

Reviewed on `piece/ui-composer` at `7dccbdc`, in a worktree of its own. Suite
green at **118 tests across 8 spec files** before and after every mutation; all
mutations reverted and `git status --porcelain` is empty. `qmllint` (CI's own
invocation, `ci.yml:535-569`) exits 0 on the five changed QML files.

Correctness, security and readability findings were read first; nothing below
repeats one. The one overlap with readability is deliberate and split by
dimension: that file's first box is about the component's *legibility* as a
closed set, this file's first box is about **where the invariant lives**.

## Findings

- [ ] **`dev-writer`** — `FeedScreen.qml:183-195` — `capability` holds two
      differently-shaped objects depending on which branch assigned it, so every
      reader of it has to know which one it got
      **Scenario:** the open arm assigns `probe.value` **wholesale** — whatever
      object core sent, with whatever fields — and the closed arm constructs a
      view-owned `{canPost: false, reason: <string>}`. One property, two shapes,
      and the shape is decided by a condition the reader of the property cannot
      see. The consequence is already in the file: the closed gate's reason
      `Text` needs a `!== undefined` guard (line 663) that is **dead against the
      closed arm** — which always supplies a string — and live only because the
      open arm can leave `reason` absent. **Measured:** replacing that guard with
      a bare `text: screen.capability.reason` and running `tst_vote_and_gate.qml`
      emits `FeedScreen.qml:663: Unable to assign [undefined] to QString` on
      fourteen tests, every one of them an **open**-gate case driving the
      `'{"canPost":true,"identity":"aa"}'` fixture — the closed-gate tests emit
      nothing. 29 of 29 still pass.
      This is the shape CLAUDE.md names: a guard restated per consumer because
      the data structure does not hold the invariant. Normalising both arms into
      one shape — `{canPost: <bool>, reason: <string>}`, constructed from
      `probe` in one place rather than passed through on one path — makes the
      guard at 663 unnecessary rather than merely explained, and means a second
      consumer of `capability.reason` inherits the invariant instead of
      rediscovering it. It is the same "establish it where the value is produced"
      move that `voteTarget()` already made one screen over, applied to the
      probe reply instead of the row.
      **Severity: medium.** Nothing renders wrongly today (the guard holds, and
      the closed body's binding is only ever *evaluated*, not shown, on the open
      path), but the seam currently holds two positions about one reply — the
      thing the spec's row-shape requirement calls out by name at spec.md:614-619
      — and it is the position the piece was already corrected on once.

- [ ] **`dev-writer`** — `PublishOutcome.qml:35` and `55-59` — the "one value,
      not three booleans" invariant is enforced in `Composer` and **not** in the
      component that renders it, so the totality is a property of the caller
      rather than of the type
      **Scenario:** `design.md:44-72` argues the one-string shape makes two
      outcomes on screen at once impossible "because one variable cannot hold two
      values". That argument is about `Composer.outcome`, and it holds there:
      `applyReply` is total, every exit sets one of four strings, and the
      correctness reviewer could not break it. But `PublishOutcome` is a
      separately registered QML type (`qmldir`) with `outcome` as a public
      writable property, and it partitions the value space **twice, differently**
      — three elements ask "is it one of the two successes?", two ask "is it the
      one refusal?". An unrecognised string satisfies neither partition's
      positive case, so it renders the refusal headline *and* both success
      sentences together, with core's `detail` suppressed. Measured directly
      against the component for `"deferred"`, `"Refused"`, `"REFUSED"`,
      `"refused "` and `"stored "` — identical three-line contradiction each time
      (the rendering is quoted in `findings/readability.md`).
      The architectural point is not the rendering, it is that **the invariant
      the design rests on is enforced one component away from the component that
      depends on it.** A second caller of `PublishOutcome` — the thread screen
      is the obvious one, since the reply composer lands there — inherits the
      rendering and not the guarantee. Putting the partition in one place (an
      `isSuccess` readonly beside `isRefusal`, both derived from one
      classification, with the headline keyed on the same classification) makes
      the type total on its own terms.
      **Measured that nothing notices:** I added a fourth outcome to
      `Composer.applyReply` and the whole suite passed, 118 of 118. **Severity:
      medium.**

- [ ] **`design-reviewer`** — `FeedScreen.qml:743-776` — this piece adds two
      `MarginNote`s to the column `piece/drop-apparatus` (#70) deletes, and one
      of them is the second copy of a sentence the tests exclude from a sweep
      **Scenario:** the piece correctly moved both *load-bearing* obligations out
      of the column and says so at 733-742. It then adds two new notes to it:
      `ON PUBLISHING` (749) and `ON THE ARROWS` (756). Neither is required —
      spec.md:544-550 is explicit that the score's absence "does not oblige the
      view to carry prose about why no number is there" — so deleting them loses
      no requirement, which is the right outcome. The thing to settle before the
      merge is that `ON PUBLISHING` is a **second delivery denial**, listed in
      `tst_composer_claims.qml:211-213`'s `otherKnownDenials()` so the sweep does
      not flag it, and that entry is excluded-but-not-pinned — the file's own
      comment calls it "the weaker of the two arrangements" and accepts it only
      because the sentence is leaving the tree.
      **Measured:** `git diff main...piece/drop-apparatus -- FeedScreen.qml`
      deletes an `apparatus:` block of **three** notes; this branch's block has
      **five**. The merge conflicts, and whoever resolves it decides whether two
      sentences survive that nothing requires and one test quietly accommodates.
      **Severity: low**, and it is a coordination item rather than a defect —
      the right resolution is to drop both notes with the column and delete the
      `otherKnownDenials()` entry in the same change, so a dead exclusion does
      not outlive the string it excludes.

## Judgement on the questions asked

**Is `Core.qml` the right seam for three more wrappers? Yes, and the trap is not
re-introduced.** The sibling's warning (`a2e508d`, currently on
`piece/ui-onboarding`) is about three methods that answer a refusal as a wire
*success* — `keep_identity`/`kept:false`, `who_am_i`/`hasIdentity:false`,
`get_capabilities`/`canPost:false` — where `ok:true` means only that the module
answered. I checked the three publish methods against core rather than assuming:
`wire.rs:1563-1564` emits `{"opId", "wasNew"}` and `wire.rs:5709`, `6780` and
`6870` assert a refusal carries **no** `opId` and no `wasNew`, so a publish
refusal is always `{"error":…}` and always arrives as `ok:false`. There is no
"success that is a no" on these three paths, so the trap does not apply, and the
wrappers correctly interpret nothing — `Core.qml:98-105` says why in as many
words. The two branches do not textually conflict either: the sibling's note
lands inside `call()`, these wrappers append after `getCapabilities`.

The seam is used correctly downstream too, which is the half that matters:
`applyReply` guards on `typeof reply.value.opId === "string"` rather than on the
absence of an error, and `voteOn` applies the identical test at
`FeedScreen.qml:157`. Both treat `ok:true` as "the module answered" without
being told to, which is the behaviour the sibling's warning exists to produce.

**Does the gate seam have one consistent policy? Not quite — see the first box.**
The specific defect the sibling review found is fixed and fixed well:
`voteTarget()` establishes the row's op in one place, both consumers go through
it, and the design note at `design.md:196-235` rewrites the false "by
construction" claim and names the general shape of the error rather than only
the instance. What is left is the same inconsistency moved up one level: the
view now validates a row's element shape and still takes the *probe reply's*
shape on trust when the answer is yes. It is a smaller instance of the same
thing, not a new one.

**`voteTarget()` / `voteOn` belt-and-braces: right, and right in the safe
direction.** Three sites express "does this row have a target" —
`voteTarget()`'s `typeof === "string" && !== ""`, `voteOn`'s
`typeof !== "string" || === ""`, and `interactive`'s `target !== ""`. Restating
a guard invites drift, but the drift that matters is a guard becoming *weaker*
than its caller assumes, and here `voteOn`'s is the strictly weaker
precondition: a `voteTarget` that started returning a non-empty non-string would
still be stopped at the call. The comment at 144-146 gives the reason a comment
must — `voteOn` is reachable from anywhere in the file — and the three tests
behind it (`test_a_targetless_vote_call_is_refused_even_if_reached_directly`
covers `""`, `undefined` and `null`) pin the restated guard independently of the
binding. I would keep this as it is.

**Shipping the uninstantiated reply path: right.** The alternative arguments
both lose. Instantiating it in `FeedScreen` would put a reply box under a thread
head, which replies to a thread's current version from a feed row — a
thread-view affordance on a screen that is not one, and the spec anticipated
exactly this at spec.md:32-39, scoping every reply requirement to "when a reply
is submitted or refused" rather than to a screen offering one. Deleting the
reply path and re-adding it with the thread screen would mean the thread screen
arrives carrying both a new screen and an untested publish path, and the
awkwardness of that is the signal CLAUDE.md's "make the change easy, then make
the easy change" is about — this piece made the room, the next change makes the
small change. The decisive fact is that the path is not dead code in the sense
that matters: `kind: "reply"` is driven by nine tests across two files, including
the two-refusals-rendered-identically case and the argument-JSON assertion, so
the code *paths* are exercised even though no screen instantiates the component.
`Composer.kind` decides one thing — which core method is called — and every rule
the spec states about a refusal is the same rule for both modes, which is why
one component for both is the right factoring rather than a speculative one.

**Nothing else here depends on the apparatus column.** Beyond the two notes in
the third box, I grepped the changed view files and the four test files: the
only remaining coupling is `tst_vote_and_gate.qml`'s `isApparatusColumn` walker,
which is used by the *placement* assertion — it exists to prove the missing-box
statement survives the column's removal, so it is the thing that should outlive
the column, not a dependency on it. When the column goes,
`test_the_apparatus_walker_actually_excludes_the_column` fails (it asserts the
column is rendered at all) and that is the correct, loud failure: the walker has
nothing left to exclude and the placement test collapses into a presence test,
which is the state it was written to escape. Worth knowing before `#70` lands,
but it is that change's box, not this one's.

## What I could not check

- **Whether the merge with `piece/drop-apparatus` resolves the way the third box
  recommends.** Both branches touch `FeedScreen.qml`'s `apparatus:` block and
  neither is merged; I can see the conflict is coming and not who resolves it.
- **Anything about rendering or layout.** QtTest drives properties and signals,
  never pixels. The `PublishOutcome` contradiction in the second box is a
  property-level measurement; whether the three contradictory lines would even be
  legible together on screen is not something this repo can see.
- **The `Core.qml` merge order with `piece/ui-onboarding`.** The two diffs do not
  overlap textually, so git will take both, but I did not run the merge.
