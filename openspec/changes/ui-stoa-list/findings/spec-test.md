# Spec/test findings — `ui-stoa-list`

Reviewed dimension: **does the test suite pin what the spec requires, and can
each test fail for the reason it names?** Read the spec and the tests; did not
read `JoinScreen.qml`, `StoaListScreen.qml`, `StoaReference.qml` or `Main.qml`
to judge a test's adequacy. The one exception is the mutation sampling below,
which necessarily edits code — each mutation changed only the lines it names,
and the tree was restored and proved clean with `git status --porcelain`
returning empty rather than from memory.

The four earlier findings files (correctness, security, readability,
architecture) were read first and nothing they raised is repeated here.

**The spec is 15 requirements and 42 scenarios, not the 13 and 38 the task
description claims.** Counted with `grep -c` against
`specs/stoa-navigation-view/spec.md`. Mentioned because the brief was a claim
and every claim here was checked against the artifact.

Baseline: **100 tests across 5 spec files, all green**, before and after every
mutation.

---

## Mutations run

Five, each applied to one named location, run through
`dialectica-ui/tests/run-qml-tests.sh`, then reverted. Every mutation was
confirmed to have **landed** (`git diff --stat -- dialectica-ui/src` non-empty)
and, for the three that plant rendered text, confirmed to be **actually on
screen** — via a throwaway `TestCase` in the tests directory that dumped the
visible-text corpus through a deliberately failing `compare`, then deleted. That
second check is the one this file's own header says is easy to skip: an absence
assertion over a corpus with no candidate in it proves nothing, and so does a
mutation that never rendered.

| # | Mutation | Result |
|---|---|---|
| 1 | `StoaListScreen.qml:498` creation-outcome caption → `"CREATED — THIS IS ITS ADDRESS. You moderate this Stoa, and joining it generates you an identity for it alone."` | **SURVIVED** — 100/100 pass |
| 2 | `StoaListScreen.qml:476` a visible `TextInput objectName:"creatorKeyField"` labelled `CREATOR KEY` added beside the title field | **SURVIVED** — 100/100 pass |
| 3 | `StoaListScreen.qml:351` row's `AddressLabel` replaced with a hand-written `head8 + "…" + tail6` elision | **SURVIVED** — 100/100 pass |
| 4 | `JoinScreen.qml:299` caption → `"FOUNDING TITLE — FIXED FOREVER — AND THIS STOA'S PRESENT NAME, AS ITS MODERATOR HAS IT TODAY"` | **SURVIVED** — 100/100 pass |
| 5 | `ScreenFrame.qml:10` `property alias apparatus: app.content` → `property var apparatus: []`, to check `bodyText`'s callers as well as the helper | **CAUGHT** — reverted before measuring further; see the prose note below |

Four of five survived. Each survivor is a box.

---

- [ ] **`tester`** — `tst_stoa_screens.qml` has no test scanning the **list or
      the creation outcome** for a moderator or per-Stoa-identity claim, so
      Requirement "Nothing on these screens claims a per-Stoa identity, a
      membership, or a moderator" (spec.md:583) is pinned over the wrong corpus
      **Scenario:** spec.md:611 says **WHEN** the list renders a Stoa, *and the
      creation outcome renders a newly created one*, **THEN** nothing rendered
      states that the user moderates it. The only test on this subject,
      `test_nothing_on_the_preview_promises_a_per_stoa_identity` (line 940),
      scans `bodyText(joinScreen)` after a join — a different screen entirely.
      Nothing scans `StoaListScreen`.
      **Measured (mutation 1):** I changed the creation-outcome caption to read
      `CREATED — THIS IS ITS ADDRESS. You moderate this Stoa, and joining it
      generates you an identity for it alone.` — both claims this requirement
      forbids, the second being the one the spec calls "the one failure here that
      could actually harm someone". **All 100 tests passed.** The planted
      sentence was confirmed rendered and visible: the corpus dump returned
      `…CREATED — THIS IS ITS ADDRESS. You moderate this Stoa, and joining it
      generates you an identity for it alone.\nb02d5e77a41c…`. This is the
      file's own documented corpus defect — "an absence assertion is only as
      strong as its corpus" — recurring in the one requirement the header calls
      the most harmful to get wrong.
      **Severity:** high. The fix is a `bodyText(listScreen)` scan in the created
      state, reusing the `\bidentity\b` and `you moderate` predicates that
      already exist, plus a corpus assertion (`body.indexOf("CREATED") >= 0`) so
      it cannot go vacuous the way its sibling did.

- [ ] **`tester`** — no test asserts the **absence of a creator-key or identity
      field** on the create affordance, so the central prohibition of
      "Creating a Stoa asks for a title and nothing else" (spec.md:447) is
      unpinned
      **Scenario:** spec.md:473 requires **THEN** it accepts a title **AND** it
      offers no field for a creator key or an identity to create under. The spec
      states the stake plainly: "A field for one would be a field that mints a
      Stoa nobody can moderate, at an address that cannot be un-minted."
      `test_the_create_affordance_is_present_when_no_key_exists` (line 1200) is
      the only test here and asserts only that one `createStoaButton` exists and
      is enabled — the scenario's *first* clause, never its second.
      **Measured (mutation 2):** I added a visible `TextInput` with
      `objectName: "creatorKeyField"` and a `CREATOR KEY` label beside the title
      field. **All 100 tests passed.** Confirmed rendered — the corpus dump
      returned `…CREATE A STOA\nTransport Notes\nCREATOR KEY\nCreate it…` and a
      walker keyed on the objectName found it visible.
      **Severity:** high. Note that a naive walker will miss this: the shipped
      title field is a bare `TextInput` with no `placeholderText`, so a probe
      keyed on that property returns nothing on the clean tree. The assertion
      wants a walk over elements exposing `text`-input behaviour, or an explicit
      count of input fields on the create row — pinned as a number, since
      `<= 1` would pass on zero and zero is a different defect.

- [ ] **`tester`** — a hand-rolled second address abbreviation in a list row
      survives, so "A second abbreviation MUST NOT be written" (spec.md:23-27)
      is pinned by nothing
      **Scenario:** the requirement is explicit about why: "an elision that keeps
      only a head and a tail is the shape vanity-address generators are built to
      defeat, and a second implementation is how one screen quietly acquires the
      weaker form." `test_a_row_carries_the_address_as_well_as_the_title`
      (line 340) checks `shown.indexOf("7f3a91c4") >= 0` and says in its own
      comment that it asserts "the ADDRESS is there in some form, not that a
      second elision was written" — so the prose requirement is knowingly
      unpinned.
      **Measured (mutation 3):** I replaced the row's `AddressLabel` with a
      `Text` rendering `row.rowStoa.slice(0,8) + "…" + row.rowStoa.slice(-6)` —
      the weaker head-and-tail form, bypassing the component that owns the
      8-8-6 abbreviation. **All 100 tests passed.** It passes precisely because
      both the right and the wrong implementation render the head `7f3a91c4`,
      which is the only thing asserted. This is the repo's defect family exactly:
      the test asks a question both implementations answer the same way.
      **Severity:** medium-high. A predicate that does distinguish them: assert
      the row's address element is an `AddressLabel` (it exposes `full`, which a
      bare `Text` does not), or assert the middle-8 group is on screen — the
      8-8-6 form renders a middle group that a head-and-tail elision drops.
      Either fails on mutation 3 and passes on the shipped code.

- [ ] **`tester`** — `test_no_current_title_is_rendered_while_nothing_resolves_one`
      (line 883) blocks two literal strings where the requirement is about what
      a caption **claims**, and an equivalent claim passes
      **Scenario:** the test forbids `CURRENT TITLE` and `chosen by a moderator`.
      Requirement "No current title is rendered until one has been resolved"
      (spec.md:254) forbids rendering "a current title, a present name, or any
      title attributed to a moderator", and its scenario at spec.md:278 says
      **AND** nothing rendered attributes a title to a moderator **or presents
      one as the Stoa's present name**.
      **Measured (mutation 4):** I extended the founding-title caption to
      `FOUNDING TITLE — FIXED FOREVER — AND THIS STOA'S PRESENT NAME, AS ITS
      MODERATOR HAS IT TODAY`. **All 100 tests passed.** It carries neither
      blocked literal while asserting both things the requirement forbids, on a
      build where metadata resolution is not implemented — so it tells the reader
      a moderator has not renamed this Stoa, which is the exact false claim the
      requirement exists to prevent.
      **Severity:** medium-high. This is the second defect family the file's
      header documents — "the assertion reads the right value and asks the wrong
      question… fails on an innocent reword, passes on a fluent lie" — recurring
      in a requirement the tester's pass did not revisit. The fix wants the same
      shape used for the address note: forbid the *conjunction* of a
      present-tense naming word (`present name`, `current`, `now called`,
      `today`, `as it stands`) with the title as its subject, and forbid
      attributing any title to a moderator, rather than blocking two phrasings.

- [ ] **`spec-writer`** — the `stoa:` display prefix is behaviour two tests pin
      and **no requirement describes**, with no `NO SPEC` marker
      **Scenario:** `test_a_display_prefix_is_stripped_before_anything_is_sent`
      (line 615) and `test_a_repeated_display_prefix_is_stripped_rather_than_forwarded`
      (line 629) pin that `stoa:` is stripped on the way in, stripped repeatedly,
      stripped across interleaved whitespace, and never added on the way out.
      `grep -in "prefix\|stoa:"` over the spec returns **nothing** — the spec is
      entirely silent. The second of those tests is the regression for a shipped
      security defect (correctness findings, `StoaReference.qml:45`): a doubled
      prefix reached `join_stoa` and came back as "does not hash to this
      address", converting a malformed paste into a verification accusation
      pointed at the user's sender. That is squarely the harm
      "A malformed address and an unjoinable one are different failures"
      (spec.md:326) exists to prevent, and the requirement does not mention the
      mechanism that produced it.
      **Severity:** medium. The other four `NO SPEC` markers are declared and the
      architecture reviewer has already judged which of them belong in the spec;
      this one is the same category of decision **without the marker**, which is
      the shape worth more attention rather than less. Capturing it under the
      malformed-versus-unverified requirement would also give the regression a
      requirement to cite.

- [ ] **`spec-writer`** — the spec specifies **no route out of any terminal
      state**, so the strandedness the architecture reviewer measured on the feed
      recurs twice more with no requirement to cite
      **Scenario:** `grep -in "cancel\|back\|return"` over the spec returns no
      navigation-return language anywhere. The architecture reviewer opened
      boxes for `list → feed`. Two further one-way transitions are not covered by
      those boxes: (a) **preview → joined** — "A join is reported from the core's
      reply" (spec.md:375) requires the screen to report success and stops; no
      scenario says the user then reaches the Stoa, returns to the list, or that
      a cancel affordance exists at all. The implementation *has* a `cancelled`
      signal and the suite calls `join.cancelled()` at line 825, but purely as
      setup for the re-preview test — so a cancel route is **relied on by the
      tests and required by no scenario**, and deleting it would leave the suite
      green. (b) **create → created** — "A created Stoa's address is shown"
      (spec.md:497) requires the address be rendered and stops; nothing requires
      the new Stoa to join the list, and no test asserts a reload after creation
      (`reload()` appears only as test setup, lines 501/550/1446).
      **Severity:** medium. The architecture reviewer's proposed scenario ("a
      user who has opened a Stoa can return to the list without restarting")
      fixes one of the three. The general form — every state a user can enter has
      a specified way out — is what stops the next view piece rediscovering this,
      and is the same family of rendering obligation this capability already
      owns.

- [ ] **`dev-writer`** — `openspec validate --strict` **fails on this change
      today**, blocking the runner's final gate
      **Scenario:** `tasks.md` has two sections numbered `## 9` — "Making the
      absence assertions honest about the apparatus" (line 148) and "The tester's
      pass" (line 182) — so task IDs 9.1 through 9.6 are each declared twice.
      Measured: `openspec validate ui-stoa-list --strict` exits 1 with six
      `Task ID "9.N" is duplicated` warnings. `tasks.md:15` names
      `openspec validate --strict, then archive` as the last gate, so this is
      not cosmetic.
      **Severity:** low, but it is a hard block on merge and costs one renumber.
      Addressed to `dev-writer` as the owner of `tasks.md`.

- [ ] **`spec-writer`** — spec.md:427-432 states a prohibition its own scenario
      requires violating, and a reader could resolve the ambiguity either way
      **Scenario:** "A Stoa already held whose title matches…" reconciles itself
      against the idempotence rule with: "Comparing *titles* to surface a
      lookalike is a rendering decision… Comparing *addresses* to decide whether
      a completed join was new is a claim about what the core did… The first is
      required here; the second is forbidden." But that requirement's **second
      scenario** (spec.md:442) — "A same-title Stoa that is the same address is
      not shown as a second Stoa" — cannot be satisfied without comparing
      addresses against the held listing, which
      `test_the_same_address_is_not_shown_beside_itself_as_a_second_stoa`
      (line 1183) duly does. The qualifier that makes this consistent ("to decide
      whether a completed join was new") is present in the prose but the summary
      sentence drops it, so the paragraph reads as forbidding address comparison
      outright while the scenario below mandates one.
      **Severity:** low. No defect in the code or the tests — both do the right
      thing. `validate --strict` checks heading structure only and cannot see
      this; it is the kind of self-inconsistency the repo's own archive notes say
      has happened here before. One clause restores it: forbid comparing
      addresses *for the purpose of deciding whether a join was new*, which is
      what the surrounding prose already means.

- [ ] **`spec-writer`** — `docs/PLAN.md` still states as **not built** two
      passages describing behaviour this spec now contracts, and the branch's own
      PLAN.md edit fixed the neighbouring three while leaving these
      **Scenario:** the branch already corrects §9.1 Stage D and §9.2 items 2, 6
      and 7 into the right strikethrough-plus-pointer shape — verified with
      `git diff origin/main -- docs/PLAN.md`, which shows exactly those hunks.
      Two passages in the same subject were missed and are live in the branch
      copy:
      **(a) §4.8 Phase 1, PLAN.md:711-714** — *"Still to get right, and still not
      built: in-post addresses are **attacker-supplied content**. Render them as
      an explicit affordance the reader chooses to act on, never auto-join, and
      show what is being joined before joining it. That is a UI obligation — see
      §5.5, which holds it."* The spec's "Joining shows what is being joined, and
      joins nothing until the user acts" (spec.md:206) contracts every clause of
      that sentence, and its scenarios at 229 and 236 pin both halves.
      **(b) §9.1 subsection 5, PLAN.md:3540-3546** — the same obligation restated
      as mechanics and closing *"**This is a UI obligation and is not built**,
      which is why it stays here rather than moving."* That trailing clause is
      PLAN.md's own rule for when a passage should move to a spec, and its
      condition is now met. The paragraph immediately below it (PLAN.md:3548-3551)
      duplicates the title-is-the-forgeable-half reasoning that the spec now owns
      across three requirements — including a near-verbatim twin of spec.md:312's
      "decoration, freely chosen, and matched against nothing".
      **Severity:** medium. Both are the (b) and (c) shapes the archive guidance
      names, in a file the branch has already opened and half-corrected, so the
      cost is small and the inconsistency is visible to the next reader.

- [ ] **`spec-writer`** — PLAN.md routes this spec's central obligation to
      **§11.1, a section that does not exist**, and the spec does not say it has
      taken it over
      **Scenario:** PLAN.md:3265-3273 says *"§11.1 'Rendering obligations,
      collected' arrives with the `vouching-state` change and is not in this file
      until that lands"* and names the three obligations bound for it — the
      second being *"a join confirmation that must show the address and not only
      the title"*. Measured: `grep -n "^### 11.1\|^## 11.1"` over PLAN.md returns
      **nothing**, while `§11.1` is cited **16 times** (lines 1515, 1592, 1654,
      1659, 3262, 3265, 3270, 3272, 3308, 3384, 3431-3432, 3504, 3512, 3548,
      3589, 4052). That join-confirmation obligation is now contracted by this
      spec's "A listed Stoa is rendered with its address, never with its title
      alone" (spec.md:11) and the preview's full-address requirement
      (spec.md:217-221) — so it has a home, and PLAN.md's forward reference to a
      never-landed list is the wrong pointer for it. PLAN.md:1515 in particular
      reads *"The interface consequence is therefore identical in shape to the
      join confirmation, and belongs on §11.1's list"*, aiming a live
      cross-reference at a list this spec has superseded for that case.
      **Severity:** medium. Not a defect in this change's code or tests, and not
      a reason to build §11.1 here; the ask is that PLAN.md's join-confirmation
      entries point at `stoa-navigation-view` instead, so the obligation is not
      waiting on an unrelated change to acquire a home it already has.

---

## What was clean

**Coverage is good.** Of 42 scenarios, 38 have a test that pins the behaviour
they describe, several of them many-to-many and several stronger than the
scenario requires. The three-state list distinction, the non-array `items`
guard, the unreachable-core and non-JSON paths, the round trip between
`shareText` and `parse`, the preview-makes-no-call assertion against the bridge's
**call log** rather than against appearance, the four reference-identity tests
driving `Main.qml` rather than a fresh screen, and the two strengthened
meaning-over-literal tests (`…say_different_things_to_do`,
`…cannot_be_simplified_into_an_unqualified_verified`) are all genuinely strong —
the last two in particular assert structure with a conjunction, which is the
right answer to the family the header documents and is measurably better than
the blocklists they replaced. The four scenarios without a test are the two named
in boxes above (creator-key field, moderator claim on list and creation) plus
their halves.

**The clipboard boundary is stated rather than papered over, and the spec does
not require what no test can establish.** `grep -in "clipboard\|copy"` over the
spec returns nothing: the share requirement says what is *produced* must carry
both halves, which `test_a_share_carries_both_halves_and_the_address_in_full`
pins directly on `StoaReference.shareText`'s return value — no clipboard needed.
`test_a_share_string_reaches_the_clipboard_sink_verbatim` asserts what the sink
was *asked* to copy and says so in its own comment; `tasks.md:43-47` and its
"What remains unverifiable" section say the same. All three agree, and none
claims more than it has. This is the right handling of an honest limit.

**No scenario in this spec is untestable as written.** I looked specifically for
the sibling piece's "*No affordance offers a rename*" shape — a prohibition with
no QML predicate — and particularly in the verification-claim requirements
(spec.md:289, 583), which are about what a screen must not let a reader conclude
and are therefore the natural home for one. Every scenario there has a workable
predicate: a corpus scan with a negation regex, which the shipped tests
demonstrate is both writable and capable of failing. The two uncovered scenarios
are **untested, not untestable** — my mutations 1 and 2 each produced a rendered,
walker-findable artefact, which is the proof that a predicate exists.

**Helpers and their callers are pinned on both bounds.** The prompt flagged the
sibling failure where a corpus filter was pinned and the *function calling it*
was mutated to return `""` with every test green. That does not reproduce here:
`bodyText` calls `apparatusText`, and if `apparatusText` returned `""` the
callers would still be caught — `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus`
asserts `body.indexOf("started collecting") >= 0` and
`body.indexOf("Nym Research") >= 0`, which fail on an empty return, and
`test_nothing_on_the_preview_promises_a_per_stoa_identity` and
`test_joining_a_stoa_already_held_is_success_with_no_warning` each assert their
corpus is non-empty *before* asserting an absence over it. The `if (app !== "")`
guard is correctly scoped: it skips only the disjointness check, which is the one
assertion that legitimately stops applying when the apparatus is removed.
`genesisFor` is likewise pinned positively (through the sink test's `shareText`
round trip) and negatively (line 509), and `canShare` is pinned through the
visible-button count rather than through its own return value.

**PLAN.md's open-questions section needs nothing from this change.** §13's live
entries are identity derivation, LEZ key paths, SDS participant ceilings,
moderator-set ordering and ranking — none of which this spec answers — and its
two resolved entries are already in the correct strikethrough shape. So the
staleness above is entirely the future-intent and duplication kinds, not a
question this spec has quietly answered. The branch's own PLAN.md edit is also
the right shape where it landed: strikethrough preserving the superseded wording,
plus a pointer naming the capability, which keeps the history legible.

**The spec does not depend on any PLAN.md section by number**, so no requirement
here was written against a passage `origin/main` has since changed out from under
it — its citations are all to sibling specs (`stoa-membership`, `stoa-metadata`,
`stoa-genesis`, `module-wire-contract`, `posting-capability`). The staleness runs
the other way only.

## What I could not check

- **Anything visual.** Whether the planted `CREATOR KEY` field or the extended
  caption would be *noticed* by a reviewer looking at the running app is not
  something `qmltestrunner` can answer, and neither is whether the absent share
  button reads as absence rather than breakage.
- **Counts spelled in words or in non-digit glyphs**, which
  `test_no_digit_is_rendered_that_the_reply_did_not_supply` names as its own
  residue at line 1521. I did not mutate it because the test already declares
  the limit honestly; it is a real gap but a declared one.
- **The core's replies.** Every fixture here is a fake. I did not verify any
  reply shape against `wire.rs`; the architecture reviewer did that for
  `create_stoa` and `list_stoas`.
