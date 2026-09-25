## Stages

Every review row covers this change's own diff **and** `git diff 2eada33 c1f1a8f`, the four commits merged with #165 after its review round. `proposal.md`, "Review scope", names them.

- [x] spec — `spec-writer`
- [ ] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — does not apply: this change alters no behaviour, so there is nothing new to test. The four #165 commits under review carry their own tests, and `spec-test-reviewer` reads those.
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`
