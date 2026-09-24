# Security review — identity-spec-coherence

Dimension reviewed: **security only** (per dispatch prompt). Correctness,
readability and architecture are covered by other `code-reviewer` instances.

## Scope actually touched

`git diff origin/main...HEAD` (three dots) is a spec/design/proposal/tasks
change plus one new test in `dialectica-core/src/wire.rs`. No production code
changes (confirmed: the diff touches only `mod tests`). No new dependency, no
new wire method, no new network-facing surface.

## What I checked

- Read `proposal.md`, `design.md`, `tasks.md`, the `identity` delta, and the
  direct Purpose edits to `openspec/specs/identity/spec.md` and
  `openspec/specs/identity-onboarding/spec.md`.
- Cross-checked design.md's factual claims against the actual code rather than
  trusting the prose:
  - `mint_master_key`'s `keystore_path.exists()` early-return (`wire.rs:1331`)
    and `Keystore::create`'s `AlreadyExists` refusal (`keystore.rs:940`) both
    exist exactly as described, and are layered as claimed (deleting the first
    alone does not let the second write over a held key — the mint just returns
    the error shape).
  - `git grep -n "write_to(" dialectica/rust-lib` and
    `git grep -n "\.create(" dialectica/rust-lib` confirm the claim that
    outside tests, the replacing write (`write_to`) is called only from inside
    `Keystore::create`, and `.create(` is called only from `mint_master_key`
    (`wire.rs:1349`) and `keep_selection` (`wire.rs:982`), plus the `seed_store`
    example (a dev tool, not shipped). No other production path can write over
    a held keystore file.
  - The new scenario's second observation (a published post carries the same
    key) is not vacuous: `authoring::post` sets `author: who.key.public_key()`
    (`authoring.rs:331`), i.e. the author field is derived from the actual
    signing key used, not an independently caller-set field. The test's
    `entry.op.op.author.to_hex() == before_key` assertion genuinely proves the
    op was signed by the key that was reported, not merely that a field says
    so.
  - `who_am_i` and `publishing_key`/`publishing` both resolve the identity by
    opening the keystore fresh (the test's `open` closure reads the file via
    `Keystore::open`), matching design.md's claim that neither path is served
    from a cached value.
  - The cross-reference "the operation that creates a master key" (used in the
    new requirement text) is an existing, unambiguous phrase already used in
    `identity-onboarding/spec.md:806`, contrasted there with the read-only
    "whether this peer holds a master key" operation — the new requirement's
    disambiguation note is not inventing a term, it is citing one that already
    exists in the sibling spec.
- Independently reproduced the tester's claimed mutation rather than trusting
  `tasks.md`'s report of it: edited `mint_master_key`'s `exists()` branch to
  generate a fresh keystore and `write_to` it (bypassing `Keystore::create`'s
  guard) instead of reading the existing key back, and ran only the new test.
  It failed exactly as `tasks.md` describes — `left: "f7a8…"` (the original
  key) vs `right: "5e3c…"` (the freshly generated one) — confirming the
  scenario is not satisfied by a build that silently rotates the key. Reverted
  the edit immediately after; `git diff origin/main...HEAD -- dialectica/rust-lib/dialectica-core/src/wire.rs`
  now shows only the test addition, matching the piece's own diff with nothing
  left over.
- Ran the full suite after restoring
  (`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`):
  1142 + 30 passed, 0 failed, matching the count `tasks.md` §2.4 reports.
- Checked whether the Purpose-paragraph rewrites understate or overstate a
  security-relevant property. They do neither: the old `identity` Purpose
  claimed unlinkability across Stoas as a present-tense property, which is
  false in this release (one machine key is used everywhere); the new text
  states plainly that unlinkability is suspended and the same user is visibly
  one key across Stoas. That is a documentation fix in the safer direction —
  it removes an overclaim rather than introducing one — not a new disclosure
  risk.

## Findings

None. This piece changes no production code path, and the one thing it adds
(a test) checks a real, non-vacuous security property (the key an identity
reports and the key an op is signed with cannot be silently swapped by
re-running the key-creation call) against the actual guards in the code, which
I verified both by reading and by an independent mutation. The requirement's
broader SHALL ("no way to replace the key behind an identity while keeping the
identity") is only witnessed by one named operation, but that narrowing is
explicitly disclosed as a trade-off in `design.md`'s Risks section rather than
hidden, and a second key-writing operation would need — and, per the same
section, is expected to get — its own scenario. I do not think that residual
gap needs a checkbox here: it is a known, stated scope boundary of this piece
rather than a defect introduced by it.

No findings to tick.
