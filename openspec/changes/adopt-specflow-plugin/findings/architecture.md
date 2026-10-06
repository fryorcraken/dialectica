# Findings — review: architecture

Dimension: **architecture only**. That means where each rule and fact now lives
(plugin, overlay, `CLAUDE.md`), whether the overlay maps onto the plugin's actual
agents, whether the stage block is the shape the plugin expects, the
plugin-version dependency, and the `.gitignore`/`settings.json` wiring. Reviewed
at `20643177` against `origin/main`. The plugin was read at its checkout HEAD
`be56964` and at its installed cache (`~/.claude/plugins/cache/agent-spec-flow/specflow/0.1.0`).

- [x] **`dev-writer`** — `CLAUDE.md:450-507`, `.claude/specflow/PROJECT.md:18-23,40-41,99-116` — rules that must reach every session now live only in the overlay
      **Scenario:** `CLAUDE.md` on `origin/main` held these inline, in the section it called "the most important section in this file": read YAML/JSON with `yq`/`jq` and never Python; run QML specs through `run-qml-tests.sh` and never `qmltestrunner`; no `QT_QPA_PLATFORM=`/`VAR=value` prefix (with the wrapper named); never `readlink`/`ls` a `/nix/store` path; stage the SDK before `cargo test`; `nix build ./dialectica#lgx` because the root has no flake. After this change each is in the overlay alone. The overlay's own header (`PROJECT.md:4`) addresses "every specflow agent", and the plugin preloads it only into specflow agents (`flow` skill) and `/specflow:pm`. Any other reader of this repo gets only one row in the block's table pointing at the overlay: an ad-hoc owner session, an `Explore`/`general-purpose` subagent, a plugin agent such as `agent-skills:code-reviewer`, or a session fixing dependabot PR #145. For example, such a session asked to check a workflow YAML reaches for `python3 -c 'import yaml…'` and costs the owner a click, which is the exact case the `yq`/`jq` rule was written for. The block contradicts the placement itself (`CLAUDE.md:443-445`): "A rule that must reach every session and agent belongs in `CLAUDE.md`, outside this block". Severity: medium. Either keep a short project section in `CLAUDE.md` outside the block holding the six Bash-cost rules (the overlay can point to it), or record in `design.md` why one table-row hop is enough for non-specflow sessions.
      **Measured:** `grep -n -E "yq|run-qml-tests|/nix/store|logos-rust-sdk-src|dialectica#lgx|QT_QPA_PLATFORM" CLAUDE.md` matches only `--jq` in the generic block, the `path:./dialectica#lgx` flake-ref note and the `check_bindings` account. None of the six rules is there. The same pattern matches 11 lines of `PROJECT.md`.
      **Open — needs the owner's decision.** Issue #195 decides where four of
      these go — "check every dialectica-specific sentence in those sections
      has a home in the overlay: the `run-qml-tests.sh` rule, the
      `QT_QPA_PLATFORM` prefix, `/nix/store` reads, the `nix build .#lgx`
      wrapper…" — and its draft overlay carries them. All six were in
      sections the issue lists as replaced. But the issue does not weigh
      readers outside specflow, and the block's own rule is that a rule for
      every session goes in `CLAUDE.md`, outside the block, "proposed to the
      owner"; so this is the owner's to settle, not mine to tick. Proposal:
      a short `CLAUDE.md` section outside the block, about six lines, each
      rule in one line pointing to the overlay for its reason:
      `yq`/`jq` never Python; `run-qml-tests.sh` never `qmltestrunner`; no
      `VAR=` prefix, the wrappers being that script and
      `nix build ./dialectica#lgx`; never `readlink`/`ls` a `/nix/store`
      path; stage the SDK per `README.md` before `cargo test`. The overlay
      keeps the full text. The alternative is a `design.md` line accepting
      the table-row hop for non-specflow sessions. Decision needed: add that
      section, or accept the hop.
      **Fixed** in the commit "List the six shell rules in CLAUDE.md, outside
      the specflow block". The owner decided: "put them outside the block".
      `CLAUDE.md` gains "Shell rules for every session", between "Security
      posture" and `<!-- specflow:begin v0.1.0 -->`. It has one line per rule,
      each naming the overlay section that holds the reason (`## Hazards`,
      `## Test layers`, `## Build`), and copies no reason. The SDK line points
      at `README.md`, "Building", for the command. The overlay is unchanged:
      nothing in it says these rules live only there. `design.md` records the
      decision under "Six shell rules are listed in `CLAUDE.md`, outside the
      block". The block is untouched. `grep -b -n -F "<!-- specflow:"` gives
      begin/end at bytes 24944/34412 in `CLAUDE.md` and 1796/11264 in the
      plugin's `skills/sync/SKILL.md`, a 9468-byte span in both. `cmp -i
      24944:1796 -n 9489 CLAUDE.md <plugin>/skills/sync/SKILL.md` (span plus
      the 21-byte end marker) exits 0 with no output.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:160-166` — the plugin-version dependency is not recorded, and the issue's premise about it is false
      **Scenario:** #195 says "A directory marketplace is read live from its folder, so there is no version pin yet", and `design.md` says nothing about versions. In fact Claude Code runs a copy, not the checkout. `~/.claude/plugins/installed_plugins.json` gives `installPath …/cache/agent-spec-flow/specflow/0.1.0` with `gitCommitSha 77fce4d`, while the checkout's HEAD is `be56964`. Both call themselves `0.1.0`. `/specflow:run`'s only version check compares the `CLAUDE.md` stamp with the literal `v0.1.0` (`skills/run/SKILL.md` preflight 3), so it cannot tell the two contents apart. If the plugin is reinstalled, the flow silently changes to whatever is in the working tree, uncommitted files included. If it is not reinstalled, a fix committed upstream never arrives. The "byte for byte" `cmp` of the block (`design.md:145-151`) ran against the checkout, not against the copy that runs. It holds today only because `skills/sync/SKILL.md` happens to be identical in both. Severity: medium. Record the plugin commit the overlay and block were reconciled against, and the fact that the flow runs from a version-keyed cache, so drift without a version bump is invisible. That is the self-invalidating form of present state that the block's "Keeping documents true" asks for.
      **Measured:** `diff -rq ~/.claude/plugins/cache/agent-spec-flow/specflow/0.1.0 ~/src/fryorcraken/agent-spec-flow` lists `skills/init/SKILL.md` (checkout +3 lines), `DECISIONS.md` and `LESSONS.md` as differing, and does not list `skills/sync/SKILL.md`. `grep -rn -i -E "live|pin|be56964|77fce4d|cache" openspec/changes/adopt-specflow-plugin/` finds no version or pin statement.
      **Fixed** in the commit "Record the plugin's install, revision and trust
      model, and pin the checks": a new decision, "What runs is an installed
      copy, keyed by version". It says the issue's "read live from its
      folder" is wrong, that a session runs a version-keyed cached copy, that
      two contents can both be `0.1.0` with the stamp and `/specflow:run`'s
      preflight unable to tell, and that the overlay and block were
      reconciled against plugin `77fce4d`, with the command that says when to
      re-check. The pin itself is not decided here: the issue already decides
      there is none until the plugin is published, and the Risks entry from
      `findings/security.md` says the follow-up should pin a commit. Sources:
      the cache path is the plugin's own `DECISIONS.md` ("Verified during the
      v0.1.0 build"), not a read of `~/.claude/plugins`, which is outside this
      agent's working directories; `git -C <clone> diff --stat 77fce4d --
      agents skills` at `756e76a` lists only `skills/init/SKILL.md`, and
      `git -C <clone> diff --exit-code 77fce4d -- skills/sync/SKILL.md` prints
      nothing, so the `cmp` holds against v0.1.0 as committed, not only the
      working tree. `tasks.md` 2.3 now names `77fce4d` as the reference.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/tasks.md:56-58` (and the PR body's "Owner step") — the owner step adds a marketplace entry that is already registered, in a file no worktree has, and does not name the plugin install
      **Scenario:** task 3.3 tells the owner to add `extraKnownMarketplaces.agent-spec-flow` (directory source) to the main checkout's `.claude/settings.local.json`. Three problems. (a) That marketplace is already registered user-wide in `~/.claude/plugins/known_marketplaces.json`, same path, so on this machine the step changes nothing. (b) `settings.local.json` is untracked, so no worktree has it. A session started directly in a piece worktree, which is the plugin's own "a second piece gets a second session in its own worktree", would not see an entry that lived only there. (c) The plugin is installed only at `scope: local` for `projectPath …/agent-spec-flow/tmp/sandbox`. Nothing in the change or the PR says to install `specflow@agent-spec-flow` for this project, or at user scope. I have not verified whether tracked `enabledPlugins` alone makes Claude Code install it. The issue's fourth check, a fresh session listing `specflow:*`, is the only thing that will show it, and it runs after merge. Severity: low to medium. If the owner step is wrong, the first `/specflow:run` fails preflight with no plugin present, or runs a stale cache. Name the actual missing step (install for this project, or confirm `enabledPlugins` triggers it), and correct `design.md:197-199`, which names the marketplace entry as the gap.
      **Measured:** `find …/piece-195-specflow-adoption/.claude -maxdepth 1` lists `settings.json` and `specflow`, with no `settings.local.json`. My own worktree is the same. `installed_plugins.json` has a single `specflow@agent-spec-flow` entry, `"scope": "local"`, sandbox `projectPath`.
      **Fixed** in the commit "Record the plugin's install, revision and trust
      model, and pin the checks". (a) and (b) were already overtaken: the
      step is now `README.md`'s clone plus `claude plugin marketplace add`,
      whose default scope is `user`, and no `settings.local.json` is
      involved. (c) is the real gap, and it is now named: `README.md` gains a
      third step, `claude plugin install specflow@agent-spec-flow --scope
      project` from the repository root, and `tasks.md` 3.3 and the PR body's
      owner step carry it. I could not verify whether a tracked
      `enabledPlugins` alone triggers the install without changing the
      owner's Claude Code configuration, so the README asks for the install
      rather than relying on it. The plugin's own `README.md` lists `install`
      as a step after `marketplace add`, and its `DECISIONS.md` records the
      v0.1.0 test installing explicitly. `design.md`'s marketplace section
      and its Risks entry now name both steps, and `tasks.md` 3.4 gives the
      issue's fourth check an owner row, which is what will show whether it
      worked.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/tasks.md:3-22`, `design.md:189-193` — the owner's decision on #174 items 1–4 has no checkbox, so nothing stops the merge before it is made
      **Scenario:** `design.md` drops four live flow rules from the active flow. The re-review round, `NO SPEC:` routed to the `spec-writer`, the `closer` merging `main` with no force and no conflict resolution, and runner fast-forwards are all replaced by the plugin's rebase plus `--force-with-lease` and conflict-resolving `closer`. The PR body says "Owner decision needed before the next `/specflow:run`". But that decision appears only in prose. Both the in-repo `closer` (Step 1) and the plugin's `closer` gate on `findings/` boxes and stage-block rows only, and task 3.3's unticked box is in `## Implementation`, which neither reads. So the PR can merge green with the question unanswered. After the merge the next `/specflow:run` is the plugin runner, which reads only the overlay, and the overlay records nothing about the regression (`## Lessons` reads `None.`). The first piece then runs a `closer` that force-pushes and resolves conflicts, which this repo's archived decision `2026-09-27-171-workflow-rules` forbade, and no file the runner reads says so. Severity: high for the flow, nil for dialectica's code. This box is the gate: tick it only once the owner's answer is recorded. If they accept the regression, that is a `design.md` line plus the overlay's `## Lessons` with the upstream issue link, per the block's own lessons rule (`CLAUDE.md:446-448`). If they file it upstream first, it is the issue link.
      **Measured:** `grep -n "Lessons" -A 2 .claude/specflow/PROJECT.md` shows `None.` under `## Lessons`. `grep -rn -E "re-review|--admin|fast-forward" ~/src/fryorcraken/agent-spec-flow/skills ~/src/fryorcraken/agent-spec-flow/agents` matches nothing for `re-review` or `fast-forward`, which matches `design.md:73-76`.
      **Fixed** in the commit that ticks this box. The owner took the box's
      second branch, filing upstream: <https://github.com/fryorcraken/agent-spec-flow/issues/1>
      ("Carry dialectica's #174 flow rules into specflow (v0.2.0)") carries all
      five #174 rules into the plugin, and its acceptance list includes the
      overlay's rule-5 copy going once dialectica pins 0.2.0. The link is now
      in `design.md`'s "Carry them upstream" decision and its Risks entry, in
      `proposal.md`'s "Not a no-op for the flow", and in PR #196's body in
      place of the `<issue link>` placeholder.
      `git grep -n -F "agent-spec-flow/issues/1" -- openspec/changes/adopt-specflow-plugin`
      lists the citations. Not done: the overlay's `## Lessons`, which this box
      asks for only on the accept branch, and which is the owner's file. So
      the gap the scenario ends on still holds until the plugin ships 0.2.0:
      the plugin runner reads nothing that names the regression.

## Checked and clean

**`.gitignore` / `settings.json` wiring.** `git ls-files .claude` lists exactly
`.claude/settings.json` and `.claude/specflow/PROJECT.md`. This worktree, cut by
`isolation: "worktree"`, has both. `git check-ignore -v --no-index` ignores
`settings.local.json` (`.gitignore:68`) and leaves the overlay and
`settings.json` tracked. The order `.claude/*` then negations is the one git
honours. `enabledPlugins` sits beside `worktree.baseRef: "head"`, which is unchanged.
Keeping `!.claude/agents/` with the directory empty is harmless, and it matches
`skills/init/SKILL.md` § 2.

**The overlay maps onto the plugin's real readers.** It has all eight required
headings from `openspec/specs/project-overlay/spec.md`, and no line reads exactly
`TODO`. Each role section names a role that reads it. `spec-writer`, `dev-writer`,
`tester`, `code-reviewer`, `spec-test-reviewer` and `closer` all preload
`specflow:flow`, whose "A `## <your role>` section … adds project rules for your
role" covers them. `code-reviewer.md:20`, `dev-writer.md:65` and
`tester.md:59` also cite their sections explicitly, and `skills/pm/SKILL.md:20`
reads `## pm`. No section names an agent the plugin lacks. `## closer`'s
`--admin`/`BLOCKED` rule adds to `agents/closer.md` Step 6 and does not
contradict it. `## Extra stages: None.` is right, since the re-review row cannot
enter there (`design.md:100-102`).

**Stage block shape.** This change's block has the plugin roster's twelve rows
(`openspec/specs/stage-block/spec.md`), with identical text, plus the in-repo
runner's re-review row. That row is correct for a piece the in-repo runner
closes. The archive will freeze it at thirteen rows, and nothing in the plugin
reads an archived block. **Changes in flight:** `openspec list` shows only
`relevance-votes` besides this one. `grep -c "^## Stages"` on its `tasks.md`
prints `0`. It last changed in `f59061d8` (#106), and no `piece/` branch or open
PR carries it. So no in-flight block can be misread by the plugin's runner, and a
blockless change reads as "untracked" under both flows alike. The issue's check
about `ui-remaining-screens` is moot: that change is archived
(`2026-09-28-ui-remaining-screens`).

**References to deleted files.** `git grep` for the deleted paths outside
`openspec/changes/archive/` finds only this change's own folder. The CI comments
that cite `CLAUDE.md` (`ci.yml:164,180,262`, `ui-tests.yml:254`) cite "Module
contract traps" and the `lgs` rewrite note, and both are kept.

**Not boxed, a style preference:** the overlay's `## dev-writer` and
`## code-reviewer` restate `CLAUDE.md`'s "Security posture", which every session
already has injected. That makes two copies of one rule. It is harmless while
they agree, but pointing at the section would be the one-copy form.
