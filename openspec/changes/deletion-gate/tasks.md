# Tasks

## Stages

- [x] ~~spec — `spec-writer`~~ — **no spec delta, struck rather than deleted.**
      A CI gate asserts nothing about module behaviour, so there is no
      capability to contract and no test in any suite that could cover such a
      requirement; the change declares `skip_specs: true` in its
      `.openspec.yaml`. The row stays visible because a missing row reads as an
      oversight and the next reader cannot tell which. `spec-writer` did run,
      and owns `proposal.md`, this block and that marker file — see
      proposal.md's Capabilities section for why a capability would be wrong
      rather than merely absent.
- [ ] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] CI green, PR merged — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`

## Implementation
