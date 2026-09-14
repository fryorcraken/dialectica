# spec-test review — `op-clock`

**I did not read the implementation.** I read the seven spec deltas, the
proposal, `tasks.md`, `docs/PLAN.md` where the spec cites it, and the test code.
The only implementation lines I opened are the ones I mutated, and I read no
further than the line under the edit. Mutations were run in
`.claude/worktrees/review-clock-spec-test` (its own tree, so no reviewer saw
another's broken code) and every one was restored; `git status --porcelain` is
empty there and the suite is back to **972 + 30 passing**.

## Findings

- [ ] **`tester`** — `thread-read`'s clamping scenario is not pinned anywhere,
      and two independent mutations prove it.
      **Where:** `dialectica/rust-lib/dialectica-core/src/wire.rs:1872` and
      `dialectica/rust-lib/dialectica-core/src/thread.rs:787`.
      **The scenario:** `openspec/changes/op-clock/specs/thread-read/spec.md:44`
      — *"An implausible asserted time is reported as clamped … THEN the
      displayed time is clamped AND the item reports that it was clamped"*. The
      same requirement is stated again at
      `specs/op-ordering/spec.md:187` and `:193`.
      **Measured, mutation A:** changed `wire.rs:1872` from
      `"clamped": asserted.clamped` to `"clamped": false` — a peer that can
      never report a clamp. **All 972 + 30 tests passed.**
      **Measured, mutation B:** changed `thread.rs:787` from
      `format_asserted(c.asserted_ms, now_ms)` to
      `format_asserted(c.asserted_ms, c.asserted_ms)` — the reader's clock
      disconnected from the clamp, so clamping can never fire on any read at
      all. **All 972 + 30 tests passed**, and `rustc` emitted
      `warning: unused variable: now_ms` into a green run.
      **Why the existing coverage does not reach it:** `asserted_time.rs`'s
      tests are excellent and pin `format_asserted` itself at both boundaries,
      but they call it directly. Nothing pins that the *read path* wires the
      reader's clock into it. The one wire test that touches the field
      (`wire.rs:6629-6641`) asserts the `clamped` key **exists**, never that it
      is ever `true`. Every thread and wire fixture uses one asserted-time
      value, `A_TIME` — see `grep -rn "asserted_ms: "` across `thread.rs` and
      `wire.rs`: three hits, all `A_TIME`.
      **Failure this admits:** a post asserting the year 2387 renders as an
      ordinary time with `clamped: false`, and the view has no way to say the
      value was not the author's. The spec calls clamping *"defence in depth"*
      (`op-ordering/spec.md:181`), which is exactly why its absence is quiet.
      **Fix:** one test reading a thread whose op asserts a far-future time,
      asserting `assertedTime.clamped == true` and that `text` is the clamped
      value rather than the asserted one. **Severity: high** — two surviving
      mutations on a spec scenario stated twice.

- [ ] **`tester`** — `thread-read`'s *"The sequence does not follow the asserted
      times"* has no test, and cannot have a non-vacuous one with the present
      fixtures.
      **Where:** `openspec/changes/op-clock/specs/thread-read/spec.md:95` —
      *"WHEN a thread's replies carry asserted times that disagree with the
      order their counters give THEN the returned sequence is the one the
      counters give AND the asserted times did not affect it."*
      **Measured:** every asserted time in every `thread.rs` and `wire.rs`
      fixture is the single constant `A_TIME` (three occurrences, no others).
      A fixture in which all asserted times are equal cannot distinguish
      "ordered by counter" from "ordered by asserted time" — the recorded
      defect family, applied to this field.
      **Mitigating, and why this is medium rather than high:** `OpEntry`
      (`arrival.rs:314-317`) carries only `counter` and `id`, so `cmp_ops`
      structurally *cannot* read `asserted_ms`. The exclusion holds by type.
      That makes the risk of regression low but leaves the scenario unpinned,
      and it also means `arrival.rs`'s
      `the_wall_clock_confers_no_position` and
      `a_far_future_wall_clock_does_not_reach_the_head_of_the_order` are
      near-tautologies: they vary a field the comparator's input type does not
      carry, so no comparator mutation can make either fail.
      **Fix:** either a thread read whose replies' asserted times run opposite
      to their counters, or — cheaper and arguably better — a note on those two
      `arrival.rs` tests recording that the property is type-enforced and that
      they are documenting it rather than measuring it. **Severity: medium.**

- [ ] **`tester`** — `authoring.rs:807` asserts an order against a fixture that
      does not force the two explanations apart. It passes today by luck.
      **Where:**
      `dialectica/rust-lib/dialectica-core/src/authoring.rs:780-808`,
      `the_second_authoring_carries_the_higher_counter`.
      **The shape:** the two ops differ only in their counter, so which op id
      sorts lower is a property of the digest. The test never asserts the
      relation between the two ids, so "ordered by counter" and "ordered by
      ascending op id" are both live explanations of
      `assert_eq!(ids, vec![second.id, first.id])`.
      **Measured:** it *did* fail under my ascending-op-id mutation — the
      digests happen to fall the right way (`second.id` begins `0e ff…`,
      `first.id` begins `74 fb…`). So it is non-vacuous **today**, by
      coincidence, and would go silently vacuous on any change to the body
      string, the key, the Stoa title or `A_TIME`.
      **Contrast with the code that gets this right:**
      `arrival.rs:515-524` *searches* for a disagreeing pair and
      `arrival.rs:537-551` asserts the fixture really disagrees;
      `revision.rs`'s `two_revisions_disagreeing` does the same and
      `revision.rs:978` re-asserts it at the point of use. This test should
      borrow that shape.
      **Fix:** add `assert!(second.id > first.id, "the fixture must make the
      two rules disagree")` — or search bodies until they do — so the
      coincidence becomes a checked precondition.
      **Severity: medium** — the assertion is sound now; what is missing is the
      guard that keeps it sound.

- [ ] **`tester`** — `authoring.rs:1714` has the same unguarded shape, on the
      hostile-counter path.
      **Where:** `authoring.rs:1654-1715`,
      `a_peer_can_still_publish_after_receiving_a_maximal_counter`, at
      `assert_eq!(ids[0], hostile_id, "and it takes its place at the head")`.
      **The shape:** the hostile op (`counter: u64::MAX`) and the honest op
      (`counter: 5`) have different bodies, so their ids are unrelated digests
      and nothing asserts how they rank. `ids[0] == hostile_id` is satisfied by
      an id-ordered log with probability ½, and only `ids[0]` is checked — the
      rest of the returned order is unexamined.
      **Note this is the security-relevant half of the advance-bound
      requirement** (`op-ordering/spec.md:117`, *"An over-bound op still takes
      its place in the order"*), which is why it is listed separately from the
      one above rather than folded into it.
      **Fix:** assert the id relation, or assert the whole `ids` vector rather
      than only its head. **Severity: medium.**

- [ ] **`tester`** — two `authoring.rs` fixtures named for *receiving* an op
      have only one author, so a clock that ignored received ops would pass
      both.
      **Where:** `authoring.rs:1619`
      (`publishing_after_receiving_advances_past_what_was_received`, hand-built
      op at `:1630` uses `author: key.public_key()`) and `authoring.rs:1654`
      (`a_peer_can_still_publish_after_receiving_a_maximal_counter`, `:1669`
      and `:1684`, same key).
      **The scenario:** the spec's requirement is about *causality between
      peers* — `op-ordering/spec.md:61`, *"Publishing after receiving advances
      past what was received"*, and `:52`, *"Every peer that receives both
      computes the same relation"*. Both fixtures' comments say "an op from
      somebody else" / "from a peer"; neither fixture has a second identity.
      An implementation that folded only over *its own* prior ops passes both.
      **Fix:** sign the received op with a second key. One line each.
      **Severity: low-medium** — the numeric assertions (`== 501`, `== 6`) are
      strong and would catch most breakages; what they cannot catch is a clock
      scoped to the peer's own authorship.

- [ ] **`tester`** — `every_publish_path_stamps_a_counter` is a hand-maintained
      list that its own comment calls a sweep.
      **Where:** `authoring.rs:1768`, with the paths enumerated literally at
      `:1777-1786` (`post`, `reply`, `vote`).
      **The scenario:** the comment at `:1769-1772` says a *fourth* publish
      path that forgot the clock is what it guards against. It cannot see one —
      a new publish-shaped function enters the crate with this test green. This
      is the recorded `hand-maintained sweep lists go stale silently` shape,
      and `op.rs`'s `one_of_each_kind()` is already called out for it in
      `tasks.md`'s notes section.
      **Fix:** either derive the enumeration (as `one_of_each_kind` does for
      kinds) or reword the comment to claim only what the test does.
      **Severity: low** — the claim is the defect, not the coverage.
      *(The `architecture` reviewer reports this independently; if their box is
      actioned, tick this one with it.)*

- [ ] **`spec-writer`** — `op-ordering`'s scenario *"The clock survives a
      restart without a stored counter"* asserts something no test can check as
      written.
      **Where:** `openspec/changes/op-clock/specs/op-ordering/spec.md:25-29`:
      *"THEN its clock is the value it had before AND **the value was
      recomputed from the ops rather than read from a counter recorded beside
      them**"*. The same shape is at `:31-36` (*"AND the sequence the ops were
      appended in did not affect it"*) and `:37-41`.
      **The scenario:** the second clause is a claim about *how* a value was
      produced, not about any observable. A test can show the value is right
      after a restart and right under a reordered replay — and the tests do
      (`arrival.rs:807-825`, `clock_from_counters` over reversed and
      maximal-first sequences). It cannot show that no counter was recorded
      beside the ops, short of inspecting the schema.
      **This is a spec defect rather than a coverage gap**, and it is worth
      distinguishing because the derivation property *is* discharged
      structurally — `OpLog::clock` is a default method over `iter_stoa`, so
      there is no stored counter to read. The spec should say what is
      observable (the value survives, and is sequence-independent) and record
      the "derived, never stored" part as a design constraint, which is where
      `design.md` already carries it.
      **Severity: low.**

- [ ] **`spec-writer`** — three `NO SPEC:` markers are field *names*, and one of
      them is load-bearing in a way the spec does not acknowledge.
      **Where:** `wire.rs:1814` (`authorKey`, pre-existing), `wire.rs:1825`
      (`position`), `wire.rs:1859` (`assertedTime`, with its `text` /
      `authorAsserted` / `clamped` shape).
      **The author's characterisation is right as far as it goes** — the spec
      requires the values and names no fields
      (`thread-read/spec.md:5`, `op-ordering/spec.md:139-141`), so a name
      chosen by the implementation is a genuine gap of the mildest kind.
      **What the spec does not settle and should:** `op-ordering/spec.md:141`
      requires the ordering position be *"carried separately, as its own
      field"*, and the proposal (Q5) goes further — *"an opaque token a view can
      only compare for sequence"*. Whether `position` is opaque, and what a view
      may assume about it, is contracted nowhere; a view that started doing
      arithmetic on it would be within the spec's letter. The `assertedTime`
      *shape* is the stronger case: `wire.rs:6629-6636` pins the nested object
      to exactly `{authorAsserted, clamped, text}` with no number, which is the
      spec's central defence
      (`op-ordering/spec.md:139`, *"SHALL NOT surface it as a bare number"*) —
      that shape deserves to be in the spec rather than discovered from a test.
      **Severity: low** for the names, **medium** for `position`'s opacity and
      the `assertedTime` shape being uncontracted.

## Corroborating the correctness reviewer, without duplicating a box

`findings/correctness.md` reports `moderation.rs`'s
`the_order_is_by_lamport_and_not_by_op_id` and
`a_later_hide_reverses_an_earlier_unhide` as fully vacuous. **I confirm it
independently and from the spec side**, and I open no box of my own because
theirs already covers it.

The mechanism is visible without reading any implementation:
`moderation.rs:1357-1360` sets the fixture up with
`Arrival::ordered(2, …)` and `Arrival::ordered(9, …)`. Those are *transport*
arrival values. This change's whole point is that ordering no longer reads them
— `op-ordering/spec.md:238`, *"Ordering SHALL NOT consult the transport's
Lamport timestamp or message id"*, enforced structurally because `OpEntry`
cannot name an `Arrival`. So the fixture's carefully-built disagreement (the
comment at `:1318-1328` is a model of how to think about this) is wired to a
channel the comparator stopped listening to, and the test now passes on the
`Hide` bias alone.

It is consistent with my own measurements: my three comparator mutations each
failed three `moderation.rs` tests, and **neither of these two was ever among
them.** A test that survives every mutation of the rule it is named for is the
strongest evidence available that it measures nothing.

The spec-side reading worth adding for whoever fixes it: this is not a test that
was always weak. It was *correct* before this change and was **falsified by
it**, in the same way the `moderation-resolution` delta says the "exactly two
ops can ever exist" premise was falsified. A fixture built on transport
arrivals is now the test equivalent of the prose retractions `tasks.md`'s notes
section lists — and that list did not include test fixtures.

## What I checked and found clean, in prose

**The ordering rule is thoroughly pinned, and the fixture-vacuity defence is
real.** `arrival.rs:493-551` is the best thing in this change: it states the
defect family in a comment, *searches* for a pair whose counter order and op-id
order disagree rather than hardcoding one, and then guards the guard with
`the_disagreeing_fixture_really_disagrees`. I measured it. Replacing the
both-carry-a-counter arm of `cmp_ops` with `a.id.cmp(b.id)` — precisely
"explanation B" — **failed 16 tests** across `arrival`, `authoring`,
`log::contract`, `moderation`, `revision` and `thread`. Replacing it with
`Ordering::Equal` failed the same 16.

**Migration is pinned with genuinely mixed fixtures.** Inverting the
`(Some, None)` / `(None, Some)` arms **failed 5 tests**, including
`an_op_carrying_a_counter_leads_one_carrying_none` (at counter **zero**, which
is the value a sentinel design would collide with),
`ops_with_and_without_a_counter_coexist_with_the_counter_ones_first_in_memory`,
`the_ordered_branch_is_chosen_by_the_leading_op_not_by_all_of_them` and
`a_revision_carrying_a_counter_beats_one_carrying_none_whatever_its_op_id` —
the last of which *searches* for a clock-carrying op whose id is the higher, so
it would lose under the degraded rule. The irreversible-by-design migration is
the best-covered thing here.

**The advance bound is pinned at its edge from both sides.** Flipping `<=` to
`<` failed 4 tests including `a_counter_exactly_at_the_bound_advances_the_clock`
("the bound is inclusive") and `a_gap_larger_than_the_bound_stops_the_ladder`.
One redundancy worth nothing more than this sentence:
`a_counter_within_the_bound_advances_the_clock` (`arrival.rs:828`) and
`a_counter_exactly_at_the_bound_advances_the_clock` (`arrival.rs:833`) are
byte-for-byte the same assertion, and the first's name promises a value strictly
inside the bound. Not a finding — the boundary pair that matters is the second
and third. (The `readability` reviewer has this one.)

**The composer guard: the author's claim is honest, and slightly overstated in
one direction.** I deleted the `if (root.publishing) return` guard at
`DComposer.qml:242`. Exactly **one** test failed —
`test_a_submit_while_one_is_outstanding_sends_nothing`, reaching 5 calls instead
of 1 by recursing from inside `callModule`. That is a real measurement of a real
guard, reached the only way it can be reached today, and the long comment at
`tst_composer.qml:827-843` is scrupulously honest about why the tap-twice test
is not written. One correction: the comment's *"Each fails if the guard is
removed"* is **not** true of all four — `test_the_control_is_absent_while_a_
publish_is_outstanding` and the two re-enable tests passed with the guard gone,
because they exercise the flag and the `visible` binding rather than the
re-entrancy check. Worth a one-word softening, not a finding.

**`the_recorded_arrival_does_not_reach_the_sort_key_at_all` measures more than
the author credits it with.** The brief flagged it as possibly restating
`SortKey::of`'s signature. It does not: `log/sqlite.rs:1604-1644` drives five
distinct `Arrival` shapes through `log.append(op, arrival)` — which *does* take
an `Arrival` — and reads the stored `sort_has_counter` / `sort_counter` columns
back out of SQLite, which is what the `ORDER BY` compares. A regression writing
arrival-derived values into the sort columns fails it. The author under-sold
this one.

**`asserted_time.rs` is exemplary.** Boundaries from both sides at the allowance
and the floor, a leap day, a century non-leap year and a 400-year leap year each
derived rather than looked up, truncation-not-rounding, saturating arithmetic at
`u64::MAX` in both arguments, and constants pinned with a self-check that the
floor really is the date its comment claims. My only note is that its coverage
stops at the function boundary, which is the first finding above.

**`op-format` is well covered.** `the_counter_round_trips_at_every_boundary_
value` uses boundary values and dedups the encodings so no two counters encode
alike; `both_clock_fields_participate_in_the_op_id` varies one field at a time
against a base and asserts the *encodings* differ as well as the ids, so it pins
that the field reaches the preimage rather than restating SHA-256's avalanche —
the shape my brief warned about is avoided.

**The two `REMOVED` requirements survived their move.** `op-ordering`'s removal
of *"The Lamport timestamp alone decides whether an op is ordered"* and *"A
message id present without a Lamport timestamp does not order"* are both genuine
subsumptions, not silent losses: the first's live question is answered by the
retained-and-re-aimed degraded-order requirement, and the second is covered by
the strictly broader *"Ordering SHALL NOT consult the transport's Lamport
timestamp or message id"* — which is enforced structurally, since `OpEntry`
cannot name an `Arrival`. No requirement text changed during a move.

**`openspec validate --changes --strict` passes, and the spec is
self-consistent on a full read.** The two withdrawn prohibitions are withdrawn
*explicitly and in place*, with the reasoning preserved and re-aimed rather than
deleted, which is the shape that stops a later reader believing the old text is
live. I found no requirement contradicting another.

**`docs/PLAN.md` has genuinely shed what the spec now carries.** Checked against
`origin/main` (`9357417`), not the branch's copy. §5.7's and §6's "no Lamport
value reaches us", the suspended-reversibility note, §9.1 §8's feed-label
bullet, §13's Lamport and `createdAt` entries, and the "withdrawal has not
reached the spec" contradiction are each struck through with a pointer to the
spec and the history left legible. The falsified "exactly two ops can ever
exist" premise is called out as *false and load-bearing* rather than quietly
deleted, and the composer obligation the change creates is recorded as a live
obligation rather than a closed question. This is the right shape throughout.
