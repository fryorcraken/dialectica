# Re-review, design — adopt-specflow-plugin (#195)

Scope: design-record defects that `20643177..HEAD` introduced or exposed.
Round one's findings are not re-opened; their outcomes were read as claims
and checked.

Read: `git diff 20643177..HEAD` (every non-findings file), `design.md` in full,
`proposal.md`, `tasks.md`, issue #195 (`gh api`), PR #196's body,
agent-spec-flow#1's body and its one comment (`gh api …/issues/1/comments`,
id `6010040773`), the plugin checkout at `756e76a` (`git -C … log --stat
77fce4d..756e76a`, `ls-files`, `DECISIONS.md`, `agents/closer.md`,
`skills/run/SKILL.md`, `skills/init/SKILL.md`, `skills/flow/SKILL.md`),
`origin/main:.claude/agents/RUNNER.md`, and `gh api
repos/fryorcraken/agent-spec-flow`.

## What holds

- **The rule list matches the upstream comment.** design.md's rules 6–12 have
  the same numbering, the same subject and the same locations as the
  comment's "New rules (6–12)"; none is missing and none is extra. Spot-read
  `RUNNER.md:277-291` and `:519-531` at `origin/main`: each says what the
  entry says. The issue body lists 1–5, as design.md says. The three routes
  to `main` match the comment's "Security framing", with the same plugin
  locations; `agents/closer.md:92-98` and `skills/run/SKILL.md:220-232` read
  as described.
- **The plugin account holds.** `77fce4d..756e76a` touches only
  `skills/init/SKILL.md` (`be56964`), `DECISIONS.md`, `LESSONS.md` and
  `README.md`. The cache path is in `DECISIONS.md` "Verified during the
  v0.1.0 build". The plugin tracks no `hooks/`, `.mcp.json`, `bin/` or plugin
  `settings.json`, and `plugin.json` declares none. Every commit
  `77fce4d..756e76a` carries a good signature from the owner's key.
  `marketplace add` and `install` both default to `--scope user` (their
  `--help`). `init/SKILL.md` § 1 writes the allowlist design.md lists, and
  line 16 is the quoted approval sentence.
- **Round one's outcomes are true**, except the two noted in boxes 2 and 4
  below: design-review boxes 1–4, security box 1, architecture boxes 2–3,
  spec-test box 1 each describe text that is in design.md and the tree.
- The owner decisions this round must not reopen — acceptance of the
  regression, the six rules outside the block, the README's three steps, no
  allowlist — are each recorded, and recorded as the owner's.

## Findings

- [ ] **`dev-writer`** — `design.md:163-165` against `design.md:178-179` — the two paragraphs disagree on whether rules 6–12 depend on rule 1
      **Scenario:** line 165 says of rules 6–12, "None of them depends on rules 1 to 5". Line 179, fourteen lines on, says "Rule 11's stop hands its commit to rule 1's round, which the overlay cannot add", and uses that as the reason 11 is not in the overlay. A reader of agent-spec-flow#1 specifying v0.2.0 gets two answers on which rules can land alone. The upstream comment the design cites does not make line 165's claim: it names "Rules 6, 7, 8, 10 and 12" as independent of the original five, not all seven. Rule 9 is classified nowhere in design.md, and its own text depends on rule 1: `RUNNER.md:527-531` justifies the freeze by "Record the call" (step 3), which is the re-review record.
      **Measured:** `origin/main:.claude/agents/RUNNER.md:527-531`; the comment's "Does this issue's own text stand?" section. Restate line 165 as the comment does: 6, 7, 8, 10 and 12 independent; 11 a stop that feeds rule 1's round; 9 justified by rule 1's record but useful without it.

- [ ] **`dev-writer`** — `design.md:286-288` — "The reasons stay only in the overlay, so the two cannot disagree on a reason" is false for one of the six lines
      **Scenario:** `CLAUDE.md:420-421` reads "the repository root has no flake, so `.#lgx` fails there", which is the reason, and the overlay's `## Build` (`PROJECT.md:45-46`) states the same reason. They agree today. design.md's sentence is what tells the next editor they need not check. `findings/architecture.md` box 1's outcome repeats it ("copies no reason"). Separately, `design.md:279` calls one rule "no-`/nix/store`-read", and the PR body says "no `/nix/store` reads". The rule at `CLAUDE.md:416-417` and `PROJECT.md:126-128` forbids only `readlink` or `ls` of a store path to find an artefact. The overlay says reading builder source does mean reading the store.
      **Measured:** `Read CLAUDE.md` 404-421 and `.claude/specflow/PROJECT.md` 45-46, 126-128. Either drop the clause from `CLAUDE.md:421`, or have design.md say which line carries its reason and why. Name the `/nix/store` rule as it is written.

- [ ] **`dev-writer`** — `design.md:299-308`, `design.md:323-325` and the Risks entry at `design.md:424-440` — the marketplace decision gives "unpublished" as its forcing constraint, which no longer holds as stated, and records no alternative to a local clone of a moving branch
      **Scenario:** `gh api repos/fryorcraken/agent-spec-flow` reports `"private":false`, created 2026-10-06, and the README itself clones it from `https://github.com/fryorcraken/agent-spec-flow`. So "While the plugin is unpublished its marketplace is a local clone" leaves out the constraint that actually applies. The plugin's `DECISIONS.md` decision 20 (`6023db6`) says: "No release tag, and dialectica stays on its local marketplace, until a full piece has run on the plugin". design.md does not cite it. Two alternatives are now available and neither is recorded with what rules it out. (a) A tracked `github` marketplace source pinned to a commit. This would close the Risks entry's "names `specflow@agent-spec-flow` with no source or revision", and that entry already asks for a pinned commit as the follow-up. (b) A README clone checked out at the reconciled `77fce4d`. Today the README clones the default branch, which is `756e76a` now and could be anything later. A contributor therefore installs a revision design.md's "What runs is an installed copy" never reconciled. The re-check command at `design.md:344-345` is the only guard, and the README does not mention it.
      **Measured:** `gh api repos/fryorcraken/agent-spec-flow` (`private:false`, `default_branch: main`); plugin `DECISIONS.md:37`; `README.md` "Working with the agent flow", step 1 (no checkout). Record decision 20 as the constraint, and add (a) and (b) to the alternatives with what rules each out.

- [ ] **`dev-writer`** — `design.md:214-221` — the decision to leave the overlay's `## Lessons` at `None.` after the regression was accepted and filed upstream is not recorded, and a round-one outcome misstates which branch the owner took
      **Scenario:** the block's rule (`CLAUDE.md:469-471`) pairs two steps for a specflow gap the owner agrees to: record it under the overlay's `## Lessons`, and file it on `agent-spec-flow`. The plugin's `run` skill says the same (`skills/run/SKILL.md:239-244`). This change did the second (agent-spec-flow#1) but not the first: `PROJECT.md:139-141` still reads `None.`. `findings/architecture.md` box 4 asked for a `## Lessons` entry "if they accept the regression". Its outcome says the owner "took the box's second branch, filing upstream", that `## Lessons` was asked "only on the accept branch", and that "the plugin runner reads nothing that names the regression". design.md:216 records that the owner **accepted**. So the accept branch is the one taken, and its second half is neither done nor ruled out in design.md. The owner's "the overlay does not carry the rules" is not this question: a `## Lessons` line that points at the issue carries no rule. The issue named `PROJECT.md` among the `.claude/` changes the owner asked for, so "the owner's file" does not settle it either. This box does not ask for the entry. It asks that design.md say whether `## Lessons` points at agent-spec-flow#1, and if not, why, and what the plugin runner reads instead that names the regression.
      **Measured:** `Read .claude/specflow/PROJECT.md` 139-141; `findings/architecture.md:87-102`; `CLAUDE.md:469-471`; plugin `skills/run/SKILL.md:239-244`.

- [ ] **`dev-writer`** — `design.md` (no entry) — the edits outside the issue's scope, eight source comments and one phrase in another live change's `tasks.md`, are argued only in `findings/`, which the `closer` deletes
      **Scenario:** issue #195's "What changes" table lists `.claude/`, `CLAUDE.md`, two docs, `settings.json` and `.gitignore`, and nothing under `dialectica/`, `dialectica-ui/` or another change's folder. This piece edits `feed.rs`, `keystore.rs`, `log/mod.rs`, `membership.rs`, `moderation.rs`, `src/lib.rs`, `Cargo.toml`, `tst_render_probe.qml` and `openspec/changes/relevance-votes/tasks.md`. The PR body says this in one bullet. design.md says nothing, and `design.md:211` still describes `relevance-votes` only as having no stage block, not as a folder this piece edits. The reasoning that makes these edits safe sits in `findings/readability.md:142-151` and `findings/spec-test.md:15-26`: they repoint citations of deleted text, they are comment-only, and `cargo test` passes. That includes the accepted gap that `src/lib.rs`'s `///` edit sits behind `cfg(logos_scaffold)`, which `cargo test` does not compile, and `nix build ./dialectica#lgx` "was not run for a `///` edit". All of it is deleted with `findings/` at close, and the `closer` is told to confirm durable reasoning moved to design.md first. A departure from the issue's scope has to be argued in design.md, and this one has an unrun gate behind it. Record the departure, why each edit was needed (each cited text this change deleted), and what makes it safe, the `lib.rs` build gap included.
      **Measured:** `git diff 20643177..HEAD --stat -- dialectica dialectica-ui openspec/changes/relevance-votes` (9 files, every hunk a comment); `git grep -n -F -e "relevance-votes" -e "Cargo.toml" -e "tst_render_probe" -- openspec/changes/adopt-specflow-plugin/design.md` hits only `:211`.

## Outside this review's scope, for the runner

- `tasks.md:72-75`, "Worked at this change's last edit to `CLAUDE.md`: begin
  23900", is out of date. `3b32d6f8` edited `CLAUDE.md` after that line was
  written, and `grep -b -n -F "<!-- specflow:" CLAUDE.md` now prints `24944`
  and `34412`. The method still holds. Only the worked numbers are stale.
- agent-spec-flow#1's comment cites dialectica's overlay `PROJECT.md:185-192`
  for rule 5. In this tree that rule is at `PROJECT.md:197-204`. The owner
  made accurate tracking the condition of acceptance, so the line cite is
  worth correcting, or replacing with the section name `## closer`.
