# Correctness findings — `theme-unshadow`

Reviewed on `piece/theme-unshadow` @ `77edb82`, in a worktree of its own. Both
halves of the new CI gate were re-run rather than read, the QML suite was run,
and every numeric claim in `design.md` / `tasks.md` was checked against a
command.

**What is clean**, stated in prose rather than as boxes so the list below is
only things that must happen:

- **The rename is complete.** `git grep -nw "Theme"` over the whole tracked tree
  returns only deliberate prose — `DTheme.qml`'s header, `ci.yml`'s step
  comment, `CLAUDE.md`, and the change's own documents. No QML binding, no
  `qmldir` entry and no test reference still names the old singleton. The five
  stale `docs/IDENTICON.md` references are all corrected (lines 84, 333, 804,
  811, 818), and nothing outside `dialectica-ui/` still points at the old name.
- **No token moved.** `git diff main:…/Theme.qml piece/theme-unshadow:…/DTheme.qml`
  is a pure comment-only addition of 34 header lines; every colour, font and
  metric is byte-identical, so the proposal's frozen-constant claim holds and no
  identity's identicon changes appearance.
- **The gate's demonstrated failing case reproduces.** Run against a clean
  worktree of `main`: exit 1 with all three arms firing. Run against this tree:
  exit 0, `ok: no QML type name collides with the host`. Both measured here.
- **The withdrawn premise does not survive as fact anywhere.** Every one of the
  five `outrank` hits in this change (`ci.yml:585`, `CLAUDE.md:306`,
  `DTheme.qml:18`, `design.md:45`/`:107`, `proposal.md:15`) sits inside an
  explicit "held and withdrew … measured and FALSE" framing, and in each file
  the true C++-registration mechanism is stated *before* the withdrawn one. A
  reader who stops at the first paragraph now leaves with the correct model.
  Task 3.4 claimed this and the claim is true.
- **Gates pass.** `run-qml-tests.sh`: 41 passed, 0 failed across 4 spec files,
  matching task 4.1 exactly. Qt6 `qmllint --unqualified disable` over the QML
  files: clean, exit 0.
- **Scoping is correct.** `ApparatusColumn.qml` and `MarginNote.qml` surviving
  with renamed references is right for this piece; their deletion belongs to
  `piece/drop-apparatus` (PR #70).

---

- [x] **`tester`** — `.github/workflows/ci.yml:639` — the reference arm's regex
      `[^A-Za-z]Theme\.` cannot match a `Theme.` that starts a line, so a real
      stale binding passes all three gates green
      **Scenario:** in `MarginNote.qml`, write the binding across two lines —
      `font:` on one line and `Theme.note` at column 0 on the next. This is a
      genuinely broken binding: at runtime the property is `undefined`, which is
      precisely the defect this piece exists to prevent. Measured in this
      worktree, all three checks reported success:
      the gate's arm 3 returned **0 matches, exit 0**; Qt6 `qmllint` **exit 0**;
      and `run-qml-tests.sh` **41 passed, 0 failed, exit 0**. The regex requires
      a character before `Theme`, and at column 0 there is none.
      A second invisible form: `Theme` and `.ink` split across lines
      (`border.color: Theme` / newline / `.ink`) — also 0 matches.
      **Severity: high.** This is the gate's own subject matter escaping it, and
      the escape is a plain line break rather than anything exotic. The fix is
      `(^|[^A-Za-z])Theme\.` — or, better, the sibling gate's shape (see the
      readability/architecture lane's comparison and the entry below).

      **CONFIRMED FIXED** — verified independently rather than inherited from
      `dev-writer`'s measurement, which is why the box stayed open. Both of your
      mutations were re-applied to `MarginNote.qml` on this tree and the rebuilt
      gate (`(?<![A-Za-z])Theme\b`) run against each:
      - `font:` / newline / `Theme.note` at column 0 — **exit 1**, cited at
        `MarginNote.qml,line=40`.
      - `color: Theme` / newline / `.inkSoft` — **exit 1**, cited at
        `MarginNote.qml,line=40`.
      Against the unmutated tree: **exit 0**, `ok: 17 QML file(s) checked`. That
      last run is the half that matters for the invariant — the gate is full of
      legitimate `DTheme.` references, so a pattern that fired on everything
      would also have "caught" both mutations. It fires on the broken form and
      not the correct one, which is the discrimination the box asked for.
      Each mutation was confirmed present with `git diff --stat` before the gate
      was run, since a mutation that fails to apply is indistinguishable from a
      gate that fails to catch it.

- [x] **`tester`** — `dialectica-ui/tests/run-qml-tests.sh:102` — a
      `ReferenceError` raised inside a component the suite instantiates is a
      QWARN, not a failure, so `design.md:93-98`'s "the existing suite checks the
      rename's completeness" is false for any component no test asserts against
      **Scenario:** same mutation as above, in `MarginNote.qml` (instantiated
      transitively by `tst_feed_states`, asserted against by nothing). The runner
      printed `ReferenceError: Theme is not defined` **33 times** across the
      `FeedStates` spec and still reported `Totals: 12 passed, 0 failed`, whole
      suite exit 0. Contrast the same mutation in `tst_identicon.qml` itself,
      where the reference is inside a `compare()` and the spec does fail with
      `Uncaught exception: Theme is not defined`.
      **Severity: high.** `design.md` claims the rename is "covered from both
      directions" — the CI gate's reference arm and the suite. Both directions
      share the same blind spot, so the two are not independent, and the document
      states a coverage guarantee the measurement does not support.
      The actionable part: the runner should fail on a QML `ReferenceError`
      appearing in runner output, which would make this class of defect visible
      for every component regardless of what a spec asserts.

      **FIXED — in the runner, and with a wider pattern than the box asked for.**

      *Reproduced first.* Your mutation, re-applied here: 35 `ReferenceError:
      Theme is not defined` lines across `tst_feed_states`, `Totals: 12 passed,
      0 failed`, suite exit 0. (You recorded 33; the count drifts run to run
      with instantiation order — 33, 35 and 36 all observed. The count is
      incidental, the blindness is not.)

      *Where the fix went: `run-qml-tests.sh`, not the tests.* A spec can only
      assert about a component it instantiates and reads, and the defect is
      precisely one in components no spec asserts against — so per-component
      assertions would be the `hand-maintained sweep lists go stale silently`
      trap, with a new component entering the module green. The runner sees
      every spec's output whatever it asserts, so one check there is total over
      components in a way assertions cannot be.

      *Not `QT_FATAL_WARNINGS`, the apparent one-liner.* It aborts on the first
      warning of any kind, killing the run with a crash instead of a diagnosis
      and taking the remaining specs with it — blunt in exactly the way your box
      warns against. `qmltestrunner -help` lists no flag that escalates a
      warning to a failure, so reading its output is the mechanism available.

      *Two patterns, not one, and this is the part that goes beyond the box.*
      Keying only on `ReferenceError` — what the box literally asks for — would
      still have passed a broken binding. Measured, on this tree:
      | mutation | ci.yml gate | `ReferenceError`s | runner now |
      |---|---|---|---|
      | `Theme.note` at column 0 | exit 1 | 36 | **exit 1** |
      | `DThemeTypo.note` | **exit 0** | 36 | **exit 1** |
      | `DTheme.noSuchToken` | **exit 0** | **0** | **exit 1** |
      | unmutated | exit 0 | 0 | exit 0 |
      Row 3 is the one that matters: a token that does not exist on a correctly
      named singleton raises **no `ReferenceError` at all** — Qt reports
      `Unable to assign [undefined] to QColor` — and still reports 12 passed, 0
      failed. That is the same runtime symptom as the outage (every token
      `undefined`), and a `ReferenceError`-only check is blind to it. So
      `check_bindings` matches both families.
      Rows 2 and 3 also answer the independence question your box raised: the
      static gate reports **clean** on both, so the runner check is not a second
      copy of the gate — it covers strictly more.

      *The check is itself tested*, in `dialectica-ui/tests/tst_check_bindings.sh`,
      gated by its own CI step ahead of the suite. A helper is part of the
      measurement: a check narrowed to nothing passes as quietly as a correct
      one. Six cases pin both directions — three outputs it must reject
      (verbatim `qmltestrunner` text, not paraphrase) and three it must accept:
      a real clean run, an unrelated Qt "undefined behaviour" warning, and test
      names containing the word `undefined` (this suite already has two, e.g.
      `test_has_more_defaults_to_false_rather_than_undefined`, so the
      false-positive case is live rather than hypothetical).
      Proved by mutating the check itself: dropping the `Unable to assign`
      pattern fails case 3 alone; widening it to the bare word `undefined` fails
      four cases. That second result corrected me — I predicted two. The
      `ReferenceError` text says "is not **defined**", so a bare-`undefined`
      pattern misses it entirely; the two message families share no common
      substring, which is the concrete reason one loose pattern cannot replace
      two precise ones. Recorded in the comment so it is not re-derived.

      *What this still cannot catch*, stated because a gate's blind spots should
      be written down rather than discovered: a binding that resolves to a
      *wrong but defined* value (`DTheme.paper` where `DTheme.ink` was meant)
      produces no diagnostic at all and is invisible to every gate here; and the
      host collision itself remains unreproducible under `qmltestrunner`, where
      basecamp is absent — this closes the runtime half of the blind spot, not
      the host-precedence half, which stays a static check plus a real launch.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:639` — the reference arm
      globs only `dialectica-ui/src/qml/*.qml`, so `dialectica-ui/tests/` is
      outside the gate entirely
      **Scenario:** restoring the old name in `tst_identicon.qml:75`
      (`String(Theme.markInk)`) leaves the gate at **0 matches, exit 0** —
      measured. Here the suite does catch it (that spec fails with
      `Uncaught exception: Theme is not defined`), so the consequence today is
      limited; but the gate is written as the authority on "no surviving bare
      `Theme.` reference in the view" (`proposal.md:52`) and it does not read
      four of the sixteen QML files in the module. The glob also cannot reach a
      subdirectory, should one ever be added under `src/qml/`.
      **Severity: medium** — a scope gap in the gate, currently backstopped by
      the suite for referenced tokens only.

      **FIXED** — `tasks.md` 4.5. The glob is now `rglob("*.qml")` over
      `dialectica-ui/`, so the gate reads all 17 QML files rather than 13.
      *Mutation that survives without it:* restoring the old name at
      `tst_identicon.qml:75` (`String(Theme.markInk)`) — under the old glob the
      gate returned 0 matches, exit 0; under the new one it fails at
      `tests/tst_identicon.qml:75`. Re-run both ways here, reproducing your
      measurement.
      Two things your entry prompted beyond the glob itself. The gate now
      **reports the file count it checked** (`ok: 17 QML file(s) checked`) and
      fails outright if it finds none, so a glob that silently stops matching is
      visible rather than silent — the failure mode a scope gap is. And the
      backstop you describe as limited turns out not to exist at all for most
      files: see the QWARN box above, which I reproduced.

- [x] **`dev-writer`** — `openspec/changes/theme-unshadow/design.md:66` — the
      "80-odd bare `Theme.` references" figure is wrong; the gate's own arm
      reports 113
      **Scenario:** running the step's exact pipeline against a clean worktree of
      `main` gives **113 lines** after the comment filter (137 occurrences if
      counted per-match rather than per-line). Per-file: `FeedScreen` 45,
      `SanitisedText` 13, `PostHeader` 11, `ScreenFrame` 9, `MarginNote` 7,
      `ApparatusColumn` 6, `FlatButton` 6, `Main` 5, `Identicon` 4,
      `VoteControl` 4, `AddressLabel` 3 — summing to 113. No reading of the
      command produces "80-odd".
      **Severity: low** (a documentation defect, not a code one), but this repo
      treats a number in a comment as a claim, and the figure is offered as
      evidence that the gate was proven to fire. The arm did fire; the count
      reported alongside it is not the one the command returns.
      The "89 errors" figure elsewhere is *not* a defect — it is explicitly
      framed as a reported symptom from a basecamp launch, not a measurement
      this tree can reproduce, and it is labelled as such.

      **FIXED** — `design.md` now reads **116**, with the split stated: 113
      under `src/qml/` and 3 in `tests/tst_identicon.qml`. Your 113 is confirmed
      exactly; the extra 3 are the files the old glob could not reach, so the
      widened gate and the corrected count come out of the same fix and the two
      numbers cross-check each other.
      *How it was got:* counted from the rebuilt gate's own output against a
      clean `main` worktree, not re-derived by hand — `grep -c` on the two path
      prefixes, 113 + 3 = 116. The document now also says where the figure comes
      from, so the next person can re-run the command rather than trust the
      number, which is the property the original "80-odd" lacked.
      You were right that this repo treats a number as a claim; I had offered it
      as evidence the gate fired, and the gate firing was true while the count
      beside it was invented.

- [x] **`dev-writer`** — `openspec/changes/theme-unshadow/tasks.md:114` — task
      4.2 says qmllint was run over "all thirteen QML files"; the module contains
      sixteen
      **Scenario:** `find dialectica-ui -name "*.qml"` returns 16 — twelve under
      `src/qml/` plus `DTheme.qml`, and four under `tests/`. Thirteen is the
      count of `src/qml/*.qml`, which is what CI's qmllint step globs, so the
      work was done correctly and only the description of its scope is off. Worth
      correcting because it is the same off-by-scope that produced the gate gap
      above: `src/qml/*.qml` is being described as if it were the whole module.
      **Severity: low**, documentation only.

      **FIXED, and the finding's own figure is one low — the module holds 17,
      not 16.** `tasks.md` 4.2 now says "the 13 files in `dialectica-ui/src/qml/`"
      and states the module total as 17 with the split.
      *Measurement:* `find dialectica-ui -name "*.qml"` returns 17 here, and the
      rebuilt gate independently reports `ok: 17 QML file(s) checked`. The count
      is the same on `main`: `git ls-tree -r --name-only main -- dialectica-ui`
      lists 13 under `src/qml/` and 4 under `tests/`. I think the 16 came from
      counting `main`'s twelve `src/qml` components **plus** `DTheme.qml` as an
      addition — but `DTheme.qml` replaces `Theme.qml` rather than joining it, so
      `src/qml/` is 13 on both trees.
      Flagging it rather than quietly using my own number, since the whole point
      of the box is that a stated count should be one a command returns. Both
      commands are named above so this one is checkable too.
      Your diagnosis of *why* it mattered was the useful part and is what I acted
      on hardest: the same off-by-scope produced the gate gap, so the fix was to
      widen the gate (box above) rather than only to correct a word.
