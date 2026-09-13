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

- [x] **`dev-writer`** — `end_to_end.rs:865`, `end_to_end.rs:1962` and
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

      **Fixed**, and the finding is confirmed in every particular. I re-ran the
      greps rather than taking them on trust: `grep -rn "indistinguishable"` over
      `docs/` returns only two `IDENTICON.md` lines, neither related, so the
      quoted phrase exists nowhere in the documentation.

      **The substance is real and is written down — in `docs/UI-BRIEF.md`, not in
      PLAN.md.** Under "Non-negotiable rendering obligations" the fifth entry is
      *"Distinguish an empty result from a failed one — a storage failure must
      never render as an empty feed. An empty feed and 'we could not read the
      store' look identical and mean opposite things."* That is a **stronger**
      source than the phantom, because it names the storage-failure case the wire
      test actually constructs, where the fabricated paraphrase spoke only of a
      Stoa nobody had posted in. Both citation sites now point there; the first
      site states the obligation in one clause instead, since a fixture-level
      comment does not need the reference at all.

      **Cited by name and not by number.** UI-BRIEF's obligation list contains a
      `2b`, so its numbering is not stable enough to cite — the same decay the
      README rule is about. (A concurrent `tester` independently fixed the second
      site while I was working and did cite "rendering obligation 5" by number;
      its text is on disk and I left it alone rather than edit a file another
      writer holds. Worth one follow-up word, noted in my report.)

      **On the question the brief asked — does "extend rather than rename" survive
      without the citation? Yes, and it never depended on it.** The load-bearing
      evidence is the mutation, not the reference: mutating
      `list_threads_from_request` to return an empty page instead of `error_json`
      kills the wire test **while the `feed::list_threads` test one layer down
      stays green**. That is a measurement of the seam, and it is what proves the
      layer was needed. The citation only ever labelled *why the distinction
      matters to a reader*. `design.md` now says so explicitly, and records the
      relay failure — that the citation came from `findings/architecture.md` and
      was carried onward on trust — beside the decision, so the archive keeps the
      lesson and not just the correction.

      The argument is in fact **stronger** than when the citation propped it up:
      the `spec-test` reviewer measured that hardcoding `"page": 0, "hasMore":
      false` in `wire.rs` left every e2e test green, so extending was right *and*
      the extension was incomplete. Evidence, where the citation was decoration.

## A PLAN.md claim this change made false and did not fix

- [x] **`dev-writer`** — `docs/PLAN.md:3810-3813` (unchanged by this branch:
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

      **Fixed**, taking the first of the two options offered. The sentence now
      reads *"compares cargo's result against a count of `#[test]` attributes
      derived from the Rust tree, so it cannot rot… Read the workflow for which
      paths it walks — naming them here would be a second copy that drifts."*
      The principle the bullet exists to teach (assert against a derived number,
      never a literal) is untouched; only the claim about *which* paths is
      removed, and it is removed rather than updated so the same rot cannot
      recur — CLAUDE.md's self-invalidating rule applied.

      Note the line moved: it is at `docs/PLAN.md:3922-3926` after merging
      `origin/main` (`af21c9f`), not 3810-3813. The finding's `git diff --stat`
      observation still held — this branch had not touched `docs/`.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:720` prints
      `f"#[test] functions in src: {declared}"` while `declared` is now counted from
      the whole tree including `tests/` and `examples/`. **Why it matters:** the
      gate's own log line is the first thing read when the job goes red, and it
      names a directory that is no longer what was measured — so a maintainer
      debugging a mismatch looks in `src/` for a count that came from four places.
      The entire point of the recorded decision is that the misleading-red failure
      mode is what the list was replaced to avoid; this label reintroduces a
      smaller version of it. One word: `in the Rust tree`.

      **Fixed**, and slightly beyond what was asked. The label is now
      `f"#[test] functions in the Rust tree ({tree}): {declared}"` — it interpolates
      `tree`, the same variable the rglob walks, so the log line cannot disagree
      with what was measured even if the root is changed later. A fixed string
      saying "the Rust tree" would have been correct today and rottable; this one
      is derived from the thing it describes, which is the same principle the
      gate's own design rests on.

## Counts in the file that describe it wrongly

These are a suggestion-tier group but they share one cause, which is the reason to
report them together rather than separately: the header describes the mutation table
in prose, the table grew twice, and the prose was not re-derived either time.

- [x] **`dev-writer`** — `end_to_end.rs:109` says *"The **four** notes under the
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

      **Fixed by taking the recommended route — every count removed rather than
      corrected.** All four numbers re-derived by command first, and all four of
      the reviewer's figures are right:

      - `grep -c "^//! \*\*Note [0-9]"` → **5** notes, not four.
      - Four of them changed the file, not three. Notes 1 (added a test), 2
        (reordered assertions), 4 (added a fixture guard) and 5 (moved a guard)
        each did; note 3 is "the load-bearing mutation", a measurement that
        changed nothing — which is why the tally was easy to get wrong.
      - Table rows, counted per group from the source: 134-144 = **11**,
        154-160 = **7**, 169-172 = **4**, 179-180 = **2**, total **24**. So
        "eighteen" was exactly 11+7 and went stale when the last two groups
        landed, as the finding says. (`grep -c "^//! | \`"` returns 19 and is the
        wrong probe — it misses the five rows whose first cell does not open with
        a backtick. Recording that because the plausible one-liner undercounts.)

      The edits: *"The four notes… three of those disagreements"* → *"The notes
      below… most of those disagreements"*, plus an explicit sentence saying no
      count is quoted and why; *"re-running eleven mutations"* → *"re-running the
      whole table"* in both `end_to_end.rs` and `design.md`; and `design.md`'s
      *"eleven (now eighteen) rows"* → *"rows of measured mutations"*. Nothing
      now needs re-deriving when the table grows a third time, which is the point.

- [x] **`dev-writer`** — `end_to_end.rs:114` says **"EACH ROW WAS MEASURED ONCE, on
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

      **Fixed — the recorded decision now applies at all four groups, not one.**
      The header sentence becomes *"EACH ROW WAS MEASURED ONCE, at the commit its
      group heading names"*, so the header and the headings say the same thing.

      Commits derived from `git log`, not assumed.
      `git log --oneline -- <the test file>` gives exactly three commits —
      `9bb2bc1`, `1342aa9`, `f007bcd` — so group 1 keeps `9bb2bc1` and group 2
      becomes **`1342aa9`, the readability and architecture findings**. Groups 3
      and 4 both become **`f007bcd`**: I checked rather than split them across the
      two remaining commits, and
      `git log -S "two_temp_dirs_with_the_same_tag_get_different_unguessable_names"`
      returns `f007bcd` alone, which is the commit that added group 4's subject
      test. Group 4's heading previously named nothing at all and now reads *"Two
      more at the same commit `f007bcd`"*.

      This is the half of the finding that mattered most: "when the review findings
      were addressed" stops resolving the moment `findings/` is deleted, which is
      this piece's next stage. A commit hash survives that, and `design.md` now
      states the reason so the constraint is recorded rather than just obeyed.

## A decision recorded as rejected that the code still argues

- [x] **`dev-writer`** — `end_to_end.rs:296-302`, the doc comment on `FIELD_CAP`,
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

      **Fixed.** The constant's doc now gives the reach argument and names the two
      tests that make the strength argument false, mirroring the module header
      rather than contradicting it:

      > **Hardcoded because an integration test cannot see a private `const` at
      > all.** That is the whole reason, and it is enough of one — the answer is
      > about **reach**, not about strength.
      >
      > It is specifically NOT that this file is the only guard against a drifted
      > cap. `op.rs::the_field_cap_is_pinned_to_a_known_answer` asserts
      > `MAX_FIELD_LEN == 150 * 1024` and `stoa.rs` asserts
      > `MAX_TITLE_BYTES == 1024`…

      The "do not update this to match the code" paragraph is kept — it is the
      conclusion, which `design.md` records as surviving. Only the rejected
      premise is gone.

      `TITLE_CAP` needed no edit: "Hardcoded for the reason `FIELD_CAP` is"
      inherits by reference, so fixing the referent fixed it. That is worth noting
      as the reason the by-reference form was the right shape — one edit, two
      surfaces, and they cannot drift apart.

      Confirmed the reviewer's verification independently: both pinning tests exist
      and are hardcoded, and both pass in the run below.

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
