# Tasks

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

Every box below is ticked against a gate or a named test that was **run in this
worktree**. Where a requirement's scenario cannot be discharged by anything in
this tree, it is said so rather than ticked — an unticked box is a gap, a box
claiming a test verifies what it structurally cannot is a false statement a
reviewer will believe.

### The reachability rule

- [x] `check_qml_reachable.py` reads `qmldir` and the view's sources together,
      and computes reachability **transitively** from `Main.qml`.
- [x] Proven to fail on the pre-change tree: run against `ffc0795`'s
      `dialectica-ui`, it names six types — `DKeyNameWindow`,
      `DOnboardingScreen`, `DStatusBar`, `DVouchStamp`, `DIdentityChip` and
      `DTip`. The sixth is the transitive case and the reason the walk is not a
      grep: `DTip` **was** instantiated, but only by `DStatusBar` and
      `DVouchStamp`, both unmounted.
- [x] Deliberate non-instantiation is recorded in `qmldir`, beside the
      registration, with a stated reason — not in a list inside the checker.
      `tst_check_qml_reachable.py` pins that a record with no reason is refused,
      that a stale record on a reached type is refused, and that a record
      separated by a blank line does not drift onto the next type.
- [x] A newly registered type is covered without being listed:
      `::a_registered_type_nothing_instantiates`.
- [x] The gate's own tests run in CI **before** the gate, and pin both
      directions plus four "measured nothing" cases.

### Identity routing

- [x] Both probes read on every render, neither cached:
      `tst_navigation.qml::both_probes_are_re_read_on_every_render`.
- [x] Three outcomes, not two — an identity that cannot be used is **not** routed
      to creation: `::an_unusable_identity_is_not_offered_identity_creation`.
- [x] No navigation-level `hasIdentity` state exists to go stale. `Main.qml`
      routes on a signal; the feed's `hasIdentity` is `readonly` and derived, so
      there is no setter a stale value could be written through.
- [x] A degenerate probe reply claims no identity:
      `::a_degenerate_identity_reply_claims_no_identity` drives `"true"`, `1`,
      `null` and `{}`.

**Not discharged here, and not ticked:** "No persisted value stands in for
either probe" — the view persists nothing at all, so there is no code that could
fail this. Satisfied by construction: the QML sandbox has no filesystem access
(PLAN §2.1), and `grep` finds no storage API in `dialectica-ui/src/qml/`. That
is an absence a reviewer can check, not a test result.

### Onboarding, in and out

- [x] Reached from the feed's identity chip:
      `::the_create_affordance_reaches_the_onboarding_screen`, which also checks
      the Stoa travels — every core call on that screen takes one.
- [x] Left without keeping anything, making no keep call:
      `::the_screen_can_be_left_without_keeping_anything`.
- [x] A refused keep and a failed keep each still offer the way out:
      `::a_refused_or_failed_keep_still_leaves_a_way_out` drives both.
- [x] A kept identity returns to the feed and the identity shown comes from a
      **fresh** `who_am_i`, not the keep signal:
      `::a_kept_identity_returns_to_the_feed_and_re_asks_who_i_am`.
- [x] The screen selects no successor and was given none — the back affordance is
      declared in `Main.qml` outside the card, where no phase is in scope.

### The thread route

- [x] A feed row opens its thread with the Stoa, the root op and the record:
      `::a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op`.
- [x] No record is invented where the view holds none:
      `::no_record_is_invented_for_a_stoa_the_view_has_none_for`.
- [x] The feed is reached again from the thread, without a restart:
      `::the_feed_is_reached_again_from_the_thread`, and
      `::a_thread_that_could_not_be_read_still_offers_the_way_back` for the case
      where the way out matters most.

### One screen, one source

- [x] Exactly one main-area screen in every reachable state:
      `::exactly_one_screen_is_shown_in_every_reachable_state` walks all five.
- [x] No transition leaves two states set:
      `::no_transition_leaves_two_states_set_at_once`.

### Shared chrome

- [x] The lamps accompany every main-area screen:
      `::the_status_bar_accompanies_every_main_area_screen` walks all five.
- [x] Unbound chrome does not report health:
      `::an_unbound_lamp_does_not_report_health`.
- [x] Chrome is not withheld until its values arrive — it is declared outside
      every per-screen `visible:` binding, so there is no state that withholds
      it. The walk above renders it in states where nothing has been bound.

### Gates run in this worktree

- [x] `check_qml_reachable.py dialectica-ui` — 23 registered, 21 reached, 2
      recorded.
- [x] `tst_check_qml_reachable.py` — 16 cases, both directions.
- [x] `check_qml_names.py dialectica-ui` — 43 files, 23 entries.
- [x] `tst_check_qml_names.py` — 17 cases.
- [x] `check_qml_members.sh` — 24 files.
- [x] `tst_check_qml_members.sh` — 5 cases.
- [x] `run-qml-tests.sh` — 19 specs, 0 failed.

**What these gates cannot see**, said rather than left to read as coverage:

- The reachability gate proves a type is **instantiated** somewhere the root
  reaches, never that a user can get to it at runtime. A screen mounted behind a
  condition that is never true passes it. That half is a real basecamp launch.
- **No basecamp launch was performed for this change.** The app has not been seen
  rendering. Every claim here is from the static gates and `qmltestrunner`.
- `check_qml_members.sh` globs `src/qml/*.qml` only, so the new test specs are
  outside it. `check_qml_names.py` does reach them.
- The QML suite runs with the host absent, so it is blind to a host type-name
  collision by construction — that is the `D` prefix's job, not a test's.
