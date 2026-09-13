# identity-onboarding — correctness review

Dimension: **correctness only.** Security, readability and architecture are held
by other reviewers and are not covered here.

Baseline measured before any mutation: **553 passing**
(`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`).
Every mutation below was applied in an isolated worktree and reverted; the tree
was confirmed clean with `git status` before committing.

---

- [x] **1. On a fresh install, the identity kept is not the candidate the user chose**

**For:** `dev-writer` (and `spec-writer`, for the scenario that would have caught it)
**Severity:** high — silently stores an identity the user did not choose, which
the spec itself calls unrecoverable
**Where:** `dialectica/rust-lib/src/lib.rs:306-317` (`Dialectica::master_key`),
reached from `:404-424` (`generate_identity_slate`) and `:426-460`
(`keep_identity`)

`master_key` mints a **fresh random keystore** whenever no keystore file exists:

```rust
Err(core::keystore::KeystoreError::NotFound) => {
    Ok(core::keystore::Keystore::generate())
}
```

`Keystore::generate` is genuinely random (`keystore.rs:665-672` →
`SecretKey::generate` → `getrandom::fill`). Both handlers call `master_key`
independently, so on a fresh install:

1. `generateIdentitySlate` mints master key **A**, derives five candidates from
   A, returns their addresses and public keys, remembers the nonce.
2. The user looks at A's five addresses and picks index 2.
3. `keepIdentity` calls `master_key` again, which mints master key **B**,
   reproduces the slate from the same nonce but **against B**, and writes B to
   disk together with path P.

The reply says `kept:true` and reports B-at-P's address. The user chose
A-at-P's. The nonce check does not catch this — both slates are equally live,
and the nonce is the same. The `live_slate` field only pins *which paths* were
offered, never which master key they were offered under.

`lib.rs:294-301` argues this is harmless because "the identity kept is always
one of the candidates of the key that was stored — never a candidate of a key
that was discarded." That is true and beside the point: the spec's requirement
is about the candidate the *user selected*, and the spec says an address "is the
only unforgeable way to tell two candidates apart"
(`specs/identity-onboarding/spec.md:98-99`). The address the user was shown is
not the address they get.

### Reproduction

Added temporarily to `wire.rs`'s test module, modelling the adapter's two calls
with two distinct fixed roots (`[7u8;32]` for the slate, `[8u8;32]` for the
keep) instead of two random ones, so the outcome is deterministic:

```
thread 'wire::tests::TEMP_fresh_install_keeps_a_candidate_the_user_never_saw'
panicked at dialectica-core/src/wire.rs:3135:
assertion `left == right` failed:
  the kept identity is NOT the candidate the slate showed at index 2
  left:  "a66be1e3495d3849fe409d42a31932abd11c0bdcfb588637f5e042a8de945244"  (kept)
  right: "9253330f6d836f2cad3937af14344619165d2f0175a0954b2745256cd9c97311"  (offered)
```

### Why no existing test sees it

This is exactly this project's recurring defect family, one layer up from where
the tester found it. `wire.rs`'s helpers `slate_through_the_wire` (`:1659`) and
`keep_through_the_wire` (`:1909`) each call `a_master_key()` (`:1649`), which is
`Keystore::from_root_for_test([7u8; 32])` — **a fixed root**. So slate and keep
agree because both used the same key, and the reply/record/expectation all agree
with each other while none of them is the key the adapter would actually have
used. `a_kept_identity_is_the_candidate_the_slate_offered_at_that_index`
(`:2970`) is the test written precisely to catch a wrong-index keep, and it
cannot see this because it holds the master key constant across both calls.

The adapter is `#[cfg(logos_scaffold)]`, so `cargo test` cannot reach it at all
— which is why the whole two-call key-identity question is untested by
construction.

### Note for the spec

`spec.md` has no scenario asserting that the kept identity is the candidate the
slate *displayed*, across two calls, with the master key not held fixed. Every
keep scenario is satisfiable by a slate and a keep that agree with each other.
A scenario of the form "**WHEN** a slate is generated on a device with no
existing master key, and a candidate from it is then kept / **THEN** the
identity kept is the one the slate displayed at that index" would be failed by
the current code.

**Fixed** in `63133c9`. The diagnosis is exactly right, including that the
`live_slate` field "only pins *which paths* were offered, never which master key
they were offered under" — that sentence is the fix, stated as a defect.

`core::wire::OnboardingSession` now holds `(keystore, live_slate)` together and
mints at most once per module lifetime; the mint-or-open decision moved out of the
adapter into `core`. Carrying a *commitment* to the key in the slate reply instead
was considered and rejected in `design.md`: it detects the divergence without
giving the user the identity they chose, because by then the key that produced
their slate is gone.

The regression test is
`on_a_fresh_install_the_identity_kept_is_the_candidate_the_slate_showed`, and it is
built around this finding's "why no existing test sees it" section. It supplies
**no** master key — its opener returns `NotFound`, so minting actually happens —
and asserts a relationship between the two replies rather than a comparison
against a constant, so it cannot be satisfied by a fixture handing the same value
to both sides. Measured failing before the fix with the same signature the finding
reports: identical path, different addresses, `kept: true`.

Restoring the per-call mint fails that test plus
`refreshing_a_slate_offers_candidates_of_one_master_key` and
`keeping_an_identity_in_a_second_stoa_succeeds_and_reuses_the_master_key`, **and
nothing else in 562** — which independently confirms this finding's claim that the
whole question was untested by construction.

Three consequences of the same seam, found by other reviewers, are fixed in the
same commit: the second-Stoa dead end, the unrecoverable partial state, and the
probe/`whoAmI` disagreement. See `design.md`.

**For `spec-writer`:** the "Note for the spec" section is a live gap and is **not**
closed by this fix. The spec still has no scenario asserting that the kept identity
is the candidate the slate *displayed* across two calls with the master key not
held fixed; every keep scenario remains satisfiable by a slate and a keep that
agree with each other. The scenario this finding drafts is the right one, and a
test for it now exists — but the contract does not require it, which is the thing
that let the defect through.

---

- [x] **2. `SecretKey::generate`'s `expect` is reachable from a dispatch handler, outside the panic guard, and its doc comment says it is not**

**For:** `dev-writer`
**Severity:** medium — a reachable abort on the two most-called onboarding
paths; also a now-false claim in a doc comment that exists to prevent exactly
this
**Where:** panic at
`dialectica/rust-lib/dialectica-core/src/identity.rs:301`; made reachable by
`dialectica/rust-lib/src/lib.rs:312-314`

```rust
// identity.rs:301
getrandom::fill(&mut seed).expect("the OS random source must be available to mint a key");
```

Its doc comment, `identity.rs:296-298`, states:

> It is also **not reachable from a dispatch handler** — key generation happens
> at keystore setup, not while serving an inbound op.

This change makes that false. On a fresh install (no keystore file), **every**
`generateIdentitySlate` and `keepIdentity` call goes through
`master_key`'s `Err(NotFound) => Ok(Keystore::generate())` arm at `lib.rs:313`
and reaches that `expect`.

Two things make it worse than the usual `expect`:

- **It is outside `guarded`.** The panic happens in the adapter, at
  `lib.rs:313`, *before* `core::generate_identity_slate` is called — so
  `catch_unwind` at `wire.rs:51` never sees it. PHASE0-FINDINGS §3's measured
  consequence applies in full: the module process aborts, the caller waits out
  its 20s timeout, and every later call reports `MODULE_NOT_LOADED`.
- **The same mitigation was already applied next door and missed here.**
  `SlateNonce::generate` (`onboarding.rs:100-104`) returns a `Result` rather
  than `expect`, and its doc comment cites this exact contrast: *"generating a
  slate is something a dispatch handler does on request, and a panic there
  aborts the module process. `SecretKey::generate` runs at keystore setup, which
  is why it may `expect`."* That reasoning was correct when written and is no
  longer true of the master key.

Triggering the panic needs `getrandom::fill` to fail — on Linux, `getrandom(2)`
denied by a seccomp/container policy, or called before the pool is initialised.
Low probability; it is the reachability that is the defect, and it is the class
of thing this file's own comments are written to keep off handler paths.

**Fixed** in the commit that follows this file's update. Both halves of the finding
are addressed, and they needed different fixes:

- **Outside the guard.** Closed as a side effect of the defect-1 fix: the mint moved
  from the adapter into `core::wire::OnboardingSession::keystore_for`, which is
  called inside both `guarded("generate_identity_slate", …)` and
  `guarded("keep_identity", …)`. A panic there would now be caught and converted
  rather than aborting the process.
- **The `expect` itself.** `SecretKey::generate` now returns
  `Result<Self, RandomnessUnavailable>` and `Keystore::generate` propagates it. This
  is the half that matters structurally: catching a panic is a backstop, while a
  `Result` means reverting is a compile error at `Keystore::generate` rather than a
  silent change of failure mode.

The finding's strongest observation is the one that decided the shape: *"the same
mitigation was already applied next door and missed here"* — `SlateNonce::generate`
took the fallible shape and its comment cited the contrast with this function. So
rather than update that comment to preserve an asymmetry that had stopped being
true, the asymmetry is gone.

`RandomnessUnavailable` is a new type in `identity.rs` and
`KeystoreError::NoRandomness` a new arm, rather than reusing `KeyError` or
`KeystoreError::Io` — `Io`'s message is "keystore could not be read", which names
the wrong operation for a mint and would send a reader to check a file that does not
exist yet.

**What no test can show, and is not claimed.** `getrandom` cannot be made to fail
from a test without installing a seccomp policy, and a test that installed one would
be testing the sandbox. `minting_a_key_is_fallible_rather_than_a_panic` pins the
*shape* — the fallible signature, the message naming a fix, the conversion into
`KeystoreError` — which is what would have to be undone to reintroduce the panic.
The failure itself is unobservable from the suite and the test says so.

**A second gap this opened and closed:** `every_error_message_names_a_fix` lists
`KeystoreError`'s variants by hand and, unlike its sibling
`every_reason_is_distinguishable_from_every_other`, had no count guard — so
`NoRandomness` went in silently uncovered. It now carries the same hardcoded
`assert_eq!(all.len(), 18)`.

---

- [x] **3. The slate's distinctness walk can be deleted with the whole suite green**

**For:** `tester`
**Severity:** low — a documented-as-unreachable guard, reported as a measured
coverage gap rather than as a defect
**Where:** `dialectica/rust-lib/dialectica-core/src/onboarding.rs:256-259`

Mutation applied — the `paths.contains` skip removed, so a colliding path is
kept twice:

```rust
let path = derive_path(&nonce, step);
paths.push(path);            // `if paths.contains(&path) { continue; }` deleted
```

**Result: 553 passed, 0 failed.** Nothing observes it.

This is expected and I am not calling it a defect: a collision is a ~2⁻³¹
per-pair event, `MAX_PATH_WALK` is documented at `:67-78` as "a bound, not an
expectation," and `every_candidate_in_a_slate_is_distinct` (`:467`) cannot
manufacture one. Recorded because the walk is the mechanism behind the spec's
"no two candidates share a public key," and a reader should know that guard is
carried by argument rather than by a gate. Testing it would need
`derive_path` injectable, which is a larger change than the guard is worth —
worth a note in `design.md` rather than a test.

**Open — `tester`'s, and the reviewer's own recommendation is not to test it.**
`dev-writer` is leaving this box empty rather than ticking it, because the finding
asks for a `design.md` note and I do not want to record "noted" as though the
coverage gap were closed. The measurement stands: the `paths.contains` skip can be
deleted with 563 green.

The reviewer's judgement that this is not a defect is right and I am not arguing
with it. What is worth deciding rather than inheriting is whether the note belongs
in `design.md` — `tester` owns whether any test is written here, and the existing
`design.md` entry on `from_nonce`'s walk already argues the reproducibility
constraint that makes `derive_path` non-injectable. If `tester` agrees no test is
warranted, this box should be ticked as **rejected, with the argument** rather than
fixed.

**`tester`: the coverage gap is REJECTED as untestable, and a DIFFERENT hole in the
same five lines is FIXED. Both were measured on the post-merge tree.**

**First, the finding reproduces.** Baseline after the two `origin/main` merges is
**655**, not 563 — the merges brought in the request envelope and
`authoring-content`. With the `paths.contains` skip deleted:

| Mutation at `onboarding.rs:275-279` | Predicted | Observed |
|---|---|---|
| `if paths.contains(&path) { continue; }` deleted | 655 pass, 0 fail | **655 pass, 0 fail** |

**Why it is inert, which the brief asked be established before any replacement was
written.** Not "guaranteed elsewhere" and not "unreachable" — the third option:
**reachable in principle, not manufacturable in a test.** `SlateNonce`'s field is
private but this module's own tests construct one directly (`a_nonce()` does), so a
colliding nonce *would* work as a fixture. There simply is not one to be had: a
collision needs two of indices 0..4 to agree on 31 masked bits, ~10 pairs at 2⁻³¹,
so ~2.7×10⁸ nonces to expect one. At ~10⁶ nonces/sec that is minutes *optimised* and
hours in a debug test binary — a one-off search whose answer could be hardcoded, but
not a test, and not worth the compute for a guard the reviewer already agrees is not
a defect.

The spec settles it. *"**WHEN** a slate is requested, **THEN** no two candidates in
it share a public key"* scopes the requirement to slates **as actually requested**,
which `every_candidate_in_a_slate_is_distinct` does test faithfully. The mechanism
behind it is not what the scenario names. Writing a test that manufactured a
collision would be testing a state the spec does not describe, and per `tester.md`
the honest move is to report rather than weaken. **Rejected, no test.**

**Second — and this is the part worth the reviewer's time — probing that walk turned
up a hole in it that IS reachable, and it was open.** Nothing connected a slate's
paths to the indices they come from:

| Mutation at `onboarding.rs:271` | Predicted | Observed |
|---|---|---|
| `for step in 0..MAX_PATH_WALK` → `1..MAX_PATH_WALK` | 655 pass, 0 fail | **655 pass, 0 fail** |

An off-by-one in the walk offers five candidates that are still perfectly distinct
and still derived by the correct function, so every existing assertion in the file
held — `the_slate_constants_are_pinned_to_known_answers` pins `derive_path` but says
nothing about which indices a *slate* uses, and
`every_candidate_in_a_slate_is_distinct` pins that the five differ from each other.
Two tests either side of the property, and the property itself unheld. That is this
project's defect family again: two explanations — "the walk starts at 0" and "the
walk starts anywhere and the candidates are distinct" — producing the same answer.

It matters in the silent direction the pinning test exists for: the walk's index is
the entire input to a candidate's path, so an off-by-one changes which identity every
user of the build is offered and keeps, with no error anywhere.

**Fixed** by `onboarding.rs::a_slates_paths_are_the_derivation_at_indices_zero_to_four`.
Expectations are the five OpenSSL digests masked **by hand**, not read back from
`derive_path` — indices 0 and 1 reproduce the two values the existing pinning test
already carries, which is the cross-check that the method is independent. Index 4 is
the one digest whose top bit is already clear (`7b51f060`), so it also pins that the
mask leaves an in-range value alone.

| Mutation | Predicted | Observed |
|---|---|---|
| `for step in 1..MAX_PATH_WALK` | the vector shifts one place: index 0's 52,135,370 drops off the front | **exactly that** — `left: [1652162698, 1112410086, 378410041, 2068967520, 899711034]` against `right: [52135370, 1652162698, 1112410086, 378410041, 2068967520]` |

Suite **656 passed, 0 failed** with the test in and the tree restored.

---

## Verified sound — the write-order atomicity

The brief asked me to check both directions of the keystore/path-record write
order. **Both are genuinely pinned, and the newer test fails for the reason it
names.** Measured:

| Mutation at `wire.rs:530-542` | Result |
|---|---|
| Writes **reversed** (record first, keystore second) | **550 pass, 3 fail** — `a_keep_whose_keystore_write_fails_records_no_path`, `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`, `each_keep_refusal_reason_is_pinned_to_its_own_situation` |
| Record write error **discarded**, `kept:true` reported (`let _ = targets.paths.record_path(...)`) | **551 pass, 2 fail** — `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`, `each_keep_refusal_reason_is_pinned_to_its_own_situation` |

The second row is the direction `design.md:181-210` argues about — keystore
written, path record not — and it is caught. The fixture is honest about
reaching that state: `wire.rs:2733-2737` asserts `dir.keystore_path().exists()`
*before* asserting `kept:false`, so a fixture that failed earlier (at `open`
rather than at `record_path`) fails the test rather than passing it vacuously.
Dropping the table via a second live connection is the right mechanism, and the
comment at `:2694-2697` correctly explains why a directory-in-the-way fixture
would exercise a different arm. `each_keep_refusal_reason_is_pinned_to_its_own_situation`
reaches the same state through the handler and pins the reason substring, which
is a second independent observation of it.

## Verified sound — everything else read

- **Decoder strictness.** `SlateNonce::from_hex` (`onboarding.rs:118-125`) and
  `Address::from_hex` refuse non-hex, short, empty and 33-byte input by exact
  length via `try_into`, with the 33-byte case pinned (`:656-659`) — the case a
  `>=` check waves through.
- **No reachable panic in `identity_store.rs`.** No `unwrap`/`expect`/indexing
  outside tests; the claim at `:39-41` holds. On-disk content is validated, not
  trusted: blob length at `:394-400` → `StoaNotAnAddress`, integer range at
  `:423-428` → `PathOutOfRange`, both refusing rather than coercing. Truncated
  and arbitrary file content are swept at `:643` and `:675`, and both sweeps go
  on to call `path_for` and `all_paths` on a store that opened — the empty-file
  case, where SQLite accepts a zero-length file, is what makes that
  load-bearing.
- **HKDF `expect`s are sound.** `identity.rs:377` and `:431` request 32 bytes
  against HKDF-SHA512's 255×64 = 16,320-byte cap. Not a finding.
- **Fixed-size slicing is compile-time in-bounds.** `onboarding.rs:166`
  (`digest[..4]` on a `[u8; 32]`), `identity.rs:427-428` (`info[..32]` /
  `info[32..]` on a `[u8; 36]` from `[u8; 32]` and `[u8; 4]`).
- **Off-by-one in the slate.** `Candidate::index` is assigned from
  `candidates.len()` *before* the push (`onboarding.rs:280`), so it always
  equals the position; pinned at `:482-484` and `wire.rs:1716-1718`. The
  post-loop `candidates.len() != SLATE_SIZE` check (`:287-292`) makes a short
  walk an error rather than a truncated set. `candidate()` (`:302-309`) uses
  `.get()` with no clamp or modulo, and `:613` asserts both that out-of-range is
  refused *and* that every in-range index is satisfied — so the refusal cannot
  be unconditional.
- **`Display` underflow.** `of.saturating_sub(1)` at `:388` with the `of: 0`
  case pinned at `:808-810`. A panic in a `Display` impl would abort the module
  process like any other.
- **`whoami_for` reads the record.** `distinct_stoas_report_distinct_identities`
  (`wire.rs:2462`) and
  `a_record_restored_beside_a_master_key_names_the_identities_in_use` (`:3007`)
  both derive expectations independently from `[7u8;32]` and the path, and pin
  two different paths — so a handler hardcoding a path, or ignoring the record,
  is caught.
- **CI's source-layout gates.** `.github/workflows/ci.yml:684-696` lists
  `dialectica/rust-lib/src` and `dialectica/rust-lib/dialectica-core/src`
  explicitly and fails loudly if either is missing. This change moves and
  renames no file, both roots exist, and the `#[test]` count matches the 553
  reported. The gate is intact.

- [x] **One note that is not a finding today**

`parse_index` at `wire.rs:918-933` ends `Ok(Some(v as usize))`, where `v` is a
`u64`. On a 32-bit target that cast **truncates**, so `{"index": 4294967296}`
would become `0` and keep candidate 0 — the "stores an identity the user did not
choose" outcome. Not reachable: CI builds no 32-bit target. Mentioned only so a
future 32-bit port does not have to rediscover it; a `usize::try_from` would
close it.

**Fixed** anyway, in the same commit, and ticked because it was actioned rather
than because it was a defect. `usize::try_from` with an explicit refusal, exactly as
suggested. Security review raised the same thing independently as S6.

It costs nothing and the reason to take it now rather than leave the note is that
the property stops depending on which target the code is built for — the refusal is
a decision in the source rather than a consequence of `usize` happening to be 64
bits. The refusal itself is unreachable on any target CI builds, so **no test
exercises it**, and I am not claiming one does.

---

## Tree state

Three mutations and one temporary test were applied during this review, each
reverted with `git checkout --` immediately after measurement.
`git status --short` is empty; the only file this review adds is this one.
