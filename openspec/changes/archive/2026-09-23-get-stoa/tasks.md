# Tasks

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
- [x] findings all ticked, `findings/` deleted — `closer`
- [x] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

## 1. The resolver

- [x] 1.1 Widen `Moderators::authorises` to `pub(crate)` with no change to its body (design decision 2); verify `cargo test -p dialectica-core moderation` stays green.
- [x] 1.2 Add `stoa_metadata.rs` with `Founding::of`, `CurrentMetadata` and `resolve` over `OpLog::iter_stoa` (decisions 1, 3, 5, 6, 7); verify with resolver tests covering every scenario of "Current metadata resolves by last-write-wins, falling back to genesis", first run against a null implementation that always falls back.
- [x] 1.3 Pin the scope term with a log whose `iter_stoa` returns every op (decision 4); verify deleting the scope term from `authorises` turns exactly that test red.
- [x] 1.4 An adversarial log (forged, non-moderator, cross-Stoa, other kinds, `u64::MAX` and absent counters, maximum-length fields) resolves without panicking; verify with a resolver test.

## 2. The wire method

- [x] 2.1 Add `wire::get_stoa(request, store)`: envelope, `parse_stoa`, `genesis_for`, then the store, then `resolve` (decisions 8, 10); verify with wire tests for the reply shape, the fallback and renamed cases, a binding op carrying the founding title, unaltered display text, and no `foundingTitle` field.
- [x] 2.2 Every refusal in "`getStoa` refuses what it cannot answer" is the error shape with no success field beside it, including an unopenable store and an unreadable one; verify with wire tests.
- [x] 2.3 Add `get_stoa` to `every_request_taking_method` and `a_served_request`; verify the envelope sweeps (non-object, size cap, unrecognised field, served request) pass for it.
- [x] 2.4 Satisfied by construction, not by a test: "Asking about a Stoa does not join it" and "MUST NOT append or publish any op". `get_stoa` takes no membership store, never binds the log mutably, and hands the resolver `&L`, where `append` needs `&mut` (decision 9). A test listing Stoas before and after would pass for the null implementation.
- [x] 2.5 Re-export `get_stoa` from `dialectica-core`, and declare and forward it in the adapter trait (`dialectica/rust-lib/src/lib.rs`); verify `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares` passes and `nix build .#lgx` succeeds.

## 3. Stale claims

- [x] 3.1 Correct the doc comments that say nothing resolves metadata ops (`wire.rs`'s `FOUNDING_TITLE`, the adapter's `list_stoas` doc); verify `git grep -n "nothing resolves"` finds none under `dialectica/rust-lib`. `DJoinScreen.qml`'s matching comment is left for #143, which owns the view.
