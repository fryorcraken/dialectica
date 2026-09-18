# Tasks — move the Bash-costs rules into CLAUDE.md

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change reverts
      agent-instruction files and edits `CLAUDE.md` prose; it alters no module
      behaviour and no requirement. Declared as `skip_specs: true` alongside
      `schema:` in `.openspec.yaml`, which carries the argument.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — prose and CI-config changes only. The one
      executable claim (that `ci.yml` still parses and the surviving gates still
      run) is verified below rather than by a new test; adding a test file under
      `.claude/` is the precise thing this change exists to stop doing.
- [x] review: correctness — `code-reviewer`
- [ ] ~~review: security — `code-reviewer`~~ — no untrusted input, no logic, no
      network or storage surface touched.
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] ~~review: spec-test — `spec-test-reviewer`~~ — no spec delta
      (`skip_specs: true`), nothing for a spec-test review to check.
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

### The revert

- [x] **`git revert` of `51ab7f8`**, which applied cleanly to every path but
      one. The seven role files and `.claude/agents/README.md` are back to their
      pre-#119 content, `BASH-COSTS.md` and the two gate scripts are deleted, and
      the archived `openspec/changes/archive/2026-09-18-agent-bash-costs/` record
      goes with them — it documents work that no longer exists.
- [x] **`README.md`'s "Every agent pays CLAUDE.md's Bash costs" section is whole
      again**, including the never-chain rule, the do-not-pipe-a-long-output
      paragraph and the `tar | grep -q` SIGPIPE trap that #119 had moved out.
- [x] **`ci.yml`'s two Bash-costs `lint` steps removed.** This was the revert's
      only conflict: the QML-reachability gate landed inside the same block
      after #119, so the revert could not apply mechanically. Resolved by
      dropping only the Bash-costs steps and their comment.
- [x] **The surviving gates still run.** `grep -n` over `ci.yml` finds
      `check_probe_twins`, `check_qml_reachable`, `check_qml_names` and
      `check_qml_members` each still invoked with their own tests, and finds no
      remaining `bash_costs` or `BASH-COSTS` reference anywhere in the tree.
- [x] **`ci.yml` still parses** —
      `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
      returns clean.

### The fold into CLAUDE.md

Each item below is something the existing "How to work in this repo, and what
Bash costs" section did **not** already carry. Everything else in
`BASH-COSTS.md` was checked against that section and found to be a duplicate; it
is not re-added.

The first pass of that check missed one row, found by the readability review and
folded in below: the claim of completeness was made before it held. `git show
51ab7f8:.claude/agents/BASH-COSTS.md` is the source to re-check against, which is
the point of naming it rather than asking the next reader to trust the sweep.

- [x] **`for`/`while` named in the costs table.** The row said "a loop", which
      does not read as a ban on the specific construction an agent reaches for.
      Now `for`/`while` by name, with the `env VAR=` prefix added beside the
      bare `VAR=value` one.
- [x] **Reads outside the working directories are a named cost**, with the three
      measured instances spelled out: `/tmp`, the session scratchpad, an
      unpacked package. Previously this lived only in the scratch-files section,
      as a convention rather than as a thing that costs a click.
- [x] **The absolute/relative row is corrected.** It read "an **absolute** path
      as an argument" free / "a **relative** path" costly, which contradicts the
      worktree dispatch flow: an agent arrives standing in the right tree. The
      row now distinguishes what it was actually about — a path the checker can
      resolve *before* the command runs — and a new paragraph says to prefer
      plain relative paths inside your own worktree, naming the measured
      incident where a typo'd username in a long absolute path became a blocked
      read rather than a missing file.
- [x] **Reading an old version of a file is a costs-table row.** `git diff
      origin/main -- <path>` is one plain command; materialising the old version
      to a scratch file first is the shape it replaces. This row was in
      `BASH-COSTS.md`, had no counterpart in the pre-#119 `CLAUDE.md`, and was
      dropped rather than folded on the first pass.
- [x] **`Grep`/`Glob` named as the replacement for a corpus file.** CLAUDE.md
      insists a ban with no named replacement redirects the habit, and the
      loop-that-builds-a-file-to-grep case had no replacement named. Added to
      the existing "when you forbid a tool, name the replacement" bullet rather
      than as a new section.
- [x] **The `env VAR=` prefix named against its measured incident.**
      `QT_QPA_PLATFORM=offscreen qmltestrunner …` is now called out beside the
      existing `qmltestrunner` correctness rule, with the replacement: the
      wrapper that sets the environment, and `nix build .#lgx` for
      scaffold-gated Rust code.

### `.claude/` is the owner's

- [x] **New section, placed before the Bash-costs section** because it governs
      whether you may act at all rather than how. Covers everything under
      `.claude/` — role files, `README.md`, `RUNNER.md`, `settings.json`, hooks,
      skills, anything added later — and names the propose-instead alternative.
- [x] **"It would make agents work better" is named as not-authorisation**, with
      the instance that produced the rule, because a rule naming only its own
      instance would not have prevented that instance.
- [x] **The narrower `settings.json` line is generalised, not duplicated.** It
      now points at the new section instead of asserting a parallel rule about
      one file — a specific rule about one file is what invited reading the rest
      of `.claude/` as fair game.
- [x] **The asymmetry is stated once**: `CLAUDE.md` reaches every session and
      agent at startup; `.claude/agents/*.md` reaches only the agent it names.
      That is what makes the rule stick rather than read as territorial.

### Correction carried across the revert

- [x] **`dev-writer.md`'s "Absolute paths" bullet is fixed in place.** It
      contradicted the worktree flow and every other role file. This defect
      predates #119 and is not part of the unauthorised restructure, so it is
      corrected rather than restored verbatim — narrowly, without reintroducing
      the "What Bash costs here" section #119 added.
- [x] **The causal chain is recorded in `CLAUDE.md`, not only in a commit
      message.** The backwards absolute/relative table plausibly put the wrong
      rule in `dev-writer.md`, which plausibly produced the measured typo'd-path
      incident. It sits beside the incident it explains, where a reader meets the
      fact, and is labelled plausible rather than proven — nobody asked the agent
      why it typed that path.
