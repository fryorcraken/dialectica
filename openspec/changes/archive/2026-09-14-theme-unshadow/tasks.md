## Stages

- [ ] ~~spec — `spec-writer`~~ — **does not apply.** Renaming an identifier adds
      no requirement: no token changes name, value or meaning, and nothing a view
      may claim or must refuse to claim moves. `.openspec.yaml` sets
      `skip_specs: true` with that reason. Struck through rather than omitted,
      because "does not apply" and "nobody did this" are different states and the
      block exists to tell them apart.
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [x] findings all ticked, `findings/` deleted — runner
- [x] `openspec validate --strict`, then `archive` — runner

**The `tester` row covers two passes**, and section 7 is the second. The first
built `check_bindings` and its test (section 6). The second came back after
review to close the four `tester` boxes in `findings/spec-test.md`, which is
where a stage is *supposed* to be re-entered rather than handed to whoever is
nearest: three of those four boxes are about gates measuring nothing, and a
`dev-writer` proving its own gate is what the stage split exists to prevent.

**Note for whoever takes the two `tester` boxes in `findings/correctness.md`,
so neither is re-done from scratch:**

- The **first** box (the reference arm's regex missing a column-0 `Theme.`) is
  **already fixed in the code** — `tasks.md` 4.4, anchored to
  `(?<![A-Za-z])Theme\b`, with both of the reviewer's mutations re-run against
  the new gate and caught. The box stays unticked because it is not
  `dev-writer`'s to tick and a `tester` should confirm the fix independently
  rather than inherit my measurement. What is left there is verification, not
  repair.
- The **second** box (a `ReferenceError` in an instantiated component is a QWARN,
  not a failure) is **untouched and is real work.** It lives in
  `run-qml-tests.sh`, which this piece does not modify. `design.md`'s false
  "covered from both directions" claim — which that box is the evidence for —
  **is** corrected here, under "What actually checks the rename, and what does
  not", since the prose was mine even though the runner fix is not.

## 1. The rename

- [x] 1.1 `git mv dialectica-ui/src/qml/Theme.qml dialectica-ui/src/qml/DTheme.qml`
      — recorded by git as a rename, so the file's history follows it.
- [x] 1.2 `qmldir` line 1 becomes `singleton DTheme 1.0 DTheme.qml` — verified by
      reading the file, and by the CI gate's qmldir arm, which fires on the old
      text and is silent on the new.
- [x] 1.3 Update every reference in the view. **Eleven component files**:
      `AddressLabel`, `ApparatusColumn`, `FeedScreen`, `FlatButton`, `Identicon`,
      `Main`, `MarginNote`, `PostHeader`, `SanitisedText`, `ScreenFrame`,
      `VoteControl`. Verified by `grep -rn "Theme"` over `dialectica-ui/`, which
      now returns only `DTheme.` references plus the six lines in `DTheme.qml`'s
      own header comment that discuss the old name on purpose.
- [x] 1.4 Update `dialectica-ui/tests/tst_identicon.qml`, which reads six palette
      constants through the singleton to pin the identicon's ink indexing — four
      assertion lines and one comment.
- [x] 1.5 Fix the two prose mentions the dotted pattern does not reach:
      `Identicon.qml:73` ("frozen constants in DTheme") and `tst_identicon.qml:63`
      ("the named DTheme roles"). Found by grepping bare `Theme`, not `Theme.`,
      which is the grep that catches a comment left behind.
- [x] 1.6 Give `DTheme.qml` a header explaining why the name has a `D` — including
      what a component test structurally cannot see, so the next person to reach
      for one reads the measurement first.

## 2. The gate

- [x] 2.1 Add `no QML type name collides with the host` to
      `.github/workflows/ci.yml`.
      **Superseded — this row describes a gate that no longer exists**, and is
      kept rather than rewritten because a ticked row that quietly changes its
      meaning is worse than one that says what it used to claim. It said "three
      arms: a `.qml` file named after a host type, a `qmldir` entry declaring
      one, and a bare `Theme.` reference". There was never a **filename** arm —
      the script reads `qmldir` and the `.qml` bodies, never a `.qml` basename.
      It also placed the step in the `qml` job. The shipped arms and their home
      are in 4.7, 4.9, 4.10 and 4.11 below; `design.md` carries the reasoning.
- [x] 2.2 **Prove it fails.** Run against `/home/fryorcraken/src/rad/dialectica`
      (the `main` checkout, which carries the defect): exit 1, with all three arms
      firing — `'Theme' is a type name basecamp also registers`, `qmldir declares
      'Theme'`, and the bare-reference list. Run against this worktree: exit 0,
      `ok: no QML type name collides with the host`. A gate that has only ever been
      seen green proves nothing, which is why this was measured both ways.
- [x] 2.3 Confirm the comment-exclusion arm is exercised rather than dead —
      `grep -n "[^A-Za-z]Theme\."` over `DTheme.qml` returns two comment lines, so
      without the comment handling the gate would be permanently red on a correct
      tree.
      **Superseded in its mechanism, not its finding.** This row said "without
      the `grep -v`"; `grep -n "grep -v"` over `ci.yml` now returns nothing,
      because 4.6 replaced that filter with `strip_comments`. The observation
      still holds and is now pinned from the accepting side as a test case
      ("a bare Theme inside //, /* */ and trailing comments") rather than
      confirmed by hand.
- [x] 2.4 Document the third arm's known false positive rather than leaving it to
      be rediscovered: only a **leading** `//` is excluded, so a bare `Theme.` in a
      `/* */` block comment or in a trailing comment after code fails the arm while
      being correct. Not fixed — narrowing to real bindings needs a QML parser,
      which is the elaborate thing this gate deliberately is not. The step's
      comment now says a red should be checked against the cited line first, so a
      comment-style failure is diagnosed in seconds rather than investigated as a
      collision.
      **Superseded: the false positive is gone rather than documented.** 4.6
      replaced the leading-`//` exclusion with `strip_comments`, which handles
      block and trailing comments too, so there is no longer a correct tree this
      arm reddens. The cross-reference this row used to carry pointed at a
      `design.md` heading the 4.6 rewrite renamed; the live section is
      "Two comment-strippers, and the claim narrowed to match".

## 3. Making the failure visible, and what could not be made visible

- [x] 3.1 Reproduce the symptom: a staged copy of the view with a token-less
      `Theme.qml` gives `ScreenFrame.implicitWidth` **0** and `color #ffffff`
      instead of paper `#efe9dc`, against 1000 and `#efe9dc` on the real tree.
      This shows what happens once the singleton carries no tokens.
- [x] 3.2 Try to reproduce the *mechanism* under `qmltestrunner`, and fail —
      twice. A competing `Theme` singleton on the runner's `-import` path does not
      shadow the plugin directory's own `qmldir` entry, and neither does making
      that directory a named module on the import path. Both measured on Qt 6.10.3;
      both left `implicitWidth` at 1000. Recorded in `design.md` under "Dead ends"
      so the next person does not spend the same afternoon.
- [x] 3.3 State the honest limit rather than claim a gate that measured nothing:
      **no component test in this repo can see this collision**, because it lives
      in basecamp's C++ type registration. The test layer covers that every
      reference resolves and that the identicon indexing is unmoved; it cannot
      cover the shadowing itself.
- [x] 3.4 Make the measured mechanism the **rationale** everywhere, not a later
      finding. The first draft of this change opened four documents with the
      premise 3.2 disproved — that a host registration *outranks* a plugin
      directory's `qmldir` entry — and stated the true mechanism eleven lines
      further down in the same block. A reader stopping at the first paragraph,
      which is where a rationale is expected to live, left with the wrong model,
      and the file looked authoritative in both directions. Found by the closer
      reviewing PR #67. Fixed in `ci.yml`, `CLAUDE.md`, `DTheme.qml`,
      `proposal.md` and `design.md`; the premise is kept in each as a
      **named-and-withdrawn** one, because it is the intuitive wrong answer and
      acting on it means building a reproduction that cannot work. Verified by
      `git grep -n "outrank"`, whose every hit in this change is now inside an
      explicit withdrawal.

## 4. Gates

- [x] 4.1 `dialectica-ui/tests/run-qml-tests.sh` — **41 passed, 0 failed**, across
      4 spec files (`tst_core_call` 13, `tst_feed_states` 12, `tst_identicon` 7,
      `tst_sanitised_text` 9).
      **An earlier version of this row overclaimed and is corrected**: it said a
      missed reference "would fail with `Theme is not defined` rather than pass
      with a wrong value". That is true only where the reference sits inside a
      `compare()`. `qmltestrunner` reports a `ReferenceError` in an instantiated
      component as a QWARN, so the same stale reference in `MarginNote.qml`
      leaves the suite green — measured, see 4.4. The suite's real contribution
      here is pinning the identicon ink indexing; the CI gate is what checks the
      rename's completeness.
- [x] 4.2 `qmllint --unqualified disable -I <qml dir>` over the **13 files in
      `dialectica-ui/src/qml/`** — clean, no output. The flag is the repo's
      existing one, for the `logosModule` bridge qmllint cannot see; it was not
      widened for this change.
      **Scope corrected**: this row previously said "all thirteen QML files",
      describing `src/qml/*.qml` as if it were the whole module. The module holds
      **17** — 13 under `src/qml/` plus 4 under `tests/` — verified by
      `find dialectica-ui -name "*.qml"` and by `git ls-tree -r main`, which
      shows the same 13 + 4 split (`main` has `Theme.qml` where this tree has
      `DTheme.qml`, so the count is 17 on both). The review's figure of 16 is one
      low. This is the same off-by-scope that produced the gate gap in 4.5, which
      is why it was worth chasing to an exact number rather than softening the
      wording.
- [x] 4.3 `git diff origin/main --stat` read before committing, and every listed
      file confirmed as one this change meant to touch.

## 4b. Rebuilding the gate after correctness and security review

- [x] 4.4 **Anchor the reference check**, closing the finding's headline defect.
      The old regex `[^A-Za-z]Theme\.` required a character before `Theme`, so a
      binding split across two lines with `Theme.note` at column 0 was invisible
      to it. Reproduced before fixing: the gate's arm returned **0 matches, exit
      0**, `qmllint` **exit 0**, and `run-qml-tests.sh` **41 passed, exit 0** —
      three green gates over a genuinely broken binding. The second form,
      `Theme` split from `.ink` across a newline, was equally invisible.
      Now `(?<![A-Za-z])Theme\b`; both mutations are caught at
      `MarginNote.qml:40`, and both were re-run after the fix to confirm it.
- [x] 4.5 **Widen the scope to every QML file in the module.** The old glob read
      `src/qml/*.qml` only, leaving `dialectica-ui/tests/` outside the gate
      entirely — verified by restoring the old name in `tst_identicon.qml:75`,
      which left the gate at 0 matches, exit 0. Now an `rglob("*.qml")` over
      `dialectica-ui/`, which also reaches a subdirectory under `src/qml/` should
      one ever be added. Same mutation now fails at
      `tests/tst_identicon.qml:75`. The gate reports the file count it checked,
      so a glob that silently stops matching is visible rather than silent.
- [x] 4.6 **Strip comments once, up front**, rather than filtering per-check —
      `piece/publish-envelope`'s shape, adopted because there is one place to be
      right about what a comment is and the next check inherits it. This also
      **removes the false positive** the previous version documented rather than
      fixed: a bare `Theme.` in a `/* */` block comment and one trailing a line
      of code both now pass, where the old `grep -v` on a leading `//` caught
      neither. Measured both forms. Block comments are replaced by the newlines
      they spanned so reported line numbers stay honest.
- [x] 4.7 **Enforce the prefix convention instead of enumerating host names**,
      closing the security box. The old arm banned five names basecamp was known
      to occupy; the gate now requires every `singleton` in `qmldir` to be
      `D`-prefixed, which is total over host registrations that have not happened
      yet. `Core` is grandfathered **with its reason written in the step** — it
      predates the convention, this piece deliberately did not rename it, and it
      is itself the evidence that the host does not claim every name. The
      comment says explicitly that it is a grandfather clause and not a
      precedent, and that the fix for a second name is the `D`, not a second
      exemption.
- [x] 4.8 **Re-prove the whole gate both ways after the rewrite.** Against the
      `main` checkout: exit 1, reporting the un-prefixed `qmldir` singleton and
      **116** bare-reference lines — 113 under `src/qml/` (matching the review's
      independent count exactly) plus the 3 in `tests/` the old scope could not
      see. Against this tree: exit 0, `ok: 17 QML file(s) checked`. Suite still
      41 passed; `qmllint` still clean; working tree confirmed byte-clean after
      every mutation via `git status --short`.

## 4c. Closing the architecture and readability findings

- [x] 4.9 **The prefix rule reads every `qmldir` entry, not only `singleton`
      lines** — the high-severity architecture box. Confirmed before fixing
      rather than inherited: `Theme 1.0 Identicon.qml` added to `qmldir`, the
      pre-fix gate re-run from a verbatim copy of its logic, **exit 0, `ok: 17
      QML file(s) checked`** over a `qmldir` declaring a type named `Theme`.
      The widened gate exits 1 at `line=13` on the same tree. The eleven
      existing component names enter `GRANDFATHERED` explicitly, which trades
      one silent gap for eleven enumerated ones; `design.md` records why that,
      and not eleven renames, is this piece's scope.
- [x] 4.10 **The stale-reference arm derives its names from `qmldir`** rather
      than hardcoding `Theme`. Measured: renaming `Core` to `DCore` leaves 33
      bare `Core.` references across `FeedScreen.qml` and three specs, and the
      pre-fix gate reports **exit 0** on every one. The set is seeded from the
      rejected names too — without that the gate against `main` reported the
      `qmldir` line and fell silent about the bodies. Re-measured against
      `main` after both changes: **117 error lines = 1 qmldir + 116 bare
      references, 3 of them in `tests/tst_identicon.qml`**, which is exactly
      the figure and split `design.md` has carried throughout.
- [x] 4.11 **The gate is a script with its own tests, and moved to `lint`.**
      `dialectica-ui/tests/check_qml_names.py` takes a module root;
      `tst_check_qml_names.py` beside it holds 16 cases. It needs no Qt, so it
      sits in `lint` next to `no QML component shadows a Qt built-in` — the
      same question asked of a different namespace — instead of behind a Qt6
      `apt-get`. Workflow re-parsed with `yaml.safe_load` to confirm both jobs
      are intact and the step order is what it reads as.
- [x] 4.12 **Prove the test can fail, in the ways a tuner would break it.**
      Six mutations, each applied and reverted: entry walk narrowed back to
      `singleton` → 3 fail; file walk narrowed to `src/qml/*.qml` → 1 fails;
      reference arm hardcoded back to `Theme` → 2 fail; `Theme` added to
      `GRANDFATHERED` → 3 fail; rejected-name seeding removed → 1 fails; and
      **`qmldir_entries` stubbed to return nothing → 12 of 16 fail**, which is
      the one that matters. Seven of those twelve fail *because* each rejection
      case also asserts on the message text: without that assertion they would
      have counted as correctly-rejected while the gate measured nothing. That
      is the defect family this repo has shipped before — a filter pinned from
      both sides whose corpus-builder was then emptied with every test green.
- [x] 4.13 **Scope the grandfather clause where the wrong move is made.** The
      set is consulted by the prefix rule alone, so a reader hitting a red from
      the reference arm could add a name to it and change nothing, silently.
      The comment now states that it exempts a name from the prefix rule only,
      and that nothing exempts a missing file or a bare reference.
- [x] 4.14 **Cut the fourth copy of the narrative.** The step carried ~108
      lines of comment for ~50 of code, retelling what `CLAUDE.md`,
      `DTheme.qml` and `design.md` already say — four copies that must change
      together. `ci.yml` now carries what a reader of a red needs (why static,
      why a prefix, why it is a script, why it is in `lint`) and points at
      `CLAUDE.md`'s trap entry for the mechanism, which is where a person
      adding a singleton meets it.

## 5. Documentation

- [x] 5.1 Record the trap in `CLAUDE.md` under "Module contract traps", which is
      where build- and launch-time failures of this kind are collected. Includes
      the tell in a launch log (`qrc:/qt/qml/Logos/` for a name we own), why
      `Core` working is not evidence, and why the gate is static.
- [x] 5.2 `docs/UI-BRIEF.md` checked for whether this change makes it wrong — it
      does not. The brief states what the UI must show, hide and refuse to claim;
      it names no QML identifier, so a singleton rename leaves every obligation in
      it true. Verified by `grep -n "Theme"` over that file, which returns nothing.
- [x] 5.3 Fix the five stale references in `docs/IDENTICON.md` (lines 84, 333,
      804, 811, 818), which name `Theme.qml`, `Theme.markInFeed` and `Theme.mark*`
      while recording the palette's frozen-constant obligation. **Found only by
      widening the sweep past `dialectica-ui/`** — the brief for this piece pointed
      at the UI directory, and checking UI-BRIEF alone would have left a document
      about the identicon palette pointing at a filename that no longer exists.
      Verified by `git grep -n "Theme"` over the whole tracked tree, which now
      returns only `DTheme` references plus the comment lines in `ci.yml` and
      `DTheme.qml` that discuss the old name on purpose.

## 6. Tests — the `tester` stage

Both boxes in `findings/correctness.md` addressed to `tester`. Implementation
code was mutated to prove each check can fail and restored after each; the tree
was confirmed byte-clean with `git status --porcelain` and
`git diff -- dialectica-ui/src .github docs dialectica openspec CLAUDE.md`
before committing, which lists nothing.

- [x] 6.1 **Confirm 4.4's anchored regex independently**, rather than inherit
      `dev-writer`'s measurement — the reason that box stayed open. Both of the
      review's mutations re-applied to `MarginNote.qml` and the gate run against
      each: `font:` / newline / `Theme.note` at column 0 → **exit 1** at
      `line=40`; `color: Theme` / newline / `.inkSoft` → **exit 1** at `line=40`.
      Unmutated tree → **exit 0**, `ok: 17 QML file(s) checked`. The exit-0 run
      is the half that carries the weight: the tree is full of legitimate
      `DTheme.` references, so a pattern firing on everything would have "caught"
      both mutations too. Each mutation was confirmed present with `git diff
      --stat` before the gate ran, since a mutation that fails to apply looks
      exactly like a gate that fails to catch it.
- [x] 6.2 **Make the suite fail on a binding that evaluates to `undefined`** —
      `check_bindings` in `run-qml-tests.sh`. The runner previously reported a
      QML runtime error inside an instantiated component as a QWARN and exited
      0; reproduced first, 35 `ReferenceError: Theme is not defined` lines with
      `12 passed, 0 failed`. The fix belongs in the runner rather than in the
      specs because the defect is in components no spec asserts against, so
      per-component assertions would be a sweep list going stale on the next
      component added. Not `QT_FATAL_WARNINGS`: it aborts on the first warning
      of any kind, so the run crashes instead of diagnosing and the remaining
      specs never execute. `qmltestrunner -help` lists no flag that escalates a
      warning to a failure.
- [x] 6.3 **Two patterns, because one is provably insufficient.** A check keyed
      only on `ReferenceError` — what the finding asked for — still passes a
      broken binding: `DTheme.noSuchToken` raises **no `ReferenceError` at all**
      and reports `Unable to assign [undefined] to QColor`, with the suite at 12
      passed, 0 failed. Measured across three mutations, with the static gate's
      verdict beside the runner's:
      | mutation | ci.yml gate | `ReferenceError`s | runner |
      |---|---|---|---|
      | `Theme.note` at column 0 | exit 1 | 36 | exit 1 |
      | `DThemeTypo.note` | **exit 0** | 36 | exit 1 |
      | `DTheme.noSuchToken` | **exit 0** | **0** | exit 1 |
      | unmutated | exit 0 | 0 | exit 0 |
      Rows 2 and 3 are the independence evidence `design.md` previously claimed
      without: the static gate passes both, so the runner check covers strictly
      more than the gate rather than duplicating it.
- [x] 6.4 **Test the check itself**, in `dialectica-ui/tests/tst_check_bindings.sh`
      with its own CI step ahead of the suite. A helper is part of the
      measurement: narrowed to nothing it passes as quietly as a correct one.
      Six cases, both directions — three verbatim `qmltestrunner` outputs it
      must reject, three it must accept (a real clean run, an unrelated Qt
      "undefined behaviour" warning, and test names containing the word
      `undefined`, of which this suite already has two). Fixtures are copied
      output rather than paraphrase, so the test measures what Qt prints rather
      than what the regex expects.
      Proved by mutating the check: dropping the `Unable to assign` pattern
      fails case 3 alone; widening to the bare word `undefined` fails **four**
      cases where I predicted two. The prediction was wrong in an informative
      way — `ReferenceError` text reads "is not **defined**", so a
      bare-`undefined` pattern misses that family entirely. The two families
      share no common substring, which is the concrete reason two precise
      patterns cannot collapse into one loose one; recorded in the runner's
      comment so it is not re-derived.
      The extraction guard was itself proved to fire: renaming `check_bindings`
      makes the test exit 1 with its named "could not extract" diagnostic rather
      than a confusing shell error.
- [x] 6.5 **What these gates still cannot see**, written down rather than left
      to be rediscovered. A binding resolving to a *wrong but defined* value
      (`DTheme.paper` where `DTheme.ink` was meant) emits no diagnostic and is
      invisible to every gate here. And the host collision itself remains
      unreproducible under `qmltestrunner`, where basecamp is absent — 6.2
      closes the **runtime** half of the blind spot, not the host-precedence
      half, which stays a static check plus a real basecamp launch. The suite is
      now a genuine second direction for stale and undefined references, which
      is what `design.md`'s corrected prose claims, and nothing more.

## 7. Tests, second pass — closing the `tester` boxes from review

All four `tester` boxes in `findings/spec-test.md`. Three are the same defect
wearing three hats: a gate whose *input* can go empty while the gate itself
still looks correct. Each fix below is proven with a mutation that reaches it.

- [x] 7.1 **The runner's corpus is measured end to end.**
      `tst_check_bindings.sh` gained a two-bound case that drives the real
      `run-qml-tests.sh` over a staged component — sound token must exit 0,
      missing token must exit 1. Proved with the reviewer's own mutation,
      `: > "$out"`: the case fails, the original six stay green.
- [x] 7.2 **The runner has ONE capture site, because the test can only drive
      one.** Adding the named-spec argument path created a second, and measured:
      gutting the suite loop left every case green while gutting the argument
      path failed one — the finding reproduced inside its own fix. Both paths
      now go through `run_spec()`. Re-measured after the collapse: the mutation
      fails the case. Structural rather than a second test case, so a third call
      site inherits the coverage instead of needing its own.
- [x] 7.3 **The name gate's walk is pinned against an independently counted
      floor.** `tst_check_qml_names.py` now counts `.qml` files from disk itself
      and requires the gate to report that many, because the shipped-module case
      asserted only exit 0 and survived `rglob("*.qml")` → `rglob("Core.qml")`
      (the real module contains a `Core.qml`). Re-running that mutation fails
      **6** cases where it previously failed 5, the new one naming the
      narrowing. The floor is derived, never read back out of the gate's own
      summary.
- [x] 7.4 **The undefined-member class is gated, and the gate is testable.**
      `dialectica-ui/tests/check_qml_members.sh` —
      `qmllint --missing-property error` over every file in `src/qml` — with
      `tst_check_qml_members.sh` pinning both bounds, both wired into the `qml`
      job. A script rather than a flag in `ci.yml` for the reason 4.11 gives: a
      heredoc cannot be called, so it cannot be tested. Proved by three
      mutations: dropping the flag (reverting to the configuration the repo was
      in) fails the rejection case; narrowing the glob fails two; the real
      `Main.qml` defect fails the shipped-view case.
- [x] 7.5 **Its limit is recorded in all three places it could mislead.** With
      `-I src/qml`, qmllint resolves `DTheme` to our own file — a *different
      resolution* than the app performs — so a green says nothing about the host
      collision. Re-measured both ways. Stated in the script header,
      `design.md` and `CLAUDE.md`, each keeping the two claims apart: it covers
      members, never the collision.
- [x] 7.6 **`design.md`'s "in any component it instantiates" is corrected** to
      "in the components a spec instantiates", with the `Main.qml` measurement
      recorded as why the qualifier is load-bearing — three gates green over the
      binding painting the screen's ground.
- [x] 7.7 **The `internal` question the `dev-writer` flagged is settled by
      measurement.** `internal Foo Foo.qml` does **not** export the name; what
      makes it worth gating is that the name is live inside the directory, by
      **filename**, independent of the qmldir line. The gate's behaviour is
      unchanged and correct; its recorded reason was wrong and is fixed in
      `qmldir_entries`, its test case, and `CLAUDE.md`. The first probe passed
      with the `internal` line deleted — a fixture where two explanations give
      the same answer — so only import-by-module-name distinguishes them.
- [x] 7.8 **Implementation untouched, verified by diff rather than memory.**
      `git diff --stat` against the piece branch lists test scripts, CI, docs
      and findings only; nothing under `dialectica-ui/src/`. Every mutation
      above was made to a gate or to `Main.qml` and restored, with the suite
      re-run green afterwards.

## 8. Pinning CI's Qt, after the gate outran it

The gate 7.4 added is newer than the Qt `ubuntu-latest` supplies, so the `qml`
job failed on `check_qml_members.sh`'s preflight probe and **skipped the five
steps after it**. Diagnosis and the rejected alternatives are in `design.md`,
"The gate outran CI's Qt, so CI's Qt is pinned".

- [x] 8.1 **Replace the apt Qt with a pinned upstream one.** The `Install Qt`
      apt step becomes `jurplel/install-qt-action` at `version: "6.8.3"`,
      SHA-pinned to `48d3ad6db93f3627c8ee7a0454bc6f3744f7e730` (v4.3.1) —
      matching this workflow's existing convention for third-party actions, and
      necessary here because `v4` in that repo is a **branch**, not a tag. The
      tag SHA was read from `gh api repos/jurplel/install-qt-action/git/ref/tags/v4.3.1`
      rather than copied from prose, and `releases/latest` confirms v4.3.1 is
      current.
- [x] 8.2 **The comment says which flag forces the floor and how that was
      established**, not a bare number. `--missing-property` is confirmed at the
      pinned version in Qt's own source — tag `v6.8.3`,
      `src/qmlcompiler/qqmljslogger.cpp` declares
      `qmlMissingProperty{ "missing-property" }` and registers it in
      `defaultCategories()` — read via `gh api` at the exact tag, not the `6.8`
      branch. Also measured at 6.10.3 (`qmllint --help`) and absent at v6.4.2.
      The comment names what to re-run if the pin is ever lowered:
      `tst_check_qml_members.sh`, whose preflight is the failing assertion.
- [x] 8.2a **A wrong claim of mine, corrected in all three places it reached.**
      I first wrote that 6.4.2 accepts neither flag. It accepts `--unqualified`;
      only `--missing-property` is absent, the 6.4 category being named
      `--property`, and per-category `--<name> <level>` options exist as a
      mechanism in 6.4 already. Verified myself at tag `v6.4.2` rather than
      relayed. Fixed in the `ci.yml` comment, the gate's error message and
      `design.md`, each of which now says the floor is **6.5** and why the
      message names two flags while one is at fault.
- [x] 8.3 **`archives:` deliberately unset**, so the default desktop package
      supplies `qmllint`, `qmlformat`, `qmltestrunner` and the `QtTest` QML
      module from one qtdeclarative build. Slimming to `qtbase qtdeclarative`
      would drop qtsvg/qtwayland and risk the headless run.
- [x] 8.3a **The SHA pin's limit is recorded rather than implied away.** The
      pinned commit is a *composite* action delegating to
      `jurplel/install-qt-action/action@v4` — a floating branch ref — so the
      pin covers input forwarding but not the fetcher or the apt list, and a
      caller cannot pin a nested `uses:`. Read at the pinned SHA myself.
      Accepted because this job ships no artefact (`build`/`release` install no
      Qt), so the exposure is this job lying about the view rather than a
      backdoored `.lgx` — the case the `install-nix-action` pins defend. Noted
      in `ci.yml` and `design.md` that re-pinning this line cannot fix it.
- [x] 8.3b **Dropping `libgl1-mesa-dev` verified, not assumed.** `install-deps`
      defaults to true and installs that exact package plus the `libxcb-*` set,
      `libxkbcommon-x11-0` and, for Qt >= 6.5, `libxcb-cursor0` — a requirement
      Qt added at 6.5 that a hand-written apt line here would have missed.
      Checked in the action's source at the pinned commit. `run-qml-tests.sh`
      already exports `QT_QPA_PLATFORM=offscreen`, which is still needed; if
      the runner ever fails on GL context creation rather than the platform
      plugin, `QT_QUICK_BACKEND=software` is the first thing to try, recorded
      in the `ci.yml` comment.
- [x] 8.4 **A step asserts the discovery actually landed on the pinned Qt.**
      The scripts' candidate lists name absolute apt paths first; those no
      longer exist, so discovery falls through to `PATH`. `the Qt on PATH is
      the pinned one` prints each tool's resolved path and `--version` and
      re-runs the flag probe, so a bare `qmllint` resolving to some other Qt
      fails by name rather than as a confusing diagnostic six steps later.
- [x] 8.5 **The gate's error message names the real requirement.** It said "a
      Qt5 qmllint?"; the case that actually fired was Qt **6.4.2**, which is
      Qt6 — so a reader checking "is this Qt6?" got `yes` and was no further
      forward. It now names the flag, states that being Qt6 is not sufficient,
      cites 6.4.2 against 6.8.3/6.10.3, and gives the two commands to check.
- [x] 8.6 **Proven to fire, not just edited.** The rejection branch was driven
      by running a scratch copy of the gate with its candidate list forced to
      `/usr/bin/qmllint` (Qt5, `qmllint 1.0`), which exits 1 with
      `Unknown options: unqualified, missing-property` — the new message
      printed in full. Scratch copy deleted; `git status` clean but for the two
      intended files.
- [x] 8.7 **Local gates re-run after the message change** —
      `tst_check_qml_members.sh` all four cases pass, `run-qml-tests.sh` 4 spec
      files / 41 passed / 0 failed, both on Qt 6.10.3. The workflow parses as
      YAML and the `qml` job still lists all six original steps.
- [x] 8.8 **Only CI can confirm the pin in action.** The flag's presence at
      6.8.3 is settled by Qt's source, but nothing local exercises
      `install-qt-action` itself: that it installs on the runner, puts the
      tools on `PATH`, and supplies the GL/xcb libraries `qmltestrunner` needs
      headless are all unproven here. The five previously-skipped steps remain
      unverified until a run goes green. Step 8.4 exists so a wrong pin or a
      wrong `PATH` fails loudly in the second step rather than silently.

## 9. Bringing the branch current with `main`

`main` gained #63 (the three-screen navigator and the Stoa screens) and then
#62 (the composer) while this branch was open. Both were written against the
old `Theme` name and before this gate existed, so merging produced work the
rows above do not describe. **Sections 1–8 are left as they were written:** a
ticked row that silently changes its meaning is worse than one that is visibly
about an earlier tree, which is the convention 2.1 already set here.

- [x] 9.1 **Section 1.3's "eleven component files" is now the count for the
      pre-merge tree, not the merged one.** The merge added five more files
      reading the singleton — `StoaListScreen`, `JoinScreen`, `Composer`,
      `PublishOutcome` and a rewritten `FeedScreen` — for 196 `Theme` →
      `DTheme` substitutions in this merge on top of what 1.3 recorded. Proven
      by a word-level diff against `origin/main` carrying 196 `-Theme` and 196
      `+DTheme` words and, once the twelve file headers are subtracted, no
      other changed word in any of the six files.
- [x] 9.2 **The two conflicted files take `main`'s version wholesale**, so that
      no screen #63 or #62 added is dropped by a partial merge resolution.
      Verified by `git hash-object` against the merge's stage 3 before any
      rename: `Main.qml` = `ff76b06`, `FeedScreen.qml` = `dc055de`, each
      identical to `origin/main`'s blob. The renames were applied only after
      that check passed.
- [x] 9.3 **Six new types got the `D` rather than an exemption**, per the
      owner's decision and `check_qml_names.py`'s own instruction that a new
      type takes the prefix. `StoaReference`, `ClipboardSink`, `StoaListScreen`
      and `JoinScreen` arrived with #63; `PublishOutcome` and `Composer`
      arrived with #62 after this branch's brief was written. `GRANDFATHERED`
      is untouched and still lists exactly the eleven pre-convention names.
- [x] 9.4 **Six `git mv` renames**, so history follows each file:
      `StoaReference`, `StoaListScreen`, `JoinScreen`, `ClipboardSink`,
      `Composer` and `PublishOutcome` each gain the `D`. Fifty-nine call sites
      updated across nine source and four test files, including the comments
      naming the old filenames — a comment citing a file that no longer exists
      is the stale citation this repo has paid for before.
- [x] 9.5 **Re-run of every gate on the merged tree**, which is the first time
      any of them has seen #62's or #63's files: `check_qml_names.py` ok at 28
      QML files and 18 qmldir entries, `tst_check_qml_names.py` all seventeen
      cases, `check_qml_members.sh` ok at 19 files, `tst_check_qml_members.sh`
      all four, `tst_check_bindings.sh` all eight, and `run-qml-tests.sh` at 9
      spec files / 204 passed / 0 failed on Qt 6.10.3. Section 4.1's "41
      passed" was the pre-merge corpus.

## 10. The pin was checked against the wrong assertion

Run 34761666585 failed the `the QML member gate's own tests` step on 6.8.3 with

    Invalid logging level "error" provided for "missing-property"
    (allowed are: disable, info, warning)

**Section 8.2's check was real but insufficient, and that is the finding.** It
confirmed the CATEGORY `missing-property` exists at v6.8.3 in Qt's own source —
true, and still true. The gate needed something narrower: the LEVEL VALUE
`error`, which arrived *after* the category. 6.8.3 offers three levels; 6.10.3
offers four. A source check of the category cannot see a missing level, so the
pin was chosen against an assertion that did not cover the requirement.

Section 8's rows are left ticked and as written, per 9's convention: they
describe work that was done, and 8.2's source citation is accurate about what it
actually checked.

- [x] 10.1 **The escalation no longer uses the level `error`.**
      `--missing-property warning -W 0` replaces it in `check_qml_members.sh`
      and in `ci.yml`'s probe. Both versions accept it, so there is ONE code
      path rather than a version-conditional one — a fallback branch taken only
      on CI is the branch nobody runs locally. Measured at 6.10.3 both ways:
      exit 255 naming `noSuchDeskAtAll` on a bad member, exit 0 over the shipped
      19 files.
- [x] 10.2 **`-W 0` widens the gate, and the trade is recorded rather than
      glossed.** It fails on ANY qmllint warning, not only `missing-property`.
      Accepted because the shipped tree is measured clean of every other
      category, and because the alternative — `--<category> disable` for all the
      rest — is a hand-maintained sweep list that goes stale silently the moment
      qmllint adds a category. A gate stricter than its name fails loudly; one
      looser is the defect this file exists to prevent. The failure message now
      tells a reader to check the category before assuming a missing member.
- [x] 10.3 **The preflight probe was the root cause, and it is fixed.** It
      appended `--help`, which makes Qt's parser print usage and exit 0 BEFORE
      validating any level value — so it proved a flag NAME was known and
      nothing about the VALUE, which is what broke. Measured:
      `--missing-property totalGibberish --help` exits 0 on a linter that
      rejects `totalGibberish` outright when linting a file. The probe now lints
      a minimal generated QML file. The same defective `--help` probe was in
      `ci.yml` and is fixed there too — it had been PASSING on 6.8.3 while the
      gate it claims to vet was unrunnable.
- [x] 10.4 **A fifth self-test case pins the preflight**, proven to fail first.
      A stub linter accepts `--help` and rejects any escalation level, exactly
      as 6.8.3 does. Reaching the gate needed a `QMLLINT` override: PATH order
      does not work, because the gate tries absolute candidates before the bare
      name, so the case would have measured the machine's real qmllint and
      passed for the wrong reason.
- [x] 10.5 **The case asserts on the MESSAGE, not the exit code, and the first
      two attempts did not.** A stub rejecting the level fails the gate twice —
      in the preflight and again in the real lint run — so an exit-code
      assertion passes identically with the broken `--help` probe restored.
      Measured: it did. The fixture tree has every member present, so "reads a
      member that does not exist" is necessarily false there, and a gate
      emitting it has skipped its preflight. That string is the assertion.
- [x] 10.6 **Local gates re-run**: `tst_check_qml_members.sh` all five cases,
      `check_qml_members.sh` ok at 19 files, `check_qml_names.py` ok at 28 files
      / 18 qmldir entries, `tst_check_qml_names.py` all seventeen,
      `tst_check_bindings.sh` all eight, `run-qml-tests.sh` 9 spec files / 204
      passed / 0 failed. `yamllint -d relaxed` reports only line-length
      warnings. All on Qt 6.10.3.
- [x] 10.7 **Only CI can confirm 6.8.3 accepts the replacement.** Every
      measurement above is on 6.10.3, the only Qt available locally. That
      `--missing-property warning` and `-W 0` both work at 6.8.3 is read from
      that version's own help output in the failing run's log (which lists the
      three levels and `-W, --max-warnings`), not executed. The defect itself
      WAS reproduced locally — it lives in the probe, not in Qt — but the fix
      passing on the pinned Qt stays unproven until a run goes green. This row
      stays unticked alongside 8.8 for that reason.

## 11. The run went green, so 8.8 and 10.7 are ticked

Run **34762530270** on `81ce3de`, the branch tip. Both rows above named the same
condition — a green run on the pinned Qt — and both are ticked on it. Their
prose is left as written, per 9's convention: it is accurately about the tree it
was written against, and a row whose text silently changes meaning is the thing
that convention exists to stop.

The evidence is read **off the job's own log**, not inferred from its exit, which
is the distinction 10.3 was written about:

- `qmllint 6.8.3` and `qmlformat 6.8.3`, resolved out of
  `/home/runner/work/dialectica/Qt/6.8.3/gcc_64/bin/` — so 8.4's assertion that
  the discovery landed on the pinned Qt fired against the real thing.
- `ok: qmllint runs the member gate's --missing-property/-W invocation` — the
  fixed preflight, **linting a generated file** rather than appending `--help`.
  This is 10.7's open question answered by execution rather than by reading
  6.8.3's help output: the level `warning` with `-W 0` is accepted by the pinned
  binary when actually asked to lint.
- The five steps that skipped on every prior run executed with real output:
  `ok: a member that exists on DTheme — accepted` (the gate's own tests),
  `ok: 19 QML file(s) checked, every member read off a known type exists`,
  `qmllint`, the runner's undefined-binding check, the component suite on
  `QtTest library 6.8.3`, and `ok: all 9 QML spec file(s) ran`.

**What this does not prove** is anything about the collision itself, which no CI
job can see — the host is absent under `qmltestrunner`, as the design records at
length. The green confirms the gates run on the pinned Qt; it does not widen what
they measure.
