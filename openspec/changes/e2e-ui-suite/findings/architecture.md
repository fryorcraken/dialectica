# Architecture review — `e2e-ui-suite`

Scope: architecture only (one of four `code-reviewer` dimensions). Reviewed
`git diff origin/main...HEAD` against the local tree (one commit ahead of the
PR's remote ref), design.md, proposal.md, tasks.md, and compared structurally
against radicle-logos-module's `.github/workflows/ui-tests.yml` and
`docs/e2e.md` per the owner's standing direction.

## Finding

- [ ] **`dev-writer`** — `.github/workflows/ui-tests.yml:89` and
      `.github/workflows/ci.yml:1141` — the sitometres version pin is
      duplicated across two workflow files with nothing but a comment to keep
      them in sync, which is the exact "hand-maintained list goes stale
      silently" shape CLAUDE.md names and this same piece closes elsewhere
      (the "every spec in the tree is in the matrix" step, and
      `tst_scaffold_values_unchanged.py`).

      **Scenario:** `ui-tests.yml`'s `SITOMETRES` env var and `ci.yml`'s
      `ui-specs` job each hardcode `@paradoxcomputer/sitometres@0.1.2`
      independently (`ui-tests.yml:89`, `ci.yml:1141`). `ci.yml:1136-1137`'s
      comment says "The same pinned version ui-tests.yml runs the specs
      with. A different version here would validate against a schema the run
      does not enforce" — but nothing checks that the two strings actually
      match. Bump one and forget the other (exactly the failure mode
      `ui-tests.yml`'s own "Every spec in the tree is in the matrix" step
      exists to catch for the spec matrix) and `ui-specs` validates every spec
      against a schema version the expensive job does not run, silently
      defeating design.md D8's claim that "the schema checked is the one
      enforced." The failure is invisible: both jobs stay green, because a
      spec valid under 0.1.2 is very likely also valid under whatever the
      other file now runs — until a step vocabulary or field genuinely
      changes between versions, at which point the cheap job's whole purpose
      (catch a malformed spec before the Basecamp build) is defeated for
      exactly the case it exists to catch.

      The same duplication exists for `LGS_VERSION: "0.3.1"`
      (`ui-tests.yml:79`, `ci.yml:1472`), with the same "kept in step with
      ci.yml's build job" comment and no check — lower stakes, since a `lgs`
      version drift fails loudly at build time rather than silently
      validating against the wrong schema, but the same shape.

      **Not measured as a live drift** — both files currently agree (`0.1.2`
      and `0.3.1` respectively) — this is a structural gap, not a claim that
      the pins are wrong today.

      **Suggested fix, not prescribed:** either read `SITOMETRES` from a
      single place (e.g. `ci.yml`'s `ui-specs` job installs from a version
      string `ui-tests.yml` also reads via `${{ vars.* }}` or a committed
      file), or add a cheap check — in the spirit of
      `tst_scaffold_values_unchanged.py`'s "extract the real step by name,
      don't retype it" — that greps both workflow files for the pin and fails
      if they disagree.

## What was checked and is clean

- **`adjudicate-ui-run.py` vs. radicle's inline heredoc (design.md D1):** the
  move from a heredoc to a checked-in, tested script is a genuine
  simplification, not just a relocation — it is exercised by `ci.yml`'s cheap
  `ui-specs` job on every PR, closing exactly the gap the piece's own
  rationale names (both QML-gate defects shipped because a heredoc cannot be
  run without pushing a branch). `tst_adjudicate_ui_run.py` and
  `tst_scaffold_values_unchanged.py` both extract the real script/step by name
  rather than retyping it, avoiding the class of test that stays green through
  a change to the thing it claims to pin.
- **`ui-tests.yml`'s "Every spec in the tree is in the matrix" step** has no
  counterpart in radicle's workflow (radicle relies on the comment-only
  discipline in `docs/e2e.md`, and paid for that once — `SPEC` was hardcoded
  to one file while three specs sat running nowhere). Dialectica's version
  closes that gap with an automated, bidirectional count check. This is a
  structural improvement over the model it is copying, not merely parity.
- **The `lgs`-first move (D2–D4):** builds and installs go through
  `lgs basecamp setup`/`install` rather than radicle's raw `nix build` +
  `lgs basecamp build --variant lgx`, matching the owner's "`lgs` is
  preferred in CI wherever a verb exists" instruction. The two "both verbs
  rewrite scaffold.toml" and "inspector must be present" guards are carried
  from radicle with the same reasoning, adapted correctly for the `setup`
  path (reading `basecamp.state` instead of `nix build`'s own out-link).
- **Root handles on `Main.qml` (D6):** the five `readonly` properties are a
  thin projection of state screens already own — none is a second copy of
  membership or join state — and the diff is purely additive (25 lines, no
  edits to existing navigation logic), so the test surface and the navigator
  logic stay visibly separate in the file (comment-delimited block) rather
  than interleaved.
- **No Rust files are touched by this piece** (`git diff --stat` — no `.rs`
  paths), so `cargo mutants` scoped to changed files has nothing to run
  against; not applicable to this review.
- **No new dependency manifest.** Neither `dialectica-ui/package.json` nor a
  lockfile exists or is added — `sitometres` and `yaml` are pulled via
  `npx --yes` / `npm install --no-save`, matching radicle's precedent exactly
  (radicle-ui has no package.json for this either). No lockfile-drift surface
  introduced.
- **`check_qml_reachable.py`'s doc-comment update** correctly narrows its own
  "what this cannot see" claim now that a real e2e layer exists, and correctly
  scopes it ("cover only the screens their specs drive") rather than
  overclaiming full coverage.
- **`ci.yml`'s header/comment corrections** (removing the stale "dialectica
  has no e2e harness" claims) are accurate against the new `ui-tests.yml` and
  `ui-specs` job, and the "Not yet a job" section's rewritten `install`/
  `delivery_module` bullet is backed by a real check elsewhere in `ci.yml`
  (the `lint` job's `role == "dependency"` assertion at line 479, pre-existing
  and unmodified by this diff) — the claim is not decorative.

## Findings by recipient

- `dev-writer`: 1
