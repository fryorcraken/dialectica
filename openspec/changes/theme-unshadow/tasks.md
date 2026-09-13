## Stages

- [ ] ~~spec — `spec-writer`~~ — **does not apply.** Renaming an identifier adds
      no requirement: no token changes name, value or meaning, and nothing a view
      may claim or must refuse to claim moves. `.openspec.yaml` sets
      `skip_specs: true` with that reason. Struck through rather than omitted,
      because "does not apply" and "nobody did this" are different states and the
      block exists to tell them apart.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

**The `tester` row is left unticked and unstruck, deliberately.** There is a real
question for that stage to ask — whether the existing QML specs can still fail
after the rename, and whether the CI gate discriminates — but it is not a
question `dev-writer` may answer about its own work. What `dev-writer` measured
is in `design.md` and in section 3 below; a `tester` asking it independently is
the point of the row. It is not struck through because this is not a case of
"does not apply": the stage applies and has not been done.

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

- [x] 2.1 Add `no QML type name collides with the host` to the `qml` job in
      `.github/workflows/ci.yml`, after the `qmllint` step. Three arms: a `.qml`
      file named after a host type, a `qmldir` entry declaring one, and a bare
      `Theme.` reference surviving in the view.
- [x] 2.2 **Prove it fails.** Run against `/home/fryorcraken/src/rad/dialectica`
      (the `main` checkout, which carries the defect): exit 1, with all three arms
      firing — `'Theme' is a type name basecamp also registers`, `qmldir declares
      'Theme'`, and the bare-reference list. Run against this worktree: exit 0,
      `ok: no QML type name collides with the host`. A gate that has only ever been
      seen green proves nothing, which is why this was measured both ways.
- [x] 2.3 Confirm the comment-exclusion arm is exercised rather than dead —
      `grep -n "[^A-Za-z]Theme\."` over `DTheme.qml` returns two comment lines, so
      without the `grep -v` the gate would be permanently red on a correct tree.

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

## 4. Gates

- [x] 4.1 `dialectica-ui/tests/run-qml-tests.sh` — **41 passed, 0 failed**, across
      4 spec files (`tst_core_call` 13, `tst_feed_states` 12, `tst_identicon` 7,
      `tst_sanitised_text` 9). This is not a formality: a missed reference leaves a
      bare `Theme` that resolves to nothing under the runner, so the suite would
      fail with `Theme is not defined` rather than pass with a wrong value.
- [x] 4.2 `qmllint --unqualified disable -I <qml dir>` over all thirteen QML files
      — clean, no output. The flag is the repo's existing one, for the
      `logosModule` bridge qmllint cannot see; it was not widened for this change.
- [x] 4.3 `git diff origin/main --stat` read before committing, and every listed
      file confirmed as one this change meant to touch.

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
