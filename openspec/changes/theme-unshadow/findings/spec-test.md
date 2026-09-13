# Spec-test findings — `theme-unshadow`

Reviewed on `piece/theme-unshadow` @ `d43dcc1`, in a worktree of its own. The
spec position, the tests and the gates were read; the implementation was read
only where a mutation required it. Every claim below is a measurement, run in
this worktree, with the tree confirmed byte-clean (`git status --porcelain`,
empty) after each restore and the suite re-run green at the end.

The readability/architecture findings at `d43dcc1` were read first. The known
high-severity gap — arm 1 reading only `singleton` lines — is **not** re-reported
here; a `dev-writer` owns it. What is below is the test question, which is
different: not "does the gate check the right thing" but "would anything notice
if it stopped checking".

## The spec position is sound, and that is not a dodge

`.openspec.yaml` sets `skip_specs: true`, and the reason given holds up. No
token changes name, value or meaning; nothing a view may claim or must refuse to
claim moves. `openspec validate --all --strict` passes 17 items, and honours the
marker with an INFO rather than ignoring it. There is no scenario to walk and no
requirement without a test, because there is no requirement. A requirement
invented so validation passed would have described what our singleton is called,
which is not system behaviour — the file says exactly that and is right.

`docs/PLAN.md` on `origin/main` says nothing about QML type naming
(`git grep -n "Theme\|singleton\|qmldir" origin/main -- docs/PLAN.md` returns two
hits, both about the core module's Rust singleton and `createNode`). So there is
no shed-behaviour question and no staleness: this piece specifies nothing PLAN.md
holds as an open question or future intent.

There are **no `NO SPEC:` markers** in `dialectica-ui/` or `.github/`. Given
`skip_specs`, their absence is consistent rather than a gap.

## What is clean, measured rather than read

- **`check_bindings` is genuinely pinned from both bounds, and its self-test
  genuinely fails.** Mutating the pattern to `ReferenceErrorZZZ` /
  `Unable to assignZZZ` — the "narrowed until nothing matches" shape — makes
  `tst_check_bindings.sh` report **3 case(s) failed, exit 1**, naming each
  must-catch case, while the three must-accept cases stay green. That is a
  discriminating instrument, not a vacuous one, and the piece was right to build
  it.
- **The runner catches the runtime symptom in components no spec asserts
  against.** `DTheme.noSuchToken` substituted for `DTheme.note` in
  `MarginNote.qml` (a `font:` binding, a different QMetaType from the `color:`
  one the dev-writer measured): runner **exit 1**, `Unable to assign [undefined]
  to QFont`, 36 lines reported. Same in `ScreenFrame.qml`: **exit 1**. `tasks.md`
  6.3's table reproduces.
- **The gate's failing case reproduces exactly.** Run against the `main`
  checkout: exit 1, arm 1 firing on the un-prefixed `qmldir` singleton and 116
  bare-`Theme` lines including the 3 in `tests/tst_identicon.qml`. Against this
  tree: exit 0, `ok: 17 QML file(s) checked`. Both bounds, as `design.md` claims.
- **`tst_identicon.qml`'s palette assertions survived the rename with their
  teeth.** They pin ink *indexing* against named `DTheme` roles rather than
  merely that the inks differ, and the file's own comment records the rotation
  mutation that motivated it. That is the one place in the suite where a stale
  singleton reference is inside a `compare()` and therefore fails a spec.

---

- [ ] **`tester`** — `dialectica-ui/tests/run-qml-tests.sh:171` vs
      `dialectica-ui/tests/tst_check_bindings.sh` — the self-test pins the
      check but not the corpus it runs over, so gutting the corpus-builder
      leaves all six cases green while the suite goes blind
      **Scenario:** the brief's predicted failure, and it reproduces here. Add
      one line after the runner invocation in `run-qml-tests.sh`'s spec loop:
      `: > "$out"` — the runner still executes every spec, still prints every
      `---` header, still writes the full log to the CI console via `cat "$out"`
      before the truncation is visible in the check. Then break a real binding
      (`color: DTheme.noSuchPaper` in `ScreenFrame.qml`, a binding
      `tst_feed_states` instantiates and whose QWARNs I watched the unmutated
      runner catch at exit 1). Measured:
      - `run-qml-tests.sh` → **exit 0**. A genuinely broken binding passes.
      - `tst_check_bindings.sh` → **all six cases passed, exit 0.** It supplies
        its own fixtures from `mktemp`, so it never touches `$out` and cannot
        see that `$out` is now empty.
      - the `every QML spec file actually ran` step → **green**, because it
        counts `^--- tst_` lines and those are still printed.
      All three CI steps that are supposed to cover this pass. This is precisely
      the shape the self-test's own header warns about — "a check narrowed until
      it matches nothing passes every run just as quietly as a correct one" —
      applied one level up: the check is not narrowed, its *input* is.
      **The fix is one case, not a redesign.** `tst_check_bindings.sh` needs a
      seventh assertion that runs the real `run-qml-tests.sh` against a
      deliberately broken component and requires exit 1 — an end-to-end case
      alongside the six fixture cases. Without it the corpus is the one link in
      the chain nothing measures, and it is the link a future edit to the spec
      loop (adding a filter, redirecting a stream, changing where `$out` is
      written) will break silently.
      **Severity: high.** The runner check is half of `design.md`'s "genuinely
      two directions" claim, and this is the mutation that removes it with every
      gate green.

- [ ] **`tester`** — `.github/workflows/ci.yml:648-757` — the static gate is the
      piece's primary instrument and is the only one in the change with no test,
      so narrowing its walker to near-nothing passes every gate in the repo
      **Scenario:** `check_bindings` got `tst_check_bindings.sh` on the stated
      principle that "a helper is part of the measurement". The gate is not a
      helper, it is *the* gate — `design.md:59-62` calls it the answer to a
      defect no component test can see — and it has no equivalent. Its
      "proven to fail" evidence (`tasks.md` 2.2, 4.8) is a narrative of runs
      someone did once, not an artifact CI re-runs.
      Measured: I extracted the step's script verbatim and confirmed the copy is
      faithful in both directions (exit 0 with `ok: 17 QML file(s) checked` on
      this tree; exit 1 with 116 error lines on the `main` checkout). Then one
      mutation — `rglob("*.qml")` → `rglob("Core.qml")`:
      - against this tree: **`ok: 1 QML file(s) checked, every singleton
        D-prefixed, no bare Theme reference`, exit 0.**
      Arm 3 now reads one file that contains no theme reference at all, so it is
      vacuous, and the gate says `ok`. Nothing in the repo fails. The `17` → `1`
      in the message is the only signal, and **no test asserts on that number** —
      `tasks.md` 4.5 introduced the count specifically "so a glob that silently
      stops matching is visible rather than silent", but visible-in-a-log is not
      the same as gated, and a reviewer does not read a green step's stdout.
      The `if not qml_files` guard catches only the walker going to *zero*; a
      walker narrowed to one harmless file sails past it.
      **The fix is a fixture pair, and the piece already knows the shape.** A
      `dialectica-ui/tests/tst_qml_name_gate.sh` holding two tiny `qmldir` +
      `.qml` corpora — one clean, one carrying the old name and an un-prefixed
      singleton — with the gate required to exit 0 on the first and 1 on the
      second, and the script lifted out of the heredoc so both CI and the test
      call the same code. That also fixes the gate being untestable-by-
      construction: inline in a workflow `run:` block, nothing can call it.
      **Severity: high.** The vacuity mutation is exactly "tightened until
      green", and this gate is the one instrument standing between the repo and
      a repeat of the outage.

- [ ] **`tester`** — `dialectica-ui/src/qml/Main.qml` — no gate in the repo
      covers the top-level component, so a broken token binding in the screen
      the user actually sees passes the suite, the static gate and qmllint
      **Scenario:** `design.md:148-151` states the suite "now fails on any
      binding that evaluates to `undefined` at runtime, **in any component it
      instantiates**". The qualifier is load-bearing and the document does not
      say which components those are. `Main.qml` is not one of them — no spec
      instantiates it. Measured, with `color: DTheme.desk` on line 36 changed to
      `color: DTheme.noSuchDesk`:
      - `run-qml-tests.sh` → **exit 0**, 41 passed, 0 failed, zero diagnostics.
      - the static gate → **exit 0**, `ok: 17 QML file(s) checked`. A D-prefixed
        typo contains no bare `Theme`, which the piece already documents.
      - `qmllint --unqualified disable -I …` over all 13 `src/qml` files, the
        exact CI invocation → prints
        `Warning: Main.qml:36:23: Member "noSuchDesk" not found on type "DTheme"
        [missing-property]` and **exits 0**. The CI step gates on
        `rc -ne 0` (`ci.yml:565`), so the warning is printed into a green log
        and nobody reads it.
      Three gates green over a binding that renders the desk as an unstyled
      default — the same class of symptom as the outage, in the one file that is
      the whole screen's ground.
      **This is not the host-collision blind spot and must not be filed under
      it.** `design.md:158-159` correctly records that a *wrong-but-defined*
      value is invisible; this is an *undefined* value, which the piece claims
      is covered. The gap is coverage of `Main.qml`, not an impossibility.
      **Severity: high**, and it is the cheapest of the three to close — see the
      next box.

- [ ] **`tester`** — `.github/workflows/ci.yml:561` — `qmllint` already detects
      every undefined-token defect in this change and is configured to exit 0 on
      all of them; escalating one category closes the `Main.qml` gap in one flag
      **Scenario:** a disproved impossibility, of the kind this repo has paid to
      record. `ci.yml:595-601`, `CLAUDE.md`, `DTheme.qml:33-37` and
      `design.md:52-62` all argue that the component layer is "structurally
      blind" here and a static *name* check is the only instrument available.
      That is true of the host collision. It is **not** true of the undefined
      tokens the collision produces, and qmllint sees those today — it just does
      not fail on them. Measured, same `Main.qml:36` mutation:
      - `qmllint --unqualified disable --missing-property error -I …
        src/qml/Main.qml` → `Error: … Member "noSuchDesk" not found on type
        "DTheme" [missing-property]`, **exit 255**.
      - the same flag over all 13 `src/qml` files on the **unmutated** tree →
        **exit 0**. Pinned from both bounds; it is not a flag that reddens the
        tree.
      One flag on an existing step turns the `Main.qml` hole, the `DThemeTypo`
      hole and every other missing-token hole from "green with a warning" into a
      failure, statically, without instantiating anything — and it reaches every
      file in `src/qml/`, not only the ones a spec happens to construct.
      **What this does not do**, so the box is not oversold: it does not see the
      host collision (the piece is right that nothing in CI does), and it does
      not see a wrong-but-defined value. It closes the undefined-token class,
      which is the class the piece claims to have closed and has closed only for
      instantiated components.
      **Severity: medium.** The defect is the coverage gap in the box above;
      this box is the available fix and the disproved "only a name check is
      possible" framing that kept it from being found. Whoever takes this should
      also correct that framing in the four places it appears, since leaving it
      is what makes the next person stop looking.

- [ ] **`dev-writer`** — `dialectica-ui/src/qml/DTheme.qml:33-35` — the
      "a check that cannot fail" claim is stated more broadly than it measured,
      and as written it would discourage the assertion that *would* work
      **Scenario:** the header says "Under `qmltestrunner` the host is simply
      absent, so `verify(DTheme.paper !== undefined)` passes whatever this file
      is called — a check that cannot fail." The second clause is false as
      written. I wrote that exact assertion into a probe spec and ran it against
      this tree's `src/qml` import path: it passes. But it passes *because the
      qmldir declares `DTheme`* — a companion assertion in the same spec
      confirms that a name the `qmldir` does not declare (`Theme`) throws rather
      than resolving. So the check fails if the singleton is renamed, if its
      `qmldir` entry is dropped, or if its file goes missing; what it cannot
      fail on is the host collision specifically, because the host is absent.
      **Severity: low**, documentation precision — but it is the sentence that
      tells the next person not to bother, and `design.md:55-57`,
      `ci.yml:597-599` and `CLAUDE.md` repeat it. "Cannot fail *on the
      collision*" is the true and equally short version. The same overreach is
      what left qmllint's `missing-property` unexamined (box above), so this is
      the wording that cost something rather than a pedantic one.

## What I could not check

- **The collision itself.** I did not launch basecamp, so the claim that
  `DTheme` resolves correctly under the real host is unverified here. The piece
  is explicit that no gate covers this and that a real launch is required; I am
  recording that the gap is still open rather than disputing it.
- **Whether the gate's shipped heredoc and my extracted copy diverge in any way
  my two-direction check did not exercise.** I confirmed identical output on the
  clean tree and on `main`; a difference that shows only on a third input would
  not have surfaced. This is itself an argument for the gate being a callable
  script rather than an inline heredoc, which the second box asks for.

## Mutations run, and what each returned

| mutation | suite | static gate | qmllint | self-test |
|---|---|---|---|---|
| unmutated baseline | exit 0, 41 passed | exit 0, 17 files | exit 0 | 6/6 pass |
| `MarginNote.qml` `font: DTheme.noSuchToken` | **exit 1** | exit 0 | — | — |
| `ScreenFrame.qml` `DTheme.noSuchCardWidth` + `noSuchPaper` | **exit 1** | exit 0 | — | — |
| `Main.qml` `color: DTheme.noSuchDesk` | exit 0 | exit 0 | exit 0 (warns) | — |
| `Main.qml` same, `--missing-property error` | — | — | **exit 255** | — |
| gate walker `rglob("*.qml")` → `rglob("Core.qml")` | — | **exit 0, 1 file** | — | — |
| `check_bindings` patterns → `…ZZZ` | — | — | — | **3 failed, exit 1** |
| `: > "$out"` in the spec loop + real broken binding | **exit 0** | exit 0 | — | **6/6 pass** |

Rows 4, 6 and 8 are the findings: a mutation that breaks the stated behaviour
and leaves every gate green. Rows 2, 3 and 7 are the piece working as
documented. Row 5 is the disproved impossibility.

After the last mutation the tree was restored and confirmed clean with
`git status --porcelain` (empty output) and `run-qml-tests.sh` re-run at exit 0.
