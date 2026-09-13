# Spec/test review — `ui-onboarding`

Read: `specs/view-identity-onboarding/spec.md` (14 requirements, 48 scenarios),
`tst_onboarding_states.qml`, `tst_launch_branch.qml`. The implementation was not
read except for the exact lines mutated in part 2 and the two `NO SPEC:` comment
blocks.

Baseline before and after: **102 pass across 6 spec files**, tree restored
(`git status --porcelain` empty).

## Findings

- [x] **`tester`** — `test_no_copy_claims_the_identity_cannot_be_linked_elsewhere`
      cannot fail for the claim it names.
      **Scenario:** the sweep is a blocklist of five phrasings — `"cannot be
      linked"`, `"in this Stoa only"`, `"anywhere else"`, `"unlinkable"`,
      `"not be linked"`. I appended to the intro copy: *"This key stays private
      to this Stoa and no observer can connect it to your other Stoas."* That is
      the false privacy claim the spec calls "the one failure here that could
      actually harm someone", stated in plain words, and **all 49 tests passed.**
      It matches no needle: "stays private to this Stoa", "connect it to", "your
      other Stoas".
      **Measured:** mutation applied to `OnboardingScreen.qml:338`, suite run,
      49/49 green; restored.
      A blocklist cannot catch a phrasing nobody anticipated — the same lesson
      the correctness reviewer already recorded inside
      `test_a_row_shows_its_address_and_its_mark_and_nothing_else`, where the
      fix was to assert the **set** rather than a list of exclusions. That fix
      was not carried to this test, which guards the more dangerous property.
      **Severity: high** — this is the spec's own nominated worst case, and the
      test that exists for it is green against it.
      **Fixed**, by carrying the set-assertion instrument across exactly as you
      say it should have been. New `test_the_screen_says_only_what_it_is_
      allowed_to_say` holds `authoredCopy`, an **allowlist of every sentence
      this screen may show**, and drives all three phases (intro/slate, refused,
      kept) so each phase's copy is built before the walk. Every string the
      sweep finds must be in that list; the only exclusions are the values THIS
      TEST handed the screen — the two candidate addresses and two distinctive
      fixture strings (`REASON-FIXTURE`, `KEPT-ADDRESS-FIXTURE`) standing in for
      module-supplied text, matched exactly rather than by pattern so the
      exclusion is not a hole.
      That works because the screen's copy is entirely **authored** — no string
      on it is computed — so the set is finite and writable down. Adding a
      privacy claim now requires typing it into a list whose failure message
      names the requirement forbidding it.
      **Mutation 1, yours exactly:** appended *"This key stays private to this
      Stoa and no observer can connect it to your other Stoas."* to
      `OnboardingScreen.qml:338`. **2 of 52 fail**, the allowlist naming the
      whole sentence verbatim.
      **Mutation 2, and this is the one that matters:** because I had also
      widened the named blocklist, mutation 1 no longer proves the allowlist is
      what carries the property — the extended needle list could have caught it.
      So I ran a *different* fluent lie, matching no needle in the original five
      or my extension: *"What you do here is yours alone; nothing follows you
      between forums."* **The blocklist test PASSED. The allowlist caught it
      alone**, 1 of 52, naming the sentence. That is the measurement your
      finding predicts and it is why the allowlist, not a longer list of
      needles, is the fix.
      **I kept the named test** rather than deleting it, with its needles
      widened and lowercased. Two reasons: a reader asking "where is cross-Stoa
      unlinkability pinned?" should find a test called that, and the two fail
      differently — the allowlist says *nobody authorised this sentence*, the
      named test says *that sentence is the forbidden claim*. The allowlist is
      the load-bearing one and its comment says so, so nobody mistakes the
      needle list for the guard.

- [x] **`tester`** — the mark is not pinned at all: it can be deleted from every
      candidate row, or fed a shared constant, with the suite green.
      **Scenario:** the requirement is titled "A candidate row shows the full
      address **and the mark**", and its scenario says "each mark is derived from
      that row's address rather than from a shared or fixed value". No test in
      either file mentions the mark, the Identicon, or asserts a row contains a
      non-`Text` child. `tst_identicon.qml` tests the component in isolation; it
      says nothing about the row.
      **Measured:** two mutations at `OnboardingScreen.qml:420-423`.
      (a) `address: row.modelData.address` → `address: "00".repeat(32)`, so every
      candidate renders the identical mark — **49/49 passed**. (b) the whole
      `Identicon` block deleted, so no row shows a mark — **49/49 passed**. Both
      restored.
      The mark is the spec's second recognition channel against an impersonator
      grinding a lookalike address. Both halves of the scenario are unpinned.
      Note `visibleTextsOn()` cannot see it — the mark is not a `Text` — so the
      row-contents test's exact-set assertion passes unchanged either way.
      **Severity: high.**
      **Fixed** with the different instrument you correctly say this needs
      rather than a wider text sweep. `marksOn()` walks for Identicons —
      duck-typed on `address` **plus** two of the mark's own selector functions
      (`_form`, `_inkA`), because `address` alone also matches `AddressLabel`
      and would report a row as marked when it only shows its address. New
      `test_every_row_carries_a_mark_drawn_from_its_own_address` asserts both
      halves of the scenario.
      **Mutation (b), the deletion:** the `Identicon` block removed from the
      row. **1 of 52 fails** — `row 0 must carry exactly one mark … Actual: 0
      Expected: 1`. As you predicted,
      `test_a_row_shows_its_address_and_its_mark_and_nothing_else` **passed
      throughout**, which is the measurement that a text sweep cannot see this.
      **Mutation (a), the shared constant:** `address: row.modelData.address` →
      `address: "00".repeat(32)`. **1 of 52 fails**, showing both values —
      `Actual: 0000…  Expected: 4444…`.
      **A third assertion you did not ask for, covering a gap between your two
      mutations.** An address check alone would pass on a mark bound to the
      right address that *drew* from something else, so `markSignature()` reads
      the eight selectors Identicon actually derives (form, three inks, angle,
      pitch, duty, weave) and the test requires the two rows' tuples to differ.
      Verified the fixture discriminates: `4444…` gives form 2 / angle 120 /
      pitch 2 / weave 2, `5555…` gives form 8 / angle 15 / pitch 3 / weave 1, so
      the assertion is not passing on a coincidence of two addresses that happen
      to collide.
      Both bounds on the walker are pinned, as `candidateRowsOn()` already was:
      `compare(marks.length, 1)` per row, so a walker finding nothing fails
      rather than passing vacuously, and a row carrying two marks fails too.
      **What this still cannot see, and I will not claim it does:** whether the
      mark *reads* as a badge or a verification. That is the spec's other mark
      clause, it is a visual judgement, and your "what I could not check"
      section already places it correctly outside what any QML test reaches.

- [x] **`tester`** — neither the slate request nor the `who_am_i` request has its
      `stoa` field asserted; both can be sent empty.
      **Scenario:** the spec scenario "A request carries the fields its method
      reads" says *"the slate call and the identity-report call are made for a
      Stoa → each request is a JSON object carrying that Stoa"*. Only the **keep**
      request's `stoa` is asserted (`tst_onboarding_states.qml:541`).
      `test_no_request_names_an_identity` walks every request but only checks
      fields that must be **absent**.
      **Measured:** `Core.qml:146` `{ stoa: stoa }` → `{}` — **102/102 passed**.
      Separately `Core.qml:168` `{ stoa: stoa }` → `{}` — **12/12 launch-branch
      passed**. Both restored.
      A slate or identity report for the wrong (or no) Stoa is the failure this
      scenario exists to catch, and nothing would catch it.
      **Severity: medium** — core would presumably reject it, but the view's
      contract is what this capability owns.
      **Fixed**, one test per call, each asserting against the address the test
      itself handed the component rather than against whatever the request
      carried.
      `test_the_slate_request_carries_the_stoa_it_was_made_for`
      (`tst_onboarding_states.qml`) and
      `test_the_identity_report_request_carries_the_stoa_it_asks_about`
      (`tst_launch_branch.qml`).
      **Mutations, both yours:** `Core.qml:146` `{ stoa: stoa }` → `{}` fails
      the first (`Actual: undefined  Expected: abab…`); `Core.qml:168` the same
      fails the second, identically. Run together, since they are independent
      sites.
      Agreed on the severity framing and worth recording why it is not lower:
      identity is per-Stoa (`whoAmI(stoa)`), so a report naming the wrong Stoa
      is not a near-miss — it is the launch branch being decided by a different
      keystore's answer. That is in the second test's failure message.

- [x] **`tester`** — the spec pins a literal heading that no test contains.
      **Scenario:** "The opening state says an identity is being chosen" requires
      the heading `Choose the identity you will keep here.` I replaced it with
      `"Set up your account"` — account-setup framing, which is precisely the
      mental model PLAN §5.2.1 says generates a support question that cannot be
      answered — and **49/49 passed** (`OnboardingScreen.qml:315`; restored).
      `test_the_true_half_of_the_claim_is_still_made` covers only the scenario's
      second bullet.
      **Severity: medium** — the spec states an exact string; either pin it or
      stop stating it exactly.
      **Fixed** by pinning it, which is the right half of your either/or: the
      wording is load-bearing rather than decorative, for the reason your own
      scenario gives — "Set up your account" teaches account-setup framing, and
      the support question that framing generates ("how do I change my
      username?") is the one this flow structurally cannot answer.
      `test_the_true_half_of_the_claim_is_still_made` now requires the heading
      as an **exact element** (`texts.indexOf(...)`), not a substring of the
      joined copy, so a heading merely *containing* the sentence alongside
      something else does not satisfy it.
      **Mutation, yours exactly:** `OnboardingScreen.qml:315` →
      `"Set up your account"`. **Fails 2 of 52** — the named pin, and the
      allowlist, which reports the unauthorised string independently. Two
      independent detections of one mutation is fine here: they are different
      properties (this heading must be present / no unauthorised copy may be),
      and the second is what would catch a heading changed to something nobody
      thought to forbid.

- [x] **`tester`** — `test_the_uniqueness_note_states_the_obligation_without_a_word_count`
      does not do what its comment claims.
      **Scenario:** the comment says it is "written as a sweep over the spellings
      rather than a pin on one, so this fails on the reintroduction of **ANY**
      number". The sweep is eight literals (`"two words"` … `"5 words"`). I
      reworded the note to *"Your three-word name is not unique and names are not
      identifiers…"* — a word count, reintroduced — and **49/49 passed**
      (`OnboardingScreen.qml:534`; restored). `"three-word"` (hyphenated,
      attributive) is the most natural way the count comes back, and it is not on
      the list. Nor are `"a pair of words"`, `"3-word"`, or any capitalised form
      (unlike the `username` sweep, this one does not lowercase).
      **Severity: medium.** The shape the readability reviewer endorsed is right;
      the needle list under it is narrower than the endorsement assumes. A digit
      and hyphenated-ordinal regex over the joined copy would close it.
      **Fixed** by the route you name, and the endorsed shape is kept intact —
      still "assert no number at all" rather than a pin on the current one. Only
      the needle list under it changed.
      Three regexes over the **lowercased** copy replace the eight literals:
      `(one|…|ten|[0-9]+)[ -]words?\b` (covers `"three words"`, `"three-word"`,
      `"3 word"`, `"3-word"`, singular and plural); `(a pair|a trio|a couple|a
      set) of words`; and `(name|names|words) (is|are|of) <number>` for the
      count stated after the noun. Lowercasing closes the capitalisation hole
      you note.
      **Mutation, yours exactly:** `OnboardingScreen.qml:534` reworded to *"Your
      three-word name is not unique and names are not identifiers…"*. **Fails**,
      naming the needle: `found "three-word"`.
      **The regexes are themselves pinned**, which matters more than usual here:
      a pattern that matched nothing would make the sweep silently vacuous,
      which is the same defect one level down. So the test asserts each of five
      spellings — including your `"three-word"` and the literals the old list
      covered — is caught by at least one pattern. A regex edited into
      uselessness fails that immediately rather than going quiet.

- [x] **`tester`** — `everyTextOn()` is not pinned at one of its call sites, and
      that site is the unlinkability sweep.
      **Scenario:** neutering the helper (`return []` at its head) failed five of
      its six callers on their corpus bounds, but
      `test_no_copy_claims_the_identity_cannot_be_linked_elsewhere` **passed** —
      its loop body simply never runs, so it is vacuously green against an empty
      corpus. It is the only `everyTextOn` caller with no `texts.length > N`
      guard.
      **Measured:** mutation applied to `tst_onboarding_states.qml:1239`, run,
      44 passed / 5 failed with that test among the passes; restored.
      This compounds the first finding: the test is weak on its needle **and**
      weak on its corpus. `visibleTextsOn()` by contrast is pinned at all four of
      its call sites — mutating it to `return []` failed every one.
      **Severity: medium**, and fixing the first finding should fix this one too.
      **Fixed**, and your prediction held — but I added the bound explicitly
      rather than relying on that, because "the other fix probably covers it" is
      how a corpus bound goes missing in the first place.
      Both `everyTextOn` callers that guard an ABSENCE now carry
      `verify(texts.length > 15, …)`: the named unlinkability test, and the new
      allowlist test (which needs it most — an allowlist over an empty corpus is
      vacuously satisfied, the exact shape you measured).
      **Mutation:** `everyTextOn()` → `return []` at its head. Now **fails 7**
      including both, where it previously left the unlinkability sweep among the
      passes. Restored.
      **That run also found a defect in one of my own fixes**, which is why it
      was worth running rather than predicting. The apparatus test below counted
      the column with `collectText` and the whole screen with `everyTextOn`, so
      neutering one desynchronised them and it reported *"Found 0 carrier(s), 1
      of them in apparatus"* — a negative remainder. It failed, but by luck
      rather than by arithmetic. Both sides now draw from the same walk.
      The `> 15` floor is deliberate rather than `> 0`: the screen shows 22
      distinct authored strings plus addresses, so 15 is comfortably below the
      real corpus and far above any accidental truncation — a walk that reached
      only the heading would fail it.

- [x] **`spec-writer`** — the scenario "No affordance offers a rename" has no
      test and, as written, no test could discharge it.
      **Scenario:** *"the onboarding screen is shown in any of its states → it
      offers no control that edits, renames or replaces a name or an identity"*.
      There is no QML predicate for "reads as renaming"; a reviewer can see it,
      a `TestCase` cannot enumerate the controls that do not exist. Either
      narrow it to something checkable (e.g. no `TextInput`/`TextField`
      descendant anywhere in the tree, in any phase — which *is* testable and
      would catch the realistic regression), or mark it as a review obligation
      rather than a scenario. The repo's rule is never to write a scenario no
      test can honestly discharge.
      **Severity: medium** (spec defect, not a code defect).
      **Fixed** — narrowed to your first option, which captures the obligation
      rather than abandoning it. The requirement now reads "SHALL NOT offer a
      control into which text can be entered, in any of its states", and the
      scenario is "No text can be entered anywhere on the screen": every element
      in the tree, each phase in turn, none accepting text entry. The reasoning
      sits in the requirement rather than only here — a rename affordance needs
      somewhere to type, so text-entry absence is the property that catches the
      realistic regression and can honestly fail, while "reads as a rename" has
      no predicate and is now named in the spec as a review obligation on this
      screen rather than left as a scenario nothing discharges.
      **This leaves a test gap, and I am not the agent to close it.** The
      property holds today — `grep -rn "TextInput\|TextField\|TextEdit"` over
      `dialectica-ui/src/qml/` returns nothing — but no test asserts it, so the
      scenario is now discharged by the code and undischarged by the suite. It
      needs one `tester` test walking each phase's tree. Raised in my report to
      the coordinator rather than as a new box, since adding a row is what the
      one-row-per-agent rule exists to prevent.

- [x] **`spec-writer`** — the first requirement's "by a named wrapper rather than
      by a method string written at the call site" clause is unverifiable at
      runtime.
      **Scenario:** I replaced `Core.generateIdentitySlate(screen.stoaAddress)`
      with an inline `Core.call("generate_identity_slate", [...])` at the call
      site — the exact thing the clause forbids — and **49/49 passed**. The fake
      bridge sees only the method name, which is identical either way. This is a
      source-structure requirement wearing a behavioural scenario's clothes; its
      scenario ("Each call reaches the bridge under its own method name") tests
      something else and passes regardless. Say plainly that this half is a
      review/lint obligation, or drop it from the scenario's scope.
      Related and smaller: the scenario's "each names the same module as every
      other core call" is only covered transitively by `tst_core_call.qml` — the
      onboarding fake bridge discards its `module` argument. Acceptable as
      many-to-many coverage while every wrapper routes through `call()`, but it
      is not pinned here.
      **Severity: low.**
      **Fixed** — the clause is removed from the requirement, taking your first
      option. The requirement now contracts only what is observable: the three
      calls go through the one bridge, and "each SHALL reach that bridge under
      its own distinct method name". Your measurement is what settled it — an
      inline `Core.call(...)` at the call site is indistinguishable at the bridge
      from a wrapper, so the clause was a source-structure rule wearing a
      scenario's clothes.
      The spec now says so in prose, and names your 49/49 result as the reason
      without citing you by name: *"an earlier version of this requirement
      demanded wrappers, and substituting an inline call at a call site left the
      whole suite green. A requirement nothing can discharge is worse than none,
      so this is left to review and lint."* The *reasons* for wrappers — one edit
      on a rename, a load error instead of a silent empty screen — are kept in
      that prose rather than dropped, so the next reader sees why the convention
      is worth keeping even though no scenario enforces it.
      **Not cited to `design.md`, deliberately.** My first attempt pointed there;
      I checked and `design.md` names the wrappers (`:6`) without arguing them,
      so the pointer would have been a citation to an argument that is not
      made — which is the defect `design-review.md:232` had just caught in my
      `proposal.md`. `design.md` is `dev-writer`'s file and I did not edit it.
      The smaller half — "each names the same module" being covered only
      transitively — is left as you judged it: acceptable, unpinned, and now
      recorded here rather than in the requirement.

## What was clean

Coverage is otherwise good, and most of it is pinned well.

Every requirement other than the two named above has at least one test that
would fail if the behaviour changed. The three-outcome keep distinction — the
trap this screen exists for — is pinned from both directions: each outcome
against its own phase, the phases against each other, `keptIdentity` against
`null` on every non-keep, and the `identityKept` signal against firing on a
refusal. The launch branch is pinned on all four replies including the two
absent cases and their reason strings, and the re-ask is driven through the
screen's real signal rather than a test hook.

The encryption and backup-gap reports are now measured on rendered, visible text
with each reply pinned to its own hardcoded phrase in **both** directions — the
safety-relevant direction (an unencrypted key must never produce the reassuring
sentence) is asserted explicitly. `visibleTextsOn()` and `visibleTextMatching()`
are both pinned, and so is every one of their callers; `candidateRowsOn()` is
pinned on its upper bound as well as its lower one. The three-valued handling of
`encrypted` and `recoveryNeedsTheRecord` is pinned on all three values each.

The address-in-full requirement is genuinely pinned: setting `full: false`
fails `test_a_row_shows_its_address_and_its_mark_and_nothing_else` immediately.
The exact-set shape of that test is the right instrument and should be the
template for the copy sweeps above.

The two `NO SPEC:` markers (`OnboardingScreen.qml:86` and `:377`) are both
benign — no own `who_am_i` call, and an unpinned button label — and both are
already noted by the readability reviewer. No unmarked decision was found that
the existing findings do not cover.

**Untestable by any QML test, and correctly kept out of the spec:** whether the
mark presents as a badge, whether the phases are visually distinct, legibility,
and whether the candidate row leaves room for a name to arrive above the address
without the layout moving. The last is promised to an external designer in
`design.md` and `UI-BRIEF.md` but the **spec does not state it** — the spec's
mark requirement is phrased as "SHALL NOT be presented as a verification, a
badge, or anything that reads as checked", which is a copy obligation rather than
a visual one. That division is right and needs no change.

## Spec soundness and PLAN.md

The spec is internally consistent. The one place two requirements appear to
collide — "no row presents a name" against "the screen states that a name is not
unique" — is addressed in the spec's own prose ("This is required even though no
row shows a name"), and both halves are tested.

- [x] **`spec-writer`** — `docs/PLAN.md` on `origin/main` §5.2.1 contradicts both
      this spec and PLAN's own §5.2.
      **Scenario:** the UI-obligations bullet at `PLAN.md:1671-1674` reads
      *"**Never imply a user's names are linked across Stoas** … The names are
      unlinkable by construction and an interface that groups them has undone
      §5.2 in the presentation layer."* That states the property as holding and
      instructs a designer to protect it. §5.2 at `PLAN.md:914-919` says the
      opposite for the MVP — one key signs in every Stoa, "Cross-Stoa
      unlinkability is **suspended, not withdrawn**… **Do not describe the MVP as
      having it**" — and this spec's ninth requirement forbids the screen from
      claiming it. A reader reaching §5.2.1 first is told to design for a
      property that does not hold, which is how the false copy this spec removes
      got written in the first place. `docs/UI-BRIEF.md:92-107` already handles
      the suspension correctly; PLAN's own bullet is the stale one.
      **Severity: medium.** `openspec validate --strict` cannot see this — it
      checks heading structure only.
      **Fixed elsewhere — PR #64 (`docs/name-shape-sweep`), not duplicated
      here.** That PR's second commit is this finding, acted on: its body names
      it as *"Added after review, from a spec-test reviewer's finding on
      `piece/ui-onboarding`"*, and quotes the `copy.json` sentence as the copy
      the stale bullet generated.
      **Verified rather than taken on trust**, since a PR body is a claim.
      `git grep -n "unlinkable by construction" FETCH_HEAD -- docs/PLAN.md` on
      that branch returns exactly one line (`:1959`), and reading it shows the
      phrase surviving only inside the strike-through record of its own
      correction. The live bullet now says §5.2 is the authority and deliberately
      does not restate it, adds the direction that actually shipped (never claim
      identities *cannot* be linked), and quotes "this key cannot be linked to
      you anywhere else" as the copy it must not generate. That is your finding's
      fix, including the half about the direction the old bullet did not cover.
      **Not edited here on purpose.** Your own diagnosis is that two copies of a
      suspended-property rule is how §5.2.1 drifted; two branches editing §5.2.1
      in the same week is the same failure one level up, and would conflict at
      merge. Left to #64.

PLAN.md is otherwise current against this spec: the slate and unlimited
regeneration at `PLAN.md:950-953` are struck through and marked *"Built — see the
`identity-onboarding` spec"*, which is the right shape. The stale four-word count
at `PLAN.md:947` and `:1682` is already recorded as deferred with a named
successor (`docs/name-shape-sweep`, `d3e7579`) in `UI-BRIEF.md:195-202`, so it is
not opened here.

## Mutations run

| # | Mutation | Result |
|---|---|---|
| 1 | `Core.qml:146` slate request `{stoa}` → `{}` | **survived** 102/102 |
| 2 | `Core.qml:168` `who_am_i` request `{stoa}` → `{}` | **survived** 12/12 |
| 3 | `OnboardingScreen.qml:103` named wrapper → inline `Core.call(...)` | **survived** 49/49 |
| 4 | `OnboardingScreen.qml:421` row mark fed a shared constant | **survived** 49/49 |
| 5 | `OnboardingScreen.qml:420-423` row mark deleted entirely | **survived** 49/49 |
| 6 | `OnboardingScreen.qml:534` word count reintroduced as `"three-word"` | **survived** 49/49 |
| 7 | `OnboardingScreen.qml:338` unlinkability claim reworded back in | **survived** 49/49 |
| 8 | `OnboardingScreen.qml:315` spec-pinned heading → `"Set up your account"` | **survived** 49/49 |
| 9 | `OnboardingScreen.qml:438` `full: true` → `full: false` | caught (1 fail) |
| 10 | `tst_onboarding_states.qml` `visibleTextsOn()` → `return []` | caught (4 fails, all callers) |
| 11 | `tst_onboarding_states.qml` `everyTextOn()` → `return []` | caught at 5 of 6 callers; **unlinkability sweep survived** |

All mutations reverted; `git status --porcelain` is empty and the suite is back
to 102 passing.
