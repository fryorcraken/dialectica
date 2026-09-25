# Tasks — workflow rules (#171, #170, #169, #133)

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change edits agent
      instructions under `.claude/agents/` only and alters no behaviour of
      dialectica, so no requirement in `openspec/specs/` changes. Declared as
      `skip_specs: true` alongside `schema:` in `.openspec.yaml`. The
      `spec-writer`'s work here is `proposal.md` and this block.
- [ ] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — prose-only change to agent instructions. No CI
      job, script or test reads `.claude/agents/`, so there is no executable
      behaviour to assert. A test pinning the new wording would fail when the
      wording changed, not when the rule was wrong. The check that can see
      this change is the six reviewers reading the prose.
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation
