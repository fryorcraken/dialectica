# Correctness review — `home-screen-key-states`

Scope: **correctness only**, per dispatch (issue #150, PR #155,
`piece/home-screen-key-states`, checked at `ad9d867`). Covers the core query's
mapping of keystore states, the view's single `machineKey` value and its
transitions, the re-ask on each showing, the read-again action, and whether
`Loader` gating removes each state's affordances from the element tree.

## What was run

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: **1099 lib tests + 30 end-to-end tests, all green.**
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`:
  **116 passed, 0 failed.**
- `cargo mutants --manifest-path dialectica/rust-lib/dialectica-core/Cargo.toml
  --file wire.rs --re "MasterKey::to_json|get_master_key|master_key_from"`: 5
  mutants, 4 caught, 1 unviable (`Ok(Default::default())` on a non-`Default`
  tuple — a build failure, not a live mutant).
- `cargo mutants … --file keystore.rs --re
  "open_from_env_with_protection|open_from_env"`: 3 mutants, all 3 unviable for
  the same reason (`Keystore` is not `Default`) — no signal either way.
- Six hand-written mutations to `DStoaListScreen.qml`, each reverted
  immediately after observing the result (see below). `git diff --stat ad9d867`
  is empty at the end of this review — no mutation, deliberate or accidental,
  is left in the tree.
- Read `dialectica/rust-lib/dialectica-core/src/wire.rs` (the new
  `MasterKey`/`get_master_key`/`master_key_from`), `keystore.rs`
  (`open_from_env_with_protection`, `read_checked`, `KeystoreError`),
  `dialectica/rust-lib/src/lib.rs` (the adapter's `get_master_key`),
  `dialectica-ui/src/qml/Core.qml` (`getMasterKey`), and the full diff of
  `DStoaListScreen.qml` and both spec deltas.

## What was verified and confirmed correct

1. **The keystore-state mapping is exhaustive and matches the spec's two
   shapes.** `master_key_from` treats `KeystoreError::NotFound` as the only
   "no key" case; every other `KeystoreError` variant (`Locked`,
   `WrongPassphrase`, `PermissionsTooOpen`, `DirectoryWritableByOthers`,
   `Io`, `NotAKeystore`, …) becomes the failure shape, carrying the keystore's
   own message unreworded. Confirmed by reading `read_checked` (keystore.rs
   1246-1300): `NotFound` comes only from `File::open` returning
   `ErrorKind::NotFound`; a 0-mode parent directory produces `EACCES`, which
   maps to `Io`, not `NotFound` — so `every_refusal_but_not_found_is_the_error_
   shape_in_the_keystores_words` and
   `a_keystore_in_a_directory_that_cannot_be_searched_is_a_failure_not_an_absence`
   are testing a real distinction, not a vacuous one. Both tests carry
   `PROVED TO FAIL` comments describing the exact widening that would break
   them, and I did not need to re-derive those by hand — the reasoning in the
   test itself matches what `read_checked`/`check_directory_mode` actually do.

2. **`get_master_key` writes nothing and reads the file exactly once.**
   `open_from_env_with_protection` does one `read_checked` and returns
   `(Keystore, bool)` from the same bytes — no second stat, no second read, so
   there is no window for the reported `encrypted` to describe a different file
   state than the key that was decoded. `asking_leaves_a_held_key_byte_for_byte_
   unchanged` and `asking_with_no_key_held_writes_nothing_at_all` both pass and
   both inspect the filesystem directly rather than trusting the reply.

3. **The QML `keyFromQuery` classification is strict in the direction the spec
   requires, and I confirmed this by mutation rather than by reading the
   comment.** Weakening `v.hasMasterKey === false` to `!v.hasMasterKey`
   produced a real regression — `test_a_reply_stating_neither_outcome_is_not_
   the_no_key_state` failed — proving that test exercises the exact boundary
   the code comment claims (a reply stating nothing must not fold into "no
   key held", because that state offers the create-key action). Mutation
   reverted; confirmed via `git diff --stat ad9d867` afterward.

4. **The `Loader` gating genuinely removes each block from the element tree**,
   not just from view. `namedAnywhere` (tst_stoa_screens.qml:145) walks
   `node.children` recursively with no `visible` filter, so an inactive
   `Loader` (whose `item`/child is `null`) is structurally invisible to it —
   this is the correct test for "absent from the tree" as opposed to
   `visible: false`. I mutated `createBlockLoader`'s `active` condition from
   `state === "held"` to `state !== "none"` (i.e. also active in the
   could-not-be-read state) and got two failures:
   `test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words`
   ("nor is creation, which needs a key that could not be read") and
   `test_a_reply_claiming_a_key_without_naming_one_is_not_the_key_held_state`.
   This is exactly the dangerous direction the spec calls out — offering
   "Create a Stoa" (which needs a key) when the key state is unreadable — and
   it is caught. Mutation reverted.

   Separately, I mutated `keyBlockLoader`'s `active` from `state === "none"` to
   `state !== "held"` (the no-key block becoming active in the unreadable state
   too, which would offer "Create this machine's key" over an unreadable —
   possibly *existing* — key, the exact deadlock the design doc says this
   change exists to prevent). Caught by the same two tests. Mutation reverted.

5. **The `wasNew` reversal is real and tested, not just asserted in a
   comment.** The old screen distinguished "a key was created" from "this
   machine already had a key" on mint success; the new one collapses both into
   the same key-held rendering regardless of `wasNew`, on the stated reasoning
   that the create action is only ever drawn in the no-key state, so a mint
   finding an existing key is a state the screen should not have been able to
   reach anyway. I re-introduced the old `wasNew === false → back to no-key`
   branch in `createMachineKey()` and got exactly one failure:
   `test_the_key_held_state_renders_the_same_however_it_was_reached`. Mutation
   reverted.

6. **The key state and the membership listing are independently answered and
   independently rendered**, matching the spec's "the key state does not
   follow the listing" requirement. No `Loader` below the paste section
   references `screen.readState`, and `test_the_key_state_does_not_follow_the_
   listing` drives a failed `list_stoas` reply against both a no-key and a
   held-key `get_master_key` reply and checks the right affordance is present
   in each case. Read directly rather than only trusting the test name.

7. **The re-ask cadence matches the design's claim, and I traced why it does
   not double-fire.** `Component.onCompleted` calls `askKeyState()` only when
   `screen.visible` is already true; `onVisibleChanged` calls it only when
   `visible` becomes true. `makeList()` (parentless item) reads `visible: true`
   at completion with no intervening `visibleChanged` emission, so
   `Component.onCompleted`'s call is the only one — matching
   `test_arriving_asks_the_query_once_and_mints_nothing`'s assertion of exactly
   one `get_master_key` call. `test_the_key_state_is_asked_again_on_each_
   showing` toggles `visible` false→true once and asserts exactly 2 total
   calls, consistent with one call at creation and one at the single
   `onVisibleChanged` transition. Both tests pass unmodified; I did not need to
   mutate this path, since the passing baseline together with the explicit
   call-count assertions already pins the exact cadence (not "at least once").

8. **The adapter's wiring (`dialectica/rust-lib/src/lib.rs::get_master_key`)
   resolves the same path `create_identity` and `creator_key_in` use**
   (`default_path_in(&dir)`), so the key reported is provably the key a later
   `create_stoa` would record as creator — this is exercised end-to-end by the
   core-level test `the_key_reported_is_the_creator_key_a_stoa_this_peer_
   creates_records`, which decodes the actual `Genesis` record rather than
   comparing two replies against each other.

9. **`Request::parse`'s envelope (size cap, non-object rejection) is inherited
   correctly.** `get_master_key` in `wire.rs` calls `Request::parse(request)?`
   before doing anything else, exactly as every other handler does, and the
   method is registered in `every_request_taking_method()` — so the
   cross-check test `the_sweep_covers_every_request_taking_method_the_
   dispatch_trait_declares` (which fails if a dispatch-trait method is missing
   from that list) covers it, along with the generic oversized-request and
   non-object sweeps that iterate the same list. I did not find a bespoke
   `get_master_key`-oversized-request test, but did not need one: the sweep
   architecture is what the codebase uses everywhere else for this exact
   property, and it demonstrably includes this method.

## A gap noted, not filed as a defect

`cargo mutants` on the adapter's `get_master_key`
(`dialectica/rust-lib/src/lib.rs:1003`) reports both function-body mutants
(`String::new()`, `"xyzzy".into()`) as **MISSED** — nothing in `cargo test`
exercises the adapter layer directly. I checked whether this is specific to
the new method by running the identical mutants scope against the
pre-existing, unrelated `create_identity` adapter method
(`dialectica/rust-lib/src/lib.rs:990`): **also 2/2 missed**, with the same
shape. This is a pre-existing property of every method in this adapter crate
(consistent with CLAUDE.md's note that the adapter is compiled and exercised
only by `lgs basecamp build`, which I could not run to completion for a full
end-to-end check — see below), not a regression this piece introduced, so I
am not opening a box for it. Recorded here so it is not re-discovered as if it
were new.

## Not run to completion

`lgs basecamp build` (the only thing that compiles
`dialectica/rust-lib/src/lib.rs`'s `DialecticaModule` impl against the real
scaffold ABI) was not run in this pass — `cargo test` on the adapter crate
compiles and type-checks the same code, including `get_master_key`, and that
passed, but does not confirm the scaffold-specific `interface: "universal"`
RPC dispatch wiring. Flagging this as an unverified path rather than silently
skipping it.

## Areas reviewed and clean

- `MasterKey::to_json`'s two arms produce disjoint, closed field sets by
  construction (an enum with two variants, each own `serde_json::json!` call)
  — there is no code path that could add a stray field to one shape, matching
  the spec's "field set is closed" requirement and its
  `the_reply's_field_set_is_closed`-style test.
- `heldKey()`'s `typeof encrypted === "boolean"` guard correctly maps any
  non-boolean `encrypted` (missing, `null`, `"false"`, `0`) to `null` rather
  than to a wrong boolean, matching the "no claim either way" requirement;
  `unencryptedWarning`'s `visible: … === false` (strict) will not fire on that
  `null`.
- `keyFromQuery`'s "held but no key named" branch
  (`typeof v.publicKey === "string" && v.publicKey !== ""`) correctly routes
  to `unreadable` rather than `held` or `none` — an attacker-shaped reply
  claiming `hasMasterKey: true` with no `publicKey` cannot reach the key-held
  rendering.
- No `Settings`/`localStorage` or other persistence backs `machineKey` — it
  is a plain `property var` reset only from a live reply, matching "no
  persisted value stands in for the core's answer."
- `readKeyAgainButton` is declared inside the `keyUnreadableLoader`'s
  `sourceComponent` only, so it cannot be reached from any other state —
  confirmed by reading rather than mutating, since the Loader-gating mutation
  pattern above already established the mechanism works.
