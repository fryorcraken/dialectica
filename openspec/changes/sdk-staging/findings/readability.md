# Readability review — #168 sdk-staging

Dimension covered: **readability only** (correctness, security, and
architecture are other reviewers' rows).

Scope read: `README.md`, `.github/workflows/ci.yml` (the "Stage the
logos-rust-sdk source" step and its comment block, plus the re-grounded
`install-nix-action` comment), `.gitignore`, `CLAUDE.md`, and the whole
`openspec/changes/sdk-staging/` folder (`proposal.md`, `design.md`,
`tasks.md`, `.openspec.yaml`), against `git diff 8368b2f...HEAD`.

## Finding

- [x] **`dev-writer`** — `openspec/changes/sdk-staging/proposal.md:73` and
      `openspec/changes/sdk-staging/design.md:84` — dangling reference to an
      undefined "shallow-`//` trap"
      **Scenario:** a reader of `design.md`'s D1 Alternatives, or of
      `proposal.md`'s "Out of scope", hits: "the shallow-`//` trap discussed
      with it" (design.md) / "the shallow-`//` trap discussed earlier"
      (proposal.md). Neither document — nor anywhere else in the tree —
      ever says what that trap is. `git grep -n "shallow"` over
      `openspec/`, `CLAUDE.md` and `docs/` finds exactly these two lines and
      nothing else; `git log --all -p -- openspec/changes/sdk-staging/proposal.md`
      shows the phrase entered the file already dangling, in the first commit
      that added it — there is no earlier passage in this file's own history
      that "discussed" it. The term traces back to the issue owner's comment
      ("the re-export and the shallow-`//` trap **in the proposal** are
      unnecessary"), which refers to a passage in an *earlier draft* of the
      proposal that named and explained the trap; that explanation was
      dropped when the draft was cut down (correctly — the owner said the
      whole thing is unnecessary), but the two references to it were kept.
      A reader who was not on the issue thread has no way to learn what was
      avoided, only that something was. This is the kind of thing
      `design.md`'s Alternatives section exists to make self-contained.
      **Fix shape:** either give the trap one clause of its own explanation
      where it is first mentioned (design.md's Alternatives, since
      proposal.md's mention can then point at design.md), or drop both
      references since the alternative is already ruled out for the stated,
      sufficient reason ("a second pin beside `flake.lock`" is argued
      earlier in the same bullet, only for a *different* alternative — the
      Cargo-git-dependency one — so it doesn't stand in for the missing
      explanation here).
      **Measured:** `git grep -n "shallow"` across `openspec/`, `CLAUDE.md`,
      `docs/` returns exactly 2 lines, both references, 0 definitions.
      Severity: minor — it costs a future reader context, not correctness;
      nothing downstream depends on the trap being understood, since the
      alternative it describes was already ruled out on other, stated
      grounds.
      **Fixed** in the commit that ticks this box: I took the second fix
      shape and dropped both references. The re-export alternative in
      `design.md` D1 now rests on the owner's ruling and on the reason
      already given ("the build's own flake is not changed to serve
      developer tooling"). `proposal.md`'s out-of-scope bullet points to
      D1. I did not take the first shape because nothing can source the
      explanation. `git log --all -G shallow` finds no commit that defines
      the term, and the draft that explained it was never committed. The
      owner's comment names the trap without explaining it. Any clause
      written now would be a reconstruction presented as the lost text.
      Checked with `git grep -n -F "shallow"` over `openspec/`,
      `CLAUDE.md`, `docs/`, `README.md`, `.gitignore` and `.github/`:
      after the change it matches only this findings file. Prose only, so
      no test can go red on it. `cargo test` (1180 + 30 passed) and
      `nix build ./dialectica#lgx` both passed on the edited tree.

## What was clean

- **README "Building"** — the new staging paragraph reads well: it states
  the failure mode verbatim (`failed to load manifest for dependency
  logos-rust-sdk`), gives the one command, says when to re-run it ("again
  whenever the builder pin ... moves"), and hands off to the existing
  `cargo test` block with a one-word "Then:" transition. Ran both commands
  from a fresh worktree with no `dialectica/logos-rust-sdk-src` staged; they
  work exactly as written (see hand-back).
- **`.github/workflows/ci.yml`** — the rewritten comment block above the
  staging step is the strongest prose in the diff: it states the failure
  verbatim, says *why* `--inputs-from` rather than restating *what* it does,
  and the closing two paragraphs (why a literal rev would be a second source
  of truth; why `test -f` exists) each answer a "why not the obvious
  alternative" a reader would actually ask. No restated-code comments found.
  The re-grounded `install-nix-action` comment (D5) reads as a correction,
  not a patch: it states the old reason no longer holds, gives the new one,
  and cites the exact command (`jq "[.nodes[] | .locked.type] | unique"
  dialectica/flake.lock`) a reader can run to check it.
- **`.gitignore`** — the old comment's own staging recipe is fully replaced
  by a pointer to the README, with a one-line reason ("so that this comment
  cannot drift from it") that says why a pointer and not a second copy.
- **`CLAUDE.md`** — the new bullet sits in the right place (the "costs a
  click every time" gotcha list, right after the sibling `nix build
  ./dialectica#lgx` fix it shares a topic with), is scoped to one thing (stage
  before testing, don't wait for someone else), and does not duplicate the
  README's command — `git grep -n "logos-rust-sdk-src\|rust-sdk-src"
  CLAUDE.md` finds only the one bullet, no second copy to drift.
- **`tasks.md`** — each `[x]` task states what was verified and the
  observed result, not just "done"; the struck rows for `spec` and `tests`
  each give a reason a reader can check rather than asserting the skip.
- Did not find: restated-code comments, a vague `handle`/`process` name,
  inconsistent terminology between README/ci.yml/design.md/.gitignore for
  the same command or concept, or any place where the diff's structure
  fought the existing file's shape.

## Hand-back

Read issue #168 with `gh issue view 168 --json body,comments`, including the
owner's 2026-09-25 correction comment (the local-run concern was withdrawn,
and the relative `--inputs-from ./dialectica` form was confirmed as the
fix), before starting this review.

Verification run from this tree's root (a fresh worktree with no
`dialectica/logos-rust-sdk-src`):

- `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src` — exit 0, staged the link.
- `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` — `dialectica`: 0 passed/0 failed; `dialectica-core`: 1180 passed/0 failed (67.75s); `end_to_end`: 30 passed/0 failed. All green, matching `tasks.md` task 2.1's recorded numbers.
- `nix build ./dialectica#lgx` — succeeded (one ignored, non-fatal "SQLite database busy" eval-cache warning; a `result` symlink was produced, confirmed via `git status --ignored`).

Branch: `worktree-wf_f99d9290-b34-3` (confirmed via `git rev-parse
--abbrev-ref HEAD`; not `piece/168-sdk-staging`, not the repository root —
safe to mutate, though this docs/CI-only piece gave no code to mutate
against `cargo mutants`, so none was run). No mutations were made to the
tree; the only changes in this worktree are the staged SDK out-link
(`dialectica/logos-rust-sdk-src`), the `nix build ./dialectica#lgx` `result`
symlink, `dialectica/rust-lib/target/` from `cargo test`, and this findings
file — all gitignored except the findings file itself. The tree is ready to
prune once this commit is cherry-picked.

Did not edit `tasks.md` (the runner ticks the readability row) and did not
edit anything under `.claude/`. No role-file change is suggested by this
review.

## Re-review of 67a6605

Read issue #168 with `gh issue view 168 --json body,comments`, including the
owner's 2026-09-25 comment, before starting this re-review.

The fix takes the second fix shape offered in the original finding: it drops
both dangling references rather than writing a new clause to explain the
trap. Checked that this answers the finding and leaves nothing dangling:

- `git grep -n -F "shallow" openspec/ CLAUDE.md docs/ README.md .gitignore
  .github/` matches only lines inside this findings file itself (the
  original finding text and the "Fixed" note describing the fix) — zero
  matches in `proposal.md` or `design.md`. The two dangling references are
  gone and nothing new introduces the term.
- `proposal.md:71-73` now reads self-contained: "No `flake.nix` change and
  no re-export of the builder's output. The owner's comment says that if
  `--inputs-from` works, the re-export is unnecessary. `design.md` D1
  records it among the alternatives." The new cross-reference resolves:
  `design.md:38` has `### D1. Resolve the builder with --inputs-from
  ./dialectica`, and the re-export bullet sits inside D1's "Alternatives,
  and what ruled each out" list (`design.md:82-85`), so "D1 records it among
  the alternatives" is accurate, not just plausible-sounding.
  `git grep -n "design\.md" openspec/changes/sdk-staging/proposal.md` shows
  this is the only design.md cross-reference in the file — a new pattern for
  this doc, but not a inconsistent one, since nothing else in proposal.md
  needed one before.
- `design.md:82-85`, the re-export bullet, now stands on two grounds only:
  the owner's ruling and the restated reason ("the build's own flake is not
  changed to serve developer tooling"). Both are already established
  earlier in the same document (Non-Goals, D1's own text), so the bullet
  does not lean on anything unstated.
- `git log --all -G shallow --oneline -- openspec/` confirms the underlying
  factual claim in the dev-writer's "Fixed" note: no commit in this
  project's history ever defines the "shallow-`//`" term — it entered
  already dangling in `d9f747b`/`c658303` and `67a6605` is the commit that
  removes both references. The dev-writer's claim that "nothing can source
  the explanation" holds.

No new findings. Both changed passages are clear standalone prose with a
resolvable cross-reference, and the commit fully answers the finding.
