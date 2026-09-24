## Why

The `identity` spec's scenario "An identity is a pure function of its root and
its Stoa" contradicts other requirements in the same file and cannot be tested
(#157). Its **AND** clause, "no operation exists that yields a different key for
the same pair", has been false since the path-taking derivation landed: three
scenarios in the same spec require exactly such keys. Its **THEN** clause repeats
a scenario the first requirement already has. `openspec validate --strict` passes
the file anyway, because validation does not check whether scenarios contradict
each other.

Two Purpose paragraphs have drifted the same way. `identity`'s promises
unlinkability across Stoas, which this release suspends. `identity-onboarding`'s
presents the slate and the keep as how a fresh install reaches posting, when in
this release it gets there by creating the machine key, and no view reaches the
slate.

This is 0.0.1 work because a contract that contradicts itself cannot be relied
on, and the contradiction sits in the requirement that forbids rotation.

## What Changes

- **The scenario is rewritten, not deleted, and its claim changes to the
  requirement's own.** The requirement's real claim still holds and stays: *there
  SHALL be no way to replace the key behind an identity while keeping the
  identity.* The scenario now checks that claim at the one point in this release
  where it can fail: asking for a master key while one is held leaves the
  identity in use unchanged, both as reported and as signed.
- **Why this option and not the other.** The issue offers two rewrites. Here is
  what each gives when the test is "can a test fail against it?":
  - *The requirement's own claim, stated as the issue words it* ("no operation
    replaces the key behind an identity while keeping the identity"). As worded,
    this keeps the defect. It is still a claim about every operation in the API,
    and no test can enumerate those. So it is taken, but stated over a named
    operation and an observed outcome. A test fails if asking for a master key
    again changes the key the identity report names or the key a post is signed
    with.
  - *Scoping the old scenario to one derivation scheme and one path.* Every
    clause that survives this is already a scenario in the first requirement. The
    THEN clause is "The same root, Stoa and path always yield the same identity",
    and "at any time" is "A derived identity survives storage and reload". It
    could not fail unless those also failed. It would also check derivation
    determinism, which belongs to the first requirement, under a requirement about
    rotation. Taking it would amount to deleting the scenario under another
    name.
- **The old THEN clause is dropped.** "The result is the same key" duplicates
  "The same root and Stoa always yield the same identity" in *A user has one
  identity per Stoa, and it is permanent*. The new scenario compares the identity
  in use before and after an operation. That is a different observation from
  repeated derivation, so it does not bring the duplicate back.
- **The requirement's prose gains one paragraph** saying that in this release the
  key behind the identity in use is the machine key, and that asking for a master
  key while one is held MUST leave it unchanged. Without it, the new scenario
  would test something the requirement text does not state. The rest of the
  requirement and its second scenario are carried unchanged.
- **The requirement is renamed, because the tool leaves no other way to drop the
  scenario.** `openspec` 1.13.0 refuses a `MODIFIED` block that omits a scenario
  the live spec still has ("archive refuses to drop them"). It also refuses
  `REMOVED` and `ADDED` under one name ("Requirement present in both ADDED and
  REMOVED"). Both were run against this change. That left two ways out. One was to
  keep the old scenario name on new content, which some specs here do. But this
  name, "An identity is a pure function of its root and its Stoa", states the
  false claim, so keeping it would leave the contradiction in the heading. The
  other, taken here, follows `join-preview-getstoa`: `REMOVED` with Reason and
  Migration, and the requirement `ADDED` as *Identity does not rotate: the key
  behind an identity is never replaced*. The name keeps "Identity does not
  rotate" as its prefix, so a search for the old name still finds it. No live
  spec, test or code cites either the old requirement name or the removed
  scenario name.
- **Both Purpose paragraphs are brought in line with 0.0.1**, and both keep what
  they say about per-Stoa identity. #108 (milestone 0.0.3) switches per-Stoa
  identity back on, and at that point these paragraphs become true in full again
  rather than needing to be rewritten back.
  - `identity`: unlinkability across Stoas becomes what the per-Stoa derivation
    provides once per-Stoa identity is restored, not a property of the identity in
    use today. "An address" becomes "a Stoa address", the only kind that remains.
  - `identity-onboarding`: a fresh install reaches posting by creating its machine
    key. The slate and the keep stay contracted and stay built, but no view reaches
    them, and a kept choice does not change the identity in use.

No identity behaviour and no key derivation changes. The operation the new
scenario names already reports an existing master key rather than replacing it.
The change is to the contract's text, plus the test the rewritten scenario needs.

### Which edits are direct, and which are delta

The closer's archive must not undo either kind. A reader diffing the promoted
spec against the delta must not mistake the direct edits for drift, which is the
`stoa-genesis` trap in `docs/OPENSPEC-ARCHIVE.md`.

| Edit | File | Kind | What archive does to it |
|---|---|---|---|
| Purpose of `identity` | `openspec/specs/identity/spec.md` | **direct**, in this change's commits | nothing. No delta names the Purpose, and a delta's Purpose would be ignored for an existing capability |
| Purpose of `identity-onboarding` | `openspec/specs/identity-onboarding/spec.md` | **direct**, in this change's commits | nothing. This change has no delta for `identity-onboarding` at all |
| *Identity does not rotate, and this is a contract not an omission* | `specs/identity/spec.md` in this change | **delta**, `REMOVED` | deletes that requirement block, scenarios included, from `openspec/specs/identity/spec.md` |
| *Identity does not rotate: the key behind an identity is never replaced*: the old text plus one paragraph, the rewritten scenario, and the second scenario carried verbatim | `specs/identity/spec.md` in this change | **delta**, `ADDED` | adds that block to `openspec/specs/identity/spec.md`, expected **at the end of the file** rather than where the removed one stood |

The two kinds cannot collide. `REMOVED` and `ADDED` act only on requirement
blocks, each running from its `### Requirement:` heading through its last
scenario. The Purpose sits above `## Requirements`, where no delta reaches.

**The direct edits must not also appear in the delta, and the delta's requirement
must not also be edited directly.** Either duplication would make archive either
a no-op or a silent revert of the direct edit. The live `identity` spec in this
change's commits still carries the old requirement. That is deliberate, because
`REMOVED` has to find it to remove it.

After archive, the promoted `identity` spec should differ from the tree before
archive in exactly two hunks. The old requirement block is gone from its position
between *Authenticity is not authority* and *A secret key cannot be copied…*.
The new block is expected as the last requirement in the file, after *In this
release one machine key is the identity in every Stoa*. That position was not
observed by running archive here, and it does not matter which position the
tool picks. The check is that the new block's text matches the delta byte for
byte and that nothing else moved. A moved position is not drift.

The unarchived `first-run-identity` change carries an `ADDED` delta for
`identity-onboarding`. That delta names no Purpose and does not touch this
change's requirement, so it does not matter which of the two archives first.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity`: *Identity does not rotate, and this is a contract not an
  omission* is removed, and replaced by *Identity does not rotate: the key
  behind an identity is never replaced*. The untestable, self-contradicting
  scenario becomes one that checks the requirement's own claim at a point where
  it can fail. The prose states what the key behind the identity in use is in
  this release. The Purpose paragraph is edited directly (see the table above).
- `identity-onboarding`: **Purpose paragraph only, edited directly.** No
  requirement changes, so there is no delta file for this capability.

## Impact

- `openspec/specs/identity/spec.md` and `openspec/specs/identity-onboarding/spec.md`:
  the two Purpose paragraphs, edited directly.
- `openspec/changes/identity-spec-coherence/specs/identity/spec.md`: the
  `REMOVED` and `ADDED` delta.
- Tests: the rewritten scenario needs a test that cites it. At the time of the
  issue, no test cited the old one. The behaviour it pins already exists: the
  wire method that creates a master key reports an existing key and writes
  nothing. So the test needs no core change to pass. It must still be shown to
  fail against a build whose master-key request replaces a held key.
- No code, wire shape or UI change.
