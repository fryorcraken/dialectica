# Architecture review — e2e-suite-review

Scope: architecture only (script boundaries, what each gate owns, duplication
between `ci.yml` and `ui-tests.yml`, and readiness for the next piece's wider
coverage). Covers every file PR #164 merged (`git show --stat 8368b2f`, read
at this piece's HEAD after fast-forwarding onto `ee8e145`), and this piece's
own diff (`git diff 8368b2f...HEAD`, which touches only `CLAUDE.md` and
`openspec/changes/e2e-suite-review/*` — no shell, QML or workflow file). For
`ci.yml`, only #164's own hunks (the `ui-specs` job and its comment edits), per
the brief.

Compared against the named model, `radicle-logos-module`'s
`.github/workflows/ui-tests.yml` and `docs/e2e.md`, read from
`/home/fryorcraken/src/rad/radicle-logos-module`.

## Findings

- [ ] **`dev-writer`** — `.github/workflows/ui-tests.yml:118-133` and
      `.github/workflows/ci.yml:1168-1180` — the apt third-party-source
      workaround is duplicated verbatim across the two workflows, with no
      shared script and no check that would catch the copies drifting.
      **Scenario:** both steps run the identical five-line dance (`mkdir -p
      /etc/apt/disabled-sources.d`, `find … -exec mv …`, `apt-get update -qq
      || echo ::warning`, `apt-get install …`) before installing `yq`. It
      exists because "radicle's identical step once failed `update` on a 403
      from a preconfigured third-party source" (both files' own comments say
      so, and `ci.yml`'s copy points at `ui-tests.yml`'s as the one it
      mirrors). If a future runner image adds a new failing default source, or
      Ubuntu renames the disabled-sources convention, fixing one copy and
      missing the other silently reopens the exact failure this workaround
      exists to close — in whichever workflow was not touched. This is the
      same class of drift the piece explicitly guards elsewhere: two literals
      that must stay in sync (`SITOMETRES`, `LGS_VERSION`) got
      `tst_ui_tool_pins.sh`, and a guard's own mechanism duplicated by
      description rather than by reference got `tst_scaffold_values_unchanged.sh`
      wired into CI (design.md D11). This duplicated *procedure* has no
      analogous check, and design.md's D11/D12 discussion of the apt step
      only records that it "mirrors ui-tests.yml's own" — it does not
      consider or reject a shared script as an alternative the way D8
      considered and rejected one shared file for the tool-version pins.
      A small drift already exists between the two copies: `ui-tests.yml`'s
      step prints `tomlq --version` after the install as a sanity check;
      `ci.yml`'s copy of the same install does not, even though the
      `ui-specs` job goes on to run `tst_scaffold_values_unchanged.sh`, which
      depends on `tomlq` working.
      **Severity:** low-to-moderate. Nothing is broken today — both copies are
      otherwise identical and the suite's own tests pass locally (verified:
      `tst_adjudicate_ui_run.sh`, `tst_ui_tool_pins.sh` and
      `tst_scaffold_values_unchanged.sh` all pass against the real `yq`/`jq`
      installed in this environment). The risk is future silent divergence,
      not a present defect, which is why this is a moderate finding and not a
      correctness one. A fix would extract the shared body into one script
      (e.g. `dialectica-ui/tests/install-apt-yq.sh`, parametrised by the extra
      package list) that both workflows invoke, mirroring how
      `require-jq-yq.sh` and `adjudicate-ui-run.sh` are already shared rather
      than duplicated.

## Clean

Everything else checked against the architecture questions in the brief came
out clean, and is recorded here rather than as boxes, per the instruction not
to pad the list:

- **Script boundaries.** `adjudicate-ui-run.sh`, `require-jq-yq.sh`,
  `tst_ui_tool_pins.sh` and `tst_scaffold_values_unchanged.sh` each do one job
  and are composed rather than duplicated: the three callers source
  `require-jq-yq.sh` once for the shared "is this the jq-wrapper yq" guard,
  and `tst_scaffold_values_unchanged.sh` extracts the real `ui-tests.yml`
  steps by name with `yq` rather than re-typing them (the same discipline
  `tst_check_bindings.sh` already uses elsewhere in this repo). Ran all three
  test scripts locally against the real `yq`/`jq` on this machine; all pass,
  confirming the documented behaviour is real rather than merely claimed.
- **What each gate owns.** The cheap/expensive split (`ci.yml`'s `ui-specs`
  job validates schema and tests the adjudicator/tool-pin/scaffold-guard
  scripts against fixtures, with no Basecamp; `ui-tests.yml`'s `spec` job
  builds a real Basecamp and drives it) is a clean boundary, matches the
  model repo's own split, and the "every spec in the tree is in the matrix"
  and "specs parse against the schema" checks both glob the same directory,
  so they cannot drift apart from each other by construction.
- **Extraction vs. the model.** `adjudicate-ui-run.sh` is a checked-in,
  independently-tested script; the model repo's equivalent step is a `python3
  - <<PY` heredoc inline in `ui-tests.yml`. Dialectica's shape is the
  improvement CLAUDE.md already argues for elsewhere in this repo ("a heredoc
  cannot be tested… run without pushing a branch") — this piece's own
  Main.qml handles and `tst_e2e_handles.qml` follow the same discipline.
- **Growth readiness.** The matrix (`strategy.matrix.spec: [join]`) and the
  "every spec in the tree is in the matrix" count already generalise to more
  specs with no reshaping — adding `feed.yaml` etc. is adding one matrix
  entry and one file. `Main.qml`'s five read-only handles are a single,
  clearly-scoped, well-commented block ("the suite's interface, and nothing
  else reads them"); nothing about the shape forces a redesign to add more
  per screen. `lgs basecamp install` already seeds **both** `alice` and
  `bob` profiles (comment at `ui-tests.yml:73`: "`install` fills both seeded
  profiles; one is enough to run in") though only `alice` is driven today —
  a second, already-seeded profile is present but unused, which is a
  plausible head start for the follow-up piece's "successful join with a
  seeding peer" rather than something that piece will have to build from
  nothing. That said, **no mechanism for running a second peer exists yet**
  (sitometres drives one Basecamp process; the model repo's own answer to an
  analogous need, `write.yaml`/`local.yaml`'s `seed_write_profile` Rust
  example run conditionally on `matrix.spec`, seeds a profile directly rather
  than running a second live peer) — this is correctly left undecided rather
  than approximated, per the proposal's "Not in this piece", and is not a
  finding against this piece or #164.
- **Duplication the piece already guards.** `SITOMETRES` and `LGS_VERSION`
  are each written twice (once per workflow) and kept equal only by
  `tst_ui_tool_pins.sh` — this is deliberate (design.md D8, "two literals and
  a check, not one home") and the check's own tests pass locally. Not a
  finding.
- **`check_qml_reachable.py`'s docstring edit** (#164's only change to that
  file) replaces a stale branch-name reference (`piece/e2e-sitometres`) with
  an accurate pointer to `tests/ui/`/`ui-tests.yml` — a real improvement, not
  a new gap.
- **This piece's own diff** (`git diff 8368b2f...HEAD`) touches no shell, QML
  or workflow file — only `CLAUDE.md` (the yq/jq line) and this change's own
  `openspec/` files, plus the two break/revert pairs on `scaffold.toml` and
  `DStoaListScreen.qml`, which net to zero per `tasks.md` 4.1. There is no
  architecture surface in this piece's own diff beyond the one CLAUDE.md
  line, which is a single sentence in an existing paragraph and raises no
  concern.

## Note on this review's own worktree

This worktree was dispatched (`isolation: "worktree"`) forked from a HEAD that
predated the piece being opened (`8368b2f`, PR #164's merge commit, before the
`Open e2e-suite-review` commit). `pwd` and the branch name
(`worktree-agent-a64a0e706689bb4a3`, not `piece/…` and not the repo root)
passed the required check, so this was fast-forwarded locally
(`git merge --ff-only piece/134-e2e-suite-review`) to reach the piece's actual
tip (`ee8e145`) before reviewing — a fast-forward merge from a strict ancestor,
not a rewrite, and no push was made. Recorded here in case the runner wants to
check why `baseRef: "head"` did not land this worktree on the piece's current
tip.
