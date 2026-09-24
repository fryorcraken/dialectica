# Readability review — home-screen-key-states-followup

Dimension: **readability only** (comments, test names, stale references, functions
with two jobs, unmeasured numbers in comments). Correctness, security and
architecture are covered by other reviewer instances and are out of scope here.

## Scope and method

HEAD at review time: `0cbe1d4` (PR #155, merged). A full six-reviewer round
already covered the piece up to the pre-rebase state; this pass focuses on the
ten commits that landed after that round, per the task brief:

`2480536` (rename), `4b29627` (paste-availability test + mutation),
`9efae86` (archive), `6aa76d7` (Decision 8/9/15, #153 interaction),
`fca9199` (view-navigation spec amendment), `18ec847` (split no-call test),
`4296bc5` (comment + Decision 15 follow-up), `f9617cc` (#154 blank-title
reconciliation), `2602da0` (merge of main holding #154, conflict resolution in
`stoa-navigation-view/spec.md` and `tst_stoa_screens.qml`), `aac415b` (tick
rows).

Read every one of the ten with `git show <sha>`. Cross-checked every renamed
test, amended comment, and rewritten scenario against the code and spec text
it now describes, using `git grep -n -F` for stale-phrase sweeps rather than
trusting the commit messages' own claims. Ran the full `tst_stoa_screens.qml`
(136/136) and `tst_navigation.qml` (24/24) suites, and the Rust suite for
`dialectica`/`dialectica-core` (1141 + 30/1141 + 30). Reproduced two of the
mutation-testing claims made in commit messages by hand (see below) rather than
accepting them on the strength of the prose.

## Mutations run and reverted (tree confirmed clean via `git diff --stat 0cbe1d4` before writing this file)

1. **`dialectica-ui/src/qml/DStoaListScreen.qml`** — added
   `visible: machineKey.state !== "unreadable"` to the `pasteSection`
   `ColumnLayout` (the exact regression shape `4b29627`'s commit message
   describes). Result: 4 failed / 132 passed, matching the commit's claim of
   the new test plus three pre-existing layout-order tests. Reverted;
   `git diff --stat` on the file shows no changes afterward.
2. **`dialectica-ui/src/qml/Main.qml`** — added a `Core.whoAmI()` call at the
   top of `acquireIdentity()` (the mutation `18ec847`'s commit message
   describes). Result: `test_following_the_route_makes_no_call_of_its_own`
   failed with `Actual: [who_am_i,get_master_key]` vs
   `Expected: [get_master_key]`, exactly as claimed, and
   `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`
   stayed green, also as claimed. Reverted; `git diff --stat` on the file shows
   no changes afterward.

Both mutation claims in the commit messages I checked this way turned out to be
true, not just plausible-sounding — "run the claim, don't read it" did not turn
up a discrepancy here.

## Findings

None. Everything checked below was clean.

## Areas checked and clean

- **`view-navigation`'s scenario rename.** The old scenario name "Following the
  route asks the module for nothing" and its matching comment
  ("**It reaches the module for nothing.**" in `Main.qml`) are gone from every
  live location. `git grep -n -F "asks the module for nothing"` and
  `git grep -n -F "reaches the module for nothing"` across `dialectica-ui/` and
  `openspec/` find zero hits in live text. The one surviving hit for the old
  scenario title is in `openspec/changes/archive/2026-09-24-machine-identity-scope/specs/view-navigation/spec.md`,
  which is a different, already-archived change's own frozen delta describing
  what it changed — appropriately historical, not a live stale reference.
  `Main.qml`'s comment on `acquireIdentity()` now correctly says "**It makes no
  call of its own**" and names the list's `get_master_key` call as the list's,
  matching `fca9199`'s amended requirement text word for word.

- **The two split scenarios and their tests.** `fca9199` split "Following the
  route asks the module for nothing" into "Following the route makes no call of
  its own" and "Following the route creates no key, requests no slate and keeps
  nothing." `18ec847` added
  `test_following_the_route_makes_no_call_of_its_own` and trimmed the
  `NO SPEC` marker out of
  `test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing`.
  Both names match what they assert; both are present and pass in
  `tst_navigation.qml` (confirmed by the mutation above, which is exactly the
  case the first test's control — `closeFeed()` sharing `acquireIdentity()`'s
  body — exists to catch).

- **`"always offered"` and `"empty title MUST be accepted"`.** Neither phrase
  appears anywhere under `openspec/specs/` (`git grep -n -F` both, zero hits).
  The replacement blank-title language (`f9617cc`, folded into `2602da0`) is
  consistent between the live spec and the archived delta, as the commit
  message claims — spot-checked by diffing the two files' text, which is
  byte-identical apart from path.

- **`test_no_key_state_raises_identity` → `test_none_of_the_key_states_claim_identity`**
  (`2480536`). Body unchanged, name now matches: the function loops over all
  three key states and asserts none say "identity." Also correctly ticked in
  the (now-archived, but checked at the time) `readability.md` finding it
  answers.

- **`test_pasting_stays_available_when_the_key_state_could_not_be_read`**
  (`4b29627`). Name matches assertions: builds the could-not-be-read fixture,
  asserts `pasteSection`/`pasteField`/`pasteButton` are all rendered, and that
  clicking paste still reaches the `previewRequested` signal — not just
  "elements exist," as the commit message also claims. Confirmed via mutation
  above.

- **`test_a_missing_identity_offers_the_route_to_the_stoa_list`** (`6aa76d7`,
  fixed per Decision 15). Fixture now states `get_master_key`:
  `{"hasMasterKey":false}` and asserts `createKeyButton`, matching `who_am_i`'s
  `hasIdentity:false`. Consistent with the design.md narrative and green in the
  suite run.

- **Decision 8 and Decision 9's "concurrent writers" language** (`6aa76d7`).
  Both `design.md` and the two code comments in `DStoaListScreen.qml`
  (`askKeyState()`'s doc comment and
  `test_returning_home_from_a_feed_asks_the_key_state_again`'s) were updated
  together to drop the removed per-Stoa-keep writer and state the remaining
  writers (another Basecamp instance, a keystore file changed by hand). No
  leftover reference to the old "keeping a per-Stoa identity inside a feed also
  writes the master key" reasoning found anywhere live.

- **Decision 15 and its two citations**
  (`test_every_blank_title_reaches_the_core_rather_than_being_refused_here`,
  `test_pasting_stays_available_when_the_key_state_could_not_be_read`, both
  cited by exact name in `design.md`). Both names exist verbatim in
  `tst_stoa_screens.qml` — confirmed with `git grep -n -F`, not by eye.

- **`4296bc5`'s two-file update** (`Main.qml` comment + Decision 15's closing
  paragraph). Both now correctly name the ruling (`fca9199`) and the two tests
  it produced, rather than saying the question was "reported to the
  spec-writer" (which would now be stale, since it has been answered). Checked
  the exact test names cited resolve to real functions.

- **The merge conflict resolution in `tst_stoa_screens.qml`** (`2602da0`). The
  hand-resolved hunk around `test_the_placeholder_is_never_submitted_as_a_title`
  correctly kept this piece's `test_none_of_the_key_states_claim_identity`
  rename and correctly merged in #154's blank-title comment
  ("The empty title is blank, and `stoa-membership` refuses a blank title...").
  The comment is accurate to the merged behaviour (fixture now answers
  `create_stoa` with the error shape and asserts `screen.created === null`).
  `test_the_mint_request_names_no_stoa`, which the diff shows deleted by name,
  survives with its assertion intact under
  `test_creating_the_key_calls_the_mint_once_and_names_no_stoa` — not a silent
  coverage loss, just a rename that the diff's `-`/`+` makes look more
  dramatic than it is.

- **`test_an_empty_founding_title_still_gets_a_row_with_its_address`**
  (pre-existing, not part of the ten commits, but re-checked because its
  assertion text — `"an empty title is legal and must not be omitted"` — reads
  at a glance like it could contradict #154's blank-title-is-refused rule).
  It does not: this test is about **rendering** a list row for a Stoa whose
  founding title a peer already stored as empty (arbitrary/legacy peer data),
  not about **creating** one with a blank title. The two concerns are
  independent (a reader must still render whatever a peer's record says,
  regardless of what the local creation path now refuses), so "legal" here is
  accurate in its own context and not a stale reference to the removed
  create-time rule.

- **`wire.rs`'s `who_am_i_reports_that_the_master_key_alone_recovers_the_identity_in_use`**
  and related #153-premised test names — consistent with the "one machine key
  signs in every Stoa" claim these commits build on; no drift found.

## What I could not fully verify

`lgs basecamp build` was not run (a full nix build was judged out of proportion
to a readability-only pass given the QML and Rust suites both ran clean and
green, and the ten commits touch no build wiring). If the runner wants that
gate specifically exercised, another pass should run it; nothing in the ten
commits' diffs touches `metadata.json`, flake files, or the adapter, so I judge
the risk of a readability-relevant build-time surprise here as low.

## Branch and mutation state

- Branch: `worktree-wf_fff9ace7-faa-3` (confirmed with
  `git rev-parse --abbrev-ref HEAD`; not `piece/home-screen-key-states-followup`
  and not the repository root — worktree isolation held).
- Mutations left in the tree: **none.** Both mutations described above were
  reverted with `Edit` immediately after observing their effect, and
  `git diff --stat 0cbe1d4` was re-run before writing this file and shows only
  this findings file.
- Tree is ready to prune once this commit is cherry-picked.
