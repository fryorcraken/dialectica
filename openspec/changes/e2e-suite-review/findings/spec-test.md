# spec-test review — e2e-suite-review

Reviewed against: this change's `proposal.md`, the archived
`openspec/changes/archive/2026-09-25-e2e-ui-suite/proposal.md`, GitHub issue
#134 (`gh issue view 134`), and the `openspec/specs/stoa-navigation-view` and
`openspec/specs/view-navigation` requirements `join.yaml` claims to exercise.
Read at the piece branch's tip (`piece/134-e2e-suite-review`, `ee8e145`) via
`git show`, since this worktree's own branch was forked from an older base
(`8368b2f`) that predates the piece — see the process note at the end.

## Findings

- [x] **`spec-writer`** — two `screenShown` assertions `join.yaml` makes have
      no backing scenario in `view-navigation` or `stoa-navigation-view`,
      although this change's proposal.md states plainly that "every behaviour
      `join.yaml` asserts is already a requirement."
      1. `it starts on the Stoa list` asserts `root.screenShown === 'list'`
         on a fresh run. No requirement states what screen the view opens on.
         It is *entailed* by other requirements taken together — the
         per-Stoa onboarding screen must not be reachable
         (`view-navigation`, "Acquiring an identity is reached from the
         navigator"), the feed is not rendered before a Stoa is chosen
         (`view-navigation`, "The view holds no Stoa of its own..."), and
         exactly one screen is shown at a time (`view-navigation`, "Exactly
         one screen is shown...") — but no scenario says the survivor of
         that process of elimination is the list.
      2. `it is refused and the list is still up` asserts
         `root.screenShown === 'list'` after a malformed paste.
         `stoa-navigation-view`'s "Input that is not a Stoa reference is
         refused before any call" requires the refusal and that no join call
         is made, but no scenario there or in `view-navigation` states that
         the screen does not navigate to the preview on malformed input.
      **Scenario:** a future navigator that renders an interim or splash
      screen before the first listing resolves, or that flips to the preview
      screen as soon as a paste action is taken and only discovers the parse
      failure once there, would break `join.yaml` while contradicting no
      scenario's literal text — so a reviewer reading only the spec would
      see nothing wrong with either behaviour.
      **Measured:** by reading; `grep -n "navigat" openspec/specs/stoa-navigation-view/spec.md openspec/specs/view-navigation/spec.md`
      returns no requirement or scenario heading, only prose uses of the
      word. This is a gap to evaluate and capture (or explicitly decline),
      not a defect in `join.yaml` — the asserted behaviour is almost
      certainly the intended one, it just is not yet a requirement anyone
      can cite.

      **Outcome (`spec-writer`): fixed.** Both behaviours are now contracted.
      This change has a delta to `view-navigation`
      (`specs/view-navigation/spec.md`) that adds two requirements:
      - "The view opens on the Stoa list". It covers the opening screen, and
        makes it independent of the core's answers: an empty listing with no
        key, a failed listing, a failed key query, and no bridge at all. So a
        splash or interim screen shown in the list's place would now contradict
        a scenario.
      - "A paste refused as not a Stoa reference leaves the list rendered". It
        has two scenarios: plain text that is not a reference, and a JSON
        object missing its `genesis`. The second closes the navigate-first,
        discover-the-failure-later shape named above for the typed-half check
        as well as the JSON parse.

      They go in `view-navigation`, not `stoa-navigation-view`, because
      `view-navigation`'s Purpose gives it the transitions and gives the list
      screen's contents to `stoa-navigation-view`. Both requirements say which
      `stoa-navigation-view` requirement owns the refusal itself.

      The view already behaves this way (`Main.qml`'s `screenShown`, and
      `DStoaListScreen.preview()` returning before `previewRequested` on a
      failed parse), so no code changes. Existing component tests pin the
      plain-text paste (`tst_e2e_handles.qml`,
      `test_the_paste_failure_is_the_list_screens_own`). They also pin the
      opening screen for an empty listing with a failed key query
      (`tst_stoa_screens.qml`,
      `test_the_view_supplies_no_stoa_of_its_own_before_one_is_chosen`, whose
      fake bridge answers `get_master_key` with the error shape). The `tester`
      needs to cover the rest: the opening screen for an empty listing with no
      key held, for a failed listing, and with no bridge, and the
      missing-`genesis` paste.

      One correction to the finding: the "already a requirement" sentence is
      not in this change's `proposal.md`. It is in the archived
      `e2e-ui-suite/proposal.md` (line 132), in its `.openspec.yaml`, and in
      `join.yaml`'s header. This `proposal.md` now says the claim held for
      every step but these two, and that the delta makes it true.
      `.openspec.yaml` no longer declares `skip_specs`, and the stage block's
      spec row is restored. `openspec validate e2e-suite-review --strict`
      passes.

## Checked and clean

- **Every other `join.yaml` step maps to an explicit scenario.** Membership
  read-without-error and the no-Stoas count map to "Holding no Stoas and
  failing to read membership are different screens." The no-key state
  (`createKeyButton` present, `createTitleField`/`createStoaButton` absent)
  maps to "Creating a Stoa asks for a title and nothing else..." → "The
  affordance is not instantiated when no key is held," paired with "With no
  key held, making the key is the only task on the screen." The
  well-formed-but-unverifiable paste, the preview-before-join, and the
  presented refusal map respectively to "Opening an address previews rather
  than joins," "The join call is made only on the user's explicit action"
  (inferred from "Joining shows what is being joined..."), and "A record
  that does not match its address is a distinct failure" /
  "A join is reported from the core's reply, never assumed." `join.yaml`'s
  own header comments correctly name the requirement each block is checking,
  and I found nothing that named the wrong one.
- **Layer is correct.** `tst_e2e_handles.qml` is explicit in its own header
  comment about what a component test can and cannot see — it pins that
  each handle *projects* the right state, and states plainly that whether
  sitometres can *read* that state through the host's inspector "only the
  e2e run can [see], and that is the run's job." It does not claim
  end-to-end coverage it cannot deliver, so there is no coverage gap hiding
  behind a component-layer green here. `join.yaml` itself is the only test
  that can observe the cross-process behaviour `stoa-navigation-view`'s
  Purpose paragraph names (the Theme/DTheme-style defect class), and it is
  the one driving a real Basecamp.
- **The three adjudication conditions issue #134 and the archived proposal
  set (pass verdict, every step passed, count equals the spec's) are all
  three independently pinned in `tst_adjudicate_ui_run.sh`**, each with a
  paired case that isolates it (a stopped-early report with nothing failed;
  a report with *more* steps than the spec, not only fewer; a failed step
  inside an otherwise-passing report; an inconclusive verdict; all three at
  once). None of the three defect shapes from this repo's known test-defect
  family (length-only diff standing in for content, a hash/property proven
  rather than the content, a position pinned without a value) appears in
  any of the four test files I read in full
  (`tst_adjudicate_ui_run.sh`, `tst_scaffold_values_unchanged.sh`,
  `tst_ui_tool_pins.sh`, `tst_e2e_handles.qml`).
- **`tst_ui_tool_pins.sh` and `tst_scaffold_values_unchanged.sh` extract the
  real `run:` bodies from the real workflow files by step name** rather than
  retyping them, so a rename fails loudly rather than passing on a stale
  copy, and each negative case is a real fixture with exactly one property
  changed from the paired positive one (a `sed` derivation with a defensive
  check that the derivation actually changed something, in the scaffold
  test's case).
- **The two CI proofs `tasks.md` records (sections 2 and 3) show what they
  claim.** I independently pulled all four runs with `gh run view` rather
  than trusting the transcript:
  - Red, no-key step: [36140112191](https://github.com/fryorcraken/dialectica/actions/runs/36140112191)
    — job failed at "Run the spec" and "The run proved what the spec asks,"
    with the exact annotations `tasks.md` 2.1 quotes: `verdict is 'fail',
    expected 'pass'`, and `steps that did not pass:` naming "a fresh profile
    is offered a key and not a Stoa" and every step after it, verbatim.
  - Green revert: [36141018728](https://github.com/fryorcraken/dialectica/actions/runs/36141018728)
    — job green, no failing annotations.
  - Red, scaffold guard: [36142050293](https://github.com/fryorcraken/dialectica/actions/runs/36142050293)
    — job failed at "lgs left scaffold.toml's values alone," with the
    annotation `an lgs basecamp verb changed a value in scaffold.toml — see
    the diff above`, and the adjudicator step also red with `no JSON report
    — sitometres was killed before it could write one (job timeout?), so
    nothing was proved` — matching `tasks.md` 3.1's transcript exactly,
    including the mis-predicted message design.md D1 flags as its own
    finding (which is a correctness/design matter, not filed again here).
  These four runs corroborate that both checks can fail for the reason
  claimed, independent of the tasks.md narrative.
- **NO SPEC marker** (`tst_adjudicate_ui_run.sh:110`) is the inherited one
  from #164 that this piece's own proposal.md and design.md D1 discuss at
  length and deliberately leave unresolved, with reasoning recorded. It is
  a decision, not an unmarked gap, so no fresh finding is opened for it.
- **Part 4 (capabilities moved between changes) does not apply.** This
  piece declares no capability changes (`.openspec.yaml`: `skip_specs:
  true`), and neither the proposal nor design.md moves a requirement
  between capabilities.
- **Part 5 (spec soundness).** No self-contradiction found across the two
  specs read in full. Against issue #134: the proposal's narrower scope
  (review #164's merged code, prove two checks in CI, add the yq/jq rule)
  is consistent with the issue's request for wider coverage as a *later*
  step — the PR carries "Part of #134," not a closing keyword, matching the
  issue's own framing that this suite is a starting point.

## Mutation sampling (part 2) — blocked, not skipped

I attempted two independent mutations, one per the QML/shell layers named in
the brief:

1. `.github/workflows/ui-tests.yml`'s `SITOMETRES` pin
   (`@paradoxcomputer/sitometres@0.1.2` → `...@0.1.3`), to sample-check that
   `tst_ui_tool_pins.sh`'s "the workflows as committed" case (the one
   exercised against the real files rather than a jq-edited copy) actually
   fires on a real mismatch and not only on its synthetic fixtures.
2. `dialectica-ui/src/qml/Main.qml`'s `stoaCount` handle
   (`list.visibleRows.length` → the constant `0`), the exact null-implementation
   `tst_e2e_handles.qml`'s own header comment names as what each fixture is
   chosen to fail against, to sample-check that claim rather than take the
   comment's word for it.

Both `Edit` calls were refused by the harness's own permission classifier
("Modify Shared Resources"), not by anything in this repo's tooling. Per this
task's instruction on a refused tool call, I did not retry either one or
attempt a different tool to reach the same edit. Both files are therefore
judged by reading alone (see "Checked and clean" above) rather than by a
measured mutation. Nothing here is reported as a mutation surviving, because
no mutation ran; this is a gap in how much of part 2 I could complete, not a
finding about the tests themselves. Whoever picks this up next and has Edit
access to this worktree may want to run those same two mutations before
trusting my read-only judgement of `tst_ui_tool_pins.sh` and
`tst_e2e_handles.qml` completely.

## Process note: this worktree's base predates the piece

`git rev-parse --abbrev-ref HEAD` on this worktree is
`worktree-agent-a7e544a3de5d8edf8`, forked from `8368b2f` — the commit `main`
was at when PR #164 merged, *before* `piece/134-e2e-suite-review` was opened.
`openspec/changes/e2e-suite-review/` therefore does not exist in this
worktree at all; every file under it in this report (`proposal.md`,
`tasks.md`, `design.md`, `.openspec.yaml`) was read with
`git show piece/134-e2e-suite-review:<path>` against the local branch of that
name, not from this worktree's working tree. The six named test files and
the two specs *do* exist here, because PR #164 already merged them to `main`.

Consequence: I have **not** ticked my row in `tasks.md`'s stage block, and
did not attempt to, because `tasks.md` does not exist in this worktree — a
locally-written copy would be a brand-new file with no shared history, and
cherry-picking a same-path "add" onto a branch that already has that file
(with different content) is a conflict, not the one-line tick a rebase
should be. Only this findings file is committed here. The runner should tick
the `review: spec-test` row on `piece/134-e2e-suite-review` directly when it
picks up this commit.
