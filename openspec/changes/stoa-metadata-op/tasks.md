# Tasks

## 1. The variant

- [x] 1.1 Add `OpKind::StoaMetadata { title: String, description: String }` to
      `op.rs`, with `STOA_METADATA: u8 = 4` — the next free discriminant, never
      inserted into the used range. Verify `cargo build` succeeds.
- [x] 1.2 Add the encode arm to `canonical_bytes`: title then description, each
      through the existing `put_bytes` length-prefix helper. No new helper — a
      format where a second call site spells its own prefix is a format where one
      of them eventually spells it differently.
- [x] 1.3 Add the decode arm to `decode`, through `take_string`, so the field
      cap and the bounds check come from the shared path rather than a copy.

## 2. Encoding properties

- [x] 2.1 Add the new kind to `one_of_each_kind()` so every existing
      cross-kind property — round-trip, truncation at every prefix length,
      trailing bytes, wire round-trip, sign-and-verify — covers it without a new
      test each. Verified: the suite goes 120 → 134.
- [x] 2.2 Write `the_metadata_fields_participate_in_the_encoding`: vary title and
      description each in turn, requiring both the bytes and the op id to differ.
      Mutation-verified: dropping `description` from the encode arm fails it
      (with 8 others).
- [x] 2.3 Write `a_metadata_ops_title_and_description_cannot_be_confused` — the
      concatenation trap, `("ab","c")` against `("a","bc")`. Mutation-verified:
      replacing both `put_bytes` calls with raw `extend_from_slice` fails it.

## 3. The kind byte inside the signature

- [x] 3.1 Write `a_metadata_op_and_a_revision_do_not_share_a_preimage`, built so
      the two differ ONLY in kind. The fixture aligns the two kind-specific tails
      byte-for-byte (a Revise target whose first four bytes spell the metadata
      title's length), and asserts that alignment BEFORE asserting the property —
      without which the test would pass for a dozen reasons unrelated to the kind
      byte. Mutation-verified: deleting `out.push(self.kind.to_byte())` fails it.
- [x] 3.2 Write `a_signature_over_a_metadata_op_does_not_verify_as_a_moderation`,
      mirroring `a_signature_over_one_kind_does_not_verify_as_another`: sign a
      metadata op, attach its signature to a moderation, require the substitution
      to fail AND the original to verify. Mutation-verified: a `verify` that
      returns `true` unconditionally fails it.

## 4. Hostile input

- [x] 4.1 Write `an_over_long_metadata_title_is_refused_before_allocating` and
      `an_over_long_metadata_description_is_refused_before_allocating`, asserting
      the specific `FieldTooLong` error rather than merely that decoding failed.
      The description needs its own case: a cap applied to the first string only
      would leave the second an open lever. Mutation-verified: removing the cap
      fails EXACTLY the four cap tests (two pre-existing, these two).
- [x] 4.2 Write `invalid_utf8_in_metadata_is_refused` for both fields.
      Mutation-verified: `from_utf8_lossy` fails it.
- [x] 4.3 `a_hostile_op_is_never_a_panic_for_any_input_shape` and
      `truncation_at_any_point_is_refused` cover the new kind via 2.1.

## 5. What the op does not carry

- [x] 5.1 Write `a_metadata_op_carries_no_policy_and_no_ordering_field`: the
      encoding's length pinned against HARDCODED field sizes, not values read
      back off the fixture. Mutation-verified: pushing one extra byte into the
      encode arm fails it.
- [x] 5.2 Write `a_metadata_op_by_a_non_moderator_is_authentic`. Not a
      `NO SPEC:` — the "authenticity, not authority" requirement specifies it.
- [x] 5.3 `an_unknown_op_kind_is_refused` still passes with 4 taken, and
      `the_next_free_kind_discriminant_is_refused_as_unknown` pins that 5 is what
      a future policy-changing kind would take. Mutation-verified: defaulting an
      unknown discriminant fails exactly those two.
- [x] 5.4 Write `renaming_a_stoa_does_not_change_its_address`, checked against
      the hardcoded address from `stoa.rs`'s own pinned known-answer test rather
      than a value this test computed. Mutation-verified: dropping the title from
      the genesis encoding fails it.

## 6. Gates

- [x] 6.1 `cargo test --manifest-path <WORKTREE>/dialectica/rust-lib/Cargo.toml
      -p dialectica-core` — 134 passed, 0 failed.
- [x] 6.2 `cargo fmt ... -p dialectica-core --check`.

      **The gate cannot be clean locally, and that is a pre-existing condition,
      not this change's.** The repo was formatted by CI's pinned `stable`
      (dtolnay/rust-toolchain, 2026-09-03); this machine's rustfmt is 1.9.0
      (2026-04-14), an OLDER generation that disagrees about eleven pre-existing
      hunks in `identity.rs`, `stoa.rs` and untouched parts of `op.rs`.

      Verified by measurement rather than assumed: stashing this change and
      re-running produces the SAME eleven diffs and no others, so this change
      introduces zero new ones. Running `cargo fmt` to satisfy the local version
      would reformat other people's code AWAY from what CI wants and fail the
      gate it was meant to pass — the trap here is the fix, not the failure.
- [x] 6.3 `cargo clippy ... -p dialectica-core --all-targets -- -D warnings` —
      clean.
- [x] 6.4 Mutation-verify each claimed property. Twelve mutations run, each
      broken, measured and restored; the table is in the change report.

## 7. Documentation

- [x] 7.1 PLAN.md §5.7: "The op itself is not built" replaced with a one-line
      statement that it exists and carries no policy, pointing at design.md for
      the reasoning. A second line records that resolution is still not built.
- [x] 7.2 PLAN.md §13: the metadata-`policy` question struck as answered, with
      the reopening conditions named and the argument NOT restated.
