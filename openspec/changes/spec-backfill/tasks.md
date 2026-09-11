# Tasks

**This is a record, not a plan.** The code was written in PR #5, long before
these documents. What follows is what was done to produce the specs, in the
order it was done — not a checklist anyone worked through while writing
`identity.rs`.

**What this change does:** adds two capability specs, a design record, an audit
of which requirements are pinned by tests, and **one test** — see the scope
exception below.

**What it does not do:** change any behaviour, or fix the four gaps it found.
Every other finding is in `design.md` for someone else to act on.

### The one scope exception, and why it was authorised

This change was scoped "no code". One test was added anyway, on an explicit
decision by whoever is running the change, and the reasoning is worth recording
because the exception is narrow:

- It is a **test**, not behaviour. The change stays behaviour-free.
- **No other open branch touches `stoa.rs`**, so this is the only place it lands
  without a conflict. The alternative was filing it against a branch that would
  have to reopen to take it.
- It closes a gap under an **already-merged** requirement, found by writing the
  spec. Leaving it open would mean shipping a spec that documents a hole while
  the fix waits for an owner.

It is in its own commit, so the documents can be reviewed without it.

## 1. Read before writing

- [x] 1.1 Read the archived `2026-09-11-op-model` change in full — proposal,
      design, tasks, spec delta — as the shape and quality bar for a retroactive
      spec, and `2026-09-10-stoa-genesis-record` for the house style it set.
- [x] 1.2 Read both merged specs, to join a style rather than invent one, and
      to find the claims they already make on identity's behalf.
- [x] 1.3 Read `identity.rs` and `wire.rs` in full including their tests — the
      tests are where the authors recorded what they thought the contract was.
- [x] 1.4 Read PLAN.md §2.4, §2.5, §5.1–§5.4 and PHASE0-FINDINGS §3, from this
      worktree's fork point at `origin/main`.
- [x] 1.5 Read `cursor.rs` and `lib.rs` to settle whether the cursor is reachable
      by any caller. It is `pub(crate)` behind a private `mod`.

## 2. Decide the scope

- [x] 2.1 Decide against a `cursor` capability, and record both the reasoning
      and the condition that would reopen it. In `design.md`.
- [x] 2.2 Choose `identity` and `module-wire-contract` as the capability names,
      and argue both against the alternatives considered.
- [x] 2.3 Decide against splitting `identity-addressing` out, on the README's
      rule about not pre-generalising a first instance.
- [x] 2.4 Decide to leave `op-format`'s and `stoa-genesis`'s identity claims
      where they are rather than moving them into `identity`.
- [x] 2.5 Establish that pagination has no implementation, by reading every
      handler in `wire.rs` and the contract trait's five methods. Leave it out
      of the spec rather than write scenarios nothing can check.

## 3. Write the specs

- [x] 3.1 `specs/identity/spec.md` — eleven requirements, each with at least one
      scenario, none naming a function, a type or an algorithm.
- [x] 3.2 `specs/module-wire-contract/spec.md` — four requirements covering the
      JSON convention, the single failure shape, the panic guard, and decoding
      another module's reply.
- [x] 3.3 Keep rationale to a sentence or two per requirement where it earns its
      place, per house style, and put everything longer in `design.md`. A recent
      design review flagged rationale under all eight requirements of a spec as
      having gone too far.

## 4. Verify, do not assume

- [x] 4.1 Trace every requirement to a test, or record that there is none. Table
      below.
- [x] 4.2 Mutation-verify the three load-bearing claims rather than believing
      them: the weak-key refusal, the address binding, and the guard's payload
      handling. Table in `design.md`; source restored and `git status` confirmed
      clean.
- [x] 4.3 Record what could not be written as a truthful requirement at all —
      the entropy-failure panic, the compile-time trait denials, and the
      out-of-process half of the guard property. In `design.md`.

## 4b. Close the gap the spec revealed (the scope exception)

- [x] 4b.1 Write `a_creator_key_that_can_never_verify_is_refused_distinguishably`
      in `stoa.rs`: an all-zero creator key, asserting
      `Err(InvalidCreator(WeakPublicKey))` at the genesis boundary.
- [x] 4b.2 Watch it FAIL first, per the rule that a regression test which has
      never failed proves nothing. With the weak-key guard disabled it reports
      `Ok(Genesis { creator: PublicKey(0000…) })` — the defect made visible.
- [x] 4b.3 Restore the guard; confirm the test passes and the whole suite is
      green. No other test changed behaviour in either direction.
- [x] 4b.4 Confirm the new test introduces no formatting diff, by comparing
      `cargo fmt --check -p dialectica-core` against a stashed baseline. The 13
      hunks it reports are all pre-existing and none is in the added code — see
      the note under Gates.

## 5. Gates

- [x] 5.1 `cargo test -p dialectica-core` green before the change: 126 passed.
- [x] 5.2 Green after it: 127 passed — the 126 unchanged, plus the one added
      test. No existing test changed behaviour.
- [x] 5.3 SDK symlinked into the worktree to make the gate runnable, mirroring
      what CI's nix step does. The path is gitignored and adds nothing to the
      diff.
- [x] 5.4 `cargo fmt --check -p dialectica-core` reports **13 hunks, all
      pre-existing and none in the added test** — verified by stashing this
      change and re-running, which reports the identical set. This is the
      documented trap: local rustfmt disagrees with CI's pinned stable about
      `identity.rs`, `op.rs` and untouched parts of `stoa.rs`, so bare
      `cargo fmt` would rewrite other people's code away from what CI wants.
      **Not run.** Baseline recorded here so the next person knows the number
      is not theirs.

## Requirement → test

**Every row was checked by opening the test.** A row saying a requirement is
pinned means the named test exercises it; a row saying **NONE** means a search of
the suite found nothing, and that is the valuable half of this table. Test names
are unqualified within their module's `tests` block.

### `identity`

| Requirement | Test that pins it |
|---|---|
| One identity per Stoa, permanent | `a_derived_stoa_key_is_deterministic`, `a_derived_key_round_trips_through_a_keystore`, `a_derived_key_signs_and_verifies_like_any_other` |
| Identities are unlinkable across Stoas | `different_stoas_get_unlinkable_keys`, `different_roots_get_different_keys_in_the_same_stoa`, `a_derived_key_is_not_the_root_key` — **but see gap 1**: these pin only that keys differ, not the absence of public derivation |
| A key that can never verify is refused at the parse | `a_low_order_public_key_is_refused_at_the_parse` (both low-order cases and the negative control), `a_right_length_public_key_that_is_not_a_point_is_rejected`, `verification_refuses_a_low_order_key_obtained_around_the_parse`, and now `stoa::tests::a_creator_key_that_can_never_verify_is_refused_distinguishably` at the consuming boundary. Mutation-verified: before this change **one test** caught removal of the guard, now two |
| A wrong-length key or signature is an error, never a panic | `malformed_key_and_signature_bytes_are_rejected_rather_than_panicking` (lengths 0/31/33/64 and 0/63/65), `an_authored_op_with_malformed_fields_is_false_rather_than_fatal` |
| Signing is domain-separated by purpose | `signing_is_domain_separated_from_a_bare_digest`, `a_signature_verifies_against_its_own_key`, `a_signature_does_not_verify_over_different_bytes`, `a_signature_does_not_verify_against_a_different_key` |
| An address is derived from a record, never a bare key | `an_author_address_is_not_a_bare_hash_of_the_key`, `different_keys_get_different_addresses`, `an_author_address_and_a_stoa_address_never_collide` |
| The derivation constants are pinned | `the_wire_constants_are_pinned_to_known_answers` — all four scenarios, hardcoded hex |
| An address's display form parses strictly | `an_address_survives_a_hex_round_trip`, `address_parsing_rejects_attacker_supplied_junk` (not-hex, empty, short, and one byte too long) |
| Verification binds the key to the claimed author | `a_validly_signed_op_under_the_wrong_key_is_still_rejected`, `an_authored_op_verifies_when_the_key_matches_the_claimed_author`, `an_authored_op_is_rejected_when_the_bytes_were_tampered_with`. Mutation-verified: **one test** catches removal of the binding |
| Authenticity is not authority | `op::tests::verification_answers_authenticity_and_not_authority`, `op::tests::a_revision_by_a_different_author_is_authentic_and_still_not_valid` — **in `op.rs`, not here**, and both are one-sided positive assertions the `op-model` change already flagged as weaker than their names suggest |
| Identity does not rotate | `a_derived_stoa_key_is_deterministic` pins the positive half. **The absence of a rotation operation is pinned by NONE** — see gap 2 |
| A secret key cannot be copied, logged or serialised | `a_secret_key_survives_a_byte_round_trip`, `an_all_zero_secret_key_is_accepted_because_every_seed_is_valid` pin the round trip. **The trait denials are pinned by NONE** — see gap 3 |

### `module-wire-contract`

| Requirement | Test that pins it |
|---|---|
| Every method takes JSON and returns JSON | `every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape` (four handlers, well-formed and malformed), `version_reports_what_it_was_given`, `ping_echoes_its_payload` |
| Failure is always the error shape | `ping_rejects_malformed_json_without_unwinding`, `ping_rejects_a_missing_field_rather_than_defaulting_it` (asserts no result field alongside), `parse_channel_id_distinguishes_missing_from_wrong_typed`, `parse_channel_id_errors_are_already_the_wire_shape`, `guard_output_survives_a_panic_payload_containing_json_metacharacters` |
| A panic becomes the error shape and the module keeps serving | `guard_converts_a_panic_into_the_error_shape`, `guard_carries_a_formatted_panic_payload` (mutation-verified as the only test separating the two payload types), `guard_names_the_method_that_panicked`, `guard_passes_a_success_through_untouched`, `panic_probe_returns_the_error_shape_instead_of_unwinding`. **The "keeps serving" half is pinned by NONE in the suite** — see gap 4 |
| A reply from another module is decoded, never guessed at | `channel_exists_propagates_deliverys_own_error_rather_than_wrapping_it`, `callee_error_reads_an_envelope_without_demanding_deliverys_extra_fields`, `callee_error_does_not_claim_an_error_for_a_legitimate_value` (seven non-envelope shapes), `channel_exists_refuses_to_guess_at_an_unrecognised_reply`, `channel_exists_normalises_deliverys_verbatim_string_to_a_boolean`, `channel_exists_also_accepts_a_real_boolean` |

## The gaps, named

Four requirements are not fully pinned. None is a shipped defect; all four are
places where the spec states more than the suite checks, and saying so is the
point of writing the spec.

1. **Cross-Stoa unlinkability** — the tests show two derived keys differ. The
   requirement is that no public derivation exists, which is an absence and
   cannot be tested. The requirement is kept because it constrains future
   change; the scenarios claim only the checkable part.
2. **No rotation** — nothing pins that no operation replaces an identity's key,
   because that too is an absence. Determinism is pinned; the absence of an
   alternative is a review-time property.
3. **The secret key's trait denials** — `Clone`, `Debug` and `Serialize` are
   withheld at compile time, and the suite has no compile-fail harness. Adding
   `#[derive(Clone)]` would be a silent contract change.
4. **"The module keeps serving"** — the suite proves the guard converts a panic
   into the error shape. It cannot prove the counterfactual, because without the
   guard the tests do not fail, they abort the test binary. The property was
   verified experimentally against a running host in PHASE0-FINDINGS §3, which
   is evidence but is not a gate.

### A gap in an already-merged spec — found here, and CLOSED here

`stoa-genesis`'s scenario *A creator key that can never verify a signature is
refused* was pinned by nothing at the genesis boundary. Measured by disabling
the weak-key guard and watching every `stoa.rs` test stay green — the only
failure was the `identity.rs` primitive test.

**Fixed** by `a_creator_key_that_can_never_verify_is_refused_distinguishably`,
in its own commit. Watched failing before it passed: with the guard disabled it
reports `Ok(Genesis { creator: PublicKey(0000…) })` against the expected
`Err(InvalidCreator(WeakPublicKey))` — which is the defect itself made visible,
a record decoding cleanly with a creator who can never authorise anything.

The existing `an_invalid_creator_key_is_refused` uses `[0x02; 32]`, which is not
a valid Edwards point, so it only ever exercised the *other* refusal. That is
the fixture trap this project keeps paying for: a test on a path where one of
two rules is silent cannot tell the two apart. The new test makes them disagree
— a well-formed key refused for the other reason — so it also pins the
"distinguishably" half, which nothing asserted before.

## Two findings for whoever reads the `identity` spec next

Neither belongs in a spec, and both are things a reader of these requirements
would otherwise have to discover by measurement.

### "Authenticity is not authority" is pinned only from `op.rs`

The `identity` spec's requirement *Authenticity is not authority* has **no test
in `identity.rs`**. Both tests that pin it —
`op::tests::verification_answers_authenticity_and_not_authority` and
`op::tests::a_revision_by_a_different_author_is_authentic_and_still_not_valid` —
live in `op.rs`, and both are **one-sided positive assertions** that the
`op-model` change already flagged as weaker than their names suggest: each
asserts only that `verify()` returned `true`, so neither can fail against a
`verify` that has stopped checking anything.

**This matters more now than when it was flagged**, because the property is
load-bearing for `moderation.rs`, which decides authority on read. The
separation between "this op is authentic" and "this op is permitted" is the
thing moderation correctness rests on, and it is currently pinned by two tests
that cannot fail for the reason they name, in a module that is not identity's.

The `op-model` change recorded the cheap improvement: have each also assert that
a deliberately broken version of the same op does **not** verify, so each test
contains both directions.

### Two security properties each rest on exactly one test

Mutation-verified, not estimated:

| Property | Caught by | Everything else |
|---|---|---|
| The address binding in `verify_authored_op` | `a_validly_signed_op_under_the_wrong_key_is_still_rejected` | green |
| The weak-key refusal in `PublicKey::from_bytes` | `a_low_order_public_key_is_refused_at_the_parse` (plus, now, the new `stoa.rs` test) | green |

**The detail worth keeping is an assumption disproved by measurement:** `op.rs`
has its own verification tests — `an_op_signed_by_someone_else_is_rejected`
among them — which one might reasonably assume cover the address binding. They
**measurably do not**. Disabling the binding left every one of them green.

Both checks are of the kind whose removal leaves everything apparently working:
signatures still verify, records still decode. A single test is the whole
distance between the property and a silent regression.
