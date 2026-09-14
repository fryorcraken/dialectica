# Correctness review — prose claims

**Dimension: correctness of prose claims** (the only dimension reviewed; the
owner scoped this piece to one review). Reviewed at `4d1add6`. The branch moved
under me mid-review — I branched from `9fc8c6a` and the dev-writer landed
`4d1add6` (the `feed.rs`/`names.rs` rebase repair) while I read; I re-read at the
new tip and everything below is measured against `4d1add6`.

Every citation the repair *introduces* was verified by reading the target, not by
grepping for the filename.

---

- [ ] **`dev-writer`** — `openspec/changes/drop-ui-brief/design.md:85-102` (§2.3) —
      the `agora` decision describes a wordlist exclusion that does not exist,
      and cites a line that is about something else
      **Scenario:** §2.3 is titled "`agora` stays in the wordlist, on a reason
      that survives" and says the entry is in an "exclusion table", that "the
      first-to-drop flag stays", and that a name reading as `agora` is "the same
      failure `stoa` itself is excluded for". All three are false on this branch.
      `names.rs:1353` is a test named `the_lists_carry_no_exclusion_of_any_kind`
      which asserts `stoa`, `agora`, `archon`, `strategos`, `tyrannos` and
      `genesis` are **PRESENT** in `NOUNS`, with the comment "The contract now
      says no word is kept out for what it says... their absence would be the
      defect and their presence is the requirement." `grep -c "^agora$"
      dialectica/rust-lib/dialectica-core/wordlists/nouns.txt` returns **1** —
      `agora` ships in the list. There is no exclusion table and no
      first-to-drop flag anywhere in `names.rs`, `nouns.rs` or
      `gen_wordlists.rs`.
      **Measured:** the cited site `:1368` — `docs/PLAN.md` line 1368 on
      `origin/main` — is about credential expiry windows ("A wall-clock timestamp
      is author-asserted, so a lying `createdAt` extends a proof's life"),
      not about a wordlist. `grep -rn "agora"` over the deleted
      `docs/UI-BRIEF.md` returns **zero** hits, so the brief never carried the
      worked example §2.3 says the entry's reason died with. And `git grep -n
      "UI-BRIEF" origin/main -- .../names.rs .../names/nouns.rs` shows the only
      brief citation in that area was `names.rs:37`, which is obligation 6 about
      rendering, not a wordlist entry.
      **Severity: high** — this is a fabricated decision record. Nothing in the
      diff touches a wordlist (`git diff --stat origin/main...` shows no
      wordlist, `names/` or spec file changed), so the section documents work
      that was never done, on a citation that does not resolve, contradicting
      the code. A future curator reading §2.3 would believe an exclusion screen
      exists and that `agora` is flagged first-to-drop — the exact position the
      `the_lists_carry_no_exclusion_of_any_kind` test was written to prevent
      being reintroduced.
      **Genuine defect**, not stylistic.

- [ ] **`dev-writer`** — `docs/PLAN.md:3472-3473` — the new publish-prohibition
      paragraph claims to be the only place the prohibition can be stated, and
      it is not
      **Scenario:** the rewritten sentence reads "**The half of the rendering
      obligation that is true today is the prohibition, and it is stated here
      because no other half can be.**" `design.md:76-80` (§2.2) makes the same
      claim in stronger terms: "With the brief gone there is no other document
      that states it in prose, so PLAN.md states it."
      Both are false. `openspec/specs/composer-view/spec.md:294` is a live
      requirement headed **"A successful publish claims local storage and never
      delivery"**, and it states the prohibition in prose, in more detail, with
      five scenarios. It covers every clause the new PLAN.md paragraph does and
      one it does not: `:306` — "The view SHALL NOT display a count of peers
      reached, a delivery state, or **a progress indicator that resolves into a
      delivery claim**" — which is the no-spinner rule; and `:309` goes further
      than the brief ever did, requiring the view to *positively deny* delivery
      knowledge rather than merely stay silent.
      **Measured:** `grep -rn "^### Requirement" openspec/specs/composer-view/spec.md`
      lists it; `git diff --stat origin/main...piece/drop-ui-brief -- openspec/specs/`
      is **empty**, so that requirement predates this change and was live
      throughout. The brief's own obligation 9 text ("a spinner that can never
      resolve is worse than no spinner") is reproduced almost verbatim in the new
      PLAN.md prose while the spec that already contracts it goes unnamed.
      **Severity: medium.** The prohibition itself is stated truthfully — nothing
      here is a *wrong rule*. What is false is the justification for stating it,
      and that matters because §2.2 calls this "the most consequential" of the
      four delegations and rests the decision to restate on the claim that
      nothing else holds it. PLAN.md's own convention is to name the spec where a
      view obligation has been contracted — it does exactly that at `:2904` and
      `:3530` for `composer-view`. The repair should say the prohibition is
      contracted by `composer-view`'s "A successful publish claims local storage
      and never delivery" and that PLAN.md records why the ordering went that
      way, rather than asserting no other document can hold it.
      **Genuine defect** (a false claim in newly-written prose), not stylistic.

- [ ] **`dev-writer`** — `openspec/changes/drop-ui-brief/design.md:160-176` (§4) —
      the count of surviving `SPEC.md` citations is wrong
      **Scenario:** §4 says "Six such citations remain across `sanitise.rs`,
      `SanitisedText.qml`, `FeedScreen.qml`, `tst_sanitised_text.qml` and
      `IDENTICON.md`", then "**Two of the six were unavoidable and are fixed
      here**" and "**The other four are left alone**".
      **Measured:** `grep -rn "SPEC.md"` over `docs/`, `dialectica/` and
      `dialectica-ui/` at `4d1add6` returns **seven**, not four:
      `IDENTICON.md:129`, `FeedScreen.qml:330`, `tst_sanitised_text.qml:13`,
      `sanitise.rs:56`, `sanitise.rs:96`, `SanitisedText.qml:10`,
      `SanitisedText.qml:39`. Two files carry two each, which is what the
      one-per-file arithmetic missed. So the original total was nine, not six.
      **Severity: low.** The substantive judgements in §4 are sound — I verified
      `git log --all --diff-filter=D --name-only -- "*SPEC.md"` returns nothing,
      so "never existed in this repository's history" is true, and leaving them
      for their own piece is right. Only the number is wrong, and it is the kind
      of number a follow-up piece would size its scope from.
      **Genuine defect** (a stated measurement that does not hold), minor.

---

## What I checked and found clean

**The four PLAN.md delegations (§2.2's table) are otherwise sound.** I read each
rewritten passage against the code and the specs:

- `:1503` (the auto-join restatement) faithfully carries the brief's line at
  `UI-BRIEF.md:409` — "render it as an affordance the reader chooses to act on.
  **Never auto-join.**" — into PLAN.md's own voice with no loss. The rule is
  independently borne out by `stoa-navigation-view`'s "Joining shows what is
  being joined, and joins nothing until the user acts" (`:307`) and by
  `Main.qml:122-127`, whose comment says acting on a pasted reference "reaches a
  PREVIEW and joins nothing".
- `:1539-1541` (the policy layer) is the load-bearing rewrite and it holds. Each
  of the three named instances resolves to a live view-capability requirement:
  "what a publish may claim" → `composer-view:294`; "a storage failure never
  renders as an empty result" → `stoa-navigation-view:69` ("Holding no Stoas and
  failing to read membership are different screens"); "an address accompanies
  every generated name" → `view-identity-onboarding:394` and
  `stoa-navigation-view:12`. The phrase "the view capabilities carry" is exact —
  there are precisely three view capabilities in `openspec/specs/` and all three
  are represented.
- `:1451` (the two interface notes) — the replacement reason, "no surface exists
  yet for them to be contracted against", is the true one and is better than what
  it replaced.
- `:2844` (the feed's `restored` gap) — the rewrite drops a claim about a
  document rather than making a new one, and the surrounding statement about
  `restored` being inexpressible in the feed is unchanged.

**§2.1's three spec mappings all resolve**, verified by reading the targets:
`op-transport/spec.md:242`, `view-identity-onboarding/spec.md:394`, and
`module-wire-contract/spec.md:268` ("Failure is always the error shape, and never
a partial success"). And the claim that grounds §2.1 — `grep -rn "UI-BRIEF\|UI
brief"` over `openspec/specs/` returns nothing — is true, which is what makes
deleting the brief safe for the requirement set.

**`ScreenFrame.qml`'s moved contract is accurate about the code it now sits in.**
All four header clauses check out against `Main.qml` and against the file's own
body: `Main.qml:104` reads `pane.implicitHeight` for `contentHeight` (clause 1);
`Main.qml:119/143/178` give each screen `Layout.alignment` and
`Layout.preferredWidth` and **no height** (clause 2); the zero-measurement
behind clause 3 is at `ScreenFrame.qml:53-54` and pinned by
`tst_screen_frame_geometry.qml:183`. The header's closing promise — "the numbers
behind each clause are in the comments below" — is true.

Note for the record, since it looks like a loss and is not: the brief's **fifth**
clause (an explicit-height card *does* give a `fillHeight` child real slack —
484 of 600 — and the trailing-`Item` spacer trap) is absent from the new header
but survives intact at `ScreenFrame.qml:82-90`, with its own measurements (156
vs 176). Nothing from that section of the brief was dropped.

**The two assertion-message rewordings changed no condition.**
`tst_screen_frame_geometry.qml:183` still `compare(filler.height, 0, ...)` with
only the message string altered, and `tst_feed_copy.qml` likewise. §2.5's claim
that no assertion, fixture or expected value was touched is correct — the diff
shows only comment and string-literal lines in every test file.

**The rebase repair at `feed.rs:138` and `names.rs:37` (commit `4d1add6`) is
correct.** Both citations it introduces resolve on this branch:
`generated-names/spec.md:586` is headed exactly "A name is never unique, never an
identifier, and never numbered", quoted verbatim; and `names.rs`'s internal
pointer to *What a name is NOT* resolves to `names.rs:50`, whose content
("Not unique, not an identifier, and never numbered", the `#2` numbering
argument) supports what `names.rs:37` now claims of it. The survey grep
`grep -rn "UI-BRIEF\|UI brief\|ui-brief\|UI_BRIEF"` over `docs/`, `dialectica/`,
`dialectica-ui/`, `CLAUDE.md` and `openspec/specs/` returns **nothing** at
`4d1add6`, so design.md §1's re-run instruction now holds (it did **not** hold at
`9fc8c6a`, where those two lines still stood).

**The remaining source and test comment repairs are faithful.** `Core.qml:41`,
`FeedScreen.qml:8/394/652`, `tst_feed_states.qml`, `tst_feed_extent_claim.qml`,
`sanitise.rs`, `wire.rs:509` and `end_to_end.rs` each replace an ordinal
reference with the rule it depended on, and in each case the stated rule matches
what the surrounding code does. Two are improvements on what they replaced:
`end_to_end.rs:2344` now attaches the *cite a requirement by its heading, never
by an ordinal* lesson to the surviving `module-wire-contract` requirement rather
than to a deleted file, and `sanitise.rs:20` stops attributing its governing rule
to a `SPEC.md` that never existed. `IDENTICON.md`'s rewrite reconstructs the
brief's obligation-6 argument (pigeonhole collision, grinding, "the address is
the identity") accurately and cites `view-identity-onboarding`'s requirement,
which resolves.

**The obligations the dispatch flagged as brief-only are visibly accounted for,
not silently dropped.** Obligations 3 (vouching never shown), 7 (master key is
not a complete backup), 8 (unencrypted key state) and the score floor-at-zero go
with the brief under the owner's §2.1 decision; the annotation-column ban is a
separate matter and survives independently as `composer-view:342`'s scenario
"The denial survives without the annotation column". §2.1 states the decision
and the check that made it safe, so the loss is recorded rather than silent —
which is what the dispatch asked be visible in `design.md`.

**Out of scope as instructed and confirmed untouched:** the ~70 archive
citations (`git diff --stat` shows no `openspec/changes/archive/` file changed)
and the pre-existing `SPEC.md` defect beyond the count above.

---

**Tree state:** I worked in `.claude/worktrees/review-drop-ui-brief-correctness`
on branch `review/drop-ui-brief/correctness` and mutated nothing in the piece
worktree except this findings file and my own `tasks.md` row. The review worktree
is removed. No reviewer branch was pushed.
