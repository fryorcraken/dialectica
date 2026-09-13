## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

**Written late, and every row but the runner's was already done when it was
written** — which is the cost task 7.8 recorded rather than a reason not to write it.
`spec-writer` owns this block and it was missing from the first spec pass; `dev-writer`
declined to author it because its own file forbids adding rows, which was the right
call about rows and left the block absent altogether.

**Each review row is ticked against a file, not a memory.** There is one
`findings/<dimension>.md` per row — `correctness`, `security`, `readability`,
`architecture`, `spec-test`, `design-review` — each written by the instance that did
that review, so a tick here is checkable rather than asserted. The two runner rows are
unticked because they are the runner's and neither has happened: `findings/` still
exists, and the change is not archived.

**A reader arriving now should not conclude the stages were tracked as they ran.**
They were not. What this block is good for from here is the two rows that are still
open.

Every box below is checked because the work was done AND the verification named
in it was run. Where reality differed from the plan, the task carries a note
rather than a tick alone — a checklist that hides a correction is worse than one
that never made the claim.

## 1. Derivation takes a path

- [x] 1.1 Add `derive_stoa_key_at_path(root, stoa, path)` to `identity.rs` under a
  new salt constant `/dialectica/2/Identity/Stoa`, leaving `derive_stoa_key`
  byte-identical — verified: `the_wire_constants_are_pinned_to_known_answers`
  passes with its original four assertions untouched.
- [x] 1.2 Extend that pinned test with a hardcoded expectation for the new
  derivation at a fixed root, Stoa and path — verified by watching it FAIL twice:
  once with the salt reverted to version 1, once with the path encoded
  little-endian. Both expected values were computed with `openssl kdf`, and that
  invocation was first validated by reproducing the existing version-1 pinned
  value exactly — which is what makes them independent rather than merely
  plausible.
- [x] 1.3 Add tests for determinism, different paths giving different keys, and
  the two schemes not colliding at path 0.

  **Correction worth recording:** the non-collision test does NOT fail when the
  salt is reverted, and the code comment now says so. Reverting the salt still
  leaves the two schemes distinct, because the path-taking one appends four bytes
  to the HKDF info. That is precisely why the salt bump is argued in `design.md`
  as a *decision* rather than relied on as a mechanism — and why 1.2's pinned
  assertion, not this test, is what guards it.
- [x] 1.4 Give `Keystore` a `stoa_key_at_path` / `stoa_public_key_at_path` /
  `stoa_address_at_path` trio — verified by
  `a_path_taking_keystore_identity_is_the_one_that_signs`, which signs and checks
  `verify_authored_op` against the reported address, with a negative case so the
  assertion is not vacuous.

## 2. The path record on disk

- [x] 2.1 Add `identity_store.rs` with a two-column SQLite table and its own
  `PRAGMA user_version` — verified: a fresh file creates, version 9999 is refused
  as `UnknownLayoutVersion` naming both numbers, a version stamped over a missing
  table is refused as `LayoutDoesNotMatchItsVersion`, and a table missing the
  `path` column is refused too. No `unwrap`/`expect` outside `#[cfg(test)]`.
- [x] 2.2 Implement `record_path`, `path_for(stoa)` and `all_paths()` — verified
  across a real file reopened twice, two Stoas recorded separately, and every
  pairing read back against a hardcoded expected set.

  Also verified by mutation: replacing `INSERT` with `INSERT OR REPLACE` makes
  `a_second_choice_for_one_stoa_is_refused_and_changes_nothing` fail, and making
  `path_from_row` clamp with `as u32` makes
  `a_stored_path_outside_u32_is_refused_rather_than_clamped` fail.
- [x] 2.3 Verify nothing machine-local is stored — verified by
  `reading_the_record_back_needs_nothing_beyond_the_record`, which reads the file
  with a SECOND connection sharing nothing with the writer and asserts the table
  holds exactly the two named columns.
- [x] 2.4 Feed the store arbitrary and truncated file content — verified. Note
  that SQLite treats a zero-length file as a fresh database, so the empty case
  *opens*; the test therefore also exercises every read on an accepted file,
  which is what makes the assertion about not panicking rather than about not
  opening.

## 3. The slate

- [x] 3.1 Add `onboarding.rs` with `SLATE_SIZE = 5`, a `SlateNonce`, and
  `derive_path` masking the top bit — verified by watching both the pinned
  constants test and `every_derived_path_is_below_two_to_the_thirty_one` FAIL
  with the mask removed. The two pinned values were computed with `openssl dgst`
  and the mask applied by hand; both digests have the top bit set, so a mask that
  was accidentally a no-op cannot hide.
- [x] 3.2 Implement `Slate::generate` walking the index forward until five
  distinct paths are held — verified: no two candidates share a path, a public
  key or an address, and a nonce reproduces an identical slate.
- [x] 3.3 Verify two generated slates differ — verified over eight regenerations
  rather than one pair, since a single pair differing could be luck.
- [x] 3.4 Verify a slate's values carry no secret — verified twice, by a byte
  search and by taking every exposed value as key material and checking none
  signs as any candidate. Both tests carry a positive control, so a broken search
  fails rather than passing silently.

  **This task found a real fixture defect.** The byte search failed on its first
  run because the master key and the nonce were both `[7u8; 32]`, so the search
  found the nonce and reported it as the master key — two explanations, one
  answer. The fixture now uses a distinct master key and asserts the two differ.
- [x] 3.5 The secret material held while deriving a slate is in a `Zeroizing`
  buffer.

  **No test asserts this, deliberately, and that is worth stating rather than
  ticking past.** `keystore.rs`'s own `generate` records that review found its
  explicit `bytes.zeroize()` deletable with the whole suite green, because a
  stack local after its function returns is not observable from a test. The same
  applies here. What was done instead is the shape that cannot be forgotten: the
  derived key is moved straight into the `Zeroizing` with no plain binding for
  anyone to omit a wipe on. A test claiming to cover this would be the coverage
  claim the keystore change was criticised for.

## 4. The wire methods

- [x] 4.1 Add `generate_identity_slate` — verified: the count is reported as a
  hardcoded 5 (not read back from the array), offering a `count` field changes
  nothing, malformed input is the error shape with no result beside it, and the
  reply's JSON key names are pinned.

  Additionally verified that a refused request does NOT call `remember`, so a
  malformed call cannot supersede a slate the user is still looking at.
- [x] 4.2 Add `keep_identity` — verified: an out-of-range selection is refused
  and writes neither store, a superseded nonce is refused, and no live slate at
  all is refused. Verified by mutation: removing the nonce comparison makes
  `a_selection_against_a_superseded_slate_is_refused_rather_than_satisfied` fail.
- [x] 4.3 Verify the write order and the second-keep refusal.

  **The write-order half was a gap this task found.** Reversing the two writes
  left the entire suite green — measured, not assumed — so
  `a_keep_whose_keystore_write_fails_records_no_path` was written for it, and
  watched to fail with the order reversed (`left: Some(11140858), right: None`).
  Without it, `design.md`'s central claim about atomicity would have been
  unpinned.
- [x] 4.4 Verify the encryption report — verified in both directions, and against
  the FILE via `Keystore::is_encrypted` rather than only against the argument, so
  a reply whose boolean had drifted from what was written would fail.
- [x] 4.5 Add `who_am_i` — verified: all four states of `design.md`'s table
  produce pairwise-distinct reasons, identity and reason are never both present
  or both absent, the reported public key derives the reported address, and
  `recoveryNeedsTheRecord` is carried.
- [x] 4.6 Verify no new method aborts — verified over eleven arbitrary inputs
  against all three methods, plus a panicking keystore dependency and a panicking
  record dependency.

## 5. The adapter and the module contract

- [x] 5.1 Add the three methods to the `DialecticaModule` trait and forward each
  into `core`, holding the live slate nonce as a second field — verified: the
  field is an `Option`, so `Default` still supplies the one genuinely
  parameterless constructor `interface: "universal"` requires, and no
  constructor parameter was added.
- [x] 5.2 Verify each adapter body is a forward plus the lookups.

  **Two helpers were extracted rather than repeating four lines a fifth time**,
  which is CLAUDE.md's "fourth slightly-different guard" signal: `storage_dir`
  (the persistence-path lookup, previously duplicated in two handlers and heading
  for five) and `master_key` (open-or-mint). `protection_from_env` went into
  `keystore.rs` rather than the adapter, so the passphrase byte handling is
  decided once — a second copy would eventually map two different passphrases
  onto one key.

  The adapter is not reachable by `cargo test` at all (PLAN.md §2.3), so these
  are verified by reading rather than by a test, which is the reason the bodies
  are kept this thin.

## 6. Gates

- [x] 6.1 The full suite passes, with no warnings from either of this repo's crates
  — the warnings in the output are the SDK's own and predate this change. Run
  `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
  -p dialectica-core`; the tests this change adds are the ones listed against the
  tasks above.

  **This box used to record "546 passed, 0 failed (475 before this change)", and
  the count was wrong by the time anyone read it** — two commits landed after the
  tick, and review measured 553. CLAUDE.md names "how many tests pass" as the
  canonical thing not to write down, and the reason is exactly this: a total
  cannot fail loudly, it just goes quietly stale, and a reader cannot tell whether
  it was true when written. The self-invalidating form ties the claim to a diff
  someone can check instead of to a number nobody can.
- [x] 6.2 `cargo clippy -p dialectica -p dialectica-core --all-targets -- -D
  warnings` exits 0. Clippy caught three lints in the new tests
  (`bool_assert_comparison` twice, `unnecessary_to_owned` once), now fixed.

  **`cargo fmt --check` exits 0 and that measures nothing here.** It does not
  follow path dependencies, so it never reaches `dialectica-core` — where every
  file this change adds lives. Reporting it as a passed gate is the "exit 0 on a
  gate that measured nothing" that `.claude/agents/README.md` says is worse than
  no gate.

  Checked per file instead, with `rustfmt --check --edition 2021 --config
  skip_children=true`. Readability review found the new files carrying 20 hunks;
  they are now clean. `wire.rs` was formatted in its own commit ahead of the
  behaviour changes, because 8 of its hunks reproduce against `origin/main`'s copy
  and so are not this change's — mixing those into a behaviour diff makes it
  reviewable for neither. `keystore.rs`'s pre-existing hunks are deliberately
  untouched for the same reason. `identity.rs` picked up about five, because they
  sit inside test bodies this change edits.
- [x] 6.3 `openspec validate identity-onboarding --strict` — **passes**, after two
  defects the implementation pass found were fixed in the delta by `spec-writer`.
  Recorded rather than ticked silently, because the failures were real and the
  reason they are gone is a spec edit rather than a code one.

  1. `identity/spec.md`: the MODIFIED requirement had **renamed** the live spec's
     scenario "The same root and Stoa always yield the same identity" to "The
     same root, Stoa and path…" instead of keeping it alongside. A MODIFIED block
     replaces the whole requirement, so `openspec archive` would have **dropped**
     the two-input determinism scenario. Both scenarios are now present, and the
     requirement prose now states that the two-input derivation remains, so the
     retained scenario is in contract rather than orphaned. Confirmed by
     reverting the fix and watching `validate --strict` name the omission:
     *"MODIFIED … omits scenario(s) the current spec still has"*.
  2. `identity-onboarding/spec.md`: "A generated name and a mark are not settled
     by this capability" carried no `#### Scenario:` block. It now asserts the
     observable thing — that no reply of this capability carries a name or a mark
     field, while both still carry the public key and the address such values
     would be derived from.

     **Coverage of the new scenario was partial and is now complete.**
     `the_whoami_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
     asserts the whole serialised string, so a name field added to the whoami
     reply fails it. `the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
     checked each expected key was *present*, not that the key set was exactly
     those — so a `name` field added to a slate candidate left it green. That was
     recorded here as `tester`'s to add, and the spec-test reviewer measured it:
     `"displayName"` on every candidate, 553 passed, 0 failed.

     It is closed. That test now collects each candidate's keys and asserts the
     set exactly, so an **added** key fails as well as a removed one; re-applying
     the reviewer's mutation fails it and nothing else.

     Recorded rather than quietly corrected, because the review's point about this
     entry is the durable one: the honest account lived in `tasks.md`, which is
     deleted before merge, while the overstated claim lived in `slate_json`'s doc
     comment, which survives. An admission in a tracker is not a gate.

  Both behaviours were already covered in code —
  `a_derived_stoa_key_is_deterministic` and `a_path_derived_key_is_deterministic`
  both exist and pass — so nothing was untested; what was at risk was the
  contract losing a requirement on archive.

  **A third thing blocked the archive and no longer does.**
  `openspec archive identity-onboarding` aborted with *"Validation errors in
  rebuilt spec for identity … Spec must have a Purpose section"* and wrote
  nothing, because the live `openspec/specs/identity/spec.md` had no `## Purpose`
  — an artifact of the specs being hand-merged before the CLI was installed.

  The fix was on `docs/flow-tooling` and **has since landed on `main`** (it came in
  with the commit that gave three specs a Purpose). This branch was rebased onto
  `origin/main` during the review-fix pass, so it now carries that Purpose and the
  abort is gone. Nothing is duplicated here, because two Purposes for one spec is
  two answers.

  Kept rather than deleted because the trap is live for any change branched before
  that landed, and because the failure mode is the dangerous kind: `archive`
  aborts and **writes nothing**, so it looks like a no-op rather than an error
  someone must fix.

  With that Purpose temporarily in place, the archive applies cleanly and the
  rebuilt `identity` requirement carries **both** determinism scenarios, all
  three original scenarios and the three new ones — verified by running the real
  archive in a throwaway worktree, reading the merged file, then reverting
  everything the archive wrote.
- [x] 6.4 The `logos-rust-sdk-src` symlink is removed and absent from the commit
  — confirmed by `git status`.

## 7. Acting on the review findings

Six reviewers wrote `findings/`; this section records what the fix pass did to the
code. Each entry's evidence lives in the findings file it answers, and every finding
carries its own outcome there — **that** is the gate, not this list.

- [x] 7.1 **The identity kept is the one the slate showed.** Three reviewers found
  independently that `master_key` minted a fresh key per call, so on a fresh install
  the slate showed candidates of key A and the keep wrote key B. `OnboardingSession`
  in `core` now holds `(keystore, live_slate)` together and mints at most once —
  verified by `on_a_fresh_install_the_identity_kept_is_the_candidate_the_slate_showed`,
  measured failing first, and by restoring the per-call mint and watching it plus two
  others fail and nothing else.
- [x] 7.2 **A second Stoa can be kept.** `Keystore::create`'s `AlreadyExists` was
  serving as both the second-keep guard and the per-Stoa gate, and the keystore is one
  file per install — so `chosen_paths` could never hold a second row through any wire
  call, making a spec scenario unreachable through the API. The refusal moved to the
  primary key — verified by
  `keeping_an_identity_in_a_second_stoa_succeeds_and_reuses_the_master_key` and, in
  the other direction, by
  `a_second_keep_for_one_stoa_is_still_refused_after_the_second_stoa_fix`, which is
  there so the fix cannot have bought a reachable second Stoa at the price of a
  replaceable identity.
- [x] 7.3 **The probe and `whoAmI` report one identity.** The salt bump left
  `getCapabilities` on the pathless derivation. `posting_identity` in `core` now
  consults the record — verified by
  `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa`, whose
  strongest assertion is that an op signed at the recorded path verifies against the
  address the probe reported, and by
  `the_probe_and_whoami_give_one_reason_when_no_choice_is_recorded_for_this_stoa`.
- [x] 7.4 **The path mask and the path guard are one constant.** `path_from_row`
  bounded the whole of `u32` while `derive_path` masks below 2³¹, so a hand-edited row
  in between derived a working identity nobody chose. `onboarding::PATH_LIMIT` is now
  the single bound, applied on read **and** on write — three tests, each watched
  failing first, one of which pins the two rules to each other rather than each to a
  literal.
- [x] 7.5 **Minting a key cannot panic.** `SecretKey::generate`'s `expect` had become
  reachable from a dispatch handler while its comment said otherwise. Both halves
  fixed: the mint moved inside `guarded`, and the signature is fallible.
  **Deliberately not proven by a test that forces the failure** — `getrandom` cannot be
  made to fail without a seccomp policy, and a test installing one would be testing
  the sandbox. `minting_a_key_is_fallible_rather_than_a_panic` pins the shape instead
  and says so.
- [x] 7.6 **Documents corrected against the code.** `design.md`'s atomicity concession
  claimed a partial state was repairable by the user's next choice, which 7.2 shows
  could never be made; its "nothing was written anywhere" ignored the
  `identity.sqlite` created first; its `check_layout` risk entry described a guard the
  code has. Line-number citations replaced with symbol chains, because the ones it
  carried were pre-change and three reviewers found them stale independently.
- [x] 7.7 **Six comments and one helper that had outlived the code**, each named in
  `findings/readability.md`: `parse_stoa`'s three unconverted copies, `parse_index`'s
  caller count and its `as usize`, the slate reply's presence-only pin, the zeroize
  comment's false claim, `OnboardingDir`'s wrong precedent, `who_am_i`'s off-by-one.
- [x] 7.8 **`tasks.md` has no stage block, and I have not added one.**
  `.claude/agents/README.md` specifies one at the top of this file, one row per agent
  instance, each agent ticking only its own row — and it is the mechanism by which "an
  unticked row with no agent running is a stage nobody is doing" is checkable. This
  file starts at section 1.

  Left for `spec-writer`, which owns that block, rather than written by me: my own
  agent file says to tick exactly one row and **never add a row**, because concurrent
  agents' cherry-picks must not touch the same lines. Authoring the whole block would
  be the same hazard at larger scale, and I would also be guessing at how many
  reviewer rows to create for reviewers who have already run.

  Unticked because it is a real gap in this change's tracking, not a note.

  **Written, `spec-writer`.** The block is at the top of this file. `dev-writer` was
  right on both counts — the block is `spec-writer`'s and the no-added-rows rule is
  about not authoring other agents' lines while they run — and the second reason it
  gave for declining, "guessing at how many reviewer rows to create for reviewers who
  have already run", turned out not to be a guess: there is one `findings/` file per
  reviewer, so the row count is read off the directory. Six review rows, one per
  dimension. Each tick names a file rather than a recollection, and the block says
  plainly that it was written after the fact so nobody reads it as live tracking.

## What is NOT in this change, and where it went

- **`check_layout` does not prove the `PRIMARY KEY`** (`findings/security.md` S4),
  so a replaced `identity.sqlite` with a constraint-less table opens and
  `path_for` returns whichever row SQLite hands back first. A schema-verification
  design rather than a guard, and it should decide about `all_paths` returning two
  rows for one Stoa at the same time. Recorded in `design.md`'s Risks; that
  finding's box is left **open**.
- **`derive_stoa_key_at_path` leaves a derived seed on the stack unwiped**
  (`findings/security.md` S5's substantive half). Fixing it means giving
  `identity.rs` a memory-lifetime obligation it currently defers to `keystore`,
  which is a change to that module's contract. Recorded in `design.md`'s Risks;
  the comment that falsely claimed the obligation was discharged **is** fixed.
- **The pathless `Keystore` trio now has no production caller.** Retiring it would
  delete three public methods from the secret-holding type, in a change whose
  proposal declares `keystore` untouched. Recorded in `design.md`'s Risks.
- ~~**The spec gaps** the findings route to `spec-writer`: no scenario requires that
  the kept identity be the candidate the slate *displayed* across two calls; no
  requirement states the recorded path's admissible range; no requirement names
  `path` as a reply field; and `proposal.md` still says `posting-capability`'s
  derivation is untouched, which 7.3 makes false.~~

  **All four closed by `spec-writer`**, and this list was the most useful handover in
  the file because it named them in one place. The spec now carries: a scenario
  requiring the identity kept to be the candidate displayed at the selected position,
  for every position; a requirement bounding the recorded path to what this module's
  derivation could have produced, with the bound required to be the same on the write
  and the read side; a requirement carrying `path` in all three identity-naming
  replies, plus one closing each reply's field set so the contract and the pinning test
  say the same thing. `proposal.md` no longer claims `posting-capability`'s derivation
  is untouched — it says which key the probe reports changed, and why that needs no
  delta: the requirement governing it was already right and the code was violating it.
  Outcomes are in `findings/spec-test.md`.
