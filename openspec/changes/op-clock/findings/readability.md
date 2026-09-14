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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/feed.rs:5-8`
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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/moderation.rs:425-426`
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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/thread.rs:225-238`
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

- [ ] **`dev-writer`** — `openspec/changes/op-clock/design.md:86` — a citation
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

- [ ] **`dev-writer`** — `openspec/changes/op-clock/tasks.md:20` — the stage row
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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs:265-296`
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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/authoring.rs:204`
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

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/arrival.rs:827-841`
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

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs:1585-1601`
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

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/moderation.rs:1247-1277`
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

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/asserted_time.rs:252-263`
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

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7422`
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
