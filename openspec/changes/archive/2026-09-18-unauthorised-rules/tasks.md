# Tasks — remove unauthorised general rules from code comments

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change edits comment
      prose only and alters no behaviour and no requirement. The sentences being
      removed were never requirements, which is the finding itself. Declared as
      `skip_specs: true` alongside `schema:` in `.openspec.yaml`, which carries
      the argument — including why ratifying them instead was rejected.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — comment-and-prose changes only, no behaviour
      added, nothing a test could assert.
- [x] review: correctness — `code-reviewer`
- [ ] ~~review: security — `code-reviewer`~~ — not dispatched: comments-only
      change, no untrusted input, no logic.
- [ ] ~~review: readability — `code-reviewer`~~ — not dispatched: comments-only
      change, no untrusted input, no logic.
- [ ] ~~review: architecture — `code-reviewer`~~ — not dispatched: comments-only
      change, no untrusted input, no logic.
- [ ] ~~review: spec-test — `spec-test-reviewer`~~ — not dispatched: no spec
      delta (`skip_specs: true`), nothing for a spec-test review to check.
- [x] review: design — `design-reviewer`
- [x] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

### Verify before editing

- [x] Verify `composer-view/spec.md` requires inert affordances, and that the
      "general and SHALL NOT be read as being about any one field" sentence is
      present. Both quotes confirmed before relying on them.
- [x] Verify `VoteControl.qml`'s argument IS ratified, so it is correctly
      excluded. Confirmed against "The vote control displays no score".
- [x] Search `openspec/specs/`, `openspec/changes/` (including the archive) and
      `docs/` for recorded authority behind each swept claim.
- [x] Read `docs/PLAN.md` §10 verbatim to establish what the "worse than no
      gate" citations may actually claim.

### Edit the sites

- [x] `FeedScreen.qml` — drop the generalisation and "worse than absent"; keep
      the local fact that one ordering means nothing to select.
- [x] `DComposer.qml` — keep the measured defect account; drop the rule about
      how comments in this repo may be written.
- [x] `check_qml_members.sh` — keep the measured `-W 0` trade; drop "never to
      raise the `-W` ceiling".
- [x] `DIdentityChip.qml` — drop "should not be dropped to save a row"; keep the
      grounded mark-is-not-an-identifier claim and cite its source instead of
      asserting a standing rule.
- [x] `DStatusBar.qml` and `DTheme.qml` — demote the palette prohibition to a
      statement of present fact plus its reason.
- [x] `tst_stoa_screens.qml` — keep the mutation-testing finding; drop the
      methodology rule for all absence assertions.
- [x] The four "worse than no gate" sites — soften to what `docs/PLAN.md` §10
      actually says.

### Gates

Comment-only edits to shell and Python gates can break the gates themselves, so
each gate's own test runs too.

- [x] `dialectica-ui/tests/check_qml_names.py dialectica-ui`
- [x] `dialectica-ui/tests/check_qml_members.sh`
- [x] `sh dialectica-ui/tests/run-qml-tests.sh`
- [x] `dialectica-ui/tests/tst_check_qml_members.sh`
- [x] `dialectica-ui/tests/tst_check_qml_names.py`
- [x] Confirm the full diff touches only comments — no logic, no renames, no
      assertions.

## What a gate here cannot see

Stated rather than left implied, because every gate above passed both before and
after these edits and a reader could mistake that green for verification.

**No gate in this repo can check the property this change is about.** The edits
are comment text; the gates compile QML, resolve members and names, and run
specs — none reads English prose for whether a sentence claims authority it does
not have. Their green proves only that the edits broke nothing, which for a
comment-only change is the expected outcome and not evidence the change is
right.

The gate tests (`tst_check_qml_members.sh`, `tst_check_qml_names.py`) do check
something real here, and it is narrow: the edited comments live inside a shell
script and a Python module, where a mangled comment can break the file that
contains it. That is what those two runs verify — not the correctness of the
sweep.

The review that can see this change is a human or a `code-reviewer` reading the
prose against `openspec/specs/`.
