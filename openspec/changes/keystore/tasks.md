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
      tests fail, restore.

      **Inline, not "see the report".** A reader of this file has to be able to
      check whether these recorded their survivors — which is the exact defect
      §10.2 is about, and leaving the first fourteen out of band would have
      left it unverifiable in the document whose credibility it damaged.

      | # | Mutation | Result |
      |---|---|---|
      | 1 | Permission check disabled | 1 fails |
      | 2 | Wrong passphrase accepted (AEAD error → unauthenticated fallback) | 7 fail |
      | 3 | Never-prompt replaced by `stdin().read_line()` | 1 fails |
      | 4 | AAD deleted entirely | **SURVIVED** → fixed, §4.6 |
      | 5 | KDF cost ceiling removed | 1 fails |
      | 6 | Write goes straight to destination | **SURVIVED** → fixed, §6.4 |
      | 7 | `CanPost` also carries `reason` | 2 fail |
      | 8 | `Locked` and `WrongPassphrase` collapsed | 2 fail |
      | 9 | Environment consulted before the file | 1 fails |
      | 10 | `create` overwrites | 1 fails |
      | 11 | Write mode 0600 → 0644 (LEZ's) | 9 fail |
      | 12 | Unknown protection defaults to `None` | 1 fails |
      | 13 | Empty-passphrase refusal removed | 1 fails |
      | 14 | Salt and nonce fixed at zero | 1 fails |

- [x] 10.2 **The table above was incomplete, and that is the finding.** It
      claimed fourteen verifications and did not include the one on this
      change's central property: deleting `bytes.zeroize()` from
      `Keystore::generate` left all 184 tests green. Found by review, not here.

      **A mutation table listing only kills hides its own misses**, which is
      worse than a missing test because it stops anyone looking. The survivors
      are now rows, not footnotes:

      | # | Mutation | Result |
      |---|---|---|
      | 15 | Delete the wipe in `generate` | **SURVIVED** → fixed structurally, §12.1 |
      | 16 | `root` field loses `Zeroizing` | compile error, names `must_be_zeroizing` |
      | 17 | Skip `Zeroizing`'s destructor | 1 fails |
      | 18 | The demonstrated "names a fix" bypass | 1 fails |
      | 19 | Probe ignores its lookup | 9 fail |
      | 20 | Two error messages collide | 1 fails |
      | 21 | Add a stored passphrase verifier | 9 fail |
      | 22 | `stoa_address` reports the wrong identity | 2 fail |
      | 23 | `Fn` reverted to `FnOnce` | **SURVIVES** — see below |

      23 is recorded as a survivor rather than quietly dropped. `FnOnce` is a
      supertrait of `Fn`, so `&F` satisfies it and the signature change is not
      observable; it is justified on honesty rather than enforceability, and
      claiming otherwise would be the same defect as the original table.

## 11. Security review findings

Everything below was found by review after the first three commits, each with a
working proof of concept. Recorded as its own section rather than folded into
the sections above, because *what the original tests could not see* is the part
worth keeping.

- [x] 11.1 **[CRITICAL] Symlink capture of the root secret via a predictable
      staging path.** `.mode()` applies only on creation, so an existing
      symlink at `identity.tmp` was followed and the seed written through it in
      the clear, at the attacker's mode, after which the rename completed
      normally and nothing looked wrong.

      Regression test written FIRST and watched fail —
      `the root secret was written through a planted symlink`. Fixed with
      `create_new(true)` and a randomised staging name; **each verified
      sufficient alone** by reverting the other and re-running.

      What let it through: the write test checked the *destination* after the
      rename. The staging file was never examined by any test.
- [x] 11.2 **[HIGH] The per-knob cost ceilings multiplied.** m=1 GiB, t=32,
      p=16 were each inside their own cap and together measured 302 seconds —
      15x the caller's 20-second timeout, uninterruptible, with nothing for
      `guarded` to catch. Replaced with `MAX_WORK_FACTOR` bounding the product.

      The lesson, which generalises: **the boundary was tested and the product
      of boundaries was not.** `a_cost_at_the_ceiling_is_still_attempted` varied
      one knob and said so honestly, which is exactly why the corner was never
      executed.

      The new test asserts the REFUSAL is instant rather than that the
      acceptance is tolerable — the bound's job is to reject before any work,
      which is a property of the code; a wall-clock budget at the ceiling only
      asserts the machine was fast enough that day, and cost ~5s every run.
- [x] 11.3 **[MEDIUM] check-then-read resolved one name twice**, and
      `fs::metadata` follows symlinks. Replaced with a single open and
      `File::metadata()` on the handle. `open_from_env` likewise now reads once
      instead of twice — it was safe only incidentally, because
      `from_file_bytes` re-derives protection from the bytes it decodes.
- [x] 11.4 Bound how much is read from an attacker-supplied path, via `take`
      as well as the size check — `metadata().len()` is 0 for a FIFO.
- [x] 11.5 Check the **containing directory's** write bits. It is what made
      11.1 possible, and a 0600 keystore in a 0777 directory is not protected
      by its own mode. Write bits only: a readable directory discloses only
      that a keystore exists.
- [x] 11.6 Make "adding `#[derive(Debug)]` here is a security regression"
      visible to the person about to do it, rather than enforced by absence.
      Verified by adding the derive and watching the test fail.
- [x] 11.7 Add spec requirements for the cost bound, the intermediate path, the
      single-open read and the directory check. Each existed only in code and
      design.md, so nothing stopped a later contributor relaxing one as a
      convenience.

## 12. Spec-test review findings

A second review, blind to the implementation. Two surviving mutations and four
untested or untestable scenarios.

- [x] 12.1 **[HIGH] Zeroization had no test at all.** Every `Zeroizing` in the
      file could have been removed with nothing failing beyond compile errors,
      and the explicit wipe in `generate` could be deleted with nothing failing
      at all.

      Fixed **structurally rather than with a test**, because the wipe was not
      testable: a stack local after its function returns is not observable.
      `generate` no longer makes the copy — `to_bytes()` moves straight into
      the `Zeroizing`. `Zeroize` is no longer imported by the library, so a
      reappearing `use` is the signal someone has reintroduced a hand-rolled
      wipe.

      Plus the two tests the reviewer named: behavioural (raw pointer post-drop,
      with a control proving the technique can see the difference) and
      structural (a compile-time bound on every secret-bearing field).
- [x] 12.2 **[MEDIUM] `every_error_message_names_a_fix` passed for the wrong
      reason**, demonstrated: a pure fault statement with "restore" as a noun
      stayed green. Now requires the verb at a word boundary in the guidance
      clause after `;` or an em dash. Its own regression test pins the exact
      bypass.
- [x] 12.3 Restate the two **unobservable** requirements as outcomes —
      read-once → "content that passed no permission check is never used";
      tag-decides → "nothing stored can verify a passphrase on its own". Both
      now have tests; the second kills a stored verifier with 9 failures.
- [x] 12.4 Change `lookup` to `Fn` so "the probe is callable repeatedly" is
      satisfiable, and test it. Recorded as a **surviving mutation**: `FnOnce`
      is a supertrait, so the change is not observable.
- [x] 12.5 Test that an occupied staging path is refused directly, rather than
      only through the symlink test — which passes with `create_new` reverted,
      because the randomness catches it there.
- [x] 12.6 Sweep **all 17** error variants pairwise for distinguishability,
      not the 7 the spec happens to name. `UnknownVersion`/`UnknownProtection`
      and the three "restore it from a backup" variants were the near misses.
- [x] 12.7 Test the reported identity **end to end** — derive, sign, and verify
      the op is attributed to the address the probe named, plus a negative so
      the assertion is not vacuous. Every other probe test used the literal
      `"abcd"`.
- [x] 12.8 Decide whether reason text is a stable interface. **It is not** —
      recorded as a spec requirement, because reasons must stay free to improve
      and one has already been rewritten for naming a fault without a fix.
- [x] 12.9 Add the three missing `NO SPEC:` markers.

## 13. Gates

- [x] 13.1 `cargo test -p dialectica-core` — green.
- [x] 13.2 `cargo fmt --check` — clean.
- [x] 13.3 `cargo clippy -p dialectica-core --all-targets -- -D warnings` —
      clean.

## 14. Documentation

- [x] 14.1 Replace PLAN.md §5.6's specification with a one-line summary that the
      keystore exists, per the document model — the reasoning now lives in
      `design.md` and keeping a second copy is the failure mode that model
      exists to prevent.
