# Readability review — e2e-suite-review

Scope: every file `git show --stat 8368b2f` lists (as it stands at HEAD
`ee8e145` on `piece/134-e2e-suite-review`), and this piece's own diff
(`git diff 8368b2f...HEAD`, three dots), which is `CLAUDE.md` plus the four
`openspec/changes/e2e-suite-review/` documents. For `ci.yml`, only #164's
hunks (`git show 8368b2f -- .github/workflows/ci.yml`). For `Main.qml`, only
the five read-only root handles #164 added (confirmed against
`git show 8368b2f -- dialectica-ui/src/qml/Main.qml`; the rest of the file —
`screenShown`, the transition functions — predates #164 and is out of scope).

## Findings

- [x] **`dev-writer`** — `openspec/changes/e2e-suite-review/design.md:39,109,167`
      — this piece's own Decisions reuse the letters `D1` and `D4`, which the
      merged code already cites, unqualified, to mean the *archived*
      `e2e-ui-suite` change's `design.md` — and after this piece archives,
      both documents permanently coexist under `openspec/changes/archive/`
      with contradictory content under the same label.

      **Scenario:** `dialectica-ui/tests/adjudicate-ui-run.sh:19` says
      "design.md D1 records which checks in tst_adjudicate_ui_run.sh go red
      without it" — about the adjudicator's three-condition logic.
      `dialectica-ui/tests/tst_adjudicate_ui_run.sh:113` cites the same "D1"
      for the missing-report behaviour. Both predate this piece and are
      unchanged by it (`git show 8368b2f` created them; this piece's diff
      does not touch either file). This piece's own `design.md:39` opens
      `### D1 — The scaffold guard's red run answers "does lgs itself change
      a value"` — a different decision about a different check, with no
      overlap in subject. A reader (or an agent doing exactly what this
      repo's own house style asks — "run the command before you write the
      number" — by grepping `^### D1` across the tree) gets two hits with
      unrelated, non-obviously-wrong content and no path in either citing
      comment to disambiguate. The same collision exists for `D4`:
      `dialectica-ui/tests/tst_scaffold_values_unchanged.sh:4` cites "design.md
      D4" for "both `lgs basecamp setup` and `lgs basecamp install` rewrite
      `scaffold.toml`" (archived D4, about the Basecamp build source), while
      this piece's own `design.md:167` opens `### D4 — The CLAUDE.md line
      sits in "When you forbid a tool, name the replacement"` — again
      unrelated.

      `D8`, `D11` and `D12` (cited unqualified at `ci.yml:1163,1193,1201`,
      `ui-tests.yml:110`, `adjudicate-ui-run.sh:27`, `require-jq-yq.sh:25`,
      `tst_adjudicate_ui_run.sh:122`, `tst_ui_tool_pins.sh:12`) do **not**
      collide: this piece's `design.md` stops at `D5`, so those letters
      resolve to the archived document by elimination. Checked, not assumed:
      `grep -n "^### D" openspec/changes/e2e-suite-review/design.md` returns
      only D1–D5.

      **Not this piece's introduction of the pattern** — bare `design.md`
      citations are a repo-wide convention (e.g. `tst_stoa_screens.qml:2609`,
      `tst_thread_navigation.qml:19`), and `ci.yml:1131` shows the pattern's
      own author knew to scope it ("the e2e-ui-suite change's design.md,
      D12") two lines above an unscoped "design.md D12" at `ci.yml:1163` in
      the same job. What is this piece's introduction is the actual
      collision: choosing `D1`/`D4` for its own Decisions when the code it is
      reviewing already uses those exact letters, unqualified, for something
      else. Renumbering this piece's Decisions (or qualifying every citation
      this piece's `design.md` adds with the change name, the way
      `Main.qml:128` already does for the one citation #164 itself added —
      `` (`e2e-ui-suite` design.md, "Root handles for the suite") ``) removes
      the collision without touching the archived text.

      **Severity:** moderate — no test or CI gate is affected (`.openspec.yaml`
      declares `skip_specs: true`), but the ambiguity is permanent once this
      piece archives, and it sits in exactly the kind of numeric citation
      CLAUDE.md's own "a number in a comment is a claim" principle warns
      reads as more precise than it is.

      **Fixed** in the commit that flips this box, by the second of the two
      remedies the finding offers: every decision citation in the suite's
      files now names its change. That covers `adjudicate-ui-run.sh` (D1,
      D12), `tst_adjudicate_ui_run.sh`, `require-jq-yq.sh`,
      `tst_ui_tool_pins.sh` (D8), `tst_scaffold_values_unchanged.sh` (D4, and
      its "this piece's tasks.md and design.md Risks"), ci.yml's `ui-specs`
      steps (D8, D11, D12), and `ui-tests.yml` and `join.yaml`'s by-title
      citations. Each new citation this change's earlier commits added was
      written qualified. Renumbering was rejected: the next change on the
      suite would start at D1 again. The finding's elimination argument for
      D8/D11/D12 also stopped holding once this change's Decisions reached
      D10 (design.md D10). **No test fails without this, and none could:**
      it is prose. The check is `git grep -n -F "design.md"` over those
      files, where every hit now names `e2e-ui-suite` or `e2e-suite-review`
      (the two unprefixed hits in `ui-tests.yml` are the second line of a
      wrapped qualified citation). Bare citations outside the suite, such as
      `Main.qml:82,102,509`, predate #164, cite other changes and are left
      alone.

## Checked and clean

- `.github/workflows/ui-tests.yml` (full file) and `.github/workflows/ci.yml`
  (#164's hunks): comments are extensive, accurate against the code they sit
  beside, and consistently explain *why* rather than restating the line
  below them (e.g. the apt-source-disabling step, the `BASECAMP_REV` cache
  key derivation, the `--strict`/report-vs-exit-code rationale).
- `dialectica-ui/src/qml/Main.qml`'s five new handles and their comment block
  (lines 110–133): clear, names what reads them and why each is read-only,
  and is the one place in this diff that already scopes its `design.md`
  citation correctly.
- `dialectica-ui/tests/tst_e2e_handles.qml`: each fixture's comment states
  which null-implementation it is chosen to defeat; consistent with the
  assertions each test makes.
- `dialectica-ui/tests/adjudicate-ui-run.sh`,
  `dialectica-ui/tests/tst_adjudicate_ui_run.sh`,
  `dialectica-ui/tests/require-jq-yq.sh`,
  `dialectica-ui/tests/tst_ui_tool_pins.sh`,
  `dialectica-ui/tests/tst_scaffold_values_unchanged.sh`: the shell rewrite no
  reviewer read before this piece. Function and variable names read plainly
  (`require_jq_yq`, `expect_exit`, `extract_step`), the `problem()`/`problems`
  accumulator pattern is explained where it is used, and every fixture-pairing
  comment (e.g. "the pairing case" in `tst_scaffold_values_unchanged.sh:154`)
  says which false-positive it exists to rule out. No dead code, no
  unexplained magic value.
- `dialectica-ui/tests/ui/join.yaml` and `validate-ui-specs.mjs`: the spec's
  header comment states coverage and non-coverage explicitly; the validator's
  comments are accurate against its own short body.
- `dialectica-ui/tests/check_qml_reachable.py`: #164 changed only the
  docstring's account of what "the static half" pairs with (now correctly
  naming `tests/ui/` and `ui-tests.yml` rather than a piece that no longer
  exists under that name); the rest of the file is unchanged and out of scope.
- `CLAUDE.md`'s new line (this piece's own diff): sits where design.md D4
  says it would, is phrased by the approval-click cost the way the rest of
  that paragraph is, and is consistent with the surrounding bullets' style.
- The four `openspec/changes/e2e-suite-review/` documents read clearly on
  their own terms — proposal, design and tasks cross-reference each other
  correctly, and Observed/Predicted pairs in `tasks.md` are easy to tell
  apart from each other.
