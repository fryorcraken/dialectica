# spec-test review — home-screen-key-states-followup

Scope per the brief: everything added or changed **after** the six-reviewer
round on `2480536`, i.e. the ten commits `2480536^..aac415b`, with the merge
of #154 (`2602da0`) and the interaction between #153/#154 and this piece as
now sitting together on `main` (`0cbe1d4`) as the main focus. Read only:
`openspec/specs/stoa-navigation-view/spec.md`, `identity-onboarding/spec.md`,
`view-navigation/spec.md`, `dialectica-ui/tests/tst_stoa_screens.qml`,
`tst_navigation.qml`, and the `#[cfg(test)]` modules of `wire.rs` and
`keystore.rs`. The implementation outside those test modules was not read.

## What I checked

- Read each of the ten post-review commits (`git show <sha>`) individually:
  `4b29627`, `9efae86`, `59dbd0c`, `6aa76d7`, `fca9199`, `18ec847`, `4296bc5`,
  `f9617cc`, `2602da0`, `aac415b`.
- Verified the lead about `test_a_created_stoa_is_openable_from_the_creation_reply_alone`
  and `test_a_creation_success_carrying_no_address_is_a_failure` by reading
  `bridgeFor()` (returns `{"error":"no fake reply for get_master_key"}` when a
  method has no fixture reply) and the could-not-be-read requirement in
  `stoa-navigation-view/spec.md`. Confirmed it, below.
- Ran the full Rust suite (`cargo test --manifest-path
  dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`): 1141 + 30
  passed, 0 failed.
- Ran `tst_stoa_screens.qml` and `tst_navigation.qml` individually through
  `run-qml-tests.sh`: 136 and 24 passed, 0 failed.
- Ran `lgs basecamp build` from the tree root: succeeded for both `lgx` and
  `lgx-portable`.
- One mutation (budget: 1–2). Restored; tree is clean against `0cbe1d4` except
  for this findings file (verified with `git diff --stat 0cbe1d4` below).
- Checked `wire.rs` diff scope between the first-round tip and `aac415b`: the
  214-line change is entirely #154's blank-title work, arriving via the merge
  `2602da0`, not this piece's own commits — confirmed with
  `git diff --stat f9617cc..2602da0 -- .../wire.rs` matching the full delta.
  It does not touch `get_master_key` and has no interaction with this piece's
  key-state tests beyond the QML-level blank-title/key-gating composition
  covered below.
- Grepped both test files for `NO SPEC:` markers: one pre-existing marker
  remains in `tst_navigation.qml` (unrelated, appearance-claim), six
  pre-existing in `tst_stoa_screens.qml` (all noted in the archived proposal
  as predating this piece). The one marker this round did carry —
  on `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`,
  about whether the list's `get_master_key` call belonged to the route or the
  list — was resolved by the spec-writer's ruling in `fca9199` and its marker
  correctly removed in `18ec847`, replaced by a citation of the two split
  scenarios. No new unmarked spec gaps found in the post-review diff.

## Mutation run

**`Main.qml`'s `acquireIdentity()`**, adding `Core.whoAmI("")` before
`root.enterOnly("", null)` — the exact mutation the commit message for
`18ec847` claims was run when the test was written. Ran
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_navigation.qml`
before and after.

- Before: 24 passed, 0 failed.
- After: `test_following_the_route_makes_no_call_of_its_own` **FAILED**:
  `Actual (): [who_am_i,get_master_key]` vs `Expected (): [get_master_key]`.
  `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`
  stayed green, exactly as the commit message predicts (`who_am_i` is not one
  of the three method names that test checks).
- Reverted with `Edit`; confirmed via `git diff --stat` on `Main.qml` showing
  no output (clean) before writing findings.

This independently confirms `18ec847`'s claim and settles that
`test_following_the_route_makes_no_call_of_its_own` can fail for the reason
it names. I did not run a second mutation — the highest-value remaining
question (below) is a structural fixture defect provable by reading the spec
and the test together, with no mutation needed to convict it.

## Findings

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:778` and
      `:2627` — `test_a_created_stoa_is_openable_from_the_creation_reply_alone`
      and `test_a_creation_success_carrying_no_address_is_a_failure` drive
      `screen.create()` directly on a fixture whose `get_master_key` has no
      fake reply, which is the could-not-be-read key state
      (`bridgeFor` returns `{"error":"no fake reply for get_master_key"}` for
      any unlisted method, and `stoa-navigation-view`'s "A key state that
      could not be read is told apart from both others" requires: "In this
      state the screen MUST NOT instantiate the action that creates this
      machine's key, and MUST NOT instantiate the create affordance.").
      **Scenario:** on a real machine in the could-not-be-read state there is
      no create button to press — the affordance these two tests exercise
      does not exist in the element tree for the state their own fixture
      puts them in. Calling `create()` as a bare method sidesteps that
      absence entirely, so what each test actually proves is "the
      `create()`/`createState`/`genesisFor`/`canShare` bookkeeping behaves
      correctly when invoked," not "a user who creates a Stoa gets an
      openable, shareable address" or "a success reply with no address is
      rendered as a failure" — both of which are claims about the key-held
      state, the only state the affordance is reachable from. **Measured:**
      no other test in the file exercises `genesisFor()`/`canShare()` for a
      just-created Stoa through a `spec.heldKeyReply(...)` fixture (the only
      other tests touching those two functions after creation are these
      same two); `test_the_created_address_is_rendered` and
      `test_the_same_title_twice_is_one_stoa_reported_twice` show the
      correct pattern — `get_master_key: spec.heldKeyReply(aKeyHex(), false)`
      — that these two tests should follow but do not. Both requirements
      these tests are meant to pin
      ("A created Stoa's address is shown..." in `stoa-navigation-view`, and
      the `NO SPEC:` marker on the no-address case) are about the key-held,
      user-reachable path, and neither is actually tested through it.
      Rewriting both fixtures to use `spec.heldKeyReply(...)` and driving
      `create()` (or the real create button) from that state would close the
      gap with no change to what each test asserts.
      **Fixed** in the commit that ticks this box. Both fixtures now carry
      `get_master_key: spec.heldKeyReply(aKeyHex(), false)`, and each test
      drives creation through `createStoaButton` found with `visibleNamed`,
      with its count compared to 1, not through a bare `create()`. The
      assertions after that are unchanged. The reasoning is in `design.md`
      Decision 2: the fixture change alone would be decoration, because
      `create()` never reads the key state. The baseline showed both tests
      passing in the could-not-be-read state. The `NO SPEC:` marker on the
      second test is left in place (it predates #155). Mutations, each
      restored, predicted and observed:
      - Drop the `get_master_key` reply from both fixtures. Predicted: both
        red at the button lookup. Observed: both red, `Actual 0` against
        `Expected 1`, 134 passed, 2 failed.
      - `onClicked: {}` on `createStoaButton`. Predicted: both red at
        `createState`. Observed: both red (`""` against `"created"` and
        `"failed"`). `test_the_placeholder_is_never_submitted_as_a_title`,
        which already drove the button, went red too: 133 passed, 3 failed.
      - Remove `rememberGenesis(...)` from `create()` and disable its
        no-address guard. Predicted: each test red at the assertion its name
        is about. Observed: the first red at `genesisFor` (`""` against the
        genesis), the second red at `createState` (`"created"` against
        `"failed"`), 134 passed, 2 failed.

## Areas checked and found clean

- **`fca9199`/`18ec847`/`4296bc5` (the `view-navigation` reconciliation).**
  The amended requirement text is self-consistent (no call of its own / the
  list's calls are the list's / creates-requests-keeps-nothing are three
  separate, testable claims), each has a scenario, and each scenario has a
  test that can fail for the reason it names —
  `test_following_the_route_makes_no_call_of_its_own` mutation-confirmed
  above, `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`
  unchanged in its own assertions and still green. `Main.qml`'s comment and
  `design.md` Decision 15 both correctly state the amended boundary and cite
  the two tests. No inconsistency found between `view-navigation` and
  `stoa-navigation-view` over the one `get_master_key` call any more.
- **`6aa76d7` (Decision 8/9 premise update after #153).** The comment and
  design.md rewrite correctly drop the now-false "a per-Stoa keep also writes
  the master key" reason (that screen is unmounted per #153) while leaving
  the requirement's own text — "asked on every showing" — untouched and still
  correctly justified (another Basecamp instance, a hand-edited keystore
  file). No behaviour or test changed by this commit, matching its own
  commit message; confirmed by its diff touching only comments and
  `design.md`.
- **`f9617cc`/`2602da0` (reconciling the key-gated create requirement with
  #154's blank-title rule).** The live spec's blank-title paragraph, its two
  new scenarios ("A title made only of blank characters reaches the core as
  typed", "A creation refused for a blank title renders the core's reason"),
  and the placeholder scenario are all covered by
  `test_every_blank_title_reaches_the_core_rather_than_being_refused_here`
  and `test_the_placeholder_is_never_submitted_as_a_title` in
  `tst_stoa_screens.qml`, both correctly driven from
  `spec.heldKeyReply(aKeyHex(), false)` (the create affordance's only
  reachable state) rather than the no-key or could-not-be-read states. The
  live spec's requirement text is byte-identical between
  `openspec/specs/stoa-navigation-view/spec.md` and the archived change's
  copy, as the commit message claims — spot-checked the blank-title
  paragraph and both new scenarios side by side.
- **`get_master_key` Rust tests in `wire.rs`.** Whole-object
  `assert_eq!(reply_of(&out), serde_json::json!({...}))` comparisons against
  independently-derived expectations throughout (not self-consistency
  checks); real filesystem effects asserted for the "writes nothing" and
  "byte-for-byte unchanged" tests; the permission/unsearchable-directory
  tests carry their own PROVED-TO-FAIL notes citing the exact code change
  that would turn them red. This section of `wire.rs` did not change in the
  post-review commits (confirmed: the only post-review `wire.rs` diff is
  #154's blank-title work, arriving via the `2602da0` merge), so it carries
  forward the first round's review rather than being new.
- **Issue #150 staleness.** Not re-checked independently this round — the
  first-round spec-test review already did this (per its findings, now
  archived) and nothing in the post-review commits touches the home screen's
  key-state scenarios themselves; they only reconcile boundary wording with
  #153/#154. No new staleness introduced.
