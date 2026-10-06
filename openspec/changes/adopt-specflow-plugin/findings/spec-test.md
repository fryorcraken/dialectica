# spec-test review — adopt-specflow-plugin (#195)

Contract: issue #195 ("What changes", "Checks", the acceptance conditions),
`proposal.md`, and `tasks.md`'s stage block and task list. Tests: the `Verify:`
lines in `tasks.md` and the issue's four Checks. No spec delta and no test
suite, so this review judges whether each check can fail and whether it pins
what the issue requires. Read fresh: the issue via `gh api`, the plugin's
contract from `/home/fryorcraken/src/fryorcraken/agent-spec-flow` (clean,
HEAD `be56964`; `skills/sync/SKILL.md` unchanged since the v0.1.0 commit
`77fce4d`).

- [x] **`dev-writer`** — `tasks.md:65` (4.2 Verify) and the issue's first Check — the reference search misses a deleted file cited by another spelling, and two such citations are live
      **Scenario:** a citation of the deleted `.claude/agents/README.md` written as "agents README" survives the search, which only matches `agents/README\.md`, `RUNNER\.md`, `OPENSPEC-ARCHIVE` and `PROJECT-MANAGEMENT`. It also names 4 of the 11 deleted files: none of the seven role files (`closer.md`, `code-reviewer.md`, `design-reviewer.md`, `dev-writer.md`, `spec-test-reviewer.md`, `spec-writer.md`, `tester.md`) is searched for.
      **Measured:** `git grep -n -i -E "flow README|role file|agents README|RUNNER\b" -- . ":!openspec/changes/archive/" ":!openspec/changes/adopt-specflow-plugin/"` finds `dialectica/rust-lib/dialectica-core/src/keystore.rs:3105` ("Per the agents README that is worse than a missing test") and `openspec/changes/relevance-votes/tasks.md:114` ("agents README: \"never write a scenario that cannot be tested\""). The 4.2 Verify prints neither. The seven role-file names are clean today; the only hits for `\.claude/agents` are `.gitignore:49,69` (the kept re-admit) and `CLAUDE.md:444` (inside the plugin block). Repoint or remove the two citations; the rule each quotes now lives in the `specflow:flow` skill. Widen the Verify to all eleven names plus `agents README`, case-insensitive. Severity: medium. The issue's check exists to catch exactly this, and it passes over two hits.
      **Fixed** in the commit "Repoint comments that cite text the specflow
      adoption deleted". `keystore.rs` keeps its own claim and names the
      `flow` skill's "say what a check cannot see" rule; `relevance-votes/tasks.md`
      now cites the `flow` skill, which carries "Never write a scenario that
      cannot be tested" verbatim (`skills/flow/SKILL.md:259` in the plugin).
      That second edit is a one-phrase citation repoint in another change's
      folder, so this piece now touches `relevance-votes` too; the edit changes
      nothing about its stage-block state. 4.2's Verify is widened to the
      eleven names plus `agents README`, case-insensitive, and prints nothing
      on this commit. Your broader search (`flow README|role file|agents
      README|RUNNER\b`, `-i`) now hits only the word "runner" in prose and
      `qmltestrunner`, none of them a citation.

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/log/mod.rs:32-34` and `dialectica/rust-lib/dialectica-core/src/membership.rs:118-120` — comments quote, in quotation marks, `CLAUDE.md` wording that the block replaced and that no longer appears there
      **Scenario:** a reader checks the quoted rule against `CLAUDE.md` and finds a different sentence. That is the citation-that-reads-like-evidence shape this repo has been bitten by. No check in the issue or `tasks.md` covers prose that quotes the deleted sections, as opposed to prose that names a deleted file.
      **Measured:** `log/mod.rs` quotes "Keep it separate, so 'is it called everywhere?' stays a question with an answer." The block (`CLAUDE.md:561-562`) reads "…has an answer." `membership.rs` quotes "…over adding a branch that checks it." The block (`CLAUDE.md:553`) reads "…over a branch that checks it." The block cannot be edited (it is `/specflow:sync`'s), so the comments are what changes. Drop the quotation marks or quote the new text. Every other `CLAUDE.md's "…"` citation outside `.claude/` still resolves: "a guard is a job", "pass what it needs", "do not let a function quietly acquire a second caller", "the fourth slightly-different copy of a guard", "make room for the change in front of", "what Bash costs", and "Module contract traps". Severity: low.
      **Fixed** in the commit "Repoint comments that cite text the specflow
      adoption deleted", together with the three further quotations
      `findings/readability.md` names (`moderation.rs`, `lib.rs`, `feed.rs`):
      each now quotes the block's current wording. See that file's box for the
      test run.

- [x] **`dev-writer`** — `tasks.md` (no row) and the issue's second Check — "`openspec/changes/ui-remaining-screens/tasks.md` still has its `## Stages` block" names a path that no longer exists, and `tasks.md` drops the check without saying so
      **Scenario:** run literally, the check fails on a missing file whatever this change does. Read leniently, it passes vacuously. Either way it measures nothing about "any other change in flight keeps working".
      **Measured:** `git ls-files openspec/changes` shows the change archived as `openspec/changes/archive/2026-09-28-ui-remaining-screens/` (#189, `645b4e73`). The only other non-archived change is `relevance-votes`, and `git grep -n -E "^## Stages" -- openspec/changes` finds no stage block in it. That predates this piece and is not caused by it, but it means there is currently no in-flight stage block for the claim to be about. A correct check has two parts. (a) This piece touches no other change: `git diff --stat origin/main...HEAD` lists only `openspec/changes/adopt-specflow-plugin/` under `openspec/`, which I measured. (b) The plugin's own detection, `grep -c "^## Stages" openspec/changes/<name>/tasks.md` (`skills/run/SKILL.md:103`), runs on each non-archived change. Record the corrected check and its result in `tasks.md`, and note in the PR that the issue's check was stale. Severity: low.
      **Fixed** in the commit "Record the plugin's install, revision and trust
      model, and pin the checks": `tasks.md` 4.3 records the stale path, the
      corrected two-part check and its results, and the PR body notes it.
      Re-run on this branch: (a) `git diff --stat origin/main...HEAD --
      openspec` lists this change's folder and
      `openspec/changes/relevance-votes/tasks.md`, whose 2-line edit is the
      citation repoint from your first box; it touches no stage-block line.
      (b) `openspec list` shows `adopt-specflow-plugin` and
      `relevance-votes`; `grep -c "^## Stages"` on their `tasks.md` prints
      `1` and `0`.

- [x] **`dev-writer`** — `tasks.md:35-36` (1.1 Verify) — the heading half has no command, and neither half can see a required heading whose section is empty
      **Scenario:** a required section loses its whole body, which is what a lost dialectica rule looks like. The overlay still has eight headings and no `TODO` line, so the Verify passes. `/specflow:run` passes too, because it refuses only on a line reading exactly `TODO` (`openspec/specs/project-overlay/spec.md:34` in the plugin).
      **Measured (mutation, restored with `git restore`):** I deleted lines 55-60 of `.claude/specflow/PROJECT.md`, the whole body of `## Mutation tool`. `grep -c -x "TODO" .claude/specflow/PROJECT.md` printed `0`, and `grep -c -E "^## (Test layers|Build|Mutation tool|CI gates|Never commit|Hazards|Extra stages|Lessons)$" .claude/specflow/PROJECT.md` printed `8`. Both survived. Write the heading command into the Verify (the one above works; on the unmutated tree it prints `8`), and add a check that each required heading has a non-blank line before the next `## `. Severity: low. The issue's check mirrors the plugin's contract exactly, but the issue's requirement ("holds every dialectica-specific rule") is about content, and nothing measures content.
      **Fixed** in the commit "Record the plugin's install, revision and trust
      model, and pin the checks": 1.1's Verify now gives three commands, your
      heading count, the `TODO` count, and `grep -n -A2` over the eight
      headings, read for each heading's third line being text. Watched
      failing: with `## Mutation tool`'s body deleted by `Edit` and restored
      the same way, the first two printed `8` and `0` and the third printed
      `58:## Mutation tool`, `59-`, `60:## CI gates`. On the restored tree
      they print `8`, `0`, and eight groups each ending in a `<n>-` text line.
      The Verify says plainly that none of the three judges content; that
      stays 1.2's reading.

- [x] **`dev-writer`** — `tasks.md:47-49` (2.3 Verify) — the `cmp` check gives no command, no offsets and no reference revision, so a re-reviewer or the `closer` cannot reproduce it
      **Scenario:** offsets read with `grep -b -o` are match offsets, not line starts. My first attempt used 23658/1801, which is 5 bytes into each begin line, and `cmp` then reported "EOF on CLAUDE.md". That reads as a mismatch, but it came from the span, not the content. The reference is also a live directory marketplace with no version pin. The plugin checkout has no `v0.1.0` tag (`git -C … tag` prints nothing), so the same command run after the plugin moves compares against a different block while the stamp still reads `v0.1.0`. On "both files edited alike": the plugin file is outside this repo, its checkout is clean, and `skills/sync/SKILL.md` is unchanged between `77fce4d` and `be56964`. So that risk is ruled out today, but only by these extra measurements, which the Verify does not name.
      **Measured:** `cmp -i 23653:1796 -n 9490 CLAUDE.md /home/fryorcraken/src/fryorcraken/agent-spec-flow/skills/sync/SKILL.md` exits 0 with no output. The span is begin-line start to EOF: 33143 − 23653 = 9490 bytes, which is also 11269 + 16 + 1 − 1796 = 9490 for the plugin's end-marker line. The blocks are identical. Write that command, or an equivalent, into the Verify, and name `77fce4d` as the reference. Severity: low.
      **Fixed** in the commit "Record the plugin's install, revision and trust
      model, and pin the checks": 2.3's Verify names `77fce4d`, checks the
      clone's file against it with `git -C <clone> diff --exit-code 77fce4d
      -- skills/sync/SKILL.md` (prints nothing), and gives the derivation
      rather than fixed numbers, because this pass's `CLAUDE.md` edits moved
      the begin offset from 23653 to 23900. The cause of the 5-byte trap is
      recorded: this machine's `grep -b` prints the match offset even without
      `-o`, so the pattern must start at `<!--`. With
      `grep -b -n -F "<!-- specflow:"`: `CLAUDE.md` 23900 and 33368,
      `wc -c` 33390; `SKILL.md` 1796 and 11264. Span 9490 both sides;
      `cmp -i 23900:1796 -n 9490 CLAUDE.md <clone>/skills/sync/SKILL.md`
      exits 0, and `cmp -i 23900:1801 …` reports "differ: byte 1", so the
      check can fail.

- [x] **`dev-writer`** — `tasks.md:56-58` (3.3) and the issue's fourth Check — "a session started after the merge lists the `specflow:*` agents and skills" has no row and no owner, and it is the only check that can show `enabledPlugins` works
      **Scenario:** the marketplace entry is missing or misnamed in `settings.local.json`. Nothing fails until the next piece's `/specflow:run`, by which time the in-repo agents are gone. Every other check passes on a tree where the plugin never loads.
      **Measured:** this check cannot run before the merge. This agent's session was dispatched from the piece, and it lists no `specflow:*` skill or agent type, which is expected before 3.3. What can be checked today holds. `jq . .claude/settings.json` shows `enabledPlugins` containing `specflow@agent-spec-flow: true` beside `worktree.baseRef: "head"`. The key matches the plugin's manifests: marketplace `name` is `agent-spec-flow`, plugin `name` is `specflow`, version `0.1.0`. Add the post-merge session check as an owner row beside 3.3, so it is ticked by someone rather than assumed. Severity: medium, because it is the only end-to-end check and it currently belongs to nobody.
      **Fixed** in the commit "Record the plugin's install, revision and trust
      model, and pin the checks": `tasks.md` 3.4 is that owner row, unticked,
      saying why it is the only end-to-end check. 3.3 now also carries the
      plugin install step that `findings/architecture.md` found missing.

## Clean, and what the change decided that the issue did not

I measured these and they hold. None has a `Verify:` line, but each is pinned by
the command given here:

- **Kept `CLAUDE.md` sections are byte-identical to `main`.** `git diff -U0
  origin/main HEAD -- CLAUDE.md` has no hunk inside main's lines 318-682 ("What
  this is" through "Scaffold") or 742-756 ("Security posture"). The "Where to
  look for what" table holds exactly the `docs/SCAFFOLD.md` and `docs/SOURCES.md`
  rows. No kept section cites a removed section by title.
- **"No behaviour change" for the product.** `git diff --stat
  origin/main...HEAD` touches only `.claude/`, `CLAUDE.md`, `.gitignore`, the two
  deleted `docs/` files and this change's folder. No code, test, workflow or spec
  file is touched.
- **`.gitignore`.** `git check-ignore --no-index -v
  .claude/specflow/PROJECT.md` exits 1 (not ignored). What actually puts the
  overlay in every worktree is that the file is tracked. Removing the
  `!.claude/specflow/` line would not take it out of any worktree; it would only
  make a future `git add` of a new file there refuse loudly.
- **`openspec validate adopt-specflow-plugin --strict`** passes, and honours
  `skip_specs: true` alongside `schema: spec-driven`.
- **"Every dialectica-specific sentence has a home" (1.2)** has no mechanical
  check and cannot easily have one: it is a reading. I sampled it and found
  every item present in the overlay. Sampled: the six items the issue names
  (`run-qml-tests.sh`, `QT_QPA_PLATFORM`, `/nix/store`, the `#lgx` wrapper,
  which is now `./dialectica#lgx`, the QML logging env, `dialectica/metadata.json`),
  plus the `stoa-genesis` archive trap, the `pm` policy, the `--admin` and
  `BLOCKED` closer rules, the `yq`/`jq` rule, SDK staging, and the
  `the_wire_constants_are_pinned_to_known_answers` reference. The
  design-reviewer's reading is the only gate on the rest.

Choices the issue left open, which the change made. None needs a box here:

- `docs/PROJECT-MANAGEMENT.md`: the issue said "delete, or reduce". The change
  deleted it.
- The overlay was reconciled against `645b4e73` rather than taken from the
  issue's `6a7e02b9` draft, and it adds a `## spec-writer` section the draft
  did not have.
- `!.claude/agents/` is kept although the directory is now empty.
- The reference search exempts this change's own folder. The issue's wording
  ("hits only `openspec/changes/archive/`") only becomes literally true once
  this change is archived.
- **The issue's "No behaviour change" premise is false for the flow.** #174's
  rules are not in plugin v0.1.0. The change says so in `proposal.md`, and the
  PR body asks for an owner decision before the next `/specflow:run`. That is
  the owner's to settle, not a test gap.
