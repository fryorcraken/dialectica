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
- [x] Unspecified choices carry `// NO SPEC:` markers (three of them, all on the
      partial-metadata cases the contract cannot currently produce).
- [x] Mutation-verify every asserted property. Six mutations, each caught:

| # | Mutation | Tests that failed |
|---|---|---|
| 1 | Lamport compared ascending instead of descending | `a_higher_lamport_timestamp_orders_first`, `the_message_id_does_not_override_the_lamport_timestamp`, `sorting_puts_the_current_version_first`, `a_lamport_timestamp_without_a_message_id_still_orders` |
| 2 | Message-id tiebreak reversed to descending | `equal_lamport_timestamps_are_broken_by_ascending_message_id`, `a_message_id_is_compared_by_bytes_not_by_length` |
| 3 | Unordered ops sort above ordered ones | `an_op_the_transport_ordered_beats_one_it_did_not`, `a_message_id_without_a_lamport_timestamp_does_not_order` |
| 4 | Op-id last resort removed from the ordered branch | `the_order_is_total_over_distinct_ops` |
| 5 | `unordered()` fabricates `lamport: Some(0)` | `an_unordered_arrival_records_no_values_at_all`, `absence_is_not_equal_to_a_zero_lamport_timestamp`, `nothing_here_can_produce_a_lamport_timestamp` |
| 6 | `is_ordered_by_transport` accepts a message id alone | `a_message_id_without_a_lamport_timestamp_does_not_order` |

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
- **That the degraded order is ever exercised in anger.** Every op today arrives
  `unordered()`, so today the degraded path is the ONLY path. The suite covers
  both, but production currently exercises one.
- **Whether `Arrival` is constructed correctly at the boundary.** There is no
  boundary wiring yet, deliberately — nothing to decode. The first consumer will
  be the store.
