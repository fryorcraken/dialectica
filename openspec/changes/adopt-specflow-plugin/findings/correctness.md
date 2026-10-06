# Findings — review: correctness

Dimension: **correctness** only. Reviewed `git diff origin/main` on the piece
(HEAD `20643177`), the issue (#195) read fresh, and the plugin checkout at
`agent-spec-flow` `be56964`.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:13` — the QML layer's "Cannot see" column says "`Main.qml` is instantiated by no spec", which is false in this tree
      **Scenario:** a `tester` or `spec-test-reviewer` reads the overlay (as the plugin's `flow` skill tells it to, before its first command), concludes navigation wiring in `Main.qml` is out of reach of the QML suite, and routes a requirement about screen entry/exit to "uncovered" or to the e2e layer — while `tst_navigation.qml` and `tst_stoa_screens.qml` both declare `Component { id: mainComponent; Main {} }` and drive it. `dialectica-ui/tests/check_qml_members.sh:16-20` already records this: "**Main.qml IS instantiated by a spec now** … that particular blind spot is closed and this account is history rather than current state." The clause was carried from the issue's draft (and `CLAUDE.md`'s past-tense `DTheme.noSuchDesk` story) without re-checking. The true blind spot is narrower: a component **no** spec constructs is invisible to `check_bindings`, which `check_qml_members.sh` exists to cover. Severity: medium, since it is a wrong statement of what a gate can see, which the flow says reviewers rely on.
      **Measured:** `grep -n -c "mainComponent.createObject\|createTemporaryObject(mainComponent" dialectica-ui/tests/tst_navigation.qml` → 27; `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_navigation.qml` → 28 passed, every one driving `Main`.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong". The row's Sees column now names `Main.qml`'s
      navigation as driven by `tst_navigation.qml` and `tst_stoa_screens.qml`,
      and its Cannot-see column states the narrower blind spot: an undefined
      binding in a component no spec constructs, with `check_qml_members.sh`
      as what covers members everywhere. "Wiring" is dropped from that column
      for the same reason. Re-measured: `git grep -n -F "Main {" --
      dialectica-ui/tests` lists five specs declaring a `Main` component
      (`tst_e2e_handles`, `tst_feed_mouse_clicks`, `tst_navigation`,
      `tst_stoa_screens`, `tst_thread_navigation`). The `CLAUDE.md` sentence
      you noted outside the boxes is now past tense.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:88` — "The release job's `[ "$(… | grep -c …)" -gt 0 ]` form is deliberate" names the wrong job
      **Scenario:** someone touching the `release` job (`ci.yml:1708-1738`) looks for the form to keep and finds none; someone editing the `build` job's "Stage artifacts" step, where the form actually lives, has no reason to think the overlay protects it. Inherited verbatim from main's `.claude/agents/README.md:502` ("`ci.yml`'s release job carries this one"), so it was already wrong there; the overlay is where it now gets read.
      **Measured:** `grep -n "grep -c" .github/workflows/ci.yml` → the only `[ "$(tar tzf … | grep -c …)" -gt 0 ]` lines are 1689 and 1693, inside `build:` (starts line 1469); `release:` starts at 1708 and contains no `grep` at all.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong": the line now names the `build` job's "Stage
      artifacts" step and the `tar tzf … | grep -c` form, and points at the
      step's own comment for the reason. Re-measured with
      `git grep -n -E "grep -c|^  [a-z-]+:$" -- .github/workflows/ci.yml`:
      the `tar tzf` lines are 1689 and 1693, between `build:` (1469) and
      `release:` (1708). `design.md` lists it among the corrections.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:13` and `:28` against `CLAUDE.md:466` — the overlay prescribes `sh dialectica-ui/tests/run-qml-tests.sh …` as the QML command, while the specflow block's own table lists `sh <relative-path>` under "Costs a click every time"; the sentence that reconciled the two was dropped
      **Scenario:** an agent follows the block (which the `flow` skill says every agent reads before its first shell command), sees `sh <relative-path>` priced as a click, and either avoids the script — reaching for bare `qmltestrunner`, the exact hazard the overlay forbids — or pays an approval click per spec. Main's `CLAUDE.md:217-219` resolved this explicitly: "Passing it one file is the supported shape and needs no approval click." That sentence is dialectica-specific (it depends on this repo's allowlist), it is not in the overlay, and it is not in `design.md`'s "Dropped, deliberately". Since the block cannot be edited here, the fix is a line in the overlay stating the script is this project's supported, unprompted shape — and, if the owner agrees the block's row is too broad, a `## Lessons` candidate upstream.
      **Measured:** `git grep -n "needs no approval click" origin/main -- CLAUDE.md` → line 218; the same search on HEAD over `CLAUDE.md` and `.claude/specflow/PROJECT.md` → no match.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong". `## Test layers` now says one spec file is the
      supported shape, and a new bullet names this project's own commands —
      `nix build …`, `lgs …`, and the script with one spec — as what the
      block's "the project's own" row means, with the script as the exception
      to its `sh <relative-path>` row and an instruction not to route around
      it to a bare `qmltestrunner`. Carried from `origin/main:CLAUDE.md:83`
      and `:218-219`. Not filed as a `## Lessons` candidate: the block's row is
      right for scripts in general, and whether to propose narrowing it is the
      owner's call.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:109-113` — the "Basecamp swallows QML errors" hazard points to `CLAUDE.md`'s "Module contract traps" for "the full account", and tells the reader to set two `[basecamp.env]` switches that the tracked `scaffold.toml` already sets
      **Scenario:** a reader following the pointer finds the `Theme`/`DTheme` collision and the missing-dependency tile in "Module contract traps", but nothing on the two switches or the "three distinct causes, one symptom" account — that lives in `docs/SCAFFOLD.md`, "`[basecamp.env]`: two settings that exist to make failure visible" (lines 91-104). And "To see them, set …" reads as an action to take, when the right action is to check they survived the last `lgs basecamp` verb (`git diff scaffold.toml`), since `lgs` rewrites that file. Before this change the switches were stated in `CLAUDE.md`'s "Before anything else, make the failure visible"; after it, `CLAUDE.md` no longer names them, so the overlay's line is now the only pointer and should aim at `docs/SCAFFOLD.md`. Severity: low.
      **Measured:** `grep -n -E "QT_FORCE_STDERR_LOGGING|QT_LOGGING_RULES" CLAUDE.md docs/SCAFFOLD.md` → hits only in `docs/SCAFFOLD.md:95,98`; `grep -n -E "QT_" scaffold.toml` → lines 58-59 already set both.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong". The hazard now says `scaffold.toml`'s
      `[basecamp.env]` sets both switches, tells the reader to check with
      `git diff scaffold.toml` that they survived an `lgs` verb, points at
      `docs/SCAFFOLD.md`, "`[basecamp.env]`", for what each prevents, and
      narrows the `CLAUDE.md` pointer to what that section holds.
      Re-measured: `git grep -n -E "QT_|basecamp.env" -- scaffold.toml` →
      lines 57-59.

- [x] **`dev-writer`** — `.claude/specflow/PROJECT.md:108` — "the UI scripts refuse it by name" misdescribes the guard
      **Scenario:** a reader expecting a name or version check (and so expecting a renamed or wrapped Go `yq` to slip through, or a correctly-behaving one to be refused) reads `dialectica-ui/tests/require-jq-yq.sh`, whose own comment says the opposite: "It PROBES the one property the callers rely on, YAML in and JSON out, rather than parsing a version string". The refusal message names the tool it wants (kislyuk/yq), not the Go one. Accurate form: the UI scripts probe the `yq` on `PATH` and refuse any that does not turn YAML into JSON. Severity: low.
      **Measured:** `require-jq-yq.sh:28-34` — the only test is `printf 'a: [1, 2]\n' | yq -c .` compared against `{"a":[1,2]}`.
      **Fixed** in the commit "Correct the overlay and CLAUDE.md where review
      found them wrong", in your accurate form: the UI scripts probe the `yq`
      on `PATH` (`require-jq-yq.sh`) and refuse any that does not turn YAML
      into JSON. Read at `require-jq-yq.sh:22-34`.

## Checked and clean

- **The `CLAUDE.md` block is byte-identical to `skills/sync/SKILL.md`'s.**
  `cmp -n 9490 -i 23653:1796 CLAUDE.md …/skills/sync/SKILL.md` reports no
  difference over the whole block, both markers included, and the block ends
  the file (`od` shows `-->\n` at byte 33143, the file's length). Stamp
  `v0.1.0` matches `plugin.json` and the `run` skill's preflight. Note for
  whoever re-verifies: `grep -b` printed offsets 5 bytes off (23658, 1801) on
  this machine; `od -c` is what located the `<` of each begin marker.
- **The overlay passes the plugin's own requirements**: all eight required
  headings present; `grep -c -x "TODO" .claude/specflow/PROJECT.md` → 0;
  role sections (`spec-writer`, `dev-writer`, `tester`, `code-reviewer`,
  `spec-test-reviewer`, `closer`, `pm`) are the permitted kind. `## closer`'s
  `--admin`/`BLOCKED` rules add a prohibition the plugin's `closer` does not
  make and contradict nothing in it (plugin `agents/closer.md` names neither).
- **`.gitignore` re-admits what the piece adds.** `git check-ignore -v -n
  --no-index` reports `.claude/specflow` matched by `!.claude/specflow/`
  (line 71) and `.claude/specflow/PROJECT.md`, `.claude/agents/foo.md` not
  ignored, while `.claude/settings.local.json` and `.claude/worktrees/x` stay
  ignored by `.claude/*`. `git ls-files .claude` → `settings.json`,
  `specflow/PROJECT.md`. All five lines `/specflow:init` writes are present.
- **`settings.json`** parses (`jq .`), keeps `worktree.baseRef: "head"`, and
  `specflow@agent-spec-flow` matches the plugin name in `plugin.json` and the
  marketplace name in `marketplace.json`.
- **No dangling reference outside the archive.** `git grep -n -E
  "agents/README\.md|RUNNER\.md|OPENSPEC-ARCHIVE|PROJECT-MANAGEMENT"` excluding
  `openspec/changes/archive/` and this change → nothing. Wider searches for
  `.claude/agents`, the nine role-file names and the removed `CLAUDE.md`
  section titles hit only `.gitignore`'s deliberate re-admit, the block's own
  generic sentence, and `check_probe_twins.sh`'s pointer to "what Bash costs",
  which still resolves to the block's heading.
- **Commands the overlay names were run and work**: `cargo test
  --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p
  dialectica-core` exits 0 after the README's staging `nix build`; the same
  without `-p` runs **0 tests** and reports `ok`, so the "load-bearing `-p`"
  claim holds (if anything, "almost none" understates it);
  `dialectica-ui/tests/tst_check_qml_names.py` and `check_qml_names.py
  dialectica-ui` both pass; the one-spec QML form runs; `nix flake show
  ./dialectica` lists `packages.x86_64-linux.lgx` and `git ls-files flake.nix`
  is empty, so `./dialectica#lgx` over `.#lgx` is right.
- **CI facts match `ci.yml` and `ui-tests.yml`**: jobs `lint`, `qml`,
  `ui-specs`, `rust`, `build`, `release`; both workflows trigger on
  `pull_request` and pushes to `main`, `ci.yml` also on `v*`; both set
  `cancel-in-progress`; the `Tests` step's `#[test]` count and `every QML spec
  file actually ran` exist as named; the `qmllint` step runs with
  `--unqualified disable -I dialectica-ui/src/qml`; `ci.yml:512-513` (in
  `lint`) records the keystore-accessor mutants as unviable. Branch protection
  requires four contexts (`Lint`, `QML lint`, `Rust core tests`, `Build LGX`),
  so "not every job is a required check" holds.
- **Never-commit entries** each appear in `.gitignore`.
- **Hazards and role rules** check out: `3dddf035` is the stoa-genesis
  commit; `stoa-metadata`'s delta opens on a title line with no Purpose and
  both `spec-backfill` deltas open on `## ADDED Requirements`; `identity` and
  `op-format` each carry an authenticity-is-not-authority requirement;
  `op-ordering`'s Purpose names `op-format`'s no-ordering-field requirement;
  `identity.rs:849` holds `the_wire_constants_are_pinned_to_known_answers`;
  `docs/PHASE0-FINDINGS.md` §3 and §8 are the sections cited; `main` has no
  merge commits and every recent title ends `(#n)`.
- **`design.md`'s factual claims** hold: the cited commits exist with the
  stated subjects; the plugin has no `re-review`, `--admin` or
  `fast-forward` anywhere in `skills/` or `agents/`; its `spec-writer`
  roster is twelve rows against main's thirteen; #170 records the `--admin`
  merge of PR #165; `relevance-votes` has no `## Stages` block;
  `openspec validate adopt-specflow-plugin --strict` passes.
- **Kept `CLAUDE.md` sections are unchanged**: the diff touches only the
  intro, the table rows, the removed generic sections and the appended block;
  "Security posture" is moved, not edited.

Outside this piece's diff, so no box: `CLAUDE.md:258-259` still says in
passing that no spec instantiates `Main.qml`. It is phrased as history of the
`noSuchDesk` incident, but it is the same stale claim as the first box, and
whoever fixes the overlay may want to word that sentence as past tense too.
