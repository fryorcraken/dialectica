# `identity-onboarding` — architecture review

**Dimension reviewed: ARCHITECTURE ONLY.** Correctness, security and readability
are other reviewers'. Where a finding below has a correctness or security shadow,
it is noted and left to them — what is claimed here is only that the *shape* is
wrong.

Judged against `CLAUDE.md`'s own commitments: the forced core/UI split, "the core
API is the deliverable", "complexity in the data structure, not the logic", "make
the change easy then make the easy change", and "one function, one job".

**Verification posture.** Every claim below was run or read, not inferred. The
derivation chain `design.md` asserts was walked by hand; the address divergence in
A1 was reproduced with a throwaway test that was then reverted. Baseline suite
**553 passed** before and after. `cargo fmt --check` and CI's `clippy -D warnings`
(scoped `-p dialectica -p dialectica-core`) both clean, so **CI would pass** — which
is part of the finding in A1, not a reassurance.

---

## A1 — `getCapabilities` and `whoAmI` report two different author addresses for the same user in the same Stoa

**For: `dev-writer`** (and `spec-writer` for the `posting-capability` consequence)
**Severity: high — genuine defect, the most serious in the change**
`dialectica/rust-lib/src/lib.rs:382` vs `dialectica/rust-lib/dialectica-core/src/wire.rs:700`

`getCapabilities` derives the reported identity through the **pathless** chain:

```
lib.rs:382   open_from_env(&path).map(|ks| ks.stoa_address(stoa).to_hex())
             -> keystore.rs:712 stoa_address -> :706 -> :701 -> :702
                crate::identity::derive_stoa_key(&self.root, stoa)   // salt v1
```

`whoAmI` derives it through the **path-taking** chain under the bumped salt:

```
wire.rs:700  keystore.stoa_public_key_at_path(stoa, path)
             -> keystore.rs:733 -> :728
                crate::identity::derive_stoa_key_at_path(&self.root, stoa, path)  // salt v2
```

`STOA_KEY_SALT` is `/dialectica/1/Identity/Stoa` and `STOA_KEY_SALT_WITH_PATH` is
`/dialectica/2/Identity/Stoa`, and `identity.rs`'s own new test
`the_path_taking_scheme_does_not_collide_with_the_scheme_without_one` asserts the
two schemes **must** disagree. They do. So two shipped wire methods answer "who
posts here" with two different addresses.

**Concrete illustration, reproduced and then reverted.** A temporary test in
`wire.rs` kept candidate 0 of a live slate against a fixed root `[7u8; 32]` and
`a_stoa()`, then compared three values:

```
kept (Kept::Stored.address)          90eadabd92d0c4ec81669b002e1070e69df7c2d25ad507d0012681cad5b38c53
whoAmI (Whoami::Identity.address)    90eadabd92d0c4ec81669b002e1070e69df7c2d25ad507d0012681cad5b38c53   -- agrees
getCapabilities (ks.stoa_address)    656c6003a040b37bda69ac1c34d35de81d6c193e8f571dedffb8845d2355e153   -- DISAGREES
```

`kept == whoAmI` passed; the `getCapabilities` assertion failed. The user keeps
`90eadabd…`, the posting probe tells the view they post as `656c6003…`.

**Why this is architectural and not merely a bug.** Three commitments break at once:

1. **`CLAUDE.md`: "JSON shapes are source-independent, so a view renders without
   branching on where the data came from."** A view now holds two author addresses
   from two methods and has no way to decide which one signs. There is no field in
   either reply that distinguishes them.
2. **The merged `posting-capability` spec, requirement "The identity reported is
   the one that would sign"** (`openspec/specs/posting-capability/spec.md:99-108`):
   *"the identity reported SHALL be the one an op published now would be
   attributed to, derived from the key that would actually sign it"*, with the
   rationale *"the user sees one handle and posts under another."* That is now
   literally the state. `proposal.md:59-60` declares `posting-capability`
   **"unchanged. The probe's shape, its reasons and its derivation are
   untouched"** — the shape is untouched and the *contract* is broken, which is the
   distinction the proposal's own "Not modified, deliberately" section exists to
   make and got backwards.
3. **The change's own MODIFIED `identity` requirement**
   (`openspec/changes/identity-onboarding/specs/identity/spec.md:27-29`): *"A user
   SHALL have exactly one identity within a Stoa, and that identity SHALL be
   reproducible from the user's root secret, the Stoa's address, and the derivation
   path recorded for that Stoa."* After this change there is exactly one identity
   per Stoa and it is the path-derived one, so `getCapabilities` reports an
   identity the contract says the user does not have.

**The measurement.** Zero tests fail. `getCapabilities`' own tests supply the
address through an injected `lookup` closure (`wire.rs:3512` passes
`|_| Ok("abcd".to_string())`), so the suite never exercises the real derivation
behind the probe, and the only place the two chains meet is the adapter — which is
`#[cfg(logos_scaffold)]` and therefore compiled out of `cargo test` entirely (see
A2). The divergence is invisible to every gate this project runs.

**What the shape should have been.** This is the "two derivation functions coexist
— seam or duplication?" question the review brief posed. It is duplication, not a
seam: `keystore.rs:712`'s `stoa_address` doc comment still says it is *"what the
probe reports, and what an op published now is attributed to"*, and that sentence
is now false. Either the probe reads the path record and uses the path-taking trio
(which makes `getCapabilities` depend on `IdentityStore`, a real API decision worth
taking on purpose), or the pathless trio is retired from the live path and kept
only as the primitive the non-collision test needs. What cannot stand is two
public trios each claiming to be "the" per-Stoa identity.

---

## A2 — `master_key()` mints a throwaway root, so on a fresh install the kept identity is **not** any candidate the user was shown; the logic sits where no gate can reach it

**For: `dev-writer`**
**Severity: high — genuine defect**
`dialectica/rust-lib/src/lib.rs:306-317`, called at `:417` and `:434`

```rust
fn master_key(dir: &Path) -> Result<Keystore, KeystoreError> {
    match core::keystore::open_from_env(&path) {
        Ok(ks) => Ok(ks),
        Err(KeystoreError::NotFound) => Ok(core::keystore::Keystore::generate()),  // fresh random root
        Err(e) => Err(e),
    }
}
```

`Keystore::generate()` is a fresh random root every call. On a fresh install —
the only install onboarding exists for — `generateIdentitySlate` calls
`master_key()` and gets root **R1**, derives and displays five candidates of R1,
and discards R1 when the handler returns. `keepIdentity` then calls `master_key()`
again, gets an unrelated root **R2**, and `keep_selection` (`wire.rs:516`)
recomputes the slate from the *same nonce* against **R2** — producing five
different candidates — and stores candidate `index` of that set.

**Concrete failure scenario.** Fresh install, no keystore. User calls
`generateIdentitySlate`, is shown five addresses, picks the third because they like
its generated name, and calls `keepIdentity` with `{"index":2,"slate":"<nonce>"}`.
The reply is `{"kept":true,"address":"<an address the user has never seen>",…}`.
No error anywhere. The nonce matched, the index was in range, both writes
succeeded. The user chose from one set and was given a member of another.

**Why the code's own defence does not hold.** `lib.rs:296-301` states the
consequence and then argues it away:

> *"What makes it harmless is that the keep mints its own key too and writes THAT
> one, so the identity kept is always one of the candidates of the key that was
> stored — never a candidate of a key that was discarded."*

That sentence is true and irrelevant. It establishes internal consistency between
`Kept::Stored` and what is on disk; it does not establish that the kept identity is
one the **user selected**, which is the thing the spec protects. The spec's
requirement "Every entry point refuses malformed input rather than guessing"
(`spec.md:368-369`) says a selection *"SHALL NOT be satisfied by any other
candidate"*, and the reason `onboarding.rs:298-301` gives for refusing rather than
coercing is exactly this: coercing *"would store an identity the user did not
choose — which is unrecoverable, because the choice cannot be recomputed."* The
ephemeral-key path does by construction what the index guard was written to
prevent.

**The architectural cause, and it is the finding.** `lib.rs:320-323` states this
file's own governing rule, unchanged by this diff:

> *"A thin adapter and nothing more. Every method forwards straight into `core`,
> which is where the guard and the decisions live. **If a body here ever grows past
> one line, that logic belongs in `core` — otherwise it is logic no test can
> reach.**"*

The change adds a 12-line decision function (`master_key`) plus three
multi-statement handlers to that file, and `master_key` is where the defect lives.
It is not merely untested — it is **structurally untestable**: `struct Dialectica`
and its two `impl` blocks are `#[cfg(logos_scaffold)]` (`lib.rs:259`, `:265`,
`:325`), and
`build.rs:40-42` sets that cfg only when `generated/provider_gen.rs` exists, which
it never does under `cargo test`. `build.rs:14-17` says so outright and forbids
stubbing it. So the one function that decides which master key a slate and a keep
each see is compiled out of the only gate that executes any logic.

**The measurement.** Every core-side onboarding test supplies a **fixed** root via
`Keystore::from_root_for_test([7u8; 32])` (`wire.rs:1649-1651`), used at 24 call
sites. A fixture that hands both calls the same key cannot distinguish "the slate
and the keep agree on the master key" from "they were given the same one by the
harness" — this project's recorded defect family, two explanations for one answer.
CI's test-count gate (`.github/workflows/ci.yml:644-711`) counts `#[test]`
attributes and so is blind to a whole `impl` being cfg'd out.

**What the shape should have been.** The mint-or-open decision is a `core`
decision: it takes a directory and returns a keystore, has no SDK type in its
signature, and is exactly what `core` exists to hold. Moved there it is testable,
and the fix then becomes available — carry the root that produced a slate alongside
the nonce (the module already holds 32 bytes across calls; holding the un-written
keystore is the same span), or write the minted keystore at slate time and make
`keepIdentity` open rather than mint. Either is a `core` decision with a test. Note
that fixing it in the adapter would leave it equally unreachable.

---

## A3 — `parse_stoa` was extracted for the three new handlers and three pre-existing copies were left behind; the fourth-copy signal was answered halfway

**For: `dev-writer`**
**Severity: medium — genuine defect of shape, no behavioural divergence today**
`dialectica/rust-lib/dialectica-core/src/wire.rs:257-264`

The new helper's own doc comment names the hazard:

> *"One job, because three handlers below need it and a fourth copy would
> eventually disagree with the first three about whether a missing field and a
> wrong-typed one are the same mistake."*

The three new handlers use it (`:307`, `:460`, `:633`). The three **pre-existing**
inline copies were not converted:

| copy | lines |
|---|---|
| `get_capabilities` | `:210-217` |
| `list_threads_inner` | `:811-818` |
| `list_threads_from_request` | `:889-896` |

So the tree now holds one helper and three hand-rolled copies of it — six stoa
parses where the comment argues for one. I diffed the three against the helper:
they are behaviourally identical **today**, which is precisely why this is a shape
finding rather than a correctness one. The cost arrives on the next change that
tightens the parse (a length bound, a lowercase-hex rule): it lands in one place
and three handlers keep the old behaviour, with no test failing, because each copy
has its own passing tests.

**`CLAUDE.md`, "put the complexity in the data structure":** *"When you find
yourself writing the fourth slightly-different copy of a guard, that is the signal
to reshape rather than to add a fourth test."* The signal was read correctly — the
helper exists — and then the reshape stopped at the new call sites.

**The ordering this should have had.** `CLAUDE.md`'s "make the change easy, then
make the easy change": converting the three existing handlers to `parse_stoa` is a
no-behaviour-change refactor that leaves every gate green on its own, and it is the
commit that should have preceded the feature. Done first, the feature diff adds
three handlers that each use an existing helper and introduces no new parse at all.

---

## A4 — `storage_dir()` has the same half-done extraction, in the adapter

**For: `dev-writer`**
**Severity: low — genuine defect of shape**
`dialectica/rust-lib/src/lib.rs:275-284`

`storage_dir()` was extracted with the comment *"Factored out at the point
CLAUDE.md names: this was the same four lines in two handlers and would have been
in five."* The three new handlers use it (`:405`, `:427`, `:463`). The two
pre-existing ones were left on the old inline form: `get_capabilities` at
`:374-379` and `list_threads` at `:387-392`, each still spelling the same `let
Some(dir) = … else { return core::error_json("the host has not yet told this
module where its storage is; try again once the module is ready") }`.

The error string is now duplicated in three places (`:277-279`, `:376-378`,
`:389-391`). Same family as A3 and the same fix: the conversion is a
no-behaviour-change refactor that belonged in its own commit before the feature.
Lower severity than A3 only because the string is a constant and a divergence would
be cosmetic rather than semantic — but it is also the same file A2 shows no test can
reach, so a divergence here is a divergence nothing would catch.

---

## A5 — `identity_store.rs` is a second copy of `log/sqlite.rs`'s store machinery, and `design.md` claims it is not

**For: `dev-writer`**, with a note for **`spec-writer`** on the design record
**Severity: medium — genuine defect of shape**
`dialectica/rust-lib/dialectica-core/src/identity_store.rs:200-299` against
`dialectica/rust-lib/dialectica-core/src/log/sqlite.rs:257-352`

The two-store split itself is **sound and I would keep it.** The justification
checks out: `log/sqlite.rs:405-409` and `:591` do state that there is no migration
path by design, so a `chosen_paths` table in `ops.sqlite` really would be a
`LAYOUT_VERSION` bump that permanently refuses every store a prior build wrote. I
verified that rather than taking it. The differing failure domains and the
separately-copyable record are real secondary benefits.

What is wrong is the **duplication the split was allowed to carry**, and
specifically that `design.md:296-301` records the opposite:

> *"**`identity.sqlite` has no `check_layout` equivalent** → … Recorded as a known
> asymmetry rather than copied, because copying it would be copying 40 lines of
> machinery for a table with two columns."*

`identity_store.rs` has a `check_layout` (`:238-249`) — the risk register entry
describes a tree that does not exist. And the copying went well past
`check_layout`. Second copies of `log/sqlite.rs`, structure for structure:

| duplicated item | `identity_store.rs` | `log/sqlite.rs` |
|---|---|---|
| `LAYOUT_VERSION` const + pinning test | `:58`, `:807-814` | `:74`, `:899` |
| `storage()` error adapter | `:130-132` | (same shape) |
| `in_memory()` + its doc argument | `:195-198` | `:252-255` |
| `from_connection` version triage | `:200-219` | `:257-296` |
| `check_layout` + `LIMIT 0` + `QueryReturnedNoRows`-is-success | `:238-249` | `:330-352` |
| `create_schema` with pragma-last + `ROLLBACK` on failure | `:271-299` | `:354-…` |
| `UnknownLayoutVersion` / `LayoutDoesNotMatchItsVersion` arms | `:76`, `:82` | (same) |
| `TempDir` test fixture | `:823-845` | `:864-…` |

**Concrete illustration of the cost.** The pragma-last invariant now exists twice,
each guarded by its own SHOUTING comment (`identity_store.rs:262-270`,
`log/sqlite.rs`'s equivalent). A future change that learns something about SQLite
version stamping — say that `user_version` should be read inside a transaction —
has to find and fix both, and the second is findable only by knowing it exists.
Likewise a future third store inherits nothing and writes a third copy.

**`CLAUDE.md`, "the fourth slightly-different copy of a guard is the signal to
reshape".** Two stores sharing this machinery is the point at which a
`versioned_sqlite` helper — open, triage the version, create-or-check, with the
layout claim and the schema passed in — is the reshape. That is a
no-behaviour-change refactor of `log/sqlite.rs` that should have **preceded** this
change, after which `identity_store.rs` is a schema string, two statements and a
decode guard: genuinely the "table with two columns" the design describes.

**Also:** the `TempDir` test fixture is now in its **fourth** copy —
`keystore.rs:1436`, `log/sqlite.rs:864`, `identity_store.rs:823`, and
`wire.rs:1620` as `OnboardingDir`. `wire.rs:1617-1619`'s own comment says *"Same
shape as `keystore.rs`'s and `log/sqlite.rs`'s"*, i.e. it identifies itself as the
copy CLAUDE.md names and proceeds anyway. Four copies of a `Drop`-guard temp
directory is the textbook instance of the rule.

---

## A6 — Three public API surfaces added with no production caller

**For: `dev-writer`**
**Severity: low — genuine defect of shape (speculative widening)**

`CLAUDE.md`: *"do not refactor speculatively. Make room for the change in front of
you, not one you imagine"*, and *"widening [the API] is a decision to make on
purpose rather than a side effect of needing one more field."* Three additions have
no non-test caller anywhere in the tree (grepped across both crates):

- `Keystore::stoa_key_at_path` (`keystore.rs:728`) — used only by `keystore.rs`'s
  own tests and by `wire.rs`'s tests.
- `Keystore::stoa_address_at_path` (`keystore.rs:739`) — same.
- `IdentityStore::all_paths` (`identity_store.rs:371`) — used only by tests.

Only `stoa_public_key_at_path` (`:733`) has a production caller, at `wire.rs:700`.

`all_paths` is the defensible one: the spec does require the record be *"readable
in full… so that it can later be exported"* (`spec.md:166`), and a method is a
reasonable way to discharge that. The other two are the trio-for-symmetry pattern,
added because the pathless trio has three members. Each is a public method on the
secret-holding type, and a public method that returns a `SecretKey` is the kind of
surface `keystore.rs:697-700`'s "there is no accessor for the root" argument is
carefully narrow about.

Note also that the three additions plus `slate_for`, `slate_from_nonce`
(`:755`, `:767`) and module-level `protection_from_env` (`:365`) make **six** new
public items on `keystore`, whose spec `proposal.md:52-57` lists under *"Not
modified, deliberately — `keystore` — unchanged."* The file format is indeed
unchanged, which is what that paragraph argues about; the **type's API** is not, and
the proposal does not say so. A reader taking "keystore — unchanged" at face value
will not expect six new public items.

---

## A7 — `keystore` now depends on `onboarding`, which is the right call, recorded for the record

**For: `dev-writer`**
**Severity: informational — not a defect**

`Keystore::slate_for` / `slate_from_nonce` (`keystore.rs:755`, `:767`) return
`crate::onboarding::Slate`, so the secret-holding module imports the onboarding
vocabulary, while `onboarding.rs:214` deliberately takes `&[u8; 32]` to avoid
depending on `Keystore`. That looks inverted at first read and is not: the
alternative is a root accessor on `Keystore`, which is exactly the accessor
`keystore.rs:697-700` argues must not exist. Passing the operation in rather than
the secret out is the correct direction and the doc comment at `:748-752` makes the
argument.

Flagged only so a later reader does not "fix" the direction. The one thing I would
change is where these two live: they are onboarding's methods wearing a keystore
signature, and a free function in `onboarding.rs` taking `&Keystore` plus a
`pub(crate)` root accessor would keep the dependency one-way — but that trades a
module boundary for a crate-internal secret accessor, and the current shape is the
better of the two. No action needed.

---

## A8 — `UI-BRIEF.md` now describes a flow the code does not implement

**For: `dev-writer`**
**Severity: low — genuine defect, follows from A2**
`docs/UI-BRIEF.md:168-176`

The brief was updated in the same change, which is what `CLAUDE.md` requires, and
the two new obligations (7, recovery needs the record; 8, an unencrypted key is a
state to show) are both correct and well-argued. But the onboarding paragraph now
tells the designer:

> *"five identities, pick one, refresh for more… Core now serves this — a slate
> call returns five candidates with an address and a public key each, a keep call
> stores the one chosen"*

Given A2, on a fresh install a keep does **not** store the one chosen, and
"refresh for more" hands out candidates of a *different* master key each press
rather than more candidates of one. The brief is designed against by someone who
cannot read the code, so it is currently promising a flow that does not exist.
Fixing A2 makes this paragraph true; nothing needs changing here **if** A2 is
fixed, and this must not be resolved by weakening the brief instead.

---

## A9 — `design.md`'s derivation-chain citations point at pre-change line numbers

**For: `spec-writer`** (and flagged for the design reviewer, whose dimension this
mostly is)
**Severity: low**
`openspec/changes/identity-onboarding/design.md:9-13`

The brief asked me to verify this chain rather than trust it. **The chain is
real** — I walked it: `lib.rs:382` → `keystore.rs:712` `stoa_address` → `:706`
`stoa_public_key` → `:701` `stoa_key` → `:702` `derive_stoa_key(&self.root, stoa)`.

The **citations are stale**: `design.md` gives `lib.rs:251` and
`keystore.rs:651/:645/:640/:641`, which are the pre-change line numbers. In the
tree these documents ship with, `lib.rs:251` is a doc-comment line about the slate
nonce. `proposal.md:217-220` repeats the same stale numbers.

Worth fixing because this project's recorded failure mode is that the most
convincing citation is the unread one — a reader who checks `lib.rs:251`, finds
prose about a nonce, and concludes the chain claim is fabricated would be drawing
the wrong lesson from a correct claim. Prefer citing the symbol chain
(`stoa_address` → `stoa_public_key` → `stoa_key` → `derive_stoa_key`), which does
not rot.

---

## Noted for the tester, outside my dimension

Recorded only because I tripped over them while reading for shape; not my findings
to make and not counted above.

- `wire.rs:3504` `every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape`
  is the "the property holds for EVERY method" gate and was **not** extended with
  `generate_identity_slate`, `keep_identity` or `who_am_i`. The gate's own comment
  says *"The wire contract is only useful if it holds for EVERY method"*.
- `Keystore::from_root_for_test` (`keystore.rs:689`) supplies the same fixed root
  `[7u8; 32]` to 24 call sites, which is what makes A2 invisible; and
  `onboarding.rs` uses `[7u8; 32]` for both the master key and the nonce in most
  tests, which two of its tests explicitly work around (`:678`, `:732`) having been
  bitten once.
- I did not run `cargo mutants`; it is the tester's instrument and my brief scopes
  me to architecture.

---

## Areas that were clean

Said plainly rather than padded:

- **The nonce-not-held-state decision (`design.md`, `onboarding.rs:22-33`) is
  right, and it is the best decision in the change.** It collapses "superseded" and
  "never existed" into one comparison, so there is no second path to get wrong, and
  it makes "generating a slate writes nothing" hold by the signature rather than by
  a line somebody has to not add. `generate_identity_slate` has no store parameter
  — the requirement is structural. This is the CLAUDE.md rule applied correctly.
- **`stoa BLOB PRIMARY KEY` plus `INSERT` without `OR REPLACE`**
  (`identity_store.rs:278`, `:318`) makes one-path-per-Stoa and refuse-to-replace
  properties of the schema rather than guards at each write. Complexity in the data
  structure, exactly as asked.
- **`Kept` and `Whoami` as enums with one payload each** (`wire.rs:368`, `:563`),
  following `Capability`, so a success-with-no-identity and a failure-with-one are
  unrepresentable. Serialisation is total and no handler has a branch to get wrong.
  `Whoami`'s fourth state — master key present, no path recorded for this Stoa —
  is named as its own reason rather than collapsed into "you are nobody", which is
  the right call and the one the two-store split forced.
- **`Kept::Stored.encrypted` taken from the `Unlock` used rather than by re-reading
  the file** (`wire.rs:552`). Re-reading would report the protection of whatever is
  at the path *now*; the value that is true is the one this code used. Correct, and
  the less obvious of the two options.
- **`recovery_needs_the_record` as a boolean rather than prose**
  (`wire.rs:579`), so the change that implements backup flips a value rather than
  altering a reply shape. Right instinct about which future changes are cheap.
- **`SLATE_SIZE` is not a request parameter** — the request has nowhere to put a
  count, which is the strongest form of "not requested by the caller" and closes an
  unbounded-derivation request at the module boundary by construction.
- **`protection_from_env` placed in `keystore` rather than the adapter**
  (`keystore.rs:369`), so the `OsStr`-bytes handling is decided once. A second copy
  in the adapter would eventually map two passphrases onto one key, and the doc
  comment says so.
- **The core/UI split is respected.** Nothing added to `dialectica-ui`; every
  network- and disk-touching decision is in core or (see A2, which is the exception
  and the finding) the adapter. `UI-BRIEF.md` was updated in the same change, as
  `CLAUDE.md` requires.
- **CI's structural gates survive.** No file moved or renamed; the test-count gate
  (`ci.yml:684-687`) names both source roots and both still exist; the panic-guard
  gate still finds `catch_unwind` in `wire.rs`. `cargo fmt --check` and
  `clippy -D warnings` clean for both packages.
- **No new dependency.** `rusqlite`, `sha2`, `zeroize`, `hex`, `getrandom` and
  `hkdf` were all already in the tree, so there is no licence or maintenance
  question to answer. Reusing SQLite for the second store rather than hand-rolling
  an atomic JSON write is the right trade and `design.md:111-119` argues it well.

---

## Tree state

Two temporary mutations were made and both are reverted:

- a throwaway test in `wire.rs` to reproduce A1's address divergence — removed;
- `dialectica/logos-rust-sdk-src`, the gitignored SDK symlink, created so `cargo`
  could resolve the manifest at all. It is gitignored and untracked, so the tree is
  clean with it in place.

`git status --short` in the review worktree is **empty** apart from this findings
file. Suite re-run after reverting: **553 passed, 0 failed**, matching the stated
baseline.
