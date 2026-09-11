# Tasks

## 1. Settle the storage decision before writing any of it

- [x] Read §3.3 (the local store is ours), §4.7 (durability tiers), §5.7
      (revision ordering), §9 (build order), §13 (open questions).
- [x] Weigh SQLite now against a trait with an in-memory implementation now.
      **Chose the trait**, on §9 Phase 1 naming `Store` traits explicitly, on two
      resolver changes already consuming the contract in parallel, and on a
      database making Phase 1's "fakes with no node running" harder rather than
      easier. Recorded in `design.md` with what would reverse it.
- [x] Confirm `rusqlite` would be *permitted* (zero-SDK-types is about `lp_*`
      symbols, not ordinary crates) so the decision is a choice and not a
      constraint. It is a choice; no dependency added.

## 2. The spec

- [x] `specs/op-log/spec.md` as a delta with `## ADDED Requirements`.
- [x] Require that nothing is verified or filtered on append.
- [x] **Keep reasoning out of the spec.** The first draft carried a rationale
      paragraph under every requirement, several duplicating `design.md`
      verbatim. That is the two-copies-drift the document model exists to
      prevent, and design review caught it; the spec now states behaviour only,
      and the reasoning lives in `design.md` alone.
- [x] Require dedup by op id, and require the caller be told which happened.
- [x] Require that a re-arrival does not overwrite recorded metadata, and say
      which arrival wins. This is observable, so it belongs in the spec.
- [x] Require that dedup precede ordering, after the `cmp_ops` precondition was
      raised mid-change (see §4).
- [x] Require every read be defined over a partial set, with no error outcome.
- [x] Require ordered and unordered arrivals to coexist in one log, so the
      upstream fix needs no migration.

## 3. The code

- [x] `log.rs`: `Entry`, `Appended`, the `OpLog` trait, `MemoryOpLog`.
- [x] `MemoryOpLog` is the implementation AND the Phase 1 fake — not a
      `#[cfg(test)]` double that could drift from what ships.
- [x] `HashMap<OpId, Entry>`, so one op id is one entry by construction rather
      than by a guard at each insertion site.
- [x] `append` uses `entry()`'s occupied/vacant split, so there is no code path
      that overwrites an existing entry and therefore none that can replace
      recorded arrival metadata.
- [x] All three reads share one `sorted()` body, so they cannot disagree about
      what "in order" means.
- [x] No comparison of its own: ordering delegates entirely to `cmp_ops`.
- [x] Insertion order is unreachable through any method.
- [x] `Entry::target` maps kind to target in one place; a `Post`'s `parent` is
      deliberately not a target.
- [x] `lib.rs`: one `pub mod` line.
- [x] Op format, `arrival.rs` and transport wiring all untouched.

## 4. The `cmp_ops` precondition, raised mid-change

The coordinator reported an exhaustive-probe finding on the branch this forked
from: `cmp_ops` returns `Equal` for two entries sharing an `OpId` but carrying
different `Arrival` metadata, and its doc comment promised totality
unconditionally. Sorting a list built *before* dedup would therefore consult an
undefined order.

- [x] Confirm the log cannot build such a list. It cannot: entries are keyed by
      op id, and `sorted()` iterates map values.
- [x] Make that structural rather than incidental in the module documentation —
      the property is "there is no intermediate list", not "we remember to dedup".
- [x] Add `one_op_arriving_twice_with_different_metadata_is_still_one_entry`,
      using the exact pair the probe found (`unordered()` versus a message id
      with no Lamport value — both unordered, so both fall through to the op-id
      comparison and tie).
- [x] Add `no_two_entries_in_a_read_ever_share_an_op_id`, the general form, over
      a log built entirely from re-deliveries under varying metadata.
- [x] Decide and specify which arrival survives. **First wins**; the rejected
      alternative (prefer the richer arrival) and why it is wrong are in
      `design.md`.

## 5. Tests, and mutation-verifying them

- [x] Expectations hardcoded, never read back from the implementation.
- [x] **A count over a fixture is hardcoded, not derived from the fixture.**
      `nothing_is_filtered_on_the_way_in` asserts `log.len() == 6` rather than
      `every_op_kind().len()`, because a derived count agrees with the fixture
      however wrong the fixture becomes — a kind dropped from the list would
      silently lower both sides and the test would keep passing. Same property
      as pinning a constant with a hardcoded value instead of reading it back
      from the code that produced it. The pair must be updated together, and
      `Entry::target`'s exhaustive match is what forces the reminder.
- [x] The two-ops fixture *determines* which op id is lower rather than assuming
      it, following `arrival.rs`'s precedent.
- [x] Unspecified behaviour marked: one `// NO SPEC:` on
      `a_resolver_can_be_written_against_the_trait_alone`.
- [x] Mutation-verify every asserted property. Six mutations, each caught:

| # | Mutation | Tests that failed |
|---|---|---|
| 1 | `append` overwrites an existing entry and always returns `Stored` (last-write-wins) | `the_same_op_appended_twice_is_one_entry`, `the_append_result_says_whether_the_op_was_new`, `dedup_is_by_op_id_and_not_by_signature`, `a_second_arrival_does_not_overwrite_the_recorded_metadata`, `an_unordered_re_arrival_does_not_erase_a_recorded_order`, `a_re_arrival_does_not_move_the_op_in_the_read_order`, `one_op_arriving_twice_with_different_metadata_is_still_one_entry` |
| 2 | `sorted` orders by op id instead of delegating to `cmp_ops` | `reading_returns_ops_in_lamport_order_not_insertion_order`, `the_message_id_tiebreak_is_used_and_is_not_the_op_id`, `ordered_and_unordered_ops_coexist_with_the_ordered_ones_first`, `a_re_arrival_does_not_move_the_op_in_the_read_order`, `a_restricted_read_preserves_the_unrestricted_relative_order` |
| 3 | `append` rejects an op whose signature does not verify (filtering on write) | `an_op_with_an_invalid_signature_is_stored_anyway` |
| 4 | `Entry::target` returns a `Post`'s `parent` as its target | `a_post_names_no_target_and_is_never_returned_by_a_target_read`, `an_entry_reports_the_target_its_kind_names` |
| 5 | `iter_stoa` ignores the Stoa and returns everything | `a_stoa_restricted_read_excludes_other_stoas` |
| 6 | `iter_target` also matches an op by its own id | `a_target_read_does_not_return_the_target_itself`, `a_target_restricted_read_returns_every_kind_that_names_the_target`, `a_post_names_no_target_and_is_never_returned_by_a_target_read`, `a_restricted_read_preserves_the_unrestricted_relative_order` |
| 7 | `append` upgrades a stored entry when a richer arrival comes (richer-wins) | `a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one` — **and nothing else** |

**Mutation 7 survived the entire suite when review ran it, and that was a real
hole.** Richer-wins is the alternative both this change's `design.md` and
`arrival.rs`'s module docs name as the tempting wrong answer, and all 30 log
tests passed against it. The cause was a fixture trap of the same family as
mutation 2's: **every re-arrival test delivered the ordered copy FIRST**, and in
that direction first-wins and richer-wins give the same answer. The one direction
that separates them — unordered first, then ordered — had no test.

Worse, `an_unordered_re_arrival_does_not_erase_a_recorded_order` called itself
"the sharp direction" in a comment while being the blunt one. A comment asserting
a test's discriminating power, on a test that has none, is the failure this
project's test discipline exists to catch.

Fixed by adding `a_richer_re_arrival_does_not_upgrade_a_poorer_recorded_one`,
verified failing against richer-wins (`left: Some(7), right: None`) and passing
against the shipped code, and by correcting the blunt test's comment to say which
mistake it actually catches (last-write-wins) and which it cannot.

| # | Mutation | Tests that failed |
|---|---|---|
| 8 | `iter_stoa` matches only the first BYTE of the Stoa address | `two_stoas_sharing_an_address_prefix_are_not_confused` — and nothing else |
| 9 | `iter_target` matches only the first BYTE of the target op id | `two_targets_sharing_an_op_id_prefix_are_not_confused` — and nothing else |

**Mutations 8 and 9 also survived the whole suite before these tests existed, and
they are the most serious misses of the change.** They are rows 5 and 6 blunted:
those mutations are *maximally* wrong (ignore the key entirely; match an op by its
own id) and so die easily, which made the coverage look better than it was. What
was tested was "does it filter at all?", never "does it filter on the **whole**
key?" — and every fixture used two subjects without ever checking they collide in
no prefix.

A prefix-matching read is a cross-Stoa and cross-target leak in a
censorship-resistant forum: `iter_stoa` would bleed another Stoa's ops into a
Stoa read, and `iter_target` would hand a moderation resolver ops aimed at a
**different post**, so one post's `Hide` silently hides an unrelated one. With
32-byte identifiers a collision is rare enough never to surface in testing and
certain enough to surface eventually.

Reachability was **probed, not assumed**: over 300 simple fixture titles there
are real first-byte collisions at `("S0","S178")` (address byte 0 = 98) and
`("p0","p37")` (op id byte 0 = 8). Those exact fixtures are what the two new
tests use. Because the collisions are hash-derived, each test carries a **fixture
guard** — `assert_eq!` on the colliding byte and `assert_ne!` on the full values —
so that an encoding change breaks the fixture loudly instead of silently
retiring the property. That is the "a new check can retire an old test" hazard
applied to our own fixture.

Rows 5 and 6 are kept rather than replaced: the all-or-nothing mutations are still
true, just coarse, and the pair of rows is the useful record of how coarse.

**Mutation 2 found a real defect in the tests, which is why it is worth doing.**
On its first run only ONE test failed. The Lamport-ordering fixtures used ops
whose op-id order happened to coincide with their Lamport order, so a log that
ignored the arrival metadata entirely and sorted by op id agreed with them — the
exact "test that cannot fail for the reason it names" defect the agents README
records three instances of. Fixed by assigning Lamport values and message ids
*against* op-id order, using the existing `two_posts_by_ascending_id` fixture, and
by adding `the_message_id_tiebreak_is_used_and_is_not_the_op_id`. The mutation
then failed 5 tests.

## 6. PLAN.md

- [x] One line in §3.3 that the op log exists, per the document model.
- [x] Reasoning stays in `design.md`; not duplicated into PLAN.md.

## The pattern behind mutations 2, 7, 8 and 9

Four of the nine mutations initially survived, and all four failed the same way:
**a fixture that varied the subject but not the property under test.**

- Mutation 2: Lamport values assigned *along* op-id order, so ignoring the
  metadata agreed with the expectation.
- Mutation 7: every re-arrival delivered the ordered copy *first*, the one
  direction where first-wins and richer-wins agree.
- Mutations 8 and 9: two subjects that differ in the first byte, so a read
  comparing one byte behaves exactly like one comparing all 32.

In each case the test named the right property and the fixture could not
distinguish it. The generalisation, which is worth more than the four fixes: **a
test of a discriminating rule must use inputs on which the rule and its most
plausible wrong neighbour disagree.** Choosing inputs that merely satisfy the rule
is how a suite reaches a high count while pinning very little, and a mutation run
is the only thing that surfaces it — the tests all passed, in every case, for the
entire time they were wrong.

## What the green gate structurally cannot see

Recorded because a passing suite here proves less than it appears to.

- **That the log holds what a real peer would hold.** Nothing constructs an
  `Arrival` from a real `channelMessageReceived` yet, and no op has ever reached
  this code from a network. Every test appends ops this process just signed.
- **That the ordered path is ever exercised in anger.** Production constructs
  `Arrival::unordered()` for everything today, so the degraded order is the only
  order a running peer uses. The suite covers both branches; a peer exercises one.
  This is inherited from `op-ordering` and is not fixable here.
- **That the first-wins rule is the right choice.** It is a decision, not a
  derivation. The tests pin that it *is* first-wins; they cannot tell you that
  last-wins would have been worse, because both converge as long as every peer
  makes the same choice. The argument in `design.md` is what carries that weight.
- **Whether `Vec<&Entry>` survives a real peer's history.** Every read allocates a
  vector of references to the whole matching set and sorts it. At the sizes a test
  builds this is free; at the size of a Stoa with months of history it may not be,
  and the suite would not notice. Named in `design.md` as the part of the trait
  most likely to need redrawing when the SQLite implementation lands.
- **That two peers actually converge.** `two_logs_with_the_same_ops_read_the_same_order`
  builds two logs in one process from one set of ops. It proves the read order does
  not depend on insertion sequence; it cannot prove two *machines* agree, which
  needs the transport that does not yet supply the metadata.
