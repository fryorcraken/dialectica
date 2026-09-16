# Readability findings — `op-clock`

Reviewed dimension: **readability only**. Correctness, security and architecture
are held by other instances.

Reviewed at `880bb2b`, three-dot against `origin/main` (`9357417`), in a
throwaway worktree that has since been removed.

## What is clean, in prose

**The central distinction is stated where someone would violate it, not only in
`design.md`.** It leads `op.rs`'s module header (lines 30-74), it is on the
`OpClock::asserted_ms` field itself (`op.rs:535-559`), it is on the wire
serialiser beside the JSON keys (`wire.rs:1848-1865`), it is in the SQLite
schema at the one reserved column a decay would reach for
(`log/sqlite.rs:460-501`), and it reaches the external designer who cannot read
the code (`docs/UI-BRIEF.md:559-566`). `asserted_time.rs`'s module header is the
best single piece of prose in the change: it argues the *shape* rather than the
rule, names clamping as defence in depth rather than the defence, and records
that the reading peer's own clock is untrusted too. A future author reaching for
the asserted time in a comparison would have to pass four separate warnings.

**The counter-advance bound is legible and will survive a "simplification".**
`arrival.rs:227-241` heads the argument `THE COUNTERS ARE SORTED FIRST, AND THAT
IS THE WHOLE CORRECTNESS ARGUMENT`, then gives the concrete two-peer divergence
the arrival-order version produces. `clock_from_counters`'s body carries the
matching inline reasons, including why the subtraction is written
`counter - clock` rather than `clock + ADVANCE_BOUND` (overflow → panic → module
abort). The spec states the same requirement independently at
`specs/op-ordering/spec.md:83-85`.

**Test names and failure messages in the new QML suite are precise and
non-vacuous.** `tst_composer.qml:820-843` states before the tests are read what
they can and cannot show — that a "tap twice, count one call" test would be
measuring the single-threaded event loop and is deliberately absent. The five
new tests assert what the component does, and `test_the_control_is_absent_while_
a_publish_is_outstanding` makes its observation from inside the fake bridge,
which is the only instant at which the flag is true.

**`docs/UI-BRIEF.md` was kept true in the same change**, including the clamped
state and the no-time-shown state the design needs.

**`DComposer.qml`'s guard comment is honest about buying nothing today** and says
why it is still worth writing. Its claim that `applyReply` "has five exits" is
accurate (lines 272, 281, 302, 312, 316).

## Findings

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/feed.rs:5-8`
      and `:67-69` — the feed's module header still argues the case this change
      overturns, and was missed by the retraction sweep
      **Scenario:** `tasks.md:76-83` lists four files carrying "written arguments
      against exactly what this change builds" and requires each be retracted or
      cited. `op.rs`, `arrival.rs`, `transport.rs` and `log/sqlite.rs` were all
      duly rewritten. `feed.rs` was not on that list and was not touched — its
      diff against `origin/main` contains only mechanical `clock: None` additions
      to test fixtures. It still reads: *"both are defined in terms of a Lamport
      timestamp that **does not reach us** — so both would today fall back to
      ascending op id, 'a hash [that] carries no recency whatever'"* (5-8), and
      *"whether the count is worth its cost **before Lamport values arrive**"* /
      *"an ordering that carries no recency"* (67-69). A counter now reaches us,
      inside the op, and `feed.rs` pages `cmp_ops` order which leads with it. A
      reader arriving at this file — the module that *names* the ordering
      `convergent` precisely because it carried no recency — is told the opposite
      of what the code now does, and would reasonably conclude the naming
      decision is stale and open for change.
      **Measured:** the author knew the reasoning had moved —
      `tasks.md:112` records "§9.1 §8's feed-label bullet is resolved by the
      field landing" and `docs/PLAN.md` and `docs/UI-BRIEF.md:578-585` were both
      updated for it. Only the code comment was left behind. Severity: genuine
      defect, and the highest-value one here, because this is the exact
      "four files argue against this change" trap the piece's own task list
      opened — with a fifth file the list missed.

      **Fixed** in `8825ddc`. See the architecture reviewer's box on the same
      file for the substance; in short the `convergent` name survives and is
      re-argued from "a counter is causal, not temporal", with the wall-clock
      named as explicitly not the substitute.

      Worth recording that this finding, the architecture reviewer's and the
      design reviewer's each named a *different* fifth site, each believing it
      was the one the sweep missed. The list was therefore rebuilt by grepping
      the claim's several phrasings rather than from any of the three, which
      found sites in `wire.rs`, `revision.rs`, `thread.rs`, `authoring.rs`, two
      more in `log/mod.rs` and `moderation.rs` than were reported, and one in
      `FeedScreen.qml` that is **rendered on screen** with a test pinning it
      (`9ec147a`). The note about the `docs/UI-BRIEF.md` line in this file's
      "what is clean" section is moot: that file was deleted by #83 before this
      branch rebased onto it.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/moderation.rs:425-426`
      — `resolve`'s convergence argument still cites "recorded arrivals", which
      this change removed from every ordering decision
      **Scenario:** the paragraph reads *"The bias is a pure function of the two
      ops' actions and **their recorded arrivals**, so every peer holding the same
      ops computes the same answer."* `resolve` (430-494) reads `e.op.op.kind`,
      `moderators.authorises(e)` and `first.op.op.clock.is_some()` — never an
      `Arrival`, and `cmp_ops` can no longer name one at all. The sentence is
      left over from the pre-change version and sits inside a doc block the
      change otherwise rewrote at length (370-406, the falsified two-op premise).
      A reader checking the convergence claim goes looking for the arrival the
      comparison depends on, finds none, and cannot tell whether the comment or
      the code is the stale half. Worse, "recorded arrivals" is precisely the
      per-peer value whose use would *break* convergence — so the sentence
      currently argues for convergence by citing the one input that would destroy
      it. Severity: genuine defect; the surrounding prose is otherwise the most
      carefully updated in the change, which makes the one stale clause more
      misleading, not less.

      **Fixed** in `8825ddc`, and the sentence is rewritten around this
      finding's sharpest observation rather than merely corrected. It now says
      the bias is a function of the candidates' actions and **their own signed
      clocks**, then states what is deliberately *not* an input and why: a
      recorded arrival is per-peer, so citing one would argue for convergence
      from the one value that would destroy it. Naming the excluded input is
      what stops the next author restoring it.

      Two further sites in the same file, not reported and equally stale, are
      included: the module header at 78-80 and 95-98 described `resolve` as
      reading the leading candidate's `Arrival` and branching on it, which is
      the pre-clock mechanism — `resolve` reads `first.op.op.clock.is_some()`
      and `cmp_ops` has no `Arrival` to name. That section is the one this file
      says "the author of a fourth resolver reads to learn the house
      discipline", so it mattered more than the `resolve` doc itself.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/thread.rs:225-238`
      — `Placed`'s doc claims a compiler-enforced property the type does not have
      **Scenario:** the doc says the obvious shape *"is to build a `ThreadItem`
      with an empty `position` and fill it in afterwards, and that shape has a
      hole"*, then claims *"A type that **cannot express a position** removes the
      possibility"* and *"'every item has a real position' is the compiler's to
      enforce rather than a loop's to remember."* But `Placed` is
      `struct Placed { item: ThreadItem }` — it wholly contains a `ThreadItem`,
      and `resolve_item` sets `position: String::new()` at `thread.rs:791`, which
      is the exact `""` placeholder the doc says was rejected. The property is
      real but its mechanism is privacy, not expressiveness: `Placed` and its
      field are module-private, so `Placed::at` is the only route *out of this
      module*. `ThreadItem`'s fields are `pub`, so any consumer crate can
      construct one with any `position` it likes.
      **Why a reader is misled:** someone hardening this later reads "the
      compiler enforces it", does not re-derive the argument, and moves `Placed`
      or widens its visibility believing the guarantee travels with the type. It
      does not. Severity: genuine defect — the claim is the strongest kind a
      comment can make (CLAUDE.md's "complexity in the data structure") and the
      code earns a weaker one.

      **Fixed** in `05a6faf` — and by changing the code to earn the claim
      rather than by weakening the comment, because the claim was the one worth
      having. `Placed` now holds the fields instead of a built `ThreadItem`, so
      `Placed::at` is the only code naming the `ThreadItem { .. }` literal and
      `resolve_item` has no `position` to set.

      This finding's diagnosis of the *mechanism* — privacy, not
      expressiveness — is what made the fix obvious, and the doc now says so in
      those terms, including that wrapping a built item would have required
      constructing the `""` placeholder the type exists to rule out. The
      finding's observation that `ThreadItem`'s fields are `pub` so any consumer
      crate can construct one with any position it likes remains true and is
      **not** claimed against: the doc's scope is this module's construction
      path, which is where the hole was.

- [x] **`dev-writer`** — `openspec/changes/op-clock/design.md:86` — a citation
      attributed to the spec that the spec does not contain
      **Scenario:** decision 3 reads *"Ascending order is what makes this a
      function of the op set rather than of arrival order, and it is the subtlety
      **the spec says it had to fix mid-draft**."* `specs/op-ordering/spec.md`
      states the requirement thoroughly at lines 83-85 — including the same
      two-peer divergence example — but says nothing about having fixed anything
      mid-draft, and the string "mid-draft" appears nowhere in the change's specs.
      A reader who goes to the spec for the drafting history this sentence
      promises finds no such account and cannot tell whether they are looking at
      the wrong document or the sentence is wrong.
      **Measured:** `grep -rn "mid-draft"` over `openspec/changes/op-clock/`
      returns exactly one hit, this line. Severity: genuine defect, and squarely
      the project's recorded "persuasive citations get fabricated" trap — the
      claim is the kind a reviewer accepts because it is plausible and specific.
      The drafting admission that *does* exist and is worth keeping visible is
      `proposal.md:94-95` on `openspec validate --strict` passing a suite that
      contradicts itself; decision 3 should point there or drop the attribution.

      **Fixed.** The attribution is replaced with what the spec does say,
      quoted and line-cited: `op-ordering/spec.md:83`'s "The excess SHALL be
      measured against a value computed from the ops the peer holds, and SHALL
      NOT be measured against whatever the peer's clock happened to be at the
      instant the op arrived", plus `:85`'s reason that the arrival-order
      version "is the one that falls out of writing the check on the receive
      path".

      That is a stronger sentence than the one it replaces — it cites a
      requirement rather than a drafting anecdote — so the fix is neither of the
      two this finding offers. The `proposal.md:94-95` pointer is not added,
      because that admission is about `validate --strict`'s blind spot and not
      about the fold, and attaching it here would be a second loose citation in
      the place the first one was.

      Recorded because this is the project's "persuasive citations get
      fabricated" family and the propagation is visible: the correctness
      reviewer repeated the same phrase ("the subtlety the spec records as a
      mid-draft fix") in their own clean-findings prose, having read it here
      rather than in the spec.

- [x] **`dev-writer`** — `openspec/changes/op-clock/tasks.md:20` — the stage row
      undercounts `design.md`'s decisions, and the three it hides include the
      store-layout trap
      **Scenario:** the row says *"`design.md` carries **the six decisions** and
      the constants' reasoning"*. `design.md` has nine (`grep -n "^### "` returns
      headings 1 through 9). A reader trusting the row stops at decision 6
      ("Clamping is read-time") and never reaches decision 7 (the publish path),
      decision 8 (`score_epoch` and the newly-available wrong answer) or
      decision 9 (`LAYOUT_VERSION` and the `check_layout` defect). Decisions 8
      and 9 are the two most consequential records in the document.
      **Measured:** nine `###` headings; `tasks.md` says six at line 20 and "the
      six answers" at line 11. Severity: genuine defect — a pointer that
      undercounts is worse than no pointer, because it reads as complete.

      **Fixed**, by removing the number rather than correcting it: the row now
      says "the decisions (`grep -n "^### " design.md` counts them)". The repo's
      own rule decides this — do not write down what a command can answer — and
      it is not pedantry here, because three more decisions landed in this same
      findings pass, so "nine" would have been wrong again within the hour.

      Line 11's "the six answers" is left as it stands: it counts the owner's
      six *questions* in `proposal.md`, a different and still-correct number.
      This finding reads the two as one claim; they are not.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs:265-296`
      — `check_layout`'s own doc never says that its column list must be updated
      when a column is dropped, which is the defect the author hit
      **Scenario:** `design.md:193-206` records that `check_layout` still named
      the four deleted sort columns, that the persistence tests caught it, and
      that in production it *"would have made every store permanently unopenable,
      with no migration path by design"*. That is the most expensive near-miss in
      the piece, and the review brief asks whether it is recorded where it would
      prevent recurrence. At the code it is not. `check_layout`'s doc explains
      *why* it names columns (278-286) and that it must not become a migration
      (291-296), but never states the converse obligation: **this list is part of
      the layout, and a column removed from `CREATE TABLE` must be removed here
      in the same edit.** The nearest statement is in a test comment
      (`sqlite.rs:1027-1031`), which says such a mistake "would refuse every real
      store" — true, but materially weaker than "permanently unopenable, no
      migration path", and a reader editing the schema is not reading the tests.
      The `create_schema` doc carries an analogous durable warning for the pragma
      ordering (363-377, *"A later 'add a migration' refactor is the change this
      warning is addressed to"*), which is exactly the shape this one is missing.
      **Why it recurs:** as `design.md` says, the failure is invisible in a schema
      diff — the `CREATE TABLE` and the check are ~200 lines apart and nothing
      ties them together. The next person to drop a column has no reason to open
      `check_layout`. Severity: genuine defect against this project's own
      standard that a comment records what a command cannot tell you; the piece
      paid for this lesson and did not bank it at the point of use.

      **Fixed** in `6156082`, in the `create_schema` shape this finding names as
      the model. `check_layout` now carries a heading stating the converse
      obligation directly — this list is part of the layout, a column dropped
      from `CREATE TABLE` must be dropped here in the same edit — addressed to
      the next schema editor, with the consequence at full strength
      ("permanently unopenable, with no migration path by design") rather than
      the test comment's weaker "would refuse every real store", and with the
      reason it recurs: the two sites are hundreds of lines apart and nothing
      ties them together.

      The observation that the test comment is the wrong home is why the warning
      went to the function rather than being strengthened where it was — a
      reader editing the schema is not reading the tests. The test is named in
      the new text as what catches it, with the caveat that a test catches it
      only after someone has written it.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/authoring.rs:204`
      — `now_ms` names two different clocks across the crate, which is the one
      naming collision this change's own rules forbid
      **Scenario:** `Authorship::now_ms` is the **author's** wall clock, signed
      into the preimage as their assertion. Every other `now_ms` in the crate is
      the **reading peer's** clock, used only to clamp for display:
      `asserted_time.rs:108`, `thread.rs:473`, `thread.rs:714`, `wire.rs:1630`,
      `wire.rs:1655`, `wire.rs:1736`. Six sites mean one thing; one means the
      other. The two are the opposite ends of the mechanism — one is
      attacker-chosen data entering the signed bytes, the other is trusted-ish
      local state that decides only rendering — and the review criterion for this
      piece is that every name says which of the two it is.
      **Why a reader is misled:** someone adding a field to `Authorship`, or
      threading a clock through a new publish path, reads `now_ms` and reasonably
      assumes it is the same `now_ms` they last saw in `wire.rs` — the reader's
      clock. The doc comment at 199-203 disambiguates for anyone who reads it;
      the *name* does not, and a name is what gets read at a call site. The test
      fixtures make it worse by spelling both roles `A_TIME`
      (`authoring.rs:530`, `thread.rs:807`).
      Severity: genuine defect, lower than the others — nothing is currently
      wrong, and the fix is a rename (`asserted_ms` at the authoring end, matching
      `OpClock::asserted_ms` which it is assigned to verbatim at
      `authoring.rs:235`). Recorded because this change's entire thesis is that
      the two clocks must never be confusable, and this is the one place in the
      crate where their names are identical.

      **Fixed** in `6156082`, taking the rename this finding proposes:
      `Authorship::now_ms` is now `asserted_ms`, matching
      `OpClock::asserted_ms`, which it is assigned to verbatim. The compiler
      named all six call sites, including two — `examples/seed_store.rs` and
      `tests/end_to_end.rs` — outside the set this finding enumerates, which is
      the rename doing what a doc comment could not.

      The field's doc now says *why* it is not `now_ms`, in this finding's own
      terms: every other such value in the crate is the reading peer's clock,
      this one enters the signed preimage, and on every peer but the writer it
      is attacker-chosen data. That belongs on the field because the next person
      to add one reads it there.

      **Not done here:** the `A_TIME` fixtures spelling both roles the same way.
      That is a test-fixture change and the `tester` holds this change's fixture
      findings, so it is left rather than edited underneath them — recorded so
      it is not read as overlooked.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/arrival.rs:827-841`
      — two tests with different names carry a byte-identical assertion, and the
      comment claiming they bracket the boundary is false
      **Scenario:** `a_counter_within_the_bound_advances_the_clock` (828) and
      `a_counter_exactly_at_the_bound_advances_the_clock` (833) both assert
      `clock_from_counters([ADVANCE_BOUND]) == ADVANCE_BOUND` — same input, same
      expectation. The second's comment says *"The inclusive edge, **from below**.
      Together with the test after it this pins WHERE the boundary is, which a
      test at a far-away value cannot."* But the two tests it pins between are
      `ADVANCE_BOUND` and `ADVANCE_BOUND + 1` — that is (at, one-past), not
      (below, above). **Nothing in the file exercises a counter strictly below the
      bound.**
      **Why a reader is misled:** someone changing `clock_from_counters`'s
      comparison reads three test names covering within / at / past and believes
      the interior of the accepting range is pinned. It is not; only the edge is,
      twice. The `within` test also cannot fail for the reason its name gives.
      Severity: genuine defect — this is the project's recorded "pinned literals
      can't enforce distinguishability" shape, and the surrounding tests
      (`a_pair_whose_counter_and_id_disagree`) show the author knows the pattern.

      **Fixed.** The three functions collapse into one table,
      `the_advance_bound_accepts_up_to_and_including_itself_and_nothing_past_it`,
      with four rows: `ADVANCE_BOUND / 2` (strictly inside — the case that did not
      exist), `ADVANCE_BOUND - 1`, `ADVANCE_BOUND`, `ADVANCE_BOUND + 1`. A table
      rather than four functions for CLAUDE.md's reason, and because three
      near-identical functions is what let two of them drift into one.

      **Two mutations, chosen so the new row is proved to carry its own weight
      rather than to ride along:**
      - `counter - clock <= ADVANCE_BOUND` → `<`: **fails** on the edge row.
      - `clock = counter` → `clock = counter.max(ADVANCE_BOUND)`: **fails on the
        strictly-inside row FIRST** (got 1000000, wanted 500000), which is a
        mutation the (at, one-past) pair alone could not see. That is the
        measurement that makes the new row load-bearing rather than decorative.
      Both restored.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs:1585-1601`
      — the test the comment credits with catching the sentinel bug can no longer
      catch it, and its loop values are inert
      **Scenario:** `an_op_carrying_a_counter_sorts_ahead_of_one_carrying_none_at_
      every_counter` loops over `[0, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX]` and
      asserts `with.has_counter < without.has_counter`. `SortKey::of`
      (`sqlite.rs:156-171`) sets `has_counter` to 0 or 1 from `clock.is_some()`
      **without reading `clock.counter` at all** — the loop variable reaches only
      the `.counter` field, which this assertion never touches. Deleting four of
      the five values, or replacing them all with `0`, weakens the test by
      nothing.
      **Why a reader is misled:** the comment states *"THIS IS THE TEST THAT
      CAUGHT THE SENTINEL BUG … `counter_sort_key(0)` is exactly `i64::MAX` …
      **Counter 0 is in this list for that reason and must stay.**"* Under the
      two-column shape the change adopted, that is no longer true — the sentinel
      is gone precisely because `has_counter` is separate, which is what makes the
      loop inert. A future author weighing a change to `counter_sort_key`'s
      encoding reads this as boundary coverage and gets none. The test that
      genuinely exercises the collision end-to-end is the sibling at
      `sqlite.rs:1540`. Severity: genuine defect — a comment instructing the next
      reader to preserve a value that does nothing is worse than silence.

      **Fixed by making the loop values live, rather than by softening the
      comment.** The test is now
      `an_op_carrying_a_counter_reads_ahead_of_one_carrying_none_at_every_counter`
      and goes through a real `log.iter()`, so each counter reaches
      `counter_sort_key` and then the `ORDER BY` that compares both columns. Per
      iteration it SEARCHES for a body pair giving the counter-LESS op the lower
      op id, guards that relation, and appends counter-less first — so an id-
      ordered read and an insertion-ordered read each give the wrong answer. Two
      alternative explanations ruled out at once.

      Counter 0 now earns the place the old comment claimed for it.
      **Mutation, written as the exact historical defect:** `SortKey::of`'s
      `has_counter: 0` → `i64::from(counter_sort_key(clock.counter) == i64::MAX)`,
      which reintroduces the in-band sentinel. It **fails on the counter-0
      iteration** — the two ops come back in the wrong order. Under the old form
      the same mutation was invisible. Restored.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/moderation.rs:1247-1277`
      — a test names its fixtures "forged" and asserts on the adjacent line that
      they are not
      **Scenario:** `a_forged_hide_does_not_win_against_a_binding_unhide_however_
      many_are_minted` binds `let forged = a_moderation_at(..., &outsider(), ...)`
      and immediately asserts `assert!(forged.verify(), "the fixture must be
      authentic")`. This file defines "forged" precisely and elsewhere: the
      `a_forged_moderation` helper (`moderation.rs:717`) asserts
      `assert!(!forged.verify(), "the fixture must be an actual forgery")`. So
      within one file, `forged` means *signature does not verify* in one place and
      *validly signed by a non-moderator* in another, with the two assertions
      contradicting each other verbatim.
      **Why a reader is misled:** authenticity and authority are two separate
      rejection paths in `resolve`, and the file tests them separately. This test
      exercises the **authority** path while its name advertises the
      **authenticity** one, so the suite reads as though forgery-under-minting is
      covered when the non-moderator case is. Severity: genuine defect, and the
      project's recorded "two tests asserting the opposite of their names" family.
      The fix is a rename (`an_unauthorised_hide_...`), not a change to the
      assertions, which are correct. Note the loose usage predates this branch
      (`a_stoa_where_a_forged_unhide_sorts_first`, line 635), but this is the
      instance where the contradiction sits on adjacent lines.

      **Fixed as a rename, exactly as the finding prescribes** — the assertions
      were correct and are untouched. It is now
      `an_unauthorised_hide_does_not_win_against_a_binding_unhide_however_many_
      are_minted`, the local binding is `unauthorised` rather than `forged`, and
      the `verify()` assertion's message now says WHY authenticity is being
      asserted in a test about authority (a signature failure would reject on the
      other path, so the test would stop measuring the one it names).

      **Mutation, on the path the new name claims:** `resolve`'s binding filter
      with `&& moderators.authorises(e)` deleted. It **fails** on "no number of
      non-binding hides may hide a target". So the test does measure the
      authority path, which is what the rename asserts about it. Restored.

      **A wrong guess, recorded rather than quietly dropped.** I first wrote here
      that this test also fails under the `if false && first.op.op.clock.
      is_some()` mutation used for the correctness boxes. I then ran it: it
      **passes** under that mutation, because with one binding Unhide and twenty
      non-binding Hides the degraded branch's `Hide` preference finds nothing
      binding to prefer and falls back to the same op. The claim was plausible
      and untrue, which is exactly why it had to be run rather than reasoned.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/asserted_time.rs:252-263`
      — the comment says the value clamps; the assertion says it does not
      **Scenario:** `the_maximum_representable_instant_formats_rather_than_
      panicking` comments *"**The value clamps**, so what this really pins is that
      NOTHING on the path overflows"*, and three lines later asserts
      `assert!(!out.clamped, "u64::MAX against a u64::MAX reader is in range")`.
      The assertion is the correct one — a `u64::MAX` asserted time against a
      `u64::MAX` reader clock is inside the saturated allowance — so the comment
      is stale rather than the test wrong. A reader reconciling the two has to
      re-derive the saturation argument to work out which half to trust.
      Severity: genuine defect, minor, and the cheapest fix in this list. Called
      out because `asserted_time.rs` is otherwise the best-documented file in the
      change and this is its one internal contradiction.

      **Fixed**, in the direction the finding identifies as correct: the comment
      now says the value does NOT clamp and why (a `u64::MAX` claim against a
      `u64::MAX` reader is inside the saturated allowance), with a parenthetical
      recording what it used to say, so the next reader does not have to
      re-derive which half was wrong. The assertion is untouched.

      **No mutation, and no pretence of one**: a comment cannot fail a test. The
      assertion it contradicted already has one —
      `a_maximal_reader_clock_does_not_overflow_the_allowance` is the pair that
      pins the `saturating_add`.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7422`
      — a test named for what the wire reports never reads the wire
      **Scenario:** `re_publishing_an_op_the_peer_holds_says_it_was_not_new`.
      In this file `wasNew` is the wire reply field (`wire.rs:1964`) and is the
      only thing that reports newness to a caller, so the name reads as a claim
      about the boundary's report. The second half of the test calls
      `log.append(held, ...)` directly and asserts
      `Appended::AlreadyPresent` — it never calls `publish_post`, never produces
      a JSON reply, and never reads `wasNew`. Its sibling immediately above
      (`wire.rs:7377`) *does* assert `wasNew` on the wire, so the pair reads as
      "the wire says true here and false there" when only one is a wire
      assertion. A change that dropped or inverted `wasNew` on the dedup path
      would leave this test green. Severity: genuine defect — either rename to
      name the log-level fact, or extend it to the wire, but the current name
      claims boundary coverage the body does not provide.

      **Fixed by rename**, to
      `appending_an_op_the_peer_already_holds_reports_already_present`, which is
      what the body measures. The comment now records why the old name was a
      false boundary claim.

      The rename rather than the extension, and the reasoning is worth stating so
      it can be overruled: the `wasNew: true` side of the wire IS asserted, by the
      sibling immediately above
      (`a_second_authoring_of_one_body_is_a_second_op_and_says_so`), so the gap is
      one direction of one field.

      **Recorded as a residual gap rather than closed, and the reason is a
      fixture problem I could not solve within this box's scope.** `wasNew` is
      `published.was_new()`, so reaching `false` through the wire needs a publish
      whose op the log already holds — which after this change means the peer must
      already hold an op with the SAME counter and the SAME asserted time as the
      one `publish` is about to stamp. That is constructible (append a
      hand-built op matching what the next publish will produce, then publish),
      but it is a fixture that must predict `publish`'s own output, and getting it
      wrong yields a test that passes for the ordinary `wasNew: true` reason. I
      did not write it rather than write one I could not prove distinguishes the
      two. **Open for whoever takes the `wasNew`-on-the-dedup-path question next:
      that field has no wire test in the `false` direction.**

## Not findings, recorded so nobody re-opens them

- `asserted_time.rs`'s hand-rolled `civil_from_days` is documented as to why it
  is not a date crate, is total over `u64`, and its tests pin the 4/100/400-year
  rules with derived rather than looked-up constants. Clean.
- `OpEntry`'s "cannot name an `Arrival`" claim **is** structurally true — the
  struct has two fields, neither of which is an `Arrival` — unlike the `Placed`
  claim above. The distinction is why one is a finding and the other is not.
- `position` being an opaque decimal string rather than the counter is argued at
  `thread.rs:169-191` and at the wire; the reasoning (a counter invites
  arithmetic that means nothing) is sound and stated where a caller reads it.
- `log/contract.rs:816`, `whether_an_arrival_was_ordered_survives_storage`, is
  named for a predicate this change deleted. **Not opened as a box** because the
  branch declares the drift in the test's own comment ("This test's SUBJECT has
  changed and its name has not"), so a reader who opens it is not deceived. Worth
  a rename whenever the file is next touched, since the name is what appears in a
  CI failure list — but a disclosed mismatch is a different thing from the four
  above, and the author made the call deliberately.
- `authoring.rs`, `revision.rs`, `thread.rs` and `op.rs`'s new tests were swept
  for name/assertion mismatch and are clean. `revision.rs` is the strongest of
  them: every ordering test draws from `two_revisions_disagreeing`, a helper that
  searches for a fixture whose op ids rank opposite to their counters and panics
  if it cannot find one, which closes the project's recorded "two explanations
  give the same answer" family at the point it would otherwise open.
  `op.rs:3146` is weaker than its name but concedes exactly that in its own
  comment and names the stronger structural argument.
