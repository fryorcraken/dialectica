# spec-test review — `home-screen-key-states`

Scope: `openspec/changes/home-screen-key-states/{proposal,design}.md` and both
spec deltas, against `dialectica-ui/tests/tst_stoa_screens.qml` and the
`#[cfg(test)]` modules in `dialectica/rust-lib/dialectica-core/src/{wire,keystore}.rs`
("Asking whether a master key is held" in `wire.rs`; the environment-unlock test
in `keystore.rs`). Implementation code was not read, except for the two
mutations in Part 2, each restored immediately after its test run. GitHub issue
#150 was read fresh via `gh issue view`.

## 1. Scenario coverage

Walked every scenario in both spec deltas against the test names and bodies.
Coverage is thorough and mostly many-to-many, as expected. All 8
`identity-onboarding` scenarios and effectively all `stoa-navigation-view`
scenarios (key-state derivation, no-key state, key-held state, could-not-be-read
state, copy table) have a directly corresponding test that asserts the actual
claim (element-tree absence via `namedAnywhere`/invisible-walking, not opacity;
verbatim text; call counts via the recording fake bridge). One gap found:

- [ ] **`tester`** — "Pasting stays available when the key state could not be
      read" (`stoa-navigation-view`, "A key state that could not be read is
      told apart from both others", scenario at spec.md lines 416–419) has no
      test.
      **Scenario:** `test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words`
      and the other could-not-be-read tests (lines 2875–3087 of
      `tst_stoa_screens.qml`) never touch `pasteSection`, `pasteField` or
      `pasteButton`. Every `pasteSection`/`pasteField`/`pasteButton` assertion
      in the file (lines 3204–3225, 3351–3352) is inside a no-key-state or
      key-held-state fixture. If a future change put the paste section inside
      one of the state `Loader`s by mistake instead of leaving it outside all
      of them (design.md Decision 7's stated shape), no test in this file would
      catch pasting disappearing specifically in the could-not-be-read state.
      **Severity:** low — the requirement is very likely satisfied by
      construction (design.md Decision 7 says the paste section sits outside
      every key-dependent `Loader`), and the equivalent scenario is directly
      tested for the other two states. But it is the one scenario in this
      delta's could-not-be-read requirement with no test naming it, and the
      "not instantiated" tests for that state check five different absences
      without checking this one presence.

## 2. Mutation testing

Budget used: two mutations, both in QML (the only file I could get an edit
through — see note below). No implementation was read beyond the lines
touched.

**Auto-mode classifier blocked every attempted edit to `wire.rs`**, first for
"Security Weaken" (hardcoding the `encrypted` field to `false`, to check
`the_protection_reported_is_the_openers_and_not_a_constant`) and then for
"Modify Shared Resources" (folding `KeystoreError::Locked` into the `NotHeld`
arm, to re-check `every_refusal_but_not_found_is_the_error_shape_in_the_keystores_words`).
Both were legitimate, narrowly-scoped, restore-immediately mutations within
this role's mandate; the classifier refused the edit itself, not a shell
command, so there was no way to route around it within the allowed shapes. I
did not attempt a third route. **The Rust side of this piece therefore
received no mutation from me this pass.** This is a smaller gap than it looks:
the test file itself already carries two "PROVED TO FAIL" comments pinning
exactly the two mutations design.md's Decisions 2 and 3 claim were measured
(`every_refusal_but_not_found_is_the_error_shape_in_the_keystores_words` at
wire.rs:5737–5739, `a_keystore_in_a_directory_that_cannot_be_searched_is_a_failure_not_an_absence`
at wire.rs:5767–5769), so those two are independently evidenced without my
needing to re-run them.

Mutations run (QML, `dialectica-ui/src/qml/DStoaListScreen.qml`), each via
`sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_stoa_screens.qml`,
each reverted with `Edit` immediately after reading the result, tree confirmed
clean afterward with `git diff --stat ad9d867`:

1. **`onClicked` of "Try reading the key again"** changed from
   `screen.askKeyState()` to also call `Core.createIdentity()` (simulating a
   mint riding along with the read-again press). **Caught**:
   `test_reading_the_key_again_asks_the_query_and_mints_nothing` went from PASS
   to FAIL ("and nothing minted, although the fake would have answered a
   mint" — actual 1, expected 0). Reverted; `git diff --stat` clean.
2. **The unencrypted-warning `visible` binding** loosened from
   `screen.machineKey.encrypted === false` to `screen.machineKey.encrypted !== true`
   (so an omitted or non-boolean `encrypted` field would also draw the
   warning). **Caught**: the suite went from 116 passing to 115 passing / 1
   failing; `test_an_omitted_protection_field_makes_no_claim_either_way` is the
   one test absent from the passing list on the mutated run (present and
   passing on the clean run). Reverted; `git diff --stat` clean.

No test in either mutation survived. **Nothing is left mutated in the tree —
both edits were reverted before this file was written**, confirmed by
`git diff --stat ad9d867` showing only this findings file.

I did not find a test in this file matching the three named defect shapes
(same-length-difference string comparison, hash-moved-so-property-held,
pinned-position-without-value). The two "field set is closed" Rust tests
compare whole JSON objects with `assert_eq!` against a literal, which is the
correct shape for that property (not the weak "position only" shape), and the
QML `visibleNamed`/`namedAnywhere` walkers distinguish "hidden" from "absent,"
which is the property this repo's CLAUDE.md specifically flags as the
recurring trap here.

## 3. `NO SPEC:` markers

`git grep -n -F "NO SPEC:"` over both test files returns markers, but none
fall inside the ranges this piece touches: `wire.rs` lines 5541–5789 (the
"Asking whether a master key is held" block) and the `keystore.rs`
environment-unlock test (~2939–3060) carry none; the QML markers at lines 653,
2100, 2113, 2127, 2143, 2357 are all pre-existing, in sections unrelated to the
key states (row title styling, share format, empty paste, created-address
render, half-not-a-string, placeholder-not-submitted). This piece introduces no
new marked or unmarked spec gap that I could find — every choice design.md
records (the three-state shape, the error-vs-boolean split, `=== true`/`=== false`
strictness, the refusal-inside-the-value shape) is backed by an explicit
Decision with its own "what breaks without it" note, which is the opposite of
an undocumented choice.

## 4. Requirements moved between capabilities

Not applicable in the REMOVED-here/ADDED-there sense Part 4 describes: both
halves of this piece's one REMOVED/ADDED pair ("Creating a Stoa asks for a
title...") live in the same capability, `stoa-navigation-view`, not two
different ones. I checked it anyway for the same defect shape (text changed
during the move): the REMOVED requirement's unreversed half — "a title and
nothing else," "the core's reason on a refusal," "an empty title passed
through" — appears in the ADDED requirement's body essentially verbatim
("MUST take a title and MUST NOT take... a creator key," "the screen MUST
render the reason the core gave, unreworded," "An empty title MUST be
accepted... and passed through rather than refused"), each still backed by its
own test (`test_the_create_affordance_offers_exactly_one_field_and_it_is_not_a_key`,
`test_a_creation_refused_for_want_of_a_key_renders_the_cores_reason`,
`test_an_empty_title_reaches_the_core_rather_than_being_refused_here`). No
requirement text changed in transit.

## 5. Spec soundness

- **Self-consistency**: read both spec deltas in full. No internal
  contradiction found — the "offered only in key-held state" requirement, the
  no-key state's "create affordance is not instantiated," and the
  could-not-be-read state's "MUST NOT instantiate... the create affordance"
  agree with each other and with the copy table (which restates already-scoped
  row-action copy for completeness, not a behavior change; "not a change to
  the list rows" in `proposal.md`'s "What this is not" is upheld — the table
  entries for the row actions are copy, not new behavior, and are already
  tested by `test_a_listed_row_renders_its_actions_verbatim`).
- **Testability**: no scenario asserts something no test could check. The one
  I checked hardest — "No persisted value stands in for the core's answer" —
  has no test by name, but the screen file references no storage API at all
  (`git grep` for `localStorage`/`Settings`/`persist` in
  `DStoaListScreen.qml` returns nothing), consistent with design.md's stated
  platform constraint that the sandbox gives the view no storage. I did not
  write this up as a coverage gap in Part 1: a test asserting "no persisted
  value" when nothing in the codebase could read one back would be asserting a
  negative the platform already forecloses, not pinning an implementation
  choice.
- **Staleness against issue #150**: read fresh via `gh issue view 150 --repo
  fryorcraken/dialectica`. The issue's headline is a single `hasMachineKey`
  boolean, "per the bundle's `FeedScreen.qml` precedent for `hasIdentity` (one
  flag, not several booleans combined ad hoc)." The spec instead has three key
  states behind one value. This is not an unexamined drift: `proposal.md`'s
  "Why" section and design.md's Decision 6 both name the issue's framing
  directly and give the concrete reason a boolean cannot work here (a boolean
  folds "could not be read" into one of the other two, and doing so the unsafe
  direction reopens exactly the trap — "already had a key, nothing was
  replaced" — the issue exists to remove). The `FeedScreen.qml`/`hasIdentity`
  precedent itself checks out: `git grep` confirms
  `readonly property bool hasIdentity: screen.identity.hasIdentity === true`
  exists in that file today. I found no requirement in this delta specified
  against a scope the issue no longer states.

## Summary

One low-severity coverage gap (Part 1, paste-section presence in the
could-not-be-read state). Both mutations run were caught. No unproven-but-weak
test found, no unmarked spec gap, no cross-capability requirement loss, no
self-consistency or testability defect, and issue #150 is not stale against
this delta — the one place the delta departs from the issue's own framing is
argued explicitly in `proposal.md` and `design.md`, not silently substituted.
