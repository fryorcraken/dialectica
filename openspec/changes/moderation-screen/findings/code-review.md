# Code review — moderation-screen (all four dimensions)

Reviewed by one instance covering correctness, security, readability and
architecture in one pass, per dispatch instruction ("most of this is inert
UI"). Scope: the piece's own three commits (`ad5564b..0fa357b`), weighted per
the brief toward the spec/PLAN.md amendments.

Gates run in this worktree, all green: `check_qml_names.py dialectica-ui` (46
files, 25 entries), `check_qml_members.sh` (26 files), `check_qml_reachable.py
dialectica-ui` (25 registered, 23 reached, 2 recorded), `run-qml-tests.sh`
full suite (20 spec files, 0 failed), `openspec validate moderation-screen
--strict` (valid). All match `tasks.md`'s claimed figures exactly.

Three manual mutations were tried and reverted (`git status` clean at every
checkpoint): dropping `"moderating"` from `stateNames` (7 tests failed, the
reshape's invariant is real), adding a `Core.call` inside an inert button
(exactly one precise failure, `test_nothing_on_the_moderation_screen_calls_the_core`),
and stripping one `textFormat: Text.PlainText` (caught by two tests). All
three confirm the tests they were aimed at are non-vacuous. A fourth
mutation — rewording the authority notice to imply governance without using
any word on the hardcoded list — is the one finding below with a measurement
behind it.

## Findings

- [ ] **`dev-writer`** — `openspec/changes/moderation-screen/proposal.md`,
      `design.md`, `docs/PLAN.md` §9.2 — **(architecture)** the amendment
      never acknowledges that it diverges from the design bundle's own
      README rule 3 ("Never show a count of anything global. Every number
      counts what this machine holds.").
      **Scenario:** the bundle's README (`tmp/ui-bundle-new/handoff/README.md`
      line 40) states the count rule as a *design* rule, not merely a spec
      requirement; the amendment addresses only the spec side
      (`stoa-navigation-view`) and PLAN.md, and nowhere in `proposal.md`,
      `design.md`, or the PLAN.md §9.2/§6 amendment is the bundle's own rule
      mentioned or reconciled. The task brief explicitly flagged this as a
      thing to check, and it is unaddressed — not contradicted loudly, just
      silently absent. A reader who has the bundle open and the amendment
      open would not learn from either that they now differ on this exact
      point.
      **Severity:** low-to-moderate — the code side is honest (D5, and the
      rendered placeholder genuinely is not "what this machine holds"
      either, so no rule is violated in effect), but the amendment's own
      stated goal ("a reader ... finds the reasoning in the requirement
      itself") is undercut by the one silent gap the reviewer was asked to
      check for.

- [ ] **`dev-writer`** — `dialectica-ui/tests/tst_moderation_screen.qml:288-305`
      — **(correctness / security)** `test_the_screen_claims_no_moderator_authority`
      is a four-phrase hardcoded blocklist and a rewording that conveys the
      same forbidden claim passes clean.
      **Scenario (measured):** changed `inertNoticeHeading`'s text to
      `"NOTHING ON THIS SCREEN PUBLISHES ANYTHING. THIS STOA IS UNDER YOUR
      GOVERNANCE."` — a sentence that asserts moderator authority as plainly
      as any of the four blocked phrases — and reverted. Full result:
      `tst_moderation_screen.qml` reported **13 passed, 0 failed**, including
      `test_the_screen_claims_no_moderator_authority` itself. The
      `moderation-view` requirement this test exists to pin ("no control
      claims moderator authority") is a real requirement and the screen is
      the one place a false claim of authority costs a user something; the
      test as written only catches a rewrite that reuses one of four
      strings.
      **Not a hidden defect** — the test's own comment says `NO SPEC: ...
      does not enumerate the words that would`, and `tasks.md`/the dispatch
      brief both flag this test as one the author already considers weak.
      Recorded here with the measurement the brief asked for, so the
      decision (strengthen vs. accept) can be made with a number rather than
      an impression. A stronger version would assert on the *absence of a
      predicate* (something claims capability over the Stoa) rather than a
      word list, which is hard to phrase generically in QML string matching
      — a plausible reason to accept the weaker form for this MVP, but that
      argument does not currently appear anywhere in the change.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DModeratedList.qml:99-100`
      vs. `DModerationScreen.qml:333,410` — **(readability)** a comment
      overclaims what the code does. The comment says the `Loader`
      "Supplies the delegate's `required property var rowData`", but neither
      `rowDelegate` instance in `DModerationScreen.qml` declares `rowData`
      as `required` — both declare a plain `property var rowData: ({...})`
      with a default object.
      **Scenario:** a reader trusting the comment would expect QML to refuse
      to instantiate a delegate that omits `rowData`, the way `required
      property var modelData` does two lines above it in the same file; in
      fact a delegate without `rowData` would silently fall back to the
      declared default object rather than erroring, which is a different
      failure mode than the comment describes (the whole point of `required`
      being enforcement, not documentation).
      **Severity:** low — harmless today because `onLoaded: item.rowData =
      block.modelData` always sets it, and both current call sites declare
      the same default shape. Worth a one-line fix (either add `required` to
      both `rowData` declarations, or soften the comment) since this is
      exactly the comment-drift class CLAUDE.md's review guidance warns
      about, and the current defaults (`{ name: "", address: "", postCount: 0
      }` / `{ excerpt: "", authorAddress: "", moderatorAddress: "" }`) would
      render silently blank rather than erroring if a future call site
      forgot to bind a delegate at all.

- [ ] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:2014-2027`
      — **(correctness, informational)** the unexplained vanishing-test
      defect this file documents (two tests written, one never appeared in
      the run under two different names, folding them into one function that
      demonstrably runs is what fixed it) has no established root cause.
      **Scenario:** nothing here indicates the same silent loss could not
      recur with the *next* test added to this file or a sibling one — the
      comment itself says the cause "is NOT established" and that an earlier
      guess (a runner enumeration limit) was wrong. This is not a defect in
      the shipped screen — the assertions that would have been lost are
      folded into `test_the_row_count_placeholder_claims_no_measurement` and
      do run, confirmed in the suite output (67 passed, 1 failed — the 1
      being this review's own reverted mutation, not this issue). Flagged as
      informational because the prompt named this "this repo's worst defect
      shape" and asked for attention: worth a standing note or a follow-up
      investigation (e.g. bisecting `qmltestrunner`'s test-function discovery
      against a two-function reproduction) rather than a blocking finding,
      since the author was transparent about the gap rather than concealing
      it.

## Clean

- **The spec deltas** (`stoa-navigation-view` amendment,
  `moderation-view` addition) read as honest, narrow amendments: the
  permanent prohibition (no global counts) is kept intact, the relaxed half
  states exactly what would restore the stricter form (a core call
  answering the number), and the retired scenario is narrowed under its own
  name rather than dropped — `openspec validate --strict` accepts this,
  confirming the MODIFIED block is well-formed.
- **PLAN.md's ruling 3 amendment** (§6, §9.2) strikes rather than deletes
  the original reasoning, attributes the reversal to the owner by name, and
  explicitly records the previous agent's refusal ("A previous agent refused
  to build the screen on the strength of the unamended ruling and asked
  rather than proceeding, which is the outcome this file is written for") —
  exactly the traceability the brief asked to verify.
- **The new `moderation-view` capability** is a reasonable split rather than
  scope creep: it mirrors the existing `composer-view`/`content-authoring`
  and `stoa-navigation-view`/`stoa-membership` pattern of a view-behaviour
  capability separate from the core-logic one (`moderation-resolution`
  already exists for the read-time authority check and is untouched).
- **Every `Text` in `DModerationScreen.qml` and `DModeratedList.qml` carries
  an explicit `textFormat: Text.PlainText`**, confirmed both by reading the
  file (no bare `Text {}` without it) and by mutation (stripping one is
  caught by two tests). This addresses the security concern the brief
  raised about `AutoText`'s markup-sniffing default on peer-supplied
  content.
- **The navigator reshape (`enterOnly`/`stateNames`, D3)** genuinely holds
  the "no two states set at once" invariant by construction — confirmed by
  mutation (dropping the new state from `stateNames` breaks 7 tests across
  reachability, the walk-coverage check, and the clear-on-enter tests).
  `openThread`'s claimed by-construction guarantee (early return on `chosen
  === null`) was read and is correct as described.
- **The `Repeater`-over-`ListView` fix (D4)** is a real, well-documented
  correctness fix, not a style choice — the height-not-yet-available failure
  mode it replaces is a genuine Qt 6.10.3 behaviour and not overstated.
- **No control on the screen is wired to anything** — confirmed by reading
  every `FlatButton` in `DModerationScreen.qml` (`markModeratedButton`,
  `moderateAuthorButton`, `unmoderateAuthorButton` ×2 instances via the
  delegate, `unmoderatePostButton`): none declares `onClicked` except
  `cancelButton`/`moderationBackButton`, which only call `screen.closed()`.
  No `Core.` reference exists anywhere in either new file.
- **The route in (`MODERATE` link in `FeedScreen.qml`) is a small, isolated
  diff** consistent with the existing signal-based navigation pattern; D6's
  reasoning for not gating it on moderation capability was read and is
  sound given `getModerationCapability` does not exist and
  `stoa-membership`'s creator-key staleness gap (both independently
  verified against the cited specs).
- **The Stoa-list count placeholder (D5)** is a fixed string
  (`"counts not yet available"`), not derived from any reply — confirmed by
  reading the diff and by mutation (deriving it from `visibleRows.length`
  is caught immediately by the digit-check half of
  `test_the_row_count_placeholder_claims_no_measurement`).
- `check_qml_members.sh` and the absence of `isPerson` anywhere in the two
  new files confirms the dropped-property concern from the brief is a
  non-issue here (the guard test for it lives in `tst_identity_chip.qml`
  from a prior piece and is unrelated to this screen, but nothing in this
  piece reintroduces the property).

## Not covered

No basecamp launch was performed (matches design.md's own stated gap) — the
visual rendering was not verified against the reference beyond what the
static gates and component tests can see. No Rust changed, so no Rust gate
applies.
