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
- [x] 4.3 `truncation_at_any_point_is_refused` and `trailing_bytes_are_refused`
      cover the new kind via 2.1, because both iterate `one_of_each_kind()`.

      **`a_hostile_op_is_never_a_panic_for_any_input_shape` did NOT**, and an
      earlier version of this line claimed it did. It was false: that test
      seeded from `a_post()` alone, capped at the first 80 bytes, and flipped to
      a byte set containing no other kind's discriminant — so no other decode
      arm was ever the arm being fuzzed. Two reviewers caught the claim. A false
      claim here is worse than a missing test, because it tells the next reader
      not to look.

      Fixed in its own commit as the pre-existing defect it is: the sweep now
      iterates `one_of_each_kind()`, covers every byte rather than the first 80,
      and includes every allocated discriminant in its flip set.

- [x] 4.4 Add metadata-specific cases for the two properties that rested on a
      single non-metadata test each: `a_lying_metadata_length_prefix_is_refused_
      in_either_direction` (over-claim → `LengthMismatch`, under-claim →
      `TrailingBytes`) and `trailing_bytes_after_a_metadata_op_are_refused`.

      Note the under-claim must be applied to the DESCRIPTION, not the title:
      shrinking the title's prefix makes the decoder read the description's
      length out of the middle of the title's text, and the error becomes
      whatever those four bytes spell — a test passing for an unrelated reason.
      Found by writing it the obvious way first and watching it fail.

- [x] 4.5 Pin the cap's VALUE, which nothing checked.

      Both cap tests probed `u32::MAX` — ~28,000x the cap — proving *a* cap
      exists and nothing about *where*. Review demonstrated the gap: raising
      `MAX_FIELD_LEN` to 150 MB left the whole suite green, as did an
      off-by-one. A cap silently drifted to 150 MB still refuses 4 GiB and still
      leaves open the very memory-exhaustion lever the cap exists to close.
      This is the repo's own defect class — asserting so far outside the
      boundary that the boundary is unconstrained.

      Fixed with a boundary PAIR (`MAX_FIELD_LEN` accepted given real input,
      `MAX_FIELD_LEN + 1` refused as `FieldTooLong`) for both the pre-existing
      Post path and the metadata path, plus
      `the_field_cap_is_pinned_to_a_known_answer` hardcoding `150 * 1024`. The
      pin is load-bearing and separate: the boundary pair is expressed in terms
      of `MAX_FIELD_LEN`, so it MOVES with a drifted cap and cannot catch one.
      `cargo mutants` structurally cannot catch it either — it mutates
      functions, not `const` values.

      Inherited, not introduced; fixed for both paths in one commit.

- [x] 4.6 After rebasing onto main's title cap, pin that the op format's TWO
      length bounds stay distinguishable:

        1. the CAP   — `len > MAX_FIELD_LEN` => `FieldTooLong`
        2. the INPUT — a claim under the cap but past the remaining bytes
                       => `LengthMismatch`

      Each was pinned separately; nothing pinned that they stay DIFFERENT, and
      a decoder reporting either for both passed all of them. The op-format
      analogue of a property op-model reached independently for the genesis
      record. Mutation-verified in BOTH directions: the cap reporting
      `LengthMismatch` fails 7 tests, the input bound reporting `FieldTooLong`
      fails 4, and the new test is the only one in both sets.

      Note my lying-prefix tests already claimed 1000 bytes — well under
      `MAX_FIELD_LEN` — so they were never at risk of the defect main had to
      fix in `stoa.rs`, where a `u32::MAX` claim died on the cap and the
      lying-prefix path stopped being exercised at all.

- [x] 4.7 Readability, from review. Two changes, no behaviour:

      `take_checked_length` → `take_length_within_cap`. "Checked" said nothing
      about WHAT was checked, over a real semantic double-duty: the return is a
      BYTE count in `take_string` and an ELEMENT count in `take_string_list`. A
      reviewer reading the latter concluded the cap bounded the list's total
      bytes and was corrected only by the comment below it. The unit split now
      lives in the function's own doc, which let that comment shrink from five
      lines to three.

      Byte offsets were hand-rederived in ten places, each spelled differently
      (`1 + 1 + 32 + 32`, `… + 4 + 2`, `title_at + 2 + 4`). Extended the
      existing `KIND_AT` precedent with `STOA_AT`, `AUTHOR_AT`,
      `KIND_FIELDS_AT`, `LEN_PREFIX`, `OPTION_TAG`, `POST_BODY_LEN_AT`,
      `POST_BODY_AT`, plus a `metadata_offsets(title_len)` helper returning
      named fields.

      The helper earns its place over more constants because the description's
      position DEPENDS on the title's length — the one offset that is not
      constant. Previously each test hardcoded its own fixture's title length
      as a magic number and explained the coupling in a comment; now it is an
      argument. `an_over_long_metadata_description_is_refused_before_allocating`
      was the worst instance and lost both its ad-hoc fixture and the comment
      propping up `1 + 1 + 32 + 32 + 4 + 2`.

      Mutation-verified as load-bearing, not cosmetic: an off-by-one in
      `metadata_offsets` fails a test. Suite count unchanged at 149, which is
      what makes this a refactor rather than a change.

## 5. What the op does not carry

- [x] 5.1 Write `a_metadata_op_carries_no_policy_and_no_ordering_field`: the
      encoding's length pinned against HARDCODED field sizes, not values read
      back off the fixture. Mutation-verified: pushing one extra byte into the
      encode arm fails it.
- [x] 5.2 Write `a_metadata_op_by_a_non_moderator_is_authentic`. Not a
      `NO SPEC:` — the "authenticity, not authority" requirement specifies it.
- [x] 5.3 `an_unknown_op_kind_is_refused` still passes with 4 taken, and
      `an_unallocated_kind_discriminant_is_refused_as_unknown` pins that the
      first unallocated discriminant refuses with a NAMED error.
      Mutation-verified: defaulting an unknown discriminant fails exactly two.

      Renamed and rewritten after review: it previously hardcoded 5 and said so
      in prose. If another kind landed at 5 first, the prose would go stale
      while the test still passed — it only ever pinned "5 is unknown today".
      It now derives the value by scanning above the highest allocated
      discriminant, so it keeps testing the first genuinely free one. The same
      correction was made in design.md, which no longer names a number.

- [x] 5.5 Write `display_text_is_preserved_exactly_and_never_normalised`.

      Strict UTF-8 with no normalisation is correct and is a CANONICALITY
      requirement, not an oversight: normalising at decode would mean an
      accepted byte string re-encodes to something else, giving two peers two
      op ids for one op. Nothing recorded that, and nothing tested it.

      The test asserts hostile-shaped titles (RLO, zero-width space, Cyrillic
      homoglyph, case variant, combining accent) survive byte-identically, and
      that the combining/precomposed pair stays distinct — the pair NFC would
      collapse. Mutation-verified: a decoder stripping bidi and zero-width
      controls fails this test and ONLY this test.
- [x] 5.4 Write `renaming_a_stoa_does_not_change_its_address`, checked against
      the hardcoded address from `stoa.rs`'s own pinned known-answer test rather
      than a value this test computed. Mutation-verified: dropping the title from
      the genesis encoding fails it.

## 6. Gates

- [x] 6.1 `cargo test --manifest-path <WORKTREE>/dialectica/rust-lib/Cargo.toml
      -p dialectica-core` — all pass. (Count grew as review findings landed;
      run the suite for the current number rather than trusting one written
      here.)
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

- [x] 7.3 Restructure the spec delta across two capabilities once `op-format`
      merged. Wire facts to `op-format` (including amending its now-false closed
      enumeration of four kinds); the concept to `stoa-metadata`. The reasoning
      and the rule for the next kind's author are in design.md so this is not
      re-derived.

- [x] 7.4 Record the Unicode limitation in the spec, in both halves: `op-format`
      gains "Text fields are validated as UTF-8 and not otherwise transformed"
      (why non-normalisation is required, and that display text is therefore
      attacker-controlled); `stoa-metadata` gains "A displayed title is never an
      identifier" (who must mitigate, and how). PLAN §4.8 frames name
      impersonation as a discovery-listing concern; this is the rendering half.

- [x] 7.5 Name the cap's value (150 KiB) in `op-format`'s requirement, so it is
      checkable from the artefacts a spec reviewer is given rather than only
      from the source. A reviewer previously could not tell from spec and tests
      whether the cap was 150 KiB, 1 MiB or unbounded.
