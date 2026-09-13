# Findings: spec-test review — `stoa-lifecycle` / `stoa-membership`

Reviewed blind to the implementation: the spec at
`openspec/changes/stoa-lifecycle/specs/stoa-membership/spec.md`, and the `mod tests`
blocks in `membership.rs`, `wire.rs` and `keystore.rs`. `design.md` was not read.

Two implementation bodies were read, and each reading is itself reported below as a
finding — `MembershipStore::join`'s guard (entry 2) and `join_stoa`'s body (entry 2),
because a mutation result could not be explained without them. Signatures elsewhere
were read to know what a test can call.

Baseline: **550 tests pass**. Every mutation listed was restored and the suite
re-run to 550 green; `git status --short` is empty.

`openspec validate stoa-lifecycle --strict` passes.

---

## Coverage map

Requirement → scenario → the test(s) pinning it.

### Creating a Stoa produces a genesis record the creator can moderate

| Scenario | Test |
|---|---|
| Creation returns the address of the record it built | `wire.rs:1969` `creation_returns_the_address_of_the_record_it_built` — expected address derived independently from a hardcoded title + key, not read back from the reply |
| The creator is the caller's own key | `wire.rs:2042` `the_creator_is_the_callers_own_key_and_no_creator_is_accepted_from_the_request`; `wire.rs:2000` `the_creator_key_is_whatever_the_lookup_supplies_and_this_handler_chooses_none`; `keystore.rs:3022` `the_creator_of_a_stoa_this_keystore_made_can_moderate_it`; `keystore.rs:2937` `the_identity_key_is_pinned_to_a_known_answer` (frozen hex) |
| The created Stoa is listed immediately | `wire.rs:2085` `a_created_stoa_is_listed_immediately_with_no_op_having_arrived` — **but see entry 4**: "without any op having been received" is not what this test exercises |
| "The record MUST be retained" (requirement prose, no scenario of its own) | `wire.rs:1995-1996` (reads the record back out of the store and checks it verifies); `membership.rs:1199` `the_retained_record_still_verifies_against_its_address` |

### Creating a Stoa requires a usable signing key, and says so rather than inventing one

| Scenario clause | Test |
|---|---|
| the call reports a failure | `wire.rs:2102` `creation_without_a_usable_key_fails_and_records_nothing` — all four `KeystoreError` variants, error message asserted against the keystore's own |
| no Stoa is recorded | same test, `store.len() == 0` |
| no key was created as a side effect | **none — known-untestable, already routed to `spec-writer`.** Not re-litigated. |

### A title the genesis record cannot carry is refused before a Stoa exists

| Scenario | Test |
|---|---|
| An over-long title creates nothing | `wire.rs:2139` `an_over_long_title_creates_nothing` (1025); `membership.rs:1045` `an_unencodable_record_is_refused_before_anything_is_written`; `membership.rs:1066` `an_encode_failure_does_not_report_itself_as_a_failure_to_read` |
| A title at the maximum length creates a Stoa | `wire.rs:2156` `a_title_at_the_maximum_length_creates_a_stoa` — 1024 hardcoded. **See entry 5**: the spec's "length" is unit-ambiguous and only ASCII is exercised |
| An empty title is accepted rather than refused | `wire.rs:2177` `an_empty_title_is_accepted_rather_than_refused`; `membership.rs:1601` `an_empty_title_is_recordable_and_reads_back_empty` |
| A title carrying control or bidirectional characters is not rejected | `wire.rs:2193` `a_title_carrying_control_or_bidirectional_characters_is_not_rejected_for_that_reason` (also checks the listing path); `membership.rs:1616` `a_title_carrying_bidi_and_zero_width_characters_is_retained_unchanged` |

### Creating the same Stoa twice yields one Stoa, not two

| Scenario | Test |
|---|---|
| The same creator and title reach the same Stoa | `wire.rs:2215` `the_same_creator_and_title_reach_the_same_stoa`; `keystore.rs:3060` `the_identity_key_is_stable_and_differs_between_keystores` (the premise) |
| Two titles are two Stoas | `wire.rs:2238` `two_titles_are_two_stoas` |

### Joining takes an address and the record it names, and verifies rather than trusts

| Scenario | Test |
|---|---|
| A matching record joins | `wire.rs:2267` `a_matching_record_joins_and_the_reply_carries_the_address_and_founding_title`; `membership.rs:993` `a_matching_record_is_recorded_and_read_back` |
| A record that does not match the address is refused | `wire.rs:2288` `a_record_that_does_not_match_the_address_is_refused_and_joins_neither_stoa`; `membership.rs:1009` `a_record_differing_in_any_field_is_refused_and_records_nothing`; `membership.rs:1141` `a_join_cannot_overwrite_another_stoas_retained_record`. **See entry 2** — the wire half of this cannot fail for the reason it names |
| A malformed record is refused without a membership | `wire.rs:2323` `a_malformed_record_is_refused_without_a_membership` — five byte strings, four distinguishable refusals asserted; an all-zero (low-order-point) creator among them |
| Verification consults nothing but the two inputs | `wire.rs:3291` `a_join_verified_at_the_wire_needs_no_op_log_and_no_prior_membership`; `membership.rs:1169` `verification_consults_only_the_two_inputs`. **See entries 2 and 4** |
| "A join MUST return what the caller needs to show what was joined" | `wire.rs:2278-2279` (address and founding title, both against literals) |

### Joining a Stoa the peer is already in changes nothing and is not a failure

| Scenario | Test |
|---|---|
| A repeated join is idempotent | `wire.rs:2385` `a_repeated_join_succeeds_and_leaves_one_membership_unchanged`; `membership.rs:1102` `a_repeated_join_is_idempotent_and_leaves_the_record_untouched` (the `Joined::AlreadyIn` value is the actual kill, and the test says so) |
| Creating and then joining the same Stoa is one membership | `wire.rs:2414` `creating_and_then_joining_the_same_stoa_is_one_membership` |
| "MUST NOT disturb what was already retained" **at the wire** | **none — known-untestable, already routed.** Pinned at the store layer by `membership.rs:1102`. Not re-litigated. |

### A joined Stoa's genesis record is retained, not only its address

| Scenario | Test |
|---|---|
| The retained record still verifies against its address | `membership.rs:1199` `the_retained_record_still_verifies_against_its_address` (across a reopen); `wire.rs:3161-3172` |
| Founding values are answerable from what was retained | `membership.rs:1227` `the_founding_title_and_policy_are_answerable_from_what_was_retained` (title hardcoded, policy asserted) |
| "…and no network call is needed" | Pinned only by construction (the store has no network reach). Structurally unobservable; acceptable, same class as the two already-routed items — mentioned here for completeness, not raised as an entry. |

### Membership survives a restart

| Scenario | Test |
|---|---|
| Created and joined Stoas both survive | `wire.rs:3092` `a_created_and_a_joined_stoa_both_survive_a_restart_with_their_founding_values` (hardcoded titles); `membership.rs:1316` `memberships_outlive_the_store_object_that_recorded_them` |
| The retained record survives too | `wire.rs:3158-3172`; `membership.rs:1199` |
| A refused join leaves nothing behind a restart | `wire.rs:3176` `a_join_refused_at_the_wire_leaves_nothing_behind_a_restart` (with a survivor Stoa); `membership.rs:1344` `a_refused_join_leaves_nothing_behind_a_restart` |

### Adding membership does not make an existing store unreadable

| Scenario | Test |
|---|---|
| A store holding ops and no memberships opens | `wire.rs:2920` `a_store_holding_ops_and_no_memberships_opens_and_keeps_its_ops` — real `SqliteOpLog`, ops written first, bodies read back off disk against hardcoded literals, listing asserted empty |
| A membership is recordable into such a store | `wire.rs:2974` `a_membership_is_recordable_into_a_store_that_previously_held_none`; supported by `wire.rs:3034` `the_membership_store_does_not_write_into_the_op_logs_file` (byte-for-byte file comparison) |

**Verdict on the brief's "check this hard" item: genuinely covered.** Both scenarios
fail under mutation (M1, M5 below), and the three tests are not redundant with each
other — M5 kills the second scenario's test while the first survives, which is
correct rather than a gap. The in-memory blind spot is real elsewhere though — see
entry 4.

### Listing reports the Stoas the peer is in, and no others

| Scenario | Test |
|---|---|
| Only joined and created Stoas are listed | `wire.rs:2530` `an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership` + `wire.rs:2974` (the listing is exactly the joined Stoa, not the one the op names). **See entry 4** |
| A peer in no Stoa lists nothing | `wire.rs:2490` `a_peer_in_no_stoa_lists_nothing_and_reports_no_failure`; `membership.rs:1367` |
| Every Stoa is reachable by paging and appears once | `wire.rs:2506` `every_stoa_is_reachable_by_paging_and_appears_once` (7 over pages of 3); `wire.rs:3248` `a_listing_page_that_is_not_the_last_says_so_on_the_wire`; `membership.rs:1378`, `membership.rs:1429` |
| Each item carries the address, not only the title | `wire.rs:2434` `the_listing_envelope_is_the_ecosystems_pagination_shape` |

### A listed title is a founding title, and is identified as such

| Scenario | Test |
|---|---|
| The reported title is marked as founding | `wire.rs:2460` `a_listed_title_is_named_as_founding_and_never_as_a_bare_title` — asserts `foundingTitle` present **and** bare `title` absent, on both the listing and the join reply |

### An op for a Stoa the peer is not in creates no membership

| Scenario | Test |
|---|---|
| A gossiped op does not enrol the peer | `wire.rs:2530` — **entry 4: cannot fail for the reason it names** |
| Many ops for an unjoined Stoa still enrol nobody | `wire.rs:2530` — same |
| An op for an unjoined Stoa does not disturb the memberships that exist | `wire.rs:2574-2606` — same |

### Membership is not lost because a Stoa has no ops

| Scenario | Test |
|---|---|
| A Stoa with no ops is still joined | `wire.rs:2085`, `wire.rs:3092` (survives restart with no op ever) |
| An empty op log does not empty the listing | `wire.rs:2610` `an_empty_op_log_does_not_empty_the_listing`; `membership.rs:1587` `membership_is_not_lost_because_a_stoa_has_no_ops` — **entry 4** |

### Every one of these calls answers in the module's failure shape and never aborts

| Scenario | Test |
|---|---|
| A hostile request is an error rather than an abort | `wire.rs:2628` `a_hostile_request_is_an_error_rather_than_an_abort_and_carries_no_result` — 6 create / 10 join / 6 list shapes, every one asserted error-only, plus a good call afterwards |
| A failed call records nothing | `wire.rs:2708` `a_failed_call_records_nothing_and_disturbs_no_retained_record` — over a store that already holds a membership, which is the half an empty-store test cannot see |

**No requirement in this spec has zero tests.** Three individual scenario clauses
have none: the two already routed to `spec-writer`, and the three scenarios under
"An op for a Stoa the peer is not in creates no membership" (entry 4), which have
tests that do not exercise them.

---

## Findings

- [ ] **1. `spec-writer` — the whole "op does not create membership" requirement is untestable as written: there is no receive path**

The requirement and all three of its scenarios are phrased around an op **reaching
the peer**: "*WHEN* an op addressed to a Stoa the peer is not in reaches the peer",
"*WHEN* many ops … reach the peer", "*WHEN* a peer in one Stoa receives an op".

There is no receive path on this surface. `grep` for `fn receive`, `on_message`,
`fn ingest`, `fn handle_incoming` across `dialectica/rust-lib/src/lib.rs` and
`dialectica-core/src/lib.rs` returns nothing. Nothing in this module can be handed
an inbound op; ops only exist because a test constructs and signs them and appends
them to a log the membership code cannot reach.

So the scenarios describe behaviour at a boundary that does not exist yet — the
README's own rule: *"behaviour that does not exist yet cannot be covered."* This is
the **third** untestable item, distinct from the two already routed, and it is a
larger one: it is a whole requirement with three scenarios rather than one clause.

What *is* checkable today, and what the tests actually check, is the structural
fact: `list_stoas` is handed a `MembershipStore` and there is no path from one to an
op log, so membership cannot be derived from ops. That is worth specifying — but as
a statement about the listing's material, not as a statement about an arriving op.
Suggested reshape: keep the requirement's *argument* (it is the security argument
and it is good), state the checkable half ("the listing's contents are exactly what
membership records; nothing in this capability reads the op log"), and say plainly
that the arriving-op direction becomes testable when a receive path exists.

**Failure scenario:** none needed — the finding is that no scenario here can be
reproduced, because the `WHEN` clause names an event the module cannot experience.

`specs/stoa-membership/spec.md:265-288`

**Outcome: OPEN — `spec-writer`'s, and the box stays unticked.** Noted by `dev-writer`
so it is not read as forgotten.

The structural claim is confirmed by measurement, not accepted: `grep` for `fn receive`,
`on_message`, `fn ingest`, `fn handle_incoming` across both crates returns nothing, and
`list_stoas` is handed a `MembershipStore` with no path from one to an op log. There is
no boundary for an arriving op to cross.

**What changed on the test side, which narrows but does not close this.** Entry 4's fix
made the checkable half actually checked: `an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership`
and `an_empty_op_log_does_not_empty_the_listing` now put a real `SqliteOpLog` in the
same directory as the membership store, so an implementation that derived membership
from what it found there is caught — verified by mutating `MembershipStore::open` to do
exactly that. That is the entry's own "suggested reshape" exercised from the test side:
*"the listing's contents are exactly what membership records; nothing in this capability
reads the op log."*

What remains, and why it is not mine: the requirement's three scenarios are phrased
around an op **reaching the peer**, and rewording them — keeping the security argument
while stating the checkable half and saying plainly that the arriving-op direction
becomes testable when a receive path exists — is a change to the spec.

---

- [x] **2. `tester` — the four wire-level verification tests cannot fail for the reason they name: a second, redundant guard catches everything**

**Mutation M2, run:** in `membership.rs:405`, replaced the self-authenticating guard
`if !genesis.matches(stoa)` with `if false`, deleting the store's verification
entirely.

**Predicted:** every test of "a record that does not match the address is refused",
at both layers, fails.

**Observed:** only the four `membership.rs` tests failed —
`a_record_differing_in_any_field_is_refused_and_records_nothing`,
`a_join_cannot_overwrite_another_stoas_retained_record`,
`verification_consults_only_the_two_inputs`,
`a_refused_join_leaves_nothing_behind_a_restart`. **546 passed.** Every wire-level
test survived, including the three that name verification in their own names:

- `wire.rs:2288` `a_record_that_does_not_match_the_address_is_refused_and_joins_neither_stoa` — re-run alone, passes
- `wire.rs:3291` `a_join_verified_at_the_wire_needs_no_op_log_and_no_prior_membership`
- `wire.rs:3176` `a_join_refused_at_the_wire_leaves_nothing_behind_a_restart`

Explaining that survival required reading `join_stoa`'s body, which is itself the
finding the brief describes — *"If you find yourself needing the body to answer a
question, that is itself a finding: it means the test does not stand on its own."*
The body (`wire.rs:690-706`) says so explicitly: `genesis_for` already verifies
(`wire.rs:326`), and `store.join` verifies *again*, with a comment arguing the
redundancy is deliberate.

**The redundancy is defensible; the test coverage of it is not.** Two guards enforce
one property and **no test distinguishes them**, so either one can be deleted and
the suite stays green in the layer the deletion did not touch. The wire tests read
as "the wire refuses a mismatched record" and in fact only establish "*something*
somewhere refuses it" — the classic two-explanations-give-the-same-answer shape this
project keeps paying for, in its sharpest form: the two explanations are two live
code paths.

Concretely: delete `wire.rs:326`'s check and the four `membership.rs` tests still
pass while the wire tests pass too (via the store). Delete the store's check and the
wire tests still pass (via `genesis_for`). Neither guard is pinned by a test that
fails when only that guard goes.

**What would fix it:** one test per guard that reaches only that guard. The store's
is already reachable (`membership.rs` calls `join` directly — those four tests are
the ones that failed, and they are the honest half). The wire's needs a case where
`store.join` would *not* refuse — which, given both guards check the same predicate,
means the test has to assert on something other than the outcome: that the refusal
carries `genesis_for`'s message rather than `MembershipError::RecordDoesNotMatch`'s,
for instance. `wire.rs:3354-3357` already hardcodes the expected message — and the
message it expects is the **store's** (`"the genesis record does not hash to the
Stoa address it was given with"`, `membership.rs:196-199`), which is why that test
survives M2's removal of the store's guard: `genesis_for` must render the same
sentence. If the two guards' messages are identical, that is worth knowing too, and
worth a test.

`wire.rs:2288`, `wire.rs:3291`, `wire.rs:3176`, `wire.rs:2323`

**Outcome: FIXED, by reshaping rather than by the test this entry proposes — and the
proposed test provably could not have worked.**

Addressed by `dev-writer` rather than `tester` because the answer turned out to be a
code change: the reason no test could pin either guard is that the two guards existed.

**First, I ran the other direction of M2, which this entry did not.** Deleting
`wire::genesis_for`'s guard (`if !genesis.matches(stoa)` → `if false`) leaves
**550 of 550 passing**. So the finding is symmetric and worse than reported: the store's
guard was held by four tests, and the wire's by **none**. Each was covered only by the
other still being there.

**Why the suggested fix cannot work.** The entry proposes asserting *"that the refusal
carries `genesis_for`'s message rather than `MembershipError::RecordDoesNotMatchAddress`'s"*.
It then half-discovers the obstacle — noting `wire.rs:3354` already expects the store's
sentence and that `genesis_for` "must render the same sentence" — and
`findings/readability.md` entry 7 completes it: the string *"the genesis record does not
hash to the Stoa address it was given with"* was **hardcoded twice, byte-identically, in
two modules.** No assertion on the reply can distinguish which guard fired, so no test
at the wire could have been written to pin the wire's guard.

**What was done instead**, per CLAUDE.md's preference for an invariant that holds by
construction over a branch that checks it. `Membership` was already the verified pair —
`list` returns it, and its doc already claimed the store guarantees the two agree.
`Membership::verified(&Address, &Genesis)` is now the only way to build one from a
caller's pair, `MembershipStore::join` takes a `&Membership` and has **no guard**, and
`genesis_for` returns the pair so verification happens once per request rather than
twice.

**The measurement that shows the entry's defect is gone.** Deleting the one remaining
guard now fails **8 tests across both layers** — the four in `membership.rs` and four at
the wire, including all three this entry names as surviving M2:
`a_record_that_does_not_match_the_address_is_refused_and_joins_neither_stoa`,
`a_join_verified_at_the_wire_needs_no_op_log_and_no_prior_membership`, and
`a_join_refused_at_the_wire_leaves_nothing_behind_a_restart`. Before: 4 failures or 0,
depending on which copy you deleted. There is no deletion left that leaves the property
untested.

Two consequences recorded rather than glossed. The mismatch tests now assert against
`Membership::verified`, which is the honest site since that is where refusal lives. And
`verification_consults_only_the_two_inputs` got **weaker**: comparing an empty store's
answer against a populated one's was a real test while verification had a `self` to
reach through, and is satisfied by construction now that it is an associated function
with no store — the test says so and keeps only the refusal assertion that remains
observable. 29 test call sites changed; `design.md` carries the full reasoning under
*"Verification is a constructor, not a guard"*.

---

- [ ] **3. `spec-writer` — the spec forbids refusing a store "on the grounds that it predates membership", and the store refuses several such stores**

"Adding membership does not make an existing store unreadable" says: *"Opening such
a store MUST NOT be refused on the grounds that it predates membership."*

Eleven tests in `membership.rs` pin refusals at open that the spec never mentions,
and two of them are refusals of files that *do* predate this build:

- `a_store_from_an_unknown_layout_version_is_refused_and_names_both_numbers` (`membership.rs:806`)
- `a_store_from_an_older_layout_version_is_refused_too` (`membership.rs:829`) — an **older** layout is exactly a store that predates something
- `a_store_stamped_with_our_version_but_missing_the_table_is_refused_at_open` (`membership.rs:860`)
- `a_store_whose_table_lost_a_column_is_refused_too` (`membership.rs:892`)
- `a_fresh_store_is_created_rather_than_refused` (`membership.rs:784`) — the one case the spec's sentence *does* cover, and the tests treat `user_version == 0` as the "predates membership" sentinel

The spec has **no requirement about a layout version at all**, in either direction.
The behaviour looks right (refusing both directions is the op log's position, and
the reasoning in the code is careful), but nobody decided it against this contract,
and the one sentence that touches the area reads as forbidding part of it. A reader
of the spec alone would conclude a store stamped with an unknown version must open.

This is the `NO SPEC:` family without the marker — behaviour a test pins that no
scenario describes, which the brief says is worth more attention rather than less.
The distinction the spec needs: *membership state's absence* never refuses (that is
the requirement as written, and it is honoured), while a *membership store* claiming
a layout this build does not have is refused in both directions.

**Failure scenario:** a user upgrades, downgrades, and reopens. The downgraded build
refuses `stoas.sqlite` with `UnknownLayoutVersion`. Whether that is correct is
unanswerable from the spec, and `a_store_from_an_older_layout_version_is_refused_too`
pins the answer permanently without a requirement behind it.

`specs/stoa-membership/spec.md:202-206`; `membership.rs:784`, `806`, `829`, `860`, `892`

**Outcome: OPEN — `spec-writer`'s, and the box stays unticked.** Noted by `dev-writer`.

The distinction the entry proposes is the right one and I want to endorse it explicitly,
because it is the sentence a spec-writer can act on: *membership state's absence* never
refuses (which is the requirement as written, and the code honours it — `user_version ==
0` creates the schema rather than refusing), while a *membership store claiming a layout
this build does not have* is refused in both directions.

Nothing here is `dev-writer`'s to fix. The behaviour is what the entry says it is, and
it is behaviour the entry agrees looks right; what is missing is a requirement, and
inventing one in code would be deciding the contract from the implementation — the
inversion this flow exists to prevent. Per my own agent file, an unspecified *observable
behaviour* belongs in the spec, not in my head.

One correction to the entry's framing, which does not change its conclusion.
`a_fresh_store_is_created_rather_than_refused` is described as treating `user_version ==
0` as "the 'predates membership' sentinel". It is slightly stronger than a sentinel: 0 is
the value SQLite reports for a database nothing has stamped, so it is the *absence* of a
version rather than a chosen marker — which is why "there is nothing to convert, because
this file did not exist" holds without a migration. Worth knowing when writing the
requirement, since it is what makes the no-migration claim structural.

---

- [x] **4. `tester` — every op-log fixture in the membership tests is decorative; three tests pass with zero ops**

The tests for the two op-log-boundary requirements build a `MemoryOpLog`, append ops
to it, assert the count, and then assert things about a `MembershipStore` that has no
connection to that log. `wire.rs:2539-2540`'s own comment concedes it: *"the two
types share no state at all."*

**Mutation M4, run:** in `wire.rs:2548`, changed the op-population loop from `0..5`
to `0..0` and the fixture assertion at `wire.rs:2563` from `5` to `0` — so the
"many ops addressed to a Stoa the peer is not in" are now **no ops at all**.

**Predicted:** `an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership` fails,
since its name and all three of its scenarios are about ops existing.

**Observed:** **passes.** The test named for ops enrolling nobody proves nothing
about ops. Its real content is `store.contains(unjoined) == false` on a store into
which nothing was ever joined — true of any empty store.

The same shape, unmutated but visible by reading:

- `wire.rs:2610` `an_empty_op_log_does_not_empty_the_listing` — creates a
  `MemoryOpLog`, asserts `len() == 0`, and never touches it again. The log's
  emptiness cannot affect a listing that never consults a log. The surviving
  assertion (`listed(&store,20).len() == 3`) is the same assertion
  `every_stoa_is_reachable_by_paging_and_appears_once` already makes.
- `wire.rs:3318-3340`, the third case of
  `a_join_verified_at_the_wire_needs_no_op_log_and_no_prior_membership` — a real
  on-disk `SqliteOpLog` with three ops for the Stoa under verification, opened in a
  block and dropped. `join_stoa` is never given it. The case differs from the second
  only in the membership store's contents.
- `membership.rs:1587` `membership_is_not_lost_because_a_stoa_has_no_ops` — honest
  about itself (*"this store holds no ops and has no way to reach any"*), which makes
  it a restatement of `every_stoa` rather than a test of the requirement.

`a_membership_is_recordable_into_a_store_that_previously_held_none` (`wire.rs:2974`)
is the **one** test in this family that is not decorative: the op store and the
membership store are the same directory on a real disk, the op's Stoa and the joined
Stoa differ, and `wire.rs:3027-3030` asserts the fixture's two Stoas differ so the
listing assertion means something. It is doing the work the other four claim to.

Related to entry 1: these fixtures are decorative *because* there is no receive path,
so this is the same defect seen from the test side. But the test-side fix is
independent and worth doing either way — a fixture that cannot affect the assertion
should not be in the test, because it tells the next reader the property is covered.

`wire.rs:2530`, `wire.rs:2610`, `wire.rs:3318`, `membership.rs:1587`

**Outcome: FIXED**, all four sites, and the entry is right on every one of them.
Addressed here rather than by `tester` because three of the four needed the
fixture to be rebuilt around a real on-disk store, which is a change to what the
test *does* and not to what it asserts.

The rule this entry rests on, and the one I applied: **a fixture that cannot
affect the assertion should not be in the test.** A `MemoryOpLog` beside a
`MembershipStore` shares no state, so ops in it cannot reach anything — the fix is
to put the ops where an implementation that went looking would find them, which is
the same directory as the membership store on a real disk.

- **`an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership`** — rebuilt on a
  real `SqliteOpLog` in the directory the membership store is opened in, reached
  through `with_membership_store` / `list_stoas`. Also now asserts the error arm
  explicitly, because a derived listing can fail either by listing a Stoa nobody
  joined or by choking on what it derived.
- **`an_empty_op_log_does_not_empty_the_listing`** — same, the other direction: a
  real op store that exists, opens and holds nothing, in the same directory.
- **`a_join_verified_at_the_wire_needs_no_op_log_and_no_prior_membership`**, third
  case — was an in-memory store beside an on-disk log, so the ops sat somewhere the
  store could not have reached even in principle. Now on disk, in that directory.
  **This entry's sibling defect, not listed here but the same one:** `wire.rs:3337`
  carried a second copy of *"the fixture must reach the assertion"*, and there it
  was not merely false but backwards — reaching the assertion is exactly what must
  not happen, since the assertion is that the three replies agree. Corrected.
- **`membership_is_not_lost_because_a_stoa_has_no_ops`** — **deleted**, with the
  reasoning left in a comment where it was. The entry is right that it was a
  restatement of `every_stoa`; on checking,
  `every_stoa_is_reachable_by_paging_and_appears_exactly_once` already asserts the
  same thing over a larger population and a page size that does not divide it, so a
  rewrite would have been a third copy. The op-log property is labelled
  satisfied-by-construction at that layer with what makes the absence real:
  `list`'s only material is `self.conn`, `open`/`in_memory` are the only
  constructors and neither can be handed a log, and `grep -n "OpLog\|ops.sqlite"
  membership.rs` returns nothing outside comments.

**The tests that fail without the fix, both mutations run.** Mutating
`MembershipStore::open` to derive membership from a sibling `ops.sqlite`:
`an_op_for_a_stoa_the_peer_is_not_in_creates_no_membership` **fails**. Mutating it
the other way — `DELETE FROM stoas` when the sibling log is empty:
`an_empty_op_log_does_not_empty_the_listing` **fails**, `left: 0, right: 3`. Both
old versions passed under both mutations. Also re-ran this entry's own `0..5` →
`0..0`, which the old test survived and the new one fails on.

`a_membership_is_recordable_into_a_store_that_previously_held_none` is left alone —
this entry is right that it was already the honest one, and it is now the shape the
other three follow.

---

- [ ] **5. `spec-writer` — "maximum length" does not say bytes or characters, and no test at this surface exercises the difference**

Two scenarios turn on a length bound:

- *"a title longer than a genesis record can carry"*
- *"a title of exactly the maximum length a genesis record carries"*

The bound is in **bytes** — `stoa.rs:103` `MAX_TITLE_BYTES: usize = 1024`, checked as
`title.len() > MAX_TITLE_BYTES` (`stoa.rs:261`), and the error renders "title is {n}
bytes". The spec says "length", which in a JSON string surface a view author will
read as characters.

Both boundary tests use `"x".repeat(1024)` and `"x".repeat(1025)` — ASCII, where
bytes and characters coincide, so the distinction is never exercised at this surface.
`wire.rs:2173` even asserts `reply[FOUNDING_TITLE].as_str().unwrap().len() == 1024`,
which is a Rust byte length that happens to equal the character count for this
fixture.

**Failure scenario, reproducible:** create a Stoa titled with 400 CJK characters
(1200 bytes). The call fails. A view that counted characters against a "1024
maximum" it read from this spec shows the user a title well inside the limit and an
error it cannot explain. Nothing in the spec or in these tests says which count the
user is up against.

The requirement also correctly defers the bound itself to `stoa-genesis` — so the
fix may belong there, with this capability's scenarios saying "in bytes" so the
ordering requirement they exist for is unambiguous. A test with a multi-byte title
at 1024 bytes (fewer characters) and one at 1025 bytes would pin it.

`specs/stoa-membership/spec.md:69, 75`; `wire.rs:2156`, `wire.rs:2139`

**Outcome: OPEN — `spec-writer`'s, and the box stays unticked.** Noted by `dev-writer`,
with the code side verified.

**Verified rather than accepted:** `MAX_TITLE_BYTES: usize = 1024` and the check is
`title.len() > MAX_TITLE_BYTES` (`stoa.rs:103`, `:261`), which is a **byte** length in
Rust; the error renders "title is {n} bytes". The cap is enforced a second time inside
`Genesis::decode` at `stoa.rs:296`, on the decoded length, also in bytes. So the entry's
CJK scenario is right: 400 CJK characters is 1200 bytes and is refused, and a view that
counted characters against a "1024 maximum" would show a title well inside the limit and
an error it cannot explain.

`dev-writer` should not resolve this. The entry's point is that the *spec* says "length",
which a view author reads as characters on a JSON string surface — and which count the
user is up against is observable behaviour, so it belongs in the spec rather than in my
head. The entry is also right that the fix may belong in `stoa-genesis`, which owns the
bound, with `stoa-membership`'s two scenarios saying "in bytes" so the ordering
requirement they exist for is unambiguous. That is a two-spec decision.

The test the entry asks for — a multi-byte title at 1024 bytes and one at 1025 — is
`tester`'s once the spec says which count it means. I have not added it, because a test
written before the contract decides would pin whichever answer the implementation
happens to give, which is the defect this flow exists to catch.

---

- [ ] **6. `spec-writer` — unmarked spec gaps: behaviour these tests pin that no scenario describes**

Six `NO SPEC:` markers sit in the Stoa-membership tests and are correctly placed
(`wire.rs:557`, `633`, `2500`, `2801`, `2822`; `membership.rs:498`, `1372`, `1530`,
`1602`). Each is a gap for the spec-writer to capture or change, per the role:

- **the posting policy's wire spelling** — `"open"`, `wire.rs:2800`
  `the_policy_a_created_stoa_declares_is_reported_by_name`. A view branches on the
  string, so this is a lasting surface decision. The spec requires the policy be
  *answerable* and names no field and no value.
- **which policy a creation declares** — always `Open`, no parameter. `wire.rs:633`.
- **`hasMore` on an empty listing** — `false`. `wire.rs:2500`, `membership.rs:1372`.
- **an unrecognised request field** — ignored. `wire.rs:2820`
  `an_unknown_field_in_a_request_is_ignored_rather_than_refused`.
- **`per_page` of zero** — an empty page with `has_more = false`.
  `membership.rs:1514`, `membership.rs:498`.

Five more are pinned by tests with **no marker**, which the brief flags as worth more
attention:

- **`stoas.sqlite` as the on-disk file name**, hardcoded on both sides at
  `wire.rs:2781` `the_membership_path_is_a_file_of_its_own_beside_the_op_logs`. The
  test's own comment says changing it "orphans their memberships" — a durable,
  user-visible decision with no requirement behind it. (The *separateness* is
  implied by the coexistence requirement; the *name* is not.)
- **a store that cannot be opened is the error shape, not an empty listing** —
  `wire.rs:2740`, which asserts the reason reaches the view and names the store. The
  spec's failure-shape requirement enumerates only request-content failures ("an
  absent field, a field of the wrong type, an address that is not an address…"); a
  store-level failure is a different class and is the one where a flattened answer
  invites the user to re-paste every address they hold. Worth a scenario.
- **the layout-version refusals** — entry 3 above.
- **a total listing order independent of join sequence** —
  `membership.rs:1406` `the_order_is_total_and_does_not_depend_on_the_sequence_of_joins`.
  The spec requires reachability-by-paging and once-only, which a stable order
  implies but does not state; the test's argument (a page boundary meaning two
  different things on two peers) is a cross-peer property worth specifying.
- **a corrupt or misfiled retained row is reported, never skipped** —
  `membership.rs:1252` and `membership.rs:1287`. The retention requirement says the
  retained record "MUST remain the one that verifies against the address it is
  retained under" but says nothing about what happens when it does not. These two
  tests pin a good answer (report, don't skip) with no requirement behind it, and it
  is exactly the kind of answer that becomes permanent by accident.

`wire.rs:2781`, `2740`; `membership.rs:1406`, `1252`, `1287`

**Outcome: OPEN — `spec-writer`'s, and the box stays unticked.** Noted by `dev-writer`,
with one item of it fixed because it was a code defect as well as a spec gap.

**What I did act on.** The entry's third bullet in the unmarked list — "the
layout-version refusals — entry 3 above" — is left to `spec-writer` with entry 3. But the
two `NO SPEC:` markers about `per_page` of zero (`membership.rs:498`, `:1530`) were doing
something worse than marking a gap: they justified the guard by citing the wire's
`clamp_per_page`, inverting the layer dependency. That is
`findings/architecture.md` entry 6, and both comments now argue from the function being
`pub` instead. **The marker stays** — the gap is real and still the spec-writer's — only
the false reasoning beside it is gone.

**Everything else stands as written and is not mine.** Each of the five unmarked items is
observable behaviour the spec does not describe, which my own agent file routes to the
spec rather than to me: the `stoas.sqlite` file name, a store-level failure being the
error shape rather than an empty listing, the total listing order, and a corrupt retained
row being reported rather than skipped. I did not add `NO SPEC:` markers to those tests,
and that is deliberate — a marker I add to behaviour I did not choose would claim the
decision was mine, when four of the five predate or fall outside this change's intent.
The entry naming them with line numbers is the durable record.

Two notes for whoever writes the requirements, both from this change:

- **The `stoas.sqlite` name now has one home.** `membership_path_in` moved into
  `membership.rs` (`findings/architecture.md` entry 2), so the name is decided in the
  module that owns the file rather than in `wire.rs`. The *separateness* argument is
  already in `design.md`; the *name* is still unspecified, as the entry says.
- **The store-level failure shape is worth a scenario for the reason the entry gives**,
  and it is now the only guarded label left on that path: `with_membership_store`'s
  method parameter is gone (`findings/architecture.md` entry 5), so a panic in `open` is
  reported under a generic label while the reason a store could not be opened still
  reaches the view in full. `wire.rs`'s test asserting the reason is unchanged.

---

- [x] **7. `dev-writer` — the branch is behind `origin/main` and merging it reverts three specs' `## Purpose` sections**

`git diff origin/main -- openspec/specs` shows four-line deletions from
`openspec/specs/identity/spec.md`, `module-wire-contract/spec.md` and
`stoa-metadata/spec.md` — each removing that spec's `## Purpose` paragraph.

The branch did not make these edits. `git diff <merge-base> HEAD -- openspec/specs`
is **empty**; the merge base is `3fa4df890b823b3c6dfcca7b2048ef3d94cccbc9`, and the
Purposes were *added* on `main` afterwards by `2b9b7f5` ("Correct the flow README's
CLI claim, and give three specs a Purpose (#37)").

So this is stale-branch drift, not a spec defect in this change — but it is a live
merge hazard: the `stoa-membership` spec cites `module-wire-contract`,
`stoa-metadata` and `posting-capability` by name, and merging without a rebase
silently removes the Purpose paragraphs a reader of those citations lands on.

**Failure scenario:** merge as-is; `openspec/specs/module-wire-contract/spec.md`
loses the paragraph #37 added, with nothing in this PR's diff review explaining why.
Rebase onto `origin/main` and confirm the three files show no change.

**Outcome: FIXED**, and the entry's diagnosis was exact — `git diff origin/main --
openspec/specs` showed twelve deletions across those three files, and
`git log --oneline origin/main --not HEAD` showed the single missing commit,
`2b9b7f5` (#37).

**Merged rather than rebased**, which departs from the entry's suggested verb and
not from its requirement. Thirteen commits sit on this branch, four of them
reviewers' findings files cherry-picked from their own worktrees; a rebase rewrites
every one and then needs a force-push, which the flow forbids. Nothing was at risk
in the merge: `git diff <merge-base> HEAD` over `openspec/specs` and
`.claude/agents/README.md` is **empty**, so the two sides touch disjoint files and
the merge carried main's additions in without a conflict.

**Confirmed after the merge**, which is the check the entry asks for:
`git diff origin/main --stat -- openspec/specs` is now **empty**, and
`grep -n "## Purpose"` finds the section in all three of
`openspec/specs/identity/spec.md`, `module-wire-contract/spec.md` and
`stoa-metadata/spec.md`.

One thing worth adding to the entry's stakes, and it makes this worse than a prose
regression. `2b9b7f5`'s own commit message records that before it, these three
specs had no `## Purpose`, so `openspec list --specs` reported all three as holding
**zero requirements** and `openspec show --type spec --json` **errored** on them —
the inventory understated three capabilities to zero, identity among them.

And `docs/OPENSPEC-ARCHIVE.md`, which arrived on `main` in `6e31bc0` (#49) and is
now merged into this branch, states it outright: **`archive` aborts and writes
nothing when the target spec has no `## Purpose`.** `openspec archive` is the last
stage row on this piece. So merging as-is would not have quietly lost three
paragraphs — it would have blocked the archive, at the point where the failure is
least expected.

Sequence worth recording, because I initially declined to assert that last
sentence: when I first wrote this outcome the archive doc was on an unmerged branch
and not in this tree, so the claim was unverifiable here and I said so rather than
repeating it. It is now verifiable and verified, in the file that asserts it.
Separately measured: `openspec validate --specs --strict` from inside this worktree
reports **11 passed, 0 failed**.

---

- [ ] **8. `spec-writer` — `docs/PLAN.md` §4.8 Phase 1 still says an address alone is enough to join, which is the claim this spec exists to correct**

Checked against `docs/PLAN.md` on `origin/main` (and the branch's copy, which is
unchanged in this section).

The change's PLAN.md shedding is otherwise **thorough and well done** — §5.5 and the
Stage D API block are struck through, marked BUILT, pointed at the capability, and
the wrong single-argument `joinStoa({address})` signature is explicitly retracted
with the reason. §9.1's Stage D is marked Built. That is the right shape.

What survives, in the section §5.5 cites as its authority:

- **`docs/PLAN.md:643-646`** — *"A Stoa address is a copyable string. **Importing
  one is how you join a Stoa nobody told the app about**"*.
- **`docs/PLAN.md:648-649`** — *"an address must be **self-authenticating** —
  **pasting it is enough to verify what you joined**"*.

The spec's requirement "Joining takes an address and the record it names" says the
opposite, in bold: *"**An address alone is not joinable**, and that is a property of
the address rather than a limitation of this call."* An address is sufficient to
*verify* a record someone hands over and insufficient to *reconstruct* one — so
pasting an address is precisely not enough.

This is the same error §5.5 now says it made twice ("this section specified the wrong
input, twice"), left standing in the section §5.5 defers to. A reader following the
citation chain lands on the retracted claim, stated more confidently than the
retraction.

Two smaller staleness items in the same family:

- **`docs/PLAN.md:3704`** — MVP item 7, *"Join a Stoa by address"*. Same wrong shape,
  and items 2 and 7 are now built and unmarked, where §5.5 and §9.1 mark theirs.
- **`docs/PLAN.md:3733`** — *"sharing and joining by address, §4.8 Phase 1"*.

A strikethrough plus "answered: see the `stoa-membership` capability" is the shape
the flow README asks for, so the question's history stays legible.

**Outcome: OPEN — `spec-writer`'s, and the box stays unticked.** Noted by `dev-writer`
after re-reading every cited line, because this repo has twice shipped a citation that
was persuasive and unread.

**All four citations verified present, at the lines given**, in this worktree after
merging `origin/main` twice:

- `PLAN.md:643-644` — *"A Stoa address is a copyable string. Importing one is how you
  join a Stoa nobody told the app about"*
- `PLAN.md:648-649` — *"an address must be **self-authenticating** — pasting it is
  enough to verify what you joined"*
- `PLAN.md:3704` — MVP item 7, *"Join a Stoa by address"*
- `PLAN.md:3733` — *"joining by address, §4.8 Phase 1"*

And the entry's reading of the second one is the subtle part, so it is worth restating
as confirmation rather than paraphrase: an address is sufficient to **verify** a record
someone hands over and insufficient to **reconstruct** one, so "pasting it is enough"
is true of verification and false of joining. That is exactly the distinction the spec's
"**An address alone is not joinable**" makes.

Not mine to fix: `docs/PLAN.md` is `spec-writer`'s input document, read from
`origin/main`, and the entry is right that the shedding this change already did (§5.5,
the Stage D API block, §9.1) is the model — strikethrough plus a pointer at the
capability, so the question's history stays legible. Editing PLAN.md prose from a code
fix is how two copies of a rationale come to disagree with nobody able to tell which is
stale.

The entry's own assessment that the shedding was *"thorough and well done"* matches what
I found; what survives is in the sections §5.5 **defers to**, which is why a reader
following the citation chain still lands on the retracted claim.

---

## Checked and clean

Verified as properly pinned by a test that can fail. Mutations run are named.

- **The coexistence requirement the design pivots on** — the brief's "check this
  hard" item. Both scenarios are genuinely covered, and the three tests are
  complementary rather than redundant.

  **Mutation M1, run:** `wire.rs:803`, `membership_path_in` changed from
  `dir.join("stoas.sqlite")` to `dir.join("ops.sqlite")` — the membership store
  sharing the op log's file, the failure a separate file exists to prevent.
  **Predicted:** all three coexistence tests fail. **Observed:** four failures —
  `a_store_holding_ops_and_no_memberships_opens_and_keeps_its_ops`,
  `a_membership_is_recordable_into_a_store_that_previously_held_none`,
  `the_membership_store_does_not_write_into_the_op_logs_file`,
  `the_membership_path_is_a_file_of_its_own_beside_the_op_logs`. Matched, plus one.

  **Mutation M5, run:** `wire.rs:782`, `with_membership_store` changed to open
  `MembershipStore::in_memory()` and ignore the path — a store that persists nothing
  at all, this project's recurring defect family. **Predicted:** the restart tests
  and the second coexistence scenario fail; the first coexistence scenario survives,
  since it records no membership. **Observed, exactly:**
  `a_created_and_a_joined_stoa_both_survive_a_restart_with_their_founding_values`,
  `a_join_refused_at_the_wire_leaves_nothing_behind_a_restart`,
  `a_membership_is_recordable_into_a_store_that_previously_held_none`,
  `the_membership_store_does_not_write_into_the_op_logs_file`,
  `a_store_that_cannot_be_opened_is_the_error_shape_and_not_an_empty_listing` —
  and `a_store_holding_ops_and_no_memberships_opens_and_keeps_its_ops` survived, as
  predicted.

  M5 also **verifies the claim `wire.rs:3186-3191` makes about itself** — that
  without the surviving Stoa, `a_join_refused_at_the_wire_leaves_nothing_behind_a_restart`
  would pass for a store persisting nothing. It is the survivor Stoa that fails
  under M5. The comment's "Measured:" is accurate.

- **`MEMBERSHIP_LAYOUT_VERSION`, the consensus-critical constant.**
  **Mutation M3, run:** `membership.rs:86`, `1` → `2`. **Predicted:** the hardcoded
  pin fails, which is what `cargo mutants` structurally cannot see. **Observed:**
  three failures — `the_membership_layout_version_is_pinned_to_a_known_answer`,
  `a_store_from_an_unknown_layout_version_is_refused_and_names_both_numbers`
  (which hardcodes `expected == 1` rather than comparing the constant to itself),
  and `the_membership_layout_version_is_independent_of_the_op_logs`. The
  `VERSION_1` defect family is closed here.

- **The store layer's verification** — the four `membership.rs` tests that failed
  under M2 are the honest half of entry 2, and they fail for exactly the reason they
  name. `a_record_differing_in_any_field_is_refused_and_records_nothing` varies each
  field in turn and checks the peer is in neither the claimed nor the supplied
  record's Stoa.

- **`creation_returns_the_address_of_the_record_it_built`** — derives the expected
  address independently from a hardcoded title and key rather than reading it back
  out of the reply. This is the correct shape for the family of defects this role
  exists to catch.

- **`keystore.rs:2937` `the_identity_key_is_pinned_to_a_known_answer`** — frozen hex
  (`ea4a6c…d22c`), not a re-derivation, and its comment states exactly what the
  sibling test does *not* catch. Address-determining and correctly pinned.

- **`keystore.rs:3022` `the_creator_of_a_stoa_this_keystore_made_can_moderate_it`** —
  asserts through `Moderators::of`, the real authority check, not through an
  equality the test arranged. Pins the creator/poster key identity the spec's "the
  creator key is the key the caller would sign an op with" requires.

- **`a_malformed_record_is_refused_without_a_membership`** (`wire.rs:2323`) — the
  fixture-collapse trap is explicitly defended against: five byte strings, each
  refusal's message asserted to contain a distinct substring, and
  `messages.dedup().len() == 4` asserted so four cases cannot all be one truncation
  path. Includes an all-zero creator (a low-order point), which is the dangerous
  case. This is the model the rest of the suite should follow.

- **`a_listed_title_is_named_as_founding_and_never_as_a_bare_title`** — asserts the
  **absence** of `title` as well as the presence of `foundingTitle`, on both reply
  paths. A test asserting only presence would pass a reply carrying both.

- **`a_hostile_request_is_an_error_rather_than_an_abort_and_carries_no_result`** —
  22 malformed requests across three methods, each checked for error-and-no-result
  via a shared helper, plus a good call afterwards proving the module still answers.

- **`a_failed_call_records_nothing_and_disturbs_no_retained_record`** — run over a
  store that already holds a membership, so "unchanged" is distinguishable from
  "wiped". The right answer to the two-explanations problem.

- **Pagination boundaries** — `has_more_is_false_on_the_last_page_and_true_before_it`
  and `a_listing_page_that_is_not_the_last_says_so_on_the_wire` use a population
  filling exactly two pages, hardcode the counts, and assert both halves. The two
  regression tests (`a_per_page_at_the_conversion_boundary…`,
  `a_per_page_of_zero_terminates…`) each state which assertion is the kill and why
  the obvious one is not.

- **`openspec validate stoa-lifecycle --strict`** passes, and reading the spec in
  full found no requirement contradicting another. The one tension is entry 3, which
  is a silence rather than a contradiction.

## Tree state

All five mutations restored. `git status --short` empty. Full suite re-run after
restoration: **550 passed, 0 failed.**
