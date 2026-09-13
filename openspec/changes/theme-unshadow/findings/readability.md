# Readability findings — `theme-unshadow`

Reviewed on `piece/theme-unshadow` @ `76a58d2`, in a worktree of its own, after
the dev-writer's rebuild of the gate. Both halves of the gate were run rather
than read, the QML suite was run, and every figure in `design.md` and `tasks.md`
was checked against a command.

**What is clean**, in prose rather than as boxes so the list below is only things
that must happen:

- **The gate's failure message is diagnosable well inside a minute.** Run
  against a clean `main` worktree (`git worktree add --detach … main`, then the
  step's script): it names the file, the line, the offending identifier, the
  replacement, *and* the runtime consequence — "a bare Theme resolves to
  basecamp's singleton, so every token read from it is undefined at runtime".
  The qmldir arm's message is better still: it explains the C++-registration
  mechanism and then says `Rename to 'DTheme'`. A reader who has never seen this
  piece knows what to type. The prior reviewer judged the old message adequate;
  the new one is strictly better, because the un-prefixed-singleton arm names a
  *specific* replacement string rather than a category.
- **The `116` figure checks out exactly.** Running the shipped gate's logic
  against a clean `main` worktree gives 116 error lines: 113 under `src/qml/`
  and 3 in `tests/tst_identicon.qml`, matching `design.md:66` and its stated
  split. The earlier "80-odd" is gone. The `89 errors` figure is correctly
  framed as a reported launch symptom rather than a measurement.
- **The `17` file count checks out.** `find dialectica-ui -name "*.qml"` returns
  17 (13 under `src/qml/`, 4 under `tests/`), and the gate independently prints
  `ok: 17 QML file(s) checked`. The dev-writer was right and the correctness
  reviewer's 16 was one low; `tasks.md` 4.2 now states 17 with the split and
  names both commands.
- **`41 passed, 0 failed`** reproduces here: 13 + 12 + 7 + 9 across the four
  spec files, matching `tasks.md` 4.1.
- **The withdrawn premise reads correctly in every copy.** In `ci.yml`,
  `CLAUDE.md`, `DTheme.qml`, `design.md` and `proposal.md` the true
  C++-registration mechanism is stated before the withdrawn one, and the
  withdrawal is explicit. A reader stopping at the first paragraph leaves with
  the right model.
- **`docs/IDENTICON.md` is consistently updated** — all five references now read
  `DTheme`, and `docs/UI-BRIEF.md` genuinely names no QML identifier, so the
  claim that the brief survives untouched is true.

---

- [x] **`dev-writer`** — `openspec/changes/theme-unshadow/tasks.md:91` — a ticked
      task cites a `design.md` section that no longer exists
      **Scenario:** task 2.4 ends "Also in `design.md` under *Why the gate
      excludes comment lines*". `grep -n "Why the gate excludes comment lines"`
      over `design.md` returns nothing: the rewrite at 4.6 renamed that section
      to "Comments are stripped once, and the false positive is gone". A reader
      following the pointer to understand the comment handling finds no such
      heading and has to guess which of the eight sections replaced it.
      **Severity: low**, documentation only — but it is the exact decay this
      repo names: the rewrite corrected the *content* of 4.6 and left 2.4's
      cross-reference pointing at the pre-rewrite shape.

      **Fixed.** 2.4 now carries a "**Superseded**" note saying the false
      positive is gone rather than documented, and points at the live section,
      which after this pass is "Two comment-strippers, and the claim narrowed to
      match" — that heading absorbed the old 4.6 content plus the fix for the
      fourth architecture box, so pointing 2.4 at the heading you found missing
      would have stranded it a second time.

- [x] **`dev-writer`** — `openspec/changes/theme-unshadow/tasks.md:73-83` — tasks
      2.1 and 2.3 describe a gate that no longer exists, with no correction note,
      while 4.4–4.7 carry the corrections
      **Scenario:** 2.1 says the step has "Three arms: a `.qml` file named after a
      host type, a `qmldir` entry declaring one, and a bare `Theme.` reference".
      The shipped arms are (1) every `qmldir` singleton is `D`-prefixed, (2) each
      declared singleton's file exists, (3) no bare `Theme` reference. There is
      no filename arm at all — I checked: the script reads `qmldir` and the
      `.qml` bodies, never a `.qml` *basename*. 2.3 then says "without the
      `grep -v` the gate would be permanently red"; `grep -n "grep -v"` over
      `ci.yml` returns nothing, because 4.6 replaced it with `strip_comments`.
      Rows 4.1, 4.2 and 4.5 each carry an explicit "**Scope corrected**" /
      "**FIXED**" note where they were superseded; 2.1 and 2.3 do not, so they
      read as current descriptions of the shipped gate and are not.
      **Severity: medium.** `tasks.md` is the record a design-reviewer and the
      archive step read. Three of its ticked rows describe a mechanism the piece
      deliberately replaced, and two of them say so nowhere.

      **Fixed.** 2.1 and 2.3 now carry "**Superseded**" notes in the same style
      as 4.1/4.2/4.5, kept rather than rewritten — a ticked row that quietly
      changes its meaning is worse than one that says what it used to claim and
      where the truth now lives.

      2.1's note states the specific thing you verified: **there was never a
      filename arm.** I checked the same way and agree — the script reads
      `qmldir` and the `.qml` bodies, never a `.qml` basename. The row also
      claimed the `qml` job, which the third architecture box has now moved.

      2.3's note separates its mechanism from its finding: `grep -v` is gone
      (your `grep -n "grep -v"` returning nothing is correct), but the
      observation it recorded — that `DTheme.qml`'s header would redden a
      correct tree without comment handling — still holds, and is now pinned as
      a test case from the accepting side rather than confirmed by hand.

      Six new rows (4.9–4.14) record this pass, so the same decay does not
      recur: they describe the shipped gate, not the one being replaced.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:571-745` — the step carries
      roughly 108 lines of comment for ~50 lines of code, and most of it is a
      fourth copy of a narrative already in three other files
      **Scenario:** the preamble (571–636, 66 lines) plus in-script comments
      (645–651, 657–660, 668–685, 708–709, 720–730 — 42 lines) tell the same
      story as `CLAUDE.md:295-338`, `DTheme.qml:4-37` and `design.md`'s first two
      sections: what the collision was, where it lives, the withdrawn precedence
      premise, the two `-import` measurements, why `Core` resolving is not
      evidence, why no component test can see it, and the QWARN limitation. Four
      copies that must change together. The repo's own rule is that a comment
      earns its place by saying what a command cannot — but here four files say
      the same uncommandable thing, and the next person to refine the wording
      will fix one or two of them.
      **Severity: medium** (readability/maintenance, not a defect). The
      load-bearing part *for a CI step* is narrow: why static and not a test,
      why a prefix and not a list, and what a red means. The mechanism narrative
      belongs in `CLAUDE.md` — which is where a person adding a singleton meets
      it — with the step pointing there. Concretely: keeping 595–636 and
      replacing 571–594 with a one-line pointer to the `CLAUDE.md` trap entry
      would lose nothing a reader of a red needs.

      **Fixed, and further than you asked**, because the architecture boxes
      forced the same conclusion from a different direction: the Python is no
      longer in `ci.yml` at all. It is
      `dialectica-ui/tests/check_qml_names.py`, which makes the narrative
      question moot for the workflow and testable for the gate.

      What remains in `ci.yml` is 32 lines of comment (221–252) over two steps
      of two lines each — down from ~108 over ~50 — and
      it carries only the four things you identify as load-bearing *for a CI
      step*: why static rather than a test, why a prefix rather than a list, why
      a script rather than a heredoc, and why it is in `lint`. The mechanism,
      the withdrawn precedence premise and the measurements now live in
      `CLAUDE.md`'s trap entry, which the step points at by name — your
      reasoning that this is where a person ADDING a singleton meets it is the
      reason, and I have quoted it in the comment.

      That takes the copies from four to three (`CLAUDE.md`, `DTheme.qml`,
      `design.md`). `DTheme.qml`'s header is the one I did not cut: it is what a
      person editing the singleton reads, and it is the file whose *name* is the
      fix. If you think three is still one too many, that is a fair follow-up
      and I would take `DTheme.qml` down to a pointer too — but I would rather
      not do it in the same pass that rewrote the gate.

- [x] **`dev-writer`** — `.github/workflows/ci.yml:686` — the grandfather clause
      is written so the next person adds a `D`, but the *code* does not say which
      of the three arms the exemption applies to
      **Scenario:** `GRANDFATHERED = {"Core"}` is preceded by 18 lines (668–685)
      that are genuinely good on the "why" — it predates the convention, it is
      itself the evidence the host does not claim every name, `DCore` is the end
      state, "do not add a second name here — add the `D` instead". That reads
      exactly as it should. What is missing is the *scope*: the set is consulted
      only by arm 1, so a reader who hits a red from arm 3 and sees a
      `GRANDFATHERED` set at the top of the script may reasonably try adding a
      name to it — which would do nothing, silently. One line stating "this set
      exempts a name from the prefix rule only; nothing exempts a bare `Theme`
      reference" closes the gap at the point the wrong move is made.
      **Severity: low.** The reasoning is well written; it is the blast radius of
      the exemption that is unstated, and an unexplained-scope exemption is the
      shape that decays into a second one.

      **Fixed**, with your sentence essentially as written. The set now carries
      a "SCOPE OF THE EXEMPTION" paragraph stating that it exempts a name from
      the prefix rule **only** — not a missing file, and not a bare reference —
      and it says so at the point the wrong move is made, i.e. at the set
      itself, framed for the reader who arrived from a reference-arm red.

      Your prediction that an unexplained-scope exemption "decays into a second
      one" came true during this pass, in a way worth recording: the first
      architecture box's fix required adding **eleven** names to that set. The
      comment now distinguishes the two kinds — `Core`'s original reason, and
      the eleven components which are there because the widened rule would
      otherwise fail on them — and states that the list is the enumeration of
      what is unprotected rather than a place to put the twelfth. The "do not
      add a name here — add the `D` instead" line you liked is kept verbatim.

      Pinned: adding `Theme` to the set fails three test cases, so the tuning
      move that turns a red green is itself red.
