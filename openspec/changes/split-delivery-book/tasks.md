# Tasks — split the channel book and the inbound queue out of `delivery.rs` (#205)

## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec delta: a no-behaviour code move, `skip_specs: true` in `.openspec.yaml` says why; `proposal.md` is written
- [ ] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — owner decision 2026-10-11: no-behaviour move, tests have zero diff
- [ ] review: correctness, security, readability, architecture — `code-reviewer`
- [ ] ~~review: spec-test — `spec-test-reviewer`~~ — owner decision 2026-10-11: no-behaviour move, tests have zero diff
- [ ] ~~review: design — `design-reviewer`~~ — owner decision 2026-10-11: no-behaviour move, tests have zero diff
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`
