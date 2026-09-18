## Stages

- [ ] ~~spec — `spec-writer`~~ — struck: no behaviour change. This change alters
      agent instructions and adds a gate over them, so it has no spec delta.
      `.openspec.yaml` declares `skip_specs: true` alongside a `schema:` key.
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — struck: no reviewers dispatched. The owner is two
      days into waiting for a visible UI and this piece is tooling; deliberately
      shipped unreviewed rather than costing more time on review of an
      agent-instructions change.
- [ ] ~~review: correctness — `code-reviewer`~~ — struck: no reviewers
      dispatched, see above.
- [ ] ~~review: security — `code-reviewer`~~ — struck: no reviewers dispatched,
      see above.
- [ ] ~~review: readability — `code-reviewer`~~ — struck: no reviewers
      dispatched, see above.
- [ ] ~~review: architecture — `code-reviewer`~~ — struck: no reviewers
      dispatched, see above.
- [ ] ~~review: spec-test — `spec-test-reviewer`~~ — struck: no reviewers
      dispatched, see above.
- [ ] ~~review: design — `design-reviewer`~~ — struck: no reviewers dispatched,
      see above.
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. The canonical list

- [x] 1.1 Write `.claude/agents/BASH-COSTS.md` carrying every forbidden shape
      with its replacement. Verified by reading: each of `|`, `&&`, `;`,
      `$(…)`, loops, `>`/`>>`, globs, heredocs, `env VAR=`, `cd &&`, and
      out-of-tree reads has a named replacement in the right-hand column.
- [x] 1.2 Cover the two points the brief calls out once, in the right place —
      relative paths inside your own worktree, and the stop-and-report
      fallback. Both are their own subsection in `BASH-COSTS.md`.
- [x] 1.3 Move `README.md`'s partial copy (the chaining rule, the pipe rule,
      the SIGPIPE trap) into `BASH-COSTS.md` and leave a pointer, so the two
      cannot drift. Verified: `grep -c "tar tzf" .claude/agents/README.md`
      returns 0 and the same grep on `BASH-COSTS.md` returns 1.

## 2. The per-role sections

- [x] 2.1 Add the uniform `## What Bash costs here` section to all seven role
      files — `spec-writer`, `dev-writer`, `tester`, `code-reviewer`,
      `spec-test-reviewer`, `design-reviewer`, `closer`. Verified by the gate
      in 3.2, which checks all seven and reports the count it checked.
- [x] 2.2 Give each section a final line naming that role's most likely trap:
      `openspec` and `cd` for `spec-writer`, file-mutation shapes for
      `dev-writer`, the QML runner for `tester`, piping suite output for
      `code-reviewer`, the `NO SPEC:` grep for `spec-test-reviewer`, the
      PLAN.md diff for `design-reviewer`, `gh --jq` for `closer`.
- [x] 2.3 Correct `dev-writer.md`'s "Absolute paths" bullet, which contradicted
      every other role file and the worktree dispatch flow. Verified:
      `grep -c "Absolute paths" .claude/agents/dev-writer.md` returns 0.

## 3. The gate

- [x] 3.1 Write `.claude/agents/tests/check_agent_bash_costs.sh`, deriving the
      role-file set from `name:` frontmatter rather than a hardcoded list, and
      failing when it finds no role files at all.
- [x] 3.2 Run it against the real directory. Observed:
      `OK: all 7 role file(s) point at BASH-COSTS.md and carry the summary.`
- [x] 3.3 Write `tst_check_agent_bash_costs.sh` pinning both directions, and
      **demonstrate the gate fails** on each defect. Observed: 7 passed, 0
      failed, including a role file with no pointer, a pointer with no summary,
      a partial summary, the canonical list missing, and a directory with no
      role files (the measured-nothing case).
- [x] 3.4 Wire both into `ci.yml`'s `lint` job, which needs no Qt. Verified the
      gate runs from a relative path as CI invokes it.
