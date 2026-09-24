## Context

The motivation is in `proposal.md` (Why). This file covers the choices behind the
rewrite. The main question it answers is why the change needs **no code**, and
which existing guards the new scenario depends on.

Before this change the `identity` spec contradicted itself in a way nothing could
catch. The scenario "An identity is a pure function of its root and its Stoa"
said that *"no operation exists that yields a different key for the same pair"*.
Three scenarios in the same file require exactly such keys:

- "Different paths for one Stoa yield different identities"
- "The path-taking scheme does not collide with the scheme without one"
- "A per-Stoa identity is not the root identity"

The contradiction dates from the path-taking derivation (#58, `26d8a08`, on
2026-09-13). `openspec validate --strict` passed the file throughout, because
validation does not check whether scenarios contradict each other
(`docs/OPENSPEC-ARCHIVE.md`). No test cited the scenario either:
`git grep -F "pure function of its root" -- dialectica` returned nothing. That is
how it survived. A scenario that no test cites can be false without anything
turning red.

The two Purpose paragraphs had drifted the same way, and for a structural reason:
a delta cannot change a Purpose. #149 (`machine-identity-scope`) made the machine
key the identity in every Stoa, but it could not edit either Purpose, so both
still described per-Stoa identity as current.

## Goals / Non-Goals

**Goals:**

- Every scenario in the `identity` requirement that forbids rotation can be
  checked by a test that fails against a build which rotates.
- Neither Purpose paragraph states something this release does not do.

**Non-Goals:**

- No change to identity behaviour, key derivation, the wire shape or the view.
  The behaviour the new scenario names already exists (Decision 4).
- No new guard. Decision 4 records the guards that already exist, so the test the
  tester writes can be shown to depend on them.
- Restoring per-Stoa identity. That is #108. Both Purposes keep their per-Stoa
  text so that #108 does not have to write it back (Decision 3).

## Decisions

### 1. The scenario checks the requirement's own claim for one named operation, not for every operation

The requirement's claim still holds and stays: *there SHALL be no way to replace
the key behind an identity while keeping the identity.* The issue put the defect
as *"'No operation exists' is a claim about the whole API, and no test can show
it"*. So the question was how to state that claim in a form a test can check.

The issue offered two rewrites, and a third option was considered:

- **The requirement's claim, worded as the issue words it** ("no operation
  replaces the key behind an identity while keeping the identity"). Rejected in
  that form. It still quantifies over every operation in the API, which is the
  defect being fixed. It is taken in a narrower form: one named operation and two
  observations.
- **Scope the old scenario to one derivation scheme and one path.** Rejected.
  Every clause that survives the scoping is already a scenario under *A user has
  one identity per Stoa, and it is permanent*. "The same root, Stoa and path
  always yield the same identity" covers the THEN clause. "A derived identity
  survives storage and reload" covers "at any time". The scoped scenario would
  fail only when those fail. It would also test derivation determinism under a
  requirement about rotation. That would delete the scenario while keeping its
  name.
- **Delete the scenario outright.** Rejected. The requirement's remaining
  scenario, "An identity is named by its key and by nothing beside it", counts
  identifiers. It says nothing about whether the key can be replaced, so the
  requirement's central claim would have no scenario at all.

**Why this operation.** In this release, the operation that creates a master key
(`createIdentity`, core `create_identity` → `mint_master_key`) is the only one
that:

- writes the key behind the identity in use,
- takes no Stoa, and
- is reachable from a first-run control.

Its reply names a key, so a caller who presses the button twice has a plausible
reason to think a second key was made. The only other code path that writes the
keystore is `keep_selection`. `git grep -n "write_to("` over `dialectica/rust-lib`
finds no production caller of the replacing write other than `Keystore::create`
itself. Outside tests, `create` is called from these two places and from the
`seed_store` example, which is a development tool and is not part of the module. It calls `create` only when no file exists
(`wire.rs`, the comment above that call), and *Keeping an identity does not
replace an existing one* already contracts it.

**Why two observations, the report and the signature.** They are the identity as
the user sees it and the identity as peers see it. A build could change one
without the other, for example by caching the reported key while signing with a
new one. The THEN and the AND clauses each close one of those cases. In this code
both observations open the keystore from disk on every call, so the two share a
source. The report goes through `posting_identity` over the adapter's
`open_keystore`. The signature goes through `publishing_key` over the
`open_from_env` call in the adapter's `publishing`. Both resolve
`default_path_in(&dir)`. The scenario does not assume that.

**The old THEN clause is dropped, and nothing replaces it.** "The result is the
same key" duplicated "The same root and Stoa always yield the same identity". The
new scenario compares the identity in use before and after an operation. That is
a different observation from deriving the same key twice, so the duplicate does
not come back.

### 2. REMOVED plus ADDED under a new name, not MODIFIED

`openspec` 1.13.0 refuses both obvious shapes:

- a `MODIFIED` block that leaves out a scenario the live spec still has
  ("archive refuses to drop them"), and
- `REMOVED` and `ADDED` under the same name ("Requirement present in both ADDED
  and REMOVED").

The spec-writer ran both against this change.

That left two options:

- **Keep the old scenario name and give it new content.** Other specs here have
  done this. Rejected, because this name, "An identity is a pure function of its
  root and its Stoa", states the false claim. The contradiction would survive in
  the heading.
- **Remove the requirement and add it back under a new name**, with Reason and
  Migration, as `join-preview-getstoa` did. Taken. The new name, *Identity does
  not rotate: the key behind an identity is never replaced*, keeps "Identity does
  not rotate" as its prefix. A search for the old name therefore still finds it.

No live spec, test or code cites either the old requirement name or the removed
scenario name, so the rename breaks no reference. A `git grep -F` for each name
across `openspec/specs`, `dialectica`, `dialectica-ui`, `docs` and `CLAUDE.md`
returns only the heading being removed.

### 3. The Purposes are edited directly, and they keep their per-Stoa text

A delta's Purpose is ignored for an existing capability, so the only way to fix a
Purpose is to edit `openspec/specs/` in this change's own commits. #149 left both
Purposes alone for exactly this reason. `proposal.md`'s table ("Which edits are
direct, and which are delta") lists each edit, its kind, and what archive does
to it.

**Both paragraphs add a statement about this release. Neither deletes its
per-Stoa description.** The alternative was to rewrite each Purpose for the
machine key alone. It was rejected for two reasons:

- #108 (milestone 0.0.3) restores per-Stoa identity. At that point the per-Stoa
  sentences become true again, and deleting them now would mean writing them
  back then.
- Both capabilities still contract the per-Stoa derivation, the slate and the
  keep. All of them are built and tested. A Purpose that left them out would
  describe less than the capability contracts.

So `identity`'s Purpose now says that unlinkability is what the per-Stoa
derivation *can* provide, and is suspended in this release. `identity-onboarding`'s
Purpose now says that a fresh install reaches posting by creating its master key,
and that no view reaches the slate.

### 4. No code changes. Two existing guards hold the behaviour, and a test must be proved against a build that actually replaces the key

Before writing this I checked the claim that nothing needs to change, rather than
taking it from the proposal. The requirement's new paragraph ("asking for a master
key while one is held MUST leave the identity in use unchanged") already holds.
Two guards hold it, one layered on the other:

1. **`mint_master_key`'s `keystore_path.exists()` branch** (`wire.rs`). It returns
   the existing key with `wasNew:false` and generates nothing.
2. **`Keystore::create`'s `AlreadyExists` refusal** (`keystore.rs`). It sits
   underneath the first guard and refuses to write over an existing file.

**The layering is what the tester needs to know.** Removing guard 1 alone does
**not** produce a build that replaces the key. Guard 2 refuses the write, and the
mint returns the error shape instead. The identity in use is still unchanged in
that build, so a test of the new scenario stays green against it. This is not a
guess: the comment on
`a_mint_over_an_existing_keystore_replaces_nothing_and_reports_it_as_not_new`
records that deleting guard 1 makes that test fail at `error.is_none()`, which
means the call was refused, not that the key was replaced.

So "delete the guard" is the wrong mutation for this scenario. The test must be
shown red against a build whose mint **writes a fresh root over a held one**. One
way to build that is to have the `exists()` branch generate a new keystore and
call `write_to`, which replaces a file by design (its doc says so). Guard 1 then
answers a different scenario from this one: its existing test checks that the
repeat call **succeeds** and that the file's **bytes** are unchanged. This
scenario checks what the user sees and what the network sees.

**Which layer can see this.** Core's `cargo test` reaches `create_identity`,
`who_am_i` and `publish_post` directly, and
`one_machine_key_posts_replies_and_votes_in_two_stoas` shows how to chain them.
One fixture trap applies. An opener written as `|| Ok(a_master_key())` returns the
same key whatever is on disk, so a replaced key cannot change what it reports. A
test using it cannot tell "replaced" from "not replaced". The opener has to read
the file, as `|| Keystore::open(&dir.keystore_path(), …)` does.

The adapter's wiring is behind `cfg(logos_scaffold)`, and only `nix build .#lgx`
compiles it. That wiring is what makes `createIdentity`, `whoAmI` and a publish
all resolve the same keystore path, `default_path_in(&dir)`. No `cargo test` sees it. This change does not touch it, so no
test here claims to cover it.

## Risks / Trade-offs

- **[Risk] "Asks for a master key" also reads as the read-only query.** The
  delta's scenario and prose say "asks for a master key". `identity-onboarding`
  has two operations that could match: the one that *creates* a master key
  (`createIdentity`) and the one that *reports whether one is held*
  (`getMasterKey`, which "MUST NOT create, write, replace or modify any stored
  key"). Under the second reading the scenario cannot fail by construction, and a
  test written that way would stay green against a mint that replaces the key.
  `proposal.md` (What Changes, and Impact) means the creating operation, and
  Decision 1 builds on that reading. → Reported to the spec-writer, who decides
  whether the delta should name the operation as `identity-onboarding` does ("the
  operation that creates a master key"). Until then, the tester should read it as
  the creating operation.

- **[Risk] Archive places the ADDED requirement at the end of `identity`'s
  spec, not where the removed one stood.** → This is not drift. `proposal.md`
  says the check is that the new block matches the delta byte for byte and that
  nothing else moved. The closer checks that, not the position.
- **[Risk] The direct Purpose edits and the delta are mistaken for each other at
  archive**, which is the `stoa-genesis` trap in `docs/OPENSPEC-ARCHIVE.md`. →
  The two kinds act on disjoint regions (the Purpose sits above
  `## Requirements`, and deltas act only on requirement blocks). `proposal.md`'s
  table names each edit and its kind.
- **[Risk] A future contradiction between scenarios would pass
  `openspec validate --strict` just as this one did.** → This change does not
  fix that. The defence this change adds is structural: every scenario in the
  requirement is now one a test can cite. A scenario that some test cites and
  that contradicts another scenario forces two tests to disagree, and one of
  them turns red.
- **[Trade-off] The scenario names one operation, not every operation.** A second
  operation that could replace the key would not be covered by this scenario. It
  would still break the requirement's SHALL, which covers every operation. The
  scenario is where a test can see the SHALL, not the whole of it. A new
  key-writing operation needs its own scenario, as `keep_selection` has one in
  `identity-onboarding`.
