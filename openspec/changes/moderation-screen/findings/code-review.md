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

- [x] **`dev-writer`** — `openspec/changes/moderation-screen/proposal.md`,
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

      **Fixed** in `design.md` D5, which now carries the divergence in the
      place a reader of the amendment meets it. The finding is accepted in
      full: the bundle's rule 3 is quoted verbatim with its line number, and
      the entry states that the owner's reversal was of PLAN.md ruling 3
      (whose subject is the screen) and that **nothing in it addresses
      counting** — so the bundle's rule is recorded as *not* overridden by
      anyone, rather than as silently superseded.

      The reconciliation the finding's own severity note anticipates is made
      explicit: no rule is violated in effect because `counts not yet
      available` renders no numeral and asserts no quantity, so the bundle
      forbids a count that is not this machine's and the placeholder shows no
      count at all. D5 also names where this becomes a real divergence — the
      moment anything renders a number in that position, at which point the
      bundle's rule is the one to re-read first and the question is the
      owner's rather than the implementer's.

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_moderation_screen.qml:288-305`
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

      **Fixed** — the test was strengthened rather than the weaker form
      accepted, because the measurement made the gap concrete enough to close.
      `test_the_screen_claims_no_moderator_authority` now asserts a
      grammatical property over every rendered string: a second-person marker
      (`you`, `your`, `yours`, `you're`) within **3 words** of a governance
      noun (`moderator`, `stoa`, `govern`, `authority`, `permission`, ... 19
      in all), matched as words rather than substrings. Recorded as D7 in
      `design.md`.

      **Verified against this finding's own mutation.** Setting
      `inertNoticeHeading` to `"NOTHING ON THIS SCREEN PUBLISHES ANYTHING.
      THIS STOA IS UNDER YOUR GOVERNANCE."` — the exact string measured here
      as passing 13 of 13 — now fails: `'your' stands within 3 words of
      'stoa'`. The old blocklist's own `YOU ARE A MODERATOR HERE` fails too
      (`'moderator' stands within 3 words of 'you'`), so the rewrite loses no
      coverage. Both mutations reverted; `git diff` on `DModerationScreen.qml`
      is empty.

      **The window is the load-bearing part and was measured, not chosen.**
      Plain co-occurrence anywhere in a string does not work: the notice body
      honestly reads "...publishing a moderation, so every control here is
      inert: acting on one changes nothing, for you or for anyone else",
      pairing `moderation` with `you` at 15 words while claiming the opposite
      of authority — an unwindowed draft failed on it. A disclaimer keeps the
      halves apart; a claim puts them side by side.

      **The suggested form was considered and rejected as unreachable.**
      Asserting the absence of a predicate is stronger, and QML string
      matching cannot ask whether a sentence claims capability over a Stoa.
      The pairing is the nearest checkable proxy, and its three blind spots —
      a claim using neither marker, one split across two `Text` elements, one
      spaced wider than the window — are now written at the test rather than
      left to be discovered. The `NO SPEC` marker is kept and widened to cover
      both the noun set and the window.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DModeratedList.qml:99-100`
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

      **Fixed** by the second of the two options offered (soften the comment),
      because the first is **not available** — and that turned out to be the
      more useful half of the finding. Recorded as D8 in `design.md`.

      `required` does not merely go unused here; it cannot work. A `Loader`
      builds its component first and emits `onLoaded` second, so there is no
      point at which a required property could be supplied. Measured on Qt
      6.10.3 by adding `required property var rowData` to the author delegate:
      `Required property rowData was not initialized` per row, and
      `test_the_two_lists_are_separate_with_their_own_controls` saw **0 rows
      where it expects 2**. Reverted.

      The comment was also wrong in a second way the finding does not name: it
      credited `setSource`-style initial properties, where the code does an
      imperative `onLoaded` assignment. Both claims are gone. The replacement
      states the constraint, names the measurement, and records the trade
      explicitly — `Loader` buys the two lists a shared body and gives up the
      construction-time enforcement `required property var modelData` has two
      lines above, so an unbound delegate renders blank rather than erroring.
      That is why the defaults are the shapes the delegates read.

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:2014-2027`
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

      **Fixed** — as a gate rather than as an investigation, which is the part
      worth arguing. The finding asks whether the same silent loss could recur
      with the next test added; it now cannot recur *silently*.
      `check_every_test_ran` in `run-qml-tests.sh` compares the test names a
      spec declares against the names the runner reported and fails the run on
      any declared test that did not execute. **It needs no root cause**,
      which is what makes it the right answer to an incident whose cause was
      never established. Recorded as D9 in `design.md`.

      **The bisection the finding suggests was performed, and it found a real
      mechanism — but not this one.** QtTest treats `test_foo_data()` as the
      DATA PROVIDER for `test_foo()`, so declaring both removes **both** from
      the run: the `_data` body is called as a provider, returns undefined,
      and the other is skipped for want of rows. Measured on Qt 6.10.3 as `3
      passed, 0 failed` with neither executed, the only trace a `WARNING: ...
      no data supplied` line — not a failure, and not a pattern
      `check_bindings` greps for. **No `_data` name appears anywhere in
      `tst_stoa_screens.qml`'s history**, so this is the same defect *shape*,
      not the cause, and it is written down as such rather than claimed as a
      diagnosis.

      Two hypotheses were tested and ruled out, recorded in D9 so the next
      person does not repeat them: a **duplicate function name** fails loudly
      at compile (`Duplicate method name`), and a **helper sharing a test's
      prefix or taking an argument** runs normally.

      The gate has its own test, `dialectica-ui/tests/tst_check_every_test_ran.sh`
      (10 cases, both directions plus two end-to-end), wired into `ci.yml`'s
      `lint`-adjacent QML job beside `tst_check_bindings.sh`. Two mistakes
      made while building it are pinned by fixtures, both of which made the
      check vacuous in opposite ways: reading any `::name()` rather than
      result lines only, which let the skipped half of a `_data` pair read as
      having run; and a `[^:]*::` name extraction that matched nothing because
      `qmltestrunner::<Case>::` contains colons itself, so every test read as
      missing.

      **Measured on the file this finding names:** 76 declared test functions,
      78 reported (the two extra being `initTestCase`/`cleanupTestCase`), no
      test missing. The reviewer's "67 passed" reflects their mutated tree,
      not a live discrepancy.

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
