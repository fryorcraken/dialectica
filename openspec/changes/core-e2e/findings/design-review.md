# Design review — `core-e2e`

**`design.md` exists** and is unusually good for a test-only piece: nine Decisions
entries, most of them carrying all four parts a good entry needs — what was chosen,
the constraint, the alternatives with what ruled each out, and the cost. Three
entries (`store_at` parameterisation, the 16-byte shared prefix, `skip_specs` +
`schema`) are the best-shaped in the change folder: each names a rejected
alternative and a measurement rather than an assertion. The dispatch brief said
there is no `design.md`; that is wrong, and the usual first question therefore
applies normally.

Verified independently, all holding: PLAN.md from `origin/main` does **not** mention
the `canonical_bytes`/`decode` asymmetry (`grep -n -i
"canonical_bytes\|MAX_FIELD_LEN\|153600\|poison\|asymmetr"` over the `origin/main`
copy finds only §4.4's 150 KiB cap and unrelated mutex poisoning); the repository
has **no issues at all** (`gh issue list --state all --repo fryorcraken/dialectica`
returns nothing); `openspec validate --strict core-e2e` passes and reports
`skip_specs` honoured; the suite is **531 passed, 0 failed** (506 + 25), matching
the stated baseline; `9bb2bc1` is genuinely the commit that added the file; the
`target/` exclusion is measured correct (one `target/` in the tree); and the
mutation table's four group counts (11 / 7 / 4 / 2) are each accurate against the
rows. The `§9.1` quotation at `end_to_end.rs:1088` is real — its text is at
PLAN.md:3349, inside §9.1's "What a thread view is".

The findings below are one serious contradiction inherited from a reviewer's own
citation, one PLAN.md claim this change invalidated and did not fix, and three
stale self-describing counts of the kind this file has already been corrected for
twice.

## The citation this piece propagated without checking

- [ ] **`dev-writer`** — `end_to_end.rs:865`, `end_to_end.rs:1962` and
      `design.md:47` cite **`§11.1 obligation 5`, a section that does not exist in
      `origin/main`'s PLAN.md**, and attribute to it a quoted phrase that appears
      nowhere in the file. PLAN.md:3196-3204 says so explicitly: *"§11.1 'Rendering
      obligations, collected' arrives with the `vouching-state` change and is not in
      this file until that lands… If §11.1 is absent when you read this, that change
      has not merged yet."* It is absent. The one list feeding that future section
      is at PLAN.md:1588-1617, it is about **names**, and its fifth entry is "Never
      present the recognition layers as adding up to a defence" — not anything about
      empty feeds. The quoted string *"an empty feed is indistinguishable from a
      Stoa nobody has posted in"* returns **zero** hits from `grep -rn
      "indistinguishable"` over PLAN.md (both copies) and all of `docs/`.
      **Verified:** `grep -n "^### \|^#### " ` over the `origin/main` copy shows §11
      ("Traps, collected", line 3837) has no subsections at all.
      **Why it matters:** this is the same defect `tasks.md` 4.2 already closed once
      on this file — "replace the fabricated ordinal citation" — reintroduced at a
      different citation, with a fabricated *quotation* on top of the fabricated
      ordinal. It is load-bearing rather than decorative: `design.md:47-51` rests
      the whole "extend rather than rename" decision on this obligation being an
      obligation about what a reader sees, so the best-argued entry in the file is
      argued from a source that does not exist. A reader would conclude PLAN.md
      contracts something it does not, and would go looking for a numbered list to
      check obligation 5 against. Note the provenance: the citation originates in
      `findings/architecture.md:209,245` and was carried into the code and into
      `design.md` on trust — a reviewer's citation is not a verified one.
      The substance survives; only the attribution is false. Either cite
      `§9.1 #### 1/#### 2` and `§2.5` (which do carry the error-shape exclusivity),
      or state the obligation in one clause and drop the section number —
      `.claude/agents/README.md`'s "a spec must never cite a PLAN section number"
      exists for exactly this decay, and a test comment inherits the reason.

## A PLAN.md claim this change made false and did not fix

- [ ] **`dev-writer`** — `docs/PLAN.md:3810-3813` (unchanged by this branch:
      `git diff --stat origin/main...HEAD -- docs/` is empty) still says *"The
      test-count check compares cargo's result against the count of `#[test]`
      attributes **in `src/`**, so it cannot rot."* After this change the gate walks
      `dialectica/rust-lib` entire — `src/`, `tests/`, `examples/` and any future
      `benches/`. **Why it matters:** the §10 sentence is not a status line but the
      *reasoning* for the gate's shape ("assert against a derived number, never a
      literal"), and it now describes a narrower gate than the one that ships. This
      is question 4 of the design review with the answer "no": the reasoning this
      change acted on is still in PLAN.md, in a now-wrong form, while
      `design.md`'s "One rglob, not a list of source roots" holds the current
      version — two copies, and the wrong one is the one a reader reaches from
      CLAUDE.md's "read PLAN.md before any design decision". Either narrow the PLAN
      sentence to a line saying the gate exists and derives its number, or strike
      the `src/` phrasing; the argument belongs in `design.md`, which already has it.

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:720` prints
      `f"#[test] functions in src: {declared}"` while `declared` is now counted from
      the whole tree including `tests/` and `examples/`. **Why it matters:** the
      gate's own log line is the first thing read when the job goes red, and it
      names a directory that is no longer what was measured — so a maintainer
      debugging a mismatch looks in `src/` for a count that came from four places.
      The entire point of the recorded decision is that the misleading-red failure
      mode is what the list was replaced to avoid; this label reintroduces a
      smaller version of it. One word: `in the Rust tree`.

## Counts in the file that describe it wrongly

These are a suggestion-tier group but they share one cause, which is the reason to
report them together rather than separately: the header describes the mutation table
in prose, the table grew twice, and the prose was not re-derived either time.

- [ ] **`dev-writer`** — `end_to_end.rs:109` says *"The **four** notes under the
      table"*; there are **five** (`grep -c "^//! \*\*Note [0-9]"` → 5 — note 5 was
      added by `f007bcd`, the branch tip). The same sentence says *"three of those
      disagreements changed this file"*; four did — note 1 added a test, note 2
      reordered assertions, note 4 added a fixture guard, note 5 moved a guard.
      `end_to_end.rs:123` and `design.md:177` both say *"re-running **eleven**
      mutations"* and `design.md:169` says the table is *"eleven (now **eighteen**)
      rows"*; it is **24** (11 + 7 + 4 + 2, each group heading's own count verified
      accurate). "Eighteen" was true at 11+7 and went stale when the last two groups
      landed. **Why it matters:** this file's own recorded dead end says *"get a
      count, ordinal or duration from a command before writing it"* and names
      quantities as the worst case *"because a quantity reads as though someone
      measured it"* — and these are the counts describing the table that exists to
      prove the tests were watched failing. A reader who counts the notes and gets a
      different number than the header does has no way to tell which other header
      claim was re-derived. The self-invalidating fix CLAUDE.md asks for is to stop
      quoting the total at all: "the notes below" and "re-running the table" say the
      same thing and cannot rot.

- [ ] **`dev-writer`** — `end_to_end.rs:114` says **"EACH ROW WAS MEASURED ONCE, on
      the date its group says"**, and **no group states a date.** Group 1 names a
      commit (`9bb2bc1`, verified as the adding commit); groups 2 and 3 name a review
      round ("when the review findings were addressed"); group 4 names nothing at
      all. `design.md:173` records the decision as *"grouped by when each group was
      measured, **with the commit named**"* — so the recorded decision is applied at
      one of four groups. **Why it matters:** this is the partial-application shape
      the design review exists to catch, and the cost is exactly what the decision
      was made to prevent: "when the review findings were addressed" is not
      resolvable to a tree once `findings/` is deleted before merge, which is the
      next step in this piece's own stage block. A row whose measurement cannot be
      located is the "reads as though somebody checked" case the same paragraph
      warns about. `git log` gives the two commits (`1342aa9`, `f007bcd`) that the
      two rounds correspond to.

## A decision recorded as rejected that the code still argues

- [ ] **`dev-writer`** — `end_to_end.rs:296-302`, the doc comment on `FIELD_CAP`,
      still makes the argument `design.md:24-30` records as **rejected**: *"importing
      it would be worse if it were not: a test phrased in terms of the constant moves
      with the constant, so a cap that drifted upward would still pass. This number
      is the requirement."* `design.md` calls that premise false for this crate and
      says *"The conclusion survived; the argument did not."* The module header was
      fixed (lines 51-65 give the reach argument and explicitly disclaim the strength
      one), but the constant's own doc — the thing a reader hovering `FIELD_CAP`
      sees — was not. **Verified:** both pinning tests exist and are hardcoded —
      `op.rs:1650` `assert_eq!(MAX_FIELD_LEN, 150 * 1024)` in
      `the_field_cap_is_pinned_to_a_known_answer`, and `stoa.rs:940`
      `assert_eq!(MAX_TITLE_BYTES, 1024, "the title bound is network-visible")`.
      **Why it matters:** `design.md` says explicitly why the distinction was worth
      recording — *"the answer is about **reach**, not about strength"* — and the
      constant's doc is where the next person asking whether to import a constant
      into a test will actually look. Two surfaces in one file now give opposite
      answers, with the one design.md rejected sitting closer to the code. This is
      the serious tier: the code contradicts a recorded decision, at the one site
      the decision was recorded to govern. The `TITLE_CAP` doc at line 305
      inherits it by reference ("Hardcoded for the reason `FIELD_CAP` is").

## Not findings — checked and sound

Recorded so the next reviewer does not re-litigate them.

**The over-cap defect entry is the right call and is honestly recorded.** Asserting
the defect as it stands rather than the fix is argued from the constraint (a test
asserting the fix would fail on `main`), the alternative is named, and the cost is
stated in the strongest available terms — `end_to_end.rs:270-277` says in capitals
that the test is the sole record, names the two search terms, and tells whoever
files the issue to replace the paragraph. The at-cap half genuinely makes it a
fencepost rather than an absurd-value test, and the panic message names the
replacement. That the repository has no issue tracking it is a real gap, but it is
a *recorded* gap in both `design.md` and `proposal.md`, which is what this review
asks for; the box belongs on whoever files it, not on this piece.

**Note 5's reasoning is sound, and its refusal to generalise is the valuable half.**
I checked the claim that "move every guard to the end" is the wrong rule. It holds:
`Moderators::authorises` (`moderation.rs:206-216`) conjoins three terms —
`stoa` scope, `contains`, and `verify()` — so a defect in the scope or verify term
leaves `contains` correct, the substantive assertions in
`a_hide_by_a_non_moderator_leaves_the_thread_visible` fire, and the retained guard
at line 1191 is the message that distinguishes them. The `iter_target` guard at
`end_to_end.rs:1264` is correctly left in front of its assertion for the stated
reason. The rule the note draws — *ask which mutation would trip this guard, and
whether that is the mutation the test is aimed at* — is true and will stay true,
because it is about a relationship rather than about a count.

**The rglob is the right shape.** It trades a staleness that fails misleadingly
(a list a new target escapes, reported as cfg-gating) for one that fails loudly
(`tree.is_dir()` plus `declared == 0` plus the mismatch), and the `examples/`
consequence is stated as the real decision rather than left incidental. The
`"target" in p.parts` exclusion is broader than `target/` at the root — it would
also skip a legitimate directory named `target` — but that is a hypothetical this
tree does not have, and the entry's measurement (one `target/`, confirmed) plus the
`ran != declared` mismatch means the failure would be loud rather than silent.
Not worth a box.

**The sectioning rule is worth its entry.** Organising by boundary crossed rather
than capability, with three applicable rules and the explicit ban on counts in
headings, is a decision with a real alternative that a later author can apply
without guessing. It is also the entry most likely to survive, because it states
a rule rather than a measurement.
