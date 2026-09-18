# Architecture review — `bash-costs-to-claude-md`

Scope: architecture dimension only, per dispatch. Reviewed the two piece
commits (`4a9d70a`, `1ec25bb`) against `main`, `.claude/agents/README.md`, and
`CLAUDE.md`.

## Central claim verified

The asymmetry the change rests on — "`CLAUDE.md` is injected into every Claude
session and agent at startup... a file under `.claude/agents/` is read only by
the agent it names" (CLAUDE.md:59-61) — is real. I am direct evidence of it:
this review was dispatched as a `code-reviewer` agent via `isolation:
"worktree"`, and `CLAUDE.md`'s full content arrived in my system context as a
"project instructions" block before I read anything myself. The mechanism the
proposal relies on is not a hopeful claim; it is observable from inside any
dispatched agent's own context.

## Diagnosis vs. action, and whether the fold actually answers the diagnosis

#119's measurement ("five of seven role files carried no local copy... four
dispatched agents cost the owner approval prompts") is not disputed anywhere
in this change — only the unauthorised *action* taken in response to it. The
owner's own ruling is quoted verbatim in `proposal.md:12-14`, which is the
correct authorisation source for reverting `.claude/`.

Given the verified injection mechanism, folding the rules into `CLAUDE.md`
does answer the original problem: every dispatched agent receives `CLAUDE.md`
whether or not it reads its own role file closely, so the "five of seven role
files had no local copy" gap is closed at the point that actually reaches
every agent, not duplicated nine times.

## Enforcement removed

The gate (`check_agent_bash_costs.sh`) is deleted along with its tests and its
two `ci.yml` `lint` steps. This is the right call given the gate itself was
part of the unauthorised artefact, and I could find no other mechanical check
on `CLAUDE.md`'s Bash-costs content post-revert — it is now advisory only, same
as it was before #119. That is a real trade-off worth naming plainly rather
than treating as free: there is nothing today that fails CI if `CLAUDE.md`'s
Bash-costs section silently drifts out of date or an agent role file
reintroduces a contradiction. Given this rule's mechanism (an LLM reading
prose, not code executing a contract), a gate over it was always weaker
evidence than a gate over behaviour — so the loss is smaller than a gate over
logic would be, but it is not zero.

## `ci.yml` coherence, verified directly

`git show 4a9d70a -- .github/workflows/ci.yml` shows a clean removal of
exactly the two Bash-costs steps and their explanatory comment; the
probe-twins step above and the QML-reachability gate below are untouched.
`python3 -c "import yaml; yaml.safe_load(...)"` against the current file
parses without error. No `BASH-COSTS` or `bash_costs` string remains anywhere
in the tree outside the (expected) archived record and this change's own
proposal/tasks discussing it as history — confirmed with
`grep -rln "BASH-COSTS|check_agent_bash_costs"` across the worktree.

## The revert is not a blind `git revert`

Worth recording because it is easy to assume `git revert` was applied
mechanically and left it there: `dev-writer.md`'s "Absolute paths" bullet was
**not** restored to its pre-#119, self-contradictory wording (which told
agents to prefer absolute paths, in conflict with the worktree-dispatch flow
and every other role file). The commit correctly keeps #119's fix to that
bullet and only removes the duplicated "What Bash costs here" block. Verified
via `git show 4a9d70a -- .claude/agents/dev-writer.md`.

## Findings

- [ ] **`dev-writer`** — `.claude/agents/README.md:374` — a second, narrower
      copy of the `.claude/settings.json`-is-the-owner's rule survives the
      revert, uncorrected, right beside the new generalised rule this change
      added to `CLAUDE.md`.
      **Scenario:** `CLAUDE.md:46-63` states the generalised rule — "Do not
      add, edit, delete or restructure anything under `.claude/`... unless the
      owner asked" — and explicitly frames it as replacing a narrower rule
      about one file: *"a rule about one file is what invited reading the rest
      as fair game... generalised rather than left beside the new one"*
      (CLAUDE.md:53-57, echoed in `tasks.md:94-97`: *"not duplicated"*).
      `.claude/agents/README.md:374-375` still independently says *"It is the
      **user's** file. Do not edit it on your own initiative; machine-local
      settings belong in `settings.local.json`"* — a second phrasing of the
      same rule, for the same file, that this change's own stated design
      principle says not to leave beside the new one. It links forward to the
      new section at line 138 (`"the owner's — see ... below before editing
      it"`) but the older, independent assertion at line 374 was left as-is
      rather than reduced to a pointer. Low severity — the two statements do
      not currently conflict — but it is exactly the shape ("a narrow rule
      plus a general rule covering the same ground") that this change's own
      commit message names as the cause of #119's overreach, now reproduced
      once, inside the very code that reverts #119.
      **Measured:** `grep -n "the owner's|the user's|owner asked|do not edit it
      on your own" .claude/agents/README.md` returns only line 374; `CLAUDE.md`
      carries the generalised rule at lines 46-63 with no cross-reference back
      to this line.

- [ ] **`dev-writer`** — `openspec/changes/archive/2026-09-18-agent-bash-costs/`
      — the restored archive folder carries no marker that the work it
      describes was later reverted (by this piece, PR #131).
      **Scenario:** a future agent or reviewer greps for "Bash costs" or
      "BASH-COSTS.md" while investigating permission-prompt costs, lands in
      this archived `proposal.md`/`design.md`/`tasks.md`, and reads a fully
      checked-off, coherent design for `.claude/agents/BASH-COSTS.md`, a
      per-role-file gate, and CI wiring — all `[x]`'d as done, with none of it
      present in the tree and nothing in the archived files themselves saying
      why. The only place that explains the restoration is
      `openspec/changes/bash-costs-to-claude-md/tasks.md:33-35`, in a
      *different* change folder the reader has no reason to find by grepping
      the archive. `docs/OPENSPEC-ARCHIVE.md` documents several archive traps
      but not this one — reverted archived changes are undiscussed there, so
      this is not a violation of a stated convention, just an unaddressed
      gap this change newly creates precedent for. Low severity, since
      `git log`/`gh pr view` can always recover the story, consistent with
      CLAUDE.md's "do not write down anything a command can answer" — but a
      one-line note at the top of the archived proposal.md ("reverted by
      PR #131") would have cost little and saved the grep-then-guess path.
      **Measured:** `grep -rn "revert" docs/OPENSPEC-ARCHIVE.md` returns one
      unrelated hit (about specs reverting together with their change); no
      hits for "revert" anywhere under
      `openspec/changes/archive/2026-09-18-agent-bash-costs/`.

## Clean

- The central injection-asymmetry claim: verified directly, not just
  asserted.
- `ci.yml`: surgical removal, parses, neighbouring gates untouched.
- No stray `BASH-COSTS`/`bash_costs` references outside the archive and this
  change's own docs.
- The `.claude/` ownership rule's catch-all (`CLAUDE.md:48-51`): stated as a
  blanket over the directory with a non-exhaustive example list ("anything
  added later"), which is the correct structural answer to the
  hand-maintained-sweep-list trap this repo has been bitten by before — it
  does not repeat #119's narrow-rule shape at the point it was introduced.
- `dev-writer.md`'s "Absolute paths" bullet: correctly kept in its #119-fixed
  form rather than reverted to the contradictory pre-#119 text.
- `tasks.md` stage block: well-formed, correctly strikes the stages that do
  not apply (spec, tests, several review dimensions) with stated reasons.
- The archived folder's own checked-off tasks are internally consistent as a
  historical record (they describe what was true when written); the gap is
  only that nothing signals the record is now stale, per the finding above.
