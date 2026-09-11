# Tasks

## 1. Settle whether SDS's order reaches us

- [x] Read LIP-109 (`sds.md`) for what SDS maintains per message and how it
      orders: `lamport_timestamp`, `message_id`, `causal_history`, `bloom_filter`.
- [x] Read the Reliable Channel API spec for what the layer above SDS emits to an
      application. **Found the loss point**: `MessageReceivedEvent` carries the
      payload and nothing else.
- [x] Read `logos-delivery-module` at tag `v0.2.1` — the authoritative source, not
      the stale working tree §12 warns about — for what the module forwards.
      **Found `channelMessageReceived`'s `timestamp` is a local `CLOCK_REALTIME`
      read**, not the message's.
- [x] Check every other route: other events, methods/RPCs, the payload envelope,
      `storeQuery`. None carries it.
- [x] Check whether delivery arrives already in causal order. SDS gates delivery
      on causal dependencies, but that is a per-peer linearisation and says
      nothing about concurrent ops — which is the case §5.7 must resolve.
- [x] Corroborate against the Nim transport and the JS reference client. Both have
      the same gap; SDS is not yet wired into the Nim transport at all.
- [x] Conclude: **outcome (b)**. Recorded in `design.md` with citations.

## 2. The spec

- [x] `specs/op-ordering/spec.md` as a delta with `## ADDED Requirements`.
- [x] State the ordering rule as the transport's, not ours.
- [x] Require that absent metadata is represented rather than fabricated.
- [x] Define the degraded order, and require it be distinguishable from a real one.

## 3. The code

- [x] `arrival.rs`: `MessageId`, `Arrival`, `cmp_ops`.
- [x] Both metadata fields are `Option`, so the degraded path cannot be skipped by
      a caller who forgot to check.
- [x] No constructor derives a Lamport value — the property that keeps a second
      clock from existing.
- [x] `lib.rs`: one `pub mod` line.
- [x] `op.rs`: correct the doc comment that says the gap is Phase 2's to close.
- [x] Leave the op format untouched; `an_op_carries_no_ordering_fields` still
      passes unmodified.

## 4. Tests, and mutation-verifying them

- [x] Tests assert against hardcoded expectations, not against a second run of the
      implementation.
- [x] The two-ops fixture *determines* which op id is lower rather than assuming
      it, so a test cannot pass for the wrong reason.
- [x] Unspecified choices carried `// NO SPEC:` markers — three of them, all on
      partial-metadata cases. **None remain**: §8 records two promoted into the
      spec and one deleted as derivable. A grep for `NO SPEC` in this change now
      finds nothing but prose explaining their removal.
- [x] Mutation-verify every asserted property. Six mutations, each caught:

| # | Mutation | Tests that failed |
|---|---|---|
| 1 | Lamport compared ascending instead of descending | `a_higher_lamport_timestamp_orders_first`, `the_message_id_does_not_override_the_lamport_timestamp`, `sorting_puts_the_current_version_first`, `a_lamport_timestamp_without_a_message_id_still_orders` |
| 2 | Message-id tiebreak reversed to descending | `equal_lamport_timestamps_are_broken_by_ascending_message_id`, `a_message_id_is_compared_by_bytes_not_by_length` |
| 3 | Unordered ops sort above ordered ones | `an_op_the_transport_ordered_beats_one_it_did_not`, `a_message_id_without_a_lamport_timestamp_does_not_order` |
| 4 | Op-id last resort removed from the ordered branch | `the_order_is_total_over_distinct_ops` |
| 5 | `unordered()` fabricates `lamport: Some(0)` | `an_unordered_arrival_records_no_values_at_all`, `absence_is_not_equal_to_a_zero_lamport_timestamp`, `two_entries_sharing_an_op_id_can_tie_which_is_why_callers_dedup` |
| 6 | `is_ordered_by_transport` accepts a message id alone | `a_message_id_without_a_lamport_timestamp_does_not_order` |
| 7 | Boundary rule non-uniform outside the population's range (`lamport == 0` loses to unordered) | `an_op_the_transport_ordered_beats_one_it_did_not`, `a_message_id_without_a_lamport_timestamp_does_not_order` |
| 8 | Boundary rule non-uniform **within** the population (`lamport == 2` loses to unordered) | `the_order_is_transitive_across_every_combination`, `the_order_is_antisymmetric_across_every_combination` — **and nothing else** |

## 6. Review follow-up

- [x] **Finding 1 — `cmp_ops` had an unstated precondition.** Two records
      sharing an op id compare `Equal` despite differing metadata, because the
      op id is the last resort in every branch. Not a bug (§3.1 dedups by op
      id) but the doc comment promised totality unconditionally, and the store
      being built now is the caller that could violate it. Precondition stated
      in the doc comment, added to the spec as a requirement (it constrains
      callers, not the implementation), and pinned by
      `two_entries_sharing_an_op_id_can_tie_which_is_why_callers_dedup`.
- [x] **Finding 2 — transitivity was reasoned but not tested.** Added a
      deterministic 27-element population over every
      `lamport × message_id × op_id` combination, sweeping all 729 pairs for
      antisymmetry and all 19,683 triples for transitivity.
- [x] Captured the structural argument (two totally-ordered blocks, uniform
      boundary rule) in `design.md` and in the test's comment.

**Mutation 8 is the one that justifies the new tests.** It makes the boundary
rule consult a value rather than only presence, which produces a genuine
transitivity violation — `lamport(2) < lamport(1) < unordered`, yet
`lamport(2) > unordered`. Every pre-existing test passes under it. Only the two
exhaustive law tests fail.

## 7. Second review: two surviving mutations, both fixture-shape defects

Both survived 18/18 tests. Neither was a code defect — the code was already
correct, proven by an exhaustive external probe — but the suite could not have
detected a regression into either.

| # | Surviving mutation | Why it survived | Now caught by |
|---|---|---|---|
| C | `cmp_tiebreak`'s `(None, None)` arm returns `Less` instead of `Equal` | `ordered(..)` ALWAYS sets a message id and `unordered()` short-circuits before the tiebreak, so **no fixture ever built a lamport-only arrival**. The arm was unreachable from the suite, though `one_op_compared_against_itself_is_equal` and `the_order_is_the_same_whichever_way_the_pair_is_presented` both claimed the property by name. | `one_op_compared_against_itself_is_equal`, `the_order_is_the_same_whichever_way_the_pair_is_presented`, `the_order_is_antisymmetric_across_every_combination` |
| F | Op id checked BEFORE the message-id tiebreak | **Every test of the tiebreak compared two arrivals against the same `op.id()`**, so an op-id step returned `Equal` and fell through invisibly. The tiebreak was only ever exercised where the competing rule was silent. | `the_message_id_tiebreak_outranks_the_op_id_last_resort` |

Survivor C is a real `Ord` contract violation, not a cosmetic one: it makes
`cmp_ops` non-reflexive, which newer std can panic on in a debug build.

Survivor F is this repo's own recorded defect class in a new costume — the
`"ab"` vs `"abc"` shape, where a fixture is built so the two candidate
explanations cannot be told apart. The fix applies the "cross the two orderings"
technique already used in
`two_ops_with_message_ids_but_no_lamport_fall_back_to_op_id`, which had simply
never been applied to the ordered branch: two DISTINCT ops at equal Lamport,
where the lower-op-id op carries the HIGHER message id, so the rules disagree.

Both re-run after fixing; both now fail under their mutation.

**The general lesson, which is the transferable part**: a convenience
constructor that always sets a field (`ordered`) hides the arm where that field
is absent. Fixtures should reach the type's full shape through `from_parts`, not
only through the constructors the happy path uses.

## 8. Spec promotions from the second review

- [x] **Markers 2 and 3 promoted** into one requirement, "The Lamport timestamp
      alone decides whether an op is ordered". Partial metadata is not
      hypothetical: it is the shape the upstream fix would actually produce,
      since SDS sends ephemeral messages with the Lamport value unset and
      `design.md` proposes an optional `lamportTimestamp`. Which half decides
      orderedness was the most consequential unspecified decision in the change.
- [x] **Marker 1 deleted** rather than promoted — R5 makes such an op unordered
      and R4 orders unordered ops by op id, so the composition is derivable and
      the marker was over-cautious. The comment now says so.
- [x] R1's "signature unaffected" scenario removed and the remaining one marked
      structural: `Arrival` and `Op` are unrelated types, so these were
      type-level facts written as behavioural scenarios.
- [x] `nothing_here_can_produce_a_lamport_timestamp` folded into
      `an_unordered_arrival_records_no_values_at_all`. The name overstated the
      assertion — the property is enforced by the type's API surface, not by a
      runtime check — so the claim now sits in `Arrival`'s docs where it is
      true, and the test keeps only what it can witness.

Promoting the requirement changed **no test**. The behaviour was already
implemented and already covered; promotion moved the justification from a test
comment into the contract, which is where a decision of that consequence
belongs. The only test change in this round came from the survivors.

Mutation 7 is the same defect placed outside the population's Lamport range,
and it is caught by the semantic tests instead — which is the useful contrast:
the law tests are not a superset of the semantic ones, they cover a different
axis. Writing `distinct_op_ids_never_compare_equal_across_every_combination`
scoped to distinct *elements* first is also what surfaced Finding 1 independently
— it failed on exactly the pair the reviewer's probe found.

## 5. PLAN.md

- [x] Strike §13's question and record the answer in the style of the other
      answered entries.
- [x] Keep the reasoning in `design.md`, not duplicated into PLAN.md.

## What the green gate structurally cannot see

Recorded because a passing suite here proves less than it appears to.

- **That this order agrees with SDS's.** No SDS order reaches this code, so the
  agreement §5.7 depends on is untested and untestable here. The tests prove the
  rule is implemented as written and that it is peer-independent; they cannot
  prove the rule's inputs will be the transport's when the transport supplies
  them. The first test that could exist is an integration test against a delivery
  module that forwards the fields — which is the upstream change `design.md`
  specifies.
- **That either order is exercised in anger.** No op arrives with an `Arrival` at
  all: nothing outside `arrival.rs` constructs one from a received message,
  because no boundary wiring exists yet. So the suite covers both the transport
  order and the degraded order, and production exercises neither. When wiring
  lands it will construct `unordered()` until the upstream fields arrive, making
  the degraded path the only live one — but that is a prediction about a change
  not yet written, not a statement about today.
- **Whether `Arrival` is constructed correctly at the boundary.** Deliberately
  unwritten — there is nothing to decode until the transport forwards the fields.
  The op log consumes `Arrival` but does not build one from a message either.
