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
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] CI green, PR merged — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`

### What the `tester` did, and the blind spot that remains

This piece's row was left **unticked rather than struck**, unlike `core-e2e`'s,
because there was genuinely something a `tester` could do here. There was, and it
is the shape this note predicted: **not in `examples/`, but in
`dialectica-core/tests/end_to_end.rs`**, as an integration test over a `tempdir`
that mints a keystore, founds a Stoa, records a path, publishes and reads back,
independently of the binary.

Two tests, in a new `publish path` section of that file — added under its own
sectioning rule 2, which had named the write path as the boundary it was waiting
for:

- `a_store_seeded_from_two_identities_carries_exactly_those_two_authors` — the
  distinct-author **set** over `authoring::post`/`reply`/`vote`, which is what a
  `contains` pair structurally cannot assert and is what the false "every seeded
  op is by ⟨one address⟩" report line slipped past.
- `the_seeding_sequence_builds_a_nested_thread_whose_author_the_probe_does_not_report`
  — the sequence above, plus the three-level nesting and the self-invalidating
  probe-versus-signing inequality.

Three mutations were run, each predicted before the run and each observed as
predicted; the table is in `end_to_end.rs`'s own mutation section, per that
file's rule. The load-bearing one: the reviewer's `wire.rs:324` mutation, which
closes the three-derivations gap at the probe. It makes the example **exit 0**
and now makes this suite **fail**, which is exactly the difference the boxes were
about.

**The blind spot itself is not closed, and that is deliberate.** `examples/` is
still a target CI compiles and never runs. The example keeps its inline
assertions — they are what makes a bad *run* of the tool loud, and a test and a
run answer different questions. What changed is that the properties no longer
live only there. A workflow step that runs examples was weighed and declined: it
would gate on a dev tool's exit code, and `design.md` already records why a third
Rust step duplicating two working gates is the wrong move. That decision is a
`ci.yml` change and outside a tester's remit either way.

**A `#[test]` must never be added to `examples/seed_store.rs`.** CI's test-count
gate counts declarations across the whole Rust tree with `examples/` in scope and
cargo does not run an example's tests, so one declared there fails the job. That
is the correct outcome; the file says so at the top. The two tests above are in
`tests/`, which cargo does run, so `declared` and `ran` both move together.

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
      thread read — was recorded here as not existing in core, which was true at
      the time and is **not true now**: it merged as #61 (`53f08b3`), one commit
      past this branch's merge base. See the design round below; the correction
      that went back to the correctness reviewer was right when written and has
      been superseded. Measured discriminating.
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

### Acting on review (readability + architecture + spec-test + design, 12 boxes)

Ten `dev-writer` boxes closed; two `tester` boxes left open, and the reason each
stays open is appended to it in `findings/spec-test.md`.

- [x] **The report's false universal.** "Every seeded op is by ⟨one address⟩" was
      false for five of nine ops. Reproduced before editing. The block now prints
      four labelled addresses with a per-author op count against each, counted back
      from the store by `seeded_ops_by` rather than restated from the writes.
- [x] **An assertion over the distinct-author *set*.** The two checks beside the
      claim were `authors.contains(...)` — existential where the claim was
      universal, so no `contains` could ever contradict it. Added a set assertion
      over all nine ops via `OpLog::iter`. Proved it fails: a third author panics;
      the collapse to one author panics at the feed assertion just above it.
- [x] **The `assert_ne!` that could not fire.** Its two operands were keystore
      derivations this example made itself, so it asserted that two HD paths
      differ. The left operand is now `wire::posting_identity`'s own answer — the
      module's value, not a re-derivation of it. Re-ran the reviewer's mutation at
      `wire.rs:324`: the seeder now exits 101 where it exited 0, printing the
      message that names what to delete. `wire.rs` restored.
- [x] **The probe's address, re-derived by hand.** Same edit. The two agreed,
      which was the problem — `keystore.rs:448-453` records that two call sites
      agreeing is not one derivation, after such a pair re-diverged with every gate
      green, and this file is compiled by no test and run by no gate.
- [x] **`ops.sqlite` spelled twice.** `const OPS_LOG` plus an `ops_log_path(dir)`
      helper that `store_files` and the open both call. A helper rather than a
      bound path because `usage()` calls `store_files` before `main` has a
      directory.
- [x] **The module docstring's uniform-authorship claim**, 620 lines above the
      report line and sharing no phrasing with it. Corrected, plus a paragraph on
      the four/five split so a reader meets the asymmetry in the preamble.
- [x] **Assertion messages that fight `assert_eq!`.** Dropped `", not {found}"`
      from both counts; re-ran the reviewer's reproduction to see the message read
      correctly on failure.
- [x] **`no---fresh` and a 121-character help line.** Fixed by wording and by
      breaking the filename list onto indented lines. Verified by running `--help`,
      which is the only way either was visible.
- [x] **`SEEDED_PATH = 0`'s false docstring.** Zero is not a path onboarding
      produces. Re-measured over 2000 generated nonces: zero hits. Docstring
      corrected and a Decisions entry added with both alternatives and the cost.
- [x] **Two wrong figures**, "six of eight" error types where it is ten of twelve.
      Re-counted; replaced the number with the relation and the two greps that
      answer it, in both `design.md` and the module docstring.
- [x] **`design.md` had no rule about what the report may claim**, which is the gap
      the false universal fell through. New Decisions entry recording the rule,
      what it forecloses, and the rejected alternative.
- [x] **The `cargo fmt` gate cannot reach this crate.** Re-ran `cargo fmt … -v`:
      two files, neither in `dialectica-core`. Recorded in `design.md` and the
      module docstring as a pre-existing repo-wide gap, not this piece's to fix.
- [x] **The "core has no thread read" justification**, false since #61. Removed
      from `design.md` and the code. The assertion is **not** moved onto
      `read_thread`: it does not exist on this branch, whose merge base predates
      the merge, and this piece may not pull `main` in. The follow-up is recorded
      in `design.md` and at the assertion site, with every argument `read_thread`
      takes noted as already in scope.
- [x] **`docs/UI-BRIEF.md` told the designer creator and poster are the same key.**
      Grepped for citers first — nothing outside the brief and the findings file.
      The factual claim is replaced by what is true (they are different keys today,
      so moderation does not bind, a known gap under review, citing `ci.yml`'s
      exemption and this `design.md`); the design instruction about different
      *people* survives.
- [x] Re-ran the gates: seeder green on a fresh directory, `--help` checked by
      running it, `rustfmt --check` clean on the example, `clippy --all-targets -D
      warnings` clean for `dialectica-core`, suite 733 + 26 unchanged.
