# Tasks — key-sweep

## Stages

- [x] spec — `spec-writer`. 22 requirements rewritten, 1 removed, 4 renamed,
      across six capabilities. `content-authoring` was audited and deliberately
      left alone; the proposal says why, so that "considered" and "missed" are
      distinguishable from the delta alone.
- [x] design + code — `dev-writer`. `PublicKey::address`,
      `AUTHOR_ADDRESS_PREFIX` and five keystore/wire accessors deleted;
      `verify_authored_op` lost its claimed-author parameter and the guard that
      compared a value to itself. 987 Rust tests green (unchanged count: 2
      deleted, 2 added, 12 renamed — every one accounted for in the handover),
      286 QML assertions green, clippy clean, `nix build .#lgx` green so the
      `cfg(logos_scaffold)` adapter is known to compile.
- [x] tests — `tester`. Test-list diff against `origin/main` verified name by
      name: 13 gone, 13 new, 987 both sides — 11 renames, 2 deletions (both
      properties OF the deleted derivation), 2 additions. The handover said "2
      deleted, 2 added, 12 renamed"; the twelfth was a rename of a top-level
      integration test, so the shape is right and the count of renames was one
      over. Five mutations run and restored, each failing as predicted — the
      retired author-address pin proved to be the deleted derivation's real
      output (repointing `STOA_ADDRESS_PREFIX` makes
      `no_derivation_turns_a_public_key_into_an_address` fail `left == right`).
      **Three surviving comments credited the deleted address re-derivation for
      a refusal the SIGNATURE check produces** — `op.rs`'s two test comments
      (which task 5.2 claims to have corrected), `transport.rs`'s forged-op test
      and `revision.rs`'s module doc; all corrected, the mechanism measured by
      stubbing `verify_op_bytes`. `an_op_whose_key_does_not_bind_to_its_claimed_
      author_is_refused` gained the control clause its own spec scenario names —
      that the forged signature is VALID under the attacker's key — without
      which a junk-signature fixture passed identically.
      `verification_consults_the_key_..._and_nothing_beside_it` renamed to
      `verification_takes_no_author_identifier_beside_the_key`: the arity check
      discharges the scenario's first clause structurally and only the
      `supplied` half of its second, and the old name claimed the `derived`
      half too. Both `// NO SPEC:` markers confirmed genuinely unspecified.
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`. 3 findings, none blocking on
      shape: 2 for `spec-writer` (the proposal promises two bare "The address is
      the identity" sentences are scoped to say Stoa and neither delta makes the
      edit — `stoa-metadata` has no delta at all; and one public key is now
      reported under three field names across six replies, which this change
      created by collapsing three values into one and records nowhere), 1 for
      `dev-writer` (the shared `address` property's declaration says nothing
      about now holding two kinds of value). **Leaving `Address` unenforced is
      defensible** — every non-test `from_bytes` site is a Stoa address and
      `OpId` is already its own type, so the newtype buys less than before;
      the absence pin leaves a runtime witness. `verify_authored_op` still earns
      its name and is not a pass-through — it owns the three parse refusals on
      the attacker path and its arity is compile-gated. 987 Rust tests green.
- [ ] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`. 3 findings, all for `dev-writer`, all
      gaps in the record rather than code contradicting it. Every one of the
      seven Decisions is taken as recorded, and Decision 7's six `origin/main`
      line citations were each read and are accurate. Both `// NO SPEC:` markers
      exist (`feed.rs:657`, `wire.rs:6696`) and both cite a design entry that
      exists. `docs/PLAN.md` is consistent — the one surviving "The address is
      the identity" (`:3060`) is Stoa-scoped by its own paragraph. The gaps: the
      rotation affordance is given up with no Decisions entry (it migrated to the
      `identity` spec and `identity.rs` but not here); the slate distinctness
      clause was *replaced* with derivation-path rather than dropped, unrecorded;
      and `design.md:128` cites an `op-transport` "verification step" the spec
      does not contain — measured, zero grep matches either side.
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## Implementation

Ordered by dependency: the derivation goes first, because every other task is a
call site the compiler names once it is gone.

## 1. Delete the derivation

- [x] 1.1 Delete `PublicKey::address` and `AUTHOR_ADDRESS_PREFIX` from
      `identity.rs`, leaving a comment where the method was saying what it
      derived and why not to add one back. Verify: `cargo build -p
      dialectica-core` fails only at call sites, never inside `identity.rs`.
- [x] 1.2 Rewrite `Address`'s doc comment to state the survivor positively — an
      address identifies a Stoa — and record that `derive_stoa_key`'s `Address`
      parameter is an *input* rather than a leftover. Verify by reading: the
      phrase "One type for both" appears nowhere.
- [x] 1.3 Drop `verify_authored_op`'s `author: &Address` parameter and the
      `key.address() != *author` guard. Verify: the new
      `verification_consults_the_key_the_op_carries_and_nothing_beside_it` binds
      `verify_authored_op` to a `fn(&[u8], &[u8], &[u8]) -> bool`, so a
      reintroduced parameter is a COMPILE error rather than a silent widening.
- [x] 1.4 Retire the author-address pin without touching the other three.
      Verify: `no_derivation_turns_a_public_key_into_an_address` passes, and its
      pinned hex was MEASURED to be the deleted derivation's own output — a probe
      recomputing `SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || key)` inline was run
      and watched fail an `assert_ne!` against it.

## 2. Delete the accessors that returned one

- [x] 2.1 Delete `Keystore::identity_address`, `Keystore::stoa_address` and
      `Keystore::stoa_address_at_path`, driving the deletion by RETURN TYPE
      rather than by name — the last two take a Stoa address and return an author
      one. Verify: `identity::stoa_address` (the Stoa derivation) still exists and
      `the_wire_constants_are_pinned_to_known_answers` still pins it.
- [x] 2.2 Collapse `creator_and_poster_in` and `poster_address_in` into
      `creator_key_in`, whose pair's second element was a pure function of its
      first. Verify: `the_creator_a_creation_names_is_the_identity_the_probe_reports`
      passes through both wire handlers, and the adapter compiles under
      `nix build .#lgx`.

## 3. Re-point the six reply sites

- [x] 3.1 `feed.rs`'s `FeedRow::author` and `thread.rs`'s `ThreadItem::author`
      carry the key's hex. Verify: `the_author_is_the_public_key_that_signed` and
      `every_item_carries_the_key_that_signed_and_no_second_identifier`.
- [x] 3.2 Delete `ThreadItem::author_key` and the `authorKey` JSON field, the
      second of two fields whose justification is gone. Verify: the thread key-set
      assertions in `a_thread_reply_carries_exactly_its_contracted_keys_and_no_others`
      compare the WHOLE set, so a restored `authorKey` fails there.
- [x] 3.3 Delete `Candidate::address`, `Kept::Stored::address` and
      `Whoami::Identity::address` — each had a `publicKey` already beside it.
      Verify: the three pinned JSON shapes, plus
      `a_slate_reply_carries_a_public_key_and_no_address_for_every_candidate`.
- [x] 3.4 Point `posting_identity` at `stoa_public_key_at_path`. Verify:
      `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` and
      `the_key_a_publish_signs_with_is_the_identity_the_probe_reports`.

## 4. The view

- [x] 4.1 Rename `PostHeader.identityAddress` to `identityKey` and rewire
      `FeedScreen`. Verify: `check_qml_members.sh` and the QML suite pass, and
      `check_qml_names.py` reports clean.
- [x] 4.2 Point `DOnboardingScreen` at `publicKey` on the slate candidate and the
      keep reply — `isCandidate`, the kept-reply guard, `keptIdentity`, and the
      row and kept-state bindings — and correct the uniqueness note to name the
      public key, which the spec requires. Verify: 286 QML assertions green with
      `check_bindings` reporting no undefined binding. **Measured:** reverting one
      kept-state binding to `keptIdentity.address` leaves all 52 assertions in
      that spec "passed" and is caught only by `check_bindings`, so that gate —
      not the assertions — is what covers this.
- [x] 4.3 Leave `Identicon` and `AddressLabel` alone, both generic over a Stoa
      address, and correct `Identicon`'s comment, which said the rename was this
      piece's to make. Verify by reading: neither component's `address` property
      changed, and no comment claims a pending rename.

## 5. Correct every claim the code disproves

- [x] 5.1 Correct `op.rs`' three claims that `verify_authored_op` "re-derives"
      an address or "binds the key to the claimed address", and `identity.rs`'
      paragraph saying the address check is why the function exists.
- [x] 5.2 Correct the two test comments crediting the address check for a
      refusal the SIGNATURE check produces — `op.rs`'s forged-op test and
      `revision.rs`'s `a_forged_revision_is_dropped`. **These tests passed before
      and pass now**; only the stated reason was wrong, and it was wrong before
      this change too, since the only caller derived the claimed author from the
      op's own key.
- [x] 5.3 Scope `stoa.rs`' bare "The address is the identity" to say Stoa, and
      correct `transport.rs`' `FailsVerification` doc, which described a binding
      step that no longer exists. Verify: `grep -i "author address"` over
      `dialectica*/src` returns only sentences about the deletion itself.
- [x] 5.4 Put `names.rs`, `Identicon.qml`, `docs/PLAN.md` and `docs/IDENTICON.md`
      into the past tense — each said issue #80 "deletes" the address, and one
      said `FeedRow` "currently has" the address-only gap, which the same change
      closes.

## 6. Gates

- [x] 6.1 `cargo test -p dialectica -p dialectica-core`: 987 green, the same
      count as the baseline. Accounted for exactly: 2 tests deleted (both about
      the deleted derivation's own properties), 2 added, 12 renamed.
- [x] 6.2 `cargo clippy --all-targets -- -D warnings`: clean, no new warning.
- [x] 6.3 `nix build .#lgx`: green. This is the ONLY gate that compiles the
      `#[cfg(logos_scaffold)]` adapter, where a compile error reached review on
      #85 with every local gate passing.
- [x] 6.4 `run-qml-tests.sh` and `check_qml_names.py`: green.

**Not done, deliberately.** The feed reply's missing capability is not written —
the proposal records the gap and says a feed contract is a capability's worth of
work. The `// NO SPEC:` markers on the feed row's `author` and the thread item's
surviving field name are what make the two choices this left visible.

**Not done, pre-existing.** The nine unformatted files in `dialectica-core` are
untouched; that is the known CI fmt-gate gap and reformatting them would bury
this diff.
