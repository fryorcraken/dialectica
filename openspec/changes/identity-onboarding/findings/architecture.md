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

- [x] **A1 — `getCapabilities` and `whoAmI` report two different author addresses for the same user in the same Stoa**

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

**Fixed** in `63133c9` — the **first** of the two shapes offered, and the finding's
framing of it as "a real API decision worth taking on purpose" is why it is recorded
in `design.md` rather than just done.

`core::wire::posting_identity(stoa, keystore, paths)` consults the record and derives
at the recorded path; `get_capabilities_from_stores` is what the adapter forwards to.
So `getCapabilities` does now depend on `IdentityStore`, deliberately.

**Why not the second shape.** Retiring the pathless trio from the live path does not
answer the question, because the probe would then have no identity to report at all —
the question is not which of two derivations to use, it is whether the probe may
answer without consulting the record. It may not: a master key with no recorded choice
for this Stoa cannot post as anyone, and that is now `canPost: false` with a reason
naming the missing choice, reported through the same constant `whoAmI` uses so the two
cannot describe one state in two vocabularies. It previously answered `canPost: true`
there.

**The sentence the finding says is now false has been corrected**, and it was the
useful pointer: `Keystore::stoa_address`'s doc comment claimed to be "what the probe
reports". As a side effect the whole pathless trio now has **no production caller** —
recorded in `design.md`'s Risks rather than deleted, because removing three public
methods from the secret-holding type belongs in a change whose proposal does not
declare `keystore` untouched.

Measured: returning the pathless address fails
`the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` and **nothing
else in 563**, confirming this finding's "zero tests fail" measurement.

**For `spec-writer`, and not closed:** `proposal.md` still declares
`posting-capability` unchanged with its derivation untouched. The finding is precise
about why that matters — *"the shape is untouched and the contract is broken, which is
the distinction the proposal's own 'Not modified, deliberately' section exists to make
and got backwards."* That correction is `spec-writer`'s to make.

---

- [x] **A2 — `master_key()` mints a throwaway root, so on a fresh install the kept identity is **not** any candidate the user was shown; the logic sits where no gate can reach it**

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

**Fixed** in `63133c9`, and this entry's last sentence is the one that shaped the fix
more than anything else in the six files: *"fixing it in the adapter would leave it
equally unreachable."* The temptation was a two-line change to `master_key` caching its
result. That would have worked and stayed invisible to every gate, which is the defect
this finding is actually about.

So the decision moved. `core::wire::OnboardingSession::keystore_for` holds the
mint-or-open choice, and the session holds `(keystore, live_slate)` as one value —
which is the **first** of the two shapes offered here, chosen because the finding's own
observation that *"the module already holds 32 bytes across calls; holding the
un-written keystore is the same span"* is exactly right.

**Why not the second shape** (write the keystore at slate time): the spec requires
"Generating a slate SHALL NOT write to storage", and `generate_identity_slate` having
no store parameter is what makes that structural rather than a line somebody has to not
add — which this same review calls the best decision in the change. Trading that away
to fix this would have broken a requirement to repair a defect.

**On the measurement.** The finding identifies `from_root_for_test([7u8; 32])` at 24
call sites as what makes this invisible, and it is right — so the regression test
supplies **no** key at all. Its opener returns `NotFound`, minting actually happens, and
the assertion is a relationship between two replies rather than a comparison against a
constant. That is the only fixture shape that can distinguish "the slate and the keep
agree" from "the harness gave them the same one".

Restoring the per-call mint fails it plus two others and nothing else in 563.

**What is left in the adapter** is what the finding says cannot move: the host's
directory and the environment the protection is read from. `Dialectica`'s second field
is a `Default`-constructible `core` type, so `interface: "universal"`'s parameterless-
constructor requirement still holds.

**Still unreachable by `cargo test`, and that has not changed:** the adapter is
`#[cfg(logos_scaffold)]`. What changed is that there is no longer a *decision* in there
to be unreachable. The CI test-count gate is still blind to a whole `impl` being cfg'd
out, as this finding notes; nothing here fixes that.

---

- [x] **A3 — `parse_stoa` was extracted for the three new handlers and three pre-existing copies were left behind; the fourth-copy signal was answered halfway**

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

**Fixed** in `d8f9816`: all three pre-existing copies converted, so the tree holds one
`parse_stoa` and six call sites.

The finding's diagnosis of why this is worth doing despite nothing being broken is the
whole of it: *"they are behaviourally identical **today**, which is precisely why this
is a shape finding rather than a correctness one. The cost arrives on the next change
that tightens the parse … it lands in one place and three handlers keep the old
behaviour, with no test failing, because each copy has its own passing tests."* A
finding whose evidence is that nothing fails is the kind that gets waved through.

`parse_stoa`'s doc comment now says "every handler that takes a Stoa" rather than
naming a count — the old text said "three handlers below", which was a number that
went stale the moment a fourth arrived, and is the same class as R7's test count.

**The ordering criticism is accepted and not remediable now.** The conversion should
have been its own commit before the feature, and landing it in a fix-pass commit that
also touches five comments is not that. What I did do was apply the same principle
where it still had force: `wire.rs`'s formatting went in its own commit ahead of the
behaviour changes (see R8), for exactly the reason this entry gives.

---

- [x] **A4 — `storage_dir()` has the same half-done extraction, in the adapter**

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

**Fixed** in `63133c9`: `get_capabilities` and `list_threads` both use
`storage_dir()`, so the string exists once. `grep` for it in `src/lib.rs` returns a
single hit.

Two more extractions went in alongside, for the same reason and found while doing
this: `Self::paths(&dir)` and `Self::open_keystore(&dir)`, each of which was about to
be its third and fourth copy across the five handlers. The A2 fix removed the
`master_key` helper this entry sits beside, so the adapter's private helpers are now
`storage_dir`, `paths` and `open_keystore` — all three "where is the file" questions
that genuinely cannot move to `core`.

**The last sentence is the one worth keeping on the record**, because it is the
argument for fixing a cosmetic duplication at all: this is the file no test reaches,
so a divergence here is a divergence nothing would catch. That is why "the string is a
constant and a divergence would be cosmetic" is not a reason to leave it.

---

- [x] **A5 — `identity_store.rs` is a second copy of `log/sqlite.rs`'s store machinery, and `design.md` claims it is not**

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

**PARTIALLY fixed; box left OPEN.** Two of the three claims here need different
answers, so splitting them:

**The `design.md` misstatement is fixed** (`d8f9816`). The risk entry said
`identity.sqlite` has *no* `check_layout` equivalent, describing a tree that does not
exist — readability's R1 found the same thing. It now states what `check_layout` does
prove and what it does not, which turned out to matter: security's S4 measured that
`LIMIT 0` proves column names and nothing about constraints, so the entry's residual
risk is real rather than rhetorical.

**The `versioned_sqlite` reshape is NOT done, and that is why this box is empty.** The
finding is right that two stores sharing this machinery is the reshape point, and right
that it should have *preceded* this change. Both of those make it a poor thing to do
now:

- It is a refactor of `log/sqlite.rs`, a file this change does not otherwise touch, in
  a fix pass already carrying four high-severity defect fixes. The commit that reshapes
  the op log's open-and-triage path needs to be reviewable as that and nothing else.
- The finding's own ordering argument says so: done first, `identity_store.rs` would
  have been written as "a schema string, two statements and a decode guard". Done last,
  it is a rewrite of a file whose tests are the only thing standing between the op log
  and a silently unopenable store.

**The `TempDir` fourth copy is recorded rather than unified**, per R6: the comment now
names all four copies and says whoever needs a fifth should decide deliberately. The
`wire.rs` comment that *"identifies itself as the copy CLAUDE.md names and proceeds
anyway"* — a fair hit — now at least identifies the right precedent, which it did not.

Left open because a reviewer reading a ticked box here would reasonably conclude the
duplication was addressed. One table in it was corrected; the eight rows of duplicated
machinery are all still there.

**Deferred** in `0c99b87` — **and now actually written down, which the previous pass did not do.** The
pass above reasoned the deferral correctly and then left it only here, in a directory
the runner deletes before merge. That is a drop, not a defer: the eight rows of
duplicated machinery would have gone unrecorded the moment `findings/` went.

`design.md`'s Risks / Trade-offs now carries it, in the entry beginning
*"`identity_store.rs` is a second copy of `log/sqlite.rs`'s open-and-triage machinery,
and a `versioned_sqlite` helper is the reshape that would remove it"*, placed directly
after the "Two SQLite files where there was one" entry — because that decision is what
this duplication cost, and the two belong beside each other. It records all eight
duplicated items, the concrete cost (the pragma-last invariant existing twice, each
under its own shouting comment, so a future SQLite lesson has to find both), the
`versioned_sqlite` shape itself, both reasons it is not done in this pass, and where it
goes: a no-behaviour-change change taking `log/sqlite.rs` and `identity_store.rs`
together, with a third store as the forcing event. The `TempDir` fourth copy is
recorded in the same entry rather than as a separate note, since it is the same rule.

**Line numbers were re-measured rather than copied.** The finding's `identity_store.rs`
citations had drifted — the fix pass moved the file substantially — so the design.md
entry cites current lines: `check_layout` at `:274` (not `:238-249`), `create_schema`
at `:311`, the pragma-last comment at `:302`, the `LAYOUT_VERSION` pinning test at
`:1076`, `TempDir` at `:1086`. `log/sqlite.rs`'s numbers were unchanged and verified.
`keystore.rs:1436` and `wire.rs:1620` from the `TempDir` list are now `:1466` and
`:2536`. The finding's reasoning holds at every one; only the addresses moved.

Nothing here was reconsidered as belonging in this piece. The finding's own ordering
argument is what rules it out, and it got stronger rather than weaker: the pass this
box sat through added four high-severity fixes, so a `log/sqlite.rs` rewrite would now
arrive in an even less reviewable diff.

---

- [x] **A6 — Three public API surfaces added with no production caller**

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

**OPEN, and the A1 fix made this worse in an instructive way.** Nothing is deleted and
the box stays empty.

`stoa_address_at_path` — one of the two the finding calls "the trio-for-symmetry
pattern" — **now has a production caller**: `posting_identity` uses it, because the A1
fix needed exactly that method. So one of the two flagged additions was not speculative
after all; it was needed by a defect nobody had found yet.

`stoa_key_at_path` still has none. And the A1 fix retired the *pathless* trio from the
live path, so `Keystore::stoa_key`, `stoa_public_key` and `stoa_address` now have no
production caller either — three more than this finding counted, and pre-existing rather
than added. Deleting them is a real question and it is not this change's: removing three
public methods from the secret-holding type, in a change whose proposal declares
`keystore` untouched, is precisely the kind of silent widening-in-reverse the finding
objects to. Recorded in `design.md`'s Risks.

`all_paths` the finding itself calls defensible, and I agree: the spec requires the
record be readable in full.

**The proposal correction is `spec-writer`'s**, and it is the substantive half. The
finding's distinction — *"The file format is indeed unchanged, which is what that
paragraph argues about; the **type's API** is not"* — is exactly the distinction A1's
"derivation untouched" claim got wrong too. Same paragraph, same failure mode, twice.

Left open because the API surface question is unresolved either way: one item gained a
caller, three lost theirs, and the proposal still says none of it happened.

**Deferred** in `0c99b87`, **with the full count written into `design.md` so it is not
recounted a third time.** The pass above was right that the deletion is not this change's, and it
did record the pathless trio — but only the pathless trio. A6's own subjects,
`stoa_key_at_path` and the proposal's `keystore — unchanged` claim, had no durable
home, and `findings/` is deleted at merge.

`design.md`'s Risks / Trade-offs now carries all of it, appended to the existing
*"`Keystore::stoa_key`, `stoa_public_key` and `stoa_address` now have no production
caller"* entry rather than as a second entry, because it is one question asked from
two ends and splitting it is how the count drifted. It records: that
`stoa_address_at_path` gained a production caller in `posting_identity` (with the
honest reading — symmetry was a weak argument that happened to be right, not a
vindicated one); that `stoa_key_at_path` still has no handler caller; that `all_paths`
needs no defence; that the retirement question is therefore **four** methods, not
three, and must be decided together; why the deletion needs a proposal rather than a
commit; and the format-versus-API conflation that the proposal still carries.

**One correction to the finding, which I could not tick honestly without making.** A6
says `stoa_key_at_path` is *"used only by `keystore.rs`'s own tests and by `wire.rs`'s
tests"*. Grepping the whole tree: `stoa_public_key_at_path` calls it at
`keystore.rs:764`, one line below its definition at `:758`. So it is not test-only
code — it is a `pub` method with a production caller inside the same type and no
*external* caller but tests. That changes the remedy rather than the verdict: making
it private is the cheap answer, and its cost is that those tests lose the ability to
reach a `SecretKey` at a path. `design.md` states it that way. The finding's
conclusion — that this is speculative public surface — stands; the evidence line
needed one word changed.

Line numbers were re-measured: the finding's `keystore.rs:728/:733/:739` are now
`:758/:763/:769`, and `:697-700` is `:727`. `IdentityStore::all_paths` is at
`identity_store.rs:472`, not `:371`.

The **proposal correction remains `spec-writer`'s** and is not discharged by this
box — `proposal.md:52-57` still says `keystore` is unchanged. It is named in
`design.md` so the next reader sees the claim is known-wrong rather than trusted, but
a design.md note is not a fixed proposal. Flagged in my report.

---

- [x] **A7 — `keystore` now depends on `onboarding`, which is the right call, recorded for the record**

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

**No action taken, as the finding says is right.** Ticked because "no action needed" is
the outcome and the entry's purpose — stopping a later reader from "fixing" the
direction — is served by it being on the record.

Worth confirming it survived the fix pass unchanged, since the session work moved
several things across this boundary: `slate_for` and `slate_from_nonce` still return
`crate::onboarding::Slate`, `onboarding.rs:214` still takes `&[u8; 32]` rather than a
`Keystore`, and no root accessor was added. `OnboardingSession` holds a whole
`Keystore` rather than a root, which keeps the same rule — the operation is passed in,
the secret does not come out.

The reasoning here also answered a question the A2 fix raised: `keystore_for` returns
`&Keystore` rather than the root for the same reason `slate_for` exists at all.

---

- [x] **A8 — `UI-BRIEF.md` now describes a flow the code does not implement**

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

**Fixed by fixing A2, which is what this entry prescribes** — and the prescription
mattered, because weakening the brief was the cheaper option and would have been the
wrong one. The paragraph is now true: a keep stores the candidate the slate showed
(`on_a_fresh_install_the_identity_kept_is_the_candidate_the_slate_showed`), and
refreshing offers more candidates of one identity's key
(`refreshing_a_slate_offers_candidates_of_one_master_key`). Both are named tests
because "the brief is now accurate" should rest on something a gate can check.

That second test exists **because of this entry**. The A2 fix would have been complete
without it — the kept-identity property is what the defect was about — but "refresh for
more" is a separate claim the brief makes to a designer, and it deserved its own
assertion rather than being inherited.

The brief's paragraph is unchanged, deliberately. I did add one thing the fixes make
newly relevant, to obligation 8: `encrypted` describes the master key, which is **one
file per install**, so the first keep reports the protection it wrote and a second
Stoa's keep reports the protection the existing file has. A designer showing that flag
per Stoa could otherwise render "this identity is encrypted, that one is not", which is
not a state that can occur. That is a consequence of the A5/design-review-1 fix
(a second Stoa being keepable at all), so it could not have been written before.

---

- [x] **A9 — `design.md`'s derivation-chain citations point at pre-change line numbers**

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

**Fixed** in `63133c9`, by the symbol-chain form this entry recommends. `design.md`'s
Context paragraph now walks `stoa_address` → `stoa_public_key` → `stoa_key` →
`identity::derive_stoa_key` with no line numbers, and records *why* the change was
made — because "line numbers in a `design.md` rot inside the same change" is a
generalisation worth leaving behind, and three reviewers spent effort rediscovering it
independently.

The reasoning in this entry is the part I kept verbatim in the document: a reader who
checks a stale citation, finds unrelated prose, and concludes the claim was fabricated
draws the wrong lesson from a correct claim. That is a more specific hazard than
"citations should be accurate", and it is the one this repo has actually hit.

One thing added beyond the fix: the paragraph now notes that `getCapabilities` **no
longer uses that chain** at all, after A1. The chain claim was true when written and
the change it describes has moved past it, so leaving it uncorrected would have made an
accurate citation point at an abandoned design — the same failure one level up.

Addressed to `spec-writer` in the original. I fixed `design.md` because it is
`dev-writer`'s file; **`proposal.md:217-220` repeats the same stale numbers and I have
not touched it**, since the proposal is `spec-writer`'s.

---

- [x] **Noted for the tester, outside my dimension**

Recorded only because I tripped over them while reading for shape; not my findings
to make and not counted above.

- `wire.rs:3504` `every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape`
  is the "the property holds for EVERY method" gate and was **not** extended with
  `generate_identity_slate`, `keep_identity` or `who_am_i`. The gate's own comment
  says *"The wire contract is only useful if it holds for EVERY method"*.

**Open — `tester`'s**, and I am leaving it rather than taking the first item myself.

I verified the first observation: that gate still does not include the three onboarding
methods, and its comment still claims the property holds for every one. It is a genuine
hole and it is a *test* hole — the gate is a test asserting a property across the wire
surface, and `tester` owns the suite. Extending it also means deciding what the three
handlers' "exactly one top-level shape" is, which is a reading of the contract rather
than a mechanical addition.

The other two observations need no action: `from_root_for_test`'s 24 fixed-root call
sites are what made A2 invisible, and the A2 fix answered that by adding a test that
supplies **no** key rather than by changing those sites. `cargo mutants` is correctly
identified as the tester's instrument.

Unticked because the gate's comment is currently false about three methods, which is
exactly the "a claim nothing checks" shape this project keeps finding.
- `Keystore::from_root_for_test` (`keystore.rs:689`) supplies the same fixed root
  `[7u8; 32]` to 24 call sites, which is what makes A2 invisible; and
  `onboarding.rs` uses `[7u8; 32]` for both the master key and the nonce in most
  tests, which two of its tests explicitly work around (`:678`, `:732`) having been
  bitten once.
- I did not run `cargo mutants`; it is the tester's instrument and my brief scopes
  me to architecture.

**`tester`: the hole was real and is now CLOSED — by `dev-writer` during the
`origin/main` merge, not by me. I verified it rather than duplicating it, and the
verification found the coverage is nine times wider than this entry describes.**

Two corrections to the entry, both from reading the post-merge tree:

- **The gate has been renamed and its false claim removed.** It is now
  `every_handler_answers_with_a_json_object_for_any_request_shape`, and its own
  comment says the old name's *"exactly one top-level shape"* was a claim nothing
  checked — the body asserts `is_object()`. So the specific sentence this entry
  quotes no longer exists to be false. The rename is the right call: asserting "one
  shape" would mean asserting the key set, and the success shapes differ per method
  (`pong`, `channelId`, `items`/`page`/`hasMore`).
- **`:3504` is a pre-merge line number.** The gate is at `wire.rs:7961` now. Noted
  only because this file's A-something entry above makes the same point about stale
  citations in `design.md`, and the lesson applies to findings too.

**All three onboarding methods are in `every_request_taking_method()`**
(`wire.rs:6952-6961`), each wrapped as a `fn` with a fresh session per call, each
with a served fixture in `a_served_request`. The helper feeds **nine** sweeps, not
one — so the fix extended coverage far beyond the object-shape gate this entry names.

**Proven reached, not assumed.** A sweep listing a method proves nothing if the
sweep never executes it, so I mutated a handler in a way that leaves `ping` alone:

| Mutation | Predicted | Observed |
|---|---|---|
| `who_am_i`'s success arm replaced by `error_json(...)` | `a_request_within_the_cap_is_still_served` fails naming `who_am_i` | **exactly that** — `who_am_i refused a request well under the cap: {"error":"deliberate mutation…"}` |

**What I found while verifying, and it is a defect rather than a test gap** — though
`dev-writer` found it independently and has it written up in `design.md`, so this is
corroboration and not a new report. Trying to mutate `generate_identity_slate` back
to its pre-merge `serde_json::from_str` produced a **compile error**, not a test
failure: `parse_stoa` takes `&Request`, and `Request::parse` is the type's only
constructor. So for a handler that reads `stoa`, the envelope is structural — the
`MAX_REQUEST_BYTES` cap and the by-name non-object refusal cannot be bypassed while
still compiling. That is CLAUDE.md's put-it-in-the-data-structure rule holding, and
it is a stronger guarantee than the sweep.

The residual is on the **publish** surface, and it is live on `main`:
`publish_post`, `publish_reply` and `publish_vote` open with `parsed_object(request)`
— a bare `serde_json::from_str` — so they reach neither the cap nor the object
refusal, and they are not in the sweep list. `design.md` carries the full write-up
with line numbers and the ownership argument (it belongs to `authoring-content`,
already merged). **Not fixed here and not mine to fix**; recorded so the two accounts
agree.

The entry's other two observations still need no action, and I confirm the reasoning:
`from_root_for_test`'s fixed root is exactly what
`on_a_fresh_install_the_identity_kept_is_the_candidate_the_slate_showed` sidesteps by
supplying **no** key. I did not run `cargo mutants` either — targeted mutations
answered the questions the boxes asked, and the suite is 656 green.

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
