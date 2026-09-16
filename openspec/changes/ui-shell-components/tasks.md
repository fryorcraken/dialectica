## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec-writer ran for this piece. The design
      bundle at `tmp/ui-bundle-new/handoff/` (`SPEC.md` + `copy.json`) is the
      contract, and the dispatch brief named it as such. This piece adds three
      shared components and four token/property changes; it adds no requirement
      to `openspec/specs/`, so there is no delta. **The unspecified behaviour it
      does introduce is marked `NO SPEC:` in the tests** — an unknown lamp state,
      an unbound lamp state, an unknown button kind, and the `muted` opacity
      value — and each is a candidate for a spec-writer to capture.
      **The judgement is now also recorded where the tooling can read it**:
      `.openspec.yaml` (`schema: spec-driven` + `skip_specs: true`, with the
      measurement) and `proposal.md`. `openspec validate --strict` passes.
- [x] design + code — `dev-writer`
- [ ] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, PR merged — `closer`

## 1. Theme tokens

- [x] 1.1 Add `statusOk`, `statusDegraded`, `statusFailed` to `DTheme.qml`, with
      the bundle's comment that this is the only place green and orange are
      allowed. Verified by `tst_status_bar.qml`, which reads all three off
      `DTheme` and asserts each lamp state maps to its own distinct colour.
- [x] 1.2 `apparatusWidth` is gone. **Not deleted by this piece — it was already
      absent at the branch point**, and the dispatch brief's claim that
      `DTheme.qml:161` still declares it is wrong. Measured: `grep -n
      apparatusWidth dialectica-ui/src/qml/DTheme.qml` returns nothing, and
      `git log -S apparatusWidth -- dialectica-ui/src/qml/DTheme.qml` names
      `3901e99` (#70, this branch's base) as the commit that removed it. The box
      is ticked because the end state the task asks for holds, and the sentence
      says who made it hold; no code of mine is involved.
- [x] 1.3 Refresh the `paperDeep` and `accent` comments to the bundle's wording.
      Verified by reading the file; comments are not executable and no test
      claims to cover them.

## 2. FlatButton

- [x] 2.1 Reshape `kind` dispatch from three parallel ternary chains into one
      `kinds` table (design D5). **Not behaviour-preserving, and lands as one
      commit with 2.2 for that reason**: tracing the old chains showed an
      unrecognised kind rendered with no fill, no border and paper-coloured text
      — an invisible button that still accepted clicks. It now falls back to
      `secondary`. Verified by
      `tst_flat_button.qml::test_an_unrecognised_kind_falls_back_to_secondary`,
      proved able to fail by restoring the old fallback spec (mutation B3).
- [x] 2.2 Add `destructive-outline` and `secondary-micro`. Verified by
      `tst_flat_button.qml`, which asserts each kind's fill/border/text-ink
      relation and that an unknown kind falls back to `secondary`.

## 3. Identicon

- [x] 3.1 Add `muted` as a root-level opacity (design D4). Verified by
      `tst_identicon_muted.qml`: muted is strictly less opaque than unmuted, and
      every selector `tst_identicon.qml` pins is byte-identical with `muted` set.

## 4. The three components

- [x] 4.1 `DStatusBar.qml` — three lamps in the fixed order DELIVERY, STORAGE,
      ZONE, each with a tooltip. `lampState` rather than `state` (design D1), and
      an unrecognised state renders degraded (design D2). Verified by
      `tst_status_bar.qml`.
- [x] 4.2 `DVouchStamp.qml` — outlined VOUCH / filled VOUCHED, vouched visible
      without hover, un-vouched only while `revealed`. Verified by
      `tst_vouch_stamp.qml`.
- [x] 4.3 `DIdentityChip.qml` — identity present shows mark, CURRENT IDENTITY,
      name and address; absent shows the sentence and the create button.
      No `isPerson` (design D3). Verified by `tst_identity_chip.qml`.
- [x] 4.4 Register all three in `qmldir`. Verified by
      `dialectica-ui/tests/check_qml_names.py dialectica-ui` passing, which also
      proves each declared file exists.

## 5. Gates

- [x] 5.1 Full QML suite green: `dialectica-ui/tests/run-qml-tests.sh`.
- [x] 5.2 Name gate green: `dialectica-ui/tests/check_qml_names.py dialectica-ui`.
- [x] 5.3 Member gate green: `dialectica-ui/tests/check_qml_members.sh`.
- [x] 5.4 Every new test proved able to fail by mutation, one mutation at a time.
      The mutations are named in the PR body.
- [x] 5.5 `openspec validate ui-shell-components --strict` passes. It did **not**
      before this pass — the change folder had no `.openspec.yaml` and no
      `proposal.md`, so it failed with "Change must have at least one delta".
      Both now exist; the `schema: spec-driven` line is what makes `skip_specs`
      be honoured rather than silently ignored.

## 6. Review findings

Each box in `findings/` carries its own outcome and the commit. Summarised here
because several changed behaviour rather than prose.

- [x] 6.1 **The tooltip markup sink.** `ToolTip.text:` binds into a `Text` at
      `textFormat: StyledText`. Fixed with `DTip`, which pins `PlainText` on its
      own content item — the attached form routes through a shared instance and
      cannot be given one. Both call sites converted, including the one whose
      strings are literals.
- [x] 6.2 **The two guards that reported clean over it.** The test walkers now
      descend `data` and `contentItem`, so a popup is reachable; each new
      tooltip test asserts its count before its formats, so a walker narrowed
      back to `children` fails rather than passing. CI gains a step banning
      attached tooltip bindings — measured firing on the reintroduced defect and
      clean on the tree.
- [x] 6.3 **`DVouchStamp` takes `SPEC.md:88`'s identity gate**, defaulting
      closed, gating both disjuncts of the visibility rule (design D9).
- [x] 6.4 **The lamp state defaults are `degraded`**, not `ok` (design D2a).
- [x] 6.5 **The kind lookup asks `hasOwnProperty`.** `kinds[kind] !== undefined`
      reached `Object.prototype`, so seven inherited member names defeated the
      fallback and rendered white-on-black at NaN size (design D5d).
- [x] 6.6 **The chip draws no mark for an empty address.** An unpadded empty
      address renders the zero address's mark under `CURRENT IDENTITY`; the mark
      is now gated on the address being present.
- [x] 6.7 **The kind sweep derives from the table** rather than restating it,
      with required-field and NaN-dimension assertions the derivation makes
      free, and a corpus check so a missing table cannot make every sweep
      vacuous.
- [x] 6.8 **Each component's file header states what it asks of a screen
      author**, since five or six screen pieces will be written against a
      contract that names none of these components. Written into
      `DStatusBar.qml`, `DIdentityChip.qml` and `DVouchStamp.qml` — the files an
      author already has open — rather than into a document of their own, which
      is the shape #83 rejected when it deleted `docs/UI-BRIEF.md` and moved
      `ScreenFrame`'s implementer contract into its own header.
- [x] 6.9 **Four false or mis-scoped prose claims corrected**: the "ONLY green
      and orange" claim in two files, the `PlainText`-because-peer-supplied
      justification, D3's and D4's citations, and D4's overstatement about
      surviving a retune.
