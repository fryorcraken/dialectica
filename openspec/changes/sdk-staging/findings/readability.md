# Readability review — #168 sdk-staging

Dimension covered: **readability only** (correctness, security, and
architecture are other reviewers' rows).

Scope read: `README.md`, `.github/workflows/ci.yml` (the "Stage the
logos-rust-sdk source" step and its comment block, plus the re-grounded
`install-nix-action` comment), `.gitignore`, `CLAUDE.md`, and the whole
`openspec/changes/sdk-staging/` folder (`proposal.md`, `design.md`,
`tasks.md`, `.openspec.yaml`), against `git diff 8368b2f...HEAD`.

## Finding

- [ ] **`dev-writer`** — `openspec/changes/sdk-staging/proposal.md:73` and
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
