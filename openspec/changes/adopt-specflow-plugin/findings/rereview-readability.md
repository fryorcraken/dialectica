# Re-review: readability — `code-reviewer`

Dimension: readability only. Scope: `20643177..HEAD`, excluding `findings/`
(`git diff 20643177..HEAD --stat -- . ":!openspec/changes/adopt-specflow-plugin/findings"`:
15 files). Every line of that diff was read, plus the whole of the overlay,
`design.md`, `tasks.md` and README's new section in their current form.

- [x] **`dev-writer`** — `CLAUDE.md:406-409` — the new section's intro says
      *"The reason for each is in the overlay … under the section named; read it
      there rather than restating it here."* That holds for four of the six
      bullets and not the other two. The `yq`/`jq` bullet points at `## Hazards`,
      whose entry (`PROJECT.md:113-117`) says which `yq` to use but not why
      Python is banned. The `VAR=` bullet also points at `## Hazards`, whose
      entry (`PROJECT.md:110-112`) names the wrappers and gives no reason. The
      reason, the permission click, is in the block's own table and at
      `CLAUDE.md:527`. The intro's second clause also fails in the other
      direction: the last bullet (`CLAUDE.md:420-421`) restates its reason, *"the
      repository root has no flake, so `.#lgx` fails there"*. `design.md:286-288`
      repeats the claim (*"The reasons stay only in the overlay, so the two
      cannot disagree on a reason"*), so the fix belongs there too. Reword the
      intro to promise what the sections actually hold (the detail, or the
      wrapper), or add the two missing reasons to the overlay.
      **Scenario:** a session told never to read YAML with Python follows
      the pointer to `## Hazards` for the reason, finds a note on telling two
      `yq` binaries apart, and has to guess what the rule protects.
      **Measured:** `git grep -n -i -e "python" -- .claude/specflow/PROJECT.md`
      matches only line 113, the rule itself. Severity: low, a pointer that
      over-promises.
      **Fixed** in the commit "Give each CLAUDE.md shell rule its reason in
      the overlay", by making the intro's promise true rather than weakening
      it. Checked all six against the section each names. Two had a reason
      already (`qmltestrunner`'s Qt5 exit, the SDK's manifest failure) and
      `## Build` holds the `.#lgx` one. Three did not, and the overlay's
      `## Hazards` gains each: the `python3` click and the `VAR=` click, both
      restored from the removed `CLAUDE.md` text
      (`git grep -n -F "never Python" origin/main -- CLAUDE.md`, and the
      `QT_QPA_PLATFORM` paragraph above `origin/main:CLAUDE.md:225`), and for
      `/nix/store`, which had none on `main` either, that the store is
      outside the working directories. The other direction: the `nix build`
      line no longer restates its reason; it reads "…`./dialectica#lgx`, not
      `.#lgx`. `## Build`." design.md's "Six shell rules" section now says
      each line holds the rule and replacement, the section the reason, and
      records the three reasons put back. The block is untouched:
      `cmp -i 24899:1796 -n 9490 CLAUDE.md <clone>/skills/sync/SKILL.md`
      exits 0 with offsets derived after the edit.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:27-28` — *"Passing it one
      spec file is the supported shape."* was added in this range, lifted from
      `origin/main:CLAUDE.md:218-219`. There it went on *"…and needs no approval
      click"*, so "supported" meant supported by the permission checker. Without
      that clause it reads as "the no-argument form is unsupported". That
      contradicts the QML row four lines up (`:13`), which gives the
      no-argument form first, and `CLAUDE.md:412`, which writes
      `run-qml-tests.sh [<spec>]`. Lines 31-35 already say the one-spec form is
      the free one, so line 28 is a second copy whose meaning changed when it
      moved. Drop it, or say "the shape that needs no approval click".
      **Scenario:** an agent wanting the whole suite reads line 28, takes the
      no-argument run to be unsupported, and loops over specs one at a time.
      Severity: low.
      **Fixed** in the commit "Name the project's own commands without
      calling them free". The sentence is dropped; the bullet now reads "Run
      the QML suite through the script, never through `qmltestrunner`
      directly (see Hazards), either whole or with one spec file", matching
      the QML row and `CLAUDE.md`'s `run-qml-tests.sh [<spec>]`. The "no
      approval click" meaning is not restored either:
      `findings/rereview-architecture.md` box 5 is that it depends on an
      untracked allowlist, and the bullet below now says so.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:187-191`
      — "What was considered instead" opens with *"Write them into the overlay
      anyway. Rejected: … an overlay rule that contradicts a plugin rule leaves
      two instructions and no precedence."* It now comes straight after the
      paragraph this range inserted (`:178-185`), which says rules 6 to 10 and
      12 *"add to the plugin without contradicting it, so the overlay could
      carry them"*. "Them" now covers twelve rules, and the rejection reason
      holds only for 1 to 4. The real reason for 6 to 10 and 12, one source per
      rule by the owner's choice, appears only in the paragraph above. Scope the
      bullet to rules 1 to 4, or give it both reasons.
      **Scenario:** a reader asking why rule 7 is not in the overlay reads the
      options list, finds "contradicts a plugin rule", and finds the paragraph
      just above saying it does not.
      Severity: low. Internal inconsistency introduced by the insertion.
      **Fixed** in the commit "Classify #174's rules 6 to 12 by their
      dependence on rules 1 to 5". The bullet gives both reasons, each scoped:
      for rules 1 to 4, an overlay rule that contradicts the plugin leaves no
      precedence; for rules 6 to 10 and 12, which contradict nothing, the
      owner chose one source per rule over a copy that would have to be
      removed again; rule 11 is the first reason at one remove, since its
      stop hands over to rule 1's round. The paragraph above it now says
      "Rules 6 to 12", and the section's "items" are "rules" throughout,
      which also settles this file's unboxed rules/items observation.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:163-165`
      — *"Rules 6, 7, 8 and 10 describe v0.1.0's behaviour as written. Rule 12
      does as well…"* reads as "the plugin already does this", which is the
      opposite of the list it summarises. Rule 6 says *"The plugin's `run` skill
      says nothing about a conflicting pick"*, and rule 9 says the plugin freezes
      the piece *"only while the `tester` runs"*. The intended sense seems to be
      that the gap each rule closes exists in v0.1.0 as written. The same
      three-line paragraph classifies rules 6, 7, 8, 10, 11 and 12 and leaves
      out rule 9 without saying why, while the next paragraph (`:180`) does
      include it ("Rules 6 to 10 and 12"). One rewrite of the paragraph fixes
      both.
      **Scenario:** a reader deciding whether rules 6 to 12 are still needed
      under the plugin takes "describe v0.1.0's behaviour" as "already in
      v0.1.0" and concludes the issue comment over-reports.
      Severity: low-medium. The sentence can be read with the opposite meaning,
      in the section that justifies the accepted regression.
      **Fixed** in the commit "Classify #174's rules 6 to 12 by their
      dependence on rules 1 to 5". The paragraph now opens "Each of rules 6
      to 12 closes a gap that plugin v0.1.0 has as written; none describes
      something the plugin already does", and classifies all seven,
      rule 9 included, in three bullets.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:433-435`
      — *"What moved is instruction trust only: … It is low today because the
      plugin is the owner's"*. The nearest antecedent of "It" is "instruction
      trust", and "instruction trust is low" says the opposite of what is
      meant, which is that the risk is low. Name the subject ("The risk is
      low…").
      Severity: low.
      **Fixed** in the commit "Record the plugin repository's real trust
      state, and the marketplace constraint", which rewrote the sentence
      while correcting its claim (`findings/rereview-security.md` box 1): it
      now reads "The risk is still rated low because the repository sits in
      the owner's own namespace", with the subject named.

- [x] **`dev-writer`** — `README.md:122` — *"This step goes away once the plugin
      is published…"* closes a section that introduced itself as *"three
      steps"* (`:93`) and numbered them. "This step" makes a newcomer ask which
      one: only step 2 obviously goes away, yet the sentence refers to all
      three. Say "These steps go away…".
      Severity: low.
      **Fixed** in the commit "Record the plugin repository's real trust
      state, and the marketplace constraint": "These steps go away once
      `agent-spec-flow` has a release…", which also replaces "published"
      (`findings/rereview-design.md` box 3). The same commit moves the
      "`~/src/agent-spec-flow` is only an example" sentence above step 1,
      this file's unboxed observation, so it is read before the steps that
      use the directory.

## Clean, in prose

- **Every "Fixed" claim in `findings/readability.md` holds.** The `CLAUDE.md`
  intro colon now ends "…worth reading in full". `git grep -F "missing-property
  error" -- CLAUDE.md` prints nothing. The overlay hazard points at
  `docs/SCAFFOLD.md`'s "`[basecamp.env]`" heading, which exists (`:91`). The
  static-gates row names all three pairs. `tst_render_probe.qml` and
  `Cargo.toml` are repointed, and README "Building" exists at `:52`. The five
  quotations now match the block's wording. The remaining "stays a question
  with an answer" uses (`authoring.rs:304`, `wire.rs:2714`, `Main.qml:235`,
  `FeedScreen.qml:228`, `DThreadScreen.qml:112`) are not quotation-marked
  citations of `CLAUDE.md`. "four" is gone from `design.md:46`. The dropped
  narratives are grouped by source file.
- **No sibling left behind on the repointed comments.** Every other quotation
  attributed to `CLAUDE.md` in `dialectica/` and `dialectica-ui/` resolves
  against the current file. The two new specflow-skill quotations resolve in
  the plugin checkout: `skills/flow/SKILL.md:259` "Never write a scenario that
  cannot be tested" and `:263` "Say what a gate cannot see".
- **No stale references.** `git grep` for `RUNNER.md`, `OPENSPEC-ARCHIVE`,
  `PROJECT-MANAGEMENT`, `agents README`, `agents/README` and `.claude/agents/`
  outside `archive/` and this change finds only `.gitignore:49,69` and
  `CLAUDE.md:467`, which name the kept directory.
- **The section pointers resolve.** The new section's pointers (`## Hazards`,
  `## Test layers`, `## Build`, README "Building") all name headings that
  exist. The section sits after "Security posture" and before the block,
  which is the right place for a rule the owner wants outside the block.
- **README matches `tasks.md` 3.3/3.4.** Both list the clone, `marketplace
  add` on the clone, `install … --scope project` from the root, and a
  session listing `specflow:*` as the check.
- **The overlay's changes check out.** The `grep -c` correction names a real
  step and comment (`ci.yml:1662`, `:1685`). The `require-jq-yq.sh`
  description matches its probe (`:28`). The `## pm` addition reads cleanly.
- Observations, not boxes (stylistic, or owner-decided):
  - `CLAUDE.md:414-415`'s `VAR=` bullet repeats the block's own
    `CLAUDE.md:527` almost word for word, in the same file. The owner chose
    the six rules, so this is not filed. The intro's "not only specflow ones"
    suggests the block binds fewer sessions than it does: it is in
    `CLAUDE.md` too.
  - `design.md`'s "#174" section calls the same numbered list "rules" in some
    places and "items" in others (`:156`, `:167`, `:172`, `:178`, `:180`),
    sometimes within one paragraph.
  - "Dropped, deliberately" now holds two entries that say they are not
    dropped (`design.md:391-394`, `:400-404`). The intro ("no home in the
    overlay") covers them, but the heading does not.
  - The reflow in `feed.rs:272-273` and `log/mod.rs:34` left short ragged
    lines. The same goes for `design.md:253` (overlong) and `:318-319`.
  - README says "`~/src/agent-spec-flow` is only an example" (`:118`) after
    step 3, although it applies to steps 1 and 2. A newcomer typing the steps
    in order meets it too late.
  - `CLAUDE.md:259-264`'s new parenthetical ("at the time … specs drive it
    now, but …") makes one sentence run eight lines. It reads correctly but
    slowly.

No mutations were made in this tree.
