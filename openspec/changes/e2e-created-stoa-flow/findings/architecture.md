# Architecture findings — `e2e-created-stoa-flow`

Reviewed **only** the architecture dimension, against `git diff fe093be...HEAD`
(three dots), per the runner's brief. Read `proposal.md`, `design.md` and
`tasks.md` first. Four questions were named explicitly and are answered below,
then the rest of the diff (specs' structure/matrix, next-piece readiness,
dependencies, CI gates).

## D8's placement: `Main.qml` withholding one binding of a pair

**The mechanism is technically sound, not a coincidence that happens to pass
today's tests.** `feed.stoaAddress`'s binding reads `feed.stoaGenesis` (a
property of the *same* object) as part of its own expression:

```qml
stoaAddress: root.chosen !== null && feed.stoaGenesis === root.chosen.genesis
    ? root.chosen.stoa : ""
stoaGenesis: root.chosen !== null ? root.chosen.genesis : ""
```

That read makes `stoaAddress` a declared dependent of `stoaGenesis` in QML's
own binding graph, in addition to both depending on `root.chosen`. QML's
binding evaluation is pull-based on a dependency graph, not on the order two
independent listeners happen to fire in — so this is the correct way to force
"evaluate B's current value before deciding A" when A and B are both
re-triggered by the same upstream change, and it does not rely on QML's
(unspecified) notification order between `stoaAddress` and `stoaGenesis`,
which is exactly the bug #152 exploited. I traced the return path too
(`closeThread()` builds a *new* `chosen` object with the *same* field values):
`stoaGenesis` re-evaluates to an unchanged string, so no signal fires, and
`stoaAddress`'s guard condition is already true when it re-evaluates — no
extra hop, no flicker. I found no input that defeats it, given the invariant
`enterOnly` already guarantees (every transition passes through `chosen ===
null` first, so the guarded properties always reset to `""` between two real
Stoas — this is the same reason the design.md alternatives analysis gives for
rejecting "re-read on both halves").

**Whether `Main.qml` is the right layer, versus the screens owning it:** the
real root cause is upstream of `Main.qml` — `Main.qml` already holds one
atomic value (`root.chosen`, assigned once per transition through `enterOnly`)
and then decomposes it into three independently-bound primitive properties
(`stoaAddress`, `stoaTitle`, `stoaGenesis`) so each screen stays testable with
plain property assignment. That decomposition is what reintroduces the
ordering problem the guard then has to repair. Design.md's own "Rejected
alternatives" (hand each screen one object instead of three properties) names
this and defers it, citing a 20-test-file blast radius across every
standalone screen test — a reasonable scope call for this piece, not an
oversight. **Handing the guard to the screens instead was tried and
measured to be worse**: design.md's "re-read on both halves" alternative
(the shape `DJoinScreen` uses) sends one read carrying a stale record before
correcting itself, which the every-read check in `tst_navigation.qml` caught
red ("read 1 of 2 carried the record") — that is a live, reproducible
violation of `view-navigation`'s "MUST NOT send a placeholder... in place of
a real one," not a hypothetical. So `Main.qml` owning the guard is the
correct call *given* the three-property interface; the properly-shaped fix is
the one-object interface, already named as the deferred follow-up.

**What is genuinely a forward risk, already tracked, not newly found by me:**
the two guards written so far are not identical in shape — the feed's checks
one prior property (`stoaGenesis`), the thread's checks two
(`stoaAddress` and `stoaGenesis`) — because each screen's trigger sits at a
different depth in its own property pair/triple. That is the "fourth
slightly-different guard" shape CLAUDE.md warns about, one instance early.
Design.md's Risks section already names this exact concern ("The trigger
guards in `Main.qml` are a rule the next screen must copy") and gives a
concrete trigger for when it must stop being deferred ("a third is the point
at which the one-object interface stops being a follow-up"). I looked for an
automated backstop that would catch a *third* screen's author forgetting to
copy the guard (a lint, a generic test) and found none — the only backstop
today is the two screen-specific `tst_navigation.qml` tests and the design.md
prose. That is a real gap in enforcement, but it is not new information this
piece owes a fix for: the risk is stated, the trigger condition for revisiting
it is concrete and checkable, and no defect exists in what has shipped. I am
not opening a checkbox for it — restating an already-recorded, correctly-scoped
risk as a finding would be the "observation nobody needs to act on" the brief
says to keep out of the checkbox list.

## Growth of root handles on `Main.qml`

Thirteen read-only projections now live on `Main.qml`'s root (six from
`e2e-ui-suite`, seven added here). The file already draws the boundary this
question is asking whether it needs: a marked section
(`// ---- what the end-to-end suite reads ----`) states the rule for
everything under it — read-only, each a projection of state a screen already
owns, "the suite's interface, and nothing else reads them" — and gives the
reason `screenShown` may never gain a key/identity handle (so the boundary is
not just aspirational, it already turned down one addition). I verified the
"nothing else reads them" claim rather than trusting the comment:

```
git grep -n "listedStoas\|createdStoa\|feedReadState\|feedRowCount\|feedCanPost\|threadReadState\|threadItemCount" -- '*.qml' '*.yaml'
```

Every hit outside `dialectica-ui/tests/` is either the property declaration
itself or a comment; no production QML reads any of the seven new handles.
The section is a real boundary, consistently kept, not a surface drifting
open. The only thing worth naming for a future reviewer: the boundary is
enforced by a grep any reviewer can run, not by a QML access-control
mechanism — there is nothing that would *fail the build* if a screen started
reading `root.feedRowCount`. That is consistent with how the rest of this
codebase enforces conventions (documented + checked, not machine-enforced)
and I am not treating it as a defect.

## The four specs' structure and matrix

`create.yaml`, `feed.yaml`, `thread.yaml`, `moderation.yaml` share one shape
(D1/D2): a prefix that only waits for what the next action needs and asserts
nothing about screens already covered by an earlier spec, then one block per
spec asserting the one observation it owns. I counted every file's steps by
hand against `tasks.md`'s predicted/observed counts and they match: `create`
12, `feed` 18 (17 + the D9 `wait_for` step), `thread` 23 (21 + one D9 step),
`moderation` 15. The matrix (`ui-tests.yml`, `spec: [join, create, feed,
thread, moderation]`) is the one hand-maintained list here, and it is checked
against the directory by count in the same workflow ("Every spec in the tree
is in the matrix," comparing `find dialectica-ui/tests/ui` against
`strategy.job-total`) — so a spec added to the directory and not the matrix
fails loudly rather than silently running nowhere, closing the
hand-maintained-list trap CLAUDE.md warns about. `join.yaml`'s pruned "does
NOT cover" paragraph now names only the successful join, matching what the
other three files actually now cover — checked against the diff, not assumed.

## Readiness for the next piece (the seeder)

This piece leaves no structural debt for the join-with-seeder piece. No view
code changes are proposed or made for it; `join.yaml`'s scope statement is
narrowed accurately rather than left overclaiming; the properties the
proposal says the next piece's seeder reference needs (decodable record,
address as its hash, an unheld Stoa name, a recognisable title) are stated as
requirements on the *reference*, not on any interface this piece would need
to change. I found nothing in `Main.qml`, `DJoinScreen.qml` or the composer
that the next piece would need to touch differently because of what landed
here.

## Also checked

- **Dependencies:** none added — `git diff fe093be...HEAD --stat -- '*.toml'
  '*.lock' '*.nix' package.json` is empty.
- **`cargo mutants`:** not applicable — `git diff fe093be...HEAD --stat --
  '*.rs'` is empty; this piece touches no Rust.
- **CI gates deriving expectations from source layout:** `.github/workflows/
  ci.yml` is untouched by this diff. Its `ui-specs` job globs
  `dialectica-ui/tests/ui/` from the directory rather than a hand-kept list
  (comment at ci.yml:1123), so the four new spec files need no matching edit
  there. No file was moved or renamed in this diff, so there is no
  gate-measuring-a-directory-that-moved risk to check.
- **Following the radicle module's e2e setup / preferring `lgs` in CI:**
  `ui-tests.yml`'s only change in this diff is the matrix line; the `lgs
  basecamp setup`/`install` shape (already further from radicle's `lgs
  basecamp build` than radicle itself, per the file's own header comment) predates
  this piece and this piece does not regress it.
- **`DComposer`'s `kind`-derived names:** only `"post"` and `"reply"` values
  exist (`DComposer.qml:21`), so `root.kind + "DraftField"`/`"SubmitButton"`
  cannot collide with a third kind that does not exist.

## Gate record

- [x] **Architecture review of `e2e-created-stoa-flow`: clean.** No blocking
      finding. One already-tracked, correctly-scoped forward risk (the D8
      guard-copy pattern) is discussed above and deliberately left as prose,
      not a checkbox, per the brief's "do not open a box for an observation
      nobody needs to act on."
