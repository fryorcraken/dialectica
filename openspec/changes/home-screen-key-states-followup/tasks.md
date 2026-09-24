# Tasks — home-screen-key-states follow-up

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** Both open findings are
      about tests (a fixture in the wrong key state, and a test control tied to
      its subject only by comments), and neither changes a requirement.
      `.openspec.yaml` declares `skip_specs: true` alongside `schema:`, and
      `proposal.md` gives the reason for each finding.
- [ ] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

The six review rows are ticked because those reviews **have already run**.
This piece exists because of them: the owner asked for a final six-reviewer
round over #155 as merged (`0cbe1d4`), and `findings/` holds its output, one
file per row. The flow answers a finding in place, where the addressee ticks
its box and the reviewer does not run again, so no second round is due for
these rows. The `findings all ticked` row is what makes sure both open
findings actually get answered before archive.

## Implementation
