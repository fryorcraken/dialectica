# Architecture review — `e2e-successful-join`

Scope: architecture dimension only (this is one of four `code-reviewer`
instances on this piece; correctness, security and readability are each
covered by a separate instance). Reviewed `git diff
2bda577f7cf70e84f5fb59265f81c2f621224011...HEAD` against `proposal.md`,
`design.md` and `tasks.md`, with emphasis on D1 (committed vs generated
reference), D2 (the core test's textual coupling to a UI spec file), D3
(separate spec file vs steps in `join.yaml`), and the suite as a whole.

## D1 — committed, not generated: sound

The decision is well-supported and the two independent reasons it gives are
each individually sufficient: (a) sitometres 0.1.2 cannot take a value from
outside a spec file, so a generated reference could only reach the run via an
in-job rewrite of the spec, which would make the three static readers of that
file (the `ui-specs` validator, the matrix count check, the adjudicator's step
count) check something other than what ran; (b) a reference produced by the
same core that verifies it moves with that core, so an encoding change that
breaks every reference already shared in the wild would leave the spec green —
exactly the drift this suite exists to catch early. Reason (b) holds regardless
of (a), which matters below.

**One documentation-precision observation, not a defect.** design.md's D1
also gives a *cost* argument against generating per run — building a Rust
toolchain "in a job that today builds nothing with cargo, which adds cost to
every run of it." The radicle module's `ui-tests.yml` (the owner-named
reference implementation, `write`/`local` in its matrix) shows that this cost
is confinable to the one job that needs it via `if: matrix.spec == '<name>'`
gated steps, so the "every run" framing overstates what per-spec gating would
actually cost. It does not change the decision — reason (b), the
compatibility-surface argument, is unaffected by whether the cost is job-wide
or gated, and git branches (radicle's seed data) carry no encoding-compatibility
concern analogous to a genesis record written under an older version. Flagging
this for whoever reads design.md next, since the review brief asked for the
radicle comparison; not something that needs a code change.

## D2 — the core test's textual coupling to `dialectica-ui/tests/ui/seeded-join.yaml`

This is the one place this piece points a `dialectica-core` integration test at
a path outside its own crate, in the sibling UI module — a direction the
core/UI split otherwise never runs (core has no UI dependency; here a *test*
does). It is deliberate and reasoned in design.md (spec is the source of
truth because it's what a user actually pastes; a constant in the test would
be a second copy needing a third thing to keep the two equal), and it is
scoped to `tests/`, not `src/` — no production surface depends on it, and
`Cargo.toml` gained no new dependency to support it.

**Verified the "does it survive a move/rename" question by breaking it.**
Changed `SPEC` to a nonexistent filename and ran
`cargo test --manifest-path dialectica/rust-lib/dialectica-core/Cargo.toml
--test seeded_reference`: all three tests fail cleanly with

```
cannot read the seeded join spec at .../seeded-join-MOVED.yaml: No such file or directory (os error 2)
If it moved, update SPEC here; the reference it should paste is
  {"stoa":"a4b3e43d...","genesis":"018146..."}
```

— i.e. exactly the failure mode design.md D2 and the file's own doc comment
claim: a named path and a ready-to-paste replacement, not a compile error
pointing nowhere. Reverted the edit; `cargo test` is green again (3 passed).
This is the right shape for a coupling that has to exist: it fails loudly and
specifically rather than silently or cryptically.

The textual (substring) extraction, rather than a YAML parse, is also
justified (no YAML parser in `dialectica-core`, and adding a dev-dependency
for one test is exactly the kind of dependency-wall erosion `Cargo.toml`
argues against) and fails closed: `assert_eq!(starts.len(), 1, ...)` rejects
zero or multiple occurrences rather than guessing. Confirmed the one
occurrence in `seeded-join.yaml` is unique (`grep -n '{"stoa"'` returns one
line) and that the header comments in that file do not reproduce the literal.

No dependency was added (`Cargo.toml` diff is empty), and the constant offset
`"../../../"` in `SPEC` is the kind of thing `cargo-mutants` cannot see
(confirmed: `cargo mutants --file tests/seeded_reference.rs --list` returns
nothing at all — cargo-mutants does not mutate integration-test files by
default, so this file has no automated mutation coverage either way) — which
is exactly why it was worth checking by hand above rather than trusting a
green mutants run that never ran.

## D3 — a separate file, `seeded-join.yaml`

The reasoning holds up mechanically: sitometres stops at the first failed step
and marks the rest `inconclusive` (a documented, checkable property), so
appending to `join.yaml` would make a refusal-path regression swallow the
successful-join assertions, and vice versa a successful-join regression would
never get isolated from the refusal spec's report. As its own file, a red job
name (`seeded-join`) says which path broke without reading step numbers. The
`join.yaml` header was updated to point at the new file instead of describing
a gap that no longer exists (checked in the diff — consistent, not left
stale). The one QML change (`joinCancelButton`) is a single `objectName`
addition with no behavioural change, reused (not duplicated) by both the new
spec and two new `tst_navigation.qml` tests.

The matrix and its self-check are both dynamic — `ui-tests.yml`'s "every spec
in the tree is in the matrix" step compares a file count against
`strategy.job-total` rather than a hand-maintained number, and
`validate-ui-specs.mjs` globs the directory rather than listing filenames —
so adding the sixth spec required no parallel hand-edited count anywhere, and
none of the "hand-maintained sweep list goes stale silently" shape this
codebase has been bitten by before. Checked both.

## The suite as a whole

Six specs, one per matrix job, running in parallel with a shared store cache —
structurally unchanged from the five-spec baseline plus one row. Nothing in
this piece entangles the seeded-reference machinery with anything else in a
way that would make it hard to remove later: `seeded_reference.rs` and
`seeded-join.yaml` are the only two files that know about the fixture, and
design.md D5 states in advance, and checkably, the exact condition under which
this approach stops being appropriate ("once #176 lands... a second profile
under `lgs basecamp launch` becomes the seeder" — #102 item 7). That is the
right way to leave a deliberately temporary design decision: the boundary
condition is written down rather than left to be rediscovered.

## Verdict

- [x] **none** — no architecture defect found. D1–D3 are each independently
      reasoned and the reasoning was checked against the code rather than
      taken on faith (D2's failure mode was reproduced by breaking `SPEC` and
      reverting; D1's cost aside is a documentation nuance for design.md, not
      a code issue, and doesn't change the decision). The suite's dynamic
      count checks (matrix `job-total`, `validate-ui-specs.mjs`'s glob, the
      Rust "declared tests" gate) mean the sixth spec needed no parallel
      hand-maintained update anywhere, and none was missed.
