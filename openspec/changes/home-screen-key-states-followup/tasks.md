# Tasks — home-screen-key-states follow-up

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** Both open findings are
      about tests (a fixture in the wrong key state, and a test control tied to
      its subject only by comments), and neither changes a requirement.
      `.openspec.yaml` declares `skip_specs: true` alongside `schema:`, and
      `proposal.md` gives the reason for each finding.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

The six review rows are ticked because those reviews **have already run**.
This piece exists because of them: the owner asked for a final six-reviewer
round over #155 as merged (`0cbe1d4`), and `findings/` holds its output, one
file per row. The flow answers a finding in place, where the addressee ticks
its box and the reviewer does not run again, so no second round is due for
these rows. The `findings all ticked` row is what makes sure both open
findings actually get answered before archive.

## Implementation

## 1. Creation tests driven from the key-held state (spec-test finding, design.md Decision 2)

- [x] 1.1 Give `test_a_created_stoa_is_openable_from_the_creation_reply_alone`
      and `test_a_creation_success_carrying_no_address_is_a_failure` a
      `get_master_key: spec.heldKeyReply(aKeyHex(), false)` reply, and drive
      each through `createStoaButton` found with `visibleNamed`, not through a
      bare `create()`. Verify: `tst_stoa_screens.qml` green, 136 passed.
- [x] 1.2 Prove the fixture is load-bearing: dropping both replies fails both
      tests at the button lookup (`0` against `1`). Restore it.
- [x] 1.3 Prove the button is the driver: `onClicked: {}` on
      `createStoaButton` fails both tests at `createState`. Restore it.
- [x] 1.4 Prove each test still fails for its own reason: removing
      `rememberGenesis(...)` from `create()` fails the first at `genesisFor`,
      and disabling the no-address guard fails the second at `createState`.
      Restore both.
- [x] 1.5 Leave the pre-existing `NO SPEC:` marker on
      `test_a_creation_success_carrying_no_address_is_a_failure` in place. It
      predates #155 and is out of scope.

## 2. The identity-route test's control no longer depends on `closeFeed()` (architecture finding, design.md Decision 1)

- [x] 2.1 Replace the control in
      `test_following_the_route_makes_no_call_of_its_own` (`feed.closed()`,
      which runs `closeFeed()`) with `showView.chosen = null`, which shows the
      list without running any navigator function. Keep the
      `screenShown === "list"` check on the control. Verify:
      `tst_navigation.qml` green, 24 passed.
- [x] 2.2 Rewrite the comment on `Main.qml`'s `acquireIdentity()`, which
      described the old coupling. No code in `Main.qml` changes. Verify:
      `lgs basecamp build` succeeds.
- [x] 2.3 Measure M1–M4 (design.md Decision 1's table) against both the new
      control and the old one, restoring after each. Verify: new control is
      red on M1, M3 and M4 and green on M2; old control is red on M1 and M2
      and green on M3 and M4.

## 3. Gates

- [x] 3.1 Full QML suite: `sh dialectica-ui/tests/run-qml-tests.sh`.
- [x] 3.2 `check_qml_names.py`, `check_qml_members.sh`,
      `check_qml_reachable.py`, and qmllint on the three changed QML files, as
      `.github/workflows/ci.yml` runs them.
- [x] 3.3 `openspec validate home-screen-key-states-followup --strict`.
