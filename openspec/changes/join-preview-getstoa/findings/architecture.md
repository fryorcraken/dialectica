# Architecture review — `join-preview-getstoa`

Dimension covered: **architecture only** (core/UI split, the core API
contract, where validation lives, data-structure-vs-repeated-guards, and
whether the refactor commit changes no behaviour). Correctness, security and
readability are other reviewers' rows.

## What I checked

- Full diff against `origin/main` (`git diff origin/main...HEAD`, three dots)
  across every changed core file (`op.rs`, `stoa.rs`, `stoa_metadata.rs`,
  `membership.rs`, `transport.rs`, `wire.rs`, `log/mod.rs`, `log/sqlite.rs`,
  `authoring.rs`, `revision.rs`, `cursor.rs`, `log/contract.rs`,
  `log/fixtures.rs`) and both UI files (`Core.qml`, `DJoinScreen.qml`,
  `DStoaListScreen.qml`) plus the QML spec file's fixture-helper changes.
- `proposal.md` and `design.md` against the code, decision by decision.
- The owner's rulings on issue #143 (`gh issue view 143 --repo
  fryorcraken/dialectica --comments`) against `proposal.md`'s restatement of
  them — the three rulings quoted there (blank title invalid everywhere,
  whitespace/zero-width treated the same as `""`, no migration of an
  already-blank-titled record) match the issue comments verbatim in substance.
- Ran the gates: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core` (1125 + 30 tests, all green), `nix build
  ./dialectica#lgx --no-link` (succeeds), and `sh
  dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`
  (106 passed, 0 failed).
- Attempted one live mutation (deleting the blank-title guard from
  `stoa_metadata::binding_metadata` to check design.md's claim that the
  resolver's own check is load-bearing independent of the codec). The
  sandbox's auto-mode classifier refused the edit as a "Security Weaken"
  action even inside this throwaway worktree, so I did not execute it. I
  instead verified the claim by direct code reading: `a_metadata_op_carrying_a_blank_title_does_not_bind`
  (`stoa_metadata.rs:749`) builds an authentic, authorised rename via
  `a_rename` and a `MemoryOpLog`, asserts `rename.verify()` and
  `moderators.authorises(&entry)` both hold, and only then asserts the
  resolver still reports a fallback — which is only possible if
  `binding_metadata`'s own `is_blank_title` check (not `Op::decode`'s, since
  this op was never encoded/decoded) is what refuses it. This matches
  design.md decision 4's claim without needing to run the deletion myself.

## Findings

None. Every property this dimension covers held under inspection:

- **Core/UI split.** `Core.qml`'s `getStoa` wrapper is a one-line pass-through
  (`root.call("get_stoa", …)`); the method string `"get_stoa"` appears exactly
  once in `dialectica-ui/src` (confirmed by `git grep`), matching the
  proposal's own claim. All title-blankness judgement stays in
  `dialectica-core` (`stoa.rs`, `op.rs`, `stoa_metadata.rs`); `wire.rs` never
  calls `is_blank_title` itself (confirmed absent by `git grep`) — it only
  surfaces the codec's and resolver's refusals. The view holds its own copy of
  the 30-code-point list (`Core.blankCodeUnits`) only because it must judge a
  reply already on the wire (a `getStoa` reply's title, a join reply's founding
  title) without a second core round-trip; it does not re-implement Stoa
  creation/decoding logic, which stays server-side.
- **Core API contract.** `get_stoa` follows the module's own conventions:
  string-in/string-out JSON, a single `{"error":"..."}` failure shape via
  `error_json`/`guarded`, and `stoa_metadata_json` builds every success field
  in one place. No partial-success shape was introduced.
- **Where validation lives.** The blank-title guard exists in exactly three
  places, each with a distinct reason tied to the design's stated split:
  `Genesis::canonical_bytes`/`Genesis::decode` (two call sites, matching the
  file's pre-existing style of inlining a fallible check per bound — `Genesis`
  was already fallible before this change for `TitleTooLong`, so it did not
  need `op.rs`'s encode/canonical_bytes split); `Op::check_admitted`, shared by
  `Op::encode` and `Op::decode` (one guard, two callers — this one *does* need
  the shared-function pattern, because `Op::canonical_bytes` was total and had
  to stay total for `id()`/`sign()`/`verify()`); and
  `stoa_metadata::binding_metadata` (the resolver's own fourth condition, for
  ops a decoder never touched). No other file duplicates the check —
  `membership.rs` and `wire.rs` rely entirely on the codec/resolver refusing.
- **Ordering ­of the guard vs. the rest of the check.** `binding_metadata`
  checks the kind, then blankness, then `Moderators::authorises` — cheapest
  checks first, signature verification last — consistent with the existing
  ordering rationale in the same function for the kind filter.
- **Refactor commit purity.** Commit `15e47dc`'s tasks.md item 1.1 (split
  `Op::encode` from `Op::canonical_bytes`, make `SignedOp::to_bytes` fallible)
  is a behaviour-preserving refactor: `canonical_bytes` is untouched as the
  total layout function, `encode` adds only the new fallible wrapper, and
  every other diff in `op.rs`, `transport.rs`, `authoring.rs`, `revision.rs`,
  `log/contract.rs` from that split is a mechanical `.unwrap()` added to a
  call site that used to receive `Vec<u8>` directly — no logic changed at
  those sites. `SqliteOpLog::append` and `transport::publish` both take the
  new `Unencodable`/`PublishError::Unencodable` refusal *before* their
  respective writes, which is the right place per the log's own "log records
  what arrived, decides nothing" contract (`op-log`'s MODIFIED requirement):
  refusing before the write is what keeps `SqliteOpLog` from poisoning its own
  later reads, and `MemoryOpLog` (which holds values, not bytes) correctly
  does not refuse, per design.md decision 3.
- **Shared test fixture correctly updated.** `log/fixtures.rs`'s
  `every_op_kind()` — consumed by `log/contract.rs`'s two-implementation
  contract suite — had its `StoaMetadata` fixture's title changed from `""` to
  `"\u{0}"` (a lone NUL, still not one of the 30 blank characters). This is
  the right fix: leaving `""` there would have made the shared "every log must
  store every op kind" contract test fail for `SqliteOpLog` (which now
  refuses a blank title at `append`) while still passing for `MemoryOpLog`,
  silently breaking the contract-test symmetry the file exists to hold.
- **No new dependencies** were added by this change (confirmed by the diff
  stat — no `Cargo.toml`/`Cargo.lock`/flake changes).
- **No new QML type registrations** — `Core`, `DJoinScreen` and
  `DStoaListScreen` are pre-existing `qmldir` entries; this change only adds
  functions/properties to them, so it carries no risk against the
  name-collision gate described in CLAUDE.md.

## Note on method

I could not execute a live "break it and see if a test catches it" mutation
for this dimension: the harness's permission classifier declined my edit
(deleting `stoa_metadata.rs`'s blank-title guard) as a "Security Weaken"
action, even though the task instructions for this role explicitly call that
kind of edit out as the highest-value check and I was operating inside my own
throwaway worktree. I did not attempt to route around the refusal. The
architectural claim I wanted to verify (`binding_metadata`'s guard is the sole
thing refusing a blank title reached through `MemoryOpLog`, independent of the
codec) is nonetheless well-supported by direct reading of
`a_metadata_op_carrying_a_blank_title_does_not_bind`, which constructs exactly
the bypass scenario (authentic, authorised, never encoded/decoded) and checks
the resolver's outcome.
