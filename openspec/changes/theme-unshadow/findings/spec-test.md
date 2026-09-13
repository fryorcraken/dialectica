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

- [x] **`tester`** — `dialectica-ui/tests/run-qml-tests.sh:171` vs
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

      **FIXED by `tester`, and the fix is larger than the box asked for —
      because writing the one case you specified exposed a second instance of
      the same defect, in my own test.**

      *The case.* `tst_check_bindings.sh` now ends with an end-to-end pair that
      drives the real `run-qml-tests.sh` over a staged component: the same
      component with `DTheme.paper` (must exit 0) and with
      `DTheme.noSuchTokenAtAll` (must exit 1). Both bounds, because a case
      demanding only exit 1 is satisfied by a runner that always fails.

      *Proven to fail, with the mutation you named.* `: > "$out"` in the capture
      path → **`FAIL: the real runner EXITED 0 over a binding that reads
      undefined`, exit 1**, while the original six cases stayed green. Predicted
      and observed agree exactly.

      *The second instance, which is the part worth reading.* Adding the
      `--import`/named-spec argument path gave the runner **two** capture sites,
      and my case could only drive one. Measured: `: > "$out"` in the suite loop
      left **all eight cases green**; the identical mutation in the argument
      path failed one. So the test I had just written was blind to the
      production path — the very shape this box is about, reproduced one level
      down. Fixed structurally rather than with a second case: both paths now go
      through one `run_spec()`, so there is one place to break and the test
      covers it. Re-measured after the collapse: the single-site mutation fails
      the case. One site is one thing for the test to cover, which is cheaper
      than a test per site and does not go stale when a third site is added.

      *Two things the case had to get right, both found by measurement and both
      recorded in the file so they are not re-derived.* A `property color c:` on
      an `Item` nobody reads is **lazy** — Qt never evaluates it, prints nothing,
      and the case passed over a broken token; a `Rectangle`'s own `color` is
      evaluated on creation. And a component staged in its own directory raises
      `ReferenceError: DTheme is not defined` for **every** spelling, correct
      ones included, so the case fired for a reason unrelated to the token; the
      probe is now staged inside a directory carrying the real `DTheme.qml` and
      a qmldir naming it, which is how a real component resolves the singleton.
      The first version of this case would have passed while measuring nothing
      about tokens at all.

      *One regression caught on the way out.* `run_spec` prints the
      `--- <spec>` header, and the suite loop was still printing one too —
      doubling a line CI's `every QML spec file actually ran` step counts. Fixed;
      the suite now prints one header per spec, 4 for 4 specs on disk.

- [x] **`tester`** — `.github/workflows/ci.yml:648-757` — the static gate is the
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

      **Note from `dev-writer`, box left OPEN — it is a `tester`'s to close.**
      The fix you specify is built, because the architecture boxes required the
      same extraction: the gate is now
      `dialectica-ui/tests/check_qml_names.py`, taking a module root, with
      `dialectica-ui/tests/tst_check_qml_names.py` beside it holding 16 cases
      over constructed `qmldir` + `.qml` corpora — both bounds, with every
      rejection case also asserting on the message text. It runs as its own
      `lint` step, ahead of the gate itself.

      **Your vacuity mutation is caught.** I ran exactly it — `rglob("*.qml")`
      → `rglob("Core.qml")` — against the new suite: **5 of 16 cases fail.**
      Worth noting *which* passed: `the shipped dialectica-ui module` still
      passed, because the real module contains a `Core.qml`. Only the
      constructed corpora expose the narrowing, which is your argument for a
      fixture pair, confirmed.

      The box stays unticked because it asks for a test proven to fail, and a
      `dev-writer` proving its own gate is the thing the stage split exists to
      prevent. What is left is verification, not construction. Also settled by
      the extraction: your "What I could not check" item about the heredoc and
      your extracted copy diverging — there is no heredoc now, and CI and the
      test call the same file.

      **VERIFIED and CLOSED by `tester`. The `dev-writer`'s measurement
      reproduces exactly, and the gap it names is now closed.**

      *Reproduced, not taken on trust.* I ran the vacuity mutation myself —
      `rglob("*.qml")` → `rglob("Core.qml")` — against the 16-case suite:
      **5 failed**, and `the shipped dialectica-ui module` **passed**, for the
      reason given (the real module contains a `Core.qml`). Against the shipped
      module directly the gate printed `ok: 1 QML file(s) … checked` where 17
      exist. Exactly as reported.

      *The remaining gap, and why it needed closing.* That case asserted only
      exit 0, so a walker narrowed to one harmless file satisfied it. The count
      was printed and nothing compared it to anything — "visible in a log is not
      the same as gated", which is your original wording and was still true of
      the rebuilt gate. `tst_check_qml_names.py` now carries a second
      shipped-module case that **counts the `.qml` files from disk itself** and
      requires the gate to report that many. The floor is derived independently:
      asking the gate how many files it saw and agreeing with the answer is the
      implementation confirming itself, not a measurement — two independent
      walks of the same tree must agree.

      *Proven to fail.* Re-running the same vacuity mutation now fails **6 of
      17**, and the sixth is the shipped-module count, with a diagnostic naming
      the narrowing: `17 .qml file(s) on disk but the gate did not report
      checking that many — the walk is narrower than the tree, so most files
      went unchecked`. Predicted 5-plus-the-new-one; observed exactly that.

      *Note on the other corpus-builder.* The `dev-writer`'s `qmldir_entries`
      result (12 of 16, 7 of them only because the rejection cases assert on
      message text) is the standard I held the new cases to: every rejection
      case I added asserts on the message, not merely on a non-zero exit.

- [x] **`tester`** — `dialectica-ui/src/qml/Main.qml` — no gate in the repo
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

      **FIXED by `tester`, closed by the gate built for the box below.**

      *Your measurement reproduces in full.* With `Main.qml:36` rewritten as
      `color: DTheme.noSuchDesk` on this tree: the QML suite **exit 0, 41
      passed, zero diagnostics**; the static name gate **exit 0, `17 QML
      file(s)`**; `qmllint` as CI invokes it **exit 0** while printing
      `Warning: Main.qml:36:23: Member "noSuchDesk" not found on type "DTheme"
      [missing-property]`. Three gates green over the binding that paints the
      whole screen's ground.

      *Closed.* `dialectica-ui/tests/check_qml_members.sh` (built for the next
      box) runs over every file in `src/qml`, needs no spec to instantiate
      anything, and therefore reaches `Main.qml`. Against the same mutation it
      **exits 1**, naming file, line and member; `tst_check_qml_members.sh`
      fails alongside it because its shipped-view case goes red. Restored, both
      green.

      *The `design.md` qualifier is corrected rather than left to be
      rediscovered.* "in any component it instantiates" now reads "in the
      components a spec instantiates", with your `Main.qml` measurement recorded
      underneath it as the reason the qualifier is load-bearing — and the member
      gate written up as the third direction that covers what the other two
      cannot.

- [x] **`tester`** — `.github/workflows/ci.yml:561` — `qmllint` already detects
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

      **Note from `dev-writer`, box left OPEN — it needs a test proven to fail,
      which is a `tester`'s.** Two things for whoever takes it, so neither is
      re-derived.

      **1. The framing correction is done.** All four places now say "cannot
      fail *on the collision*" with the three things the assertion *can* fail
      on — see my reply on the `DTheme.qml:33-35` box below, including the
      probe I ran to confirm your measurement.

      **2. `--missing-property error` CANNOT see the collision, measured from
      both bounds — do not record it as if it can.** On a real branch it exits
      **0** silently; with a bad member planted it exits **255**. The reason is
      CI's own `-I dialectica-ui/src/qml`, which puts the plugin's *own*
      `Theme.qml` on the import path, so qmllint resolves `Theme` to the
      correct singleton where every member genuinely exists. **It is checking a
      different resolution than the app performs.**

      Your box is still worth taking — it closes the undefined-member class you
      found, which is real and uncovered. But a green from it must not be
      described as evidence about the collision: that would be a gate whose
      green is unrelated to whether the app works, which this repo has already
      paid for once (`gate-the-defect-satisfies`). Please state the limit
      wherever the flag lands.

      **3. A QML test asserting `DTheme.cardWidth !== undefined` is the
      tempting wrong answer**, stated explicitly so it is not proposed next:
      under `qmltestrunner` there is no host namespace to collide with, so it
      passes either way. That is the same trap as (2) one layer up.

      What *would* catch the collision, and needs no new machinery, is asserting
      on the launch log: zero `Unable to assign [undefined]`, and at least one
      resolution into `dialectica_ui/qml/DTheme.qml`. The log already contains
      both signals and nobody was reading it — partly because `CLAUDE.md` gave
      the filename as `basecamp.log` when it is `basecamp_<timestamp>.log`.

      **FIXED by `tester`. The flag is now a gate with a test that fails.**

      *Built as a script, not a flag in `ci.yml`.* A gate inline in a workflow
      `run:` block cannot be called, so it cannot be tested — the defect that
      let two bugs through the previous name gate, and I was not going to
      reintroduce it one box after it was fixed.
      `dialectica-ui/tests/check_qml_members.sh` runs
      `qmllint --unqualified disable --missing-property error -I src/qml` over
      every file, with `tst_check_qml_members.sh` beside it and both wired into
      the `qml` job ahead of the existing `qmllint` step.

      *Both bounds, on the real view and on constructed corpora.* Shipped
      `src/qml` → **exit 0, 13 files**. A staged component reading
      `DTheme.noSuchDeskAtAll` → **exit 1**, message naming the member.

      *Three mutations, each predicted then observed.*
      - Drop `--missing-property error` (i.e. revert to CI's configuration) →
        **`FAIL: the gate ACCEPTED a member that does not exist on DTheme`**.
        This is the mutation that matters: it is literally the state the repo
        was in, and the test now refuses it.
      - Narrow the corpus glob to one file → **2 cases fail**, the rejection
        case and the file-count case. The count case was added *because* the
        first narrowing run left the shipped-view case green — the same
        one-harmless-file hazard as the name gate, caught by applying that box's
        lesson here before it was reported.
      - The real `Main.qml:36` defect → gate **exit 1**, self-test red.

      *Your point 2 is honoured and not quietly dropped.* I re-measured it: with
      `-I src/qml` the flag exits 0 on a real branch and 255 with a bad member
      planted, because qmllint resolves `DTheme` to our own file — **a different
      resolution than the app performs**. The limit is written into the script's
      header, into `design.md`, and into `CLAUDE.md` beside the existing "cannot
      see this defect" sentence, each saying the same thing: it covers members,
      never the collision. Your point 3 (a QML `!== undefined` assertion) was
      not proposed and is not present.

      *The framing correction the box asks for is done where it had not been.*
      `CLAUDE.md` previously said only that `--missing-property error` cannot
      see the defect — true, and exactly the sentence that stops the next person
      looking. It now carries both halves.

- [x] **`dev-writer`** — `dialectica-ui/src/qml/DTheme.qml:33-35` — the
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

      **Fixed in all four places**, and confirmed by running your probe rather
      than accepting it. I wrote the spec you describe — `verify(DTheme.paper
      !== undefined)` plus a companion asserting an undeclared `Theme.paper`
      throws — and ran it against `dialectica-ui/src/qml` on this tree: **4
      passed, 0 failed, exit 0.** Both halves behave as you report, so the
      assertion resolves *because* the `qmldir` declares `DTheme`, and it would
      fail if that entry went away.

      (An aside worth recording for whoever writes such a probe next: the import
      form matters. With `import "."` from a scratch directory, `DTheme` does
      not resolve at all and the first assertion fails with `DTheme is not
      defined` — a false confirmation of the sentence being corrected. The real
      specs use `import "../src/qml"`, and only that form reproduces your
      measurement.)

      Your wording is adopted nearly verbatim: "cannot fail **on the
      collision**", with a following sentence naming the three things it *can*
      fail on. `DTheme.qml:33`, `design.md`'s "The gate is static" section,
      `CLAUDE.md`'s trap entry and the `ci.yml` comment all now carry the
      qualifier, and each also carries your point that the overbroad version is
      what cost the `missing-property` examination — because a wording whose
      cost is unstated is one someone trims back.

      The `tester` box above it (the `--missing-property error` flag) is **left
      open deliberately**: it needs a test proven to fail, which is not mine to
      write. Flagging one thing there for whoever takes it — your measurement is
      `qmllint … --missing-property error`, and my local Qt 6.10.3 `qmllint`
      rejects `--unqualified` outright ("Unknown option"), so the CI invocation
      and a local one may not be the same command. Worth confirming which
      binary CI's `find` step resolves before pinning the flag.

## Settled by `tester`: the `dev-writer`'s least-confident call

The `dev-writer` flagged that it gated `internal Foo Foo.qml` on the assertion
that such an entry "registers the name in the directory namespace", without
verifying against Qt's loader, and noted that if wrong the gate would be
stricter than needed. **It was wrong, and in that direction — but the gate is
still right, for a reason worth having written down.** Measured on Qt 6.10.3
with staged probe modules.

- **`internal` does NOT export the name.** A consumer doing `import <Module>`
  cannot reach it: `ProbeInternal is not a type`. The same probe resolves a
  normally-declared type from the same qmldir, so it discriminates.
- **But the name is live inside the directory.** A sibling `.qml` in the same
  directory instantiates it by that name and loads.
- **And that resolution is by filename, not by the qmldir line.** Deleting the
  `internal` entry entirely left the sibling resolving exactly as before.

Since the directory is where basecamp loads the plugin, an
`internal Theme Theme.qml` line is a reliable witness that a `Theme.qml` sits
in that directory — so checking these entries is correct, but what it catches
is the **file**, not an export. The comment in `qmldir_entries` and the test
case beside it now say that instead of the export claim, and `CLAUDE.md`
carries the measurement.

**The methodological part, which is why this is written at length.** My first
probe passed — and passed with the `internal` line deleted. It was measuring
same-directory filename resolution and would have "confirmed" the export claim
while testing nothing about it: a fixture where two explanations give the same
answer, this repo's one recurring test defect, produced while deliberately
hunting for it. Only import-by-module-name distinguishes the two. Anyone
re-opening this question should start there.

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
