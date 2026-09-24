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
- [x] CI green, title/body checked, PR merged — `closer`

## Implementation

### 1. Make room in the op codec (refactor, no behaviour change)

- [x] 1.1 Split `Op::encode` (the encoding) from `Op::canonical_bytes` (the total layout id/sign/verify use), make `SignedOp::to_bytes` fallible, and give the SQLite log and `transport::publish` an `Unencodable` refusal. Verify: `cargo test -p dialectica -p dialectica-core` passes unchanged in count.

### 2. The blank title in the core

- [x] 2.1 Add `stoa::BLANK_CHARACTERS` (30 code points) and `stoa::is_blank_title`. Verify: a test pins every member, the empty title, and U+200E and U+180E as not blank.
- [x] 2.2 Refuse a blank genesis title on encode and decode as `GenesisError::BlankTitle`, one failure for every blank title, after the structural checks. Verify: `stoa.rs` tests for the empty title, each of the 30 alone, the mixed blank title, and a visible letter among blank characters round-tripping unchanged.
- [x] 2.3 Refuse a blank metadata-op title through one guard, `Op::check_admitted`, called by `encode` and `decode`, as `OpError::BlankTitle`; an empty or blank description stays valid. Verify: `op.rs` tests on both sides, plus a transport test refusing one from a peer.
- [x] 2.4 Add the resolver's fourth binding condition in `stoa_metadata::binding_metadata`. Verify: a creator-signed blank-title op held in a `MemoryOpLog` falls back, and a later one does not displace a binding op; the empty-title-binds test is inverted.
- [x] 2.5 Confirm create, join, `getStoa` and the listing refuse a blank title through the codec, with reasons naming it blank, and correct the wire doc comments claiming an empty title is accepted. Verify: wire tests for each path, and the SQLite log refusing to store an unencodable op.

### 3. The view

- [x] 3.1 Add `Core.getStoa`, `Core.isBlankTitle` (same 30 code points) and `Core.stoaMetadataFrom`, treating a misshapen or blank-titled reply as a failure. Verify: QML spec over the normaliser, including all 30 blank characters and U+200E/U+180E.
- [x] 3.2 In `DJoinScreen`, look up the reference on screen, keyed to that reference; derive `foundingTitle` from the join or a fallback lookup and `currentTitle`/description from a non-fallback lookup; render the fallback note, the lookup failure panel and the rewritten no-founding-title note; correct the `currentTitle` comment. Verify: `tst_stoa_screens.qml` specs for fallback, non-fallback, description, failure, malformed and blank replies, and a second reference.
- [x] 3.3 Keep the lookalike comparison over founding titles only and treat a blank join title as unavailable. Verify: QML specs that a current title is not compared and a blank title matches nothing.
- [x] 3.4 Update the specs that pinned the superseded behaviour (no call at preview, an empty title accepted at creation) and correct `DStoaListScreen`'s empty-title comment. Verify: `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml` passes.

### 4. Gates

- [x] 4.1 `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`, `cargo clippy … -D warnings` and the workspace `cargo fmt --check` CI runs pass. A direct `fmt --check` on `dialectica-core` reports only drift that is on `main` already (`identity.rs`, and two `wire.rs` test sites: one from `get-stoa`, one from `machine-identity-scope`).
- [x] 4.2 `nix build ./dialectica#lgx` succeeds. The flake is under `dialectica/`, and there is none at the repository root.
- [x] 4.3 Every QML spec passes through `run-qml-tests.sh`, and the `check_qml_members`, `check_qml_names` and `check_qml_reachable` gates and qmllint are clean.
