# Design review — `home-screen-key-states`

No findings. `design.md`'s Decisions were checked against the code, against
GitHub issue #150, and against the bundle at
`tmp/ui-design/home-screen-handoff/handoff/` (`SPEC.md` "Home: the machine
key", `copy.json`'s `homeMachineKey` block). All three owner decisions named
in the task (1, 2, 10) are recorded faithfully and implemented as recorded.
No undocumented decision was found in the code, and nothing in `design.md`
contradicts the issue's stated scope.

## What was checked

- **Decision 1** (widen the core API with `get_master_key` rather than mock
  `hasMachineKey` in the view). `wire.rs` adds `get_master_key` /
  `master_key_from`, wired through `every_request_taking_method`,
  `NO_REQUIRED_FIELD` and `a_served_request` exactly as the entry describes.
  `rust-lib/src/lib.rs` adds the trait method and a one-line adapter forward.
  `Core.qml` adds `getMasterKey()`. All present; `cargo test -p dialectica -p
  dialectica-core` passes (1099 + 30 tests), and `lgs basecamp build` builds
  both modules from this worktree, so the adapter — compiled only under that
  build — compiles too.
- **Decision 2** (an unreadable keystore is the error shape, not
  `hasMasterKey:false` plus a reason, departing from the `who_am_i` /
  `get_capabilities` probe precedent on purpose). `master_key_from` maps only
  `KeystoreError::NotFound` to `MasterKey::NotHeld`; every other
  `KeystoreError` passes through as `Err`, which `get_master_key` turns into
  the `{"error":…}` shape. Confirmed against the wire.rs tests
  `every_refusal_but_not_found_is_the_error_shape_in_the_keystores_words`,
  `a_key_whose_permissions_are_too_open_is_a_failure_not_an_absence`, and
  `a_keystore_that_is_not_a_keystore_is_a_failure_not_an_absence`, all
  passing. On the view side, `Core.qml`'s `getMasterKey()` and
  `DStoaListScreen.qml`'s `keyFromQuery()` read `reply.ok` first, so a
  refusal always reaches the could-not-be-read state and never the no-key
  state — the trap the entry describes (mint refuses a key the user already
  holds) cannot occur through this path.
- **Decision 10** (the unencrypted warning is conditional on the reply,
  where the bundle draws it unconditionally — an owner departure from the
  bundle, recorded as such). `DStoaListScreen.qml`'s `unencryptedWarning`
  Text has `visible: screen.machineKey.encrypted === false`; `heldKey()`
  stores anything not a boolean as `null`, so a missing field renders no
  claim either way. `test_an_omitted_protection_field_produces_no_claim`-
  equivalent scenarios (the "protection" tests in `tst_stoa_screens.qml`)
  pass, and this matches `stoa-navigation-view`'s "With a key held…" spec
  requirement, which states the same three-way rule (warn / no warn / no
  claim) — so the departure from the bundle's unconditional warning is
  written into both `design.md` and the spec, not just asserted in one
  place.

Also checked, per this reviewer's remit (decisions recorded vs. code, and
vs. the issue), beyond the three named:

- Decisions 3, 4, 5, 7, 8, 9, 11, 12, 13, 14 all have code at the cited
  location matching the recorded choice, and every entry that describes a
  guard states what mutation was used to prove it (Decisions 2, 3, 6, 7, 8,
  13, 14 each name a specific mutation and its red test).
- Decision 9's claim that the view never reads `wasNew` was checked with
  `git grep -n -F -e "wasNew" -- dialectica-ui/src/qml/DStoaListScreen.qml`:
  the only occurrence is in a comment.
- Decision 11's component-mapping table was checked against the tree:
  `DStatusBar` is mounted once in `Main.qml` outside any screen's `visible:`
  binding (confirmed by `git grep`); no `DKeyLine.qml` file exists (the
  rejected-alternative claim); `DIdentityChip` is referenced only in a
  removal-rationale comment in `DStoaListScreen.qml`, never instantiated on
  this screen.
- The issue (`gh issue view 150`) was read fresh rather than from the brief.
  Its "What this needs" and the CLAUDE.md constraints section match
  `design.md`'s Decisions 1, 6, 7, 11 point for point — the one-flag
  precedent from `FeedScreen.qml`/`hasIdentity`, the `D`-prefix requirement,
  the "not drawn, not disabled" rule, and the bundle-to-repo component
  mapping. Nothing in the issue's current text is contradicted by
  `design.md` without justification; the one BREAKING departure from the
  existing `stoa-navigation-view` spec ("always offered") is argued in both
  the proposal and the spec delta's `## REMOVED Requirements` reason/
  migration text, which is the issue's own reasoning (the "hiding it would
  be hiding it on a guess" problem) carried into `design.md` and the spec
  rather than left only in the issue.

## Gates run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
  -p dialectica-core` — 1099 unit tests + 30 end-to-end tests, all passing.
- `sh dialectica-ui/tests/run-qml-tests.sh
  dialectica-ui/tests/tst_stoa_screens.qml` — 116 passed, 0 failed.
- `lgs basecamp build`, plainly from the worktree root — both `dialectica`
  and `dialectica-ui` build under `lgx` and `lgx-portable`.
- `dialectica-ui/tests/check_qml_names.py` — ok, 53 files, 25 `qmldir`
  entries, every type `D`-prefixed or grandfathered.
- `sh dialectica-ui/tests/check_qml_members.sh` — ok, 26 files checked.

A fresh SDK symlink (`dialectica/logos-rust-sdk-src`) was created before
building, per the task's instruction; it is gitignored and not part of this
commit.

No findings to report.
