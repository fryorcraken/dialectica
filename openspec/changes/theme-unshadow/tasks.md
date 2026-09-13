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
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

**The `tester` row is left unticked and unstruck, deliberately.** There is a real
question for that stage to ask — whether the existing QML specs can still fail
after the rename, and whether the CI gate discriminates — but it is not a
question `dev-writer` may answer about its own work. What `dev-writer` measured
is in `design.md` and in section 3 below; a `tester` asking it independently is
the point of the row. It is not struck through because this is not a case of
"does not apply": the stage applies and has not been done.

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
