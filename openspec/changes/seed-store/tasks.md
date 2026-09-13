## Stages

- [ ] ~~spec — `spec-writer`~~ — **does not apply.** A developer tool adds no
      requirement: the seeder calls only public API that `stoa-membership`,
      `content-authoring`, `keystore`, `identity-onboarding` and
      `module-wire-contract` already contract, and it adds no method, field or
      reply shape to the module surface. `.openspec.yaml` sets `skip_specs: true`
      with that reason. Struck through rather than omitted, because "does not
      apply" and "nobody did this" are different states and this block exists to
      tell them apart.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] CI green, PR merged — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`

### A note for the `tester`

This piece's row is left **unticked rather than struck**, unlike `core-e2e`'s,
because there is genuinely something a `tester` can do here and it is not
obvious what.

What the example asserts about itself, it asserts inline, and those assertions
run — a failing one exits non-zero. What nothing checks is that the program still
*works*: CI compiles it (measured, see `design.md`) and nothing runs it. The gap
a `tester` could close is in `tests/end_to_end.rs`, not in `examples/`, and the
shape worth considering is whether the seeding sequence — mint a keystore, found
a Stoa, record a path, publish, read back — holds as an integration test over a
`tempdir`, independently of the binary.

**A `#[test]` must never be added to `examples/seed_store.rs`.** CI's test-count
gate counts declarations across the whole Rust tree with `examples/` in scope and
cargo does not run an example's tests, so one declared there fails the job. That
is the correct outcome; the file says so at the top.

## Implementation

- [x] Recover the #31 original from `8ff562f` and read it as a specification of
      what the tool should do, not as code to apply. Confirmed it does not apply:
      `publish::*` is now `authoring::*` with different signatures,
      `log::StoaRegistry` no longer exists, `create_stoa` has moved to the wire
      layer over a `MembershipStore`, and `SecretKey::generate()` as an identity
      source has gone from "the only option" to actively wrong.
- [x] Decide which crate. `dialectica-core`, for three reasons recorded in
      `design.md`; the deciding one is that the outer crate does not build in a
      plain checkout at all.
- [x] Establish that nothing behind `cfg(logos_scaffold)` is needed. Every path
      the adapter derives is a `core` function over a `&Path`, so the host
      directory is an ordinary argument.
- [x] Write the four store paths through the `core` functions that name them —
      `keystore::default_path_in`, `IdentityStore::default_path_in`,
      `membership::membership_path_in` — and flag the one with no accessor
      (`ops.sqlite`, spelled inline by the adapter) as copied by hand.
- [x] Mint a keystore with `protection_from_env`, name `identity_public_key` as
      the creator, sign with `stoa_key`, record a chosen path. Each position
      matches the module's.
- [x] Refuse an existing store before the first open; `--fresh` deletes the four
      by name, printing each.
- [x] Seed two roots, three replies at two levels of nesting, four votes from two
      identities in both directions.
- [x] Read the feed back through `feed::list_threads` and assert two hardcoded
      numbers against it.
- [x] Assert the creator moderates the Stoa it founded, against the record rather
      than a local variable.
- [x] Print the address, the record, a pasteable `listThreads` request, and the
      three `Main.qml` properties spelled as that file declares them.
- [x] Print both author addresses and assert they still disagree, so the day the
      three-derivations gap closes this fails and names what to delete.
- [x] Prove the author assertion discriminates: pointed at `posting_address` it
      fails, which was run. Restored.
- [x] Measure whether CI compiles examples rather than assuming. Planted a type
      error; both `cargo test` and `cargo clippy --all-targets` failed on it. No
      workflow change made.
- [x] Verify the printed request end to end: fed the exact emitted JSON to
      `wire::list_threads_from_request` against the seeded directory, which
      returned the two-row feed.
- [x] Confirm zero `#[test]` in the example, so CI's count gate stays balanced.
- [x] Gates: `rustfmt --check` clean on this file, `clippy --all-targets -D
      warnings` clean, full suite green.

### Acting on review (correctness + security, 10 findings, all `dev-writer`)

- [x] **The tautological moderator assertion.** Reproduced the real property
      first: `contains(&founder.public_key())` panics, so the signing key is not a
      moderator and a hide against a seeded Stoa is refused. Replaced with both
      halves asserted as they are, the negative one self-invalidating; report gains
      the consequence in words; `design.md` gains a section.
- [x] **Nesting unasserted.** Added `parent`/`thread` assertions over all three
      replies, read back through `OpLog::get`. The reviewer's suggested route — a
      thread read — does not exist in core (`feed.rs` has `list_threads` only;
      `piece/thread-read` is in flight), so that correction is recorded in the
      finding. Measured discriminating.
- [x] **Both feed rows had one author.** `second_root` is now the visitor's, so
      the docstring's two-identity claim is true in the feed; the assertion checks
      both rows by lookup rather than indexing `items[0]`.
- [x] **`--fresh` deletes before asserting.** Documented in the help text; not
      fixed by reordering, because the assertions are about the store the run
      writes. The staging-and-rename alternative is named and declined as widening.
- [x] **`identities.sqlite` typo** in `proposal.md`.
- [x] **`expect` on the recorded path** → `ok_or_else(…)?`, so every failure in
      `main` is a one-line message.
- [x] **Root secret into a world-writable directory.** `keystore::open_in` after
      writing, reusing the existing guard rather than copying it. Reproduced: the
      `chmod 777` case now exits 1 naming the mode.
- [x] **Protection never reported.** One report line, matched on the `Unlock`
      value actually used to write the file rather than re-reading the environment.
- [x] **`--fresh` deletes by name without checking.** Guard before the first
      deletion, over `identity.key` only, via `Keystore::is_encrypted` rather than a
      copied `MAGIC` byte. Reproduced both ways; the decoy file survives.
- [x] **No pointer to the contradicting docstrings.** Added to the module
      docstring, naming both and saying the adapter disagrees with them. The
      docstrings themselves are left for whoever resolves the gap — editing
      `keystore.rs` would widen this piece.
- [x] Re-ran all gates after the fixes: `rustfmt --check` clean, clippy clean for
      both packages, suite 733 + 26 unchanged, seeder exercised on a fresh
      directory, an existing store, `--fresh`, a world-writable directory and a
      non-dialectica directory.
