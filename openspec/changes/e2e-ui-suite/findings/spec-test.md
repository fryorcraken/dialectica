# spec-test review — e2e-ui-suite

Scope: this piece is test-only (`skip_specs: true`). The contract reviewed
against is `proposal.md`, the three adjudication conditions (`tasks.md` /
issue #134 / #120's carried reasoning), and the `stoa-navigation-view` /
`view-navigation` requirements `join.yaml`'s header claims to exercise.
Reviewed test files: `dialectica-ui/tests/ui/join.yaml`,
`tst_adjudicate_ui_run.py`, `tst_scaffold_values_unchanged.py`,
`tst_e2e_handles.qml`, `validate-ui-specs.mjs`. I did not read `Main.qml`,
`adjudicate-ui-run.py` (beyond the mutated lines below), the workflows'
logic, or design.md.

## Verified independently

- `python3 dialectica-ui/tests/tst_adjudicate_ui_run.py` — all checks pass.
- `python3 dialectica-ui/tests/tst_scaffold_values_unchanged.py` — all checks
  pass, including the cosmetic-rewrite-vs-real-change distinction.
- `sh dialectica-ui/tests/run-qml-tests.sh dialectica-ui/tests/tst_e2e_handles.qml`
  — 6/6 pass.
- `gh run view 36090846720 --repo fryorcraken/dialectica` and
  `gh api repos/fryorcraken/dialectica/actions/runs/36090846720/attempts/1` —
  confirms both attempts (cold and rerun) of "sitometres join spec" on head
  commit `9611a1f` (an ancestor of the branch's current head `54cd596`, whose
  only diff from `9611a1f` is a design.md/tasks.md-only commit) succeeded,
  corroborating tasks.md 4.1/4.2 rather than trusting them.
- `gh pr checks 164 --repo fryorcraken/dialectica` — "UI spec validation" and
  "sitometres join spec" both show `pass` on the current head, corroborating
  tasks.md 1.2/1.3 (I could not run `validate-ui-specs.mjs` myself — see
  below).

## Coverage (part 1)

`join.yaml`'s header claims six things are covered, each traced to a
requirement in `stoa-navigation-view` or `view-navigation`: app opens on the
list; membership read is distinguished from empty; no-key state renders the
key block and omits the create affordance; malformed paste refuses without
navigating; well-formed paste previews without joining; a core-refused join
is presented as a refusal. I checked each against the step assertions and
against the cited requirements' scenarios — all six are accurately described
and the assertions genuinely test the claimed behaviour (not a name that
promises more than the assertion checks). `tst_e2e_handles.qml` exercises all
five of `Main.qml`'s root handles it names (`listReadState`, `stoaCount`,
`pasteFailure`, `joinState`, `joinFailure`), each with a passing and a
discriminating fixture. No unfaithful scenario paraphrase found, and no
scenario asserting something no test could check.

Part 4 (requirements moved between capabilities) doesn't apply: no spec
delta. Part 5 (spec soundness): the two specs `join.yaml` cites are unchanged
by this piece and out of scope to re-litigate; the issue-staleness check
(`gh issue view 134`) shows `proposal.md` correctly scopes out what #134 asks
for beyond this piece (successful join, feed, thread, moderation) and labels
itself "Part of #134" rather than closing it — consistent, not stale.

## Mutation sampling (part 2)

Both attempted mutations were refused by the permission classifier before
any edit landed (`git status` is clean — nothing to restore):

1. `dialectica-ui/src/qml/Main.qml`, changing
   `readonly property int stoaCount: list.visibleRows.length` to
   `readonly property int stoaCount: 0` — refused, "Modify Shared
   Resources".
2. `dialectica-ui/tests/adjudicate-ui-run.py`, changing
   `if len(steps) != expected:` to `if False:` (disabling the step-count
   branch tasks.md 1.1 and the file's own docstring both discuss) — refused,
   "Security Test Removal".

Per my instructions I did not retry either or work around the refusal. #1
means `tst_e2e_handles.qml`'s `stoaCount` discrimination (claimed already
measured in tasks.md 3.2/design.md D6) is not independently re-verified by
me — it stands as the dev-writer's/tester's self-report only.

#2 led to a finding by reading rather than executing, below.

- [x] **`dev-writer`** — `tasks.md` 1.1 vs `adjudicate-ui-run.py`'s own
      docstring (the line right above `if len(steps) != expected:`) —
      disagree about how many `tst_adjudicate_ui_run.py` checks go red when
      the step-count branch is disabled. tasks.md says "exactly three of its
      checks red"; the code's own docstring says "exactly five checks red"
      for the identical mutation. I could not run the mutation myself (see
      above), so I traced it by hand against the six fixtures in
      `tst_adjudicate_ui_run.py`: "stopped early" loses both its checks
      (`exit 1`, "names the cause") since with the branch gone that report
      now has no problems and exits 0 — 2; "more steps than the spec" loses
      the same pair by the same reasoning — 2; "every failing condition is
      reported" keeps its exit code (verdict and bad-step problems still
      fire) but loses only its "reports the count" check, since that
      specific message string is gone — 1. Total: 5, matching the code's
      docstring, not tasks.md's "three". **Scenario:** whichever of the two
      numbers is wrong is a stale or miscounted claim recorded as a
      measurement (README's "a number in a comment is a claim, and this
      repo fabricates them" is exactly this shape) — one of the two needs
      correcting, and neither party has re-run it since I could not.
      **Measured:** by hand-tracing the fixtures against the visible
      mutation target, not by executing it (permission-refused both times I
      tried); severity low (doesn't affect merge-readiness of the tests
      themselves, both numbers describe a check that does discriminate), but
      the discrepancy itself should not ship uncorrected.

      **Fixed** in the commit that ticks this box, together with the matching
      `readability.md` finding. The mutation ran this time, in this worktree:
      `if False and len(steps) != expected:` gave `5 check(s) failed`, the
      same five you traced by hand. So the docstring and design.md D1 were
      right, and tasks.md 1.1's "three" was stale from before the tester's
      "more steps" case. tasks.md 1.1 now points at D1 for the count instead
      of carrying a copy. Mutation reverted; the suite is green.

## Spec gaps, marked and unmarked (part 3)

`grep -rn "NO SPEC" dialectica-ui/tests/ui/join.yaml dialectica-ui/tests/tst_adjudicate_ui_run.py dialectica-ui/tests/tst_scaffold_values_unchanged.py dialectica-ui/tests/tst_e2e_handles.qml dialectica-ui/tests/validate-ui-specs.mjs`
returns nothing — no marked gaps in any of the five files.

One unmarked candidate, worth a decision rather than a fix:

- [ ] **`dev-writer`** — `tst_adjudicate_ui_run.py`, "a missing report says
      nothing was proved, not that a file is absent" — this pins a specific
      behaviour (message contains "nothing was proved", not a traceback, not
      a bare file-not-found) for a case that is not one of the three
      adjudication conditions this piece's contract names (pass verdict,
      every step passed, step count matches). It is a boundary the dev chose
      to handle a particular way, with no `NO SPEC:` marker and — since I am
      blind to `design.md` — no way for me to confirm it is recorded there
      as a Decision. **Scenario:** a future change to the adjudicator's
      missing-report message would have nothing pointing back at why "not
      that a file is absent" mattered, beyond this test's own comment.
      **Measured:** not run — this is a documentation-completeness question,
      not a behavioural defect; confirm it's in design.md's Decisions (it
      plausibly already is, under D1's "docstring's attribution corrected to
      the measured one", which I have not read) or mark it.

## Was tasks.md section 5 complete?

Section 5's two items (the no-key/create-affordance `not_text` risk, and
whether `lgs` itself ever rewrites a scaffold value) are the two real,
CI-only-provable gaps in this suite, and both are stated precisely enough to
act on. I found nothing else in that category — the discrepancy above is a
documentation defect I could find by reading, not a new untested risk
requiring a CI run to settle.

## Not run

`node dialectica-ui/tests/validate-ui-specs.mjs` — the owner's rule permits
only `npx --yes <pkg>@<version>` for Node tools, and the combination needed
to run this script (`yaml` + `@paradoxcomputer/sitometres@0.1.2`, no local
`node_modules`) was refused by the permission classifier ("Untrusted Code
Integration") on the first attempt. Per my instructions I did not retry.
Corroborated instead via CI: `gh pr checks 164` shows "UI spec validation"
passing on the current head.
