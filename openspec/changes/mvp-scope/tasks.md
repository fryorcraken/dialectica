# Tasks — record four owner scope rulings on the MVP

## Stages

- [ ] ~~spec — `spec-writer`~~ — **no spec delta.** This change records staging
      decisions in `docs/PLAN.md` and alters no behaviour and no requirement.
      Every capability the four rulings touch stays exactly as it is. Declared as
      `skip_specs: true` alongside `schema:` in `.openspec.yaml`, which carries
      the argument — including why ruling 4 is not contract material.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

`docs/PLAN.md` only, plus `design.md`. **No code changes**, so no gate in this
repo can see this piece — see the note under "What no gate can see" below.

- [x] `design.md` written, with the two migrated reasoning passages under
      Decisions 6a/6b; 6c records the third candidate and why it stayed in
      PLAN.md instead
- [x] **Ruling 1** — §7.2 gains a block-quoted scope note; §9.2's MVP item 5 is
      struck and pointed at the ruling
- [x] **Ruling 2** — §9.1's question 8 rewritten in place as a decided
      exclusion, keeping its reasoning and striking only the "what would decide
      it" clause
- [x] **Ruling 3** — §6's existing note extended with the UI half and with the
      contract-level block
- [x] **Ruling 4** — §9.2 gains the four-ruling section, the two-case rule, and
      the statement that the two merged requirements are not overridden
- [x] **The case 2 list** — new §9.2 subsection, seeded with the five verified
      instances
- [x] §9.2's open question at the old lines 3599-3608 resolved in place rather
      than answered by a parallel entry
- [x] Migrated reasoning leaves no second copy in PLAN.md — verified by grep:
      `grep -c "teaches users the app is broken" docs/PLAN.md` returns 1, in the
      strike-and-point passage that names what was resolved
- [x] Every citation re-read against this tree rather than relayed from the
      proposal. **No list of what was checked is kept here**, because the
      citations themselves are the checkable artefact: each names a file and a
      line, so a reader re-runs the check by reading them. A verification list
      would be a second copy going stale against the citations it describes —
      and the earlier version of this line pointed at "the report", which
      returns to the runner and is never persisted anywhere a reader can open.

### What no gate can see

**This piece changes no code, so every gate in this repo is structurally blind
to it.** `cargo test`, the QML suite, `check_qml_names.py` and
`check_qml_members.sh` all pass unchanged and none of them measured anything
about this change. That is stated rather than reported as a green run, per
`.claude/agents/README.md`: "exit 0 on a gate that measured nothing is worse
than no gate."

The only mechanical check that applies is `openspec validate --strict`, which
checks the change's own structure and not PLAN.md's content.
