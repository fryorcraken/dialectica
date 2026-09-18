# Design review — `bash-costs-to-claude-md`

**No `design.md` exists for this change**, and that is correct rather than a
gap: the change reverts seven agent-instruction files and CI steps to their
pre-#119 content and edits `CLAUDE.md` prose. It introduces no new data format,
no security boundary, no new dependency, and no migration or performance
complexity — the conditions `dev-writer.md` lists as the triggers for writing
one. `proposal.md` carries the reasoning instead, which is the right place for
it given no `design.md` exists to hold a Decisions section. This review checks
the change's decisions as recorded in `proposal.md` and `tasks.md` against the
code, against the owner's rulings, and against `CLAUDE.md`.

## Verified against the owner's rulings — all four landed, cleanly

- **Ruling 1 (unauthorised)** and **ruling 2 (revert, fold into CLAUDE.md)**:
  `git diff 2ee719a HEAD -- .claude/agents/README.md .claude/agents/closer.md
  .claude/agents/code-reviewer.md .claude/agents/design-reviewer.md
  .claude/agents/spec-test-reviewer.md .claude/agents/spec-writer.md
  .claude/agents/tester.md .claude/agents/RUNNER.md` (`2ee719a` = the last
  commit before #119) is empty for all eight files — byte-identical to
  pre-#119. `BASH-COSTS.md` and `.claude/agents/tests/` are absent from the
  tree. `.github/workflows/ci.yml` diffed the same way shows only the
  (unrelated, authorised) probe-twins and QML-reachability gates as additions —
  no Bash-costs steps survive.
- **Ruling 3 (`.claude/` is the owner's, add to CLAUDE.md)**: present at
  `CLAUDE.md:46-63`, naming the trigger instance ("a runner noticed dispatched
  agents were costing the owner approval prompts, and rewrote all seven role
  files") without calling the diagnosis wrong — it says explicitly "the
  diagnosis was correct and the action was still not the runner's to take."
  This is the framing the review brief asked to check for, and it holds: a
  future reader of this section will not conclude #119 was reverted for being
  mistaken.
- **Ruling 4a (restore the archive)**: `openspec/changes/archive/
  2026-09-18-agent-bash-costs/` is present, restored in a separate commit
  (`1ec25bb`) after the revert had deleted it, with a commit message stating
  why plainly ("a reverted change is still something that happened").
- **Ruling 4b (keep the dev-writer.md correction)**: verified —
  `.claude/agents/dev-writer.md` is the only one of the eight role/README files
  that differs from pre-#119. `git diff 2ee719a HEAD -- .claude/agents/dev-writer.md`
  shows the bullet changed from "Absolute paths, and `Read`/`Edit`/`Write` over
  shell file manipulation" to the corrected form naming relative paths inside
  the worktree. `proposal.md:45-49` records this as a deliberate exception with
  its reason (contradicted the worktree flow, predates #119), matching the
  brief's expectation — it does not read as an incomplete revert.

## The `.openspec.yaml` convention

Confirmed correct. Per-change `.openspec.yaml` files do exist elsewhere in this
repo (`archive/2026-09-18-ui-render-probe/`, `archive/2026-09-18-agent-bash-costs/`,
others), and the archived record this change reverts carries `schema:
spec-driven` alongside `skip_specs: true` — exactly the shape this change's own
`.openspec.yaml` follows, with a comment explaining why `schema` is required for
`skip_specs` to be honoured. This piece did not repeat the runner's claimed
error of assuming per-change `.openspec.yaml` files don't exist.

## Findings

- [x] **`dev-writer`** — `openspec/changes/bash-costs-to-claude-md/proposal.md:45-49`
      (and `CLAUDE.md`, `tasks.md:104-108`) — the causal link between two
      recorded facts is stated only in the commit message, not in any file a
      future reader will open.

      The commit message for `4a9d70a` says of the dev-writer.md fix: "is
      **plausibly where the typo'd absolute path in one measured incident came
      from**." That is a real causal claim — that `CLAUDE.md`'s own
      absolute/relative table was backwards (confirmed: `git show 51ab7f8:CLAUDE.md`
      shows "an **absolute** path as an argument" was listed as free and "a
      **relative** path" as costly, exactly backwards for the worktree-dispatch
      flow that has stood agents inside the correct tree since `e95303b`), and
      that `dev-writer.md`'s matching "Absolute paths" bullet (confirmed
      present verbatim since before #119, `git log --follow`) is plausibly
      where a dispatched dev-writer picked up the habit that produced the
      measured "typo'd username" incident CLAUDE.md now documents at line 108.

      None of `CLAUDE.md`, `proposal.md`, or `tasks.md` state this connection.
      `CLAUDE.md:107-108` records the typo incident as a fact with no cause
      given. `proposal.md:45-49` and `tasks.md:104-108` record the dev-writer.md
      fix as a fact with no cause given. A reader of either file sees two
      correct, isolated facts and has no way to learn that the first
      plausibly explains the second — exactly the kind of reasoning
      CLAUDE.md's own "Keeping this file true" section asks to be written down
      ("why a decision went the way it did… That reasoning does not rot"),
      and it is this piece's most interesting causal discovery, currently
      recoverable only by reading `git log` for commit `4a9d70a` specifically.

      **Scenario:** a future contributor reads `CLAUDE.md:107-108`'s
      "mistyped username… has already cost a click" and, separately,
      `dev-writer.md`'s corrected bullet, with no path between them; they
      cannot judge whether the two are related or coincidental, and cannot
      confirm the CLAUDE.md table fix was sufficient to prevent a recurrence,
      because the file never says the table was the origin.

      **Fixed in `CLAUDE.md`, not `proposal.md`** — the finding left the
      placement to be judged, and the two archive differently. `proposal.md` goes
      to `changes/archive/`, which is where someone investigating a past decision
      looks; this is not a past decision, it is a live reason the corrected row
      is worded as it is, and a reader meets it at the incident rather than by
      going looking. So it sits directly under the typo-incident sentence in the
      relative-paths bullet: the backwards table, `dev-writer.md`'s matching
      bullet, the blocked read, and the note that the corrected row and bullet
      are what defend against a recurrence.

      **Labelled plausible, not proven**, which is the part worth keeping: nobody
      asked the agent why it typed that path, so the chain is the only account
      that fits rather than a measured cause. Writing it as established would
      have manufactured support for a claim that is inferred — and a reader who
      later finds a different cause needs to see that this one was an inference.

      `tasks.md` gains a matching item under "Correction carried across the
      revert", so the change's own record says the chain was written down
      somewhere durable rather than left in `4a9d70a`'s message.

This is the change's one real gap. Everything else — the revert's completeness,
the recorded exception, the archive restoration, the owner-authorisation
framing, and the `.openspec.yaml` convention — is in good shape and needs no
further action.
