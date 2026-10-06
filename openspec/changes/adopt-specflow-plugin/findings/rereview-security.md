# Re-review, security — adopt-specflow-plugin

Dimension: **security** only. Scope: `20643177..HEAD` excluding `findings/`
(15 files: the README's new "Working with the agent flow", `CLAUDE.md`, the
overlay, `design.md`, `proposal.md`, `tasks.md`, and comment edits in Rust,
QML, `Cargo.toml` and `relevance-votes/tasks.md`). Each answered box in the
first round's `findings/security.md` was read as a claim and checked. The
plugin was read at `<clone>` HEAD `756e76a`.

- [x] **`dev-writer`** — `openspec/changes/adopt-specflow-plugin/design.md:434` (with `README.md:98`) — the first round's box 2 is marked Fixed, but its trust account leaves the risk wider than it records. The new Risks entry says the flow's push, rebase, archive and merge rules moved out from under `main`'s "signed-commit, PR-only protection". It calls the risk low because the plugin's "commits are signed by the owner". It never says that the destination has no protection at all. `agent-spec-flow`'s `main` requires no signature and no pull request, and it does not refuse a force-push. The new README makes the gap wider: it tells every contributor to `git clone` that repository's moving default branch and install whatever it holds, with no pinned commit and no verification step. "Signed by the owner" describes the five commits that exist today. It binds nothing a newcomer clones tomorrow. Severity: low (the namespace is the owner's), but the recorded mitigation is not a mitigation.
      **Scenario:** someone obtains a GitHub credential for the owner's account, such as a leaked `gh` token, without the GPG key. Before this change, rewriting the `closer`'s merge rules meant a signed commit to dialectica's `main`, which `required_signatures` and `enforce_admins` enforce. After it, the same token can push an unsigned commit, or force-push, to `agent-spec-flow` `main`, and nothing on GitHub refuses either. Any contributor who follows `README.md`'s step 1 afterwards installs the rewritten `closer` as `specflow@agent-spec-flow` 0.1.0, under the version the overlay was reconciled against, and nothing in this repo tells them to check. Combined with the accepted `required_approving_review_count: 0`, those instructions govern what reaches dialectica's `main`.
      **Measured:** `gh api repos/fryorcraken/agent-spec-flow/branches/main/protection` → 404 `"Branch not protected"`; `gh api repos/fryorcraken/agent-spec-flow/rulesets` → `[]`; `gh api repos/fryorcraken/agent-spec-flow/rules/branches/main` → `[]`. `git grep -n -i -E "protect|unpinned|verify-commit|signed" -- design.md README.md` finds no statement of the plugin repo's own protection, and no pin or verify step in the README. Fix, in one of two forms. Either record the true state in the Risks entry, that the destination has no branch protection and the README clones an unpinned branch, and drop "signed by the owner" as the reason the risk is low. Or make the README check out the revision `design.md` says the overlay was reconciled against, for example `git -C <dir> checkout 756e76a`, optionally with `git -C <dir> verify-commit HEAD`, so that the recorded reconciliation is what contributors run. The owner can also turn on protection for `agent-spec-flow` `main`, but that is the owner's action, not this piece's.
      **Deferred**, the pin, to `agent-spec-flow`'s release, by the owner's
      decision for this pass: `README.md` keeps cloning the default branch
      with no pinned checkout until then. It now lives in `design.md`, whose
      Risks entry is corrected in the commit "Record the plugin repository's
      real trust state, and the marketplace constraint", the box's first
      form. Re-measured, plain: `gh api
      repos/fryorcraken/agent-spec-flow/branches/main/protection` returns 404
      "Branch not protected"; `…/rulesets` and `…/rules/branches/main` both
      return `[]`. The entry now says the destination has no protection
      (no signature, no pull request, no force-push refusal), that the
      README installs that moving default branch with no pin and no
      verification, and walks this box's credential scenario. "Signed by the
      owner" is kept only as the author's own practice, explicitly not a
      mitigation; the low rating rests on the namespace being the owner's.
      It names `77fce4d` as the reconciled revision, points at "What runs is
      an installed copy" for the re-check command (derived fresh:
      `git -C <clone> diff --stat 77fce4d -- agents skills`, which at
      `756e76a` lists only `skills/init/SKILL.md`), and says pinning waits
      for the release and that protecting `agent-spec-flow`'s `main` is the
      owner's action.

## Checked and clean

- **Comment-only source edits.** In `git diff 20643177..HEAD` every changed
  line in `feed.rs`, `keystore.rs`, `log/mod.rs`, `membership.rs`,
  `moderation.rs`, `src/lib.rs`, `Cargo.toml` and `tst_render_probe.qml` is a
  `///`, `//!`, `//` or `#` comment line. No code, attribute or dependency line
  changed.
- **"Security posture"** reads the same at `origin/main:CLAUDE.md:742-756` and
  `HEAD:CLAUDE.md:388-402`, line for line, and both standing rules are intact.
  The overlay's `## dev-writer`, `## code-reviewer` and `## closer` sections are
  untouched in this range.
- **What each README step grants.** `claude plugin marketplace add` declares
  the marketplace at `user` scope by default (`--help`), so it reaches every
  project and worktree on the machine, which `design.md` says.
  `claude plugin install … --scope project` writes `enabledPlugins` into the
  tracked `.claude/settings.json`, which already holds exactly that entry and
  no `permissions` or `extraKnownMarketplaces` key. The installed copy lives in
  the user-level cache (`DECISIONS.md:58-60`), shared across worktrees. The
  README's `git diff .claude/settings.json` check would show any extra write
  the install made. I did not run the install, because it changes the owner's
  Claude Code configuration.
- **"Enabling it runs no code and grants no permission" holds at `756e76a`.**
  `git -C <clone> ls-files` lists no `hooks/`, `.mcp.json`, `bin/` or plugin
  `settings.json`. `plugin.json` declares no inline `hooks` or `mcpServers`.
  No agent or skill frontmatter carries `allowed-tools`, `tools`,
  `permissionMode`, `hooks` or `mcpServers`. No skill or agent contains a
  `` !` `` shell injection (`git grep -P "!\x60"` is empty).
  `scripts/check-version.sh` is a maintainer script, referenced only from
  `README.md`, `DECISIONS.md` and `LESSONS.md`.
- **The six shell rules in `CLAUDE.md`** each forbid a shape or name a
  narrower command; none advises a pipe, a `cd`, a prefix or a write. The
  overlay's new "project's own commands" bullet is prose. The allowlist stays
  machine-local and `settings.json` gains no `permissions`.
- **Protective rules.** No new text mentions or relaxes pushing `main`,
  `--admin`, force-push, `--no-gpg-sign` or `.claude/` ownership. `design.md`
  mentions `--force-with-lease` only to describe accepted route 1.
- **First-round box 1 (Accepted)** matches `design.md`'s "The owner accepted
  the regression" section: three routes, `required_approving_review_count: 0`
  as the reason protection does not catch them, and the Risks entry is updated
  to match. **Box 3 (Fixed)**: `gh pr view 196` carries no `/home/` path, and
  no token or key appears in the body.
- **Machine-local paths and secrets.** `git grep -F "/home/fryorcraken"` hits
  only `findings/security.md` and `findings/spec-test.md`, which the `closer`
  deletes before the merge, and three archived changes that predate this
  piece. The README uses `~/src/agent-spec-flow` as a stated example. The
  range adds no token, key or credential.
