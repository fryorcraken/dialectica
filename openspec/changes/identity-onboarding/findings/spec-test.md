# spec-test review — `identity-onboarding`

Reviewed from `openspec/changes/identity-onboarding/specs/{identity,identity-onboarding}/spec.md`,
`openspec/specs/identity/spec.md`, `proposal.md`, `tasks.md` and the `#[cfg(test)]`
blocks of `wire.rs`, `onboarding.rs`, `identity_store.rs`, `identity.rs`. The
implementation bodies were read **only** at the lines mutated, and each such read
is disclosed in the finding that needed it.

Baseline measured: **553 passed, 0 failed** (`cargo test -p dialectica
-p dialectica-core`). `openspec validate identity-onboarding --strict` — passes.

## Verdict up front

**All seven of commit `2f3fccf`'s tests can fail, and each fails for the reason
it names.** Proven by eight implementation mutations, listed in the appendix.
Three of them reproduce the tester's own report exactly, including the counts:
keep-candidate-0 (552 pass / 1 fail), hardcoded-Stoa slate (552/1),
discard-the-record-error (551/2). In each case the *only* failures were the new
tests — every pre-existing test in the group passed, which independently
confirms the tester's claim that the whole keep group was structurally blind
rather than merely thin.

The pinned constant was verified against an authority outside this repository.
`f1e32c8f4601cb1651be57d58e39f28cc1a6e4ef8a69b9bdd2155ef953a7572b` was
reproduced with `openssl kdf`, after first validating the invocation by
reproducing the version-1 value `b62b6b59…` exactly. The change's other four
path-scheme pinned constants reproduce too. tasks.md's claim about this is true.

**Two findings follow. Neither is a defect in the seven new tests.** One is a
live, measured coverage gap the seven did not close; the other is PLAN.md
shedding that has not happened.

---

- [ ] **Finding 1 — `spec-writer` / `tester`: the slate half of the name-and-mark scenario is unpinned, and a `displayName` on every candidate passes all 553 tests**

**Severity: medium.** A spec scenario with no test that can fail on it, on the
one requirement whose whole purpose is to keep a separate contract from being
settled here by accident.

**The spec scenario**
`openspec/changes/identity-onboarding/specs/identity-onboarding/spec.md:454-460`,
under "A generated name and a mark are not settled by this capability":

> - **WHEN** a slate is generated, and separately the identity in use is asked for
> - **THEN** no candidate in the slate reply carries a display name or a visual mark

**The whoami half is pinned. The slate half is not.**
`the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
(`wire.rs:2510`) asserts the whole serialised string, so a field added to the
whoami reply fails it.
`the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
(`wire.rs:1696`) checks each expected key is **present** — it never checks the
key set is exactly those:

```rust
for field in ["index", "path", "address", "publicKey"] {
    assert!(candidate.get(field).is_some(), ...);
}
```

**Measured, not inferred.** I added a `displayName` to every slate candidate in
`slate_json` (`wire.rs:349`, the one line read for this):

```rust
"publicKey": c.public_key.to_hex(),
"displayName": "Brave Otter",
```

Result: **553 passed, 0 failed.** The reply the view receives now carries a
display name this capability explicitly declines to define, and no gate says so.
Mutation reverted; tree confirmed clean.

**Failure scenario to reproduce.** Apply the two-line edit above, run the suite,
observe 553/0. A view written against `candidate.displayName` would then be
written against a name no contract defines — which is exactly the outcome the
requirement's prose ("a caller written against it would be written against a
name this capability never defined") exists to prevent.

**This is already known and was left open deliberately.** tasks.md:174-181 says
so in as many words — *"The slate half of the scenario is therefore unpinned; a
test asserting the candidate's key set exactly is `tester`'s to add."* The seven
tests in `2f3fccf` did not add it. I am recording it because an admission in a
checklist is not a gate: tasks.md is deleted before merge, and the gap then has
no owner.

**For `tester`:** assert the candidate key set exactly, the way the whoami test
does — e.g. collect each candidate object's keys and `assert_eq!` against a
hardcoded sorted `["address","index","path","publicKey"]`. Note the `assert!(…
is_some())` loop is the weaker template that got copied; fixing the family
(assert the key set, not key presence) matters more than fixing this instance.

**For `spec-writer`, a smaller point on the same scenario.** Its first bullet
also asserts *"both replies still carry the public key and the address a name and
a mark would be derived from"*. That half is covered twice over
(`a_slate_reply_carries_an_address_and_a_public_key_for_every_candidate`,
`who_am_i_reports_an_identity_with_its_address_and_public_key`). No action — only
noting the scenario is half-covered rather than uncovered.

**The `tester` half is DONE; the box stays OPEN for `spec-writer`.**

**Done:** `the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against` now
collects each candidate's keys and `assert_eq!`s the set against a hardcoded sorted
array, so an **added** key fails as well as a removed one — which is the fix this
finding specifies, including its advice about the weaker template: *"the `assert!(…
is_some())` loop is the weaker template that got copied; fixing the family (assert the
key set, not key presence) matters more than fixing this instance."* The top-level key
set is pinned the same way for the same reason.

Verified by re-applying the exact mutation measured here — `"displayName": "Brave Otter"`
on every candidate — which now fails this test and **nothing else in 563**. It survived
all 553 before.

I took this on rather than leaving it to `tester` because `slate_json`'s own doc comment
claimed the stronger pinning (readability's R2), so the comment was going to have to
change either way, and a comment that says "the key set is pinned" beside a test that
does not pin it is the state this project keeps finding. Fixing the test was the honest
direction.

**Open, and for `spec-writer`:** the reason this finding exists is that an admission in
`tasks.md` is not a gate — *"tasks.md is deleted before merge, and the gap then has no
owner."* A test now closes the measured hole, but the underlying question is about the
contract, and I cannot settle it:

- The scenario asserts that **no** candidate carries a display name or a visual mark. The
  test now pins one exact key set, which is a stronger statement than the scenario
  makes — it forbids *every* addition, not just names and marks. If a later change wants
  to add a field to a candidate, it will fail a test whose scenario does not obviously
  forbid it.
- `path` is in that key set and **no requirement names it** (readability R4, now marked
  `NO SPEC:`). So the test pins a field the contract does not require, in the assertion
  that enforces a contract requirement. That wants deciding in the spec rather than by me.

Left unticked so that decision has an owner, which is the finding's own point.

---

- [ ] **Finding 2 — `spec-writer`: PLAN.md §5.2.1 on `origin/main` still carries behaviour the spec now specifies, including one question it calls undecided that the spec has decided**

**Severity: low-medium.** Not a code or test defect. It is the §6 shedding the
flow requires, measured against `origin/main` rather than the branch's copy
(`git show origin/main:docs/PLAN.md`, extracted to `tmp/plan-main.md`).

§5.6 shows the right shape and is the model — *"**Built** — see the `keystore`
and `posting-capability` specs"*, then only what is not built. §5.2.1 has not had
that treatment. Three places, in ascending order of how much they matter:

**a. States as present-tense design what the spec now specifies.**
`docs/PLAN.md:889-892` (origin/main):

> **But a user is not handed one.** At onboarding they are shown a slate of five
> generated identities and pick one, and they may refresh the slate as many times
> as they like.

Both halves are now requirements — "A user is offered several candidate
identities and chooses one" (fixed count reported with the set) and its
"Regeneration is not limited" scenario. Two copies of one rule is the failure the
flow README names; PLAN.md should point at the spec.

**b. Calls undecided a question the spec has answered.** `docs/PLAN.md:1393-1395`:

> Whether the keystore writes on every refresh or only on selection is an
> implementation question with no user-visible consequence, and is deliberately
> not decided here.

The spec decides it: **"Nothing is stored before a candidate is kept" —
"Generating a slate SHALL NOT write to storage."** And it is tested
(`generating_a_slate_writes_nothing`, `wire.rs:1786`, which asserts the directory
is still empty after three slates). A reader of PLAN.md alone would believe this
is still theirs to choose. The flow's prescribed shape is a strikethrough plus
"answered: see `identity-onboarding`", so the question's history stays legible.

**c. Duplicates the reasoning the spec's requirement carries.**
`docs/PLAN.md:1386-1395` ("Regeneration discards keys, and the user cannot see
it… an identity becomes real when it signs, and nothing signs during
onboarding") is now also in the spec's "Keeping a candidate persists it" prose
(*"An identity becomes real when it signs, and nothing signs during onboarding"*
— near-verbatim). Per the flow, reasoning goes to `design.md` and out of
PLAN.md, not into both.

**Not in scope for me, flagged for whoever routes this:** the grinding analysis
(§5.2.1 "Grinding — and the slate makes this the central finding") is reasoning,
so whether it survives in PLAN.md is `design-reviewer`'s call, not mine.

**All three places are shed; the box stays OPEN because the file is `spec-writer`'s.**

Done, in the shape §5.6 models and this finding cites as the right one — struck through,
with a one-line summary that the thing exists and a pointer to the spec, reasoning
removed rather than annotated. Design review's finding 8 raised the same section from the
other direction (it measured `git diff origin/main -- docs/PLAN.md` as empty), and the
two agree on what to do, so I did it:

- **(a)** the "But a user is not handed one" paragraph now points at the spec for the
  fixed count and the unlimited regeneration, keeping only what is not a requirement
  anywhere — that a name is *chosen*, and carries intent.
- **(b)** the undecided write question is struck and answered: nothing writes on refresh,
  **structurally**, because the slate handler has no store parameter. The finding is right
  that *"a reader of PLAN.md alone would believe this is still theirs to choose"*, which is
  the worst of the three, and that `generating_a_slate_writes_nothing` already pins it.
- **(c)** the duplicated reasoning is deleted rather than cross-referenced, per the flow's
  rule that reasoning goes to `design.md` and **out** of PLAN.md.

I also struck the five-keypairs duplicate-redraw paragraph (design review's finding, not
this one) and corrected the §5.2.1 open question, which described a scheme with two
variants and named one.

**The grinding analysis is left alone**, and both reviewers who mention it decline to
rule — this one calls it `design-reviewer`'s, and `design-reviewer` calls it a judgement
call. My reading, recorded so the next agent need not re-derive it: it analyses an
*attack* rather than describing built behaviour, nothing in this change answers it, and
PLAN.md is where not-yet-built reasoning belongs. It stays.

**Why unticked.** `docs/PLAN.md` is not `dev-writer`'s file — the flow gives PLAN.md
reading to `spec-writer` and `dev-writer` and its authorship to neither cleanly, and this
finding is addressed to `spec-writer`. The edits are made because leaving a document
contradicting the code for a document-ownership reason is the failure this whole entry is
about. But `spec-writer` should confirm the shedding matches what the spec now says,
rather than me ticking my own edit to their file.

---

## Coverage walk — what was clean

Every other requirement and scenario in both delta files maps to at least one
test whose expected value does **not** come from the thing under test. Stated
plainly rather than padded into findings:

| Requirement | Pinned by | Independent because |
|---|---|---|
| Several candidates offered, count fixed and reported | `a_slate_reply_carries_the_fixed_count_and_that_many_candidates`, `a_slate_reply_takes_no_count_from_the_caller` | `count` asserted against hardcoded `5`, not the array length |
| Every candidate distinct | `every_candidate_in_a_slate_is_distinct` | pairwise over path, key and address |
| Another set yields different candidates | `a_second_slate_shares_no_candidate_with_the_first` (8 rounds), `two_slates_in_a_row_offer_different_candidates` | accumulates across rounds, not one pair |
| Regeneration not limited | `regeneration_is_not_limited` (200 rounds) | — |
| One master key, differing by path alone | `a_candidates_key_is_the_path_taking_per_stoa_derivation`, `one_nonce_gives_different_slates_for_different_master_keys_and_stoas` | compares against the derivation primitive directly |
| Reproducible from master key + path | `a_path_derived_key_is_deterministic`, `a_nonce_reproduces_an_identical_slate` | — |
| Slate reply carries public values only | `no_secret_appears_anywhere_in_a_slate_reply`, `a_slate_carries_no_secret_anywhere_in_its_candidates`, `no_value_a_slate_exposes_can_sign_as_any_candidate` | byte search over the raw reply string, plus a positive control; master key `0xa5` ≠ nonce `0x07`, which is the fixture defect this project ships and here is fixed |
| Keeping persists; identity reported is the one kept | **`a_kept_identity_is_the_candidate_the_slate_offered_at_that_index`** (new), `a_kept_identity_survives_a_restart_and_is_the_one_reported`, `a_kept_identity_can_sign_as_the_identity_it_reported` | the new one reads the key from the SLATE reply and requires the KEEP reply to match — two calls, so agreement is a property of the code |
| Keeping either completes or changes nothing | `a_keep_whose_keystore_write_fails_records_no_path`, **`a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`** (new), **`a_failed_keep_leaves_the_record_no_fuller_than_it_found_it`** (new) | both write directions now covered; the third counts rows across a failure against a hardcoded `before` |
| Chosen path recorded and survives restart | `a_recorded_path_survives_a_restart`, `distinct_stoas_record_distinct_paths`, `every_recorded_pairing_reads_back_in_full` | expected set hardcoded |
| Record fully readable, nothing machine-local | `reading_the_record_back_needs_nothing_beyond_the_record` | a SECOND connection sharing nothing with the writer |
| One device, one writer | **`a_record_restored_beside_a_master_key_names_the_identities_in_use`** (new) | copies both files to a fresh directory, then a second target with a different recorded path — so a handler ignoring the record fails |
| Interface can state recovery needs the record | `who_am_i_reports_that_recovery_needs_more_than_the_master_key` | pinned to `true`; flipping the field to `false` fails it (mutation 7) |
| Keeping does not replace an existing identity; refusal distinguishable | `a_second_keep_is_refused_and_leaves_the_stored_identity_unchanged`, **`each_keep_refusal_reason_is_pinned_to_its_own_situation`** (new) | the new one pins four reasons to vocabulary chosen from the situation, and reaches the storage-failure reason THROUGH the handler — the pre-existing test built it outside |
| Current identity reportable, separately from posting | `who_am_i_reports_an_identity_…`, `who_am_i_reports_nobody_with_a_reason_…`, `an_unusable_identity_is_distinguishable_from_an_absent_one` (4 states pairwise), `distinct_stoas_report_distinct_identities` | whoami JSON pinned as a whole string; each address recomputed from the recorded path |
| Protection applied is reported | `the_keep_reply_reports_whether_the_master_key_was_encrypted` | both directions, and checked against the FILE via `Keystore::is_encrypted`, not only the argument |
| Every entry point refuses malformed input; nothing aborts | `a_malformed_{slate,keep,whoami}_request_…`, `no_onboarding_handler_panics_whatever_it_is_given_or_whatever_fails`, `a_malformed_slate_request_does_not_supersede_the_live_slate` | 11 arbitrary inputs × 3 methods, plus panicking keystore and record dependencies |
| Nothing stored before a candidate is kept | `generating_a_slate_writes_nothing` | asserts the directory is EMPTY, not that a named file is absent |
| Two derivation schemes distinguishable | `the_path_taking_scheme_does_not_collide_with_the_scheme_without_one`, `the_wire_constants_are_pinned_to_known_answers`, **`the_whole_path_reaches_derivation_and_not_only_its_low_byte`** (new) | see below |
| `identity` MODIFIED: all six scenarios | `a_derived_stoa_key_is_deterministic`, `a_path_derived_key_is_deterministic`, `different_paths_give_different_identities_in_one_stoa`, `the_path_taking_scheme_does_not_collide_…`, `a_path_derived_key_signs_and_verifies_like_any_other` | the two-input scenario is retained verbatim and still tested |

**The `identity` MODIFIED block is sound (§4 check).** All three scenarios the
live `openspec/specs/identity/spec.md` carries are restated **verbatim** in the
delta, and three are added. The one prose change — "root secret and the Stoa's
address **alone**" becoming "…and the derivation path recorded for that Stoa" —
is the declared subject of the change, argued in the delta's Purpose, with the
recomputability guarantee explicitly moved onto the recorded path rather than
dropped. Nothing went missing in the move. tasks.md 6.3's account of catching a
renamed scenario before it dropped the two-input guarantee is consistent with
what is in the file now.

**Two scenarios that cannot be tested, correctly so — no test demanded.**

- [ ] *"Slate material in memory is cleared when discarded"* →
  *"the buffer that held it is overwritten rather than left with it in place"*.
  A stack local after its function returns is not observable from a test;
  `keystore.rs`'s own review established this. tasks.md 3.5 records it and takes
  the structural route instead (the derived key moved straight into a
  `Zeroizing` with no plain binding). **Correctly untested.** I note for
  `spec-writer` only that this scenario is, as written, the kind the flow README
  warns against ("Never write a scenario that cannot be tested") — the
  requirement is worth keeping, the scenario asserts an unobservable.

  **OPEN for `spec-writer`.** Agreed on both halves and **no test was written**, per the
  brief's instruction not to write one that pretends otherwise. The scenario asserts an
  unobservable and the requirement is worth keeping; rewording the scenario so it
  asserts something checkable is a spec edit.

  One thing that arrived after this review and bears on the same requirement: security
  S5 found that `derive_stoa_key_at_path` leaves a derived per-Stoa seed on the stack
  **unwiped**, and that this now happens five times per slate rather than once at
  keystore setup. So the requirement is not merely untestable — there is a real gap
  underneath it, recorded in `design.md`'s Risks. The comment in `onboarding.rs` that
  claimed the obligation was discharged is fixed; the seed is not.

  That strengthens rather than weakens the case for keeping the requirement, and it is
  why this is `spec-writer`'s to reword rather than mine to quietly drop.
- *"A slate reply value cannot sign"* is testable and IS tested
  (`no_value_a_slate_exposes_can_sign_as_any_candidate`), with the right control.
  Flagging only that it correctly avoids the trap the agent brief names: every
  32-byte string is a valid Ed25519 seed, so "this constant is invalid key
  material" would have been the wrong assertion, and the test says so in a
  comment.

- [ ] **NO SPEC markers — two, both in the change's new module, both properly
attributed and both genuine spec gaps rather than defects.** For `spec-writer`
to capture or change, not for anyone to fix:

- `identity_store.rs:700` — a stored path outside `u32`. Choice taken: refuse
  rather than clamp, on the argument that a clamped path derives a valid key and
  hands the user an identity they did not choose. The reasoning is right and the
  spec is silent.
- `identity_store.rs:746` — a stored Stoa key of the wrong length. Refused
  rather than padded, same argument.

Both are reachable only by editing the file, which is why the spec never reached
them. Both are pinned by mutation per tasks.md 2.2 (`as u32` clamping makes
`a_stored_path_outside_u32_is_refused_rather_than_clamped` fail).

**OPEN for `spec-writer`, as addressed — and the inventory has changed under it, which
is the reason to leave this visible rather than tick it as "noted".**

**There are now three markers, not two.** `path`'s appearance in the slate, keep and
whoami replies gained one (readability R4): no requirement or scenario names a reply
field for it, so it was a contract by silence and is now a visible choice.

**The first marker's bound moved.** The refusal is no longer "outside `u32`" — security
S1 measured that `u32` was the wrong bound, because `derive_path` masks every path it
writes below 2³¹, so rows in [2³¹, 2³²) were accepted and derived working identities
nobody chose. It is now `onboarding::PATH_LIMIT`, expressed as one constant shared with
the mask. The marker's *reasoning* is unchanged and was always right; what changed is
the range it guards, and the test is renamed
(`a_stored_path_this_build_could_not_have_written_is_refused`).

**So there is a fourth thing for `spec-writer` here that this finding could not have
known:** the spec states **no admissible range for a recorded path**, which is why the
original bound was free to be the wrong one. That is not marked `NO SPEC:` because it is
not an arbitrary filling of a silence — the range is the mask's, and the mask follows
from the requirement that a recorded path be what the derivation produced — but it is a
gap, and it is the one that let a high-severity defect through.

Recorded in `design.md` under Decisions. Unticked because all four are contract
questions and none is `dev-writer`'s to settle.

**No unmarked gaps found.** I checked each test in the four modules for behaviour
no scenario describes. The closest was
`identity_store.rs:473`'s primary-key refusal of a second choice for one Stoa,
which the test attributes to "Keeping an identity does not replace an existing
one" — that requirement is about the keystore, but the store-level refusal is a
strictly stronger reading of the same rule rather than a new decision, and the
wire-level behaviour it produces is covered by
`a_second_keep_is_refused_and_leaves_the_stored_identity_unchanged`. Not
reporting it as a gap.

---

## Appendix — the eight mutations run

Every one applied, measured, and reverted. `git status --porcelain` empty after
each and at the end of the review. Implementation lines read only where the
mutation required it, named per row.

| # | Mutation | Line read | Result | Caught by |
|---|---|---|---|---|
| 1 | `keep_selection` validates the index then keeps candidate `0` regardless | `wire.rs:520-523` | 552 pass / **1 fail** | **only** `a_kept_identity_is_the_candidate_the_slate_offered_at_that_index` (new) |
| 2 | `generate_identity_slate` derives against a hardcoded Stoa instead of the parsed one | `wire.rs:315` | 552 / **1** | **only** `a_slate_is_offered_for_the_stoa_it_was_asked_about` (new) |
| 3 | keep discards the path record's write error (`let _ = …`) and reports `kept:true` | `wire.rs:538-542` | 551 / **2** | **only** `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity` and `each_keep_refusal_reason_is_pinned_to_its_own_situation` (both new) |
| 4 | `whoami_for` ignores the recorded path, deriving at a fixed `0` | `wire.rs:680-681` | 549 / **4** | `a_record_restored_beside_a_master_key_names_the_identities_in_use` (new) + 3 pre-existing |
| 5 | a refused out-of-range keep records a path anyway | `wire.rs:520-523` | 551 / **2** | `a_failed_keep_leaves_the_record_no_fuller_than_it_found_it` (new) + `a_selection_outside_the_set_is_refused_and_stores_nothing` |
| 6 | `KeystoreError::AlreadyExists` message stops naming its situation while staying distinct from the others | `keystore.rs:580` | 552 / **1** | **only** `each_keep_refusal_reason_is_pinned_to_its_own_situation` (new) |
| 7 | `recovery_needs_the_record: true` → `false` | `wire.rs:707` | 552 / **1** | `who_am_i_reports_that_recovery_needs_more_than_the_master_key` |
| 8 | path encoded as its low byte only; then little-endian | `identity.rs:428` | 550 / **3**; then 551 / **2** | `the_whole_path_reaches_derivation_and_not_only_its_low_byte` (new) in both. **`the_wire_constants_are_pinned_to_known_answers` survives the low-byte mutation** — which is precisely the gap the new test was written for, confirmed |
| — | `displayName` added to every slate candidate | `wire.rs:349` | **553 / 0 — SURVIVES** | nothing. See Finding 1 |

Mutations 1, 2, 3 and 6 were caught by the new tests **and by nothing else**,
which is the strongest available evidence that the seven are not redundant with
what was already there. Mutation 8's row is the one worth keeping: the
pre-existing pinned-constants test passes under a derivation that reads only the
path's low byte, because paths 0 and 1 differ in the last byte alone — two
explanations, one answer, this project's defect family, and the new test's third
pinned value at `0x01000000` closes it.

**Independent verification of the new pinned constant.** Not read back from the
code:

```
$ openssl kdf -keylen 32 -kdfopt digest:SHA512 \
    -kdfopt hexkey:07…07 \
    -kdfopt hexsalt:2f6469616c6563746963612f322f4964656e746974792f53746f61 \
    -kdfopt hexinfo:6b1f…9cd8 01000000 HKDF
F1:E3:2C:8F:46:01:CB:16:51:BE:57:D5:8E:39:F2:8C:C1:A6:E4:EF:8A:69:B9:BD:D2:15:5E:F9:53:A7:57:2B
```

matching `identity.rs:1290`'s hardcoded
`f1e32c8f4601cb1651be57d58e39f28cc1a6e4ef8a69b9bdd2155ef953a7572b`. The
invocation was validated first by reproducing the version-1 salt's value
`b62b6b592aeb0779541bbe8beac60d8f505342c37c6a9bc990920d93e68026cf` exactly. The
change's other four path-scheme constants (paths 0 and 1 under salt version 2)
also reproduce. tasks.md 1.2 and `2f3fccf`'s commit message are accurate on this
point — verified, not taken on trust.
