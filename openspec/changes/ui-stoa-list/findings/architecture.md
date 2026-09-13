# Architecture findings — `ui-stoa-list`

Reviewed dimension: **architecture**. (A sibling file in this commit covers
readability; correctness and security were reviewed before me and I read both
before starting so as not to re-report them.)

Method: baseline suite green at **100 tests across 5 spec files** before any
probe and after. Every "measured" claim below was produced by driving
`Main.qml` and the shipped screens under `qmltestrunner` in a scratch
`TestCase` in my own worktree, deleted afterwards; the tree is clean.

Judgements I was asked for are stated up front, then the boxes.

**On `StoaReference` owning both halves — the pairing is right.** A share whose
output the paste field cannot read is a string whose recipient can do nothing
with it, and the failure surfaces on a third party's machine. Putting
`shareText` and `parse` in one file makes them one decision with one reviewer,
and the round-trip test composes them (`parse(shareText(...))`) rather than
pinning a literal, so the pairing is enforced and not merely asserted. It is
also correctly *not* a screen: both screens and the list rows use it, and a
singleton is the right shape for a thing with no state. No box.

**On `ClipboardSink` as a component — the seam is right, for the stated
reason.** The justification that survives scrutiny is not the module-availability
one (that only explains the `TextEdit`); it is that **what gets copied differs
from what is displayed** at two of the three receivers, so the "what actually
gets copied" decision needs one home. `lastCopied` is honest about being the
testable half. No box.

**On `Main.qml` without a `StackView` — correct, and for the right reason,**
but the file has outgrown the argument in a way that is now a defect; see the
first box.

---

- [ ] **`dev-writer`** — `Main.qml:31,83-85` — once a Stoa is chosen the user
      cannot get back to the list, because nothing ever clears `chosen`
      **Scenario:** `chosen` is assigned by `onStoaChosen` (line 84) and by
      nothing else; `previewing` has `onCancelled` to clear it (line 112) and
      `chosen` has no counterpart. `FeedScreen` declares **no signals at all** —
      measured by enumerating its members: every `back`/`close`/`leave`/`exit`/
      `dismiss`-prefixed name returns the empty set. So `screenShown` (41-44)
      evaluates to `"feed"` permanently from the first row the user opens.
      Measured end-to-end on `Main.qml`: start on `"list"`, set `chosen`,
      `screenShown` is `"feed"`, and there is no property, signal or affordance
      in the shipped tree that returns it to `"list"`. A user who opens any Stoa
      is stranded there until the app restarts — including the user who opened
      the wrong row.
      **Severity:** high, and in my judgement the most serious thing in this
      dimension. The change's stated purpose is that "the list becomes the view's
      entry point and the feed is reached from it" (proposal.md:34); reachable
      once, with no way back, is not navigation. It also makes the share and
      join affordances unreachable after the first open, which is the whole of
      what this piece added.
      **Measured:** 100 of 100 tests pass with this holding. The suite asserts
      `no feed before a Stoa is chosen` and `the feed is given the address and
      the record`, and nothing asserts anything after that transition — the
      gap is structural, not an oversight in one test.
      **Not an argument for a `StackView`.** D5's reasoning against one is
      sound and I would keep it; the fix is a `cancelled`/`closed` signal on
      `FeedScreen` setting `chosen = null`, which is the symmetric counterpart
      of the `onCancelled` that already exists for `previewing`.

- [ ] **`spec-writer`** — `specs/stoa-navigation-view/spec.md:538` — the
      requirement that puts the feed behind the list does not require a route
      back, so the strandedness above is spec-compliant
      **Scenario:** "The view holds no Stoa of its own, and the feed is reached
      from the list" requires the address to travel from the row to the feed and
      forbids a defaulted property. Its three scenarios cover arriving at the
      feed and never leaving it. So the implementation satisfies every scenario
      while producing a one-way trip, and the reviewer above has no requirement
      to cite. A screen the user cannot leave is a rendering obligation of the
      same family as the ones this capability already owns — the core cannot
      discharge it, and only the view can. Worth a scenario ("a user who has
      opened a Stoa can return to the list without restarting"), so the next
      view change inherits it rather than rediscovering it.
      **Severity:** medium. Addressed to `spec-writer` rather than `dev-writer`
      deliberately: the code fix is the box above, and this box is the reason
      the gate did not catch it.

- [ ] **`dev-writer`** — `Main.qml:41-44` — `screenShown` silently discards a
      preview requested while a feed is open, and the precedence is undocumented
      **Scenario:** `screenShown` is an ordered ternary testing `chosen` first,
      so `previewing` is only consulted when `chosen` is null. Measured: with
      `chosen` set, assigning `previewing = {stoa, genesis}` leaves
      `screenShown` at `"feed"` — the `JoinScreen` is rebound to the new
      reference behind an invisible panel and nothing is shown. Today this is
      unreachable *because* nothing on the feed emits a preview request, so it
      is latent rather than live; but the spec's own model is that an address
      inside a post is an affordance a reader acts on
      (spec.md:206-215), and a post lives on the feed. The piece that adds that
      affordance will set `previewing` from the feed and get silence, with no
      comment in this file to warn it.
      **Severity:** medium (latent). This is CLAUDE.md's "complexity in the data
      structure, not the logic": two independent nullable properties admit the
      state `(chosen ≠ null, previewing ≠ null)`, which has no rendering, and a
      ternary resolves it by accident of ordering. One property naming the
      screen — or clearing `chosen` when a preview is requested — makes the
      impossible state unrepresentable instead of merely unreachable.

- [ ] **`spec-writer`** — `StoaReference.qml:79-86` — the reference encoding is a
      compatibility surface decided by a `NO SPEC`, and it should be in the spec
      **Scenario:** `{"stoa":"<hex>","genesis":"<hex>"}`, bare hex, no `stoa:`
      prefix, is what `shareText` emits. A user who has copied a reference holds
      a string in that shape, and the recipient may paste it days later, so
      changing the encoding strands strings already in the wild — which is a
      property no other `NO SPEC` in this change has. The spec requires both
      halves (spec.md:161-204) and names no format, and `design.md:275-278`
      already concedes *"It is worth being in the spec."* I agree, and the
      reason is sharper than compatibility alone: the format is the **only**
      thing making the share and the paste one decision across a version
      boundary — within one build `StoaReference` enforces it, but between two
      builds nothing does, and a spec requirement is what a future change reads
      before touching it.
      **Severity:** medium. The encoding choice itself is sound — JSON is
      self-describing, so a truncated paste fails as *malformed* rather than as
      a verification mismatch, which is the distinction the whole join screen
      rests on. The box is about where the decision is recorded, not what it is.
      The other three `NO SPEC` choices (empty paste, creation without a `stoa`
      field, non-string half) have no such cross-version cost and I would leave
      them as design decisions.

- [ ] **`spec-writer`** — `StoaListScreen.qml:45,146-150` — the conditional
      share requirement is honest, but "a Stoa you just created cannot be
      shared" is a degradation the spec does not name
      **Scenario:** I checked the core claim rather than taking it: `wire.rs:1587`
      documents `create_stoa` as `{"title":…} -> {"stoa",…,"foundingTitle",…,
      "policy"}` and `wire.rs:2024` documents `list_stoas` items as
      `{"stoa","foundingTitle"}`. Neither carries the genesis record, so the
      design's framing constraint is accurate and the conditional requirement is
      **not** a way of dodging anything — the view genuinely cannot produce a
      joinable string for those rows, and inventing one would fail verification
      on the recipient's machine. The spec is right to make the affordance's
      absence the correct rendering.
      What it does not say is the case a user will actually hit first: **create a
      Stoa, and it is immediately unshareable.** The spec's scenario is "no share
      is offered for a Stoa whose record the view does not hold" (spec.md:200),
      which is true but reads as an edge case; the shipped behaviour is that the
      *primary* creation flow ends with nothing to hand anybody. `design.md` does
      not list this among its risks either — it lists the restart case
      (`design.md:190-192`) but not the create case, which is worse because it
      needs no restart. Name it, so the core piece that widens the listing item
      has a requirement pointing at it.
      **Severity:** medium. The code already says this plainly at 146-150 and is
      not at fault; the gap is that the spec and the risk list do not.
      The degradation *is* legible in the view — the button is simply absent and
      the apparatus note explains the absence — so the rendering half is right.

- [ ] **`dev-writer`** — `StoaListScreen.qml:91` — `rows` keeps the previous
      page's items after a failed reload, so the screen's state is safe only
      because two places agree
      **Scenario:** `reload()`'s failure path deliberately does not write `rows`
      (the comment at 71-72 says so, and the reasoning is sound — a failure must
      not blank a good listing underneath a banner). The rows are then hidden by
      the `Repeater` model being gated on `readState === "ok"` (line 304). So
      "stale rows are not rendered" holds because of a guard 213 lines from the
      state that makes it necessary, and `screen.rows` is left holding a listing
      that `readState` says was not read. The correctness reviewer noted this
      as invisible to the user and flagged it for me rather than opening a box;
      I am opening one because it is a live trap for a second reader of `rows`,
      and `Main.qml:95` is already one — `heldStoas: list.readState === "ok" ?
      list.rows : []` re-implements the same guard at the second call site.
      That is CLAUDE.md's signal exactly: the same guard written twice, in two
      files, is the point at which the data shape should absorb it.
      **Severity:** low-medium. No current defect — both call sites guard
      correctly, measured. The fix is to make `rows` inaccessible when
      `readState !== "ok"` (a derived `visibleRows`, or clearing on failure with
      the previous page held separately), so a third caller inherits the
      invariant instead of having to remember the guard.

---

## The `Main.qml` collision with `piece/ui-onboarding`

I read `piece/ui-onboarding`'s `Main.qml` to characterise the conflict and
changed nothing on that branch.

**The two pieces do not merely disagree about what `Main` owns — they are
mechanically incompatible, and a textual merge will not reveal it.** This branch
*deletes* `stoaAddress`/`stoaTitle`/`stoaGenesis` from `Main.qml` and makes the
Stoa arrive from the membership listing. The onboarding branch *keeps* those
properties and builds `askWhoAmI()` on top of `stoaAddress`, whose first
statement is:

```
if (root.stoaAddress === "") {
    root.identityState = "failed"
    root.identityFailure = "No Stoa address was given to this view."
    return
}
```

with `Component.onCompleted: root.askWhoAmI()`. So onboarding asks the core
"do I have an identity **in this Stoa**" at launch, from a property this branch
removes because nothing may supply one at launch. Merge them naively and
onboarding either fails to compile (the property is gone) or, if someone
"fixes" it by reading `chosen`, reports `identityState = "failed"` on every cold
start — because at launch `chosen` is null by this branch's central design
decision, which is the spec requirement at spec.md:538 ("No Stoa is rendered
before one has been chosen").

**My judgement: this branch's position is the right one, and onboarding's
launch-time `whoAmI(stoaAddress)` is the half that has to move.** Three reasons,
in order of weight:

1. **The spec forbids what onboarding depends on.** `stoa-navigation-view`
   requires the top-level view not to carry a Stoa address as a property with a
   default, and states the reason: a second source for that value is a build
   that ships a hardcoded Stoa. Onboarding's gate is that second source.
2. **The sequencing is wrong on its own terms, independent of this branch.**
   Identity in this release is one key across every Stoa — onboarding's own
   `Main.qml` comment and this branch's `JoinScreen.qml:485-490` both say so.
   Asking "do I have an identity in Stoa X" before the user has any Stoa asks a
   per-Stoa question of a global fact, and the honest answer at launch is not
   available because there is no X.
3. **Only one of the two can hold the invariant it needs.** This branch's
   invariant (`Main` holds no Stoa of its own) is enforced by the absence of a
   place to put one; onboarding's (`stoaAddress` is non-empty at
   `Component.onCompleted`) is enforced by a developer filling a property in by
   hand, which is the thing being removed.

**What I am not doing:** deciding onboarding's redesign, which is that branch's
reviewers' call — the plausible shapes are asking `whoAmI` without a Stoa, or
moving the identity branch behind the list so it runs when a Stoa is first
opened. I am recording that **the two branches hold opposite positions on what
`Main` owns, that the conflict is semantic rather than textual, and that a merge
which resolves it line-by-line will produce a view that reports a failed
identity check on every launch.** Whoever merges second needs to know that
before they open the conflict markers, not after.

## What was clean

`Core.qml` is the right seam and stayed narrow: three wrappers added, each
naming its method once, with `call()` still the single normaliser. Declining to
default `perPage` inside `listStoas` is the correct call for the stated reason —
a default buried in a wrapper is a number two screens share without either
choosing it.

The `lookalikes` / `join()` separation survives inspection as a structural
property rather than a convention: `lookalikes` reads only titles and
`heldStoas`, `join()` reads neither, and the two cannot reach each other's
input. The spec distinguishes comparing *titles* (required) from comparing
*addresses* (forbidden) and the code keeps them in different functions rather
than behind a flag in one.

The guard-supplies-its-own-input question I was told to ask of every defence
here now answers correctly: `lookalikes` depends on the **derived**
`foundingTitle`, so it can only compare against a title the core returned for
the reference on screen. The earlier shape — where a carried-over title made
both sides of the equality the same string and silently suppressed the panel —
is unrepresentable, not merely fixed, because there is no variable left that can
hold another Stoa's title. That is the right resolution of the sharpest instance
in this change.

`ClipboardSink` being owned by `Main` and injected into both screens as a
property (rather than each screen making its own) means there is one clipboard
and one place the copy decision lives; the three `copyRequested` receivers each
state what they copy and why it differs from what is shown.

## What I could not check

- **Anything visual**, including whether the absent share button reads as
  absence rather than as a broken row — which is the degradation this design
  leans on being legible.
- **The core's side.** Every reply in my probes was a fake. I verified the
  `create_stoa` and `list_stoas` reply *shapes* against `wire.rs:1587` and
  `wire.rs:2024` by reading them, not by running the core.
- **`cargo mutants`** does not apply: the implementation diff is QML only. The
  Rust in the branch diff (`transport.rs`, ~2700 lines) arrived with the merge
  of `main` at `7993d79`, not with this piece.
