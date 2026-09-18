# Tasks — record four owner scope rulings on the MVP

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change records staging
      decisions in `docs/PLAN.md` and alters no behaviour and no requirement.
      Every capability the four rulings touch stays exactly as it is. Declared as
      `skip_specs: true` alongside `schema:` in `.openspec.yaml`, which carries
      the argument — including why ruling 4 is not contract material.
- [ ] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`
