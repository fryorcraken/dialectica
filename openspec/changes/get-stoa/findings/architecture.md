# Architecture review — `get-stoa`

Dimension covered: **architecture only** (correctness, security and
readability are separate reviewer passes; not duplicated here — see their own
findings files under this directory).

No findings. This change is architecturally sound; nothing below is a box to
tick.

## Scope read

`dialectica/rust-lib/dialectica-core/src/stoa_metadata.rs` (new, in full,
including its test module), the `wire.rs` diff (`get_stoa`,
`stoa_metadata_json`, `FOUNDING_TITLE`'s doc, the sweep entry, the associated
tests), `moderation.rs`'s one-line visibility change and its surrounding doc,
`identity.rs`'s diff (case-insensitive parsing test), `dialectica-core/src/lib.rs`'s
module declaration and re-export list, `dialectica/rust-lib/src/lib.rs`'s
`DialecticaModule::get_stoa` trait declaration and adapter forward, and the
four spec deltas under `openspec/changes/get-stoa/specs/` for boundary
consistency with `proposal.md` and `design.md`.

## What I checked and how

**Module boundary and dependency direction.** `stoa_metadata.rs` depends on
`moderation::Moderators` (for `authorises` and the moderator set) and nothing
else project-specific; `moderation.rs` has no code dependency back —
`git grep -n -F "stoa_metadata::" -- dialectica/rust-lib/dialectica-core/src`
shows the only appearance in `moderation.rs` is a doc comment explaining why
`authorises` became `pub(crate)`. The layering is `log` → `moderation` →
`stoa_metadata` → `wire`, with no cycle. This matches the "moderation resolver
with a different subject" framing the module doc states and CLAUDE.md's "put
the complexity in the data structure" principle: `stoa_metadata::resolve`
reuses `Moderators::authorises` rather than re-deriving the three-check
conjunction, which is the shape the file's own doc says the *next* resolver
should follow — and it followed it.

**Wire-layer shape versus existing precedent.** `get_stoa`'s signature
(`request: &str, store: impl FnOnce() -> Result<L, OpLogError>`) is not a new
shape — it is `list_threads_from_request`'s shape, reused verbatim (parse →
`parse_stoa` → `genesis_for` → open store → resolve), because `get_stoa` has
no adapter-level log to receive the way `list_threads`/`list_threads_inner`
does. Confirmed by reading both call sites (`wire.rs:1557` and `wire.rs:2207`)
side by side rather than trusting `design.md`'s characterisation.

**A property I checked for divergence and found none.** `list_threads_inner`
carries an explicit `genesis_address != stoa` re-check after `genesis_for`
(`wire.rs:1493-1499`) that `get_stoa` does not repeat. This looked at first
like a dropped guard, but `genesis_for` calls `Membership::verified`, whose own
doc (`membership.rs:105-135`) records that this exact duplicate check was
identified and removed by an earlier change ("two guards enforcing one
property, and no test distinguished them... deleting `genesis_for`'s left 550
of 550 passing"). `get_stoa` not re-adding it is not a regression; it is the
correct application of a lesson this codebase already learned once, and
re-adding it here would have been the actual finding.

**Dispatch-sweep integration (design.md decision 14).** Verified
`get_stoa` is declared on `DialecticaModule` (`rust-lib/src/lib.rs:234`),
forwarded by the adapter with no membership store
(`rust-lib/src/lib.rs:954-965`, matching `list_threads`'s pattern exactly), and
present in `every_request_taking_method`'s fixture vec
(`wire.rs:10488-10513`) with a served fixture that exercises the `Declared`
path (a renamed Stoa) rather than only the fallback, as design.md claims.

**Verify-before-open ordering, checked by mutation, not just by reading.**
`design.md` decision 10 and the code comment both assert genesis verification
happens before the store opens. I mutated `wire::get_stoa` to open the store
first (moving the `store()` call ahead of `genesis_for`), rebuilt, and ran
`a_request_that_fails_verification_never_opens_the_store` in isolation: it
failed immediately —
`{"genesis":"...","stoa":"..."} opened the store before being refused`. This
confirms the ordering property is a real, enforced architectural invariant and
not merely documented intent. The mutation was reverted immediately after;
`git status --porcelain` and `git diff --stat -- wire.rs` both show no diff
before this findings file was written.

**`cargo mutants`, scoped.** `--file dialectica-core/src/stoa_metadata.rs`:
12 mutants, 8 caught, 4 unviable (did not compile), 0 survived.
`--file dialectica-core/src/wire.rs --re "get_stoa|stoa_metadata_json"`:
4 mutants, all 4 caught, 0 survived. No architecture-relevant gap (e.g. an
unused parameter, a branch that could be deleted without changing behaviour)
turned up in either run. Both were run from this worktree with the gitignored
`logos-rust-sdk-src` symlink created for the build and not committed.

**Core-API-as-deliverable (CLAUDE.md's framing for this review).** `getStoa`
widens the wire contract by exactly one method, does not touch any existing
method's request or reply shape (`listStoas`/`createStoa`/`joinStoa` keep
`foundingTitle` untouched — confirmed by reading `stoa_reply` at
`wire.rs:2157-2169`, unchanged by this diff), and the reply shape it adds
(`stoa`, `title`, `description`, `policy`, `isGenesisFallback`) has no field
that duplicates information the caller already holds except by design
(`policy` reuses `policy_name`, shared with the membership calls, rather than
inventing a second spelling). The one deliberate omission — no `foundingTitle`
on this reply — is argued in design.md decision 11 and matches the owner's own
stated preference quoted there.

**Core/UI split respected.** No `dialectica-ui/` files are touched.
`stoa-navigation-view`'s spec delta restates behaviour purely in terms of the
new wire fields (`title`, `isGenesisFallback`) without describing or assuming
any QML wiring, and explicitly defers the view work to #143. This is the
correct shape for a change that widens the core contract without yet having a
consumer — architecture in the spec layer matches architecture in the code.

**One thing worth naming, not worth a box.** The readability reviewer's one
finding (`findings/readability.md`, addressed to `tester`) is about test-double
duplication (a fifth/sixth/seventh hand-rolled always-failing `OpLog` stub
across the crate). I read that finding and agree with its severity assessment
(minor, no behavioural risk) — it is a test-file maintainability observation
rather than a production-code architecture problem, so I am not duplicating it
here under a different dimension. `stoa_metadata.rs`'s own two stubs
(`UnscopedLog`, `UnreadableLog`) are each purpose-built to isolate one specific
check (the scope term, the store-failure path) and are not interchangeable
with each other or with `wire.rs`'s stubs, so extracting a shared fake would
need to preserve that per-stub specificity rather than simply merging bodies.

## Areas checked and clean

- **Single responsibility per module.** `stoa_metadata.rs` decides what a
  Stoa is called; `moderation.rs` decides what is rendered; `wire.rs` parses,
  verifies and shapes the reply. No function found doing two of these jobs.
- **No new dependency.** This change adds no crate to `Cargo.toml`.
- **No hand-maintained sweep list newly at risk.** The dispatch-completeness
  gate is `every_request_taking_method`, a test-enforced sweep, not the
  `pub use` convenience list in `dialectica-core/src/lib.rs` (which is
  documented as exactly that — a convenience, not a gate — in the comment
  immediately above it). `get_stoa` is correctly present in both.
- **CI gates.** No source file was moved or renamed by this change, so the
  fmt-gate and layout-derived CI gates CLAUDE.md warns about continue to see
  what they saw before.

## Not reviewed here (other dimensions' lanes)

Whether the resolver's ordering/binding logic is correct on adversarial input,
whether any check can be bypassed via a different call path, and comment/name
quality are covered by the correctness, security and readability passes
respectively (all already committed to this tree, all with no or one minor
finding).
