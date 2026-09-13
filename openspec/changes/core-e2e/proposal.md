## Why

Every test in `dialectica-core` was an in-crate `#[cfg(test)]` module, and two
properties of that are structural rather than incidental: such a test can reach
private items, and it mostly runs in memory. Together they leave two things
unchecked that no amount of unit testing reaches.

**The public API is unexercised as an API.** A test that borrows a `pub(crate)`
fixture or pokes a private field proves the mechanism works; it is no evidence
that an outside consumer can use the crate at all. A crate whose public surface
was missing a method entirely would pass the whole suite.

**"Rebuildable by replay" is a claim about a file**, and `SqliteOpLog::in_memory`
by construction cannot survive being dropped. Persistence assertions made against
an in-memory store are assertions about process memory wearing a persistence
vocabulary.

The third reason is this crate's own defect history: its expensive defects have
been **cross-layer**, living in the seam between encode and decode, between the
wire shape and the store, between memory and disk. A per-change suite cannot see
one, because each layer is correct in isolation.

## What Changes

- **A new integration target**, `dialectica-core/tests/end_to_end.rs`, importing
  `dialectica_core` as an outside consumer does and touching nothing private.
  Every store is a file on disk; every restart is a dropped connection and a
  reopened path.
- **The read path is covered from the JSON request a view sends** —
  `wire::list_threads_from_request` — down to the bytes in a SQLite file, plus
  the keystore file that mints the signing identity.
- **A live cross-layer defect on `main` is reproduced and documented**:
  `Op::canonical_bytes` is infallible and writes any length while `Op::decode`
  refuses a field over 150 KiB, so one byte over the cap signs, appends `Ok`, and
  poisons every subsequent ordered read of the store permanently. The test asserts
  the defect **as it stands**, because this change is not the one that fixes it.
- **CI's test-count gate is corrected.** Its `roots` list could not see an
  integration target, and the paragraph above it claimed a protection against a
  `#[test]` in `examples/` that a review measured as absent. One rglob with
  `target/` excluded replaces the list, and the claim becomes true.
- **No implementation file changes behaviour.** The only non-test edits are to
  `ci.yml`.

## Capabilities

### New Capabilities

None. This change adds no requirement to any capability: it asserts behaviour
that `openspec/specs/` already contracts — `op-log`'s persistence requirements,
`module-wire-contract`'s error shape, `moderation-resolution`'s authority rules —
from a vantage point the existing tests structurally could not occupy. A test
target is a way of checking a contract, not a clause in one.

`.openspec.yaml` therefore sets `skip_specs: true`. That is the opt-out for a
change with no spec-level behaviour change, and inventing a requirement to
satisfy validation would put a false clause in the contract.

### Modified Capabilities

None.

## Impact

- **New file:** `dialectica/rust-lib/dialectica-core/tests/end_to_end.rs`.
- **Modified:** `.github/workflows/ci.yml` — the test-count gate's declared-count
  logic and the comment that described it.
- **No new dependency.** The target reaches `rusqlite`, `serde_json` and `hex`
  directly, all already normal (not dev) dependencies of the crate, so there is
  no licence or maintenance question to answer.
- **The count gate's numbers move**, legitimately, whenever tests are added. The
  gate asserts `ran == declared` rather than a floor, so this is a property and
  not a figure to maintain.
- **One defect is now documented in a test and nowhere else.** The over-cap
  asymmetry has no issue and no PLAN.md entry; the test says so explicitly and
  names what should replace the paragraph once it is filed.
