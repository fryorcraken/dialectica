# Tasks

## 1. The types

- [x] 1.1 Create `dialectica-core/src/stoa.rs` with `Policy` (one variant,
      `Open`, with an explicit discriminant) and `Genesis` (version, creator
      `PublicKey`, epoch `u64`, policy, title `String`). Register the module in
      `lib.rs` and verify `cargo build` succeeds.
- [x] 1.2 Add `GenesisError` covering unknown version, unknown policy,
      truncated input, trailing bytes, and a length prefix disagreeing with the
      input. Verify each variant is constructible and distinct — a decoder that
      cannot say WHICH way input was malformed sends the reader looking in the
      wrong place.

## 2. Canonical encoding

- [x] 2.1 Write `encodes_identically_every_time` and
      `two_records_differing_in_any_field_encode_differently` (vary each field
      in turn). Verify both FAIL before `canonical_bytes` exists.
- [x] 2.2 Implement `Genesis::canonical_bytes()`: version, creator key, epoch
      big-endian, policy discriminant, then the title length-prefixed. Verify
      2.1's tests pass.
- [x] 2.3 Write a test proving the title's length is CARRIED, not inferred from
      where input ends, and verify it FAILS with the length prefix stubbed out.

      Done, and the verification earned its keep: the first version
      (`a_title_boundary_cannot_be_moved`, comparing "ab" with "abc") PASSED
      with the prefix deleted — different-length titles differ either way, so it
      proved nothing. Replaced with
      `the_title_length_is_encoded_and_not_merely_implied`, which appends a byte
      to a valid encoding and requires `TrailingBytes`; without the prefix the
      title absorbs it and the test fails with `Truncated`.

## 3. Strict decoding

- [x] 3.1 Write the rejection tests — truncated at every length, trailing bytes,
      a lying length prefix, an unknown policy discriminant, an unknown version.
      Verify each FAILS before `decode` exists.
- [x] 3.2 Implement `Genesis::decode(&[u8])`, rejecting every case in 3.1 and
      returning the matching `GenesisError`. Verify the tests pass and that no
      case returns a partially-populated record.
- [x] 3.3 Write `decode_of_encode_is_the_identity` over several records
      (including an empty title and a multi-byte UTF-8 title) and verify it
      passes.

## 4. Address and verification

- [x] 4.1 Write `a_record_verifies_against_its_own_address` and
      `a_substituted_record_fails_verification`. Verify both FAIL before
      `Genesis::address()` exists.
- [x] 4.2 Implement `Genesis::address()` as `stoa_address(&self.canonical_bytes())`
      and verify 4.1's tests pass.
- [x] 4.3 Write `two_stoas_with_the_same_title_have_different_addresses`
      (differing creator keys) and verify it passes — the title is not identity.
- [x] 4.4 Verify `policy`, `epoch` and `version` each participate in the
      address, by asserting a record differing only in that field yields a
      different address.

## 5. Gates

- [x] 5.1 Run `cargo test -p dialectica-core` and verify every test passes and
      the count matches the `#[test]` count CI derives.
- [x] 5.2 Run `cargo fmt --check` and
      `cargo clippy -p dialectica-core --all-targets -- -D warnings`; verify both clean.
- [x] 5.3 Run `openspec validate stoa-genesis-record --type change --strict`
      and verify it passes.

## 6. Documentation

- [x] 6.1 Record in PLAN.md §13 that the `policy` field question is answered —
      the field now exists with `open` as its only variant — rather than leaving
      an open question that has been closed in code.
