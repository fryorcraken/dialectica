# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] re-review: every commit after the review round — runner
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

This piece is spec text only. The spec-writer did groups 1 to 3 before this
checklist was written, and each item names the commit that did it. Group 4 is
the dev-writer's. `design.md` says why the work took these shapes.

## 1. Archive the shipped changes, oldest merge first

- [x] 1.1 `sqlite-projection` (#20): op-log delta rebased onto the live spec
      (`1f5df8f`), then archived (`0519416`). Verify: the promoted `op-log`
      still holds the blank-title encoding refusal.
- [x] 1.2 `first-run-identity` (#128): two falsified sentences corrected
      (`b1d8966`), then archived (`b545d3b`).
- [x] 1.3 `ui-thread-view` (#122): citations and one out-of-scope scenario
      corrected (`584359b`), then archived as `thread-view` (`d70f662`).
- [x] 1.4 `ui-remaining-screens` (#124): archived with no correction
      (`2fa2b11`).
- [x] 1.5 `moderation-screen` (#127): four corrections (`8e508d8`), then
      archived as `moderation-view` (`eb8be6b`).

## 2. Live-spec corrections, carried as this change's deltas

- [x] 2.1 `moderation-resolution` and `stoa-genesis`: deleted-`PLAN.md`
      citations replaced (`49b98ca`). Verify:
      `git grep -n "Requirement: A post is never edited in place"` and
      `git grep -n "Requirement: Joining shows what is being joined"` each hit
      one live spec.
- [x] 2.2 `generated-names`: the #81 contradiction and the #80 wording
      (`92b3162`). Verify: `openspec/specs/generated-names/spec.md` holds
      "Requirement: Core exposes the derivation to a caller", which the amended
      text cites.
- [x] 2.3 `feed-view`: the bare `design.md` citation and the garbled clause
      (`1bcf2bf`).

## 3. `view-navigation` becomes the single home for screen entry and exit

- [x] 3.1 `view-navigation`'s Purpose names `thread-view` and `moderation-view`
      (in place, `06bfd48`).
- [x] 3.2 `thread-view`'s route requirement folded into `view-navigation`'s
      `MODIFIED` thread requirement. `moderation-view`'s route requirement is
      moved to `view-navigation` as `ADDED`. Both screens' Purposes are amended
      in place (`ff55cf7`). Verify: each scenario of the two `REMOVED`
      requirements is present in the `view-navigation` delta, or is named by
      the `REMOVED` block's Migration as already covered.

## 4. Design and verification

- [x] 4.1 Proposal SHAs re-pointed at this branch's commits after the
      re-signing rebase (`e9a2fa2`). Verify: every SHA `proposal.md` cites
      resolves with `git merge-base --is-ancestor <sha> HEAD`.
- [x] 4.2 `design.md` written, with its Decisions: shipped-means-merged,
      `relevance-votes`, deltas over `skip_specs`, `view-navigation` as the
      single home, and #186.
- [x] 4.3 Every `MODIFIED` and `REMOVED` heading in this change's deltas matches
      a live `### Requirement:` line character for character, and the one
      `ADDED` heading matches none. Verify: `git grep -n -E` over
      `openspec/specs` for each heading.
- [x] 4.4 `openspec validate spec-tidy --strict` reports the change valid.
- [x] 4.5 `openspec validate --specs --strict` reports every live spec passing
      and none failing.
- [x] 4.6 `sh dialectica-ui/tests/run-qml-tests.sh` passes every spec file. This
      piece touches no QML, so a green suite shows only that nothing in code
      moved. It cannot see the spec text, and no gate checks a code comment's
      citation against the live specs.
- [x] 4.7 Four code comments cited a route requirement this piece moved, or
      called an archived change "unarchived" (`design.md`, Risks). They now
      point at `view-navigation`, `thread-view` or `moderation-view`, with
      comment-only edits taken in on the runner's instruction. Verify:
      `git grep -n "unarchived" -- dialectica-ui` finds nothing, the QML suite
      passes, and `node dialectica-ui/tests/validate-ui-specs.mjs` accepts both
      yaml files.
