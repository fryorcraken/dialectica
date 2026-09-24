# Architecture review — `home-screen-key-states`

Dimension covered: **architecture only** (per dispatch prompt). Correctness,
security and readability are separate reviewers' rows.

## What I checked

- The core API widening (`get_master_key`) against CLAUDE.md's "The core API
  is the deliverable": read `proposal.md` and `design.md` decisions 1–5 for
  the deliberateness of the widening, then the actual diff
  (`dialectica/rust-lib/dialectica-core/src/wire.rs`,
  `dialectica/rust-lib/dialectica-core/src/keystore.rs`,
  `dialectica/rust-lib/dialectica-core/src/lib.rs`,
  `dialectica/rust-lib/src/lib.rs`) against those decisions.
- The core/UI split: confirmed the QML view never reaches storage itself and
  only calls through `Core.getMasterKey()`.
- Complexity placement (data structure vs. logic): the `machineKey` value
  shape (`{state, …}`, three states) and the `Loader active:` pattern in
  `DStoaListScreen.qml`.
- The three hand-maintained request-sweep lists in `wire.rs`
  (`every_request_taking_method`, `NO_REQUIRED_FIELD`, `a_served_request`) —
  confirmed `get_master_key` was added to all three, and that a fourth
  mechanism (`the_sweep_covers_every_request_taking_method_the_dispatch_trait_
  declares`) derives the list from the dispatch trait and fails naming any
  method left out, so these lists are not purely hand-maintained-and-hopeful.
- `D`-prefix and `qmldir` conformance: confirmed no new QML type/file was
  added (`Core.qml` got one wrapper method, `DStoaListScreen.qml` — already
  `D`-prefixed and already registered — grew four `Loader` blocks with inline
  `sourceComponent`s, not new files), so no new `qmldir` entry was needed.
- New dependencies: none (`git diff` on both `Cargo.toml` files and
  `Cargo.lock` is empty).
- CI gate/source-layout drift: `.github/workflows/ci.yml` is untouched by
  this change; nothing moved or renamed.

## Verification performed

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core`: 1099 + 30 passed, 0 failed.
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`:
  116 passed, 0 failed. A bare `sh dialectica-ui/tests/run-qml-tests.sh` (no
  args) also ran the full 25-spec-file suite green, confirming no other
  screen regressed.
- `python3 dialectica-ui/tests/check_qml_names.py`: ok, 53 files / 25 qmldir
  entries, every declared type `D`-prefixed or grandfathered.
- `sh dialectica-ui/tests/check_qml_members.sh`: ok, 26 files, every member
  read off a known type exists.
- `lgs basecamp build` (plain, from the worktree root): succeeded — the
  adapter's new `get_master_key` method on `DialecticaModule` compiles under
  `cfg(logos_scaffold)`.
- `cargo mutants` scoped to the new code with `-F
  "get_master_key|master_key_from|MasterKey"` on `wire.rs`: 5 mutants found,
  4 caught, 1 unviable (a `Default`-derived stub that doesn't compile — a
  tool limitation, not a coverage gap). Scoped to
  `open_from_env_with_protection`/`open_from_env` on `keystore.rs`: 3 mutants
  found, all unviable for the same `Default` reason — `Keystore` has no
  `Default` impl, so `cargo mutants`'s stock replacement can't build. This is
  a known blind spot of the tool (per CLAUDE.md's own note that it "mutates
  functions, not `const` values"; here it also can't stub a type with no
  `Default`), not a claim that this function is untested — `design.md`
  documents four hand-measured "proved to fail" mutations against exactly
  these functions (widening `NotFound` to a catch-all, adding an `exists()`
  short-circuit, etc.), which is stronger evidence than a generic mutant run
  would have produced anyway.

## Findings

None risen to the level of a blocking architecture defect. Below is what I
looked at closely and why it is clean, not a padded checklist.

**The core API widening is deliberate and narrow, matching CLAUDE.md's "The
core API is the deliverable."** `get_master_key` follows the existing
conventions exactly: `std::string` in, `std::string` JSON out, failure is
always `{"error":"..."}` (never `{"hasMasterKey":false,"reason":…}` — decision
2 in `design.md` explains why that shape was rejected, and
`a_key_whose_permissions_are_too_open_is_a_failure_not_an_absence` /
`a_keystore_that_is_not_a_keystore_is_a_failure_not_an_absence` pin it). The
reply's field set is closed by construction through the two-arm `MasterKey`
enum (`Held { public_key, encrypted }` / `NotHeld`), which is exactly "put the
complexity in the data structure" — a `NotHeld` carrying a stray `publicKey`
or a `Held` missing one is a shape that cannot be built, not a shape a test
has to catch after the fact.

**The three hand-maintained sweep lists were all updated, and are not purely
hand-maintained.** `NO_REQUIRED_FIELD`, `every_request_taking_method`, and
`a_served_request` each gained a `get_master_key` entry
(`dialectica-core/src/wire.rs:10680`, `:10977`,
`sweep_dir_name("master")`/`master_m` near `:10870`). Unlike a plain
hand-maintained list with nothing to notice an omission (the pattern
CLAUDE.md's "hand-maintained sweep lists go stale silently" warns about), a
fourth mechanism — `the_sweep_covers_every_request_taking_method_the_dispatch_
trait_declares` — derives the expected method set from the `DialecticaModule`
trait declaration in `dialectica/rust-lib/src/lib.rs` and fails, naming the
missing method, if any dispatch-surface method is absent from
`every_request_taking_method`. This is the safety net the design doc's
"ADD YOUR METHOD HERE" comment describes, and it is real: `cargo test` passed
with `get_master_key` present in all three lists, which is consistent with
(not proof of, but consistent with) that check having been satisfied rather
than skipped.

**Core/UI split holds.** `dialectica-ui/src/qml/Core.qml`'s `getMasterKey()`
is a one-line forward (`root.call("get_master_key", [JSON.stringify({})])`),
matching every other wrapper in the file. `DStoaListScreen.qml` never touches
storage or the filesystem itself; it only normalises the JSON reply through
`keyFromQuery`. No network- or disk-touching logic leaked into the view.

**Complexity is in the data structure, not the logic, in the view too.** The
`machineKey` value (`{state:"none"|"held"|"unreadable", …}`) replaces what
would otherwise be a fourth ad-hoc boolean-flag combination (the trap
CLAUDE.md names directly: "the fourth slightly-different guard is a signal to
reshape"). Each of the four `Loader`s gates on exactly one string comparison
against `machineKey.state`; none stacks a second condition. This is a
reasoned application of "put the complexity in the data structure," not a
restatement of the code — `design.md` decision 6 explains why the two-outcome
fold `Core.qml`'s existing `identityFrom` uses was rejected for this
three-outcome question, and the two helpers are legitimately different shapes
for different questions rather than one being an unnecessary duplicate of the
other.

**`D`-prefix and `qmldir` conformance is clean because nothing new was
registered.** The change adds no new `.qml` file and no new `qmldir` line
(confirmed: `git diff --stat` on `dialectica-ui/src/qml/` touches only
`Core.qml` and the already-registered, already-`D`-prefixed
`DStoaListScreen.qml`). `design.md` decision 11 states this was a deliberate
choice — a `DKeyLine` component was considered and rejected because it would
have one caller — and both `check_qml_names.py` and `check_qml_members.sh`
pass over the result.

**No new dependency, no CI-workflow drift.** Both are clean by omission:
`Cargo.toml`/`Cargo.lock` are untouched, and `.github/workflows/ci.yml` has no
diff, so there is nothing to check for a source-layout gate measuring a moved
directory.

**One thing I looked at hardest for a defect and did not find one:** the
`Component.onCompleted` / `onVisibleChanged` pair in `DStoaListScreen.qml`
(lines 259–272) both guard with `if (screen.visible) screen.askKeyState()`,
which reads at first glance like the two-call-site duplication CLAUDE.md
warns about ("a branch must be got right at every call site"). It survives
scrutiny: `design.md` decision 8 records a measured Qt 6.10.3 harness
constraint (a parentless item created via `createObject(null)` does not fire
`onVisibleChanged` reliably on first show), so the two-line duplication is
load-bearing rather than accidental, and both paths are pinned by tests
(`test_the_key_state_is_asked_again_on_each_showing`,
`test_returning_home_from_a_feed_asks_the_key_state_again`). I do not think
this needs to move to a shared function — it already is one shared function
(`askKeyState()`); the two lines calling it are the minimum needed to cover
"first shown" and "shown again," and merging them further is not obviously
possible given the harness constraint recorded.

## Mutations left in the tree

None. `cargo mutants` was run and each mutant is applied, tested and reverted
automatically by the tool inside its own scratch build directories — it does
not mutate this worktree's source files in place. `git status` and `git diff
--stat ad9d867` both confirm a clean tree before this findings file was
written and staged.
