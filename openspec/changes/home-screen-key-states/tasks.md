## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

### 1. The keystore opener (refactor, no behaviour change)

- [x] 1.1 Add `keystore::open_from_env_with_protection`, returning the key and
      the file's protection from one read, and route `open_from_env` through
      it. Verified: the full `cargo test` suite passes unchanged on the
      refactor commit alone.
- [x] 1.2 Pin its protection reporting in the one test allowed to set
      `DIALECTICA_PASSPHRASE`: encrypted file reported `true`, plain file
      reported `false` while a passphrase is set. Verified:
      `the_environment_decides_the_unlock_only_after_the_file_has_spoken`.

### 2. The core query (`identity-onboarding`)

- [x] 2.1 `wire::get_master_key` and `master_key_from`, with the two-arm
      `MasterKey` reply. Only `NotFound` maps to no key held, and every other
      keystore error is the error shape. Verified by the `wire.rs` tests under
      "Asking whether a master key is held", including the two mutations
      design.md Decisions 2 and 3 name.
- [x] 2.2 Add it to `every_request_taking_method`, `NO_REQUIRED_FIELD` and
      `a_served_request`. Verified: the envelope sweeps and
      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
      pass.
- [x] 2.3 The trait method and the adapter forward in `rust-lib/src/lib.rs`.
      Verified: `lgs basecamp build` builds both modules from this worktree.
- [x] 2.4 Export from `dialectica-core`'s `lib.rs`. Verified: the adapter
      compiles under `lgs basecamp build`.

### 3. The view (`stoa-navigation-view`)

- [x] 3.1 `Core.getMasterKey()`, the one wrapper. Verified:
      `test_each_core_method_is_named_once_and_reached_through_the_wrapper`.
- [x] 3.2 `machineKey`, one value with three states, set only from a reply in
      this run (`askKeyState`, `createMachineKey`), with strict `=== true` /
      `=== false`. Verified: the key-state tests in `tst_stoa_screens.qml`.
- [x] 3.3 Ask on arrival and on every later showing (`onVisibleChanged`).
      Verified: `test_the_key_state_is_asked_again_on_each_showing` and
      `test_returning_home_from_a_feed_asks_the_key_state_again`.
- [x] 3.4 Key block, could-not-be-read block, create affordance and key line
      as `Loader`s active on one state each. The paste section sits outside
      every Loader. Verified: the not-instantiated tests, which walk invisible
      elements too.
- [x] 3.5 Verbatim `homeMachineKey` copy, including the heading and the
      placeholder, plus the conditional unencrypted warning. Verified: the copy
      tests and the three protection tests.
- [x] 3.6 Invert or replace the existing tests that pinned the removed
      behaviour, and add `get_master_key` to every creation fixture. Verified:
      `tst_stoa_screens.qml` and the full QML suite pass.
- [x] 3.7 Refactor, no behaviour change: carry a refused mint as `refusal`
      inside the no-key value instead of a separate property cleared on each
      ask (design.md Decision 13). Verified:
      `test_a_mint_failure_does_not_outlive_the_showing_it_happened_in`, now
      citing "A refused mint is not rendered on a later showing", and the
      mutation Decision 13 names.
- [x] 3.8 The could-not-be-read state's verbatim statement and its
      "Try reading the key again" action, which calls `askKeyState()` from
      inside that state's Loader (design.md Decision 14). Verified: the five
      tests under "could not be read: what failed, and reading the key again",
      and the mutation Decision 14 names.

### 4. Gates

- [x] 4.1 `cargo fmt`, `cargo clippy -D warnings` and `cargo test` over
      `-p dialectica -p dialectica-core`.
- [x] 4.2 `check_qml_names.py`, `check_qml_members.sh`,
      `check_qml_reachable.py`, and qmllint over every `src/qml` file with CI's
      flags.
- [x] 4.3 The full QML suite through `run-qml-tests.sh`.

### Satisfied by construction

- "Asking writes nothing": `get_master_key` reaches only a read.
  `open_from_env_with_protection` calls `read_checked`, `protection_of`,
  `unlock_for` and `from_file_bytes`, and none of them writes. The tests assert
  the observable half: the directory stays empty, and the keystore is unchanged
  byte for byte.
- "The protection reported is not the caller's passphrase": the handler has no
  `unlock` parameter through which one could arrive. See design.md Decision 4.
