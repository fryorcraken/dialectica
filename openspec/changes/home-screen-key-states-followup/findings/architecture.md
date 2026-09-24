# Architecture review — home-screen-key-states-followup

Dimension: **architecture** only (as assigned). Correctness, security and
readability are other reviewers' rows.

## Scope and method

HEAD at review time: `0cbe1d4` (PR #155, merged to `main`), matching the
brief. Reviewed the ten commits after the pre-rebase review round
(`2480536^..aac415b`, each read with `git show`), the whole feature as merged
(`git show 0cbe1d4 --stat`), the archived change
`openspec/changes/archive/2026-09-24-home-screen-key-states/`, and the
promoted specs `stoa-navigation-view`, `identity-onboarding` and
`view-navigation`. Built and ran the full gate set from the worktree root:

- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` — 1141 + 30 tests, 0 failed.
- `sh dialectica-ui/tests/run-qml-tests.sh` (all 25 spec files, including
  `tst_stoa_screens.qml` and `tst_navigation.qml` individually first) — every
  file 0 failed.
- `lgs basecamp build` — both `lgx` and `lgx-portable` artefacts built clean.
- `python3 dialectica-ui/tests/check_qml_names.py` — 53 QML files, 25 qmldir
  entries, clean.
- `openspec validate --all --strict` — 29 passed, 1 failed
  (`change/sqlite-projection`), matching `aac415b`'s claim that this failure
  is pre-existing on `main` and untouched by this piece.

One mutation was made and reverted (see below); `git diff --stat 0cbe1d4`
shows only this findings file.

## What I checked and found clean

**One source of truth for the key state and the create affordance.**
`stoa-navigation-view`'s "Creating a Stoa asks for a title and nothing else,
and is offered only once the core reports a key" (spec.md:877) is the single
requirement governing both the key-state gate and the blank-title rule — 23
requirements total in the live spec, no duplicate or stray "always offered"
survivor from the pre-#154 wording. `DStoaListScreen.qml`'s `create()`
function passes the title through unvalidated, matching the spec and its own
comment ("a check here would be a second copy of the blank list that could
drift from it") exactly.

**Capability boundaries after `fca9199`/`f9617cc`.** The four capabilities
named in the brief own genuinely distinct rules, not two of them asserting
one:
- `identity-onboarding` owns the wire contract of `get_master_key` (reply
  shape, what counts as "held" vs "unreadable" vs "none") — spec.md:779.
- `view-navigation`'s *Acquiring an identity is reached from the navigator*
  (spec.md:331) owns only that following the route makes no call of its own,
  and that the calls a *showing* makes belong to the thing being shown, not
  the route. It explicitly defers "what the list shows" to
  `stoa-navigation-view`.
- `stoa-navigation-view` owns when the create affordance is instantiated,
  what it accepts, and what a blank title does — spec.md:877.
- `stoa-membership`/`stoa-genesis` own what "blank" means and that a blank
  title is refused at all; `stoa-navigation-view` cites both rather than
  restating the rule (verified: `git grep` shows the blank-title definition
  and refusal live only in `stoa-genesis`/`stoa-membership`, cited by name
  from the UI spec).

No pair of these overlaps on the same rule. `fca9199`'s resolution — "the
route makes no call of its own; the calls the list makes when shown are the
list's" — is the correct fix for the contradiction it names (the old
`view-navigation` text and `stoa-navigation-view`'s every-showing
`get_master_key` requirement were genuinely in tension over one call), and
both live specs now state that split identically. Confirmed byte-identical
against the archived delta (`openspec/changes/archive/.../specs/view-navigation/spec.md`
vs `openspec/specs/view-navigation/spec.md`, lines 331–390 match exactly).

**Live specs vs. archived deltas.** All three delta files
(`identity-onboarding`, `stoa-navigation-view`, `view-navigation`) match their
corresponding live-spec sections. `2602da0`'s merge-conflict resolution in
`openspec/specs/stoa-navigation-view/spec.md` correctly composed #153's
key-gating with #154's blank-title/getStoa additions: no requirement was
silently dropped, the requirement count (23) and the "byte-identical to
f9617cc's" claim both check out.

**`D`-prefix and `qmldir` conformance.** No new QML type was registered in the
post-review window (`Core.qml`, `DJoinScreen.qml`, `DStoaListScreen.qml`,
`Main.qml` are all pre-existing files, and `qmldir` is untouched). The naming
gate (`check_qml_names.py`) passes clean.

**`openspec validate` and the `aac415b` tasks.md tick.** Reran the validator
myself; the one failure it reports is `sqlite-projection`, exactly as the
commit claims — not a defect newly introduced by this piece.

## Finding

- [x] **`dev-writer`** — `dialectica-ui/src/qml/Main.qml:231` and
      `dialectica-ui/tests/tst_navigation.qml:283` —
      `test_following_the_route_makes_no_call_of_its_own`'s control
      (`closeFeed()`) has no structural guarantee of staying in lock-step
      with `acquireIdentity()`, only two comments asking a future editor to
      notice
      **Scenario:** both functions are independently-declared, each simply
      `root.enterOnly("", null)` — not one delegating to the other. If a
      future change gives `closeFeed()` a call of its own (feed-specific
      cleanup, say) without an equal and opposite change to
      `acquireIdentity()`, `test_following_the_route_makes_no_call_of_its_own`
      fails — but its failure message ("acting on the identity route must
      reach the bridge for exactly the calls an ordinary return to the list
      makes, and none besides") blames the *route*, when the route is
      unchanged and the *control* drifted. A developer chasing that message
      would look at `acquireIdentity()` and find nothing wrong there.
      **Measured:** added `Core.whoAmI("")` to `closeFeed()` only (leaving
      `acquireIdentity()` untouched) and reran
      `tst_navigation.qml`: 23 passed, 1 failed —
      `test_following_the_route_makes_no_call_of_its_own` failed with
      `Actual (): [get_master_key]`, `Expected (): [who_am_i,get_master_key]`,
      at the same assertion and with the same wording the route-divergence
      case would produce. Reverted immediately after
      (`git diff --stat 0cbe1d4` is empty; only this findings file remains).
      **Assessment:** this is not an oversight — `design.md` Decision 15
      documents the coupling and its risk explicitly ("If the two functions
      diverge, the control stops being equivalent to a plain showing... 
      Whoever makes the two diverge must give the test a control that is
      still only a showing of the list"), and `Main.qml`'s own comment on
      `acquireIdentity()` repeats the warning at the point a future edit
      would be made. The mitigation is real but purely textual: nothing
      would catch a change to `closeFeed()` alone that a reviewer's read of
      two comments doesn't catch. Given `CLAUDE.md`'s own "hand-maintained
      sweep lists go stale silently" pattern, this is worth a low-cost
      structural fix (e.g. a one-line test asserting the two functions
      produce the same navigator state transition, or having
      `acquireIdentity()` delegate to `closeFeed()` by name so a diff to one
      is necessarily a diff to both) rather than a doc-only safeguard. Low
      severity: the risk is well-documented, the failure mode is "wrong
      blame in a red CI run" rather than "defect ships silently", and no
      such divergence exists on `main` today.
      **Fixed** in the commit that ticks this box, and by neither of the two
      suggested options. The control no longer runs `closeFeed()`. It writes
      `chosen = null` on a feed-showing `Main`, which shows the list without
      running any navigator function, so nothing links the test to
      `closeFeed()` at all. `design.md` Decision 1 gives the reasoning and
      cites Decision 15 of the archived `home-screen-key-states` design. In
      short: delegating `acquireIdentity()` to `closeFeed()` is the trap
      Decision 15 names, and a same-transition test cannot see calls, so it
      stays green in this finding's own scenario. `Main.qml`'s comment on
      `acquireIdentity()` is rewritten, and no code there changes.
      Mutations to `Main.qml`, each restored, predicted and observed, new
      control and then old control:
      - M1: `Core.whoAmI("")` in `acquireIdentity()`. Predicted: new red.
        Observed: new red, `[who_am_i,get_master_key]` against
        `[get_master_key]`. Old: red, same message.
      - M2: `Core.whoAmI("")` in `closeFeed()` only (this finding's
        scenario). Predicted: new green. Observed: new green, 24 passed. Old:
        red, `[get_master_key]` against `[who_am_i,get_master_key]`, which
        reproduces this finding.
      - M3: M2, plus `acquireIdentity() { root.closeFeed() }`. Predicted: new
        red. Observed: new red, same message as M1. Old: **green**, 24
        passed, while the route calls `who_am_i`. This is the delegation
        option's failure, measured.
      - M4: `Core.whoAmI("")` in `enterOnly()`. Predicted: new red. Observed:
        new red, same message as M1. Old: green, because both sides made the
        call.

## Areas not covered

Correctness, security and readability are other reviewers' dimensions per the
brief and are not assessed here beyond what overlapped unavoidably with
architecture (e.g. reading the blank-title implementation to confirm the
capability-boundary claim).
