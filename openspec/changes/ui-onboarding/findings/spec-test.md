# Spec/test review — `ui-onboarding`

Read: `specs/view-identity-onboarding/spec.md` (14 requirements, 48 scenarios),
`tst_onboarding_states.qml`, `tst_launch_branch.qml`. The implementation was not
read except for the exact lines mutated in part 2 and the two `NO SPEC:` comment
blocks.

Baseline before and after: **102 pass across 6 spec files**, tree restored
(`git status --porcelain` empty).

## Findings

- [ ] **`tester`** — `test_no_copy_claims_the_identity_cannot_be_linked_elsewhere`
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

- [ ] **`tester`** — the mark is not pinned at all: it can be deleted from every
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

- [ ] **`tester`** — neither the slate request nor the `who_am_i` request has its
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

- [ ] **`tester`** — the spec pins a literal heading that no test contains.
      **Scenario:** "The opening state says an identity is being chosen" requires
      the heading `Choose the identity you will keep here.` I replaced it with
      `"Set up your account"` — account-setup framing, which is precisely the
      mental model PLAN §5.2.1 says generates a support question that cannot be
      answered — and **49/49 passed** (`OnboardingScreen.qml:315`; restored).
      `test_the_true_half_of_the_claim_is_still_made` covers only the scenario's
      second bullet.
      **Severity: medium** — the spec states an exact string; either pin it or
      stop stating it exactly.

- [ ] **`tester`** — `test_the_uniqueness_note_states_the_obligation_without_a_word_count`
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

- [ ] **`tester`** — `everyTextOn()` is not pinned at one of its call sites, and
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

- [ ] **`spec-writer`** — the scenario "No affordance offers a rename" has no
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

- [ ] **`spec-writer`** — the first requirement's "by a named wrapper rather than
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

- [ ] **`spec-writer`** — `docs/PLAN.md` on `origin/main` §5.2.1 contradicts both
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
