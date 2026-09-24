# spec-test review — identity-spec-coherence

Scope reviewed: the change's delta (`openspec/changes/identity-spec-coherence/specs/identity/spec.md`),
the two directly-edited Purpose paragraphs (`openspec/specs/identity/spec.md`,
`openspec/specs/identity-onboarding/spec.md`), and the new test
`creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`
in `dialectica/rust-lib/dialectica-core/src/wire.rs`. Implementation code was
read only for the one mutation below, and no further than the lines mutated.

## Coverage (part 1)

- **REMOVED requirement** (*Identity does not rotate, and this is a contract
  not an omission*): no test obligation — the block is deleted, and its
  untestable scenario ("no operation exists that yields a different key...")
  goes with it.
- **ADDED requirement**, scenario *Creating a master key while one is held
  does not replace the identity in use*: pinned by the new test. The WHEN/THEN/AND
  clauses line up one-for-one with the test's steps (mint → `who_am_i` →
  mint again → `who_am_i` again → publish), and the opener reads the keystore
  file fresh on every call (`Keystore::open`), not a fixture constant — which
  is what makes the two "identity in use" observations independent
  re-derivations rather than echoes of the mint's own reply.
- **ADDED requirement**, scenario *An identity is named by its key and by
  nothing beside it*: carried verbatim, unchanged by this piece. Confirmed
  byte-for-byte identical between the delta's `ADDED` block and the requirement
  it replaces in the live spec. Coverage is many-to-many and pre-existing:
  `who_am_i_reports_an_identity_by_its_public_key_and_no_address` (wire.rs
  :5040) asserts `v.get("address").is_none()` on the identity report, and the
  `identity-onboarding` field-set-closure tests cover the same reply shape
  from the other side. Not new work for this piece, and none needed.

## Can the test fail? (part 2 — mutation run)

One mutation, on `mint_master_key`'s `keystore_path.exists()` branch in
`dialectica/rust-lib/dialectica-core/src/wire.rs` (around line 1331): changed it
from re-opening and reporting the existing key to generating a fresh keystore
and calling `write_to` over the held file (bypassing `Keystore::create`'s
`AlreadyExists` guard) — the exact mutation design.md prescribes, since
deleting the `exists()` guard alone does not replace anything (guard 2 refuses
the write and the test would stay green vacuously).

Command: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
dialectica -p dialectica-core
creating_a_master_key_while_one_is_held_does_not_replace_the_identity_in_use`

Result: **the test did not survive** — it failed exactly where expected:

```
assertion `left == right` failed: the second report must name the public key the first report named
  left: "9b85d6a8...78ac"
 right: "92f7d01b...df90e22"
```

Mutation restored (`git diff dialectica/rust-lib/dialectica-core/src/wire.rs`
now empty). Full suite re-run afterward and green: 1142 + 30 passed, matching
tasks.md's own report. No mutated code is left in the tree.

## NO SPEC markers (part 3)

None found near or introduced by this piece's work (searched
`dialectica-core/src/wire.rs` for `NO SPEC`). This change adds prose and one
test; it introduces no new undocumented behavioural choice.

## Delta fidelity (part 4)

Not a cross-capability move — this is a same-capability `REMOVED`+`ADDED`
rename, and proposal.md is explicit about that. Checked anyway: every carried
paragraph (the SHALL sentence, the "key that can be discarded at will"
paragraph, and "The affordance that was being held open...") is byte-for-byte
identical between the `REMOVED` requirement in the live spec and the `ADDED`
block in the delta, except the one new paragraph and the one rewritten
scenario the proposal documents. The dropped old THEN/AND clause ("the result
is the same key... no operation exists...") does not reappear anywhere else in
the delta. The two Purpose edits touch only the lines above `## Requirements`
in each file (`git show 43622b9 -- openspec/specs`), matching task 1.3's claim
exactly.

## Spec soundness (part 5)

- [x] **`spec-writer`** — the `identity-onboarding` Purpose paragraph this
      piece edits directly claims more than the capability's Requirements
      establish. **Where:** `openspec/specs/identity-onboarding/spec.md`,
      line 4, the clause newly added by commit `43622b9`: "Defines how a user
      comes to have an identity: **how a peer obtains its master key** and
      reports whether it holds one, ...". **Measured:** `git grep -rn "creates
      a master key\|obtains its master key\|generate.*master key"
      openspec/specs/` finds no `### Requirement:` anywhere (in
      `identity-onboarding`, `identity`, `posting-capability`, or `keystore`)
      that specifies the create/generate-a-master-key operation itself — what
      it does when no key is held, its reply shape, or that it is idempotent
      in general. The only formally specified operation about a master key
      besides derivation-from-slate is the read-only query ("Whether this peer
      holds a master key is reportable without creating one" — `getMasterKey`,
      which explicitly must create nothing) and, as of this piece, one single
      narrow property of the create operation (non-replacement while a key is
      held), stated in a *different* capability (`identity`). The
      "obtains its master key" half of the new Purpose sentence has no
      requirement to point to. This piece's own new scenario prose leans on
      that gap: it disambiguates its named operation as "the one
      `identity-onboarding` provides for a peer to obtain its master key" —
      but `identity-onboarding` provides no requirement by that description,
      only the Purpose sentence this same commit just wrote. **Scenario:** a
      reader trying to verify "what MUST `createIdentity` do the first time it
      is called, with no key held" has no requirement to check it against —
      the Purpose promises the capability defines this, and nothing does.
      **Severity:** moderate — not a defect in what this piece asserts (the
      one property it does add is correctly pinned and testable, confirmed by
      the mutation above), but the Purpose edit overclaims coverage the
      Requirements section doesn't have, and the new scenario's own
      disambiguating prose now depends on that overclaim being true.

      **Outcome (`spec-writer`): premise rejected, cross-reference fixed, the
      real gap deferred to a separate issue.** The requirement does exist. It is
      *A peer with no master key can obtain one without naming a Stoa* (with
      *Obtaining a master key never replaces one* beside it), in
      `openspec/changes/first-run-identity/specs/identity-onboarding/spec.md`.
      It specifies what the operation does with no key held, its reply (key,
      protection, whether this call created it) and that a repeat call succeeds
      without replacing. `first-run-identity` merged in #128 and was never
      archived. `git ls-tree origin/main openspec/changes/` still lists it. So
      the search above, scoped to `openspec/specs/`, could not see it. The
      Purpose's "how a peer obtains its master key" describes that
      requirement, which even uses the word "obtain". Removing the phrase would
      make the Purpose wrong once that change archives, so the Purpose is left
      as it is.

      The finding's second half holds, though. The delta's prose pointed at the
      operation by a description, and a reader of the promoted spec could not
      follow it anywhere. Fixed: the paragraph now names both requirements it
      distinguishes, *A peer with no master key can obtain one without naming a
      Stoa* and *Whether this peer holds a master key is reportable without
      creating one*. A `git grep -F` for the first name now finds the dangling
      reference until `first-run-identity` archives. `proposal.md` records the
      dependency beside its existing note on that change. This is a prose edit
      to the requirement. The scenario is unchanged, so no new test is needed.

      Deferred: archiving `first-run-identity` is not #157's scope. The closer
      of this piece archives this change only. The runner is to raise an issue.

## Areas checked and clean

- `openspec validate identity-spec-coherence --strict` passes.
- The GitHub issue (`gh issue view 157 --repo fryorcraken/dialectica`) matches
  proposal.md's account with no staleness: the AND-clause contradiction, the
  three colliding scenarios, and the two Purpose-drift paragraphs are all
  exactly as the issue states.
- The phrase "the operation that creates a master key" is used identically in
  the delta and in `identity-onboarding`'s existing "Whether this peer holds a
  master key is reportable without creating one" requirement — no terminology
  drift between the two capabilities.
- The Stoa named in the new scenario is consistent throughout the test
  (`slate_request()` and the publish both use `a_stoa()`), matching the
  scenario's "that Stoa" / "into it" wording.
