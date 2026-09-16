# spec-test review — `op-clock`

**I did not read the implementation.** I read the seven spec deltas, the
proposal, `tasks.md`, `docs/PLAN.md` where the spec cites it, and the test code.
The only implementation lines I opened are the ones I mutated, and I read no
further than the line under the edit. Mutations were run in
`.claude/worktrees/review-clock-spec-test` (its own tree, so no reviewer saw
another's broken code) and every one was restored; `git status --porcelain` is
empty there and the suite is back to **972 + 30 passing**.

## Findings

- [x] **`tester`** — `thread-read`'s clamping scenario is not pinned anywhere,
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

      **Fixed**, with two tests in `wire.rs` rather than one, because the two
      mutations are two different defects and one test cannot be honest about
      both:

      - `a_thread_read_clamps_an_implausible_asserted_time_and_reports_the_clamp`
        — a table over five asserted times read through `read_thread` at a fixed
        reader clock: inside the allowance, the year 2387, one ms past the
        allowance, exactly at the allowance, and zero. It asserts `clamped`, the
        rendered `text`, and that `authorAsserted` stays true through a clamp.
        The at-the-edge and one-past-the-edge rows differ by one millisecond, so
        a build that clamped everything and a build that clamped nothing each
        fail.
      - `a_thread_read_clamps_against_the_readers_clock_rather_than_the_ops_own`
        — the SAME asserted instant read against two different reader clocks,
        getting two different verdicts. That is the property no single-value
        fixture can express, and it is what mutation B disconnects.

      **Every expected string is hardcoded and derived independently**, by
      `date -u -d @<seconds>`, not read back out of the crate. Three of my four
      first attempts at those constants were WRONG and the `date` check caught
      them before the test ran — which is the argument for the independence rule
      in one measurement.

      **Mutation A** (`wire.rs`, `"clamped": asserted.clamped` → `"clamped":
      false`): **both tests fail**, on `clamped` left `false` right `true`.
      **Mutation B** (`thread.rs`, `format_asserted(c.asserted_ms, now_ms)` →
      `format_asserted(c.asserted_ms, c.asserted_ms)`): **both tests fail**, and
      the far-future row now renders `2387-05-05T13:20:00Z` unclamped — the
      reader sees "posted in 2387", which is the failure this finding describes.
      Both restored; `git diff` shows test files only.

- [x] **`tester`** — `thread-read`'s *"The sequence does not follow the asserted
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

      **Fixed**, taking the first branch — the thread read — because it is the
      one a regression could reach. `thread.rs` gains `a_reply_at`, a reply
      fixture taking the counter and the asserted time as SEPARATE parameters
      (the whole cause of this finding was one `A_TIME` standing for both), and
      `the_sequence_follows_the_counters_and_not_the_asserted_times`.

      The fixture makes the two rules give opposite answers: counters run 3/2/1
      (descending order, so that sequence) while the asserted times run
      2024/2025/2026 the other way, so the highest counter claims the EARLIEST
      instant. The replies are named for the time they CLAIM rather than for
      their position, so a sequence ordered by time in either direction fails.
      The test also asserts the three rendered times, so a read that emitted no
      time at all could not satisfy the sequence assertion alone. The three
      instants are verified against `date -u`.

      **Mutation:** `cmp_ops`'s counter arm inverted to ascending. It **fails**,
      returning the exact reverse of the expected id vector. The same failure
      output shows the three ids in an order that distinguishes ascending-op-id
      ordering too, so no second mutation was needed. Restored.

      **On the second half — the two `arrival.rs` near-tautologies — I take the
      finding's own reading and open no change.** `the_wall_clock_confers_no_
      position` and `a_far_future_wall_clock_does_not_reach_the_head_of_the_
      order` vary a field `OpEntry` does not carry, so no comparator mutation can
      make either fail; they document a type-enforced property. The new thread
      test is what now measures the scenario end to end, which is the stronger of
      the two fixes the finding offered.

- [x] **`tester`** — `authoring.rs:807` asserts an order against a fixture that
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

      **Fixed**, taking the `assert!` form rather than the search: this test is
      about two publishes of one fixed body through the real `post` path, and
      varying the body would make it a different test. The guard's message says
      to re-roll the fixture rather than to delete the guard.

      **The finding's cited digests are stale, and that is itself worth having
      measured.** It says `second.id` begins `0e ff…` and `first.id` begins
      `74 fb…` — i.e. `second.id < first.id`. I wrote the guard that way round
      first and it **failed immediately**: in the current tree `second.id >
      first.id`. The `now_ms` → `asserted_ms` rename in commit `6156082` re-rolled
      the preimage between the review and this fix, which is exactly the drift
      this guard exists to catch, arriving inside the same change. The guard is
      now `second.id > first.id`, which is the relation under which ascending op
      id names the FIRST op and the counter rule names the second — so they
      genuinely disagree.

      Since the review's cited values no longer describe this tree, the guard is
      what any future reader should trust; the digest prefixes in the box above
      are left as written rather than edited, per the rule about not rewriting a
      reviewer's text.

- [x] **`tester`** — `authoring.rs:1714` has the same unguarded shape, on the
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

      **Fixed**, taking both halves of the suggested fix. The assertion is now the
      WHOLE three-element vector — `[hostile, this peer's own publish at 6, the
      honest op at 5]`, which is descending counter over three distinct counters
      so no tiebreak is involved — plus a guard asserting the sequence is NOT the
      sorted-by-id one, so the fixture cannot drift into the two rules agreeing.

      **Mutation:** `cmp_ops`'s counter arm led by `a.id.cmp(b.id)`. It **fails**,
      returning the three ids in ascending-id order. Worth recording from the
      failure output: the hostile op's id (`74 9b…`) is the HIGHEST of the three,
      so the old `ids[0] == hostile_id` would have been satisfied by
      descending-op-id order as well — it is only under ascending op id that the
      hostile op lands last, and only the whole-vector form sees that. Restored.

- [x] **`tester`** — two `authoring.rs` fixtures named for *receiving* an op
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

      **Fixed.** `ANOTHER_ROOT` is a second peer's root secret, documented with
      this finding's own reasoning, and both fixtures now sign their received ops
      with a key derived from it — with an `assert_ne!` on the two public keys, so
      "the fixture needs two identities" is checked rather than assumed.

      **Mutation, written to be exactly the implementation this finding
      describes:** `publish` reading its clock from a fold filtered to the
      publishing author's own ops, instead of `log.clock(&op.stoa)`. **Exactly
      these two tests fail** out of the 44 in the file — `== 501` becomes `== 1`
      and `== 6` becomes `== 1` — and no other test in the whole suite notices.
      Before the second key, that mutation left both green. Restored.

- [x] **`tester`** — `every_publish_path_stamps_a_counter` is a hand-maintained
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

      **Already fixed by the `dev-writer` in `e8b1535`**, taking the stronger of
      the two options this box offered: the test is now
      `publish_stamps_a_counter_onto_an_op_of_any_kind`, derived from
      `log::fixtures::every_op_kind()` rather than naming three builders, with a
      floor assertion on the kind count. Verified by reading the current body, not
      inferred from the commit subject. Ticked here as that box instructs.

- [x] **`spec-writer`** — `op-ordering`'s scenario *"The clock survives a
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

      **Fixed**, taking the option this box names: say what is observable, and
      carry the derivation as a constraint rather than a SHALL a test cannot
      distinguish from its negation.

      The scenario is now *"The clock survives a restart"* and asserts only that
      the value is the one it had before. *"The clock survives a rebuild by
      replay"* lost *"AND the sequence the ops were appended in did not affect
      it"* — a restatement of its own WHEN, not a second check.

      The third site the box names, *"A rebuild reaches the same answer as the
      original ingest"*, was **not** removed, because on reading it is
      observable rather than a how-claim — it just was not phrased as the
      observable. It now reads *"AND both equal the highest counter within the
      bound, and neither equals the over-bound op's counter"*, which a fixture
      can check by construction. Recorded rather than silently done: this box
      listed three sites and two of them moved for the reason given; the third
      was a weak phrasing of a real check, not an unobservable claim.

      The derivation property is now stated once, in the requirement body, as
      *"What a reader can check, and what only an implementation can"* — naming
      why no caller can observe it (a stored counter kept in step is
      indistinguishable through the read interface, and the cases where it
      diverges are not reachable through this contract's operations) and
      pointing at `design.md` for how it is discharged. The `SHALL NOT persist`
      sentence above it stands; what changed is that it no longer masquerades as
      something a scenario checks.

- [x] **`spec-writer`** — three `NO SPEC:` markers are field *names*, and one of
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

      **Fixed for the medium half; the low half is left as a `NO SPEC:` marker
      on purpose**, and the split is deliberate rather than partial work.

      `thread-read`'s added requirement now contracts both:

      - **`position` identifies where an item sits in the whole thread and is
        explicitly NOT a sort key.** What is contracted is what a caller may
        *do* with it: the value is stable across page sizes, indexes the whole
        thread rather than restarting per page, and is unique within a thread —
        and nothing further. Not a number, no arithmetic, no adjacency or fixed
        distance, not comparable against another thread's, **and not something a
        caller may sort by**. A view doing arithmetic on it was within the old
        spec's letter and is now outside it.

        **The first draft of this contract said "comparable for sequence", and
        it was wrong** — caught by re-reading the value rather than the prose,
        and recorded because the wrong version is the plausible one. `position`
        is `index.to_string()`, unpadded (`thread.rs:291`), so a caller
        comparing positions as strings gets `"10" < "2"` and a sequence that is
        not the returned one. The existing test uses a five-item thread
        (`thread.rs:3044-3086`), so the defect is invisible today and a scenario
        promising comparability would have been a requirement the implementation
        fails at ten items. The proposal's Q5 phrasing — *"an opaque token a
        view can only compare for sequence"* — is therefore **not** what the
        spec now says, and the difference is deliberate: the sequence a caller
        renders is the one the read returned, which is what `thread-read`
        already required and what the implementation actually delivers.
      - **The asserted time is a grouped value holding exactly three members** —
        display text, author-assertion marker, clamped report — with **no fourth
        member carrying the instant in any numeric or otherwise comparable
        form**. That is the shape `wire.rs:6629-6636` pins, lifted out of the
        test and into the contract, which is what the box asks for: the central
        defence of this change should not be discoverable only from a test.

      Four scenarios were added so each clause is checkable rather than prose:
      positions reproduce the returned sequence; a position indexes the whole
      thread and not the page; the position is not surfaced as a quantity; the
      three members travel together and no fourth carries the instant.

      **Two things this box's work uncovered, both routed on rather than closed
      here, because they are not the `spec-writer`'s to touch:**

      - **`thread.rs:197`'s doc comment makes the same false promise** — *"A
        caller compares two of these for sequence"*. It is code, and this role
        writes no code. It should read what the spec now says: a caller renders
        the returned sequence, and the position says where an item sits in the
        whole thread. **Routes to a `dev-writer`.**
      - **The unpadded index is the underlying issue, and the spec was narrowed
        to it rather than around it.** Zero-padding the position would make
        comparison work and would let the stronger contract stand. That is a
        behaviour change with its own tests, so it is not made here; the spec
        now describes what the value does. If a later change pads it, the
        comparability clause can be added back and this note is the record of
        why it was absent.

      **The field NAMES stay uncontracted, and that is the answer rather than an
      omission.** `authorKey`, `position`, `assertedTime`, `text`,
      `authorAsserted` and `clamped` remain the implementation's choice. Naming
      them in the spec would pin the wire spelling without adding a property
      anyone depends on — the properties that matter are opacity and the
      three-member shape, and both are now stated in terms a rename cannot
      falsify. Leaving a `NO SPEC:` marker in place is itself a decision, as the
      brief for this role says, so it is recorded here: the markers at
      `wire.rs:1818`, `:1829` and `:1863` are correct as they stand and should
      not be removed by a later pass reading them as unfinished.

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
