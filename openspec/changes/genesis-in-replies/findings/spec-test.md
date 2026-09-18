# spec-test review: genesis-in-replies

Reviewed the spec delta (`openspec/changes/genesis-in-replies/specs/stoa-membership/spec.md`)
against the tests in `dialectica/rust-lib/dialectica-core/src/wire.rs` and
`dialectica-ui/tests/tst_stoa_screens.qml`, blind to `wire.rs`, `membership.rs`
and `DStoaListScreen.qml`'s implementation, per role. The one exception is the
mutation in finding 1, which necessarily touched `DStoaListScreen.qml` — the
mutation was applied, tested, and reverted; the tree carries no mutation now.

## Process note (not a spec/test finding)

**No `## Stages` block exists in this piece's `tasks.md`.** Per the runner's
correction mid-review: the runner created the change folder directly instead of
dispatching a `spec-writer`, so there is no stage roster and no `spec-test` row
for me to tick. I have not added one. A fixer is reportedly being dispatched to
add the block.

## 1. Does every scenario have a test?

All four ADDED scenarios and the two new MODIFIED scenarios ("Creation returns
the record as well as the address", "A Stoa is openable from the listing alone")
have a covering test. One is a partial match, flagged below; everything else is
clean.

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs`,
      `a_stoa_is_openable_from_the_listing_after_a_restart`
      **Scenario:** "A Stoa is openable from the listing alone" says *"the
      caller can read that Stoa's feed using only what the listing reported."*
      The test ends at `join_stoa`, not `read_feed` — it never calls the method
      the scenario names. This is a defensible substitution rather than a real
      gap (`read_feed` is not defined in `wire.rs`; grepped and confirmed
      absent, so it lives elsewhere, and `join_stoa` shares the same
      `(stoa, genesis)` input contract `read_feed` takes). Flagging it because
      the scenario names a specific call and the test silently swaps it for a
      sibling with the same shape — worth the spec-writer or tester deciding
      whether "read that Stoa's feed" should be reworded to name the shared
      input contract instead of a specific call, so the test-to-scenario
      mapping is exact rather than "close enough."

- [ ] **`spec-writer`** — `specs/stoa-membership/spec.md:30` ("A reply naming a
      Stoa carries the record..." requirement)
      **Scenario:** "This requirement adds no storage obligation. The retention
      requirement **above** already requires the record be kept..." — but
      within this delta file the retention text (inside "A joined Stoa's
      genesis record is retained, not only its address", an existing,
      untouched requirement elsewhere in the live spec) is not guaranteed to
      land **above** the new ADDED requirement once merged. `docs
      /OPENSPEC-ARCHIVE.md` documents only "prepend + rename `## ADDED
      Requirements` → `## Requirements`" for a *new* capability; for an
      *existing* one (this case), it does not specify where new ADDED
      requirements are spliced relative to the 12 existing requirements
      already in `openspec/specs/stoa-membership/spec.md`. Two other
      above/below references in this same delta (lines 76, 117) are safe
      because they sit inside MODIFIED blocks that keep their existing
      position; this is the only new cross-reference whose target position is
      undetermined. If the merge places the new requirement before "A joined
      Stoa's genesis record is retained..." (which sits 7th among 12 in the
      live spec), the word "above" becomes false in the merged document — a
      forward reference by position rather than by name, the same class of
      fragility `.claude/agents/README.md` calls out for PLAN section numbers.
      **Measured**: confirmed by reading `docs/OPENSPEC-ARCHIVE.md` (no
      splice-position rule for an existing capability) and diffing the delta's
      requirement order against the live spec's (`git show
      piece/genesis-in-replies:openspec/specs/stoa-membership/spec.md`, grep
      `^### Requirement`). Low severity — readable either way once merged,
      since the reader can find the named requirement's actual text — but the
      directional claim itself may end up false. Cite the requirement by name
      instead of by position.

## 2. Can each test actually fail?

**Three Rust tests, verified to fail first.** Checked out `46708ba` (the
commit that adds the three new `wire.rs` tests, before the fix in `e23cd41`)
in this worktree and ran each with `cargo test --manifest-path
dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core <name>`:

- `a_creation_reports_the_record_its_address_is_the_hash_of` — **FAILED**,
  panic: `the reply must carry a 'genesis' record for the view to open or
  share: {"foundingTitle":"Agora","policy":"open","stoa":"80329cf0..."}`
- `a_listed_stoa_carries_the_record_its_address_is_the_hash_of` — **FAILED**,
  same panic message against the listing item shape.
- `a_listed_record_round_trips_through_the_join_the_view_performs` — **FAILED**,
  `called 'Option::unwrap()' on a 'None' value`.

Checked out `piece/genesis-in-replies` HEAD (`ce8ab44`) and ran the full core
suite: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica
-p dialectica-core` → 1022 passed (dialectica-core), 30 passed (end_to_end), 0
failed. All three now pass, on the property named (the relation `genesis.address()
== reply["stoa"]`, not presence-only) — confirmed by reading the `genesis_of`
helper, which decodes the hex, decodes the record, and asserts the address
equality before returning.

**None of the three seeds `genesisByStoa` or any internal map directly.**
Grepped `wire.rs` at the piece HEAD for `genesisByStoa`/`genesis_by_stoa`: no
hits. `create()` and `listed()` (the test helpers) call `create_stoa` and
`list_stoas` — the actual handlers under test — never construct membership
state by hand. The trap the author claims to have avoided is genuinely absent.

**Three of four QML specs, verified to fail first.** Checked out `e23cd41`
(core fix landed, QML fix not yet) and overlaid the final `tst_stoa_screens.qml`
(`git checkout piece/genesis-in-replies -- dialectica-ui/tests/tst_stoa_screens.qml`),
then ran `sh dialectica-ui/tests/run-qml-tests.sh
dialectica-ui/tests/tst_stoa_screens.qml`: **78 passed, 3 failed** — exactly
`test_a_created_stoa_is_openable_from_the_creation_reply_alone`,
`test_a_listed_stoa_is_openable_from_the_reply_alone`, and
`test_a_record_survives_paging_away_from_the_stoa_that_carried_it`, each on an
empty-string mismatch (`Actual (): ` vs. a hex record). The fourth,
`test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one`, already
passed at that commit — matching the claim. Restored the test file and
verified the full suite is green at HEAD: 81 passed, 0 failed.

- [ ] **`tester`** — `dialectica-ui/tests/tst_stoa_screens.qml:632-654`
      (`test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one`)
      **Scenario:** claimed as a "both-directions guard... holds in both
      directions by design," pinning that `rememberGenesis` does not write an
      empty string into `genesisByStoa`. **Mutation measured**: removed the
      `|| genesis === ""` half of `DStoaListScreen.qml`'s guard (`if (typeof
      genesis !== "string" || genesis === "") return;` → `if (typeof genesis
      !== "string") return;`), leaving an empty string free to be written into
      the map. Ran `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_stoa_screens.qml`: **the mutation survived** —
      81 passed, 0 failed, including this test. Root cause: `genesisFor`
      collapses both "never recorded" (`undefined`, falls through the ternary
      to `""`) and "recorded as an empty string" (`typeof "" === "string"` is
      true, so the stored `""` is returned as `""`) to the identical
      observable value `""`. The test asserts `genesisFor(...) === ""` and
      `canShare(...) === false` for both the missing-field and the
      empty-field row, and both hold identically whichever branch
      `rememberGenesis` takes on the empty string — a direct instance of this
      repo's "two explanations give the same answer" test-defect family. The
      guard's *outward* effect (no spurious share button, no `""` reaching
      `read_feed`) is real and currently correct, but this specific test
      cannot detect a regression that reintroduces the write, because nothing
      it asserts can tell the two code paths apart. Reverted the mutation
      immediately after the run; the tree is restored (`git status --short`
      confirms clean at time of writing). To make this a real guard, the test
      would need to observe the write itself — e.g. a second `reload()` or
      `rememberGenesis` call with a legitimately-non-empty record for the same
      address afterward, checking the empty write didn't leave a "recorded"
      marker that shadows a later legitimate one, or exposing
      `genesisByStoa` and checking the key is truly absent (`stoa in
      screen.genesisByStoa === false`) rather than checking `genesisFor`'s
      lossy projection.

**Restart test uses a real file and a genuinely reopened store**, verified.
`a_stoa_is_openable_from_the_listing_after_a_restart` (`wire.rs`) creates a
`WireTempDir` (real temp directory, `Drop`-cleaned), opens a `MembershipStore`
at a real path, creates a Stoa, drops that handle, and reopens the store twice
more (`MembershipStore::open(&path)`) before listing and joining. This is not
an in-memory fixture — the in-memory helper `a_membership_store()` used by the
other four wire.rs tests is a different function entirely
(`MembershipStore::in_memory()`), and this test deliberately avoids it. The
claim that in-memory fixtures structurally cannot reach this scenario is
consistent with what I read: nothing about `MembershipStore::in_memory()`
persists across a second `open()` call by construction. This test was **not**
part of the "proved to fail first" set (it lands in a later commit,
`57a6f14`, after the wire.rs fix `e23cd41` was already applied) — that's
consistent with the author's claim, which only named "three Rust tests" as
fail-first; this fourth one is presented separately and correctly as a
regression pin rather than a TDD-first test.

## 3. NO SPEC markers

Grepped the piece's diff (`git diff 2549468..piece/genesis-in-replies --
wire.rs tst_stoa_screens.qml DStoaListScreen.qml`) for `NO SPEC`: none added by
this piece. No unmarked unspecified behaviour found either — every new test's
assertions trace to a scenario in the delta, and the encode-failure branch
(next section) is the one place behaviour exists with no test either pinning
it or marking it, which is a coverage gap rather than an unmarked default.

## 4. Requirements moved between capabilities

Not applicable — the delta has no `REMOVED Requirements` section, only
`ADDED` and `MODIFIED`. Nothing to check here.

## 5. Spec soundness

- [ ] **`tester`** — spec scenario "A record that cannot be encoded is a
      failure, not an empty field" (ADDED requirement, `stoa-membership`)
      **Scenario:** no test in the diff exercises this. Grepped `wire.rs` at
      piece HEAD for anything constructing an unencodable retained record or
      asserting on the encode-failure branch of `stoa_reply`/
      `membership_page_json`: no hits — the only four new Rust tests are the
      three "proved to fail first" ones plus the restart test, none of which
      forces an encode failure. Read (not executed, to stay within the
      mutation budget) `stoa_reply`'s definition and confirmed the branch
      exists in the implementation (`match genesis.canonical_bytes() { Ok(b)
      => ..., Err(e) => return error_json(&e.to_string()) }`), so the
      requirement is implemented but its specific behavioural contract —
      "the call reports a failure" and "no success reply is emitted carrying
      an empty or absent record" — has no regression test pinning it. Given
      this repo's own "Every bug fixed ships with a regression test... A
      regression test that has never failed proves nothing" standard
      (CLAUDE.md), and that this exact class of bug (an empty/absent record
      silently reported as success) is the one this whole piece exists to
      fix, the absence of a test for the sibling failure-shape guarantee is
      worth closing before merge.

- Self-consistency and staleness against PLAN.md (`origin/main`): no
  contradiction found within the delta beyond the above/below fragility
  already flagged in section 1. `docs/PLAN.md` on `origin/main` already
  points at `stoa-membership` with "BUILT: see the `stoa-membership`
  capability" for `createStoa`/`listStoas` rather than restating a reply
  shape, so there is nothing in PLAN.md this piece needed to supersede or
  that reads as stale against it — grepped PLAN.md for the specific bug
  text ("cannot open", "ended mid-field", "genesis record beside") and found
  no reference, so this piece isn't answering a question PLAN.md posed.

## 6. PLAN.md shed status

Nothing in `docs/PLAN.md` on `origin/main` described this bug, this reply
shape, or an open question this spec now answers — so there is nothing PLAN.md
needed to strike through or move out. Clean.
