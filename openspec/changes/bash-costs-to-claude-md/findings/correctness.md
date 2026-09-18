# Correctness review — `bash-costs-to-claude-md`

Reviewed only the **correctness** dimension, per dispatch. No findings — every
claim in the task brief was checked against the tree and held. Recorded below
as verification notes (prose, not checkboxes — nothing here needs a fixer).

## 1. Is the revert exact?

`git diff 51ab7f8^ HEAD -- .claude/agents` returns exactly one hunk, in
`dev-writer.md`, replacing the old "Absolute paths, and `Read`/`Edit`/`Write`
over shell file manipulation" bullet with the corrected relative-paths-in-worktree
wording. Nothing else under `.claude/agents` differs from its pre-`51ab7f8`
state. Confirmed independently (not trusting the proposal's claim) by diffing
`git show 51ab7f8^:.claude/agents/README.md` against the working tree's copy —
byte-identical (`diff` exit 0).

`.claude/agents/tests/` (added by #119: `check_agent_bash_costs.sh`,
`tst_check_agent_bash_costs.sh`) no longer exists. `.claude/agents/BASH-COSTS.md`
no longer exists.

## 2. Was anything wrongly deleted?

No. `.claude/agents/README.md`'s "Every agent pays CLAUDE.md's Bash costs"
section is present and complete — verified by reading the live file directly
(not just grepping): it carries the never-chain rule (`cd somewhere && cargo
test` prompts even though `cargo test` is allow-listed), the don't-pipe-a-long-output
paragraph (`| tail -30` turns an approved call into a prompt), and the
`tar tzf … | grep -q` SIGPIPE trap (exit 141, `[ "$(… | grep -c …)" -gt 0 ]` as
the fix) verbatim, matching the pre-`51ab7f8` copy exactly.

## 3. Is `ci.yml` correct?

`git diff 2549468 HEAD -- .github/workflows/ci.yml` shows exactly one hunk
removed: the two Bash-costs `lint` steps (`the agent Bash-costs gate's own
tests` and `every agent role file carries the Bash-costs rules`) and their
explanatory comment block. Nothing else changed.

- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
  → parses clean.
- The probe-twins gate (`check_probe_twins.sh`, `tst_check_probe_twins.sh`) and
  the QML reachability gate (`check_qml_reachable.py`,
  `tst_check_qml_reachable.py`) both survive, each still invoked by name — plus
  `check_qml_names.py`, `check_qml_members.sh` and `check_bindings`
  (`tst_check_bindings.sh`), none of which this piece should have touched and
  none of which did.

## 4. Is the CLAUDE.md correction factually right?

This was the load-bearing claim, so I checked it against the actual pre-piece
state rather than trusting the proposal's prose. `git show 2549468:CLAUDE.md`
confirms the table at that commit really did read:

> `an **absolute** path as an argument` (free) | `a **relative** path, or any
> path after `cd`` (costs a click)

That is genuinely wrong for a dispatched agent under `isolation: "worktree"` —
the same document's own `EnterWorktree` subsection said such an agent "arrives
inside its own worktree with a working cwd, plain relative paths and no
approval clicks," and `.claude/agents/README.md` (unmodified by this piece)
independently confirms the same thing under "How an agent gets the right
tree." The old table row contradicted the rest of its own file.

The corrected table (`a path the checker can resolve **before** the command
runs` vs. `any path after `cd``) plus the new paragraph ("Inside your own
worktree, prefer plain relative paths… Absolute paths remain the rule for
anything reaching *outside* the tree you are in") resolves the contradiction
and matches what I was actually told to do and did in this review (dispatched
into my own worktree, used relative paths throughout, e.g.
`.claude/agents/README.md`, `CLAUDE.md`). The `dev-writer.md` one-bullet fix
(item 1 above) matches this corrected wording, not the old contradictory one —
so the two changes are consistent with each other, which is what you'd expect
if the same defect motivated both.

I could not find a case where the new table or its prose is wrong in the
other direction (e.g. overstating when absolute paths are needed) — it still
correctly states absolute paths are required for reaching outside the current
tree, matching the "outside working directories" costs and the
`readlink`/`ls` `/nix/store` warning elsewhere in the file, both untouched.

## 5. Nothing else under `.claude/` changed

`git diff 2549468 HEAD --stat -- .claude/` shows only the 11 files touched by
`51ab7f8` being reverted (9 role/readme files edited or restored, 2 test files
deleted), 25 insertions / 457 deletions — no other path under `.claude/`
appears. `grep -rln "BASH-COSTS|check_agent_bash_costs"` over `.claude/agents/`
returns nothing.

## Archived record

`git diff 51ab7f8 HEAD -- openspec/changes/archive/2026-09-18-agent-bash-costs/`
is empty — all 4 files (`.openspec.yaml`, `design.md`, `proposal.md`,
`tasks.md`) are byte-identical to what `51ab7f8` originally added.

## Other checks

- `openspec validate bash-costs-to-claude-md --strict` → valid (skip_specs
  honoured, zero deltas accepted as expected for a prose/config-only change).
- Remaining `BASH-COSTS` references in the tree are all inside
  `openspec/changes/archive/2026-09-18-agent-bash-costs/` (the restored
  historical record, correctly preserved) and
  `openspec/changes/bash-costs-to-claude-md/{proposal,tasks}.md` (this piece's
  own description of what it reverted) — none are live pointers a reader would
  follow into a now-deleted file.

**No defects found on the correctness dimension.** The revert is exact, the
restored content is complete, `ci.yml` parses and its surviving gates are
intact, the CLAUDE.md path-cost correction is factually right and internally
consistent, and no stray `.claude/` changes exist.
