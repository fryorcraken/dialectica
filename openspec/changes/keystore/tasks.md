# Tasks

## 1. Investigate the reference before copying it

- [x] 1.1 Read radicle/heartwood's actual keystore: format, cipher, KDF, unlock
      paths, where the prompt lives, whether permissions are checked, whether
      writes are atomic. Record findings in `design.md` rather than assuming
      §5.6's one-line summary is complete.

      Two of its properties turned out to be **gaps**: heartwood never stats the
      key file (`ssh-key`'s reader carries an unimplemented
      `// TODO(tarcieri): verify file permissions`), and its writer truncates in
      place with no temp file, no rename and no fsync. Both are closed here.

- [x] 1.2 Decide the cipher and KDF against named alternatives, and record what
      ruled each out. Check what is already in `Cargo.toml` before adding a
      dependency.

## 2. The types, and the states they make unrepresentable

- [x] 2.1 Create `dialectica-core/src/keystore.rs`; register in `lib.rs`;
      `cargo build`.
- [x] 2.2 `Protection` with an explicit discriminant per scheme, so whether a
      keystore is encrypted is RECORDED rather than inferred from a failure to
      decrypt. Verify the two states are distinguishable without unlocking.
- [x] 2.3 `Unlock` as an enum on the input side, so the deferred agent path is a
      new variant rather than a signature change. Verify by inspection that
      adding a variant touches no existing caller.
- [x] 2.4 `Passphrase` with no `Debug` and no constructor that reads anything —
      the structural half of "the module never prompts".
- [x] 2.5 `KeystoreError` with one variant per distinguishable state, since the
      probe's reasons are these and a reason can only name a fix if the error
      said which thing went wrong.

## 3. The format, decoded strictly

- [x] 3.1 Write `the_file_layout_is_pinned_to_hardcoded_offsets_and_lengths`
      with literal bytes — magic, version, protection, recorded KDF parameters,
      total length. Verify it FAILS before the encoder exists.
- [x] 3.2 Implement `to_file_bytes` / `from_file_bytes` over the shared
      `Cursor`, and verify 3.1 passes.
- [x] 3.3 Write the rejection tests: not-a-keystore, unknown version, unknown
      protection, trailing bytes, truncation at EVERY prefix length. Verify each
      fails before the corresponding arm exists.
- [x] 3.4 Write `a_hostile_keystore_file_is_never_a_panic_for_any_input_shape`,
      matching `op.rs`'s sweep — every byte position × several values, both
      unlock kinds against every mutated file, plus arbitrary inputs.

      **This sweep found a real bug**, see 5.3. It also cost the shipped Argon2
      parameters a test-only alternative: at 64 MiB × several hundred
      decryptions the binary was OOM-killed, so the sweeps build files whose
      RECORDED parameters are cheap — which exercises parameter portability as
      a side effect.

## 4. Encryption

- [x] 4.1 Write `the_plaintext_secret_is_absent_from_an_encrypted_file`,
      searching the bytes for the seed directly. A round-trip test alone would
      pass on a format that stored the plaintext beside the ciphertext — which
      is the LEZ failure §5.6 names.
- [x] 4.2 Write `a_wrong_passphrase_is_refused_and_named_as_such` and
      `a_passphrase_differing_in_one_byte_is_refused`. Verify both fail before
      the AEAD is wired.
- [x] 4.3 Write `a_flipped_ciphertext_byte_is_refused_rather_than_decrypted`
      over every byte after the header.
- [x] 4.4 Write `two_keystores_with_the_same_secret_and_passphrase_differ` —
      the per-write salt and nonce. Identical files would announce identical
      secrets, and a reused nonce is a break of the cipher.
- [x] 4.5 Implement Argon2id + XChaCha20-Poly1305 with the header as associated
      data. Verify 4.1–4.4 pass.
- [x] 4.6 Write `the_header_is_authenticated_and_not_merely_read`.

      **Mutation-driven**: deleting the AAD left the whole suite green, because
      salt and nonce are already inputs to the key and cipher, so every test
      aimed at the AAD was passing for a different reason. What it alone covers
      is the version byte; pinned at the function, since no v2 exists to test
      through and the guarantee has to exist before that version does.

## 5. Hostile parameters

- [x] 5.1 Record the KDF parameters in the file rather than assuming this
      build's, so a cost change is not reported as a wrong passphrase.
- [x] 5.2 Write `altered_kdf_parameters_do_not_open_the_file`.
- [x] 5.3 Write
      `an_enormous_declared_kdf_cost_is_refused_before_anything_is_allocated`
      and implement the ceiling.

      **The one finding here that was not predicted.** `argon2::Params::new`
      accepts an `m_cost` up to `u32::MAX` — four terabytes — and 3.4's sweep
      flipped one byte of a recorded cost and got the binary SIGKILLed. In a
      module process that is PHASE0-FINDINGS §3's death reached with no
      unwinding, so `guarded` has nothing to catch. Checked BEFORE
      `Params::new`, since the allocation is in `hash_password_into`.
- [x] 5.4 Write `a_cost_at_the_ceiling_is_still_attempted`, so the cap does not
      quietly become "only this build's parameters open".

## 6. The filesystem

- [x] 6.1 Write `a_keystore_readable_by_others_is_refused_before_it_is_read`
      over every mode granting group or other access. Verify it fails before
      the check exists. A check written as `mode == 0o600` passes this; one
      written as `mode & 0o044` misses the execute and group-write bits.
- [x] 6.2 Write `a_written_keystore_is_owner_only_from_the_moment_it_exists`,
      asserting a hardcoded `0o600` rather than the constant.
- [x] 6.3 Write `creating_over_an_existing_keystore_is_refused` and verify the
      original is still readable afterwards.
- [x] 6.4 Write `the_bytes_never_go_straight_to_the_destination` and
      `a_failed_write_leaves_an_existing_keystore_intact`.

      **Mutation-driven**: writing straight to the destination — exactly what
      radicle's stack does — left the suite green, because a crash mid-write
      cannot be arranged in a test. So the mechanism is checked where it IS
      observable: the staging path and the destination differ and are siblings
      (a cross-filesystem rename is a copy and not atomic), and a write made to
      fail leaves the previous key readable.

## 7. The unlock paths

- [x] 7.1 Implement `unlock_from_env` / `open_from_env` with
      `DIALECTICA_PASSPHRASE`, asking the FILE first and the environment
      second.
- [x] 7.2 Write `the_environment_decides_the_unlock_only_after_the_file_has_spoken`
      covering: unencrypted with nothing set, encrypted with nothing set
      (`Locked`), encrypted with an EMPTY variable (`Locked`, matching
      ssh-keygen), the right passphrase, the wrong one (`WrongPassphrase`, not
      `Locked`), a passphrase set against an unencrypted keystore, and a
      missing file.

      One `#[test]`, deliberately: `set_var` is process-global and cargo runs
      tests in threads, so splitting it would race intermittently — which is
      worse than a long test, because an intermittent failure teaches people to
      re-run rather than to look.
- [x] 7.3 Record the environment path's limitation — visible to other local
      processes — in the code and in design.md. It cannot be mitigated from
      here; the deferred agent path is what closes it.

## 8. Errors

- [x] 8.1 Write `no_error_message_carries_key_material_or_a_passphrase`, feeding
      distinctive markers through and checking every message AND `Debug` form.
- [x] 8.2 Write `every_error_message_names_a_fix` over every variant. §5.6
      requires it of the probe's `reason`, and the probe's reasons ARE these
      strings, so enforcing it here covers every variant at once including ones
      added later.

      `// NO SPEC:` — "names a fix" is checked as "contains an imperative verb
      from a list". The spec requires actionable guidance and cannot define it
      mechanically; this is the closest checkable proxy. It caught one real
      case: `Io` was passing the OS message through and naming no fix.

## 9. The probe

- [x] 9.1 `Capability` as an enum with one payload per variant, so
      identity-xor-reason is unrepresentable rather than checked.
- [x] 9.2 Write `the_capability_json_is_pinned_to_the_exact_shape_the_plan_specifies`
      with literal JSON strings — a view is written against those key names and
      renaming one is a breaking change no type checker catches.
- [x] 9.3 Write `a_successful_probe_carries_no_reason_and_a_failed_one_no_identity`.
- [x] 9.4 Write `each_failure_state_produces_a_distinguishable_reason` and
      `no_passphrase_and_a_wrong_passphrase_are_different_reasons`.
- [x] 9.5 Write `a_keystore_failure_is_an_answer_and_not_an_error_reply` and
      `a_malformed_request_is_the_error_shape_rather_than_a_capability` — the
      two sides of the deliberate departure from §2.5.
- [x] 9.6 Write `the_probe_reports_the_identity_for_the_stoa_it_was_asked_about`
      (§5.2: one identity per Stoa) and
      `the_probe_is_never_a_panic_even_when_the_lookup_panics`.
- [x] 9.7 Implement `get_capabilities` and `capability_for`; add the probe to
      `every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape`
      and re-export from `lib.rs`.

## 10. Mutation verification

- [x] 10.1 Break each security property in turn, run the suite, record which
      tests fail, restore. Fourteen mutations; two survived and were fixed by
      tests that now exist (4.6 and 6.4). Table in the report.

## 11. Gates

- [x] 11.1 `cargo test -p dialectica-core` — green.
- [x] 11.2 `cargo fmt --check` — clean.
- [x] 11.3 `cargo clippy -p dialectica-core --all-targets -- -D warnings` —
      clean.

## 12. Documentation

- [x] 12.1 Replace PLAN.md §5.6's specification with a one-line summary that the
      keystore exists, per the document model — the reasoning now lives in
      `design.md` and keeping a second copy is the failure mode that model
      exists to prevent.
