# Review: readability — `code-reviewer`

Dimension: readability only. Reviewed the whole diff against `origin/main`:
`CLAUDE.md`, `.claude/specflow/PROJECT.md`, `.gitignore`,
`.claude/settings.json`, `proposal.md`, `design.md`, `tasks.md`,
`.openspec.yaml`, and every comment in the tree that cites `CLAUDE.md` or a
deleted file (`git grep` for the four deleted doc names, the nine role-file
names, the removed section titles, and quoted `CLAUDE.md` sentences).

- [x] **`dev-writer`** — `CLAUDE.md:9-13` — the intro's last sentence, *"The
      specflow block carries its own table for the flow, the runner and the
      project overlay:"*, ends in a colon that introduces the **dialectica**
      table (`docs/SCAFFOLD.md`, `docs/SOURCES.md`) directly below it, not the
      specflow one 400 lines further down. Move that sentence after the table,
      or end the previous sentence ("…worth reading in full") with the colon.
      **Scenario:** a reader takes the two-row table as the specflow block's
      table and looks in it for the runner and the overlay.
      **Severity:** low; misleading transition, not a wrong fact.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong": the colon now ends "…worth reading in full", and the
      specflow-table sentence follows the two-row table. The block is
      untouched: `cmp` against the plugin's `skills/sync/SKILL.md` still exits
      0 at the new offsets (see `tasks.md` 2.3).

- [x] **`dev-writer`** — `CLAUDE.md:250-256` and `:275` — the retained "Module
      contract traps" entry still says *"`qmllint --missing-property error`
      cannot see this defect: CI passes `-I dialectica-ui/src/qml`, which puts
      our own `Theme.qml` on the import path"*, and *"It does catch every
      undefined MEMBER"* with "It" being that `error` invocation. `design.md:50-55`
      calls exactly this wording wrong (Qt 6.8.3 rejects the level `error`; the
      escalation is `--missing-property warning -W 0` in `check_qml_members.sh`;
      the singleton is `DTheme`), and the overlay (`PROJECT.md:80-83`) carries the
      corrected version. After this change the two always-read files give two
      accounts of the same gate, and the change's own design says which is wrong.
      Correct the `CLAUDE.md` sentences to match the overlay (the block's
      "Prune as you go"), or file it and tick this with the issue number.
      **Measured:** `git ls-files dialectica-ui/src/qml/Theme.qml` prints
      nothing; `ci.yml:894` runs `qmllint --unqualified disable
      --missing-property warning -W 0`.
      **Severity:** medium; contradictory instructions across two files every
      agent reads in full.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong". `CLAUDE.md` now says `qmllint` (no level), names the
      singleton file `DTheme.qml`, says `check_qml_members.sh` runs
      `--missing-property warning -W 0` because the pinned Qt rejects
      `error`, and calls the later reference "qmllint's `missing-property`
      check", the wording `ci.yml:260` uses. The same edit puts "no spec
      instantiates `Main.qml`" in the past tense, which
      `findings/correctness.md` noted outside its boxes. Re-measured:
      `git ls-files dialectica-ui/src/qml/DTheme.qml` lists the file,
      `git grep -n -F "missing-property error" -- CLAUDE.md` prints nothing.
      The section is a kept one; this is the block's "Prune as you go", not a
      rewrite.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:109-113` — *"`CLAUDE.md`'s
      "Module contract traps" has the full account"* is a cross-reference
      orphaned by the move. In the deleted `.claude/agents/README.md:570-579` it
      followed a sentence about the `Theme`/`DTheme` collision, which is what
      "Module contract traps" gives the full account of. Here it follows the
      QML-logging switches and the "missing manifest field" case, neither of
      which "Module contract traps" mentions — the logging switches left
      `CLAUDE.md` in this change and now live only in this bullet. Either name
      what the section accounts for (the `DTheme` collision and
      `check_bindings`) or drop the pointer.
      **Measured:** `git grep -n -E "QT_FORCE_STDERR_LOGGING|manifest field" --
      CLAUDE.md` prints nothing.
      **Severity:** low.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong", together with `findings/correctness.md`'s box on the
      same lines. The hazard now points at `docs/SCAFFOLD.md`,
      "`[basecamp.env]`", for the switches, and names what "Module contract
      traps" does hold: the `DTheme` collision and the `check_bindings`
      account.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:14` — the "Static QML
      gates" row's Sees column claims three properties (name collision,
      undefined member, unreachable type), but its Command column gives one
      gate, introduced with a colon as if it were the list: *"each gate as
      `ci.yml`'s `lint` and `qml` jobs run it, its own tests first:
      `tst_check_qml_names.py`, then `check_qml_names.py dialectica-ui`"*.
      `check_qml_members.sh` and `check_qml_reachable.py` are not named anywhere
      in the overlay, so an agent wanting to run the member or reachability gate
      locally has to go to `ci.yml` to find out they exist. Name all three
      pairs, or say "for example".
      **Measured:** `ci.yml:313-316` and `:1010-1013` run the other two pairs.
      **Severity:** low.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong": the row names all three pairs, each with its job,
      and the Sees column lists the three properties in the same order. Each
      command was run on this tree: `tst_check_qml_reachable.py` "all cases
      passed"; `check_qml_reachable.py dialectica-ui` "ok: 25 registered
      type(s)…"; `check_qml_members.sh` "ok: 26 QML file(s) checked". The
      member gate's Qt floor (6.5) is from its own error text.

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_render_probe.qml:14-15` —
      *"which CLAUDE.md records as indistinguishable from a plugin that failed
      to load"* cites a sentence this change removed with "Before anything else,
      make the failure visible" (`origin/main:CLAUDE.md:740`). The fact now lives
      in the overlay's `## Hazards` ("Basecamp swallows QML errors"). Repoint the
      comment there.
      **Measured:** `git grep -n -E "indistinguishable|never clicked" --
      CLAUDE.md` prints nothing.
      **Severity:** low.
      **Fixed** in the commit "Repoint comments that cite text the specflow
      adoption deleted": the comment now points at the overlay's `## Hazards`
      ("Basecamp swallows QML errors"), and no longer attributes the blank-view
      claim to a file. `sh dialectica-ui/tests/run-qml-tests.sh
      dialectica-ui/tests/tst_render_probe.qml`: 11 passed, 0 failed.

- [x] **`dev-writer`** — `dialectica/rust-lib/Cargo.toml:36` — *"see `cargo
      test` handling in CLAUDE.md/CI"* points at the "stage the SDK yourself
      before `cargo test`" rule this change moved out of `CLAUDE.md`
      (`origin/main:CLAUDE.md:228`). It is now in the overlay's `## Test layers`,
      and the command itself in `README.md`, "Building". Repoint to `README.md`,
      "Building".
      **Measured:** `git grep -n logos-rust-sdk-src -- CLAUDE.md` prints nothing;
      `README.md:75` holds the staging command.
      **Severity:** low.
      **Fixed** in the commit "Repoint comments that cite text the specflow
      adoption deleted": the comment now names `README.md`, "Building", as
      where the staging command is. `cargo test --manifest-path
      dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` after
      the edit: 1187 + 30 + 3 passed, 0 failed.

- [x] **`dev-writer`** — five comments quote, in quotation marks and attributed
      to `CLAUDE.md`, wording the block replaced:
      `dialectica/rust-lib/dialectica-core/src/log/mod.rs:32-34`,
      `dialectica/rust-lib/dialectica-core/src/moderation.rs:208-209`,
      `dialectica/rust-lib/src/lib.rs:571-572` and
      `dialectica/rust-lib/dialectica-core/src/feed.rs:270-271` quote *"…so 'is
      it called everywhere?' stays a question with an answer"* (the block, at
      `CLAUDE.md:561-562`, now says *"has an answer"*);
      `dialectica/rust-lib/dialectica-core/src/membership.rs:118-120` quotes
      *"…over adding a branch that checks it"* (now *"over a branch that checks
      it"*, `CLAUDE.md:552-554`). The rule is intact; the quotation is not, so a
      reader searching `CLAUDE.md` for the quoted text finds nothing. Re-quote
      the current wording or drop the quotation marks. Stylistic; one pass over
      five comments.
      **Measured:** `git grep -n -E "stays a question|over adding a branch" --
      CLAUDE.md` prints nothing.
      **Severity:** low.
      **Fixed** in the commit "Repoint comments that cite text the specflow
      adoption deleted": all five now quote the block's wording ("has an
      answer"; "over a branch that checks it"). `git grep -n -E "stays a
      question|over adding a branch" -- dialectica dialectica-ui` still lists
      five comments — `authoring.rs`, `wire.rs`, `Main.qml`, `FeedScreen.qml`,
      `DThreadScreen.qml` — that use the phrase as their own prose, not as a
      quotation of `CLAUDE.md`, so they were left. Doc comments only:
      `cargo test` as above, 0 failed. `lib.rs`'s comment sits in the adapter,
      which `cargo test` does not compile; `nix build ./dialectica#lgx` was
      not run for a `///` edit.

- [ ] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:46` —
      *"`gh api …/branches/main/protection` lists four required contexts"* is a
      present-state count a command answers, two lines before the sentence
      explaining that the overlay names the command, not the list, *"because the
      list is a command's answer"*. The decision needs only that the UI jobs are
      not among the required contexts. Drop "four", or pin it to the commit it
      was read at.
      **Severity:** low; breaks the block's "Keeping documents true" rule in the
      paragraph that applies it.

- [ ] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:172-177`
      — the "Stories behind kept rules" bullet lists dropped items by shorthand
      only (*"six findings recovered from a reflog after a branch rename; the
      "wrong for two years" comment"*), with no source file. The section exists
      so a reader can check that nothing was lost silently, and these items
      cannot be found without already knowing the deleted
      `.claude/agents/README.md` (the "wrong for two years" story is its line
      517). The last bullet in the same list does name its source file, so the
      list is inconsistent. Give each item its deleted file. Line 175 also runs
      past the file's wrap width.
      **Severity:** low.

Clean, in prose:

- No reference to a deleted file (`.claude/agents/*.md`, `RUNNER.md`,
  `docs/OPENSPEC-ARCHIVE.md`, `docs/PROJECT-MANAGEMENT.md`) outside
  `openspec/changes/archive/` and this change's own folder. The `.gitignore` and
  `CLAUDE.md:444` mentions of `.claude/agents/` refer to the directory, which is
  still re-admitted, and say why.
- The other code comments that cite `CLAUDE.md` still resolve: "what Bash costs"
  (`check_probe_twins.sh:20`) matches the block's heading; "a guard is a job",
  "put the complexity in the data structure", "pass what it needs", "do not let
  a function quietly acquire a second caller", "the fourth slightly-different
  copy of a guard" and "make room for the change in front of you" are all in
  the block; and the `ci.yml`, `ui-tests.yml`, `check_qml_names.py` and
  `tst_scaffold_values_unchanged.sh` citations point into sections this change
  kept. `transport.rs:456`'s quotation ("keep handler bodies free of partial
  mutation") was not in `CLAUDE.md` on `main` either: older than this change, so
  not filed against it.
- The block's three pointers into the overlay (`## CI gates`, `## Hazards`, the
  overlay's wrappers) all land on a heading that says what they claim.
- `proposal.md`, `tasks.md` and `.openspec.yaml` are clear. `design.md` reads as
  reasoning: its commit list in Context is there to explain why the draft
  diverged, not to narrate what landed.
- Observation, not a box: five of the overlay's seven `## Build` bullets restate
  `CLAUDE.md`'s "Module contract traps" and "Scaffold" sections, which every
  agent also reads. That came from the owner's draft and fills a required
  heading, so it is not filed. But the qmllint finding above shows the cost of
  two copies.
